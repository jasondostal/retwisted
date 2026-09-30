#!/usr/bin/env python3
"""Build a runtime asset pack from a totally-twisted RE checkout.

v2: packs ALL series of a module (multi-series modules like Coming Soon /
FrankenScreen need several), plus the module's sounds and the shared
Twisted Sound bank (sndS-expanded WAVs preferred — the raw rips of
30001-30013 are compressed noise).

  assets/<slug>/
    meta.json
    compounds/<base>/c_NNN.png
    sounds/*.wav

Sequence frame: {png, dx, dy, bx, by} — png relative to pack root; dx/dy =
link offset entering the frame; bx/by = compound bounds origin.

meta.json format (v3 — `palettes` became SLOT-INDEXED, `base_clut` added):

  module     RE ripped/ directory name this pack came from
  field      background colour, 8-bit RGB triple
  series     {"<series base>": [ {first, frames: [{png,dx,dy,bx,by,w,h}]} ]}
  baked      {"<series base>": [[r,g,b], ...]} — the RLEP bank's own CTAB,
             DENSE and indexed by pixel value (rlep2png does `ctab[pixel]`).
             Provenance only: it records what the compound PNGs were
             rendered with. It is NOT the runtime-recolour source — see
             `palettes` / `base_clut`.
  palettes   {"<clut id>": {"<slot>": [r,g,b], ...}} — every 'clut' resource
             in the module, keyed by the resource's OWN colour-table slot
             index. Mac cluts address device slots and may be sparse (they
             redefine specific slots, not a contiguous 0..N run), so an
             entry's position in the file is NOT its slot. v2 stored a
             positionally-sorted list and threw the slot away; zipping such
             a list against anything produced garbage recolours.
  base_clut  "<clut id>" | null — the device palette the packed art is baked
             against, i.e. the clut in effect when the compounds were
             rendered. A runtime LoadCLUT(target) means "device slot i now
             holds target[i]", so the engine's colour remap is exactly
             `base_clut[slot] -> target[slot]` for every slot the target
             redefines; a colour whose slot the target does not redefine
             passes through unchanged (that is how sparse cluts work on the
             Mac). null = this pack does no runtime recolour and `palettes`
             is informational only.
  strings    {"<STR# id>": ["...", ...]}
  help       the module's TEXT 1000 blurb (UTF-8, LF) -- the paragraph the
             original control panel printed under the controls. '' when the
             module has none.
  slider_words {"<sUnt id>": [[raw value, "word"], ...]} -- the words the
             control panel printed under each slider.
  menus      {"<MENU id>": ["item", "-", ...]} -- popup items, separators
             kept.

Also written, once per run and NOT per module: assets/_shared/faceplate.png,
the banner that topped the original control panel (Twisted Faceplate PICT
128, with After Dark 3.0's PICT 128 as the fallback). It is collection-wide,
so it lives beside the <slug>/ dirs rather than inside one.

v4 adds `music` (see MUSIC below). It is additive: the engine reads every
new key behind `#[serde(default)]`, so a v3 pack still loads and a v4 pack
still works against an engine that predates it.

  music      {"songs": {...}, "instruments": {...}, "samples": {...}} | absent
             After Dark's MIDI background music, from the shared Twisted
             Sound bank. See MUSIC.

MUSIC — what `cmid` actually is (RE'd 2026-09-13)
-------------------------------------------------
A `cmid` resource is a **SoundMusicSys-compressed standard MIDI file**: a
big-endian u32 decompressed length, then Berkeley/SoundMusicSys LZSS over a
plain SMF. resource_dasm already unwraps it (`ResourceFile::decode_cmid` ->
`decompress_soundmusicsys_data`), and the RE checkout's
`ripped/shared-twisted-sound/Twisted_Sound.rsrc_cmid_<id>.midi` files are
the byte-exact SMFs that come out. There is no Berkeley-private event list
to decode: the four tunes are format-1 SMFs, 480 ticks per quarter, one
`set tempo` meta each, note on/off + program change and nothing else (no
controllers, no pitch bend, no aftertouch).

| cmid | track name      | tracks | tempo      | end     | used by      |
|------|-----------------|--------|------------|---------|--------------|
| 10   | `Mime Hunt`     | 8      | 128.00 bpm | 54.84 s | mime-hunt    |
| 20   | `Coming Soon.5` | 6      | 240.00 bpm | 33.00 s | coming-soon  |
| 30   | `Horror Cue`    | 4      | 100.00 bpm | 31.20 s | frankenscreen|
| 40   | `Dawn Cue`      | 4      |  67.50 bpm | 16.00 s | mowin-boris  |

(`cmid` 4x0 are the same tunes with fewer tracks — reduced-voice variants
for slower Macs, track names prefixed "4". Not packed; the captures match
the full versions.)

They are voiced by the bank's `MDRV` "MIDI Synth 3.43" through its `csnd`
sample resources. The `INST` resources are the patch map, one per GM
program, and resource_dasm has already decoded them to
`Twisted_Sound.rsrc_INST_<program>_<name>.json`: a list of key regions with
`{key_low, key_high, base_note, freq_mult, filename}`. Channel 9 is
percussion and uses `INST` 127 (a key-per-drum map). Every `csnd` WAV is
mono IEEE-float32 at 22254 Hz (the Mac rate) and carries a `smpl` chunk
whose loop points are BYTE offsets, not frame indices — this packer converts
them to frames.

So the pack carries: the SMF bytes, the sample WAVs the songs' programs
need, and a flattened instrument table.

  music.songs       {"<song id>": {file, name, ppq, length_ms, note_ons}}
                    `file` is the SMF, packed verbatim. `length_ms` is the
                    end-of-track, `note_ons` the total note-on count — both
                    are what engine::music's decode test ratchets against.
  music.samples     {"<key>": {file, rate, base_note, loop_start, loop_end}}
                    loop_* are FRAME indices, 0/0 = no loop.
  music.instruments {"<GM program>": [{key_low, key_high, base_note,
                    freq_mult, sample}]} — `sample` keys into music.samples.

Usage: pack_assets.py <re-repo-root> <module-dir> [slug]
       (packs every series that has an OFst directory + compound dir)
"""
import json
import re
import shutil
import subprocess
import struct
import sys
import zlib
from pathlib import Path

