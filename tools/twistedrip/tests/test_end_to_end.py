"""End-to-end gate test: tools/rip.py, run on the real floppy release, for
one small module (bungee-roulette -- the smallest full pack, ~1 s to rip),
compared byte-for-byte against a freshly built tools/pack_assets.py
reference via the same parity.py the CLI gate uses.

This is the one test that exercises ingest -> rsrc.parse -> pack.build()
together through the actual CLI entry point, rather than each lane's
per-file oracle. It skips (never fails) when the Macintosh Garden originals
or the private RE checkout aren't present on this machine -- see
conftest.py's _require_private_checkout; neither is repo content."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

# tools/rip.py lives directly under tools/, one level above this package.
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import rip  # noqa: E402

from .. import parity  # noqa: E402
from .conftest import RE_ROOT  # noqa: E402

FLOPPY_SIT = RE_ROOT / "original" / "After_Dark_-_Totally_Twisted.sit"

pytestmark = pytest.mark.skipif(
    not FLOPPY_SIT.exists(), reason="floppy original .sit not present on this machine")


def test_rip_py_matches_pack_assets_for_bungee_roulette(ref_pack_dir, tmp_path):
    ref = ref_pack_dir("bungee-roulette")

    out_root = tmp_path / "ripped"
    rc = rip.rip(FLOPPY_SIT, out_root, only="bungee-roulette")
    assert rc == 0

    got = out_root / "bungee-roulette"
    assert got.is_dir(), "rip.py produced no bungee-roulette pack"

    tally = parity.compare_module(ref, got)
    if tally.messages:
        print("\n" + "\n".join(tally.messages))
    assert tally.fail == 0, f"{tally.fail} parity failure(s) -- see messages above"
    assert tally.warn == 0, f"{tally.warn} parity warning(s) -- see messages above"
    assert tally.ok > 0
