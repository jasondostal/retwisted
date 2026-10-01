//! Mowin' Boris — ported function-by-function from the Ghidra decompile of
//! module CODE 129 "DynaMBoris" (totally-twisted docs/decompiled/mowin-boris/,
//! private) with the Library 4.0 sprite classes and assembly listings read
//! alongside it. Listing addresses cite resource_dasm ones (raw − 4).
//! Second pass 2026-09-14: the §3.0 audit of the crashed GLM lane — every cat
//! behaviour below cites a function in THIS module's M129_M129.c.
//!
//! ## Objects (controller ctor `fn20` @4840)
//! - Sound bank of 5 channels (`fn40` @59E2, `fn41` @5A12): snd 1001..1005
//!   (Meow, Purr, Cat Chop, Head Chop, Mower Buzz). Mower Buzz (1005) is a
//!   47.1 ms seamless loop played on the engine's looping sound channel.
//! - Background music: `cmid` 40 "Dawn Cue", one play from start (16.0 s).
//! - One RLESequence over art bank 1000 (`g0B60`), shared across all sprites.
//! - CATS (`fn87` @0000 ctor tail, `fn90` @00D4 message handler,
//!   `fn96` @07D6 chooser): 1..6 cats (control 1, `g0B30`).
//! - MOWER (`fn06` @35F8, `fn08` @36E2 DoState, `fn10` @3D9E chooser):
//!   `g0B3C`. When a cat's squash ladder completes, `fn90` @0x46A POSTS the
//!   cat's index to the mower's state object (state-vtbl +0x48) and hides
//!   the cat. `fn08` dequeues it (+0x40) only on the update after the
//!   mower's current run is done (+0x46), and `fn13` @410E picks the kill
//!   run (4/6/9; 7/8 only under +0x12E; revenge 2/3). Only the drive states
//!   1/4/5/11 SetPos — the kill runs play with the mower parked. The ENTER
//!   paints the kill run's first frame; the decal is stamped on its EXIT,
//!   the update after its last frame (`fn08_exit_decal`), and `fn10` sends
//!   run 4 on to 6/9 so a drag-under kill splats too. Capture
//!   (mowin-boris-av.mp4, run 9): cat hidden 26.25 s, mower parks 26.40 s,
//!   splat stamped 26.80 s under the mower, fully uncovered 27.75 s — the
//!   "late" splat is authentic. `fn107` @20E6 (life 2→4) and `fn108` @2136
//!   (life 4→`fn100`) are called from the mower's chooser (`fn10`, listing
//!   3441/3443) and the state-10 ENTER.
//!
//!   PASSES and ROWS (`fn08` drive branch, update 10, `fn14` @41D4 /
//!   `fn15` @4254): a drive update SetPos-es by the step (+0x12A, the Mower
//!   Speed with a sign) and sends the mower to 10 only once its frame is
//!   fully off screen; the update-10 pick sends it straight back (EXIT 10
//!   reverses the step, `fn14` steps y down one blade height with x
//!   untouched, back to the top via `fn15` past the field's bottom). The
//!   ctor's first message starts the song and a 16 s wait during which the
//!   drive neither moves nor turns. Capture: a turn hides the mower for 2-3
//!   frames after its last ~5 px sliver; rows 43 px apart.
//!
//!   The DRAG (`fn10` + `fn08`, mower +0x124/+0x12E/+0x130): a kill run's
//!   end may flatten the dead cat (life 3, +0x12E) instead of reviving it;
//!   that EXIT stamps 545 (blood only), the cat lies in its drag states
//!   4..9 on the blade's spot, the mower drives ON until a 3-5 s timer, then
//!   12 (stop), 11 (backs up at −2·step), chops it (7/8, decal 422), 13, 5.
//!   Capture (mowin-boris.mp4 34-41 s): kill 34.32 s, drives on, reverses
//!   at 20 px/tick from ~38.3 s, chop 40.20 s, resumes left at 10 px/tick.
//!
//!   REVENGE (`fn08` update 10, +0x12C/+0x134/+0x138): after 60 s of normal
//!   phase (never on the first pass) a pass rolls `Random15() % 16` against
//!   the Revenge setting (Always = every pass); a revenge pass drives state
//!   1 with a cat at the wheel (runs 517/537/522), restarts `cmid` 40 with a
//!   fresh 16 s wait, parks while any cat is on screen (`fn106` @2098), and
//!   keeps state-10 cats hidden (`fn90` update 10). The head-chop pair 2/3
//!   is only what `fn13` answers when that mower actually hits a cat. No
//!   capture shows a revenge pass (the 60 s capture ends before the first
//!   possible roll); a longer Revenge Always take would settle its look.
//! - BLADE LAYER (`g0B44`, second mower-side object): `fn92` @053C steers the
//!   cat away from ITS position; `fn94` @06E6 reorders the cat behind it.
//!   Ported as the mower's blade-rect centre (see Approximations).
//! - FLUTTERBY (`fn73` @2230, `fn74` @22AE handler, `fn78` @280A jink).
//! - LAWN PLANTER (`g0B40`: `fn60` @318C ctor, `fn63` @31C6 = `fn64` @31E0
//!   + `fn65` @32B6): one sprite toggling Show/Hide that stamps a grass
//!   blade (run 5) or, 4/256, a `g06BE` flower on every other call — see
//!   `fn63`. And DECALS (`fn16` @4358, `fn56` @45DE, `fn57` @46B6): the permanent ground
//!   stamps every kill leaves — see `fn08_exit_decal`.
//!
//! ## Cat machine (all from M129_M129.c)
//! - Fields (byte offsets): +0x40 pos; +0x92 state sub-object → +0x96 current
//!   state, +0x98 previous; +0x11C cat index (ctor stores the loop counter
//!   there, listing 0x214AF2 region); +0x11E life (1 alive, 2 dead-pending,
//!   4 revive — `fn13`/`fn104`/`fn107`); +0x120 enter timestamp (the 72nd
//!   int of the object); +0x124 same-state repeat; +0x128 off-field flag;
//!   +0x12A far-off-field flag; +0x12C resolved flag (`fn100` sets 1);
//!   +0x12E busy; +0x130 squash counter; +0x132/0x134 squash offsets.
//! - `fn87` ctor tail: state = 10, flip forced 0, `fn100` flags.
//! - Update (`fn90`, message = state, no flag bits): if the sequence finished
//!   (+0x46) → pending-check (vtbl +0x38) then `fn96` → SetState; else run:
//!   `fn93` mower-hit test (when not squashing), the +0x12E busy block, then
//!   the ladder: squash==0 → advance tick + `fn94`; squash<3 → `fn99` squash
//!   step + `fn94`, counter++; else counter=0, `fn13` kill dispatch
//!   (vtbl+0x48, listing 0x464-0x468), SetState(10).
//! - `fn93` @065A: cat on-screen (+0x34) and not squashing, and the test point
//!   (`fn97` @155A — ported as the cat's centre) inside the mower's `fn12`
//!   @3FEC rect → clear +0x12A/+0x128, squash = 1. `fn12` is the BLADE rect
//!   (compound channel 2), not the mower's whole frame — see fn12_mower_rect.
//! - `fn99` @1AA0: on squash==1 store (mower-rect centre − pos)/2 in
//!   +0x132/0x134; every step SetPos(pos + offset). Three steps total.
//! - `fn96` @07D6: off-field flags from the frame rect vs the field
//!   (g0B70 left / g0B72 top / g0B74 right / g0B76 bottom); far-off +
//!   off-screen → `fn100`, return 10. Per-state ladder verbatim: fixed
//!   transitions, `fn95` pools (avoid value), `fn92` mower-avoid in the six
//!   walk families and state 49, `fn98` heading when the mower is busy or the
//!   cat is far off. State 1 holds 2500 ms past the enter timestamp; states
//!   51/54 hold 5000 ms.
//! - `fn98` @17E2: heading toward the mower (or field centre when the mower
//!   is off): fixed-point angle → quadrant → g0148 = [25,22,36,39], then the
//!   per-state overrides (1|2→3, 3→47, 4..9 keep, 10 keep, 11..21→50,
//!   22..41 keep, 42..48→50, 50→49). Clears +0x128.
//! - `fn92` @053C: gated by ignore<1, rand%31 != 27, blade layer +0x34;
//!   |dx|<301, |dy|<301, |dx|+|dy|<451 → g002E[(dx>0)*4+(dy>0)*2+(|dy|<|dx|)]
//!   = [36,28,22,28,39,32,25,32], else 10.
//! - `fn101` @1BAC placement on enter: states 5/8 (prev not 6/9) → mower
//!   rect centre; 22 → mirror `fn91` x across g0B74; 25 → `fn91` point;
//!   28 → (g0B74+39, g0B76/4 + rand% (g0B76/2)); 32 → (g0B70−39, same y);
//!   36 → mirror both; 39 → mirror y. `fn91` @04A0: left-half roll →
//!   (rand x, top−39), else (left−39, upper-half rand y).
//! - `fn103` @1EF0: SetRun (vtbl +0x7C, the run pushed from `g0150` column 1
//!   at listing 00001F3C), then the flip coin against column 2, then `fn101`.
//!   The old header had this backwards: `+0x7C` is not "stop sequence", and
//!   `fn101` runs LAST, so its absolute placements win over the hand-off.
//! - `fn100` @1B76 revive: +0x12C=1, +0x12E=0, +0x130=0, +0x11E=1, +0x12A=0,
//!   +0x128=1 — in place, no move.
//! - Sounds (`fn47` @5DD4 CueSound, 16-slot queue): the cat cues ONLY on
//!   enter 0x8001 (bank slot 0 = Meow) and enter 0x8014 (slot 1 = Purr),
//!   both gated by g0B32 == 0 (listing 0x14C-0x158 and 0x230-0x23C). The
//!   lane-1 "2 % purr per tick" was invented — gone.
//!
//! ## Library sprite semantics
//!
//! (This section and `frame_offset` used to contradict each other — "pos is
//! the CENTRE" here against "bank-anchored top-left" at the sprite model.
//! The code has always been the second one. Reconciled in favour of the code,
//! which is also what the vtable says.)
//!
//! - **The anchor is a bank-space TOP-LEFT, not a centre.** Every offset in
//!   this file is `topleft(b) − topleft(a)` in the bank's own coordinates;
//!   under flip the crop mirrors, so dx negates and the width difference
//!   comes off. The generic §2 "pos is the centre of the frame" model is the
//!   equivalent statement for runs whose frames share a size — boris's do
//!   not (the mower's drive crops are 112/120/125 px tall on one origin), and
//!   centre differences would drift the anchor every wrap.
//! - **SetPos (vtbl +0x88) still addresses the frame CENTRE** — that part of
//!   §2 holds: every SetPos site (`fn99`, `fn101`, `fn14`/`fn15`, the
//!   planter) names where the drawn frame's centre goes. The port keeps the
//!   anchor on the marker's top-left and converts at the call
//!   (`set_pos_centre`, `fn15`, `fn63`); `visual_centre` is the inverse. So
//!   "pos is the centre" (what the C passes) and "the anchor is a top-left"
//!   (what the port stores) are two views of one model, not two models.
//! - **The anchor sits on the run's LINK MARKER**, the record one id below
//!   the run's first frame (`L132 fn1186` @1186). The marker is not part of
//!   the animation: run 105 is frames 105..108 and 104 is its marker.
//! - Advance: one-shot steps d6 to `last` and reports done; loop steps d6 and
//!   on wrap moves the anchor by `off(last)` — the marker-to-last offset,
//!   which is the authored cycle stride — and restarts at `first`. That is
//!   `L132 fn0316` @0316: at `last` it queues `+0x4E = +0x44`, and the next
//!   tick re-enters the run through `fn028A` @028A.
//! - SetRun is the same arithmetic: the anchor moves to where the currently
//!   drawn frame is, which is `fn028A`'s shared-part registration of the
//!   current frame against the new run's marker (`L135 fn3F2E`) followed by
//!   marker→first (`fn3DDC`), collapsed to top-left differences. **Both
//!   hand-off models in §7.1 exist in this bank and the cat vtable binds the
//!   LINKED one**: `g02C8+0x7C = L132 fn0204`, whose `+0x108` is `fn028A`,
//!   with `+0xF0 = fn1186` and `+0x114 = fn1526` — the same slots
//!   frankenscreen and mime-hunt bind, not the unlinked form §7.1 credited to
//!   boris. Decoded from `emu/ghidra/mowin-boris` A5 globals at `g02C8`.
//! - **The ENTER draws.** `fn53` @645C (state-vtbl +0x1C) is the module's own
//!   SetState: it fires EXIT, stores prev/current, and calls ENTER (+0x20)
//!   INLINE — it never raises L135's pending flag, so a state change happens
//!   inside the pump that triggered it. `fn90`'s enter path ends with `+0x84`
//!   (listing 2102EC), a real draw tick that paints the new run's first frame
//!   without advancing (`fn028A` left `+0x48 = 1`). §2's "an enter never
//!   draws" is the generic rule and is wrong for this module.
//!
//! ## Frame driver (`fn22` @5056)
//! 100 ms tick gate (`g0B20 = now + 100`). Each tick pumps Flutterby, Mower,
//! Lawn Planter, and Cats in order, draws, then calls the planter ten more
//! times, each followed by a draw (listing 215154) — 11 planter calls, so
//! 5 or 6 stamps, per tick.
//!
//! ## What stays approximate
//! - `fn12` @3FEC decompile failed (Ghidra "Cannot properly adjust input
//!   varnodes"); read off the listing 3FEC-410A and ported as compound
//!   channel 2 (the blade deck) translated onto the drawn frame rect — see
//!   `fn12_mower_rect`. The blade layer `g0B44` is tracked at that rect's
//!   centre; the C keeps a separate sprite whose pos mirrors the mower.
//! - GAP(fn97): `fn97` @155A's hit/depth test point is ported as the cat's
//!   frame centre. SUPERSEDED claim (was: "the C's boundsCentre collapses to
//!   the same point under the centre-pos draw model"): `fn97` is not a
//!   bounds centre. For states 11..21, 28..35, 42, 43 and 49..56 it is the
//!   vertical middle of the frame rect's RIGHT edge; for 22..27 and 36..41
//!   the same, but on the LEFT edge whenever the mower's x is left of that
//!   right edge; for every other state (the drag states 4..9 included) the
//!   +0x40 pos, i.e. the centre. Left as the centre on purpose — it sets which cats the blade
//!   catches, and the current catch rate is the one the eye passed
//!   (2026-09-29); transcribing it is a separate, eye-gated change.
//! - `fn88` @0080 (enter 10): if the sprite is on-screen (vtbl +0x34 = the
//!   library flag at +0x2C), Hide it (`L135 fn5052`); `fn89` @00AC (exit 10):
//!   if hidden, Show it (`L135 fn50D6`). A state-10 cat is invisible, and
//!   `fn93`'s first test is that same +0x34 flag, so a hidden cat is never
//!   hit. `fn90` has no +0x11E check anywhere: the kill does NOT freeze the
//!   cat. It sits hidden in state 10 — SUPERSEDED 2026-09-29: not "until
//!   its stopped run's tick finishes" but, per `fn90`'s update-10 branch,
//!   until `fn100` resolves it (+0x12C, dropped again on EXIT 10) and the
//!   mower is neither busy nor on a revenge drive, or at once if `fn10`
//!   picked it for the drag — then `fn96`(10) picks an edge state (g0046),
//!   or 5/8 for the drag, and `fn101` places it. (The lane-2 "life != 1
//!   cats are frozen" gate was invented; it left the revived cat visible on
//!   the mower, re-hit every ~600 ms, dragged across the lawn.) The SPRITE
//!   hiding is all state 10 does — the kill's ground decals are separate
//!   objects and stay on the lawn (`fn16_stamp`).
//! - SUPERSEDED 2026-09-29: "the mower's +0x12A side flag is derived from
//!   its drive direction; the decompile never shows it being written". It is
//!   the signed drive step: `fn08` renormalises its magnitude to the Mower
//!   Speed at the top of every message and EXIT 10 flips its sign.
//! - Sounds on the kill runs (Cat Chop / Head Chop) keep the first pass's
//!   mapping; the three controller cue sites (213838/2139C2/213A4C) are in
//!   the mower's fn10 region but their ids were not individually decoded.
//!
//! ## Facing
//! Bank 1000 carries TWO opposite art conventions and the motion linter's
//! single global hypothesis cannot express that: the cats' and flutterby's
//! unflipped art faces RIGHT (run 105 strides +47 px unflipped, and `g0150`
//! gives flip = 0 to state 32, the one `fn101` enters from the left edge),
//! while the mower's unflipped art faces LEFT (`fn17` @43F8 toggles the flip
//! bit until it equals `0 < +0x12A`, the sign of the drive step). Scored per
//! actor over a 60 s run the port agrees with its travel 98 % (cats), 96 %
//! (flutterby) and 99.5 % (mower) of the time; scored globally the mower's
//! ~16 % share of advances reads as the whole module being "23.5 %
//! backwards". `each_actor_faces_the_way_it_travels` is the per-actor gate.
//!
//! ## Motion gate (docs/motion-lint.md)
//! The linter puts all of bank 1000 in one class, which mixes the two art
//! conventions above and makes its `backwards` figure unreadable. Run it per
//! actor instead (`--trace-every 50`, 3610 ticks, `-c 0=100 -c 1=2 -c 2=3
//! -c 3=0`):
//!
//! ```text
//!                        still  hold   adv  ap95  rev  jerk tele  backwards
//!   cats   before          68%   2.0  10.2  14.3   7%     6    1   2.9% [A]
//!   cats   after           55%   2.0   8.9  14.3   5%     0    0   1.8% [A]
//!   mower  after           53%   2.0  10.3  10.3   0%     0    0   0.0% [B]
//!   CAPTURE (actors only)  54%   2.0   9.8  14.4   2%     0    0   -
//! ```
//!
//! Capture row: `capture_track.py --fps 20 --fg field --min-area 1200
//! --dilate 1`, which resolves the twelve actors. The default blob settings
//! glue each actor to the lawn dressing around it and report `adv` 7.2 — the
//! source of the "the port steps 40 % too far" reading. It does not: the
//! mower steps exactly 10 px per tick at Manic on both sides (measured on
//! the capture by colour key, mode 10 in 176 of 357 samples).
//!
//! ## Original quirks kept
//! - Cats revive IN PLACE where they were squashed (`fn100` moves nothing);
//!   a revived cat must walk back inside the 16 px inner margin before
//!   `fn92` avoidance re-engages (+0x128 gate).
//! - `fn96` has NO boundary test of its own — a cat can walk off the field;
//!   only far-off + fully off-screen triggers the in-place reset.
//! - Mower busy freeze: while the mower plays a kill run, walk-family cats
//!   head toward it (`fn98`) instead of pooling.
//! - A cat mown by the revenge mower is never revived: `fn13` marks life 2
//!   only off revenge, so nothing ever calls `fn100` on it and `fn90`'s
//!   update 10 keeps it hidden for good.
//! - A dragging mower cannot turn (the off-screen test wants no drag timer
//!   and an idle deck): it drives off the field and backs in again.
//! - A drag-under run 4 is a drive state: if it drives off screen before its
//!   run ends it turns, and `fn10`'s 4 → 6/9 splat never comes.
//! - `fn14` steps down while the blade's bottom is still inside the field,
//!   so the last row before the wrap (anchor 412) shows only the top of the
//!   mower; the wrap row (`fn15`, anchor 25) is one above the first pass's.

