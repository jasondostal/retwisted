//! A folder of already-extracted Mac files on a modern disk: resource forks
//! come from the native APFS/HFS+ named fork (`<file>/..namedfork/rsrc`) or,
//! failing that, an AppleDouble `._<file>` sidecar.

use std::path::{Path, PathBuf};

use crate::util::Be;

/// Every regular file under `root` (not following directory symlinks),
/// sorted the way Python sorts `Path` objects: component-wise.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            let Ok(ft) = entry.file_type() else { continue };
            if ft.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort_by(|a, b| {
        let ca: Vec<_> = a
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let cb: Vec<_> = b
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        ca.cmp(&cb)
    });
    out
}

/// The resource-fork entry (id 2) of an AppleDouble (or AppleSingle) blob.
pub fn appledouble_rsrc(blob: &[u8]) -> Vec<u8> {
    if blob.len() < 26
        || !(blob.starts_with(b"\x00\x05\x16\x07") || blob.starts_with(b"\x00\x05\x16\x00"))
    {
        return Vec::new();
    }
    let n = blob.u16_at(24).unwrap_or(0) as usize;
    let mut pos = 26;
    for _ in 0..n {
        if pos + 12 > blob.len() {
            break;
        }
        let id = blob.u32_at(pos).unwrap_or(0);
        let off = blob.u32_at(pos + 4).unwrap_or(0) as usize;
        let len = blob.u32_at(pos + 8).unwrap_or(0) as usize;
        if id == 2 {
            return crate::util::pyslice(blob, off, off.saturating_add(len)).to_vec();
        }
        pos += 12;
    }
    Vec::new()
}

/// A file's resource fork: the native named fork first, then `._<name>`.
pub fn read_fork(file: &Path) -> Vec<u8> {
    let mut named = file.as_os_str().to_owned();
    named.push("/..namedfork/rsrc");
    if let Ok(d) = std::fs::read(PathBuf::from(named)) {
        if !d.is_empty() {
            return d;
        }
    }
    if let (Some(parent), Some(name)) = (file.parent(), file.file_name()) {
        let mut side = std::ffi::OsString::from("._");
        side.push(name);
        let sidecar = parent.join(side);
        if sidecar.is_file() {
            if let Ok(b) = std::fs::read(&sidecar) {
                return appledouble_rsrc(&b);
            }
        }
    }
    Vec::new()
}