FIELDS = {
    # EMPTY as of 2026-09-01, and that is the finding: every module verified
    # against DEPTH=32 golden captures draws on the BLACK After Dark blank
    # (flying-toilets, phlegm-boy, message-mayhem, coming-soon, shock-clocks,
    # mowin-boris, voyeur, bungee, mikes). The original 2026-08-29 "toilets
    # draw on WHITE" finding -- this table's founding entry -- was an
    # artifact of Basilisk II's broken 8-bit palette path, like every other
    # white-field reading from that rig. d32-toilets/g_08: toilets fly on
    # black. Per-module overrides would accumulate here if a verified
    # non-black field ever appears; none has.
    # 2026-09-01 REVERSAL: the 2026-08-30 "white field" findings for
    # phlegm-boy and message-mayhem were artifacts of Basilisk II's broken
    # 8-bit palette path (SDL_SetWindowGammaRamp unsupported; some modules'
    # index-0 rendered WHITE). DEPTH=32 retakes show phlegm-boy,
    # coming-soon, and shock-clocks all on BLACK, matching the After Dark
    # blank. Their entries are removed -- black default stands.
    # coming-soon: NO entry -- black default confirmed. The DEPTH=32 retake
    # (scratchpad d32-coming-soon/g_02..g_11) shows the poster card and the
    # mascot spotlight on BLACK, and the mascot's solid black compound
    # backing (rows 86+ of the 92x140 series-10000 frames) disappears into
    # it exactly as the "authentic black backing" reading predicted. The
    # 8-bit capture that showed a white field was the palette artifact
    # described above.
    # mowin-boris: NO entry -- it keeps the black default. The 2026-08-30
    # (0,85,0) "green lawn" guess is REVERTED: the Basilisk sleep-mode
    # golden capture of 2026-09-01 (scratchpad boris-sleep/g_01..g_15)
    # shows the real module drawing on a BLACK field, with the meadow made
    # of thousands of individually stamped 1x6 grass-blade sprites
    # (compound 5) plus the six flower compounds 10/12/14/16/18/20. That
    # scatter is drawn by the module (see mowin_boris.rs), not by the
    # field, and §6.1's art 94 "solid black mowed patch" reads correctly
    # only against a black field -- it erases stamped blades, it does not
    # cut holes in a flat green clear.
}

