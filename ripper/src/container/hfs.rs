//! Read-only HFS (Mac OS Standard) volume reader -- the subset of what the
//! Python ripper gets from `machfs.Volume.read()`: locate the Master
//! Directory Block, walk the catalog B*-tree's leaf chain, and pull each
//! file's data and resource forks through its extent records (plus the
//! extents-overflow tree for fragmented forks).
//!
//! Like machfs, the MDB is found by scanning 512-byte steps for the `BD`
//! signature 1024 bytes in, which is how a hybrid CD's HFS partition (behind
//! an Apple Partition Map, sharing the disc with an ISO 9660 side) is found
//! without parsing the partition map -- and the ISO 9660 side is never read.

use super::MacFile;
use crate::util::{mac_roman, Be};
use crate::Error;
use std::collections::HashMap;

/// Offset of the HFS volume inside `image`, or None if there is none.
pub fn find_volume(image: &[u8]) -> Option<usize> {
    let mut i = 0usize;
    while i + 1026 <= image.len() {
        if &image[i + 1024..i + 1026] == b"BD" {
            return Some(i);
        }
        i += 512;
    }
    None
}

struct Vol<'a> {
    img: &'a [u8],
    al_bl_st: usize,
    al_blk_siz: usize,
    overflow: HashMap<(u32, u8, u16), [u8; 12]>,
}

impl<'a> Vol<'a> {
    fn block_off(&self, block: usize) -> usize {
        512 * self.al_bl_st + self.al_blk_siz * block
    }

    fn extents(rec: &[u8]) -> Vec<(usize, usize)> {
        (0..3)
            .map(|k| {
                (
                    u16::from_be_bytes([rec[k * 4], rec[k * 4 + 1]]) as usize,
                    u16::from_be_bytes([rec[k * 4 + 2], rec[k * 4 + 3]]) as usize,
                )
            })
            .filter(|&(_, n)| n != 0)
            .collect()
    }

    /// machfs `getfork`: gather every extent (first record, then overflow
    /// records keyed by the fork's starting allocation block), concatenate,
    /// and cut to the logical length.
    fn fork(&self, size: usize, first: &[u8], cnid: u32, fork: u8) -> Result<Vec<u8>, Error> {
        let nblocks = size.div_ceil(self.al_blk_siz);
        let mut list = Vec::new();
        let mut accum = 0usize;
        for (a, b) in Self::extents(first) {
            accum += b;
            list.push((a, b));
        }
        while accum < nblocks {
            let rec = self
                .overflow
                .get(&(cnid, fork, accum as u16))
                .ok_or_else(|| {
                    Error::Container(format!(
                        "HFS: missing extents-overflow record for CNID {cnid}"
                    ))
                })?;
            let more = Self::extents(rec);
            if more.is_empty() {
                return Err(Error::Container(
                    "HFS: empty extents-overflow record".into(),
                ));
            }
            for (a, b) in more {
                accum += b;
                list.push((a, b));
            }
        }
        let mut out = Vec::with_capacity(size);
        for (a, b) in list {
            let s = self.block_off(a);
            let e = self.block_off(a + b);
            out.extend_from_slice(crate::util::pyslice(self.img, s, e));
            if out.len() >= size {
                break;
            }
        }
        out.truncate(size);
        Ok(out)
    }
}

/// All leaf records of an HFS B*-tree (machfs `dump_btree`), in leaf-chain order.
fn leaf_records(tree: &[u8]) -> Result<Vec<&[u8]>, Error> {
    let node = |start: usize| -> Result<(u32, Vec<&[u8]>), Error> {
        let flink = tree.u32_at(start)?;
        let nrecs = tree.u16_at(start + 10)? as usize;
        let mut offs = Vec::with_capacity(nrecs + 1);
        for k in 0..=nrecs {
            offs.push(tree.u16_at(start + 512 - 2 * (k + 1))? as usize);
        }
        let mut recs = Vec::with_capacity(nrecs);
        for k in 0..nrecs {
            recs.push(crate::util::pyslice(
                tree,
                start + offs[k],
                start + offs[k + 1],
            ));
        }
        Ok((flink, recs))
    };
    let (_, header) = node(0)?;
    let hdr = header
        .first()
        .ok_or_else(|| Error::Container("HFS: empty B-tree header node".into()))?;
    let first_leaf = hdr.u32_at(10)? as usize;
    let last_leaf = hdr.u32_at(14)? as usize;
    let mut out = Vec::new();
    let mut leaf = first_leaf;
    let mut guard = 0usize;
    loop {
        let (flink, recs) = node(512 * leaf)?;
        out.extend(recs);
        if leaf == last_leaf {
            break;
        }
        leaf = flink as usize;
        guard += 1;
        if guard > tree.len() / 512 + 1 {
            return Err(Error::Container("HFS: B-tree leaf chain loops".into()));
        }
    }
    Ok(out)
}

