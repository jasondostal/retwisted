//! Phlegm Boy — ported function-by-function from the Ghidra decompile of
//! module CODE 129 "DynaPhlegm" (totally-twisted docs/decompiled/phlegm-boy/,
//! private) with the Library 4.0 sprite classes read alongside it. Listing
//! addresses are the raw resource_dasm ones (raw − 4).
//!
//! ## Objects (controller ctor `fn01` @0612)
//! - sound bank of 11 channels (`fn25` @4842): slots 3/6/7/9 load the SHARED
//!   `sndS` resources 30011/30010/30005/30013, the rest snd 128+i
//! - one RLESequence over art bank 9000 (`g0170`), SHARED by both sprites
//! - the BOY (`fn36`/`fn41`/`fn42` @0E5A/@0F30/@1000, 45 states) and the
//!   LOOGIE (`fn16`/`fn19`/`fn20` @3BCE/@3C68/@3CC2, 11 states)
//! - `g0178`: the gag id the boy posts, which is ALSO the loogie state it
//!   enters (loogie update 0: `SetState(g0178)`); `g017A/g017C` the launch
//!   point; `g0182..g0188` the world XRect (left, top, right, bottom)
//!
//! ## Library sprite semantics (see bungee_roulette.rs for the L135 proofs)
//! - `pos` (boy +0xD6/+0xD8, loogie +0xCE/+0xD0) is the CENTRE of the current
//!   frame; links are L135 `fn3DDC` (`engine::l135::link`): the rect
//!   mid-point difference in bank space, rounded `(flip + l + r) >> 1` and
//!   x-negated under the SEQUENCE's flip bit (every call passes sprite
//!   flag 0). The plain `centre(to) − centre(from)` it replaced (superseded
//!   2026-09-29) was 1 px off per mirrored step on odd-parity width pairs.
//! - `fn45` @348C / `fn21` @469C: loop advance that reports the wrap (delta
//!   not applied on the wrap step, d6 back to first); `fn46` @3538 applies
//!   the wrap delta too. States 3/13 and loogie 7 use an inline stepper that
//!   wraps at `d6 >= last` (the run's last frame is never shown).
//! - every enter is `SetRun(N)`: `g0274 = N; [link(old last → N)]; g0272 =
//!   N − 1 + len(N); d6 = N − 1` — the first advance shows N with no delta.
//! - `SetState` fires the exit at once; enter+update run next frame; a
//!   transitioning update skips the draw/reschedule tail.
//! - the facing coin (`fn47` @35E0) toggles the shared sequence's flip slot
//!   (vtbl `+0x0C` = L133 `fn0702`, `+0x3C ^= 1`): every frame is mirrored
//!   while `g0270 == 2`, and `fn3DDC` reads the same bit.
//!
//! ## Frame driver (`fn04` @0C64)
//! The boy runs when `+0xD2 <= now` and reschedules `now + 100`; the loogie
//! runs only in the frames the boy ran and only while `g0178 != 0`. A
//! keyboard-state change (`Required1.fn41DC`, here the caps-lock toggle)
//! opens the +0xCC/+0xCA window fn49 and state 8 read.
//!
//! ## What stays approximate
//! - Message line: `STR# 128 + rand()%11` at construction goes to After
//!   Dark's own message line; nothing is drawn (`SHOW_QUIP_CAPTIONS`).
//! - The flash (`+0x2C(1)/(0)`) at a loogie run's end leaves the last drawn
//!   frame on the background (the snot); the idle-deadline wipe clears it.
//! - Sound-bank busy time is the packed WAV length.

use std::collections::HashMap;

use engine::l135::{self, FrameBox, LinkModel};
use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TextDraw, TickClock, SCREEN_H,
    SCREEN_W,
};

const BASE: u32 = 9000;

// cue ids 0x80..0x8A as the module passes them to fn28
const SND_HAWK_LOOG_1: u32 = 128;
const SND_NEW_BOUNCE: u32 = 129;
const SND_NEW_SPLATTER: u32 = 130;
const SND_PIG_SNORT: u32 = 131; // slot 3 loads shared 30011 Burst
const SND_HAWK_LOOG_5: u32 = 132;
const SND_THOUGHTSNOT_BURP: u32 = 133;
const SND_SNOT_FLICK: u32 = 134; // slot 6 loads shared 30010 Armpit
const SND_SNORT_A: u32 = 135; // slot 7 loads shared 30005 drip_slither
const SND_NEW_COPTER: u32 = 136;
const SND_SNORT_B: u32 = 137; // slot 9 loads shared 30013 Slither2
const SND_HELI_SLOSH: u32 = 138;

/// `fn25` @4842: what each bank slot actually loads.
fn snd_resource(id: u32) -> u32 {
    match id {
        SND_PIG_SNORT => 30011,
        SND_SNOT_FLICK => 30010,
        SND_SNORT_A => 30005,
        SND_SNORT_B => 30013,
        _ => id,
    }
}

/// One packed frame's bounds in bank space: the compound rect (`bx,by,w,h`)
/// shifted by the frame's own OFst offset (`dx,dy`) — L135 `fn3A70`
/// (sequence `+0x18`) copies the OFtb rect and offsets it by the OFst
/// record's two words.
#[derive(Clone, Copy)]
struct Geom {
    bx: i32,
    by: i32,
    w: i32,
    h: i32,
    dx: i32,
    dy: i32,
}


/// A sprite as the module drives it: `x,y` is the centre of frame `d6`.
#[derive(Clone, Copy, Default)]
struct Ent {
    x: i32,
    y: i32,
    d6: i32,
}

struct Boy {
    e: Ent,
    first: i32,    // g0274
    last: i32,     // g0272
    dc: i32,       // +0xDC: frame offset roll / bounce cue offset
    w: i32,        // +0xDE: GetBounds width (fn47: frame 30, fn48: the look run)
    h: i32,        // +0xE0
    fid_rate: i32, // +0xE2
    fid_count: i32, // +0xE4
    ea: i32,       // +0xEA: frame the hold parks on
    ec: i32,       // +0xEC: last fidget pick
    ee: i32,       // +0xEE: the state the hold releases into
    f0: i64,       // +0xF0: hold length ms
    f2: u64,       // +0xF2: idle (squeegie) deadline
    f6: u64,       // +0xF6: hold / wince deadline
    ca: i32,       // +0xCA: key-window counter
    cc: bool,      // +0xCC: key-window flag
    next_due: u64, // +0xD2
    state: i32,
    pending: bool,        // state sub-object +0xE
    request: Option<i32>, // state sub-object +0x32 (fn5382 / fn5760)
}

struct Loogie {
    e: Ent,
    first: i32, // g034C
    last: i32,  // g034A
    rate: i32,  // +0xD4
    state: i32,
    pending: bool,
}