# The device palette each pack's art is baked against -> meta.json base_clut.
# A runtime LoadCLUT(target) is "base_clut slot i is now target slot i", so
# this is the ONLY thing that makes a recolour slot-index-correct. Add an
# entry only where the RE spec actually pins the boot palette; a module with
# no entry gets base_clut = null and the engine leaves its colours alone.
BASE_CLUT = {
    # docs/behavior/chameleon.md §2.1/§6.6: the ctor loads clut 1500
    # (ChamGreen) as the boot palette, and the colour-change rolls
    # RandomBelow(9)+1500 over the nine slot-aligned Cham* cluts. All 20 of
    # clut 1500's colours occur in the bank-1000 CTAB (at scattered CTAB
    # positions — the CTAB is the union of all nine schemes, which is why
    # CTAB position is NOT the clut slot).
    'chameleon': '1500',
    # docs/behavior/toxic-swamp.md §4/§6: clut 1200 ("toxy pal") is pushed at
    # init and re-pushed every frame. It is the palette the 9000/9001/9002
    # art is already baked in, so LoadCLUT(1200) is a no-op recolour — the
    # engine resolves base == target to the identity map.
    'toxic-swamp': '1200',
    # coming-soon: deliberately absent. Its per-movie cluts 20000+m / 21000+m
    # are 10-/14-entry accent tables and the posters are per-movie ART (bank
    # 20001+m), already carrying the movie's colours; 172/174 poster colours
    # come from clut 10000 "Final 256 all", which is a SORTED colour list, so
    # its index carries no device-slot meaning. With no recoverable slot
    # correspondence the faithful result is pass-through, not a guess.
}


# After Dark's MIDI background music: which shared-bank `cmid` song(s) each
# module's pack carries. See MUSIC in the module docstring.
#
# The pairing is not a guess. mime-hunt and frankenscreen were already named
# by the disassembly (SONG 10 / SONG 30), and the tunes carry their module in
# their own SMF track-name meta event: cmid 10 is "Mime Hunt", 20 is "Coming
# Soon.5", 30 is "Horror Cue" (frankenscreen.rs's own name for SONG 30), 40
# is "Dawn Cue". The 2026-09-13 QEMU captures then pin 40 to Mowin' Boris at
# 84.6 % note agreement (docs/emulator/audio-captures.md).
#
# mowin-boris is packed but NOT wired: the module has no Music control at all
# and the original's reason for playing SONG 40 anyway is still open (see
# mime_hunt.rs / the audio-captures "Open sub-question"). Packing the asset
# costs ~1 MB of gitignored pack and leaves the boris lane something to wire.
SONGS = {
    'mime-hunt': [10],
    'coming-soon': [20],
    'frankenscreen': [30],
    'mowin-boris': [40],
}

SHARED_SOUND = 'shared-twisted-sound'


def smf_scan(data):
    """Minimal SMF reader: returns (ppq, length_ms, note_ons, programs, name).

    Only what the pack needs — the engine has its own full decoder. Handles
    running status, meta and sysex events. `programs` is the set of GM
    program numbers any Program Change selects, plus 127 whenever channel 9
    (percussion) is played, because the drum map is INST 127 and no Program
    Change ever names it."""
    if data[:4] != b'MThd':
        raise ValueError('not an SMF')
    hlen = struct.unpack_from('>I', data, 4)[0]
    _fmt, ntrk, div = struct.unpack_from('>HHH', data, 8)
    if div & 0x8000:
        raise ValueError('SMTPE division not supported')
    o = 8 + hlen
    # (tick, us_per_quarter). Most Twisted tunes carry exactly one, but
    # Coming Soon's own "Rhumba Data" changes tempo at tick 23040, and
    # collapsing that to a single tempo mis-measures the tune by 10 s.
    tempos = []
    max_tick = 0
    note_ons = 0
    programs = set()
    name = ''
    for _ in range(ntrk):
        if data[o:o + 4] != b'MTrk':
            raise ValueError('bad track header')
        tlen = struct.unpack_from('>I', data, o + 4)[0]
        body = data[o + 8:o + 8 + tlen]
        o += 8 + tlen
        p, t, running = 0, 0, None
        while p < len(body):
            dt = 0
            while True:
                b = body[p]; p += 1
                dt = (dt << 7) | (b & 0x7F)
                if not b & 0x80:
                    break
            t += dt
            st = body[p]
            if st < 0x80:
                st = running
            else:
                p += 1
                if st < 0xF0:
                    running = st
            if st == 0xFF:
                mt = body[p]; p += 1
                ln = 0
                while True:
                    b = body[p]; p += 1
                    ln = (ln << 7) | (b & 0x7F)
                    if not b & 0x80:
                        break
                payload = body[p:p + ln]; p += ln
                if mt == 0x51 and ln == 3:
                    tempos.append((t, int.from_bytes(payload, 'big')))
                elif mt == 0x03 and not name:
                    name = payload.decode('mac-roman', 'replace')
            elif st in (0xF0, 0xF7):
                ln = 0
                while True:
                    b = body[p]; p += 1
                    ln = (ln << 7) | (b & 0x7F)
                    if not b & 0x80:
                        break
                p += ln
            else:
                hi, ch = st & 0xF0, st & 0x0F
                n = 1 if hi in (0xC0, 0xD0) else 2
                args = body[p:p + n]; p += n
                if hi == 0x90 and args[1] > 0:
                    note_ons += 1
                    if ch == 9:
                        programs.add(127)
                elif hi == 0xC0 and ch != 9:
                    programs.add(args[0])
        max_tick = max(max_tick, t)
    tempos.sort()
    ms, prev, us = 0.0, 0, 500000.0
    for tt, tu in tempos:
        if tt >= max_tick:
            break
        ms += (tt - prev) / div * us / 1000.0
        prev, us = tt, tu
    ms += (max_tick - prev) / div * us / 1000.0
    return div, round(ms), note_ons, programs, name


