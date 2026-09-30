"""Byte/value-exact oracle test for inst.py.

Oracle: the files pack_music() (tools/pack_assets.py) actually reads under
ripped/shared-twisted-sound/ -- resource_dasm's *_INST_<program>_<name>.json
dumps (compared on the 'regions' list, the only part pack_music() reads) and
*_csnd_<id>_*.wav dumps (compared byte-for-byte).
"""
import json
import os
import unittest
from pathlib import Path

from .. import rsrc, inst

RE_ROOT = Path(os.environ.get("TWISTEDRIP_RE_ROOT", Path.home() / "working" / "totally-twisted"))
SHARED_RSRC = RE_ROOT / "extracted" / "Twisted_Sound.rsrc"
SHARED_RIPPED = RE_ROOT / "ripped" / "shared-twisted-sound"


@unittest.skipUnless(RE_ROOT.exists(), f"private RE checkout not found at {RE_ROOT}")
class TestInstOracle(unittest.TestCase):
    def test_inst_regions_match_pack_music_json(self):
        self.assertTrue(SHARED_RSRC.exists())
        self.assertTrue(SHARED_RIPPED.exists())

        bank = rsrc.parse(SHARED_RSRC.read_bytes())
        base_filename = SHARED_RSRC.name

        inst_ids = sorted(i for (t, i) in bank if t == "INST")
        self.assertGreater(len(inst_ids), 0)

        total = ok = 0
        failures = []
        for iid in inst_ids:
            matches = list(SHARED_RIPPED.glob(f"{base_filename}_INST_{iid}_*.json")) or \
                list(SHARED_RIPPED.glob(f"{base_filename}_INST_{iid}.json"))
            if not matches:
                continue
            total += 1
            oracle_regions = json.loads(matches[0].read_text())["regions"]

            decoded = inst.decode_inst(bank[("INST", iid)].data)
            got_regions = inst.resolve_regions(decoded, bank, base_filename)

            # Oracle regions may have keys in a different order; compare as
            # dicts (dict equality ignores key order) in list order.
            if got_regions == oracle_regions:
                ok += 1
            else:
                failures.append(f"INST {iid}: regions mismatch\n  got:    {got_regions}\n  oracle: {oracle_regions}")

        print(f"\ninst.py INST region parity: {ok}/{total} instruments' regions identical")
        self.assertGreater(total, 0)
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures))

    def test_csnd_wavs_byte_exact(self):
        self.assertTrue(SHARED_RSRC.exists())
        self.assertTrue(SHARED_RIPPED.exists())

        bank = rsrc.parse(SHARED_RSRC.read_bytes())
        base_filename = SHARED_RSRC.name

        csnd_ids = sorted(i for (t, i) in bank if t == "csnd")
        self.assertGreater(len(csnd_ids), 0)

        total = ok = 0
        failures = []
        for cid in csnd_ids:
            matches = list(SHARED_RIPPED.glob(f"{base_filename}_csnd_{cid}_*.wav")) or \
                list(SHARED_RIPPED.glob(f"{base_filename}_csnd_{cid}.wav"))
            if not matches:
                continue
            total += 1
            oracle = matches[0].read_bytes()
            try:
                got = inst.csnd_to_wav(bank[("csnd", cid)].data)
            except Exception as e:  # noqa: BLE001
                failures.append(f"csnd {cid}: raised {e!r}")
                continue
            if got == oracle:
                ok += 1
            else:
                failures.append(f"csnd {cid}: {len(got)} bytes vs oracle {len(oracle)} bytes")

        print(f"\ninst.py csnd WAV parity: {ok}/{total} WAV files byte identical")
        self.assertGreater(total, 0)
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures))


if __name__ == "__main__":
    unittest.main()