use std::collections::{HashMap, VecDeque};

use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TickClock, SCREEN_H, SCREEN_W,
};

pub const BASE: u32 = 1000;

pub const SONG_DAWN_CUE: u32 = 40;

pub const SND_MEOW: u32 = 1001;
pub const SND_PURR: u32 = 1002;
pub const SND_CAT_CHOP: u32 = 1003;
pub const SND_HEAD_CHOP: u32 = 1004;
pub const SND_MOWER_BUZZ: u32 = 1005;

pub const SCATTER_TUFT: u32 = 5;
/// `g06BE`, the six flower runs `fn65` @32B6 picks from (A5 decode). The
/// old table was 10..=20, five of which (11, 13, 15, 17, 19) are not in the
/// pack, so 5 of every 11 flower rolls drew nothing.
pub const PLANTER_FLOWERS: [u32; 6] = [10, 12, 14, 16, 18, 20];

pub const MAX_TUFTS: usize = 24000;

/// The first pass's row: the mower's anchor y after `fn15` @4254 + one
/// `fn14` @41D4 step. Rows are NOT a table: `fn14` moves the mower down by
/// the blade deck's height (channel 2, 43 px) on every ENTER out of state
/// 10, and `fn15` puts it back at the top once the blade's bottom has passed
/// the field's. GAP(fn15 y): `fn15` derives its y from a `+0x94` compound
/// query plus the frame rect that is not decoded; this value is the
/// capture's (deck rows at y 69/113/155/199/241 in mowin-boris.mp4, pitch
/// 43 = the blade height, which is the part `fn14` does say).
pub const MOWER_FIRST_ROW_Y: i32 = 68;

/// `g0150` (A5+0x150), the 55 × (state, run, flip) shorts `fn103` @1EF0 scans
/// before pushing the run to vtbl `+0x7C`. Read out of the module's A5 data
/// block, not inferred: the listing at 00001F3C pushes column 1 (`[A0+0x02]`)
/// as the SetRun argument and tests column 2 (`+0x04`) against the facing bit.
///
/// **These are OFst frameNums as written (§10.1); they are NOT pack block
/// firsts.** Every run here except state 49 sits one id above the block that
/// contains it — e.g. state 28 runs `105`, and the pack block is 104..108, so
/// the animation is 105..108 and record 104 is the run's link marker. The
/// previous table stored the block firsts, which made every walk draw its own
/// marker once per cycle on top of the frame it had just drawn.
pub const BORIS_STATES: &[(u16, u32, bool)] = &[
    (1, 300, false),
    (2, 285, false),
    (3, 305, false),
    (4, 443, true),
    (5, 433, true),
    (6, 453, true),
    (7, 443, false),
    (8, 433, false),
    (9, 453, false),
    (11, 150, false),
    (12, 155, false),
    (13, 175, false),
    (14, 195, false),
    (15, 205, false),
    (16, 220, false),
    (17, 160, false),
    (18, 170, false),
    (19, 165, false),
    (20, 185, false),
    (21, 230, false),
    (22, 80, true),
    (23, 75, true),
    (24, 90, true),
    (25, 80, false),
    (26, 75, false),
    (27, 90, false),
    (28, 105, true),
    (29, 95, true),
    (30, 115, true),
    (31, 125, true),
    (32, 105, false),
    (33, 95, false),
    (34, 115, false),
    (35, 125, false),
    (36, 60, true),
    (37, 55, true),
    (38, 70, true),
    (39, 60, false),
    (40, 55, false),
    (41, 70, false),
    (42, 145, false),
    (43, 140, false),
    (44, 260, false),
    (45, 255, false),
    (46, 250, false),
    (47, 310, false),
    (48, 270, false),
    (49, 30, false),
    (50, 315, false),
    (51, 40, true),
    (52, 35, true),
    (53, 50, true),
    (54, 40, false),
    (55, 35, false),
    (56, 50, false),
];

/// `g084A`, the 12 × (state, run) longs `fn18` @446E scans for the mower
/// (`&M129_g084A + (state_index << 2)`). Same A5 decode as `BORIS_STATES`,
/// same frameNum convention: state 5 drives on run **370** (block 369..371,
/// marker 369), not 369. The old table was the block firsts AND had state 1
/// pointing at 544 — the blood-splat decal art, not the mower at all.
pub const MOWER_RUNS: &[(u16, u32)] = &[
    (1, 517),
    (2, 537),
    (3, 522),
    (4, 377),
    (5, 370),
    (6, 413),
    (7, 503),
    (8, 493),
    (9, 399),
    (11, 468),
    (12, 463),
    (13, 512),
];

/// Flutterby flee table A5+0x470: octant -> state.
pub const FLEE_T: [u16; 8] = [6, 4, 2, 3, 5, 8, 7, 9];
pub const FLEE_INV: [usize; 8] = [2, 3, 1, 4, 0, 5, 6, 7];

/// `g04D8`, the 8 × (state, run, flip) shorts `fn80` @2FEC scans for the
/// flutterby. A5 decode; same frameNum convention. The flip column also
/// corrects states 7/8/9, which the prose-era table had inverted.
pub const FLB_STATES: &[(u16, u32, bool)] = &[
    (2, 360, false),
    (3, 350, true),
    (4, 350, false),
    (5, 340, true),
    (6, 340, false),
    (7, 320, false),
    (8, 330, true),
    (9, 330, false),
];

// fn95 transition pools (A5 data; lengths from the fn96 push sites).
pub const L_IDLE: &[u16] = &[1, 1, 1, 3]; // g003E, 4, first roll stands
pub const L10: &[u16] = &[22, 25, 28, 32, 36, 39]; // g0046, 6
pub const L_WALK: &[u16] = &[12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 46, 50, 50]; // g0052, 15, repeat only from 12
pub const L22: &[u16] = &[22, 22, 22, 24, 25, 25, 28, 51]; // g0070, 8
pub const L25: &[u16] = &[22, 25, 25, 25, 27, 32, 32, 54]; // g0080, 8
pub const L28: &[u16] = &[22, 22, 28, 28, 28, 30, 31, 36, 51]; // g0090, 9
pub const L32: &[u16] = &[25, 32, 32, 32, 34, 35, 39, 39, 54]; // g00A2, 9
pub const L36: &[u16] = &[36, 36, 36, 38, 39, 28, 28, 51]; // g00B4, 8
pub const L39: &[u16] = &[28, 32, 36, 36, 39, 39]; // g00C4, 6
pub const L44: &[u16] = &[2, 2, 44, 45, 45, 48]; // g00D0, 6, repeat only from 45
pub const L49: &[u16] = &[23, 26, 29, 33, 37, 40, 43, 43, 49, 49, 49, 49, 52, 52, 55, 55]; // g00DC, 16
pub const L51: &[u16] = &[22, 28, 36, 51, 51, 51, 51, 53]; // g00FC, 8
pub const L54: &[u16] = &[25, 32, 39, 54, 54, 54, 54, 56]; // g010C, 8

/// g0148: heading by quadrant (fn98).
pub const QUAD_HEADING: [u16; 4] = [25, 22, 36, 39];
/// g002E: mower-avoid heading by (dx>0)*4 + (dy>0)*2 + (|dy|<|dx|) (fn92).
pub const AVOID_HEADING: [u16; 8] = [36, 28, 22, 28, 39, 32, 25, 32];

pub fn oct_state(oct: usize) -> u16 {
    FLEE_INV
        .iter()
        .position(|o| *o == oct % 8)
        .map(|i| i as u16 + 2)
        .unwrap_or(2)
}

/// One packed frame's bounds in bank space.
#[derive(Clone, Copy, Default)]
pub struct Geom {
    pub bx: i32,
    pub by: i32,
    pub w: i32,
    pub h: i32,
    pub dx: i32,
    pub dy: i32,
}

#[derive(Clone, Copy, Default)]
pub struct Ent {
    pub x: i32,
    pub y: i32,
    pub d6: i32,
}

/// CSpriteBoris. Field comments cite the byte offsets from `fn90`/`fn100`.
pub struct Cat {
    pub e: Ent,             // +0x40 pos
    pub state: u16,         // +0x96 current state
    pub prev: u16,          // +0x98 previous state
    pub first: i32,
    pub last: i32,
    pub flip: bool,
    pub visible: bool,      // +0x2C library on-screen flag (fn88 Hide / fn89 Show)
    pub seq_done: bool,     // +0x46 sequence finished (set by the advance tick)
    pub pending: bool,      // state-vtbl +0x38 pending-check
    pub enter_ms: u64,      // +0x120
    pub repeat: u16,        // +0x124 same-state repeat count
    pub zside: u16,         // +0x126
    pub index: u16,         // +0x11C (ctor stores the loop counter)
    pub life: u16,          // +0x11E: 1 alive, 2 dead-pending, 4 revive
    pub off: bool,          // +0x128
    pub far_off: bool,      // +0x12A
    pub resolved: bool,     // +0x12C
    pub busy: bool,         // +0x12E
    pub squash: u16,        // +0x130
    pub squash_dx: i32,     // +0x132
    pub squash_dy: i32,     // +0x134
    pub last_drawn: Option<SpriteDraw>,
}

/// The mower (`g0B3C`). Field comments cite the byte offsets `fn06` @35F8
/// (ctor tail), `fn08` @36E2 (DoState) and `fn10` @3D9E (chooser) use.
pub struct Mower {
    pub e: Ent,
    pub state: u16,
    pub first: i32,
    pub last: i32,
    /// Sign of the drive step +0x12A (`|step|` is the Mower Speed, `g0B2A`,
    /// renormalised at the top of every `fn08` message). EXIT 10 flips it.
    pub dir: i32,
    pub flip: bool,
    /// The library Show/Hide flag: ENTER 10 hides (vtbl +0x2C), `fn17`
    /// @43F8 shows on the ENTER after 10 (vtbl +0x30).
    pub active: bool,
    /// +0x11E: set by the ctor; the first `fn08` message clears it, restarts
    /// the song and arms the 16 s wait (`g07FE`).
    pub music_cue: bool,
    /// +0x11C: raised on EXIT 10, dropped once the mower is on screen. A
    /// launching mower is not sent back to 10 by the off-screen test.
    pub launch: bool,
    /// +0x120: set by the ctor, cleared by every update-10 pick — the first
    /// pass can never be a revenge pass.
    pub first_pass: bool,
    /// +0x122: `fn14` @41D4's first-placement flag (`fn15` once).
    pub first_place: bool,
    /// +0x124: set by `fn10` @3D9E with the drag roll and consumed by the
    /// very EXIT that follows: `fn08` stamps 545 (blood only) there, because
    /// the cat is not left on the lawn — it lies flattened in the cat's drag
    /// states 4..9 until the mower backs over it.
    pub dragging: bool,
    /// +0x126: the Mower Buzz loop is on (drive update, gated by `g0B32`);
    /// ENTER 10 stops it.
    pub buzz: bool,
    /// +0x12C: revenge — a cat drives the mower (runs 517 / 537 / 522).
    pub revenge: bool,
    /// +0x12E: raised on ENTER 12 and dropped on EXIT 13 (`fn08` @36E2).
    /// While it is up the drive step is `-2 x step` (state 11 backs over the
    /// flattened cat at double speed) and `fn13` @410E picks the 7/8 pair.
    pub deck_busy: bool,
    /// +0x130: the drag timer `fn10` arms (now + 3000 + rand % 2000); a
    /// drive update that finds it expired enters 12.
    pub drag_timer: u64,
    /// +0x134 / +0x138: when the current normal / revenge phase began. A
    /// revenge roll needs 60 s of normal phase behind it.
    pub normal_since: u64,
    pub revenge_since: u64,
    /// +0x46: the current run finished on the last `+0x84` tick
    /// (`L132 fn0316`). `fn08` only dequeues a kill message or asks `fn10`
    /// for the next state on the update AFTER that.
    pub seq_done: bool,
    /// +0x48 (L132): `fn028A` sets it on SetRun so the ENTER's `+0x84` tick
    /// paints the run's first frame without advancing.
    pub first_draw: bool,
    /// The mower state object's message queue. A cat whose squash ladder
    /// completes posts its index here (`fn90` @0x46A, state-vtbl `+0x48`);
    /// `fn08` dequeues it (`+0x40`) and hands it to `fn13` only once the
    /// current run is done — so a kill waits out the drive run in progress,
    /// and a second hit during a kill run waits out the kill.
    pub kill_msgs: VecDeque<usize>,
    /// `g07FE`: the song wait. A drive update neither moves nor tests the
    /// screen edge until it has passed (0 = none).
    pub wait_until: u64,
    pub last_drawn: Option<SpriteDraw>,
}

pub struct Flutterby {
    pub e: Ent,
    pub state: u16,
    pub first: i32,
    pub last: i32,
    pub flip: bool,
    pub active: bool,
    pub armed: bool,
    pub oct: usize,
    pub last_drawn: Option<SpriteDraw>,
}

/// A ground stamp left by `fn16` @4358. The C keeps a 16-slot redraw table
/// on `g0B64` and frees a slot once the mower has driven clear of it; the
/// pixels are already on the lawn by then and nothing erases them, so the
/// port just keeps the sprite. Never flipped — `fn16` takes no facing, the
/// four art ids already encode the direction the mower was going.
pub struct Decal {
    pub x: i32,
    pub y: i32,
    pub fno: u32,
}

pub struct Scatter {
    pub fno: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// `L132 fn1186` @1186: the record one id below a run's first frame is that
/// run's LINK MARKER when the pack carries it (`fn1186` returns `id − 1`
/// whenever the sequence is live — `+0x82`). The art tool writes the marker
/// as the lead-in of the pack block the run lives in, so `marker(105) = 104`
/// while `marker(30) = 30` (block 30 is a lone record and 29 was never
/// ripped; every block in bank 1000 is separated by at least one hole, so
/// "the id below exists" and "same block" are the same test here).
///
/// The marker is the anchor the library hands runs off on: `fn028A` @028A
/// registers the current frame against it (`L135 fn3F2E`, shared part) and
/// then steps marker→first (`fn3DDC`). Anchoring the port on the marker makes
/// the loop wrap and the SetRun hand-off the same arithmetic.
fn marker_of(geom: &HashMap<u32, Geom>, first: i32) -> i32 {
    if first > 0 && geom.contains_key(&((first - 1) as u32)) {
        first - 1
    } else {
        first
    }
}

/// Bank-anchored sprite model. `pos` is where the run's MARKER record sits;
/// the drawn frame's top-left is `pos + off(first, d6, flip)`. The movement
/// lives in the frames: run 60's frames 60..63 stride the cat through the
/// compound and frame 63 repeats marker 59's art at the cumulative (22,−17),
/// so the next cycle's `pos += off(last)` puts 60 exactly one stride on.
///
/// Offsets are TOP-LEFT differences, not centre differences: the mower's
/// drive run (370..371) crops share one bounds origin but differ in crop
/// height, and centre differences would drift the anchor every wrap. Top-left
/// differences are zero for such runs and still carry the walk stride. Under
/// flip the crop mirrors around the anchor, which also mirrors its width.
fn frame_offset(geom: &HashMap<u32, Geom>, first: i32, d6: i32, flip: bool) -> (i32, i32) {
    let aref = marker_of(geom, first);
    let (fx, fy) = topleft(geom, aref);
    let (cx, cy) = topleft(geom, d6);
    let mut dx = cx - fx;
    if flip {
        let wd = geom.get(&(d6 as u32)).map(|g| g.w).unwrap_or(0);
        let wf = geom.get(&(aref as u32)).map(|g| g.w).unwrap_or(0);
        dx = -dx - (wd - wf);
    }
    (dx, cy - fy)
}

/// The on-screen centre of the currently drawn frame (what fn97/fn99/fn101's
/// SetPos-centre semantics address).
fn visual_centre(geom: &HashMap<u32, Geom>, e: &Ent, first: i32, flip: bool) -> (i32, i32) {
    let (ox, oy) = frame_offset(geom, first, e.d6, flip);
    let (w, h) = match geom.get(&(e.d6 as u32)) {
        Some(gm) => (gm.w, gm.h),
        None => (0, 0),
    };
    (e.x + ox + (w >> 1), e.y + oy + (h >> 1))
}

fn topleft(geom: &HashMap<u32, Geom>, fno: i32) -> (i32, i32) {
    match geom.get(&(fno as u32)) {
        Some(gm) => (gm.bx + gm.dx, gm.by + gm.dy),
        None => (0, 0),
    }
}

/// Advance a looping run: step d6; at the wrap reset to first and advance the
/// anchor by the cycle stride. The stride is `off(last)` — the offset of the
/// run's final frame FROM THE MARKER (`frame_offset` is marker-relative), so
/// `first` resumes one authored stride on from where `last` was drawn. That
/// is `fn0316` @0316's restart (`+0x4E = +0x44` at `last`, then `fn028A`
/// re-enters through the marker) with the part registration collapsed to the
/// top-left difference the marker already encodes.
///
/// Reports a finished cycle (drives the +0x46 re-pool in fn96).
fn adv_loop(geom: &HashMap<u32, Geom>, e: &mut Ent, first: i32, last: i32, flip: bool) -> bool {
    if e.d6 < last {
        e.d6 += 1;
        false
    } else {
        let (sx, sy) = frame_offset(geom, first, last, flip);
        e.x += sx;
        e.y += sy;
        e.d6 = first;
        true
    }
}

/// Advance a one-shot run: step d6 up to last, no anchor movement.
fn adv_oneshot(_geom: &HashMap<u32, Geom>, e: &mut Ent, _first: i32, last: i32, _flip: bool) -> bool {
    if e.d6 >= last {
        return true;
    }
    e.d6 += 1;
    e.d6 >= last
}

/// SetRun boundary: re-anchor so the currently drawn frame's centre stays
/// where it was, then start the new run at its first frame. `placed` skips
/// the continuity shift (fn101 set an absolute position).
fn set_run(geom: &HashMap<u32, Geom>, e: &mut Ent, old_first: i32, new_first: i32, flip: bool, placed: bool) {
    if !placed {
        let (ox, oy) = frame_offset(geom, old_first, e.d6, flip);
        e.x += ox;
        e.y += oy;
    }
    e.d6 = new_first;
}

pub struct MowinBoris {
    pub pack: Pack,
    pub geom: HashMap<u32, Geom>,
    // Controls
    pub speed_code: i32,   // Control 0: 1/2/5/10 px per step
    pub cats_wanted: i32,  // Control 1: 1..6 cats
    pub revenge: i32,      // Control 2: 0, 1, 3, 5, 16
    pub mower_sound: bool, // Control 3: checkbox == g0B32 sound gate
    // State
    pub clock: u64,
    pub next_due: u64,
    pub pending_sounds: Vec<u32>,
    pub cats: Vec<Cat>,
    pub mower: Mower,
    pub flutterby: Option<Flutterby>,
    pub decals: Vec<Decal>,
    /// Plays of `cmid` 40 started (L134 `fn0C98` calls): the ctor's +0x11E
    /// start, then one per revenge toggle (`fn08` @36E2, listing 213B4A /
    /// 213BC4). The shell restarts the tune each time this changes.
    pub song_plays: u32,
    pub tufts: Vec<Scatter>,
    pub flowers: Vec<Scatter>,
    /// The lawn planter's Show/Hide state (`fn65` toggles it every call).
    pub planter_shown: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

fn build(pack: Pack) -> Option<MowinBoris> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    let mut geom = HashMap::new();
    for seq in pack.series(BASE) {
        for (i, f) in seq.frames.iter().enumerate() {
            let img = pack.image(&f.png);
            geom.insert(
                seq.first + i as u32,
                Geom {
                    bx: f.bx,
                    by: f.by,
                    w: img.w as i32,
                    h: img.h as i32,
                    dx: f.dx,
                    dy: f.dy,
                },
            );
        }
    }

