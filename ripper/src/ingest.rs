//! Ingest: turn whatever the user has into {Mac file name: resource fork}.
//! Port of tools/twistedrip/ingest.py, minus its `unar` subprocess: the
//! StuffIt / BinHex / MacBinary unpacking unar did is in `container::*`.
//!
//! Detection is by content, in ingest.py's order:
//!   - a directory is walked as a folder of already-extracted files;
//!   - bytes that carry an HFS volume (the hybrid CD `.iso`) are read as
//!     HFS -- never as ISO 9660, which is the Windows side of the same disc;
//!   - anything else is unpacked as an archive (StuffIt 5, classic StuffIt,
//!     BinHex, MacBinary). If that yields a `*.iso` (the CD `.sit` wrapper),
//!     the first one (path order) is read as HFS instead.
//!
//! Within any of those, a file is kept only if its resource fork carries
//! After Dark module/sound/art resources and no host-software markers --
//! decided from the fork's type list, never from the file name.

use std::collections::HashMap;
use std::path::Path;

use crate::container::{binhex, folder, hfs, sit, MacFile};
use crate::util::Be;
use crate::Error;

const ASSET_MARKERS: [&[u8; 4]; 9] = [
    b"ADgm", b"RLEP", b"snd ", b"sndS", b"cmid", b"INST", b"csnd", b"PAT#", b"BOIL",
];
const INFRA_MARKERS: [&[u8; 4]; 5] = [b"cdev", b"CPLD", b"INIT", b"BNDL", b"FREF"];
const SKIP_NAMES: [&str; 3] = ["Icon\r", ".DS_Store", "__MACOSX"];

/// Python-dict-shaped result: insertion-ordered, a repeated key keeps its
/// first position and takes the newest value.
#[derive(Default, Debug, Clone)]
pub struct Forks {
    order: Vec<String>,
    map: HashMap<String, Vec<u8>>,
}

impl Forks {
    pub fn insert(&mut self, name: String, fork: Vec<u8>) {
        if !self.map.contains_key(&name) {
            self.order.push(name.clone());
        }
        self.map.insert(name, fork);
    }
    pub fn get(&self, name: &str) -> Option<&Vec<u8>> {
        self.map.get(name)
    }
    pub fn len(&self) -> usize {
        self.order.len()
    }
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.order
            .iter()
            .map(|k| (k.as_str(), self.map[k].as_slice()))
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }
    fn extend(&mut self, other: Forks) {
        for k in other.order {
            let v = other.map.get(&k).cloned().unwrap_or_default();
            self.insert(k, v);
        }
    }
}

/// The resource types listed in a resource fork's map (coarse sniff only).
pub fn resource_fork_types(fork: &[u8]) -> Vec<[u8; 4]> {
    let inner = || -> Result<Vec<[u8; 4]>, Error> {
        if fork.len() < 16 {
            return Ok(vec![]);
        }
        let map_off = fork.u32_at(4)? as usize;
        let map_len = fork.u32_at(12)? as usize;
        let map = crate::util::pyslice(fork, map_off, map_off.saturating_add(map_len));
        if map.len() < 28 {
            return Ok(vec![]);
        }
        let tl = map.u16_at(24)? as usize;
        if tl + 2 > map.len() {
            return Ok(vec![]);
        }
        let n = map.u16_at(tl)? as usize + 1;
        let mut out = Vec::new();
        let mut pos = tl + 2;
        for _ in 0..n {
            if pos + 4 > map.len() {
                break;
            }
            let mut t = [0u8; 4];
            t.copy_from_slice(&map[pos..pos + 4]);
            out.push(t);
            pos += 8;
        }
        Ok(out)
    };
    inner().unwrap_or_default()
}

pub fn is_asset_fork(fork: &[u8]) -> bool {
    if fork.is_empty() {
        return false;
    }
    let types = resource_fork_types(fork);
    if types.iter().any(|t| INFRA_MARKERS.contains(&t)) {
        return false;
    }
    types.iter().any(|t| ASSET_MARKERS.contains(&t))
}

fn skipped(path: &[String]) -> bool {
    let name = path.last().map(String::as_str).unwrap_or("");
    name.starts_with("._") || SKIP_NAMES.contains(&name) || path.iter().any(|p| p == "__MACOSX")
}

/// ingest.py `_harvest_tree` over an in-memory file list (what unar would
/// have written to disk): path order, later same-name files win.
fn harvest_files(files: &[MacFile]) -> Forks {
    let mut sorted: Vec<&MacFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut found = Forks::default();
    for f in sorted {
        if skipped(&f.path) {
            continue;
        }
        if is_asset_fork(&f.rsrc) {
            found.insert(f.name().to_string(), f.rsrc.clone());
        }
    }
    found
}

/// ingest.py `_harvest_tree` over a real folder.
pub fn harvest_dir(root: &Path) -> Forks {
    let mut found = Forks::default();
    for p in folder::walk_files(root) {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parts: Vec<String> = p
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        if skipped(&parts) {
            continue;
        }
        let fork = folder::read_fork(&p);
        if is_asset_fork(&fork) {
            found.insert(name, fork);
        }
    }
    found
}

