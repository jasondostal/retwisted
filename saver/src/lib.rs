//! C ABI for embedding a retwisted module in a native host — today the
//! macOS `ScreenSaverView` in `macos/`, tomorrow whatever else wants a
//! 640×480 frame and a list of sounds.
//!
//! Shape of the contract:
//!
//! ```c
//! for (uint32_t i = 0; i < rtw_module_count(); i++)   // the catalogue
//!     menu_add(rtw_module_name(i), rtw_module_slug(i));
//!
//! RtwRuntime *rt = rtw_create("bungee-roulette", "/…/Resources/assets");
//! rtw_set_control(rt, 0, 3);                       // optional
//! for (;;) {
//!     rtw_tick(rt, now_ms, h, m, s, mx, my, down, caps);
//!     const uint32_t *px = rtw_pixels(rt);          // 640×480, 0RGB
//!     const char *snd;
//!     while ((snd = rtw_next_sound(rt))) play(snd); // absolute .wav paths
//! }
//! rtw_destroy(rt);
//! ```
//!
//! Two things the host does NOT get to decide:
//!
//! * **Sim time.** `now_ms` is the host's monotonic wall clock and is used
//!   only to decide *how many* ticks are due. The module's own clock is
//!   derived from the tick count on the module's own grid
//!   (`engine::TickClock` — 40 ms for most, After Dark's 16.625 ms Mac tick
//!   for the modules that gate on `Resource.f4724()`), the way the player
//!   shell and `bin/frame` do it — modules gate on `now < deadline` and a
//!   jittery clock makes those gates swallow ticks at random. A stall
//!   (display sleep, a slow first frame) is absorbed by dropping the
//!   backlog past [`MAX_CATCHUP_TICKS`], never by fast-forwarding minutes
//!   of animation into one frame. Pacing is in microseconds because the Mac
//!   tick is not a whole millisecond; `rtw_tick_us` reports it.
//! * **Composition.** The frame comes out of `app::compose`, the same pass
//!   the other two front ends draw through.
//!
//! Every entry point is panic-safe: a panic inside Rust would abort the
//! host process if it unwound across the ABI boundary (and the host here is
//! `legacyScreenSaver.appex`, where an abort is a crash report and a dead
//! screensaver), so each call is wrapped in `catch_unwind` and degrades to
//! a no-op / null / zero.

use app::{compose, ImageCache, SIM_H, SIM_PIXELS, SIM_W};
use engine::{ControlDef, ControlKind, Ctx, Module, Pack, Random15, RandomLong, TickClock};
use std::ffi::{c_char, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};

pub mod rip;
pub use rip::{
    rtw_rip_free, rtw_rip_message, rtw_rip_module_count, rtw_rip_poll, rtw_rip_start, RipJob,
    RTW_RIP_DONE, RTW_RIP_FAILED, RTW_RIP_RUNNING,
};

/// Same cap the player shell uses: a stall costs animation, not CPU.
const MAX_CATCHUP_TICKS: u32 = 4;

/// Ceiling on the decoded-sprite cache, in bytes of RGBA.
///
/// `app::compose` decodes a compound the first time it is drawn and keeps
/// it, keyed by `(png, clut)`. That is the right trade for the player and
/// for `bin/frame`, which live for seconds; a screensaver runs all night
/// inside `legacyScreenSaver.appex` and walks its whole pack, several times
/// over when the module recolours (chameleon's cluts multiply every
/// compound). Measured on this tree, 20 000 frames — 13 minutes of
/// chameleon — carried the process to **130 MB RSS** and still climbing,
/// and voyeur to 90 MB; bungee, whose art is small, settles under 8 MB.
///
/// So the cache is capped. Crossing the cap drops it whole and the next
/// compose re-decodes what is on screen *now* — the working set, not the
/// accumulated history. Measured at this cap over the same 20 000 frames:
/// chameleon peaks at 35 MB RSS instead of 130 and drops the cache about
/// every 700 ticks (~25 s), voyeur at 44 MB instead of 90; the refill frame
/// costs 2.3 ms against chameleon's 16.6 ms budget and 11.7 ms against
/// voyeur's 60 ms, so not even a dropped frame. An LRU would spread that
/// cost further, but `compose` owns the map and cannot report hits; a
/// bounded clear needs nothing from the shared draw path and leaves the
/// other two front ends byte-identical.
const IMAGE_CACHE_BYTES: usize = 24 * 1024 * 1024;

/// The cap, for a test that wants to assert against it rather than restate
/// it. Not part of the C ABI: the host has no say in it.
pub fn image_cache_cap() -> usize {
    IMAGE_CACHE_BYTES
}

pub struct Runtime {
    /// The module this runtime runs — kept because which `sUnt` resource
    /// labels which slider is per slug (`app::slider_bands`).
    slug: String,
    pack: Pack,
    pack_root: PathBuf,
    module: Box<dyn Module>,
    /// The module's control set, read once at construction. `ControlDef`s
    /// are static descriptions (name, kind, default) — only the *values*
    /// move, and those live inside the module — so the host can walk this
    /// list while the sim runs.
    controls: Vec<ControlDef>,
    /// The decoded-sprite cache — SHARED by every runtime of this pack in
    /// the process (see [`acquire_pack`]), unless a test gave it a private
    /// one with [`Runtime::set_cache_cap`].
    cache: Arc<Mutex<SpriteCache>>,
    pixels: Vec<u32>,
    /// Composed frame is up to date with the last tick.
    fresh: bool,
    rng: RandomLong,
    rng15: Random15,
    /// Ticks run; the module clock is `clock.now_ms(ticks)`.
    ticks: u64,
    /// The module's tick grid, read once at construction.
    clock: TickClock,
    /// Host-clock origin (µs) of tick 0; `None` until the first pump.
    epoch_us: Option<u64>,
    /// snd ids fired but not yet drained by the host, oldest first.
    sound_queue: Vec<u32>,
    /// Backing store for the pointer handed out by `rtw_next_sound`.
    sound_path: Option<CString>,
    /// Backing store for the pointer handed out by `rtw_loop_sound`.
    loop_path: Option<CString>,
    /// Backing store for the pointer handed out by the control-string calls
    /// (`rtw_control_name`, `rtw_control_item`, `rtw_control_band_label`).
    /// One slot: each call invalidates the last, which is all a host that
    /// copies the string straight into an `NSString` needs.
    str_scratch: Option<CString>,
    /// The last `(song, plays started)` the module reported through
    /// `Module::music`, tracked on every tick whether or not a host has
    /// opened the music channel — so a channel opened mid-run knows what is
    /// sounding, and a change of either number is a new play exactly as in
    /// the player shell's `pump_music`.
    music_state: Option<(u32, u32)>,
    /// The music channel, if the host opened one (`rtw_music_open`). Weak:
    /// the HOST's handle owns it, so closing the handle frees the decoded
    /// instrument bank even while this runtime lives on.
    music: Weak<Mutex<MusicChannel>>,
}

