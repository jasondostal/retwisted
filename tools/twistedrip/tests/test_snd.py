"""Byte-exact oracle test for snd.py.

Two independent oracles, matching snd.py's two code paths:

  * Plain 'snd ' resources (no 'sndS' companion at the same id in the same
    resource fork): ripped/<module>/*_snd_<id>_*.wav, resource_dasm's
    float32 WAV.
  * 'snd ' resources that DO have a same-id 'sndS' companion: the module's
    own ripped/<module>/expanded/snd_<id>_*.wav, produced by
    scripts/snds_expand.py's 8-bit PCM decoder. Every module can carry its
    own local sndS-companioned sounds (not just the shared Twisted Sound
    bank -- e.g. Phlegm Boy has sndS 132/136/138 at its own ids), plus the
    shared bank itself (extracted/Twisted_Sound.rsrc) against
    ripped/shared-twisted-sound-expanded/.
"""
import os
import unittest
from pathlib import Path

from .. import rsrc, snd

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
class TestSndOracle(unittest.TestCase):
    def test_plain_and_expanded_snd_across_modules(self):
        plain_total = plain_ok = 0
        expanded_total = expanded_ok = 0
        failures = []

        for module_rel, ripped_name in MODULE_CASES:
            module_path = ADT_ROOT / module_rel
            ripped_dir = RIPPED_ROOT / ripped_name
            if not module_path.exists() or not ripped_dir.exists():
                continue

            fork = (module_path / "..namedfork" / "rsrc").read_bytes()
            parsed = rsrc.parse(fork)
            base_filename = module_path.name
            expanded_dir = ripped_dir / "expanded"

            for (res_type, res_id), res in parsed.items():
                if res_type != "snd ":
                    continue
                sndS_res = parsed.get(("sndS", res_id))

                if sndS_res is not None:
                    if not expanded_dir.exists():
                        continue
                    matches = list(expanded_dir.glob(f"snd_{res_id}_*.wav")) or list(expanded_dir.glob(f"snd_{res_id}.wav"))
                    if not matches:
                        continue
                    expanded_total += 1
                    oracle = matches[0].read_bytes()
                    try:
                        got = snd.to_wav(res.data, sndS_res.data)
                    except Exception as e:  # noqa: BLE001
                        failures.append(f"{ripped_name} sndS {res_id}: raised {e!r}")
                        continue
                    if got == oracle:
                        expanded_ok += 1
                    else:
                        failures.append(
                            f"{ripped_name} sndS {res_id}: {len(got)} bytes vs oracle {len(oracle)} bytes")
                else:
                    matches = list(ripped_dir.glob(f"{base_filename}_snd_{res_id}_*.wav")) or \
                        list(ripped_dir.glob(f"{base_filename}_snd_{res_id}.wav"))
                    if not matches:
                        continue
                    plain_total += 1
                    oracle = matches[0].read_bytes()
                    try:
                        got = snd.to_wav(res.data)
                    except Exception as e:  # noqa: BLE001
                        failures.append(f"{ripped_name} snd {res_id}: raised {e!r}")
                        continue
                    if got == oracle:
                        plain_ok += 1
                    else:
                        failures.append(
                            f"{ripped_name} snd {res_id}: {len(got)} bytes vs oracle {len(oracle)} bytes")

        print(f"\nsnd.py plain-snd parity: {plain_ok}/{plain_total} WAV files byte identical")
        print(f"snd.py sndS-expanded parity: {expanded_ok}/{expanded_total} WAV files byte identical")
        self.assertGreater(plain_total, 0)
        self.assertGreater(expanded_total, 0)
        if failures:
            self.fail(f"{len(failures)} mismatches (of {plain_total + expanded_total}):\n" + "\n".join(failures[:50]))

    def test_shared_twisted_sound_expanded(self):
        rsrc_path = RE_ROOT / "extracted" / "Twisted_Sound.rsrc"
        expanded_dir = RIPPED_ROOT / "shared-twisted-sound-expanded"
        self.assertTrue(rsrc_path.exists())
        self.assertTrue(expanded_dir.exists())

        fork = rsrc_path.read_bytes()
        parsed = rsrc.parse(fork)

        total = 0
        ok = 0
        failures = []
        for (res_type, res_id), res in parsed.items():
            if res_type != "snd ":
                continue
            sndS_res = parsed.get(("sndS", res_id))
            if sndS_res is None:
                continue
            matches = list(expanded_dir.glob(f"snd_{res_id}_*.wav")) or list(expanded_dir.glob(f"snd_{res_id}.wav"))
            if not matches:
                continue
            total += 1
            oracle = matches[0].read_bytes()
            try:
                got = snd.to_wav(res.data, sndS_res.data)
            except Exception as e:  # noqa: BLE001
                failures.append(f"sndS {res_id}: raised {e!r}")
                continue
            if got == oracle:
                ok += 1
            else:
                failures.append(f"sndS {res_id}: mismatch")

        print(f"\nsnd.py shared Twisted Sound bank sndS-expanded parity: {ok}/{total} WAV files byte identical")
        self.assertGreater(total, 0)
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures))


if __name__ == "__main__":
    unittest.main()
