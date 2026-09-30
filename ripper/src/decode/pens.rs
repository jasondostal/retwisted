//! Message Mayhem's 'Pens' 500 stroke font -> the pens500.json document.
//! Port of tools/twistedrip/decode/pens.py (grammar, the fn24 sign quirk and
//! the duplicate-point rule are documented there). No Berkeley text lives
//! here: glyph data comes from the caller's resource bytes at run time.

use crate::json::{Json, Obj};
use crate::util::mac_roman;
use crate::Error;

fn parse_int_like_fn24(tok: &[char]) -> Result<i64, Error> {
    let first = tok
        .first()
        .ok_or_else(|| Error::Decode("Pens: empty number".into()))?;
    let mut acc = *first as i64 - 48;
    for &c in &tok[1..] {
        acc = acc * 10 + (c as i64 - 48);
    }
    Ok(acc)
}

/// Python `int(str)` for the plain decimal forms the font uses.
fn py_int(tok: &[char]) -> Result<i64, Error> {
    let s: String = tok.iter().collect();
    s.trim()
        .replace('_', "")
        .parse::<i64>()
        .map_err(|_| Error::Decode(format!("Pens: bad number {s:?}")))
}

fn index_of(text: &[char], c: char, from: usize) -> Result<usize, Error> {
    text[from..]
        .iter()
        .position(|&x| x == c)
        .map(|p| p + from)
        .ok_or_else(|| Error::Decode(format!("Pens: missing {c:?}")))
}

fn pairs(nums: &[i64]) -> Vec<[i64; 2]> {
    (0..nums.len().saturating_sub(1))
        .step_by(2)
        .map(|k| [nums[k], nums[k + 1]])
        .collect()
}

fn pts_json(p: &[[i64; 2]]) -> Json {
    Json::Arr(
        p.iter()
            .map(|q| Json::Arr(vec![Json::Int(q[0]), Json::Int(q[1])]))
            .collect(),
    )
}

fn parse(text: &[char]) -> Result<(Obj, Vec<Json>), Error> {
    let mut glyphs = Obj::new();
    let mut order = Vec::new();
    let n = text.len();
    let mut i = 0;
    while i < n {
        if text[i] == '\0' {
            break;
        }
        let ch = text[i];
        if text.get(i + 1) != Some(&':') {
            return Err(Error::Decode(format!("Pens: expected ':' at {}", i + 1)));
        }
        i += 2;
        let mut groups = Vec::new();
        let mut advance = Json::Null;
        while i < n {
            let c = text[i];
            if c == '<' {
                let j = index_of(text, '>', i)?;
                advance = Json::Int(py_int(&text[i + 1..j])?);
                i = j + 1;
                break;
            }
            if c != '[' && c != '{' {
                return Err(Error::Decode(format!(
                    "Pens: unexpected {c:?} at {i} in glyph {ch:?}"
                )));
            }
            let close = if c == '[' { ']' } else { '}' };
            let j = index_of(text, close, i)?;
            let body = &text[i + 1..j];
            i = j + 1;
            let toks: Vec<&[char]> = body.split(|&x| x == ',').collect();
            let nums_raw = toks
                .iter()
                .map(|t| parse_int_like_fn24(t))
                .collect::<Result<Vec<_>, _>>()?;
            let nums_true = toks
                .iter()
                .map(|t| py_int(t))
                .collect::<Result<Vec<_>, _>>()?;
            let mut g = Obj::new();
            let (nums, buggy, arc) = if c == '[' {
                let buggy = nums_raw != nums_true;
                (nums_raw.clone(), buggy, None)
            } else {
                if nums_true.len() < 2 {
                    return Err(Error::Decode("Pens: arc group without a centre".into()));
                }
                let anchor_neg = toks[0].first() == Some(&'-');
                let anchor = [nums_true[0].abs(), nums_true[1]];
                (nums_true[2..].to_vec(), false, Some((anchor, anchor_neg)))
            };
            let pts_raw = pairs(&nums);
            let mut pts: Vec<[i64; 2]> = Vec::new();
            for p in &pts_raw {
                if pts.last() == Some(p) {
                    continue;
                }
                pts.push(*p);
            }
            g.set("kind", if arc.is_some() { "arc" } else { "poly" });
            g.set("points", pts_json(&pts));
            if let Some((anchor, neg)) = arc {
                g.set(
                    "arc_centre",
                    Json::Arr(vec![Json::Int(anchor[0]), Json::Int(anchor[1])]),
                );
                g.set("sweep_reverse", neg);
            }
            if pts_raw != pts {
                g.set("raw_points", pts_json(&pts_raw));
            }
            if buggy {
                g.set("raw_neg_bug", true);
                g.set("points_as_authored", pts_json(&pairs(&nums_true)));
            }
            groups.push(Json::Obj(g));
        }
        let mut glyph = Obj::new();
        glyph.set("advance", advance);
        glyph.set("groups", Json::Arr(groups));
        glyphs.set(ch.to_string(), glyph);
        order.push(Json::Str(ch.to_string()));
    }
    Ok((glyphs, order))
}