/// After Dark's MDRV music channel for one runtime: the engine's synth plus
/// the pack's decoded songs. Shared between the runtime (which starts and
/// stops songs as the module asks, on the sim thread) and the host's
/// [`MusicHandle`] (which pulls PCM, on the audio thread) — hence the mutex,
/// which is the same arrangement as the player shell's `MusicStream`.
pub struct MusicChannel {
    player: engine::music::Player,
    songs: std::collections::HashMap<u32, Arc<engine::music::Song>>,
}

impl MusicChannel {
    /// Start or stop per the module's `(song, play)` state. A song id the
    /// pack does not carry is silence, as in the shell.
    fn follow(&mut self, want: Option<(u32, u32)>) {
        match want.and_then(|(song, _)| self.songs.get(&song)) {
            Some(s) => self.player.play(s.clone()),
            None => self.player.stop(),
        }
    }
}

/// What `rtw_music_open` hands the host: an owning reference to the
/// channel. Rendering through it never touches the runtime, so the audio
/// thread and the sim thread only ever meet at the channel's mutex.
pub struct MusicHandle(Arc<Mutex<MusicChannel>>);

/// Kind codes on the wire — keep in step with `include/retwisted_saver.h`.
pub const RTW_CONTROL_SLIDER: i32 = 0;
pub const RTW_CONTROL_POPUP: i32 = 1;
pub const RTW_CONTROL_CHECKBOX: i32 = 2;

