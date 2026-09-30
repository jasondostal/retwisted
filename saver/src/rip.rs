//! First-run ripping over the C ABI (`rtw_rip_*`).
//!
//! The shippable .saver carries no Berkeley art. On first run the host asks
//! the user for their own Totally Twisted download and hands the path to
//! [`rtw_rip_start`]; the rip (`twistedrip::rip`, the pure-Rust ripper in
//! `ripper/`) runs on a background thread and the host polls it from its
//! main thread with [`rtw_rip_poll`] / [`rtw_rip_message`].
//!
//! Poll-based on purpose: `twistedrip::rip`'s progress callback fires on the
//! thread that called `rip`, i.e. the worker. The worker only ever writes a
//! small shared state (step, total, message, outcome); the host reads it on
//! its own schedule — no callbacks across the ABI, no threads for Swift to
//! reason about.
//!
//! **A half-rip never looks like a pack.** The rip writes into a temp dir
//! that is a SIBLING of the assets root (`<parent>/.rip-<pid>-<n>`), never
//! inside it — `rtw_pack_exists` asks for `<root>/<slug>/meta.json`, and the
//! temp dir has no such path. Only when `rip` has returned Ok (and the job
//! was not cancelled) is each ripped top-level directory (`<slug>/`,
//! `_shared/`) moved into the root, each with ONE atomic rename (a
//! `renamex_np(RENAME_SWAP)` when replacing an existing pack on macOS). A
//! module the new input did not contain is left alone, so ripping a
//! one-module download next to a full set adds, it does not wipe.

use std::ffi::{c_char, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::guarded;

/// Job states, as `rtw_rip_poll` returns them.
pub const RTW_RIP_RUNNING: i32 = 0;
pub const RTW_RIP_DONE: i32 = 1;
pub const RTW_RIP_FAILED: i32 = 2;

#[derive(Default)]
struct State {
    step: u32,
    total: u32,
    /// Progress line while running; the summary once done; the human
    /// error once failed.
    message: String,
    outcome: i32,
    modules: u32,
}

/// Opaque job handle (`RtwRip` in the header).
pub struct RipJob {
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    /// `rtw_rip_message`'s one scratch slot.
    scratch: CString,
}

/// What a user should read for a ripper failure. Worded for someone who
/// picked a file in an open panel, not for someone reading a backtrace; the
/// detail the ripper gave is kept in brackets where it helps a bug report.
pub fn human_error(e: &twistedrip::Error) -> String {
    use twistedrip::Error as E;
    let not_tt = "That file isn't a Totally Twisted download. Choose the floppy \
                  release (.sit), the CD release (.sit or .iso), or a folder \
                  of the expanded module files.";
    match e {
        E::Container(s) if s.starts_with("unrecognised input") => not_tt.into(),
        E::Unsupported(s) if s.contains("encrypted") => {
            "That archive is password-protected; the ripper can't open it.".into()
        }
        E::Unsupported(s) if s.contains("StuffIt compression method") => format!(
            "That StuffIt archive uses a compression method the ripper doesn't \
             support. Expand it with The Unarchiver or StuffIt Expander and \
             choose the resulting folder instead. ({s})"
        ),
        E::Unsupported(s) => format!("Unsupported data in that download ({s})."),
        E::Io(s) if s.contains("Operation not permitted") || s.contains("Permission denied") => {
            format!(
                "The screen saver wasn't allowed to read or write a file. Choose \
                 the download again with Locate…, or see docs/saver.md for the \
                 command-line route. ({s})"
            )
        }
        E::Io(s) => format!("Couldn't read or write a file ({s})."),
        E::NoSuchModule(_) => not_tt.into(),
        E::Truncated | E::Container(_) | E::Decode(_) => format!(
            "That download looks damaged or incomplete — try downloading it \
             again. ({e})"
        ),
    }
}

/// The message for an input that unpacked fine but held no modules at all.
pub const NO_MODULES: &str = "No Totally Twisted modules were found in that \
    file. It may be a different After Dark release or a demo — choose the \
    floppy (.sit) or CD (.sit / .iso) release of Totally Twisted.";

fn temp_sibling(root: &Path) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let parent = root.parent().unwrap_or(Path::new("."));
    parent.join(format!(
        ".rip-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        N.fetch_add(1, Ordering::Relaxed)
    ))
}

