//! OFst/OFtb compound-frame tables and compound composition. Port of
//! tools/twistedrip/decode/oftb.py (field layout, mirror bits and Coming
//! Soon's shadow-channel rule are documented there).

use std::collections::HashMap;
use std::sync::Arc;

use super::rlep::{self, Image};
use crate::util::Be;
use crate::Error;

pub const FLIP_H: u16 = 1;
pub const FLIP_V: u16 = 2;

/// Channels drawn as a black silhouette, keyed by the module's Mac name.
pub fn shadow_channels(module_name: &str) -> &'static [u16] {
    match module_name {
        "Coming Soon" => &[3, 4, 5, 6, 7, 8],
        _ => &[],
    }
}

/// (frameNum, frameOffset, dx, dy) records of an OFst.
pub fn parse_ofst(data: &[u8]) -> Result<Vec<(u16, u32, i16, i16)>, Error> {
    let count = data.u16_at(12)? as usize;
    let mut recs = Vec::with_capacity(count);
    for i in 0..count {
        let o = 14 + i * 10;
        recs.push((
            data.u16_at(o)?,
            data.u32_at(o + 2)?,
            data.i16_at(o + 6)?,
            data.i16_at(o + 8)?,
        ));
    }
    Ok(recs)
}

/// XRect (x1, y1, x2, y2).
pub type XRect = (i16, i16, i16, i16);

#[derive(Clone, Debug)]
pub struct Part {
    pub art: u16,
    pub chan: u16,
    pub flags: u16,
    pub rect: XRect,
}

pub fn parse_oftb_entry(data: &[u8], off: usize) -> Result<(XRect, Vec<Part>), Error> {
    let bounds = (
        data.i16_at(off)?,
        data.i16_at(off + 2)?,
        data.i16_at(off + 4)?,
        data.i16_at(off + 6)?,
    );
    let n = data.u16_at(off + 8)? as usize;
    let mut items = Vec::with_capacity(n);
    for j in 0..n {
        let o = off + 0x0A + j * 14;
        items.push(Part {
            art: data.u16_at(o)?,
            chan: data.u16_at(o + 2)?,
            flags: data.u16_at(o + 4)?,
            rect: (
                data.i16_at(o + 6)?,
                data.i16_at(o + 8)?,
                data.i16_at(o + 10)?,
                data.i16_at(o + 12)?,
            ),
        });
    }
    Ok((bounds, items))
}

/// An art frame rasterised to its IHDR size: None = transparent.
#[derive(Clone, Debug)]
pub struct Grid {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<[u8; 3]>>,
}

impl Grid {
    fn flipped(&self, flags: u16, shadow: bool) -> Grid {
        let mut out = vec![None; self.w * self.h];
        for y in 0..self.h {
            let sy = if flags & FLIP_V != 0 {
                self.h - 1 - y
            } else {
                y
            };
            for x in 0..self.w {
                let sx = if flags & FLIP_H != 0 {
                    self.w - 1 - x
                } else {
                    x
                };
                let p = self.px[sy * self.w + sx];
                out[y * self.w + x] = if shadow { p.map(|_| [0, 0, 0]) } else { p };
            }
        }
        Grid {
            w: self.w,
            h: self.h,
            px: out,
        }
    }
}

/// oftb.py `frame_pixels`.
pub fn frame_pixels(rows: &[rlep::Row], ctab: &rlep::Ctab, w: usize, h: usize) -> Grid {
    let mut px = vec![None; w * h];
    for (y, r) in rows.iter().take(h).enumerate() {
        for run in r {
            for (j, &v) in run.px.iter().enumerate() {
                if run.x + j < w {
                    px[y * w + run.x + j] = Some(rlep::color(ctab, v));
                }
            }
        }
    }
    Grid { w, h, px }
}

/// Every frame of one bank rasterised at its IHDR size, plus its sorted ids.
pub struct BankArt {
    pub ids: Vec<u16>,
    pub frames: Vec<(u16, Arc<Grid>)>,
}