def wav_sample_meta(path):
    """(rate, base_note, loop_start, loop_end) for a resource_dasm csnd WAV.

    The `smpl` chunk's loop points are BYTE offsets into the data chunk
    (WAVFile.cc: "Start and end are byte offsets into the wave data, not
    sample indexes"), so they are divided back down by the frame size here —
    read them raw and every sustained instrument loops at 4x its real point."""
    d = path.read_bytes()
    rate, bits, chans = 22254, 32, 1
    base, ls, le = 60, 0, 0
    o = 12
    while o + 8 <= len(d):
        cid = d[o:o + 4]
        sz = struct.unpack_from('<I', d, o + 4)[0]
        body = d[o + 8:o + 8 + sz]
        if cid == b'fmt ':
            _f, chans, rate, _br, _ba, bits = struct.unpack_from('<HHIIHH', body)
        elif cid == b'smpl' and len(body) >= 36:
            base = struct.unpack_from('<I', body, 12)[0]
            nloops = struct.unpack_from('<I', body, 28)[0]
            if nloops >= 1 and len(body) >= 60:
                ls, le = struct.unpack_from('<II', body, 36 + 8)
        o += 8 + sz + (sz & 1)
    frame = max(1, (bits // 8) * max(1, chans))
    return rate, base, ls // frame, le // frame


def pack_music(re_root, slug, out):
    """Export the module's song(s), the instruments they need and those
    instruments' samples. Returns the meta.json `music` section or None."""
    ids = SONGS.get(slug)
    if not ids:
        return None
    shared = re_root / 'ripped' / SHARED_SOUND
    mdir = out / 'music'
    sdir = mdir / 'samples'
    songs, programs = {}, set()
    for sid in ids:
        src = shared / f'Twisted_Sound.rsrc_cmid_{sid}.midi'
        if not src.exists():
            print(f'  WARNING: no cmid {sid} in {SHARED_SOUND}/ — music skipped')
            continue
        data = src.read_bytes()
        ppq, length_ms, note_ons, progs, name = smf_scan(data)
        sdir.mkdir(parents=True, exist_ok=True)
        (mdir / f'song_{sid}.mid').write_bytes(data)
        songs[str(sid)] = {'file': f'music/song_{sid}.mid', 'name': name,
                           'ppq': ppq, 'length_ms': length_ms, 'note_ons': note_ons}
        programs |= progs
        print(f'  song {sid} "{name}": {length_ms / 1000:.2f} s, '
              f'{note_ons} note-ons, programs {sorted(progs)}')
    if not songs:
        return None

    # The INST resources resource_dasm decoded, one JSON per GM program.
    inst_by_prog = {}
    for f in shared.glob('*_INST_*.json'):
        m = re.search(r'_INST_(\d+)_', f.name)
        if m:
            inst_by_prog[int(m.group(1))] = f

    instruments, samples = {}, {}
    for prog in sorted(programs):
        f = inst_by_prog.get(prog)
        if f is None:
            print(f'  WARNING: song wants GM program {prog}, no INST resource')
            continue
        regions = []
        for r in json.loads(f.read_text())['regions']:
            wav = shared / r['filename']
            if not wav.exists():
                print(f"  WARNING: INST {prog} wants missing {r['filename']}")
                continue
            # csnd 3201 / snd 12742 -> "csnd_3201" / "snd_12742": the sample
            # key must not be the raw filename (spaces, sharps, trailing
            # blanks — 'piano C4  .wav' really does end in two spaces).
            m = re.search(r'_((?:c)?snd)_(\d+)[_.]', r['filename'])
            key = f'{m.group(1)}_{m.group(2)}' if m else wav.stem
            if key not in samples:
                rate, wbase, ls, le = wav_sample_meta(wav)
                shutil.copy(wav, sdir / f'{key}.wav')
                samples[key] = {'file': f'music/samples/{key}.wav', 'rate': rate,
                                'base_note': wbase, 'loop_start': ls, 'loop_end': le}
            regions.append({'key_low': r['key_low'], 'key_high': r['key_high'],
                            'base_note': r['base_note'],
                            'freq_mult': r.get('freq_mult', 1.0), 'sample': key})
        if regions:
            instruments[str(prog)] = regions
    print(f'  music: {len(songs)} songs, {len(instruments)} instruments, '
          f'{len(samples)} samples')
    return {'songs': songs, 'instruments': instruments, 'samples': samples}


def parse_clut_bin(path):
    """Mac 'clut' resource: u32 seed, u16 flags, u16 ctSize, then
    (ctSize+1) colorSpecs of {u16 value, 3×u16 rgb}. Returns
    {index: (r,g,b) 8-bit}."""
    d = path.read_bytes()
    if len(d) < 8:
        return {}
    _seed, _flags, ct_size = struct.unpack_from('>IHH', d)
    out = {}
    for i in range(ct_size + 1):
        off = 8 + i * 8
        if off + 8 > len(d):
            break
        v, r, g, b = struct.unpack_from('>HHHH', d, off)
        out[v] = (r >> 8, g >> 8, b >> 8)
    return out


def parse_rlep_ctab(path):
    """The bank's own CTAB (what rlep2png bakes compound PNGs against).
    Reuses the RE repo's rlep2png parser."""
    import rlep2png
    data = path.read_bytes()
    for tag, param, flags, payload in rlep2png.chunks(data):
        if tag == b'CTAB':
            return rlep2png.parse_ctab(param, payload)
    return []


def pack_str_strings(mdir):
    """STR# resources. Two filename shapes occur:
      <stem>_STR#_<id>_<name>_<idx>.txt   (named list)
      <stem>_STR#_<id>_<idx>.txt          (bare list, e.g. the 1_xx run)
    → {id: [strings in idx order]} (ids kept as strings; JSON keys)."""
    groups = {}
    for f in mdir.glob('*_STR#_*.txt'):
        m = re.search(r'_STR#_(\d+)_(?:(.+)_(\d+)|(\d+))\.txt$', f.name)
        if not m:
            continue
        sid = int(m.group(1))
        idx = int(m.group(3) or m.group(4))
        groups.setdefault(sid, []).append((idx, f.read_text(errors='replace')))
    out = {}
    for sid, items in groups.items():
        items.sort()
        out[str(sid)] = [s for _, s in items]
    return out


def pack_slider_words(mdir):
    """sUnt resources (resource_dasm dumps them raw: <stem>_sUnt_<id>[_<name>].bin)
    → {id: [[value, word], ...]}. Layout: u16 count, then rows of i16 value +
    Pascal string, unpadded."""
    out = {}
    for f in mdir.glob('*_sUnt_*.bin'):
        m = re.search(r'_sUnt_(\d+)(?:_.*)?\.bin$', f.name)
        if not m:
            continue
        d = f.read_bytes()
        rows, o = [], 2
        for _ in range(int.from_bytes(d[0:2], 'big') if len(d) >= 2 else 0):
            if o + 3 > len(d):
                break
            v = int.from_bytes(d[o:o + 2], 'big', signed=True)
            n = d[o + 2]
            rows.append([v, d[o + 3:o + 3 + n].decode('mac-roman', 'replace')])
            o += 3 + n
        out[m.group(1)] = rows
    return out


def pack_menus(mdir):
    """MENU resources (resource_dasm dumps them as text: <stem>_MENU_<id>[_<name>].txt,
    one `Name: '<item>'` line per item) → {id: [items]}, '-' separators kept."""
    out = {}
    for f in mdir.glob('*_MENU_*.txt'):
        m = re.search(r'_MENU_(\d+)(?:_.*)?\.txt$', f.name)
        if not m:
            continue
        out[m.group(1)] = re.findall(r"^    Name: '(.*)'$", f.read_text(errors='replace'), re.M)
    return out


def write_png(w, h, rgba_rows):
    """8-bit RGBA PNG, no filtering, zlib level 9.

    Byte-for-byte the same writer as tools/twistedrip/decode/rlep.py's
    write_png -- deliberately duplicated rather than imported, because this
    script is the parity gate's REFERENCE implementation and must not share
    code with the thing it is checking. parity.py catches any drift: the
    faceplate PNG has to come out byte-identical from both packers.
    """
    def chunk(tag, payload):
        c = tag + payload
        return struct.pack('>I', len(payload)) + c + struct.pack('>I', zlib.crc32(c))
    raw = b''.join(b'\x00' + row for row in rgba_rows)
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9))
            + chunk(b'IEND', b''))


