"""Extract the 'Pens' 500 stroke font (Message Mayhem) into the pens500.json
shape. Byte-oriented port of the RE repo's scripts/pens_extract.py -- algorithm
verbatim; only the I/O shape changed (resource bytes in, dict out; no
filesystem paths, no CLI). No Berkeley text is embedded here: decode() reads
it from the caller-supplied resource bytes at call time.

Grammar (recovered from MOD fn20/fn22/fn24/fn27 in CODE 129 'DynaMessy'):

    <font>  := <glyph>*
    <glyph> := CHAR ':' <group>* '<' DIGITS '>'
    <group> := '[' <pt> (',' <pt>)* ']'                  polyline
             | '{' ['-'] <centre> ',' <pt> (',' <pt>)* '}'  arc run
    <pt>    := INT ',' INT                       units, y-up, cell is 0..100

`<NNN>` is the advance width in the same units.
For '{', the FIRST pair is not a point: fn27 (0x5CF0..0x5E10) stores it into the
writer's arc centre (obj+0x538/+0x53C) and only then starts reading points, so the
point run of a '{' group begins at its SECOND pair.  Each consecutive point pair is
swept about that shared centre by fn25 (0x4B42) with BOTH the angle and the radius
interpolated linearly -- an Archimedean-spiral segment, not a circular arc.  A '-'
immediately after '{' is consumed by fn27 at 0x5D06 as a sweep-direction flag
(obj+0x32), NOT as a minus sign; the centre's x is its magnitude.

The centre's y is measured DOWN from the top of the em box, while point y is
measured UP from the baseline (fn25 flips points with `100 - y` and then adds
YOFF = 100*(1-scale); the centre gets neither).

Two original-code quirks are reproduced faithfully and flagged:

  * `raw_neg_bug` -- fn24's decimal accumulator (0x4688, 0x474C) does
    `acc = c - '0'` on the FIRST character with no sign handling, so a '-' seeds
    the accumulator with -3 (45-48).  For a '[' group nothing strips the '-',
    so "-5" parses as -25 and "-1" as -29.  Inside '{' the '-' *is* consumed by
    fn27 at 0x5D06 (setting the negate flag obj+0x32), so anchors are clean.
  * duplicated consecutive points are no-ops: fn27 0x5F84..0x5FA6 detects
    prev == next and reads one more pair instead of drawing a zero-length
    segment.  We keep them in `raw` and drop them in `points`.
"""


def _parse_int_like_fn24(tok):
    """Reproduce fn24's accumulator exactly (no sign handling)."""
    acc = ord(tok[0]) - 48
    for ch in tok[1:]:
        acc = acc * 10 + (ord(ch) - 48)
    return acc


def _parse(text):
    glyphs = {}
    order = []
    i = 0
    n = len(text)
    while i < n:
        if text[i] == "\0":
            break
        ch = text[i]
        assert text[i + 1] == ":", (i, repr(text[i:i + 8]))
        i += 2
        groups = []
        advance = None
        while i < n:
            c = text[i]
            if c == "<":
                j = text.index(">", i)
                advance = int(text[i + 1:j])
                i = j + 1
                break
            if c not in "[{":
                raise ValueError(f"unexpected {c!r} at {i} in glyph {ch!r}")
            close = "]" if c == "[" else "}"
            j = text.index(close, i)
            body = text[i + 1:j]
            i = j + 1
            toks = body.split(",")
            nums_raw = [_parse_int_like_fn24(t) for t in toks]
            nums_true = [int(t) for t in toks]
            if c == "[":
                kind = "poly"
                anchor = None
                # '[' groups go through fn24 unfiltered -> the sign bug applies
                nums = nums_raw
                buggy = nums_raw != nums_true
            else:
                kind = "arc"
                # fn27 strips a leading '-' before the centre and sets obj+0x32
                anchor_neg = toks[0].startswith("-")
                anchor = [abs(nums_true[0]), nums_true[1]]
                nums = nums_true[2:]
                buggy = False
            pts_raw = [[nums[k], nums[k + 1]] for k in range(0, len(nums) - 1, 2)]
            pts = []
            for p in pts_raw:
                if pts and pts[-1] == p:
                    continue
                pts.append(p)
            g = {"kind": kind, "points": pts}
            if kind == "arc":
                g["arc_centre"] = anchor
                g["sweep_reverse"] = anchor_neg
            if pts_raw != pts:
                g["raw_points"] = pts_raw
            if buggy:
                g["raw_neg_bug"] = True
                g["points_as_authored"] = [
                    [nums_true[k], nums_true[k + 1]]
                    for k in range(0, len(nums_true) - 1, 2)
                ]
            groups.append(g)
        glyphs[ch] = {"advance": advance, "groups": groups}
        order.append(ch)
    return glyphs, order