impl Runtime {
    /// `assets_dir` is either the pack directory itself or the root that
    /// holds `<slug>/meta.json`. The bundle ships the latter
    /// (`Contents/Resources/assets/bungee-roulette/`), a dev run usually
    /// points at the repo's `assets/`; accepting both removes a whole class
    /// of "the saver draws nothing" support question.
    pub fn new(slug: &str, assets_dir: &Path) -> Option<Runtime> {
        let root = pack_root(slug, assets_dir);
        let (pack, cache) = match acquire_pack(&root) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("retwisted: pack {} failed to load: {e}", root.display());
                return None;
            }
        };
        let module = app::modules::make(slug, pack.clone())?;
        let controls = module.controls();
        let clock = module.clock();
        Some(Runtime {
            slug: slug.to_string(),
            pack,
            pack_root: root,
            module,
            controls,
            cache,
            pixels: vec![0; SIM_PIXELS],
            fresh: false,
            // Fixed seeds would make every screensaver launch play out the
            // identical run; seed off the clock instead.
            rng: RandomLong::new(seed()),
            rng15: Random15::new(seed() as u32 | 1),
            ticks: 0,
            clock,
            epoch_us: None,
            sound_queue: Vec::new(),
            sound_path: None,
            loop_path: None,
            str_scratch: None,
            music_state: None,
            music: Weak::new(),
        })
    }

    pub fn controls(&self) -> &[ControlDef] {
        &self.controls
    }

    fn control(&self, index: i32) -> Option<&ControlDef> {
        usize::try_from(index).ok().and_then(|i| self.controls.get(i))
    }

    /// The legal raw-value range for control `index`, inclusive. Sliders
    /// carry their own; a checkbox is 0/1; a popup runs from the base its
    /// def declares across its items — bungee's Jumper is 1-based because
    /// the module says so, not because the wrapper keeps a table of
    /// exceptions (it used to; that was a bug waiting for the second
    /// 1-based popup).
    pub fn control_range(&self, index: i32) -> Option<(i32, i32)> {
        Some(self.control(index)?.range())
    }

    /// The `sUnt` words for control `index` — the ones After Dark's panel
    /// printed under the slider instead of numbers. Empty for every control
    /// that had none (popups, checkboxes), and for a pack ripped before the
    /// rippers carried `sUnt`. Read from the pack: see [`app::slider_bands`].
    pub fn bands(&self, index: i32) -> &[(i32, String)] {
        match usize::try_from(index) {
            Ok(i) => app::slider_bands(&self.pack, &self.slug, &self.controls, i),
            _ => &[],
        }
    }

    /// Hand out `s` as a C string with runtime lifetime. Invalidates the
    /// previous control string, which the contract allows.
    fn scratch(&mut self, s: &str) -> *const c_char {
        match CString::new(s) {
            Ok(c) => {
                self.str_scratch = Some(c);
                self.str_scratch.as_ref().unwrap().as_ptr()
            }
            Err(_) => {
                self.str_scratch = None;
                std::ptr::null()
            }
        }
    }

    /// Host timer interval in microseconds — the module's tick period.
    pub fn tick_us(&self) -> u64 {
        self.clock.period_us().max(1)
    }

    pub fn tick_ms(&self) -> u64 {
        (self.tick_us() / 1000).max(1)
    }

    /// One module tick. `hms` is the host's local wall clock (Shock Clocks
    /// tells real time), `mouse` is in sim coordinates or `(-1, -1)` when
    /// the pointer is outside the field.
    pub fn tick_once(&mut self, hms: (u8, u8, u8), mouse: (i32, i32), down: bool, caps: bool) {
        self.ticks += 1;
        let mut ctx = Ctx {
            rng: std::mem::replace(&mut self.rng, RandomLong::new(1)),
            rng15: std::mem::replace(&mut self.rng15, Random15::new(1)),
            sounds: Vec::new(),
            caps_lock: caps,
            now_ms: self.clock.now_ms(self.ticks),
            local_hms: hms,
            mouse,
            mouse_down: down,
        };
        self.module.tick(&mut ctx);
        self.rng = ctx.rng;
        self.rng15 = ctx.rng15;
        self.sound_queue.extend(ctx.sounds);
        self.pump_music();
        // Bound the queue: a host that never drains sounds must not grow
        // this without limit.
        if self.sound_queue.len() > 32 {
            let drop = self.sound_queue.len() - 32;
            self.sound_queue.drain(..drop);
        }
        self.fresh = false;
    }

    /// Run whatever ticks the host clock says are due; returns how many ran.
    pub fn pump(
        &mut self,
        now_ms: u64,
        hms: (u8, u8, u8),
        mouse: (i32, i32),
        down: bool,
        caps: bool,
    ) -> u32 {
        let now_us = now_ms.saturating_mul(1000);
        let epoch = *self.epoch_us.get_or_insert(now_us);
        let mut ran = 0;
        while now_us.saturating_sub(epoch) >= self.clock.at_us(self.ticks + 1)
            && ran < MAX_CATCHUP_TICKS
        {
            self.tick_once(hms, mouse, down, caps);
            ran += 1;
        }
        // Still behind after the burst: drop the backlog by moving the
        // epoch. The tick counter — and so the module's clock — never moves
        // backwards or jumps.
        if now_us.saturating_sub(epoch) >= self.clock.at_us(self.ticks + 1) {
            self.epoch_us = Some(now_us.saturating_sub(self.clock.at_us(self.ticks)));
        }
        ran
    }

    /// The composed frame, 640×480 0RGB. Composition is lazy so a host that
    /// draws less often than it ticks pays for one compose per draw.
    pub fn pixels(&mut self) -> &[u32] {
        if !self.fresh {
            // A poisoned lock means a compose panicked mid-insert; the map
            // itself is still a valid map of fully decoded images.
            let mut c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            compose(&self.pack, &mut c.map, self.module.as_ref(), &mut self.pixels);
            app::present_gamma(&mut self.pixels);
            c.trim(&self.slug, self.ticks);
            self.fresh = true;
        }
        &self.pixels
    }

    /// Decoded bytes held in the sprite cache (shared: every runtime of this
    /// pack in the process sees the same number).
    pub fn cache_bytes(&self) -> usize {
        self.cache.lock().map(|c| c.bytes()).unwrap_or(0)
    }

    pub fn cache_len(&self) -> usize {
        self.cache.lock().map(|c| c.map.len()).unwrap_or(0)
    }

    /// Lower the cap so a test can cross it in a few hundred frames instead
    /// of the ten-odd minutes of real animation [`IMAGE_CACHE_BYTES`] takes.
    /// Rust-side only — deliberately NOT a `rtw_*` entry point, because the
    /// host has no business sizing this. It gives this runtime a PRIVATE,
    /// empty cache: a test must not shrink the cache other runtimes of the
    /// same pack are sharing, nor measure their decodes as its own.
    pub fn set_cache_cap(&mut self, bytes: usize) {
        self.cache = Arc::new(Mutex::new(SpriteCache::new(bytes)));
        self.fresh = false;
    }

    /// True when this runtime shares its pack (and sprite cache) with
    /// `other` — both were created from the same pack root while the other
    /// was alive.
    pub fn shares_pack_with(&self, other: &Runtime) -> bool {
        Arc::ptr_eq(&self.pack.meta, &other.pack.meta) && Arc::ptr_eq(&self.cache, &other.cache)
    }

    pub fn field(&self) -> [u8; 3] {
        self.module.field()
    }

    /// The module's music state as of the last tick — `Module::music`.
    pub fn music_state(&self) -> Option<(u32, u32)> {
        self.music_state
    }

    /// Follow the module's music state, exactly as `App::pump_music` does in
    /// the player shell: only a CHANGE of `(song, plays)` touches the synth —
    /// a new play restarts the tune from the top, `None` silences it. Every
    /// gate (mime hunt's Music bands, coming soon's replay count, boris's
    /// Mower Sound interplay) is the module's own; this only obeys it.
    fn pump_music(&mut self) {
        let want = self.module.music();
        if want == self.music_state {
            return;
        }
        self.music_state = want;
        if let Some(ch) = self.music.upgrade() {
            if let Ok(mut ch) = ch.lock() {
                ch.follow(want);
            }
        }
    }

    /// Open the music channel: decode the pack's songs and instrument bank
    /// (a few hundred KB to ~1 MB of samples — which is why it is only done
    /// when a host is actually going to play it) and start whatever the
    /// module currently wants playing, FROM THE TOP. `None` for a module
    /// whose pack has no music (nine of the thirteen).
    ///
    /// Opening a second time while a handle is alive returns the same
    /// channel, so there is never more than one synth per runtime.
    pub fn open_music(&mut self) -> Option<MusicHandle> {
        if let Some(ch) = self.music.upgrade() {
            return Some(MusicHandle(ch));
        }
        let assets = self.pack.music()?;
        let player = engine::music::Player::new(assets.bank, engine::music::OUT_RATE);
        let mut ch = MusicChannel { player, songs: assets.songs };
        ch.follow(self.music_state);
        let ch = Arc::new(Mutex::new(ch));
        self.music = Arc::downgrade(&ch);
        Some(MusicHandle(ch))
    }

    /// Oldest undrained sound as an absolute `.wav` path, if the pack has
    /// the file. Ids with no packed sound are skipped, not reported.
    pub fn next_sound(&mut self) -> Option<PathBuf> {
        while !self.sound_queue.is_empty() {
            let id = self.sound_queue.remove(0);
            let p = self.pack_root.join("sounds").join(format!("{id}.wav"));
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}

/// The decoded-sprite cache of one pack, capped at [`IMAGE_CACHE_BYTES`].
pub struct SpriteCache {
    map: ImageCache,
    /// `map.len()` the last time the cache was weighed — the weighing walks
    /// every entry, and the cache only grows on a frame that decoded
    /// something new.
    weighed: usize,
    cap: usize,
}

impl SpriteCache {
    fn new(cap: usize) -> SpriteCache {
        SpriteCache { map: ImageCache::new(), weighed: 0, cap }
    }

    fn bytes(&self) -> usize {
        self.map.iter().map(|(k, v)| k.0.len() + v.rgba.len()).sum()
    }

    /// Drop the whole cache once it passes its cap. Only weighed after a
    /// compose that decoded something new — it cannot grow otherwise.
    fn trim(&mut self, slug: &str, ticks: u64) {
        if self.map.len() == self.weighed {
            return;
        }
        self.weighed = self.map.len();
        let bytes = self.bytes();
        if bytes > self.cap {
            eprintln!(
                "retwisted: {slug} sprite cache {} KB over the {} KB cap after {ticks} ticks — dropping it",
                bytes >> 10,
                self.cap >> 10,
            );
            self.map.clear();
            self.weighed = 0;
        }
    }
}

/// One loaded pack, shared by every live runtime built from it.
struct SharedPack {
    pack: Pack,
    sprites: Arc<Mutex<SpriteCache>>,
}

/// Packs loaded in this process, by pack root.
///
/// The host (`legacyScreenSaver.appex`) runs three or four views of this
/// saver in one process — the pane preview, the sheet thumbnail, Preview, the
/// real run — usually on the SAME module. Each used to parse its own copy of
/// `meta.json` (up to 3.2 MB of JSON: mime hunt, voyeur) and decode its own
/// copy of every sprite it drew (up to [`IMAGE_CACHE_BYTES`] each). Now the
/// first runtime of a pack loads it and the rest share the parse and the
/// decoded sprites. Entries live exactly as long as some runtime uses them:
/// [`rtw_destroy`] sweeps the ones nothing references any more, so a pack is
/// loaded only while a module on it is instantiated.
fn pack_registry() -> &'static Mutex<std::collections::HashMap<PathBuf, SharedPack>> {
    static REG: OnceLock<Mutex<std::collections::HashMap<PathBuf, SharedPack>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// The pack at `root` and its shared sprite cache — loaded now if no live
/// runtime has it, shared if one does.
fn acquire_pack(root: &Path) -> std::io::Result<(Pack, Arc<Mutex<SpriteCache>>)> {
    let key = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut reg = pack_registry().lock().unwrap_or_else(|e| e.into_inner());
    sweep(&mut reg);
    if let Some(s) = reg.get(&key) {
        return Ok((s.pack.clone(), s.sprites.clone()));
    }
    let pack = Pack::load(root)?;
    let sprites = Arc::new(Mutex::new(SpriteCache::new(IMAGE_CACHE_BYTES)));
    reg.insert(key, SharedPack { pack: pack.clone(), sprites: sprites.clone() });
    Ok((pack, sprites))
}

/// Forget every pack only the registry still holds. `Pack` clones share one
/// `Arc<Meta>`, so a count of 1 means no runtime and no module has it.
fn sweep(reg: &mut std::collections::HashMap<PathBuf, SharedPack>) {
    reg.retain(|_, s| Arc::strong_count(&s.pack.meta) > 1 || Arc::strong_count(&s.sprites) > 1);
}

/// Is the pack at `root` (a pack directory) loaded in this process right
/// now — i.e. does some live runtime use it? For tests and logs.
pub fn pack_is_loaded(root: &Path) -> bool {
    let key = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut reg = pack_registry().lock().unwrap_or_else(|e| e.into_inner());
    sweep(&mut reg);
    reg.contains_key(&key)
}

impl Drop for Runtime {
    /// A host handle may outlive the runtime by a render callback or two
    /// (the audio thread is not the sim thread). Stop the synth so what it
    /// renders from here on is silence, not the tail of a module that no
    /// longer exists.
    fn drop(&mut self) {
        if let Some(ch) = self.music.upgrade() {
            if let Ok(mut ch) = ch.lock() {
                ch.player.stop();
            }
        }
    }
}

impl MusicHandle {
    /// Render `out.len()` mono frames at [`engine::music::OUT_RATE`].
    /// Silence when nothing is playing or the lock is poisoned — the audio
    /// thread must never be the thing that dies.
    pub fn render(&self, out: &mut [f32]) {
        match self.0.lock() {
            Ok(mut ch) => ch.player.render(out),
            Err(_) => out.fill(0.0),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.0.lock().map(|ch| ch.player.is_playing()).unwrap_or(false)
    }

    pub fn set_volume(&self, v: f32) {
        if let Ok(mut ch) = self.0.lock() {
            ch.player.set_volume(v);
        }
    }
}

/// Where `slug`'s pack lives under `assets_dir`: the directory itself when
/// it *is* a pack (`meta.json` in it), otherwise `<assets_dir>/<slug>`. Both
/// spellings are accepted — the bundle ships a root of thirteen, a dev run
/// usually points straight at one — and this is the single place that rule
/// is written, so [`rtw_pack_exists`] cannot answer for a different path
/// than [`rtw_create`] would load.
fn pack_root(slug: &str, assets_dir: &Path) -> PathBuf {
    if assets_dir.join("meta.json").exists() {
        assets_dir.to_path_buf()
    } else {
        assets_dir.join(slug)
    }
}

fn seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5EED_CAFE)
        | 1
}

