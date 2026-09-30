"""Decode Mac After Dark 'RLEP' sprite resources (Library 4.0 / DynaModules)
to PNGs. Byte-oriented port of the RE repo's scripts/rlep2png.py -- same
algorithm verbatim, including every documented quirk; only the I/O shape
changed (bytes in, bytes out; no filesystem paths, no CLI).

Container (verified against LIB40_RLE 68k disassembly, CODE 133 of Library 4.0):
  16-byte big-endian chunk headers, same RLID codec as AD 4.x Windows
  (see the RE repo's flying-toasters-reverse-engineered/tools/rlid.py):
    +0 tag ('RLID' file header, 'CSTM' frame pixel strip, 'IHDR' rect -- absent
        in Totally Twisted banks), +4 param (high byte = depth bitmask, bits
        16-23 = native depth code, low word = frame id), +8 flags (nonzero =
        last chunk), +C u32 chunk size incl. header.
  Colors come from the bank's own CTAB chunk (after the pixel strips): param
  low byte = entry count, payload = count * 4 bytes (flag, R, G, B). One table
  serves both index widths -- 4-bit opcodes address its first 16 entries,
  8-bit opcodes the whole table (the engine's [A2 + D0*4] lookup is shared).
  IHDR chunks (also trailing) carry each frame's Mac Rect (t, l, b, r).

The sibling `Rdat <id>` resource is the bank's own manifest and is used here
as a completeness check (see parse_rdat): its 50-byte record ends with u16
maxWidth, maxHeight, 1, colorCount, frameCount. That reading was verified
against all 168 RLEP banks in the collection, so a bank that yields fewer
CSTM chunks (or a smaller CTAB) than its Rdat declares is a real rip
failure -- and none does.

RLE stream: opcode byte, low nibble = op, high nibble = H:
  0: H==1 end row; H==0 end sprite; H==3 copy row (next 2 bytes BE = row idx)
  1: skip H transparent px (H==0: count = next byte)
  2: run of c16[H], count = next byte
  3/4/5: 1/2/3 px of c16[H]
  6: run of c256[next], count = H (H==0: count = byte after color)
  7: H==0 packed 4-bit literals (count=next, nibble pairs, c16);
     H==1 8-bit literals (count=next, c256)
  8: dither pair from one nibble-packed byte (c16), count = H or next
  9: dither pair from two bytes (c256), count = H or next
"""
import struct
import zlib

SENTINEL = 0x0AEDF8F8


def chunks(data: bytes):
    pos = 0
    while pos + 16 <= len(data):
        tag = data[pos:pos + 4]
        if struct.unpack_from('>I', data, pos)[0] == SENTINEL:
            break
        param, flags, size = struct.unpack_from('>III', data, pos + 4)
        if size < 16:
            break
        yield tag, param, flags, data[pos + 16:pos + size]
        pos += size


