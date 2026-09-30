//! StuffIt archives: the StuffIt 5 format (the floppy release and the CD
//! image's wrapper) and the classic `SIT!`-family format (the CD's StuffIt
//! SE self-extracting installer keeps one of these, signature `STi2`, in its
//! data fork; BinHex-wrapped demos carry one too).
//!
//! The directory walking is ours -- a port of the layout The Unarchiver's
//! XADMaster reads (the reference `unar` the Python ripper shells out to),
//! including its sequential SIT5 walk and the SIT5 "end of directory" marker
//! entries. The per-fork decompressors (method 13 LZ+Huffman, method 15
//! Arsenic) are in sitcodec.rs. Those two plus "store" are the only methods
//! any Totally Twisted release uses (floppy + CD wrapper: Arsenic; CD
//! installer + BinHex demo: LZ+Huffman); anything else is reported, not
//! guessed at.

use super::{sitcodec, MacFile};
use crate::util::{mac_roman, Be};
use crate::Error;

/// One compressed fork, decompressed on demand (the CD's 40 MB .iso data
/// fork should only be inflated when something actually wants it).
#[derive(Clone, Debug)]
pub struct PackedFork {
    pub method: u8,
    pub ulen: u32,
    pub offset: usize,
    pub clen: usize,
    pub sit5: bool,
}

impl PackedFork {
    fn empty(sit5: bool) -> Self {
        PackedFork {
            method: 0,
            ulen: 0,
            offset: 0,
            clen: 0,
            sit5,
        }
    }

    pub fn decompress(&self, archive: &[u8]) -> Result<Vec<u8>, Error> {
        if self.ulen == 0 {
            return Ok(Vec::new());
        }
        let raw = archive.bytes_at(self.offset, self.clen)?;
        let ulen = self.ulen as usize;
        // Classic archives keep flag bits above the low nibble.
        let method = if self.sit5 {
            self.method
        } else {
            self.method & 0x0F
        };
        let out = match method {
            0 => raw.to_vec(),
            13 => sitcodec::sit13(raw, ulen)?,
            15 if self.sit5 => sitcodec::arsenic(raw, ulen)?,
            _ => {
                return Err(Error::Unsupported(format!(
                    "StuffIt compression method {} ({}) -- only store, LZ+Huffman (13) and Arsenic (15) are implemented",
                    self.method,
                    if self.sit5 { "StuffIt 5" } else { "classic" }
                )))
            }
        };
        if out.len() != self.ulen as usize {
            return Err(Error::Container(format!(
                "StuffIt method {}: got {} bytes, expected {}",
                self.method,
                out.len(),
                self.ulen
            )));
        }
        Ok(out)
    }
}

/// A file entry, forks still packed.
#[derive(Clone, Debug)]
pub struct SitFile {
    pub path: Vec<String>,
    pub file_type: [u8; 4],
    pub creator: [u8; 4],
    pub data: PackedFork,
    pub rsrc: PackedFork,
}

impl SitFile {
    pub fn unpack(&self, archive: &[u8]) -> Result<MacFile, Error> {
        Ok(MacFile {
            path: self.path.clone(),
            file_type: self.file_type,
            creator: self.creator,
            data: self.data.decompress(archive)?,
            rsrc: self.rsrc.decompress(archive)?,
        })
    }
}

pub fn is_sit5(data: &[u8]) -> bool {
    data.len() >= 100
        && data.starts_with(b"StuffIt ")
        && data.get(0x50..0x52) == Some(&b"\x1a\x00"[..])
}

pub fn is_classic(data: &[u8]) -> bool {
    const SIGS: [&[u8; 4]; 12] = [
        b"SIT!", b"ST46", b"ST50", b"ST60", b"ST65", b"STin", b"STi2", b"STi3", b"STi4", b"ST4s",
        b"ST4t", b"STmx",
    ];
    data.len() >= 22
        && data.get(10..14) == Some(&b"rLau"[..])
        && SIGS.iter().any(|s| data.starts_with(&s[..]))
}

/// List the files of a StuffIt 5 archive. Like unar, a structural error
/// after some entries were read ends the walk rather than failing it: the
/// floppy release's archive trails off into garbage after its last real
/// entry ("Archive parsing failed!" from unar, after extracting everything).
pub fn list_sit5(data: &[u8]) -> Result<Vec<SitFile>, Error> {
    let _version = data.u8_at(82)?;
    let num_root = data.u16_at(92)? as usize;
    let first = data.u32_at(94)? as usize;
    let mut out = Vec::new();
    // parent-dir header offset -> path of that dir
    let mut dirs: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();
    let mut remaining = num_root;
    let mut off = first;
    while remaining > 0 {
        match sit5_entry(data, off) {
            Ok(Sit5Entry::Marker { next }) => off = next,
            Ok(Sit5Entry::Dir {
                name,
                parent,
                numfiles,
                next,
            }) => {
                let mut path = dirs.get(&parent).cloned().unwrap_or_default();
                path.push(name);
                dirs.insert(off, path);
                remaining += numfiles;
                remaining -= 1;
                off = next;
            }
            Ok(Sit5Entry::File {
                name,
                parent,
                file_type,
                creator,
                data: d,
                rsrc,
                next,
            }) => {
                let mut path = dirs.get(&parent).cloned().unwrap_or_default();
                path.push(name);
                out.push(SitFile {
                    path,
                    file_type,
                    creator,
                    data: d,
                    rsrc,
                });
                remaining -= 1;
                off = next;
            }
            Err(e) => {
                if out.is_empty() {
                    return Err(e);
                }
                break;
            }
        }
    }
    Ok(out)
}

