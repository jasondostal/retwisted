"""decode/pict.py + the two additive pack outputs it feeds:
assets/_shared/faceplate.png and meta.json's `help`.

Oracles, both from the private RE checkout (so the whole module skips on a
machine that only has the public retwisted checkout):

* resource_dasm's own decode of the same PICT resources, dumped as 32-bit
  BMPs -- the faceplate decoder must be pixel-identical to it;
* resource_dasm's `*_TEXT_1000.txt` dump -- the help text must come out of
  the raw Mac Roman resource as the same string.

The last test is the one the parity gate cares about: tools/pack_assets.py
(reference) and tools/twistedrip/pack.py (ripper) reach the faceplate PNG
by completely different routes -- a BMP dump vs the raw PICT -- and the
bytes have to match exactly.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

from .. import pack
from ..decode import pict
from .conftest import RIPPED, load_resources

FACEPLATE_NAME = 'Twisted Faceplate'
FACEPLATE_SET = 'ADTotallyTwistedset1'


def _pack_assets_module():
    """tools/pack_assets.py loaded as a module (it is a script, not a
    package member, and importing it must not run main())."""
    path = Path(__file__).resolve().parents[3] / 'tools' / 'pack_assets.py'
    spec = importlib.util.spec_from_file_location('_pack_assets_ref', path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def _bmp(sub: str, pattern: str) -> Path:
    d = RIPPED / sub
    if not d.is_dir():
        pytest.skip(f'{d} not present')
    f = next(d.glob(pattern), None)
    if f is None:
        pytest.skip(f'no {pattern} in {d}')
    return f


@pytest.mark.parametrize('sub,pattern,size', [
    ('twisted-faceplate', '*_PICT_128_*.bmp', (185, 48)),
    ('twisted-faceplate', '*_PICT_129*.bmp', (185, 47)),
])
def test_pict_matches_resource_dasm(sub, pattern, size):
    """The 8-bit colour faceplate and its 1-bit B&W companion."""
    ref = _pack_assets_module()
    res = load_resources(FACEPLATE_NAME, FACEPLATE_SET)
    rid = int(pattern.split('_PICT_')[1][:3])
    w, h, rgba = pict.decode(res[('PICT', rid)].data)
    assert (w, h) == size

    bw, bh, rows = ref.bmp_rgba(_bmp(sub, pattern))
    assert (bw, bh) == size
    assert rgba == b''.join(rows)


def test_faceplate_png_identical_from_both_packers(tmp_path):
    """The parity gate's real question: pack.py (raw PICT) and
    pack_assets.py (resource_dasm BMP) must write the same PNG bytes."""
    ref = _pack_assets_module()
    res = load_resources(FACEPLATE_NAME, FACEPLATE_SET)

    got = pack.build_faceplate({FACEPLATE_NAME: res}, tmp_path)
    assert got is not None and got.name == 'faceplate.png'
    assert got.parent.name == pack.SHARED_DIR

    ref_out = tmp_path / 'ref'
    assert ref.pack_faceplate(RIPPED.parent, ref_out) is not None
    assert (ref_out / ref.SHARED_DIR / 'faceplate.png').read_bytes() == got.read_bytes()


def test_faceplate_absent_is_not_an_error(tmp_path):
    assert pack.build_faceplate({}, tmp_path) is None
    assert not (tmp_path / pack.SHARED_DIR).exists()


@pytest.mark.parametrize('slug,mac_name,set_dir,ripped_dir', [
    ('shock-clocks', 'Shock Clocks', 'ADTotallyTwistedset1', 'shock-clocks'),
    ('mime-hunt', 'Mime Hunt', 'ADTotallyTwistedset1', 'mime-hunt'),
    ('message-mayhem', 'Message Mayhem', 'ADTotallyTwistedset2', 'message-mayhem'),
])
def test_help_text_matches_resource_dasm(slug, mac_name, set_dir, ripped_dir):
    ref = _pack_assets_module()
    mdir = RIPPED / ripped_dir
    if not mdir.is_dir():
        pytest.skip(f'{mdir} not present')
    got = pack.build_help(load_resources(mac_name, set_dir))
    assert got, f'{slug} has no TEXT 1000'
    assert '\r' not in got
    assert got == ref.pack_help(mdir, mac_name)


def test_help_absent_is_empty_string():
    assert pack.build_help({}) == ''
