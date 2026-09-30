#!/usr/bin/env python3
"""Tests for motion_lint's detectors, on synthetic traces.

Synthetic because the point is to pin down what each detector CLAIMS to
see: a smooth walker must score zero on everything (no crying wolf at a
lane that actually fixed its module), and each defect must be found where
it was planted. Run with `python3 tools/test_motion_lint.py`.
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import motion_lint as ml  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))


def line(t, sprites, sounds=()):
    return {"t": t, "sprites": list(sprites), "sounds": list(sounds)}


def sp(png, x, y, flip=False, pal=0):
    return {"png": png, "x": x, "y": y, "flip": flip, "pal": pal}


def walker(png, x0, y, dx, n, t0=0, dt=50, flip=None):
    """A perfectly smooth actor: one sprite per frame, `dx` px each time."""
    out = []
    for i in range(n):
        f = (dx < 0) if flip is None else flip
        out.append((t0 + i * dt, sp(png, x0 + i * dx, y, flip=f)))
    return out


def merge(*streams):
    """Interleave per-actor streams into frames keyed by timestamp."""
    frames = {}
    for st in streams:
        for t, s in st:
            frames.setdefault(t, []).append(s)
    return [line(t, frames[t]) for t in sorted(frames)]


def write(recs):
    fd, path = tempfile.mkstemp(suffix=".jsonl")
    with os.fdopen(fd, "w") as f:
        for r in recs:
            f.write(json.dumps(r) + "\n")
    return path


def summarise(recs, **kw):
    path = write(recs)
    anchor = kw.get("anchor", "center")
    try:
        frames, fires, _ = ml.read_trace(path, anchor=anchor)
        return ml.analyse(frames, fires, "test", **kw)
    finally:
        os.unlink(path)


CLS = "compounds/2000"
A = f"{CLS}/a.png"
B = f"{CLS}/b.png"


class SmoothIsClean(unittest.TestCase):
    """The false-positive guard: a well-behaved actor scores zero."""

    def test_smooth_walker_has_no_defects(self):
        s = summarise(merge(walker(A, 10, 100, 4, 120)))
        c = s["classes"][CLS]
        self.assertEqual(c["tracks"], 1)
        self.assertEqual(c["step"]["median"], 4.0)
        self.assertEqual(c["step"]["max"], 4.0)
        self.assertEqual(c["jerk"]["count"], 0)
        self.assertEqual(c["teleports"]["count"], 0)
        self.assertEqual(c["stuck"]["count"], 0)
        self.assertEqual(c["reversals"]["fraction"], 0.0)
        self.assertEqual(c["cadence"]["hold_median"], 1.0)
        self.assertEqual(c["cadence"]["advance_median"], 4.0)

    def test_oversampling_is_not_jerk(self):
        """Sampling a 100 ms animation every 50 ms shows every advance as
        'held 2 frames then stepped'. That is the sample rate, not a
        defect, and the cadence-relative test must not report it."""
        stream = []
        x = 10
        for i in range(200):
            stream.append((i * 50, sp(A, x, 100)))
            if i % 2 == 1:
                x += 6
        s = summarise([line(t, [spr]) for t, spr in stream])
        c = s["classes"][CLS]
        self.assertEqual(c["cadence"]["hold_median"], 2.0)
        self.assertEqual(c["jerk"]["count"], 0, c["jerk"]["worst"])


class JerkDetection(unittest.TestCase):
    def test_freeze_then_lurch_is_caught(self):
        """The toxic f537105 signature: the same art at the same place for
        several frames, then one catch-up hop."""
        stream, x, t = [], 10, 0
        planted = []
        for i in range(120):
            stream.append((t, sp(A if i % 2 else B, x, 100)))
            t += 50
            if i % 20 == 19:  # freeze for 3 extra frames, then catch up
                for _ in range(3):
                    stream.append((t, sp(A if i % 2 else B, x, 100)))
                    t += 50
                x += 24
                planted.append(t)
            else:
                x += 4
        s = summarise([line(t, [spr]) for t, spr in stream])
        c = s["classes"][CLS]
        self.assertGreaterEqual(c["jerk"]["count"], 5, c["jerk"])
        worst = c["jerk"]["worst"][0]
        self.assertGreaterEqual(worst["step"], 20)
        self.assertGreaterEqual(worst["held_frames"], 4)
        # and it points at real timestamps, not invented ones
        ts = {j["t"] for j in c["jerk"]["worst"]}
        self.assertTrue(ts <= set(planted), f"{ts} not among {planted[:5]}…")

    def test_a_long_hold_alone_is_not_jerk(self):
        """An actor that stops and then resumes at its normal pace is
        idle, not jerky — otherwise every pause in every module scores."""
        stream, x, t = [], 10, 0
        for i in range(60):
            stream.append((t, sp(A, x, 100)))
            t, x = t + 50, x + 4
        for _ in range(40):  # two seconds of standing still
            stream.append((t, sp(A, x, 100)))
            t += 50
        for i in range(60):
            x += 4
            stream.append((t, sp(A, x, 100)))
            t += 50
        s = summarise([line(t, [spr]) for t, spr in stream])
        self.assertEqual(s["classes"][CLS]["jerk"]["count"], 0)


class TeleportDetection(unittest.TestCase):
    def test_a_big_step_is_reported_and_breaks_the_track(self):
        stream = walker(A, 10, 100, 4, 60)
        jump_t = stream[-1][0] + 50
        stream.append((jump_t, sp(A, 400, 100)))
        stream += [(jump_t + 50 * (i + 1), sp(A, 404 + 4 * i, 100)) for i in range(60)]
        s = summarise([line(t, [spr]) for t, spr in stream])
        c = s["classes"][CLS]
        self.assertEqual(c["teleports"]["count"], 1)
        e = c["teleports"]["worst"][0]
        self.assertEqual(e["t"], jump_t)
        self.assertAlmostEqual(e["d"], 154.0, places=1)
        self.assertEqual(c["tracks"], 2, "a teleport breaks the track in two")

    def test_below_threshold_is_not_a_teleport(self):
        stream = walker(A, 10, 100, 4, 30)
        t = stream[-1][0] + 50
        stream.append((t, sp(A, stream[-1][1]["x"] + 50, 100)))
        s = summarise([line(t, [spr]) for t, spr in stream])
        self.assertEqual(s["classes"][CLS]["teleports"]["count"], 0)
        s60 = summarise([line(t, [spr]) for t, spr in stream], teleport=40)
        self.assertEqual(s60["classes"][CLS]["teleports"]["count"], 1)


class BackwardsDetection(unittest.TestCase):
    def test_one_moonwalker_among_three_walkers(self):
        """Three actors flip to face where they are going; one does not.
        The minority is what shows up — roughly its share of the moving
        frames."""
        good = [
            walker(A, 10, 100, 4, 100, flip=False),   # right, unflipped
            walker(A, 500, 200, -4, 100, flip=True),  # left, flipped
            walker(A, 10, 300, 4, 100, flip=False),
        ]
        bad = walker(A, 500, 400, -4, 100, flip=False)  # left, NOT flipped
        s = summarise(merge(*good, bad))
        b = s["classes"][CLS]["backwards"]
        self.assertEqual(b["best"], "A")
        self.assertAlmostEqual(b["fraction"], 0.25, places=2)

    def test_consistent_flipping_scores_zero(self):
        s = summarise(
            merge(
                walker(A, 10, 100, 4, 100, flip=False),
                walker(A, 500, 200, -4, 100, flip=True),
            )
        )
        self.assertEqual(s["classes"][CLS]["backwards"]["fraction"], 0.0)

    def test_a_module_that_never_flips_reports_nothing(self):
        """Both hypotheses are mirrors, so with a constant flip flag the
        number would always be <= 50% and mean nothing. Say n/a instead."""
        s = summarise(
            merge(
                walker(A, 10, 100, 4, 100, flip=False),
                walker(A, 500, 200, -4, 100, flip=False),
            )
        )
        b = s["classes"][CLS]["backwards"]
        self.assertIsNone(b["fraction"])
        self.assertEqual(b["flip_varies"], 0.0)

    def test_reversals_still_work_without_flip_data(self):
        """The orientation-free check: an actor shuttling back and forth
        every advance scores ~100%, a smooth one ~0%."""
        shuttle = [(i * 50, sp(A, 100 + (4 if i % 2 else 0), 100)) for i in range(100)]
        s = summarise([line(t, [spr]) for t, spr in shuttle])
        self.assertGreater(s["classes"][CLS]["reversals"]["fraction"], 0.9)
        s2 = summarise(merge(walker(A, 10, 100, 4, 100)))
        self.assertEqual(s2["classes"][CLS]["reversals"]["fraction"], 0.0)


class StuckDetection(unittest.TestCase):
    def test_a_frozen_actor_beside_a_moving_sibling(self):
        frozen = [(i * 50, sp(B, 300, 300)) for i in range(400)]  # 20 s
        s = summarise(merge(walker(A, 10, 100, 4, 400), frozen))
        c = s["classes"][CLS]
        self.assertEqual(c["stuck"]["count"], 1)
        k = c["stuck"]["worst"][0]
        self.assertEqual(k["at"], [300.0, 300.0])
        self.assertGreaterEqual(k["ms"], 10_000)

    def test_everything_frozen_is_not_reported_as_stuck(self):
        """If the whole class is still, that is a module-wide question,
        not a stuck actor — and flagging it would drown the signal."""
        a = [(i * 50, sp(A, 100, 100)) for i in range(400)]
        b = [(i * 50, sp(B, 300, 300)) for i in range(400)]
        s = summarise(merge(a, b))
        self.assertEqual(s["classes"][CLS]["stuck"]["count"], 0)

    def test_a_short_freeze_is_not_stuck(self):
        frozen = [(i * 50, sp(B, 300, 300)) for i in range(100)]  # 5 s < 10 s
        s = summarise(merge(walker(A, 10, 100, 4, 100), frozen))
        self.assertEqual(s["classes"][CLS]["stuck"]["count"], 0)


class ClassesAndStitching(unittest.TestCase):
    def test_different_series_never_stitch_together(self):
        other = "compounds/1000/x.png"
        s = summarise(
            merge(walker(A, 10, 100, 4, 50), walker(other, 12, 102, 4, 50))
        )
        self.assertEqual(sorted(s["classes"]), ["compounds/1000", CLS])
        for c in s["classes"].values():
            self.assertEqual(c["tracks"], 1)

    def test_two_actors_stay_two_tracks(self):
        s = summarise(merge(walker(A, 10, 100, 4, 100), walker(A, 10, 400, 4, 100)))
        self.assertEqual(s["classes"][CLS]["tracks"], 2)

    def test_disappearance_breaks_a_track_and_gap_forgives_it(self):
        st = walker(A, 10, 100, 4, 40)
        recs = [line(t, [spr]) for t, spr in st]
        recs.insert(20, line(recs[19]["t"] + 25, []))  # one empty frame
        self.assertEqual(summarise(recs)["classes"][CLS]["tracks"], 2)
        self.assertEqual(summarise(recs, gap=1)["classes"][CLS]["tracks"], 1)


class Filters(unittest.TestCase):
    def test_exclude_png_drops_the_scenery_and_leaves_the_cast(self):
        """The boris case: 2028 mown-trail tiles in the same series as the
        mower and the cats. Without a filter every statistic is a
        statistic about grass."""
        trail = [(i * 50, sp(f"{CLS}/c_005.png", 20 * k, 400)) for i in range(60) for k in range(30)]
        recs = merge(walker(A, 10, 100, 4, 60), trail)
        path = write(recs)
        try:
            frames, _, dropped = ml.read_trace(path, exclude=[r"c_005[.]png$"])
            self.assertEqual(dropped, 60 * 30)
            s = ml.analyse(frames, [], "t")
            self.assertEqual(s["classes"][CLS]["tracks"], 1)
            frames2, _, d2 = ml.read_trace(path, only=[r"c_005[.]png$"])
            self.assertEqual(d2, 60)
            self.assertEqual(ml.analyse(frames2, [], "t")["classes"][CLS]["tracks"], 30)
        finally:
            os.unlink(path)

    def test_a_mostly_static_class_is_scenery_not_two_thousand_stuck_actors(self):
        trail = [(i * 50, sp(B, 20 * k, 400)) for i in range(400) for k in range(30)]
        s = summarise(merge(walker(A, 10, 100, 4, 400), trail))
        c = s["classes"][CLS]
        self.assertTrue(c["stuck"]["scenery"])
        self.assertEqual(c["stuck"]["count"], 0)
        self.assertEqual(c["stuck"]["static_tracks"], 30)


class OpenWater(unittest.TestCase):
    """Toxic-swamp's errata: critters enter across an edge and never
    materialise in open water. A sprite that appears with its whole box
    on screen is therefore a defect the golden capture already ruled
    out."""

    def sized(self, png, x, y, w=40, h=20):
        d = sp(png, x, y)
        d["w"], d["h"] = w, h
        return d

    def test_an_entry_across_the_edge_is_not_open_water(self):
        recs = [line(i * 50, [self.sized(A, -30 + 4 * i, 100)]) for i in range(60)]
        recs = [line(0, [])] + recs  # so the entering track is born mid-trace
        s = summarise(recs, anchor="origin")
        self.assertEqual(s["classes"][CLS]["churn"]["open_water"], 0)

    def test_a_sprite_that_appears_mid_screen_is_flagged(self):
        recs = [line(i * 50, []) for i in range(40)]
        recs += [line((40 + i) * 50, [self.sized(A, 300, 200)]) for i in range(30)]
        s = summarise(recs, anchor="origin")
        c = s["classes"][CLS]["churn"]
        self.assertEqual(c["open_water"], 1)
        b = c["open_water_worst"][0]
        self.assertEqual(b["at"], [300.0, 200.0])
        self.assertEqual(b["size"], [40, 20])

    def test_the_opening_cast_is_not_an_entry(self):
        """Every module places its initial actors in the first tick or
        two; those placements are not entries across an edge."""
        recs = [line(0, [])]
        recs += [line((1 + i) * 50, [self.sized(A, 300, 200)]) for i in range(40)]
        s = summarise(recs, anchor="origin")
        self.assertEqual(s["classes"][CLS]["churn"]["open_water"], 0)
        late = summarise(recs, anchor="origin", settle_ms=0)
        self.assertEqual(late["classes"][CLS]["churn"]["open_water"], 1)

    def test_a_track_present_from_the_first_frame_is_not_a_birth(self):
        recs = [line(i * 50, [self.sized(A, 300, 200)]) for i in range(30)]
        self.assertEqual(
            summarise(recs, anchor="origin")["classes"][CLS]["churn"]["open_water"], 0
        )


class Sounds(unittest.TestCase):
    def test_counts_gaps_and_bursts(self):
        recs = [line(i * 50, [sp(A, 10 + i, 100)]) for i in range(100)]
        recs[0]["sounds"] = [9000]
        recs[10]["sounds"] = [9000]
        recs[12]["sounds"] = [9000, 9001, 9001]
        s = summarise(recs)
        a = s["sounds"]["9000"]
        self.assertEqual(a["count"], 3)
        self.assertEqual(a["min_gap_ms"], 100)  # frames 10 -> 12
        self.assertEqual(a["first_ms"], 0)
        b = s["sounds"]["9001"]
        self.assertEqual(b["count"], 2)
        self.assertIsNone(b["min_gap_ms"], "two fires in one sample is not a gap")
        self.assertEqual(b["burst_max"], 2)


class Thresholds(unittest.TestCase):
    def test_fail_on_parsing_and_check(self):
        self.assertEqual(ml.parse_fail_on("jerk=0,teleports=2"), {"jerk": 0.0, "teleports": 2.0})
        with self.assertRaises(SystemExit):
            ml.parse_fail_on("wobble=1")
        summary = {"totals": {"jerk": 3, "teleports": 0, "stuck": 0, "backwards": 0.1, "reversals": 0.0}}
        self.assertTrue(ml.check_thresholds(summary, {"jerk": 0}, out=open(os.devnull, "w")))
        self.assertFalse(ml.check_thresholds(summary, {"jerk": 3}, out=open(os.devnull, "w")))

    def test_cli_exit_code_is_the_ratchet(self):
        """The whole point of --fail-on: a lane can wire it into CI."""
        recs = [line(t, [spr]) for t, spr in walker(A, 10, 100, 4, 60)]
        path = write(recs)
        try:
            ok = subprocess.run(
                [sys.executable, os.path.join(HERE, "motion_lint.py"), path,
                 "--quiet", "--fail-on", "jerk=0,teleports=0"],
                capture_output=True, text=True,
            )
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
            recs.append(line(recs[-1]["t"] + 50, [sp(A, 400, 100)]))
            path2 = write(recs)
            bad = subprocess.run(
                [sys.executable, os.path.join(HERE, "motion_lint.py"), path2,
                 "--quiet", "--fail-on", "teleports=0"],
                capture_output=True, text=True,
            )
            self.assertEqual(bad.returncode, 1, bad.stdout + bad.stderr)
            self.assertIn("FAIL teleports", bad.stdout)
            os.unlink(path2)
        finally:
            os.unlink(path)

    def test_human_table_and_json_render(self):
        import io

        s = summarise(merge(walker(A, 10, 100, 4, 60)))
        buf = io.StringIO()
        ml.table(s, out=buf)
        text = buf.getvalue()
        self.assertIn(CLS, text)
        self.assertIn("TOTAL", text)
        json.dumps(s)  # the summary must be serialisable, all of it


if __name__ == "__main__":
    unittest.main(verbosity=2)
