//! Chameleon — ported function-by-function from the Ghidra decompile of
//! module CODE 129 "Chameleon" + CODE 130 (totally-twisted
//! docs/decompiled/chameleon/, private). Listing addresses are the raw
//! resource_dasm ones (Ghidra address − segment base − 4).
//!
//! ## Objects (controller ctor `fn18` @head, M129)
//! - one `AnimationSequence` over art bank 1000, SHARED by every lizard
//!   (`g0460`); the sprite list is `g047E`.
//! - `g0414`: the 13-channel sound bank (`fn32` @1512 / `fn34` @160A); the
//!   ids come from the 13-word table at A5 `g003A`, dumped below as
//!   [`SOUND_TABLE`]. `fn35` @1760 is the player.
//! - `g0424[0..10]`: up to ten lizards, class ctor `M130_fn10` @0100, state
//!   sub-object at `+0x92`, handler `M130_fn17` @057E (3 878 lines of C,
//!   27 states). Each lizard carries a 1-based index at `+0x11E`; the
//!   parallel per-index arrays are `g0418` (busy) and `g03DE` (the request
//!   the controller posts) and `g0120` (the pair handshake).
//! - `g0486` = clut 1500 (the boot palette); `g048A` = clut 1100.
//! - the controller itself is a Library state machine too (`fn28` @0982
//!   ctor, `fn29` @09C6 handler, states 0/1/2/3/4).
//!
//! ## Library 4.0 sprite semantics (see bungee_roulette.rs for the L135
//! ## proofs; every slot below is LIBRARY code — this class overrides none)
//! - `pos` (`+0x40` x, `+0x42` y) is the CENTRE of the current frame;
//!   `centre(f) = (bx+dx+w/2, by+dy+h/2)` in bank space and
//!   `link(from,to) = centre(to) − centre(from)`.
//! - `+0x7C` = `SetRun(N)` (L132 `fn0204` → `fn028A`): the LINKED
//!   hand-off (vtable `M130_g0232`: `+0x7C` = `fn0204`, `+0x108` =
//!   `fn028A`, `+0xCC` = `fn0D3C`, `+0xF0` = `fn1186`, `+0xD0` = `fn0DF4`;
//!   all 72 `+0x7C` sites in `M130_fn17` go through it). From the frame
//!   showing, pass through the lead-in `N − 1` when that record exists,
//!   registering onto it with L135 `fn3F2E` — the first body part both
//!   frames carry keeps its screen spot, and the sprite's mirror toggles
//!   when that part's flip bit differs — then `fn3DDC`-link to `N`. Then
//!   `first = N`, `last = N − 1 + len(N)`, `+0x48` ("fresh") set so the
//!   NEXT advance does not step. See `Chameleon::hand_off`.
//!   *(superseded 2026-09-29: "Unlinked — the position does not move".
//!   It moves, and it mirrors: the turn 0x1B's body is authored mirrored
//!   from frame 30, so the turn → walk hand-off flips the lizard; the
//!   folder/pair runs 0x1A2.. and 0x3xx do the same.)*
//! - `+0x84` = the per-frame advance (L132 `fn0316`): steps `frame + 1`
//!   by the link, and on the SAME call that reaches `last` either queues
//!   `first` (repeats left, `+0x4C >= 1`) or a chained run in `+0x4E` —
//!   handed off by `fn028A` on the next call — or raises `+0x46`. See
//!   `Chameleon::advance`. *(superseded 2026-09-29: "returns to `first`
//!   with NO delta; the walk's stride is exactly `centre(last) −
//!   centre(first)`" — the loop is a linked hand-off `5 → 1 → 2` and a
//!   cycle travels (±17, 0), not (15, +4); and `+0x50` is not "loop once
//!   more" but "remove at end".)*
//! - `+0xCC` (L132 `fn0D3C`) is the linked frame jump the module uses
//!   deliberately (`+0xCC(10)`, `+0xCC(9)`, `+0xCC(0x374)`); it registers
//!   through `fn3F2E` exactly like the hand-off, not by frame centres.
//! - `+0xFC` (L132 `fn12DE`, mode 1) returns a run's net displacement,
//!   `centre(last) − centre(first)`; the walks divide by its x.
//! - `+0x104` (L132 `fn13E0`) is `frame − first`, so the sound tail's
//!   `+0x104() + first` is simply the absolute frame number.
//! - `SetState(s)` fires the exit at once and flags pending; the enter and
//!   the update then run in the next `Run`. **A handler branch that calls
//!   SetState returns without the sound tail.** The lizard handler has no
//!   exit cases at all.
//!
//! ## Frame driver
//! The controller's state 3 runs EVERY module frame with no gate of its
//! own, and the module frame is the Mac tick (`TickClock::MacTick`) — the
//! `RandomBelow(50000)` rare-event roll is per frame. Each lizard gates
//! itself on `+0x15A` with `+0x166` ms (100 everywhere; 75 in the fire
//! state), which on the truncated `TickCount()*16.625` grid is the measured
//! ~106.4 ms family.
//!
//! ## Numbering
//! A `SetRun` / `+0xCC` argument **is** the `OFst` record's `frameNum`, as
//! written — no ±1 (`docs/engine/library40-api.md` §10.1). L135 `fn3252`
//! builds `map[frameNum] = recordIndex` over `maxFrameNumber + 1` words;
//! `fn441A` is `1 <= id <= maxFN && map[id] >= 0`, `fn4456` walks the run
//! length forward from `id`, and `fn3BD6` resolves art at
//! `OFst + 0xE + map[id] * 10`. Nothing subtracts one anywhere. The pack's
//! `meta.json` is keyed by the same `frameNum`, so a C id indexes [`geom`]
//! / [`seq_end`] directly — [`fid`] is only the `i32 → u32` cast.
//!
//! An `OFst` table is *blocks* of consecutive `frameNum`s with unallocated
//! gaps between them, and Berkeley's art tool usually emitted a **lead-in
//! record one id before an animation's real first frame** (33 of
//! chameleon's 62 blocks). So most ids here happen to be `blockStart + 1`
//! — which is what made the old `pk(id) = id − 1` look right. Resolving an
//! id means finding the block that CONTAINS it and **entering the run at
//! that id**; the run then plays to the block's last frame. Never an exact
//! `first` match. Three ids enter further in than +1 and that is correct:
//! C 10 (`R_IDLE_B`, block 8..10, the last frame alone), C 0x20A = 522 and
//! C 0x211 = 529 (both inside the 44-frame fire block 510..553, which
//! `R_FIRE` = C 511 enters at +1). Two ARE block starts, because the module
//! addresses a lead-in itself: C 0x374 = 884 (block 884..923) and C
//! 0x3CB = 971 (block 971..982).
//!
//! Every constant below is the C id, unchanged.
//!
//! ## What stays approximate
//! - `+0x94`/`+0x98` (L132 `fn05D4`/`fn0718`) walk the run forward until
//!   the sprite leaves a rect and then place it there. Ported as a clamp of
//!   the centre into the field rect.
//! - `+0xE8` (L132 `fn10FE`) insets the partner's frame rect before the
//!   pair states take its CENTRE — an inset does not move a centre, so it
//!   is dropped.
//! - Message line: `STR# 128 + RandomBelow(23)` at construction goes to
//!   After Dark's own message line; nothing is drawn unless
//!   `SHOW_QUIP_CAPTIONS`.
//!
//! ## GAPs (present in the C, absent here — see the ledger)
//! - `GAP(desktop snapshot)`: `fn30` @0DB4 scans the live desktop into the
//!   ten 20-byte slot records at `g0056` (rect) / `g0314` (in-use flag).
//!   retwisted runs on a flat mid-grey field, so no slot is ever free and
//!   the four window states (0x0B, 0x12 FolderSex, 0x13 TossEat,
//!   0x14 TongueEat) bail back to state 0 exactly as the C does when the
//!   scan finds nothing. Desktop tiering is a .saver-time decision.
//! - `GAP(control 3)`: `fn21` @0870 reads a fourth control into
//!   `g0450 = value − 1`, but only three `sVal` resources ship, so the read
//!   returns 0 and `g0450` is permanently −1 — the default behaviour
//!   ladder. The `g0450` 1..12 cases are ported and dead, as shipped.
//! - ~~`GAP(camo blend)`~~ *(closed 2026-09-29: "the engine's palette
//!   support is a single-clut substitution, so the colour switches at the
//!   roll". The blend is ported — per lizard, in 16-bit, with the C's
//!   truncation and its unapplied last step — and drawn through a
//!   per-sprite palette, `engine::DYN_PAL`. See `camo_fade` and the
//!   2026-09-29 camouflage erratum.)*
//! - `GAP(puddle)`: no permanent vomit puddle exists in the C. The prose
//!   port drew one (170×44, forever); `M130_fn17`'s ThrowUp state 0x15 has
//!   no such sprite and no decal list. Removed.
//!
//! ## Original quirks kept
//! - Zest 100 reads `g0026[100/20] = g0026[5]`, one long PAST the five-entry
//!   idle pool, landing in the sound-id table: `0x07D007D1`. The idle delay
//!   becomes ~65 000 s and the act ladder freezes (`IDLE_POOL_OVERRUN`).
//! - State 0's request-1 path writes an off-screen spawn x into `+0x134`
//!   and then every entered state overwrites `+0x134` with the live
//!   position before it is read. The off-screen value is dead.
//! - The window states pick their facing by comparing the slot rect's
//!   VERTICAL midpoint with the field's vertical midpoint (`M130_fn17`
//!   @2EF2/@3462/@3922) — a facing decided by y.
//! - `SetRun` does not clear the repeat counter `+0x4C`, so a walk that
//!   ends early leaves repeats for whatever run follows until they drain.
//!
//! ## Errata
//! - **2026-09-19 — `pk(id) = id − 1` was wrong; it is gone.** The first
//!   port of this module claimed "every C `SetRun` id is `pack first + 1`"
//!   and mapped every id down by one. That is not the engine's rule (see
//!   "Numbering" and `library40-api.md` §10.1): ids are `frameNum`s as
//!   written. The `−1` only looked right because of the lead-in records,
//!   and it manufactured two fake `GAP(rip holes)` — C 0x374 = 884 and
//!   C 0x3CB = 971 are live block starts that the pack has always
//!   carried; `id − 1` fell into the inter-block gaps at 883 and 970 and
//!   read as missing art, which lost half the mating animation. With the
//!   `−1` gone every named run, every `TONGUE_RUNS` entry and both mating
//!   ids resolve. Consequences: each run now starts one frame later than
//!   the old port drew it and the lead-in duplicate is no longer shown,
//!   and the walk's stride `centre(last) − centre(first)` is 15 px (run
//!   2..5), not the 17 px the old port measured over 1..5.
//!   *(2026-09-29: the 15 px is only `+0xFC`'s divisor; on screen a cycle
//!   is 17 px after all, because the loop hands off through the lead-in
//!   1 — see `advance`.)*
//! - **2026-09-29 — the run hand-off was ported unlinked; it is linked.**
//!   `set_run` applied no delta and never touched the mirror, `+0xCC`
//!   linked frame centres, and a loop wrapped with no delta. Symptoms:
//!   the walk sank 4 px per cycle (9.4 px/s) and crawled at 35.2 px/s, a
//!   lizard that played the turn kept its old mirror and walked on the
//!   way it came, and the body jumped ±37..103 px on the idle ↔ pair /
//!   folder / fire hand-offs (10 ↔ 0x374, 10 ↔ 0x1A2, 615 → 10), drawn
//!   facing the wrong way where those runs author it mirrored. Now
//!   `hand_off` = `fn028A`, `jump_linked` = `fn0D3C` → `fn3F2E`,
//!   `advance` = `fn0316` (which also raises `+0x46` on the call that
//!   REACHES `last`, one tick earlier than the old port, and dequeues the
//!   `+0x80` chain itself), links = `fn3DDC` with its flip rounding, part
//!   rects laid out as `fn3BD6` does. GOLDEN `chameleon-long-av.mp4`
//!   (walk frames template-matched, 30 tracks): 39.4 px/s, vy 0.0, mirror
//!   matches direction 30/30; the port now walks 39.9 px/s, vy 0.0.
//! - **2026-09-29 — the camouflage snapped; it fades.** `M130_fn17`'s
//!   prologue blends colour-table positions 0..20 of `+0x14A` (base)
//!   toward `+0x152` (target, `LoadCLUT(RandomBelow(9) + 1500)` at the
//!   roll) into `+0x14E`, per channel in 16-bit:
//!   `base + step * ((target − base) / 20)`, one step when `+0x15E < now`
//!   (then `+0x15E = now + 500`, i.e. every 31 Mac ticks = 515 ms).
//!   Steps 0..18 are applied to the lizard's own colour lookup by
//!   `M130_fn24`; step 19 is copied into `base` instead and never shown.
//!   So a roll shows 19 changes over 9.3 s (the first re-applies the
//!   previous fade's step 19), settles at 18/20 of the way, and `base`
//!   drifts — colours are not the nine Cham cluts after the first fade.
//!   The old port changed `pal` to the target clut at the roll: one jump,
//!   exactly onto the clut. GOLDEN `chameleon-long.mp4` f2753..f2846:
//!   19 changes, 516.7 ms apart, linear, ending (70,254,0) for a target
//!   R of 51; the port, from the same start: 19 changes 515.4 ms apart,
//!   ending (71,251,0). Ratchets `camo_fade_matches_the_golden`,
//!   `camo_colour_never_snaps`, `compose_draws_the_blended_palette`.

use std::collections::HashMap;

use engine::l135::{self, FrameBox, LinkModel};
use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TextDraw, TickClock, SCREEN_H,
    SCREEN_W,
};

const BASE: u32 = 1000;

/// A C run/frame id as a pack key. The pack is keyed by the same `OFst`
/// `frameNum` the C writes, so this is only the `i32 → u32` cast — there is
/// no ±1. See "Numbering" above and `library40-api.md` §10.1.
fn fid(c: i32) -> u32 {
    c.max(0) as u32
}

// ---------------------------------------------------------------------------
// A5 tables (dumped from emu/ghidra/chameleon/blocks/A5_globals.bin)

/// `g003A`, 13 words: what `fn34` @160A loads into each sound channel.
/// Ids below 30000 are the module's own `snd`; the rest are the shared bank.
const SOUND_TABLE: [u32; 13] = [
    2000, 2001, 2002, 2003, 2004, 30007, 2005, 30009, 30008, 2007, 2008, 30005, 30013,
];

/// `g0026`, five longs: the idle pool state 3's enter indexes by `Zest/20`.
const IDLE_POOL: [i64; 5] = [10000, 7500, 5000, 2500, 0];

/// `g0026[5]` — off the end of [`IDLE_POOL`] and into [`SOUND_TABLE`]'s
/// first two words read as one long. Zest 100 lands here. Quirk kept.
const IDLE_POOL_OVERRUN: i64 = 0x07D0_07D1;

/// `M130_g0000`, nine words: the TongueEat activity runs state 0x14 rolls.
const TONGUE_RUNS: [i32; 9] = [0x37, 0x42, 0x4d, 0x59, 0x64, 0x6f, 0x7b, 0x85, 0x90];

/// `fn35` @1760 suppresses a cue while the running clip has more than
/// 180 ms left AND the last index played is <= this one.
const SND_TAIL_MS: i64 = 180;

