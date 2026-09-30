//! Container layers: everything between "the file the user downloaded" and
//! "a list of Mac files with both forks".

pub mod binhex;
pub mod folder;
pub mod hfs;
pub mod sit;
pub mod sitcodec;

/// One classic Mac file: its path inside whatever contained it, Finder
/// type/creator, and both forks.
#[derive(Clone, Debug, Default)]
pub struct MacFile {
    pub path: Vec<String>,
    pub file_type: [u8; 4],
    pub creator: [u8; 4],
    pub data: Vec<u8>,
    pub rsrc: Vec<u8>,
}

impl MacFile {
    pub fn name(&self) -> &str {
        self.path.last().map(String::as_str).unwrap_or("")
    }
}
