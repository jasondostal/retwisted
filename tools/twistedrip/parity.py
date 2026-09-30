"""The gate: rip every module and diff against a reference assets/ tree.
Exit non-zero on any difference. See docs/ripper.md 'The gate':

  PNG: byte-identical preferred; pixel-identical (RGBA) acceptable, WARN.
  WAV: byte-identical.
  meta.json: identical after canonical JSON (sorted keys) round-trip.
  pens500.json: identical after canonical round-trip.

Every directory under the reference tree is compared, which includes the
collection-wide `_shared/` (the control panel's faceplate.png) alongside the
13 module slugs -- no special case is needed, and `--module _shared` narrows
to just it.

Usage: python3 -m tools.twistedrip.parity <reference-assets-dir> <ripped-assets-dir>
       [--module SLUG [SLUG ...]]
"""
import argparse
import json
import struct
import sys
import zlib
from pathlib import Path

# ---------------------------------------------------------------------------
# Minimal PNG decoder (8-bit RGB/RGBA, no interlace) for the pixel-identical
# fallback. Our own writer (decode/rlep.py write_png) only ever emits 8-bit
# RGBA, colour type 6, no interlace, but a reference pack made some other
# way (Pillow, etc.) may use RGB or a different filter mix per row -- both
# are handled; anything else (16-bit, palette, interlaced) is reported as
# undecodable rather than guessed at.
# ---------------------------------------------------------------------------