pub struct PhlegmBoy {
    pack: Pack,
    geom: HashMap<u32, Geom>,
    behavior: i32, // sVal 1000 raw
    mess: i32,     // sVal 1001 raw
    cached_beh: i32, // controller +0x18 = Behavior/15
    cached_mess: i32, // controller +0x1A = Mess/16
    g026a: i32,    // Mess/16 + 1
    g026c: i32,    // Behavior/15
    g026e: i32,    // facing 1 left / 2 right / 3 up / 4 down
    g0270: i32,    // side coin 1 / 2 (2 = art mirrored)
    g0268: i32,
    g0178: i32,    // gag id / loogie entry state
    g017a: (i32, i32), // launch point
    clock: u64,    // g017E
    key_deadline: u64, // controller +0x14
    key_state: bool,   // Required1.g0134
    boy: Boy,
    loo: Loogie,
    bank_busy_until: u64,
    residue: Vec<SpriteDraw>,
    /// What each sprite's last draw tail put on the screen. An enter never
    /// draws and a transitioning update skips the tail, so the previous
    /// image stays up until the next tail (the boy's d6 sits one before the
    /// run after every SetRun — drawing it would be a blank frame).
    boy_img: Option<SpriteDraw>,
    loo_img: Option<SpriteDraw>,
    need_ctor_rolls: bool,
    message: Option<String>,
    restart_pending: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

fn build(pack: Pack) -> Option<PhlegmBoy> {
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
    let mut m = PhlegmBoy {
        pack,
        geom,
        behavior: 50,
        mess: 50,
        cached_beh: 0,
        cached_mess: 0,
        g026a: 1,
        g026c: 0,
        g026e: 1,
        g0270: 1,
        g0268: 1,
        g0178: 0,
        g017a: (0, 0),
        clock: 0,
        key_deadline: 0,
        key_state: false,
        boy: Boy {
            e: Ent::default(),
            first: 1,
            last: 1,
            dc: 0,
            w: 0,
            h: 0,
            fid_rate: 1,
            fid_count: 0,
            ea: 0,
            ec: 0,
            ee: 0,
            f0: 800,
            f2: 0,
            f6: 0,
            ca: 0,
            cc: false,
            next_due: 0,
            state: 0,
            pending: true,
            request: None,
        },
        loo: Loogie { e: Ent::default(), first: 1, last: 1, rate: 0, state: 0, pending: true },
        bank_busy_until: 0,
        residue: Vec::new(),
        boy_img: None,
        loo_img: None,
        need_ctor_rolls: true,
        message: None,
        restart_pending: false,
    };
    m.reset_state();
    Some(m)
}

/// The draw canvas bounds the controller ctor reads into `g018C..g0192`
/// (`SetDrawOrigin()`'s canvas, vtbl `+0x60` @06B4): a Library XRect,
/// `(left, top, right, bottom)` — the order fn47's own GetBounds
/// (`w = r − l` @3656) and the ctor's width/height tests read it in. The
/// saver's canvas is the whole 640×480 screen.
const CANVAS: (i32, i32, i32, i32) = (0, 0, SCREEN_W, SCREEN_H);

/// Controller ctor @06C6–@072E: `g0182..g0188` copy the canvas rect, then
/// `right = left + 400` when it is narrower than 400 (@0702) and `bottom =
/// top + 200` when shorter than 200 (@0722). At 640×480 neither clamp bites, so
/// the world rect IS the screen — measured: the golden's two right-side
/// walk-ins stop with the stripe centre at x 518/519, inside the port's
/// [484, 575] range for right = 640 (right = 480 would stop them < 420).
const fn world_rect(c: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let (l, t, mut r, mut b) = c;
    if r - l < 400 {
        r = l + 400;
    }
    if b - t < 200 {
        b = t + 200;
    }
    (l, t, r, b)
}

const WORLD: (i32, i32, i32, i32) = world_rect(CANVAS);
const LEFT: i32 = WORLD.0; // g0182
const TOP: i32 = WORLD.1; // g0184
const RIGHT: i32 = WORLD.2; // g0186
const BOTTOM: i32 = WORLD.3; // g0188

impl PhlegmBoy {
    // ---- bank / sequence helpers ------------------------------------------

    fn frame_box(&self, f: i32) -> Option<FrameBox> {
        self.geom
            .get(&(f as u32))
            .map(|g| FrameBox { bx: g.bx, by: g.by, w: g.w, h: g.h, dx: g.dx, dy: g.dy })
    }

    /// Sequence `+0x78` = L135 `fn3DDC` (the RLESequence vtable,
    /// `L135 g015E`): every call site in this module (fn45/fn46/fn21/fn22,
    /// fn51 @3B0A, the inline steppers of states 3/13 and loogie 7) passes
    /// sprite flag 0, so the flip `fn3DDC` rounds and negates by is the
    /// SEQUENCE's own `+0x3C` bit 0 — the bit the facing coin toggles
    /// (vtbl `+0x0C` = L133 `fn0702`, which XORs `+0x3C` of every entry in
    /// the `+0x5E` list, entry 0 being the sequence itself). Hence
    /// `seq_flip = (g0270 == 2)` with the mid-points rounded
    /// `(flip + l + r) >> 1`.
    fn link(&self, from: i32, to: i32) -> (i32, i32) {
        let model = LinkModel { seq_flip: self.g0270 == 2, flip_rounding: true };
        match (self.frame_box(from), self.frame_box(to)) {
            (Some(a), Some(b)) => l135::link(&a, &b, false, model),
            _ => (0, 0),
        }
    }

    /// Sequence `+0x18`: a frame's (w, h).
    fn bounds(&self, f: i32) -> (i32, i32) {
        self.geom.get(&(f as u32)).map(|g| (g.w, g.h)).unwrap_or((0, 0))
    }

    /// `first − 1 + len(first)` (sequence `+0x90` counts the run).
    fn last_of(&self, first: i32) -> i32 {
        first + self.pack.seq_len(BASE, first as u32).max(1) as i32 - 1
    }

    /// Controller (re)construction: `fn39` @0EF4 and `fn41` @0F30 / `fn19`
    /// @3C68. The rand() calls run in `tick` (they need the ctx).
    fn reset_state(&mut self) {
        self.g026c = self.behavior / 15;
        self.g026a = self.mess / 16 + 1;
        self.cached_beh = self.behavior / 15;
        self.cached_mess = self.mess / 16;
        self.g0270 = 1;
        self.g026e = 1;
        self.g0268 = 1;
        self.g0178 = 1; // @0998, right after the loogie is built
        self.g017a = (0, 0);
        self.bank_busy_until = 0;
        self.residue.clear();
        self.boy_img = None;
        self.loo_img = None;
        let b = &mut self.boy;
        b.e = Ent::default();
        b.first = 1;
        b.last = 1;
        b.dc = 0;
        b.w = 0;
        b.h = 0;
        b.fid_count = 0;
        b.ec = 0;
        b.cc = false;
        b.ca = 0;
        b.f0 = 800;
        b.f6 = 0;
        b.ee = 0;
        b.ea = 0;
        b.state = 0;
        b.pending = true;
        b.request = None;
        b.next_due = 0;
        let l = &mut self.loo;
        l.e = Ent { x: -100, y: -100, d6: 0 };
        l.first = 1;
        l.last = 1;
        l.rate = 0;
        l.state = 0;
        l.pending = true;
        self.need_ctor_rolls = true;
    }

