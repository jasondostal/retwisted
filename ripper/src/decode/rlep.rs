//! After Dark 'RLEP' sprite banks (Library 4.0 / DynaModules): the RLID
//! chunk container, the CSTM run-length opcode stream, the CTAB colour
//! table and IHDR frame rects. Port of tools/twistedrip/decode/rlep.py --
//! the chunk layout and opcode table are documented there.

use crate::util::Be;

pub const SENTINEL: u32 = 0x0AED_F8F8;

/// Mac Rect order: (top, left, bottom, right).
pub type Rect = (i16, i16, i16, i16);

#[derive(Clone, Debug)]
pub struct Run {
    pub x: usize,
    pub px: Vec<u8>,
}

pub type Row = Vec<Run>;

/// (tag, param, flags, payload) for each 16-byte-headed chunk.
pub fn chunks(data: &[u8]) -> Vec<([u8; 4], u32, u32, &[u8])> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 16 <= data.len() {
        let tag = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
        if u32::from_be_bytes(tag) == SENTINEL {
            break;
        }
        let param = data.u32_at(pos + 4).unwrap();
        let flags = data.u32_at(pos + 8).unwrap();
        let size = data.u32_at(pos + 12).unwrap() as usize;
        if size < 16 {
            break;
        }
        out.push((
            tag,
            param,
            flags,
            crate::util::pyslice(data, pos + 16, pos + size),
        ));
        pos += size;
    }
    out
}

/// Decode one CSTM stream into rows of runs. Running off the end of the
/// stream ends decoding like rlep.py's `except IndexError`: the row in
/// progress is kept if it has any runs; a run being assembled is dropped.
pub fn decode_rows(stream: &[u8]) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut row: Row = Vec::new();
    let mut x = 0usize;
    let n = stream.len();
    let mut i = 0usize;

    macro_rules! get {
        ($idx:expr) => {
            match stream.get($idx) {
                Some(&b) => b,
                None => {
                    if !row.is_empty() {
                        rows.push(row);
                    }
                    return rows;
                }
            }
        };
    }
    let emit = |row: &mut Row, x: &mut usize, px: Vec<u8>| {
        if !px.is_empty() {
            let len = px.len();
            row.push(Run { x: *x, px });
            *x += len;
        }
    };

    while i < n {
        let b = stream[i];
        i += 1;
        let (op, h) = (b & 0xF, b >> 4);
        match op {
            0 => {
                if h == 0 || h == 1 {
                    rows.push(std::mem::take(&mut row));
                    x = 0;
                    if h == 0 {
                        break;
                    }
                } else if h == 3 {
                    let r = ((get!(i) as usize) << 8) | get!(i + 1) as usize;
                    i += 2;
                    let copy = if r < rows.len() {
                        rows[r].clone()
                    } else {
                        Vec::new()
                    };
                    rows.push(copy);
                    row = Vec::new();
                    x = 0;
                }
            }
            1 => {
                let cnt = if h != 0 { h as usize } else { get!(i) as usize };
                if h == 0 {
                    i += 1;
                }
                x += cnt;
            }
            2 => {
                let cnt = get!(i) as usize;
                i += 1;
                emit(&mut row, &mut x, vec![h; cnt]);
            }
            3..=5 => emit(&mut row, &mut x, vec![h; (op - 2) as usize]),
            6 => {
                let c = get!(i);
                i += 1;
                let mut cnt = h as usize;
                if cnt == 0 {
                    cnt = get!(i) as usize;
                    i += 1;
                }
                emit(&mut row, &mut x, vec![c; cnt]);
            }
            7 => {
                if h == 0 {
                    let mut cnt = get!(i) as usize;
                    i += 1;
                    let mut px = Vec::with_capacity(cnt);
                    while cnt >= 2 {
                        let v = get!(i);
                        i += 1;
                        px.push(v >> 4);
                        px.push(v & 0xF);
                        cnt -= 2;
                    }
                    if cnt == 1 {
                        px.push(get!(i) >> 4);
                        i += 1;
                    }
                    emit(&mut row, &mut x, px);
                } else if h == 1 {
                    let cnt = get!(i) as usize;
                    i += 1;
                    let px = crate::util::pyslice(stream, i, i + cnt).to_vec();
                    emit(&mut row, &mut x, px);
                    i += cnt;
                }
            }
            8 => {
                let mut cnt = h as usize;
                if cnt == 0 {
                    cnt = get!(i) as usize;
                    i += 1;
                }
                let v = get!(i);
                i += 1;
                let mut px = Vec::with_capacity(cnt);
                for _ in 0..cnt / 2 {
                    px.push(v >> 4);
                    px.push(v & 0xF);
                }
                if cnt & 1 != 0 {
                    px.push(v >> 4);
                }
                emit(&mut row, &mut x, px);
            }
            9 => {
                let mut cnt = h as usize;
                if cnt == 0 {
                    cnt = get!(i) as usize;
                    i += 1;
                }
                let (c1, c2) = (get!(i), get!(i + 1));
                i += 2;
                let mut px = Vec::with_capacity(cnt);
                for _ in 0..cnt / 2 {
                    px.push(c1);
                    px.push(c2);
                }
                if cnt & 1 != 0 {
                    px.push(c1);
                }
                emit(&mut row, &mut x, px);
            }
            _ => {}
        }
    }
    rows
}