/// The camouflage cluts: `RandomBelow(9) + 1500`, never the current one.
const CLUT_BASE: i32 = 1500;
const CLUT_COUNT: u32 = 9;
/// One interpolation step of the 20-entry blend per 500 ms (`+0x15E`).
const COLOUR_HOLD_MS: u64 = 500;
/// 20 steps (`+0x158` counts to 0x14).
const FADE_STEPS: i32 = 20;
/// The blend covers colour-table POSITIONS 0..20 (`RectOp1`/`RectOp2`
/// index `ctTable[i]`); every Cham clut 1500..1508 is exactly those 20
/// entries with `value == position`, so position and slot coincide.
const FADE_ENTRIES: usize = 20;

/// A Cham colour table as the Mac holds it: 20 × (r, g, b) in 16-bit
/// `RGBColor` components. The blend runs in these units, not in 8-bit.
type Clut16 = [[u16; 3]; FADE_ENTRIES];

/// `+0x94` = now + 900 000: the controller's 15-minute lifespan.
const MASTER_MS: u64 = 900_000;
/// The per-frame rare-event roll in controller state 3.
const RARE_EVENT_MOD: u32 = 50_000;

// Named runs (C space) — every one is a `SetRun` argument in `M130_fn17`.
const R_WALK: i32 = 2; // 5 frames, net (17, 0) per cycle
const R_IDLE_A: i32 = 9;
const R_IDLE_B: i32 = 10;
const R_LOOK_LEFT: i32 = 0x0e;
const R_TURN: i32 = 0x1b;
const R_TOSS_END: i32 = 0x9d;
const R_LOOK_UP: i32 = 0xaf;
const R_VOM_A: i32 = 0xb8;
const R_VOM_B: i32 = 0xc0;
const R_VOM_C: i32 = 200;
const R_VOM_HIT_SELF: i32 = 0xdc;
const R_VOM_HIT_OTHER: i32 = 0xe0;
const R_VOM_D: i32 = 0xe6;
const R_VOM_E: i32 = 0xf6;
const R_BLINK_IN: i32 = 0x11c;
const R_BLINK_A: i32 = 0x124;
const R_BLINK_B: i32 = 0x135;
const R_YAWN: i32 = 0x143;
const R_TOSS_CLIMB: i32 = 0x156;
const R_SEX_CLIMB: i32 = 0x171;
const R_SEX_LOOP: i32 = 0x181;
const R_SEX_END: i32 = 0x187;
const R_RIDE_JOIN: i32 = 0x196;
const R_FOLDER_A: i32 = 0x1a2;
const R_FOLDER_B: i32 = 0x1d7;
const R_FOLDER_C: i32 = 0x1eb;
const R_FIRE: i32 = 0x1ff;
const R_FIRE_B: i32 = 0x20a;
const R_FIRE_C: i32 = 0x211;
const R_PAIR_A: i32 = 0x22e;
const R_PAIR_B: i32 = 0x23e;
const R_PAIR_C: i32 = 0x254;
const R_PAIR_D: i32 = 0x272;
const R_PAIR_E: i32 = 0x294;
const R_PAIR_F: i32 = 0x29d;
const R_PAIR_G: i32 = 0x2ad;
const R_PAIR_H: i32 = 0x2c1;
const R_FACE_OFF: i32 = 0x32a;
const R_SLAVE_ANCHOR: i32 = 0x374;
const R_MATE_A: i32 = 0x375;
const R_MATE_B: i32 = 0x39e;
const R_MATE_C: i32 = 0x3cb;
const R_MATE_D: i32 = 0x3dc;
const R_STROLL: i32 = 0x40e;
const R_ARRIVE: i32 = 0x429;

// ---------------------------------------------------------------------------
// Geometry

/// One packed frame's bounds in bank space: the compound rect (`bx,by,w,h`)
/// shifted by the frame's own OFst offset (`dx,dy`) — L135 `fn3A70`. The
/// module measures `w,h` off the compound image, not the pack record.
type Geom = FrameBox;

/// The saved sprite state `+0xC0` (L132 `fn0C7E`) writes into the lizard's
/// own 10-byte scratch at `+0x86` and `+0xC4` (`fn0CB6`) restores.
#[derive(Clone, Copy, Default)]
struct Saved {
    frame: i32,
    first: i32,
    x: i32,
    y: i32,
    flip: bool,
}

/// One lizard (`M130_fn10` @0100 builds it, `M130_fn17` @057E drives it).
#[derive(Clone)]
struct Liz {
    // library sprite fields
    x: i32,           // +0x40
    y: i32,           // +0x42
    flip: bool,       // +0x3C bit 0
    frame: i32,       // +0x3A (C space)
    first: i32,       // +0x44
    last: i32,        // first − 1 + len(first)
    fresh: bool,      // +0x48
    wrapped: bool,    // +0x46
    repeat: i32,      // +0x4C
    /// `+0x50`: when the run ends, flag the sprite for removal (`+0x54`)
    /// and hand off to frame 0 instead of raising `+0x46` — see `advance`.
    /// (Read as "loop once more" until 2026-09-29; superseded.)
    remove_at_end: bool,
    /// `+0x4E`: the run `advance` hands off to on its next call (−1: none).
    queued: i32,
    /// The run queue `+0x80` (L132 `fn0240`) fills behind its first id;
    /// this class only ever queues one (`+0x80(0x11C, want, −1)`).
    queue: Option<i32>,
    // module fields
    idx: i32,         // +0x11E, 1 based
    facing: i32,      // +0x122
    partner: i32,     // +0x126 (1-based lizard index, or a slot, or −1)
    activity: i32,    // +0x128 (the TongueEat run)
    slot: i32,        // +0x12A
    from: (i32, i32), // +0x134 / +0x136
    to: (i32, i32),   // +0x138 / +0x13A
    handshake: i32,   // +0x13D
    f13e: bool,       // +0x13E
    was_mate_b: bool, // +0x13F
    holding: bool,    // +0x140
    save: Saved,      // +0x86
    /// `+0x14A`: the colour table the blend starts from. Clut 1500 at
    /// construction; at the end of every fade it becomes a copy of `work`
    /// (L129 `fn3D3C`), i.e. 19/20 of the way to the target — never the
    /// target itself, so the colour drifts from fade to fade.
    base: Clut16,
    /// `+0x14E`: the table each blend step writes.
    work: Clut16,
    /// `+0x152`: `LoadCLUT(+0x156)`, the table the blend heads for.
    target: Clut16,
    /// What `M130_fn24` @5364 last pushed into this lizard's own colour
    /// lookup (`+0x146`'s RLE sequence; in 8-bit, its device slots):
    /// clut 1500 from `M130_fn25` @55EC at construction, then `work` at
    /// every step EXCEPT the one that completes the fade. This is what
    /// the sprite is drawn in.
    shown: Clut16,
    clut: i32,        // +0x156
    fade: i32,        // +0x158
    deadline: i64,    // +0x15A
    colour_due: i64,  // +0x15E
    delay: i64,       // +0x166
    state: i32,       // sub-object +4
    prev: i32,        // sub-object +6
    pending: bool,    // sub-object +0xE
}

impl Liz {
    fn new(idx: i32) -> Liz {
        Liz {
            x: -1000,
            y: -1000,
            flip: false,
            // the `fn0058` reset: no frame showing (`+0x3A = 0`), so the
            // first SetRun registers nothing
            frame: 0,
            first: 0,
            last: 0,
            fresh: false,
            wrapped: true,
            repeat: 0,
            remove_at_end: false,
            queued: -1,
            queue: None,
            idx,
            facing: 0,
            partner: 0,
            activity: TONGUE_RUNS[0],
            slot: -1,
            from: (-1000, -1000),
            to: (-1000, -1000),
            handshake: 0,
            f13e: false,
            was_mate_b: false,
            holding: false,
            save: Saved::default(),
            base: [[0; 3]; FADE_ENTRIES],
            work: [[0; 3]; FADE_ENTRIES],
            target: [[0; 3]; FADE_ENTRIES],
            shown: [[0; 3]; FADE_ENTRIES],
            clut: CLUT_BASE,
            fade: FADE_STEPS,
            deadline: 0,
            colour_due: 0,
            delay: 100,
            state: 0,
            prev: 0,
            pending: true,
        }
    }
}

pub struct Chameleon {
    pack: Pack,
    geom: HashMap<u32, Geom>,
    /// end of the packed sequence containing a compound — how `len(N)` is
    /// recovered for a C run id that starts mid-sequence (C 10, C 522, …).
    seq_end: HashMap<u32, u32>,
    /// The Cham cluts 1500..1508 in 16-bit units, for `LoadCLUT`. A clut the
    /// pack lacks is simply absent (and then loads as "no change").
    cluts: HashMap<i32, Clut16>,

    // controls
    quantity: i32, // sVal 1000 raw
    zest: i32,     // g045E
    vommeter: i32, // g0458

    // controller globals
    g045a: i32,           // Quantity/10, min 1
    g045c: i32,           // the live lizard count
    g0450: i32,           // control 3 − 1: always −1, see GAP(control 3)
    g0453: bool,          // rare event armed
    g0454: bool,          // wrap up now
    g0456: i32,           // caps-lock smite: 2 = pending, 1 = claimed
    g030c: bool,
    /// `g0310`: the one lizard, if any, holding a desktop slot.
    holder: Option<usize>,
    /// `g0314[i]`: slot in use. GAP(desktop snapshot) leaves all ten set,
    /// exactly as the ctor does before `fn30` runs.
    slot_used: [bool; 10],
    /// `g0056[i]`: the slot's rect (left, top, right, bottom). Never filled
    /// — see GAP(desktop snapshot).
    slot_rect: [(i32, i32, i32, i32); 10],
    /// `g03DE[1..=10]`: the request the controller posts to a lizard.
    request: [i32; 11],
    /// `g0418[1..=10]`: the lizard is busy.
    busy: [bool; 11],
    /// `g0120[1..=10]`: the pair handshake channel.
    hand: [i32; 11],

    // controller state machine (fn28/fn29)
    c_state: i32,
    c_prev: i32,
    c_pending: bool,
    c_deadline: i64, // +0x90
    c_master: i64,   // +0x94
    caps_prev: bool,

    // sound bank (fn32/fn34/fn35)
    snd_last: i32,   // +0x22
    snd_busy: i64,   // +0x1E
    lizards: Vec<Liz>,
    message: Option<String>,
    need_ctor_rolls: bool,
    now: i64,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

/// A pack clut as the Mac's 16-bit table. The pack stores the high byte;
/// every entry of the ripped `clut` 1500..1508 resources is byte-replicated
/// (`0x3333`, `0xF8F8`, `0xBDBD`, ... — checked against the raw resources
/// 2026-09-29), so `v * 0x101` restores it exactly.
fn clut16(pack: &Pack, id: i32) -> Option<Clut16> {
    let p = pack.meta.palettes.get(&id.to_string())?;
    let mut c = [[0u16; 3]; FADE_ENTRIES];
    for (k, e) in c.iter_mut().enumerate() {
        let rgb = p.get(&(k as u16))?;
        *e = rgb.map(|v| v as u16 * 0x101);
    }
    Some(c)
}

fn build(pack: Pack) -> Option<Chameleon> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    let mut geom = HashMap::new();
    let mut seq_end = HashMap::new();
    for seq in pack.series(BASE) {
        let end = seq.first + seq.frames.len() as u32 - 1;
        for (i, f) in seq.frames.iter().enumerate() {
            let img = pack.image(&f.png);
            let id = seq.first + i as u32;
            geom.insert(
                id,
                Geom { bx: f.bx, by: f.by, w: img.w as i32, h: img.h as i32, dx: f.dx, dy: f.dy },
            );
            seq_end.insert(id, end);
        }
    }
    let cluts = (CLUT_BASE..CLUT_BASE + CLUT_COUNT as i32)
        .filter_map(|id| clut16(&pack, id).map(|c| (id, c)))
        .collect();
    let mut m = Chameleon {
        pack,
        geom,
        seq_end,
        cluts,
        quantity: 60,
        zest: 80,
        vommeter: 25,
        g045a: 6,
        g045c: 6,
        g0450: -1,
        g0453: false,
        g0454: false,
        g0456: 0,
        g030c: false,
        holder: None,
        slot_used: [true; 10],
        slot_rect: [(0, 0, 0, 0); 10],
        request: [0; 11],
        busy: [false; 11],
        hand: [0; 11],
        c_state: 0,
        c_prev: 0,
        c_pending: true,
        c_deadline: 0,
        c_master: 0,
        caps_prev: false,
        snd_last: -1,
        snd_busy: 0,
        lizards: Vec::new(),
        message: None,
        need_ctor_rolls: true,
        now: 0,
    };
    m.rebuild();
    Some(m)
}

// The field rect `g0476` (left) / `g0478` (top) / `g047A` (right) /
// `g047C` (bottom) — the canvas bounds the ctor reads at @017C.
const LEFT: i32 = 0;
const TOP: i32 = 0;
const RIGHT: i32 = SCREEN_W;
const BOTTOM: i32 = SCREEN_H;

impl Chameleon {
    // ---- bank / sequence helpers ------------------------------------------

    /// `fn441A`: the id has a record.
    fn valid(&self, f: i32) -> bool {
        f >= 1 && self.geom.contains_key(&fid(f))
    }

    /// `centre(f) = (bx + dx + w/2, by + dy + h/2)` — L135 `fn3A70`.
    #[cfg(test)]
    fn center(&self, f: i32) -> Option<(i32, i32)> {
        self.geom.get(&fid(f)).map(|g| g.centre())
    }

    /// Sequence `+0x78` = L135 `fn3DDC` @3DDC ([`l135::link`], the
    /// transcribed flip-bit rounding). No move when either id is unknown.
    fn link(&self, from: i32, to: i32, flip: bool) -> (i32, i32) {
        match (self.geom.get(&fid(from)), self.geom.get(&fid(to))) {
            (Some(a), Some(b)) => l135::link(a, b, flip, LinkModel::FN3DDC),
            _ => (0, 0),
        }
    }

    /// Sequence `+0x80` = L135 **`fn3F2E` @3F2E** ([`l135::register`]), the
    /// shared-part registration every hand-off links through: the first
    /// part the two frames share stays where it is on screen, and the
    /// sprite's flip is XORed with the two parts' own flip bits (`+0xE`).
    /// Part tables come off the pack, the boxes off the module's own
    /// (image-measured) map. No record or no common part: no move, no flip
    /// change. Returns `(dx, dy, flip')`.
    ///
    /// GAP(fn3F2E vertical flip): the vertical-flip toggle has no renderer;
    /// bank 1000 never sets it on a part.
    fn shared_link(&self, cur: i32, new: i32, flip: bool) -> (i32, i32, bool) {
        let (Some(ga), Some(gb)) = (self.pack.frame(BASE, fid(cur)), self.pack.frame(BASE, fid(new)))
        else {
            return (0, 0, flip);
        };
        let (Some(ba), Some(bb)) = (self.geom.get(&fid(cur)), self.geom.get(&fid(new))) else {
            return (0, 0, flip);
        };
        match l135::register(ba, &ga.parts, bb, &gb.parts, flip) {
            Some(r) => (r.delta.0, r.delta.1, r.flip),
            None => (0, 0, flip),
        }
    }

    /// `first − 1 + len(first)` — L132 `fn1260` walks forward from `first`
    /// while `fn441A` holds, which is exactly "to the last `frameNum` of
    /// the block that CONTAINS `first`". `seq_end` is keyed per frame, so
    /// an id that enters a block partway through (C 10, C 0x20A, C 0x211)
    /// still gets that block's end — the run is short, not missing.
    /// An invalid id (frame 0) is 0.
    fn last_of(&self, first: i32) -> i32 {
        match self.seq_end.get(&fid(first)) {
            Some(&e) if first >= 1 => e as i32,
            _ => 0,
        }
    }