    /// `fn28` @4B2C: `onoff == 0` cues wait for the channel, the rest
    /// pre-empt; either way the channel is busy for the sample's length.
    fn snd(&mut self, ctx: &mut Ctx, onoff: i32, id: u32) {
        if onoff == 0 && self.bank_busy_until > self.clock {
            return;
        }
        let res = snd_resource(id);
        let len = self.pack.sound_ms(res);
        if len == 0 {
            return;
        }
        self.bank_busy_until = self.clock + len;
        ctx.sounds.push(res);
    }

    /// `rand() % 2 == 0 ? 0x89 : 0x87`, onoff 1 — enters 2/4 and tick 3.
    fn snort_coin(&mut self, ctx: &mut Ctx) {
        let id = if ctx.rng15.below(2) == 0 { SND_SNORT_B } else { SND_SNORT_A };
        self.snd(ctx, 1, id);
    }

    // ---- boy run helpers ------------------------------------------------------

    /// `g0274 = N; g0272 = N − 1 + len; d6 = N − 1`.
    fn boy_run(&mut self, first: i32) {
        self.boy.first = first;
        self.boy.last = self.last_of(first);
        self.boy.e.d6 = first - 1;
    }

    /// The linked form: `fn51(this, to = N, from = old g0272)` first.
    fn boy_run_linked(&mut self, first: i32) {
        let (dx, dy) = self.link(self.boy.last, first);
        self.boy.e.x += dx;
        self.boy.e.y += dy;
        self.boy_run(first);
    }

    fn step_delta(&self, e: &Ent, first: i32, last: i32) -> (i32, i32) {
        if e.d6 < first {
            (0, 0)
        } else if e.d6 < last {
            self.link(e.d6, e.d6 + 1)
        } else {
            self.link(last, first)
        }
    }

    /// `fn45` @348C (boy) / `fn21` @469C (loogie): true on the wrap, when
    /// no delta is applied and d6 goes back to first.
    fn adv45(&self, e: &mut Ent, first: i32, last: i32) -> bool {
        let (dx, dy) = self.step_delta(e, first, last);
        e.d6 += 1;
        if e.d6 > last {
            e.d6 = first;
            return true;
        }
        e.x += dx;
        e.y += dy;
        false
    }

    /// `fn46` @3538: the same, but the wrap delta is applied.
    fn adv46(&self, e: &mut Ent, first: i32, last: i32) -> bool {
        let (dx, dy) = self.step_delta(e, first, last);
        e.x += dx;
        e.y += dy;
        e.d6 += 1;
        if e.d6 > last {
            e.d6 = first;
            return true;
        }
        false
    }

    /// The inline stepper of states 3/13 (@15FA, @1D8A) up to the wrap test:
    /// link to the next frame, advance; true when `d6 >= last`.
    fn inline_step(&self, e: &mut Ent, first: i32, last: i32) -> bool {
        if e.d6 >= first {
            let (dx, dy) = self.link(e.d6, e.d6 + 1);
            e.x += dx;
            e.y += dy;
        }
        e.d6 += 1;
        e.d6 >= last
    }

    /// `x + w/2 < right && left < x − w/2` — the boy fully inside.
    fn boy_inside_x(&self) -> bool {
        let b = &self.boy;
        b.e.x + b.w / 2 < RIGHT && LEFT < b.e.x - b.w / 2
    }

    /// `fn50` @3A94 (the boy's `+0x7C`): at or past the edge he faces.
    fn fn50(&self) -> bool {
        let b = &self.boy;
        if b.state == 0 {
            return true;
        }
        match self.g026e {
            1 => b.e.x <= LEFT,
            2 => RIGHT <= b.e.x,
            3 => b.e.y <= TOP,
            4 => BOTTOM <= b.e.y,
            _ => false,
        }
    }

    /// `fn23` @4788: the loogie is off the screen horizontally.
    fn fn23(&self) -> bool {
        let l = &self.loo;
        if l.state == 0 {
            return true;
        }
        !(LEFT < l.e.x && l.e.x < RIGHT)
    }

    /// The facing coin's flip: the shared sequence's `+0x0C` slot mirrors
    /// every frame, and `g0270` follows.
    fn set_side(&mut self, side: i32) {
        self.g0270 = side;
    }

    /// `fn47` @35E0 — the idle picker: blink (13) or spit (208), with the
    /// reposition and the facing coin, then a 23-hold request.
    fn fn47(&mut self, ctx: &mut Ctx) {
        let b_first;
        if ctx.rng15.below(2) == 0 {
            b_first = 0xD0; // 208, the pour
        } else {
            b_first = 0xD; // 13, the pose
        }
        self.boy.first = b_first;
        self.boy.last = self.last_of(b_first);
        let (w, h) = self.bounds(b_first);
        self.boy.w = w;
        self.boy.h = h;
        if b_first == 0xD0 {
            // a random column, parked on the world rect's top edge:
            // `rand() % (g0186 − w) + w/2` — right, not the width (@3692)
            let span = (RIGHT - w).max(1);
            self.boy.e.x = ctx.rng15.below(span as u16) as i32 + w / 2;
            self.boy.e.y = TOP - h / 2 - 1;
            self.g026e = self.g0270;
            if ctx.rng15.below(2) == 0 {
                if self.g0270 == 1 {
                    self.g026e = 2;
                    self.set_side(2);
                }
            } else if self.g0270 == 2 {
                self.g026e = 1;
                self.set_side(1);
            }
            self.boy.ee = 8;
            self.boy.e.d6 = b_first;
        } else {
            // a random row, off one side of the screen facing in
            let span = (BOTTOM - h).max(1);
            self.boy.e.y = ctx.rng15.below(span as u16) as i32 + h / 2;
            if ctx.rng15.below(2) == 0 {
                let base = LEFT - w / 2;
                self.boy.e.x = base - ctx.rng15.below(100) as i32;
                if self.g0270 == 1 {
                    self.set_side(2);
                }
            } else {
                let base = RIGHT + w / 2;
                self.boy.e.x = ctx.rng15.below(100) as i32 + base;
                if self.g0270 == 2 {
                    self.set_side(1);
                }
            }
            self.g026e = self.g0270;
            self.boy.ee = 3;
            self.boy.e.d6 = b_first - 1;
        }
        // common tail @3838: metrics of frame 30, then the hold request
        let (w, h) = self.bounds(0x1E);
        self.boy.w = w;
        self.boy.h = h;
        self.boy.ea = self.boy.first;
        self.boy.f0 = 800;
        self.boy.request = Some(0x17);
    }

