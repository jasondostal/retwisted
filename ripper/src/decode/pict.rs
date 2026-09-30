//! QuickDraw 'PICT' v2 -> RGBA, for the faceplate banner only: one
//! PackBitsRect PixMap (1/2/4/8-bit indexed). Port of decode/pict.py; the
//! deliberately narrow scope is documented there.

use std::collections::HashMap;

use crate::util::{pyslice, Be};
use crate::Error;

fn fixed_len(op: u16) -> Option<usize> {
    Some(match op {
        0x0000 => 0,
        0x0011 => 2,
        0x001E => 0,
        0x001C => 0,
        0x0003 => 2,
        0x0004 => 1,
        0x0005 => 2,
        0x0008 => 2,
        0x000D => 2,
        0x001A => 6,
        0x001B => 6,
        0x00FF => 0,
        _ => return None,
    })
}

fn unpack_bits(
    data: &[u8],
    mut pos: usize,
    row_bytes: usize,
    out_len: usize,
) -> Result<(Vec<u8>, usize), Error> {
    let n;
    if row_bytes > 250 {
        n = data.u16_at(pos)? as usize;
        pos += 2;
    } else {
        n = data.u8_at(pos)? as usize;
        pos += 1;
    }
    let end = pos + n;
    let mut out = Vec::with_capacity(out_len);
    while pos < end && out.len() < out_len {
        let flag = data.u8_at(pos)?;
        pos += 1;
        if flag & 0x80 != 0 {
            let b = data.u8_at(pos)?;
            out.extend(std::iter::repeat_n(b, 257 - flag as usize));
            pos += 1;
        } else {
            out.extend_from_slice(pyslice(data, pos, pos + flag as usize + 1));
            pos += flag as usize + 1;
        }
    }
    out.truncate(out_len);
    Ok((out, end))
}

/// 'PICT' bytes -> (w, h, rgba).
pub fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u8>), Error> {
    if data.len() < 12 {
        return Err(Error::Decode("PICT too short".into()));
    }
    let (top, left, bottom, right) = (
        data.i16_at(2)?,
        data.i16_at(4)?,
        data.i16_at(6)?,
        data.i16_at(8)?,
    );
    let (pic_w, pic_h) = (right as i64 - left as i64, bottom as i64 - top as i64);
    let mut pos = 10usize;
    let n = data.len();
    let mut result = None;
    while pos + 2 <= n {
        let op = data.u16_at(pos)?;
        pos += 2;
        match op {
            0x00FF => break,
            0x0C00 => pos += 24,
            0x00A0 => pos += 2,
            0x00A1 => {
                let size = data.u16_at(pos + 2)? as usize;
                pos += 4 + size + (size & 1);
            }
            0x0001 => pos += data.u16_at(pos)? as usize,
            0x0007 => pos += 4,
            0x0008 => pos += 2,
            0x0098 => {
                let mut row_bytes = data.u16_at(pos)? as usize;
                pos += 2;
                if row_bytes & 0x8000 == 0 {
                    return Err(Error::Decode(
                        "PackBitsRect with a BitMap, not a PixMap".into(),
                    ));
                }
                row_bytes &= 0x3FFF;
                let (bt, bl, bb, br) = (
                    data.i16_at(pos)?,
                    data.i16_at(pos + 2)?,
                    data.i16_at(pos + 4)?,
                    data.i16_at(pos + 6)?,
                );
                pos += 8;
                // PixMap: version, packType, packSize, hRes, vRes, pixelType, pixelSize, ...
                data.bytes_at(pos, 36)?;
                let depth = data.u16_at(pos + 18)? as usize;
                pos += 36;
                // Colour table.
                let flags = data.u16_at(pos + 4)?;
                let cn = data.u16_at(pos + 6)? as usize;
                pos += 8;
                let mut ctab: HashMap<u16, [u8; 3]> = HashMap::new();
                for i in 0..=cn {
                    let v = data.u16_at(pos)?;
                    let (r, g, b) = (
                        data.u16_at(pos + 2)?,
                        data.u16_at(pos + 4)?,
                        data.u16_at(pos + 6)?,
                    );
                    pos += 8;
                    let key = if flags & 0x8000 != 0 { i as u16 } else { v };
                    ctab.insert(key, [(r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8]);
                }
                pos += 8 + 8 + 2;
                let (w, h) = (br as i64 - bl as i64, bb as i64 - bt as i64);
                if ![1, 2, 4, 8].contains(&depth) {
                    return Err(Error::Decode(format!(
                        "PICT pixel size {depth} not supported"
                    )));
                }
                if w < 0 || h < 0 {
                    return Err(Error::Decode("PICT negative bounds".into()));
                }
                let mut rows = Vec::with_capacity(h as usize);
                for _ in 0..h {
                    if row_bytes < 8 {
                        rows.push(pyslice(data, pos, pos + row_bytes).to_vec());
                        pos += row_bytes;
                    } else {
                        let (raw, np) = unpack_bits(data, pos, row_bytes, row_bytes)?;
                        rows.push(raw);
                        pos = np;
                    }
                }
                result = Some((
                    w as usize,
                    h as usize,
                    to_rgba(&rows, w as usize, h as usize, depth, &ctab),
                ));
            }
            0x0099 | 0x009A => {
                return Err(Error::Decode(format!(
                    "PICT opcode {op:#06x} not supported"
                )))
            }
            _ => match fixed_len(op) {
                Some(l) => pos += l,
                None => {
                    return Err(Error::Decode(format!(
                        "PICT opcode {op:#06x} not supported"
                    )))
                }
            },
        }
    }
    let (w, h, rgba) =
        result.ok_or_else(|| Error::Decode("PICT contains no PackBitsRect".into()))?;
    if (w as i64, h as i64) != (pic_w, pic_h) {
        return Err(Error::Decode(format!(
            "PICT frame {pic_w}x{pic_h} != pixmap {w}x{h}"
        )));
    }
    Ok((w as u32, h as u32, rgba))
}

fn to_rgba(
    rows: &[Vec<u8>],
    w: usize,
    h: usize,
    depth: usize,
    ctab: &HashMap<u16, [u8; 3]>,
) -> Vec<u8> {
    let mut out = vec![0u8; w * h * 4];
    let per_byte = 8 / depth;
    let mask = (1u16 << depth) - 1;
    for (y, raw) in rows.iter().enumerate().take(h) {
        for x in 0..w {
            let v: u16 = if depth == 8 {
                raw.get(x).copied().unwrap_or(0) as u16
            } else {
                let byte = raw.get(x / per_byte).copied().unwrap_or(0) as u16;
                let shift = (per_byte - 1 - (x % per_byte)) * depth;
                (byte >> shift) & mask
            };
            let c = ctab.get(&v).copied().unwrap_or([255, 0, 255]);
            let o = (y * w + x) * 4;
            out[o..o + 3].copy_from_slice(&c);
            out[o + 3] = 255;
        }
    }
    out
}
