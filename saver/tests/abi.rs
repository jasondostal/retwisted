//! Drive the screensaver's C ABI exactly the way the Swift view does.
//!
//! This is the only automated coverage the .saver bundle gets — nothing in
//! CI can look at a screen — so it asserts the two failure modes that
//! actually bite: the runtime panicking somewhere inside a few hundred
//! ticks (which in the real host means `legacyScreenSaver` crashes), and
//! the frame coming back empty (a screensaver that runs but draws a flat
//! rectangle of field colour, which looks exactly like "it works" in a
//! thumbnail).

use retwisted_saver::*;
use std::ffi::{CStr, CString};
use std::path::Path;

const SLUG: &str = "bungee-roulette";

fn assets() -> Option<CString> {
    // tests run with CWD = the crate dir
    let p = Path::new("../assets");
    if !p.join(SLUG).join("meta.json").exists() {
        eprintln!("assets/{SLUG} missing — skipping");
        return None;
    }
    Some(CString::new(p.to_str().unwrap()).unwrap())
}

#[test]
fn abi_runs_hundreds_of_ticks_and_draws_something() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null(), "rtw_create returned NULL");

        let tick = rtw_tick_ms(rt) as u64;
        assert!(tick >= 1 && tick <= 1000, "implausible tick period {tick}");

        let mut field = [0u8; 3];
        rtw_field(rt, field.as_mut_ptr());
        let field_px = (field[0] as u32) << 16 | (field[1] as u32) << 8 | field[2] as u32;

        let n = (rtw_width() * rtw_height()) as usize;
        let mut drew = false;
        let mut sounds = 0;
        // 600 host frames at the module's own period: ~24 s of screensaver.
        for i in 0..600u64 {
            let now = i * tick;
            let (h, m, s) = (
                ((12 * 3600 + now / 1000) / 3600 % 24) as u8,
                ((now / 1000) / 60 % 60) as u8,
                (now / 1000 % 60) as u8,
            );
            // wander the mouse in and out of the field, click sometimes
            let (mx, my) = if i % 7 == 0 { (-1, -1) } else { ((i as i32 * 3) % 640, (i as i32) % 480) };
            rtw_tick(rt, now, h, m, s, mx, my, i % 97 == 0, i % 131 == 0);

            let px = rtw_pixels(rt);
            assert!(!px.is_null(), "rtw_pixels returned NULL at frame {i}");
            let buf = std::slice::from_raw_parts(px, n);
            if buf.iter().any(|&p| p != field_px) {
                drew = true;
            }
            // every pixel must be a legal 0RGB value — a stray alpha byte
            // would show up as garbage in the CGImage the host builds
            assert!(buf.iter().all(|&p| p >> 24 == 0), "non-zero top byte at frame {i}");

            while !rtw_next_sound(rt).is_null() {
                sounds += 1;
                assert!(sounds < 10_000, "sound queue never drains");
            }
        }
        assert!(drew, "600 frames and the field was never painted over");
        eprintln!("abi: tick={tick}ms sounds fired={sounds}");

        rtw_destroy(rt);
    }
}

#[test]
fn abi_survives_a_stall_and_a_backwards_clock() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        // A display-sleep-sized jump must not replay hours of animation:
        // the catch-up burst is capped at 4 ticks.
        assert_eq!(rtw_tick(rt, 0, 12, 0, 0, 0, 0, false, false), 0, "first tick anchors");
        let ran = rtw_tick(rt, 3_600_000, 13, 0, 0, 0, 0, false, false);
        assert!(ran <= 4, "catch-up burst ran {ran} ticks");
        // A clock that goes backwards (host bug, or a fresh view reusing the
        // runtime) must be inert rather than panicking on the subtraction.
        assert_eq!(rtw_tick(rt, 0, 13, 0, 0, 0, 0, false, false), 0);
        assert!(!rtw_pixels(rt).is_null());
        rtw_destroy(rt);
    }
}

#[test]
fn abi_rejects_a_bogus_slug() {
    let Some(dir) = assets() else { return };
    let slug = CString::new("not-a-module").unwrap();
    unsafe {
        assert!(rtw_create(slug.as_ptr(), dir.as_ptr()).is_null());
        assert!(rtw_create(std::ptr::null(), dir.as_ptr()).is_null());
    }
}