def bmp_rgba(path):
    """resource_dasm's 32-bit PICT dump -> (w, h, rgba rows, top row first).

    resource_dasm writes an uncompressed BI_RGB BMP whose 32-bit pixels are
    stored R,G,B,A in that order (NOT the BGRA a Windows BMP would use) and
    whose rows run bottom-up when the header height is positive. Verified
    against tools/twistedrip/decode/pict.py decoding the same resource's
    raw PICT bytes: pixel-identical for both faceplates.
    """
    data = path.read_bytes()
    off = struct.unpack_from('<I', data, 10)[0]
    w, h = struct.unpack_from('<ii', data, 18)
    bpp = struct.unpack_from('<H', data, 28)[0]
    if bpp != 32:
        raise ValueError(f'{path.name}: {bpp}-bit BMP, expected 32')
    stride, px = w * 4, data[off:]
    rows = []
    for y in range(abs(h)):
        sy = (abs(h) - 1 - y) if h > 0 else y
        rows.append(px[sy * stride:(sy + 1) * stride])
    return w, abs(h), rows


# The faceplate banner that topped the original After Dark control panel:
# the "Twisted Faceplate" file's PICT 128 ("Mac Faceplate.PICT", 185x48),
# with After Dark 3.0's own PICT 128 "Default Faceplate" (185x47) as the
# fallback. Collection-wide, not per-module, so it is written once into
# assets/_shared/ next to the <slug>/ dirs. See tools/twistedrip/pack.py's
# build_faceplate, which decodes the same resource straight from the PICT.
FACEPLATE_DUMPS = (
    ('twisted-faceplate', '*_PICT_128_*.bmp'),
    ('after-dark-3.0', '*_PICT_128_*.bmp'),
)
SHARED_DIR = '_shared'


