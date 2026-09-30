"""QuickDraw 'PICT' v2 -> RGBA, for the faceplate banner art only.

Scope, deliberately narrow: the faceplate PICTs Berkeley shipped are a
version-2 header followed by ONE `PackBitsRect` (opcode 0x0098) carrying a
PixMap and its colour table, and nothing else that paints. That is the
whole of what this decoder handles:

    0x0011 VersionOp + 0x02FF Version   2 bytes of payload
    0x0C00 HeaderOp                     24 bytes
    0x00A0 ShortComment                 2 bytes   (Twisted Faceplate has one)
    0x0001 Clip                          region
    0x0098 PackBitsRect                  the picture
    0x00FF OpEndPic

Anything else is skipped when its length is known from the opcode's own
size table and is a hard error otherwise -- a silent guess would produce a
plausible-looking wrong banner, which is the one failure mode that would
not show up in a screenshot.

Verified against resource_dasm's own decode of the same resources (the
private RE checkout's `*_PICT_128_*.bmp` dumps) for both the Twisted
Faceplate PICT 128 (185x48, 8-bit indexed) and After Dark 3.0's default
faceplate PICT 128 (185x47): pixel-identical, see
tools/twistedrip/tests/test_pict.py.

No Berkeley content lives here -- only the public QuickDraw format.
"""
import struct

# Opcodes whose payload length is fixed and which paint nothing we need.
# (Everything the faceplate PICTs actually contain is handled explicitly in
# `decode`; this table only exists so an unexpected-but-harmless opcode does
# not have to become a failure.)
_FIXED = {
    0x0000: 0,    # NOP
    0x0011: 2,    # VersionOp (version byte pair)
    0x001E: 0,    # DefHilite
    0x001C: 0,    # HiliteMode
    0x0003: 2,    # TxFont
    0x0004: 1,    # TxFace
    0x0005: 2,    # TxMode
    0x0008: 2,    # PnMode
    0x000D: 2,    # TxSize
    0x001A: 6,    # RGBFgCol
    0x001B: 6,    # RGBBkCol
    0x00FF: 0,    # OpEndPic
}


def _unpack_bits(data: bytes, pos: int, row_bytes: int, out_len: int):
    """One PackBits-compressed row -> (bytes, new pos).

    `row_bytes` decides the byte-count width exactly as QuickDraw does:
    a rowBytes under 250 uses a one-byte run length, 250 or more a
    big-endian two-byte one.
    """
    if row_bytes > 250:
        (n,) = struct.unpack_from('>H', data, pos)
        pos += 2
    else:
        n = data[pos]
        pos += 1
    end = pos + n
    out = bytearray()
    while pos < end and len(out) < out_len:
        flag = data[pos]
        pos += 1
        if flag & 0x80:
            out += bytes([data[pos]]) * (257 - flag)
            pos += 1
        else:
            out += data[pos:pos + flag + 1]
            pos += flag + 1
    return bytes(out[:out_len]), end


def _read_pixmap(data: bytes, pos: int, row_bytes: int):
    """PixMap fields after rowBytes+bounds -> (fields dict, new pos)."""
    (ver, pack_type, pack_size, h_res, v_res, pix_type, pix_size,
     cmp_count, cmp_size, plane_bytes, pm_table, pm_reserved) = \
        struct.unpack_from('>HHIIIHHHHIII', data, pos)
    pos += 36
    return {
        'version': ver, 'pack_type': pack_type, 'pack_size': pack_size,
        'pixel_type': pix_type, 'pixel_size': pix_size,
        'cmp_count': cmp_count, 'cmp_size': cmp_size,
        'plane_bytes': plane_bytes, 'row_bytes': row_bytes,
    }, pos


def _read_ctab(data: bytes, pos: int):
    """Colour table -> ({index: (r, g, b)}, new pos).

    Flag bit 0x8000 ("pmExplicit off / device table") means the ColorSpec
    `value` field is meaningless and the entry's POSITION is its pixel
    index; that is how every faceplate PICT here is written. Without the
    flag the stored value is the index.
    """
    _seed, flags, n = struct.unpack_from('>IHH', data, pos)
    pos += 8
    table = {}
    for i in range(n + 1):
        v, r, g, b = struct.unpack_from('>HHHH', data, pos)
        pos += 8
        table[i if flags & 0x8000 else v] = (r >> 8, g >> 8, b >> 8)
    return table, pos


