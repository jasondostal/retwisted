"""decode/oftb.py vs the private RE checkout's oracle compound_NNN.png dumps
(ripped/<module>/compound-<base>/) across all 13 modules."""
import pytest

from tools.twistedrip.decode import oftb

from .conftest import MODULES, RIPPED, module_resources


@pytest.mark.parametrize('slug', sorted(MODULES))
def test_oftb_byte_parity(slug):
    name, _set, ripped_dir = MODULES[slug]
    module = module_resources(slug)
    rlep_by_id = {i: r.data for (t, i), r in module.items() if t == 'RLEP'}
    oftb_by_id = {i: r.data for (t, i), r in module.items() if t == 'OFtb'}
    ofst_by_id = {i: r.data for (t, i), r in module.items() if t == 'OFst'}
    assert ofst_by_id, f'{slug}: no OFst resources found'

    shadow_ch = oftb.SHADOW_CHANNELS.get(name, frozenset())
    checked = 0
    all_warnings = []
    for base, ofst_data in sorted(ofst_by_id.items()):
        oracle_dir = RIPPED / ripped_dir / f'compound-{base}'
        if not oracle_dir.is_dir():
            continue
        art = oftb.load_art_chain(rlep_by_id, base)
        _bounds, recs = oftb.parse_ofst(ofst_data)
        for fno, off, _dx, _dy in recs:
            oracle_png = oracle_dir / f'compound_{fno:03d}.png'
            if not oracle_png.exists():
                continue
            tb = oftb_by_id.get(base + (off >> 16))
            assert tb is not None, f'{slug} series {base} compound {fno}: missing OFtb bank'
            _bnds, _items, w, h, png, warnings = oftb.compose_compound(
                art, tb, off & 0xFFFF, shadow_ch)
            # A handful of compounds across the collection have known
            # gate warnings (missing/mis-sized art) baked into the RE data
            # itself -- oftb_compose.py's own `main()` reports them and
            # composes anyway. The oracle PNGs were produced the same way,
            # so byte equality (not warning-free-ness) is the actual gate.
            if warnings:
                all_warnings.append(f'{slug} series {base} compound {fno}: {warnings}')
            oracle_bytes = oracle_png.read_bytes()
            if w and h:
                assert png == oracle_bytes, (
                    f'{slug} series {base} compound {fno}: byte mismatch vs oracle')
            checked += 1
    assert checked > 0, f'{slug}: no oracle compounds found to check against'
    if all_warnings:
        print(f'{slug}: {len(all_warnings)} known gate warning(s) (byte parity still held):')
        for w in all_warnings:
            print(' ', w)