def pack_faceplate(re_root, assets_root):
    """Write assets/_shared/faceplate.png. Returns the path, or None."""
    for sub, pattern in FACEPLATE_DUMPS:
        src = next((re_root / 'ripped' / sub).glob(pattern), None) \
            if (re_root / 'ripped' / sub).is_dir() else None
        if src is None:
            continue
        w, h, rows = bmp_rgba(src)
        out = assets_root / SHARED_DIR
        out.mkdir(parents=True, exist_ok=True)
        path = out / 'faceplate.png'
        path.write_bytes(write_png(w, h, rows))
        return path
    return None


def pack_help(mdir, stem):
    """The module's TEXT 1000 blurb -- what the original control panel
    printed under the controls. resource_dasm dumps it as UTF-8 with CR
    already turned into LF, which is exactly what the pack wants (and what
    twistedrip's build_help produces from the raw Mac Roman resource)."""
    f = mdir / f'{stem}_TEXT_1000.txt'
    return f.read_text(encoding='utf-8', errors='replace') if f.is_file() else ''


def png_size(path):
    """(w, h) from a PNG's IHDR — the compound's authored bounds size, which
    the engine needs for GetBounds-style metrics (Phlegm Boy's +0xDE/+0xE0)."""
    with open(path, 'rb') as f:
        head = f.read(24)
    if head[:8] != b'\x89PNG\r\n\x1a\n' or head[12:16] != b'IHDR':
        return 0, 0
    return int.from_bytes(head[16:20], 'big'), int.from_bytes(head[20:24], 'big')


def pack_bank_frames(mdir, stem, base, out):
    """A bank with no OFst (e.g. chameleon's RLEP 10042 overlay art):
    decode each frame with rlep2png and pack as single-frame sequences
    keyed by frame id."""
    import rlep2png
    rlep = next(mdir.glob(f'{stem}_RLEP_{base}.bin'), None)
    if rlep is None:
        return None
    ctab, frames = rlep2png.parse_bank(rlep.read_bytes())
    if not frames:
        return None
    cdir = out / 'compounds' / str(base)
    cdir.mkdir(parents=True, exist_ok=True)
    seqs = []
    for fid, (rect, stream) in sorted(frames.items()):
        rows = rlep2png.decode_rows(stream)
        name = f'f_{fid:03d}.png'
        if rlep2png.render(rows, ctab, cdir / name, rect) is None:
            continue
        w, h = png_size(cdir / name)
        seqs.append({'first': fid, 'frames': [
            {'png': f'compounds/{base}/{name}', 'dx': 0, 'dy': 0, 'bx': 0, 'by': 0,
             'w': w, 'h': h}
        ]})
    return seqs