// ---------------------------------------------------------------------------
// C ABI
//
// Pointer rules: `rtw_create` hands out an owning pointer the host must give
// back to `rtw_destroy` exactly once. Every other call takes it borrowed and
// tolerates NULL. Returned pointers (`rtw_pixels`, `rtw_next_sound`) stay
// valid until the next call that could invalidate them — the next
// tick/pixels call and the next `rtw_next_sound` call respectively.

/// Guard a C entry point: never let a panic unwind into the host.
fn guarded<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("retwisted: panic caught at the C ABI boundary");
            fallback
        }
    }
}

unsafe fn cstr<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// Width of the frame buffer, in pixels (always 640).
#[no_mangle]
pub extern "C" fn rtw_width() -> u32 {
    SIM_W as u32
}

/// Height of the frame buffer, in pixels (always 480).
#[no_mangle]
pub extern "C" fn rtw_height() -> u32 {
    SIM_H as u32
}

// --- module enumeration ---------------------------------------------------
//
// The catalogue, so a host can offer a *choice* of module without knowing
// any of them: count, then a slug (the `rtw_create` argument and the
// settings-key namespace) and a display name (the After Dark title) per
// index. It is deliberately runtime-free — the settings sheet has to fill
// its Module popup before any pack is loaded, and building thirteen `Pack`s
// to read thirteen strings would be a visible stall on opening Options…
//
// Unlike every other string in this ABI these pointers are STATIC: they are
// built once into a process-lifetime table and no later call invalidates
// them. A host may hold them.

/// slug/name pairs as C strings, allocated once and never freed.
fn module_table() -> &'static [(CString, CString)] {
    static TABLE: OnceLock<Vec<(CString, CString)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        app::MODULE_TITLES
            .iter()
            .filter_map(|(slug, name)| {
                Some((CString::new(*slug).ok()?, CString::new(*name).ok()?))
            })
            .collect()
    })
}

/// How many modules this build can run. Indices `0..count` are valid for
/// [`rtw_module_slug`] / [`rtw_module_name`], in control-panel order.
#[no_mangle]
pub extern "C" fn rtw_module_count() -> u32 {
    guarded(0, || module_table().len() as u32)
}

/// Module `index`'s slug — the string to hand [`rtw_create`], and the one
/// the host should namespace its stored settings with. NULL past the end.
/// The pointer is static: it outlives every runtime and is never
/// invalidated.
#[no_mangle]
pub extern "C" fn rtw_module_slug(index: u32) -> *const c_char {
    guarded(std::ptr::null(), || match module_table().get(index as usize) {
        Some((slug, _)) => slug.as_ptr(),
        None => std::ptr::null(),
    })
}

/// Module `index`'s display name — the original After Dark title ("Mowin'
/// Boris"), not the slug. NULL past the end; static, like the slug.
#[no_mangle]
pub extern "C" fn rtw_module_name(index: u32) -> *const c_char {
    guarded(std::ptr::null(), || match module_table().get(index as usize) {
        Some((_, name)) => name.as_ptr(),
        None => std::ptr::null(),
    })
}

/// Index of `slug` in the catalogue, or -1 if this build has no such
/// module — what a host calls to validate a slug it read back out of its
/// own settings before trusting it to [`rtw_create`].
///
/// # Safety
/// `slug` must be a valid NUL-terminated C string, or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_module_index(slug: *const c_char) -> i32 {
    guarded(-1, || {
        let Some(slug) = cstr(slug) else { return -1 };
        module_table()
            .iter()
            .position(|(s, _)| s.to_str() == Ok(slug))
            .map(|i| i as i32)
            .unwrap_or(-1)
    })
}