fn ints(v: &[i64]) -> Json {
    Json::Arr(v.iter().map(|&x| Json::Int(x)).collect())
}

fn strs(v: &[&str]) -> Json {
    Json::Arr(v.iter().map(|&s| Json::Str(s.into())).collect())
}

/// 'Pens' 500 bytes -> the pens500.json document.
pub fn decode(data: &[u8]) -> Result<Json, Error> {
    let text: Vec<char> = mac_roman(data).chars().collect();
    let (glyphs, order) = parse(&text)?;

    let mut units = Obj::new();
    units.set(
        "cell",
        "x,y in font units; point y is UP from the baseline, body 0..100 (a few points reach 101); arc-centre y is DOWN from the top of the em box",
    );
    units.set("line_height", 125.0);
    units.set("em_centre_y", 50.0);
    units.set("note", "runtime multiplies every coordinate by a single-precision scale in obj+0x540 (fn23 fits it to the screen; clamped 0.1..1.0)");

    let mut grammar = Obj::new();
    grammar.set(
        "[...]",
        "polyline; fn25 0x484C steps max(|dx|,|dy|)-1 dots per segment",
    );
    grammar.set(
        "{[-]cx,cy,...}",
        "arc run about (cx,cy); fn25 0x4B42 sweeps angle AND radius linearly, max(|dx|,|dy|)*6/5 dots per segment. A leading '-' is a reverse-sweep flag consumed by fn27 0x5D06, not a minus sign.",
    );
    grammar.set(
        "<n>",
        "advance width in font units (fn21 0x3D16 / fn22 0x3D84)",
    );

    let mut brushes = Obj::new();
    brushes.set("_", "DATA 129 word tables, 5x5, index = dx*5 + dy. 3 = no ink; 0/1/2 select ink[0..2] = palette base+0/+1/+2. fn19 0x3A70.");
    brushes.set(
        "0x075C_scale_lt_0.15",
        ints(&[
            3, 3, 3, 3, 3, 3, 0, 0, 3, 3, 3, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        ]),
    );
    brushes.set(
        "0x078E_scale_0.15_to_0.7",
        ints(&[
            3, 3, 3, 3, 3, 3, 2, 2, 2, 3, 3, 1, 0, 1, 3, 3, 1, 0, 1, 3, 3, 2, 2, 2, 3,
        ]),
    );
    brushes.set(
        "0x07C0_scale_gt_0.7",
        ints(&[
            3, 2, 2, 2, 3, 2, 1, 0, 1, 2, 2, 1, 0, 1, 2, 2, 1, 0, 1, 2, 3, 2, 2, 2, 3,
        ]),
    );
    brushes.set(
        "extent",
        "fn19 stamps 5x5 when scale >= 0.15, else 3x3 (stride stays 5)",
    );

    let mut ramps = Obj::new();
    ramps.set("0", strs(&["000000", "2f2f30", "6c6b6e"]));
    ramps.set("3", strs(&["870d0d", "943032", "a2585a"]));
    ramps.set("6", strs(&["285d1a", "3a692e", "798f76"]));
    ramps.set("9", strs(&["6cfd5d", "3f9336", "215027"]));
    ramps.set("12", strs(&["68b3ff", "4a81b9", "345a80"]));
    ramps.set("15", strs(&["fd3c3c", "bc3030", "832727"]));
    let mut ink = Obj::new();
    ink.set(
        "base_index",
        "DoBlank 0x2644: 3 * rand(3) at depth >= 8, else 0 (this+0x134); DoDrawFrame 0x28A0 adds 9 while the toggle this+0x13A is set",
    );
    ink.set(
        "palette",
        "clut 601 'Bathroom Pens 256' -- 18 live entries = 6 pens x 3 shades",
    );
    ink.set("ramps", ramps);

    let mut doc = Obj::new();
    doc.set(
        "source",
        "ripped/message-mayhem/Message Mayhem_Pens_500.bin ('Pens' 500)",
    );
    doc.set(
        "module",
        "Message Mayhem (After Dark 3.x, Totally Twisted, 1995)",
    );
    doc.set("units", units);
    doc.set("grammar", grammar);
    doc.set("brushes", brushes);
    doc.set("ink", ink);
    doc.set("glyph_order", Json::Arr(order));
    doc.set("glyphs", glyphs);
    Ok(Json::Obj(doc))
}