pub type Ctab = Vec<[u8; 3]>;

pub fn parse_ctab(param: u32, payload: &[u8]) -> Ctab {
    let count = ((param >> 8) & 0xFF) as usize;
    (0..count)
        .map(|i| {
            let s = crate::util::pyslice(payload, i * 4 + 1, i * 4 + 4);
            let mut c = [0u8; 3];
            c[..s.len()].copy_from_slice(s);
            c
        })
        .collect()
}

#[inline]
pub fn color(ctab: &Ctab, v: u8) -> [u8; 3] {
    ctab.get(v as usize).copied().unwrap_or([255, 0, 255])
}

/// A decoded bank: CTAB + frames {id: (rect, stream)} in dict order.
pub struct Bank<'a> {
    pub ctab: Ctab,
    pub frames: Vec<(u16, Option<Rect>, &'a [u8])>,
}

pub fn parse_bank(data: &[u8]) -> Bank<'_> {
    let mut ctab = Vec::new();
    let mut streams: Vec<(u16, &[u8])> = Vec::new();
    let mut rects: std::collections::HashMap<u16, Rect> = std::collections::HashMap::new();
    for (tag, param, _flags, payload) in chunks(data) {
        match &tag {
            b"CSTM" => {
                let id = (param & 0xFFFF) as u16;
                if let Some(s) = streams.iter_mut().find(|(k, _)| *k == id) {
                    s.1 = payload;
                } else {
                    streams.push((id, payload));
                }
            }
            b"CTAB" => ctab = parse_ctab(param, payload),
            b"IHDR" => {
                if let (Ok(t), Ok(l), Ok(b), Ok(r)) = (
                    payload.i16_at(4),
                    payload.i16_at(6),
                    payload.i16_at(8),
                    payload.i16_at(10),
                ) {
                    rects.insert((param & 0xFFFF) as u16, (t, l, b, r));
                }
            }
            _ => {}
        }
    }
    let frames = streams
        .into_iter()
        .map(|(id, s)| (id, rects.get(&id).copied(), s))
        .collect();
    Bank { ctab, frames }
}

/// An RGBA image waiting to be PNG-encoded.
#[derive(Clone, Debug)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// rlep.py `render`: rows/ctab -> image, or None if empty. The IHDR rect is
/// authoritative for the size (trailing transparent runs are not encoded).
pub fn render(rows: &[Row], ctab: &Ctab, rect: Option<Rect>) -> Option<Image> {
    let mut h = rows.len() as i64;
    let mut w = rows
        .iter()
        .flat_map(|r| r.iter().map(|run| (run.x + run.px.len()) as i64))
        .max()
        .unwrap_or(0);
    if let Some((t, l, b, r)) = rect {
        w = w.max(r as i64 - l as i64);
        h = h.max(b as i64 - t as i64);
    }
    if w == 0 || h == 0 {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let mut rgba = vec![0u8; w * h * 4];
    for (y, r) in rows.iter().enumerate() {
        for run in r {
            for (j, &v) in run.px.iter().enumerate() {
                let o = (y * w + run.x + j) * 4;
                let c = color(ctab, v);
                rgba[o..o + 3].copy_from_slice(&c);
                rgba[o + 3] = 0xFF;
            }
        }
    }
    Some(Image {
        w: w as u32,
        h: h as u32,
        rgba,
    })
}
