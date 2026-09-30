"""Build assets/<slug>/ (meta.json v4, compounds/, sounds/, pens500.json) from
parsed resources, plus the collection-wide assets/_shared/faceplate.png. Must produce exactly what tools/pack_assets.py produces
today -- that script is the reference; its per-module tables (FIELDS,
BASE_CLUT, SONGS) and its music/meta assembly logic move here verbatim.

`build()` takes an extra `module_name` parameter beyond the stub sketch in
docs/ripper.md's pipeline table: the module's own Mac file name (e.g.
"Bungee Roulette", "Mowin' Boris"), exactly as ingest.py keys its output
dict. It is needed for two things a resource dict alone can't supply:
meta.json's `module` field (pack_assets.py sets it to the RE checkout's
`ripped/<name>` directory name, i.e. `module_name.lower().replace(' ', '-')`
-- apostrophes kept, verified against the live assets/ packs for
mowin'-boris and mike's-so-called-life) and decode.oftb.SHADOW_CHANNELS,
which is keyed by that same display name (Coming Soon's drop-shadow rule).
pack.py is entirely lane 1's file, so this is a local interface decision,
not a change to a fixed cross-lane stub.
"""
import json
import re
import struct
from pathlib import Path

from .rsrc import Resource
from .decode import rlep, oftb, pens, clut, pict
from . import snd, cmid, inst

ResMap = dict[tuple[str, int], "Resource"]


# ---------------------------------------------------------------------------
# Per-module tables, ported verbatim from tools/pack_assets.py.
# ---------------------------------------------------------------------------

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
    # field, and SS6.1's art 94 "solid black mowed patch" reads correctly
    # only against a black field -- it erases stamped blades, it does not
    # cut holes in a flat green clear.
}

