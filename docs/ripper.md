# twistedrip — the bring-your-own-originals ripper (spec, 2026-09-19)

Goal: a stranger with retwisted and the two Macintosh Garden downloads gets
identical asset packs to the ones we run today, with no private RE checkout
and no resource_dasm. Decision record: TODO.md "Distribution".

## Inputs (what the user has)
1. `After_Dark_-_Totally_Twisted.sit` — Mac floppy release. StuffIt 5, 18 module
   files with resource forks. Extract with `unar` (Homebrew); forks survive as
   `..namedfork/rsrc` on APFS.
2. `totallytwistedcd.sit` — hybrid CD; contains a 39.7 MB ISO whose HFS side
   holds the Mac installer + demos. Read the HFS side in pure Python
   (`machfs` on PyPI) — fall back to `hmount`/hfsutils only if machfs can't.
3. A plain folder of module files (someone who already extracted them).

## Usage

Two commands a stranger runs, from the repo root:

```sh
brew install unar                                   # StuffIt/BinHex extraction
pip install -r tools/twistedrip/requirements.txt    # machfs, macresources, pytest

python3 tools/rip.py After_Dark_-_Totally_Twisted.sit --out assets/
```

That rips all 13 modules into `assets/<slug>/` (meta.json, compounds/,
sounds/, and music/ + pens500.json where the module has them) — pass the
CD's `totallytwistedcd.sit` instead for the identical result from that
release, or `--only <slug>` to rip just one module. No private RE checkout,
no resource_dasm, no Berkeley content ever touches the repo (`assets/` is
gitignored). `tools/rip.py` adds its own directory to `sys.path`, so it
runs directly — `python3 -m twistedrip` is never required.

### Rust port (`ripper/`, 2026-09-29)

`ripper/` is a zero-dependency Rust crate (lib `twistedrip`, bin
`twistedrip`) that does the same job with no Python, `unar` or `machfs` --
so the macOS saver can rip the user's originals on first run:

```sh
cargo run -p ripper --release -- After_Dark_-_Totally_Twisted.sit --out assets/ [--only <slug>]
```

Library API: `twistedrip::rip(input, out_dir, &Options { only }, &mut |p: &Progress| ..) -> Result<Report, Error>`.
It is held to **byte** parity with `tools/rip.py` (every PNG, WAV, MIDI and
JSON file, from the floppy `.sit`, the CD `.sit` and the CD `.iso`), which
is why it carries its own zlib-exact level-9 deflate and a
`json.dumps(indent=1)`-exact writer. `cargo test -p ripper` runs the
synthetic, ingest, resource_dasm-oracle and end-to-end tests (skipping
whatever local originals are absent); the full 13-module x 3-input gate is
`cargo test -p ripper --release -- --ignored`. The Python ripper stays the
reference until it is retired.

## Pipeline (one Python package, `tools/twistedrip/`)
```
ingest.py   path -> {module_name: fork_bytes}      (lane 3)
rsrc.py     fork_bytes -> {(type, id): Resource}   (lane 2)  Resource = (name, data)
snd.py      snd_bytes[, sndS_bytes] -> wav_bytes   (lane 2)  incl. Berkeley delta stream
cmid.py     cmid_bytes -> smf_bytes                (lane 2)  SoundMusicSys LZSS
inst.py     INST/csnd -> what resource_dasm emitted (lane 2)  needed by pack music
decode/     rlep, oftb, pens, clut, pict decoders   (lane 1)  ported from the RE repo scripts
pack.py     resources -> assets/<slug>/ (meta.json v4, compounds/, sounds/)
            + assets/_shared/faceplate.png                (lane 1)
parity.py   rip everything, diff against a reference assets/ tree (lane 1)
rip.py      CLI entry: `python3 tools/rip.py <input> --out assets/` (integration, after lanes)
```
Interfaces are fixed by the stubs in `tools/twistedrip/*.py`. Lanes own only
their files. Nothing in this package may embed Berkeley content: no resource
bytes, no strings from STR#, no listing excerpts beyond addresses.

## The gate
`parity.py` must report zero differences for all 13 module packs plus the
shared sound bank against the packs produced by `tools/pack_assets.py` today:
- PNG: byte-identical preferred; pixel-identical (RGBA) acceptable with a WARN.
- WAV: byte-identical.
- meta.json: identical after canonical JSON (sorted keys) round-trip.
- pens500.json: identical after canonical round-trip.
Until the gate is green the ripper is not done.