def pack_series(mdir, stem, base, comp_dir, out, parse_ofst, parse_oftb_entry):
    ofst = (mdir / f'{stem}_OFst_{base}_MMOffsets.bin').read_bytes()
    (_max_w, _max_h), recs = parse_ofst(ofst)
    oftbs = {int(re.search(r'_OFtb_(\d+)', f.name).group(1)): f.read_bytes()
             for f in mdir.glob(f'{stem}_OFtb_*.bin')}

    seqs, run = [], []
    for fno, off, dx, dy in sorted(recs):
        if run and fno != run[-1][0] + 1:
            seqs.append(run)
            run = []
        run.append((fno, off, dx, dy))
    if run:
        seqs.append(run)

    cdir = out / 'compounds' / str(base)
    cdir.mkdir(parents=True, exist_ok=True)
    used, out_seqs = set(), []
    for run in seqs:
        frames = []
        for fno, off, dx, dy in run:
            src = comp_dir / f'compound_{fno:03d}.png'
            tb = oftbs.get(base + (off >> 16))
            if not src.exists() or tb is None:
                continue
            (bl, bt, _, _), items = parse_oftb_entry(tb, off & 0xFFFF)
            # the compound's part table: (art, channel, flip bits, l, t, r, b)
            # in bank space. Library 4.0 links sequences by a shared part
            # (Sprite fn3F2E) and modules address parts by channel
            # (Boris's blade is channel 2 of the mower compound).
            parts = [[art, chan, fl, r[0], r[1], r[2], r[3]] for art, chan, fl, r in items]
            name = f'c_{fno:03d}.png'
            if fno not in used:
                shutil.copy(src, cdir / name)
                used.add(fno)
            w, h = png_size(src)
            frames.append({'png': f'compounds/{base}/{name}',
                           'dx': dx, 'dy': dy, 'bx': bl, 'by': bt, 'w': w, 'h': h,
                           'parts': parts})
        if frames:
            out_seqs.append({'first': run[0][0], 'frames': frames})
    return out_seqs, len(used)