/// Remove `.rip-*` leftovers next to `root` from a rip that died mid-way
/// (host killed) more than an hour ago. Never touches a live job's dir.
fn sweep_stale(root: &Path) {
    let Some(parent) = root.parent() else { return };
    let Ok(rd) = std::fs::read_dir(parent) else {
        return;
    };
    for e in rd.flatten() {
        if !e.file_name().to_string_lossy().starts_with(".rip-") {
            continue;
        }
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > 3600);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Atomically exchange two existing paths (macOS `renamex_np(RENAME_SWAP)`).
#[cfg(target_os = "macos")]
fn swap_paths(a: &Path, b: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    extern "C" {
        fn renamex_np(from: *const c_char, to: *const c_char, flags: u32) -> i32;
    }
    const RENAME_SWAP: u32 = 0x2;
    let ca = CString::new(a.as_os_str().as_bytes())?;
    let cb = CString::new(b.as_os_str().as_bytes())?;
    // SAFETY: two valid NUL-terminated paths; libSystem call.
    if unsafe { renamex_np(ca.as_ptr(), cb.as_ptr(), RENAME_SWAP) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "macos"))]
fn swap_paths(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "no RENAME_SWAP",
    ))
}

/// Move the finished directory `src` to `dst`, replacing whatever is there,
/// so that `dst` is at every instant either the old complete pack or the
/// new complete pack. `src` is consumed.
fn install_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        return std::fs::rename(src, dst);
    }
    if swap_paths(src, dst).is_ok() {
        // `src` now holds the old pack.
        return std::fs::remove_dir_all(src);
    }
    // Fallback: a brief window with no pack at all (never a half one).
    let old = temp_sibling(dst);
    std::fs::rename(dst, &old)?;
    std::fs::rename(src, dst)?;
    let _ = std::fs::remove_dir_all(old);
    Ok(())
}

fn run(input: PathBuf, root: PathBuf, state: Arc<Mutex<State>>, cancel: Arc<AtomicBool>) {
    let set = |f: &dyn Fn(&mut State)| {
        if let Ok(mut s) = state.lock() {
            f(&mut s)
        }
    };
    let fail = |msg: String| {
        set(&|s| {
            s.outcome = RTW_RIP_FAILED;
            s.message = msg.clone();
        })
    };
    sweep_stale(&root);
    let tmp = temp_sibling(&root);
    if let Err(e) = std::fs::create_dir_all(&tmp) {
        return fail(human_error(&twistedrip::Error::Io(format!(
            "{}: {e}",
            tmp.display()
        ))));
    }
    let result = twistedrip::rip(
        &input,
        &tmp,
        &twistedrip::Options::default(),
        &mut |p: &twistedrip::Progress| {
            set(&|s| {
                s.step = p.step as u32;
                s.total = p.total as u32;
                s.message = p.message.clone();
            })
        },
    );
    let report = match result {
        Ok(r) => r,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            return fail(human_error(&e));
        }
    };
    let ripped: Vec<&str> = report
        .modules
        .iter()
        .filter(|m| m.dir.is_some())
        .map(|m| m.slug.as_str())
        .collect();
    if ripped.is_empty() {
        let _ = std::fs::remove_dir_all(&tmp);
        return fail(NO_MODULES.into());
    }
    if cancel.load(Ordering::SeqCst) {
        let _ = std::fs::remove_dir_all(&tmp);
        return fail("Cancelled.".into());
    }
    set(&|s| s.message = "installing".into());
    let installed = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&root)?;
        // Modules first, the shared banner last: every directory lands whole.
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&tmp)?
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect();
        dirs.sort_by_key(|p| p.file_name() == Some(twistedrip::pack::SHARED_DIR.as_ref()));
        for d in dirs {
            let Some(name) = d.file_name() else { continue };
            install_dir(&d, &root.join(name))?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    if let Err(e) = installed {
        return fail(human_error(&twistedrip::Error::Io(format!(
            "{}: {e}",
            root.display()
        ))));
    }
    let n = ripped.len() as u32;
    let summary = if n as usize == twistedrip::MODULE_NAMES.len() {
        format!("All {n} modules are ready.")
    } else {
        format!(
            "{n} module{} ready: {}.",
            if n == 1 { " is" } else { "s are" },
            ripped.join(", ")
        )
    };
    set(&|s| {
        s.modules = n;
        s.step = s.total.max(1);
        s.total = s.total.max(1);
        s.message = summary.clone();
        s.outcome = RTW_RIP_DONE;
    });
}

