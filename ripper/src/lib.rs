//! twistedrip: rip a user's own Totally Twisted originals into retwisted
//! asset packs. A pure-Rust port of `tools/rip.py` + `tools/twistedrip/`,
//! held to byte parity with it (see docs/ripper.md).
//!
//! The whole public surface the saver needs is [`rip`]:
//!
//! ```no_run
//! let report = twistedrip::rip(
//!     "After_Dark_-_Totally_Twisted.sit".as_ref(),
//!     "assets".as_ref(),
//!     &twistedrip::Options::default(),
//!     &mut |p| eprintln!("[{}/{}] {}", p.step, p.total, p.message),
//! )?;
//! assert!(report.all_modules_ripped());
//! # Ok::<(), twistedrip::Error>(())
//! ```
//!
//! Inputs: the floppy release's StuffIt 5 `.sit`, the hybrid-CD `.sit` or
//! `.iso` (HFS side only), a `.sit.hqx`, a MacBinary file, or a folder of
//! already-extracted module files (native resource forks or AppleDouble).

pub mod cmid;
pub mod container;
pub mod decode;
pub mod deflate;
pub mod ingest;
pub mod inst;
pub mod json;
pub mod pack;
pub mod png;
pub mod rsrc;
pub mod snd;
pub mod util;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A read ran past the end of some structure.
    Truncated,
    Io(String),
    Container(String),
    Decode(String),
    Unsupported(String),
    /// `Options::only` named no module that was found.
    NoSuchModule(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated => write!(f, "truncated data"),
            Error::Io(s) => write!(f, "I/O error: {s}"),
            Error::Container(s) => write!(f, "{s}"),
            Error::Decode(s) => write!(f, "decode error: {s}"),
            Error::Unsupported(s) => write!(f, "unsupported: {s}"),
            Error::NoSuchModule(s) => write!(f, "--only {s:?} matched no module"),
        }
    }
}

impl std::error::Error for Error {}

pub const SHARED_SOUND_NAME: &str = "Twisted Sound";

/// The 13 module Mac file names, in pack order (rip.py MODULE_NAMES).
pub const MODULE_NAMES: [&str; 13] = [
    "Bungee Roulette",
    "Chameleon",
    "Coming Soon",
    "Flying Toilets",
    "FrankenScreen",
    "Message Mayhem",
    "Mike's So-called Life",
    "Mime Hunt",
    "Mowin' Boris",
    "Phlegm Boy",
    "Shock Clocks",
    "Toxic Swamp",
    "Voyeur",
];

/// Mac file name -> assets/<slug>: lower-case, spaces to dashes, no apostrophes.
pub fn slug_for(mac_name: &str) -> String {
    mac_name.to_lowercase().replace(' ', "-").replace('\'', "")
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Rip only this slug (the faceplate is always written).
    pub only: Option<String>,
}

/// A progress tick, delivered on the calling thread.
#[derive(Debug, Clone)]
pub struct Progress {
    pub step: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ModuleReport {
    pub slug: String,
    pub mac_name: String,
    /// None when the input had no such module (rip.py's SKIP line).
    pub dir: Option<PathBuf>,
    pub series: usize,
    pub compounds: usize,
    pub sounds: usize,
    pub songs: usize,
    /// Compose-gate diagnostics (missing art etc.); never fatal.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub modules: Vec<ModuleReport>,
    pub faceplate: Option<PathBuf>,
    /// Every Mac file name ingest kept (modules, shared banks, bonus files).
    pub ingested: Vec<String>,
}

impl Report {
    /// True when every one of the 13 modules (or the `only` one) was packed.
    pub fn all_modules_ripped(&self) -> bool {
        !self.modules.is_empty() && self.modules.iter().all(|m| m.dir.is_some())
    }
}

fn count_files(dir: &Path, ext: &str, recursive: bool) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut n = 0;
    for e in rd.flatten() {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() && recursive => n += count_files(&p, ext, true),
            Ok(t) if t.is_file() && p.extension().is_some_and(|x| x == ext) => n += 1,
            _ => {}
        }
    }
    n
}

/// Rip `input` into `out_dir` (the assets/ root): one `<slug>/` per module
/// plus `_shared/faceplate.png`. Existing files are overwritten, nothing is
/// deleted.
pub fn rip(
    input: &Path,
    out_dir: &Path,
    opts: &Options,
    progress: &mut dyn FnMut(&Progress),
) -> Result<Report, Error> {
    let total = MODULE_NAMES.len() + 2;
    let mut tick = |step: usize, message: String| {
        progress(&Progress {
            step,
            total,
            message,
        })
    };

    tick(0, format!("reading {}", input.display()));
    let forks = ingest::ingest(input, &mut |m| tick(0, m.to_string()))?;
    let mut resources: HashMap<String, rsrc::ResMap> = HashMap::new();
    for (name, fork) in forks.iter() {
        resources.insert(
            name.to_string(),
            rsrc::parse(fork).map_err(|e| Error::Decode(format!("{name}: resource fork: {e}")))?,
        );
    }
    let empty = rsrc::ResMap::default();
    let shared_sound = resources.get(SHARED_SOUND_NAME).unwrap_or(&empty);
    std::fs::create_dir_all(out_dir)
        .map_err(|e| Error::Io(format!("{}: {e}", out_dir.display())))?;

    let mut modules = Vec::new();
    for (i, name) in MODULE_NAMES.iter().enumerate() {
        let slug = slug_for(name);
        if opts.only.as_deref().is_some_and(|o| o != slug) {
            continue;
        }
        tick(i + 1, slug.clone());
        let Some(module) = resources.get(*name) else {
            modules.push(ModuleReport {
                slug,
                mac_name: name.to_string(),
                dir: None,
                series: 0,
                compounds: 0,
                sounds: 0,
                songs: 0,
                warnings: vec![],
            });
            continue;
        };
        let built = pack::build(&slug, name, module, shared_sound).map_err(|e| match e {
            Error::Decode(s) => Error::Decode(format!("{slug}: {s}")),
            e => e,
        })?;
        let series = built
            .meta
            .get("series")
            .map(|s| {
                if let json::Json::Obj(o) = s {
                    o.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        let songs = match built.meta.get("music") {
            Some(json::Json::Obj(m)) => match m.get("songs") {
                Some(json::Json::Obj(s)) => s.len(),
                _ => 0,
            },
            _ => 0,
        };
        pack::write_all(out_dir, built.output)?;
        let dir = out_dir.join(&slug);
        modules.push(ModuleReport {
            compounds: count_files(&dir.join("compounds"), "png", true),
            sounds: count_files(&dir.join("sounds"), "wav", false),
            slug,
            mac_name: name.to_string(),
            dir: Some(dir),
            series,
            songs,
            warnings: built.warnings,
        });
    }

    tick(total - 1, "faceplate".into());
    let faceplate = match pack::build_faceplate(&resources) {
        Some((rel, img)) => {
            let mut out = pack::Output::default();
            out.dirs.push(PathBuf::from(pack::SHARED_DIR));
            out.files.push((rel.clone(), pack::Content::Png(img)));
            pack::write_all(out_dir, out)?;
            Some(out_dir.join(rel))
        }
        None => None,
    };

    if let Some(o) = &opts.only {
        if modules.iter().all(|m| m.dir.is_none()) {
            return Err(Error::NoSuchModule(o.clone()));
        }
    }
    tick(total, "done".into());
    Ok(Report {
        modules,
        faceplate,
        ingested: forks.names().map(str::to_string).collect(),
    })
}