def decode(data: bytes) -> dict:
    """'Pens' 500 resource bytes -> the pens500.json document (dict)."""
    text = data.decode("mac-roman")
    glyphs, order = _parse(text)
    return {
        "source": "ripped/message-mayhem/Message Mayhem_Pens_500.bin ('Pens' 500)",
        "module": "Message Mayhem (After Dark 3.x, Totally Twisted, 1995)",
        "units": {
            "cell": "x,y in font units; point y is UP from the baseline, body 0..100 "
            "(a few points reach 101); arc-centre y is DOWN from the top of the em box",
            "line_height": 125.0,
            "em_centre_y": 50.0,
            "note": "runtime multiplies every coordinate by a single-precision "
            "scale in obj+0x540 (fn23 fits it to the screen; clamped 0.1..1.0)",
        },
        "grammar": {
            "[...]": "polyline; fn25 0x484C steps max(|dx|,|dy|)-1 dots per segment",
            "{[-]cx,cy,...}": "arc run about (cx,cy); fn25 0x4B42 sweeps angle AND "
            "radius linearly, max(|dx|,|dy|)*6/5 dots per segment. A leading '-' is "
            "a reverse-sweep flag consumed by fn27 0x5D06, not a minus sign.",
            "<n>": "advance width in font units (fn21 0x3D16 / fn22 0x3D84)",
        },
        "brushes": {
            "_": "DATA 129 word tables, 5x5, index = dx*5 + dy. 3 = no ink; "
            "0/1/2 select ink[0..2] = palette base+0/+1/+2. fn19 0x3A70.",
            "0x075C_scale_lt_0.15": [3, 3, 3, 3, 3, 3, 0, 0, 3, 3, 3, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
            "0x078E_scale_0.15_to_0.7": [3, 3, 3, 3, 3, 3, 2, 2, 2, 3, 3, 1, 0, 1, 3, 3, 1, 0, 1, 3, 3, 2, 2, 2, 3],
            "0x07C0_scale_gt_0.7": [3, 2, 2, 2, 3, 2, 1, 0, 1, 2, 2, 1, 0, 1, 2, 2, 1, 0, 1, 2, 3, 2, 2, 2, 3],
            "extent": "fn19 stamps 5x5 when scale >= 0.15, else 3x3 (stride stays 5)",
        },
        "ink": {
            "base_index": "DoBlank 0x2644: 3 * rand(3) at depth >= 8, else 0 "
            "(this+0x134); DoDrawFrame 0x28A0 adds 9 while the toggle this+0x13A is set",
            "palette": "clut 601 'Bathroom Pens 256' -- 18 live entries = 6 pens x 3 shades",
            "ramps": {
                "0": ["000000", "2f2f30", "6c6b6e"],
                "3": ["870d0d", "943032", "a2585a"],
                "6": ["285d1a", "3a692e", "798f76"],
                "9": ["6cfd5d", "3f9336", "215027"],
                "12": ["68b3ff", "4a81b9", "345a80"],
                "15": ["fd3c3c", "bc3030", "832727"],
            },
        },
        "glyph_order": order,
        "glyphs": glyphs,
    }