def decode_rows(stream: bytes):
    """Decode one CSTM stream -> list of rows of (x, [pixel], is16) runs.

    is16 marks runs whose values are 4-bit hot-color indices; the rest are
    8-bit palette indices.
    """
    rows, row, x, i, n = [], [], 0, 0, len(stream)

    def emit(indices, is16):
        nonlocal x
        if indices:
            row.append((x, list(indices), is16))
            x += len(indices)

    try:
        while i < n:
            b = stream[i]; i += 1
            op, h = b & 0xF, b >> 4
            if op == 0:
                if h in (0, 1):
                    rows.append(row)
                    row, x = [], 0
                    if h == 0:
                        break
                elif h == 3:
                    ref = (stream[i] << 8) | stream[i + 1]; i += 2
                    rows.append(list(rows[ref]) if 0 <= ref < len(rows) else [])
                    row, x = [], 0
            elif op == 1:
                cnt = h or stream[i]
                if not h:
                    i += 1
                x += cnt
            elif op == 2:
                cnt = stream[i]; i += 1
                emit([h] * cnt, True)
            elif op in (3, 4, 5):
                emit([h] * (op - 2), True)
            elif op == 6:
                c = stream[i]; i += 1
                cnt = h
                if not cnt:
                    cnt = stream[i]; i += 1
                emit([c] * cnt, False)
            elif op == 7:
                if h == 0:
                    cnt = stream[i]; i += 1
                    px = []
                    while cnt >= 2:
                        v = stream[i]; i += 1
                        px += [v >> 4, v & 0xF]
                        cnt -= 2
                    if cnt == 1:
                        px.append(stream[i] >> 4); i += 1
                    emit(px, True)
                elif h == 1:
                    cnt = stream[i]; i += 1
                    emit(stream[i:i + cnt], False); i += cnt
            elif op == 8:
                cnt = h
                if not cnt:
                    cnt = stream[i]; i += 1
                v = stream[i]; i += 1
                px = [v >> 4, v & 0xF] * (cnt // 2) + ([v >> 4] if cnt & 1 else [])
                emit(px, True)
            elif op == 9:
                cnt = h
                if not cnt:
                    cnt = stream[i]; i += 1
                c1, c2 = stream[i], stream[i + 1]; i += 2
                px = [c1, c2] * (cnt // 2) + ([c1] if cnt & 1 else [])
                emit(px, False)
    except IndexError:
        if row:
            rows.append(row)
    return rows


def parse_ctab(param: int, payload: bytes):
    count = (param >> 8) & 0xFF
    return [tuple(payload[i * 4 + 1:i * 4 + 4]) for i in range(count)]


def write_png(w: int, h: int, rgba_rows) -> bytes:
    def chunk(tag, payload):
        c = tag + payload
        return struct.pack('>I', len(payload)) + c + struct.pack('>I', zlib.crc32(c))
    raw = b''.join(b'\x00' + row for row in rgba_rows)
    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9))
            + chunk(b'IEND', b''))


def render(rows, ctab, rect=None):
    """rows/ctab -> (w, h, png_bytes) | None. rect = Mac Rect (t, l, b, r).

    Trailing transparent runs are never encoded, so the decoded extent can
    undershoot the true frame size; the IHDR rect is authoritative.
    """
    h = len(rows)
    w = max((x + len(px) for r in rows for x, px, _ in r), default=0)
    if rect is not None:
        w, h = max(w, rect[3] - rect[1]), max(h, rect[2] - rect[0])
        rows = rows + [[]] * (h - len(rows))
    if not (w and h):
        return None
    out = []
    for r in rows:
        line = bytearray(w * 4)
        for x, px, _ in r:
            for j, v in enumerate(px):
                rgb = ctab[v] if v < len(ctab) else (255, 0, 255)
                line[(x + j) * 4:(x + j) * 4 + 4] = bytes(rgb) + b'\xff'
        out.append(bytes(line))
    return w, h, write_png(w, h, out)


def parse_bank(data: bytes):
    """Return (ctab, frames) where frames = {id: (rect|None, stream)}."""
    ctab, streams, rects = [], {}, {}
    for tag, param, flags, payload in chunks(data):
        if tag == b'CSTM':
            streams[param & 0xFFFF] = payload
        elif tag == b'CTAB':
            ctab = parse_ctab(param, payload)
        elif tag == b'IHDR':
            rects[param & 0xFFFF] = struct.unpack_from('>4h', payload, 4)
    return ctab, {fid: (rects.get(fid), s) for fid, s in streams.items()}


def parse_rdat(rdat: bytes):
    """The bank's own Rdat manifest -> (maxW, maxH, colors, frames) | None."""
    if len(rdat) < 0x32:
        return None
    w, h, _one, ncol, nframes = struct.unpack_from('>5H', rdat, 0x28)
    return w, h, ncol, nframes


def check_rdat(rdat: bytes | None, ctab, frames) -> str:
    """Cross-check a decoded bank against its Rdat manifest; return a status line."""
    declared = parse_rdat(rdat) if rdat is not None else None
    if declared is None:
        return ' (no Rdat manifest)'
    w = max((r[3] - r[1] for r, _ in frames.values() if r), default=0)
    h = max((r[2] - r[0] for r, _ in frames.values() if r), default=0)
    got = (w, h, len(ctab), len(frames))
    if got == declared:
        return f' (Rdat OK: {declared[3]} frames, {declared[2]} colors, max {declared[0]}x{declared[1]})'
    return (f' *** RDAT MISMATCH: declared maxW/maxH/colors/frames={declared}'
            f' but decoded={got} ***')