def decode(data: bytes):
    """'PICT' resource bytes -> (w, h, rgba_bytes). Raises ValueError on
    anything outside the narrow shape documented above."""
    if len(data) < 12:
        raise ValueError('PICT too short')
    # The leading u16 size is the classic 32 KB-limited length and is
    # ignored (a v2 picture may exceed it); the frame rect is the source of
    # truth for the picture's extent.
    top, left, bottom, right = struct.unpack_from('>4h', data, 2)
    pos = 10
    pic_w, pic_h = right - left, bottom - top

    result = None
    n = len(data)
    while pos + 2 <= n:
        (op,) = struct.unpack_from('>H', data, pos)
        pos += 2
        if op == 0x00FF:                       # OpEndPic
            break
        if op == 0x0C00:                       # HeaderOp
            pos += 24
            continue
        if op == 0x00A0:                       # ShortComment
            pos += 2
            continue
        if op == 0x00A1:                       # LongComment
            (_kind, size) = struct.unpack_from('>HH', data, pos)
            pos += 4 + size + (size & 1)
            continue
        if op in (0x0001, 0x0007, 0x0008):     # Clip / PnSize / PnMode
            if op == 0x0001:
                (size,) = struct.unpack_from('>H', data, pos)
                pos += size
            else:
                pos += _FIXED.get(op, 4)
            continue
        if op in (0x0098, 0x0099, 0x009A):
            # 0x0098 PackBitsRect, 0x0099 PackBitsRgn, 0x009A DirectBitsRect.
            # Only the first is in scope: the faceplates are 8-bit (and the
            # B&W companion PICT 129 is 1-bit) indexed PixMaps.
            if op != 0x0098:
                raise ValueError(f'PICT opcode {op:#06x} not supported')
            (row_bytes,) = struct.unpack_from('>H', data, pos)
            pos += 2
            if not row_bytes & 0x8000:
                raise ValueError('PackBitsRect with a BitMap, not a PixMap')
            row_bytes &= 0x3FFF
            bt, bl, bb, br = struct.unpack_from('>4h', data, pos)
            pos += 8
            pm, pos = _read_pixmap(data, pos, row_bytes)
            ctab, pos = _read_ctab(data, pos)
            pos += 8 + 8 + 2               # srcRect, dstRect, mode
            w, h = br - bl, bb - bt
            depth = pm['pixel_size']
            if depth not in (1, 2, 4, 8):
                raise ValueError(f'PICT pixel size {depth} not supported')
            rows = []
            for _ in range(h):
                if row_bytes < 8:
                    # Rows this narrow are stored raw, never packed.
                    raw = data[pos:pos + row_bytes]
                    pos += row_bytes
                else:
                    raw, pos = _unpack_bits(data, pos, row_bytes, row_bytes)
                rows.append(raw)
            result = (w, h, _to_rgba(rows, w, h, depth, ctab))
            continue
        if op in _FIXED:
            pos += _FIXED[op]
            continue
        raise ValueError(f'PICT opcode {op:#06x} not supported')

    if result is None:
        raise ValueError('PICT contains no PackBitsRect')
    w, h, rgba = result
    if (w, h) != (pic_w, pic_h):
        # The PixMap bounds are what was actually stored; the frame rect is
        # only a hint. Report rather than silently disagree.
        raise ValueError(f'PICT frame {pic_w}x{pic_h} != pixmap {w}x{h}')
    return w, h, rgba


def _to_rgba(rows, w, h, depth, ctab) -> bytes:
    """Unpacked index rows -> 8-bit RGBA, opaque."""
    out = bytearray(w * h * 4)
    per_byte = 8 // depth
    mask = (1 << depth) - 1
    # A 1-bit PixMap with a 2-entry table is index 0 = white, 1 = black on
    # the Mac; the table carries that, so nothing is assumed here.
    for y, raw in enumerate(rows):
        base = y * w * 4
        for x in range(w):
            if depth == 8:
                v = raw[x] if x < len(raw) else 0
            else:
                byte = raw[x // per_byte] if x // per_byte < len(raw) else 0
                shift = (per_byte - 1 - (x % per_byte)) * depth
                v = (byte >> shift) & mask
            r, g, b = ctab.get(v, (255, 0, 255))
            o = base + x * 4
            out[o] = r
            out[o + 1] = g
            out[o + 2] = b
            out[o + 3] = 255
    return bytes(out)