_CHANNELS = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def decode_png(data: bytes):
    """-> (w, h, rgba_bytes) | None (undecodable: non-8-bit, palette, interlaced)."""
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        return None
    pos = 8
    w = h = bitdepth = colortype = interlace = None
    idat = bytearray()
    n = len(data)
    while pos + 8 <= n:
        length = int.from_bytes(data[pos:pos + 4], 'big')
        tag = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + length]
        pos += 8 + length + 4
        if tag == b'IHDR':
            # IHDR is 13 bytes (w, h, depth, colour type, compression,
            # filter, interlace) -- slicing it to 10 made every call to the
            # pixel-identical fallback raise struct.error instead of
            # answering. Latent until 2026-09-19: the gate has been
            # byte-clean, so the fallback had never run.
            w, h, bitdepth, colortype, _c, _f, interlace = struct.unpack('>IIBBBBB', payload[:13])
        elif tag == b'IDAT':
            idat += payload
        elif tag == b'IEND':
            break
    if w is None or bitdepth != 8 or colortype not in (2, 6) or interlace:
        return None
    channels = _CHANNELS[colortype]
    try:
        raw = zlib.decompress(bytes(idat))
    except zlib.error:
        return None
    stride = w * channels
    out = bytearray(h * stride)
    prev = bytearray(stride)
    pos = 0
    for y in range(h):
        if pos >= len(raw):
            return None
        ft = raw[pos]; pos += 1
        line = bytearray(raw[pos:pos + stride]); pos += stride
        if len(line) < stride:
            return None
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = prev[x]
            c = prev[x - channels] if x >= channels else 0
            if ft == 1:
                line[x] = (line[x] + a) & 0xFF
            elif ft == 2:
                line[x] = (line[x] + b) & 0xFF
            elif ft == 3:
                line[x] = (line[x] + (a + b) // 2) & 0xFF
            elif ft == 4:
                line[x] = (line[x] + _paeth(a, b, c)) & 0xFF
        out[y * stride:(y + 1) * stride] = line
        prev = line
    if channels == 4:
        return w, h, bytes(out)
    rgba = bytearray()
    for i in range(0, len(out), 3):
        rgba += out[i:i + 3] + b'\xff'
    return w, h, bytes(rgba)


def compare_png(ref: bytes, got: bytes):
    """-> ('ok' | 'warn' | 'fail', message)."""
    if ref == got:
        return 'ok', ''
    dref, dgot = decode_png(ref), decode_png(got)
    if dref is None or dgot is None:
        return 'fail', 'byte mismatch and PNG undecodable for pixel fallback'
    if dref == dgot:
        return 'warn', 'byte mismatch, pixel-identical RGBA'
    if dref[:2] != dgot[:2]:
        return 'fail', f'size mismatch {dref[:2]} vs {dgot[:2]}'
    return 'fail', 'pixel mismatch'


def canonical_json_text(path: Path) -> str:
    return json.dumps(json.loads(path.read_text()), sort_keys=True)


# ---------------------------------------------------------------------------
# Tree walk.
# ---------------------------------------------------------------------------

class Tally:
    def __init__(self):
        self.ok = self.warn = self.fail = 0
        self.messages: list[str] = []

    def add(self, status, msg=''):
        if status == 'ok':
            self.ok += 1
        elif status == 'warn':
            self.warn += 1
            self.messages.append(f'WARN {msg}')
        else:
            self.fail += 1
            self.messages.append(f'FAIL {msg}')


def compare_file(rel: str, ref_path: Path, got_path: Path, tally: Tally):
    if not got_path.exists():
        tally.add('fail', f'{rel}: missing from ripped tree')
        return
    if rel.endswith('.png'):
        status, msg = compare_png(ref_path.read_bytes(), got_path.read_bytes())
        tally.add(status, f'{rel}: {msg}' if msg else '')
    elif rel.endswith('.wav') or rel.endswith('.mid'):
        if ref_path.read_bytes() != got_path.read_bytes():
            tally.add('fail', f'{rel}: byte mismatch')
        else:
            tally.add('ok')
    elif rel.endswith('.json'):
        try:
            ok = canonical_json_text(ref_path) == canonical_json_text(got_path)
        except (json.JSONDecodeError, OSError) as e:
            tally.add('fail', f'{rel}: {e}')
            return
        tally.add('ok' if ok else 'fail', '' if ok else f'{rel}: canonical JSON mismatch')
    else:
        if ref_path.read_bytes() != got_path.read_bytes():
            tally.add('fail', f'{rel}: byte mismatch')
        else:
            tally.add('ok')


# RESIDUE from an older port: files the MODULES used to generate into a pack at
# runtime (build()-time caches), not pack content -- voyeur's procedural stars /
# wall rows, shock-clocks' code-drawn hands and LCD digits, toxic-swamp's
# mirrored "m_" frames, message-mayhem's pen ink strips.
#
# As of 2026-09-19 NOTHING WRITES THESE ANY MORE. All four families are built in
# memory and handed to the compose pass by name (engine::GEN_PREFIX / the
# Module::generated hook); the writes were a read-only-bundle bug -- inside an
# installed .saver they failed silently and the module drew nothing, which is
# what "the fish swim backwards" turned out to be.
#
# The ignore stays because a live assets/ tree that an older build ran against
# still HOLDS the residue, and deleting it is not this gate's business. A tree
# built from here on simply never grows any, and then this pattern matches
# nothing. A fresh rip never had them either way, so the gate is unchanged.
import re as _re
_DERIVED = _re.compile(
    r'(^|/)(_pens/|hands/|lcd/|m_\d+\.png$|star_\d+\.png$|wall_r\d+\.png$)')


def is_module_derived(rel: str) -> bool:
    return bool(_DERIVED.search(rel))


def compare_module(ref_dir: Path, got_dir: Path) -> Tally:
    tally = Tally()
    if not got_dir.is_dir():
        tally.add('fail', f'{got_dir.name}: ripped tree has no pack for this module')
        return tally
    ref_files = {str(p.relative_to(ref_dir)) for p in ref_dir.rglob('*')
                 if p.is_file() and not is_module_derived(str(p.relative_to(ref_dir)))}
    got_files = {str(p.relative_to(got_dir)) for p in got_dir.rglob('*')
                 if p.is_file() and not is_module_derived(str(p.relative_to(got_dir)))}
    for rel in sorted(ref_files):
        compare_file(rel, ref_dir / rel, got_dir / rel, tally)
    extra = got_files - ref_files
    for rel in sorted(extra):
        tally.add('fail', f'{rel}: unexpected extra file in ripped tree')
    return tally


def compare_trees(ref_root: Path, got_root: Path, modules=None):
    if modules is None:
        modules = sorted(p.name for p in ref_root.iterdir() if p.is_dir())
    results = {}
    for slug in modules:
        results[slug] = compare_module(ref_root / slug, got_root / slug)
    return results


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('reference', type=Path, help='reference assets/ tree (today\'s pack_assets.py output)')
    ap.add_argument('ripped', type=Path, help='freshly ripped assets/ tree')
    ap.add_argument('--module', nargs='+', default=None,
                    help='limit to these slugs (or _shared)')
    args = ap.parse_args(argv)

    results = compare_trees(args.reference, args.ripped, args.module)
    any_fail = False
    for slug, tally in results.items():
        status = 'CLEAN' if not tally.fail and not tally.warn else (
            'WARN' if not tally.fail else 'FAIL')
        print(f'{slug}: {status}  ok={tally.ok} warn={tally.warn} fail={tally.fail}')
        for m in tally.messages:
            print(f'  {m}')
        any_fail = any_fail or tally.fail > 0
    print()
    print('GATE FAILED' if any_fail else 'GATE CLEAN')
    return 1 if any_fail else 0


if __name__ == '__main__':
    sys.exit(main())
