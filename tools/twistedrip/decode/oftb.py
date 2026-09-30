"""Parse Mac After Dark OFst/OFtb compound-frame tables and compose PNGs.
Byte-oriented port of the RE repo's scripts/oftb_compose.py -- algorithm
verbatim; only the I/O shape changed (resource bytes + an in-memory art
chain in, PNG bytes out; no filesystem paths, no CLI).

Mac-side analog of AD 4.x Windows CompoundSequence resources. Field names
below are Berkeley's own, from the ResEdit TMPL 128/130 resources shipped in
the Flying Toilets Demo module. All fields big-endian; rects are portable
XRect order (x1, y1, x2, y2) = (l, t, r, b), NOT Mac Rect order.

  OFst "MMOffsets" (directory into OFtb):
    +0 u16 maxNumParts, +2 u16 maxFrameNumber, +4 mxBnds (x1, x2, y1, y2),
    +C u16 entry count, then 10-byte entries:
      +0 u16 frameNum
      +2 u32 frameOffset -- displayed as one hex long by Berkeley's TMPL but
         actually split: high u16 = OFtb bank of the series (base + k),
         low u16 = byte offset within that OFtb
      +6 s16 offset.x, +8 s16 offset.y (link offset applied when a
      sequence enters at this frame)

  OFtb "Compound Frames":
    +0 u16 entry count, then variable-length entries (walkable linearly,
    so OFtb banks without their own OFst still parse):
      +0 compBn XRect (x1, y1, x2, y2) -- compound bounds
      +8 u16 NumParts
      +A parts, 14 bytes each:
          +0 u16 artNum -- 1-based: artNum - 1 is a CSTM/IHDR FRAME ID.
             Frame ids are allocated globally across the series' RLEP banks
             (bank base holds 0..n, the next bank's ids continue after it,
             sometimes with gaps), so art is resolved from the union of all
             chained banks. A module bank whose ids restart at 0 begins a
             new, separate series.
          +2 u16 chanNum   +4 u16 flags -- MIRROR BITS, see below
          +6 bounds XRect -- placement; its size must equal the art frame's
             IHDR size (validated as a hard gate)

Part flags (collection-wide the field only ever holds 0/1/2/3):
  bit 0 = flip horizontal, bit 1 = flip vertical. These select the engine's
  mirrored blit variants (Library 4.0 CODE 135 `LIB40_Sprite` exports
  `FlipVer`/`Horiz_`; the RLE segment's three blit entries are forward,
  mirrored, masked). Verified against Frankenscreen series 1000, where the
  same eyeball art is placed with flags 0/1/2 and the placement rect shifts
  purely in X for flag 1 and purely in Y for flag 2 -- the registration shift
  a mirror about the sprite's own centre produces:
      art 15 flag 0 @(308,213,350,252)   flag 1 @(319,212,361,251)   [X only]
      art 14 flag 0 @(316,215,354,258)   flag 2 @(315,206,353,249)   [Y only]
  Coming Soon compound 1 is the clearest visual proof: one half-disc art drawn
  twice, flag 0 then flag 3, which is a whole moon only when mirrored.
  Ignoring the field doubled the unmirrored half instead -- 12,259 of 86,738
  part placements were affected.

Shadow channels (see SHADOW_CHANNELS): a module's own compound drawer may
paint some channels as a black silhouette rather than in the art's own
colours, which is how Coming Soon's mascot gets its drop shadow. Composing
those parts in colour renders the same sprite twice a few pixels apart -- the
"double stamp" ghost -- so the channel rule has to be honoured here.

Black rectangles in the output are NOT a matting bug: modules draw onto a
blanked (black) After Dark screen, so Berkeley's artists left sprite
backgrounds opaque black where it was free, and even used a solid-black art
frame as an explicit "mowed patch" eraser. Composite these PNGs over black,
not over white.
"""
from .rlep import parse_bank, decode_rows, write_png
import struct


def parse_ofst(data: bytes):
    max_w, max_h, count = struct.unpack_from('>3H', data, 8)
    recs = []
    for i in range(count):
        fno, off, dx, dy = struct.unpack_from('>HIhh', data, 14 + i * 10)
        recs.append((fno, off, dx, dy))
    return (max_w, max_h), recs


def parse_oftb_entry(data: bytes, off: int):
    bounds = struct.unpack_from('>4h', data, off)
    (nitems,) = struct.unpack_from('>H', data, off + 8)
    items = []
    for j in range(nitems):
        o = off + 0x0A + j * 14
        art, chan, fl = struct.unpack_from('>3H', data, o)
        rect = struct.unpack_from('>4h', data, o + 6)
        items.append((art, chan, fl, rect))
    return bounds, items


FLIP_H, FLIP_V = 1, 2

