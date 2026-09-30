#!/usr/bin/env python3
"""capture_track — the same motion table, measured off a QEMU capture.

The golden captures under `totally-twisted/emu/captures/qemu/*.mp4` are
the only adjudicator retwisted has: they are the original doing the thing.
This pulls frames out of one with ffmpeg, finds the moving sprites as
blobs against a median background, stitches them with the SAME tracker
`tools/motion_lint.py` uses on a port trace, and prints the SAME table —
so "the port's fish advance 3 px per step, the original's advance 7"
becomes a thing you can read off two adjacent rows instead of a thing a
lane asserts.

Usage
  tools/capture_track.py emu/captures/qemu/toxic-swamp.mp4
  tools/capture_track.py cap.mp4 --fps 20 --json cap.json
  tools/capture_track.py cap.mp4 --mask 0,0,200,40     # extra dead zone

The in-module control panel (the white box QEMU leaves mid-screen, and
the mouse cursor parked on its Stop button) is masked out by default:
it is not module motion, and its Stop-button hover would otherwise track
as a very stuck actor. `--no-panel-mask` turns that off.

Pure PIL + numpy — no scipy, no OpenCV.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import motion_lint as ml  # noqa: E402

# The After Dark control panel as QEMU parks it in a 640x480 capture,
# measured off toxic-swamp.mp4 and mime-hunt.mp4 (identical in both):
# the white box runs x 340..467, y 148..329. A few px of margin covers
# the drop shadow and the cursor arrow that sits on the Stop button.
PANEL_RECT = (336, 144, 472, 334)


def extract_frames(path, fps, outdir, start=None, duration=None):
    """ffmpeg -> outdir/f%05d.png at `fps`. Returns the file list."""
    cmd = ["ffmpeg", "-v", "error", "-y"]
    if start is not None:
        cmd += ["-ss", str(start)]
    cmd += ["-i", path]
    if duration is not None:
        cmd += ["-t", str(duration)]
    cmd += ["-vf", f"fps={fps}", os.path.join(outdir, "f%05d.png")]
    subprocess.run(cmd, check=True)
    return sorted(
        os.path.join(outdir, f) for f in os.listdir(outdir) if f.endswith(".png")
    )


def background(files, samples=48):
    """Per-pixel median over frames spread across the whole clip.

    Median rather than "the first frame" because these modules draw
    permanent scenery (toxic-swamp's swamp floor, its bound man) that no
    single frame separates from the moving cast — and because a median
    survives a sprite parked over one spot for a few seconds.
    """
    idx = np.linspace(0, len(files) - 1, min(samples, len(files))).astype(int)
    stack = np.stack([np.asarray(Image.open(files[i]).convert("RGB")) for i in idx])
    return np.median(stack, axis=0).astype(np.int16)


def field_color(bg):
    """The module's field (background) colour: the most common pixel of the
    background model. `compose` fills the screen with it every frame, so
    "differs from the field" is "something was drawn here" — including art
    that never moves, which background subtraction by definition cannot
    see."""
    flat = bg.reshape(-1, 3).astype(np.int32)
    key = (flat[:, 0] << 16) | (flat[:, 1] << 8) | flat[:, 2]
    vals, counts = np.unique(key, return_counts=True)
    k = int(vals[counts.argmax()])
    return np.array([(k >> 16) & 255, (k >> 8) & 255, k & 255], dtype=np.int16)


def dilate(mask, iters=1):
    """3x3 binary dilation by shifted OR — the whole of the morphology we
    need, and the reason this file needs no scipy. Sprites in these
    modules are drawn with transparent interiors and 1 px outlines, so
    without a dilate-to-close pass one fish labels as eight blobs."""
    for _ in range(iters):
        m = mask
        out = m.copy()
        out[1:, :] |= m[:-1, :]
        out[:-1, :] |= m[1:, :]
        out[:, 1:] |= m[:, :-1]
        out[:, :-1] |= m[:, 1:]
        out[1:, 1:] |= m[:-1, :-1]
        out[:-1, :-1] |= m[1:, 1:]
        out[1:, :-1] |= m[:-1, 1:]
        out[:-1, 1:] |= m[1:, :-1]
        mask = out
    return mask


def erode(mask, iters=1):
    """The dual of `dilate` — used first, to drop the single-pixel dither
    noise MPEG leaves around a hard-edged sprite before it gets grown into
    a blob of its own."""
    for _ in range(iters):
        m = mask
        out = m.copy()
        out[1:, :] &= m[:-1, :]
        out[:-1, :] &= m[1:, :]
        out[:, 1:] &= m[:, :-1]
        out[:, :-1] &= m[:, 1:]
        mask = out
    return mask


def blobs(mask, min_area=40, weight=None):
    """Connected components by run-length union-find, 8-connected.

    Row runs rather than per-pixel flood fill: a 640x480 frame has a few
    hundred runs and 300k pixels, and this has to survive 1200 frames of
    a capture in pure Python.

    `mask` says what is connected to what; `weight` (default: `mask`) says
    which pixels the centroid is averaged over. Splitting the two is what
    lets the dilation be aggressive enough to glue one mime's head, body
    and feet into a single blob without the grown halo dragging the
    centroid around — the centroid stays on the art the module drew.

    Returns [{"x","y","area","bbox"}] with (x, y) the component centroid.
    """
    if weight is None:
        weight = mask
    h, w = mask.shape
    parent = []

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def union(a, b):
        ra, rb = find(a), find(b)
        if ra != rb:
            parent[max(ra, rb)] = min(ra, rb)

    runs = []  # (y, s, e, label)
    prev = []
    for y in range(h):
        row = mask[y]
        if not row.any():
            prev = []
            continue
        d = np.diff(row.astype(np.int8))
        starts = (np.where(d == 1)[0] + 1).tolist()
        ends = (np.where(d == -1)[0] + 1).tolist()
        if row[0]:
            starts.insert(0, 0)
        if row[-1]:
            ends.append(w)
        cur = []
        for s, e in zip(starts, ends):
            lbl = len(parent)
            parent.append(lbl)
            for ps, pe, pl in prev:
                if ps < e + 1 and s - 1 < pe:  # 8-connected: touch diagonally
                    union(lbl, pl)
            cur.append((s, e, lbl))
            runs.append((y, s, e, lbl))
        prev = cur

    xs = np.arange(w)
    acc = {}
    for y, s, e, lbl in runs:
        r = find(lbl)
        wr = weight[y, s:e]
        n = int(wr.sum())
        if n == 0:
            continue
        sx = float((xs[s:e] * wr).sum())
        a = acc.get(r)
        if a is None:
            acc[r] = [n, sx, y * n, s, y, e, y + 1]
        else:
            a[0] += n
            a[1] += sx
            a[2] += y * n
            a[3] = min(a[3], s)
            a[4] = min(a[4], y)
            a[5] = max(a[5], e)
            a[6] = max(a[6], y + 1)
    out = []
    for n, sx, sy, x0, y0, x1, y1 in acc.values():
        if n < min_area:
            continue
        out.append(
            {"x": sx / n, "y": sy / n, "area": int(n), "bbox": [x0, y0, x1, y1]}
        )
    out.sort(key=lambda b: -b["area"])
    return out


def frames_from_capture(
    files,
    fps,
    masks,
    thresh=40,
    min_area=40,
    erode_iters=1,
    dilate_iters=4,
    max_blobs=24,
    class_by="none",
    fg="diff",
    progress=False,
):
    """-> ml-shaped frames: [(t_ms, [obs, ...]), ...].

    `fg` picks what counts as foreground:
      * `diff`  — differs from the median background. Sees MOVING art
        only: an actor that holds still for most of the clip is absorbed
        into the background model and vanishes from the table. That is
        the right default for a motion gate, and the reason capture-side
        track counts run below the number of actors on screen.
      * `field` — differs from the module's field colour, i.e. everything
        the module drew, static scenery included. Use it when the
        question is "how many actors are there", not "what is moving";
        expect permanent scenery to show up as motionless tracks.

    `class_by="area"` buckets blobs into power-of-two size classes. It is
    off by default because a blob whose area breathes across a bucket
    boundary changes class and breaks its own track — the class hint the
    port trace gets free from the compound directory has no honest
    equivalent here, and a bad hint is worse than none.
    """
    bg = background(files)
    ref = bg if fg == "diff" else field_color(bg).reshape(1, 1, 3)
    out = []
    for i, f in enumerate(files):
        img = np.asarray(Image.open(f).convert("RGB")).astype(np.int16)
        m = (np.abs(img - ref).max(axis=2) > thresh)
        for (x0, y0, x1, y1) in masks:
            m[max(0, y0):max(0, y1), max(0, x0):max(0, x1)] = False
        core = erode(m, erode_iters)
        bs = blobs(dilate(core, dilate_iters), min_area=min_area, weight=core)[:max_blobs]
        t = int(round(i * 1000.0 / fps))
        obs = []
        for b in bs:
            if class_by == "area":
                cls = f"area~{1 << max(0, int(b['area']).bit_length() - 1)}"
            else:
                cls = "blob"
            obs.append(ml.obs(t, b["x"], b["y"], cls, area=b["area"]))
        out.append((t, obs))
        if progress and i % 100 == 0:
            print(f"  ... {i}/{len(files)} frames", file=sys.stderr)
    return out


def parse_rect(spec):
    parts = spec.split(",")
    if len(parts) != 4:
        raise SystemExit(f"--mask wants x0,y0,x1,y1 — got {spec!r}")
    return tuple(int(p) for p in parts)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("video")
    ap.add_argument("--fps", type=float, default=10.0, help="sample rate (default 10)")
    ap.add_argument("--ss", type=float, default=None, help="start seconds into the clip")
    ap.add_argument("--duration", type=float, default=None, help="seconds to read")
    ap.add_argument("--mask", action="append", default=[], help="x0,y0,x1,y1 dead zone (repeatable)")
    ap.add_argument("--no-panel-mask", action="store_true", help="keep the control panel in frame")
    ap.add_argument("--thresh", type=int, default=40, help="per-channel diff from background (default 40)")
    ap.add_argument("--min-area", type=int, default=60, help="smallest blob in px (default 60)")
    ap.add_argument("--erode", type=int, default=1, help="erode passes before dilate (default 1)")
    ap.add_argument("--dilate", type=int, default=4, help="dilate passes, to glue one actor's fragments (default 4)")
    ap.add_argument("--max-blobs", type=int, default=24, help="largest N blobs per frame (default 24)")
    ap.add_argument("--class-by", choices=("area", "none"), default="none",
                    help="stitching class hint: power-of-two blob area, or one class for everything (default)")
    ap.add_argument("--fg", choices=("diff", "field"), default="diff",
                    help="foreground: moving art only (diff, default) or everything drawn (field)")
    ap.add_argument("--teleport", type=float, default=60.0)
    ap.add_argument("--match-max", type=float, default=None)
    ap.add_argument("--gap", type=int, default=0)
    ap.add_argument("--stuck", type=float, default=10.0)
    ap.add_argument("--jerk-factor", type=float, default=2.0)
    ap.add_argument("--jerk-hold", type=int, default=2)
    ap.add_argument("--jerk-min-px", type=float, default=2.0)
    ap.add_argument("--label", default=None)
    ap.add_argument("--json", dest="json_out", default=None)
    ap.add_argument("--quiet", action="store_true")
    ap.add_argument("--keep-frames", default=None, help="keep the extracted PNGs here")
    ap.add_argument("--fail-on", default=None)
    a = ap.parse_args(argv)

    if not shutil.which("ffmpeg"):
        raise SystemExit("ffmpeg not on PATH")

    masks = [parse_rect(s) for s in a.mask]
    if not a.no_panel_mask:
        masks.append(PANEL_RECT)

    tmp = a.keep_frames or tempfile.mkdtemp(prefix="captrack-")
    os.makedirs(tmp, exist_ok=True)
    try:
        files = extract_frames(a.video, a.fps, tmp, a.ss, a.duration)
        if not files:
            raise SystemExit(f"{a.video}: ffmpeg produced no frames")
        frames = frames_from_capture(
            files,
            a.fps,
            masks,
            thresh=a.thresh,
            min_area=a.min_area,
            erode_iters=a.erode,
            dilate_iters=a.dilate,
            max_blobs=a.max_blobs,
            class_by=a.class_by,
            fg=a.fg,
            progress=not a.quiet,
        )
    finally:
        if not a.keep_frames:
            shutil.rmtree(tmp, ignore_errors=True)

    label = a.label or os.path.splitext(os.path.basename(a.video))[0] + " (capture)"
    summary = ml.analyse(
        frames,
        [],  # a capture carries no snd ids; the .wav beside it is another lane
        label,
        teleport=a.teleport,
        gap=a.gap,
        stuck_s=a.stuck,
        jerk_factor=a.jerk_factor,
        jerk_hold=a.jerk_hold,
        jerk_min_px=a.jerk_min_px,
        match_max=a.match_max,
        anchor="center",
    )
    summary["capture"] = {
        "video": a.video,
        "fps": a.fps,
        "frames": len(frames),
        "masks": [list(m) for m in masks],
        "thresh": a.thresh,
        "min_area": a.min_area,
        "fg": a.fg,
        "erode": a.erode,
        "dilate": a.dilate,
        "class_by": a.class_by,
    }
    if not a.quiet:
        ml.table(summary)
    if a.json_out:
        text = json.dumps(summary, indent=2)
        if a.json_out == "-":
            print(text)
        else:
            with open(a.json_out, "w") as f:
                f.write(text + "\n")
    if a.fail_on:
        if ml.check_thresholds(summary, ml.parse_fail_on(a.fail_on)):
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
