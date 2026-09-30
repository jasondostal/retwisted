"""pack.py's art/meta path (build_art + build_palettes + build_strings, i.e.
everything that doesn't depend on lane 2's snd/cmid/inst) vs a freshly
pack_assets.py-generated reference pack, across all 13 modules. Sounds,
music and pens500.json are covered by test_pens.py and, once lane 2 lands,
a follow-up sound/music parity test -- they are deliberately excluded here
so this path is testable without lane 2."""
import json

import pytest

from tools.twistedrip import pack

from .conftest import MODULES, module_resources


@pytest.mark.parametrize('slug', sorted(MODULES))
def test_pack_art_meta_parity(slug, ref_pack_dir, tmp_path):
    name, _set, _ripped = MODULES[slug]
    module = module_resources(slug)
    ref = ref_pack_dir(slug)

    out = tmp_path / slug
    out.mkdir()
    # A handful of compounds across the collection carry known gate warnings
    # (missing/mis-sized art) baked into the RE data itself -- see
    # decode/oftb.py's docstring and test_oftb.py. Byte equality below is
    # the actual gate, not warning-free-ness.
    series, baked, warnings = pack.build_art(slug, name, module, out)
    palettes = pack.build_palettes(module)
    strings = pack.build_strings(module)
    help_text = pack.build_help(module)
    meta = {
        'module': name.lower().replace(' ', '-'),
        'field': pack.FIELDS.get(slug, [0, 0, 0]),
        'series': series,
        'palettes': palettes,
        'base_clut': pack.BASE_CLUT.get(slug),
        'baked': baked,
        'strings': strings,
        # TEXT 1000, the module's own blurb. Not lane-2-dependent, so it
        # belongs in this gate rather than in the excluded set: pack_assets.py
        # gets it from resource_dasm's *_TEXT_1000.txt dump and pack.py from
        # the raw Mac Roman resource, and the two have to agree exactly.
        'help': help_text,
        # Control-panel words: pack_assets.py reads the raw sUnt dumps and
        # the MENU text dumps, pack.py the resources; they must agree.
        'slider_words': pack.build_slider_words(module),
        'menus': pack.build_menus(module),
    }

    # compounds/ must be byte-identical, file for file.
    ref_pngs = {p.relative_to(ref) for p in (ref / 'compounds').rglob('*.png')}
    got_pngs = {p.relative_to(out) for p in (out / 'compounds').rglob('*.png')}
    missing = ref_pngs - got_pngs
    extra = got_pngs - ref_pngs
    assert not missing, f'{slug}: missing compound PNGs {sorted(missing)[:10]}'
    assert not extra, f'{slug}: unexpected extra compound PNGs {sorted(extra)[:10]}'
    mismatched = [rel for rel in ref_pngs
                  if (ref / rel).read_bytes() != (out / rel).read_bytes()]
    assert not mismatched, f'{slug}: byte-mismatched PNGs {mismatched[:10]}'

    ref_meta = json.loads((ref / 'meta.json').read_text())
    ref_meta.pop('music', None)  # lane-2-dependent, covered separately
    assert json.dumps(meta, sort_keys=True) == json.dumps(ref_meta, sort_keys=True), (
        f'{slug}: meta.json (art/meta subset) mismatch')


def test_parse_sunt_and_menu_synthetic():
    """Layout checks on made-up bytes (no original needed)."""
    sunt = bytes([0, 2, 0, 0, 3]) + b'Low' + bytes([0, 50, 4]) + b'High'
    assert pack._parse_sunt(sunt) == [[0, 'Low'], [50, 'High']]
    assert pack._parse_sunt(b'') == []
    menu = bytes(14) + bytes([5]) + b'Title'
    for item in (b'Alpha', b'-', b'Beta'):
        menu += bytes([len(item)]) + item + bytes(4)
    menu += bytes([0])
    assert pack._parse_menu(menu) == ['Alpha', '-', 'Beta']
    assert pack._parse_menu(b'') == []
