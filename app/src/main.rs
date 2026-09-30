//! retwisted player shell.
//!
//! Keys: [ / ] cycle modules · 1-9 select control · -/= adjust · window
//! title shows module + selected control. Caps Lock is passed through to
//! modules (several easter eggs count it).

use app::{compose, modules, ImageCache};
use engine::{ControlKind, Ctx, Module, Pack, RandomLong, Random15, TickClock};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const SIM_W: usize = engine::SCREEN_W as usize;
const SIM_H: usize = engine::SCREEN_H as usize;

/// Most ticks one catch-up burst may run. A stall (window drag, display
/// sleep, a slow module rebuild) must show up as a pause, never as the sim
/// fast-forwarding through minutes of simulated time in one frame.
const MAX_CATCHUP_TICKS: u32 = 4;

struct App {
    assets_root: std::path::PathBuf,
    slugs: Vec<String>,
    slug_ix: usize,
    module: Option<Box<dyn Module>>,
    controls: Vec<engine::ControlDef>,
    control_vals: Vec<i32>,
    control_ix: usize,
    pack_root: std::path::PathBuf,
    rng_seed: u64,
    ctx_caps: bool,
    /// Ticks run since the module started. The sim clock is *derived* from
    /// it (`clock.now_ms(tick)`) and never from the wall clock: modules
    /// carry their own `now < deadline` gates (Mike's §6 master gate re-arms
    /// `g0910 = now + 100`), and feeding them a wall clock that drifts a
    /// millisecond either side of the grid makes those gates swallow ticks
    /// at random. A tick-derived clock also keeps the live run
    /// bit-identical to `bin/frame`.
    tick: u64,
    /// The module's grid — `Millis(tick_ms)`, or the Mac tick for modules
    /// that gate on `Resource.f4724()`.
    clock: TickClock,
    /// Wall-clock origin of tick 0. Tick `t` is due at `epoch + at_us(t)`;
    /// a dropped backlog moves the epoch, never the tick counter.
    epoch: Instant,
    rng: RandomLong,
    rng15: Random15,
    images: ImageCache,
    /// shell-side pack handle for remap/strings queries (the module owns its
    /// own copy; meta.json is tiny).
    pack: Option<Pack>,
    /// last present() layout: (scale, ox, oy) — maps window px → sim px.
    view: (usize, i32, i32),
    mouse_pos: Option<(f64, f64)>,
    mouse_down: bool,
    sim: Vec<u32>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    _audio: Option<rodio::OutputStream>,
    audio_handle: Option<rodio::OutputStreamHandle>,
    /// The original mixes ALL module sounds through ONE channel; a new
    /// sound pre-empts the current one. One Sink reproduces that.
    channel: Option<rodio::Sink>,
    /// Looping sound channel (resumes when channel is empty).
    loop_channel: Option<(u32, rodio::Sink)>,
    volume: f32,
    /// The MUSIC channel — After Dark's MDRV synth, a SEPARATE mix that
    /// neither pre-empts `channel` nor is pre-empted by it (engine::music's
    /// module docs carry the capture evidence). One Sink, fed forever by a
    /// `MusicStream` that renders silence when no song is playing.
    music: Option<Arc<Mutex<engine::music::Player>>>,
    music_sink: Option<rodio::Sink>,
    music_songs: std::collections::HashMap<u32, std::sync::Arc<engine::music::Song>>,
    /// Last `(song, play)` the module asked for; a change of `play` is a
    /// new play and restarts the tune from the top.
    music_state: Option<(u32, u32)>,
}

/// A never-ending rodio source pulling from the engine's music player.
///
/// The player is behind a mutex because rodio pulls on its own thread while
/// the sim ticks on this one. A poisoned lock or a missing player renders
/// silence rather than killing the audio thread — losing the music is a
/// worse bug than losing it *loudly*, but crashing the mixer is worse still.
struct MusicStream {
    player: Arc<Mutex<engine::music::Player>>,
    buf: Vec<f32>,
    pos: usize,
    rate: u32,
}

impl MusicStream {
    fn new(player: Arc<Mutex<engine::music::Player>>) -> MusicStream {
        let rate = player.lock().map(|p| p.out_rate()).unwrap_or(engine::music::OUT_RATE);
        MusicStream { player, buf: vec![0.0; 1024], pos: 1024, rate }
    }
}

