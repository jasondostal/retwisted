//! End-to-end: `twistedrip::rip` on the user's originals vs the live
//! ../assets tree, with parity.py's rules (PNG/WAV/MID byte-identical, JSON
//! identical after a sorted-keys round trip). Skips when inputs are absent.
//!
//! The full 13-module x 3-input gate is `#[ignore]`d (it deflates ~1.7 GB
//! of RGBA); run it with
//!     cargo test -p ripper --release -- --ignored

mod common;

use std::path::Path;

use common::*;
use twistedrip::{rip, Options};

fn rip_into(input: &Path, tag: &str, only: Option<&str>) -> std::path::PathBuf {
    let out = scratch(tag);
    let report = rip(
        input,
        &out,
        &Options {
            only: only.map(str::to_string),
        },
        &mut |_| {},
    )
    .unwrap();
    assert!(report.all_modules_ripped(), "{report:?}");
    assert!(report.faceplate.is_some());
    out
}

#[test]
fn floppy_bungee_roulette_matches_live_pack() {
    let sit = original("After_Dark_-_Totally_Twisted.sit");
    if skip_unless(&sit) || skip_unless(&assets().join("bungee-roulette")) {
        return;
    }
    let out = rip_into(&sit, "e2e-bungee", Some("bungee-roulette"));
    for sub in ["bungee-roulette", "_shared"] {
        let bad = compare_trees(&assets().join(sub), &out.join(sub), true);
        assert!(
            bad.is_empty(),
            "{sub}: {} differences: {:?}",
            bad.len(),
            &bad[..bad.len().min(20)]
        );
    }
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn floppy_cd_sit_and_iso_rip_identically() {
    let inputs = [
        original("After_Dark_-_Totally_Twisted.sit"),
        original("totallytwistedcd.sit"),
        extracted().join("totallytwistedcd.iso"),
    ];
    if inputs.iter().any(|p| skip_unless(p)) {
        return;
    }
    let outs: Vec<_> = inputs
        .iter()
        .enumerate()
        .map(|(i, p)| rip_into(p, &format!("e2e-x{i}"), Some("flying-toilets")))
        .collect();
    for o in &outs[1..] {
        let bad = compare_trees(&outs[0], o, false);
        assert!(bad.is_empty(), "{bad:?}");
        // Byte-identical, JSON included (not just canonically equal).
        for (rel, p) in files_under(&outs[0]) {
            assert_eq!(
                std::fs::read(&p).unwrap(),
                std::fs::read(o.join(&rel)).unwrap(),
                "{rel}"
            );
        }
    }
    for o in outs {
        let _ = std::fs::remove_dir_all(o);
    }
}

#[test]
fn unknown_only_slug_is_an_error() {
    let sit = original("After_Dark_-_Totally_Twisted.sit");
    if skip_unless(&sit) {
        return;
    }
    let out = scratch("e2e-only");
    let r = rip(
        &sit,
        &out,
        &Options {
            only: Some("no-such-module".into()),
        },
        &mut |_| {},
    );
    assert!(matches!(r, Err(twistedrip::Error::NoSuchModule(_))));
    // rip.py still writes the faceplate on an --only run.
    assert!(out.join("_shared/faceplate.png").exists());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
#[ignore = "full gate: ~1.7 GB of PNG deflate; run with --release -- --ignored"]
fn full_parity_all_modules_all_inputs() {
    let inputs = [
        original("After_Dark_-_Totally_Twisted.sit"),
        original("totallytwistedcd.sit"),
        extracted().join("totallytwistedcd.iso"),
    ];
    if inputs.iter().any(|p| skip_unless(p)) || skip_unless(&assets()) {
        return;
    }
    for (i, input) in inputs.iter().enumerate() {
        let out = rip_into(input, &format!("full{i}"), None);
        let mut subs: Vec<String> = twistedrip::MODULE_NAMES
            .iter()
            .map(|n| twistedrip::slug_for(n))
            .collect();
        subs.push("_shared".into());
        for sub in subs {
            let bad = compare_trees(&assets().join(&sub), &out.join(&sub), true);
            assert!(
                bad.is_empty(),
                "{} / {sub}: {} differences: {:?}",
                input.display(),
                bad.len(),
                &bad[..bad.len().min(20)]
            );
        }
        let _ = std::fs::remove_dir_all(&out);
    }
}
