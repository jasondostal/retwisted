"""decode/rlep.py vs the private RE checkout's oracle frame_NNN.png dumps
(ripped/<module>/frames-<bank>/) across all 13 modules."""
import pytest

from tools.twistedrip.decode import rlep

from .conftest import MODULES, RIPPED, module_resources


@pytest.mark.parametrize('slug', sorted(MODULES))
def test_rlep_byte_parity(slug):
    _name, _set, ripped_dir = MODULES[slug]
    module = module_resources(slug)
    rleps = {i: r.data for (t, i), r in module.items() if t == 'RLEP'}
    rdats = {i: r.data for (t, i), r in module.items() if t == 'Rdat'}
    assert rleps, f'{slug}: no RLEP resources found'

    checked = 0
    for bank_id, data in sorted(rleps.items()):
        ctab, frames = rlep.parse_bank(data)
        status = rlep.check_rdat(rdats.get(bank_id), ctab, frames)
        assert 'MISMATCH' not in status, f'{slug} bank {bank_id}{status}'

        oracle_dir = RIPPED / ripped_dir / f'frames-{bank_id}'
        if not oracle_dir.is_dir():
            continue  # oracle wasn't dumped for this bank; skip, don't fail
        for fid, (rect, stream) in sorted(frames.items()):
            oracle_png = oracle_dir / f'frame_{fid:03d}.png'
            if not oracle_png.exists():
                continue
            rows = rlep.decode_rows(stream)
            rendered = rlep.render(rows, ctab, rect)
            assert rendered is not None, f'{slug} bank {bank_id} frame {fid}: render failed'
            _w, _h, png = rendered
            assert png == oracle_png.read_bytes(), (
                f'{slug} bank {bank_id} frame {fid}: byte mismatch vs oracle')
            checked += 1
    assert checked > 0, f'{slug}: no oracle frames found to check against'
