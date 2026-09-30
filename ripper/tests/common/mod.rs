//! Shared test plumbing. Everything that touches Berkeley data reads the
//! user's own local copies at test time and SKIPS (passes with a note) when
//! they are absent -- no original, ripped resource or reference output is
//! ever committed:
//!
//!   $TWISTEDRIP_RE_ROOT (default ~/working/totally-twisted)
//!       original/   the Macintosh Garden downloads (.sit, .sit.hqx)
//!       extracted/  unar/unstuffed copies (+ the CD .iso)
//!       ripped/     the private RE checkout's resource_dasm dumps (oracles)
//!   ../assets       the live pack tree, like the app crate's tests use

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn re_root() -> PathBuf {
    std::env::var_os("TWISTEDRIP_RE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("working/totally-twisted")
        })
}
pub fn original(name: &str) -> PathBuf {
    re_root().join("original").join(name)
}
pub fn extracted() -> PathBuf {
    re_root().join("extracted")
}
pub fn ref_modules() -> PathBuf {
    extracted().join("After_Dark_-_Totally_Twisted")
}
pub fn ripped() -> PathBuf {
    re_root().join("ripped")
}
pub fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")
}

/// `if skip_unless(path) { return; }` -- note and bail when a local-only
/// input is missing.
pub fn skip_unless(p: &Path) -> bool {
    if p.exists() {
        false
    } else {
        eprintln!("SKIP: {} not present on this machine", p.display());
        true
    }
}

pub fn named_fork(p: &Path) -> Vec<u8> {
    let mut s = p.as_os_str().to_owned();
    s.push("/..namedfork/rsrc");
    std::fs::read(PathBuf::from(s)).unwrap_or_default()
}

/// The reference fork of a module file from the extracted floppy tree.
pub fn ref_fork(name: &str) -> Vec<u8> {
    for set in ["ADTotallyTwistedset1", "ADTotallyTwistedset2"] {
        let p = ref_modules().join(set).join(name);
        if p.exists() {
            return named_fork(&p);
        }
    }
    panic!("no reference fork for {name}");
}

pub fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("twistedrip-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

pub fn files_under(root: &Path) -> BTreeMap<String, PathBuf> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else if p.is_file() {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    p,
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Minimal JSON reader, for parity.py's "identical after a canonical
// (sorted-keys) round-trip" comparison of meta.json / pens500.json.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum V {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<V>),
    Obj(BTreeMap<String, V>),
}

pub fn parse_json(s: &str) -> V {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    let v = value(&b, &mut i);
    ws(&b, &mut i);
    assert_eq!(i, b.len(), "trailing JSON");
    v
}

fn ws(b: &[char], i: &mut usize) {
    while *i < b.len() && b[*i].is_whitespace() {
        *i += 1;
    }
}

fn string(b: &[char], i: &mut usize) -> String {
    assert_eq!(b[*i], '"');
    *i += 1;
    let mut out = String::new();
    let mut units: Vec<u16> = Vec::new();
    loop {
        let c = b[*i];
        *i += 1;
        if c != '\\' && !units.is_empty() {
            out.push_str(&String::from_utf16_lossy(&units));
            units.clear();
        }
        match c {
            '"' => return out,
            '\\' => {
                let e = b[*i];
                *i += 1;
                if e == 'u' {
                    let h: String = b[*i..*i + 4].iter().collect();
                    *i += 4;
                    units.push(u16::from_str_radix(&h, 16).unwrap());
                    continue;
                }
                if !units.is_empty() {
                    out.push_str(&String::from_utf16_lossy(&units));
                    units.clear();
                }
                out.push(match e {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    c => c,
                });
            }
            c => out.push(c),
        }
    }
}

fn value(b: &[char], i: &mut usize) -> V {
    ws(b, i);
    match b[*i] {
        '{' => {
            *i += 1;
            let mut m = BTreeMap::new();
            ws(b, i);
            if b[*i] == '}' {
                *i += 1;
                return V::Obj(m);
            }
            loop {
                ws(b, i);
                let k = string(b, i);
                ws(b, i);
                assert_eq!(b[*i], ':');
                *i += 1;
                m.insert(k, value(b, i));
                ws(b, i);
                let c = b[*i];
                *i += 1;
                if c == '}' {
                    return V::Obj(m);
                }
            }
        }
        '[' => {
            *i += 1;
            let mut a = Vec::new();
            ws(b, i);
            if b[*i] == ']' {
                *i += 1;
                return V::Arr(a);
            }
            loop {
                a.push(value(b, i));
                ws(b, i);
                let c = b[*i];
                *i += 1;
                if c == ']' {
                    return V::Arr(a);
                }
            }
        }
        '"' => V::Str(string(b, i)),
        't' => {
            *i += 4;
            V::Bool(true)
        }
        'f' => {
            *i += 5;
            V::Bool(false)
        }
        'n' => {
            *i += 4;
            V::Null
        }
        _ => {
            let s = *i;
            while *i < b.len() && (b[*i].is_ascii_digit() || "+-.eE".contains(b[*i])) {
                *i += 1;
            }
            V::Num(b[s..*i].iter().collect())
        }
    }
}

/// parity.py's comparison: PNG/WAV/MID/other byte-identical, JSON identical
/// after canonicalisation. Returns the list of mismatching paths.
pub fn compare_trees(reference: &Path, got: &Path, derived_ok: bool) -> Vec<String> {
    let is_derived = |rel: &str| {
        derived_ok
            && (rel.contains("_pens/")
                || rel.contains("hands/")
                || rel.contains("lcd/")
                || rel.rsplit('/').next().is_some_and(|f| {
                    (f.starts_with("m_") || f.starts_with("star_") || f.starts_with("wall_r"))
                        && f.ends_with(".png")
                }))
    };
    let r = files_under(reference);
    let g = files_under(got);
    let mut bad = Vec::new();
    for (rel, rp) in &r {
        if is_derived(rel) {
            continue;
        }
        let Some(gp) = g.get(rel) else {
            bad.push(format!("missing {rel}"));
            continue;
        };
        let (a, b) = (std::fs::read(rp).unwrap(), std::fs::read(gp).unwrap());
        let same = if rel.ends_with(".json") {
            parse_json(&String::from_utf8(a).unwrap()) == parse_json(&String::from_utf8(b).unwrap())
        } else {
            a == b
        };
        if !same {
            bad.push(format!("differs {rel}"));
        }
    }
    for rel in g.keys() {
        if !r.contains_key(rel) && !is_derived(rel) {
            bad.push(format!("extra {rel}"));
        }
    }
    bad
}
