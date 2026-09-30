"""Byte-exact oracle test for cmid.py.

Primary oracle (per docs/ripper.md): ripped/shared-twisted-sound/*.midi,
decoded from the raw 'cmid' resources in extracted/Twisted_Sound.rsrc via our
own rsrc.py (cmid resources have no raw .bin dump in the oracle tree --
resource_dasm decodes them straight to .midi, see test_rsrc.py's docstring).

Also opportunistically checks the couple of 'cmid' resources that show up
directly in module resource forks (e.g. Chameleon, Voyeur carry their own
cmid 10/410), since those give free extra coverage of the same decoder
against independently-generated oracle files.
"""
import os
import unittest
from pathlib import Path

from .. import rsrc, cmid

RE_ROOT = Path(os.environ.get("TWISTEDRIP_RE_ROOT", Path.home() / "working" / "totally-twisted"))
ADT_ROOT = RE_ROOT / "extracted" / "After_Dark_-_Totally_Twisted"
RIPPED_ROOT = RE_ROOT / "ripped"

MODULE_CASES = [
    ("ADTotallyTwistedset1/Mime Hunt", "mime-hunt"),
    ("ADTotallyTwistedset1/Mowin' Boris", "mowin'-boris"),
    ("ADTotallyTwistedset1/Phlegm Boy", "phlegm-boy"),
    ("ADTotallyTwistedset1/Shock Clocks", "shock-clocks"),
    ("ADTotallyTwistedset1/Totally Twisted Setup Guide", "totally-twisted-setup-guide"),
    ("ADTotallyTwistedset1/Toxic Swamp", "toxic-swamp"),
    ("ADTotallyTwistedset1/Twisted Art", "twisted-art"),
    ("ADTotallyTwistedset1/Twisted Faceplate", "twisted-faceplate"),
    ("ADTotallyTwistedset1/Twisted Sound", "twisted-sound"),
    ("ADTotallyTwistedset1/Voyeur", "voyeur"),
    ("ADTotallyTwistedset2/Bungee Roulette", "bungee-roulette"),
    ("ADTotallyTwistedset2/Chameleon", "chameleon"),
    ("ADTotallyTwistedset2/Coming Soon", "coming-soon"),
    ("ADTotallyTwistedset2/Flying Toilets", "flying-toilets"),
    ("ADTotallyTwistedset2/FrankenScreen", "frankenscreen"),
    ("ADTotallyTwistedset2/Message Mayhem", "message-mayhem"),
    ("ADTotallyTwistedset2/Mike's So-called Life", "mike's-so-called-life"),
    ("ADTotallyTwistedset2/Totally Twisted Setup Guide", "totally-twisted-setup-guide"),
]


@unittest.skipUnless(RE_ROOT.exists(), f"private RE checkout not found at {RE_ROOT}")
class TestCmidOracle(unittest.TestCase):
    def test_shared_twisted_sound_bank(self):
        rsrc_path = RE_ROOT / "extracted" / "Twisted_Sound.rsrc"
        ripped_dir = RIPPED_ROOT / "shared-twisted-sound"
        self.assertTrue(rsrc_path.exists(), rsrc_path)
        self.assertTrue(ripped_dir.exists(), ripped_dir)

        fork = rsrc_path.read_bytes()
        parsed = rsrc.parse(fork)
        cmid_resources = {k: v for k, v in parsed.items() if k[0] == "cmid"}
        self.assertGreater(len(cmid_resources), 0, "no cmid resources parsed from shared bank")

        total = 0
        identical = 0
        failures = []
        for (res_type, res_id), res in sorted(cmid_resources.items()):
            oracle_path = ripped_dir / f"Twisted_Sound.rsrc_cmid_{res_id}.midi"
            if not oracle_path.exists():
                continue
            total += 1
            oracle = oracle_path.read_bytes()
            try:
                got = cmid.decode(res.data)
            except Exception as e:  # noqa: BLE001
                failures.append(f"cmid {res_id}: decode raised {e!r}")
                continue
            if got == oracle:
                identical += 1
            else:
                failures.append(
                    f"cmid {res_id}: {len(got)} bytes decoded vs {len(oracle)} bytes oracle, mismatch")

        print(f"\ncmid.py shared Twisted Sound bank parity: {identical}/{total} MIDI files byte identical")
        self.assertGreater(total, 0, "no oracle .midi files found to compare against")
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures))

    def test_module_cmid_resources(self):
        total = 0
        identical = 0
        failures = []

        for module_rel, ripped_name in MODULE_CASES:
            module_path = ADT_ROOT / module_rel
            ripped_dir = RIPPED_ROOT / ripped_name
            if not module_path.exists() or not ripped_dir.exists():
                continue

            fork = (module_path / "..namedfork" / "rsrc").read_bytes()
            parsed = rsrc.parse(fork)
            base_filename = module_path.name

            for (res_type, res_id), res in parsed.items():
                if res_type != "cmid":
                    continue
                oracle_path = ripped_dir / f"{base_filename}_cmid_{res_id}.midi"
                if not oracle_path.exists():
                    continue
                total += 1
                oracle = oracle_path.read_bytes()
                try:
                    got = cmid.decode(res.data)
                except Exception as e:  # noqa: BLE001
                    failures.append(f"{ripped_name} cmid {res_id}: decode raised {e!r}")
                    continue
                if got == oracle:
                    identical += 1
                else:
                    failures.append(f"{ripped_name} cmid {res_id}: mismatch")

        print(f"\ncmid.py module cmid parity: {identical}/{total} MIDI files byte identical")
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures))


if __name__ == "__main__":
    unittest.main()
