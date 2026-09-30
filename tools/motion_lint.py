#!/usr/bin/env python3
"""motion_lint — a mechanical motion gate for retwisted modules.

Reads a motion trace written by `frame --trace` (one JSON line per sampled
instant, each holding every sprite the module would draw then) and prints
comparable motion numbers: how far things move per frame, how often they
freeze-then-lurch, whether they face the way they walk, whether they
teleport, whether they sit dead while their siblings move, and how the
sound cues are spaced.

Why it exists: lanes self-report, and a static PNG render cannot show
jerk. Three modules shipped as "fixed" with visible motion defects
(toxic-swamp fish going backwards, boris' cats, shock-clocks' monkey).
This prints numbers a lane has to clear before it may say "fixed", and
`--fail-on` makes it a ratchet.

Usage
  tools/motion_lint.py trace.jsonl
  tools/motion_lint.py trace.jsonl --json summary.json
  tools/motion_lint.py trace.jsonl --fail-on jerk=0,teleports=0,backwards=0.02

`tools/capture_track.py` feeds the SAME stitcher and the SAME table from a
QEMU capture, so the port and the golden capture can be read side by side.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import sys
from collections import defaultdict

# --------------------------------------------------------------------------
# observations and tracks
#
# An observation is one drawn thing at one instant. The port trace gives a
# sprite (png / x / y / flip / pal / w / h); the capture tracker gives a
# blob centroid. Both land in the same dict so the stitcher and every
# statistic below are shared.


def obs(t, x, y, cls, png=None, flip=None, pal=None, w=None, h=None, area=None,
        ox=None, oy=None):
    return {
        "t": t,
        "x": float(x),
        "y": float(y),
        # the sprite's own top-left, kept whatever --anchor does to x/y:
        # the open-water check needs a real box, not a centroid.
        "ox": ox,
        "oy": oy,
        "cls": cls,
        "png": png,
        "flip": flip,
        "pal": pal,
        "w": w,
        "h": h,
        "area": area,
    }


def sprite_class(png: str) -> str:
    """Class hint: the compound's directory, i.e. the series/base it came
    from (`compounds/2000/c_656.png` -> `compounds/2000`). Actors of one
    kind are packed together, so this keeps a fish from being stitched onto
    a piece of scenery — and it is the only free class signal a sprite has.
    """
    d = os.path.dirname(png)
    return d if d else "(root)"


def sprite_anchor(s, anchor="center"):
    """Where we consider the sprite to BE.

    `center` is the centroid of its box, which is what the video tracker
    measures off a capture — use it whenever port and capture numbers are
    going to be read side by side.

    `origin` is the sprite's own top-left, i.e. the module's position
    variable. It is the cleaner signal for port-internal defect hunting:
    art that changes size frame to frame (a mime's arms swinging) moves its
    centroid by a couple of px while the actor has not moved at all, and
    that wobble shows up as reversals. The capture cannot tell those apart;
    the trace can, so let it.
    """
    x, y = float(s["x"]), float(s["y"])
    w, h = s.get("w"), s.get("h")
    if anchor == "center" and w and h:
        return x + w / 2.0, y + h / 2.0
    return x, y


def read_trace(path, anchor="center", only=None, exclude=None):
    """-> (frames, sound_fires, dropped). frames is [(t, [obs, ...]), ...].

    `only`/`exclude` are lists of regexes matched against the png path.
    They exist because "the compound directory is the class" is not always
    enough: Mowin' Boris draws every tile of mown trail as a sprite in the
    same series as the mower and the cats, so by the end of a minute the
    class is 2028 tiles and 20 actors, and every statistic is a statistic
    about grass. `--exclude-png 'c_005[.]png$'` hands the lint the cast.
    """
    frames = []
    fires = []  # (t, id)
    dropped = 0
    only = [re.compile(p) for p in (only or [])]
    exclude = [re.compile(p) for p in (exclude or [])]
    with open(path) as f:
        for ln, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError as e:
                raise SystemExit(f"{path}:{ln}: not JSON: {e}")
            t = rec["t"]
            os_ = []
            for s in rec.get("sprites", []):
                png = s["png"]
                if only and not any(p.search(png) for p in only):
                    dropped += 1
                    continue
                if any(p.search(png) for p in exclude):
                    dropped += 1
                    continue
                cx, cy = sprite_anchor(s, anchor)
                os_.append(
                    obs(
                        t,
                        cx,
                        cy,
                        sprite_class(s["png"]),
                        png=s["png"],
                        flip=s.get("flip"),
                        pal=s.get("pal"),
                        w=s.get("w"),
                        h=s.get("h"),
                        ox=float(s["x"]),
                        oy=float(s["y"]),
                    )
                )
            frames.append((t, os_))
            for sid in rec.get("sounds", []):
                fires.append((t, sid))
    return frames, fires, dropped


def dist(a, b):
    return math.hypot(a["x"] - b["x"], a["y"] - b["y"])


def stitch(frames, teleport=60.0, match_max=None, gap=0):
    """Nearest-neighbour stitch of per-frame observations into tracks.

    Matching is greedy shortest-first within a class: the closest
    (track, observation) pair wins, then the next closest among what is
    left. A match farther than `teleport` still matches — otherwise a
    teleport would read as "one track died, another was born", and the
    defect would be invisible — but it CLOSES the track and opens a new
    one, and is reported as a teleport event.

    `gap` is how many consecutive frames a track may go unseen before it
    is closed (default 0: a disappearance breaks the track, as specified).

    Returns (tracks, teleports). A track is
      {"cls", "pts": [obs...], "steps": [(t, d)], "teleport_in": bool}
    """
    if match_max is None:
        match_max = teleport * 3
    active = []  # {"track":…, "last":obs, "missing":int}
    done = []
    teleports = []

    cell = max(1.0, match_max)
    for t, observations in frames:
        used_a, used_o = set(), set()
        matched = []

        # Stage 0, the fast path: the same art at exactly the same place is
        # certainly the same actor. Mowin' Boris ends a minute with 2048
        # sprites on screen (every tile of mown trail is one), and the
        # all-pairs search below is quadratic — without this, a boris trace
        # does not finish. It is also simply correct: d = 0 would win the
        # geometric match anyway.
        exact = defaultdict(list)
        for oi, o in enumerate(observations):
            exact[(o["cls"], o["png"], o["x"], o["y"])].append(oi)
        for ai, a in enumerate(active):
            last = a["last"]
            bucket = exact.get((a["track"]["cls"], last["png"], last["x"], last["y"]))
            if bucket:
                oi = bucket.pop()
                used_a.add(ai)
                used_o.add(oi)
                matched.append((0.0, ai, oi))

        # Stage 1: nearest neighbour for whatever moved, over a uniform
        # grid of `match_max`-sized cells so a crowded frame stays linear.
        grid = defaultdict(list)
        for oi, o in enumerate(observations):
            if oi in used_o:
                continue
            grid[(int(o["x"] // cell), int(o["y"] // cell), o["cls"])].append(oi)
        pairs = []
        for ai, a in enumerate(active):
            if ai in used_a:
                continue
            last = a["last"]
            cls = a["track"]["cls"]
            gx, gy = int(last["x"] // cell), int(last["y"] // cell)
            for ddx in (-1, 0, 1):
                for ddy in (-1, 0, 1):
                    for oi in grid.get((gx + ddx, gy + ddy, cls), ()):
                        d = dist(last, observations[oi])
                        if d <= match_max:
                            pairs.append((d, ai, oi))
        pairs.sort()
        for d, ai, oi in pairs:
            if ai in used_a or oi in used_o:
                continue
            used_a.add(ai)
            used_o.add(oi)
            matched.append((d, ai, oi))

        next_active = []
        for d, ai, oi in sorted(matched, key=lambda m: m[1]):
            a = active[ai]
            o = observations[oi]
            if d > teleport:
                teleports.append(
                    {
                        "t": t,
                        "cls": a["track"]["cls"],
                        "d": round(d, 2),
                        "from": [round(a["last"]["x"], 1), round(a["last"]["y"], 1)],
                        "to": [round(o["x"], 1), round(o["y"], 1)],
                        "png": o.get("png"),
                    }
                )
                done.append(a["track"])
                trk = {"cls": o["cls"], "pts": [o], "steps": [], "teleport_in": True}
                next_active.append({"track": trk, "last": o, "missing": 0})
            else:
                a["track"]["pts"].append(o)
                a["track"]["steps"].append((t, d))
                a["last"] = o
                a["missing"] = 0
                next_active.append(a)

        for ai, a in enumerate(active):
            if ai in used_a:
                continue
            a["missing"] += 1
            if a["missing"] > gap:
                done.append(a["track"])
            else:
                next_active.append(a)

        for oi, o in enumerate(observations):
            if oi in used_o:
                continue
            trk = {"cls": o["cls"], "pts": [o], "steps": [], "teleport_in": False}
            next_active.append({"track": trk, "last": o, "missing": 0})

        active = next_active

    done.extend(a["track"] for a in active)
    return done, teleports


# --------------------------------------------------------------------------
# statistics


def quantile(vals, q):
    """Nearest-rank quantile — no numpy, and no interpolation games on the
    tiny samples a short trace produces."""
    if not vals:
        return 0.0
    s = sorted(vals)
    if q <= 0:
        return s[0]
    k = max(0, min(len(s) - 1, int(math.ceil(q * len(s))) - 1))
    return s[k]


def median(vals):
    if not vals:
        return 0.0
    s = sorted(vals)
    n = len(s)
    return s[n // 2] if n % 2 else (s[n // 2 - 1] + s[n // 2]) / 2.0


def hold_runs(track, moved_eps=0.5):
    """Split a track into HOLD RUNS and the ADVANCE that ends each one.

    A hold run is a maximal stretch of consecutive samples showing the same
    art at the same place; the advance is the move that ends it. This is
    the unit every cadence statistic below is built on, and it is the thing
    that makes the numbers sample-rate independent.

    It matters because raw per-sample steps do NOT distinguish a defect
    from a sampling artefact: sample a 100 ms animation every 50 ms and
    every single advance reads as "held 2 frames, then lurched". Measured
    against the track's OWN median run length that reads as perfectly
    regular — which it is.

    Returns [{"len", "t0", "t1", "png", "step", "dx", "dy", "t_adv"}, ...];
    `step`/`t_adv` are None on the last run (nothing follows it).
    """
    pts = track["pts"]
    out = []
    i = 0
    while i < len(pts):
        j = i
        while (
            j + 1 < len(pts)
            and pts[j + 1]["png"] == pts[i]["png"]
            and abs(pts[j + 1]["x"] - pts[i]["x"]) <= moved_eps
            and abs(pts[j + 1]["y"] - pts[i]["y"]) <= moved_eps
        ):
            j += 1
        run = {
            "len": j - i + 1,
            "i0": i,
            "i1": j,
            "i_adv": j + 1 if j + 1 < len(pts) else None,
            "t0": pts[i]["t"],
            "t1": pts[j]["t"],
            "png": pts[i]["png"],
            "step": None,
            "dx": None,
            "dy": None,
            "t_adv": None,
        }
        if j + 1 < len(pts):
            run["step"] = dist(pts[j], pts[j + 1])
            run["dx"] = pts[j + 1]["x"] - pts[j]["x"]
            run["dy"] = pts[j + 1]["y"] - pts[j]["y"]
            run["t_adv"] = pts[j + 1]["t"]
        out.append(run)
        i = j + 1
    return out


def find_jerks(track, factor=2.0, hold=2, min_px=2.0, moved_eps=0.5):
    """The frozen-duplicate-frame signature, measured against the track's
    own cadence: a hold run at least `factor`x longer than this track
    normally holds, ended by an advance at least `factor`x bigger than this
    track normally advances. "It skipped its update for a few ticks, then
    caught up in one hop" — toxic f537105 was exactly this.

    Both halves are required. A long hold on its own is an actor standing
    still (that is `stuck`); a big step on its own is a teleport or a fast
    actor. It is the pairing that is the defect, and pairing them also
    means a lane cannot make the number go away by freezing harder.

    `min_px` only suppresses sub-pixel noise; the real bar is relative.
    """
    runs = hold_runs(track, moved_eps)
    advs = [r for r in runs if r["step"] is not None]
    if len(advs) < 4:  # too short to have a "normal" to deviate from
        return []
    med_run = median([r["len"] for r in runs])
    med_step = median([r["step"] for r in advs if r["step"] > moved_eps]) or 0.0
    out = []
    for r in advs:
        if r["len"] < max(hold, factor * med_run):
            continue
        if r["step"] < max(factor * med_step, min_px):
            continue
        out.append(
            {
                "t": r["t_adv"],
                "held_ms": r["t1"] - r["t0"],
                "held_frames": r["len"],
                "normal_hold": med_run,
                "step": round(r["step"], 2),
                "normal_step": round(med_step, 2),
                "png": r["png"],
            }
        )
    return out


def reversal_fraction(tracks, min_dx=1.0, moved_eps=0.5):
    """How often a track reverses horizontal direction between consecutive
    ADVANCES (not samples — a held frame is not a direction change).

    This is the orientation-free half of "going backwards": art that faces
    nowhere in particular still should not shuttle left-right-left every
    other step. A fish crossing the tank scores near 0; a fish whose
    position update is fighting itself scores high. Unlike the flip test
    below it needs no assumption about which way the art faces, so it is
    the one that still says something when a module never flips at all.

    Returns (fraction, n_pairs_considered).
    """
    rev = n = 0
    for tr in tracks:
        signs = [
            1 if r["dx"] > 0 else -1
            for r in hold_runs(tr, moved_eps)
            if r["dx"] is not None and abs(r["dx"]) >= min_dx
        ]
        for a, b in zip(signs, signs[1:]):
            n += 1
            if a != b:
                rev += 1
    return (rev / n if n else None), n


def backwards_fractions(tracks, min_dx=1.0):
    """Does the flip flag agree with the direction of travel?

    Unflipped art could face either way; the art itself does not say. So
    both hypotheses are scored and BOTH reported:
      A: unflipped art faces +x (right) -> a sprite moving left must flip
      B: unflipped art faces -x (left)  -> a sprite moving right must flip
    The lower fraction names the hypothesis the module is (mostly) living
    by; the value itself is the share of moving frames that contradict it —
    i.e. the sprites moonwalking.

    The two hypotheses are exact mirrors, so they always sum to 1 — which
    means the number says NOTHING unless the flip flag actually varies
    within the class. A class that never flips (or always flips) gets
    `flip_var = 0` and the fractions come back None: report "n/a", do not
    let a lane read 50% as a finding. `reversal_fraction` is the metric
    that still works there.

    Returns (frac_A, frac_B, n_frames_considered, flip_var).
    """
    bad_a = bad_b = n = flipped = 0
    for tr in tracks:
        pts = tr["pts"]
        for r in hold_runs(tr):
            if r["dx"] is None or abs(r["dx"]) < min_dx:
                continue
            # the flip flag AFTER the advance: which way it now faces
            after = pts[r["i_adv"]]
            if after["flip"] is None:
                continue
            n += 1
            flipped += 1 if after["flip"] else 0
            faces_right = not after["flip"]  # hypothesis A
            if (r["dx"] > 0) != faces_right:
                bad_a += 1
            if (r["dx"] > 0) == faces_right:  # hypothesis B is the mirror
                bad_b += 1
    if n == 0:
        return None, None, 0, None
    flip_var = flipped / n
    if flipped == 0 or flipped == n:
        return None, None, n, flip_var
    return bad_a / n, bad_b / n, n, flip_var


def art_direction_overlap(tracks, min_dx=1.0):
    """Is the art directional, or is the actor moonwalking?

    The flip test above goes silent on a module that never sets the flip
    bit — and toxic-swamp never does, not once in 74k sprite draws. That
    leaves two possibilities, which look identical in a static render:
      * the module carries SEPARATE left-facing and right-facing art and
        picks per direction, so flip is correctly unused; or
      * the module draws one set of art whichever way the actor is going,
        and half of them swim backwards.
    They are distinguishable: collect, per png, the directions it was on
    screen for while moving. Directional art gives two disjoint sets;
    moonwalking gives one set used both ways.

    Returns {"pngs", "bidirectional", "frac_pngs", "frac_advances",
             "worst": [(png, left, right)]}, or None when nothing moved.
    """
    dirs = defaultdict(lambda: [0, 0])  # png -> [left, right] advances
    for tr in tracks:
        pts = tr["pts"]
        for r in hold_runs(tr):
            if r["dx"] is None or abs(r["dx"]) < min_dx:
                continue
            png = pts[r["i_adv"]]["png"]
            if png is None:
                return None
            dirs[png][0 if r["dx"] < 0 else 1] += 1
    if not dirs:
        return None
    both = {p: lr for p, lr in dirs.items() if lr[0] and lr[1]}
    adv_total = sum(sum(lr) for lr in dirs.values())
    adv_both = sum(sum(lr) for lr in both.values())
    worst = sorted(both.items(), key=lambda kv: -min(kv[1]))[:5]
    return {
        "pngs": len(dirs),
        "bidirectional": len(both),
        "frac_pngs": round(len(both) / len(dirs), 4),
        "frac_advances": round(adv_both / adv_total, 4) if adv_total else 0.0,
        "worst": [{"png": p, "left": lr[0], "right": lr[1]} for p, lr in worst],
    }


def open_water_births(tracks, t0, screen=(640, 480), margin=2, settle_ms=1000):
    """Tracks that APPEAR fully inside the screen, rather than entering
    across an edge.

    Toxic-swamp's own errata pins this from the golden capture (120 s,
    template-matched): critters enter and leave across a side edge, ~46
    entries and ~43 exits, and NOT ONE fish ever materialises in open
    water. So "a sprite appeared with its whole box on screen and nothing
    of its class near it last frame" is a defect the capture has already
    ruled out — cheap to count, and it needs the box size, which only the
    port trace has.

    `settle_ms` skips the module's opening moments: every module places
    its initial cast in the first tick or two, and those placements are
    not entries.

    Caveat worth keeping in mind when reading the number: where several
    actors overlap, the stitcher can lose one and re-acquire it as a new
    track, which lands here too. Treat it as a worklist, not a verdict.
    """
    w, h = screen
    if not any(tr["pts"][0].get("w") for tr in tracks):
        # no box sizes -> this is a capture, where every blob is its own
        # bounds and "fully on screen" would be true of almost all of
        # them. The check needs the trace; say so instead of lying.
        return None
    out = []
    for tr in tracks:
        p = tr["pts"][0]
        if p["t"] <= t0 + settle_ms:
            continue
        bw, bh = p.get("w") or 0, p.get("h") or 0
        px = p["ox"] if p.get("ox") is not None else p["x"]
        py = p["oy"] if p.get("oy") is not None else p["y"]
        if px < margin or px + bw > w - margin:
            continue
        if py < margin or py + bh > h - margin:
            continue
        out.append(
            {
                "t": p["t"],
                "cls": tr["cls"],
                "at": [round(px, 1), round(py, 1)],
                "size": [bw, bh],
                "png": p["png"],
                "lived_ms": tr["pts"][-1]["t"] - p["t"],
            }
        )
    return out


def find_stuck(tracks, stuck_ms, moved_eps=0.5, max_static_frac=0.5):
    """A track that never moves and never changes art for longer than
    `stuck_ms`, WHILE another track of the same class is moving in the same
    window. The sibling clause is what separates "this module is idle by
    design" from "the monkey doesn't animate": if every clock in the class
    is frozen, that is a module-wide question, not a stuck actor.
    """
    moving_windows = defaultdict(list)
    per_cls = defaultdict(list)
    for tr in tracks:
        per_cls[tr["cls"]].append(tr)
        if any(d > moved_eps for _, d in tr["steps"]):
            moving_windows[tr["cls"]].append((tr["pts"][0]["t"], tr["pts"][-1]["t"], id(tr)))

    # Classes that are mostly motionless are SCENERY, not stuck actors.
    # Mowin' Boris draws every tile of mown trail as its own sprite in the
    # same series as the mower and the cats; without this guard a boris
    # trace reports two thousand stuck actors and the one that matters is
    # invisible. The signal we want is the minority case — one monkey
    # frozen while its siblings move.
    scenery = set()
    for cls, trs in per_cls.items():
        static = sum(1 for tr in trs if not any(d > moved_eps for _, d in tr["steps"]))
        if trs and static / len(trs) > max_static_frac:
            scenery.add(cls)

    out = []
    for tr in tracks:
        pts = tr["pts"]
        if len(pts) < 2 or tr["cls"] in scenery:
            continue
        span = pts[-1]["t"] - pts[0]["t"]
        if span < stuck_ms:
            continue
        if any(d > moved_eps for _, d in tr["steps"]):
            continue
        if any(p["png"] != pts[0]["png"] for p in pts):
            continue
        sibling = any(
            oid != id(tr) and s < pts[-1]["t"] and pts[0]["t"] < e
            for s, e, oid in moving_windows[tr["cls"]]
        )
        if not sibling:
            continue
        out.append(
            {
                "cls": tr["cls"],
                "t0": pts[0]["t"],
                "t1": pts[-1]["t"],
                "ms": span,
                "at": [round(pts[0]["x"], 1), round(pts[0]["y"], 1)],
                "png": pts[0]["png"],
            }
        )
    return out


def sound_stats(fires):
    """Per snd id: how many fires, and the tightest gap between two of them.

    Resolution is the sample interval, so two fires inside one sample read
    as gap 0 — reported separately as `burst_max` (most fires of that id
    carried by a single line) rather than pretended to be a real gap.
    """
    per = defaultdict(list)
    for t, sid in fires:
        per[sid].append(t)
    out = {}
    for sid, ts in sorted(per.items()):
        ts.sort()
        gaps = [b - a for a, b in zip(ts, ts[1:]) if b > a]
        burst = defaultdict(int)
        for t in ts:
            burst[t] += 1
        out[str(sid)] = {
            "count": len(ts),
            "min_gap_ms": min(gaps) if gaps else None,
            "median_gap_ms": median(gaps) if gaps else None,
            "burst_max": max(burst.values()),
            "first_ms": ts[0],
            "last_ms": ts[-1],
        }
    return out


def analyse(
    frames,
    fires,
    label,
    teleport=60.0,
    gap=0,
    stuck_s=10.0,
    jerk_factor=2.0,
    jerk_hold=2,

    jerk_min_px=2.0,
    match_max=None,
    anchor="center",
    screen=(640, 480),
    settle_ms=1000,
):
    """Everything above, assembled into the machine-readable summary."""
    tracks, teleports = stitch(frames, teleport=teleport, match_max=match_max, gap=gap)
    by_cls = defaultdict(list)
    for tr in tracks:
        by_cls[tr["cls"]].append(tr)

    times = [t for t, _ in frames]
    dts = [b - a for a, b in zip(times, times[1:])]
    classes = {}
    for cls, trs in sorted(by_cls.items()):
        steps = [d for tr in trs for _, d in tr["steps"]]
        med = median(steps)
        med_move = median([d for d in steps if d >= 0.5])
        runs = [r for tr in trs for r in hold_runs(tr)]
        run_lens = [r["len"] for r in runs]
        advances = [r["step"] for r in runs if r["step"] is not None and r["step"] >= 0.5]
        jerks = []
        for tr in trs:
            jerks.extend(
                find_jerks(tr, factor=jerk_factor, hold=jerk_hold, min_px=jerk_min_px)
            )
        jerks.sort(key=lambda j: -j["step"])
        fa, fb, nb, flip_var = backwards_fractions(trs)
        rev, nrev = reversal_fraction(trs)
        art = art_direction_overlap(trs)
        static = sum(1 for tr in trs if not any(d > 0.5 for _, d in tr["steps"]))
        stuck = [s for s in find_stuck(trs, stuck_s * 1000.0) if s["cls"] == cls]
        open_water = open_water_births(trs, times[0], screen=screen, settle_ms=settle_ms)
        tele = [e for e in teleports if e["cls"] == cls]
        long_tracks = [tr for tr in trs if len(tr["pts"]) > 1]
        classes[cls] = {
            "tracks": len(trs),
            "tracks_multiframe": len(long_tracks),
            "observations": sum(len(tr["pts"]) for tr in trs),
            "step": {
                "n": len(steps),
                "median": round(med, 2),
                "median_moving": round(med_move, 2),
                "p95": round(quantile(steps, 0.95), 2),
                "max": round(max(steps), 2) if steps else 0.0,
                "zero_frac": round(sum(1 for d in steps if d < 0.5) / len(steps), 3)
                if steps
                else None,
            },
            # cadence: how long the art holds still between advances, and
            # how far it moves when it does. Sample-rate independent, so
            # this is the honest read of "is the animation smooth".
            "cadence": {
                "runs": len(runs),
                "hold_median": round(median(run_lens), 2),
                "hold_p95": round(quantile(run_lens, 0.95), 2),
                "hold_max": max(run_lens) if run_lens else 0,
                "advance_median": round(median(advances), 2),
                "advance_p95": round(quantile(advances, 0.95), 2),
                "advance_max": round(max(advances), 2) if advances else 0.0,
            },
            "jerk": {
                "count": len(jerks),
                "worst": jerks[:5],
            },
            "backwards": {
                "frames": nb,
                "flip_varies": None if flip_var is None else round(flip_var, 4),
                "hypothesis_a_unflipped_faces_right": None if fa is None else round(fa, 4),
                "hypothesis_b_unflipped_faces_left": None if fb is None else round(fb, 4),
                "best": None if fa is None else ("A" if fa <= fb else "B"),
                "fraction": None if fa is None else round(min(fa, fb), 4),
            },
            "reversals": {
                "pairs": nrev,
                "fraction": None if rev is None else round(rev, 4),
            },
            # the fallback when flip never varies: does the same art get
            # drawn going both ways?
            "art_direction": art,
            "teleports": {"count": len(tele), "worst": sorted(tele, key=lambda e: -e["d"])[:5]},
            "stuck": {
                "count": len(stuck),
                "worst": sorted(stuck, key=lambda s: -s["ms"])[:5],
                "static_tracks": static,
                # most of this class never moves -> it is scenery, and the
                # stuck detector stands down rather than reporting all of it
                "scenery": bool(trs) and static / len(trs) > 0.5,
            },
            # mid-trace track churn. A jump farther than `match_max` cannot
            # be told from "one actor left, another arrived", so it lands
            # here instead of in `teleports` — which is honest, but means a
            # class with churn it should not have (a fish that respawns
            # every second) has to be read off THIS number, not that one.
            "churn": {
                "births": sum(1 for tr in trs if tr["pts"][0]["t"] > times[0]),
                "deaths": sum(1 for tr in trs if tr["pts"][-1]["t"] < times[-1]),
                "open_water": None if open_water is None else len(open_water),
                "open_water_worst": []
                if open_water is None
                else sorted(open_water, key=lambda b: -b["lived_ms"])[:5],
            },
            "span_px": {
                "x": round(max((p["x"] for tr in trs for p in tr["pts"]), default=0)
                           - min((p["x"] for tr in trs for p in tr["pts"]), default=0), 1),
                "y": round(max((p["y"] for tr in trs for p in tr["pts"]), default=0)
                           - min((p["y"] for tr in trs for p in tr["pts"]), default=0), 1),
            },
        }

    totals = {
        "jerk": sum(c["jerk"]["count"] for c in classes.values()),
        "open_water": sum(
            c["churn"]["open_water"] or 0 for c in classes.values()
        )
        if any(c["churn"]["open_water"] is not None for c in classes.values())
        else None,
        "teleports": sum(c["teleports"]["count"] for c in classes.values()),
        "stuck": sum(c["stuck"]["count"] for c in classes.values()),
        "backwards": max(
            [c["backwards"]["fraction"] for c in classes.values() if c["backwards"]["fraction"] is not None]
            or [0.0]
        ),
        "reversals": max(
            [c["reversals"]["fraction"] for c in classes.values() if c["reversals"]["fraction"] is not None]
            or [0.0]
        ),
    }
    return {
        "source": label,
        "frames": len(frames),
        "t_ms": [times[0], times[-1]] if times else [0, 0],
        "sample_ms": {"median": median(dts), "min": min(dts) if dts else 0, "max": max(dts) if dts else 0},
        "params": {
            "anchor": anchor,
            "teleport_px": teleport,
            "gap_frames": gap,
            "stuck_s": stuck_s,
            "jerk_factor": jerk_factor,
            "jerk_hold": jerk_hold,

            "jerk_min_px": jerk_min_px,
            "settle_ms": settle_ms,
        },
        "classes": classes,
        "sounds": sound_stats(fires),
        "totals": totals,
    }


# --------------------------------------------------------------------------
# human output


def table(summary, out=sys.stdout):
    """The short human table. `capture_track.py` prints this exact table for
    a QEMU capture, so the two can be diffed by eye, column by column."""
    s = summary
    span = (s["t_ms"][1] - s["t_ms"][0]) / 1000.0
    print(
        f"{s['source']}: {s['frames']} samples over {span:.1f}s "
        f"(every ~{s['sample_ms']['median']:.0f} ms)",
        file=out,
    )
    hdr = (
        f"{'class':<22} {'trk':>4} {'obs':>6} | {'med':>5} {'p95':>5} {'max':>6} {'still':>6}"
        f" | {'hold':>5} {'hmax':>4} {'adv':>5} {'ap95':>5} | "
        f"{'rev':>5} {'jerk':>5} {'tele':>4} {'stk':>3}  {'backwards':>22}"
    )
    print(hdr, file=out)
    print("-" * len(hdr), file=out)
    for cls, c in s["classes"].items():
        st = c["step"]
        cd = c["cadence"]
        b = c["backwards"]
        if b["fraction"] is None:
            bs = "n/a (flip constant)" if b["frames"] else "n/a (no motion)"
        else:
            bs = (
                f"{b['fraction'] * 100:5.1f}% [{b['best']}]"
                f" A={b['hypothesis_a_unflipped_faces_right'] * 100:.0f}"
                f"/B={b['hypothesis_b_unflipped_faces_left'] * 100:.0f}"
            )
        rev = c["reversals"]["fraction"]
        print(
            f"{cls[:22]:<22} {c['tracks']:>4} {c['observations']:>6} | "
            f"{st['median']:>5.1f} {st['p95']:>5.1f} {st['max']:>6.1f} "
            f"{(st['zero_frac'] if st['zero_frac'] is not None else 0) * 100:>5.0f}% | "
            f"{cd['hold_median']:>5.1f} {cd['hold_max']:>4} "
            f"{cd['advance_median']:>5.1f} {cd['advance_p95']:>5.1f} | "
            f"{'  n/a' if rev is None else format(rev * 100, '4.0f') + '%'} "
            f"{c['jerk']['count']:>5} {c['teleports']['count']:>4}"
            f" {'scn' if c['stuck']['scenery'] else c['stuck']['count']:>3}"
            f"  {bs:>22}",
            file=out,
        )
    t = s["totals"]
    print(
        f"TOTAL jerk={t['jerk']} teleports={t['teleports']} stuck={t['stuck']} "
        f"backwards={t['backwards'] * 100:.1f}% reversals={t['reversals'] * 100:.1f}%"
        f" open_water={'n/a' if t['open_water'] is None else t['open_water']}",
        file=out,
    )

    for cls, c in s["classes"].items():
        for j in c["jerk"]["worst"][:3]:
            print(
                f"  jerk  {cls} @{j['t']}ms: held {j['held_frames']} frames"
                f" ({j['held_ms']}ms, normal {j['normal_hold']}) then stepped"
                f" {j['step']}px (normal {j['normal_step']})  {j['png'] or ''}",
                file=out,
            )
        ad = c.get("art_direction")
        if ad and ad["bidirectional"]:
            print(
                f"  art   {cls}: {ad['bidirectional']}/{ad['pngs']} moving compounds are drawn"
                f" BOTH ways ({ad['frac_advances'] * 100:.0f}% of advances)"
                f" — expected for art with no facing (a crosshair, a ball),"
                f" a moonwalk for art with a face",
                file=out,
            )
            for w in ad["worst"][:2]:
                print(f"          {w['png']}: {w['left']} left / {w['right']} right", file=out)
        for e in c["teleports"]["worst"][:3]:
            print(
                f"  tele  {cls} @{e['t']}ms: {e['from']} -> {e['to']} = {e['d']}px",
                file=out,
            )
        if c["churn"]["births"] or c["churn"]["deaths"]:
            print(
                f"  churn {cls}: {c['churn']['births']} tracks born mid-trace,"
                f" {c['churn']['deaths']} died,"
                f" {'n/a (no box sizes)' if c['churn']['open_water'] is None else c['churn']['open_water']}"
                f" of the births fully on screen (not entering across an edge)",
                file=out,
            )
            for b in c["churn"]["open_water_worst"][:3]:
                print(
                    f"          open water @{b['t']}ms at {b['at']} {b['size'][0]}x{b['size'][1]}"
                    f" lived {b['lived_ms'] / 1000:.1f}s  {b['png'] or ''}",
                    file=out,
                )
        for k in c["stuck"]["worst"][:3]:
            print(
                f"  stuck {cls} @{k['t0']}-{k['t1']}ms at {k['at']} ({k['ms'] / 1000:.1f}s)"
                f"  {k['png'] or ''}",
                file=out,
            )
    if s["sounds"]:
        print("  sounds:", file=out)
        for sid, v in s["sounds"].items():
            gap = "n/a" if v["min_gap_ms"] is None else f"{v['min_gap_ms']}ms"
            print(
                f"    snd {sid:>6}: {v['count']:>4} fires, min gap {gap},"
                f" burst {v['burst_max']}, {v['first_ms']}..{v['last_ms']}ms",
                file=out,
            )


def parse_fail_on(spec):
    """`jerk=0,teleports=2,backwards=0.05` -> {"jerk": 0.0, ...}"""
    out = {}
    for part in spec.split(","):
        part = part.strip()
        if not part:
            continue
        k, _, v = part.partition("=")
        k = k.strip()
        if k not in ("jerk", "teleports", "backwards", "stuck", "reversals", "open_water"):
            raise SystemExit(
                f"--fail-on: unknown key {k!r}"
                " (jerk|teleports|backwards|stuck|reversals|open_water)"
            )
        out[k] = float(v)
    return out


def check_thresholds(summary, thresholds, out=sys.stdout):
    """-> list of failure strings (empty means the gate passed)."""
    fails = []
    for k, limit in sorted(thresholds.items()):
        got = summary["totals"][k]
        if got is None:
            fails.append(f"{k}=n/a (not measurable from this source)")
            continue
        if got > limit:
            fails.append(f"{k}={got} > {limit}")
    for f in fails:
        print(f"FAIL {f}", file=out)
    return fails


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("trace", help="trace.jsonl from `frame --trace`")
    ap.add_argument("--teleport", type=float, default=60.0, help="step px that breaks a track (default 60)")
    ap.add_argument("--match-max", type=float, default=None, help="max px a track may reach to match (default 3x teleport)")
    ap.add_argument("--gap", type=int, default=0, help="frames a track may go unseen (default 0)")
    ap.add_argument("--stuck", type=float, default=10.0, help="seconds motionless before stuck (default 10)")
    ap.add_argument("--jerk-factor", type=float, default=2.0, help="step multiple after a hold (default 2)")
    ap.add_argument("--jerk-hold", type=int, default=2, help="frames held before a lurch counts (default 2)")

    ap.add_argument("--jerk-min-px", type=float, default=2.0, help="a lurch must also be this big (default 3)")
    ap.add_argument("--only-png", action="append", default=[], help="keep only sprites whose png matches this regex (repeatable)")
    ap.add_argument("--exclude-png", action="append", default=[], help="drop sprites whose png matches this regex (repeatable)")
    ap.add_argument("--anchor", choices=("center", "origin"), default="center",
                    help="sprite position: box centroid (comparable with a capture) or top-left origin (cleaner for the port)")
    ap.add_argument("--settle", type=float, default=1000.0, help="ms of module start-up to ignore for open-water births (default 1000)")
    ap.add_argument("--label", default=None, help="name for the report (default: file stem)")
    ap.add_argument("--json", dest="json_out", default=None, help="write the JSON summary here ('-' for stdout)")
    ap.add_argument("--quiet", action="store_true", help="suppress the human table")
    ap.add_argument("--fail-on", default=None, help="e.g. jerk=0,teleports=0,backwards=0.02,stuck=0")
    a = ap.parse_args(argv)

    frames, fires, dropped = read_trace(
        a.trace, anchor=a.anchor, only=a.only_png, exclude=a.exclude_png
    )
    if not frames:
        raise SystemExit(f"{a.trace}: no trace lines")
    label = a.label or os.path.splitext(os.path.basename(a.trace))[0]
    summary = analyse(
        frames,
        fires,
        label,
        teleport=a.teleport,
        gap=a.gap,
        stuck_s=a.stuck,
        jerk_factor=a.jerk_factor,
        jerk_hold=a.jerk_hold,

        jerk_min_px=a.jerk_min_px,
        match_max=a.match_max,
        anchor=a.anchor,
        settle_ms=a.settle,
    )
    summary["filtered_sprites"] = dropped
    if not a.quiet:
        table(summary)
    if a.json_out:
        text = json.dumps(summary, indent=2)
        if a.json_out == "-":
            print(text)
        else:
            with open(a.json_out, "w") as f:
                f.write(text + "\n")
    if a.fail_on:
        if check_thresholds(summary, parse_fail_on(a.fail_on)):
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
