//! Bungee Roulette — ported function-by-function from the Ghidra decompile of
//! module CODE 129 "DynaBungee" (totally-twisted docs/decompiled/bungee-roulette/,
//! private) with the Library 4.0 sprite/sequence classes read alongside it.
//! Listing addresses below are the raw resource_dasm ones (raw − 4).
//!
//! ## Objects (controller ctor `fn56` @0000)
//! - a sound bank of snd 128..135 (`fn49/fn51` @0860/@09A2)
//! - one RLESequence over art bank 9000 (`g004C`)
//! - the JUMPER (`fn32/fn37/fn38` @14D6/@15F0/@1646) and the BODY
//!   (`fn06/fn09/fn10` @0B2C/@0BC6/@0C2C): each a Library sprite with a state
//!   sub-object at +0x40 driven by the module's message handler
//! - `g0056` "a body is falling" handshake, primed to 1 so the body's state-0
//!   enter fires on the first frame and releases the jumper
//!
//! ## Library sprite semantics (L135 `fn5340`/`fn55B6`, `fn16BC`, `fn3DDC`)
//! - `SetState(n)` (@55B6) fires the EXIT message for the current state at
//!   once and only flags the enter; the next `Run` (@5340) fires ENTER then
//!   UPDATE in one go. A handler branch that calls SetState returns without
//!   the draw/reschedule tail, so the sprite runs again on the very next
//!   frame — transitions cost zero delay.
//! - `pos` is the CENTRE of the current frame's bounds: `Draw` (@16BC) puts
//!   the frame at `pos - (w>>1, h>>1)`.
//! - `Link(from, to)` (@3DDC) is `centre(to) - centre(from)` in bank space,
//!   so every advance and every retarget keeps the bank-space placement of
//!   the art; nothing else moves it but the explicit `y +=` in the module.
//! - loop advance (`fn46` @20C8 / `fn13` @0FA6): `pos += link(d6, d6+1)`, or
//!   `link(da, d8)` at the last frame; one-shot (`fn45` @2018 / `fn12`
//!   @0EF6): the same, except finishing applies no delta and reports done.
//!
//! ## RNG
//! Every roll is ANSI `rand()` (seg 131 @59A0, engine `Random15`, now the
//! real high-bits LCG). Order at construction: message line
//! (`rand()%13` with Cow chosen, else `rand()%8`, @0158-0192), then the
//! Random victim (`rand()%5`, `fn35` @1570). Per jump (`fn41` @1B3E): victim
//! re-roll if Random, `rand()%100` cord roll, `rand()%W` x, taut-y roll,
//! entry frame. Splat (`fn15` @123E): `rand()%100` then `rand()%2`.
//!
//! ## What stays approximate
//! - Message line: the original writes it to After Dark's own message line;
//!   nothing is drawn on the canvas (`SHOW_QUIP_CAPTIONS` gates `texts`).
//! - Palette: depth-4 pushes clut 316; the packed art is the depth-8/32
//!   clut-304 bake, which the capture confirmed.
//! - Live control changes: the original re-reads the panel every frame and
//!   requests a restart when a bucket changes with the button up
//!   (`fn59` @068A); we arm it from `set_control`.
//! - Residue: the sprite's `+0x2C(1)/(0)` flash at a life's end leaves the
//!   last DRAWN frame on the background (capture `bungee-long.mp4`: cords
//!   from y = 0, a splat bank along the shore, wiped every N jumps).
//! - Sound-bank busy time is the packed WAV length; the original asks the
//!   Sound Manager (`SoundRateConvert`).
//! - The frame gate is `next_due <= TickCount()*16.625`, so delays 40/80
//!   fire on the 3rd/5th Mac tick; we tick on that grid (`TickClock::MacTick`).

use std::collections::HashMap;

use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, TickClock, SpriteDraw, TextDraw, SCREEN_H, SCREEN_W,
};

const BASE: u32 = 9000;

// snd resource ids 128..135, loaded as a bank at construction (fn51 @09A2)
const SND_MOO: u32 = 128; // Cow moo — jumper enter 2, kind == Cow (@17B6)
const SND_RIP: u32 = 129; // Rip in Two — snap frame d8+2, halves only (@19C8)
const SND_SPLAT: u32 = 130; // Wet Splat — body enter 2 (@0DD0)
const SND_BREAK: u32 = 131; // Cord Break — snap frame d8+4, non-halves (@198E)
const SND_BOUNCE1: u32 = 132; // Cord Bounce 1 — bounce frame d8+16 (@18EA)
const SND_COW_SCREAM: u32 = 133; // body fall, once, after the bank idles (@0D8E)
const SND_MAN_SCREAM: u32 = 134; // body fall, once, after the bank idles (@0D78)
const SND_BOUNCE2: u32 = 135; // Cord Bounce 2 — bounce enter (@185C)

/// One packed frame's bounds in bank space: the compound rect shifted by the
/// frame's own OFst offset (L135 `fn3A70`, the sequence's `+0x18`).
#[derive(Clone, Copy)]
struct Geom {
    bx: i32,
    by: i32,
    w: i32,
    h: i32,
    dx: i32,
    dy: i32,
}

/// A Library sprite as the module uses it. `x,y` is the centre of frame
/// `d6`'s bounds on screen (L135 `fn16BC`).
struct Sprite {
    x: i32,
    y: i32,
    d8: u32, // run first frame
    da: u32, // run last frame
    d6: u32, // current frame
    delay: u64,    // ms between frames (+0xE2 jumper / +0xE4 body)
    next_due: u64, // frame gate (+0xCE jumper / +0xE0 body)
}