fn is_macbinary(d: &[u8]) -> bool {
    d.len() >= 128 && d[0] == 0 && (1..=63).contains(&d[1]) && d[74] == 0 && d[82] == 0 && {
        let dl = u32::from_be_bytes([d[83], d[84], d[85], d[86]]) as usize;
        let rl = u32::from_be_bytes([d[87], d[88], d[89], d[90]]) as usize;
        128 + dl.div_ceil(128) * 128 + rl <= d.len() + 127
    }
}

fn macbinary(d: &[u8]) -> Result<MacFile, Error> {
    let nlen = d[1] as usize;
    let name = crate::util::mac_roman(&d[2..2 + nlen]);
    let dl = d.u32_at(83)? as usize;
    let rl = d.u32_at(87)? as usize;
    let data = crate::util::pyslice(d, 128, 128 + dl).to_vec();
    let rs = 128 + dl.div_ceil(128) * 128;
    let rsrc = crate::util::pyslice(d, rs, rs + rl).to_vec();
    let mut file_type = [0u8; 4];
    file_type.copy_from_slice(&d[65..69]);
    let mut creator = [0u8; 4];
    creator.copy_from_slice(&d[69..73]);
    Ok(MacFile {
        path: vec![name],
        file_type,
        creator,
        data,
        rsrc,
    })
}

/// What `unar` did for ingest.py: unpack an archive into its files. The
/// big `.iso` data fork of the CD wrapper is only inflated if asked for.
enum Unpacked {
    Files(Vec<MacFile>),
    /// StuffIt listing + the archive bytes it indexes (lazy forks).
    Sit(Vec<sit::SitFile>, Vec<u8>),
}

fn unpack_archive(bytes: Vec<u8>) -> Result<Unpacked, Error> {
    if sit::is_sit5(&bytes) {
        let list = sit::list_sit5(&bytes)?;
        return Ok(Unpacked::Sit(list, bytes));
    }
    if sit::is_classic(&bytes) {
        let list = sit::list_classic(&bytes)?;
        return Ok(Unpacked::Sit(list, bytes));
    }
    // Single-file wrappers: unar looks through them into an archive in the
    // data fork, else yields the wrapped file itself.
    let wrapped = if binhex::is_binhex(&bytes) {
        Some(binhex::decode(&bytes)?)
    } else if is_macbinary(&bytes) {
        Some(macbinary(&bytes)?)
    } else {
        None
    };
    match wrapped {
        Some(f) if sit::is_sit5(&f.data) || sit::is_classic(&f.data) => unpack_archive(f.data),
        Some(f) => Ok(Unpacked::Files(vec![f])),
        None => Err(Error::Container(
            "unrecognised input: not a folder, HFS image, StuffIt, BinHex or MacBinary file".into(),
        )),
    }
}

/// ingest.py `_ingest_hfs_image`.
fn ingest_hfs(image: &[u8], progress: &mut dyn FnMut(&str)) -> Result<Forks, Error> {
    let files = hfs::read_files(image)?;
    let mut found = Forks::default();
    for f in &files {
        let name = f.name().to_string();
        if is_asset_fork(&f.rsrc) {
            found.insert(name.clone(), f.rsrc.clone());
        }
        // The Mac installer is a StuffIt SE self-extractor (creator STi0):
        // the archive is its data fork.
        if &f.creator == b"STi0" && &f.file_type == b"APPL" {
            progress(&format!("unpacking installer \"{name}\""));
            let unpacked = match unpack_archive(f.data.clone()) {
                Ok(u) => u,
                Err(_) => continue, // unar found nothing it could extract
            };
            found.extend(harvest_unpacked(unpacked)?);
        }
    }
    Ok(found)
}

fn harvest_unpacked(u: Unpacked) -> Result<Forks, Error> {
    match u {
        Unpacked::Files(files) => Ok(harvest_files(&files)),
        Unpacked::Sit(list, bytes) => {
            // Only resource forks matter for harvesting; data forks stay packed.
            let mut files = Vec::with_capacity(list.len());
            for e in &list {
                files.push(MacFile {
                    path: e.path.clone(),
                    file_type: e.file_type,
                    creator: e.creator,
                    data: Vec::new(),
                    rsrc: e.rsrc.decompress(&bytes)?,
                });
            }
            Ok(harvest_files(&files))
        }
    }
}

/// Top level: path -> {module name: resource fork}.
pub fn ingest(path: &Path, progress: &mut dyn FnMut(&str)) -> Result<Forks, Error> {
    if path.is_dir() {
        progress("reading folder");
        return Ok(harvest_dir(path));
    }
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;

    // Probe first: an HFS/hybrid-CD image must never be treated as ISO 9660.
    if hfs::find_volume(&bytes).is_some() {
        if let Ok(found) = ingest_hfs(&bytes, progress) {
            progress("read HFS volume");
            return Ok(found);
        }
    }

    progress("unpacking archive");
    match unpack_archive(bytes)? {
        Unpacked::Sit(list, archive) => {
            let mut isos: Vec<&sit::SitFile> = list
                .iter()
                .filter(|e| e.path.last().is_some_and(|n| n.ends_with(".iso")))
                .collect();
            isos.sort_by(|a, b| a.path.cmp(&b.path));
            if let Some(iso) = isos.first() {
                progress("inflating CD image");
                let image = iso.data.decompress(&archive)?;
                progress("reading HFS volume");
                return ingest_hfs(&image, progress);
            }
            harvest_unpacked(Unpacked::Sit(list, archive))
        }
        files @ Unpacked::Files(_) => harvest_unpacked(files),
    }
}