/// The configure sheet builds itself out of these calls and nothing else,
/// so whatever it can read it must also be able to set. Walk the module's
/// controls the way the sheet does — count, name, kind, range, default,
/// popup labels — and drive every one of them to its extremes.
#[test]
fn abi_control_introspection_describes_a_settable_ui() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());

        let n = rtw_control_count(rt);
        assert!(n > 0, "bungee roulette has three controls, ABI reports none");

        for i in 0..n as i32 {
            let name_ptr = rtw_control_name(rt, i);
            assert!(!name_ptr.is_null(), "control {i} has no name");
            let name = CStr::from_ptr(name_ptr).to_str().unwrap().to_string();
            assert!(!name.is_empty(), "control {i} name is empty");

            let kind = rtw_control_kind(rt, i);
            assert!((0..=2).contains(&kind), "control {i} kind {kind}");
            let (lo, hi) = (rtw_control_min(rt, i), rtw_control_max(rt, i));
            assert!(lo <= hi, "control {i} range {lo}..{hi} is inverted");
            let def = rtw_control_default(rt, i);
            assert!(
                (lo..=hi).contains(&def),
                "control {i} ({name}) default {def} outside {lo}..={hi}"
            );

            let items = rtw_control_item_count(rt, i);
            if kind == 1 {
                assert!(items >= 2, "popup {name} has {items} items");
                assert_eq!(
                    hi - lo + 1,
                    items as i32,
                    "popup {name}: range {lo}..={hi} does not cover {items} items"
                );
                for it in 0..items as i32 {
                    let p = rtw_control_item(rt, i, it);
                    assert!(!p.is_null(), "popup {name} item {it} has no label");
                    assert!(!CStr::from_ptr(p).to_bytes().is_empty());
                }
                assert!(rtw_control_item(rt, i, items as i32).is_null(), "{name}: item past the end");
                assert!(rtw_control_item(rt, i, -1).is_null(), "{name}: negative item");
            } else {
                assert_eq!(items, 0, "non-popup {name} reports {items} items");
                assert!(rtw_control_item(rt, i, 0).is_null());
            }

            // min, max, back to default — each with ticks in between, which
            // is what the OK button does to a live view (and what makes
            // modules that restart themselves actually restart).
            for v in [lo, hi, def] {
                rtw_set_control(rt, i, v);
                for t in 0..30u64 {
                    rtw_tick(rt, (t + 1) * 40, 12, 0, 0, 320, 240, false, false);
                }
                assert!(!rtw_pixels(rt).is_null(), "{name} = {v} killed the frame");
            }
        }

        // Out-of-range introspection is inert, not fatal.
        assert_eq!(rtw_control_kind(rt, n as i32), -1);
        assert_eq!(rtw_control_kind(rt, -1), -1);
        assert!(rtw_control_name(rt, n as i32).is_null());
        assert_eq!(rtw_control_item_count(rt, 99), 0);

        rtw_destroy(rt);
    }
}

/// The one popup whose raw values are NOT 0-based: the sheet ticks the
/// right item only if the ABI says Jumper runs 1..=6. It used to say so
/// because `popup_base()` held a slug-keyed exception; now it says so
/// because the module's own `ControlDef` declares `base: 1`, and this test
/// is what proves the table's removal changed nothing.
#[test]
fn abi_bungee_jumper_popup_is_the_1_based_mac_menu() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        assert_eq!(rtw_control_kind(rt, 0), 1, "control 0 is the Jumper popup");
        assert_eq!(rtw_control_item_count(rt, 0), 6);
        assert_eq!((rtw_control_min(rt, 0), rtw_control_max(rt, 0)), (1, 6));
        assert_eq!(rtw_control_default(rt, 0), 1);
        // item 2 => raw 3 => the third jumper (MENU 1000 item 3), the value
        // tests/abi.rs has used since the wrapper landed and the one the
        // screenshot harness drives.
        let third = CStr::from_ptr(rtw_control_item(rt, 0, 2)).to_str().unwrap().to_string();
        match current_pack(SLUG) {
            Some(pack) => assert_eq!(third, pack.menu(1000)[2], "item (raw 3 - min) must be MENU item 3"),
            None => assert_eq!(third, "3", "a pack without MENUs numbers the items"),
        }
        rtw_destroy(rt);
    }
}

#[test]
fn abi_set_control_takes_the_modules_own_raw_values() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        // Jumper = Cow (raw 3, 1-based as the original's mVal 1000), and an
        // out-of-range index must be ignored rather than panic.
        rtw_set_control(rt, 0, 3);
        rtw_set_control(rt, 99, 1);
        rtw_set_control(rt, -1, 1);
        for i in 0..200u64 {
            rtw_tick(rt, i * 40, 12, 0, 0, 320, 240, false, false);
        }
        assert!(!rtw_pixels(rt).is_null());
        rtw_destroy(rt);
    }
}