def main(re_root, module, slug=None):
    re_root = Path(re_root).expanduser()
    sys.path.insert(0, str(re_root / 'scripts'))
    from oftb_compose import parse_ofst, parse_oftb_entry  # noqa: E402

    slug = slug or module.replace("'", '').replace(' ', '-')
    mdir = re_root / 'ripped' / module
    stem = next(mdir.glob('*_RLEP_*.bin')).name.split('_RLEP_')[0]

    out = Path(__file__).resolve().parent.parent / 'assets' / slug
    shutil.rmtree(out, ignore_errors=True)
    out.mkdir(parents=True)

    series = {}
    total = 0
    baked = {}
    for f in sorted(mdir.glob(f'{stem}_OFst_*_MMOffsets.bin')):
        base = int(re.search(r'_OFst_(\d+)_', f.name).group(1))
        comp_dir = mdir / f'compound-{base}'
        if not comp_dir.is_dir():
            print(f'  skip series {base}: no {comp_dir.name}/ (run oftb_compose.py)')
            continue
        out_seqs, n = pack_series(mdir, stem, base, comp_dir, out,
                                  parse_ofst, parse_oftb_entry)
        series[str(base)] = out_seqs
        total += n
        print(f'  series {base}: {len(out_seqs)} sequences, {n} compounds')

    # Banks with no OFst (separate id spaces like chameleon's RLEP 10042
    # tongue/fly overlays) pack as single-frame sequences keyed by frame id.
    packed_bases = {int(b) for b in series}
    for f in sorted(mdir.glob(f'{stem}_RLEP_*.bin')):
        base = int(re.search(r'_RLEP_(\d+)\.bin', f.name).group(1))
        if base in packed_bases:
            continue
        seqs = pack_bank_frames(mdir, stem, base, out)
        if seqs:
            series[str(base)] = seqs
            print(f'  bank {base}: {len(seqs)} raw frames (no OFst)')

    # Baked palette per series (the RLEP bank's own CTAB, dense, indexed by
    # pixel value) + every clut resource keyed by its real slot index, so the
    # engine can do runtime LoadCLUT recolours (chameleon §2.1/§6.6 — the
    # frames are palette-indexed, recolour is in-place). NOTE: a bank CTAB
    # position is a PIXEL index, not a device-palette slot, so `baked` must
    # never be zipped against a clut; the base_clut mapping below is the only
    # slot-correct route. See the meta.json format note at the top.
    for base_str, seqs in series.items():
        rlep = next(mdir.glob(f'{stem}_RLEP_{base_str}.bin'), None)
        if rlep:
            ctab = parse_rlep_ctab(rlep)
            if ctab:
                baked[base_str] = [list(c) for c in ctab]
    palettes = {}
    for f in sorted(mdir.glob('*_clut_*.bin')):
        m = re.search(r'_clut_(\d+)_', f.name)
        if not m:
            continue
        cl = parse_clut_bin(f)
        if len(cl) >= 2:
            # Keep the resource's REAL colour-table slot index. Mac cluts are
            # slot-addressed and may be sparse, so a positional list is a lie.
            palettes[m.group(1)] = {str(k): list(v) for k, v in sorted(cl.items())}
    base_clut = BASE_CLUT.get(slug)
    print(f'  palettes: {len(palettes)} cluts, baked: {len(baked)} series, '
          f'base_clut: {base_clut}')
    if base_clut is not None:
        base = palettes.get(base_clut)
        if base is None:
            print(f'  WARNING: base clut {base_clut} is not in this module')
        else:
            # The base clut must be the palette the art is drawn in, so every
            # one of its colours should occur in some bank CTAB; and its
            # colours must be distinct, or the RGB->slot reverse lookup the
            # engine does is ambiguous.
            baked_cols = {tuple(c) for tbl in baked.values() for c in tbl}
            missing = [s for s, c in sorted(base.items(), key=lambda kv: int(kv[0]))
                       if tuple(c) not in baked_cols]
            if missing:
                print(f'  WARNING: base clut {base_clut} slots {missing} '
                      f'appear in no bank CTAB')
            seen = {}
            for s, c in sorted(base.items(), key=lambda kv: int(kv[0])):
                seen.setdefault(tuple(c), []).append(s)
            dups = {c: s for c, s in seen.items() if len(s) > 1}
            if dups:
                print(f'  WARNING: base clut {base_clut} repeats colours '
                      f'{dups} — lowest slot wins in the engine remap')

    sdir = out / 'sounds'
    sdir.mkdir(exist_ok=True)
    # sndS-EXPANDED module sounds first — most module snds are Berkeley
    # delta-compressed and the raw rips are literal static (learned the
    # hard way, 2026-08-29: the moo, the flaps, the CRAIGIFY scream...)
    for w in (mdir / 'expanded').glob('snd_*.wav'):
        m = re.search(r'snd_(\d+)_', w.name)
        if m:
            shutil.copy(w, sdir / f'{m.group(1)}.wav')
    for w in mdir.glob('*_snd_*.wav'):
        m = re.search(r'_snd_(\d+)_(.+)\.wav$', w.name)
        if m and not (sdir / f'{m.group(1)}.wav').exists():
            shutil.copy(w, sdir / f'{m.group(1)}.wav')
    shared_exp = re_root / 'ripped' / 'shared-twisted-sound-expanded'
    for w in shared_exp.glob('snd_*.wav'):
        m = re.search(r'snd_(\d+)_', w.name)
        if m:
            shutil.copy(w, sdir / f'{m.group(1)}.wav')
    shared_raw = re_root / 'ripped' / 'shared-twisted-sound'
    for w in shared_raw.glob('*_snd_*.wav'):
        m = re.search(r'_snd_(\d+)_(.+)\.wav$', w.name)
        if m and not (sdir / f'{m.group(1)}.wav').exists():
            shutil.copy(w, sdir / f'{m.group(1)}.wav')

    # message-mayhem carries the 'Pens' 500 vector stroke font (Berkeley-
    # derived, so it lives in the gitignored pack, never in repo source).
    # The extractor is the RE repo's scripts/pens_extract.py; its SRC path
    # is baked relative to that repo, so run it from there.
    if slug == 'message-mayhem':
        pens = re_root / 'scripts' / 'pens_extract.py'
        if pens.exists():
            subprocess.run(
                [sys.executable, str(pens), str((out / 'pens500.json').resolve())],
                cwd=re_root, check=True, stdout=subprocess.DEVNULL)
        else:
            print(f'WARN: {pens} missing — pack has no pens500.json '
                  f'(stroke writer will fall back to compound glyphs)')

    music = pack_music(re_root, slug, out)

    meta = {
        'module': module,
        'field': FIELDS.get(slug, [0, 0, 0]),
        'series': series,
        'palettes': palettes,
        'base_clut': base_clut,
        'baked': baked,
        'strings': pack_str_strings(mdir),
        'help': pack_help(mdir, stem),
        'slider_words': pack_slider_words(mdir),
        'menus': pack_menus(mdir),
    }
    if music is not None:
        meta['music'] = music
    (out / 'meta.json').write_text(json.dumps(meta, indent=1))
    face = pack_faceplate(re_root, out.parent)
    print(f'  faceplate: {face}' if face else '  WARNING: no faceplate PICT dump found')
    nsnd = len(list(sdir.glob('*.wav')))
    print(f'{slug}: {len(series)} series, {total} compounds, {nsnd} sounds -> {out}')


if __name__ == '__main__':
    main(*sys.argv[1:4])