/// Does `assets_dir` actually carry `slug`'s art?
///
/// The catalogue above is compiled in; the packs are copied into the bundle
/// at build time and a module whose pack was not on the build machine is in
/// the one and not the other. Before this existed the only way to find out
/// was to [`rtw_create`] it — load a pack to learn that there is no pack —
/// and the Module popup happily offered a module that came up as a black
/// screen and one line in the log.
///
/// This is deliberately NOT part of the catalogue: `rtw_module_*` must stay
/// pack-free and pathless, because the popup is filled before the host has
/// decided anything. This is the separate, cheap question ("is there a
/// `meta.json`?") asked once per module with the assets root in hand, and it
/// resolves the path exactly the way `rtw_create` will.
///
/// # Safety
/// `slug` and `assets_dir` must be valid NUL-terminated UTF-8 C strings, or
/// NULL (for which the answer is false).
#[no_mangle]
pub unsafe extern "C" fn rtw_pack_exists(
    slug: *const c_char,
    assets_dir: *const c_char,
) -> bool {
    guarded(false, || {
        let (Some(slug), Some(dir)) = (cstr(slug), cstr(assets_dir)) else {
            return false;
        };
        pack_root(slug, Path::new(dir)).join("meta.json").is_file()
    })
}

/// Create a runtime for `slug`, loading its pack from `assets_dir` (either
/// the pack dir or the root containing `<slug>/`). NULL on any failure.
///
/// # Safety
/// `slug` and `assets_dir` must be valid NUL-terminated UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn rtw_create(
    slug: *const c_char,
    assets_dir: *const c_char,
) -> *mut Runtime {
    guarded(std::ptr::null_mut(), || {
        let (Some(slug), Some(dir)) = (cstr(slug), cstr(assets_dir)) else {
            return std::ptr::null_mut();
        };
        match Runtime::new(slug, Path::new(dir)) {
            Some(rt) => Box::into_raw(Box::new(rt)),
            None => {
                eprintln!("retwisted: no module for slug {slug:?} under {dir:?}");
                std::ptr::null_mut()
            }
        }
    })
}

/// Destroy a runtime. NULL-safe; must be called at most once per pointer.
///
/// # Safety
/// `rt` must come from [`rtw_create`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn rtw_destroy(rt: *mut Runtime) {
    if rt.is_null() {
        return;
    }
    guarded((), || {
        drop(Box::from_raw(rt));
        // The last runtime of a pack takes the pack (and its decoded
        // sprites) with it.
        let mut reg = pack_registry().lock().unwrap_or_else(|e| e.into_inner());
        sweep(&mut reg);
    });
}

/// Set control `index` to `value` (module-defined raw values — the same
/// numbers `ControlDef::default` carries).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_set_control(rt: *mut Runtime, index: i32, value: i32) {
    let Some(rt) = rt.as_mut() else { return };
    guarded((), || {
        if index >= 0 {
            rt.module.set_control(index as usize, value);
            rt.fresh = false;
        }
    })
}

// --- control introspection ------------------------------------------------
//
// Enough for a host to build a settings UI with no knowledge of the module:
// how many controls, and per control a name, a kind, the legal raw range,
// the factory default, and (for popups) the item labels. Values only ever
// travel back through `rtw_set_control`, so the raw numbers the module
// speaks stay the only currency.

/// How many controls this module has. 0 for a module with none (and a host
/// should then not offer a configure sheet at all).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_count(rt: *const Runtime) -> u32 {
    match rt.as_ref() {
        Some(rt) => guarded(0, || rt.controls().len() as u32),
        None => 0,
    }
}

/// Control `index`'s kind: 0 slider, 1 popup, 2 checkbox; -1 for a bad
/// index or a NULL runtime.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_kind(rt: *const Runtime, index: i32) -> i32 {
    let Some(rt) = rt.as_ref() else { return -1 };
    guarded(-1, || match rt.control(index).map(|c| &c.kind) {
        Some(ControlKind::Slider { .. }) => RTW_CONTROL_SLIDER,
        Some(ControlKind::Popup { .. }) => RTW_CONTROL_POPUP,
        Some(ControlKind::Checkbox) => RTW_CONTROL_CHECKBOX,
        None => -1,
    })
}

/// Lowest legal raw value for control `index` (0 for a bad index).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_min(rt: *const Runtime, index: i32) -> i32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || rt.control_range(index).map(|r| r.0).unwrap_or(0))
}

/// Highest legal raw value for control `index` (0 for a bad index).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_max(rt: *const Runtime, index: i32) -> i32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || rt.control_range(index).map(|r| r.1).unwrap_or(0))
}

/// The module's factory default raw value for control `index` — what the
/// host should use when it has nothing stored (0 for a bad index).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_default(rt: *const Runtime, index: i32) -> i32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || rt.control(index).map(|c| c.default).unwrap_or(0))
}

/// How many items popup control `index` offers; 0 for any other kind.
/// Item `n` is the raw value `rtw_control_min(rt, index) + n`.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_item_count(rt: *const Runtime, index: i32) -> u32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || match rt.control(index).map(|c| &c.kind) {
        Some(ControlKind::Popup { items, .. }) => items.len() as u32,
        _ => 0,
    })
}

/// Control `index`'s display name, UTF-8. NULL for a bad index. The string
/// belongs to the runtime and is valid only until the next
/// `rtw_control_name` / `rtw_control_item` call — copy it.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_name(rt: *mut Runtime, index: i32) -> *const c_char {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || {
        let Some(name) = rt.control(index).map(|c| c.name.clone()) else {
            return std::ptr::null();
        };
        rt.scratch(&name)
    })
}

/// The label of popup `index`'s item `item`, UTF-8; NULL if either index is
/// out of range or the control is not a popup. Same one-slot lifetime as
/// [`rtw_control_name`].
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_item(
    rt: *mut Runtime,
    index: i32,
    item: i32,
) -> *const c_char {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || {
        let label = match rt.control(index).map(|c| &c.kind) {
            Some(ControlKind::Popup { items, .. }) => usize::try_from(item)
                .ok()
                .and_then(|i| items.get(i))
                .cloned(),
            _ => None,
        };
        match label {
            Some(l) => rt.scratch(&l),
            None => std::ptr::null(),
        }
    })
}

// --- slider end labels (After Dark's `sUnt` words) ------------------------
//
// AD's control panels never printed a number on a slider: each `sVal` had an
// `sUnt` sibling holding (position, word) rows, and the panel spelled them
// out under the track — "One" at one end, "Hundreds!" at the other. A host
// that can only read min and max renders "0" and "100", which is the one
// thing the original never showed.
//
// Three calls, shaped like the popup-item ones: how many words, the raw
// value each sits at, and the word. A fourth, `rtw_control_band_for`, keeps
// the "which word is this value in" rule (floor, with everything below the
// first tick in the first band) on this side of the ABI so two hosts cannot
// round it differently.

