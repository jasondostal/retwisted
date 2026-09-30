"""Byte-exact oracle test for rsrc.py.

Oracle: the private RE checkout at ~/working/totally-twisted (local only,
never copied into this repo -- see docs/ripper.md). For each of the 18
Totally Twisted module files (ADTotallyTwistedset1 + set2) plus the shared
Twisted_Sound.rsrc, resource_dasm's raw `<type>_<id>[_<name>].bin` dumps under
ripped/<module>/ are compared byte-for-byte (and name-for-name) against what
rsrc.parse() extracts directly from the resource fork.

Only `.bin` dumps are used as oracle data: resource_dasm decodes some types
(snd, cmid, csnd, INST, ...) straight to a derived format (.wav, .midi, ...)
without ever writing a raw .bin, so those types have no raw-byte oracle here
-- they're covered by snd.py / cmid.py / inst.py's own oracle tests instead.

Skips (does not fail) when the private checkout isn't present, e.g. on CI or
another contributor's machine.
"""
import os
import unittest
from pathlib import Path

from .. import rsrc

RE_ROOT = Path(os.environ.get("TWISTEDRIP_RE_ROOT", Path.home() / "working" / "totally-twisted"))
ADT_ROOT = RE_ROOT / "extracted" / "After_Dark_-_Totally_Twisted"
RIPPED_ROOT = RE_ROOT / "ripped"

# (module file relative to ADT_ROOT, ripped/ subdirectory name)
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

# Filename escaping resource_dasm applies to a resource's raw Mac-Roman name
# bytes when building a `%n` filename component (TextCodecs.cc
# decode_mac_roman(char, for_filename=true) + should_escape_mac_roman_filename_char):
# control chars, '/' and ':' become '_'; everything else is the plain
# Mac-Roman -> UTF-8 mapping (same table Python's 'mac_roman' codec uses).
def _filename_escaped_name(raw_name_bytes: bytes) -> str:
    out = []
    for b in raw_name_bytes:
        if b < 0x20 or b == ord('/') or b == ord(':'):
            out.append('_')
        else:
            out.append(bytes([b]).decode('mac_roman'))
    return ''.join(out)


# Known-stale oracle fixture: re-running the checked-in resource_dasm binary
# (tools/resource_dasm/build/resource_dasm) against the live Chameleon module
# file today reproduces our parsed 8-byte 'JOHN'/130 resource exactly
# (`0000 590c 0000 3e72`), but ripped/chameleon/Chameleon_JOHN_130_DynaCham2.bin
# holds 60 unrelated bytes (a stray hex-dump-shaped text blob). This is drift
# in the oracle tree, not a parser bug -- verified by direct comparison
# against a fresh resource_dasm run, not assumed. Excluded here rather than
# silently passed.
KNOWN_STALE_ORACLE_ENTRIES = {
    ("chameleon", "JOHN", 130),
}


def _iter_oracle_bin_files(ripped_dir: Path, base_filename: str):
    """Yield (type, id, name_or_None, data) for each raw .bin dump directly
    under ripped_dir whose filename starts with '<base_filename>_'.

    Skips '_data.bin' / '_excess.bin' files: those hold leftover bytes after
    a decoded STR/STRN payload (resource_dasm's write_decoded_STR/STRN), not
    the resource's raw bytes -- they have no '.bin' raw dump of their own to
    compare against.
    """
    prefix = base_filename + "_"
    for f in sorted(ripped_dir.iterdir()):
        if not f.is_file() or f.suffix != ".bin":
            continue
        if not f.name.startswith(prefix):
            continue
        if f.name.endswith("_data.bin") or f.name.endswith("_excess.bin"):
            continue
        rest = f.name[len(prefix):-len(".bin")]
        parts = rest.split("_", 1)
        res_type = parts[0]
        remainder = parts[1] if len(parts) > 1 else ""
        # remainder is "<id>" or "<id>_<escaped-name>"
        if "_" in remainder:
            id_str, name_part = remainder.split("_", 1)
        else:
            id_str, name_part = remainder, None
        yield res_type, int(id_str), name_part, f.read_bytes()


@unittest.skipUnless(RE_ROOT.exists(), f"private RE checkout not found at {RE_ROOT}")
class TestRsrcOracle(unittest.TestCase):
    def test_all_modules_byte_exact(self):
        total = 0
        identical = 0
        failures = []

        for module_rel, ripped_name in MODULE_CASES:
            module_path = ADT_ROOT / module_rel
            ripped_dir = RIPPED_ROOT / ripped_name
            if not module_path.exists() or not ripped_dir.exists():
                continue

            fork_path = module_path / "..namedfork" / "rsrc"
            fork = fork_path.read_bytes()
            parsed = rsrc.parse(fork)
            base_filename = module_path.name

            for res_type, res_id, name_part, oracle_data in _iter_oracle_bin_files(ripped_dir, base_filename):
                if (ripped_name, res_type, res_id) in KNOWN_STALE_ORACLE_ENTRIES:
                    continue
                total += 1
                key = (res_type, res_id)
                if key not in parsed:
                    failures.append(f"{ripped_name}: missing resource {key}")
                    continue
                got = parsed[key]

                ok = True
                if got.data != oracle_data:
                    failures.append(
                        f"{ripped_name}: {key} data mismatch "
                        f"({len(got.data)} vs {len(oracle_data)} bytes)")
                    ok = False

                if name_part is not None:
                    expected_escaped = _filename_escaped_name(got.name.encode('mac_roman', errors='replace'))
                    if expected_escaped != name_part:
                        failures.append(
                            f"{ripped_name}: {key} name mismatch "
                            f"(parsed {got.name!r} -> {expected_escaped!r}, oracle filename says {name_part!r})")
                        ok = False
                elif got.name != "":
                    failures.append(f"{ripped_name}: {key} expected unnamed, parsed name={got.name!r}")
                    ok = False

                if ok:
                    identical += 1

        print(f"\nrsrc.py oracle parity: {identical}/{total} .bin resources byte-and-name identical "
              f"across {sum(1 for m, r in MODULE_CASES if (ADT_ROOT / m).exists())} module files")
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures[:50]))

    def test_shared_twisted_sound_rsrc(self):
        rsrc_path = RE_ROOT / "extracted" / "Twisted_Sound.rsrc"
        ripped_dir = RIPPED_ROOT / "shared-twisted-sound"
        if not rsrc_path.exists() or not ripped_dir.exists():
            self.skipTest("shared Twisted_Sound.rsrc oracle not present")

        fork = rsrc_path.read_bytes()
        parsed = rsrc.parse(fork)
        base_filename = rsrc_path.name

        total = 0
        identical = 0
        failures = []
        for res_type, res_id, name_part, oracle_data in _iter_oracle_bin_files(ripped_dir, base_filename):
            total += 1
            key = (res_type, res_id)
            if key not in parsed:
                failures.append(f"shared-twisted-sound: missing resource {key}")
                continue
            got = parsed[key]
            if got.data == oracle_data:
                identical += 1
            else:
                failures.append(f"shared-twisted-sound: {key} data mismatch")

        print(f"\nrsrc.py shared Twisted_Sound.rsrc parity: {identical}/{total} .bin resources byte identical")
        if failures:
            self.fail(f"{len(failures)} mismatches (of {total}):\n" + "\n".join(failures[:50]))


if __name__ == "__main__":
    unittest.main()
