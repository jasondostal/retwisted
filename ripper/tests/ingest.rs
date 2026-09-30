//! Ingest against the user's own originals (port of the intent of
//! tools/twistedrip/tests/test_ingest.py). Skips when they're absent.

mod common;

use std::collections::BTreeSet;

use common::*;
use twistedrip::ingest::{ingest, Forks};

const FLOPPY_MODULE_NAMES: [&str; 16] = [
    "Mime Hunt",
    "Mowin' Boris",
    "Phlegm Boy",
    "Shock Clocks",
    "Toxic Swamp",
    "Twisted Art",
    "Twisted Faceplate",
    "Twisted Sound",
    "Voyeur",
    "Bungee Roulette",
    "Chameleon",
    "Coming Soon",
    "Flying Toilets",
    "FrankenScreen",
    "Message Mayhem",
    "Mike's So-called Life",
];

const CD_BONUS_NAMES: [&str; 5] = [
    "MonitorSD",
    "ShutDownSD",
    "Disney Faceplate",
    "Marvel Faceplate",
    "Star Trek Faceplate",
];

fn names(f: &Forks) -> BTreeSet<String> {
    f.names().map(str::to_string).collect()
}

fn set(v: &[&str]) -> BTreeSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Bytes 16..128 of a resource fork are reserved scratch that differs
/// between the floppy and CD builds of the same file.
fn zero_reserved_header(f: &[u8]) -> Vec<u8> {
    let mut v = f.to_vec();
    for b in v.iter_mut().take(128).skip(16) {
        *b = 0;
    }
    v
}

#[test]
fn floppy_sit_matches_reference_byte_for_byte() {
    let sit = original("After_Dark_-_Totally_Twisted.sit");
    if skip_unless(&sit) || skip_unless(&ref_modules()) {
        return;
    }
    let got = ingest(&sit, &mut |_| {}).unwrap();
    assert_eq!(names(&got), set(&FLOPPY_MODULE_NAMES));
    for (name, fork) in got.iter() {
        assert!(
            fork == ref_fork(name).as_slice(),
            "{name}: fork differs from the extracted reference"
        );
    }
}

#[test]
fn plain_folder_of_modules() {
    let dir = ref_modules().join("ADTotallyTwistedset1");
    if skip_unless(&dir) {
        return;
    }
    let got = ingest(&dir, &mut |_| {}).unwrap();
    assert_eq!(
        names(&got),
        set(&[
            "Mime Hunt",
            "Mowin' Boris",
            "Phlegm Boy",
            "Shock Clocks",
            "Toxic Swamp",
            "Twisted Art",
            "Twisted Faceplate",
            "Twisted Sound",
            "Voyeur"
        ])
    );
    for (name, fork) in got.iter() {
        assert!(fork == ref_fork(name).as_slice(), "{name}");
    }
}

#[test]
fn sit_hqx_binhex_demo() {
    let hqx = original("FlyingToiletsDemo.sit.hqx");
    let reference = extracted().join("Flying Toilets Demo");
    if skip_unless(&hqx) || skip_unless(&reference) {
        return;
    }
    let got = ingest(&hqx, &mut |_| {}).unwrap();
    assert_eq!(names(&got), set(&["Flying Toilets Demo"]));
    assert!(got.get("Flying Toilets Demo").unwrap() == &named_fork(&reference));
}

#[test]
fn hybrid_cd_sit_yields_core_modules_plus_bonus_content() {
    let sit = original("totallytwistedcd.sit");
    if skip_unless(&sit) || skip_unless(&ref_modules()) {
        return;
    }
    let got = ingest(&sit, &mut |_| {}).unwrap();
    let n = names(&got);
    assert!(set(&FLOPPY_MODULE_NAMES).is_subset(&n));
    assert!(set(&CD_BONUS_NAMES).is_subset(&n));
    // Installer, host app, library, read-mes and junk must not leak through.
    for excluded in [
        "After Dark 3.0",
        "Library 4.0",
        "Totally Twisted™ Manual",
        "Messyges Custom",
        "Screen Posters",
        "delete me",
        "Totally Twisted Setup Guide",
    ] {
        assert!(!n.contains(excluded), "{excluded} leaked through");
    }
    for name in FLOPPY_MODULE_NAMES {
        assert_eq!(
            zero_reserved_header(got.get(name).unwrap()),
            zero_reserved_header(&ref_fork(name)),
            "{name}: CD fork differs from the floppy beyond the reserved header"
        );
    }
}

#[test]
fn raw_hfs_iso_skips_windows_side() {
    let iso = extracted().join("totallytwistedcd.iso");
    if skip_unless(&iso) {
        return;
    }
    let got = ingest(&iso, &mut |_| {}).unwrap();
    assert!(set(&FLOPPY_MODULE_NAMES).is_subset(&names(&got)));
    // Nothing from the ISO 9660 side (8.3 upper-case, .ZIP/.EXE) appears.
    for name in got.names() {
        assert!(
            !(name.chars().any(char::is_alphabetic) && name == name.to_uppercase()),
            "{name}"
        );
        assert!(!name.ends_with(".ZIP") && !name.ends_with(".EXE"), "{name}");
    }
}

#[test]
fn appledouble_sidecar_fallback() {
    let src = ref_modules()
        .join("ADTotallyTwistedset1")
        .join("Twisted Art");
    if skip_unless(&src) {
        return;
    }
    let data = std::fs::read(&src).unwrap();
    let rsrc = named_fork(&src);
    let mut blob = b"\x00\x05\x16\x07\x00\x02\x00\x00".to_vec();
    blob.extend_from_slice(&[0u8; 16]);
    blob.extend_from_slice(&2u16.to_be_bytes());
    let data_off = blob.len() + 24;
    let rsrc_off = data_off + data.len();
    for (id, off, len) in [(1u32, data_off, data.len()), (2, rsrc_off, rsrc.len())] {
        blob.extend_from_slice(&id.to_be_bytes());
        blob.extend_from_slice(&(off as u32).to_be_bytes());
        blob.extend_from_slice(&(len as u32).to_be_bytes());
    }
    blob.extend_from_slice(&data);
    blob.extend_from_slice(&rsrc);
    let dir = scratch("appledouble");
    std::fs::write(dir.join("Twisted Art"), b"").unwrap();
    std::fs::write(dir.join("._Twisted Art"), &blob).unwrap();
    let got = ingest(&dir, &mut |_| {}).unwrap();
    assert_eq!(names(&got), set(&["Twisted Art"]));
    assert!(got.get("Twisted Art").unwrap() == &rsrc);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn junk_input_is_an_error_not_a_panic() {
    let dir = scratch("junk");
    let p = dir.join("junk.bin");
    std::fs::write(&p, vec![0x42u8; 4096]).unwrap();
    assert!(ingest(&p, &mut |_| {}).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