    fn bounds(&self, f: i32) -> (i32, i32) {
        self.geom.get(&fid(f)).map(|g| (g.w, g.h)).unwrap_or((0, 0))
    }

    /// `+0xFC` (L132 `fn12DE`) mode 1: a run's net displacement — it saves
    /// the sprite, zeroes `pos`, seats `first` and links to `last` with
    /// `fn0DF4` (the `fn3DDC` link, under the current flip). The walks
    /// divide by its x. NOT the walk's travel per cycle: the loop's
    /// hand-off (see `advance`) adds the `last → marker → first`
    /// registration on top — 15 px here, 17 px on screen for run 2.
    fn cycle_dx(&self, i: usize, run: i32) -> i32 {
        self.link(run, self.last_of(run), self.lizards[i].flip).0
    }

    // ---- sprite ops (all library) -----------------------------------------

    /// `+0x7C` = L132 `fn0204` @0204: `+0x108` (the hand-off below), then
    /// `+0x50 = 0`, `+0x114` (`fn1526`, empty the run queue — the port keeps
    /// none, see `advance`) and `+0x46 = 0`. The repeat counter `+0x4C` is
    /// NOT touched (quirk kept, see the header).
    fn set_run(&mut self, i: usize, run: i32) {
        self.hand_off(i, run);
        let l = &mut self.lizards[i];
        l.remove_at_end = false;
        l.queue = None;
        l.wrapped = false;
    }

    /// `+0x80` = L132 `fn0240` @0240: `+0x7C` the first id, queue the rest
    /// (`+0x118`); `advance` dequeues at the run's end (`+0x11C`/`+0x120`).
    fn set_run_chain(&mut self, i: usize, first: i32, then: i32) {
        self.set_run(i, first);
        self.lizards[i].queue = Some(then);
    }

    /// `+0x108` = L132 **`fn028A` @028A**, the LINKED run hand-off this
    /// class binds (`M130_g0232`: `+0x7C` = `fn0204`, `+0x108` = `fn028A`,
    /// `+0xCC` = `fn0D3C`, `+0xF0` = `fn1186`, `+0xD0` = `fn0DF4`).
    ///
    /// 1. `+0x12C` = `fn0B70`, the `+0x52` glide — never armed here, a no-op.
    /// 2. Only when a frame is showing (`+0x3A != 0`): `fn1186` @1186 picks
    ///    the frame to pass through — `id − 1` when that record exists
    ///    (`+0x82` = 1 after the `fn0058` reset), the OFst lead-in — and
    ///    `+0xCC` (`fn0D3C`) registers the current frame onto it through
    ///    `fn3F2E` (shared part stays put; flip XOR).
    /// 3. `fn1210` / `+0xD0` (`fn0DF4`): marker → id by the `fn3DDC` link.
    /// 4. `first = id`, `+0x48` fresh, `+0x4E = −1`, `+0x2E = 0`.
    fn hand_off(&mut self, i: usize, run: i32) {
        if self.lizards[i].frame != 0 {
            let marker = if !self.valid(run) { 0 } else { l135::marker_of(run, |f| self.valid(f)) };
            self.jump_linked(i, marker);
        }
        let first = if self.valid(run) { run } else { 0 };
        self.step_to(i, first);
        let last = self.last_of(first);
        let l = &mut self.lizards[i];
        l.first = first;
        l.last = last;
        l.fresh = true;
        l.queued = -1;
    }

    /// `+0xCC` = L132 `fn0D3C` @0D3C: jump to a frame, registering the
    /// current one onto it through `fn3F2E` when both are valid (`+0x80` is
    /// 1 after the reset and this class never clears it). NOT the centre
    /// link — see the 2026-09-29 erratum.
    fn jump_linked(&mut self, i: usize, to: i32) {
        let (from, flip) = (self.lizards[i].frame, self.lizards[i].flip);
        if self.valid(from) && self.valid(to) {
            let (dx, dy, flip2) = self.shared_link(from, to, flip);
            let l = &mut self.lizards[i];
            l.x += dx;
            l.y += dy;
            l.flip = flip2;
        }
        self.lizards[i].frame = to;
    }

    /// `+0xD0` = L132 `fn0DF4` @0DF4: step to a frame by the `fn3DDC` link
    /// when both are valid.
    fn step_to(&mut self, i: usize, to: i32) {
        let (from, flip) = (self.lizards[i].frame, self.lizards[i].flip);
        if self.valid(from) && self.valid(to) {
            let d = self.link(from, to, flip);
            let l = &mut self.lizards[i];
            l.x += d.0;
            l.y += d.1;
        }
        self.lizards[i].frame = to;
    }

    /// `+0x84` = L132 `fn0316` @0316: one frame.
    ///
    /// - A run the previous call queued in `+0x4E` starts NOW, through the
    ///   same linked `fn028A` hand-off as a `SetRun`.
    /// - Unless fresh, step `frame → frame + 1` by the link while
    ///   `first <= frame < last`.
    /// - Then, in the SAME call, if the frame is `last` (or out of the
    ///   run): repeats left (`+0x4C >= 1`) → decrement and queue `first`
    ///   in `+0x4E`; else (empty queue) `+0x50` clear → raise `+0x46`;
    ///   `+0x50` set → clear it, `+0x54` (L135 `fn525A`: `+0x2E = 1`) and
    ///   queue frame 0.
    ///
    /// So a finished run raises `+0x46` on the very call that steps onto
    /// its last frame, and a handler that `SetRun`s on it never shows that
    /// frame — it only registers from it. A repeating loop DOES show
    /// `last`, then hands off `last → first − 1 → first` on the next call:
    /// run 2's `5 → 1` registers on the shared body (art 4, 0 px) and
    /// `1 → 2` links (+2, −4), so a cycle travels (+17, 0) — not the
    /// (+15, +4) of `centre(last) − centre(first)`.
    ///
    /// `+0x4A` (the hold counter) is zeroed by the reset and never armed by
    /// this class. A queued run (`+0x11C` non-empty, `+0x120` pops it) is
    /// handed off like a repeat, without raising `+0x46`.
    fn advance(&mut self, i: usize) {
        self.lizards[i].wrapped = false;
        if self.lizards[i].queued != -1 {
            let r = self.lizards[i].queued;
            self.hand_off(i, r);
        }
        let (frame, first, last) = {
            let l = &self.lizards[i];
            (l.frame, if self.valid(l.first) { l.first } else { 0 }, l.last)
        };
        if self.lizards[i].fresh {
            self.lizards[i].fresh = false;
        } else if frame < last && first <= frame {
            self.step_to(i, frame + 1);
        }
        let frame = self.lizards[i].frame;
        if last <= frame || frame < first {
            let l = &mut self.lizards[i];
            if l.repeat < 1 {
                if let Some(next) = l.queue.take() {
                    l.queued = next;
                } else if !l.remove_at_end {
                    l.wrapped = true;
                } else {
                    // GAP(+0x2E): `fn525A` flags the sprite for removal
                    // from the canvas list (L135 @2791 → `fn4D3E`); the
                    // port has no sprite list, and frame 0 draws nothing.
                    l.remove_at_end = false;
                    l.queued = 0;
                }
            } else {
                if l.repeat != 0x7fff {
                    l.repeat -= 1;
                }
                l.queued = l.first;
            }
        }
    }

    fn set_pos(&mut self, i: usize, x: i32, y: i32) {
        let l = &mut self.lizards[i];
        l.x = x;
        l.y = y;
    }

    /// `+0x94` / `+0x98` (L132 `fn05D4` / `fn0718`): walk the run forward
    /// until the sprite leaves the field rect and stop it there.
    /// APPROXIMATION — clamps the centre so the frame fits.
    fn clamp_into_field(&mut self, i: usize) {
        let f = self.lizards[i].frame;
        let (w, h) = self.bounds(f);
        let l = &mut self.lizards[i];
        let (hw, hh) = (w >> 1, h >> 1);
        if l.x - hw < LEFT {
            l.x = LEFT + hw;
        }
        if l.x + hw > RIGHT {
            l.x = RIGHT - hw;
        }
        if l.y - hh < TOP {
            l.y = TOP + hh;
        }
        if l.y + hh > BOTTOM {
            l.y = BOTTOM - hh;
        }
    }

    /// L132 `fn4590` on the drawn rect vs the field rect — state 5's exit.
    fn on_field(&self, i: usize) -> bool {
        let l = &self.lizards[i];
        let (w, h) = self.bounds(l.frame);
        let (hw, hh) = (w >> 1, h >> 1);
        l.x + hw > LEFT && l.x - hw < RIGHT && l.y + hh > TOP && l.y - hh < BOTTOM
    }

    /// `+0xC0` (L132 `fn0C7E`) → the lizard's own `+0x86` scratch.
    fn save_state(&mut self, i: usize) {
        let l = &mut self.lizards[i];
        l.save = Saved { frame: l.frame, first: l.first, x: l.x, y: l.y, flip: l.flip };
    }

    /// `+0xC4` (L132 `fn0CB6`).
    fn restore_state(&mut self, i: usize) {
        let s = self.lizards[i].save;
        let last = self.last_of(s.first);
        let l = &mut self.lizards[i];
        l.frame = s.frame;
        l.first = s.first;
        l.last = last;
        l.x = s.x;
        l.y = s.y;
        l.flip = s.flip;
    }

    /// Set the flip slot to match `+0x122`, the way every enter does with
    /// its inline `if (flip != facing) flip ^= 1`.
    fn face(&mut self, i: usize, facing: i32) {
        let l = &mut self.lizards[i];
        l.facing = facing;
        l.flip = facing != 0;
    }

    fn slot_of(&self, i: usize) -> usize {
        self.lizards[i].idx as usize
    }

    // ---- sound -----------------------------------------------------------

    /// `fn35` @1760. `idx < 0` stops the channel. Otherwise a cue plays
    /// when the last index played was higher, or the running clip has 180 ms
    /// or less left; the 180 is `− 0xB4` on the computed busy-until.
    fn snd(&mut self, ctx: &mut Ctx, idx: i32) {
        if idx < 0 {
            self.snd_last = -1;
            return;
        }
        let free = self.snd_last < 0 || idx < self.snd_last || self.snd_busy <= self.now;
        if !free {
            return;
        }
        let Some(&id) = SOUND_TABLE.get(idx as usize) else { return };
        let len = self.pack.sound_ms(id) as i64;
        if len == 0 {
            // dead slots (2006/2009..2012 never shipped a file) still take
            // the channel in the original; here nothing plays.
            self.snd_busy = self.now;
            self.snd_last = idx;
            return;
        }
        self.snd_busy = self.now + len - SND_TAIL_MS;
        self.snd_last = idx;
        ctx.sounds.push(id);
    }

    /// The tail every non-transitioning branch falls into (`LAB_00224b14`):
    /// the absolute frame number selects a cue.
    fn sound_tail(&mut self, i: usize, ctx: &mut Ctx) {
        let f = self.lizards[i].frame;
        let idx: i32 = match f {
            0x39 | 0x44 | 0x4f | 0x5b | 0x66 | 0x71 | 0x7b | 0x85 | 0x90 | 0xd4 | 0xfb | 0x2e5
            | 0x304 => 1,
            0x9d | 0x169 | 0x31a => 9,
            0xe6 => 3,
            0x165 | 0x265 | 0x2b2 => 8,
            0x182 | 0x188 | 0x34b | 0x352 | 0x355 => 10,
            0x1b0 | 0x239 | 0x254 | 0x272 | 0x28b | 0x294 | 0x29d | 0x2ad | 0x2c1 => {
                if ctx.rng.pct(2) == 0 {
                    12
                } else {
                    11
                }
            }
            0x1b3 => 8,
            0x1c1 | 0x1c9 | 0x212 => 2,
            0xb8 | 0xc0 | 0xc8 => 0,
            0x1ff => 7,
            0x3c2 | 0xaf => {
                if ctx.rng.pct(3) == 0 {
                    7
                } else {
                    6
                }
            }
            0x386 | 0x388 | 0x38a | 0x38c | 0x390 | 0x392 | 0x394 | 0x396 | 0x397 | 0x399
            | 0xec | 0xee | 0x129 | 0x12b | 0x12c | 0x12e | 0x131 => 5,
            0x138 => 4,
            _ => return,
        };
        self.snd(ctx, idx);
    }

    // ---- controller ------------------------------------------------------

    fn count(&self) -> usize {
        self.g045c.clamp(0, 10) as usize
    }

    /// `M130_fn10` @0100 with its colour tables: `LoadCLUT(+0x14A, 1500)`,
    /// copied into `+0x14E` and `+0x152` (L129 `fn3D3C`), and `M130_fn25`
    /// @55EC pushes clut 1500 into the lizard's own colour lookup.
    fn new_liz(&self, idx: i32) -> Liz {
        let mut l = Liz::new(idx);
        if let Some(&c) = self.cluts.get(&CLUT_BASE) {
            l.base = c;
            l.work = c;
            l.target = c;
            l.shown = c;
        }
        l
    }

    /// The `SpriteDraw::pal` lizard `i` draws with: each lizard wears its
    /// own blended table (`shown`), handed to the renderer as a per-sprite
    /// palette (`engine::DYN_PAL + i`, resolved by `dyn_palette`). Clut
    /// 1500 is the pack's base_clut, so a lizard still wearing it exactly
    /// draws baked (0).
    fn pal_of(&self, i: usize) -> u16 {
        match self.cluts.get(&CLUT_BASE) {
            Some(b) if *b != self.lizards[i].shown => engine::DYN_PAL + i as u16,
            _ => 0,
        }
    }

    /// Rebuild the lizard array — `fn18`'s construction loop and the
    /// `g045A != g045C` restart in controller state 3.
    fn rebuild(&mut self) {
        self.g045a = (self.quantity / 10).max(1);
        self.g045c = self.g045a.min(10);
        self.lizards = (0..self.g045c).map(|i| self.new_liz(i + 1)).collect();
        self.request = [0; 11];
        self.busy = [false; 11];
        self.hand = [0; 11];
        self.holder = None;
        self.slot_used = [true; 10];
        self.slot_rect = [(0, 0, 0, 0); 10];
        self.g0453 = false;
        self.g0454 = false;
        self.g0456 = 0;
        self.c_state = 0;
        self.c_prev = 0;
        self.c_pending = true;
        self.c_deadline = 0;
        self.c_master = 0;
        self.snd_last = -1;
        self.snd_busy = 0;
    }

    fn c_set_state(&mut self, s: i32) {
        self.c_prev = self.c_state;
        self.c_state = s;
        self.c_pending = true;
    }

    /// `fn29` @09C6.
    fn controller(&mut self, ctx: &mut Ctx) {
        if self.c_pending {
            self.c_pending = false;
            // the only enter with a body is 0x8003.
            if self.c_state == 3 {
                self.g0456 = 0;
            }
        }
        if self.c_pending {
            return;
        }
        match self.c_state {
            0 => {
                if self.c_deadline <= self.now {
                    self.g030c = false;
                    self.c_deadline = 0;
                    for i in 0..=self.count() {
                        self.busy[i] = false;
                    }
                    self.c_master = self.now + MASTER_MS as i64;
                    // fn30 @0DB4 would scan the desktop here.
                    // GAP(desktop snapshot).
                    self.c_set_state(2);
                }
            }
            1 => self.c_set_state(0),
            2 => {
                if self.c_deadline <= self.now {
                    self.c_set_state(3);
                }
            }
            3 => self.controller_run(ctx),
            4 => {
                self.g0454 = false;
                self.c_set_state(0);
            }
            _ => {}
        }
    }