impl Sprite {
    fn fresh(delay: u64) -> Self {
        Sprite { x: -100, y: -100, d8: 1, da: 1, d6: 1, delay, next_due: 0 }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum JState {
    Idle,   // 0 offscreen
    Arm,    // 1 arm the next jump once the body is done
    Dive,   // 2 looping flail, y += 10
    Bounce, // 3 cord held
    Snap,   // 4 cord snapped
    Recoil, // 5 cord / leftover half snaps back up
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum BState {
    Dormant, // 0
    Fall,    // 1 free fall + scream
    Splat,   // 2
}

/// Centre of a frame's bounds in bank space: `(l + r) >> 1, (t + b) >> 1`.
fn center(g: &HashMap<u32, Geom>, f: u32) -> Option<(i32, i32)> {
    g.get(&f).map(|gm| (gm.bx + gm.dx + (gm.w >> 1), gm.by + gm.dy + (gm.h >> 1)))
}

/// L135 `fn3DDC` (sequence `+0x78`): `centre(to) - centre(from)`.
fn link(g: &HashMap<u32, Geom>, from: u32, to: u32) -> (i32, i32) {
    match (center(g, from), center(g, to)) {
        (Some(a), Some(b)) => (b.0 - a.0, b.1 - a.1),
        _ => (0, 0),
    }
}

/// The delta the advance helpers fetch before stepping: to the next frame,
/// or from the last frame back to the first.
fn step_delta(g: &HashMap<u32, Geom>, s: &Sprite) -> (i32, i32) {
    if s.d6 < s.d8 {
        (0, 0)
    } else if s.d6 < s.da {
        link(g, s.d6, s.d6 + 1)
    } else {
        link(g, s.da, s.d8)
    }
}

/// `fn46` @20C8 (jumper) / `fn13` @0FA6 (body): looping advance, then the
/// extra fall.
fn adv_loop(g: &HashMap<u32, Geom>, s: &mut Sprite, extra_dy: i32) {
    let (dx, dy) = step_delta(g, s);
    s.x += dx;
    s.y += dy;
    s.d6 += 1;
    if s.d6 > s.da {
        s.d6 = s.d8;
    }
    s.y += extra_dy;
}

/// `fn45` @2018 (jumper) / `fn12` @0EF6 (body): one-shot advance. Finishing
/// applies no delta; the jumper rests on the last frame, the body resets to
/// the first (only the flash's last DRAWN frame matters, so it's cosmetic).
fn adv_oneshot(g: &HashMap<u32, Geom>, s: &mut Sprite, rest_on_first: bool) -> bool {
    let (dx, dy) = step_delta(g, s);
    s.d6 += 1;
    if s.d6 > s.da {
        s.d6 = if rest_on_first { s.d8 } else { s.da };
        return true;
    }
    s.x += dx;
    s.y += dy;
    false
}

/// The `SetRun(); Link(this, d6, d8); d6 = d8` idiom of every enter handler
/// (`fn47` @217A / `fn16` @13CE do the link).
fn retarget(g: &HashMap<u32, Geom>, s: &mut Sprite, first: u32, last: u32, delay: u64) {
    let (dx, dy) = link(g, s.d6, first);
    s.x += dx;
    s.y += dy;
    s.d8 = first;
    s.da = last;
    s.d6 = first;
    s.delay = delay;
}

/// Union bounds (w, h) of a run — sequence `+0x70` (L135 `fn3B04`).
fn run_bounds(g: &HashMap<u32, Geom>, first: u32, last: u32) -> (i32, i32) {
    let (mut x1, mut y1, mut x2, mut y2) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for fno in first..=last {
        if let Some(gm) = g.get(&fno) {
            x1 = x1.min(gm.bx);
            y1 = y1.min(gm.by);
            x2 = x2.max(gm.bx + gm.w);
            y2 = y2.max(gm.by + gm.h);
        }
    }
    if x1 == i32::MAX { (0, 0) } else { (x2 - x1, y2 - y1) }
}

/// L135 `fn16BC`: the frame drawn with its bounds centred on `pos`.
fn draw_of(g: &HashMap<u32, Geom>, pack: &Pack, s: &Sprite) -> Option<SpriteDraw> {
    let f = pack.frame(BASE, s.d6)?;
    let gm = g.get(&s.d6)?;
    Some(SpriteDraw {
        flip: false,
        pal: 0,
        png: f.png.clone(),
        x: s.x - (gm.w >> 1),
        y: s.y - (gm.h >> 1),
    })
}


/// Fits a one-line caption inside `max_w` px: scale 2 if it fits, else
/// scale 1, else wrap onto two scale-1 lines split at the space nearest the
/// midpoint. Only used behind `SHOW_QUIP_CAPTIONS`.
fn fit_caption(text: &str, max_w: i32) -> Vec<(String, u32)> {
    for scale in [2u32, 1] {
        if engine::font::text_width(text, scale) <= max_w {
            return vec![(text.to_string(), scale)];
        }
    }
    let chars: Vec<char> = text.chars().collect();
    let mid = chars.len() / 2;
    let mut split = mid;
    for d in 0..=chars.len() {
        if mid >= d && chars[mid - d] == ' ' {
            split = mid - d;
            break;
        }
        if mid + d < chars.len() && chars[mid + d] == ' ' {
            split = mid + d;
            break;
        }
    }
    let line1: String = chars[..split].iter().collect();
    let line2: String = chars[split..].iter().collect::<String>().trim_start().to_string();
    vec![(line1, 1), (line2, 1)]
}

pub struct BungeeRoulette {
    pack: Pack,
    geom: HashMap<u32, Geom>,
    // controls (raw values, exactly as GetControlValue returns them)
    jumper_sel: i32,   // mVal 1000, 1-based: 1 Man .. 5 Half Fish, 6 Random
    jumps_slider: i32, // sVal 1001, 0..100
    equip_slider: i32, // sVal 1002, 0..100
    // derived buckets (fn35 @1570; cached in the controller at +0x14/16/18)
    popup_idx: i32,    // g0226 (0-based; 5 = Random)
    jumps_bucket: i32, // g0224
    equip_bucket: i32, // g0220
    wipe_after: i32,   // g021C
    jump_count: i32,   // g0222
    kind: i32,         // g0054, the victim in play 0..4
    cord_holds: bool,  // g021E
    body_active: u32,  // g0056
    handoff: (i32, i32), // g0058: the jumper's pos when the snap finished
    taut_y: i32,       // jumper +0xE0
    play_w: i32,       // g0060 width, floored to 200
    play_h: i32,       // g0060 height, floored to 250
    jstate: JState,
    jpending: Option<JState>, // state sub-object +0xE: enter pending
    bstate: BState,
    bpending: Option<BState>,
    jumper: Sprite,
    body: Sprite,
    velocity: i32,    // body +0xDE
    ground_bias: i32, // body +0xDC
    ground_h: i32,    // body +0xDA: fall-run union height
    screamed: bool,   // body +0xE6
    bank_busy_until: u64, // bank +0x28
    snd_ms: [u64; 8], // durations of snd 128..135
    residue: Vec<SpriteDraw>, // what the flash left on the background
    /// Each sprite's last draw tail. Enters never draw and a transitioning
    /// update skips the tail, so the previous image stays up until then.
    jumper_img: Option<SpriteDraw>,
    body_img: Option<SpriteDraw>,
    need_ctor_rolls: bool, // the controller ctor's rand() calls, once per run
    message: Option<String>, // the STR# picked for this run
    restart_pending: bool, // fn59's restart request, fired with the button up
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

fn build(pack: Pack) -> Option<BungeeRoulette> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    let mut geom = HashMap::new();
    for seq in pack.series(BASE) {
        for (i, f) in seq.frames.iter().enumerate() {
            let img = pack.image(&f.png);
            geom.insert(
                seq.first + i as u32,
                Geom { bx: f.bx, by: f.by, w: img.w as i32, h: img.h as i32, dx: f.dx, dy: f.dy },
            );
        }
    }
    let mut snd_ms = [0u64; 8];
    for (i, slot) in snd_ms.iter_mut().enumerate() {
        *slot = pack.sound_ms(128 + i as u32);
    }

    let (popup_idx, jumps_bucket, equip_bucket, wipe_after) = derive_buckets(1, 50, 50);
    let mut m = BungeeRoulette {
        pack,
        geom,
        jumper_sel: 1,
        jumps_slider: 50,
        equip_slider: 50,
        popup_idx,
        jumps_bucket,
        equip_bucket,
        wipe_after,
        jump_count: 0,
        kind: 0,
        cord_holds: true,
        body_active: 1,
        handoff: (-100, -100),
        taut_y: 0,
        play_w: SCREEN_W.max(200),
        play_h: SCREEN_H.max(250),
        jstate: JState::Idle,
        jpending: Some(JState::Idle),
        bstate: BState::Dormant,
        bpending: Some(BState::Dormant),
        jumper: Sprite::fresh(40),
        body: Sprite::fresh(40),
        velocity: 1,
        ground_bias: 0,
        ground_h: 0,
        screamed: false,
        bank_busy_until: 0,
        snd_ms,
        residue: Vec::new(),
        jumper_img: None,
        body_img: None,
        need_ctor_rolls: true,
        message: None,
        restart_pending: false,
    };
    m.reset_state();
    Some(m)
}

/// `fn35` @1570: the cached buckets from the raw control values.
fn derive_buckets(jumper_sel: i32, jumps: i32, equip: i32) -> (i32, i32, i32, i32) {
    let popup_idx = jumper_sel.clamp(1, 6) - 1; // g0226 = value - 1
    let jb = jumps.clamp(0, 100) / 20; // g0224
    let eb = equip.clamp(0, 100) / 20; // g0220
    let mut wipe = if jb == 0 { 1 } else { jb * 4 }; // g021C @15A2-15EC
    if jb == 5 {
        wipe = 250;
    }
    (popup_idx, jb, eb, wipe)
}

impl BungeeRoulette {
    /// `first + len(first) - 1`, the original's idiom for a run's last frame
    /// (sequence `+0x90` counts the run).
    fn last_of(&self, first: u32) -> u32 {
        first + self.pack.seq_len(BASE, first).max(1) - 1
    }

    /// Controller (re)construction (`fn56` @0000). The rand() calls happen
    /// in `tick` (`need_ctor_rolls`) because they need the ctx.
    fn reset_state(&mut self) {
        self.kind = self.popup_idx.clamp(0, 4); // g0054 (Random rolls in tick)
        self.cord_holds = true;
        self.body_active = 1; // @03DA
        self.handoff = (-100, -100);
        self.jump_count = 0; // g0222 = 0 @15EC
        self.jstate = JState::Idle;
        self.jpending = Some(JState::Idle);
        self.bstate = BState::Dormant;
        self.bpending = Some(BState::Dormant);
        self.velocity = 1;
        self.ground_bias = 0;
        self.ground_h = 0;
        self.screamed = false;
        self.bank_busy_until = 0;
        self.residue.clear();
        self.jumper_img = None;
        self.body_img = None;
        self.jumper = Sprite::fresh(40);
        self.body = Sprite::fresh(40);
        self.need_ctor_rolls = true;
    }

    /// `fn53` @0A76: play one bank sound; bank +0x28 = now + its length so
    /// `fn54` @0ADE can answer "is the bank idle".
    fn play(&mut self, ctx: &mut Ctx, id: u32) {
        ctx.sounds.push(id);
        let d = self.snd_ms.get((id - SND_MOO) as usize).copied().unwrap_or(0);
        self.bank_busy_until = ctx.now_ms + d;
    }

    /// `fn41` @1B3E — arm the next jump (jumper update 1).
    fn arm_jump(&mut self, ctx: &mut Ctx) {
        // Random re-rolls the victim every jump (@1B46-1B58)
        if self.popup_idx == 5 {
            self.kind = ctx.rng15.below(5) as i32;
        }
        let first = match self.kind {
            0 | 1 => 72, // Man / Half Man
            2 => 1,      // Cow
            _ => 222,    // Fish / Half Fish
        };
        let last = self.last_of(first);

        // the roulette (@1BE6): rand() % 100 against the Equipment bucket
        let roll = ctx.rng15.below(100) as i32;
        self.cord_holds = match self.equip_bucket {
            0 => true,
            1 => roll > 24,
            2 => roll > 49,
            3 => roll > 74,
            _ => false,
        };

        // union bounds of the dive run -> +0xDC/+0xDE (w, h)
        let (_w, h) = run_bounds(&self.geom, first, last);

        // x = rand() % W, no right-edge clamp; y = -2 - h/2 (just above)
        let x = ctx.rng15.below(self.play_w as u16) as i32;
        let y = -2 - h / 2;

        // taut-y: rand() % (h/4 + min(h/2, H/2 - h/2) - 30) - h/4 (@1CDE-1D5A)
        let span = if h - 30 < self.play_h / 2 {
            h / 4 + h / 2 - 30
        } else {
            h / 4 + (self.play_h / 2 - h / 2) - 30
        };
        self.taut_y = ctx.rng15.below(span.max(1) as u16) as i32 - h / 4;

        let j = &mut self.jumper;
        j.x = x;
        j.y = y;
        j.d8 = first;
        j.da = last;
        j.delay = 40; // +0xE2 = 0x28
        j.next_due = ctx.now_ms + 40; // +0xCE = now + delay (@1D64)

        // random entry frame: d6 = d8 + rand() % (len - 1) (@1D84)
        let len = self.pack.seq_len(BASE, first).max(2);
        j.d6 = first + ctx.rng15.below((len - 1) as u16) as u32;
    }

    /// L135 `fn55B6` for the jumper: exit(current) now, enter deferred.
    fn set_jstate(&mut self, ns: JState) {
        if self.jstate == JState::Arm {
            // exit 0x4001 (@173A): count the jump; wipe past the threshold
            self.jump_count += 1;
            if self.jump_count > self.wipe_after {
                self.residue.clear();
                self.jump_count = 1;
            }
        }
        self.jpending = Some(ns);
    }

    /// L135 `fn55B6` for the body.
    fn set_bstate(&mut self, ns: BState) {
        self.bpending = Some(ns);
    }

    /// The jumper's `Run` (`fn38` @1646 through L135 `fn5340`). Returns
    /// true when the update transitioned, i.e. skipped the reschedule tail.
    fn run_jumper(&mut self, ctx: &mut Ctx) -> bool {
        if let Some(ns) = self.jpending.take() {
            self.jstate = ns;
            match ns {
                JState::Idle => {
                    // enter 0x8000 (@16CC): pos = (-100,-100)
                    self.jumper.x = -100;
                    self.jumper.y = -100;
                }
                JState::Arm => {}
                JState::Dive => {
                    // enter 0x8002 (@17B6): the cow moos on the way down
                    if self.kind == 2 {
                        self.play(ctx, SND_MOO);
                    }
                }
                JState::Bounce => {
                    // enter 0x8003 (@1840): fn42 @1DC4 (delay 80), link, snd 135
                    let first = match self.kind {
                        0 | 1 => 82,
                        2 => 7,
                        _ => 228,
                    };
                    let last = self.last_of(first);
                    retarget(&self.geom, &mut self.jumper, first, last, 80);
                    self.play(ctx, SND_BOUNCE2);
                }
                JState::Snap => {
                    // enter 0x8004 (@1906): fn43 @1E66 (delay 80), link
                    let first = match self.kind {
                        0 => 115,
                        1 => 159,
                        2 => 40,
                        3 => 261,
                        _ => 293,
                    };
                    let last = self.last_of(first);
                    retarget(&self.geom, &mut self.jumper, first, last, 80);
                }
                JState::Recoil => {
                    // enter 0x8005 (@19E4): fn44 @1F70 (delay 80), link
                    let first = match self.kind {
                        1 => 167,
                        4 => 302,
                        _ => 339,
                    };
                    let last = self.last_of(first);
                    retarget(&self.geom, &mut self.jumper, first, last, 80);
                }
            }
        }

        match self.jstate {
            JState::Idle => {
                // update 0 (@170A): straight to 1
                self.set_jstate(JState::Arm);
                true
            }
            JState::Arm => {
                // update 1 (@171E): wait for the body, then arm and dive
                if self.body_active == 0 {
                    self.arm_jump(ctx);
                    self.set_jstate(JState::Dive);
                    return true;
                }
                false
            }
            JState::Dive => {
                // update 2 (@17E0): fn46(this, 10); taut -> bounce or snap
                adv_loop(&self.geom, &mut self.jumper, 10);
                if self.taut_y <= self.jumper.y {
                    let ns = if self.cord_holds { JState::Bounce } else { JState::Snap };
                    self.set_jstate(ns);
                    return true;
                }
                false
            }
            JState::Bounce => {
                // update 3 (@1878)
                if adv_oneshot(&self.geom, &mut self.jumper, false) {
                    self.flash_jumper();
                    self.set_jstate(JState::Idle);
                    return true;
                }
                if self.jumper.d6 == self.jumper.d8 + 16 {
                    self.play(ctx, SND_BOUNCE1);
                }
                false
            }
            JState::Snap => {
                // update 4 (@1932)
                if adv_oneshot(&self.geom, &mut self.jumper, false) {
                    self.handoff = (self.jumper.x, self.jumper.y); // g0058
                    self.body_active = 1; // g0056 = 1
                    self.set_jstate(JState::Recoil);
                    return true;
                }
                let half = self.kind == 1 || self.kind == 4;
                if self.jumper.d6 == self.jumper.d8 + 4 && !half {
                    self.play(ctx, SND_BREAK);
                } else if self.jumper.d6 == self.jumper.d8 + 2 && half {
                    self.play(ctx, SND_RIP);
                }
                false
            }
            JState::Recoil => {
                // update 5 (@1A10)
                if adv_oneshot(&self.geom, &mut self.jumper, false) {
                    self.flash_jumper();
                    self.set_jstate(JState::Idle);
                    return true;
                }
                false
            }
        }
    }

    /// The `+0x2C(1); world.refresh(); +0x2C(0)` flash at a life's end: the
    /// last drawn frame stays on the background.
    fn flash_jumper(&mut self) {
        if let Some(d) = &self.jumper_img {
            self.residue.push(d.clone());
        }
    }

    fn flash_body(&mut self) {
        if let Some(d) = &self.body_img {
            self.residue.push(d.clone());
        }
    }

    /// The body's `Run` (`fn10` @0C2C). Same contract as `run_jumper`.
    fn run_body(&mut self, ctx: &mut Ctx) -> bool {
        if let Some(ns) = self.bpending.take() {
            self.bstate = ns;
            match ns {
                BState::Dormant => {
                    // enter 0x8000 (@0C78): release the jumper, park offscreen
                    self.body_active = 0; // g0056 = 0
                    self.body.x = -100;
                    self.body.y = -100;
                    self.body.d6 = 47;
                }
                BState::Fall => {
                    // enter 0x8001 (@0CD6): fn14 @1046 picks the fall run,
                    // the ground bias and the snap run's LAST frame as d6;
                    // pos = g0058; link(d6, d8); y += 6; d6 = d8
                    let (first, bias, snap_first) = match self.kind {
                        0 => (122, 16, 115),
                        1 => (184, 20, 159),
                        2 => (47, 6, 40),
                        3 => (268, 10, 261),
                        _ => (318, 10, 293),
                    };
                    let last = self.last_of(first);
                    let (_w, h) = run_bounds(&self.geom, first, last);
                    self.ground_bias = bias;
                    self.ground_h = h;
                    self.velocity = 1; // +0xDE
                    let snap_last = self.last_of(snap_first);
                    let b = &mut self.body;
                    b.x = self.handoff.0;
                    b.y = self.handoff.1;
                    b.d6 = snap_last;
                    retarget(&self.geom, b, first, last, 40);
                    b.y += 6;
                    self.screamed = false; // +0xE6
                }
                BState::Splat => {
                    // enter 0x8002 (@0DB4): fn15 @123E, link, snd 130
                    let roll = ctx.rng15.below(100) as i32;
                    let (first, last) = match self.kind {
                        0 => {
                            let s = if ctx.rng15.below(2) == 0 { 363 } else { 128 };
                            (s, if roll < 26 { self.last_of(s) } else { s + 11 })
                        }
                        1 => {
                            let s = if ctx.rng15.below(2) == 0 { 394 } else { 190 };
                            (s, if roll < 26 { self.last_of(s) } else { s + 12 })
                        }
                        2 => (53, self.last_of(53)),
                        3 => (274, self.last_of(274)),
                        _ => (324, self.last_of(324)),
                    };
                    retarget(&self.geom, &mut self.body, first, last, 80);
                    self.play(ctx, SND_SPLAT);
                }
            }
        }

        match self.bstate {
            BState::Dormant => {
                // update 0 (@0CA2): -> state g0056 once the jumper set it
                if self.body_active != 0 {
                    self.set_bstate(BState::Fall);
                    return true;
                }
                false
            }
            BState::Fall => {
                // update 1 (@0D14): fn17 ground test first, then fn13(vel),
                // vel += 2 up to 11, then the one scream once the bank idles
                if self.play_h < self.ground_bias + self.ground_h / 2 + self.body.y + 20 {
                    self.set_bstate(BState::Splat);
                    return true;
                }
                adv_loop(&self.geom, &mut self.body, self.velocity);
                if self.velocity < 10 {
                    self.velocity += 2;
                }
                if self.bank_busy_until <= ctx.now_ms && !self.screamed {
                    match self.kind {
                        0 | 1 => self.play(ctx, SND_MAN_SCREAM),
                        2 => self.play(ctx, SND_COW_SCREAM),
                        _ => {} // fish never scream
                    }
                    self.screamed = true;
                }
                false
            }
            BState::Splat => {
                // update 2 (@0DEC): one-shot; done -> flash, -> 0
                if adv_oneshot(&self.geom, &mut self.body, true) {
                    self.flash_body();
                    self.set_bstate(BState::Dormant);
                    return true;
                }
                false
            }
        }
    }
}

impl Module for BungeeRoulette {
    fn name(&self) -> &'static str {
        "Bungee Roulette"
    }

    /// Three controls. The Jumper popup keeps the original's 1-based raw
    /// values (mVal 1000 default 1 = Man; 6 = Random).
    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Jumper".into(),
                kind: ControlKind::Popup {
                    base: 1,
                    // MENU 1000, from the pack: five jumpers + Random.
                    items: self.pack.popup_items(1000, 6, false),
                },
                default: 1,
            },
            ControlDef {
                name: "Jumps".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            ControlDef {
                name: "Equipment".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
        ]
    }

    /// `fn59` @068A re-reads the panel every frame and requests a restart
    /// when a bucket changed and the button is up. Random == Random is not
    /// a change (@06A0 substitutes the cached value).
    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.jumper_sel = value,
            1 => self.jumps_slider = value.clamp(0, 100),
            2 => self.equip_slider = value.clamp(0, 100),
            _ => {}
        }
        let (popup, jb, eb, wipe) =
            derive_buckets(self.jumper_sel, self.jumps_slider, self.equip_slider);
        let changed = (popup != self.popup_idx && !(popup == 5 && self.popup_idx == 5))
            || jb != self.jumps_bucket
            || eb != self.equip_bucket;
        if changed {
            self.popup_idx = popup;
            self.jumps_bucket = jb;
            self.equip_bucket = eb;
            self.wipe_after = wipe;
            self.restart_pending = true;
        }
    }

    /// `fn59` @068A DoDrawFrame: jumper when due, then the body when due
    /// and only while g0056 is set.
    fn tick(&mut self, ctx: &mut Ctx) {
        if self.restart_pending && !ctx.mouse_down {
            self.restart_pending = false;
            self.reset_state();
        }
        if self.need_ctor_rolls {
            self.need_ctor_rolls = false;
            // message line (@0158-0192): Cow chosen -> STR# 128 + rand()%13
            // (the cow puns), else 128 + rand()%8
            let n = if self.popup_idx == 2 { 13 } else { 8 };
            let id = 128u16 + ctx.rng15.below(n);
            self.message = self.pack.strings(id).first().cloned();
            // fn35 @1570: the Random victim for the first jump
            if self.popup_idx == 5 {
                self.kind = ctx.rng15.below(5) as i32;
            }
        }
        let now = ctx.now_ms; // g005C
        if self.jumper.next_due <= now && !self.run_jumper(ctx) {
            // tail @1A9E: draw at pos, reschedule
            self.jumper_img = draw_of(&self.geom, &self.pack, &self.jumper);
            self.jumper.next_due = now + self.jumper.delay;
        }
        if self.body_active != 0 && self.body.next_due <= now && !self.run_body(ctx) {
            self.body_img = draw_of(&self.geom, &self.pack, &self.body); // tail @0E8C
            self.body.next_due = now + self.body.delay;
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        for d in &self.residue {
            out.push(SpriteDraw { flip: false, pal: 0, png: d.png.clone(), x: d.x, y: d.y });
        }
        if let Some(d) = &self.body_img {
            out.push(d.clone());
        }
        if let Some(d) = &self.jumper_img {
            out.push(d.clone());
        }
    }

    /// The STR# tagline goes to After Dark's own message line in the
    /// original, so this only draws behind `SHOW_QUIP_CAPTIONS`.
    fn texts(&self, out: &mut Vec<TextDraw>) {
        if !engine::SHOW_QUIP_CAPTIONS {
            return;
        }
        if let Some(t) = &self.message {
            let lines = fit_caption(t, SCREEN_W - 16);
            let base_y = SCREEN_H - 48;
            let mut y = base_y - (lines.len() as i32 - 1) * 8;
            for (text, scale) in lines {
                let w = engine::font::text_width(&text, scale);
                out.push(TextDraw {
                    x: (SCREEN_W - w) / 2,
                    y,
                    color: engine::contrast_ink(self.field()),
                    scale,
                    text,
                });
                y += 8 * scale as i32;
            }
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The frame gate compares against `TickCount() * 16.625`, so the 40 and
    /// 80 ms delays fire on the 3rd and 5th Mac tick (capture: 50.4 ms dive
    /// steps, 48.7 ms terminal fall steps).
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::path::Path;

    #[test]
    fn bungee_roulette_smoke() {
        let Ok(pack) = Pack::load(Path::new("../assets/bungee-roulette")) else {
            eprintln!("assets/bungee-roulette missing — skipping");
            return;
        };
        let Some(mut m) = make(pack) else {
            eprintln!("make() returned None (pack missing series) — skipping");
            return;
        };
        let mut ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut out: Vec<SpriteDraw> = Vec::new();
        let mut drew = false;
        let mut sounded = false;
        let mut pace = Pacer::new(&*m);
        for _ in 0..500 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if !ctx.sounds.is_empty() {
                sounded = true;
            }
            ctx.sounds.clear();
            out.clear();
            m.sprites(&mut out);
            if !out.is_empty() {
                drew = true;
            }
        }
        assert!(drew, "sprites() never produced output");
        assert!(sounded, "no sound ever fired");
    }

    #[test]
    fn fit_caption_short_stays_at_scale_2() {
        let lines = fit_caption("A short synthetic caption.", SCREEN_W - 16);
        assert_eq!(lines, vec![("A short synthetic caption.".to_string(), 2)]);
    }

    #[test]
    fn fit_caption_long_caption_drops_to_scale_1() {
        let text = "A synthetic caption, sixty-two characters long, for the tests!";
        assert_eq!(text.chars().count(), 62);
        let max_w = SCREEN_W - 16;
        assert!(engine::font::text_width(text, 2) > max_w);
        let lines = fit_caption(text, max_w);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].1, 1);
        assert!(engine::font::text_width(&lines[0].0, 1) <= max_w);
    }

    #[test]
    fn fit_caption_wraps_when_scale_1_still_overflows() {
        let text = "a".repeat(60) + " " + &"b".repeat(60);
        let max_w = SCREEN_W - 16;
        assert!(engine::font::text_width(&text, 1) > max_w);
        let lines = fit_caption(&text, max_w);
        assert_eq!(lines.len(), 2);
        for (line, scale) in &lines {
            assert!(engine::font::text_width(line, *scale) <= max_w);
        }
        let rejoined = format!("{} {}", lines[0].0, lines[1].0);
        assert_eq!(rejoined, text);
    }

    /// Build the module with a fixed seed and the given raw control values.
    fn rig(jumper: i32, jumps: i32, equip: i32) -> (BungeeRoulette, Ctx) {
        let pack = Pack::load(Path::new("../assets/bungee-roulette")).expect("pack");
        let mut m = build(pack).expect("build");
        m.set_control(0, jumper);
        m.set_control(1, jumps);
        m.set_control(2, equip);
        let ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        (m, ctx)
    }

    /// Bottom edge of whatever the sprite is drawing right now, in screen px.
    fn art_bottom(m: &BungeeRoulette, s: &Sprite) -> Option<i32> {
        let d = draw_of(&m.geom, &m.pack, s)?;
        Some(d.y + m.pack.image(&d.png).h as i32)
    }

    /// The frame gate is `TickCount()*16.625`: a 40 ms delay fires on the
    /// 3rd Mac tick, 80 on the 5th (capture: 465 dive steps at 50.39 ms).
    #[test]
    fn frame_gate_uses_the_mac_tick_grid() {
        let (m, _) = rig(1, 50, 50);
        assert_eq!(m.clock(), engine::TickClock::MacTick, "bungee gates on f4724");
        let pace = Pacer::new(&m);
        for from in 1..200 {
            assert_eq!(pace.ticks_for_at(from, 40, false), 3, "40 ms from tick {from}");
            assert_eq!(pace.ticks_for_at(from, 80, false), 5, "80 ms from tick {from}");
        }
        let p40 = pace.mean_period_ms(40, false, 60);
        let p80 = pace.mean_period_ms(80, false, 60);
        assert!((p40 - 49.875).abs() < 0.2, "dive period {p40} ms (capture 50.4)");
        assert!((p80 - 83.125).abs() < 0.2, "80 ms-phase period {p80} ms");
    }

    /// The dive falls 10 px per animation frame, one frame per 3 Mac ticks.
    #[test]
    fn dive_falls_ten_px_per_three_ticks() {
        let (mut m, mut ctx) = rig(1, 50, 85); // Man, always snaps
        let mut pace = Pacer::new(&m);
        let mut samples: Vec<(u64, i32)> = Vec::new();
        for _ in 0..400 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.jstate == JState::Dive && m.jpending.is_none() {
                samples.push((ctx.now_ms, m.jumper.y));
            } else if !samples.is_empty() {
                break;
            }
        }
        assert!(samples.len() > 6, "no dive observed");
        let mut at: Vec<u64> = Vec::new();
        for w in samples.windows(2) {
            let dy = w[1].1 - w[0].1;
            assert!(dy == 0 || dy == 10, "dive step was {dy} px, must be 0 or 10");
            if dy == 10 {
                at.push(w[1].0);
            }
        }
        assert!(at.len() > 4, "dive too short to time");
        let per = (at[at.len() - 1] - at[0]) as f64 / (at.len() - 1) as f64;
        assert!(
            (per - 49.875).abs() < 1.0,
            "dive advanced every {per:.1} ms; the Mac grid says 49.9, the capture 50.4"
        );
    }

    /// The fall enter links from the snap run's last frame to the fall run's
    /// first and adds 6, so the corpse starts where the body tore. In bank
    /// space the two frames share a bottom for every victim (capture
    /// `bungee-long.mp4` t≈5.5 s: the Half Man's head falls from the rip).
    #[test]
    fn snap_hand_off_keeps_the_corpse_where_the_body_tore() {
        for kind in 0..5i32 {
            let (mut m, mut ctx) = rig(kind + 1, 50, 85);
            let mut pace = Pacer::new(&m);
            let mut jumper_bottom_at_handoff: Option<i32> = None;
            let mut corpse_bottom: Option<i32> = None;
            for _ in 0..2000 {
                pace.advance(&mut ctx);
                let was_snap = m.jstate == JState::Snap && m.jpending.is_none();
                if was_snap {
                    jumper_bottom_at_handoff = art_bottom(&m, &m.jumper);
                }
                m.tick(&mut ctx);
                ctx.sounds.clear();
                if m.bstate == BState::Fall && m.bpending.is_none() {
                    corpse_bottom = art_bottom(&m, &m.body);
                    break;
                }
            }
            let j = jumper_bottom_at_handoff.expect("no snap seen");
            let c = corpse_bottom.expect("no hand-off seen");
            assert!(
                (c - j).abs() <= 12,
                "kind {kind}: corpse spawned at {c} but the body tore at {j}"
            );
        }
    }

    /// Switching a sprite to another run moves pos by the centre delta, so
    /// the pixels stay where the bank puts them.
    #[test]
    fn retarget_keeps_the_bank_space_placement() {
        let pack = Pack::load(Path::new("../assets/bungee-roulette")).expect("pack");
        let m = build(pack).expect("build");
        let mut s = Sprite::fresh(40);
        s.x = 300;
        s.y = 0;
        s.d8 = 72;
        s.da = 79;
        s.d6 = 79;
        let g79 = m.geom[&79];
        let before_top = draw_of(&m.geom, &m.pack, &s).unwrap().y - g79.by;
        retarget(&m.geom, &mut s, 115, 119, 80);
        let g115 = m.geom[&115];
        let after_top = draw_of(&m.geom, &m.pack, &s).unwrap().y - g115.by;
        assert_eq!(before_top, after_top, "dive->snap retarget moved the bank origin");
        assert_eq!(s.d6, 115);
        assert_eq!(s.delay, 80);
    }

    /// A transition costs no delay: the state that was entered runs on the
    /// very next tick (the SetState branch skips the reschedule tail).
    #[test]
    fn transitions_run_on_the_next_tick() {
        let (mut m, mut ctx) = rig(1, 50, 85);
        let mut pace = Pacer::new(&m);
        let mut tick_of_taut: Option<u64> = None;
        for _ in 0..600 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.jpending == Some(JState::Snap) && tick_of_taut.is_none() {
                tick_of_taut = Some(ctx.now_ms);
            }
            if m.jstate == JState::Snap && m.jpending.is_none() {
                let t0 = tick_of_taut.expect("snap entered without a pending tick");
                let dt = ctx.now_ms - t0;
                assert!(dt <= 17, "snap entered {dt} ms after the taut point, want next tick");
                return;
            }
        }
        panic!("no snap observed");
    }

    /// The Jumps bucket sets the wipe threshold; the exit-1 handler counts the
    /// jump and wipes past it, resetting to 1. Capture: 16 jumps apart.
    #[test]
    fn jumps_slider_bands_and_wipe_threshold() {
        for (slider, want) in [(0, 1), (19, 1), (20, 4), (40, 8), (60, 12), (80, 16), (100, 250)] {
            let (_, _, _, wipe) = derive_buckets(1, slider, 50);
            assert_eq!(wipe, want, "Jumps slider {slider}");
        }
        let (mut m, mut ctx) = rig(1, 85, 85);
        let mut pace = Pacer::new(&m);
        let mut jumps = 0;
        let mut wiped_at: Option<i32> = None;
        let mut prev_res = 0usize;
        for _ in 0..40_000 {
            pace.advance(&mut ctx);
            let was_arming = m.jpending == Some(JState::Dive);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.jpending == Some(JState::Dive) && !was_arming {
                jumps += 1;
            }
            if m.residue.len() < prev_res {
                wiped_at = Some(jumps);
                break;
            }
            prev_res = m.residue.len();
        }
        assert_eq!(wiped_at, Some(17), "wipe must land on the 17th jump");
        assert_eq!(m.jump_count, 1, "counter resets to 1, not 0");
    }

    /// The original's tagline goes to After Dark's message line, not the
    /// canvas.
    #[test]
    fn no_caption_is_drawn_on_the_canvas() {
        let (mut m, mut ctx) = rig(3, 50, 50);
        let mut pace = Pacer::new(&m);
        for _ in 0..200 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
        }
        assert!(m.message.is_some(), "the per-run STR# roll still happens");
        let mut out: Vec<TextDraw> = Vec::new();
        m.texts(&mut out);
        assert!(out.is_empty(), "the original draws no text on the canvas");
    }

    /// Every life ends by leaving its last drawn frame on the background:
    /// bounce, recoil and splat (capture t=114.9 s: 14 cords from the top,
    /// one debris bank along the shore).
    #[test]
    fn every_life_burns_its_final_frame_into_the_background() {
        let (mut m, mut ctx) = rig(6, 85, 65); // Random / Whole Bunch / So-So
        let mut pace = Pacer::new(&m);
        let mut saw_bounce_burn = false;
        let mut saw_recoil_burn = false;
        let mut saw_splat_burn = false;
        for _ in 0..4000 {
            pace.advance(&mut ctx);
            let (pj, pb, n) = (m.jstate, m.bstate, m.residue.len());
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.residue.len() > n {
                match (pj, pb) {
                    (JState::Bounce, _) => saw_bounce_burn = true,
                    (JState::Recoil, _) => saw_recoil_burn = true,
                    (_, BState::Splat) => saw_splat_burn = true,
                    _ => panic!("residue grew outside a life's final frame"),
                }
            }
        }
        assert!(saw_bounce_burn, "bounce never burned a hanging jumper");
        assert!(saw_recoil_burn, "recoil never burned a cord");
        assert!(saw_splat_burn, "splat never burned shore debris");
        assert!(m.residue.iter().any(|d| d.y <= 2), "no residue hanging from the top of the screen");
        assert!(
            m.residue.iter().any(|d| d.y + m.pack.image(&d.png).h as i32 > 420),
            "no residue on the shore"
        );
    }
}
