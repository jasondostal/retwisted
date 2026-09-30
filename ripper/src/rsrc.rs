//! Classic Mac resource-fork parser (port of tools/twistedrip/rsrc.py).
//!
//! The result keeps rsrc.py's dict semantics exactly -- insertion order =
//! type-list order then reference-list order, and a repeated (type, id)
//! keeps its first position but takes the later value -- because pack.py
//! iterates it to emit meta.json's `palettes` and `strings` in that order.

use std::collections::HashMap;

use crate::util::{latin1, mac_roman, Be};
use crate::Error;

#[derive(Clone, Debug)]
pub struct Resource {
    /// 4-char type as rsrc.py decodes it (latin-1), e.g. "RLEP", "snd ".
    pub rtype: String,
    pub id: i16,
    /// '' when unnamed (Mac Roman decoded).
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct ResMap {
    order: Vec<(String, i16)>,
    map: HashMap<(String, i16), Resource>,
}

impl ResMap {
    pub fn get(&self, rtype: &str, id: i16) -> Option<&Resource> {
        self.map.get(&(rtype.to_string(), id))
    }
    pub fn contains(&self, rtype: &str, id: i16) -> bool {
        self.map.contains_key(&(rtype.to_string(), id))
    }
    /// All resources in dict order.
    pub fn iter(&self) -> impl Iterator<Item = &Resource> {
        self.order.iter().map(|k| &self.map[k])
    }
    /// Resources of one type, in dict order.
    pub fn of_type<'a>(&'a self, rtype: &'a str) -> impl Iterator<Item = &'a Resource> + 'a {
        self.iter().filter(move |r| r.rtype == rtype)
    }
    pub fn len(&self) -> usize {
        self.order.len()
    }
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    fn insert(&mut self, r: Resource) {
        let key = (r.rtype.clone(), r.id);
        if !self.map.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.map.insert(key, r);
    }
}

pub fn parse(fork: &[u8]) -> Result<ResMap, Error> {
    let mut out = ResMap::default();
    if fork.is_empty() {
        return Ok(out);
    }
    let data_offset = fork.u32_at(0)? as usize;
    let map_offset = fork.u32_at(4)? as usize;
    let type_list_rel = fork.u16_at(map_offset + 24)? as usize;
    let name_list_rel = fork.u16_at(map_offset + 26)? as usize;
    let type_list = map_offset + type_list_rel;
    let num_types = (fork.u16_at(type_list)? as usize + 1) & 0xFFFF;
    for t in 0..num_types {
        let e = type_list + 2 + t * 8;
        let rtype = latin1(fork.bytes_at(e, 4)?);
        let num_items = fork.u16_at(e + 4)? as usize + 1;
        let ref_list_rel = fork.u16_at(e + 6)? as usize;
        let base = type_list_rel + map_offset + ref_list_rel;
        for x in 0..num_items {
            let r = base + x * 12;
            let id = fork.i16_at(r)?;
            let name_offset = fork.u16_at(r + 2)?;
            let attrs_and_offset = fork.u32_at(r + 4)?;
            let name = if name_offset != 0xFFFF {
                let at = map_offset + name_list_rel + name_offset as usize;
                let n = fork.u8_at(at)? as usize;
                mac_roman(crate::util::pyslice(fork, at + 1, at + 1 + n))
            } else {
                String::new()
            };
            let d = data_offset + (attrs_and_offset & 0x00FF_FFFF) as usize;
            let size = fork.u32_at(d)? as usize;
            let data = crate::util::pyslice(fork, d + 4, d + 4 + size).to_vec();
            out.insert(Resource {
                rtype: rtype.clone(),
                id,
                name,
                data,
            });
        }
    }
    Ok(out)
}
