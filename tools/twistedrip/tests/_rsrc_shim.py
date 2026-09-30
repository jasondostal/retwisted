"""Test-only minimal Mac resource-fork reader.

This is NOT tools/twistedrip/rsrc.py -- that interface belongs to lane 2 and
isn't implemented yet. This shim exists purely so lane 1's tests can exercise
decode/* and pack.py against real originals (via the "..namedfork/rsrc" APFS
view) before rsrc.py lands. It is deliberately tiny and lives only under
tests/; nothing outside the test suite imports it.

Classic Mac resource-fork layout (public format, e.g. Inside Macintosh):
  +0  u32 dataOffset, u32 mapOffset, u32 dataLength, u32 mapLength
  resource map (at mapOffset):
    +0..+15  copy of the above header
    +16 u32 nextResourceMap (unused on disk)
    +20 u16 fileRefNum (unused on disk)
    +22 u16 attributes
    +24 u16 typeListOffset (from map start)
    +26 u16 nameListOffset (from map start)
  type list (at mapOffset+typeListOffset):
    u16 numTypes-1, then per type: 4s type, u16 numRefs-1, u16 refListOffset
    (refListOffset relative to the type list's own start)
  reference list per type, 12 bytes/entry:
    u16 id (signed), u16 nameOffset (0xFFFF = none, relative to name list),
    u8 attributes + u24 dataOffset (relative to dataOffset), u32 handle (unused)
  resource data (at dataOffset+that u24 offset): u32 length, then bytes.
"""
import struct


def parse(fork: bytes) -> dict[tuple[str, int], tuple[str, bytes]]:
    """-> {(type, id): (name, data)}"""
    dof, mof, _dl, _ml = struct.unpack_from('>IIII', fork, 0)
    tlo, nof = struct.unpack_from('>HH', fork, mof + 24)
    tb = mof + tlo
    nt = struct.unpack_from('>h', fork, tb)[0] + 1
    out = {}
    for i in range(nt):
        t, cnt, off = struct.unpack_from('>4sHH', fork, tb + 2 + i * 8)
        type_str = t.decode('mac-roman')
        for j in range(cnt + 1):
            rid, noff, ao = struct.unpack_from('>hHI', fork, tb + off + j * 12)
            do = ao & 0xFFFFFF
            (sz,) = struct.unpack_from('>I', fork, dof + do)
            name = ''
            if noff != 0xFFFF:
                nb = mof + nof + noff
                ln = fork[nb]
                name = fork[nb + 1:nb + 1 + ln].decode('mac-roman')
            out[(type_str, rid)] = (name, fork[dof + do + 4:dof + do + 4 + sz])
    return out


def read_fork(path) -> bytes:
    with open(str(path) + '/..namedfork/rsrc', 'rb') as f:
        return f.read()