# The device palette each pack's art is baked against -> meta.json base_clut.
# A runtime LoadCLUT(target) is "base_clut slot i is now target slot i", so
# this is the ONLY thing that makes a recolour slot-index-correct. Add an
# entry only where the RE spec actually pins the boot palette; a module with
# no entry gets base_clut = null and the engine leaves its colours alone.
BASE_CLUT = {
    # docs/behavior/chameleon.md SS2.1/SS6.6: the ctor loads clut 1500
    # (ChamGreen) as the boot palette, and the colour-change rolls
    # RandomBelow(9)+1500 over the nine slot-aligned Cham* cluts. All 20 of
    # clut 1500's colours occur in the bank-1000 CTAB (at scattered CTAB
    # positions -- the CTAB is the union of all nine schemes, which is why
    # CTAB position is NOT the clut slot).
    'chameleon': '1500',
    # docs/behavior/toxic-swamp.md SS4/SS6: clut 1200 ("toxy pal") is pushed at
    # init and re-pushed every frame. It is the palette the 9000/9001/9002
    # art is already baked in, so LoadCLUT(1200) is a no-op recolour -- the
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
# module's pack carries. See MUSIC in tools/pack_assets.py's module docstring.
SONGS = {
    'mime-hunt': [10],
    'coming-soon': [20],
    'frankenscreen': [30],
    'mowin-boris': [40],
}


# ---------------------------------------------------------------------------
# SMF scanning + csnd WAV metadata, ported verbatim from tools/pack_assets.py.
# ---------------------------------------------------------------------------

def smf_scan(data: bytes):
    """Minimal SMF reader: returns (ppq, length_ms, note_ons, programs, name).

    Only what the pack needs -- the engine has its own full decoder. Handles
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


def wav_sample_meta(wav: bytes):
    """(rate, base_note, loop_start, loop_end) for a csnd WAV (see inst.csnd_to_wav).

    The `smpl` chunk's loop points are BYTE offsets into the data chunk, so
    they are divided back down by the frame size here -- read them raw and
    every sustained instrument loops at 4x its real point."""
    rate, bits, chans = 22254, 32, 1
    base, ls, le = 60, 0, 0
    o = 12
    while o + 8 <= len(wav):
        cid = wav[o:o + 4]
        sz = struct.unpack_from('<I', wav, o + 4)[0]
        body = wav[o + 8:o + 8 + sz]
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


# ---------------------------------------------------------------------------
# STR# resource decoding (generic Mac format, not Berkeley-specific; no
# string content is ever hardcoded here -- it comes from the caller's bytes).
# ---------------------------------------------------------------------------

def _parse_str_list(data: bytes) -> list[str]:
    """'STR#' resource bytes -> list[str] in resource order.
    Format: u16 count, then `count` Pascal strings (u8 length + bytes)."""
    if len(data) < 2:
        return []
    (count,) = struct.unpack_from('>H', data, 0)
    out, o = [], 2
    for _ in range(count):
        if o >= len(data):
            break
        ln = data[o]; o += 1
        out.append(data[o:o + ln].decode('mac-roman', 'replace'))
        o += ln
    return out


def build_strings(module: ResMap) -> dict:
    return {str(id): _parse_str_list(r.data)
            for (t, id), r in module.items() if t == 'STR#'}


# ---------------------------------------------------------------------------
# Control-panel words: 'sUnt' (slider tick words) and 'MENU' (popup items).
# Generic resource layouts; the words themselves only ever come from the
# caller's bytes and land in the gitignored pack.
# ---------------------------------------------------------------------------

def _parse_sunt(data: bytes) -> list[list]:
    """'sUnt' bytes -> [[raw value, word], ...] in resource order.
    Format: u16 count, then `count` rows of (i16 value, Pascal string),
    packed with no alignment padding."""
    if len(data) < 2:
        return []
    (count,) = struct.unpack_from('>H', data, 0)
    out, o = [], 2
    for _ in range(count):
        if o + 3 > len(data):
            break
        (value,) = struct.unpack_from('>h', data, o)
        ln = data[o + 2]
        o += 3
        out.append([value, data[o:o + ln].decode('mac-roman', 'replace')])
        o += ln
    return out


def build_slider_words(module: ResMap) -> dict:
    return {str(id): _parse_sunt(r.data)
            for (t, id), r in module.items() if t == 'sUnt'}


def _parse_menu(data: bytes) -> list[str]:
    """'MENU' bytes -> item texts in order ('-' separators kept verbatim).
    Format: menuID, width, height, procID, filler (5 x i16), enable flags
    (u32), title (Pascal string), then items -- Pascal string + 4 bytes
    (icon, key, mark, style) each -- up to a zero length byte."""
    if len(data) < 15:
        return []
    o = 15 + data[14]
    out = []
    while o < len(data) and data[o] != 0:
        ln = data[o]
        out.append(data[o + 1:o + 1 + ln].decode('mac-roman', 'replace'))
        o += 1 + ln + 4
    return out


def build_menus(module: ResMap) -> dict:
    return {str(id): _parse_menu(r.data)
            for (t, id), r in module.items() if t == 'MENU'}


# ---------------------------------------------------------------------------
# Art: series (OFst/OFtb-composed compounds, or raw RLEP banks with no OFst)
# + baked per-series palettes.
# ---------------------------------------------------------------------------

def png_size(png: bytes):
    if png[:8] != b'\x89PNG\r\n\x1a\n' or png[12:16] != b'IHDR':
        return 0, 0
    return int.from_bytes(png[16:20], 'big'), int.from_bytes(png[20:24], 'big')


def _pack_bank_frames(rlep_by_id: dict[int, bytes], base: int, out: Path):
    """A bank with no OFst (e.g. chameleon's RLEP 10042 overlay art):
    decode each frame with decode.rlep and pack as single-frame sequences
    keyed by frame id."""
    data = rlep_by_id.get(base)
    if data is None:
        return None
    ctab, frames = rlep.parse_bank(data)
    if not frames:
        return None
    cdir = out / 'compounds' / str(base)
    cdir.mkdir(parents=True, exist_ok=True)
    seqs = []
    for fid, (rect, stream) in sorted(frames.items()):
        rows = rlep.decode_rows(stream)
        rendered = rlep.render(rows, ctab, rect)
        if rendered is None:
            continue
        w, h, png = rendered
        name = f'f_{fid:03d}.png'
        (cdir / name).write_bytes(png)
        seqs.append({'first': fid, 'frames': [
            {'png': f'compounds/{base}/{name}', 'dx': 0, 'dy': 0, 'bx': 0, 'by': 0,
             'w': w, 'h': h}
        ]})
    return seqs


def _pack_series(rlep_by_id: dict[int, bytes], oftb_by_id: dict[int, bytes],
                  base: int, ofst_data: bytes, out: Path, module_name: str):
    (_max_w, _max_h), recs = oftb.parse_ofst(ofst_data)
    art = oftb.load_art_chain(rlep_by_id, base)
    shadow_ch = oftb.SHADOW_CHANNELS.get(module_name, frozenset())

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
    composed: dict[int, tuple] = {}  # fno -> (name, w, h, parts)
    out_seqs = []
    warnings = []
    for run in seqs:
        frames = []
        for fno, off, dx, dy in run:
            tb = oftb_by_id.get(base + (off >> 16))
            if tb is None:
                continue
            if fno not in composed:
                bounds, items, w, h, png, warn = oftb.compose_compound(
                    art, tb, off & 0xFFFF, shadow_ch)
                warnings.extend(f'compound {fno}: {w}' for w in warn)
                if png is None:
                    continue
                bl, bt, _br, _bb = bounds
                name = f'c_{fno:03d}.png'
                (cdir / name).write_bytes(png)
                parts = [[art_no, chan, fl, r[0], r[1], r[2], r[3]]
                         for art_no, chan, fl, r in items]
                composed[fno] = (name, bl, bt, w, h, parts)
            if fno not in composed:
                continue
            name, bl, bt, w, h, parts = composed[fno]
            frames.append({'png': f'compounds/{base}/{name}',
                           'dx': dx, 'dy': dy, 'bx': bl, 'by': bt, 'w': w, 'h': h,
                           'parts': parts})
        if frames:
            out_seqs.append({'first': run[0][0], 'frames': frames})
    return out_seqs, len(composed), warnings


def build_art(slug: str, module_name: str, module: ResMap, out: Path):
    """Build compounds/ + series + baked. Returns (series, baked, warnings)."""
    rlep_by_id = {id: r.data for (t, id), r in module.items() if t == 'RLEP'}
    oftb_by_id = {id: r.data for (t, id), r in module.items() if t == 'OFtb'}
    ofst_by_id = {id: r.data for (t, id), r in module.items() if t == 'OFst'}

    series: dict[str, list] = {}
    warnings: list[str] = []
    for base in sorted(ofst_by_id):
        out_seqs, _n, warn = _pack_series(
            rlep_by_id, oftb_by_id, base, ofst_by_id[base], out, module_name)
        series[str(base)] = out_seqs
        warnings.extend(warn)

    packed_bases = set(ofst_by_id)
    for base in sorted(rlep_by_id):
        if base in packed_bases:
            continue
        seqs = _pack_bank_frames(rlep_by_id, base, out)
        if seqs:
            series[str(base)] = seqs

    baked: dict[str, list] = {}
    for base_str in series:
        data = rlep_by_id.get(int(base_str))
        if data is not None:
            ctab, _frames = rlep.parse_bank(data)
            if ctab:
                baked[base_str] = [list(c) for c in ctab]

    return series, baked, warnings


def build_palettes(module: ResMap):
    """Every NAMED 'clut' resource keyed by its real slot index (see
    decode/clut.py). Matches tools/pack_assets.py's parse_clut_bin glob
    exactly: it globs resource_dasm's dumped filenames with regex
    `_clut_(\\d+)_`, which only matches a resource_dasm dump that has a
    "_<name>" suffix -- i.e. a nameless clut resource (like bungee-roulette's
    304) is silently skipped. That is a filename-pattern accident, not a
    deliberate exclusion, but the parity gate is against pack_assets.py's
    actual output, so it is reproduced here rather than "fixed"."""
    palettes = {}
    for (t, id), r in module.items():
        if t != 'clut' or not r.name:
            continue
        cl = clut.parse(r.data)
        if len(cl) >= 2:
            palettes[str(id)] = {str(k): list(v) for k, v in sorted(cl.items())}
    return palettes


# ---------------------------------------------------------------------------
# Sounds: module + shared-bank 'snd ' (with optional 'sndS' delta companion).
# Depends on lane 2's snd.to_wav; raises NotImplementedError until it lands.
# ---------------------------------------------------------------------------

def build_sounds(module: ResMap, shared_sound: ResMap, out: Path) -> int:
    sdir = out / 'sounds'
    sdir.mkdir(parents=True, exist_ok=True)
    written = set()
    for bank in (module, shared_sound):
        for (t, id), r in bank.items():
            if t != 'snd ' or id in written:
                continue
            companion = bank.get(('sndS', id))
            wav = snd.to_wav(r.data, companion.data if companion else None)
            (sdir / f'{id}.wav').write_bytes(wav)
            written.add(id)
    return len(written)


# ---------------------------------------------------------------------------
# Music: cmid song(s) + the INST/csnd instruments + samples they need.
# Depends on lane 2's cmid.decode / inst.decode_inst / inst.csnd_to_wav;
# raises NotImplementedError until they land.
#
# inst.decode_inst() is documented (inst.py) to return "the resource_dasm
# INST JSON shape": {'regions': [{'key_low','key_high','base_note',
# 'freq_mult','filename'}, ...]}, where `filename` follows resource_dasm's
# own dump-naming convention (e.g. "..._csnd_3201_pianoC4.wav" /
# "..._snd_12742_....wav"). This lets pack.py recover the sample's resource
# type + id with the exact same regex tools/pack_assets.py already uses, and
# resolve it against `shared_sound` instead of a file on disk. If lane 2's
# actual return shape differs (e.g. a direct (type, id) tuple instead of a
# filename string), only this one regex site needs to change.
# ---------------------------------------------------------------------------

_SAMPLE_KEY_RE = re.compile(r'_((?:c)?snd)_(\d+)[_.]')


def build_music(slug: str, shared_sound: ResMap, out: Path):
    ids = SONGS.get(slug)
    if not ids:
        return None
    mdir = out / 'music'
    sdir = mdir / 'samples'
    songs, programs = {}, set()
    for sid in ids:
        cmid_res = shared_sound.get(('cmid', sid))
        if cmid_res is None:
            continue
        data = cmid.decode(cmid_res.data)
        ppq, length_ms, note_ons, progs, name = smf_scan(data)
        sdir.mkdir(parents=True, exist_ok=True)
        (mdir / f'song_{sid}.mid').write_bytes(data)
        songs[str(sid)] = {'file': f'music/song_{sid}.mid', 'name': name,
                           'ppq': ppq, 'length_ms': length_ms, 'note_ons': note_ons}
        programs |= progs
    if not songs:
        return None

    # inst.decode_inst() is the pure per-resource parse (base_note, flags,
    # regions keyed by snd_id -- resource_dasm's decode_INST()).
    # inst.resolve_regions() is the second stage (resource_dasm's
    # generate_json_for_INST()): it resolves each region's snd_id against
    # `shared_sound` to the filename/base_note/freq_mult shape pack_music()
    # (and this function) actually reads. base_filename only has to be
    # stable and match resource_dasm's own naming convention closely enough
    # for _SAMPLE_KEY_RE below to find the "_<type>_<id>" it embeds --
    # "Twisted_Sound.rsrc" is what resource_dasm named the shared bank when
    # the reference packs were ripped (see tests/test_inst.py).
    inst_by_prog = {id: inst.decode_inst(r.data)
                     for (t, id), r in shared_sound.items() if t == 'INST'}

    instruments, samples = {}, {}
    for prog in sorted(programs):
        decoded = inst_by_prog.get(prog)
        if decoded is None:
            continue
        resolved = inst.resolve_regions(decoded, shared_sound, 'Twisted_Sound.rsrc')
        regions = []
        for r in resolved:
            m = _SAMPLE_KEY_RE.search(r['filename'])
            if not m:
                continue
            rtype, rid = m.group(1), int(m.group(2))
            key = f'{rtype}_{rid}'
            if key not in samples:
                res_type = 'csnd' if rtype == 'csnd' else 'snd '
                res = shared_sound.get((res_type, rid))
                if res is None:
                    continue
                wav = inst.csnd_to_wav(res.data) if rtype == 'csnd' else snd.to_wav(res.data)
                rate, wbase, ls, le = wav_sample_meta(wav)
                sdir.mkdir(parents=True, exist_ok=True)
                (sdir / f'{key}.wav').write_bytes(wav)
                samples[key] = {'file': f'music/samples/{key}.wav', 'rate': rate,
                                'base_note': wbase, 'loop_start': ls, 'loop_end': le}
            regions.append({'key_low': r['key_low'], 'key_high': r['key_high'],
                            'base_note': r['base_note'],
                            'freq_mult': r.get('freq_mult', 1.0), 'sample': key})
        if regions:
            instruments[str(prog)] = regions
    return {'songs': songs, 'instruments': instruments, 'samples': samples}


# ---------------------------------------------------------------------------
# Help text: the module's own 'TEXT' 1000 -- the blurb After Dark 3.0's
# control panel printed under the module's controls ("what is this thing,
# and what do its knobs do"). Berkeley-authored, so it only ever lands in
# the gitignored pack, never in repo source.
# ---------------------------------------------------------------------------

HELP_TEXT_ID = 1000


def build_help(module: ResMap) -> str:
    """'TEXT' 1000 -> str, or '' when the module has none.

    Classic Mac text: Mac Roman, CR line endings. Both are normalised here
    (to str + LF) so the pack carries something a modern text view can show
    directly -- which is also exactly what resource_dasm's own `*_TEXT_*.txt`
    dump contains, so tools/pack_assets.py reading that file and this
    reading the resource produce the same string. See docs/ripper.md.
    """
    res = module.get(('TEXT', HELP_TEXT_ID))
    if res is None:
        return ''
    return res.data.decode('mac-roman', 'replace').replace('\r\n', '\n').replace('\r', '\n')


# ---------------------------------------------------------------------------
# The faceplate banner: the purple "THE TOTALLY TWISTED After Dark SCREEN
# SAVER" strip that topped the original control panel.
#
# It is not a module asset -- one banner serves the whole collection -- so it
# is packed once into assets/_shared/, alongside the per-module <slug>/ dirs.
#
# Source: the "Twisted Faceplate" file's 'PICT' 128 ("Mac Faceplate.PICT",
# 185x48, an 8-bit indexed PackBitsRect PixMap; decode/pict.py). The same
# resource id in After Dark 3.0 itself is its own default faceplate (185x47)
# and is the documented fallback -- though ingest.py's INFRA filter drops
# the After Dark control panel (a 'cdev') on the floor, so a rip of either
# Macintosh Garden original always takes the Twisted one.
# ---------------------------------------------------------------------------

SHARED_DIR = '_shared'
FACEPLATE_PICT_ID = 128
#: Mac file names to look for a faceplate in, best first.
FACEPLATE_SOURCES = ('Twisted Faceplate', 'After Dark 3.0')


def build_faceplate(resources: dict[str, ResMap], assets_root: Path):
    """Write assets/_shared/faceplate.png. Returns the path, or None when
    no ingested file carries a faceplate 'PICT'."""
    for name in FACEPLATE_SOURCES:
        res = (resources.get(name) or {}).get(('PICT', FACEPLATE_PICT_ID))
        if res is None:
            continue
        try:
            w, h, rgba = pict.decode(res.data)
        except ValueError:
            continue
        rows = [rgba[y * w * 4:(y + 1) * w * 4] for y in range(h)]
        out = assets_root / SHARED_DIR
        out.mkdir(parents=True, exist_ok=True)
        path = out / 'faceplate.png'
        path.write_bytes(rlep.write_png(w, h, rows))
        return path
    return None


# ---------------------------------------------------------------------------
# pens500.json (message-mayhem only). Berkeley-derived content: never
# committed to the repo, only ever written into the gitignored pack.
# ---------------------------------------------------------------------------

def build_pens(slug: str, module: ResMap, out: Path) -> bool:
    if slug != 'message-mayhem':
        return False
    res = module.get(('Pens', 500))
    if res is None:
        return False
    doc = pens.decode(res.data)
    (out / 'pens500.json').write_text(json.dumps(doc, indent=1))
    return True


# ---------------------------------------------------------------------------
# Top level.
# ---------------------------------------------------------------------------

def build(slug: str, module_name: str, module: ResMap,
          shared_sound: ResMap, shared_art: ResMap | None,
          out: Path) -> dict:
    """Build assets/<slug>/ from parsed resources. Returns the meta dict
    that was written (for tests / callers that want it without a re-read)."""
    out.mkdir(parents=True, exist_ok=True)

    series, baked, warnings = build_art(slug, module_name, module, out)
    palettes = build_palettes(module)
    base_clut = BASE_CLUT.get(slug)

    build_sounds(module, shared_sound, out)
    build_pens(slug, module, out)
    music = build_music(slug, shared_sound, out)

    meta = {
        'module': module_name.lower().replace(' ', '-'),
        'field': FIELDS.get(slug, [0, 0, 0]),
        'series': series,
        'palettes': palettes,
        'base_clut': base_clut,
        'baked': baked,
        'strings': build_strings(module),
        'help': build_help(module),
        'slider_words': build_slider_words(module),
        'menus': build_menus(module),
    }
    if music is not None:
        meta['music'] = music
    (out / 'meta.json').write_text(json.dumps(meta, indent=1))
    return meta