    let mut m = MowinBoris {
        pack,
        geom,
        speed_code: 1, // default Sluggish
        cats_wanted: 1,
        revenge: 3, // Sometimes (50)
        mower_sound: false,
        clock: 0,
        next_due: 0,
        pending_sounds: Vec::new(),
        cats: Vec::new(),
        mower: Mower {
            // fn06 @35F8: state 10, +0x122 = +0x120 = +0x11E = 1, +0x134 =
            // +0x138 = 0. The step starts positive (fn08's renormalisation
            // of a zero step), so EXIT 10 sends the first pass LEFT.
            e: Ent { x: SCREEN_W + 50, y: MOWER_FIRST_ROW_Y, d6: 369 },
            state: 10,
            first: 369,
            last: 371,
            dir: 1,
            flip: false,
            active: false,
            music_cue: true,
            launch: false,
            first_pass: true,
            first_place: true,
            dragging: false,
            buzz: false,
            revenge: false,
            deck_busy: false,
            drag_timer: 0,
            normal_since: 0,
            revenge_since: 0,
            seq_done: false,
            first_draw: false,
            kill_msgs: VecDeque::new(),
            wait_until: 0,
            last_drawn: None,
        },
        flutterby: None,
        decals: Vec::new(),
        song_plays: 1,
        tufts: Vec::new(),
        flowers: Vec::new(),
        planter_shown: false,
    };
    m.sync_cats();
    Some(m)
}

/// See `MowinBoris::run_last`.
fn run_last_in(pack: &Pack, id: i32) -> i32 {
    for s in pack.series(BASE) {
        let (lo, hi) = (s.first as i32, s.first as i32 + s.frames.len() as i32 - 1);
        if id >= lo && id <= hi {
            return hi;
        }
    }
    id
}

fn draw_of(
    pack: &Pack,
    geom: &HashMap<u32, Geom>,
    e: &Ent,
    first: i32,
    flip: bool,
) -> Option<SpriteDraw> {
    let f = pack.frame(BASE, e.d6 as u32)?;
    let (ox, oy) = frame_offset(geom, first, e.d6, flip);
    Some(SpriteDraw {
        pal: 0,
        png: f.png.clone(),
        x: e.x + ox,
        y: e.y + oy,
        flip,
    })
}

impl MowinBoris {
    /// `L132 fn1260` @1260: the last frame of a run is found by walking
    /// forward from the run id while the next record still exists — i.e. the
    /// end of the pack BLOCK that contains the id, never `id + len(id)`.
    /// Runs are addressed by frameNum as written (§10.1), so `run_last(105)`
    /// is 108 (block 104..108) and `run_last(30)` is 30.
    fn run_last(&self, id: i32) -> i32 {
        run_last_in(&self.pack, id)
    }

    /// Field rect g0B70/g0B72/g0B74/g0B76.
    fn field_l(&self) -> i32 {
        0
    }
    fn field_t(&self) -> i32 {
        0
    }
    fn field_r(&self) -> i32 {
        SCREEN_W
    }
    fn field_b(&self) -> i32 {
        SCREEN_H
    }

    fn sync_cats(&mut self) {
        let target = self.cats_wanted.clamp(1, 6) as usize;
        while self.cats.len() < target {
            let i = self.cats.len();
            // fn87 ctor tail: state 10, flip forced 0, fn100 flags. The C
            // never sets a position — the library default (0,0) holds until
            // the first fn96 pick places the cat at an edge via fn101.
            let mut cat = Cat {
                e: Ent { x: 0, y: 0, d6: 299 },
                state: 10,
                prev: 10,
                first: 299,
                last: 300,
                flip: false,
                // Sprites come up hidden; the first exit from state 10 (fn89)
                // shows the cat once fn101 has placed it at an edge.
                visible: false,
                seq_done: false,
                pending: false,
                enter_ms: 0,
                repeat: 0,
                zside: 1,
                index: i as u16,
                life: 1,
                off: false,
                far_off: false,
                resolved: false,
                busy: false,
                squash: 0,
                squash_dx: 0,
                squash_dy: 0,
                last_drawn: None,
            };
            cat.fn100();
            cat.last_drawn = draw_of(&self.pack, &self.geom, &cat.e, cat.first, cat.flip);
            self.cats.push(cat);
        }
        self.cats.truncate(target);
    }

    /// fn95 @079A `(obj, first_ok, n, pool, cur)`: `pool[Random15 % n]`;
    /// with `first_ok` the first roll stands, otherwise re-roll while the
    /// pick == `cur` (the state being left). Callers pass `first_ok = 1`
    /// except L_WALK (`cur == 12`), L44 (`cur == 45`) and L10 (0, no 10 in
    /// the pool). The port used to read the flag as "the value to avoid":
    /// L_IDLE [1,1,1,3] then ALWAYS left idle for 3, and walk states 13..21
    /// could repeat themselves while 12 could not — both inverted.
    fn fn95(&self, pool: &[u16], first_ok: bool, cur: u16, ctx: &mut Ctx) -> u16 {
        loop {
            let pick = pool[(ctx.rng15.next() as usize) % pool.len()];
            if first_ok || pick != cur {
                return pick;
            }
        }
    }

    /// fn12 @3FEC: the mower's KILL rect — channel 2 of its current compound,
    /// on screen. Ghidra could not lift this one ("Cannot properly adjust
    /// input varnodes"); read off the listing 3FEC-410A instead.
    ///
    /// `fn12(obj, wantHidden, outRect)` (rtd 0x000A = 4+2+4 bytes of args):
    /// - 3FF4: returns 0 when `wantHidden` is 0 and the object's visibility
    ///   query (vtbl +0x34) says hidden — i.e. the visibility gate is the
    ///   CALLER'S choice. fn93 (pushing 0 at 0684),
    ///   fn99 and fn102 pass 0 and so require an on-screen mower; fn14/fn16
    ///   push 1 and read the rect of a parked one.
    /// - 4024-409E: three resource queries on the compound, vtbl +0x94 with
    ///   the channel number and the mower's current frame: channel 2 (the
    ///   blade deck) into the first local, channel 6 into the second, channel
    ///   8 into the third. 40A0: vtbl +0x24 into a fourth (`r4`) = the sprite's current
    ///   ON-SCREEN frame rect.
    /// - 40B4-4102: `out.left = ch2.left - ch6.left + r4.left`,
    ///   `out.top = ch2.top - ch8.top + r4.top`, then the size of ch2
    ///   (`out.right = out.left + (ch2.right - ch2.left)`, likewise bottom).
    ///   Channel 6 is the mower's leftmost part and channel 8 its topmost, so
    ///   those two subtractions just recover the compound's own origin: the
    ///   result is channel 2's rect translated to where the frame is drawn.
    ///
    /// For the drive run (369..371, compound origin 410,105, 109x112) channel
    /// 2 is (441,172)-(498,215) — a 57x43 box at +31,+67 from the frame's
    /// top-left. The old port built a full-frame box CENTRED on `mower.e.x/y`,
    /// but `e.x/e.y` is the run anchor = the drawn frame's TOP-LEFT, so the
    /// kill rect sat 85 px left and 123 px above the blade: the cats walked
    /// over the mower untouched and were squashed by bare grass ahead of it.
    ///
    /// GAP(fn12 flags): the +0x94 queries also pass `(mower->+0x12E != 0)`
    /// and a 0 as longs; what they select is not decoded. Flip is applied the
    /// way the blit mirrors the compound.
    fn fn12_mower_rect(&self) -> Option<[i32; 4]> {
        // vtbl +0x34 with wantHidden = 0 (fn93's `clr.w` at 0684). For the
        // mower class this is a real on-screen test, not a stored flag: fn08's
        // drive path (listing 38D2 / 3A1E) waits for it to go true after a
        // launch and parks in state 10 the moment it goes false again.
        if !self.mower.active {
            return None;
        }
        let r = self.mower_frame_rect()?;
        if r[2] <= 0 || r[0] >= SCREEN_W || r[3] <= 0 || r[1] >= SCREEN_H {
            return None;
        }
        self.mower_channel_rect(2)
    }

    /// The mower's current frame rect on screen (vtbl +0x24).
    fn mower_frame_rect(&self) -> Option<[i32; 4]> {
        let gm = self.geom.get(&(self.mower.e.d6 as u32))?;
        let (ox, oy) = frame_offset(&self.geom, self.mower.first, self.mower.e.d6, self.mower.flip);
        let (x, y) = (self.mower.e.x + ox, self.mower.e.y + oy);
        Some([x, y, x + gm.w, y + gm.h])
    }

    /// One channel of the mower's current compound, placed on screen: the
    /// part's bank-space rect less the compound origin, plus the drawn frame's
    /// top-left (fn12 @40B4). Mirrored within the frame while the sequence is
    /// flipped, because the blit mirrors the whole compound.
    fn mower_channel_rect(&self, chan: i32) -> Option<[i32; 4]> {
        let f = self.pack.frame(BASE, self.mower.e.d6 as u32)?;
        let part = f.parts.iter().find(|p| p[1] == chan).copied()?;
        let r = self.mower_frame_rect()?;
        let (rel_l, rel_t) = (part[3] - f.bx, part[4] - f.by);
        let (pw, ph) = (part[5] - part[3], part[6] - part[4]);
        let l = if self.mower.flip { r[0] + f.w - rel_l - pw } else { r[0] + rel_l };
        let t = r[1] + rel_t;
        Some([l, t, l + pw, t + ph])
    }

    /// SetPos (vtbl +0x88) with centre semantics: the anchor is placed so the
    /// run's first frame is centred on (cx, cy).
    fn set_pos_centre(&self, e: &mut Ent, first: i32, flip: bool, cx: i32, cy: i32) {
        let (w, h) = match self.geom.get(&(first as u32)) {
            Some(gm) => (gm.w, gm.h),
            None => (0, 0),
        };
        // The anchor is the run's MARKER record, so back the first frame's
        // own offset out of the requested centre.
        let (ox, oy) = frame_offset(&self.geom, first, first, flip);
        e.x = cx - (w >> 1) - ox;
        e.y = cy - (h >> 1) - oy;
    }

    /// fn91 @04A0: a random off-field entry point. Left-half roll gives
    /// (rand x, top−39); otherwise (left−39, rand y in the upper half).
    fn fn91_entry(&self, ctx: &mut Ctx) -> (i32, i32) {
        let (l, t, r, b) = (self.field_l(), self.field_t(), self.field_r(), self.field_b());
        let x = l + ctx.rng.pct((r - l) as u32) as i32;
        let mid = l + (r - l) / 2;
        if x < mid {
            (x, t - 39)
        } else {
            let y = t + (ctx.rng.pct((b - t) as u32) as i32) / 2;
            (l - 39, y)
        }
    }

