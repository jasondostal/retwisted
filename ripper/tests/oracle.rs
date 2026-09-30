//! Per-decoder oracle tests against the private RE checkout's resource_dasm
//! dumps (ripped/<module>/...) -- the intent of the Python suite's
//! test_rsrc / test_rlep / test_oftb / test_snd / test_cmid. Local-only
//! data; every test skips when it's absent.

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use common::*;
use twistedrip::decode::{oftb, rlep};
use twistedrip::rsrc::{self, ResMap};
use twistedrip::{cmid, png, snd};

/// (Mac file name, ripped/ subdir) for every module-like file on the floppy.
fn module_files() -> Vec<(String, PathBuf, PathBuf)> {
    let mut out = Vec::new();
    for set in ["ADTotallyTwistedset1", "ADTotallyTwistedset2"] {
        let Ok(rd) = std::fs::read_dir(ref_modules().join(set)) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let sub = name.to_lowercase().replace(' ', "-");
            out.push((name, e.path(), ripped().join(sub)));
        }
    }
    out.sort();
    out
}

fn load(p: &Path) -> ResMap {
    rsrc::parse(&named_fork(p)).unwrap()
}

fn dir_names(d: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(d) else {
        return vec![];
    };
    let mut v: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

fn find(d: &Path, prefix_with_sep: &str, exact: &str) -> Option<PathBuf> {
    dir_names(d)
        .into_iter()
        .find(|n| n.starts_with(prefix_with_sep) || n == exact)
        .map(|n| d.join(n))
}

fn have_oracles() -> bool {
    !(skip_unless(&ref_modules()) || skip_unless(&ripped()))
}

#[test]
fn rsrc_matches_resource_dasm_bin_dumps() {
    if !have_oracles() {
        return;
    }
    // Stale oracle fixture documented in test_rsrc.py (Chameleon JOHN 130).
    let stale = [("Chameleon", "JOHN", 130i16)];
    // resource_dasm's filename escaping of a resource name.
    let escaped = |n: &str| -> String {
        n.chars()
            .map(|c| {
                if (c as u32) < 0x20 || c == '/' || c == ':' {
                    '_'
                } else {
                    c
                }
            })
            .collect()
    };
    let (mut checked, mut bad) = (0, Vec::new());
    let mut cases: Vec<(String, ResMap, PathBuf)> = module_files()
        .into_iter()
        .map(|(n, p, r)| (n, load(&p), r))
        .collect();
    let shared = extracted().join("Twisted_Sound.rsrc");
    if shared.exists() {
        cases.push((
            "Twisted_Sound.rsrc".into(),
            rsrc::parse(&std::fs::read(&shared).unwrap()).unwrap(),
            ripped().join("shared-twisted-sound"),
        ));
    }
    for (name, res, rdir) in &cases {
        let by_type: HashMap<(String, i16), &rsrc::Resource> = res
            .iter()
            .map(|r| ((r.rtype.trim_end().to_string(), r.id), r))
            .collect();
        for f in dir_names(rdir) {
            // _data/_excess.bin hold leftovers after a decoded STR payload.
            if f.ends_with("_data.bin") || f.ends_with("_excess.bin") {
                continue;
            }
            let Some(rest) = f
                .strip_prefix(&format!("{name}_"))
                .and_then(|r| r.strip_suffix(".bin"))
            else {
                continue;
            };
            let mut it = rest.splitn(3, '_');
            let (Some(t), Some(id)) = (it.next(), it.next()) else {
                continue;
            };
            let name_part = it.next();
            let Ok(id) = id.parse::<i16>() else { continue };
            if stale.contains(&(name.as_str(), t, id)) {
                continue;
            }
            let Some(r) = by_type.get(&(t.to_string(), id)) else {
                bad.push(format!("{name}: {t} {id} in oracle but not parsed"));
                continue;
            };
            if std::fs::read(rdir.join(&f)).unwrap() != r.data {
                bad.push(format!("{name}: {t} {id} bytes differ"));
            }
            if name_part.map(str::to_string) != (!r.name.is_empty()).then(|| escaped(&r.name)) {
                bad.push(format!(
                    "{name}: {t} {id} name {:?} vs oracle {name_part:?}",
                    r.name
                ));
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "no .bin oracles found");
    assert!(
        bad.is_empty(),
        "{} of {checked} mismatched:\n{}",
        bad.len(),
        bad.join("\n")
    );
    eprintln!("rsrc: {checked} resources byte-identical to resource_dasm dumps");
}

#[test]
fn rlep_frames_match_oracle_pngs() {
    if !have_oracles() {
        return;
    }
    let (mut checked, mut bad) = (0, Vec::new());
    for (name, path, rdir) in module_files() {
        let res = load(&path);
        for r in res.of_type("RLEP") {
            let odir = rdir.join(format!("frames-{}", r.id));
            if !odir.is_dir() {
                continue;
            }
            let bank = rlep::parse_bank(&r.data);
            let mut frames: Vec<_> = bank.frames.iter().collect();
            frames.sort_by_key(|f| f.0);
            for (fid, rect, stream) in frames {
                let oracle = odir.join(format!("frame_{fid:03}.png"));
                if !oracle.exists() {
                    continue;
                }
                let img =
                    rlep::render(&rlep::decode_rows(stream), &bank.ctab, *rect).expect("render");
                if png::write_png(img.w, img.h, &img.rgba) != std::fs::read(&oracle).unwrap() {
                    bad.push(format!("{name} bank {} frame {fid}", r.id));
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
    assert!(
        bad.is_empty(),
        "{} of {checked} frames differ: {:?}",
        bad.len(),
        &bad[..bad.len().min(20)]
    );
    eprintln!("rlep: {checked} frames byte-identical");
}

#[test]
fn oftb_compounds_match_oracle_pngs() {
    if !have_oracles() {
        return;
    }
    let (mut checked, mut bad) = (0, Vec::new());
    for (name, path, rdir) in module_files() {
        let res = load(&path);
        let mut banks: Vec<(i16, &[u8])> = res
            .of_type("RLEP")
            .map(|r| (r.id, r.data.as_slice()))
            .collect();
        banks.sort_by_key(|b| b.0);
        let mut cache = HashMap::new();
        let mut ofsts: Vec<_> = res.of_type("OFst").collect();
        ofsts.sort_by_key(|r| r.id);
        for ofst in ofsts {
            let odir = rdir.join(format!("compound-{}", ofst.id));
            if !odir.is_dir() {
                continue;
            }
            let art = oftb::load_art_chain(&banks, &mut cache, ofst.id).unwrap();
            for (fno, off, _, _) in oftb::parse_ofst(&ofst.data).unwrap() {
                let oracle = odir.join(format!("compound_{fno:03}.png"));
                if !oracle.exists() {
                    continue;
                }
                let tb = res
                    .get("OFtb", (ofst.id as i64 + (off >> 16) as i64) as i16)
                    .expect("OFtb bank");
                let c = oftb::compose_compound(
                    &art,
                    &tb.data,
                    (off & 0xFFFF) as usize,
                    oftb::shadow_channels(&name),
                )
                .unwrap();
                if let Some(img) = c.image {
                    if png::write_png(img.w, img.h, &img.rgba) != std::fs::read(&oracle).unwrap() {
                        bad.push(format!("{name} series {} compound {fno}", ofst.id));
                    }
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
    assert!(
        bad.is_empty(),
        "{} of {checked} compounds differ: {:?}",
        bad.len(),
        &bad[..bad.len().min(20)]
    );
    eprintln!("oftb: {checked} compounds byte-identical");
}

#[test]
fn snd_plain_and_expanded_match_oracle_wavs() {
    if !have_oracles() {
        return;
    }
    let (mut plain, mut expanded, mut bad) = (0, 0, Vec::new());
    let mut cases: Vec<(String, ResMap, PathBuf, PathBuf)> = module_files()
        .into_iter()
        .map(|(n, p, r)| (n.clone(), load(&p), r.clone(), r.join("expanded")))
        .collect();
    let shared = extracted().join("Twisted_Sound.rsrc");
    if shared.exists() {
        cases.push((
            "Twisted_Sound.rsrc".into(),
            rsrc::parse(&std::fs::read(&shared).unwrap()).unwrap(),
            ripped().join("shared-twisted-sound"),
            ripped().join("shared-twisted-sound-expanded"),
        ));
    }
    for (name, res, rdir, edir) in &cases {
        for r in res.of_type("snd ") {
            let comp = res.get("sndS", r.id);
            let (oracle, got) = match comp {
                Some(c) => {
                    let Some(o) = find(
                        edir,
                        &format!("snd_{}_", r.id),
                        &format!("snd_{}.wav", r.id),
                    ) else {
                        continue;
                    };
                    expanded += 1;
                    (o, snd::to_wav(&r.data, Some(&c.data)))
                }
                None => {
                    let Some(o) = find(
                        rdir,
                        &format!("{name}_snd_{}_", r.id),
                        &format!("{name}_snd_{}.wav", r.id),
                    ) else {
                        continue;
                    };
                    plain += 1;
                    (o, snd::to_wav(&r.data, None))
                }
            };
            match got {
                Ok(w) if w == std::fs::read(&oracle).unwrap() => {}
                Ok(_) => bad.push(format!("{name} snd {}: bytes differ", r.id)),
                Err(e) => bad.push(format!("{name} snd {}: {e}", r.id)),
            }
        }
    }
    assert!(plain > 0 && expanded > 0);
    assert!(bad.is_empty(), "{} mismatches: {bad:?}", bad.len());
    eprintln!("snd: {plain} plain + {expanded} sndS-expanded WAVs byte-identical");
}

#[test]
fn cmid_matches_oracle_midi() {
    let shared = extracted().join("Twisted_Sound.rsrc");
    let rdir = ripped().join("shared-twisted-sound");
    if skip_unless(&shared) || skip_unless(&rdir) {
        return;
    }
    let res = rsrc::parse(&std::fs::read(&shared).unwrap()).unwrap();
    let mut checked = 0;
    for r in res.of_type("cmid") {
        let Some(o) = find(
            &rdir,
            &format!("Twisted_Sound.rsrc_cmid_{}_", r.id),
            &format!("Twisted_Sound.rsrc_cmid_{}.midi", r.id),
        ) else {
            continue;
        };
        assert_eq!(
            cmid::decode(&r.data).unwrap(),
            std::fs::read(o).unwrap(),
            "cmid {}",
            r.id
        );
        checked += 1;
    }
    assert!(checked > 0);
}