/// Every module's popup must agree with itself: the factory default has to
/// be one of the items the sheet will show, i.e. inside
/// `[min, min + item_count)`. A popup whose base is wrong fails here — a
/// 0-based def for bungee's 1-based Jumper would put the default (1) one
/// past the last item, and a 1-based def for anyone else would put their
/// default (0) below the first.
#[test]
fn abi_every_popup_default_lies_inside_its_own_items() {
    let root = Path::new("../assets");
    let mut checked = 0usize;
    for slug in app::modules::SLUGS {
        if !root.join(slug).join("meta.json").exists() {
            continue;
        }
        let dir = CString::new(root.to_str().unwrap()).unwrap();
        let cslug = CString::new(*slug).unwrap();
        unsafe {
            let rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            if rt.is_null() {
                continue; // module not implemented for this pack
            }
            for i in 0..rtw_control_count(rt) as i32 {
                if rtw_control_kind(rt, i) != RTW_CONTROL_POPUP {
                    continue;
                }
                let items = rtw_control_item_count(rt, i) as i32;
                let (lo, hi) = (rtw_control_min(rt, i), rtw_control_max(rt, i));
                let def = rtw_control_default(rt, i);
                let name = CStr::from_ptr(rtw_control_name(rt, i)).to_str().unwrap().to_string();
                assert_eq!(hi - lo + 1, items, "{slug} popup {name}: range vs items");
                assert!(
                    (lo..lo + items).contains(&def),
                    "{slug} popup {name}: default {def} is outside [{lo}, {})",
                    lo + items
                );
                // and the label the sheet would tick actually exists
                assert!(
                    !rtw_control_item(rt, i, def - lo).is_null(),
                    "{slug} popup {name}: no label at the default"
                );
                checked += 1;
            }
            rtw_destroy(rt);
        }
    }
    assert!(checked >= 10, "only {checked} popups checked — packs missing?");
}