`_shared/` is walked like any other directory under `assets/`, so
`_shared/faceplate.png` is gated the same way a module PNG is. The two
packers reach it by different routes -- `pack_assets.py` converts
resource_dasm's 32-bit BMP dump, `pack.py` decodes the raw `PICT` with
`decode/pict.py` -- and the gate is what proves they agree byte for byte.
See `tools/twistedrip/tests/test_pict.py`.

## The control panel's own assets (2026-09-19)
Two additive outputs the macOS saver's After Dark control panel needs:

- `assets/_shared/faceplate.png` -- the "Twisted Faceplate" file's `PICT`
  128 ("Mac Faceplate.PICT", 185x48, an 8-bit indexed PackBitsRect PixMap).
  After Dark 3.0's own `PICT` 128 (185x47) is the documented fallback,
  though `ingest.py`'s INFRA filter drops the After Dark `cdev`, so a rip of
  either Macintosh Garden original always takes the Twisted one.
- `meta.json`'s `help` -- the module's `TEXT` 1000, Mac Roman with CR line
  endings in the resource, normalised to UTF-8 + LF (which is also exactly
  what resource_dasm's `*_TEXT_1000.txt` dump contains, so both packers
  produce the same string).

Both are `#[serde(default)]`-safe on the Rust side: `engine::Meta` gained
`help` and an older pack reads back as `""`.

**Status, 2026-09-19: GATE CLEAN.** `tools/rip.py` against both Macintosh
Garden originals (`After_Dark_-_Totally_Twisted.sit` floppy release and
`totallytwistedcd.sit` hybrid CD), diffed with `parity.py` against a fresh
`tools/pack_assets.py` reference tree, for all 13 modules:

```
bungee-roulette: CLEAN  chameleon: CLEAN      coming-soon: CLEAN
flying-toilets:  CLEAN  frankenscreen: CLEAN  message-mayhem: CLEAN
mikes-so-called-life: CLEAN                   mime-hunt: CLEAN
mowin-boris: CLEAN      phlegm-boy: CLEAN     shock-clocks: CLEAN
toxic-swamp: CLEAN      voyeur: CLEAN
```

Re-verified 2026-09-19 after `_shared/faceplate.png` and `meta.json`'s
`help` were added, both releases, `_shared` included:
`_shared: CLEAN ok=1` and 13/13 modules CLEAN, `GATE CLEAN`.

Zero FAIL, zero WARN, both releases — and the floppy and CD rips are
byte-identical to each other (`diff -rq` clean), confirming the CD's forks
differ from the floppy's only in reserved header bytes that no lane reads.
Ripping all 13 modules end to end (ingest through pack.build) takes ~37 s
per release on the reference machine. 55 tests pass:
`python3 -m pytest -q tools/twistedrip/tests/` (including
`test_end_to_end.py`, which runs `rip.py` on the real floppy .sit for
bungee-roulette and diffs it against `pack_assets.py` with `parity.py`).

## Oracles for the intermediate stages (private RE checkout, local only)
- `ripped/<module>/` — resource_dasm's per-resource dumps (`<file>_<type>_<id>[_<name>].<ext>`).
- `ripped/shared-twisted-sound/*.midi` — decoded cmid SMFs.
- `ripped/shared-twisted-sound-expanded/*.wav` — sndS-expanded audio.
- `ripped/*/expanded/`, INST JSONs, csnd WAVs — whatever `pack_assets.py` reads.

## Module-derived caches (2026-09-19)
A live `assets/` tree also holds files the MODULES USED TO WRITE at build()
time — voyeur stars/wall rows, shock-clocks hands + LCD digits, toxic-swamp
mirrored `m_*.png` frames, message-mayhem `_pens/` strips. A fresh rip never
contains them and they are not pack content; `parity.py` ignores them on both
sides (`is_module_derived`). Verified 2026-09-19: rip of the floppy .sit vs the
live tree = CLEAN for all 13 modules once those are excluded.

**Those writes are gone as of this commit (2026-09-19).** All four families are
now generated in memory and handed to the compose pass by name
(`engine::GEN_PREFIX`, the `Module::generated` hook) instead of being written
into the pack and named by path. Writing into the pack was a real bug, not just
untidiness: an installed `.saver` bundle is read-only, every write failed
silently, and the module then drew nothing — toxic-swamp's mirrored frames were
the case that surfaced as "the fish swim backwards". A pack directory is now
read-only for the whole life of a run.

The `is_module_derived` ignore is KEPT: an `assets/` tree that an older build
ran against still holds the residue, and the gate should not start failing on
it. A tree built from here on never grows any, so the pattern simply matches
nothing. Nothing deletes the residue from a live tree — that is a separate,
manual call.