# Channels a module's compound drawer blits as a black SILHOUETTE instead of
# in the art's own colours, keyed by the resource-fork stem the rip uses (the
# module's Mac file name, e.g. "Coming Soon").
#
# Coming Soon draws its own compound tables (`MOD.f01E4`, CODE 129 DynaCS1
# @0x01E4-0x03DC -- the drawer named in docs/behavior/coming-soon.md SS8 and
# SS4). Walking the 14-byte part records it keeps a latch word at this+0x11E,
# cleared to 0 on entry (@0x01EE):
#
#   @0x0250-0x028A  chanNum in 3..8  and latch == 0  ->  latch = 4
#   @0x028C-0x02C0  chanNum >= 10    and latch != 0  ->  latch = 0
#   @0x03A4-0x03AE  blit flags = latch XOR the part's own mirror bits
#
# so every part on channels 3..8 reaches the blitter with bit 2 set. Bit 2
# selects the RLE segment's third blit entry, the MASKED one (Library 4.0
# CODE 135 residue `MaskSequence`; segment 133's blitter is `0x05C3`, args
# 18 -- exactly the 18 bytes this drawer pushes at @0x03A2-0x03D2), i.e. the
# art's silhouette in black. A companion depth branch (@0x0322-0x0378) drops
# channels 3..9 outright when the screen is 4-bit -- the shadow is the first
# thing sacrificed on a shallow screen, which is what a shadow is for.
#
# The rule is Coming Soon's alone: no other module's CODE contains the
# chanNum 3/9 comparisons around a part-record walk. Corroboration from the
# data: the module's two dedicated overlay series 13000 and 13010 put their
# single part on channel 3 every time, and their art is 100% index-0 black
# already (9,396 / 1,434 opaque pixels, all (0,0,0)) -- pre-blackened shadow
# sprites for the channel that blackens them. Series 20000's channel 3..8
# parts (arts 85/87/88) are the ones missing from the rip entirely, so the
# rule costs the poster compounds nothing.
SHADOW_CHANNELS = {
    'Coming Soon': frozenset(range(3, 9)),
}


def flip_grid(grid, flags):
    """Apply a part's mirror bits (bit 0 = horizontal, bit 1 = vertical)."""
    if flags & FLIP_H:
        grid = [row[::-1] for row in grid]
    if flags & FLIP_V:
        grid = grid[::-1]
    return grid


def blacken_grid(grid):
    """Silhouette a grid: every opaque pixel to black, holes left alone."""
    return [[None if p is None else (0, 0, 0) for p in row] for row in grid]


def part_grid(art, art_no, flags, shadow=False):
    """Art frame for a part record, mirrored per its flags. -> (w, h, grid) | None.

    `shadow` blacks the frame out (see SHADOW_CHANNELS).
    """
    fid = art_no - 1
    entry = art.get(fid)
    if entry is None:
        return None
    if not flags and not shadow:
        return entry
    w, h, grid = entry
    grid = flip_grid(grid, flags)
    return w, h, (blacken_grid(grid) if shadow else grid)


def frame_pixels(rows, ctab, w, h):
    """Rasterize decoded rows to a (w, h) RGBA byte grid (None = transparent)."""
    grid = [[None] * w for _ in range(h)]
    for y, r in enumerate(rows[:h]):
        for x, px, _ in r:
            for j, v in enumerate(px):
                if x + j < w:
                    grid[y][x + j] = ctab[v] if v < len(ctab) else (255, 0, 255)
    return grid


def load_art_chain(rlep_by_id: dict[int, bytes], base: int):
    """Merge art frames by frame id from the series' chained RLEP banks.

    Starting at bank `base`, include ascending-id banks whose frame ids
    continue past the running maximum; a bank restarting at id 0 (or below
    the chain's tail) belongs to another series and is skipped.
    """
    banks = sorted(rlep_by_id.items())
    art, tail = {}, -1
    for bid, data in banks:
        if bid < base:
            continue
        ctab, frames = parse_bank(data)
        ids = sorted(frames)
        if not ids:
            continue
        if bid != base and ids[0] <= tail:
            continue
        for fid, (rect, stream) in frames.items():
            w, h = rect[3] - rect[1], rect[2] - rect[0]
            art[fid] = (w, h, frame_pixels(decode_rows(stream), ctab, w, h))
        tail = ids[-1]
    return art


def compose_compound(art, tb: bytes, off: int, shadow_channels=frozenset()):
    """Compose one OFtb entry (offset `off` into OFtb bytes `tb`) against an
    art chain (see load_art_chain). -> (bounds, items, w, h, png_bytes|None,
    warnings). `items` is the raw (art, chan, flags, rect) part list, usable
    to build meta.json's per-frame `parts` field. `warnings` lists any
    missing-art / size-mismatch gate failures (diagnostic only)."""
    bounds, items = parse_oftb_entry(tb, off)
    bl, bt, br, bb = bounds
    w, h = br - bl, bb - bt
    canvas = [[None] * w for _ in range(h)]
    warnings = []
    for art_no, chan, fl, (il, it, ir, ib) in items:
        fid = art_no - 1
        shade = chan in shadow_channels
        entry = part_grid(art, art_no, fl, shade)
        if entry is None:
            warnings.append(f'art {art_no} (0-based {fid}) missing')
            continue
        aw, ah, grid = entry
        if (ir - il, ib - it) != (aw, ah):
            warnings.append(
                f'art {fid} is {aw}x{ah} but rect {(il, it, ir, ib)} is '
                f'{ir - il}x{ib - it} GATE-FAIL')
        for y in range(min(ah, ib - it)):
            cy = it - bt + y
            if not 0 <= cy < h:
                continue
            for x in range(min(aw, ir - il)):
                cx = il - bl + x
                if 0 <= cx < w and grid[y][x] is not None:
                    canvas[cy][cx] = grid[y][x]
    rgba = [b''.join(b'\x00\x00\x00\x00' if p is None else bytes(p) + b'\xff'
                     for p in line) for line in canvas]
    png = write_png(w, h, rgba) if w and h else None
    return bounds, items, w, h, png, warnings