    /// Controller state 3's update — the whole running module, one frame.
    fn controller_run(&mut self, ctx: &mut Ctx) {
        if ctx.caps_lock && !self.caps_prev {
            self.g0456 = 2;
        }
        self.caps_prev = ctx.caps_lock;

        if self.c_master < self.now {
            self.g0454 = true;
        }
        if !self.g0453 && (ctx.rng.pct(RARE_EVENT_MOD) == 0 || self.g0450 == 0xb) {
            self.g0454 = true;
            self.g0453 = true;
        }
        if self.g045a != self.g045c {
            self.rebuild();
            return;
        }
        let count = self.count();
        let live = (1..=count).filter(|&i| self.busy[i]).count() as i32;

        if !(self.g0454 && live == 0 && !self.g0453) {
            if self.g0453 && self.g0454 && live < 3 {
                self.g0454 = false;
                let last_free = (1..=count).filter(|&i| !self.busy[i]).next_back();
                match last_free {
                    None => self.g0453 = false,
                    Some(i) => {
                        self.request[i] = 7;
                        self.busy[i] = true;
                    }
                }
            }
            if !self.g0454 && !self.g0453 {
                let target =
                    if count < 5 { 4 } else { count as i32 - count as i32 / 3 };
                if live < target {
                    let mut i = 1;
                    while i <= count && self.busy[i] {
                        i += 1;
                    }
                    if i <= count {
                        self.request[i] = 1;
                    }
                }
            }
            for i in 0..count {
                self.run_liz(i, ctx);
            }
        } else {
            self.c_set_state(4);
        }
    }

    // ---- the lizard state machine (M130_fn17 @057E) ----------------------

    fn set_state(&mut self, i: usize, s: i32) {
        let l = &mut self.lizards[i];
        l.prev = l.state;
        l.state = s;
        l.pending = true;
    }

    /// The camouflage blend at the top of `M130_fn17` @057E, gated on
    /// `+0x158 < 20` and `+0x15E < now` (and on pixel depth > 7 —
    /// `g048E`; the port always draws in direct colour, and the golden
    /// captures are 8-bit, so the gate is always open here).
    ///
    /// For each of the 20 entries and each channel, in 16-bit units:
    /// `work = base + fade * ((target − base) / 20)` — the difference taken
    /// as an int of the two unsigned components, divided with truncation
    /// toward zero, times the step number BEFORE it is incremented. Then
    /// `fade += 1`; if that makes 20 the table is copied into `base`
    /// (`fn3D3C`) and NOT applied, otherwise `M130_fn24` @5364 applies it
    /// to the sprite. So step 0 re-applies `base` (a visible change only
    /// when the previous fade's unapplied 19th step is still pending),
    /// steps 1..18 are the visible blend, and step 19 only moves `base`:
    /// the lizard settles at 18/20 of the way and the next fade starts from
    /// 19/20. GOLDEN `chameleon-long.mp4` f2753..f2846 (one lizard toward
    /// clut 1500, 10 fps): 19 changes, the first continuing the previous
    /// fade's direction, then 18 linear ones, one per 5.17 frames, ending
    /// at R 71 (of 243 → 51) — see `camo_fade_matches_the_golden`.
    fn camo_fade(&mut self, i: usize) {
        let now = self.now;
        let l = &mut self.lizards[i];
        if l.fade >= FADE_STEPS || l.colour_due >= now {
            return;
        }
        l.colour_due = now + COLOUR_HOLD_MS as i64;
        let f = l.fade;
        for k in 0..FADE_ENTRIES {
            for c in 0..3 {
                let b = l.base[k][c] as i32;
                let d = (l.target[k][c] as i32 - b) / FADE_STEPS;
                l.work[k][c] = (b + f * d) as u16;
            }
        }
        l.fade += 1;
        if l.fade == FADE_STEPS {
            l.base = l.work;
        } else {
            l.shown = l.work;
        }
    }

    /// The re-roll shared by state 3's update and state 5's update:
    /// `do { c = 1500 + RandomBelow(9) } while (c == current)`.
    fn camo_roll(&mut self, i: usize, ctx: &mut Ctx) {
        if self.lizards[i].fade <= FADE_STEPS - 1 {
            return;
        }
        let cur = self.lizards[i].clut;
        let mut next = cur;
        while next == cur {
            next = CLUT_BASE + ctx.rng.pct(CLUT_COUNT) as i32;
        }
        // `LoadCLUT(+0x152, next)`; a clut the pack lacks leaves the old
        // target in place.
        let target = self.cluts.get(&next).copied();
        let l = &mut self.lizards[i];
        if let Some(t) = target {
            l.target = t;
        }
        l.clut = next;
        l.colour_due = self.now + COLOUR_HOLD_MS as i64;
        l.fade = 0;
    }

    /// L135 `fn5340`: fire the enter if one is pending, then the update.
    fn run_liz(&mut self, i: usize, ctx: &mut Ctx) {
        self.camo_fade(i);
        if self.lizards[i].pending {
            self.lizards[i].pending = false;
            let s = self.lizards[i].state;
            self.enter(i, s, ctx);
        }
        if self.lizards[i].pending {
            return; // an enter that transitioned skips the update
        }
        let s = self.lizards[i].state;
        self.update(i, s, ctx);
    }

    /// `now >= +0x15A`? Every update opens with this and then re-arms.
    fn due(&mut self, i: usize) -> bool {
        if self.now < self.lizards[i].deadline {
            return false;
        }
        let d = self.lizards[i].delay;
        self.lizards[i].deadline = self.now + d;
        true
    }

    fn enter(&mut self, i: usize, s: i32, ctx: &mut Ctx) {
        match s {
            0 => {
                self.lizards[i].delay = 100;
            }
            2 => {
                let to = self.lizards[i].to;
                self.set_pos(i, to.0, to.1);
                self.set_run(i, R_WALK);
                self.clamp_into_field(i);
                let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (x, y);
            }
            3 => {
                // @0EA2: RandomBelow(100) vs Zest − 10 decides whether the
                // idle pool arms a delay at all.
                let r = ctx.rng.pct(100) as i32;
                if self.zest - 10 < r {
                    let ix = (self.zest / 20).clamp(0, 5) as usize;
                    let pool =
                        if ix >= IDLE_POOL.len() { IDLE_POOL_OVERRUN } else { IDLE_POOL[ix] };
                    let delay = if pool == 0 {
                        0
                    } else {
                        let half = (pool >> 1) as u32;
                        (half as i64) + ctx.rng.pct(half.max(1)) as i64
                    };
                    self.lizards[i].deadline = self.now + delay;
                }
                self.set_run(i, R_IDLE_B);
                let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (x, y);
            }
            4 => {
                let old = self.lizards[i].facing;
                let slot = self.slot_of(i);
                self.request[slot] = 0;
                self.busy[slot] = true;
                let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (x, y);
                let tx = LEFT + 0x78 + ctx.rng.pct(((RIGHT - LEFT) - 0xa0).max(1) as u32) as i32;
                self.lizards[i].to.0 = tx;
                let facing = i32::from(tx < self.lizards[i].from.0);
                self.lizards[i].facing = facing;
                if facing == old {
                    self.set_run(i, R_WALK);
                    self.arm_walk(i, tx, -1);
                } else {
                    self.set_run(i, R_TURN);
                }
                let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (x, y);
                let d = self.lizards[i].delay;
                self.lizards[i].deadline = self.now + d;
            }
            5 => {
                let (_, h) = self.bounds(R_ARRIVE);
                let facing = self.lizards[i].facing;
                let x = if facing == 0 { LEFT - 0x28 } else { RIGHT + 0x28 };
                let span = ((BOTTOM - TOP) - h).max(1);
                let y = TOP + h / 2 + ctx.rng.pct(span as u32) as i32;
                self.lizards[i].to = (x, y);
                self.set_pos(i, x, y);
                self.set_run(i, R_ARRIVE);
                self.clamp_into_field(i);
                let (px, py) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (px, py);
                self.lizards[i].deadline = 0;
            }
            6 | 9 => {
                let p = self.lizards[i].partner;
                let me = self.lizards[i].idx;
                if (1..=10).contains(&p) {
                    self.request[p as usize] = if s == 6 { 2 } else { 3 };
                    self.hand[p as usize] = me;
                }
                self.hand[me as usize] = 0;
                self.lizards[i].handshake = p;
                let d = self.lizards[i].delay;
                self.lizards[i].deadline = self.now + d * 5;
            }
            0xb => {
                let p = self.lizards[i].partner;
                self.pair_run(i, R_FOLDER_A, p);
                // @22C2: the first live desktop record wins, unless someone
                // already holds one. GAP(desktop snapshot) leaves none live.
                let found = (0..10).find(|&k| self.slot_live(k));
                match found {
                    Some(k) if self.holder.is_none() => {
                        self.lizards[i].slot = k as i32;
                        self.holder = Some(k);
                        self.lizards[i].holding = true;
                    }
                    _ => self.lizards[i].slot = -1,
                }
            }
            0xc => {
                self.set_run(i, R_WALK);
                let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                self.lizards[i].from = (x, y);
                let edge = if self.lizards[i].flip { LEFT } else { RIGHT };
                self.arm_walk(i, edge, 3);
            }
            0xd | 0xe => {
                let want = if s == 0xd { R_BLINK_A } else { R_BLINK_B };
                let cur = self.lizards[i].first;
                if cur == R_BLINK_B || cur == R_BLINK_A {
                    self.set_run(i, want);
                } else {
                    // `+0x80(0x11C, want, −1)`: the chain's first run plays
                    // and `advance` hands it off into `want`.
                    self.set_run_chain(i, R_BLINK_IN, want);
                }
            }
            0xf => self.set_run(i, R_YAWN),
            0x10 => self.set_run(i, R_LOOK_LEFT),
            0x11 => self.set_run(i, R_LOOK_UP),
            // FolderSex (0x12) / TossEat (0x13) / TongueEat (0x14): climb on
            // a desktop slot. GAP(desktop snapshot) means none is ever free,
            // so all three bail straight back to state 0, exactly as the C
            // does when `fn30`'s scan finds nothing.
            0x12 => {
                self.lizards[i].partner = -1;
                // @2E06: up to 0x34 tries of RandomBelow(10).
                let mut tries = 0;
                loop {
                    let k = ctx.rng.pct(10) as i32;
                    if k > 9 || self.lizards[i].partner != -1 {
                        break;
                    }
                    if !self.slot_used[k as usize] {
                        self.lizards[i].partner = k;
                    }
                    tries += 1;
                    if tries > 0x33 {
                        break;
                    }
                }
                if self.lizards[i].partner >= 0 {
                    self.mount_slot(i, R_SEX_CLIMB, 2, 2);
                }
            }
            0x13 | 0x14 => {
                // @3462/@3922: the first free slot whose rect lies wholly
                // inside the field, marking each one used as it goes.
                self.lizards[i].partner = -1;
                let mut k = 0;
                while k < 10 && self.lizards[i].partner == -1 {
                    if !self.slot_used[k] {
                        let r = self.slot_rect[k];
                        if r.0 >= LEFT && r.2 <= RIGHT && r.1 >= TOP && r.3 <= BOTTOM {
                            self.lizards[i].partner = k as i32;
                        }
                        self.slot_used[k] = true;
                    }
                    k += 1;
                }
                if self.lizards[i].partner >= 0 {
                    if s == 0x14 {
                        let r = ctx.rng.pct(TONGUE_RUNS.len() as u32) as usize;
                        self.lizards[i].activity = TONGUE_RUNS[r];
                        let run = self.lizards[i].activity;
                        self.mount_slot(i, run, 6, 2);
                    } else {
                        self.mount_slot(i, R_TOSS_CLIMB, 6, 1);
                    }
                }
            }
            0x15 => self.set_run(i, R_VOM_A),
            0x16 => {
                self.lizards[i].delay = 0x4b;
                self.set_run(i, R_FIRE);
                self.lizards[i].repeat = -1;
            }
            0x18 | 0x19 => self.enter_pair_response(i, s),
            0x1a => self.lizards[i].remove_at_end = true,
            _ => {}
        }
        if !self.lizards[i].pending {
            self.sound_tail(i, ctx);
        }
    }

    /// `+0x4C = |(from.x − target)/cycle_dx| + bias`; `+0x46` cleared when
    /// the walk has any length. `bias` is −1 for state 4, +3 for state 0xC.
    fn arm_walk(&mut self, i: usize, target: i32, bias: i32) {
        let dx = self.cycle_dx(i, R_WALK);
        if dx == 0 {
            return;
        }
        let n = ((self.lizards[i].from.0 - target) / dx).abs() + bias;
        let l = &mut self.lizards[i];
        l.repeat = n;
        if n > 0 {
            l.wrapped = false;
        }
    }

    /// A desktop slot holds a live rect. GAP(desktop snapshot): never.
    fn slot_live(&self, k: usize) -> bool {
        let r = self.slot_rect[k];
        r.2 > r.0 && r.3 > r.1
    }

    /// The shared body of enters 0x8012/0x8013/0x8014: face by the slot's
    /// VERTICAL midpoint (quirk kept), start `run`, place on the slot rect,
    /// nudge by (dy, ±dx), then hand off to the walk run.
    fn mount_slot(&mut self, i: usize, run: i32, dy: i32, dx: i32) {
        let k = self.lizards[i].partner as usize;
        let r = self.slot_rect[k];
        let icon_mid_y = (r.1 + r.3) / 2;
        let field_mid_y = (TOP + BOTTOM) / 2;
        let facing = i32::from(icon_mid_y < field_mid_y);
        self.face(i, facing);
        self.set_run(i, run);
        // `+0xE0` places the sprite on the slot rect.
        self.set_pos(i, (r.0 + r.2) / 2, icon_mid_y);
        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
        let flip = self.lizards[i].flip;
        self.lizards[i].from = (x + if flip { dx } else { -dx }, y + dy);
        let from = self.lizards[i].from;
        self.set_pos(i, from.0, from.1);
        self.jump_linked(i, R_IDLE_A);
        self.set_run(i, R_WALK);
        self.clamp_into_field(i);
        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
        self.lizards[i].from = (x, y);
        let me = self.slot_of(i);
        self.request[me] = 0;
        let d = self.lizards[i].delay;
        self.lizards[i].deadline = self.now + d;
    }

    /// `+0xB0` (L132 `fn0A6C`): start the same run on me and on my partner.
    fn pair_run(&mut self, i: usize, run: i32, partner: i32) {
        if let Some(j) = self.index_of(partner) {
            self.set_run(j, run);
        }
        self.set_run(i, run);
    }

    fn index_of(&self, one_based: i32) -> Option<usize> {
        if one_based < 1 {
            return None;
        }
        let j = (one_based - 1) as usize;
        if j < self.lizards.len() {
            Some(j)
        } else {
            None
        }
    }