/// How many `sUnt` words control `index` has; 0 for a control the original
/// labelled with nothing (every popup and checkbox).
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_band_count(rt: *const Runtime, index: i32) -> u32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || rt.bands(index).len() as u32)
}

/// The raw slider value word `band` sits at — its tick position. 0 for a bad
/// index. Words are in ascending order, so band 0 is the low end and
/// `rtw_control_band_count() - 1` the high end.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_band_value(
    rt: *const Runtime,
    index: i32,
    band: i32,
) -> i32 {
    let Some(rt) = rt.as_ref() else { return 0 };
    guarded(0, || {
        usize::try_from(band)
            .ok()
            .and_then(|b| rt.bands(index).get(b))
            .map(|(at, _)| *at)
            .unwrap_or(0)
    })
}

/// Word `band` of control `index`, UTF-8; NULL for a bad index or a control
/// with no words. Same one-slot lifetime as [`rtw_control_name`] — copy it
/// before the next control-string call.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_band_label(
    rt: *mut Runtime,
    index: i32,
    band: i32,
) -> *const c_char {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || {
        let label = usize::try_from(band)
            .ok()
            .and_then(|b| rt.bands(index).get(b))
            .map(|(_, w)| w.clone());
        match label {
            Some(l) => rt.scratch(&l),
            None => std::ptr::null(),
        }
    })
}

/// Which word raw value `value` falls under: the last tick at or below it,
/// and word 0 for anything below the first tick (several tables start at 10
/// or 20 while the slider starts at 0, and the module's own arithmetic puts
/// those values in the first band too). -1 when the control has no words.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_control_band_for(
    rt: *const Runtime,
    index: i32,
    value: i32,
) -> i32 {
    let Some(rt) = rt.as_ref() else { return -1 };
    guarded(-1, || {
        app::slider_band_index(rt.bands(index), value).map(|n| n as i32).unwrap_or(-1)
    })
}

/// The module's tick period in milliseconds, rounded DOWN. 40 (25 Hz) for
/// most modules; 16 for a module on After Dark's 16.625 ms Mac tick, where
/// it is a millisecond short — prefer [`rtw_tick_us`] for the host timer.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_tick_ms(rt: *const Runtime) -> u32 {
    match rt.as_ref() {
        Some(rt) => guarded(40, || rt.tick_ms() as u32),
        None => 40,
    }
}

/// The module's tick period in MICROseconds — what the host should use for
/// `animationTimeInterval`. 40 000 for most modules, 16 625 for the modules
/// that ride After Dark's Mac tick (`TickCount()*16.625`), which is not a
/// whole number of milliseconds: rounding it to 17 runs those modules 2.3 %
/// slow and to 16 runs them 3.8 % fast.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_tick_us(rt: *const Runtime) -> u32 {
    match rt.as_ref() {
        Some(rt) => guarded(40_000, || rt.tick_us() as u32),
        None => 40_000,
    }
}

/// Advance the sim to `now_ms` (host monotonic clock, milliseconds).
/// Returns the number of module ticks that ran, 0..=4.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn rtw_tick(
    rt: *mut Runtime,
    now_ms: u64,
    hour: u8,
    minute: u8,
    second: u8,
    mouse_x: i32,
    mouse_y: i32,
    mouse_down: bool,
    caps_lock: bool,
) -> u32 {
    let Some(rt) = rt.as_mut() else { return 0 };
    guarded(0, || {
        rt.pump(
            now_ms,
            (hour, minute, second),
            (mouse_x, mouse_y),
            mouse_down,
            caps_lock,
        )
    })
}

/// Pointer to the composed frame: `rtw_width() * rtw_height()` `uint32_t`
/// in 0RGB (0x00RRGGBB). Valid until the next [`rtw_tick`] /
/// [`rtw_set_control`] / [`rtw_destroy`]. NULL only if `rt` is NULL.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_pixels(rt: *mut Runtime) -> *const u32 {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || rt.pixels().as_ptr())
}

/// The module's field (background) colour, written to `rgb[0..3]`. The host
/// letterboxes with it so the bars match the field instead of sitting as a
/// black frame around a grey module.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL; `rgb` must
/// point to at least 3 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn rtw_field(rt: *const Runtime, rgb: *mut u8) {
    if rgb.is_null() {
        return;
    }
    let c = rt.as_ref().map(|r| guarded([0, 0, 0], || r.field())).unwrap_or([0, 0, 0]);
    std::ptr::copy_nonoverlapping(c.as_ptr(), rgb, 3);
}

/// Drain one fired sound: an absolute path to a `.wav`, or NULL when the
/// queue is empty. The string is owned by the runtime and is valid until
/// the next `rtw_next_sound` call.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_next_sound(rt: *mut Runtime) -> *const c_char {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || match rt.next_sound() {
        Some(p) => {
            let Ok(c) = CString::new(p.to_string_lossy().into_owned()) else {
                return std::ptr::null();
            };
            rt.sound_path = Some(c);
            rt.sound_path.as_ref().unwrap().as_ptr()
        }
        None => {
            rt.sound_path = None;
            std::ptr::null()
        }
    })
}

/// The currently active looping sound as an absolute .wav path, or NULL when
/// no loop is sounding. The string is owned by the runtime and is valid until
/// the next `rtw_loop_sound` call.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_loop_sound(rt: *mut Runtime) -> *const c_char {
    let Some(rt) = rt.as_mut() else { return std::ptr::null() };
    guarded(std::ptr::null(), || match rt.module.loop_sound() {
        Some(id) => {
            let p = rt.pack_root.join("sounds").join(format!("{id}.wav"));
            if p.exists() {
                let Ok(c) = CString::new(p.to_string_lossy().into_owned()) else {
                    return std::ptr::null();
                };
                rt.loop_path = Some(c);
                rt.loop_path.as_ref().unwrap().as_ptr()
            } else {
                rt.loop_path = None;
                std::ptr::null()
            }
        }
        None => {
            rt.loop_path = None;
            std::ptr::null()
        }
    })
}

// --- the Randomizer -------------------------------------------------------
//
// After Dark 3.0's module list carried a "Randomizer" entry above the modules
// (STR# 143 item 1; its panel title is PICT 139). Its own panel had New… /
// Edit… / Delete… buttons for named Randomizer settings (a list of modules,
// each with a Duration, in Random or In Order — DLOG/DITL 129, MENU 500,
// sUnt 500) and ONE control on the panel itself: the **Default Duration**
// slider, `sVal 503` = 60, labelled by `sUnt 503` and turned into seconds by
// `rsVl 503`. This build has no named settings — the Randomizer here is "every
// module this bundle has art for, in random order" — so Default Duration is
// the whole of its panel, and these are its three resources, transcribed.
//
// `rsVl 503` is (slider value, seconds) with -1 = Forever. How AD read a
// value BETWEEN two rows is not settled from the resources alone; this uses
// the same floor rule the panel's words use (`app::slider_band_index`: the
// last row at or below, the first row below that), so the word under the knob
// and the time the module runs for can never disagree. The factory 60 is
// therefore "30 min." / 1800 s.

