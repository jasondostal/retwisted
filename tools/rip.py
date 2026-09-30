#!/usr/bin/env python3
"""twistedrip CLI: rip a Totally Twisted original into retwisted asset packs.

    python3 tools/rip.py <input> --out <dir> [--only <slug>]

<input> is anything tools.twistedrip.ingest.ingest() accepts: the floppy
release's StuffIt 5 .sit, the hybrid-CD .sit/.iso, a .sit.hqx, or a folder of
already-extracted module files. Resource forks are read (native APFS fork or
an AppleDouble ._<name> sidecar), parsed, and packed exactly the way
tools/pack_assets.py packs a private RE checkout's resource_dasm dumps --
this is the bring-your-own-originals replacement for that pipeline. See
docs/ripper.md.

This file adds tools/twistedrip's own parent directory to sys.path so it can
be run directly (`python3 tools/rip.py ...`); `python3 -m twistedrip` is not
required.
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from twistedrip import ingest as ingest_mod  # noqa: E402
from twistedrip import pack, rsrc  # noqa: E402

SHARED_SOUND_NAME = "Twisted Sound"
SHARED_ART_NAME = "Twisted Art"

# The 13 module Mac file names this pack ships, in the same order
# tools/pack_assets.py's callers have always packed them. This is the only
# place that needs to know the module list; everything else (series, sounds,
# music, pens) comes out of the resource forks themselves.
#
# See tools/twistedrip/tests/conftest.py's MODULES table -- this list is the
# same 13 Mac display names, without the RE checkout's `ripped/<dir>` name
# (this pipeline never touches resource_dasm dumps) or the module->series
# grouping (pack_assets.py's `set dir` is just where the private checkout
# happened to keep the .rsrc files on disk; ingest.py finds them by content,
# not by directory).
MODULE_NAMES = [
    "Bungee Roulette",
    "Chameleon",
    "Coming Soon",
    "Flying Toilets",
    "FrankenScreen",
    "Message Mayhem",
    "Mike's So-called Life",
    "Mime Hunt",
    "Mowin' Boris",
    "Phlegm Boy",
    "Shock Clocks",
    "Toxic Swamp",
    "Voyeur",
]


def slug_for(mac_name: str) -> str:
    """Mac file name -> assets/<slug> directory name. Mirrors
    tools/pack_assets.py's own `slug = module.replace("'", '').replace(' ',
    '-')`, applied directly to the display name instead of to a RE
    checkout's already-lowercased `ripped/<dir>` name: lower-case, spaces to
    dashes, apostrophes dropped. Verified against the live assets/ tree for
    both apostrophe-bearing modules (Mike's So-called Life ->
    mikes-so-called-life, Mowin' Boris -> mowin-boris)."""
    return mac_name.lower().replace(" ", "-").replace("'", "")


def rip(input_path: Path, out_root: Path, only: str | None = None) -> int:
    forks = ingest_mod.ingest(input_path)
    resources = {name: rsrc.parse(fork) for name, fork in forks.items()}

    shared_sound = resources.get(SHARED_SOUND_NAME, {})
    shared_art = resources.get(SHARED_ART_NAME)

    done = 0
    for name in MODULE_NAMES:
        slug = slug_for(name)
        if only and slug != only:
            continue
        module = resources.get(name)
        if module is None:
            print(f"{slug}: SKIP -- no \"{name}\" resource fork found in {input_path}")
            continue

        out = out_root / slug
        meta = pack.build(slug, name, module, shared_sound, shared_art, out)

        ncompounds = sum(1 for _ in (out / "compounds").rglob("*.png")) \
            if (out / "compounds").exists() else 0
        nsnd = sum(1 for _ in (out / "sounds").glob("*.wav")) \
            if (out / "sounds").exists() else 0
        music_note = ""
        if "music" in meta:
            nsongs = len(meta["music"]["songs"])
            music_note = f", {nsongs} song{'s' if nsongs != 1 else ''}"
        print(f"{slug}: {len(meta['series'])} series, {ncompounds} compounds, "
              f"{nsnd} sounds{music_note} -> {out}")
        done += 1

    # The faceplate banner is collection-wide, not per-module: one
    # assets/_shared/faceplate.png for the whole pack tree (the macOS
    # saver's control panel draws it across the top). Written on every rip,
    # --only included, because a tree with one module still needs it.
    face = pack.build_faceplate(resources, out_root)
    if face is not None:
        print(f"{pack.SHARED_DIR}: faceplate -> {face}")
    else:
        print(f'{pack.SHARED_DIR}: WARN -- no faceplate PICT found in {input_path}')

    if only and done == 0:
        print(f"error: --only {only!r} matched no module", file=sys.stderr)
        return 1
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                  formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("input", type=Path,
                     help="a .sit / .sit.hqx / .iso, or a folder of extracted module files")
    ap.add_argument("--out", required=True, type=Path, help="output assets/ directory")
    ap.add_argument("--only", default=None, help="rip only this one slug")
    args = ap.parse_args(argv)

    args.out.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    rc = rip(args.input, args.out, args.only)
    print(f"done in {time.time() - t0:.1f}s")
    return rc


if __name__ == "__main__":
    sys.exit(main())