pub fn bank_art(data: &[u8]) -> Result<BankArt, Error> {
    let bank = rlep::parse_bank(data);
    let mut ids: Vec<u16> = bank.frames.iter().map(|f| f.0).collect();
    ids.sort_unstable();
    let mut frames = Vec::with_capacity(bank.frames.len());
    for (fid, rect, stream) in &bank.frames {
        let (t, l, b, r) =
            rect.ok_or_else(|| Error::Decode(format!("RLEP frame {fid} has no IHDR rect")))?;
        let w = (r as i64 - l as i64).max(0) as usize;
        let h = (b as i64 - t as i64).max(0) as usize;
        frames.push((
            *fid,
            Arc::new(frame_pixels(&rlep::decode_rows(stream), &bank.ctab, w, h)),
        ));
    }
    Ok(BankArt { ids, frames })
}

/// oftb.py `load_art_chain`: frames by id from bank `base` and the
/// ascending-id banks whose ids continue past the chain's tail.
/// `banks` is (bank id, lazily-built art) sorted by id.
pub fn load_art_chain(
    banks: &[(i16, &[u8])],
    cache: &mut HashMap<i16, Arc<BankArt>>,
    base: i16,
) -> Result<HashMap<u16, Arc<Grid>>, Error> {
    let mut art = HashMap::new();
    let mut tail: i64 = -1;
    for &(bid, data) in banks {
        if bid < base {
            continue;
        }
        let ba = match cache.get(&bid) {
            Some(b) => b.clone(),
            None => {
                let b = Arc::new(bank_art(data)?);
                cache.insert(bid, b.clone());
                b
            }
        };
        if ba.ids.is_empty() {
            continue;
        }
        if bid != base && (ba.ids[0] as i64) <= tail {
            continue;
        }
        for (fid, g) in &ba.frames {
            art.insert(*fid, g.clone());
        }
        tail = *ba.ids.last().unwrap() as i64;
    }
    Ok(art)
}

pub struct Compound {
    pub bounds: XRect,
    pub parts: Vec<Part>,
    pub image: Option<Image>,
}

/// oftb.py `compose_compound`.
pub fn compose_compound(
    art: &HashMap<u16, Arc<Grid>>,
    tb: &[u8],
    off: usize,
    shadow: &[u16],
) -> Result<Compound, Error> {
    let (bounds, items) = parse_oftb_entry(tb, off)?;
    let (bl, bt, br, bb) = (
        bounds.0 as i64,
        bounds.1 as i64,
        bounds.2 as i64,
        bounds.3 as i64,
    );
    let (w, h) = (br - bl, bb - bt);
    let (cw, ch) = (w.max(0) as usize, h.max(0) as usize);
    let mut canvas: Vec<Option<[u8; 3]>> = vec![None; cw * ch];
    for p in &items {
        if p.art == 0 {
            continue; // 0-based id -1: never present
        }
        let shade = shadow.contains(&p.chan);
        let Some(entry) = art.get(&(p.art - 1)) else {
            continue;
        };
        let flipped;
        let grid: &Grid = if p.flags == 0 && !shade {
            entry
        } else {
            flipped = entry.flipped(p.flags, shade);
            &flipped
        };
        let (il, it, ir, ib) = (
            p.rect.0 as i64,
            p.rect.1 as i64,
            p.rect.2 as i64,
            p.rect.3 as i64,
        );
        let ymax = (grid.h as i64).min(ib - it);
        let xmax = (grid.w as i64).min(ir - il);
        for y in 0..ymax.max(0) {
            let cy = it - bt + y;
            if !(0..h).contains(&cy) {
                continue;
            }
            for x in 0..xmax.max(0) {
                let cx = il - bl + x;
                if (0..w).contains(&cx) {
                    if let Some(c) = grid.px[y as usize * grid.w + x as usize] {
                        canvas[cy as usize * cw + cx as usize] = Some(c);
                    }
                }
            }
        }
    }
    let image = if w != 0 && h != 0 {
        if w < 0 || h < 0 {
            return Err(Error::Decode("compound with negative bounds".into()));
        }
        let mut rgba = vec![0u8; cw * ch * 4];
        for (i, p) in canvas.iter().enumerate() {
            if let Some(c) = p {
                rgba[i * 4..i * 4 + 3].copy_from_slice(c);
                rgba[i * 4 + 3] = 0xFF;
            }
        }
        Some(Image {
            w: cw as u32,
            h: ch as u32,
            rgba,
        })
    } else {
        None
    };
    Ok(Compound {
        bounds,
        parts: items,
        image,
    })
}