    /// Enters 0x8018 / 0x8019: the lizard the initiator called sits down on
    /// the initiator's frame and hands control back through state 0x17.
    fn enter_pair_response(&mut self, i: usize, s: i32) {
        let me = self.lizards[i].idx as usize;
        let p = self.hand[me];
        self.lizards[i].partner = p;
        let Some(j) = self.index_of(p) else {
            self.set_state(i, 0);
            return;
        };
        let pflip = self.lizards[j].flip;
        let facing = if s == 0x18 { i32::from(pflip) } else { i32::from(!pflip) };
        self.face(i, facing);
        let (px, py) = (self.lizards[j].x, self.lizards[j].y);
        self.lizards[i].from = (px, py);
        self.set_pos(i, px, py);
        self.set_run(i, R_IDLE_A);
        let anchor = if s == 0x18 { R_SLAVE_ANCHOR } else { R_RIDE_JOIN };
        self.jump_linked(i, anchor);
        // `+0xE8` insets the anchor rect; an inset does not move its centre,
        // so the centre is simply where the sprite already is.
        let (mut cx, mut cy) = (self.lizards[i].x, self.lizards[i].y);
        cy += if s == 0x18 { 5 } else { 6 };
        cx += if self.lizards[i].flip { 2 } else { -2 };
        self.set_pos(i, cx, cy);
        self.set_run(i, R_WALK);
        self.clamp_into_field(i);
        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
        self.lizards[i].from = (x, y);
        let slot = self.slot_of(i);
        self.request[slot] = 0;
        let d = self.lizards[i].delay;
        self.lizards[i].deadline = self.now + d;
    }

    fn update(&mut self, i: usize, s: i32, ctx: &mut Ctx) {
        match s {
            0 => return self.state0(i, ctx),
            2 => {
                if !self.due(i) {
                    return;
                }
                if self.g0456 == 2 {
                    let x = self.lizards[i].x;
                    if x > LEFT - 0x28 && x < RIGHT + 0x28 {
                        self.g0456 = 1;
                        return self.set_state(i, 3);
                    }
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    // @0E8A: the test is against run 2, the run the enter
                    // started — the SECOND wrap (run 9) is what leaves.
                    if self.lizards[i].first != R_WALK {
                        return self.set_state(i, 3);
                    }
                    self.set_run(i, R_IDLE_A);
                }
            }
            3 => return self.state3(i, ctx),
            4 => {
                if !self.due(i) {
                    return;
                }
                if self.g0456 == 2 {
                    self.g0456 = 1;
                    return self.set_state(i, 3);
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    match self.lizards[i].first {
                        R_TURN => {
                            self.set_run(i, R_WALK);
                            let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                            self.lizards[i].from = (x, y);
                            let tx = self.lizards[i].to.0;
                            self.arm_walk(i, tx, -1);
                            self.advance(i);
                        }
                        R_IDLE_A => return self.set_state(i, 3),
                        _ => self.set_run(i, R_IDLE_A),
                    }
                }
            }
            5 => return self.state5(i, ctx),
            6 => return self.state6(i, ctx),
            9 => return self.state9(i, ctx),
            0xa => return self.state_a(i, ctx),
            0xb => {
                if !self.due(i) {
                    return;
                }
                if self.lizards[i].slot == -1 {
                    self.set_run(i, R_PAIR_A);
                    return self.set_state(i, 10);
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    match self.lizards[i].first {
                        R_FOLDER_A => {
                            self.holder = None;
                            self.lizards[i].holding = false;
                            self.set_run(i, R_FOLDER_B);
                        }
                        R_FOLDER_B => {
                            let run =
                                if ctx.rng.pct(2) == 0 { R_FOLDER_B } else { R_FOLDER_C };
                            self.set_run(i, run);
                        }
                        R_FOLDER_C => {
                            self.holder = None;
                            self.lizards[i].holding = false;
                            let p = self.lizards[i].partner;
                            if (1..=10).contains(&p) {
                                self.request[p as usize] = 6;
                                self.hand[p as usize] = 0;
                            }
                            if let Some(j) = self.index_of(p) {
                                self.set_pos(j, -1000, -1000);
                            }
                            return self.set_state(i, 0);
                        }
                        _ => {}
                    }
                }
            }
            0xc => {
                if !self.due(i) {
                    return;
                }
                if self.g0456 == 2 {
                    self.g0456 = 1;
                    return self.set_state(i, 3);
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    return self.set_state(i, 0);
                }
            }
            0xd | 0xe => {
                if !self.due(i) {
                    return;
                }
                if self.pair_broken(i) {
                    return self.set_state(i, 0xf);
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    // the `+0x80` chain never raises `+0x46` on 0x11C: the
                    // queue carries it into its partner run (`advance`).
                    match ctx.rng.pct(4) {
                        0 | 1 => return self.set_state(i, 0xf),
                        2 => return self.set_state(i, 0xd),
                        _ => return self.set_state(i, 0xe),
                    }
                }
            }
            0xf => {
                if !self.due(i) {
                    return;
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    return self.set_state(i, 3);
                }
            }
            0x10 | 0x11 => {
                if !self.due(i) {
                    return;
                }
                if self.pair_broken(i) {
                    return self.set_state(i, 3);
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    return self.set_state(i, 3);
                }
            }
            0x12 | 0x13 | 0x14 => {
                if !self.due(i) {
                    return;
                }
                if self.lizards[i].partner == -1 {
                    return self.set_state(i, 0);
                }
                self.advance(i);
                if !self.lizards[i].wrapped {
                    self.sound_tail(i, ctx);
                    return;
                }
                let run = self.lizards[i].first;
                // the climb run each of the three uses on the way up
                let climb = match s {
                    0x12 => R_SEX_CLIMB,
                    0x13 => R_TOSS_CLIMB,
                    _ => self.lizards[i].activity,
                };
                match run {
                    0 => return self.set_state(i, 0),
                    R_WALK => self.set_run(i, R_IDLE_A),
                    R_IDLE_A => {
                        if self.holder.is_none() {
                            self.holder = Some(self.lizards[i].partner as usize);
                            self.lizards[i].holding = true;
                            self.set_run(i, climb);
                        } else {
                            self.set_run(i, R_IDLE_A);
                        }
                    }
                    _ if s == 0x12 && run == R_SEX_CLIMB => self.set_run(i, R_SEX_LOOP),
                    _ if s == 0x12 && run == R_SEX_LOOP => {
                        let next = if ctx.rng.pct(10) == 0 { R_SEX_END } else { R_SEX_LOOP };
                        self.set_run(i, next);
                    }
                    _ if s == 0x12 && run == R_SEX_END => {
                        self.release_slot(i);
                        let next = if ctx.rng.pct(2) != 0 { 0x11 } else { 3 };
                        return self.set_state(i, next);
                    }
                    _ if s == 0x13 && run == R_TOSS_CLIMB => {
                        self.release_slot(i);
                        self.set_run(i, R_TOSS_END);
                    }
                    _ if s == 0x14 && TONGUE_RUNS.contains(&run) => {
                        self.release_slot(i);
                        self.set_run(i, R_TOSS_END);
                    }
                    R_TOSS_END => {
                        let next = if ctx.rng.pct(2) != 0 { 0x11 } else { 3 };
                        return self.set_state(i, next);
                    }
                    _ => {}
                }
            }
            0x15 => return self.state15(i, ctx),
            0x16 => return self.state16(i, ctx),
            0x17 => return self.state17(i, ctx),
            0x18 | 0x19 => {
                if !self.due(i) {
                    return;
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    if self.lizards[i].first == R_IDLE_A {
                        let p = self.lizards[i].partner;
                        let me = self.lizards[i].idx;
                        if (1..=10).contains(&p) {
                            self.request[p as usize] = 4;
                            self.hand[p as usize] = me;
                        }
                        self.save_state(i);
                        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
                        self.lizards[i].from = (x, y);
                        return self.set_state(i, 0x17);
                    }
                    self.set_run(i, R_IDLE_A);
                }
            }
            0x1a => {
                if !self.due(i) {
                    return;
                }
                self.advance(i);
                if self.lizards[i].wrapped {
                    self.lizards[i].remove_at_end = false;
                    self.lizards[i].from = (-1000, -1000);
                    self.set_pos(i, -1000, -1000);
                    return self.set_state(i, 0);
                }
            }
            _ => {}
        }
        self.sound_tail(i, ctx);
    }

    /// `*g0310 = 0; g0310 = 0; +0x140 = 0` — give the desktop slot back.
    fn release_slot(&mut self, i: usize) {
        if let Some(k) = self.holder.take() {
            self.slot_used[k] = false;
        }
        self.lizards[i].holding = false;
    }

    /// `+0x13D && g0120[me] == +0x13D` — the pair states' abort test.
    fn pair_broken(&self, i: usize) -> bool {
        let l = &self.lizards[i];
        l.handshake != 0 && self.hand[l.idx as usize] == l.handshake
    }

    /// State 0 update: read my request slot and dispatch.
    fn state0(&mut self, i: usize, ctx: &mut Ctx) {
        let me = self.lizards[i].idx as usize;
        self.busy[me] = false;
        let req = self.request[me];
        match req {
            2 | 3 => {
                self.request[me] = 0;
                let d = self.lizards[i].delay;
                self.lizards[i].deadline = self.now + d;
                self.busy[me] = true;
                self.set_state(i, if req == 2 { 0x18 } else { 0x19 });
            }
            5 => {
                self.request[me] = 0;
                self.lizards[i].deadline = 0;
                self.busy[me] = true;
                self.set_state(i, 0x1a);
            }
            7 => {
                self.request[me] = 0;
                self.busy[me] = true;
                self.lizards[i].f13e = false;
                self.lizards[i].handshake = 0;
                self.lizards[i].partner = 0;
                let facing = ctx.rng.pct(2) as i32;
                self.face(i, facing);
                self.set_state(i, 5);
            }
            1 => {
                self.request[me] = 0;
                self.busy[me] = true;
                self.lizards[i].f13e = false;
                self.lizards[i].handshake = 0;
                self.lizards[i].partner = 0;
                let facing = ctx.rng.pct(2) as i32;
                // the off-screen spawn x written here is dead — every state
                // that follows overwrites +0x134 with the live position.
                let spawn_x = if facing == 0 { LEFT - 0x28 } else { RIGHT + 0x28 };
                let y = TOP
                    + 0x28
                    + ctx.rng.pct(((BOTTOM - TOP) - 0x50).max(1) as u32) as i32;
                let x = LEFT + 0x28 + ctx.rng.pct(((RIGHT - LEFT) - 0x50).max(1) as u32) as i32;
                self.lizards[i].from = (spawn_x, y);
                self.lizards[i].to = (x, y);
                self.face(i, facing);
                let d = self.lizards[i].delay;
                self.lizards[i].deadline = self.now + d + ctx.rng.pct(5000) as i64;
                let next = if self.g0450 == 0xc {
                    0x12
                } else if ctx.rng.pct(3) != 0 {
                    2
                } else if ctx.rng.pct(3) != 0 {
                    0x14
                } else if ctx.rng.pct(2) != 0 {
                    0x12
                } else {
                    0x13
                };
                self.set_state(i, next);
            }
            _ => self.sound_tail(i, ctx),
        }
    }

    /// State 3 update: the chooser.
    fn state3(&mut self, i: usize, ctx: &mut Ctx) {
        if self.g0456 != 0 {
            self.g0456 = 0;
            return self.set_state(i, 0x16);
        }
        if self.now < self.lizards[i].deadline && !self.g0454 {
            return;
        }
        self.camo_roll(i, ctx);
        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
        self.lizards[i].from = (x, y);
        if x < LEFT + 0x50 || x > RIGHT - 0x50 {
            let s = if ctx.rng.pct(2) == 0 { 4 } else { 0xc };
            return self.set_state(i, s);
        }
        let d = self.lizards[i].delay;
        self.lizards[i].deadline = self.now + d;

        // GAP(control 3): g0450 is always −1, so this whole ladder is dead
        // in the shipped product. Ported as written.
        match self.g0450 {
            1 => return self.set_state(i, 0xc),
            2 => return self.set_state(i, 3),
            3 => return self.set_state(i, 4),
            4 => return self.set_state(i, 0x10),
            5 => return self.set_state(i, 0xd),
            6 => return self.set_state(i, 0x16),
            7 => {
                if self.claim_partner(i) {
                    let p = self.lizards[i].partner as usize;
                    self.busy[p] = true;
                    return self.set_state(i, 6);
                }
                if self.claim_partner(i) {
                    let p = self.lizards[i].partner as usize;
                    self.busy[p] = true;
                    return self.set_state(i, 9);
                }
            }
            8 => {
                if self.claim_partner(i) {
                    let p = self.lizards[i].partner as usize;
                    self.busy[p] = true;
                    return self.set_state(i, 9);
                }
            }
            9 => return self.set_state(i, 0x11),
            10 => return self.set_state(i, 0x15),
            _ => {}
        }

        if self.zest > 0x5f && ctx.rng.pct(5) == 0 {
            return self.set_state(i, 0x16);
        }
        if self.g0454 || self.lizards[i].from.1 > BOTTOM {
            return self.set_state(i, 0xc);
        }
        let r = ctx.rng.pct(13) as i32;
        match r {
            0 | 1 => return self.set_state(i, 0xc),
            2 => return self.set_state(i, 3),
            3 => return self.set_state(i, 4),
            4 => return self.set_state(i, 0x10),
            5 => return self.set_state(i, 0xd),
            6 => return self.set_state(i, 0xe),
            7 => {
                if ctx.rng.pct(5) == 0 {
                    return self.set_state(i, 0x16);
                }
            }
            10 => {
                if ctx.rng.pct(4) == 0 {
                    return self.set_state(i, 0x11);
                }
            }
            11 | 12 => {
                if self.vommeter > 0x14 && (ctx.rng.pct(100) as i32) < self.vommeter {
                    return self.set_state(i, 0x15);
                }
            }
            8 => {
                // case 8 tries its own bound, then FALLS THROUGH to case 9's
                // shared block — exactly as the C's control flow does.
                let x = self.lizards[i].from.0;
                let ok = if self.lizards[i].facing == 0 {
                    x > LEFT + 0x28
                } else {
                    x < RIGHT - 0x28
                };
                if ok && self.claim_partner(i) {
                    let p = self.lizards[i].partner as usize;
                    self.busy[p] = true;
                    return self.set_state(i, 6);
                }
                return self.try_state9(i);
            }
            9 => return self.try_state9(i),
            _ => {}
        }
        self.sound_tail(i, ctx);
    }

    fn try_state9(&mut self, i: usize) {
        let x = self.lizards[i].from.0;
        let ok = if self.lizards[i].facing == 0 {
            x > LEFT + 0x28
        } else {
            x < RIGHT - 0x28
        };
        if !ok {
            return;
        }
        if self.claim_partner(i) {
            let p = self.lizards[i].partner as usize;
            self.busy[p] = true;
            self.set_state(i, 9);
        }
    }

    /// `M130_fn14` @0416: claim the first free slot as `+0x126`.
    fn claim_partner(&mut self, i: usize) -> bool {
        let count = self.count();
        let mut k = 1;
        while k <= count && self.busy[k] {
            k += 1;
        }
        if k > count {
            return false;
        }
        self.lizards[i].partner = k as i32;
        true
    }

    /// State 5 update: walk on from off-screen and keep walking until the
    /// whole sprite has left the field.
    fn state5(&mut self, i: usize, ctx: &mut Ctx) {
        if self.now < self.lizards[i].deadline {
            return;
        }
        self.camo_roll(i, ctx);
        let d = self.lizards[i].delay;
        self.lizards[i].deadline = self.now + d;
        self.advance(i);
        if self.lizards[i].wrapped {
            if !self.on_field(i) {
                let me = self.lizards[i].idx as usize;
                self.busy[me] = false;
                self.g0453 = false;
                self.snd(ctx, -1);
                self.g030c = false;
                return self.set_state(i, 0);
            }
            let run = if ctx.rng.pct(2) == 0 { R_ARRIVE } else { R_STROLL };
            self.set_run(i, run);
        }
        self.sound_tail(i, ctx);
    }

