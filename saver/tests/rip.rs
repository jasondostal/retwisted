//! The first-run rip, driven through the C ABI exactly the way the Swift
//! Options panel drives it: `rtw_rip_start`, poll on "the main thread"
//! until it settles, read the message, free. Skips (passes with a note)
//! whatever local originals are absent — nothing Berkeley is committed.

#[path = "../../ripper/tests/common/mod.rs"]
mod common;

use common::{compare_trees, files_under, original, skip_unless};
use retwisted_saver::*;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn c(p: &Path) -> CString {
    CString::new(p.to_str().unwrap()).unwrap()
}

fn scratch(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("rtw-rip-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Poll to completion the way the panel's timer does. Returns (state,
/// message, modules, progress steps seen).
fn run(input: &Path, root: &Path) -> (i32, String, u32, Vec<(u32, u32)>) {
    unsafe {
        let job = rtw_rip_start(c(input).as_ptr(), c(root).as_ptr());
        assert!(!job.is_null(), "rtw_rip_start refused {}", input.display());
        let t0 = Instant::now();
        let mut seen = Vec::new();
        let state = loop {
            let (mut step, mut total) = (0u32, 0u32);
            let s = rtw_rip_poll(job, &mut step, &mut total);
            if seen.last() != Some(&(step, total)) {
                seen.push((step, total));
            }
            // the message is readable at every state
            assert!(!rtw_rip_message(job).is_null());
            if s != RTW_RIP_RUNNING {
                break s;
            }
            assert!(t0.elapsed() < Duration::from_secs(300), "rip never settled");
            std::thread::sleep(Duration::from_millis(20));
        };
        let msg = CStr::from_ptr(rtw_rip_message(job))
            .to_str()
            .unwrap()
            .to_string();
        let n = rtw_rip_module_count(job);
        rtw_rip_free(job);
        (state, msg, n, seen)
    }
}

/// Nothing named `.rip-*` is left beside the root once a job has settled.
fn no_temp_left(root: &Path) {
    let parent = root.parent().unwrap();
    let left: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(".rip-"))
        .map(|e| e.path())
        .collect();
    assert!(left.is_empty(), "temp dirs left behind: {left:?}");
}

/// The whole first run: the floppy release through the ABI into an empty
/// Application-Support-shaped root; all 13 packs + the faceplate land, and
/// every file the live ../assets tree holds is there, identical (JSON after
/// a canonical round trip — parity.py's rule, the ripper's own gate).
/// Then a second rip over the top: the replace path (RENAME_SWAP).
#[test]
fn rip_abi_floppy_matches_the_live_packs_and_replaces_in_place() {
    let sit = original("After_Dark_-_Totally_Twisted.sit");
    let live = Path::new("../assets");
    if skip_unless(&sit) || skip_unless(&live.join("bungee-roulette")) {
        return;
    }
    let base = scratch("floppy");
    let root = base.join("com.retwisted.saver").join("assets");
    let (state, msg, n, seen) = run(&sit, &root);
    assert_eq!(state, RTW_RIP_DONE, "{msg}");
    assert_eq!(n, 13, "{msg}");
    assert_eq!(msg, "All 13 modules are ready.");
    assert!(
        seen.iter().any(|&(s, t)| t > 0 && s > 0 && s < t),
        "no progress seen: {seen:?}"
    );
    no_temp_left(&root);
    for i in 0..rtw_module_count() {
        let slug = unsafe { CStr::from_ptr(rtw_module_slug(i)) };
        assert!(
            unsafe { rtw_pack_exists(slug.as_ptr(), c(&root).as_ptr()) },
            "{slug:?}"
        );
    }
    assert!(root.join("_shared/faceplate.png").is_file());
    // The manifest: every entry of the live tree, module-derived residue
    // aside, is in the ripped one with the same bytes.
    for sub in std::fs::read_dir(&root).unwrap().flatten() {
        let name = sub.file_name();
        let bad = compare_trees(&live.join(&name), &sub.path(), true);
        assert!(bad.is_empty(), "{name:?}: {:?}", &bad[..bad.len().min(10)]);
    }
    let manifest = files_under(&root);
    assert!(manifest.len() > 10_000, "only {} files", manifest.len());

    // A runtime comes up off the ripped root.
    unsafe {
        let slug = CString::new("bungee-roulette").unwrap();
        let rt = rtw_create(slug.as_ptr(), c(&root).as_ptr());
        assert!(!rt.is_null());
        rtw_destroy(rt);
    }

    // Replace: rip again over the top — same result, nothing stranded.
    let before = std::fs::metadata(root.join("voyeur/meta.json"))
        .unwrap()
        .len();
    let (state, msg, n, _) = run(&sit, &root);
    assert_eq!((state, n), (RTW_RIP_DONE, 13), "{msg}");
    no_temp_left(&root);
    assert_eq!(files_under(&root).len(), manifest.len());
    assert_eq!(
        std::fs::metadata(root.join("voyeur/meta.json"))
            .unwrap()
            .len(),
        before
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// Inputs that are not Totally Twisted fail with a sentence, not a
/// backtrace, and never leave a pack (or a temp dir) behind.
#[test]
fn rip_abi_refuses_what_is_not_totally_twisted() {
    let base = scratch("refuse");
    let root = base.join("app").join("assets");

    // Some random file.
    let junk = base.join("notes.sit");
    std::fs::write(&junk, b"this is not a StuffIt archive at all").unwrap();
    let (state, msg, n, _) = run(&junk, &root);
    assert_eq!((state, n), (RTW_RIP_FAILED, 0));
    assert!(
        msg.starts_with("That file isn't a Totally Twisted download"),
        "{msg}"
    );

    // A folder with nothing Mac in it.
    let empty = base.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let (state, msg, _, _) = run(&empty, &root);
    assert_eq!(state, RTW_RIP_FAILED);
    assert!(
        msg.starts_with("No Totally Twisted modules were found"),
        "{msg}"
    );

    // Missing file.
    let (state, msg, _, _) = run(&base.join("nope.sit"), &root);
    assert_eq!(state, RTW_RIP_FAILED);
    assert!(!msg.is_empty());

    // A real Mac download that holds no module packs (the Flying Toilets
    // demo, when present): unpacks fine, still refused.
    let demo = original("FlyingToiletsDemo.sit.hqx");
    if !skip_unless(&demo) {
        let (state, msg, _, _) = run(&demo, &root);
        assert_eq!(state, RTW_RIP_FAILED);
        assert!(msg.starts_with("No Totally Twisted modules"), "{msg}");
    }
    assert!(
        !root.exists() || files_under(&root).is_empty(),
        "a failed rip left files"
    );
    no_temp_left(&root);
    let _ = std::fs::remove_dir_all(&base);
}

/// Freeing a RUNNING job cancels it: the worker completes, deletes its temp
/// dir and installs nothing.
#[test]
fn rip_abi_free_while_running_installs_nothing() {
    let sit = original("After_Dark_-_Totally_Twisted.sit");
    if skip_unless(&sit) {
        return;
    }
    let base = scratch("cancel");
    let root = base.join("app").join("assets");
    unsafe {
        let job = rtw_rip_start(c(&sit).as_ptr(), c(&root).as_ptr());
        assert!(!job.is_null());
        assert_eq!(
            rtw_rip_poll(job, std::ptr::null_mut(), std::ptr::null_mut()),
            RTW_RIP_RUNNING
        );
        rtw_rip_free(job);
    }
    // The detached worker has to finish its rip before it can clean up:
    // wait for its temp dir to exist (it may not yet), then to be gone.
    let temps = || {
        std::fs::read_dir(root.parent().unwrap())
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with(".rip-"))
                    .count()
            })
            .unwrap_or(0)
    };
    let t0 = Instant::now();
    while temps() == 0 && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    loop {
        if temps() == 0 {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(120),
            "cancelled job never cleaned up"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !root.exists() || files_under(&root).is_empty(),
        "a cancelled rip installed packs"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn rip_abi_null_arguments_are_inert() {
    unsafe {
        let x = CString::new("x").unwrap();
        assert!(rtw_rip_start(std::ptr::null(), x.as_ptr()).is_null());
        assert!(rtw_rip_start(x.as_ptr(), std::ptr::null()).is_null());
        let e = CString::new("").unwrap();
        assert!(rtw_rip_start(e.as_ptr(), x.as_ptr()).is_null());
        assert_eq!(
            rtw_rip_poll(std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut()),
            RTW_RIP_FAILED
        );
        assert!(rtw_rip_message(std::ptr::null_mut()).is_null());
        assert_eq!(rtw_rip_module_count(std::ptr::null()), 0);
        rtw_rip_free(std::ptr::null_mut());
    }
}