    /// fn101 @1BAC: positional placement on enter. The edge-state dispatch is
    /// nested under PREV state == 10 (the C reads +0x26 = byte 0x98), i.e. it
    /// fires only when a cat LEAVES idle into a walk family. States 5/8 place
    /// on the mower's blade (a drag-picked cat) when prev is not 6/9.
    fn fn101_place(&self, cat: &mut Cat, ctx: &mut Ctx) -> bool {
        if cat.state == 5 || cat.state == 8 {
            if cat.prev != 6 && cat.prev != 9 {
                if let Some(rc) = self.fn12_mower_rect() {
                    self.set_pos_centre(&mut cat.e, cat.first, cat.flip, (rc[0] + rc[2]) / 2, (rc[1] + rc[3]) / 2);
                    return true;
                }
            }
            return false;
        }
        if cat.prev != 10 {
            return false;
        }
        match cat.state {
            22 => {
                let (px, py) = self.fn91_entry(ctx);
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, self.field_r() - px, py);
                true
            }
            25 => {
                let p = self.fn91_entry(ctx);
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, p.0, p.1);
                true
            }
            28 => {
                let y = self.field_b() / 4 + ctx.rng.pct((self.field_b() / 2) as u32) as i32;
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, self.field_r() + 39, y);
                true
            }
            32 => {
                let y = self.field_b() / 4 + ctx.rng.pct((self.field_b() / 2) as u32) as i32;
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, self.field_l() - 39, y);
                true
            }
            36 => {
                let (px, py) = self.fn91_entry(ctx);
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, self.field_r() - px, self.field_b() - py);
                true
            }
            39 => {
                let p = self.fn91_entry(ctx);
                self.set_pos_centre(&mut cat.e, cat.first, cat.flip, p.0, self.field_b() - p.1);
                true
            }
            _ => false,
        }
    }

    /// fn103 @1EF0 StartStateSequence. Listing order (00001F3C..00001F9A):
    /// push `g0150[i].run` and call vtbl `+0x7C` (= `L132 fn0204` → `fn028A`,
    /// SetRun linked through the marker), THEN settle the facing against
    /// `g0150[i].flip`, THEN `fn101`. fn101 runs last, so its absolute
    /// placements overwrite the hand-off — which is why it is called here
    /// after `set_run` and with the NEW run's first frame, not the old one.
    fn cat_start_sequence(&mut self, cat: &mut Cat, ctx: &mut Ctx) {
        cat.seq_done = false;
        let Some((_, fno, want_flip)) = BORIS_STATES.iter().find(|s| s.0 == cat.state).copied()
        else {
            return; // state not in the table: library fallback keeps the run
        };
        cat.flip = want_flip;
        let old_first = cat.first;
        cat.first = fno as i32;
        cat.last = self.run_last(cat.first);
        set_run(&self.geom, &mut cat.e, old_first, cat.first, cat.flip, false);
        self.fn101_place(cat, ctx);
    }

    /// fn53 @645C SetState wrapper (state sub-object +0x1C): prev saved, then
    /// the enter bookkeeping from fn90's generic-enter path.
    fn cat_set_state(&mut self, ix: usize, state: u16, ctx: &mut Ctx) {
        {
            let cat = &mut self.cats[ix];
            cat.prev = cat.state;
            cat.state = state;
            if state == cat.prev {
                cat.repeat += 1;
            } else {
                cat.repeat = 0;
                cat.enter_ms = ctx.now_ms;
            }
            // fn88 @0080 (enter 10): Hide. fn89 @00AC (exit 10): Show.
            if state == 10 {
                cat.visible = false;
            } else if cat.prev == 10 {
                cat.visible = true;
                // fn90 EXIT 0x400A: a resolved cat drops +0x12C and raises
                // +0x128 on the way out, so the next kill leaves it hidden
                // until fn100 resolves it again.
                if cat.resolved {
                    cat.resolved = false;
                    cat.off = true;
                }
            }
        }
        let mut cat = std::mem::replace(&mut self.cats[ix], Cat::EMPTY);
        self.cat_start_sequence(&mut cat, ctx);
        self.cats[ix] = cat;
        // The ENTER path's `+0x84` (listing 2102EC, right after fn103) is a
        // real draw tick: `fn028A` left `+0x48 = 1`, so `fn0316` @0316 skips
        // the advance and paints the run's first frame. §2's "an enter never
        // draws" is the generic library rule and does NOT hold here — and it
        // matters, because `fn53` @645C (the module's SetState override in
        // state-vtbl +0x1C) calls ENTER (+0x20) inline instead of raising the
        // pending flag, so the re-pool happens inside the SAME 100 ms pump as
        // the run end. Without this draw the port loses a frame to every
        // state change: the capture holds a walking cat's frame for 2 of 20
        // samples 507 times and for >= 4 only 8 times (1.3 %), where the
        // port was at 26 %.
        let (e, first, flip) = {
            let c = &self.cats[ix];
            (c.e, c.first, c.flip)
        };
        self.cats[ix].last_drawn = draw_of(&self.pack, &self.geom, &e, first, flip);
    }

    /// fn92 @053C: mower-avoid heading (targets the g0B44 blade layer).
    fn fn92(&self, cat: &Cat, ctx: &mut Ctx) -> u16 {
        if ctx.rng.pct(31) == 27 {
            return 10;
        }
        // g0B44 +0x34 on-screen test — the blade layer rides the mower.
        let Some(rc) = self.blade_rect() else { return 10 };
        let (bx, by) = ((rc[0] + rc[2]) / 2, (rc[1] + rc[3]) / 2);
        let (px, py) = visual_centre(&self.geom, &cat.e, cat.first, cat.flip);
        let dx = bx - px;
        let dy = by - py;
        let (adx, ady) = (dx.abs(), dy.abs());
        if adx < 301 && ady < 301 && adx + ady < 451 {
            AVOID_HEADING[((dx > 0) as usize) * 4
                + ((dy > 0) as usize) * 2
                + ((ady < adx) as usize)]
        } else {
            10
        }
    }

    /// fn98 @17E2: heading toward the mower (or field centre), with the
    /// per-state overrides.
    fn fn98(&self, cat: &Cat) -> u16 {
        let mut h = {
            let target = if self.mower.active {
                (self.mower.e.x, self.mower.e.y)
            } else {
                (self.field_r() / 2, self.field_b() / 2)
            };
            let (px, py) = visual_centre(&self.geom, &cat.e, cat.first, cat.flip);
            let (dx, dy) = (target.0 - px, target.1 - py);
            let ang = (dy as f64).atan2(dx as f64); // -pi..pi
            // fixed-point: q = clamp((angle + 0x800000) / 0x400000, 0, 3)
            let q = (((ang / std::f64::consts::PI) + 1.0) * 2.0).round() as i32; // 0..4
            let q = q.clamp(0, 3) as usize;
            QUAD_HEADING[q]
        };
        h = match cat.state {
            1 | 2 => 3,
            3 => 0x2f,
            4..=9 => cat.state,
            10 => 10,
            11..=21 => 0x32,
            22..=41 => h, // keep the quadrant heading
            42..=48 => 0x32,
            50 => 0x31,
            _ => h,
        };
        h
    }

    /// The mower's "busy" view `fn90`'s +0x12E block and update-10 gate test:
    /// deck busy (+0x12E) or a kill run in states 4/6/7/8/9. The revenge
    /// pair 2/3 is NOT in the list.
    fn mower_deck_busy(&self) -> bool {
        self.mower.deck_busy || matches!(self.mower.state, 4 | 6 | 7 | 8 | 9)
    }

    /// The view `fn96`'s walk families add revenge to (mower +0x12C first,
    /// then the same five states).
    fn mower_busy(&self) -> bool {
        self.mower.revenge || self.mower_deck_busy()
    }

    /// `fn104` @1FEC: the first cat whose +0x11E life equals `life`.
    fn fn104(&self, life: u16) -> Option<usize> {
        self.cats.iter().position(|c| c.life == life)
    }

    /// `fn106` @2098: is any cat on screen (vtbl +0x34)? A revenge drive
    /// parks while one is.
    fn fn106(&self) -> bool {
        self.cats.iter().any(|c| self.on_screen(c))
    }

    /// fn96 @07D6 ChooseNextState.
    fn fn96(&mut self, ix: usize, ctx: &mut Ctx) -> u16 {
        let s = self.cats[ix].state;
        if s != 10 {
            // Off-field flags from the frame rect vs the field.
            let r = self.cat_frame_rect(&self.cats[ix]);
            if !self.cats[ix].off && !self.cats[ix].far_off {
                let outside = r[0] < self.field_l()
                    || r[1] < self.field_t()
                    || r[2] > self.field_r()
                    || r[3] > self.field_b();
                if outside {
                    self.cats[ix].far_off = true;
                }
            }
            if self.cats[ix].off {
                let inside = r[0] > self.field_l() + 16
                    && r[1] > self.field_t() + 16
                    && r[2] < self.field_r() - 16
                    && r[3] < self.field_b() - 16;
                if inside {
                    self.cats[ix].off = false;
                }
            }
            if self.cats[ix].far_off && !self.on_screen(&self.cats[ix]) {
                self.cats[ix].fn100();
                return 10;
            }
        }

        match s {
            1 => {
                if ctx.now_ms < self.cats[ix].enter_ms + 2500 {
                    1
                } else {
                    self.fn95(L_IDLE, true, 1, ctx)
                }
            }
            2 => 1,
            3 => 0x2f,
            4 | 5 => {
                if ctx.rng.pct(15) < 5 {
                    6
                } else {
                    4
                }
            }
            6 => 5,
            7 | 8 => {
                if ctx.rng.pct(15) < 5 {
                    9
                } else {
                    7
                }
            }
            9 => 8,
            10 => {
                if !self.cats[ix].busy {
                    self.fn95(L10, false, 10, ctx)
                } else {
                    // A cat `fn10` picked for the drag: by the sign of the
                    // mower's effective step (+0x12A, times -2 while +0x12E
                    // is up) — 8 while it drives left, 5 while right.
                    let step = if self.mower.deck_busy { -2 * self.mower.dir } else { self.mower.dir };
                    if step < 0 {
                        8
                    } else {
                        5
                    }
                }
            }
            11 => 12,
            12..=21 => {
                if !self.cats[ix].far_off && !self.mower_busy() {
                    self.fn95(L_WALK, s == 12, s, ctx)
                } else {
                    self.fn98(&self.cats[ix])
                }
            }
            22..=41 => self.walk_family(ix, ctx, s),
            42 => 11,
            43 => 42,
            44 | 45 | 48 => self.fn95(L44, s == 45, s, ctx),
            46 => 45,
            47 => 0x32,
            49 => {
                if !self.cats[ix].far_off && !self.mower_busy() {
                    let h = self.fn92(&self.cats[ix], ctx);
                    if h == 10 {
                        self.fn95(L49, true, 0, ctx)
                    } else if ctx.rng.pct(7) == 3 {
                        0x2b
                    } else {
                        h + 1
                    }
                } else {
                    self.fn98(&self.cats[ix])
                }
            }
            50 => 0x31,
            51 => {
                if !self.cats[ix].far_off && !self.mower_busy() {
                    if self.cats[ix].enter_ms + 5000 <= ctx.now_ms {
                        0x35
                    } else {
                        self.fn95(L51, true, 0, ctx)
                    }
                } else {
                    self.fn98(&self.cats[ix])
                }
            }
            52 => 0x33,
            53 => 0x31,
            54 => {
                if !self.cats[ix].far_off && !self.mower_busy() {
                    if self.cats[ix].enter_ms + 5000 <= ctx.now_ms {
                        0x38
                    } else {
                        self.fn95(L54, true, 0, ctx)
                    }
                } else {
                    self.fn98(&self.cats[ix])
                }
            }
            55 => 0x36,
            56 => 0x31,
            _ => s, // library fallback: keep the current state
        }
    }

    /// fn96's six walk families (states 22..41): fn92 avoidance gated by the
    /// +0x128 off flag, kept headings, mirror states, pools.
    fn walk_family(&self, ix: usize, ctx: &mut Ctx, s: u16) -> u16 {
        let cat = &self.cats[ix];
        if !(!cat.far_off && !self.mower_busy()) {
            return self.fn98(cat);
        }
        let mut h = s;
        if !cat.off {
            h = self.fn92(cat, ctx);
        }
        match s {
            22 => match h {
                22 | 25 | 28 => h,
                10 => self.fn95(L22, true, 0, ctx),
                _ => 24,
            },
            25 => match h {
                22 | 25 | 32 => h,
                10 => self.fn95(L25, true, 0, ctx),
                _ => 27,
            },
            28 => match h {
                22 | 28 | 36 => h,
                10 => self.fn95(L28, true, 0, ctx),
                _ => 31,
            },
            32 => match h {
                25 | 32 | 39 => h,
                10 => self.fn95(L32, true, 0, ctx),
                _ => 34,
            },
            36 => match h {
                28 | 36 | 39 => h,
                10 => self.fn95(L36, true, 0, ctx),
                _ => 38,
            },
            39 => match h {
                32 | 36 | 39 => h,
                10 => self.fn95(L39, true, 0, ctx),
                _ => 41,
            },
            23 => 22,
            24 => 0x31,
            26 => 25,
            27 => 0x31,
            29 => 28,
            30 | 31 => 0x31,
            33 => 32,
            34 | 35 => 0x31,
            37 => 36,
            38 => 0x31,
            40 => 39,
            41 => 0x31,
            _ => self.fn98(cat),
        }
    }

    fn cat_frame_rect(&self, cat: &Cat) -> [i32; 4] {
        let (ox, oy) = frame_offset(&self.geom, cat.first, cat.e.d6, cat.flip);
        match self.geom.get(&(cat.e.d6 as u32)) {
            Some(gm) => [
                cat.e.x + ox,
                cat.e.y + oy,
                cat.e.x + ox + gm.w,
                cat.e.y + oy + gm.h,
            ],
            None => [cat.e.x, cat.e.y, cat.e.x, cat.e.y],
        }
    }

    /// vtbl +0x34 on-screen test, approximated as the frame rect intersecting
    /// the screen.
    /// vtbl +0x34: the library's on-screen flag (+0x2C, Hide/Show), narrowed
    /// to frames that actually intersect the screen.
    fn on_screen(&self, cat: &Cat) -> bool {
        if !cat.visible {
            return false;
        }
        let r = self.cat_frame_rect(cat);
        r[0] < SCREEN_W && r[2] > 0 && r[1] < SCREEN_H && r[3] > 0
    }

    /// The cat's per-update body (fn90 update path).
    fn cat_update(&mut self, ix: usize, ctx: &mut Ctx) {
        // fn90's update-10 branch (message 10, no flag bits) runs before the
        // generic tail: a hidden cat never advances a run. Unless `fn10`
        // picked it for the drag (+0x12E), it stays in 10 until it is
        // resolved (+0x12C, raised only by `fn100` — the kill chain's
        // `fn108` revive or the far-off reset), and while the mower is on a
        // revenge drive or busy. Then `fn96`(10) and SetState.
        if self.cats[ix].state == 10 {
            if !self.cats[ix].busy
                && (!self.cats[ix].resolved || self.mower.revenge || self.mower_deck_busy())
            {
                return;
            }
            let next = self.fn96(ix, ctx);
            self.cat_set_state(ix, next, ctx);
            return;
        }
        if self.cats[ix].seq_done {
            if self.cats[ix].pending {
                self.cats[ix].pending = false;
                return;
            }
            let next = self.fn96(ix, ctx);
            self.cat_set_state(ix, next, ctx);
            return;
        }
        // Sequence running.
        if self.cats[ix].squash == 0 {
            self.fn93(ix, ctx);
        }
        // fn90's +0x12E busy block: a drag-picked cat shrugs
        // off hits while the mower is not busy; its first hit-free update, or
        // any update once the mower is busy (ENTER 12), clears the flag.
        if self.cats[ix].busy {
            if self.cats[ix].squash != 0 && !self.mower_deck_busy() {
                self.cats[ix].squash = 0;
            } else {
                self.cats[ix].busy = false;
            }
        }
        let cat = &mut self.cats[ix];
        if cat.squash == 0 {
            // +0x84 advance tick
            let is_walk = (11..=21).contains(&cat.state) || (22..=41).contains(&cat.state);
            let done = if is_walk {
                adv_loop(&self.geom, &mut cat.e, cat.first, cat.last, cat.flip)
            } else {
                adv_oneshot(&self.geom, &mut cat.e, cat.first, cat.last, cat.flip)
            };
            if done {
                cat.seq_done = true;
            }
            let first = cat.first;
            cat.last_drawn = draw_of(&self.pack, &self.geom, &cat.e, first, cat.flip);
        } else if cat.squash < 3 {
            self.fn99(ix);
            let cat = &mut self.cats[ix];
            let first = cat.first;
            cat.last_drawn = draw_of(&self.pack, &self.geom, &cat.e, first, cat.flip);
            cat.squash += 1;
        } else {
            cat.squash = 0;
            // fn90 @0x46A: post this cat's index to the mower's state object
            // (state-vtbl +0x48). `fn13` runs when the MOWER dequeues it.
            self.mower.kill_msgs.push_back(ix);
            self.cat_set_state(ix, 10, ctx);
        }
    }

    /// fn93 @065A: mower-hit test.
    fn fn93(&mut self, ix: usize, _ctx: &mut Ctx) {
        let cat = &self.cats[ix];
        if !self.on_screen(cat) {
            return;
        }
        let Some(rc) = self.fn12_mower_rect() else { return };
        let (px, py) = visual_centre(&self.geom, &cat.e, cat.first, cat.flip);
        if px >= rc[0] && px <= rc[2] && py >= rc[1] && py <= rc[3] {
            let cat = &mut self.cats[ix];
            cat.far_off = false;
            cat.off = false;
            cat.squash = 1;
        }
    }

    /// fn99 @1AA0: squash one step toward the mower-rect centre.
    fn fn99(&mut self, ix: usize) {
        let Some(rc) = self.fn12_mower_rect() else { return };
        let (mcx, mcy) = ((rc[0] + rc[2]) / 2, (rc[1] + rc[3]) / 2);
        let cat = &mut self.cats[ix];
        let (px, py) = visual_centre(&self.geom, &cat.e, cat.first, cat.flip);
        if cat.squash == 1 {
            cat.squash_dx = (mcx - px) / 2;
            cat.squash_dy = (mcy - py) / 2;
        }
        cat.e.x += cat.squash_dx;
        cat.e.y += cat.squash_dy;
    }

    /// fn13 @410E: the mower's kill-run chooser, called from `fn08` when it
    /// dequeues a cat's kill message. Marks the cat dead-pending (+0x11E = 2).
    /// The 6/9 vs 7/8 split is the mower's +0x12E flag, not its current
    /// state (the old port keyed it on "already in a kill run").
    fn fn13(&mut self, ctx: &mut Ctx, ix: usize) -> u16 {
        let m = &self.mower;
        let side_neg = m.dir < 0;
        let run = if !m.revenge {
            if !m.deck_busy {
                let x = m.e.x;
                if x > self.field_l() + 0x96
                    && x < self.field_r() - 0x96
                    && ctx.rng.pct(15) < 5
                {
                    4
                } else if side_neg {
                    6
                } else {
                    9
                }
            } else if side_neg {
                7
            } else {
                8
            }
        } else if side_neg {
            2
        } else {
            3
        };
        // fn13 marks the cat only on the non-revenge branch. A cat mown by
        // the revenge drive keeps life 1 and so is never revived (Original
        // quirks kept).
        if !m.revenge {
            self.cats[ix].life = 2;
        }
        run
    }

    /// `fn10` @3D9E, the mower's ChooseNextState, called by `fn08` on the
    /// update after a run is done when no kill message is queued.
    ///
    /// Drive runs repeat (1, 5, 11; 12 → 11, 13 → 5) and 4 hands on to its
    /// splat run (6 left, 9 right). A kill run's end (2/3/6/7/8/9) is where
    /// the DRAG starts (listing 213E94-213F0C): no cat already flattened
    /// (`fn104`(3)), no revenge, deck not busy, no drag timer, the mower more
    /// than 100 px inside the field, `Random15() % 15 < 5`, and a dead-pending
    /// cat (`fn104`(2)) — that cat gets +0x12E = 1 and life 3, the mower
    /// +0x124 = 1 (so the EXIT about to fire stamps 545, not 407) and a drag
    /// timer of now + 3000 + `Random15() % 2000`, and the answer is 5.
    /// Otherwise: revenge → 1; deck busy → 13 once no cat is flattened, else
    /// 11; else 5. Every kill-run end then revives the dead-pending cats
    /// (`fn107`/`fn108`).
    ///
    /// GAP(pos): the 100 px test reads the mower's +0x40 pos; ported as the
    /// anchor x, as `fn13`'s 150 px test is. GAP(+0x128): the non-drag path
    /// also cycles a 0..9 counter at +0x128 that nothing in this module
    /// reads back; not ported.
    fn fn10(&mut self, st: u16, ctx: &mut Ctx) -> u16 {
        match st {
            1 => 1,
            4 => {
                if self.mower.dir < 0 {
                    6
                } else {
                    9
                }
            }
            5 | 13 => 5,
            11 | 12 => 11,
            2 | 3 | 6 | 7 | 8 | 9 => {
                let m = &self.mower;
                if self.fn104(3).is_none()
                    && !m.revenge
                    && !m.deck_busy
                    && m.drag_timer == 0
                    && self.field_l() + 100 < m.e.x
                    && m.e.x < self.field_r() - 100
                    && ctx.rng.pct(15) < 5
                {
                    if let Some(ix) = self.fn104(2) {
                        self.mower.drag_timer = ctx.now_ms + 3000 + ctx.rng.pct(2000) as u64;
                        self.cats[ix].busy = true;
                        self.cats[ix].life = 3;
                        self.mower.dragging = true;
                        self.fn107_fn108();
                        return 5;
                    }
                }
                let next = if self.mower.revenge {
                    1
                } else if !self.mower.deck_busy {
                    5
                } else if self.fn104(3).is_none() {
                    13
                } else {
                    11
                };
                self.fn107_fn108();
                next
            }
            // The C traps (debug stub + jump table) on any other state.
            _ => 5,
        }
    }

    /// `+0x84` tick (`L132 fn0316`): the ENTER's tick paints the first frame
    /// without advancing (+0x48); every other tick steps one frame. Reaching
    /// `last` raises +0x46 on the same tick.
    fn mower_tick84(&mut self) {
        let m = &mut self.mower;
        if m.first_draw {
            m.first_draw = false;
        } else if m.e.d6 < m.last {
            m.e.d6 += 1;
        }
        if m.e.d6 >= m.last {
            m.seq_done = true;
        }
        let first = m.first;
        m.last_drawn = draw_of(&self.pack, &self.geom, &m.e, first, m.flip);
    }

    /// fn107 @20E6 + fn108 @2136: dead-pending cats revive in place.
    fn fn107_fn108(&mut self) {
        for cat in &mut self.cats {
            if cat.life == 2 {
                cat.life = 4;
            }
        }
        for cat in &mut self.cats {
            if cat.life == 4 {
                cat.fn100();
            }
        }
    }

    /// Mower blade bounding box: channel 2 of the compound parts table — the
    /// same rect fn12 @3FEC hands the kill test, without fn12's visibility
    /// gate. Used for the lawn scrub and fn92's avoid heading.
    pub fn blade_rect(&self) -> Option<[i32; 4]> {
        self.mower_channel_rect(2)
    }

    /// The mower's vtbl +0x34: shown, and its drawn frame on screen.
    fn mower_on_screen(&self) -> bool {
        self.mower.active
            && self
                .mower_frame_rect()
                .is_some_and(|r| r[2] > 0 && r[0] < SCREEN_W && r[3] > 0 && r[1] < SCREEN_H)
    }

    /// `fn15` @4254: park the mower just off the edge it is about to drive
    /// in from — SetPos x = field right + 60 when the step is negative, field
    /// left − 60 otherwise (library SetPos: the frame's centre). GAP(fn15 y):
    /// see `MOWER_FIRST_ROW_Y`; the row is one blade height above the first
    /// pass's, because `fn14` steps down straight after its first `fn15`.
    fn fn15(&mut self) {
        let cx = if self.mower.dir < 0 { self.field_r() + 0x3C } else { self.field_l() - 0x3C };
        let first = self.mower.first;
        let w = self.geom.get(&(first as u32)).map(|g| g.w).unwrap_or(0);
        let (ox, _) = frame_offset(&self.geom, first, first, self.mower.flip);
        self.mower.e.x = cx - (w >> 1) - ox;
        self.mower.e.y = MOWER_FIRST_ROW_Y - self.blade_height();
    }

    fn blade_height(&self) -> i32 {
        self.blade_rect().map(|r| r[3] - r[1]).unwrap_or(43)
    }

    /// `fn14` @41D4, the row step on every ENTER out of state 10: `fn15`
    /// the first time (+0x122); then read the blade rect (`fn12` with
    /// wantHidden = 1) and, if its bottom is past the field's, `fn15` again
    /// (back to the top row), else move down by the blade's height with x
    /// untouched — the mower comes back in exactly where it drove off.
    fn fn14(&mut self) {
        if self.mower.first_place {
            self.mower.first_place = false;
            self.fn15();
        }
        let Some(blade) = self.blade_rect() else { return };
        if blade[3] > self.field_b() {
            self.fn15();
        } else {
            self.mower.e.y += blade[3] - blade[1];
        }
    }

    /// The module's SetState for the mower (`fn53` @645C): EXIT the state
    /// being left, store, then ENTER inline. `fn08` @36E2's EXIT branches:
    /// 0x4006/9 → 407 (545 with +0x124), 0x4007/8 → 422, 0x4002/3 → 532
    /// (`fn08_exit_decal`); 0x400A → +0x11C = 1 and the step reverses;
    /// 0x400D → +0x12E = 0. ENTER branches: 0x8006..9 Cat Chop, 0x8002/3 Head
    /// Chop, 0x800C → +0x12E = 1, 0x800A → Hide, buzz off, drag timer and
    /// +0x12E cleared, `fn107`/`fn108`, return (no run). Every other ENTER
    /// runs `fn18` @446E (SetRun from `g084A`, then `fn17` @43F8: facing to
    /// the step's sign and, coming out of 10, `fn14` + Show) and the `+0x84`
    /// tick that paints the new run's first frame.
    fn mower_set_state(&mut self, state: u16) {
        let old = self.mower.state;
        self.fn08_exit_decal(old);
        match old {
            13 => self.mower.deck_busy = false,
            10 => {
                self.mower.launch = true;
                self.mower.dir = if self.mower.dir < 0 { 1 } else { -1 };
            }
            _ => {}
        }
        self.mower.state = state;
        self.mower.seq_done = false; // fn0204 zeroes +0x46
        self.mower.first_draw = true; // fn028A sets +0x48
        match state {
            6..=9 => self.pending_sounds.push(SND_CAT_CHOP),
            2 | 3 => self.pending_sounds.push(SND_HEAD_CHOP),
            12 => self.mower.deck_busy = true,
            10 => {
                self.mower.active = false;
                self.mower.buzz = false;
                self.mower.drag_timer = 0;
                self.mower.deck_busy = false;
                self.fn107_fn108();
                return;
            }
            _ => {}
        }
        if let Some((_, fno)) = MOWER_RUNS.iter().find(|r| r.0 == state).copied() {
            let old_first = self.mower.first;
            self.mower.first = fno as i32;
            self.mower.last = self.run_last(self.mower.first);
            set_run(&self.geom, &mut self.mower.e, old_first, fno as i32, self.mower.flip, false);
        }
        self.mower.flip = self.mower.dir > 0;
        if old == 10 {
            self.fn14();
            self.mower.active = true;
        }
        self.mower_tick84();
    }

    /// `fn08` @36E2's four decal branches, keyed on the EXIT message:
    ///
    /// | exit | listing | art | what it is |
    /// |---|---|---|---|
    /// | `0x4006` / `0x4009` | 213872 | **407** | splattered cat, 74×43 |
    /// | (same, when `+0x124` is set) | 213860 | **545** | blood only, 57×43 |
    /// | `0x4007` / `0x4008` | 213884 | **422** | splattered cat, 76×43 |
    /// | `0x4002` / `0x4003` | 213A62 | **532** | blood only, revenge kill |
    ///
    /// States 6/9 are the ordinary kill runs, 7/8 the ones that chop the
    /// flattened cat on the drag's reverse pass (+0x12E up), 2/3 the revenge
    /// pair (`fn13` @410E returns exactly those; run 4 hands on to 6/9
    /// through `fn10`), so every completed kill leaves a mark on the lawn.
    /// QEMU capture at t=27.3 s shows art 407 appearing behind the mower and
    /// still there at t=59.9 s.
    ///
    /// +0x124 is raised by `fn10` @3D9E in the same breath as it picks the
    /// drag — i.e. just before THIS exit — and cleared here, so the 545 marks
    /// the spot the flattened cat was hit first; its 422 lands where the
    /// mower backs over it.
    fn fn08_exit_decal(&mut self, leaving: u16) {
        let art = match leaving {
            6 | 9 => {
                if self.mower.dragging {
                    self.mower.dragging = false;
                    545
                } else {
                    407
                }
            }
            7 | 8 => 422,
            2 | 3 => 532,
            _ => return,
        };
        self.fn16_stamp(art);
    }

    /// `fn16` @4358: read the blade rect (`fn12`, pushing 1 so a parked mower
    /// still answers), collapse it to its centre point, and hand that plus
    /// the art id to `fn56` @45DE — a 16-slot table on `g0B64`.
    ///
    /// `fn57` @46B6 walks that table every frame, re-blits each live slot and
    /// frees it once the mower's rect no longer overlaps it vertically. The
    /// table is a REDRAW list, not the decal's lifetime: the stamp is painted
    /// into the lawn and stays there — freeing the slot only stops fighting
    /// the mower's own erase. Ported as a permanent ground sprite, which is
    /// what the capture shows. (ENTER 10 also zeroes that table — L131
    /// `fn4C14` over `g0B64 + 0x46`, 224 bytes — which again only ends the
    /// redraws.)
    fn fn16_stamp(&mut self, art: u32) {
        // fn12 with wantHidden = 1 (listing 21420A / the fn16 push site):
        // an off-screen mower still reports its blade rect.
        let Some(rc) = self.mower_channel_rect(2) else { return };
        let (cx, cy) = ((rc[0] + rc[2]) / 2, (rc[1] + rc[3]) / 2);
        let Some(gm) = self.geom.get(&art).copied() else { return };
        self.decals.push(Decal {
            x: cx - (gm.w >> 1),
            y: cy - (gm.h >> 1),
            fno: art,
        });
    }

    /// `fn08` @36E2, one update of the mower.
    fn mower_advance(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        // Top of fn08, every message: the ctor's +0x11E starts the song
        // (L134 `fn0C98`, `cmid` 40 — `song_plays` starts at 1 for it) and
        // arms the 16 s wait.
        if self.mower.music_cue {
            self.mower.music_cue = false;
            self.mower.wait_until = now + 16000;
        }
        let tail = match self.mower.state {
            10 => {
                self.mower_update10(ctx);
                false
            }
            1 | 4 | 5 | 11 => self.mower_drive(ctx),
            _ => true,
        };
        if tail {
            // fn08 tail (listing 213C7E): run not done → `+0x84` tick. Run
            // done (+0x46) → dequeue a kill message (`+0x40` → `fn13`) or
            // ask `fn10`, then SetState, whose ENTER paints the new run's
            // first frame. A kill run's EXIT is where `fn08` stamps its
            // decal, so the splat lands the update after the run's last
            // frame, under the parked mower.
            if !self.mower.seq_done {
                self.mower_tick84();
            } else {
                let st = self.mower.state;
                let next = match self.mower.kill_msgs.pop_front() {
                    Some(ix) => self.fn13(ctx, ix),
                    None => self.fn10(st, ctx),
                };
                self.mower_set_state(next);
            }
        }

        // Scrub lawn under blade
        if self.mower.active {
            if let Some(blade) = self.blade_rect() {
                self.tufts.retain(|s| {
                    s.x < blade[0] || s.x > blade[2] || s.y < blade[1] || s.y > blade[3]
                });
                self.flowers.retain(|s| {
                    s.x < blade[0] || s.x > blade[2] || s.y < blade[1] || s.y > blade[3]
                });
            }
        }
    }

    /// `fn08`'s update-10 branch (listing 213B06-213C0C): pick the next
    /// pass. Revenge Always (`g0B28` = 16) is always a revenge pass;
    /// otherwise a pass after the first (+0x120), not already in revenge,
    /// with 60 s of normal phase behind it (+0x134) rolls
    /// `Random15() % 16 < g0B28`. Entering revenge raises +0x12C and
    /// re-arms the song wait, then state 1 (a cat at the wheel). A normal
    /// pick drops +0x12C (re-arming the wait if it was up) and enters 5.
    /// Each side stamps the start of its phase only when it changes.
    ///
    /// GAP(+0x4C): both toggles also call the state object's vtbl +0x4C,
    /// and the revenge-off side `fn66` @355E (sixteen 0xFFFF pairs on
    /// `g0B40`); neither is decoded, neither ported. GAP(+0x128): both
    /// toggles zero the counter `fn10` cycles.
    fn mower_update10(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        let setting = self.revenge as u32;
        let revenge = setting == 16
            || (!self.mower.first_pass
                && !self.mower.revenge
                && self.mower.normal_since + 60000 < now
                && ctx.rng.pct(16) < setting);
        let m = &mut self.mower;
        if revenge {
            if !m.revenge {
                m.revenge = true;
                m.wait_until = now + 16000;
                self.song_plays += 1;
            }
            m.first_pass = false;
            if m.revenge_since <= m.normal_since {
                m.revenge_since = now;
            }
            self.mower_set_state(1);
        } else {
            if m.revenge {
                m.revenge = false;
                m.wait_until = now + 16000;
                self.song_plays += 1;
            }
            m.first_pass = false;
            if m.normal_since <= m.revenge_since {
                m.normal_since = now;
            }
            self.mower_set_state(5);
        }
    }

    /// `fn08`'s drive branch (states 1/4/5/11, listing 213896-213A3A).
    /// Returns true when the update falls through to the tail.
    ///
    /// - A revenge drive parks (no move, no edge test) while any cat is on
    ///   screen (`fn106`), and every drive parks until the song wait ends.
    /// - +0x11C drops once the launched mower is on screen.
    /// - Not in revenge, not in 4: an expired drag timer → SetState(12);
    ///   with no flattened cat left (`fn104`(3)) a busy deck → SetState(13),
    ///   and an armed timer is dropped.
    /// - SetPos x + step, or x − 2·step while +0x12E is up (state 11 backs up
    ///   over the flattened cat at double speed).
    /// - Buzz on/off with the checkbox (+0x126, `g0B32`).
    /// - Not launching, no drag timer, deck not busy and now off screen →
    ///   SetState(10). The mower is therefore fully gone before it turns,
    ///   and the update-10 pick sends it straight back in on the next row.
    fn mower_drive(&mut self, ctx: &mut Ctx) -> bool {
        let now = ctx.now_ms;
        if self.mower.revenge && self.fn106() {
            return true;
        }
        if self.mower.wait_until != 0 {
            if now < self.mower.wait_until {
                return true;
            }
            self.mower.wait_until = 0;
        }
        if self.mower.launch && self.mower_on_screen() {
            self.mower.launch = false;
        }
        if !self.mower.revenge && self.mower.state != 4 {
            if self.mower.drag_timer != 0 && self.mower.drag_timer <= now {
                self.mower.drag_timer = 0;
                self.mower_set_state(12);
                return false;
            }
            if self.fn104(3).is_none() {
                if self.mower.deck_busy {
                    self.mower_set_state(13);
                    return false;
                }
                self.mower.drag_timer = 0;
            }
        }
        let step = self.mower.dir * self.speed_code;
        self.mower.e.x += if self.mower.deck_busy { -2 * step } else { step };
        if !self.mower.buzz && self.mower_sound {
            self.mower.buzz = true;
        } else if self.mower.buzz && !self.mower_sound {
            self.mower.buzz = false;
        }
        if !self.mower.launch
            && self.mower.drag_timer == 0
            && !self.mower.deck_busy
            && !self.mower_on_screen()
        {
            self.mower_set_state(10);
            return false;
        }
        true
    }

    fn flutterby_advance(&mut self, ctx: &mut Ctx) {
        let (spawn, x, y, oct) = if self.flutterby.is_none() {
            if ctx.rng.pct(256) < 32 {
                let edge = ctx.rng.pct(4);
                let (x, y, oct) = match edge {
                    0 => (0, ctx.rng.pct(SCREEN_H as u32) as i32, 0),
                    1 => (SCREEN_W, ctx.rng.pct(SCREEN_H as u32) as i32, 4),
                    2 => (ctx.rng.pct(SCREEN_W as u32) as i32, 0, 2),
                    _ => (ctx.rng.pct(SCREEN_W as u32) as i32, SCREEN_H, 6),
                };
                (true, x, y, oct)
            } else {
                (false, 0, 0, 0)
            }
        } else {
            (false, 0, 0, 0)
        };

        if spawn {
            let st = oct_state(oct);
            let (fno, flip) = FLB_STATES
                .iter()
                .find(|s| s.0 == st)
                .map(|s| (s.1, s.2))
                .unwrap_or((360, false));
            let mut flb = Flutterby {
                e: Ent { x, y, d6: fno as i32 },
                state: st,
                first: fno as i32,
                last: self.run_last(fno as i32),
                flip,
                active: true,
                armed: false,
                oct,
                last_drawn: None,
            };
            let first = flb.first;
            flb.last_drawn = draw_of(&self.pack, &self.geom, &flb.e, first, flip);
            self.flutterby = Some(flb);
            return;
        }

        if let Some(flb) = &mut self.flutterby {
            if !flb.active {
                self.flutterby = None;
                return;
            }

            // Flee jink if mower is nearby
            let m_dx = flb.e.x - self.mower.e.x;
            let m_dy = flb.e.y - self.mower.e.y;
            if !flb.armed && self.mower.active && (m_dx.abs() + m_dy.abs() < 200) {
                let a = (m_dy as f64).atan2(m_dx as f64).to_degrees();
                let b = (((a / 45.0).round() as i32).rem_euclid(8)) as usize;
                let s = FLEE_T[b];
                let ns = match s {
                    5 | 8 => 3,
                    6 | 7 | 9 => 4,
                    _ => s,
                };
                flb.state = ns;
                flb.armed = true;
                flb.oct = b;
                if let Some((_, fno, flip)) = FLB_STATES.iter().find(|st| st.0 == ns).copied() {
                    let new_last = run_last_in(&self.pack, fno as i32);
                    let old_first = flb.first;
                    flb.first = fno as i32;
                    flb.last = new_last;
                    flb.flip = flip;
                    set_run(&self.geom, &mut flb.e, old_first, fno as i32, flip, false);
                }
            }

            let _ = adv_loop(&self.geom, &mut flb.e, flb.first, flb.last, flb.flip);
            let first = flb.first;
            flb.last_drawn = draw_of(&self.pack, &self.geom, &flb.e, first, flb.flip);

            if flb.e.x < -30 || flb.e.x > SCREEN_W + 30 || flb.e.y < -30 || flb.e.y > SCREEN_H + 30
            {
                flb.active = false;
            }
        }
    }

    /// `fn63` @31C6, the lawn planter `g0B40`: `fn64` @31E0 then `fn65`
    /// @32B6. The planter is ONE sprite that toggles: shown → Hide; hidden
    /// → SetPos (centre) to `(Random15() % field width, Random15() % field
    /// height)`, SetRun 5 (a 1×6 grass blade) or, on `Random15() % 256 < 4`,
    /// one of the six flowers `g06BE` = [10, 12, 14, 16, 18, 20]
    /// (`Random15() % 6`, A5 decode), then Show. `fn22` @5056 calls it once
    /// in the actor pump and ten more times after the cats, each followed by
    /// a `g0B64 +0xC` draw — so every other call stamps a plant into the
    /// lawn: 5 or 6 per 100 ms tick. The stamps stay until the mower's blade
    /// paints over them (the lawn scrub in `mower_advance`).
    ///
    /// GAP(fn64 / g06BA): on a revenge drive a flower roll can instead drop
    /// one of `g06BA` = [25, 27] in the mower's wake (`Random15() % 15 < 4`,
    /// a 16-slot table on the planter that `fn64` clears as the blade passes
    /// and `fn66` @355E resets). Not ported.
    fn fn63(&mut self, ctx: &mut Ctx) {
        if self.planter_shown {
            self.planter_shown = false;
            return;
        }
        let x = ctx.rng.pct((self.field_r() - self.field_l()) as u32) as i32;
        let y = ctx.rng.pct((self.field_b() - self.field_t()) as u32) as i32;
        let fno = if ctx.rng.pct(256) < 4 {
            PLANTER_FLOWERS[ctx.rng.pct(PLANTER_FLOWERS.len() as u32) as usize]
        } else {
            SCATTER_TUFT
        };
        let (w, h) = self.geom.get(&fno).map(|g| (g.w, g.h)).unwrap_or((1, 6));
        let s = Scatter { fno, x: x - (w >> 1), y: y - (h >> 1), w, h };
        if fno == SCATTER_TUFT {
            if self.tufts.len() >= MAX_TUFTS {
                self.tufts.remove(0);
            }
            self.tufts.push(s);
        } else {
            self.flowers.push(s);
        }
        self.planter_shown = true;
    }
}