    /// `fn48` @3890 — the look picker.
    fn fn48(&mut self, ctx: &mut Ctx) {
        let pick = if ctx.rng15.below(2) == 0 {
            if ctx.rng15.below(2) == 0 { 0x10E } else { 0xFD }
        } else if ctx.rng15.below(2) == 0 {
            0xEF
        } else {
            0xEB
        };
        self.boy_run_linked(pick);
        let (w, h) = self.bounds(pick);
        self.boy.w = w;
        self.boy.h = h;
    }

    /// `fn49` @391E — the fidget picker: writes +0xEE.
    fn fn49(&mut self, ctx: &mut Ctx) {
        let count = self.boy.fid_count;
        self.boy.fid_count += 1;
        if self.boy.fid_rate < count {
            self.boy.ee = if ctx.rng15.below(3) == 0 { 7 } else { 6 };
            self.boy.fid_count = 0;
            self.boy.fid_rate = if self.g026c < 4 {
                ctx.rng15.below(2) as i32 + 1
            } else {
                ctx.rng15.below(4) as i32 + 1
            };
            return;
        }
        loop {
            self.boy.ee = match ctx.rng15.below(8) {
                0 => 0x18,
                1 => 0x1F,
                2 => 0x20,
                3 => 0x21,
                4 => 0x25,
                5 => 0x29,
                6 => 0x2A,
                _ => 0x2B,
            };
            if self.boy.ec != self.boy.ee {
                break;
            }
        }
        if self.boy.cc {
            self.boy.ee = 0x29;
            self.boy.cc = false;
        }
        self.boy.ec = self.boy.ee;
        // GetControlValue(3) != 0 → 0x21: no third control ships
    }

    fn post(&mut self, id: i32) {
        self.g017a = (self.boy.e.x, self.boy.e.y);
        self.g0178 = id;
    }

    /// `fn49; +0xEA = g0272; +0xF0 = 800; SetState(23)` — the common
    /// completion of a dozen states.
    fn hold_after_fidget(&mut self, ctx: &mut Ctx) {
        self.fn49(ctx);
        self.boy.ea = self.boy.last;
        self.boy.f0 = 800;
        self.set_bstate(0x17);
    }

    fn hold(&mut self, ea: i32, ee: Option<i32>, f0: i64) {
        self.boy.ea = ea;
        if let Some(ee) = ee {
            self.boy.ee = ee;
        }
        self.boy.f0 = f0;
        self.set_bstate(0x17);
    }

    fn set_bstate(&mut self, s: i32) {
        self.boy.state = s;
        self.boy.pending = true;
    }

    fn set_lstate(&mut self, s: i32) {
        self.loo.state = s;
        self.loo.pending = true;
    }

    /// `fn42` @1000 enter messages (0x8000 | state).
    fn boy_enter(&mut self, ctx: &mut Ctx) {
        let s = self.boy.state;
        match s {
            0 => {}
            1 => {}
            2 => {
                self.boy_run(7);
                self.snort_coin(ctx);
            }
            3 => {
                self.boy_run(0xD);
                let span = (self.boy.last - self.boy.first).max(1);
                self.boy.dc = ctx.rng15.below(span as u16) as i32;
                self.g026e = self.g0270;
            }
            4 => {
                self.boy_run(0x1E);
                self.snort_coin(ctx);
            }
            5 => self.boy_run(0xA1),
            6 => self.boy_run_linked(0xE4),
            7 => self.boy_run_linked(0xAD),
            8 => {
                self.boy_run(0xD0);
                self.boy.e.y += 1;
                let span = (self.boy.last - self.boy.first).max(1);
                self.boy.dc = ctx.rng15.below(span as u16) as i32;
                self.g026e = 4;
            }
            9 => {
                self.boy_run(0xC1);
                self.g026e = self.g0270;
                self.post(3);
            }
            10 => {
                self.boy_run(0x289);
                self.snd(ctx, 1, SND_THOUGHTSNOT_BURP);
            }
            11 => self.boy_run(0x296),
            12 => {
                self.boy_run_linked(0x25);
                self.boy.dc = 0;
            }
            13 => {
                if self.g0268 != 0 {
                    self.boy_run_linked(0x46);
                } else {
                    self.boy_run(0x46);
                }
                self.g0268 = 1;
                self.g026e = self.g0270;
            }
            14 => {
                self.boy_run_linked(0x5E);
                self.g026e = 4;
            }
            15 => {
                self.boy_run_linked(0x6C);
                self.g026e = self.g0270;
            }
            16 => {
                self.boy_run_linked(0x77);
                self.g026e = 3;
            }
            17 => {
                self.boy_run_linked(0x85);
                self.g026e = self.g0270;
            }
            18 => {
                self.boy_run_linked(0x7F);
                self.g026e = 3;
            }
            19 => {
                self.boy_run_linked(0x66);
                self.g026e = 4;
            }
            20 => {
                self.boy_run(0x185);
                self.g026e = 4;
            }
            21 => self.boy_run_linked(0x18A),
            22 => {
                self.boy_run_linked(400);
                self.snd(ctx, 1, SND_NEW_SPLATTER);
            }
            23 => {
                self.boy.e.d6 = self.boy.ea;
                self.boy.f6 = self.clock.wrapping_add(self.boy.f0 as u64);
            }
            24 => self.fn48(ctx),
            25 => self.boy_run_linked(0x1F2),
            26 => {
                self.boy_run_linked(0x1F6);
                self.snd(ctx, 1, SND_THOUGHTSNOT_BURP);
            }
            27 => self.boy_run_linked(0x1EC),
            28 => self.boy_run_linked(0x205),
            29 => self.boy_run(0xA9),
            30 => self.boy_run(1),
            31 => self.boy_run_linked(0xF3),
            32 => self.boy_run_linked(0x174),
            33 => self.boy_run_linked(0x132),
            34 => {
                self.boy_run_linked(0x13C);
                self.boy.f6 = self.clock + 1500;
            }
            35 => self.boy_run_linked(0x143),
            36 => self.boy_run_linked(0x148),
            37 => self.boy_run_linked(0x199),
            38 => {
                self.boy_run_linked(0x1A0);
                self.boy.f6 = self.clock + 1500;
            }
            39 => self.boy_run_linked(0x1A6),
            40 => {
                if ctx.rng15.below(2) == 0 {
                    self.boy_run_linked(0x1BE);
                    self.boy.dc = 1;
                } else {
                    self.boy_run_linked(0x1AD);
                    self.boy.dc = 4;
                }
            }
            41 => self.boy_run_linked(0x127),
            42 => self.boy_run_linked(0x1D3),
            43 => self.boy_run_linked(0x114),
            44 => self.boy_run_linked(0x20B),
            _ => {}
        }
    }

