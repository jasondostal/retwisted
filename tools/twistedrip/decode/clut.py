"""Mac 'clut' colour-table resource -> {slot: (r, g, b)}.

Ported verbatim from tools/pack_assets.py's parse_clut_bin (RE repo checkout,
no listing citation needed beyond the resource's own documented layout):
u32 seed, u16 flags, u16 ctSize, then (ctSize+1) colorSpecs of
{u16 value, 3x u16 rgb}. `value` is the resource's OWN colour-table slot
index -- Mac cluts are slot-addressed and may be sparse, so position in the
file is NOT the slot (see docs/ripper.md and pack_assets.py's meta.json
`palettes` note).
"""
import struct


def parse(data: bytes) -> dict[int, tuple[int, int, int]]:
    if len(data) < 8:
        return {}
    _seed, _flags, ct_size = struct.unpack_from('>IHH', data)
    out = {}
    for i in range(ct_size + 1):
        off = 8 + i * 8
        if off + 8 > len(data):
            break
        v, r, g, b = struct.unpack_from('>HHHH', data, off)
        out[v] = (r >> 8, g >> 8, b >> 8)
    return out