/// The bundle ships every pack and lets the sheet pick, so `rtw_create`
/// from the ROOT (not the pack dir) has to work for all thirteen slugs —
/// that is the single call the module popup's OK button makes.
#[test]
fn abi_creates_every_catalogued_module_from_the_assets_root() {
    let root = Path::new("../assets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let n = rtw_module_count();
    assert_eq!(n as usize, app::modules::SLUGS.len());
    let mut built = 0usize;
    for i in 0..n {
        unsafe {
            let slug_ptr = rtw_module_slug(i);
            assert!(!slug_ptr.is_null());
            let slug = CStr::from_ptr(slug_ptr).to_str().unwrap().to_string();
            let name = CStr::from_ptr(rtw_module_name(i)).to_str().unwrap().to_string();
            assert!(!name.is_empty(), "{slug} has no display name");
            if !root.join(&slug).join("meta.json").exists() {
                eprintln!("assets/{slug} missing — skipping");
                continue;
            }
            let cslug = CString::new(slug.clone()).unwrap();
            let rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            assert!(!rt.is_null(), "rtw_create({slug}) from the assets root returned NULL");
            rtw_destroy(rt);
            built += 1;
        }
    }
    assert!(built >= 10, "only {built} modules built — packs missing?");
}

/// Every module, created and driven the way the swapped-in runtime is: its
/// stored controls pushed in, then ticked. A panic in any one of them is a
/// crashed `legacyScreenSaver`, and now ANY module can be the one running.
#[test]
fn abi_every_module_ticks_without_panicking() {
    let root = Path::new("../assets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let mut ran = 0usize;
    for i in 0..rtw_module_count() {
        unsafe {
            let slug = CStr::from_ptr(rtw_module_slug(i)).to_str().unwrap().to_string();
            if !root.join(&slug).join("meta.json").exists() {
                continue;
            }
            let cslug = CString::new(slug.clone()).unwrap();
            let rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            assert!(!rt.is_null(), "{slug}: rtw_create failed");

            // The sheet's OK path: defaults in first, before any tick.
            for c in 0..rtw_control_count(rt) as i32 {
                rtw_set_control(rt, c, rtw_control_default(rt, c));
            }

            let tick = (rtw_tick_us(rt) as u64 / 1000).max(1);
            let px = (rtw_width() * rtw_height()) as usize;
            for f in 0..100u64 {
                let now = f * tick;
                let (mx, my) = if f % 7 == 0 { (-1, -1) } else { ((f as i32 * 3) % 640, (f as i32) % 480) };
                rtw_tick(rt, now, 12, (f % 60) as u8, (f % 60) as u8, mx, my, f % 31 == 0, false);
                let p = rtw_pixels(rt);
                assert!(!p.is_null(), "{slug}: NULL frame at {f}");
                let buf = std::slice::from_raw_parts(p, px);
                assert!(buf.iter().all(|&v| v >> 24 == 0), "{slug}: top byte set at {f}");
                while !rtw_next_sound(rt).is_null() {}
            }
            rtw_destroy(rt);
            ran += 1;
        }
    }
    assert!(ran >= 10, "only {ran} modules ticked — packs missing?");
}

/// Destroy-then-create is what the Module popup's OK does to a live view,
/// and doing it repeatedly must neither leak a pack nor hand back a runtime
/// still speaking the old module's controls.
#[test]
fn abi_runtimes_can_be_swapped_one_for_another() {
    let root = Path::new("../assets");
    if !root.join("bungee-roulette").join("meta.json").exists() {
        eprintln!("assets missing — skipping");
        return;
    }
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let order = ["bungee-roulette", "shock-clocks", "bungee-roulette", "mowin-boris"];
    unsafe {
        let mut rt = std::ptr::null_mut();
        for slug in order {
            if !root.join(slug).join("meta.json").exists() {
                continue;
            }
            // exactly the view's swap: old one down first, never two live.
            rtw_destroy(rt);
            let cslug = CString::new(slug).unwrap();
            rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            assert!(!rt.is_null(), "{slug}: swap-in failed");
            let n = rtw_control_count(rt);
            for i in 0..n as i32 {
                let name = CStr::from_ptr(rtw_control_name(rt, i)).to_str().unwrap().to_string();
                assert!(!name.is_empty(), "{slug} control {i}");
            }
            for f in 1..=40u64 {
                rtw_tick(rt, f * 40, 12, 0, 0, 320, 240, false, false);
            }
            assert!(!rtw_pixels(rt).is_null(), "{slug}: dead after swap");
        }
        rtw_destroy(rt);
    }
}

/// The host timer comes from the module's grid, and one of the two grids is
/// After Dark's 16.625 ms Mac tick — which is why `rtw_tick_us` exists.
#[test]
fn abi_reports_the_tick_period_in_microseconds() {
    let Some(dir) = assets() else { return };
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        // bungee rides the Mac tick
        assert_eq!(rtw_tick_us(rt), 16_625);
        assert_eq!(rtw_tick_ms(rt), 16, "ms is the rounded-down view");
        rtw_destroy(rt);
    }
}

// ---------------------------------------------------------------------------
// 2026-09-13: the three debts the multi-module bundle left behind.

/// A root holding symlinks to just the named packs, so a test can prove what
/// a call did *not* need to touch. Returns None when the packs are missing.
fn narrow_root(tag: &str, slugs: &[&str]) -> Option<std::path::PathBuf> {
    let src = Path::new("../assets").canonicalize().ok()?;
    for s in slugs {
        if !src.join(s).join("meta.json").exists() {
            eprintln!("assets/{s} missing — skipping");
            return None;
        }
    }
    let dir = std::env::temp_dir().join(format!("rtw-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).ok()?;
    for s in slugs {
        std::os::unix::fs::symlink(src.join(s), dir.join(s)).ok()?;
    }
    Some(dir)
}

/// **Create loads one pack, not thirteen.** The bundle carries 83 MB of art
/// for thirteen modules and the ledger for `0fc83c3` suspected `rtw_create`
/// of pulling all of it in. It does not — and this is what says so without
/// having to trust a measurement: each runtime is built from a root that
/// physically contains only its own pack, so needing a neighbour's art would
/// be a NULL rather than a slow frame. The swap is driven through the same
/// two roots in the view's order (destroy, then create), which is also the
/// "never two tickable runtimes" shape.
#[test]
fn abi_create_needs_only_the_selected_modules_pack() {
    let Some(a) = narrow_root("only-bungee", &["bungee-roulette"]) else { return };
    let Some(b) = narrow_root("only-clocks", &["shock-clocks"]) else { return };
    let (ca, cb) = (
        CString::new(a.to_str().unwrap()).unwrap(),
        CString::new(b.to_str().unwrap()).unwrap(),
    );
    let (bungee, clocks) = (
        CString::new("bungee-roulette").unwrap(),
        CString::new("shock-clocks").unwrap(),
    );
    unsafe {
        // the neighbour is genuinely absent from each root
        assert!(rtw_pack_exists(bungee.as_ptr(), ca.as_ptr()));
        assert!(!rtw_pack_exists(clocks.as_ptr(), ca.as_ptr()));
        assert!(rtw_pack_exists(clocks.as_ptr(), cb.as_ptr()));
        assert!(!rtw_pack_exists(bungee.as_ptr(), cb.as_ptr()));

        let rt = rtw_create(bungee.as_ptr(), ca.as_ptr());
        assert!(!rt.is_null(), "bungee needs a pack that is not its own");
        for f in 1..=60u64 {
            rtw_tick(rt, f * 20, 12, 0, 0, 320, 240, false, false);
        }
        assert!(!rtw_pixels(rt).is_null());
        // the swap: old one down FIRST, then the other root's module up.
        rtw_destroy(rt);
        let rt = rtw_create(clocks.as_ptr(), cb.as_ptr());
        assert!(!rt.is_null(), "shock-clocks needs a pack that is not its own");
        for f in 1..=60u64 {
            rtw_tick(rt, f * 40, 12, 0, 0, 320, 240, false, false);
        }
        assert!(!rtw_pixels(rt).is_null());
        rtw_destroy(rt);
    }
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

/// The decoded-sprite cache is bounded. Chameleon is the worst case in the
/// catalogue — it recolours, and the cache is keyed by `(png, clut)`, so the
/// same compound is decoded once per palette — and it walked an unbounded
/// process to 130 MB RSS in 20 000 frames before the cap. Driven here
/// against a deliberately tiny cap (the real one takes ten minutes of real
/// animation to cross, which is not a unit test), because what needs
/// proving is the mechanism: the cache is weighed, dropped, and REFILLED —
/// a cleared cache that nothing refills is a black screensaver.
#[test]
fn abi_sprite_cache_is_dropped_when_it_passes_its_cap() {
    let root = Path::new("../assets");
    if !root.join("chameleon").join("meta.json").exists() {
        eprintln!("assets/chameleon missing — skipping");
        return;
    }
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let slug = CString::new("chameleon").unwrap();
    const CAP: usize = 512 * 1024;
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        assert_eq!((*rt).cache_bytes(), 0, "nothing decoded before the first frame");
        (*rt).set_cache_cap(CAP);
        let mut field = [0u8; 3];
        rtw_field(rt, field.as_mut_ptr());
        let field_px = (field[0] as u32) << 16 | (field[1] as u32) << 8 | field[2] as u32;
        let n = (rtw_width() * rtw_height()) as usize;

        let mut peak = 0usize;
        let mut drops = 0;
        let mut drew_after_drop = false;
        let mut last = 0usize;
        for f in 1..=2_000u64 {
            rtw_tick(rt, f * 40, 12, (f % 60) as u8, (f % 60) as u8, 320, 240, false, false);
            let px = rtw_pixels(rt);
            assert!(!px.is_null(), "no frame at {f}");
            let bytes = (*rt).cache_bytes();
            if bytes < last {
                drops += 1;
            }
            if drops > 0 {
                let buf = std::slice::from_raw_parts(px, n);
                if buf.iter().any(|&p| p != field_px) {
                    drew_after_drop = true;
                }
            }
            last = bytes;
            peak = peak.max(bytes);
            while !rtw_next_sound(rt).is_null() {}
            if drops >= 2 && drew_after_drop {
                eprintln!("chameleon: {drops} cache drops by frame {f}, peak {} KB", peak >> 10);
                break;
            }
        }
        assert!(drops >= 2, "the cache was never dropped ({} KB peak)", peak >> 10);
        assert!(drew_after_drop, "nothing was drawn after the cache was dropped");
        assert!(peak <= CAP + (4 * 640 * 480), "peak {} KB over the {} KB cap", peak >> 10, CAP >> 10);
        // The shipped cap is the one the doc comment quotes.
        assert_eq!(retwisted_saver::image_cache_cap(), 24 * 1024 * 1024);
        rtw_destroy(rt);
    }
}

/// Every slider carries the original's `sUnt` words, and the sheet can ask
/// for them the same way it asks for popup items. The two ends are what goes
/// under the track; `rtw_control_band_for` is the rule for the readout.
#[test]
fn abi_sliders_carry_their_sunt_words() {
    let root = Path::new("../assets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let mut sliders = 0usize;
    let mut stale = 0usize;
    for i in 0..rtw_module_count() {
        unsafe {
            let slug = CStr::from_ptr(rtw_module_slug(i)).to_str().unwrap().to_string();
            if !root.join(&slug).join("meta.json").exists() {
                continue;
            }
            if current_pack(&slug).is_none() {
                stale += 1;
                continue;
            }
            let cslug = CString::new(slug.clone()).unwrap();
            let rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            assert!(!rt.is_null(), "{slug}");
            for c in 0..rtw_control_count(rt) as i32 {
                let name = CStr::from_ptr(rtw_control_name(rt, c)).to_str().unwrap().to_string();
                let n = rtw_control_band_count(rt, c) as i32;
                if rtw_control_kind(rt, c) != RTW_CONTROL_SLIDER {
                    assert_eq!(n, 0, "{slug} {name}: a popup/checkbox with slider words");
                    assert!(rtw_control_band_label(rt, c, 0).is_null());
                    assert_eq!(rtw_control_band_for(rt, c, 0), -1);
                    continue;
                }
                sliders += 1;
                assert!(n >= 2, "{slug} slider {name} has {n} words — nothing to label the ends with");
                let (lo, hi) = (rtw_control_min(rt, c), rtw_control_max(rt, c));
                let mut prev = i32::MIN;
                for b in 0..n {
                    let at = rtw_control_band_value(rt, c, b);
                    assert!(at > prev, "{slug} {name}: word {b} at {at} is not past {prev}");
                    assert!((lo..=hi).contains(&at), "{slug} {name}: word at {at} outside {lo}..={hi}");
                    prev = at;
                    let w = rtw_control_band_label(rt, c, b);
                    assert!(!w.is_null(), "{slug} {name}: word {b} missing");
                    assert!(!CStr::from_ptr(w).to_bytes().is_empty());
                }
                assert!(rtw_control_band_label(rt, c, n).is_null(), "{slug} {name}: past the end");
                assert!(rtw_control_band_label(rt, c, -1).is_null());
                // every legal raw value lands in a word, the ends included
                assert_eq!(rtw_control_band_for(rt, c, lo), 0, "{slug} {name}: low end");
                assert_eq!(rtw_control_band_for(rt, c, hi), n - 1, "{slug} {name}: high end");
                for v in lo..=hi {
                    let b = rtw_control_band_for(rt, c, v);
                    assert!((0..n).contains(&b), "{slug} {name}: raw {v} -> word {b}");
                }
            }
            rtw_destroy(rt);
        }
    }
    assert!(sliders >= 15 || (sliders == 0 && stale >= 10), "only {sliders} sliders checked ({stale} stale packs) — packs missing?");
}

/// The one the sheet screenshot is read against: bungee's Jumps words are
/// the pack's `sUnt 1001` rows verbatim, six of them from 0 to 100, and 100
/// is a band of its own (the module's own `raw/20` bucket 5, the 250-jump
/// wipe).
#[test]
fn abi_bungee_jumps_reads_the_packs_sunt_words() {
    let Some(dir) = assets() else { return };
    let Some(pack) = current_pack(SLUG) else { return };
    let jumps = pack.slider_words(1001);
    let equipment = pack.slider_words(1002);
    let slug = CString::new(SLUG).unwrap();
    unsafe {
        let rt = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!rt.is_null());
        assert_eq!(rtw_control_kind(rt, 1), RTW_CONTROL_SLIDER);
        assert_eq!(rtw_control_band_count(rt, 1), 6);
        assert_eq!(rtw_control_band_count(rt, 1) as usize, jumps.len());
        let word = |c, b| CStr::from_ptr(rtw_control_band_label(rt, c, b)).to_str().unwrap().to_string();
        for (b, (at, w)) in jumps.iter().enumerate() {
            assert_eq!(word(1, b as i32), *w, "Jumps word {b}");
            assert_eq!(rtw_control_band_value(rt, 1, b as i32), *at);
        }
        assert_eq!(rtw_control_band_value(rt, 1, 0), 0);
        assert_eq!(rtw_control_band_value(rt, 1, 5), 100);
        assert_eq!(rtw_control_band_for(rt, 1, 50), 2, "the default is the third word");
        assert_eq!(rtw_control_band_for(rt, 1, 99), 4);
        assert_eq!(rtw_control_band_for(rt, 1, 100), 5);
        // Equipment's words, the resource's own spelling included
        for (b, (_, w)) in equipment.iter().enumerate() {
            assert_eq!(word(2, b as i32), *w, "Equipment word {b}");
        }
        rtw_destroy(rt);
    }
}

/// The pack for `slug`, or `None` (with a note) when it is absent or was
/// ripped before the rippers carried `sUnt`/`MENU`.
fn current_pack(slug: &str) -> Option<engine::Pack> {
    let pack = engine::Pack::load(&Path::new("../assets").join(slug)).ok()?;
    if pack.meta.slider_words.is_empty() && pack.meta.menus.is_empty() {
        eprintln!("assets/{slug} predates sUnt/MENU ripping — skipping");
        return None;
    }
    Some(pack)
}

/// The probe the Module popup greys itself out with. It has to answer for
/// exactly the path `rtw_create` would load — both spellings of
/// `assets_dir` — and it must not need a runtime, a pack parse or a
/// catalogue lookup to say no.
#[test]
fn abi_pack_probe_agrees_with_create() {
    let root = Path::new("../assets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    unsafe {
        for i in 0..rtw_module_count() {
            let slug = CStr::from_ptr(rtw_module_slug(i)).to_str().unwrap().to_string();
            let cslug = CString::new(slug.clone()).unwrap();
            let on_disk = root.join(&slug).join("meta.json").exists();
            assert_eq!(rtw_pack_exists(cslug.as_ptr(), dir.as_ptr()), on_disk, "{slug}");
            let rt = rtw_create(cslug.as_ptr(), dir.as_ptr());
            assert_eq!(!rt.is_null(), on_disk, "{slug}: probe vs create");
            rtw_destroy(rt);
        }
        // the pack dir itself, not the root: same answer
        let pack = CString::new(root.join(SLUG).to_str().unwrap()).unwrap();
        let cslug = CString::new(SLUG).unwrap();
        if root.join(SLUG).join("meta.json").exists() {
            assert!(rtw_pack_exists(cslug.as_ptr(), pack.as_ptr()), "direct pack dir");
        }
        // and the nos
        let bogus = CString::new("not-a-module").unwrap();
        assert!(!rtw_pack_exists(bogus.as_ptr(), dir.as_ptr()));
        let nowhere = CString::new("/nowhere/at/all").unwrap();
        assert!(!rtw_pack_exists(cslug.as_ptr(), nowhere.as_ptr()));
        assert!(!rtw_pack_exists(std::ptr::null(), dir.as_ptr()));
        assert!(!rtw_pack_exists(cslug.as_ptr(), std::ptr::null()));
    }
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let dst = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_tree(&e.path(), &dst)?;
        } else {
            std::fs::copy(e.path(), &dst)?;
        }
    }
    Ok(())
}

/// **A pack is loaded once per process, and only while a module on it is
/// instantiated.** The host runs three or four views of the saver in one
/// process, usually on the same module; they share one parsed pack and one
/// decoded-sprite cache, and the last `rtw_destroy` frees both. Driven on a
/// private COPY of a pack (its own canonical path), so no other test's
/// runtimes can keep it alive or share it.
#[test]
fn abi_pack_is_shared_by_live_runtimes_and_freed_with_the_last() {
    let src = Path::new("../assets/flying-toilets");
    if !src.join("meta.json").exists() {
        eprintln!("assets/flying-toilets missing — skipping");
        return;
    }
    let root = std::env::temp_dir().join(format!("rtw-shared-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(src, &root.join("flying-toilets")).unwrap();
    let pack_dir = root.join("flying-toilets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    let slug = CString::new("flying-toilets").unwrap();
    unsafe {
        assert!(!pack_is_loaded(&pack_dir), "nothing loaded before a runtime exists");
        // the catalogue and the probe never load it
        assert!(rtw_module_count() > 0);
        assert!(rtw_pack_exists(slug.as_ptr(), dir.as_ptr()));
        assert!(!pack_is_loaded(&pack_dir), "rtw_pack_exists must not load the pack");

        let a = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(!a.is_null());
        assert!(pack_is_loaded(&pack_dir));
        // the pack dir itself is the same pack as root/<slug>
        let direct = CString::new(pack_dir.to_str().unwrap()).unwrap();
        let b = rtw_create(slug.as_ptr(), direct.as_ptr());
        assert!(!b.is_null());
        assert!((*a).shares_pack_with(&*b), "two views of one module must share the pack");
        run_ticks(a, 0, 50);
        assert!(!rtw_pixels(a).is_null());
        let decoded = (*a).cache_len();
        assert!(decoded > 0, "a frame decoded nothing");
        assert_eq!((*b).cache_len(), decoded, "the second view sees the first's decodes");
        run_ticks(b, 0, 50);
        assert!(!rtw_pixels(b).is_null());

        rtw_destroy(a);
        assert!(pack_is_loaded(&pack_dir), "still in use by the second runtime");
        rtw_destroy(b);
        assert!(!pack_is_loaded(&pack_dir), "the last destroy must free the pack");

        // and it comes back fresh on the next create, with an empty cache
        let c = rtw_create(slug.as_ptr(), dir.as_ptr());
        assert!(pack_is_loaded(&pack_dir));
        assert_eq!((*c).cache_len(), 0);
        rtw_destroy(c);
        assert!(!pack_is_loaded(&pack_dir));
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Tick `rt` `n` times on its own grid, starting at host time `t0` (ms).
unsafe fn run_ticks(rt: *mut Runtime, t0: u64, n: u64) -> u64 {
    let us = rtw_tick_us(rt) as u64;
    let mut t = t0;
    for k in 1..=n {
        t = t0 + (k * us).div_ceil(1000);
        rtw_tick(rt, t, 12, 0, 0, -1, -1, false, false);
    }
    t
}

fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// The music channel is its own handle, and it follows the module: a module
/// with no music gets NULL, one with music renders audible PCM while its
/// `(song, plays)` state is Some, the module's own Music gate silences it,
/// and destroying the runtime under a live handle leaves silence, not a
/// crash (the host's audio thread can be one callback behind).
#[test]
fn abi_music_channel_follows_the_module() {
    let root = Path::new("../assets");
    let dir = CString::new(root.to_str().unwrap()).unwrap();
    assert_eq!(rtw_music_rate(), 22254);
    unsafe {
        // no music in the pack -> no channel
        if root.join(SLUG).join("meta.json").exists() {
            let s = CString::new(SLUG).unwrap();
            let rt = rtw_create(s.as_ptr(), dir.as_ptr());
            assert!(rtw_music_open(rt).is_null(), "{SLUG} has no music");
            rtw_destroy(rt);
        }

        // Mowin' Boris: the dawn cue, requested from the first tick.
        if root.join("mowin-boris").join("meta.json").exists() {
            let s = CString::new("mowin-boris").unwrap();
            let rt = rtw_create(s.as_ptr(), dir.as_ptr());
            run_ticks(rt, 0, 2);
            let m = rtw_music_open(rt);
            assert!(!m.is_null(), "boris packs cmid 40");
            assert!(rtw_music_playing(m), "opened mid-song -> plays from the top");
            rtw_music_set_volume(m, 0.4);
            let mut buf = vec![0.0f32; 22254 / 2];
            // a cue may open on a rest: look through the first two seconds
            for _ in 0..4 {
                rtw_music_render(m, buf.as_mut_ptr(), buf.len() as u32);
                if peak(&buf) > 0.001 {
                    break;
                }
            }
            assert!(peak(&buf) > 0.001, "boris' dawn cue rendered silence");
            // opening again is the same channel, not a second synth
            let again = rtw_music_open(rt);
            assert!(!again.is_null());
            rtw_music_close(again);
            assert!(rtw_music_playing(m), "closing a second handle must not stop the first");
            // runtime gone under a live handle: silence, no crash
            rtw_destroy(rt);
            assert!(!rtw_music_playing(m));
            rtw_music_render(m, buf.as_mut_ptr(), buf.len() as u32);
            assert_eq!(peak(&buf), 0.0, "a destroyed runtime's music must stop");
            rtw_music_close(m);
        }

        // Mime Hunt: the module's own Music gate (raw < 5 = no plays)
        // decides — the channel just obeys.
        if root.join("mime-hunt").join("meta.json").exists() {
            let s = CString::new("mime-hunt").unwrap();
            for (raw, want) in [(0, false), (50, true)] {
                let rt = rtw_create(s.as_ptr(), dir.as_ptr());
                let music = (0..rtw_control_count(rt) as i32)
                    .find(|&c| {
                        CStr::from_ptr(rtw_control_name(rt, c)).to_str().unwrap().contains("Music")
                    })
                    .expect("mime hunt has a Music control");
                rtw_set_control(rt, music, raw);
                let m = rtw_music_open(rt);
                assert!(!m.is_null(), "mime hunt packs cmid 10");
                assert!(!rtw_music_playing(m), "nothing asked for yet");
                run_ticks(rt, 0, 200);
                assert_eq!(rtw_music_playing(m), want, "Music raw {raw}");
                let mut buf = vec![0.0f32; 22254 * 2];
                rtw_music_render(m, buf.as_mut_ptr(), buf.len() as u32);
                assert_eq!(peak(&buf) > 0.001, want, "Music raw {raw}: audible?");
                rtw_music_close(m);
                rtw_destroy(rt);
            }
        }
    }
}