/// `sUnt 503` — the words under the Default Duration slider, verbatim
/// (including the trailing space on "15 sec. ").
pub const RANDOMIZER_WORDS: &[(i32, &str)] = &[
    (0, "Short"),
    (1, "15 sec. "),
    (16, "30 sec."),
    (24, "1 min."),
    (32, "2 min."),
    (40, "5 min."),
    (48, "10 min."),
    (56, "30 min."),
    (62, "45 min."),
    (70, "1 hour"),
    (78, "1 h. 30 m."),
    (86, "2 h."),
    (92, "6 h."),
    (100, "Forever"),
];

/// `rsVl 503` — slider value -> seconds, -1 = Forever.
pub const RANDOMIZER_SECONDS: &[(i32, i32)] = &[
    (1, 15),
    (16, 30),
    (24, 60),
    (32, 120),
    (40, 300),
    (48, 600),
    (56, 1800),
    (62, 2700),
    (70, 3600),
    (78, 5400),
    (86, 7200),
    (92, 21600),
    (100, -1),
];

/// `sVal 503` — the factory Default Duration.
pub const RANDOMIZER_DEFAULT: i32 = 60;

/// Seconds each module runs for at Default Duration `value`; -1 = Forever.
pub fn randomizer_seconds(value: i32) -> i32 {
    let i = RANDOMIZER_SECONDS.iter().rposition(|(at, _)| value >= *at).unwrap_or(0);
    RANDOMIZER_SECONDS[i].1
}

/// Static C strings for the Randomizer's words, built once per process.
fn randomizer_words_c() -> &'static [CString] {
    static WORDS: OnceLock<Vec<CString>> = OnceLock::new();
    WORDS.get_or_init(|| {
        RANDOMIZER_WORDS.iter().map(|(_, w)| CString::new(*w).unwrap_or_default()).collect()
    })
}

/// The Randomizer's factory Default Duration (raw slider value, `sVal 503`).
#[no_mangle]
pub extern "C" fn rtw_randomizer_default() -> i32 {
    RANDOMIZER_DEFAULT
}

/// How many `sUnt 503` words the Default Duration slider has. The slider's
/// range is word 0's value .. the last word's value (0..100).
#[no_mangle]
pub extern "C" fn rtw_randomizer_band_count() -> u32 {
    RANDOMIZER_WORDS.len() as u32
}

/// Raw slider value word `band` sits at; 0 for a bad index.
#[no_mangle]
pub extern "C" fn rtw_randomizer_band_value(band: i32) -> i32 {
    usize::try_from(band).ok().and_then(|b| RANDOMIZER_WORDS.get(b)).map(|w| w.0).unwrap_or(0)
}