    /// State 6 — the "master" half of the mating pair.
    fn state6(&mut self, i: usize, ctx: &mut Ctx) {
        if !self.due(i) {
            return;
        }
        let me = self.lizards[i].idx as usize;
        let hs = self.lizards[i].handshake;
        let matched = hs != 0 && self.hand[me] == hs && self.lizards[i].wrapped;
        if !matched {
            let p = self.lizards[i].partner;
            let mirrored = self
                .index_of(p)
                .map(|j| self.lizards[j].partner != self.lizards[i].idx)
                .unwrap_or(true);
            if mirrored {
                self.lizards[i].partner = 0;
                return self.set_state(i, 3);
            }
        } else {
            self.lizards[i].handshake = 0;
            self.set_run(i, R_IDLE_B);
            let p = self.lizards[i].partner;
            if let Some(j) = self.index_of(p) {
                if self.lizards[j].flip != self.lizards[i].flip {
                    self.set_run(i, R_FACE_OFF);
                }
            }
        }
        if self.lizards[i].handshake != 0 {
            self.advance(i);
            if self.lizards[i].wrapped {
                let span = (0x69 - self.zest).max(1) as u32;
                if ctx.rng.pct(span) != 0 {
                    return;
                }
                if self.hand[me] == self.lizards[i].handshake {
                    return;
                }
                let run = if ctx.rng.pct(2) == 0 { R_LOOK_UP } else { R_LOOK_LEFT };
                self.set_run(i, run);
            }
            return self.sound_tail(i, ctx);
        }
        match self.lizards[i].first {
            R_FACE_OFF => {
                self.advance(i);
                if self.lizards[i].wrapped {
                    self.set_run(i, R_IDLE_B);
                }
            }
            R_IDLE_B => {
                let p = self.lizards[i].partner;
                self.pair_run(i, R_MATE_A, p);
            }
            _ => {
                self.advance(i);
                if !self.lizards[i].wrapped {
                    return self.sound_tail(i, ctx);
                }
                match self.lizards[i].first {
                    R_MATE_A => {
                        let run = if ctx.rng.pct(2) == 0 { R_MATE_C } else { R_MATE_B };
                        self.set_run(i, run);
                    }
                    R_MATE_B => return self.finish_mate(i, true, ctx),
                    R_MATE_C => match ctx.rng.pct(6) {
                        0 | 1 | 2 => return self.finish_mate(i, false, ctx),
                        3 | 4 => self.set_run(i, R_MATE_D),
                        _ => {}
                    },
                    R_MATE_D => {
                        self.set_run(i, R_PAIR_A);
                        return self.set_state(i, 10);
                    }
                    _ => {}
                }
            }
        }
        self.sound_tail(i, ctx);
    }

    /// The shared tail of runs 0x39E / 0x3CB: hand the partner back its
    /// saved pose and post request 6 to it.
    fn finish_mate(&mut self, i: usize, was_b: bool, ctx: &mut Ctx) {
        self.lizards[i].was_mate_b = was_b;
        self.jump_linked(i, R_IDLE_B);
        let p = self.lizards[i].partner;
        if let Some(j) = self.index_of(p) {
            self.restore_state(j);
        }
        if (1..=10).contains(&p) {
            self.request[p as usize] = 6;
            let v = if was_b && ctx.rng.pct(3) == 0 { 0x16 } else { 3 };
            self.hand[p as usize] = v;
        }
        self.set_state(i, 3);
    }

    /// State 9 — the other half; rolls into the ride (0x0A) or the folder.
    fn state9(&mut self, i: usize, ctx: &mut Ctx) {
        if !self.due(i) {
            return;
        }
        let me = self.lizards[i].idx as usize;
        let hs = self.lizards[i].handshake;
        if hs == 0 || self.hand[me] != hs {
            let p = self.lizards[i].partner;
            let mirrored = self
                .index_of(p)
                .map(|j| self.lizards[j].partner != self.lizards[i].idx)
                .unwrap_or(true);
            if mirrored {
                self.lizards[i].partner = 0;
                return self.set_state(i, 3);
            }
        } else {
            self.set_run(i, R_IDLE_B);
            self.lizards[i].handshake = 0;
        }
        if self.lizards[i].handshake == 0 {
            if self.lizards[i].y < 200 && ctx.rng.pct(2) == 0 && self.holder.is_none() {
                return self.set_state(i, 0xb);
            }
            return self.set_state(i, 10);
        }
        self.advance(i);
        if self.lizards[i].wrapped {
            let span = (0x69 - self.zest).max(1) as u32;
            if ctx.rng.pct(span) != 0 {
                return;
            }
            if self.hand[me] == self.lizards[i].handshake {
                return;
            }
            let run = if ctx.rng.pct(10) == 0 { R_LOOK_UP } else { R_LOOK_LEFT };
            self.set_run(i, run);
        }
        self.sound_tail(i, ctx);
    }

    /// State 0x0A — the ride.
    fn state_a(&mut self, i: usize, ctx: &mut Ctx) {
        if !self.due(i) {
            return;
        }
        let first = self.lizards[i].first;
        if first == R_IDLE_A || first == R_IDLE_B {
            let p = self.lizards[i].partner;
            self.pair_run(i, R_RIDE_JOIN, p);
            let (x, y) = (self.lizards[i].x, self.lizards[i].y);
            self.lizards[i].from = (x, y);
            self.set_run(i, R_PAIR_A);
            return self.sound_tail(i, ctx);
        }
        self.advance(i);
        if !self.lizards[i].wrapped {
            return self.sound_tail(i, ctx);
        }
        match self.lizards[i].first {
            R_PAIR_A => self.set_run(i, R_PAIR_B),
            R_PAIR_B => {
                if ctx.rng.pct(2) == 0 {
                    // `+0x80(0x272, 0x294, 0x29D, −1)` — the chain's head.
                    self.set_run(i, R_PAIR_D);
                    let _ = (R_PAIR_E, R_PAIR_F);
                } else {
                    self.snd(ctx, -1);
                    self.set_run(i, R_PAIR_C);
                }
            }
            R_PAIR_D => self.set_run(i, R_PAIR_E),
            R_PAIR_E => self.set_run(i, R_PAIR_F),
            R_PAIR_F => {
                if ctx.rng.pct(2) == 0 {
                    self.snd(ctx, -1);
                    let dir = if self.lizards[i].flip { 1 } else { -1 };
                    let probe = self.lizards[i].x + dir * 0x16e;
                    let run = if probe >= LEFT && probe <= RIGHT { R_PAIR_H } else { R_PAIR_G };
                    self.set_run(i, run);
                } else {
                    self.set_run(i, R_PAIR_F);
                }
            }
            R_PAIR_C | R_PAIR_G | R_PAIR_H => {
                let was_h = self.lizards[i].first == R_PAIR_H;
                self.jump_linked(i, R_IDLE_B);
                let p = self.lizards[i].partner;
                if (1..=10).contains(&p) {
                    self.request[p as usize] = 6;
                }
                if was_h {
                    if (1..=10).contains(&p) {
                        self.hand[p as usize] = 0;
                    }
                    if let Some(j) = self.index_of(p) {
                        self.set_pos(j, -1000, -1000);
                    }
                } else {
                    if let Some(j) = self.index_of(p) {
                        self.restore_state(j);
                    }
                    if (1..=10).contains(&p) {
                        self.hand[p as usize] = 3;
                    }
                }
                return self.set_state(i, 3);
            }
            _ => {}
        }
        self.sound_tail(i, ctx);
    }

    /// State 0x15 — ThrowUp.
    fn state15(&mut self, i: usize, ctx: &mut Ctx) {
        if !self.due(i) {
            return;
        }
        if self.pair_broken(i) {
            return self.set_state(i, 3);
        }
        self.advance(i);
        if !self.lizards[i].wrapped {
            return self.sound_tail(i, ctx);
        }
        match self.lizards[i].first {
            R_VOM_A => self.set_run(i, R_VOM_B),
            R_VOM_B => {
                let run = if ctx.rng.pct(2) == 0 { R_VOM_C } else { R_VOM_B };
                self.set_run(i, run);
            }
            R_VOM_C => {
                if ctx.rng.pct(4) == 0 && self.claim_partner(i) {
                    return self.splash(i);
                }
                self.set_run(i, R_VOM_D);
            }
            R_VOM_D => {
                if ctx.rng.pct(2) == 0 {
                    if ctx.rng.pct(5) == 0 && self.claim_partner(i) {
                        return self.splash(i);
                    }
                    self.set_run(i, R_VOM_E);
                } else {
                    self.set_run(i, R_VOM_D);
                }
            }
            // 0xE0 is the VICTIM's run, so this branch is dead in the
            // original too — `splash` transitions before it can be reached.
            R_VOM_HIT_OTHER | R_VOM_E => return self.set_state(i, 3),
            _ => {}
        }
        self.sound_tail(i, ctx);
    }

    /// `+0xB4` (L132 `fn0A9C`): the victim copies my pose, then I take the
    /// FIRST pushed run and the victim the second. Arg order read off the
    /// raw listing at `00003E62` (`0xE0`, `0xDC`, this — pushed right to
    /// left), because Ghidra drops the shorts: I get 0xDC, it gets 0xE0.
    /// It then gets request 5 (state 0x1A, "loop once more and leave").
    fn splash(&mut self, i: usize) {
        let p = self.lizards[i].partner;
        if let Some(j) = self.index_of(p) {
            let (x, y, flip, frame, first) = {
                let l = &self.lizards[i];
                (l.x, l.y, l.flip, l.frame, l.first)
            };
            let last = self.last_of(first);
            {
                let v = &mut self.lizards[j];
                v.x = x;
                v.y = y;
                v.flip = flip;
                v.frame = frame;
                v.first = first;
                v.last = last;
            }
            self.set_run(j, R_VOM_HIT_OTHER);
        }
        self.set_run(i, R_VOM_HIT_SELF);
        self.advance(i);
        self.advance(i);
        if (1..=10).contains(&p) {
            self.request[p as usize] = 5;
            self.hand[p as usize] = self.lizards[i].idx;
            self.busy[p as usize] = true;
        }
        self.set_state(i, 3);
    }

    /// State 0x16 — the Caps Lock smite: the 44-frame fire arc at 75 ms.
    fn state16(&mut self, i: usize, ctx: &mut Ctx) {
        if !self.due(i) {
            return;
        }
        self.advance(i);
        let rel = self.lizards[i].frame - self.lizards[i].first;
        if self.lizards[i].first == R_FIRE && rel == 0xc && ctx.rng.pct(2) == 0 {
            self.set_run(i, R_FIRE_C);
            self.lizards[i].deadline += ctx.rng.pct(10_000) as i64;
        }
        let (x, y) = (self.lizards[i].x, self.lizards[i].y);
        self.lizards[i].from = (x, y);
        let rel = self.lizards[i].frame - self.lizards[i].first;
        if self.lizards[i].first == R_FIRE {
            if rel == 0x12 {
                self.lizards[i].deadline += ctx.rng.pct(10_000) as i64;
                self.set_run(i, R_FIRE_B);
            }
        } else if self.lizards[i].first == R_FIRE_B && rel == 7 && ctx.rng.pct(3) == 0 {
            self.lizards[i].deadline += ctx.rng.pct(10_000) as i64;
            self.set_run(i, R_FIRE_B);
        }
        if self.lizards[i].wrapped {
            self.set_pos(i, -1000, -1000);
            self.advance(i);
            self.lizards[i].delay = 100;
            return self.set_state(i, 0);
        }
        self.sound_tail(i, ctx);
    }

    /// State 0x17 — the "slave" waits for its master to release it.
    fn state17(&mut self, i: usize, ctx: &mut Ctx) {
        let me = self.lizards[i].idx as usize;
        if self.request[me] == 6 {
            self.restore_state(i);
            self.set_run(i, R_IDLE_B);
            let h = self.hand[me];
            if h != 0 {
                return self.set_state(i, h);
            }
            self.set_pos(i, -1000, -1000);
            return self.set_state(i, 0);
        }
        let p = self.lizards[i].partner;
        let orphaned = self
            .index_of(p)
            .map(|j| self.lizards[j].partner != self.lizards[i].idx)
            .unwrap_or(true);
        if orphaned {
            self.set_pos(i, -1000, -1000);
            return self.set_state(i, 0);
        }
        self.sound_tail(i, ctx);
    }
}