impl Iterator for MusicStream {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buf.len() {
            match self.player.lock() {
                Ok(mut p) => p.render(&mut self.buf),
                Err(_) => self.buf.fill(0.0),
            }
            self.pos = 0;
        }
        let s = self.buf[self.pos];
        self.pos += 1;
        Some(s)
    }
}

impl rodio::Source for MusicStream {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl App {
    fn new(assets_root: std::path::PathBuf, start: Option<String>) -> App {
        let mut slugs: Vec<String> = modules::SLUGS
            .iter()
            .filter(|s| assets_root.join(s).join("meta.json").exists())
            .map(|s| s.to_string())
            .collect();
        if slugs.is_empty() {
            slugs.push("flying-toilets".into());
        }
        let slug_ix = start
            .and_then(|s| slugs.iter().position(|x| *x == s))
            .unwrap_or_else(|| {
                slugs
                    .iter()
                    .position(|s| s == "flying-toilets")
                    .unwrap_or(0)
            });
        let (audio, handle) = match rodio::OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("audio unavailable: {e}");
                (None, None)
            }
        };
        let mut app = App {
            assets_root,
            slugs,
            slug_ix,
            module: None,
            controls: Vec::new(),
            control_vals: Vec::new(),
            control_ix: 0,
            pack_root: std::path::PathBuf::new(),
            rng_seed: 0x5EED_CAFE,
            ctx_caps: false,
            tick: 0,
            clock: TickClock::Millis(40),
            epoch: Instant::now(),
            rng: RandomLong::new(0x5EED_CAFE),
            // the controller ctor does srand(GetTicks()) — a fresh stream
            // every launch, not one canned session
            rng15: Random15::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u32)
                    .unwrap_or(0xC0FFEE),
            ),
            images: ImageCache::new(),
            pack: None,
            view: (1, 0, 0),
            mouse_pos: None,
            mouse_down: false,
            sim: vec![0; SIM_W * SIM_H],
            window: None,
            surface: None,
            _audio: audio,
            audio_handle: handle,
            channel: None,
            loop_channel: None,
            volume: 0.4,
            music: None,
            music_sink: None,
            music_songs: std::collections::HashMap::new(),
            music_state: None,
        };
        app.load_module();
        app
    }

    fn load_module(&mut self) {
        if let Some((_, old)) = self.loop_channel.take() {
            old.stop();
        }
        self.images.clear();
        let slug = self.slugs[self.slug_ix].clone();
        let dir = self.assets_root.join(&slug);
        self.pack_root = dir.clone();
        self.pack = Pack::load(&dir).ok();
        self.module = self.pack.as_ref().and_then(|p| modules::make(&slug, p.clone()));
        match &self.module {
            Some(m) => {
                self.controls = m.controls();
                self.control_vals = self.controls.iter().map(|c| c.default).collect();
                self.control_ix = 0;
                println!("module: {} ({} controls)", m.name(), self.controls.len());
            }
            None => {
                self.controls.clear();
                self.control_vals.clear();
                println!("module {slug}: NOT YET IMPLEMENTED");
            }
        }
        self.rng_seed = self.rng_seed.wrapping_add(1);
        self.rng = RandomLong::new(self.rng_seed);
        self.load_music();
        self.reset_clock();
        self.update_title();
    }

    /// Build this pack's music channel: decode the songs + instrument bank
    /// once, and open a Sink that pulls from the player forever. Modules
    /// with no packed music get no sink at all.
    fn load_music(&mut self) {
        if let Some(s) = self.music_sink.take() {
            s.stop();
        }
        self.music = None;
        self.music_state = None;
        let (Some(h), Some(pack)) = (&self.audio_handle, self.pack.as_ref()) else { return };
        let Some(assets) = pack.music() else { return };
        let songs = assets.songs.clone();
        let mut player = engine::music::Player::new(assets.bank.clone(), engine::music::OUT_RATE);
        player.set_volume(self.volume);
        let player = Arc::new(Mutex::new(player));
        let Ok(sink) = rodio::Sink::try_new(h) else { return };
        sink.append(MusicStream::new(player.clone()));
        self.music_sink = Some(sink);
        self.music = Some(player);
        self.music_songs = songs;
    }

    /// Poll the module's music state and drive the music channel from it.
    /// A new `(song, play)` pair starts the tune from the top; `None`
    /// silences the channel. Nothing here touches the sfx sink.
    fn pump_music(&mut self) {
        let Some(player) = self.music.as_ref() else { return };
        let want = self.module.as_ref().and_then(|m| m.music());
        if want == self.music_state {
            return;
        }
        self.music_state = want;
        let Ok(mut p) = player.lock() else { return };
        match want {
            Some((song, _)) => match self.music_songs.get(&song) {
                Some(s) => p.play(s.clone()),
                None => p.stop(),
            },
            None => p.stop(),
        }
    }

    /// A freshly built module starts at sim time 0 with no tick backlog.
    fn reset_clock(&mut self) {
        self.tick = 0;
        self.clock = self.module.as_ref().map(|m| m.clock()).unwrap_or(TickClock::Millis(40));
        self.epoch = Instant::now();
    }

    /// The engine's "restart me" request (`Required1.fn_40A6`).
    ///
    /// Every module spec documents the same DoDrawFrame contract — phlegm
    /// boy §4 states it plainly: the controls are re-read every frame,
    /// compared against the module's latched copies, and *if one changed and
    /// the mouse button is up* the module asks the engine to restart it.
    /// That is the only way most controls take effect at all: FrankenScreen
    /// consumes Coherency and Blemishes inside `MF_7BA` (the creature build),
    /// so poking `set_control` at a live module changes a latch nothing will
    /// read again until the next build.
    ///
    /// So: destroy the module, construct a fresh one from the same pack, and
    /// replay every current control value into it. The shell's own control
    /// table (`controls`, `control_vals`, `control_ix`) and the title bar are
    /// deliberately untouched — they are the user's UI state, not the
    /// module's.
    fn restart_module(&mut self) {
        if let Some((_, old)) = self.loop_channel.take() {
            old.stop();
        }
        let slug = self.slugs[self.slug_ix].clone();
        let Some(pack) = self.pack.clone() else { return };
        let Some(mut m) = modules::make(&slug, pack) else { return };
        for (i, v) in self.control_vals.iter().enumerate() {
            m.set_control(i, *v);
        }
        self.module = Some(m);
        self.images.clear();
        // a restart is a fresh module launch: new RNG stream, clean clock,
        // and whatever the old instance was playing is cut off.
        self.rng_seed = self.rng_seed.wrapping_add(1);
        self.rng = RandomLong::new(self.rng_seed);
        if let Some(s) = self.channel.take() {
            s.stop();
        }
        // …and the music channel with it: the new instance's play counter
        // starts at 0, so `pump_music` would otherwise see no change.
        if let Some(p) = self.music.as_ref() {
            if let Ok(mut p) = p.lock() {
                p.stop();
            }
        }
        self.music_state = None;
        self.reset_clock();
    }

    fn update_title(&self) {
        let Some(w) = &self.window else { return };
        let name = self
            .module
            .as_ref()
            .map(|m| m.name().to_string())
            .unwrap_or_else(|| format!("{} (unimplemented)", self.slugs[self.slug_ix]));
        let ctl = if self.controls.is_empty() {
            String::new()
        } else {
            let c = &self.controls[self.control_ix];
            let v = self.control_vals[self.control_ix];
            let vs = match &c.kind {
                // popup values are raw: item n is `base + n`, and the base
                // is NOT always 0 (bungee's Jumper is the original's 1-based
                // Mac menu). Reading `items[v]` showed it one item off.
                ControlKind::Popup { items, base } => usize::try_from(v - base)
                    .ok()
                    .and_then(|i| items.get(i))
                    .cloned()
                    .unwrap_or_else(|| v.to_string()),
                ControlKind::Checkbox => if v != 0 { "on".into() } else { "off".into() },
                ControlKind::Slider { .. } => v.to_string(),
            };
            format!("  ·  [{}] {} = {}", self.control_ix + 1, c.name, vs)
        };
        w.set_title(&format!("retwisted — {name}{ctl}"));
    }

    fn adjust_control(&mut self, delta: i32) {
        if self.controls.is_empty() {
            return;
        }
        let i = self.control_ix;
        let c = &self.controls[i];
        let old = self.control_vals[i];
        let v = match &c.kind {
            ControlKind::Slider { min, max } => (old + delta * 5).clamp(*min, *max),
            ControlKind::Popup { items, base } => {
                base + (old - base + delta).rem_euclid(items.len() as i32)
            }
            ControlKind::Checkbox => 1 - old,
        };
        self.control_vals[i] = v;
        // latch it into the live module first (the msg-7 control-changed
        // handlers that latch without rebuilding — FrankenScreen's Music —
        // are the reason `set_control` exists at all) …
        if let Some(m) = &mut self.module {
            m.set_control(i, v);
        }
        // … then honour the DoDrawFrame contract: changed value + mouse
        // button up ⇒ restart the module (see `restart_module`). Clamped
        // values that did not actually move must NOT restart — the original
        // compares latched values, not keystrokes.
        if v != old && !self.mouse_down {
            self.restart_module();
        }
        self.update_title();
    }

    fn play_sounds(&mut self, pack_root: &std::path::Path, ids: &[u32]) {
        let Some(h) = &self.audio_handle else { return };
        for id in ids {
            let p = pack_root.join("sounds").join(format!("{id}.wav"));
            if let Ok(f) = std::fs::File::open(&p) {
                if let Ok(dec) = rodio::Decoder::new(std::io::BufReader::new(f)) {
                    // single channel: pre-empt whatever is playing
                    if let Some(old) = self.channel.take() {
                        old.stop();
                    }
                    if let Ok(sink) = rodio::Sink::try_new(h) {
                        sink.set_volume(self.volume);
                        sink.append(dec);
                        self.channel = Some(sink);
                    }
                    if let Some((_, loop_sink)) = &self.loop_channel {
                        loop_sink.pause();
                    }
                }
            }
        }
    }

    fn pump_loop_sound(&mut self) {
        let Some(h) = &self.audio_handle else { return };
        let want_loop = self.module.as_ref().and_then(|m| m.loop_sound());
        let one_shot_active = self.channel.as_ref().map_or(false, |s| !s.empty());

        match want_loop {
            Some(id) => {
                let matches_current =
                    self.loop_channel.as_ref().map_or(false, |(cur, _)| *cur == id);
                if !matches_current {
                    if let Some((_, old)) = self.loop_channel.take() {
                        old.stop();
                    }
                    let p = self.pack_root.join("sounds").join(format!("{id}.wav"));
                    if let Ok(f) = std::fs::File::open(&p) {
                        use rodio::Source;
                        if let Ok(dec) = rodio::Decoder::new(std::io::BufReader::new(f)) {
                            if let Ok(sink) = rodio::Sink::try_new(h) {
                                sink.set_volume(self.volume);
                                sink.append(dec.repeat_infinite());
                                if one_shot_active {
                                    sink.pause();
                                }
                                self.loop_channel = Some((id, sink));
                            }
                        }
                    }
                } else if let Some((_, sink)) = &self.loop_channel {
                    if one_shot_active {
                        sink.pause();
                    } else {
                        sink.play();
                    }
                }
            }
            None => {
                if let Some((_, old)) = self.loop_channel.take() {
                    old.stop();
                }
            }
        }
    }

    /// Compose the current frame into `sim`. The whole pass lives in the
    /// app library so this shell, `bin/frame` and the macOS screensaver
    /// cannot drift apart; see `app::compose`.
    fn draw(&mut self) {
        match (self.module.as_ref(), self.pack.as_ref()) {
            (Some(m), Some(pack)) => {
                compose(pack, &mut self.images, m.as_ref(), &mut self.sim);
                app::present_gamma(&mut self.sim);
            }
            // no module (or no pack): the shell's own "unimplemented" grey
            _ => self.sim.fill(0x0020_2020),
        }
    }

    fn present(&mut self) {
        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut())
        else {
            return;
        };
        let size = window.inner_size();
        let (pw, ph) = (size.width as usize, size.height as usize);
        if pw == 0 || ph == 0 {
            return;
        }
        surface
            .resize(
                NonZeroU32::new(pw as u32).unwrap(),
                NonZeroU32::new(ph as u32).unwrap(),
            )
            .unwrap();
        let scale = (pw / SIM_W).min(ph / SIM_H).max(1);
        let (ox, oy) = (
            (pw.saturating_sub(SIM_W * scale)) / 2,
            (ph.saturating_sub(SIM_H * scale)) / 2,
        );
        self.view = (scale, ox as i32, oy as i32);
        let mut buf = surface.buffer_mut().unwrap();
        buf.fill(0);
        for y in 0..SIM_H * scale {
            if oy + y >= ph {
                break;
            }
            let sy = y / scale;
            let dst = (oy + y) * pw + ox;
            for x in 0..SIM_W * scale {
                if ox + x >= pw {
                    break;
                }
                buf[dst + x] = self.sim[sy * SIM_W + x / scale];
            }
        }
        buf.present().unwrap();
    }

    /// Run one sim tick and hand the module its per-tick services.
    fn tick_once(&mut self) {
        self.tick += 1;
        if self.module.is_none() {
            return;
        }
        let mut ctx = Ctx {
            rng: std::mem::replace(&mut self.rng, RandomLong::new(1)),
            rng15: std::mem::replace(&mut self.rng15, Random15::new(1)),
            sounds: Vec::new(),
            caps_lock: caps_lock_state().unwrap_or(self.ctx_caps),
            now_ms: self.clock.now_ms(self.tick),
            local_hms: {
                use chrono::Timelike;
                let t = chrono::Local::now();
                (t.hour() as u8, t.minute() as u8, t.second() as u8)
            },
            mouse: self
                .mouse_pos
                .map(|(mx, my)| {
                    let (scale, ox, oy) = self.view;
                    (
                        ((mx as i32 - ox) / scale as i32).clamp(0, SIM_W as i32 - 1),
                        ((my as i32 - oy) / scale as i32).clamp(0, SIM_H as i32 - 1),
                    )
                })
                .unwrap_or((-1, -1)),
            mouse_down: self.mouse_down,
        };
        self.module.as_mut().unwrap().tick(&mut ctx);
        self.rng = ctx.rng;
        self.rng15 = ctx.rng15;
        let root = self.pack_root.clone();
        self.play_sounds(&root, &ctx.sounds);
        self.pump_loop_sound();
        self.pump_music();
    }

    /// Advance the sim to the current wall clock. Returns true when at least
    /// one tick ran, i.e. the frame is stale and wants redrawing.
    ///
    /// This is the whole of the pacing policy:
    /// * ticks fire on the module's own grid, measured from a fixed epoch
    ///   (`epoch + at_us(tick)`), so sim time stays exact and deterministic
    ///   even when the period is not a whole millisecond — the Mac tick is
    ///   16.625 ms and rounding it per tick would drift 2 % ;
    /// * at most `MAX_CATCHUP_TICKS` of them run in one burst, and any
    ///   remaining backlog is *dropped* by moving the epoch — a two-second
    ///   stall costs two seconds of animation, it does not get replayed at
    ///   CPU speed, and the module's clock never jumps backwards;
    /// * nothing is drawn here. The caller redraws once, after the burst.
    fn pump(&mut self) -> bool {
        let mut ran = 0;
        while self.epoch.elapsed() >= self.due(self.tick + 1) && ran < MAX_CATCHUP_TICKS {
            self.tick_once();
            ran += 1;
        }
        if self.epoch.elapsed() >= self.due(self.tick + 1) {
            // still behind: re-anchor so the dropped backlog is gone, with
            // the tick counter (and therefore the sim clock) untouched.
            self.epoch = Instant::now() - self.due(self.tick);
        }
        ran > 0
    }

    /// Wall-clock offset of tick `t` from the epoch.
    fn due(&self, t: u64) -> Duration {
        Duration::from_micros(self.clock.at_us(t))
    }

    fn key(&mut self, ev: KeyEvent) {
        if ev.state != ElementState::Pressed {
            return;
        }
        match ev.logical_key {
            Key::Named(NamedKey::CapsLock) => {
                self.ctx_caps = !self.ctx_caps;
            }
            Key::Character(ref s) => match s.as_str() {
                "[" => {
                    self.slug_ix =
                        (self.slug_ix + self.slugs.len() - 1) % self.slugs.len();
                    self.load_module();
                }
                "]" => {
                    self.slug_ix = (self.slug_ix + 1) % self.slugs.len();
                    self.load_module();
                }
                "," => {
                    self.volume = (self.volume - 0.1).max(0.0);
                    if let Some(s) = &self.channel { s.set_volume(self.volume); }
                    if let Some(p) = &self.music {
                        if let Ok(mut p) = p.lock() { p.set_volume(self.volume); }
                    }
                    println!("volume {:.0}%", self.volume * 100.0);
                }
                "." => {
                    self.volume = (self.volume + 0.1).min(1.0);
                    if let Some(s) = &self.channel { s.set_volume(self.volume); }
                    if let Some(p) = &self.music {
                        if let Ok(mut p) = p.lock() { p.set_volume(self.volume); }
                    }
                    println!("volume {:.0}%", self.volume * 100.0);
                }
                "-" => self.adjust_control(-1),
                "=" | "+" => self.adjust_control(1),
                d @ ("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") => {
                    let i = d.parse::<usize>().unwrap() - 1;
                    if i < self.controls.len() {
                        self.control_ix = i;
                        self.update_title();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("retwisted")
                        .with_inner_size(LogicalSize::new(SIM_W as f64, SIM_H as f64)),
                )
                .unwrap(),
        );
        let context = softbuffer::Context::new(window.clone()).unwrap();
        self.surface = Some(softbuffer::Surface::new(&context, window.clone()).unwrap());
        self.window = Some(window);
        // window + surface creation happens after App::new, and whatever it
        // cost must not land on the module as a tick backlog.
        self.reset_clock();
        self.update_title();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => self.key(event),
            // Draw only. The sim is advanced in `about_to_wait`, which asks
            // for exactly one redraw per tick; the OS may also send us a
            // RedrawRequested for an expose/resize, and re-presenting the
            // current frame is the right answer to that too.
            WindowEvent::RedrawRequested => {
                self.draw();
                self.present();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Some((position.x as f64, position.y as f64));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    self.mouse_down = state == ElementState::Pressed;
                }
            }
            _ => {}
        }
    }

    /// The pacing loop. The old version asked for a redraw unconditionally
    /// under `ControlFlow::Poll`, so the app spun the CPU redrawing at
    /// however many thousand frames a second softbuffer would take, and the
    /// tick loop inside RedrawRequested caught up in whole-tick bursts —
    /// "stuttery and way too fast". Now: run whatever ticks are due, redraw
    /// once if any did, then sleep until the next tick deadline.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.pump() {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        let next = self.epoch + self.due(self.tick + 1);
        event_loop.set_control_flow(ControlFlow::WaitUntil(next));
    }
}