/// Word `band`, UTF-8; NULL for a bad index. STATIC, like the catalogue.
#[no_mangle]
pub extern "C" fn rtw_randomizer_band_label(band: i32) -> *const c_char {
    guarded(std::ptr::null(), || {
        usize::try_from(band)
            .ok()
            .and_then(|b| randomizer_words_c().get(b))
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

/// Which word raw `value` falls under (the floor rule of
/// [`rtw_control_band_for`]).
#[no_mangle]
pub extern "C" fn rtw_randomizer_band_for(value: i32) -> i32 {
    app::slider_band_index(RANDOMIZER_WORDS, value).map(|n| n as i32).unwrap_or(-1)
}

/// Seconds each module runs for at Default Duration `value`; -1 = Forever
/// (the Randomizer then only moves on at the start of the next session).
#[no_mangle]
pub extern "C" fn rtw_randomizer_seconds(value: i32) -> i32 {
    randomizer_seconds(value)
}

// --- the music channel ------------------------------------------------------
//
// After Dark's MDRV synth is its own SndChannel: it neither pre-empts the sfx
// channel nor is pre-empted by it (engine::music's module docs carry the
// capture evidence). So it is its own handle here, pulled as PCM by the host
// on its audio thread, while the runtime keeps starting and stopping songs
// from the sim thread as the module's `(song, plays)` state changes.

/// Open `rt`'s music channel, or NULL when the module has no music in its
/// pack. Decodes the instrument bank, so call it only when the host is really
/// going to play — the saver does it only for the real-run view. What the
/// module is playing right now starts from the top. The handle is owned by
/// the host: give it back to [`rtw_music_close`] exactly once. It may outlive
/// `rt`; after `rtw_destroy` it renders silence.
///
/// # Safety
/// `rt` must be a live pointer from [`rtw_create`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_music_open(rt: *mut Runtime) -> *mut MusicHandle {
    let Some(rt) = rt.as_mut() else { return std::ptr::null_mut() };
    guarded(std::ptr::null_mut(), || match rt.open_music() {
        Some(h) => Box::into_raw(Box::new(h)),
        None => std::ptr::null_mut(),
    })
}

/// Output rate of the music channel, Hz (22254 — the `csnd` samples' own).
#[no_mangle]
pub extern "C" fn rtw_music_rate() -> u32 {
    engine::music::OUT_RATE
}

/// Render `frames` mono float samples into `out`, overwriting it. Safe to
/// call from the host's audio thread concurrently with `rtw_tick` on the
/// main thread. Writes silence for a NULL handle.
///
/// # Safety
/// `m` must be a live handle from [`rtw_music_open`], or NULL; `out` must
/// point to `frames` writable floats (or be NULL, when nothing is written).
#[no_mangle]
pub unsafe extern "C" fn rtw_music_render(m: *const MusicHandle, out: *mut f32, frames: u32) {
    if out.is_null() || frames == 0 {
        return;
    }
    let buf = std::slice::from_raw_parts_mut(out, frames as usize);
    match m.as_ref() {
        Some(m) => guarded((), || m.render(buf)),
        None => buf.fill(0.0),
    }
}

/// Channel volume, 0..1 (the synth's own gain; the shells use 0.4).
///
/// # Safety
/// `m` must be a live handle from [`rtw_music_open`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_music_set_volume(m: *const MusicHandle, volume: f32) {
    if let Some(m) = m.as_ref() {
        guarded((), || m.set_volume(volume));
    }
}

/// True while a song is sounding (events left, voices ringing, or inside the
/// tune's own length). For logs and tests; the host does not need it to play.
///
/// # Safety
/// `m` must be a live handle from [`rtw_music_open`], or NULL.
#[no_mangle]
pub unsafe extern "C" fn rtw_music_playing(m: *const MusicHandle) -> bool {
    m.as_ref().map(|m| guarded(false, || m.is_playing())).unwrap_or(false)
}

/// Release a handle from [`rtw_music_open`]. NULL-safe. Stop the host's
/// audio pull BEFORE calling this — the pointer is dead afterwards.
///
/// # Safety
/// `m` must come from [`rtw_music_open`] and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn rtw_music_close(m: *mut MusicHandle) {
    if m.is_null() {
        return;
    }
    guarded((), || drop(Box::from_raw(m)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_are_the_sim_field() {
        assert_eq!(rtw_width(), 640);
        assert_eq!(rtw_height(), 480);
    }

    #[test]
    fn null_pointers_are_inert() {
        unsafe {
            assert_eq!(rtw_tick(std::ptr::null_mut(), 0, 0, 0, 0, 0, 0, false, false), 0);
            assert!(rtw_pixels(std::ptr::null_mut()).is_null());
            assert!(rtw_next_sound(std::ptr::null_mut()).is_null());
            assert!(rtw_loop_sound(std::ptr::null_mut()).is_null());
            assert_eq!(rtw_tick_ms(std::ptr::null()), 40);
            assert_eq!(rtw_tick_us(std::ptr::null()), 40_000);
            rtw_set_control(std::ptr::null_mut(), 0, 0);
            assert_eq!(rtw_control_count(std::ptr::null()), 0);
            assert_eq!(rtw_control_kind(std::ptr::null(), 0), -1);
            assert_eq!(rtw_control_min(std::ptr::null(), 0), 0);
            assert_eq!(rtw_control_max(std::ptr::null(), 0), 0);
            assert_eq!(rtw_control_default(std::ptr::null(), 0), 0);
            assert_eq!(rtw_control_item_count(std::ptr::null(), 0), 0);
            assert!(rtw_control_name(std::ptr::null_mut(), 0).is_null());
            assert!(rtw_control_item(std::ptr::null_mut(), 0, 0).is_null());
            assert_eq!(rtw_control_band_count(std::ptr::null(), 0), 0);
            assert_eq!(rtw_control_band_value(std::ptr::null(), 0, 0), 0);
            assert!(rtw_control_band_label(std::ptr::null_mut(), 0, 0).is_null());
            assert_eq!(rtw_control_band_for(std::ptr::null(), 0, 50), -1);
            assert!(!rtw_pack_exists(std::ptr::null(), std::ptr::null()));
            rtw_destroy(std::ptr::null_mut());
            let mut rgb = [9u8; 3];
            rtw_field(std::ptr::null(), rgb.as_mut_ptr());
            assert_eq!(rgb, [0, 0, 0]);
            assert_eq!(rtw_module_index(std::ptr::null()), -1);
            assert!(rtw_music_open(std::ptr::null_mut()).is_null());
            assert!(!rtw_music_playing(std::ptr::null()));
            rtw_music_set_volume(std::ptr::null(), 0.5);
            let mut buf = [1.0f32; 8];
            rtw_music_render(std::ptr::null(), buf.as_mut_ptr(), 8);
            assert_eq!(buf, [0.0; 8]);
            rtw_music_render(std::ptr::null(), std::ptr::null_mut(), 8);
            rtw_music_close(std::ptr::null_mut());
        }
    }

    /// The catalogue answers with no runtime and no packs at all — that is
    /// the whole point of it (the Module popup is filled before anything is
    /// loaded), so it is worth asserting it needs neither.
    #[test]
    fn module_catalogue_is_runtime_free_and_static() {
        let n = rtw_module_count();
        assert_eq!(n as usize, app::modules::SLUGS.len(), "catalogue vs registry");
        for i in 0..n {
            let slug = rtw_module_slug(i);
            let name = rtw_module_name(i);
            assert!(!slug.is_null() && !name.is_null(), "module {i}");
            unsafe {
                let slug = CStr::from_ptr(slug).to_str().unwrap();
                assert!(!CStr::from_ptr(name).to_bytes().is_empty(), "{slug} has no title");
                assert_eq!(slug, app::modules::SLUGS[i as usize], "order");
                assert_eq!(
                    rtw_module_index(CString::new(slug).unwrap().as_ptr()),
                    i as i32
                );
            }
        }
        assert!(rtw_module_slug(n).is_null(), "past the end");
        assert!(rtw_module_name(n).is_null());
        assert!(rtw_module_slug(u32::MAX).is_null());
        unsafe {
            let bogus = CString::new("not-a-module").unwrap();
            assert_eq!(rtw_module_index(bogus.as_ptr()), -1);
        }
        // Static means static: a pointer taken first is still the same
        // string after every other catalogue call.
        let first = rtw_module_slug(0);
        assert_eq!(first, rtw_module_slug(0));
        assert_eq!(unsafe { CStr::from_ptr(first) }.to_str().unwrap(), "bungee-roulette");
    }

    /// The Randomizer's one control, straight off After Dark 3.0's resources:
    /// the words and the seconds agree under the floor rule, the factory value
    /// is sVal 503's 60 = "30 min." = 1800 s, and the ends are Short/Forever.
    #[test]
    fn randomizer_duration_is_sval_sunt_rsvl_503() {
        assert_eq!(rtw_randomizer_default(), 60);
        let n = rtw_randomizer_band_count() as i32;
        assert_eq!(n, 14);
        assert_eq!(rtw_randomizer_band_value(0), 0);
        assert_eq!(rtw_randomizer_band_value(n - 1), 100);
        assert_eq!(rtw_randomizer_band_value(n), 0, "past the end");
        assert!(rtw_randomizer_band_label(n).is_null());
        assert!(rtw_randomizer_band_label(-1).is_null());
        let word = |v: i32| unsafe {
            CStr::from_ptr(rtw_randomizer_band_label(rtw_randomizer_band_for(v)))
                .to_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(word(60), "30 min.");
        assert_eq!(rtw_randomizer_seconds(60), 1800);
        assert_eq!(word(0), "Short");
        assert_eq!(rtw_randomizer_seconds(0), 15, "below rsVl's first row = its first row");
        assert_eq!(word(100), "Forever");
        assert_eq!(rtw_randomizer_seconds(100), -1);
        assert_eq!(rtw_randomizer_seconds(24), 60);
        assert_eq!(rtw_randomizer_seconds(31), 60);
        // Every word but "Short" sits exactly on an rsVl row, so the word
        // and the time can never name different buckets.
        for (at, _) in RANDOMIZER_WORDS.iter().skip(1) {
            assert!(RANDOMIZER_SECONDS.iter().any(|(v, _)| v == at), "word at {at} has no rsVl row");
        }
        // Pointers are static.
        assert_eq!(rtw_randomizer_band_label(3), rtw_randomizer_band_label(3));
        // Ascending, as the slider needs.
        assert!(RANDOMIZER_WORDS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(RANDOMIZER_SECONDS.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