    /// `fn42` update messages. Returns true when the state changed (the
    /// handler returned before the draw/reschedule tail).
    fn boy_update(&mut self, ctx: &mut Ctx) -> bool {
        let s = self.boy.state;
        let (first, last) = (self.boy.first, self.boy.last);
        macro_rules! adv {
            () => {{
                let mut e = self.boy.e;
                let done = self.adv45(&mut e, first, last);
                self.boy.e = e;
                done
            }};
        }
        macro_rules! adv46 {
            () => {{
                let mut e = self.boy.e;
                let done = self.adv46(&mut e, first, last);
                self.boy.e = e;
                done
            }};
        }
        match s {
            0 => {
                self.set_bstate(1);
                true
            }
            1 => {
                // the squeegie: past the idle deadline the canvas is wiped
                if self.boy.f2 < self.clock {
                    self.residue.clear();
                    self.boy.f2 = self.clock + self.g026a as u64 * 300_000;
                }
                self.fn47(ctx);
                if let Some(r) = self.boy.request.take() {
                    self.set_bstate(r);
                    return true;
                }
                false
            }
            2 => {
                if adv!() {
                    self.set_bstate(3);
                    return true;
                }
                false
            }
            3 => {
                if self.fn50() {
                    self.set_bstate(0);
                    return true;
                }
                if self.boy.e.d6 == first + self.boy.dc {
                    self.snort_coin(ctx);
                    let b = &self.boy;
                    let lx = if self.g0270 == 1 { b.e.x - b.w / 2 } else { b.e.x + b.w / 2 };
                    let ly = b.e.y + b.h / 2 - ctx.rng15.below(30) as i32 - 3;
                    self.g017a = (lx, ly);
                    self.g0178 = 1;
                }
                let mut e = self.boy.e;
                let wrapped = self.inline_step(&mut e, first, last);
                self.boy.e = e;
                if wrapped {
                    if self.boy_inside_x() {
                        self.set_bstate(4);
                        return true;
                    }
                    self.boy.e.d6 = first;
                }
                false
            }
            4 => {
                if adv!() {
                    self.set_bstate(5);
                    return true;
                }
                false
            }
            5 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                false
            }
            6 => {
                if adv!() {
                    let r = ctx.rng15.below(100) as i32;
                    if r <= self.behavior {
                        self.set_bstate(0x2C);
                    } else if ctx.rng15.below(2) != 0 {
                        self.set_bstate(2);
                    } else {
                        self.set_bstate(0xC);
                    }
                    return true;
                }
                false
            }
            7 => {
                if adv!() {
                    self.set_bstate(8);
                    return true;
                }
                false
            }
            8 => {
                if adv!() {
                    if self.fn50() {
                        self.set_bstate(0);
                        return true;
                    }
                    self.boy.e.y += 2;
                    self.set_bstate(if 5 < self.boy.ca { 10 } else { 9 });
                    return true;
                }
                let d6 = self.boy.e.d6;
                if d6 == first + 2 {
                    self.snd(ctx, 1, SND_SNORT_B);
                }
                if d6 == last - 5 {
                    self.snd(ctx, 1, SND_SNORT_A);
                }
                if d6 == first + self.boy.dc {
                    self.post(1);
                }
                false
            }
            9 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                false
            }
            10 => {
                if adv!() {
                    self.hold(0x296, Some(0xB), 500);
                    return true;
                }
                false
            }
            11 => {
                if adv!() {
                    self.hold(0xBD, Some(9), 500);
                    return true;
                }
                false
            }
            12 => {
                let d6 = self.boy.e.d6;
                if d6 == first + 0xB || d6 == first + 0x16 {
                    self.snd(ctx, 1, SND_HELI_SLOSH);
                } else {
                    self.snd(ctx, 0, SND_NEW_COPTER);
                }
                if adv!() {
                    self.post(2);
                    self.set_bstate(0xD);
                    return true;
                }
                false
            }
            13 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if self.fn50() {
                    self.set_bstate(0);
                    return true;
                }
                let mut e = self.boy.e;
                let wrapped = self.inline_step(&mut e, first, last);
                self.boy.e = e;
                if wrapped {
                    if self.boy_inside_x() {
                        if ctx.rng15.below(2) == 0 {
                            let ns = if ctx.rng15.below(2) != 0 { 0xE } else { 0x10 };
                            self.set_bstate(ns);
                            return true;
                        }
                        if ctx.rng15.below(2) != 0 {
                            self.set_bstate(0x14);
                            return true;
                        }
                    }
                    self.boy.e.d6 = first;
                }
                false
            }
            14 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if adv!() {
                    self.set_bstate(0x13);
                    return true;
                }
                false
            }
            15 | 17 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if adv!() {
                    if ctx.rng15.below(2) == 0 && self.boy_inside_x() {
                        self.set_bstate(0x14);
                        return true;
                    }
                    self.g0268 = 0;
                    self.boy.e.x += if self.g0270 == 1 { -4 } else { 4 };
                    self.set_bstate(0xD);
                    return true;
                }
                false
            }
            16 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if adv!() {
                    self.set_bstate(0x12);
                    return true;
                }
                false
            }
            18 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if self.fn50() {
                    self.set_bstate(0);
                    return true;
                }
                if adv!() {
                    let b = &self.boy;
                    if ctx.rng15.below(2) == 0 && TOP < b.e.y - b.h / 2 {
                        self.set_bstate(0x11);
                        return true;
                    }
                    self.boy.e.y -= 0xD;
                }
                false
            }
            19 => {
                self.snd(ctx, 0, SND_NEW_COPTER);
                if self.fn50() {
                    self.set_bstate(0);
                    return true;
                }
                if adv!() {
                    let b = &self.boy;
                    if ctx.rng15.below(2) == 0 && b.e.y + b.h / 2 < BOTTOM {
                        self.set_bstate(0xF);
                        return true;
                    }
                    self.boy.e.y += 0xD;
                }
                false
            }
            20 => {
                if adv!() {
                    self.set_bstate(0x15);
                    return true;
                }
                false
            }
            21 => {
                if adv!() {
                    self.set_bstate(0x16);
                    return true;
                }
                false
            }
            22 => {
                if adv!() {
                    if self.fn50() {
                        self.set_bstate(0);
                        return true;
                    }
                    self.hold(0xBD, Some(9), 800);
                    return true;
                }
                false
            }
            23 => {
                if self.boy.f6 < self.clock {
                    if self.boy.ee == 0x17 {
                        self.fn49(ctx);
                    }
                    let ns = self.boy.ee;
                    self.set_bstate(ns);
                    return true;
                }
                false
            }
            24 => {
                if adv!() {
                    self.hold(last, Some(0x17), 800);
                    return true;
                }
                false
            }
            25 | 26 => {
                if adv!() {
                    self.hold(last, Some(0x1C), 800);
                    return true;
                }
                false
            }
            27 => {
                if adv!() {
                    let ee = if ctx.rng15.below(2) == 0 { 0x1A } else { 0x19 };
                    self.hold(last, Some(ee), 800);
                    return true;
                }
                false
            }
            28 => {
                if adv!() {
                    if ctx.rng15.below(2) == 0 && self.g0178 == 0 {
                        self.set_bstate(0xC);
                    } else {
                        self.set_bstate(2);
                    }
                    return true;
                }
                false
            }
            29 | 30 => {
                if adv!() {
                    let ns = self.boy.ee;
                    self.set_bstate(ns);
                    return true;
                }
                false
            }
            31 => {
                if self.boy.e.d6 == first {
                    self.snd(ctx, 1, SND_THOUGHTSNOT_BURP);
                }
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                false
            }
            32 => {
                if self.boy.e.d6 == first {
                    self.snd(ctx, 1, SND_HAWK_LOOG_5);
                }
                if adv!() {
                    self.fn49(ctx);
                    self.hold(first, None, 800);
                    return true;
                }
                false
            }
            33 => {
                if adv!() {
                    let ns = if ctx.rng15.below(2) != 0 { 0x24 } else { 0x22 };
                    self.set_bstate(ns);
                    return true;
                }
                if self.boy.e.d6 == first + 5 {
                    self.snd(ctx, 1, SND_SNOT_FLICK);
                }
                false
            }
            34 => {
                if self.boy.e.d6 == first + 1 {
                    self.snd(ctx, 1, SND_SNOT_FLICK);
                }
                if adv46!() && self.boy.f6 < self.clock {
                    let (dx, dy) = self.link(first, last);
                    self.boy.e.x += dx;
                    self.boy.e.y += dy;
                    self.set_bstate(0x23);
                    return true;
                }
                false
            }
            35 => {
                if adv!() {
                    self.fn49(ctx);
                    let ns = self.boy.ee;
                    self.set_bstate(ns);
                    return true;
                }
                false
            }
            36 => {
                if adv!() {
                    self.set_bstate(0x23);
                    return true;
                }
                let d6 = self.boy.e.d6;
                if d6 == first + 0xE {
                    self.snd(ctx, 1, SND_PIG_SNORT);
                }
                let k = d6 - first;
                if (k < 0xC && (k - 2).rem_euclid(3) == 0) || (0x18 < k && (k - 1).rem_euclid(3) == 0)
                {
                    self.snd(ctx, 1, SND_SNOT_FLICK);
                }
                false
            }
            37 => {
                if adv!() {
                    self.set_bstate(0x26);
                    return true;
                }
                false
            }
            38 => {
                if adv46!() && self.boy.f6 < self.clock {
                    let r = ctx.rng15.below(100) as i32;
                    let ns = if self.behavior < r { 0x27 } else { 0x28 };
                    self.set_bstate(ns);
                    return true;
                }
                false
            }
            39 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                false
            }
            40 => {
                if self.boy.e.d6 == last - self.boy.dc {
                    self.snd(ctx, 1, SND_NEW_BOUNCE);
                }
                if adv!() {
                    self.post(8);
                    self.hold_after_fidget(ctx);
                    return true;
                }
                false
            }
            41 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                if self.boy.e.d6 == first + 3 {
                    self.snd(ctx, 1, SND_HAWK_LOOG_1);
                    self.post(9);
                }
                false
            }
            42 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                let d6 = self.boy.e.d6;
                if d6 == first + 4 {
                    self.snd(ctx, 1, SND_HAWK_LOOG_1);
                }
                if d6 == first + 0xF {
                    self.post(10);
                }
                false
            }
            43 => {
                if adv!() {
                    self.hold_after_fidget(ctx);
                    return true;
                }
                if self.boy.e.d6 == first + 0xC {
                    self.snd(ctx, 1, SND_PIG_SNORT);
                }
                false
            }
            44 => {
                if adv!() {
                    self.hold(last, Some(0x1B), 800);
                    return true;
                }
                if self.boy.e.d6 == first + 5 {
                    self.post(5);
                    self.snd(ctx, 1, SND_HAWK_LOOG_1);
                }
                false
            }
            _ => {
                self.set_bstate(0);
                true
            }
        }
    }

    // ---- loogie ---------------------------------------------------------------

    fn loo_run(&mut self, first: i32) {
        self.loo.first = first;
        self.loo.last = self.last_of(first);
        self.loo.e.d6 = first - 1;
    }

    /// `fn22(this, to, from)`: `pos += centre(to) − centre(from)`.
    fn loo_link(&mut self, from: i32, to: i32) {
        let (dx, dy) = self.link(from, to);
        self.loo.e.x += dx;
        self.loo.e.y += dy;
    }

    fn loo_spawn(&mut self) {
        self.loo.e.x = self.g017a.0;
        self.loo.e.y = self.g017a.1;
    }

    /// `+0x2C(1); world.refresh(); +0x2C(0)`: the last drawn frame stays.
    fn loo_flash(&mut self) {
        if let Some(d) = self.draw_of(&self.loo.e) {
            self.residue.push(d);
        }
    }

    /// `fn20` @3CC2 enter messages.
    fn loo_enter(&mut self, ctx: &mut Ctx) {
        match self.loo.state {
            0 => {
                self.g0178 = 0;
                self.loo.e.x = -100;
                self.loo.e.y = -100;
                self.loo.e.d6 = 0x91;
            }
            1 => {
                let first = if ctx.rng15.below(2) == 0 {
                    if ctx.rng15.below(2) == 0 { 0x9D } else { 0x99 }
                } else if ctx.rng15.below(2) == 0 {
                    0x95
                } else {
                    0x91
                };
                self.loo_run(first);
                self.loo_spawn();
            }
            2 => {
                self.loo_run(0x8D);
                self.loo_spawn();
                let from = self.last_of(0x25);
                self.loo_link(from, 0x8D);
            }
            3 => {
                self.loo.e.d6 = 0x180;
                self.loo_spawn();
            }
            4 => {}
            5 => {
                self.loo_run(0x219);
                self.loo_spawn();
                self.loo_link(0x210, 0x219);
            }
            6 => {
                let from = self.loo.last;
                self.loo_link(from, 0x225);
                self.loo_run(0x225);
            }
            7 => {
                let from = self.loo.last;
                self.loo_link(from, 0x231);
                self.loo_run(0x231);
            }
            8 => {
                self.loo_run(0x1CF);
                self.loo_spawn();
                let from = self.last_of(0x1AD);
                self.loo_link(from, 0x1CF);
            }
            9 => {
                let (first, rate) = match ctx.rng15.below(3) {
                    0 => (0x245, 8),
                    1 => (599, 6),
                    _ => (0x266, 4),
                };
                self.loo.rate = rate;
                self.loo.first = first;
                self.loo.last = self.last_of(first);
                self.loo_spawn();
                let from = self.last_of(0x127);
                self.loo_link(from, first);
                self.loo.e.d6 = first - 1;
            }
            10 => {
                let (first, rate) =
                    if ctx.rng15.below(2) == 0 { (0x27F, 1) } else { (0x274, 2) };
                self.loo.rate = rate;
                self.loo.first = first;
                self.loo.last = self.last_of(first);
                self.loo_spawn();
                let from = self.last_of(0x1D3);
                self.loo_link(from, first);
                self.loo.e.d6 = first - 1;
            }
            _ => {}
        }
    }

    /// `fn20` update messages; true when the state changed.
    fn loo_update(&mut self, ctx: &mut Ctx) -> bool {
        let s = self.loo.state;
        let (first, last) = (self.loo.first, self.loo.last);
        macro_rules! adv {
            () => {{
                let mut e = self.loo.e;
                let done = self.adv45(&mut e, first, last);
                self.loo.e = e;
                done
            }};
        }
        match s {
            0 => {
                if self.g0178 != 0 {
                    let ns = self.g0178;
                    self.set_lstate(ns);
                    return true;
                }
                false
            }
            1 | 2 | 8 => {
                if adv!() {
                    self.loo_flash();
                    self.set_lstate(0);
                    return true;
                }
                false
            }
            3 => {
                self.loo.e.d6 += 1;
                if 0x181 < self.loo.e.d6 {
                    self.loo_flash();
                    self.set_lstate(0);
                    return true;
                }
                false
            }
            4 => false,
            5 => {
                if self.loo.e.d6 == last - 2 {
                    self.snd(ctx, 1, SND_NEW_SPLATTER);
                }
                if adv!() {
                    self.loo_flash();
                    let ns = if ctx.rng15.below(2) != 0 { 6 } else { 0 };
                    self.set_lstate(ns);
                    return true;
                }
                false
            }
            6 => {
                if adv!() {
                    self.set_lstate(7);
                    return true;
                }
                false
            }
            7 => {
                if self.fn23() {
                    self.set_lstate(0);
                    return true;
                }
                let mut e = self.loo.e;
                if self.inline_step(&mut e, first, last) {
                    e.d6 = first;
                }
                self.loo.e = e;
                false
            }
            9 | 10 => {
                let d6 = self.loo.e.d6;
                if d6 == last - 2 {
                    self.snd(ctx, 1, SND_NEW_SPLATTER);
                }
                if d6 == first + self.loo.rate {
                    self.snd(ctx, 1, SND_NEW_BOUNCE);
                }
                if adv!() {
                    self.loo_flash();
                    self.set_lstate(0);
                    return true;
                }
                false
            }
            _ => {
                self.set_lstate(0);
                true
            }
        }
    }

    fn draw_of(&self, e: &Ent) -> Option<SpriteDraw> {
        let f = self.pack.frame(BASE, e.d6 as u32)?;
        let g = self.geom.get(&(e.d6 as u32))?;
        Some(SpriteDraw {
            pal: 0,
            png: f.png.clone(),
            x: e.x - (g.w >> 1),
            y: e.y - (g.h >> 1),
            flip: self.g0270 == 2,
        })
    }
}