impl Cat {
    /// Scratch placeholder for take/replace dances (never observed).
    const EMPTY: Cat = Cat {
        e: Ent { x: 0, y: 0, d6: 0 },
        state: 0,
        prev: 0,
        first: 0,
        last: 0,
        flip: false,
        visible: false,
        seq_done: false,
        pending: false,
        enter_ms: 0,
        repeat: 0,
        zside: 0,
        index: 0,
        life: 0,
        off: false,
        far_off: false,
        resolved: false,
        busy: false,
        squash: 0,
        squash_dx: 0,
        squash_dy: 0,
        last_drawn: None,
    };

    /// fn100 @1B76: revive in place — flags only, no move.
    fn fn100(&mut self) {
        self.resolved = true; // +0x12C
        self.busy = false; // +0x12E
        self.squash = 0; // +0x130
        self.life = 1; // +0x11E
        self.far_off = false; // +0x12A
        self.off = true; // +0x128
    }
}

impl MowinBoris {
    fn enter_cue(&self, state: u16) -> Option<u32> {
        // fn90: enter 0x8001 -> bank slot 0 (Meow); enter 0x8014 -> slot 1
        // (Purr); both gated by g0B32 == 0 (the sound checkbox).
        if self.mower_sound {
            return None;
        }
        match state {
            1 => Some(SND_MEOW),
            20 => Some(SND_PURR),
            _ => None,
        }
    }
}