enum Sit5Entry {
    Marker {
        next: usize,
    },
    Dir {
        name: String,
        parent: usize,
        numfiles: usize,
        next: usize,
    },
    File {
        name: String,
        parent: usize,
        file_type: [u8; 4],
        creator: [u8; 4],
        data: PackedFork,
        rsrc: PackedFork,
        next: usize,
    },
}

fn sit5_entry(d: &[u8], offs: usize) -> Result<Sit5Entry, Error> {
    if d.u32_at(offs)? != 0xA5A5_A5A5 {
        return Err(Error::Container(format!(
            "StuffIt 5: no entry header at {offs:#x}"
        )));
    }
    let version = d.u8_at(offs + 4)?;
    let header_size = d.u16_at(offs + 6)? as usize;
    let flags = d.u8_at(offs + 9)?;
    let parent = d.u32_at(offs + 26)? as usize;
    let name_len = d.u16_at(offs + 30)? as usize;
    let data_ulen = d.u32_at(offs + 34)?;
    let data_clen = d.u32_at(offs + 38)? as usize;
    let name = mac_roman(d.bytes_at(offs + 48, name_len)?);
    let header_end = offs + header_size;
    if flags & 0x40 != 0 {
        if data_ulen == 0xFFFF_FFFF {
            // End-of-directory marker: header only, no second block.
            return Ok(Sit5Entry::Marker { next: header_end });
        }
        let numfiles = d.u16_at(offs + 46)? as usize;
        let skip = if version == 1 { 22 } else { 18 };
        return Ok(Sit5Entry::Dir {
            name,
            parent,
            numfiles,
            next: header_end + 14 + skip,
        });
    }
    if flags & 0x20 != 0 {
        return Err(Error::Unsupported("encrypted StuffIt 5 entry".into()));
    }
    let data_method = d.u8_at(offs + 46)?;
    let mut p = header_end;
    let flags2 = d.u16_at(p)?;
    let mut file_type = [0u8; 4];
    file_type.copy_from_slice(d.bytes_at(p + 4, 4)?);
    let mut creator = [0u8; 4];
    creator.copy_from_slice(d.bytes_at(p + 8, 4)?);
    p += 14 + if version == 1 { 22 } else { 18 };
    let mut rsrc = PackedFork::empty(true);
    if flags2 & 1 != 0 {
        rsrc.ulen = d.u32_at(p)?;
        rsrc.clen = d.u32_at(p + 4)? as usize;
        rsrc.method = d.u8_at(p + 12)?;
        p += 14;
    }
    rsrc.offset = p;
    let data = PackedFork {
        method: data_method,
        ulen: data_ulen,
        offset: p + rsrc.clen,
        clen: data_clen,
        sit5: true,
    };
    let next = p + rsrc.clen + data_clen;
    if next > d.len() {
        return Err(Error::Truncated);
    }
    Ok(Sit5Entry::File {
        name,
        parent,
        file_type,
        creator,
        data,
        rsrc,
        next,
    })
}

/// List the files of a classic (`SIT!`, `STi2`, ...) archive.
pub fn list_classic(d: &[u8]) -> Result<Vec<SitFile>, Error> {
    let total = (d.u32_at(6)? as usize).min(d.len());
    let mut off = 22;
    let mut stack: Vec<String> = Vec::new();
    let mut out = Vec::new();
    while off + 112 <= total {
        let h = d.bytes_at(off, 112)?;
        let rmethod = h[0];
        let dmethod = h[1];
        let name_len = (h[2] as usize).min(63);
        let name = mac_roman(&h[3..3 + name_len]);
        if rmethod == 32 || dmethod == 32 {
            stack.push(name);
            off += 112;
            continue;
        }
        if rmethod == 33 || dmethod == 33 {
            stack.pop();
            off += 112;
            continue;
        }
        let mut file_type = [0u8; 4];
        file_type.copy_from_slice(&h[66..70]);
        let mut creator = [0u8; 4];
        creator.copy_from_slice(&h[70..74]);
        let rulen = h.u32_at(84)?;
        let dulen = h.u32_at(88)?;
        let rclen = h.u32_at(92)? as usize;
        let dclen = h.u32_at(96)? as usize;
        let start = off + 112;
        let rsrc = PackedFork {
            method: rmethod,
            ulen: rulen,
            offset: start,
            clen: rclen,
            sit5: false,
        };
        let data = PackedFork {
            method: dmethod,
            ulen: dulen,
            offset: start + rclen,
            clen: dclen,
            sit5: false,
        };
        let mut path = stack.clone();
        path.push(name);
        out.push(SitFile {
            path,
            file_type,
            creator,
            data,
            rsrc,
        });
        off = start + rclen + dclen;
    }
    Ok(out)
}