/// Every file on the volume, in machfs `iter_paths()` order (catalog order,
/// depth-first, each folder's children in the order the catalog lists them).
pub fn read_files(image: &[u8]) -> Result<Vec<MacFile>, Error> {
    let base = find_volume(image).ok_or_else(|| Error::Container("no HFS volume".into()))?;
    let img = &image[base..];
    let m = 1024;
    let al_blk_siz = img.u32_at(m + 20)? as usize;
    let al_bl_st = img.u16_at(m + 28)? as usize;
    if al_blk_siz == 0 || !al_blk_siz.is_multiple_of(512) {
        return Err(Error::Container("HFS: bad allocation block size".into()));
    }
    let xt_size = img.u32_at(m + 130)? as usize;
    let xt_rec = img.bytes_at(m + 134, 12)?.to_vec();
    let ct_size = img.u32_at(m + 146)? as usize;
    let ct_rec = img.bytes_at(m + 150, 12)?.to_vec();

    let mut vol = Vol {
        img,
        al_bl_st,
        al_blk_siz,
        overflow: HashMap::new(),
    };
    let xt = vol.fork(xt_size, &xt_rec, 3, 0)?;
    let mut overflow = HashMap::new();
    for rec in leaf_records(&xt)? {
        if rec.first() != Some(&7) {
            continue;
        }
        let fk_type = rec.u8_at(1)?;
        let fnum = rec.u32_at(2)?;
        let fabn = rec.u16_at(6)?;
        let mut ext = [0u8; 12];
        ext.copy_from_slice(rec.bytes_at(8, 12)?);
        let fork = if fk_type == 0xFF { 1 } else { 0 };
        overflow.insert((fnum, fork, fabn), ext);
    }
    vol.overflow = overflow;
    let ct = vol.fork(ct_size, &ct_rec, 4, 0)?;

    enum Node {
        Dir,
        File(Box<MacFile>),
    }
    // cnid -> ordered children (name, cnid); plus the objects.
    let mut children: HashMap<u32, Vec<(String, u32)>> = HashMap::new();
    let mut objects: HashMap<u32, Node> = HashMap::new();
    for rec in leaf_records(&ct)? {
        let key_len = rec.u8_at(0)? as usize;
        if key_len == 0 {
            continue;
        }
        let key = crate::util::pyslice(rec, 2, 1 + key_len);
        let val_off = (1 + key_len).next_multiple_of(2);
        let val = crate::util::pyslice(rec, val_off, rec.len());
        let parent = key.u32_at(0)?;
        let nlen = key.u8_at(4)? as usize;
        let name = mac_roman(crate::util::pyslice(key, 5, 5 + nlen));
        let datarec = crate::util::pyslice(val, 2, val.len());
        match val.first() {
            Some(1) => {
                let dir_id = datarec.u32_at(4)?;
                objects.insert(dir_id, Node::Dir);
                children.entry(parent).or_default().push((name, dir_id));
            }
            Some(2) => {
                let mut file_type = [0u8; 4];
                file_type.copy_from_slice(datarec.bytes_at(2, 4)?);
                let mut creator = [0u8; 4];
                creator.copy_from_slice(datarec.bytes_at(6, 4)?);
                let fl_num = datarec.u32_at(18)?;
                let lg_len = datarec.u32_at(24)? as usize;
                let r_lg_len = datarec.u32_at(34)? as usize;
                let ext_rec = datarec.bytes_at(72, 12)?;
                let r_ext_rec = datarec.bytes_at(84, 12)?;
                let data = vol.fork(lg_len, ext_rec, fl_num, 0)?;
                let rsrc = vol.fork(r_lg_len, r_ext_rec, fl_num, 1)?;
                objects.insert(
                    fl_num,
                    Node::File(Box::new(MacFile {
                        path: Vec::new(),
                        file_type,
                        creator,
                        data,
                        rsrc,
                    })),
                );
                children.entry(parent).or_default().push((name, fl_num));
            }
            _ => {}
        }
    }

    // machfs folders are dicts: a repeated (case-insensitively equal) name
    // replaces the earlier child in place. Catalog keys are unique per
    // parent, so plain insertion order is enough here.
    let mut out = Vec::new();
    fn walk(
        cnid: u32,
        prefix: &mut Vec<String>,
        top: bool,
        children: &HashMap<u32, Vec<(String, u32)>>,
        objects: &mut HashMap<u32, Node>,
        out: &mut Vec<MacFile>,
    ) {
        let Some(kids) = children.get(&cnid) else {
            return;
        };
        for (name, id) in kids {
            if top && matches!(name.as_str(), "Desktop" | "Desktop DB" | "Desktop DF") {
                continue;
            }
            prefix.push(name.clone());
            match objects.remove(id) {
                Some(Node::File(mut f)) => {
                    f.path = prefix.clone();
                    out.push(*f);
                }
                Some(Node::Dir) => walk(*id, prefix, false, children, objects, out),
                None => {}
            }
            prefix.pop();
        }
    }
    walk(2, &mut Vec::new(), true, &children, &mut objects, &mut out);
    Ok(out)
}
