//! Mac 'clut' colour table -> {slot: (r, g, b)} (port of decode/clut.py).
//! The ColorSpec `value` is the slot; later duplicates overwrite.

use std::collections::BTreeMap;

use crate::util::Be;

pub fn parse(data: &[u8]) -> BTreeMap<u16, [u8; 3]> {
    let mut out = BTreeMap::new();
    if data.len() < 8 {
        return out;
    }
    let ct_size = data.u16_at(6).unwrap() as usize;
    for i in 0..=ct_size {
        let off = 8 + i * 8;
        if off + 8 > data.len() {
            break;
        }
        let v = data.u16_at(off).unwrap();
        let r = data.u16_at(off + 2).unwrap();
        let g = data.u16_at(off + 4).unwrap();
        let b = data.u16_at(off + 6).unwrap();
        out.insert(v, [(r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8]);
    }
    out
}