impl Module for Chameleon {
    fn name(&self) -> &'static str {
        "Chameleon"
    }

    /// `fn21` @0870 reads four controls; only three `sVal` resources ship.
    /// See GAP(control 3).
    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Quantity".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 60,
            },
            ControlDef {
                name: "Zest".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 80,
            },
            ControlDef {
                name: "Vommeter".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 25,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => {
                self.quantity = value.clamp(0, 100);
                self.g045a = (self.quantity / 10).max(1);
            }
            1 => self.zest = value.clamp(0, 100),
            2 => self.vommeter = value.clamp(0, 100),
            _ => {}
        }
    }

    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        self.now = ctx.now_ms as i64;
        if self.need_ctor_rolls {
            self.need_ctor_rolls = false;
            // @03A6: DrawText(STR# 128 + RandomBelow(23)) — After Dark's own
            // message line.
            let n = self.pack.strings(128).len();
            if n > 0 {
                let _ = ctx.rng.pct(0x17);
                self.message = self.pack.strings(128).first().cloned();
            }
        }
        self.controller(ctx);
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // `fn16` @0468 reorders the sprite list by the z key at +0x3A;
        // lower on the field draws in front.
        let mut order: Vec<(usize, &Liz)> = self.lizards.iter().enumerate().collect();
        order.sort_by_key(|(_, l)| l.y);
        for (i, l) in order {
            let Some(g) = self.geom.get(&fid(l.frame)) else { continue };
            let Some(f) = self.pack.frame(BASE, fid(l.frame)) else { continue };
            let pal = self.pal_of(i);
            out.push(SpriteDraw {
                png: f.png.clone(),
                x: l.x - (g.w >> 1),
                y: l.y - (g.h >> 1),
                pal,
                flip: l.flip,
            });
        }
    }

    /// A lizard's `shown` table as slot -> RGB. The display takes the HIGH
    /// byte of each 16-bit component (`M130_fn24`'s 24/32-bit path writes
    /// `c >> 8`; 8-bit goes through the device DAC the same way).
    fn dyn_palette(&self, pal: u16) -> Option<HashMap<u16, [u8; 3]>> {
        let l = self.lizards.get(pal.checked_sub(engine::DYN_PAL)? as usize)?;
        Some(
            l.shown
                .iter()
                .enumerate()
                .map(|(k, e)| (k as u16, e.map(|v| (v >> 8) as u8)))
                .collect(),
        )
    }

    fn texts(&self, out: &mut Vec<TextDraw>) {
        if !engine::SHOW_QUIP_CAPTIONS {
            return;
        }
        if let Some(t) = &self.message {
            let scale = 2;
            let w = engine::font::text_width(t, scale);
            out.push(TextDraw {
                text: t.clone(),
                x: (SCREEN_W - w) / 2,
                y: SCREEN_H - 48,
                color: engine::contrast_ink(self.field()),
                scale,
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::collections::BTreeSet;
    use std::path::Path;

    /// Every run/frame id the port names, in C space — the `SetRun` and
    /// `+0xCC` arguments transcribed from `M130_fn17`.
    const NAMED_IDS: [i32; 45] = [
        R_WALK,
        R_IDLE_A,
        R_IDLE_B,
        R_LOOK_LEFT,
        R_TURN,
        R_TOSS_END,
        R_LOOK_UP,
        R_VOM_A,
        R_VOM_B,
        R_VOM_C,
        R_VOM_HIT_SELF,
        R_VOM_HIT_OTHER,
        R_VOM_D,
        R_VOM_E,
        R_BLINK_IN,
        R_BLINK_A,
        R_BLINK_B,
        R_YAWN,
        R_TOSS_CLIMB,
        R_SEX_CLIMB,
        R_SEX_LOOP,
        R_SEX_END,
        R_RIDE_JOIN,
        R_FOLDER_A,
        R_FOLDER_B,
        R_FOLDER_C,
        R_FIRE,
        R_FIRE_B,
        R_FIRE_C,
        R_PAIR_A,
        R_PAIR_B,
        R_PAIR_C,
        R_PAIR_D,
        R_PAIR_E,
        R_PAIR_F,
        R_PAIR_G,
        R_PAIR_H,
        R_FACE_OFF,
        R_SLAVE_ANCHOR,
        R_MATE_A,
        R_MATE_B,
        R_MATE_C,
        R_MATE_D,
        R_STROLL,
        R_ARRIVE,
    ];

    fn ctx(seed: u64) -> Ctx {
        Ctx {
            rng: RandomLong::new(seed),
            rng15: Random15::new(seed as u32),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    fn load() -> Option<Chameleon> {
        let pack = Pack::load(Path::new("../assets/chameleon")).ok()?;
        build(pack)
    }

    /// §5 test 1: 3000 ticks on the Pacer; something drew and something
    /// sounded.
    #[test]
    fn chameleon_smoke() {
        let Some(mut m) = load() else {
            eprintln!("assets/chameleon missing — skipping");
            return;
        };
        let mut c = ctx(1);
        let mut p = Pacer::new(&m);
        let (mut drew, mut sounded) = (false, false);
        let mut out = Vec::new();
        for _ in 0..3000 {
            p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            sounded |= !c.sounds.is_empty();
            out.clear();
            m.sprites(&mut out);
            drew |= out.iter().any(|s| s.x > -500 && s.y > -500);
        }
        assert!(drew, "nothing drew in 3000 ticks");
        assert!(sounded, "nothing sounded in 3000 ticks");
    }

    /// §5 test 2 (ratchet, 2026-09-19): every id the port names resolves to
    /// a LIVE frame in the pack, and its run has length. This is the whole
    /// `pk(id) = id − 1` bug in one assert: under the old mapping
    /// `R_SLAVE_ANCHOR` (884 → 883) and `R_MATE_C` (971 → 970) landed in
    /// inter-block gaps with no record, which is why they were written off
    /// as `GAP(rip holes)`. Ids are `frameNum`s as written — §10.1.
    #[test]
    fn every_named_run_resolves_to_a_live_frame() {
        let Some(m) = load() else { return };
        for r in NAMED_IDS.into_iter().chain(TONGUE_RUNS) {
            assert!(
                m.pack.frame(BASE, fid(r)).is_some(),
                "id {r:#x} ({r}) is not in the bank"
            );
            assert!(m.last_of(r) >= r, "run {r:#x} has no length");
        }
    }

    /// The two ids the old port called rip holes are block STARTS — the
    /// lead-in records the module addresses itself (`M130_fn17` @2243FE
    /// jumps `+0xCC(0x374)`, @221A88 rolls `SetRun(0x3CB)`). Each opens a
    /// real run: 884..923 (40 frames) and 971..982 (12). `id − 1` gives
    /// 883 and 970, which are in the gaps between blocks.
    #[test]
    fn the_two_lead_in_ids_are_live_block_starts() {
        let Some(m) = load() else { return };
        for (id, last, len) in [(R_SLAVE_ANCHOR, 923, 40), (R_MATE_C, 982, 12)] {
            assert!(m.center(id).is_some(), "{id:#x} has no art");
            assert_eq!(m.last_of(id), last, "{id:#x} runs to the block end");
            assert_eq!(m.last_of(id) - id + 1, len, "{id:#x} run length");
            // the id is the block START: one below it is in the gap.
            assert!(
                m.pack.frame(BASE, fid(id - 1)).is_none(),
                "{:#x} is NOT a gap — the lead-in claim is stale",
                id - 1
            );
        }
    }

    /// A `SetRun` enters the run AT the id and plays to the containing
    /// block's last frame — the run is entered mid-block wherever the C
    /// says so. Three ids sit further into a block than the usual lead-in
    /// +1 and that is correct: the 44-frame fire block 510..553 is entered
    /// at 511, 522 and 529 (`M130_fn17` @2240E0 / @22419A pick the later
    /// two off `+0x104`), and C 10 is the last frame of block 8..10 alone.
    /// Ratchet: under `pk(id) = id − 1` every `last` here was one higher
    /// (`seq_end[id − 1] + 1`), so all six fail.
    #[test]
    fn a_set_run_enters_at_the_id_and_ends_at_the_block_end() {
        let Some(mut m) = load() else { return };
        m.rebuild();
        assert!(!m.lizards.is_empty(), "no lizards to drive");
        for (id, last) in [
            (R_FIRE, 553),
            (R_FIRE_B, 553),
            (R_FIRE_C, 553),
            (R_IDLE_B, 10),
            (R_WALK, 5),
            (R_MATE_A, 923),
            (R_SLAVE_ANCHOR, 923),
        ] {
            m.set_run(0, id);
            assert_eq!(m.lizards[0].last, last, "{id:#x} must run to {last}");
            assert!(m.lizards[0].last >= m.lizards[0].frame, "{id:#x} empty run");
        }
    }

    /// RATCHET (2026-09-29, linked hand-off): a walk loop's repeat is a
    /// `fn028A` hand-off `5 → 1 → 2`, not a no-delta wrap, so each cycle
    /// travels (±17, 0). The old wrap applied nothing and a cycle moved
    /// (+15, +4) — the lizard sank 4 px per step cycle down the screen.
    #[test]
    fn a_walk_cycle_travels_seventeen_px_on_the_level() {
        let Some(mut m) = load() else { return };
        for (flip, want) in [(false, (17, 0)), (true, (-17, 0))] {
            m.lizards[0] = m.new_liz(1);
            m.lizards[0].flip = flip;
            m.set_pos(0, 300, 240);
            m.set_run(0, R_WALK);
            m.lizards[0].repeat = 4;
            let mut at_first = Vec::new();
            for _ in 0..16 {
                m.advance(0);
                let l = &m.lizards[0];
                assert!(!l.wrapped, "repeats left: the loop must not report");
                if l.frame == R_WALK {
                    at_first.push((l.x, l.y));
                }
            }
            assert!(at_first.len() >= 3, "the loop came round: {at_first:?}");
            for w in at_first.windows(2) {
                assert_eq!((w[1].0 - w[0].0, w[1].1 - w[0].1), want, "flip={flip} {at_first:?}");
            }
        }
    }

    /// RATCHET (2026-09-29), pinned to the capture: the whole machine on the
    /// Pacer, every full walk cycle (2, 3, 4, 5, 2 drawn in order).
    /// GOLDEN `chameleon-long-av.mp4`: walk frames c_002..c_005 template-
    /// matched (silhouette, plain and mirrored) over 258 s give 30 walking
    /// tracks at a median |vx| of **39.4 px/s** and vy **0.0 px/s**, mirror
    /// agreeing with the direction 30/30. The old no-delta wrap walked 35.2
    /// px/s and sank 9.4 px/s.
    #[test]
    fn the_walk_speed_matches_the_capture() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(5);
        let mut p = Pacer::new(&m);
        // per lizard: frames drawn since the last frame 2, and that 2's (t, x, y)
        let mut seen: Vec<(Vec<i32>, Option<(u64, i32, i32)>)> = Vec::new();
        let mut last: Vec<i32> = Vec::new();
        let (mut vx, mut vy) = (Vec::new(), Vec::new());
        for _ in 0..16_000 {
            let t = p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            seen.resize(m.lizards.len(), (Vec::new(), None));
            last.resize(m.lizards.len(), 0);
            for (k, l) in m.lizards.iter().enumerate() {
                if l.frame == last[k] {
                    continue;
                }
                last[k] = l.frame;
                let s = &mut seen[k];
                if l.frame == R_WALK {
                    if let (Some((t0, x0, y0)), true) = (s.1, s.0 == [3, 4, 5]) {
                        let dt = (t - t0) as f64 / 1000.0;
                        vx.push(f64::from((l.x - x0).abs()) / dt);
                        vy.push(f64::from(l.y - y0) / dt);
                    }
                    *s = (Vec::new(), Some((t, l.x, l.y)));
                } else {
                    s.0.push(l.frame);
                }
            }
        }
        assert!(vx.len() > 50, "not enough walk cycles: {}", vx.len());
        let med = |v: &mut Vec<f64>| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };
        let (mx, my) = (med(&mut vx), med(&mut vy));
        assert!((mx - 39.4).abs() < 1.5, "walk speed {mx:.1} px/s, capture 39.4");
        assert!(my.abs() < 1.0, "walk drifts {my:.1} px/s vertically, capture 0.0");
    }

    /// RATCHET (2026-09-29): the turn (run 0x1B, frames 27..36) is authored
    /// with its body part MIRRORED from frame 30 on. The hand-off into the
    /// walk registers 36 on the lead-in 1 through `fn3F2E`, whose part flip
    /// bits differ, so the SPRITE's flip toggles — the lizard walks away in
    /// its new direction, body where the turn left it. The old port never
    /// flipped here and the lizard marched back the way it came.
    #[test]
    fn the_turn_hands_off_mirrored_onto_the_walk() {
        let Some(mut m) = load() else { return };
        for flip in [false, true] {
            m.lizards[0] = m.new_liz(1);
            m.lizards[0].flip = flip;
            m.set_pos(0, 320, 240);
            m.set_run(0, R_TURN);
            let mut n = 0;
            while !m.lizards[0].wrapped {
                m.advance(0);
                n += 1;
                assert!(n < 40, "the turn never ended");
            }
            let (cur, before) = (m.lizards[0].frame, m.lizards[0].clone());
            assert_eq!(cur, 36, "the turn ends on its last record");
            let body = m.pack.frame(BASE, 36).unwrap().parts[0];
            let lead = *m.pack.frame(BASE, 1).unwrap().parts.iter().find(|p| p[0] == body[0]).unwrap();
            assert_eq!(body[0], 4, "36 and 1 share the body, art 4");
            m.set_run(0, R_WALK);
            let l = m.lizards[0].clone();
            assert_eq!(l.flip, !flip, "the hand-off toggles the mirror");
            // the shared body keeps its screen spot through the marker, then
            // the 1 → 2 link moves on (+2 or −2, −4)
            let s36 = test_part_screen(&m, 36, &body, before.flip, (before.x, before.y));
            let (lx, ly) = m.link(1, R_WALK, l.flip);
            let s1 = test_part_screen(&m, 1, &lead, l.flip, (l.x - lx, l.y - ly));
            assert_eq!(s1, s36, "flip={flip}: the body must not move at the hand-off");
            let x0 = l.x;
            for _ in 0..4 {
                m.advance(0);
            }
            let dir = m.lizards[0].x - x0;
            assert!(if flip { dir > 0 } else { dir < 0 }, "flip={flip}: walks the new way ({dir})");
        }
    }

    /// RATCHET (2026-09-29): `+0xCC` (`fn0D3C`) registers on the shared
    /// part too, not the frame centres — the pair response's jump from the
    /// idle 10 onto the slave anchor 0x374 = 884 keeps the body (art 5)
    /// still. The old centre link slid it ±38 px.
    #[test]
    fn the_linked_jump_keeps_the_shared_body_still() {
        let Some(mut m) = load() else { return };
        for flip in [false, true] {
            m.lizards[0] = m.new_liz(1);
            m.lizards[0].flip = flip;
            m.set_pos(0, 320, 240);
            m.set_run(0, R_IDLE_B);
            let a = m.lizards[0].clone();
            m.jump_linked(0, R_SLAVE_ANCHOR);
            let b = m.lizards[0].clone();
            let pa = m.pack.frame(BASE, 10).unwrap().parts.clone();
            let pb = m.pack.frame(BASE, fid(R_SLAVE_ANCHOR)).unwrap().parts.clone();
            let (x, y) = pa.iter().find_map(|x| pb.iter().find(|y| y[0] == x[0]).map(|y| (*x, *y))).unwrap();
            assert_eq!(
                test_part_screen(&m, R_SLAVE_ANCHOR, &y, b.flip, (b.x, b.y)),
                test_part_screen(&m, 10, &x, a.flip, (a.x, a.y)),
                "flip={flip}: art {} moved",
                x[0]
            );
        }
    }

    /// The geometry the module resolves for an id is the pack record for
    /// THAT id, cross-checked against the pack itself rather than against
    /// the module's own map. Fails on `id − 1`, which read frame 1's
    /// bounds (294, 239) for run 2 instead of frame 2's (296, 235).
    #[test]
    fn geometry_resolves_to_the_record_at_the_id() {
        let Some(m) = load() else { return };
        for id in [R_WALK, R_FIRE, R_FIRE_B, R_MATE_A, R_SLAVE_ANCHOR, R_MATE_C] {
            let f = m.pack.frame(BASE, fid(id)).expect("record at the id");
            let img = m.pack.image(&f.png);
            let want = (
                f.bx + f.dx + (img.w as i32 >> 1),
                f.by + f.dy + (img.h as i32 >> 1),
            );
            assert_eq!(m.center(id), Some(want), "centre of {id:#x}");
            assert_eq!(m.bounds(id), (img.w as i32, img.h as i32), "bounds {id:#x}");
        }
        // the walk's first two frames are NOT the same record, so the
        // off-by-one was observable in the drawn art, not just in bookkeeping.
        assert_ne!(m.center(R_WALK), m.center(R_WALK - 1), "1 and 2 differ");
    }

    /// §5 test 3: 60 000 ticks, the machine visits the main loop and no
    /// lizard is stuck.
    #[test]
    fn the_machine_gets_around() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(7);
        let mut p = Pacer::new(&m);
        let mut seen: BTreeSet<i32> = BTreeSet::new();
        let mut runs: BTreeSet<i32> = BTreeSet::new();
        for _ in 0..60_000 {
            p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            for l in &m.lizards {
                seen.insert(l.state);
                runs.insert(l.first);
            }
        }
        for want in [0, 2, 3, 0xc] {
            assert!(seen.contains(&want), "state {want:#x} never ran: {seen:?}");
        }
        assert!(seen.len() >= 6, "only {} states visited: {seen:?}", seen.len());
        assert!(runs.len() >= 5, "only {} runs played: {runs:?}", runs.len());
        // nobody parked off-screen forever
        assert!(
            m.lizards.iter().any(|l| l.x > -500),
            "every lizard ended parked off-screen"
        );
    }

    /// The capture family: a lizard's own `now + 100` gate on the Mac grid
    /// averages 106.4 ms, and the fire state's `now + 75` is faster.
    #[test]
    fn the_lizard_frame_is_the_hundred_ms_mac_gate() {
        let Some(m) = load() else { return };
        assert_eq!(m.clock(), TickClock::MacTick);
        let p = Pacer::new(&m);
        let mean = p.mean_period_ms(100, false, 200);
        assert!((mean - 106.4).abs() < 1.5, "100 ms gate averaged {mean:.2} ms");
        let fire = p.mean_period_ms(0x4b, false, 200);
        assert!(fire < mean, "the fire state ({fire:.2} ms) must beat {mean:.2} ms");
    }

    /// Zest 100 indexes one long past the five-entry idle pool and lands in
    /// the sound-id table; the act ladder freezes. Original bug kept.
    #[test]
    fn zest_hundred_reads_past_the_idle_pool() {
        assert_eq!((100 / 20) as usize, IDLE_POOL.len());
        assert!(IDLE_POOL_OVERRUN > 60_000_000, "the overrun must be minutes long");
        // the two words it reads are the first two sound ids
        assert_eq!(
            IDLE_POOL_OVERRUN,
            ((SOUND_TABLE[0] as i64) << 16) | SOUND_TABLE[1] as i64
        );
    }

    /// `fn35` @1760: a cue is suppressed only while the last index played is
    /// <= this one AND the running clip has more than 180 ms left.
    #[test]
    fn the_sound_gate_is_a_hundred_and_eighty_ms_window() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(3);
        m.now = 0;
        m.snd(&mut c, 5);
        assert_eq!(c.sounds.len(), 1, "the first cue always plays");
        c.sounds.clear();
        m.snd(&mut c, 5);
        assert!(c.sounds.is_empty(), "the same index inside the window is swallowed");
        // a LOWER index pre-empts
        m.snd(&mut c, 1);
        assert_eq!(c.sounds.len(), 1, "a lower index pre-empts");
        c.sounds.clear();
        // and the window opens 180 ms before the clip ends
        m.now = m.snd_busy;
        m.snd(&mut c, 12);
        assert_eq!(c.sounds.len(), 1, "the tail window lets the next cue in");
    }

    /// Run 2 is the walk; `+0xFC` (`fn12DE` mode 1) returns
    /// `link(first, last)` = 15 px, which the walks divide by.
    /// REWRITTEN 2026-09-29: this used to call 15 px "the stride per
    /// cycle". It is only the divisor — the loop's `fn028A` hand-off adds
    /// (+2, −4) and a cycle travels (+17, 0); see
    /// `a_walk_cycle_travels_seventeen_px_on_the_level`.
    #[test]
    fn the_walk_divisor_is_fn12de_link_first_to_last() {
        let Some(m) = load() else { return };
        // C 2 enters block 1..5 at its second frame — 1 is the lead-in
        // `fn1186` passes through on every hand-off into run 2.
        assert_eq!(m.last_of(R_WALK), 5, "run 2 is frames 2..5 of block 1..5");
        assert_eq!(m.cycle_dx(0, R_WALK), 15, "+0xFC(2, 1) = link(2, 5).x");
        assert_eq!(m.cycle_dx(0, R_IDLE_A), 0, "run 9 is stationary");
    }

    /// Diagnostic: how long every lizard spends in every state, and where
    /// they end up. `cargo test -p app -- --ignored --nocapture chameleon_census`
    #[test]
    #[ignore]
    fn chameleon_census() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(9);
        let mut p = Pacer::new(&m);
        let mut hist: std::collections::BTreeMap<i32, u64> = Default::default();
        let mut onscreen = 0u64;
        for _ in 0..120_000 {
            p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            for l in &m.lizards {
                *hist.entry(l.state).or_default() += 1;
                if l.x > 0 && l.x < SCREEN_W {
                    onscreen += 1;
                }
            }
        }
        let total: u64 = hist.values().sum();
        for (s, n) in &hist {
            println!("state {s:#04x}: {:5.2}%", 100.0 * *n as f64 / total as f64);
        }
        println!("on screen: {:5.2}%", 100.0 * onscreen as f64 / total as f64);
        for l in &m.lizards {
            println!("  #{} state={:#04x} pos=({},{}) clut={}", l.idx, l.state, l.x, l.y, l.clut);
        }
    }

    /// The second eye: one actor's transitions, printed.
    /// `cargo test -p app -- --ignored --nocapture chameleon_trace`
    #[test]
    #[ignore]
    fn chameleon_trace() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(5);
        let mut p = Pacer::new(&m);
        let mut last = (-1i32, -1i32, -1i32, 0i32, 0i32);
        for _ in 0..30_000 {
            let t = p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            let Some(l) = m.lizards.first() else { continue };
            let now = (l.state, l.first, l.frame, l.x, l.y);
            if now.0 != last.0 || now.1 != last.1 {
                println!(
                    "t={t:>7} state={:#04x} run={:#06x} d6={:>4} pos=({:>5},{:>4}) clut={} fade={}",
                    now.0, now.1, now.2, now.3, now.4, l.clut, l.fade
                );
                last = now;
            }
        }
    }

    /// A part's top-left on screen for a sprite whose centre is `pos`,
    /// independent of the port's own helpers: the part rect is mirrored
    /// inside the frame rect while `flip` (L135 `fn3BD6`'s layout).
    fn test_part_screen(m: &Chameleon, f: i32, p: &[i32; 7], flip: bool, pos: (i32, i32)) -> (i32, i32) {
        let g = m.geom[&fid(f)];
        let (l, t) = (g.bx + g.dx, g.by + g.dy);
        let r = l + g.w;
        let (pl, pt, pr) = (p[3] + g.dx, p[4] + g.dy, p[5] + g.dx);
        let pl = if flip { l + r - pr } else { pl };
        // drawn at pos − (w>>1, h>>1)
        (pos.0 - (g.w >> 1) + (pl - l), pos.1 - (g.h >> 1) + (pt - t))
    }

    /// Per-tick hand-off trace, every lizard: each drawn-frame change with
    /// state, run, frame, pos, the delta applied, the flip, and — when the
    /// frame is not `prev + 1` inside the same run — the first art id the
    /// two frames' part tables share (what `fn3F2E` registers on).
    /// `cargo test -p app -- --ignored --nocapture chameleon_handoff_trace`
    #[test]
    #[ignore]
    fn chameleon_handoff_trace() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(5);
        let mut p = Pacer::new(&m);
        let mut prev: Vec<(i32, i32, i32, i32, i32, bool)> = Vec::new();
        for _ in 0..16_000 {
            let t = p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            prev.resize(m.lizards.len(), (0, 0, 0, 0, 0, false));
            for (k, l) in m.lizards.iter().enumerate() {
                let now = (l.state, l.first, l.frame, l.x, l.y, l.flip);
                let q = prev[k];
                if (q.2, q.3, q.4, q.5) == (now.2, now.3, now.4, now.5) {
                    continue;
                }
                let step = !(q.1 == now.1 && now.2 == q.2 + 1);
                // (art, slip): how far that shared part moved on screen
                let shared = if step {
                    let pa = m.pack.frame(BASE, fid(q.2)).map(|f| f.parts.clone()).unwrap_or_default();
                    let pb = m.pack.frame(BASE, fid(now.2)).map(|f| f.parts.clone()).unwrap_or_default();
                    pa.iter().find_map(|a| pb.iter().find(|b| b[0] == a[0]).map(|b| (a.clone(), b.clone()))).map(
                        |(a, b)| {
                            let sa = test_part_screen(&m, q.2, &a, q.5, (q.3, q.4));
                            let sb = test_part_screen(&m, now.2, &b, now.5, (now.3, now.4));
                            (a[0], (sb.0 - sa.0, sb.1 - sa.1))
                        },
                    )
                } else {
                    None
                };
                println!(
                    "t={t:>7} #{} st={:#04x} run={:>4} f={:>4}->{:>4} pos=({:>5},{:>4}) d=({:>4},{:>4}) flip={}{} {}",
                    l.idx,
                    now.0,
                    now.1,
                    q.2,
                    now.2,
                    now.3,
                    now.4,
                    now.3 - q.3,
                    now.4 - q.4,
                    u8::from(now.5),
                    if now.5 != q.5 { "*" } else { " " },
                    match (step, shared) {
                        (false, _) => String::new(),
                        (true, Some((a, s))) => format!("HANDOFF shared art {a} slip {s:?}"),
                        (true, None) => "HANDOFF no shared part".into(),
                    }
                );
                prev[k] = now;
            }
        }
    }

    // ---- camouflage fade (GAP(camo blend) closed 2026-09-29) --------------

    /// Clut 1500 slot 0 as the pack bakes it: the lizard's body colour.
    fn body_baked(m: &Chameleon) -> [u8; 3] {
        m.pack.meta.palettes["1500"][&0]
    }

    /// The colour lizard `i`'s body is DRAWN in, through the same public
    /// contract the renderers use: `pal_of` → `dyn_palette` / pack clut →
    /// the slot substitution.
    fn drawn_body(m: &Chameleon, i: usize) -> [u8; 3] {
        let baked = body_baked(m);
        let pal = m.pal_of(i);
        if engine::is_dyn_pal(pal) {
            let clut = m.dyn_palette(pal).expect("a DYN_PAL handle resolves");
            m.pack.remap_clut(&clut).get(&baked).copied().unwrap_or(baked)
        } else if pal != 0 {
            m.pack.remap(pal).and_then(|r| r.get(&baked)).copied().unwrap_or(baked)
        } else {
            baked
        }
    }

    fn mac16(rgb: [u8; 3]) -> [u16; 3] {
        rgb.map(|v| v as u16 * 0x101)
    }

    /// RATCHET (GAP(camo blend) closed): one fade, pinned to the golden.
    ///
    /// GOLDEN `chameleon-long.mp4` (10 fps) f2725..f2885, one lizard walking
    /// right on its own while it fades toward clut 1500. Body colour per
    /// plateau (display gamma inverted with the exponent fitted from the
    /// boot colour, 51 → 84 on the capture): before the roll (243,211,9);
    /// then 19 changes — f2753 (251,208,4), which CONTINUES the previous
    /// fade's direction (that fade's unapplied 19th step, re-applied as
    /// step 0), then 18 linear steps f2758 (242,212,5) … f2799 (162,230,2)
    /// … f2846 (70,254,0), where it stays (still (70,254,0) at f2885). The
    /// target's R is 51: the lizard settles 18/20 of the way. 93 frames
    /// f0 → f18 = 9.3 s = 18 × 516.7 ms.
    ///
    /// The old port snapped: ONE change, at the roll, straight to clut
    /// 1500's (51,255,0). That fails every assert below.
    #[test]
    fn camo_fade_matches_the_golden() {
        let Some(mut m) = load() else { return };
        m.lizards.truncate(1);
        m.lizards[0] = m.new_liz(1);
        let boot = m.cluts[&CLUT_BASE];
        {
            // the state a roll toward 1500 leaves: previous fade shown at
            // its step 18, `base` at its step 19, `fade` 0.
            let l = &mut m.lizards[0];
            l.shown[0] = mac16([243, 211, 9]);
            l.base[0] = mac16([251, 208, 4]);
            l.target = boot;
            l.clut = CLUT_BASE;
            l.fade = 0;
            l.colour_due = 0;
        }
        assert_eq!(drawn_body(&m, 0), [243, 211, 9]);
        let mut changes: Vec<(i64, [u8; 3])> = Vec::new();
        let mut last = drawn_body(&m, 0);
        for t in 1..3000u64 {
            m.now = engine::mac_clock_ms(t) as i64;
            m.camo_fade(0);
            let c = drawn_body(&m, 0);
            if c != last {
                changes.push((m.now, c));
                last = c;
            }
        }
        assert_eq!(changes.len(), 19, "golden: 19 colour changes per roll, got {changes:?}");
        assert!(changes[0].1[0] > 243, "step 0 continues the OLD fade (golden R 243 → 251)");
        let near = |got: [u8; 3], want: [u8; 3], what: &str| {
            for c in 0..3 {
                assert!(
                    (got[c] as i32 - want[c] as i32).abs() <= 4,
                    "{what}: port {got:?} vs golden {want:?}"
                );
            }
        };
        near(changes[0].1, [251, 208, 4], "step 0 (f2753)");
        near(changes[1].1, [242, 212, 5], "step 1 (f2758)");
        near(changes[9].1, [162, 230, 2], "step 9 (f2799)");
        near(changes[18].1, [70, 254, 0], "step 18 (f2846)");
        assert!(changes[18].1[0] > 51 + 10, "settles short of the target (18/20), not on it");
        let span = changes[18].0 - changes[0].0;
        assert!((span - 9300).abs() <= 100, "f0 → f18: golden 9.3 s, port {span} ms");
        for w in changes.windows(2) {
            let dt = w[1].0 - w[0].0;
            assert!((500..=533).contains(&dt), "a step every ~516.7 ms (golden), got {dt}");
        }
        // The fade's own 20th step moves `base` but is never shown.
        assert_eq!(m.lizards[0].fade, FADE_STEPS);
        assert_ne!(m.lizards[0].base, m.lizards[0].shown);
    }

    /// RATCHET: over a long run, no lizard's colour ever jumps — every
    /// drawn change is at most one blend step (≤ 1/20 of a full swing, so
    /// ≤ 13 in 8-bit), and lizards do pass through colours that are no
    /// clut's. The old substitution snapped by up to 255 in one tick.
    #[test]
    fn camo_colour_never_snaps() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(7);
        let mut p = Pacer::new(&m);
        let n = m.lizards.len();
        let mut last: Vec<[u8; 3]> = (0..n).map(|i| drawn_body(&m, i)).collect();
        let clut_bodies: BTreeSet<[u8; 3]> =
            m.pack.meta.palettes.iter().filter_map(|(_, p)| p.get(&0).copied()).collect();
        let (mut worst, mut changes, mut between) = (0i32, 0u32, 0u32);
        for _ in 0..40_000 {
            p.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            assert_eq!(m.lizards.len(), n, "the lizard array was rebuilt");
            for (i, prev) in last.iter_mut().enumerate() {
                let now = drawn_body(&m, i);
                if now != *prev {
                    changes += 1;
                    for ch in 0..3 {
                        worst = worst.max((now[ch] as i32 - prev[ch] as i32).abs());
                    }
                    between += !clut_bodies.contains(&now) as u32;
                    *prev = now;
                }
            }
        }
        assert!(changes >= 40, "the fade ran: {changes} changes");
        assert!(worst <= 13, "a colour jumped by {worst} in one tick");
        assert!(between * 10 >= changes * 9, "{between}/{changes} changes landed between cluts");
    }

    /// The compose pass (every renderer's) draws a lizard in its OWN
    /// blended palette, and the body colour it draws is `drawn_body`'s.
    #[test]
    fn compose_draws_the_blended_palette() {
        let Some(mut m) = load() else { return };
        m.lizards.truncate(1);
        m.lizards[0] = m.new_liz(1);
        {
            let l = &mut m.lizards[0];
            l.frame = R_IDLE_A;
            l.x = 320;
            l.y = 240;
            l.shown[0] = mac16([150, 222, 60]);
        }
        let want = drawn_body(&m, 0);
        assert_eq!(want, [150, 222, 60]);
        let mut cache = crate::ImageCache::new();
        let mut out = vec![0u32; crate::SIM_PIXELS];
        crate::compose(&m.pack, &mut cache, &m, &mut out);
        let body = crate::pack_rgb(body_baked(&m));
        let blended = crate::pack_rgb(want);
        let n_blend = out.iter().filter(|&&p| p == blended).count();
        assert!(n_blend > 200, "{n_blend} body pixels in the blended colour");
        assert_eq!(out.iter().filter(|&&p| p == body).count(), 0, "a body pixel kept clut 1500");
        // and the per-draw recolour never lands in the decode cache
        assert!(cache.keys().all(|(_, pal)| *pal == 0));
    }
}