impl Module for MowinBoris {
    fn name(&self) -> &'static str {
        "Mowin' Boris"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Mower Speed".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 0, // Sluggish
            },
            ControlDef {
                name: "Cats".into(),
                kind: ControlKind::Popup {
                    base: 0,
                    // The original drew Cats and Revenge as sliders (sVal/
                    // sUnt 1001 and 1002, no MENU); this port steps them as
                    // popups, one item per sUnt word, read from the pack.
                    items: self.pack.popup_items_from_slider_words(1001, 6),
                },
                default: 0,
            },
            ControlDef {
                name: "Revenge".into(),
                kind: ControlKind::Popup {
                    base: 0,
                    items: self.pack.popup_items_from_slider_words(1002, 5),
                },
                default: 2, // Sometimes
            },
            ControlDef {
                name: "Mower Sound".into(),
                kind: ControlKind::Checkbox,
                default: 0, // off
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => {
                self.speed_code = if value < 25 {
                    1
                } else if value < 50 {
                    2
                } else if value < 75 {
                    5
                } else {
                    10
                };
            }
            1 => {
                self.cats_wanted = value.clamp(0, 5) + 1;
                self.sync_cats();
            }
            2 => {
                self.revenge = match value {
                    0 => 0,
                    1 => 1,
                    2 => 3,
                    3 => 5,
                    _ => 16,
                };
            }
            3 => {
                self.mower_sound = value != 0;
            }
            _ => {}
        }
    }

    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    fn music(&self) -> Option<(u32, u32)> {
        self.pack.has_song(SONG_DAWN_CUE).then_some((SONG_DAWN_CUE, self.song_plays))
    }

    fn loop_sound(&self) -> Option<u32> {
        // +0x126: raised by the first drive update that actually moves (past
        // the song wait), dropped by ENTER 10 or the checkbox going off.
        if self.mower_sound && self.mower.buzz {
            Some(SND_MOWER_BUZZ)
        } else {
            None
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        if now < self.next_due {
            return;
        }
        self.next_due = now + 100;
        self.clock += 1;

        self.pending_sounds.clear();

        // 1. Flutterby advance
        self.flutterby_advance(ctx);

        // 2. Mower advance
        self.mower_advance(ctx);

        // 3. Lawn planter (fn63, the actor-pump call)
        self.fn63(ctx);

        // 4. Cats advance
        for i in 0..self.cats.len() {
            // Enter cues fire from the SetState path (fn90 enter branch)
            let pre_state = self.cats[i].state;
            self.cat_update(i, ctx);
            if self.cats[i].state != pre_state {
                if let Some(id) = self.enter_cue(self.cats[i].state) {
                    self.pending_sounds.push(id);
                }
            }
        }

        // 5. fn22's ten more planter calls after the draw (listing 215154).
        for _ in 0..10 {
            self.fn63(ctx);
        }

        for id in self.pending_sounds.drain(..) {
            ctx.sounds.push(id);
        }

    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // 1. Meadow scatter (tufts & flowers)
        for s in &self.tufts {
            if let Some(f) = self.pack.frame(BASE, s.fno) {
                out.push(SpriteDraw {
                    pal: 0,
                    png: f.png.clone(),
                    x: s.x,
                    y: s.y,
                    flip: false,
                });
            }
        }
        for s in &self.flowers {
            if let Some(f) = self.pack.frame(BASE, s.fno) {
                out.push(SpriteDraw {
                    pal: 0,
                    png: f.png.clone(),
                    x: s.x,
                    y: s.y,
                    flip: false,
                });
            }
        }

        // 2. Decals
        for d in &self.decals {
            if let Some(f) = self.pack.frame(BASE, d.fno) {
                out.push(SpriteDraw {
                    pal: 0,
                    png: f.png.clone(),
                    x: d.x,
                    y: d.y,
                    flip: false,
                });
            }
        }

        // 3. Squashed/dead cats behind the mower (fn94 reorder)
        for cat in &self.cats {
            if cat.visible && cat.squash != 0 {
                if let Some(sd) = &cat.last_drawn {
                    out.push(sd.clone());
                }
            }
        }

        // 4. Mower
        if self.mower.active && self.mower.state != 10 {
            if let Some(sd) = &self.mower.last_drawn {
                out.push(sd.clone());
            }
        }

        // 5. Live cats in front (fn94 z-order)
        for cat in &self.cats {
            if cat.visible && cat.squash == 0 {
                if let Some(sd) = &cat.last_drawn {
                    out.push(sd.clone());
                }
            }
        }

        // 6. Flutterby
        if let Some(flb) = &self.flutterby {
            if flb.active {
                if let Some(sd) = &flb.last_drawn {
                    out.push(sd.clone());
                }
            }
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::collections::{HashSet, VecDeque};

    fn boris() -> Option<(MowinBoris, Ctx)> {
        let pack = Pack::load(std::path::Path::new("../assets/mowin-boris")).ok()?;
        let m = build(pack)?;
        let ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };
        Some((m, ctx))
    }

    #[test]
    fn dawn_cue_plays_once_from_the_start_on_the_music_channel() {
        let Some((m, _)) = boris() else { return };
        assert_eq!(m.music(), Some((SONG_DAWN_CUE, 1)));
    }

    #[test]
    fn mowin_boris_smoke() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        let mut out = Vec::new();
        let mut drew = false;
        for _ in 0..3000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            out.clear();
            m.sprites(&mut out);
            drew |= !out.is_empty();
        }
        assert!(drew, "smoke run drew sprites");
    }

    #[test]
    fn every_named_run_ships() {
        let Some((m, _)) = boris() else { return };
        for (st, fno, _) in BORIS_STATES {
            assert!(
                m.pack.frame(BASE, *fno).is_some(),
                "cat state {st} first frame {fno} must exist in pack"
            );
        }
        for (st, fno) in MOWER_RUNS {
            assert!(
                m.pack.frame(BASE, *fno).is_some(),
                "mower state {st} first frame {fno} must exist in pack"
            );
        }
        for (st, fno, _) in FLB_STATES {
            assert!(
                m.pack.frame(BASE, *fno).is_some(),
                "flutterby state {st} first frame {fno} must exist in pack"
            );
        }
    }

    #[test]
    fn the_machine_gets_around() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        let mut cat_states = HashSet::new();
        for _ in 0..6000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if let Some(cat) = m.cats.first() {
                cat_states.insert(cat.state);
            }
        }
        assert!(
            cat_states.len() >= 5,
            "cat visited multiple state families: {cat_states:?}"
        );
    }

    #[test]
    fn cats_enter_from_field_edges_via_fn101() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        // After the first state pick every cat must be placed at/beyond a
        // field edge (fn101) rather than parked at the library default (0,0).
        for _ in 0..50 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        for cat in &m.cats {
            assert!(
                cat.e.x <= 40 || cat.e.x >= SCREEN_W - 40 || cat.e.y <= 40 || cat.e.y >= SCREEN_H - 40,
                "cat not edge-placed: pos=({},{})",
                cat.e.x,
                cat.e.y
            );
        }
    }

    #[test]
    fn squashed_cat_revives_in_place_not_at_a_random_edge() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        // Put a cat under the mower's path and let the machine work.
        m.set_control(3, 1);
        m.mower.active = true;
        m.mower.state = 5;
        m.mower.e.x = 320;
        m.mower.e.y = 240;
        m.cats[0].e.x = 320;
        m.cats[0].e.y = 240;
        let (mut saw_squash, mut saw_revive) = (false, false);
        let mut revive_pos = (0, 0);
        for _ in 0..1200 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if m.cats[0].squash != 0 {
                saw_squash = true;
            }
            if m.cats[0].life == 1 && saw_squash && !saw_revive {
                saw_revive = true;
                revive_pos = (m.cats[0].e.x, m.cats[0].e.y);
            }
        }
        assert!(saw_squash, "fn93 never registered the mower hit");
        // fn107/fn108/fn100 revive in place (no SetPos); the cat stays hidden
        // in state 10 until its run finishes, then fn101 re-places it at an
        // edge on the way out. So: life must return to 1 at least once, and
        // the revive itself must be within a blade-length of the squash.
        assert!(saw_revive, "cat never revived (fn107/fn108/fn100)");
        assert!(
            (revive_pos.0 - 320).abs() < 250 && (revive_pos.1 - 240).abs() < 250,
            "revive moved the cat: pos=({revive_pos:?})"
        );
    }

    #[test]
    fn killed_cat_hides_and_reenters_from_an_edge() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        m.set_control(3, 1);
        m.mower.active = true;
        m.mower.state = 5;
        m.mower.e.x = 320;
        m.mower.e.y = 240;
        m.cats[0].e.x = 320;
        m.cats[0].e.y = 240;
        m.cats[0].visible = true;
        // 1. run to the kill
        let mut killed = false;
        for _ in 0..1200 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if m.cats[0].life == 2 {
                killed = true;
                break;
            }
        }
        assert!(killed, "the mower never killed the cat");
        assert_eq!(m.cats[0].state, 10);
        assert!(!m.cats[0].visible, "fn88: enter 10 hides the cat");
        // 2. hidden in state 10: never drawn, never re-hit, until it leaves 10
        let mut left = false;
        for _ in 0..3000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let c = &m.cats[0];
            if c.state != 10 {
                left = true;
                break;
            }
            assert!(!c.visible, "state-10 cat became visible");
            assert_eq!(c.squash, 0, "fn93 hit a hidden cat");
        }
        assert!(left, "cat never left state 10");
        let c = &m.cats[0];
        assert!(c.visible, "fn89: exit 10 shows the cat");
        if c.busy || c.life == 3 {
            // fn10 picked it for the drag: fn96(10) sends it to 5/8 and
            // fn101 lays it on the (on-screen) mower's blade.
            assert!(matches!(c.state, 5 | 8), "drag pick went to {}", c.state);
            return;
        }
        assert!(L10.contains(&c.state), "fn96(10) picks from g0046, got {}", c.state);
        // fn101 (prev == 10) puts every entry state outside the field
        let (cx, cy) = visual_centre(&m.geom, &c.e, c.first, c.flip);
        let outside = cx < m.field_l() || cx > m.field_r() || cy < m.field_t() || cy > m.field_b();
        assert!(outside, "re-entry not at an edge: centre=({cx},{cy}) state={}", c.state);
    }

    #[test]
    fn cat_cues_are_enter_only_and_gated() {
        let Some((mut m, mut ctx)) = boris() else { return };
        // fn90 cues ONLY on enter 0x8001 (Meow) and enter 0x8014 (Purr), both
        // gated by g0B32 (the sound checkbox). Unit-check the mapping...
        m.set_control(3, 1); // muted
        assert_eq!(m.enter_cue(1), None);
        assert_eq!(m.enter_cue(20), None);
        m.set_control(3, 0); // unmuted
        assert_eq!(m.enter_cue(1), Some(SND_MEOW));
        assert_eq!(m.enter_cue(20), Some(SND_PURR));
        assert_eq!(m.enter_cue(10), None);
        assert_eq!(m.enter_cue(36), None);
        // ...and the tick path: cues fire only when a cat's state CHANGES.
        let mut pace = Pacer::new(&m);
        let mut cues_on_change_only = true;
        let mut prev = m.cats[0].state;
        for _ in 0..2000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let st = m.cats[0].state;
            let cued = ctx.sounds.iter().any(|s| *s == SND_MEOW || *s == SND_PURR);
            if cued && st == prev {
                cues_on_change_only = false;
            }
            prev = st;
            ctx.sounds.clear();
        }
        assert!(cues_on_change_only, "cue fired without a state change");
    }

    #[test]
    fn mowin_boris_checkbox_gates_only_the_buzz() {
        // fn08's +0x126: with the box ticked the buzz starts on the first
        // drive update past the 16 s song wait (not during it — the mower is
        // already in state 5 then), and stops when the box is unticked.
        let Some((mut m, mut ctx)) = boris() else { return };
        m.set_control(3, 0); // off
        assert!(!m.mower_sound);
        assert_eq!(m.loop_sound(), None);

        m.set_control(3, 1); // on
        assert!(m.mower_sound);
        let mut pace = Pacer::new(&m);
        while ctx.now_ms < 15_500 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            assert_eq!(m.loop_sound(), None, "buzz during the song wait at {} ms", ctx.now_ms);
        }
        while ctx.now_ms < 17_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        assert_eq!(m.loop_sound(), Some(SND_MOWER_BUZZ));
        m.set_control(3, 0);
        assert_eq!(m.loop_sound(), None);
    }

    #[ignore]
    #[test]
    fn trace_boris() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        for _ in 0..30000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let t = pace.now_ms();
            if m.clock % 100 == 0 {
                let cat_info = m
                    .cats
                    .first()
                    .map(|c| {
                        format!(
                            "st={} d6={} pos=({},{}) life={} sq={} off={} far={}",
                            c.state, c.e.d6, c.e.x, c.e.y, c.life, c.squash, c.off, c.far_off
                        )
                    })
                    .unwrap_or_default();
                println!(
                    "t={:6} | mower: st={} d6={} pos=({},{}) act={} | cat: {}",
                    t,
                    m.mower.state,
                    m.mower.e.d6,
                    m.mower.e.x,
                    m.mower.e.y,
                    m.mower.active,
                    cat_info
                );
            }
        }
    }

    #[test]
    fn mowin_boris_erases_under_blade() {
        let Some((mut m, mut ctx)) = boris() else { return };
        m.mower.e.x = 200;
        m.mower.e.y = 200;
        let blade = m.blade_rect().unwrap();
        m.tufts.push(Scatter {
            fno: SCATTER_TUFT,
            x: (blade[0] + blade[2]) / 2,
            y: (blade[1] + blade[3]) / 2,
            w: 8,
            h: 8,
        });
        m.tufts.push(Scatter {
            fno: SCATTER_TUFT,
            x: blade[2] + 50,
            y: blade[3] + 50,
            w: 8,
            h: 8,
        });
        assert_eq!(m.tufts.len(), 2);
        m.mower.state = 5;
        m.mower.active = true;
        m.mower_advance(&mut ctx);
        assert!(m.tufts.iter().all(|t| t.x > blade[2]));
    }
    #[ignore]
    #[test]
    fn trace_kill_cycle() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        let mut prev: Vec<(u16, i32, i32, i32, u16, u16, bool, bool)> = Vec::new();
        let mut watch: Vec<u64> = Vec::new();
        let mut lines = 0;
        for _ in 0..60000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let t = pace.now_ms() as u64;
            let mw = format!("mw st={} d6={} pos=({},{}) act={}", m.mower.state, m.mower.e.d6, m.mower.e.x, m.mower.e.y, m.mower.active);
            for (i, c) in m.cats.iter().enumerate() {
                let cur = (c.state, c.e.d6, c.e.x, c.e.y, c.life, c.squash, c.busy, c.off);
                if prev.len() <= i { prev.push(cur); watch.push(0); continue; }
                if c.squash != 0 { watch[i] = t + 20000; }
                if t < watch[i] && prev[i] != cur {
                    println!("t={t:6} cat{i} st={} prev={} d6={} pos=({},{}) life={} sq={} busy={} off={} far={} | {mw}", c.state, c.prev, c.e.d6, c.e.x, c.e.y, c.life, c.squash, c.busy, c.off, c.far_off);
                    lines += 1;
                }
                prev[i] = cur;
            }
            if lines > 350 { break; }
        }
    }
    /// Diagnostic: the splat delay. At the capture's panel settings (Manic,
    /// 3 cats, Revenge Frequent, sound off), per mown cat: the cat's hide
    /// (ladder -> state 10), the mower's kill-run ENTER, and the tick `fn16`
    /// stamps the decal (kill-run EXIT); plus kill runs and decals per 60 s.
    /// Capture reference (mowin-boris-av.mp4, run 9): hide 26.25 s, kill
    /// run parks the mower 26.40 s, stamp 26.80 s (hide->stamp ~550 ms,
    /// enter->stamp 400 ms); 1 decal in the first 60 s.
    #[ignore]
    #[test]
    fn trace_splat_delay() {
        let mut direct: Vec<u64> = Vec::new();
        let mut via4: Vec<u64> = Vec::new();
        let mut enter_to_stamp: Vec<u64> = Vec::new();
        let (mut hides_total, mut stamps_total, mut d60_total, mut runs_total) = (0, 0, 0, 0);
        let mut unstamped = 0;
        let mut unpaired = 0;
        let seeds: Vec<u64> = (1..=16).map(|k| k * 7919).collect();
        for &seed in &seeds {
            let Some((mut m, mut ctx)) = boris() else { return };
            ctx.rng = RandomLong::new(seed);
            ctx.rng15 = Random15::new(seed as u32);
            m.set_control(0, 100);
            m.set_control(1, 2);
            m.set_control(2, 3);
            m.set_control(3, 0);
            let mut pace = Pacer::new(&m);
            let mut hides: VecDeque<u64> = VecDeque::new();
            let mut enter: Option<(u64, bool)> = None;
            let mut prev_state = m.mower.state;
            let mut prev_d6 = m.mower.e.d6;
            let mut prev_sq: Vec<i32> = m.cats.iter().map(|c| c.squash as i32).collect();
            let mut clock = m.clock;
            let mut d60 = 0;
            while ctx.now_ms < 120_000 {
                pace.advance(&mut ctx);
                let nd = m.decals.len();
                m.tick(&mut ctx);
                ctx.sounds.clear();
                if m.clock == clock {
                    continue;
                }
                clock = m.clock;
                let t = ctx.now_ms;
                if t <= 60_000 {
                    d60 = m.decals.len();
                }
                let st = m.mower.state;
                if m.decals.len() > nd {
                    stamps_total += 1;
                    let art = m.decals[nd].fno;
                    if let Some((te, was4)) = enter.take() {
                        if !was4 {
                            enter_to_stamp.push(t - te);
                        }
                        // Pair with the oldest hide still waiting at ENTER
                        // (the mower's queue is FIFO); the revenge pair
                        // (532) is not a mown-cat splat.
                        if art != 532 {
                            while hides.front().is_some_and(|&h| h + 5000 < te) {
                                hides.pop_front();
                                unpaired += 1;
                            }
                            if let Some(h) = hides.pop_front() {
                                if was4 {
                                    via4.push(t - h)
                                } else {
                                    direct.push(t - h)
                                }
                            }
                        }
                    }
                }
                // A kill run ENTERs on a state change, or on a same-state
                // re-entry (a queued kill dequeued at the end of a kill).
                let reenter = st == prev_state && m.mower.e.d6 == m.mower.first && prev_d6 != m.mower.first;
                if st != prev_state || reenter {
                    if matches!(st, 2 | 3 | 4 | 6 | 7 | 8 | 9) {
                        runs_total += 1;
                        if prev_state != 4 || reenter {
                            enter = Some((t, st == 4));
                        }
                    }
                    prev_state = st;
                }
                prev_d6 = m.mower.e.d6;
                for (i, c) in m.cats.iter().enumerate() {
                    if prev_sq[i] >= 3 && c.squash == 0 && c.state == 10 {
                        hides.push_back(t);
                        hides_total += 1;
                    }
                    prev_sq[i] = c.squash as i32;
                }
            }
            unstamped += hides.len();
            d60_total += d60;
        }
        let stat = |v: &mut Vec<u64>| {
            v.sort();
            if v.is_empty() {
                return "n=0".to_string();
            }
            format!("n={} min={} med={} max={} ms", v.len(), v[0], v[v.len() / 2], v[v.len() - 1])
        };
        let n = seeds.len() as f64;
        println!("SPLAT hide->stamp, kill run 6/9 direct: {}", stat(&mut direct));
        println!("SPLAT hide->stamp, via drag-under run 4: {}", stat(&mut via4));
        println!("SPLAT kill ENTER->stamp (EXIT): {}", stat(&mut enter_to_stamp));
        println!(
            "SPLAT per seed: {:.2} cats mown, {:.2} kill runs, {:.2} decals / 120 s; {:.2} decals in the first 60 s; {} mown cats never stamped, {} hides with no kill run within 5 s",
            hides_total as f64 / n,
            runs_total as f64 / n,
            stamps_total as f64 / n,
            d60_total as f64 / n,
            unstamped,
            unpaired
        );
    }

    /// RATCHET — the splat lands on the kill run's EXIT, with the mower
    /// parked, and every mown cat gets one.
    ///
    /// `fn08` @36E2 only SetPos-es in the drive states (1/4/5/11), dequeues
    /// a cat's kill message once the current run is done, lets the ENTER
    /// paint the kill run's first frame, and stamps the decal on the EXIT
    /// the update after the last frame; `fn10` sends run 4 on to 6/9, whose
    /// EXIT stamps. Before the fix the port started the kill in the cat's
    /// own pump (skipping frame 399/413), kept driving 10 px/tick through
    /// it, stamped on the last frame's tick (hide->stamp 316 ms vs the
    /// capture's ~550 ms, ENTER->stamp 316 ms vs 400 ms), let a second hit
    /// pre-empt a running kill, and a run-4 kill left no splat at all.
    #[test]
    fn kill_run_parks_the_mower_and_stamps_on_exit() {
        for seed in [7919u64, 15838, 47514, 95028] {
            let Some((mut m, mut ctx)) = boris() else { return };
            ctx.rng = RandomLong::new(seed);
            ctx.rng15 = Random15::new(seed as u32);
            m.set_control(0, 100);
            m.set_control(1, 5);
            m.set_control(2, 3);
            m.set_control(3, 0);
            let mut pace = Pacer::new(&m);
            let mut clock = m.clock;
            let (mut prev_state, mut prev_d6, mut prev_x) = (m.mower.state, m.mower.e.d6, m.mower.e.x);
            let mut prev_last = m.mower.last;
            let (mut mown, mut stamped, mut kill_ticks, mut exits) = (0usize, 0usize, 0usize, 0usize);
            // A drag-under run 4 is a DRIVE state: fn08's off-screen test
            // sends it to 10 (fn107/fn108 revive the cat) when it drives off
            // the field before its run ends, and fn10's 4 -> 6/9 never comes.
            let mut lost4 = 0usize;
            let mut prev_sq: Vec<i32> = m.cats.iter().map(|c| c.squash as i32).collect();
            while ctx.now_ms < 120_000 {
                pace.advance(&mut ctx);
                let nd = m.decals.len();
                m.tick(&mut ctx);
                ctx.sounds.clear();
                if m.clock == clock {
                    continue;
                }
                clock = m.clock;
                let t = ctx.now_ms;
                let st = m.mower.state;
                let was_kill = matches!(prev_state, 2 | 3 | 6 | 7 | 8 | 9);
                if m.decals.len() > nd {
                    stamped += m.decals.len() - nd;
                    assert!(
                        was_kill && prev_d6 == prev_last,
                        "t={t}: decal stamped outside a kill EXIT (prev state {prev_state}, d6 {prev_d6}/{prev_last})"
                    );
                }
                // A kill run EXITs on a state change, or on a same-state
                // re-entry (a queued kill dequeued at the end of a kill).
                if was_kill && prev_d6 == prev_last && (st != prev_state || m.mower.e.d6 == m.mower.first) {
                    exits += 1;
                }
                if was_kill && st == prev_state && m.mower.e.d6 != m.mower.first {
                    kill_ticks += 1;
                    assert_eq!(m.mower.e.x, prev_x, "t={t}: mower drove during kill run {st}");
                }
                if matches!(st, 2 | 3 | 6 | 7 | 8 | 9) && (st != prev_state || m.mower.e.d6 != prev_d6 + 1) && prev_d6 != m.mower.e.d6 {
                    assert_eq!(m.mower.e.d6, m.mower.first, "t={t}: kill run {st} entered without painting its first frame");
                }
                for (i, c) in m.cats.iter().enumerate() {
                    if prev_sq[i] >= 3 && c.squash == 0 && c.state == 10 && !m.mower.revenge {
                        mown += 1;
                    }
                    prev_sq[i] = c.squash as i32;
                }
                if prev_state == 4 && st == 10 {
                    lost4 += 1;
                }
                prev_state = st;
                prev_d6 = m.mower.e.d6;
                prev_x = m.mower.e.x;
                prev_last = m.mower.last;
            }
            assert!(kill_ticks > 0, "seed {seed}: no kill run in 120 s");
            assert_eq!(stamped, exits, "seed {seed}: {exits} kill-run EXITs but {stamped} splats");
            // Every mown cat is splatted except where fn08 dequeues a second
            // kill in place of fn10's 4 -> 6/9 hand-off (one splat for two
            // cats, as the C does), a run 4 drove off the field first
            // (`lost4`), or the kill is still in flight.
            let in_flight = m.mower.kill_msgs.len()
                + usize::from(matches!(m.mower.state, 4 | 6 | 7 | 8 | 9));
            let stamped_or_lost = stamped + lost4;
            assert!(
                stamped_or_lost + in_flight + 1 >= mown && stamped_or_lost * 5 >= mown * 4,
                "seed {seed}: {mown} cats mown but only {stamped} splats stamped"
            );
        }
    }

    #[test]
    fn parked_mower_does_not_squash_cats() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        // The song wait (g07FE, 16 s): the first update-10 pick already put
        // the mower in state 5, parked just off the right edge by fn15; a
        // drive update neither moves it nor tests the edge until the wait
        // ends. A visible cat held on its blade is never hit — fn12 returns
        // nothing for an off-screen mower, so fn93 never fires.
        m.set_control(3, 1);
        let mut parked_x = None;
        while ctx.now_ms < 15_500 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if m.mower.state == 10 {
                continue;
            }
            assert_eq!(m.mower.state, 5, "the song-wait pass is a normal pass");
            let x = *parked_x.get_or_insert(m.mower.e.x);
            assert_eq!(m.mower.e.x, x, "the mower moved during the song wait");
            let r = m.mower_frame_rect().unwrap();
            assert!(r[0] >= SCREEN_W, "the parked mower is on screen: {r:?}");
            if let Some(b) = m.blade_rect() {
                let (first, flip) = (m.cats[0].first, m.cats[0].flip);
                let mut e = m.cats[0].e;
                m.set_pos_centre(&mut e, first, flip, (b[0] + b[2]) / 2, (b[1] + b[3]) / 2);
                m.cats[0].e = e;
                m.cats[0].visible = true;
            }
            assert_eq!(m.cats[0].squash, 0, "parked mower squashed a cat");
        }
        assert!(parked_x.is_some(), "the mower never left state 10");
    }

    /// Diagnostic: per-tick mower hit-rect vs cat positions after the Dawn Cue
    /// wait. Flags every tick where a cat's visual centre sits inside the
    /// mower's DRAWN rect but fn93 does not register a hit.
    #[ignore]
    #[test]
    fn trace_mower_hit_rects() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        let mut misses = 0usize;
        let mut hits = 0usize;
        let mut lines = 0usize;
        let mut last_clock = 0u64;
        loop {
            pace.advance(&mut ctx);
            let pre: Vec<u16> = m.cats.iter().map(|c| c.squash).collect();
            m.tick(&mut ctx);
            if ctx.now_ms > 300_000 { break; }
            if ctx.now_ms < 15_500 { continue; }
            if m.clock == last_clock { continue; }
            last_clock = m.clock;
            // drawn rect of the mower
            let drawn = m.mower.last_drawn.as_ref().map(|sd| {
                let g = m.geom.get(&(m.mower.e.d6 as u32));
                let (w,h) = g.map(|g|(g.w,g.h)).unwrap_or((0,0));
                [sd.x, sd.y, sd.x + w, sd.y + h]
            });
            let hit = m.fn12_mower_rect();
            let mut flag = String::new();
            for (i, c) in m.cats.iter().enumerate() {
                let (px, py) = visual_centre(&m.geom, &c.e, c.first, c.flip);
                if c.squash != 0 && pre[i] == 0 { hits += 1; flag.push_str(&format!(" HIT c{i}")); }
                if let Some(d) = drawn {
                    let inside = px >= d[0] && px <= d[2] && py >= d[1] && py <= d[3];
                    let in_hit = hit.map(|r| px>=r[0]&&px<=r[2]&&py>=r[1]&&py<=r[3]).unwrap_or(false);
                    if inside && !in_hit && c.squash == 0 && c.visible {
                        misses += 1;
                        flag.push_str(&format!(" MISS c{i}@({px},{py})"));
                    }
                }
            }
            let onscreen = hit.is_some();
            if !flag.is_empty() || (onscreen && lines < 40) {
                println!("t={:6} clk={} mower st={} act={} d6={} pos=({},{}) drawn={:?} hit={:?} | cats {:?} |{}",
                    ctx.now_ms, m.clock, m.mower.state, m.mower.active, m.mower.e.d6,
                    m.mower.e.x, m.mower.e.y, drawn, hit,
                    m.cats.iter().map(|c| {
                        let (px,py)=visual_centre(&m.geom,&c.e,c.first,c.flip);
                        (c.state, px, py, c.squash, c.visible)
                    }).collect::<Vec<_>>(), flag);
                lines += 1;
            }
            if lines > 400 { break; }
        }
        println!("== hits={hits} misses={misses}");
    }

    /// Regression for the fn12 @3FEC rect. A driving, visible mower must
    /// squash a cat standing on its blade, and the blade rect must lie inside
    /// the frame the mower is actually drawn at. Before the fix fn12 built a
    /// full-frame box CENTRED on `mower.e.x/y` — but that is the drawn
    /// top-left, so the kill rect sat 85 px left and 123 px above the blade
    /// and cats walked over the mower untouched.
    #[test]
    fn a_driving_mower_squashes_a_cat_in_its_path() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        m.mower.state = 5;
        m.mower.first = 369;
        m.mower.last = 371;
        m.mower.e.d6 = 369;
        m.mower.e.x = 260;
        m.mower.e.y = 200;
        m.mower.active = true;
        m.mower.wait_until = 0;

        let frame = m.mower_frame_rect().expect("mower frame rect");
        let hit = m.fn12_mower_rect().expect("a visible driving mower has a hit rect");
        assert!(
            hit[0] >= frame[0] && hit[1] >= frame[1] && hit[2] <= frame[2] && hit[3] <= frame[3],
            "fn12 rect {hit:?} must sit inside the drawn mower frame {frame:?}"
        );
        assert_eq!(hit, m.blade_rect().unwrap(), "fn12 is channel 2, the blade");

        // Park the cat's VISUAL CENTRE on the middle of the blade.
        let (tx, ty) = ((hit[0] + hit[2]) / 2, (hit[1] + hit[3]) / 2);
        let cat = &mut m.cats[0];
        cat.visible = true;
        cat.squash = 0;
        cat.seq_done = false;
        let (px, py) = visual_centre(&m.geom, &m.cats[0].e, m.cats[0].first, m.cats[0].flip);
        m.cats[0].e.x += tx - px;
        m.cats[0].e.y += ty - py;

        pace.advance(&mut ctx);
        m.fn93(0, &mut ctx);
        assert_eq!(m.cats[0].squash, 1, "the blade must squash a cat standing on it");
    }

    /// The mower's blade rect tracks the drawn frame in both directions.
    #[test]
    fn the_blade_rect_follows_the_mower_it_is_drawn_as() {
        let Some((mut m, _)) = boris() else { return };
        m.mower.state = 5;
        m.mower.first = 369;
        m.mower.last = 371;
        m.mower.active = true;
        for &flip in &[false, true] {
            m.mower.flip = flip;
            for &d6 in &[369, 370, 371] {
                m.mower.e.d6 = d6;
                for &(x, y) in &[(100, 60), (300, 200), (480, 330)] {
                    m.mower.e.x = x;
                    m.mower.e.y = y;
                    let f = m.mower_frame_rect().unwrap();
                    let b = m.blade_rect().unwrap();
                    assert!(
                        b[0] >= f[0] && b[1] >= f[1] && b[2] <= f[2] && b[3] <= f[3],
                        "flip={flip} d6={d6} pos=({x},{y}): blade {b:?} outside frame {f:?}"
                    );
                    assert!(b[2] > b[0] && b[3] > b[1], "blade rect is non-empty");
                }
            }
        }
    }

    // ---------------------------------------------------------------- ratchets
    //
    // The three below fail on the pre-fix file and pass now. They are the
    // motion gate (docs/motion-lint.md) expressed against the module's own
    // sprite list, so they run in `cargo test` instead of needing a capture.

    /// Drive the module on the Pacer at the capture's panel settings and
    /// return, per module update, the sprite list. Controls: Manic /
    /// 3 cats / Revenge Frequent / Mower Sound off.
    fn filmstrip(secs: u64) -> Option<Vec<Vec<SpriteDraw>>> {
        let (mut m, mut ctx) = boris()?;
        m.set_control(0, 100);
        m.set_control(1, 2);
        m.set_control(2, 3);
        m.set_control(3, 0);
        let mut pace = Pacer::new(&m);
        let mut frames = Vec::new();
        let mut last_clock = m.clock;
        while ctx.now_ms < secs * 1000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.clock != last_clock {
                last_clock = m.clock;
                let mut out = Vec::new();
                m.sprites(&mut out);
                frames.push(out);
            }
        }
        Some(frames)
    }

    fn compound_no(png: &str) -> u32 {
        png.rsplit('_')
            .next()
            .and_then(|t| t.split('.').next())
            .and_then(|t| t.parse().ok())
            .unwrap_or(0)
    }

    /// RATCHET 1 — the mown cats stay on the lawn.
    ///
    /// `fn08` @36E2 stamps one of arts 407/422/532/545 on the exit of every
    /// kill run, and the stamp is permanent (see `fn16_stamp`). Before the
    /// fix nothing ever pushed to `decals`, so this counted 0 against any
    /// number of kills. QEMU capture: one splat born at t=27.3 s, still
    /// there at t=59.9 s.
    #[test]
    fn every_kill_leaves_a_permanent_ground_decal() {
        let Some((mut m, mut ctx)) = boris() else { return };
        m.set_control(0, 100);
        m.set_control(1, 5); // six cats, so a two-minute run gets kills
        m.set_control(2, 3);
        m.set_control(3, 0);
        let mut pace = Pacer::new(&m);
        let mut kills = 0usize;
        let mut was_kill = false;
        let mut seen: Vec<(i32, i32, u32)> = Vec::new();
        while ctx.now_ms < 180_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            let kill = matches!(m.mower.state, 2 | 3 | 6 | 7 | 8 | 9);
            if kill && !was_kill {
                kills += 1;
            }
            was_kill = kill;
            // Nothing already stamped may ever move or disappear.
            for (i, d) in m.decals.iter().enumerate() {
                match seen.get(i) {
                    Some(&(x, y, f)) => assert_eq!(
                        (x, y, f),
                        (d.x, d.y, d.fno),
                        "a ground decal moved or changed art at t={}",
                        ctx.now_ms
                    ),
                    None => seen.push((d.x, d.y, d.fno)),
                }
            }
            assert!(
                m.decals.len() >= seen.len(),
                "a ground decal was removed at t={}",
                ctx.now_ms
            );
        }
        assert!(kills > 0, "the mower never played a kill run in 180 s");
        assert!(
            m.decals.len() >= kills,
            "{kills} kill runs left only {} decals",
            m.decals.len()
        );
        for d in &m.decals {
            assert!(
                matches!(d.fno, 407 | 422 | 532 | 545),
                "decal art {} is not one of fn08's four literals",
                d.fno
            );
        }
    }

    /// RATCHET 2 — a cat never draws the same (art, x, y) on two consecutive
    /// module updates.
    ///
    /// Two separate bugs put the identical frame on screen twice in a row:
    /// the run tables held pack block firsts, so every loop drew its own
    /// link marker on top of the frame it had just drawn; and the port
    /// treated the ENTER as non-drawing, so every `fn96` re-pool ate a whole
    /// 100 ms pump. Measured on the capture at 20 fps: a walking cat holds
    /// its position for >= 4 samples in 8 of 634 ticks (1.3 %). The pre-fix
    /// port did it on 26 % of walking ticks.
    #[test]
    fn a_walking_cat_never_draws_the_same_frame_twice_running() {
        let Some(frames) = filmstrip(60) else { return };
        let mut held = 0usize;
        let mut moving = 0usize;
        for w in frames.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            for sb in b.iter().filter(|s| (30..=318).contains(&compound_no(&s.png))) {
                // Match the same cat across the pair by nearest draw.
                let Some(sa) = a
                    .iter()
                    .filter(|s| (30..=318).contains(&compound_no(&s.png)))
                    .min_by_key(|s| (s.x - sb.x).abs() + (s.y - sb.y).abs())
                else {
                    continue;
                };
                if (sa.x - sb.x).abs() + (sa.y - sb.y).abs() > 40 {
                    continue; // not the same cat
                }
                moving += 1;
                if sa.png == sb.png && sa.x == sb.x && sa.y == sb.y && sa.flip == sb.flip {
                    held += 1;
                }
            }
        }
        assert!(moving > 500, "not enough cat draws to judge ({moving})");
        let pct = 100.0 * held as f64 / moving as f64;
        assert!(
            pct < 12.0,
            "cats redrew an identical frame on {pct:.1} % of updates \
             (capture: 1.3 % of walking ticks; pre-fix port: 26 %)"
        );
    }

    /// RATCHET 3 — the walk cycle is the authored stride over the authored
    /// number of frames, and the marker is not one of them.
    ///
    /// Measured off `emu/captures/qemu/mowin-boris.mp4` at 20 fps: a cat on
    /// the horizontal walk covers 47 px in FOUR module ticks (t=4.45..5.70 s,
    /// eleven consecutive ticks, no hold). 47 px is `topleft(108) −
    /// topleft(104)`, i.e. the offset of the run's last frame from its link
    /// marker — so run 105 is four frames, 105..108, and 104 is the marker.
    #[test]
    fn a_walk_run_covers_its_authored_stride_in_its_authored_frames() {
        let Some((m, _)) = boris() else { return };
        for (state, run) in [(32u16, 105i32), (39, 60), (25, 80)] {
            let (_, fno, flip) = BORIS_STATES
                .iter()
                .find(|s| s.0 == state)
                .copied()
                .expect("state in the table");
            assert_eq!(fno as i32, run, "state {state} run id");
            let marker = marker_of(&m.geom, run);
            assert_eq!(marker, run - 1, "run {run} must have a link marker below it");
            let last = m.run_last(run);
            let mut e = Ent { x: 0, y: 0, d6: run };
            let mut drawn = Vec::new();
            for _ in 0..(last - run + 2) {
                let (ox, oy) = frame_offset(&m.geom, run, e.d6, flip);
                drawn.push((e.d6, e.x + ox, e.y + oy));
                if adv_loop(&m.geom, &mut e, run, last, flip) {
                    break;
                }
            }
            let frames = (last - run + 1) as usize;
            assert_eq!(drawn.len(), frames, "run {run} draws its own frames only");
            assert!(
                drawn.iter().all(|f| f.0 != marker),
                "run {run} drew its link marker {marker}"
            );
            // One full cycle later the first frame has advanced by exactly
            // the marker-to-last offset.
            let stride = frame_offset(&m.geom, run, last, flip);
            let (ox, oy) = frame_offset(&m.geom, run, e.d6, flip);
            assert_eq!(
                (e.x + ox - drawn[0].1, e.y + oy - drawn[0].2),
                stride,
                "run {run} cycle stride"
            );
            // and no two consecutive draws land in the same place
            for pair in drawn.windows(2) {
                assert_ne!(
                    (pair[0].1, pair[0].2),
                    (pair[1].1, pair[1].2),
                    "run {run} drew two frames at the same place"
                );
            }
        }
    }

    /// RATCHET 4 — facing. The motion linter reports boris "23.5 % backwards"
    /// against ONE global hypothesis, but bank 1000 holds two opposite art
    /// conventions: the cats' and the flutterby's unflipped art faces RIGHT
    /// (run 105 strides +47 px unflipped, and `g0150` gives state 32 — which
    /// `fn101` enters from the left edge — flip = 0), while the mower's
    /// unflipped art faces LEFT (`fn17` @43F8 toggles the flip bit until it
    /// equals `0 < +0x12A`, the sign of the drive step). Scored per actor,
    /// every class agrees with its travel; scored globally the mower's ~16 %
    /// share of advances reads as the whole module's "backwards" figure.
    #[test]
    fn each_actor_faces_the_way_it_travels() {
        let Some(frames) = filmstrip(60) else { return };
        // (unflipped art faces right?, low, high)
        let classes: [(&str, bool, u32, u32); 3] =
            [("cat", true, 30, 318), ("flutterby", true, 319, 363), ("mower", false, 369, 405)];
        for (name, faces_right, lo, hi) in classes {
            let (mut agree, mut total) = (0usize, 0usize);
            for w in frames.windows(2) {
                let pick = |v: &Vec<SpriteDraw>| -> Vec<SpriteDraw> {
                    v.iter()
                        .filter(|s| (lo..=hi).contains(&compound_no(&s.png)))
                        .cloned()
                        .collect()
                };
                let (a, b) = (pick(&w[0]), pick(&w[1]));
                for sb in &b {
                    let Some(sa) = a
                        .iter()
                        .min_by_key(|s| (s.x - sb.x).abs() + (s.y - sb.y).abs())
                    else {
                        continue;
                    };
                    let (dx, dy) = (sb.x - sa.x, sb.y - sa.y);
                    if dy.abs() > 40 {
                        continue; // not the same actor
                    }
                    // Only score advances whose facing is visible: the
                    // flutterby's vertical runs (320 up, 360 down) wobble a
                    // couple of px sideways per wingbeat and carry no
                    // horizontal sense at all.
                    if dx.abs() < 4 || dx.abs() <= dy.abs() {
                        continue;
                    }
                    total += 1;
                    // The drawn art points right when `flip` matches the
                    // class's convention.
                    let points_right = if faces_right { !sb.flip } else { sb.flip };
                    if points_right == (dx > 0) {
                        agree += 1;
                    }
                }
            }
            if total < 50 {
                continue; // the flutterby may not show up in a short run
            }
            let pct = 100.0 * agree as f64 / total as f64;
            assert!(
                pct > 92.0,
                "{name}: only {pct:.1} % of {total} advances face the way they travel"
            );
        }
    }


    /// Run the module at the capture's panel (Manic, 3 cats, Revenge
    /// Frequent, sound off) and hand every completed 100 ms update to `f`.
    fn run_capture_panel(seed: u64, secs: u64, mut f: impl FnMut(&MowinBoris, u64, &[Decal])) {
        let Some((mut m, mut ctx)) = boris() else { return };
        ctx.rng = RandomLong::new(seed);
        ctx.rng15 = Random15::new(seed as u32);
        m.set_control(0, 100);
        m.set_control(1, 2);
        m.set_control(2, 3);
        m.set_control(3, 0);
        let mut pace = Pacer::new(&m);
        let mut clock = m.clock;
        while ctx.now_ms < secs * 1000 {
            pace.advance(&mut ctx);
            let nd = m.decals.len();
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.clock == clock {
                continue;
            }
            clock = m.clock;
            let fresh: Vec<Decal> = m.decals[nd..]
                .iter()
                .map(|d| Decal { x: d.x, y: d.y, fno: d.fno })
                .collect();
            f(&m, ctx.now_ms, &fresh);
        }
    }

    /// RATCHET — the drag (mower +0x124 / +0x12E, `fn10` @3D9E + `fn08`
    /// @36E2). Capture (mowin-boris.mp4, 34-41 s): a kill at 34.32 s parks
    /// the mower, which then drives ON at 10 px/tick leaving a flattened cat
    /// lying where it was hit; ~3.1 s later it stops, backs up over the cat
    /// at 20 px/tick, chops it (Cat Chop 40.20 s) and resumes the original
    /// direction at 10 px/tick. In the C: the kill run's end rolls the drag
    /// (the EXIT stamps 545, blood only), the cat lies in drag states 5/8 on
    /// the blade's spot, a drive update past the 3000 + rand%2000 ms timer
    /// enters 12, 11 moves −2·step, the hit becomes 7/8 (EXIT stamps 422),
    /// then 13 → 5. Before: +0x124 was never raised — no 545, no 12/11/13,
    /// no 7/8, no 422 at all.
    #[test]
    fn the_drag_flattens_a_cat_and_the_mower_backs_over_it() {
        let mut complete = 0;
        for k in 0..16u64 {
            let seed = 2 * k * 7919 + 1;
            // (545 time, drive dir before, entered 12 at, reversed, chopped)
            let mut drag: Option<(u64, i32, Option<u64>, bool, bool)> = None;
            let mut prev_x = 0;
            let mut prev_state = 0;
            let mut laid = false;
            run_capture_panel(seed, 120, |m, t, fresh| {
                let st = m.mower.state;
                let dx = m.mower.e.x - prev_x;
                for d in fresh {
                    if d.fno == 545 {
                        assert!(drag.is_none(), "t={t}: a second drag before the first ended");
                        assert_eq!(st, 5, "t={t}: fn10 answers 5 with the drag");
                        let flat = m.cats.iter().filter(|c| c.life == 3).count();
                        assert_eq!(flat, 1, "t={t}: fn10 flattens exactly one dead-pending cat");
                        drag = Some((t, m.mower.dir, None, false, false));
                        laid = false;
                    }
                    if d.fno == 422 {
                        let Some(dr) = drag.as_mut() else { panic!("t={t}: 422 without a drag") };
                        assert!(dr.3, "t={t}: 422 before the mower backed up");
                        assert!(laid, "t={t}: the chopped cat never lay flattened in 5/8");
                        dr.4 = true;
                    }
                }
                if let Some(dr) = drag.as_mut() {
                    // The flattened cat lies on the blade's spot (fn101, 5/8).
                    if !laid {
                        if let Some(c) = m.cats.iter().find(|c| c.life == 3 && c.visible) {
                            assert!(matches!(c.state, 5 | 8), "t={t}: flattened cat in state {}", c.state);
                            laid = true;
                        }
                    }
                    if dr.2.is_none() {
                        if st == 12 {
                            let wait = t - dr.0;
                            assert!(
                                (3000..=5200).contains(&wait),
                                "t={t}: drag timer {wait} ms (fn10: 3000 + rand % 2000; capture ~3100)"
                            );
                            dr.2 = Some(t);
                        } else if st == 5 && prev_state == 5 && dx != 0 {
                            assert_eq!(dx, dr.1 * 10, "t={t}: the mower drives ON after the first hit");
                        }
                    }
                    if st == 11 && prev_state == 11 && dx != 0 {
                        assert_eq!(dx, -dr.1 * 20, "t={t}: 11 backs up at twice the drive step (capture 20 px/tick)");
                        dr.3 = true;
                    }
                    if dr.4 && st == 5 && prev_state == 5 && dx != 0 {
                        assert_eq!(dx, dr.1 * 10, "t={t}: after 13 the mower resumes its direction");
                        complete += 1;
                        drag = None;
                    }
                }
                prev_x = m.mower.e.x;
                prev_state = st;
            });
        }
        assert!(complete >= 4, "only {complete} complete drags in 16 x 120 s (the C: ~1 per seed)");
    }

    /// RATCHET — the row turn (`fn08` drive branch + update 10 + `fn14`
    /// @41D4). Capture (mowin-boris.mp4 20.3-21.4 s and 28.6-30.1 s): the
    /// mower drives until only a ~5 px sliver is left, is gone for 2-3
    /// frames, and comes straight back in on the next row, 43 px (the blade
    /// height) lower, from the same edge. The C sends it to 10 only once its
    /// frame is fully off screen, the update-10 pick re-enters at once, and
    /// `fn14` steps y by the blade height with x untouched. Before: the port
    /// turned at a fixed ±80 px (29 px of mower popped away on the left) and
    /// stayed hidden ~20 ticks.
    #[test]
    fn the_mower_turns_only_once_fully_off_screen() {
        let mut turns = 0;
        for k in 0..8u64 {
            let seed = 2 * k * 7919 + 1;
            let (mut last_w, mut hidden, mut was_vis) = (0, 0, false);
            let (mut vanish_w, mut turned) = (0, false);
            let mut last_y = None;
            run_capture_panel(seed, 60, |m, t, _| {
                let mut spr = Vec::new();
                m.sprites(&mut spr);
                let drawn = spr.iter().any(|s| (369..=540).contains(&compound_no(&s.png)));
                let w = if drawn {
                    m.mower_frame_rect().map(|r| (r[2].min(SCREEN_W) - r[0].max(0)).max(0)).unwrap_or(0)
                } else {
                    0
                };
                // Only disappearances that end in state 10 are turns; a drag
                // may carry the mower off screen and back in reverse.
                if w == 0 && m.mower.state == 10 && !turned {
                    turned = true;
                    if t > 17_000 {
                        assert!(vanish_w <= 10, "t={t}: {vanish_w} px of mower vanished at once (capture: a ~5 px sliver)");
                    }
                }
                if w > 0 {
                    if !was_vis && turned && t > 17_000 {
                        assert!(hidden <= 3, "t={t}: mower hidden {hidden} ticks between passes (capture 2-3)");
                        if let Some(y) = last_y {
                            let dy = m.mower.e.y - y;
                            assert!(dy == 43 || dy < 0, "t={t}: next row {dy} px down (capture pitch 43)");
                        }
                        turns += 1;
                    }
                    was_vis = true;
                    turned = false;
                    hidden = 0;
                    last_w = w;
                    last_y = Some(m.mower.e.y);
                } else {
                    if was_vis {
                        vanish_w = last_w;
                    }
                    was_vis = false;
                    hidden += 1;
                }
            });
        }
        assert!(turns >= 16, "only {turns} turns");
    }

    /// RATCHET (C-pinned; no capture shows revenge — the 60 s capture ends
    /// before the first roll can happen, see below). `fn08`'s update 10:
    /// the first pass never rolls (+0x120); after 60 s of normal phase a
    /// pass rolls `Random15() % 16 < g0B28` and a revenge pass is a DRIVE in
    /// state 1 with a cat at the wheel (runs 516..518), after a fresh 16 s
    /// song wait, parked while any cat is on screen (`fn106`); cats in state
    /// 10 stay hidden for its duration (`fn90` update 10); 2/3 happen only
    /// when that mower actually hits a cat. Both toggles restart `cmid` 40
    /// (L134 `fn0C98`). Before: a "pounce" roll jumped
    /// from 10 straight into the head-chop run 2/3 with nobody hit.
    #[test]
    fn revenge_is_a_cat_driven_pass_after_60_s_of_normal_mowing() {
        let mut revenge_passes = 0;
        for k in 0..8u64 {
            let seed = 2 * k * 7919 + 1;
            let mut prev_state = 10;
            let mut in_revenge = false;
            let mut rev_start = 0;
            let mut rev_x = 0;
            let mut hidden_at_start: Vec<bool> = Vec::new();
            let mut mown_in_revenge = 0;
            let mut prev_sq: Vec<u16> = Vec::new();
            let mut plays = 1;
            run_capture_panel(seed, 180, |m, t, _| {
                let st = m.mower.state;
                if prev_sq.is_empty() {
                    prev_sq = m.cats.iter().map(|c| c.squash).collect();
                }
                if st != prev_state {
                    assert!(
                        !(prev_state == 10 && matches!(st, 2 | 3)),
                        "t={t}: 10 -> {st}: revenge jumped straight into a head chop"
                    );
                    if prev_state == 10 && st == 1 {
                        assert!(t > 60_000, "t={t}: revenge pass inside the first 60 s");
                        let now_plays = m.song_plays;
                        assert_eq!(now_plays, plays + 1, "t={t}: the revenge toggle restarts cmid 40");
                        plays = now_plays;
                        assert!((516..=518).contains(&m.mower.e.d6), "t={t}: revenge drive not on run 517");
                        in_revenge = true;
                        rev_start = t;
                        rev_x = m.mower.e.x;
                        revenge_passes += 1;
                        hidden_at_start = m.cats.iter().map(|c| c.state == 10).collect();
                    }
                    if prev_state == 10 && st == 5 {
                        if in_revenge {
                            assert_eq!(m.song_plays, plays + 1, "t={t}: leaving revenge restarts cmid 40");
                            plays = m.song_plays;
                        }
                        in_revenge = false;
                    }
                    if matches!(st, 2 | 3) {
                        assert!(in_revenge && mown_in_revenge > 0, "t={t}: head chop {st} with nobody hit");
                    }
                }
                if in_revenge {
                    // The song wait: parked, not moving, for 16 s.
                    if t < rev_start + 15_900 {
                        assert_eq!(m.mower.e.x, rev_x, "t={t}: the revenge drive moved inside its song wait");
                    }
                    for (i, c) in m.cats.iter().enumerate() {
                        if hidden_at_start.get(i) == Some(&true) {
                            assert_eq!(c.state, 10, "t={t}: cat {i} came out of 10 during revenge");
                        }
                        if prev_sq[i] >= 3 && c.squash == 0 && c.state == 10 {
                            mown_in_revenge += 1;
                        }
                    }
                }
                for (i, c) in m.cats.iter().enumerate() {
                    prev_sq[i] = c.squash;
                }
                prev_state = st;
            });
        }
        assert!(revenge_passes >= 2, "only {revenge_passes} revenge passes in 8 x 180 s at Frequent");
    }

    /// RATCHET — the lawn planter (`fn63` @31C6 / `fn65` @32B6, pumped 11
    /// times per tick by `fn22` @5056): one sprite that toggles Show/Hide,
    /// stamping a plant on every other call — 6, 5, 6, 5 … per 100 ms tick
    /// — and drawing flowers only from `g06BE` = [10, 12, 14, 16, 18, 20].
    /// Capture (mowin-boris.mp4, 4x4-cell lawn occupancy of the band
    /// y 330-478 the mower has not reached): +0.41 %/s from t = 5 to 59 s;
    /// before, the port planted 5 per tick with 5 of 11 flower ids missing
    /// from the pack and grew +0.36 %/s; now +0.41 %/s.
    #[test]
    fn the_planter_stamps_every_other_call_eleven_calls_a_tick() {
        let Some((mut m, mut ctx)) = boris() else { return };
        let mut pace = Pacer::new(&m);
        let mut clock = m.clock;
        let mut per_tick: Vec<usize> = Vec::new();
        let mut planted = m.tufts.len() + m.flowers.len();
        // The song wait: the mower is parked off screen, nothing scrubs.
        while ctx.now_ms < 15_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if m.clock == clock {
                continue;
            }
            clock = m.clock;
            let n = m.tufts.len() + m.flowers.len();
            per_tick.push(n - planted);
            planted = n;
        }
        assert!(per_tick.len() > 100);
        for w in per_tick.windows(2) {
            assert_eq!(w[0] + w[1], 11, "two ticks plant 11 (fn22's 11 fn63 calls, fn65 toggling): {w:?}");
            assert!(w[0] == 5 || w[0] == 6, "{w:?}");
        }
        for f in &m.flowers {
            assert!(PLANTER_FLOWERS.contains(&f.fno), "flower {} is not a g06BE run", f.fno);
            assert!(m.pack.frame(BASE, f.fno).is_some(), "flower {} is not in the pack", f.fno);
        }
        assert!(!m.flowers.is_empty(), "no flower in 15 s (4/256 of ~800 plants)");
    }

    /// Diagnostic for the drag / reverse-over / row-turn beats at the
    /// capture's panel (Manic, 3 cats, Revenge Frequent, sound off), 16 odd
    /// seeds x 120 s. Capture (mowin-boris.mp4): kill 34.32 s parks at deck
    /// x~267, drives on at 10 px/tick, reverses at 20 px/tick from ~38.3 s
    /// (EXIT->ENTER 12 ~3.1 s), chops the flattened cat at 40.20 s, resumes
    /// at 10 px/tick; a row turn hides the mower for 2 ticks after its last
    /// sliver leaves the edge.
    #[ignore]
    #[test]
    fn trace_drag_and_turns() {
        let mut arts: HashMap<u32, usize> = HashMap::new();
        let (mut enter12, mut rev_steps, mut fwd_steps) = (0usize, Vec::new(), Vec::new());
        let mut exit_to_12: Vec<u64> = Vec::new();
        let mut gaps: Vec<u64> = Vec::new();
        let mut pops: Vec<i32> = Vec::new();
        let mut rows: Vec<i32> = Vec::new();
        let (mut chops60, mut purrs60) = (0usize, 0usize);
        for k in 0..16u64 {
            let seed = 2 * k * 7919 + 1;
            let Some((mut m, mut ctx)) = boris() else { return };
            ctx.rng = RandomLong::new(seed);
            ctx.rng15 = Random15::new(seed as u32);
            m.set_control(0, 100);
            m.set_control(1, 2);
            m.set_control(2, 3);
            m.set_control(3, 0);
            let mut pace = Pacer::new(&m);
            let mut clock = m.clock;
            let mut prev_state = m.mower.state;
            let mut prev_x = m.mower.e.x;
            let mut last_545: Option<u64> = None;
            let mut last_vis_w = 0;
            let mut hidden = 0u64;
            let mut was_vis = false;
            while ctx.now_ms < 120_000 {
                pace.advance(&mut ctx);
                let nd = m.decals.len();
                m.tick(&mut ctx);
                if ctx.now_ms <= 60_000 {
                    chops60 += ctx.sounds.iter().filter(|&&s| s == SND_CAT_CHOP).count();
                    purrs60 += ctx.sounds.iter().filter(|&&s| s == SND_PURR).count();
                }
                ctx.sounds.clear();
                if m.clock == clock {
                    continue;
                }
                clock = m.clock;
                let t = ctx.now_ms;
                for d in &m.decals[nd..] {
                    *arts.entry(d.fno).or_default() += 1;
                    if d.fno == 545 {
                        last_545 = Some(t);
                    }
                }
                let st = m.mower.state;
                if st == 12 && prev_state != 12 {
                    enter12 += 1;
                    if let Some(t5) = last_545.take() {
                        exit_to_12.push(t - t5);
                    }
                }
                if st == 11 && prev_state == 11 && m.mower.e.x != prev_x {
                    rev_steps.push((m.mower.e.x - prev_x).abs());
                }
                if st == 5 && prev_state == 5 && m.mower.e.x != prev_x {
                    fwd_steps.push((m.mower.e.x - prev_x).abs());
                }
                let mut spr = Vec::new();
                m.sprites(&mut spr);
                let drawn = spr.iter().any(|s| (369..=540).contains(&compound_no(&s.png)));
                let vis_w = if drawn {
                    m.mower_frame_rect()
                        .map(|r| (r[2].min(SCREEN_W) - r[0].max(0)).max(0))
                        .unwrap_or(0)
                } else {
                    0
                };
                if vis_w > 0 {
                    if !was_vis && t > 20_000 {
                        gaps.push(hidden);
                        rows.push(m.mower.e.y);
                    }
                    was_vis = true;
                    hidden = 0;
                    last_vis_w = vis_w;
                } else {
                    if was_vis && t > 20_000 {
                        pops.push(last_vis_w);
                    }
                    was_vis = false;
                    hidden += 1;
                }
                prev_state = st;
                prev_x = m.mower.e.x;
            }
        }
        fn stat<T: Ord + Copy + std::fmt::Display>(v: &mut [T]) -> String {
            v.sort();
            if v.is_empty() {
                return "n=0".into();
            }
            format!("n={} min={} med={} max={}", v.len(), v[0], v[v.len() / 2], v[v.len() - 1])
        }
        let mut arts: Vec<_> = arts.into_iter().collect();
        arts.sort();
        println!("DRAG decals by art (16 seeds x 120 s): {arts:?}");
        println!("DRAG ENTER 12: {enter12}; 545 stamp -> ENTER 12 ms: {}", stat(&mut exit_to_12));
        println!("DRAG reverse step px/tick (state 11): {}", stat(&mut rev_steps));
        println!("DRAG forward step px/tick (state 5): {}", stat(&mut fwd_steps));
        println!("TURN hidden ticks between passes: {}", stat(&mut gaps));
        println!("TURN last visible width before vanishing (px): {}", stat(&mut pops));
        rows.sort();
        rows.dedup();
        println!("TURN row anchors seen: {rows:?}");
        println!(
            "CUES first 60 s per seed: Cat Chop {:.2}, Purr {:.2} (capture: 4 Cat Chop, 11 Purr)",
            chops60 as f64 / 16.0,
            purrs60 as f64 / 16.0
        );
    }
}
