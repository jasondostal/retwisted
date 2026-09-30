"""Tests for ingest.py against the real Macintosh Garden originals.

These exercise all four documented input kinds (floppy .sit, hybrid-CD
.sit and raw .iso, .sit.hqx, and a plain folder) plus the AppleDouble
fallback path. They read from the locally-sourced originals under
~/working/totally-twisted -- if that tree isn't present (a machine other
than the one this lane was built on), the tests skip rather than fail,
since those files are Macintosh Garden downloads, not repo content.
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from twistedrip.ingest import ingest  # noqa: E402

TT_ROOT = Path.home() / "working" / "totally-twisted"
ORIGINALS = TT_ROOT / "original"
EXTRACTED = TT_ROOT / "extracted"
REF_MODULES = EXTRACTED / "After_Dark_-_Totally_Twisted"

pytestmark = pytest.mark.skipif(
    not ORIGINALS.is_dir() or not REF_MODULES.is_dir(),
    reason="local Macintosh Garden originals not present on this machine",
)

# The 16 module/shared-asset files the floppy release carries once its two
# "Totally Twisted Setup Guide" read-mes are excluded.
FLOPPY_MODULE_NAMES = {
    "Mime Hunt", "Mowin' Boris", "Phlegm Boy", "Shock Clocks", "Toxic Swamp",
    "Twisted Art", "Twisted Faceplate", "Twisted Sound", "Voyeur",
    "Bungee Roulette", "Chameleon", "Coming Soon", "Flying Toilets",
    "FrankenScreen", "Message Mayhem", "Mike's So-called Life",
}

# Bonus content the CD's installer archive carries beyond the 16 core
# modules: other After Dark products' modules (EcoLogic) and bonus Twisted
# Faceplate skins. Everything else in the installer tree (the host app,
# Library 4.0, the manual, Screen Posters, "delete me", ...) must NOT show up.
CD_BONUS_NAMES = {
    "MonitorSD", "ShutDownSD",
    "Disney Faceplate", "Marvel Faceplate", "Star Trek Faceplate",
}


def _ref_fork(name: str) -> bytes:
    for set_dir in ("ADTotallyTwistedset1", "ADTotallyTwistedset2"):
        candidate = REF_MODULES / set_dir / name
        if candidate.exists():
            return Path(f"{candidate}/..namedfork/rsrc").read_bytes()
    raise KeyError(name)


def _zero_reserved_header(fork: bytes) -> bytes:
    """Zero the resource fork's 128-byte header past the four leading
    offsets/lengths. Bytes 16:128 are reserved-for-system-use scratch
    (handles, refnums, ...) that legitimately differs between two builds
    of the same archive without the resource content itself differing."""
    return fork[:16] + b"\x00" * (128 - 16) + fork[128:]


def test_floppy_sit_matches_reference_byte_for_byte():
    result = ingest(ORIGINALS / "After_Dark_-_Totally_Twisted.sit")

    assert set(result.keys()) == FLOPPY_MODULE_NAMES
    assert len(result) == 16

    for name, fork in result.items():
        assert fork == _ref_fork(name), f"{name} fork not byte-identical to reference"


def test_plain_folder_of_modules():
    result = ingest(REF_MODULES / "ADTotallyTwistedset1")

    expected = {
        "Mime Hunt", "Mowin' Boris", "Phlegm Boy", "Shock Clocks",
        "Toxic Swamp", "Twisted Art", "Twisted Faceplate", "Twisted Sound",
        "Voyeur",
    }
    assert set(result.keys()) == expected
    for name, fork in result.items():
        assert fork == _ref_fork(name)


def test_sit_hqx_binhex_demo():
    result = ingest(ORIGINALS / "FlyingToiletsDemo.sit.hqx")

    assert set(result.keys()) == {"Flying Toilets Demo"}
    ref = Path(f"{EXTRACTED / 'Flying Toilets Demo'}/..namedfork/rsrc").read_bytes()
    assert result["Flying Toilets Demo"] == ref


@pytest.mark.skipif(
    not (ORIGINALS / "totallytwistedcd.sit").exists(),
    reason="totallytwistedcd.sit not present locally",
)
def test_hybrid_cd_sit_yields_core_modules_plus_bonus_content():
    result = ingest(ORIGINALS / "totallytwistedcd.sit")

    assert FLOPPY_MODULE_NAMES <= set(result.keys())
    assert CD_BONUS_NAMES <= set(result.keys())
    # Installer/host-app/library/read-me/junk entries must not leak through.
    excluded = {
        "After Dark 3.0", "Library 4.0", "Totally Twisted™ Manual",
        "Messyges Custom", "Screen Posters", "delete me",
        "Totally Twisted Setup Guide",
    }
    assert not (excluded & set(result.keys()))

    # The CD build isn't byte-identical to the floppy build (differing
    # non-semantic reserved-header scratch bytes -- see docs/ripper.md and
    # the lane report), but the actual resource content must match once
    # that scratch region is normalized out.
    for name in FLOPPY_MODULE_NAMES:
        assert _zero_reserved_header(result[name]) == _zero_reserved_header(_ref_fork(name)), (
            f"{name}: CD-sourced fork differs from the floppy reference beyond the reserved header"
        )


@pytest.mark.skipif(
    not (EXTRACTED / "totallytwistedcd.iso").exists(),
    reason="pre-extracted totallytwistedcd.iso not present locally",
)
def test_raw_hfs_iso_given_directly_skips_windows_side():
    result = ingest(EXTRACTED / "totallytwistedcd.iso")

    assert FLOPPY_MODULE_NAMES <= set(result.keys())
    # Nothing from the ISO9660/Windows side (all-caps 8.3 names, .ZIP/.EXE)
    # should ever appear.
    assert not any(name.isupper() or name.endswith((".ZIP", ".EXE")) for name in result)


def test_appledouble_sidecar_fallback(tmp_path):
    """When a resource fork can't be read via the native APFS xattr fork
    (e.g. a non-APFS volume, or a tree unar wrote as AppleDouble), ingest
    must fall back to a `._<name>` sidecar."""
    src = REF_MODULES / "ADTotallyTwistedset1" / "Twisted Art"
    data = src.read_bytes()
    rsrc = Path(f"{src}/..namedfork/rsrc").read_bytes()

    magic, version, filler = b"\x00\x05\x16\x07", b"\x00\x02\x00\x00", b"\x00" * 16
    header = magic + version + filler + struct.pack(">H", 2)
    data_off = len(header) + 2 * 12
    rsrc_off = data_off + len(data)
    entries = (
        struct.pack(">III", 1, data_off, len(data))
        + struct.pack(">III", 2, rsrc_off, len(rsrc))
    )
    (tmp_path / "Twisted Art").write_bytes(b"")
    (tmp_path / "._Twisted Art").write_bytes(header + entries + data + rsrc)

    result = ingest(tmp_path)

    assert set(result.keys()) == {"Twisted Art"}
    assert result["Twisted Art"] == rsrc