/// The real Caps Lock state, read from the OS each tick.
///
/// Caps Lock is a lock STATE, and every module that reads it (toilets,
/// mayhem, mime-hunt's interactive mode) watches that state for edges — the
/// saver reads it the same way (`NSEvent.modifierFlags`). On macOS winit
/// does not reliably deliver Caps Lock as a key press (it arrives as a
/// flags change), so the old key-toggle never fired there. Elsewhere this
/// returns None and the key toggle in `key()` stays the fallback.
#[cfg(target_os = "macos")]
fn caps_lock_state() -> Option<bool> {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceFlagsState(state_id: i32) -> u64;
    }
    const HID_SYSTEM_STATE: i32 = 1; // kCGEventSourceStateHIDSystemState
    const ALPHA_SHIFT: u64 = 0x0001_0000; // kCGEventFlagMaskAlphaShift
    Some(unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) } & ALPHA_SHIFT != 0)
}

#[cfg(not(target_os = "macos"))]
fn caps_lock_state() -> Option<bool> {
    None
}

fn main() {
    let assets_root = std::path::PathBuf::from("assets");
    let start = std::env::args().nth(1);
    let event_loop = EventLoop::new().unwrap();
    // `about_to_wait` re-arms this as WaitUntil(next tick) after every pass;
    // Wait is just the starting state, so we block instead of spinning
    // before the first tick deadline exists.
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(assets_root, start);
    event_loop.run_app(&mut app).unwrap();
}