/// Start ripping `input` (a `.sit`, `.sit.hqx`, `.iso`, MacBinary file or a
/// folder) into the assets root `assets_root` (created if missing), on a
/// background thread. NULL for a NULL/non-UTF-8 argument.
///
/// # Safety
/// Both arguments must be valid NUL-terminated C strings, or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_rip_start(
    input: *const c_char,
    assets_root: *const c_char,
) -> *mut RipJob {
    let (Some(input), Some(root)) = (crate::cstr(input), crate::cstr(assets_root)) else {
        return std::ptr::null_mut();
    };
    if input.is_empty() || root.is_empty() {
        return std::ptr::null_mut();
    }
    let (input, root) = (PathBuf::from(input), PathBuf::from(root));
    guarded(std::ptr::null_mut(), || {
        let state = Arc::new(Mutex::new(State {
            message: format!("reading {}", input.display()),
            ..State::default()
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (st, cn) = (state.clone(), cancel.clone());
        let spawned = std::thread::Builder::new()
            .name("retwisted-rip".into())
            .spawn(move || {
                let st2 = st.clone();
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(input, root, st, cn)
                }))
                .is_err()
                {
                    if let Ok(mut s) = st2.lock() {
                        s.outcome = RTW_RIP_FAILED;
                        s.message = "The ripper hit an internal error on that file.".into();
                    }
                }
            });
        if spawned.is_err() {
            return std::ptr::null_mut();
        }
        Box::into_raw(Box::new(RipJob {
            state,
            cancel,
            scratch: CString::default(),
        }))
    })
}

/// `RTW_RIP_RUNNING` / `RTW_RIP_DONE` / `RTW_RIP_FAILED`, and (when the
/// pointers are non-NULL) the ripper's step/total for a progress bar.
/// Returns `RTW_RIP_FAILED` for a NULL job.
///
/// # Safety
/// `job` must be a live pointer from [`rtw_rip_start`], or NULL; `step` and
/// `total` must be writable or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_rip_poll(job: *const RipJob, step: *mut u32, total: *mut u32) -> i32 {
    let Some(job) = job.as_ref() else {
        return RTW_RIP_FAILED;
    };
    guarded(RTW_RIP_FAILED, || {
        let Ok(s) = job.state.lock() else {
            return RTW_RIP_FAILED;
        };
        if let Some(p) = step.as_mut() {
            *p = s.step;
        }
        if let Some(p) = total.as_mut() {
            *p = s.total;
        }
        s.outcome
    })
}

/// The job's current line: progress while running, a one-line summary when
/// done, the human-worded error when failed. UTF-8, in the job's ONE scratch
/// slot — valid until the next `rtw_rip_message` or `rtw_rip_free`.
///
/// # Safety
/// `job` must be a live pointer from [`rtw_rip_start`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_rip_message(job: *mut RipJob) -> *const c_char {
    let Some(job) = job.as_mut() else {
        return std::ptr::null();
    };
    guarded(std::ptr::null(), || {
        let msg = job
            .state
            .lock()
            .map(|s| s.message.clone())
            .unwrap_or_default();
        job.scratch = CString::new(msg.replace('\0', " ")).unwrap_or_default();
        job.scratch.as_ptr()
    })
}

/// How many module packs a finished job installed (0 until DONE).
///
/// # Safety
/// `job` must be a live pointer from [`rtw_rip_start`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_rip_module_count(job: *const RipJob) -> u32 {
    let Some(job) = job.as_ref() else { return 0 };
    guarded(0, || job.state.lock().map(|s| s.modules).unwrap_or(0))
}

/// Release a job. Freeing a job that is still RUNNING cancels it: the worker
/// finishes the rip it is in the middle of (the ripper has no abort point),
/// then deletes its temp dir instead of installing. NULL-safe.
///
/// # Safety
/// `job` must come from [`rtw_rip_start`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn rtw_rip_free(job: *mut RipJob) {
    if job.is_null() {
        return;
    }
    guarded((), || {
        let job = Box::from_raw(job);
        job.cancel.store(true, Ordering::SeqCst);
    });
}
