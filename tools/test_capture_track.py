#!/usr/bin/env python3
"""Tests for capture_track's blob finder, on synthetic frames.

The tracker and every statistic are motion_lint's and are tested there;
what is unique here is turning pixels into observations. Run with
`python3 tools/test_capture_track.py`.
"""

import os
import sys
import unittest

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import capture_track as ct  # noqa: E402


def blank(h=64, w=96):
    return np.zeros((h, w), dtype=bool)


class Morphology(unittest.TestCase):
    def test_dilate_grows_and_erode_shrinks(self):
        m = blank()
        m[10, 10] = True
        self.assertEqual(ct.dilate(m, 1).sum(), 9)  # 3x3, 8-connected
        self.assertEqual(ct.erode(ct.dilate(m, 1), 1).sum(), 1)

    def test_erode_removes_isolated_noise(self):
        m = blank()
        m[5, 5] = True  # a lone MPEG speckle
        m[20:30, 20:30] = True  # a real sprite
        e = ct.erode(m, 1)
        self.assertFalse(e[5, 5])
        self.assertTrue(e[25, 25])


class Blobs(unittest.TestCase):
    def test_two_rectangles_are_two_blobs_with_true_centroids(self):
        m = blank()
        m[10:20, 10:20] = True   # centre (14.5, 14.5)
        m[40:50, 60:70] = True   # centre (64.5, 44.5)
        bs = ct.blobs(m, min_area=10)
        self.assertEqual(len(bs), 2)
        got = sorted((round(b["x"], 1), round(b["y"], 1)) for b in bs)
        self.assertEqual(got, [(14.5, 14.5), (64.5, 44.5)])
        self.assertEqual([b["area"] for b in bs], [100, 100])

    def test_diagonal_touch_is_one_blob(self):
        m = blank()
        m[10:15, 10:15] = True
        m[15:20, 15:20] = True  # touches only at the corner
        self.assertEqual(len(ct.blobs(m, min_area=10)), 1)

    def test_min_area_drops_specks(self):
        m = blank()
        m[10:20, 10:20] = True
        m[40, 40] = True
        self.assertEqual(len(ct.blobs(m, min_area=10)), 1)

    def test_dilation_glues_fragments_but_the_centroid_stays_on_the_art(self):
        """The reason `blobs` takes a separate weight mask: one actor drawn
        as head + body + feet has to come back as ONE observation, and the
        halo the dilation adds must not drag that observation around."""
        core = blank()
        core[10:14, 30:34] = True   # head
        core[18:22, 30:34] = True   # body, a 4 px gap below the head
        grown = ct.dilate(core, 3)
        self.assertEqual(len(ct.blobs(grown, min_area=10)), 1, "the gap must close")
        b = ct.blobs(grown, min_area=10, weight=core)[0]
        self.assertAlmostEqual(b["x"], 31.5, places=1)
        self.assertAlmostEqual(b["y"], 15.5, places=1)  # midway between the two
        self.assertEqual(b["area"], 32, "area counts drawn pixels, not the halo")

    def test_a_row_touching_the_frame_edge_is_still_closed(self):
        """Run extraction has to handle a run that starts at column 0 or
        ends at the last column — an actor half off-screen is normal."""
        m = blank()
        m[10:20, 0:6] = True
        m[30:40, -6:] = True
        bs = ct.blobs(m, min_area=10)
        self.assertEqual(len(bs), 2)
        self.assertEqual(sum(b["area"] for b in bs), 120)

    def test_empty_frame_yields_nothing(self):
        self.assertEqual(ct.blobs(blank(), min_area=1), [])


class Foreground(unittest.TestCase):
    def test_field_color_is_the_dominant_pixel(self):
        bg = np.zeros((40, 40, 3), dtype=np.int16)
        bg[:, :] = [0, 0, 17]      # the field
        bg[0:5, 0:5] = [200, 0, 0]  # some scenery
        self.assertEqual(list(ct.field_color(bg)), [0, 0, 17])


class Masks(unittest.TestCase):
    def test_rect_parsing(self):
        self.assertEqual(ct.parse_rect("1,2,3,4"), (1, 2, 3, 4))
        with self.assertRaises(SystemExit):
            ct.parse_rect("1,2,3")

    def test_the_panel_rect_covers_the_qemu_control_panel(self):
        """Measured off the captures: the white box is x 340..467,
        y 148..329 in every 640x480 QEMU clip."""
        x0, y0, x1, y1 = ct.PANEL_RECT
        self.assertLessEqual(x0, 340)
        self.assertLessEqual(y0, 148)
        self.assertGreaterEqual(x1, 468)
        self.assertGreaterEqual(y1, 330)


if __name__ == "__main__":
    unittest.main(verbosity=2)