impl Module for PhlegmBoy {
    fn name(&self) -> &'static str {
        "Phlegm Boy"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Behavior".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            ControlDef {
                name: "Mess".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
        ]
    }

    /// `fn04` @0C64 compares Behavior/15 and Mess/16 against the cached pair
    /// every frame and requests a restart when either moved (button up).
    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.behavior = value.clamp(0, 100),
            1 => self.mess = value.clamp(0, 100),
            _ => {}
        }
        if self.behavior / 15 != self.cached_beh || self.mess / 16 != self.cached_mess {
            self.restart_pending = true;
        }
    }

    /// `fn04` @0C64 DoDrawFrame.
    fn tick(&mut self, ctx: &mut Ctx) {
        if self.restart_pending && !ctx.mouse_down {
            self.restart_pending = false;
            self.reset_state();
        }
        self.clock = ctx.now_ms; // g017E
        if self.need_ctor_rolls {
            self.need_ctor_rolls = false;
            // @076A: the message line, STR# 128 + rand()%11
            let id = 128u16 + ctx.rng15.below(11);
            self.message = self.pack.strings(id).first().cloned();
            // fn41 @0F30: the first fidget rate, and the two deadlines
            self.boy.fid_rate = if self.g026c < 4 {
                ctx.rng15.below(2) as i32 + 1
            } else {
                ctx.rng15.below(4) as i32 + 1
            };
            self.boy.next_due = self.clock;
            self.boy.f2 = self.clock + self.g026a as u64 * 300_000;
        }
        if self.boy.next_due <= self.clock {
            if self.boy.pending {
                self.boy.pending = false;
                self.boy_enter(ctx);
            }
            if !self.boy_update(ctx) {
                // tail @3420: draw at pos, reschedule
                self.boy_img = self.draw_of(&self.boy.e);
                self.boy.next_due = self.clock + 100;
            }
            if self.g0178 != 0 {
                if self.loo.pending {
                    self.loo.pending = false;
                    self.loo_enter(ctx);
                }
                if !self.loo_update(ctx) {
                    self.loo_img = self.draw_of(&self.loo.e); // tail @4696
                }
            }
        }
        // Required1.fn41DC: a keyboard-state change opens the +0xCC window
        if ctx.caps_lock != self.key_state {
            self.key_state = ctx.caps_lock;
            self.boy.cc = true;
            if self.boy.ca == 0 {
                self.key_deadline = self.clock + 5000;
            }
            if self.clock < self.key_deadline {
                self.boy.ca += 1;
            } else {
                self.boy.ca = 0;
            }
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        for d in &self.residue {
            out.push(d.clone());
        }
        if let Some(d) = &self.boy_img {
            out.push(d.clone());
        }
        if let Some(d) = &self.loo_img {
            out.push(d.clone());
        }
    }

    fn texts(&self, out: &mut Vec<TextDraw>) {
        if !engine::SHOW_QUIP_CAPTIONS {
            return;
        }
        if let Some(t) = &self.message {
            let w = engine::font::text_width(t, 1);
            out.push(TextDraw {
                x: (SCREEN_W - w) / 2,
                y: SCREEN_H - 48,
                color: engine::contrast_ink(self.field()),
                scale: 1,
                text: t.clone(),
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The frame gate compares against `TickCount() * 16.625`.
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::collections::HashSet;
    use std::path::Path;

    fn fresh(seed: u32) -> Option<(PhlegmBoy, Ctx)> {
        let Ok(pack) = Pack::load(Path::new("../assets/phlegm-boy")) else {
            eprintln!("assets/phlegm-boy missing — skipping");
            return None;
        };
        let m = build(pack)?;
        let ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(seed),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        Some((m, ctx))
    }

    #[test]
    fn phlegm_boy_smoke() {
        let Some((mut m, mut ctx)) = fresh(1) else { return };
        let mut pace = Pacer::new(&m);
        let mut out = Vec::new();
        let (mut drew, mut sounded) = (false, false);
        for _ in 0..3000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            sounded |= !ctx.sounds.is_empty();
            ctx.sounds.clear();
            out.clear();
            m.sprites(&mut out);
            drew |= !out.is_empty();
        }
        assert!(drew, "nothing drawn");
        assert!(sounded, "no sound");
    }

    /// Every run the module names exists in the pack, so no enter can leave
    /// the sprite on a frame that has no art.
    #[test]
    fn every_named_run_ships() {
        let Some((m, _)) = fresh(1) else { return };
        for first in [
            7, 13, 30, 161, 228, 173, 208, 193, 649, 662, 37, 70, 94, 108, 119, 133, 127, 102,
            389, 394, 400, 498, 502, 492, 517, 169, 1, 243, 372, 306, 316, 323, 328, 409, 416,
            422, 446, 429, 295, 467, 276, 523, 145, 149, 153, 157, 141, 385, 537, 549, 561, 463,
            581, 599, 614, 639, 628, 235, 239, 253, 270,
        ] {
            assert!(m.pack.seq(BASE, first).is_some(), "run {first} missing from the pack");
        }
    }

    /// Over a long run the boy visits the idle poses, the hold, the pour
    /// and at least one gag chain, and the loogie leaves snot behind.
    #[test]
    fn the_machine_gets_around() {
        let Some((mut m, mut ctx)) = fresh(0xC0FFEE) else { return };
        let mut pace = Pacer::new(&m);
        let mut seen: HashSet<i32> = HashSet::new();
        let mut lseen: HashSet<i32> = HashSet::new();
        for _ in 0..60_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            seen.insert(m.boy.state);
            lseen.insert(m.loo.state);
        }
        for s in [1, 2, 3, 4, 5, 8, 23] {
            assert!(seen.contains(&s), "boy never reached state {s}: {seen:?}");
        }
        assert!(seen.len() > 12, "boy only saw {} states: {seen:?}", seen.len());
        assert!(lseen.contains(&1), "the loogie never dripped: {lseen:?}");
        assert!(!m.residue.is_empty(), "no snot on the screen");
    }

    /// `g0182..g0188` are the canvas XRect widened to at least 400×200
    /// (ctor @0702/@0722): the 640×480 screen passes through untouched.
    #[test]
    fn the_world_rect_is_the_canvas_clamped_to_400_by_200() {
        assert_eq!(WORLD, (0, 0, 640, 480));
        assert_eq!(world_rect((10, 20, 300, 120)), (10, 20, 410, 220));
        assert_eq!(world_rect((5, 5, 405, 205)), (5, 5, 405, 205));
    }

    /// fn47's blink parks him off the right edge (`g0186 + w/2 + rand%100`)
    /// facing in, and state 3 walks run 13 until he is wholly inside
    /// (`x + w/2 < g0186`). The golden (phlegm-boy.mp4 f266 and f732) has
    /// two right-side walk-ins, both stopping with the shirt-stripe centre
    /// at x 518/519; the stripe sits 4–9 px left of the sprite centre in the
    /// stop frames, so the boy's centre is ≈ 522–528. Pin that band inside
    /// the port's stop range for right = 640 (right = 480, the QuickDraw
    /// field order, would stop every walk-in below x 420).
    #[test]
    fn right_walk_ins_stop_where_the_golden_does() {
        let (mut lo, mut hi, mut n) = (i32::MAX, i32::MIN, 0);
        for seed in [1u32, 3, 5, 7, 9, 11] {
            let Some((mut m, mut ctx)) = fresh(seed) else { return };
            m.set_control(0, 100);
            m.set_control(1, 100);
            let mut pace = Pacer::new(&m);
            let (mut prev, mut from_right) = (m.boy.state, false);
            for _ in 0..40_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                ctx.sounds.clear();
                let s = m.boy.state;
                if s == 3 && prev != 3 {
                    // off the right edge facing in (unmirrored = facing left)
                    from_right = m.boy.e.x > RIGHT && m.g0270 == 1;
                }
                if prev == 3 && s == 4 && from_right {
                    let x = m.boy.e.x;
                    assert!(x + m.boy.w / 2 < RIGHT, "stopped before wholly inside: {x}");
                    lo = lo.min(x);
                    hi = hi.max(x);
                    n += 1;
                }
                prev = s;
            }
        }
        assert!(n >= 3, "only {n} right-side walk-ins");
        assert!(lo <= 522 && 528 <= hi, "port stop band [{lo}, {hi}] misses the golden's 522..528");
    }

    /// `fn3DDC` rounds each mid-point with the sequence's flip bit,
    /// `(flip + l + r) >> 1`, so while the boy is mirrored an in-run step
    /// differs from the plain centre difference by 1 px wherever the two
    /// frames' widths differ in parity. Run 13 (the walk) steps 13→14,
    /// 14→15, 15→16 are 7/14/19 px as bare centre differences and 8/14/18
    /// under fn3DDC; unmirrored the two agree (−7). The golden cannot
    /// adjudicate 1 px (its only walks are unmirrored right-side
    /// walk-ins), so these come from the pack's rects and the C's formula.
    #[test]
    fn mirrored_steps_round_with_the_sequence_flip() {
        let Some((mut m, _)) = fresh(1) else { return };
        m.g0270 = 1;
        assert_eq!(m.link(13, 14).0, -7);
        m.g0270 = 2;
        assert_eq!([m.link(13, 14).0, m.link(14, 15).0, m.link(15, 16).0], [8, 14, 18]);
    }

    /// The squeegie: past `g026A * 300000` ms the residue is wiped.
    #[test]
    fn the_idle_deadline_wipes_the_snot() {
        let Some((mut m, mut ctx)) = fresh(7) else { return };
        m.set_control(1, 0); // Mess band 1: the wipe comes every 5 minutes
        let mut pace = Pacer::new(&m);
        let mut wiped = false;
        let mut prev = 0usize;
        for _ in 0..(6 * 60 * 60) {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.residue.len() < prev {
                wiped = true;
                break;
            }
            prev = m.residue.len();
        }
        assert!(wiped, "the idle wipe never fired in six minutes at Mess 0");
    }
}

