//! Voyeur — ported function-by-function from the Ghidra decompile of module
//! CODE 129 "Voyeur" (totally-twisted docs/decompiled/voyeur/M129_M129.c,
//! private) with the Library 4.0 sprite classes read alongside it. Listing
//! addresses cite the M129 listing column (`fnNN @XXXX`).
//!
//! Port-from-decompile pass 2026-09-15 (Fable). The window/controller
//! machine below is the C; the facade/sky/star DRAWING further down is the
//! golden-verified layout work of 2026-09-01..12 and is kept as it was (see
//! "Layout" at the end of this header). Everything the old prose-era header
//! claimed about scheduling — a 60 ms DoDrawFrame, per-window 1/500 rolls,
//! 5 s descriptor cooldowns, the "recycle in place" dead-scene fix, beat
//! lists with 5–13 s gaps between beats, a static `CELL_CLASS` table — was
//! invented or misread and is gone.
//!
//! ## Objects
//! - Controller (`fn21` @4E64 ctor, `fn27` @569E DoDrawFrame): `+0x08` the
//!   30-minute recast deadline (`now + 1800000`), `+0x10` the scene state
//!   object (`fn33` @5864 ctor, `fn39` @6D1A handler). `fn24` @53DA: window
//!   count `g0E4C = 5`, speed `g0E50 = 60`. `fn25` @53F8 creates the five
//!   window sprites (`fn09` @06FC ctor: state 0, index `g0CE8` counting from
//!   1) parked at (−1000,−1000), the shared bank-1000 RLESequence `g0CE4`
//!   (`fn06` @0612) and the 7-channel sound bank `g0CEA` (`fn41`/`fn43`,
//!   snd 1000..1006). Tagline: `DrawText(128 + RandomBelow(20))`.
//! - `fn27`: while `now <= +0x08` and no modifier key toggled (`L130
//!   fn41DC`) run the scene object; else the full recast (`fn26` destroys the
//!   windows, the grids/busy flags/`g0E48` clear, `fn25` again, a new
//!   deadline, `fn35` → state 0).
//! - Scene object `fn39`: state 0 (build: `fn36` @5990 rolls `g0E49`
//!   (anchor), `g0D0E` cols (`RandomBelow(2)+3`, forced 4 when the screen is
//!   wider than 480), `g0D0C` rows (`RandomBelow(3)+3`, `+4` band on tall
//!   screens), paints the facade and the star field, fills the cell-centre
//!   table `g0D10[row*6+col]` bottom row first) → state 2 → state 3, the
//!   main loop, every DoDrawFrame:
//!   1. `RandomBelow(500) == 0` → a random cell; a lit-free cell (`g0DE8 ==
//!      1`) always, a dark one on `RandomBelow(10) == 0`, is toggled by
//!      `fn38` @6B66 (dark → paint an empty lit window, lit → paint the
//!      closed shutter back).
//!   2. If fewer than 5 windows are busy: `RandomBelow((100 − g0E50)*10 +
//!      100) == 0` (= 1/500) → first free window (`g0CEE[1..=5]`), the
//!      weighted chooser over the 26 descriptors at `g04BA` (0x44-byte
//!      records: entry state, rows, cols, weight, placed/pin-x/pin-top
//!      flags, random-flip flag, busy byte, cooldown long, 6×6 footprint —
//!      see `DESCS`), ≤100 retries; state 0x16 refused while `g0E48` is
//!      set; `RandomBelow(20) == 0` in December → descriptor 17 (state
//!      0x13). Placement: a random top-left cell for the footprint, the
//!      pin flags and the per-state overrides (0xd/0xe/3/0x12), `fn37`
//!      @6A80 (every footprint cell must be `g0DE8 == 0`), scanning the
//!      grid on failure; 0x16 also needs the cell above free; 0x17/5/0x18
//!      need `g0E48`. On success the footprint cells become 2 with the
//!      window as owner, `g0CEE[win] = 1`, the descriptor's busy byte and
//!      the window's go flag `g0422[win]` are set with the descriptor and
//!      the (col,row).
//!   3. Draw, run every window (`fn13`), `RandomBelow(5000) == 0` → ambient
//!      snd 5/6, the skyline-window twinkle (GAP: not ported).
//! - Window sprite `fn13` @0836 (states 0..0x1c). The vtable at `g011A`
//!   resolves to L132's run driver: `+0x7C` SetRun(frame) (`fn0204` →
//!   `fn028A`: the LINKED hand-off, see "Hand-offs" below; run first =
//!   frame, current = frame, first-tick hold), `+0x80`
//!   SetRunList(frames.., −1) (`fn0240`: SetRun then queue via `+0x118`),
//!   `+0x84` the tick (`fn0316`: first frame holds one delay, then `+0xd0`
//!   SetFrame advances with the §2 link; at the run's end the repeat count
//!   `+0x4c` re-queues the run, else the queue's next run starts next tick,
//!   else `+0x46` done), `+0x88` SetPos, `+0x104` frame index. A run is the
//!   contiguous frame numbers from its first (`fn1260`). Message 0 = update,
//!   `0x8000|s` = enter s (the negative shorts in the C), `0x4000|s` exit
//!   (nothing). Every update: `if now < +0x174 return; +0x174 = now +
//!   +0x18c; tick; if done → the state's branch`, then the sound tail:
//!   the displayed frame's cue (0x23→2, 0x2c→3, 0x1bd|0x711→1, 0x7d5→4,
//!   {0x66e,0xa13,0xc08,0x7fa,0xa6f}→0, 0xa85..0xa9b→2, 0xbe9→1) through
//!   `fn44` @7B8E, which allows one sound per `g0E40` stamp (the ms clock
//!   `fn27` reads, so one per tick).
//! - State 0 polls its go flag: `g0CEE[idx] = 0` each tick; on go: pos =
//!   the cell centre (+1 x when mirrored), descriptor/cols/rows, flip =
//!   `!g0E49` (then a coin if the descriptor allows; *superseded
//!   2026-09-29*: this line used to say `g0E49` — the listing @0B76..0C16
//!   sets the flip when `g0E49` is 0, and the golden capture's saucer is
//!   mirrored with the building on the left), enter the descriptor's
//!   state. State 0x1c ends a vignette: `g0CEE[idx] = 0`, busy byte clear,
//!   the footprint cells → 1 (lit-free: blocks placement until `fn38` turns
//!   them back to 0), park at x = 1000, state 0. States 0x13/0x14/0x15 park
//!   at x = 10000 themselves. The three "10 000 ms timers" the plan told
//!   this lane to hunt are those SetPos calls.
//! - The per-vignette graphs (enter: SetRun/SetRunList + position + frame
//!   delay; update: on done, a roll per current run) are transcribed
//!   verbatim in `enter`/`update` below, run numbers as the C's hex.
//!
//! ## Timing
//! - Windows self-pace in ms (`+0x18c` per state: 100/125/150/175 ms, the
//!   0x19 idle gap `RandomBelow(8000)+5000`, state 7's `RandomBelow(13000)`).
//! - The toggle, relight and ambient rolls run once per DoDrawFrame
//!   (`fn39` state 3, @6ECA / @6F7E / @77BA), and After Dark calls
//!   DoDrawFrame far more often than once per Mac tick: `tick()` runs
//!   `DRAW_FRAMES_PER_TICK` = 185 of them (`draw_frame`), all on the tick's
//!   one `now` (`fn27` reads the ms clock per call; it only moves per tick),
//!   so the ms-paced windows still step once per deadline. The scene
//!   deadline `+0x23` is 0: no startup wait.
//! - *Superseded 2026-09-29*: "one DoDrawFrame per Mac tick" (d249548),
//!   read off six lights in 43 s of the 60 s `voyeur.mp4`. The lights were
//!   vignettes. The rate is in `fn38`'s flashes: a cell it lights (dark →
//!   lit-empty) goes dark again on the first `RandomBelow(500) == 0` roll
//!   that picks it out of cols × rows, i.e. after 500 × 12 DoDrawFrames on
//!   the golden's 4 × 3 grid. `voyeur-long-1/2` show 896 such flashes
//!   (frame-diffed cell by cell against the scene's closed and lit-empty
//!   cells) living 0.53 / 0.54 s (MLE over the 10 fps sampling) → ≈ 11 200
//!   DoDrawFrames/s ≈ 185 per 16.625 ms tick; dark stretches last ~5 s,
//!   the 1-in-10 relight. The 60 s Demo take `voyeur.mp4` flashes the same
//!   way (0.05–1 s lights from 0.6 s in). At one per tick a flash lasted
//!   ~100 s, the ambient horn/siren rolled once per 83 s (golden: 338 cue
//!   hits in 450 s) and a window vignette started once per 20 s.
//!   `GAP(draw rate)`: 185 is After Dark's host frame rate on the rig,
//!   measured, not a constant of the C.
//!
//! ## Hand-offs (2026-09-29, the library's linked model)
//! The vtable at `g011A` (decoded from `emu/ghidra/voyeur/blocks`) binds
//! `+0x7C` = L132 `fn0204`, `+0x108` = `fn028A`, `+0xCC` = `fn0D3C`,
//! `+0xD0` = `fn0DF4`, `+0xF0` = `fn1186`, `+0xDC` = `fn0F52`, `+0x94` =
//! `fn05D4`, `+0x98` = `fn0718`. Every SetRun, SetRunList, queued run and
//! repeat goes through `fn028A`: `+0xCC` onto the run's link marker (`r − 1`
//! when that record exists) through L135 `fn3F2E` — the first body part
//! the two frames share stays on screen and the flip toggles where the
//! parts' own flip flags differ — then `+0xD0` marker → run by L135 `fn3DDC`
//! (flip-aware, x negated while mirrored). What the port did before, and
//! the measured damage (census, 8 seeds × 20 min: 516 of 9719 hand-offs
//! off, max 226 px; `frame` + `motion_lint`: 23 teleports → 0):
//! - SetRun applied `centre(new) − centre(cur)`: window vignettes whose runs
//!   are authored in different bank cells threw the whole window one cell
//!   (99 px) sideways and back (state 0x19's 0xff ↔ 0x698 / 0x6cf / 0x6e2).
//! - In-run steps ignored the flip, so mirrored flyers moonwalked.
//! - A repeat restarted with no delta (*superseded*: "the fn45-style
//!   no-delta wrap" this header used to cite) — the saucer froze one step
//!   in six; the capture shows 18 px on every step, the wrap included.
//! - Three `+0xCC` calls were dropped: ENTER 0xd (0x738 before the list),
//!   0x16's saucer launch (0x1b before SetRun 0x14), 0xe's fly-out (0xa65
//!   before SetRun 0xa39).
//! - ENTER 4 ran state 6's 1099; the C runs 0x9b4 and toggles the flip
//!   again when `g0E49` is set.
//! - `+0x94`/`+0x98` (*superseded GAP*): ported as `traveller_frames` /
//!   `traveller_place`. `fn05D4` counts, on a saved copy, the frames the
//!   flyer has been in the sky rect by walking whole cycles back
//!   (`fn0F52`); `fn0718` puts it that many frames back with the repeat
//!   count that flies it exactly to where the enter placed it. The first
//!   argument (0; 0x14 for 0x15; 0x23 for 0x16) is the run handed off to
//!   first, 0 = detach. That is what brings state 0x16's saucer in from the
//!   far screen edge and lands the abduction composite on the cat's window.
//!
//! ## Free flyers against the long golden (2026-09-29, `voyeur-long-1/2`)
//! 900 s, 10 fps, no panel. Port: 12 seeds × 30 min, `trace_voyeur_flyers`.
//!
//! | | golden | port |
//! |---|---|---|
//! | saucer (0x15) crossing, first → last frame on screen | 4.0–4.2 s | 4.14–4.24 s |
//! | saucer passes edge to edge | 8 of 8 | 26 of 26 |
//! | saucer y | 156–356 | 115–350 |
//! | toaster (0x14) dive | 6.3–6.5 s, top-right → left edge, y 128 → 408..456 | 5.2–6.5 s, down and away from the flip side |
//! | saucer → next saucer | one pair 1.6 s apart | shortest 1.9 / 2.3 s (185 draws/tick); parks 1.3–2.3 s after leaving |
//! | flyer rate | 8 saucers + 3 toasters / 900 s | 3.0 + 2.4 / 900 s (was 0.9 + 1.1 at one draw per tick) |
//!
//! Settled from the C:
//! - `traveller repeat` (*superseded GAP*): the flyers' `+0x164`/`+0x166`
//!   copy is a library Point {h, v} and `g0E6C` a {l, t, r, b} rect —
//!   L135 `fn3DDC` negates the Point's FIRST word on the horizontal flip
//!   and averages rect words 0/2 for it; `fn36` tests `g0E70 − g0E6C` for
//!   the "wider than 480" column rule; 0x15 picks `g0E6C` or `g0E70` by
//!   the horizontal flip. So 0x13/0x15 divide x by the step's h against
//!   the right (flipped 0x15: left) edge and 0x14 divides y by its v
//!   against the bottom — what the port did. The old note ("possibly the
//!   vertical coordinate") misread the enter's `g0E6C`/`g0E70` as top/
//!   bottom. The step now uses the state's own run (0xa47/0xa39/0x14) as
//!   the C passes it, not whatever run is current.
//! - `g0E6C` (*superseded GAP*): `SKY`, the canvas bounds (`fn21`).
//! - The chooser's retry walks on from its last pick (@7008 / @70E2); the
//!   old port restarted every retry at record 0. Past record 25 the walk
//!   reads the `g0BA2` pick tables (static DATA) as records: weights 20
//!   and 2500, rows 20 and 5000, so both always fail the fit test
//!   (`OFF_TABLE`) and the walk sticks there until the 100-retry bail,
//!   unless two `r == 0` rolls step it back onto record 25. Starts: one
//!   per 16 s → one per 19 s (12 seeds × 30 min, `trace_voyeur_start_rate`).
//!
//! ## Vignette start rate against the long golden (2026-09-29)
//! Golden: `voyeur-long-1/2` cell by cell (4 × 3 grid, each cell's frames
//! sorted closed / lit-empty / other); a start is a closed cell giving way
//! to a run that holds a scene (other) frame, same-row cells within 0.2 s
//! merged. Port: 12 odd seeds whose scene rolls 3 rows × 30 min,
//! `VOYEUR_ROWS=3 trace_voyeur_start_rate` (flyers counted apart).
//!
//! | | golden | port, 1 draw/tick | port, 185 draws/tick |
//! |---|---|---|---|
//! | window vignette starts | 108 / 900 s (97 counting runs ≥ 1 s): one per 8.3–9.3 s | one per 19.7 s | one per 10.0 s |
//! | gap p10/25/50/75/90 (s) | 0.9/2.3/4.0/9.0/23.1 (≥ 1 s: 1.6/2.4/4.5/10.0/25.2) | 2.1/6.7/14.7/27.9/42.6 | 0.8/2.7/6.6/13.2/22.2 |
//! | saucers + toasters / 900 s | 8 + 3 | 0.9 + 1.1 | 3.0 + 2.4 |
//! | lit-empty flash life | 0.54 s | ~100 s | 0.51 s |
//!
//! The rate is now window-bound (five windows; 3/4/5-row scenes start one
//! per 10.0 / 9.5 / 9.2 s): a free window is refilled within a tick or two.
//!
//! ## GAP (decompile failed or not ported — no invented glue)
//! - `GAP(draw rate)`: `DRAW_FRAMES_PER_TICK` (see "Timing") is measured
//!   off the rig, not read from the C.
//! - `GAP(flyer rate)` (narrowed 2026-09-29): toasters match (2.4 vs 3 per
//!   900 s) but saucers do not (3.0 vs 8; P ≈ 1 % for ≥ 8 at 3.0). Both
//!   records weigh 5 and neither needs a cell, so the whole-rate cause
//!   (the draw rate, above) is fixed and what is left is saucer-only. Not
//!   found in the C; the golden is 8 events.
//! - `GAP(saucer pair)` (*superseded*): take 1's R→L saucer at y 292
//!   (18.6–22.7 s) and L→R one at y 214 (24.3–28.5 s) are two 0x15
//!   vignettes. The park frees record 19 and the window 1.3–2.3 s after the
//!   first leaves, and at 185 DoDrawFrames a tick the free window is
//!   relit within a tick or two; picking 0x15 again rolls a new flip and y.
//!   The port now does it (shortest saucer → saucer gaps 1.9 / 2.3 s over
//!   12 seeds × 30 min).
//! - `GAP(state 0xe fly-in)`: 0xe's flyer (run 0xa39 after `+0xCC(0xa65)`,
//!   backed out to the sky edge by `+0x94`/`+0x98`) flying back into its
//!   window is ported from the C but in no golden: 900 s of `voyeur-long`
//!   show no flyer leaving or entering a window and no `snd 1003`.
//! - `fn38`'s draw calls pass no frame: dark→lit paints frame 0xc (the
//!   empty lit window state 0x16 paints), lit→dark paints 0x30c.
//! - The chooser's `RandomBelow(total) == 0` walks to index −1 in the C;
//!   read as index 0. The footprint-marking loop bound decompiles as
//!   `rows + 2*row`; read as `row + rows` (what `fn37` uses).
//! - Skyline window twinkle (`+0x9a` records, 1.5 s toggles) not ported.
//! - `fn36`'s facade paint is the tiled cell 0x30c + side/roof pieces
//!   0x311/0x310 per cell; the golden-verified wall sprite 1007/781 is kept.
//!
//! ## Layout (kept from the capture-verified passes; see git history for
//! ## the measurements)
//! The building is one wall sprite (1007/781, bottom-flush, flush to the
//! anchored side), cells 99×86 on its 4×5 tiling; the sky is one of three
//! series-2000 gradients rolled per scene; the starfield is plotted at
//! runtime (`fn36` @5CBE); skyline + fire escape mirror with the anchor;
//! rooftop props sit on the roofline; the moon is a per-scene roll.

use engine::l135::{self, FrameBox, LinkModel};
use engine::{ControlDef, Ctx, Image, Module, Pack, SpriteDraw, TickClock, SCREEN_H, SCREEN_W};

const BASE: u32 = 1000;
const BASE_BUILDING: u32 = 1500;
const BASE_WALL: u32 = 1007;
const BASE_OVERLAY: u32 = 2000;

/// PlaySound(id) plays snd 1000+id (`fn43` @7A8E loads 1000..1006).
const SND_BASE: u32 = 1000;

/// `+0x08 = now + 1800000` (`fn21` @4E92, `fn27` @57DE).
const SCENE_RESET_MS: u64 = 1_800_000;
/// `g0E50` (`fn24` @53DA) — the speed knob every roll is quoted against.
const SPEED: i32 = 60;
/// `fn39`: `RandomBelow((100 − g0E50) * 10 + 100)` = 500.
const RELIGHT_ROLL: u32 = ((100 - SPEED) * 10 + 100) as u32;
/// `fn39`: the cell toggle roll and the ambient-sound roll.
const CELL_ROLL: u32 = 500;
const AMBIENT_ROLL: u32 = 5000;
const CHOOSER_RETRIES: i32 = 100;
/// DoDrawFrames per Mac tick — After Dark's host frame rate, not a constant
/// of the C. `GAP(draw rate)`: measured from `voyeur-long-1/2` (no panel,
/// 900 s): the `fn38` lit-empty flashes (dark → lit → dark, 896 of them)
/// live 0.54 s (MLE over the 10 fps sampling, takes 0.53 / 0.54 s); a lit
/// cell goes dark on its first `RandomBelow(500) == 0` roll that picks it
/// out of 4 × 3, so 500 × 12 / 0.54 s ≈ 11 200 DoDrawFrames/s ≈ 185 per
/// 16.625 ms tick. `fn27` reads the clock once per call, so all of them in
/// one tick see the same `now` and the ms-paced windows still step once.
const DRAW_FRAMES_PER_TICK: u32 = 185;
/// The weights the chooser's walk reads past the 26 descriptors (`g04BA` +
/// 26/27 × 0x44 + 6, inside the `g0BA2` pick tables). 2500 exceeds any
/// roll, so the walk never goes further.
const OFF_TABLE: [i32; 2] = [20, 2500];
const WINDOW_COUNT: usize = 5; // g0E4C
/// Bank-1000 frames the module paints into the backdrop.
/// The closed shutter cell. `fn38`/`fn36` draw it with `p_RLE_Draw` (via
/// `fn59` @4C94) — RLE ART 780, packed as series 1007 frame 780 (99×86) —
/// not through the compound bank's draw like `0xc`. Read as compound frame
/// 780 (three LIT windows, 298 px wide) it hung two cells of lit wall over
/// the skyline whenever the building stood on the left (golden `voyeur`:
/// clean). Fixed 2026-09-30.
const CLOSED_CELL: i32 = 0x30c;
const LIT_EMPTY: i32 = 0xc; // state 0x16's empty lit window (fn38 GAP)
/// Off-screen parks: `SetPos(1000, x)` in 0x1c/0xb/0xe/0x16, `SetPos(10000, x)`
/// in 0x13/0x14/0x15.
const PARK_X: i32 = 1000;
const FAR_PARK_X: i32 = 10000;
/// The sky rect `g0E6C` {l, t, r, b} the free flyers are placed in and
/// `+0x94` tests against: the ctor fills it with the draw canvas's bounds,
/// `SetDrawOrigin()`'s canvas `+0x60` (`fn21` @523A..524C) — the whole screen.
/// Golden (`voyeur-long-1/2`): every saucer crosses the full 640 px and its
/// y spans 156..356 (the port's middle half of 0..480 is 120..360).
const SKY: (i32, i32, i32, i32) = (0, 0, SCREEN_W, SCREEN_H);

// ---------------------------------------------------------------------------
// §5 building geometry — measured from the DEPTH=32 golden capture.

const WALL_FRAME: u32 = 781;
const WALL_W: i32 = 396;
const WALL_ART_ROWS: u32 = 5;
const MAX_COLS: u32 = 4;
const MAX_ROWS: u32 = 5;
const CELL_W: i32 = 99;
const CELL_H: i32 = 86;
const SKY_VARIANTS: u32 = 3;
const SKY_TILE_W: i32 = 8;
const STAR_RECT_LEFT: i32 = 0;
const STAR_RECT_TOP: i32 = 0;
const STAR_RECT_RIGHT: i32 = SCREEN_W;
const STAR_RECT_BOTTOM: i32 = SCREEN_H;
const STAR_C0: [u8; 3] = [255, 255, 204]; // DATA 129 +0x0C1C
const STAR_C1: [u8; 3] = [102, 153, 255]; // DATA 129 +0x0C22
const STAR_C2: [u8; 3] = [0, 0, 187]; // DATA 129 +0x0C28
const STAR_COLORS: [[u8; 3]; 3] = [STAR_C0, STAR_C1, STAR_C2];

#[derive(Clone, Copy)]
struct Star {
    x: i32,
    y: i32,
    color: u8,
}

/// One `g04BA` descriptor (0x44 bytes), decoded from
/// `emu/ghidra/voyeur/blocks/A5_globals.bin` at the M129 data base + 0x4BA:
/// +0 entry state, +2 rows, +4 cols, +6 weight, +8 placed (has a cell
/// footprint; 17/18/19 fly free), +9 pin to the outer column, +0xa pin to
/// the top row, +0xe random flip allowed, +0x20 the 6×6 footprint. The
/// runtime bytes (+0xd busy, +0x10 cooldown) live in `desc_busy` — the
/// cooldown is only ever zeroed (`fn21` @4F5A, `fn27` @57BC), so it never
/// rejects anything.
struct Desc {
    state: u16,
    rows: i32,
    cols: i32,
    weight: u32,
    placed: bool,
    pin_x: bool,
    pin_top: bool,
    rand_flip: bool,
    fp: [[u8; 6]; 6],
}

const DESCS: [Desc; 26] = [
    Desc { state: 0x2, rows: 1, cols: 3, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,1,1,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x3, rows: 1, cols: 3, weight: 10, placed: true, pin_x: true, pin_top: false, rand_flip: false, fp: [[1,1,1,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x4, rows: 1, cols: 4, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: false, fp: [[1,1,1,1,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x5, rows: 1, cols: 4, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: false, fp: [[1,1,1,1,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x6, rows: 1, cols: 1, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x7, rows: 1, cols: 1, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x8, rows: 1, cols: 1, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x9, rows: 2, cols: 1, weight: 3, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xa, rows: 1, cols: 1, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xb, rows: 1, cols: 1, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xc, rows: 1, cols: 2, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,1,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xd, rows: 2, cols: 3, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: false, fp: [[1,1,1,0,0,0], [1,1,1,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xe, rows: 1, cols: 1, weight: 5, placed: true, pin_x: false, pin_top: false, rand_flip: false, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0xf, rows: 1, cols: 1, weight: 5, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x10, rows: 1, cols: 1, weight: 2, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x11, rows: 1, cols: 1, weight: 20, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x12, rows: 1, cols: 2, weight: 5, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,1,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x13, rows: 0, cols: 0, weight: 0, placed: false, pin_x: false, pin_top: false, rand_flip: true, fp: [[0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x14, rows: 0, cols: 0, weight: 5, placed: false, pin_x: false, pin_top: false, rand_flip: true, fp: [[0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x15, rows: 0, cols: 0, weight: 5, placed: false, pin_x: false, pin_top: false, rand_flip: true, fp: [[0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x16, rows: 1, cols: 1, weight: 5, placed: true, pin_x: true, pin_top: false, rand_flip: false, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x17, rows: 1, cols: 2, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,1,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x18, rows: 1, cols: 1, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x19, rows: 1, cols: 1, weight: 80, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x1a, rows: 1, cols: 1, weight: 80, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
    Desc { state: 0x1b, rows: 1, cols: 1, weight: 10, placed: true, pin_x: false, pin_top: false, rand_flip: true, fp: [[1,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0], [0,0,0,0,0,0]] },
];

/// The weighted (run, weight) pick tables the enters of 0x19/0x1a/0x1b walk
/// (`g0BA2`, `g0BB2`, `g0BCE`).
const T_19: [(i32, i32); 4] = [(1, 20), (227, 20), (255, 2000), (635, 10)];
const T_1A: [(i32, i32); 7] = [(659, 20), (572, 20), (671, 20), (603, 5), (608, 5), (2268, 20), (2737, 20)];
const T_1B: [(i32, i32); 2] = [(576, 5), (1399, 5)];

/// The update tail's cue table (`fn13` @46E2..): displayed frame → snd id.
fn cue_for(frame: i32) -> Option<u32> {
    Some(match frame {
        0x23 => 2,
        0x2c => 3,
        0x1bd | 0x711 => 1,
        0x7d5 => 4,
        0x66e | 0xa13 | 0xc08 | 0x7fa | 0xa6f => 0,
        0xa85 | 0xa89 | 0xa8c | 0xa8e | 0xa93 | 0xa97 | 0xa9b => 2,
        0xbe9 => 1,
        _ => return None,
    })
}

/// The taglines are STR# 128..148, one message each, read from the pack;
/// the pick is `128 + RandomBelow(20)` so the last one (STR# 148) is
/// unreachable — original bug kept.
const STR_TAGLINE_FIRST: u16 = 128;
const TAGLINE_PICKS: u32 = 20;

/// Sky-side skyline instances `(series, frame, dx outward from the sky-side
/// wall edge, y)` — measured from the golden.
const SKY_SIDE: &[(u32, u32, i32, i32)] = &[
    (BASE_BUILDING, 1, -141, 248),
    (BASE_BUILDING, 1, 44, 248),
    (BASE_BUILDING, 1, 153, 248),
];
const ESCAPE_FRAME: u32 = 11;
const ESCAPE_DY: i32 = 16;
/// Rooftop props `(series, frame, dx from the wall's left edge, height)`,
/// bottom-flush on the roofline.
const ROOF: &[(u32, u32, i32, i32)] = &[
    (BASE_BUILDING, 4, 204, 129),
    (BASE_BUILDING, 7, 225, 53),
    (BASE_BUILDING, 5, 101, 93),
    (BASE_BUILDING, 6, 222, 42),
    (BASE_BUILDING, 8, -6, 20),
];
const MOON_FRAME: u32 = 0;
const MOON_W: i32 = 25;

/// Generated-sprite name of star colour `idx` (see [`star_image`]). Until
/// 2026-09-19 these were 1x1 PNGs written into
/// `<pack>/compounds/2000/star_N.png`; inside a read-only `.saver` bundle
/// the write failed and the whole starfield vanished.
fn star_name(idx: usize) -> String {
    format!("{}star/{idx}", engine::GEN_PREFIX)
}

/// One star: a 1x1 opaque pixel of its colour. The original PaintRect's a
/// single pixel; the engine blits images, so the pixel is an image.
fn star_image(name: &str) -> Option<Image> {
    let idx: usize = name.strip_prefix(&format!("{}star/", engine::GEN_PREFIX))?.parse().ok()?;
    let c = *STAR_COLORS.get(idx)?;
    Some(Image { w: 1, h: 1, rgba: vec![c[0], c[1], c[2], 0xFF] })
}

/// The wall sprite for a scene of `rows` rows: 1007/781 itself on five
/// rows, else a bottom crop of it (the wall is bottom-flush). The crop used
/// to be written to `<pack>/compounds/1007/wall_rN.png` — read-only bundle,
/// no wall, same bug as the stars.
fn wall_name(pack: &Pack, rows: u32) -> Option<String> {
    let full = pack.frame(BASE_WALL, WALL_FRAME)?;
    if rows >= WALL_ART_ROWS {
        return Some(full.png.clone());
    }
    Some(format!("{}wall/{rows}", engine::GEN_PREFIX))
}

/// Bottom crop of the wall art to `rows` cells. `None` when the row count
/// asks for nothing or for more than the art holds — the caller then draws
/// no wall, exactly as the old writer's `keep == 0 || keep > img.h` guard did.
fn wall_image(pack: &Pack, name: &str) -> Option<Image> {
    let rows: u32 = name.strip_prefix(&format!("{}wall/", engine::GEN_PREFIX))?.parse().ok()?;
    let full = pack.frame(BASE_WALL, WALL_FRAME)?;
    let img = pack.image(&full.png);
    let keep = rows * CELL_H as u32;
    if keep == 0 || keep > img.h {
        return None;
    }
    let skip = (img.h - keep) as usize * img.w as usize * 4;
    Some(Image { w: img.w, h: keep, rgba: img.rgba[skip..].to_vec() })
}

/// A frame painted into the backdrop (`fn59` / `g0CE4->+4` into `g0E64`,
/// then blitted): drawn between the wall and the live windows.
#[derive(Clone)]
struct Paint {
    frame: i32,
    x: i32,
    y: i32,
    flip: bool,
}

/// One window sprite (`g0CF4[i]`, `fn09` ctor) with its L132 run driver.
#[derive(Clone)]
struct Win {
    /// `+0x154` — `g0CE8` counter, 1..=5 (`fn12`/`fn39` scan 1..=g0E4E).
    idx: usize,
    state: u16,
    prev: u16,
    // L132 run driver (offsets on the sprite)
    run_first: i32, // +0x44
    cur: i32,       // +0x3a
    just_set: bool, // +0x48
    next: i32,      // +0x4e (−1 = none)
    queue: Vec<i32>,
    repeat: i32, // +0x4c
    done: bool,  // +0x46
    /// screen centre of the current frame (§2 model) — the sprite's +0x40
    pos: (i32, i32),
    /// the module's own copy at +0x164/+0x166: set by state 0 to the cell
    /// centre, re-applied with SetPos by every enter, synced back from the
    /// sprite where the C does `+0x164 = +0x40`
    home: (i32, i32),
    flip: bool, // +0x3c bit 0
    frame_delay: u64, // +0x18c
    deadline: u64,    // +0x174
    more: bool,       // +0x17c
    spill: bool,      // +0x158
    desc: usize,      // +0x17e
    col: i32,         // +0x184
    row: i32,         // +0x186
    cols: i32,        // +0x180
    rows: i32,        // +0x182
    pick: usize,      // +0x156 (index into the state's pick table)
    counter: i32,     // +0x152
}

impl Win {
    fn new(idx: usize) -> Self {
        Win {
            idx,
            state: 0,
            prev: 0,
            run_first: 0,
            cur: 0,
            just_set: false,
            next: -1,
            queue: Vec::new(),
            repeat: 0,
            done: false,
            pos: (-1000, -1000),
            home: (-1000, -1000),
            flip: false,
            frame_delay: 100,
            deadline: 0,
            more: false,
            spill: false,
            desc: 0,
            col: -1,
            row: -1,
            cols: 0,
            rows: 0,
            pick: 0,
            counter: 0,
        }
    }
}

/// The frame's `engine::l135` box (its bank rect `{l, t, r, b}` is the
/// OFtb origin + its own OFst offset); ids `<= 0` have none.
fn frame_box(pack: &Pack, f: i32) -> Option<FrameBox> {
    if f <= 0 {
        return None;
    }
    Some(FrameBox::of(pack.frame(BASE, f as u32)?))
}

/// L135 `fn3BD6 @3BD6`'s part layout, relative to the frame's mid-point
/// (`FrameBox::part_rel`): mirrored inside the frame rect while flipped.
#[cfg(test)]
fn part_rel(pack: &Pack, f: i32, part: &[i32; 7], flip: bool) -> Option<[i32; 4]> {
    Some(frame_box(pack, f)?.part_rel(part, flip))
}

/// L135 `fn3DDC @3DDC`, the in-run link `from → to` (sequence `+0x78`):
/// `l135::link` under `LinkModel::FN3DDC`.
fn link(pack: &Pack, from: i32, to: i32, flip: bool) -> Option<(i32, i32)> {
    Some(l135::link(&frame_box(pack, from)?, &frame_box(pack, to)?, flip, LinkModel::FN3DDC))
}

/// L135 **`fn3F2E @3F2E`** (sequence `+0x80`), the registration a run
/// hand-off links through (L132 `fn0D3C @0D3C`): `l135::register` — the
/// first art id the CURRENT and NEW frames' part tables share stays put,
/// the flip XORed with the two parts' own flip flags. No common part (or
/// an id `<= 0`): no move, no flip change. Returns
/// `(dx, dy, flip', shared art id)`.
fn shared_link(pack: &Pack, cur: i32, new: i32, flip: bool) -> (i32, i32, bool, Option<i32>) {
    let (Some(ga), Some(gb)) = (pack.frame(BASE, cur.max(0) as u32), pack.frame(BASE, new.max(0) as u32)) else {
        return (0, 0, flip, None);
    };
    if cur <= 0 || new <= 0 {
        return (0, 0, flip, None);
    }
    match l135::register(&FrameBox::of(ga), &ga.parts, &FrameBox::of(gb), &gb.parts, flip) {
        Some(r) => (r.delta.0, r.delta.1, r.flip, Some(r.art)),
        None => (0, 0, flip, None),
    }
}

/// L132 `fn028A @028A`'s two links from `cur` into run `r`, as a delta
/// (the test log's model): `+0xF0` = `fn1186 @1186` picks the marker,
/// `+0xCC` = `fn0D3C` registers `cur` on it through `fn3F2E`, `+0xD0` =
/// `fn0DF4` steps marker → `r` by `fn3DDC` under the new flip. `cur == 0`
/// (nothing shown since the reset): no link at all (`+0x3A` test @02A2).
#[cfg(test)]
fn handoff_delta(pack: &Pack, cur: i32, r: i32, flip: bool) -> (i32, i32, bool, Option<i32>) {
    if cur <= 0 || pack.frame(BASE, cur as u32).is_none() {
        return (0, 0, flip, None);
    }
    let marker = l135::marker_of(r, |f| f >= 1 && pack.frame(BASE, f as u32).is_some());
    let (dx, dy, flip2, art) = shared_link(pack, cur, marker, flip);
    let (lx, ly) = if marker != r { link(pack, marker, r, flip2).unwrap_or((0, 0)) } else { (0, 0) };
    (dx + lx, dy + ly, flip2, art)
}

/// What a state's done-branch asked for.
enum Next {
    Tail,
    State(u16),
}

pub struct Voyeur {
    pack: Pack,
    wins: Vec<Win>,
    /// `g0CEE[1..=5]` window busy bytes (index 0 unused, as in the C).
    busy: [bool; 6],
    /// `g0422[win]` go flag with the chooser's descriptor and (col,row).
    go: [Option<(usize, i32, i32)>; 6],
    /// descriptor busy bytes (`+0xd`).
    desc_busy: [bool; 26],
    /// `g0DE8[row*6+col]`: 0 dark/free, 1 lit-free, 2 occupied.
    grid: [[u8; 6]; 6],
    /// `g0DA0[row*6+col]`: owning window index (0 = none).
    owner: [[u8; 6]; 6],
    /// `g0E48` — the "something is on the ledge" latch.
    latch: bool,
    /// scene object state: 0 build, 3 main loop.
    ctrl_state: u16,
    /// backdrop paints in paint order
    paints: Vec<Paint>,
    /// `g0E40`: the DoDrawFrame timestamp a sound was last played on.
    sound_stamp: Option<u64>,
    scene_deadline: u64, // +0x08
    // scene globals
    rows: u32, // g0D0C
    cols: u32, // g0D0E
    wall_x: i32,
    wall_y: i32,
    sky_frame: u32,
    moon: Option<(i32, i32)>,
    wall_png: Option<String>,
    coin: bool, // g0E49
    tagline: String,
    inited: bool,
    stars: Vec<Star>,
    star_png: [String; 3],
    /// test-only trace of every run hand-off and SetPos (the ratchets and
    /// the `#[ignore]` trace read it)
    #[cfg(test)]
    log: Vec<Ev>,
}

/// A trace event: a run hand-off (`fn028A`) or a `+0x88` SetPos.
#[cfg(test)]
#[derive(Clone, Debug)]
enum Ev {
    Handoff {
        w: usize,
        state: u16,
        from: i32,
        to: i32,
        /// pos before / after the hand-off
        p0: (i32, i32),
        p1: (i32, i32),
        flip0: bool,
        flip1: bool,
        /// what the linked model (`fn3F2E` through the marker, then `fn3DDC`)
        /// gives from the same start
        model: (i32, i32, bool, Option<i32>),
    },
    Place { w: usize },
    /// `fn39` handed a window its go flag (a vignette start)
    Start { desc: usize },
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    Some(Box::new(build(pack)?))
}

fn build(pack: Pack) -> Option<Voyeur> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    let star_png = [star_name(0), star_name(1), star_name(2)];
    Some(Voyeur {
        pack,
        wins: (1..=WINDOW_COUNT).map(Win::new).collect(),
        busy: [false; 6],
        go: [None; 6],
        desc_busy: [false; 26],
        grid: [[0; 6]; 6],
        owner: [[0; 6]; 6],
        latch: false,
        ctrl_state: 0,
        paints: Vec::new(),
        sound_stamp: None,
        scene_deadline: 0,
        rows: MAX_ROWS,
        cols: MAX_COLS,
        wall_x: 0,
        wall_y: SCREEN_H - MAX_ROWS as i32 * CELL_H,
        sky_frame: 0,
        moon: None,
        wall_png: None,
        coin: false,
        tagline: String::new(),
        inited: false,
        stars: Vec::new(),
        star_png,
        #[cfg(test)]
        log: Vec::new(),
    })
}

/// A traveller step as a divisor: `|a / b| = |a / |b||` under truncation;
/// a zero step (no such run) divides by 1 instead of trapping.
fn nz(v: i32) -> i32 {
    v.abs().max(1)
}

fn rnd(ctx: &mut Ctx, n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    ctx.rng.pct(n as u32) as i32
}

impl Voyeur {
    // -----------------------------------------------------------------
    // layout (capture-verified, kept)

    /// `fn36` rolls: `g0E49`, `g0D0E`, `g0D0C`, the sky variant, the moon.
    fn roll_layout(&mut self, ctx: &mut Ctx) {
        self.coin = ctx.rng.pct(2) != 0; // g0E49
        self.cols = ctx.rng.pct(2) + 3; // g0D0E
        self.rows = ctx.rng.pct(3) + 3; // g0D0C
        if SCREEN_H > 480 {
            self.rows = ctx.rng.pct(2) + 4;
        }
        if SCREEN_W > 480 {
            self.cols = 4;
        }
        self.wall_x = if self.coin { SCREEN_W - WALL_W } else { 0 };
        self.wall_y = SCREEN_H - self.rows as i32 * CELL_H;
        self.wall_png = wall_name(&self.pack, self.rows);
        self.sky_frame = ctx.rng.pct(SKY_VARIANTS);
        self.moon = (ctx.rng.pct(2) != 0).then(|| {
            let x = ctx.rng.pct((SCREEN_W - MOON_W) as u32) as i32;
            let y = ctx.rng.pct(((SCREEN_H - STAR_RECT_TOP) / 3) as u32) as i32;
            (x, STAR_RECT_TOP + y)
        });
    }

    /// `fn36` @5CBE: `200 + rnd(200)` stars, y over the top third, two
    /// colour downgrades rising with depth.
    fn roll_stars(&mut self, ctx: &mut Ctx) {
        let count = 200 + ctx.rng.pct(200);
        let width = (STAR_RECT_RIGHT - STAR_RECT_LEFT) as u32;
        let y_bound = ((STAR_RECT_BOTTOM - STAR_RECT_TOP) / 3) as u32;
        self.stars.clear();
        for _ in 0..count {
            let x = STAR_RECT_LEFT + ctx.rng.pct(width) as i32;
            let y = ctx.rng.pct(y_bound);
            let mut color = 0u8;
            if ctx.rng.pct(2 * (y_bound / 3)) < y {
                color = 1;
            }
            if ctx.rng.pct(y_bound) < y {
                color = 2;
            }
            self.stars.push(Star { x, y: STAR_RECT_TOP + y as i32, color });
        }
    }

    fn sky_side_x(&self, dx: i32, art_w: i32) -> i32 {
        if self.coin {
            self.wall_x - art_w - dx
        } else {
            self.wall_x + WALL_W + dx
        }
    }

    /// Top-left of grid cell (col,row). Rows count UP from the bottom, as
    /// `fn36`'s `g0D10` fill does (y −= cell height per row).
    fn cell_tl(&self, col: i32, row: i32) -> (i32, i32) {
        (
            self.wall_x + CELL_W * col,
            self.wall_y + CELL_H * (self.rows as i32 - 1 - row),
        )
    }

    /// `g0D10[row*6+col]`: the cell's centre.
    fn cell_centre(&self, col: i32, row: i32) -> (i32, i32) {
        let (x, y) = self.cell_tl(col, row);
        (x + CELL_W / 2, y + CELL_H / 2)
    }

    /// `g0E68`: the lowest point of the open sky above the roof props and
    /// the skyline (the 0x13 sleigh flies in the band above it).
    fn sky_bottom(&self) -> i32 {
        let mut b = self.wall_y;
        for &(_, _, _, h) in ROOF {
            b = b.min(self.wall_y - h);
        }
        for &(_, _, _, y) in SKY_SIDE {
            b = b.min(y);
        }
        b
    }

    // -----------------------------------------------------------------
    // L132 run driver

    fn frame_size(&self, f: i32) -> (i32, i32) {
        match self.pack.frame(BASE, f as u32) {
            Some(fr) => (fr.w.max(1), fr.h.max(1)),
            None => (1, 1),
        }
    }

    /// `fn1260`: the run is the contiguous frames from `first`.
    fn run_last(&self, first: i32) -> i32 {
        if first <= 0 || self.pack.frame(BASE, first as u32).is_none() {
            return first.max(0);
        }
        let mut l = first;
        while self.pack.frame(BASE, (l + 1) as u32).is_some() {
            l += 1;
        }
        l
    }

    /// `+0xFC(1, run)` = L132 `fn12DE @12DE` in its second mode: seat the
    /// run's first frame at the origin and `+0xD0` straight to its last,
    /// i.e. the `fn3DDC` link first → last under the sprite's flip. The
    /// traveller updates divide their remaining distance by it.
    fn run_step(&self, w: usize, first: i32) -> (i32, i32) {
        let last = self.run_last(first);
        link(&self.pack, first, last, self.wins[w].flip).unwrap_or((0, 0))
    }

    /// `+0xD0` = L132 `fn0DF4 @0DF4` SetFrame: `pos` moves by the sequence's
    /// `fn3DDC @3DDC` link cur → f (flip-aware). Nothing shown yet (`+0x3A`
    /// = 0): the frame is just seated.
    fn set_frame(&mut self, w: usize, f: i32) {
        let (cur, flip) = (self.wins[w].cur, self.wins[w].flip);
        if cur > 0 {
            if let Some((dx, dy)) = link(&self.pack, cur, f, flip) {
                self.wins[w].pos.0 += dx;
                self.wins[w].pos.1 += dy;
            }
        }
        self.wins[w].cur = f;
    }

    /// `+0xCC` = L132 `fn0D3C @0D3C`: seat frame `f` so the first part it
    /// shares with the current frame stays where it is on screen, toggling
    /// the flip where the two parts' own flip flags differ (L135 `fn3F2E`;
    /// `+0x80` is 1 after the `fn0058` reset). No run change.
    fn link_to(&mut self, w: usize, f: i32) {
        let win = &mut self.wins[w];
        if win.cur > 0 {
            let (dx, dy, flip, _) = shared_link(&self.pack, win.cur, f, win.flip);
            win.pos.0 += dx;
            win.pos.1 += dy;
            win.flip = flip;
        }
        win.cur = f;
    }

    /// `+0x108` = L132 **`fn028A @028A`**, the run hand-off every SetRun,
    /// SetRunList, queued run AND repeat goes through: `+0xCC` onto the
    /// run's link marker (`+0xF0` = `fn1186 @1186`: `r − 1` when that record
    /// exists, else `r`), then `+0xD0` marker → `r`; `+0x44 = r`, the
    /// first-tick hold `+0x48 = 1`, `+0x4E = −1`. See `handoff_delta`.
    fn handoff(&mut self, w: usize, r: i32) {
        #[cfg(test)]
        let (from, p0, flip0) = (self.wins[w].cur, self.wins[w].pos, self.wins[w].flip);
        if self.wins[w].cur > 0 {
            let marker = self.marker(r);
            self.link_to(w, marker);
            self.set_frame(w, r);
        } else {
            self.wins[w].cur = r;
        }
        #[cfg(test)]
        self.log_handoff(w, from, r, p0, flip0);
        let win = &mut self.wins[w];
        win.run_first = r;
        win.just_set = true;
        win.next = -1;
    }

    /// `+0xF0` = L132 `fn1186 @1186`: the frame a hand-off into run `r`
    /// passes through — `r − 1` when that record exists, else `r`; 0 for an
    /// invalid id.
    fn marker(&self, r: i32) -> i32 {
        if r <= 0 || self.pack.frame(BASE, r as u32).is_none() {
            return 0;
        }
        l135::marker_of(r, |f| f >= 1 && self.pack.frame(BASE, f as u32).is_some())
    }

    /// `+0xDC` = L132 `fn0F52 @0F52`: take the sprite back one whole cycle
    /// of `run` — `+0xD0` onto the current run's marker (when a frame is
    /// shown), `+0xCC` onto `run`'s last frame (shared-part registration),
    /// `+0xD0` back to its first (the `fn3DDC` link last → first).
    fn cycle_back(&mut self, w: usize, run: i32) {
        if self.wins[w].cur != 0 {
            let m = self.marker(self.wins[w].run_first);
            self.set_frame(w, m);
        }
        let last = self.run_last(run);
        self.link_to(w, last);
        self.set_frame(w, run);
        self.wins[w].run_first = run;
    }

    /// The sprite rect `+0x18` meets the sky rect `g0E6C` (L132 `fn4590`).
    /// `+0x10C` refreshes that rect only while a frame is shown, so after a
    /// detach it is still the last shown frame's (`shown`). The rect is
    /// `SKY` (the canvas bounds, see there).
    fn in_sky(&self, w: usize, shown: i32) -> bool {
        let win = &self.wins[w];
        let (fw, fh) = self.frame_size(if win.cur > 0 { win.cur } else { shown });
        let (l, t) = (win.pos.0 - fw / 2, win.pos.1 - fh / 2);
        l < SKY.2 && l + fw > SKY.0 && t < SKY.3 && t + fh > SKY.1
    }

    /// `+0x94` = L132 **`fn05D4 @05D4`**: how many frames the traveller has
    /// been flying since it came into the sky rect. On a saved copy of the
    /// sprite: hand off to `s` (0 = detach: nothing shown, no link), then
    /// whole cycles of the current run BACK (`fn0F52`) while the sprite
    /// still meets the rect, counting each cycle's length (`+0x100`), then
    /// frames forward (`+0xD0`) until it meets the rect again, uncounting
    /// each. The sprite is restored (`+0xC0`/`+0xC4`).
    fn traveller_frames(&mut self, w: usize, s: i32) -> i32 {
        let saved = self.wins[w].clone();
        #[cfg(test)]
        let log_len = self.log.len();
        let (run, shown) = (self.wins[w].run_first, self.wins[w].cur);
        self.handoff(w, s);
        let mut n = 0;
        if self.in_sky(w, shown) {
            loop {
                self.cycle_back(w, run);
                n += self.run_last(run) - run + 1;
                if !self.in_sky(w, shown) {
                    break;
                }
            }
            loop {
                let last = self.run_last(self.wins[w].run_first);
                let cur = self.wins[w].cur;
                if last <= cur {
                    break;
                }
                self.set_frame(w, cur + 1);
                if self.in_sky(w, shown) {
                    break;
                }
                n -= 1;
            }
        }
        self.wins[w] = saved;
        #[cfg(test)]
        self.log.truncate(log_len);
        n
    }

    /// `+0x98` = L132 **`fn0718 @0718`**: put the traveller `n` frames back
    /// along its path and set the repeat count so that playing forward
    /// brings it exactly to where it is now. Hand off to `s`, then whole
    /// cycles back (`fn0F52`), `+0x4C` counting them from −1, until `n`
    /// frames are used up; `+0xD0` forward by the overshoot; `+0x4C` floored
    /// at 0. `n == 0`: `+0x7C(0)` (the sprite is detached).
    fn traveller_place(&mut self, w: usize, s: i32, mut n: i32) {
        let run = self.wins[w].run_first;
        if n == 0 {
            self.set_run(w, 0);
            return;
        }
        self.wins[w].repeat = -1;
        self.handoff(w, s);
        loop {
            self.wins[w].repeat += 1;
            self.cycle_back(w, run);
            n -= self.run_last(run) - run + 1;
            if n <= 0 {
                break;
            }
        }
        let f = self.wins[w].run_first - n;
        self.set_frame(w, f);
        if self.wins[w].repeat < 0 {
            self.wins[w].repeat = 0;
        }
    }

    /// `+0x7C` SetRun = L132 `fn0204 @0204`: the `fn028A` hand-off, then
    /// clear the queue and `+0x46`.
    fn set_run(&mut self, w: usize, f: i32) {
        self.handoff(w, f);
        let win = &mut self.wins[w];
        win.queue.clear();
        win.done = false;
    }

    /// `+0x80` SetRunList (`fn0240`): SetRun, then queue the rest.
    fn set_run_list(&mut self, w: usize, list: &[i32]) {
        self.set_run(w, list[0]);
        self.wins[w].queue.extend_from_slice(&list[1..]);
    }

    /// `+0x84` the tick (`fn0316`). The run queued by the previous tick —
    /// the next run in the list, or the same run again while the repeat
    /// count `+0x4C` lasts — starts through the same linked `fn028A`
    /// hand-off as an immediate SetRun.
    fn tick_run(&mut self, w: usize) {
        self.wins[w].done = false;
        let next = self.wins[w].next;
        if next != -1 {
            self.handoff(w, next);
        }
        let first = self.wins[w].run_first;
        let last = self.run_last(first);
        let cur = self.wins[w].cur;
        if !self.wins[w].just_set {
            if cur >= first && cur < last {
                self.set_frame(w, cur + 1);
            }
        } else {
            self.wins[w].just_set = false;
        }
        let cur = self.wins[w].cur;
        if !(cur >= first && cur < last) {
            let win = &mut self.wins[w];
            if win.repeat < 1 {
                if win.queue.is_empty() {
                    win.done = true;
                } else {
                    win.next = win.queue.remove(0);
                }
            } else {
                if win.repeat != 0x7fff {
                    win.repeat -= 1;
                }
                win.next = win.run_first;
            }
        }
    }

    /// `+0x104` (`fn13E0`): the displayed frame — the tail keys on it.
    fn displayed(&self, w: usize) -> i32 {
        self.wins[w].cur
    }

    // -----------------------------------------------------------------
    // module helpers

    /// `fn44` @7B8E: one sound per `g0E40` stamp (`fn27`'s ms clock: one per tick).
    fn play(&mut self, ctx: &mut Ctx, id: u32) {
        if self.sound_stamp == Some(ctx.now_ms) {
            return;
        }
        self.sound_stamp = Some(ctx.now_ms);
        ctx.sounds.push(SND_BASE + id);
    }

    /// `fn4640(&g0E38, &copy)` then a paint: the cell-sized rect centred on
    /// the window's position copy.
    fn paint_at_pos(&mut self, w: usize, frame: i32, flip: bool) {
        let (x, y) = self.wins[w].home;
        self.push_paint(Paint { frame, x: x - CELL_W / 2, y: y - CELL_H / 2, flip });
    }

    fn paint_cell(&mut self, col: i32, row: i32, frame: i32, flip: bool) {
        let (x, y) = self.cell_tl(col, row);
        self.push_paint(Paint { frame, x, y, flip });
    }

    /// Append a backdrop paint. An earlier paint of the same frame at the
    /// same spot and flip is covered pixel for pixel by the new one (same
    /// image, same mask), so it is dropped: the backdrop the list composes
    /// is unchanged and the list stays bounded (`fn38` toggles a cell
    /// several times a second at the golden's draw rate).
    fn push_paint(&mut self, p: Paint) {
        self.paints.retain(|q| (q.frame, q.x, q.y, q.flip) != (p.frame, p.x, p.y, p.flip));
        self.paints.push(p);
    }

    /// SetRun(f) then SetPos so f's rect hangs off the cell rect's top-left
    /// (the enter-2/3/0xc/0xd idiom: `+0x18(f,&rect)`, `fn4640`, centre).
    fn run_anchored(&mut self, w: usize, f: i32) {
        self.set_run(w, f);
        let (fw, fh) = self.frame_size(f);
        let (x, y) = self.wins[w].home;
        let (tx, ty) = (x - CELL_W / 2, y - CELL_H / 2);
        self.wins[w].home = (tx + fw / 2, ty + fh / 2);
        self.wins[w].pos = self.wins[w].home;
        #[cfg(test)]
        self.log.push(Ev::Place { w });
    }

    /// `SetPos(+0x164 copy)`: the enter idiom after SetRun.
    fn set_pos_home(&mut self, w: usize) {
        self.wins[w].pos = self.wins[w].home;
        #[cfg(test)]
        self.log.push(Ev::Place { w });
    }

    #[cfg(test)]
    fn log_handoff(&mut self, w: usize, from: i32, to: i32, p0: (i32, i32), flip0: bool) {
        let model = handoff_delta(&self.pack, from, to, flip0);
        let win = &self.wins[w];
        self.log.push(Ev::Handoff { w, state: win.state, from, to, p0, p1: win.pos, flip0, flip1: win.flip, model });
    }

    fn set_state(&mut self, w: usize, s: u16, ctx: &mut Ctx) {
        self.wins[w].prev = self.wins[w].state;
        self.wins[w].state = s;
        self.enter(w, s, ctx);
    }

    fn cell(&self, col: i32, row: i32) -> u8 {
        if (0..6).contains(&col) && (0..6).contains(&row) {
            self.grid[row as usize][col as usize]
        } else {
            0
        }
    }

    fn set_cell(&mut self, col: i32, row: i32, v: u8) {
        if (0..6).contains(&col) && (0..6).contains(&row) {
            self.grid[row as usize][col as usize] = v;
        }
    }

    /// Weighted walk of a (run, weight) table (the enters of 0x19/0x1a/0x1b).
    fn pick_table(ctx: &mut Ctx, t: &[(i32, i32)]) -> usize {
        let total: i32 = t.iter().map(|e| e.1).sum();
        let mut r = rnd(ctx, total);
        let mut i = 0usize;
        while r > 0 {
            r -= t[i].1;
            i += 1;
        }
        i.saturating_sub(1).min(t.len() - 1)
    }

    // -----------------------------------------------------------------
    // fn13: enter handlers (the `0x8000|s` branches)

    fn enter(&mut self, w: usize, s: u16, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        match s {
            0 => {
                self.wins[w].frame_delay = 100;
            }
            2 => {
                self.run_anchored(w, 0x2b3);
                self.wins[w].queue.extend_from_slice(&[0x365, 0x39a]);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            3 => {
                self.run_anchored(w, 0x3cd);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            4 => {
                // ENTER 4: SetRun(0x9b4), the flip toggled again when
                // `g0E49` is set, SetPos; the delay stays state 0's 100
                self.set_run(w, 0x9b4);
                if self.coin {
                    self.wins[w].flip = !self.wins[w].flip;
                }
                self.set_pos_home(w);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            5 => {
                // anchored on 0x2b3's rect, then shifted two cells across
                // (one cell back when mirrored), one pixel up, run 0x16e
                let (fw, fh) = self.frame_size(0x2b3);
                let (x, y) = self.wins[w].home;
                let (tx, ty) = (x - CELL_W / 2, y - CELL_H / 2);
                let mut px = tx + fw / 2;
                px += if !self.coin { CELL_W * 2 - 1 } else { -(CELL_W + 2) };
                self.set_run(w, 0x16e);
                self.wins[w].home = (px, ty + fh / 2 - 1);
                self.set_pos_home(w);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            6 => {
                self.set_run(w, 1099);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            7 => {
                self.set_run(w, 0xc66);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            8 => {
                self.set_run(w, 0xc1e);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 100;
                self.wins[w].deadline = now + 100;
            }
            9 => {
                self.set_run_list(w, &[0xb85, 0xbd4]);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 100;
                self.wins[w].deadline = now + 100;
            }
            0xa => {
                self.set_run(w, 0xbf6);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            0xb => {
                self.set_run(w, 0x8b8);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 100;
                self.wins[w].deadline = now + 100;
            }
            0xc => {
                // @1704: SetRun(0x58d) anchored; 1/20 → claim the cell below
                // (right/left half by flip) for the spill and paint 0xff there
                self.run_anchored(w, 0x58d);
                let (col, row, flip) = (self.wins[w].col, self.wins[w].row, self.wins[w].flip);
                let mut spill = false;
                if rnd(ctx, 20) == 0 && row != 0 {
                    let nc = col + i32::from(flip);
                    if self.cell(nc, row - 1) == 0 {
                        self.wins[w].home = self.cell_centre(nc, row - 1);
                        spill = true;
                        self.set_cell(nc, row - 1, 2);
                        self.paint_cell(nc, row - 1, 0xff, flip);
                    }
                }
                self.wins[w].spill = spill;
                self.wins[w].frame_delay = 0xaf;
                self.wins[w].deadline = now + 0xaf;
            }
            0xd => {
                // pos = the cell below, one column over when not flipped
                let (col, row, flip) = (self.wins[w].col, self.wins[w].row, self.wins[w].flip);
                let nc = col + i32::from(!flip);
                self.wins[w].home = self.cell_centre(nc, row - 1);
                self.run_anchored(w, 0x58d);
                self.wins[w].home.1 -= 1;
                self.set_pos_home(w);
                // ENTER 0xd, after the SetPos: `+0xCC(0x738)` lines 0x738
                // up on 0x58d's shared part before the list starts (the
                // port omitted it until 2026-09-29)
                self.link_to(w, 0x738);
                self.set_run_list(w, &[0x738, 0x768]);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            0xe => {
                self.set_run(w, 0xa29);
                let flip = self.wins[w].flip;
                self.paint_at_pos(w, 0xa29, flip);
                self.wins[w].home.0 += i32::from(self.coin);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            0xf => {
                self.set_run(w, 0x958);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            0x10 => {
                self.set_run(w, 0x8dc);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 100;
                self.wins[w].deadline = now + 100;
            }
            0x11 => {
                self.set_run(w, 0x806);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 150;
                self.wins[w].deadline = now + 150;
            }
            0x12 => {
                self.set_run(w, 0xb0b);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 125;
                self.wins[w].deadline = now + 125;
            }
            0x13 | 0x14 | 0x15 => {
                // free-flyers: random start in the middle half of the sky
                let run = match s {
                    0x13 => 0xa47,
                    0x14 => 0xa39,
                    _ => 0x14,
                };
                self.set_run(w, run);
                let (l, t, r, b) = SKY;
                let width = r - l;
                let x = l + rnd(ctx, width - width / 2) + width / 4;
                let y = if s == 0x13 {
                    let band = self.sky_bottom() - t - 5;
                    if band > 2 { t + rnd(ctx, band) } else { self.wins[w].home.1 }
                } else {
                    let height = b - t;
                    t + rnd(ctx, height - height / 2) + height / 4
                };
                self.wins[w].home = (x, y);
                self.set_pos_home(w);
                // `+0x94` / `+0x98` (s = 0, 0, 0x14 per the listing's pushes):
                // back up to where the flyer enters the sky, repeat count set
                // to fly it forward to this spot
                let s0 = if s == 0x15 { 0x14 } else { 0 };
                let n = self.traveller_frames(w, s0);
                self.traveller_place(w, s0, n);
                self.wins[w].more = true;
                self.wins[w].frame_delay = 100;
                self.wins[w].deadline = now + 100;
            }
            0x16 => {
                self.set_run(w, 1);
                self.set_pos_home(w);
                let flip = self.wins[w].flip;
                self.paint_at_pos(w, LIT_EMPTY, flip);
                self.wins[w].home.0 += i32::from(self.coin);
                self.set_pos_home(w);
                self.wins[w].frame_delay = 0x7d;
                self.wins[w].deadline = now + 0x7d;
            }
            0x17 | 0x18 => {
                self.set_run(w, if s == 0x17 { 0xaa } else { 0x1ff });
                self.wins[w].home.1 -= 1;
                self.set_pos_home(w);
                self.wins[w].frame_delay = 0x7d;
                self.wins[w].deadline = now + 0x7d;
                self.latch = true;
            }
            0x19 => {
                let p = Self::pick_table(ctx, &T_19);
                self.wins[w].pick = p;
                self.set_run(w, T_19[p].0);
                self.wins[w].counter = if SPEED < 0x62 { rnd(ctx, (100 - SPEED) / 4) + 4 } else { 4 };
                if self.latch && T_19[p].0 == 1 && rnd(ctx, 10) == 0 {
                    self.set_run(w, 0x14c);
                }
                self.set_pos_home(w);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            0x1a => {
                let p = Self::pick_table(ctx, &T_1A);
                self.wins[w].pick = p;
                self.set_run(w, T_1A[p].0);
                self.wins[w].counter = if SPEED < 0x62 { rnd(ctx, 100 - SPEED) + 0x14 } else { 0x14 };
                self.set_pos_home(w);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            0x1b => {
                let p = Self::pick_table(ctx, &T_1B);
                self.wins[w].pick = p;
                self.set_run(w, T_1B[p].0);
                self.set_pos_home(w);
                self.wins[w].deadline = now + self.wins[w].frame_delay;
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------
    // fn13: update

    /// The state-10-and-idle-gap block of state 0x19 (@3450..):
    /// counter down; at zero the vignette ends unless the latch holds it on
    /// run 1; else an idle gap and the picked run again.
    fn idle_19(&mut self, w: usize, ctx: &mut Ctx) -> Next {
        let c = self.wins[w].counter;
        self.wins[w].counter -= 1;
        if c == 0 {
            if !self.latch || self.wins[w].run_first != 1 {
                return Next::State(0x1c);
            }
            let f = if rnd(ctx, 2) == 0 { 0x159 } else { 0x162 };
            self.set_run(w, f);
        } else {
            self.wins[w].deadline += (rnd(ctx, 8000) + 5000) as u64;
            let f = T_19[self.wins[w].pick].0;
            self.set_run(w, f);
        }
        Next::Tail
    }

    /// End-of-vignette paint used by 0xe and 0x16 and the 0xc spill: park
    /// at x = 1000, paint the closed cell at the window's rect.
    fn park_and_close(&mut self, w: usize) {
        self.paint_at_pos(w, CLOSED_CELL, false);
        self.wins[w].pos.0 = PARK_X;
        #[cfg(test)]
        self.log.push(Ev::Place { w });
    }

    /// `+0x164 = +0x40`: sync the copy from the sprite.
    fn sync_home(&mut self, w: usize) {
        self.wins[w].home = self.wins[w].pos;
    }

    fn update(&mut self, w: usize, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        let s = self.wins[w].state;
        if s == 0 {
            // poll the go flag
            let idx = self.wins[w].idx;
            self.busy[idx] = false;
            if let Some((desc, col, row)) = self.go[idx] {
                self.busy[idx] = true;
                self.go[idx] = None;
                self.wins[w].col = col;
                self.wins[w].row = row;
                if col >= 0 {
                    let mut c = self.cell_centre(col, row);
                    c.0 += i32::from(self.coin);
                    self.wins[w].home = c;
                }
                self.wins[w].desc = desc;
                self.wins[w].cols = DESCS[desc].cols;
                self.wins[w].rows = DESCS[desc].rows;
                // @0B76..0C16: the flip is SET when `g0E49` is 0 and
                // CLEARED when it is 1 (the inlined set-flag idiom), i.e.
                // `!g0E49` — then the descriptor's coin
                let mut flip = !self.coin;
                if DESCS[desc].rand_flip && rnd(ctx, 2) != 0 {
                    flip = !flip;
                }
                self.wins[w].flip = flip;
                self.set_state(w, DESCS[desc].state, ctx);
            }
            return;
        }
        if s == 0x1c {
            // the vignette is over
            let idx = self.wins[w].idx;
            self.busy[idx] = false;
            self.desc_busy[self.wins[w].desc] = false;
            let (col, row, cols, rows) = (self.wins[w].col, self.wins[w].row, self.wins[w].cols, self.wins[w].rows);
            for r in row..row + rows {
                for c in col..col + cols {
                    self.set_cell(c, r, 1);
                    if (0..6).contains(&c) && (0..6).contains(&r) {
                        self.owner[r as usize][c as usize] = 0;
                    }
                }
            }
            self.wins[w].pos.0 = PARK_X;
            self.tick_run(w);
            self.set_state(w, 0, ctx);
            return;
        }
        if now < self.wins[w].deadline {
            return;
        }
        // pre-tick bail-outs
        match s {
            0x13 => {
                if self.sky_bottom() - 5 < 3 {
                    self.set_state(w, 0, ctx);
                    return;
                }
            }
            0xa => {
                let (col, row) = (self.wins[w].col, self.wins[w].row);
                if row < self.rows as i32 - 1 && self.owner[(row + 1) as usize][col as usize] != 0 {
                    self.set_state(w, 0x1c, ctx);
                    return;
                }
            }
            _ => {}
        }
        self.wins[w].deadline = now + self.wins[w].frame_delay;
        self.tick_run(w);
        let next = if self.wins[w].done { self.on_done(w, s, ctx) } else { Next::Tail };
        match next {
            Next::State(ns) => {
                self.set_state(w, ns, ctx);
            }
            Next::Tail => {
                let f = self.displayed(w);
                if let Some(id) = cue_for(f) {
                    self.play(ctx, id);
                }
            }
        }
    }

    /// The per-state "sequence finished" branches, keyed on the current
    /// run's first frame (`+0x44`).
    fn on_done(&mut self, w: usize, s: u16, ctx: &mut Ctx) -> Next {
        let run = self.wins[w].run_first;
        let (col, row, flip) = (self.wins[w].col, self.wins[w].row, self.wins[w].flip);
        match s {
            2 | 4 | 5 | 9 | 0xa | 0x12 | 0x17 | 0x18 | 0x1b => Next::State(0x1c),
            3 => {
                self.sync_home(w);
                self.wins[w].spill = false;
                Next::State(0x1c)
            }
            6 => {
                match run {
                    1099 => {
                        if rnd(ctx, 20) == 0 {
                            self.set_run_list(w, &[0x451, 0x453]);
                            if rnd(ctx, 20) == 0 {
                                return Next::State(0x1c);
                            }
                        }
                    }
                    0x453 => {
                        if rnd(ctx, 5) == 0 {
                            self.set_run(w, 0x45c);
                        } else {
                            self.set_run(w, 0x453);
                            if rnd(ctx, 10) == 0 {
                                self.set_run(w, 1099);
                            }
                        }
                    }
                    0x45c => {
                        if rnd(ctx, 10) == 0 {
                            match rnd(ctx, 6) {
                                0 | 1 => self.set_run(w, 0x465),
                                2 => self.set_run_list(w, &[0x4a9, 0x4c5, 0x4d7]),
                                3 => self.set_run_list(w, &[0x50e, 0x4fd]),
                                4 => self.set_run_list(w, &[0x529, 0x4fd]),
                                _ => self.set_run_list(w, &[0x559, 0x562, 0x572]),
                            }
                        } else {
                            self.set_run(w, 0x45c);
                            if rnd(ctx, 20) == 0 {
                                self.set_run(w, 0x453);
                            }
                        }
                    }
                    0x465 => {
                        if rnd(ctx, 2) == 0 {
                            self.set_run_list(w, &[0x486, 0x4a5]);
                        } else {
                            self.set_run(w, 0x47a);
                        }
                    }
                    0x4a5 | 0x4f5 | 0x4fd | 0x572 => {
                        if rnd(ctx, 3) == 0 {
                            return Next::State(0x1c);
                        }
                        self.set_run(w, 1099);
                    }
                    0x47a => self.set_run(w, 0x45c),
                    0x4d7 => {
                        if rnd(ctx, 3) == 0 {
                            self.set_run(w, 0x4f5);
                        } else {
                            self.set_run(w, 0x4d7);
                        }
                    }
                    _ => return Next::State(0x1c),
                }
                Next::Tail
            }
            7 => {
                if run == 0x19e {
                    self.latch = false;
                    return Next::State(0x1c);
                }
                if self.latch && rnd(ctx, 2) == 0 && row > 0 && self.cell(col, row - 1) == 0 {
                    self.set_run(w, 0x19e);
                    return Next::Tail;
                }
                if rnd(ctx, 8) == 0 && run != 0xc66 {
                    return Next::State(0x1c);
                }
                self.set_run(w, 0xc92);
                self.wins[w].deadline += rnd(ctx, 13000) as u64;
                Next::Tail
            }
            8 => {
                if rnd(ctx, 3) == 0 && run != 0xc1e {
                    return Next::State(0x1c);
                }
                self.set_run(w, 0xc37);
                Next::Tail
            }
            0xb => {
                match run {
                    0x8b8 => {
                        self.wins[w].pos.0 = PARK_X;
                        self.set_run_list(w, &[0x25b, 0x25b]);
                    }
                    0x25b => {
                        if rnd(ctx, 3) == 0 {
                            self.set_pos_home(w); // back on screen
                            self.set_run_list(w, &[0x8cf, 0x8cf, 0x8cf]);
                        } else {
                            self.set_run(w, 0x25b);
                        }
                    }
                    0x8cf => {
                        if rnd(ctx, 10) == 0 {
                            return Next::State(0x1c);
                        }
                        self.set_run(w, 0x8cf);
                    }
                    _ => return Next::State(0x1c),
                }
                Next::Tail
            }
            0xc => {
                match run {
                    0x58d => {
                        if rnd(ctx, 20) == 0 {
                            self.set_run_list(w, &[0x5b5, 0x5c6, 0x5d9, 0x622]);
                        } else {
                            if rnd(ctx, 40) == 0 && !self.wins[w].spill {
                                return Next::State(0x1c);
                            }
                            self.set_run(w, 0x58d);
                        }
                    }
                    0x622 => {
                        if !self.wins[w].spill {
                            return Next::State(0x1c);
                        }
                        self.set_run(w, 0x6b2);
                        self.paint_at_pos(w, CLOSED_CELL, false);
                    }
                    0x6b2 => {
                        let nc = col + i32::from(flip);
                        self.set_cell(nc, row - 1, 1);
                        return Next::State(0x1c);
                    }
                    _ => return Next::State(0x1c),
                }
                Next::Tail
            }
            0xd => {
                match run {
                    0x768 => {
                        if rnd(ctx, 2) == 0 {
                            self.set_run(w, 0x76e);
                        } else {
                            self.set_run(w, 0x7a4);
                        }
                        Next::Tail
                    }
                    0x7a4 => {
                        if rnd(ctx, 2) != 0 {
                            self.set_run(w, 0x7c0);
                            return Next::Tail;
                        }
                        Next::State(0x1c)
                    }
                    _ => Next::State(0x1c),
                }
            }
            0xe => {
                let dec = false; // December variants: GAP, no calendar in Ctx
                match run {
                    0xa29 => {
                        self.set_pos_home(w);
                        if rnd(ctx, 5) != 0 {
                            self.set_run(w, 0xa29);
                            return Next::Tail;
                        }
                        if rnd(ctx, 4) != 0 {
                            self.park_and_close(w);
                            return Next::State(0x1c);
                        }
                        if dec && rnd(ctx, 4) == 0 {
                            self.park_and_close(w);
                            return Next::State(0x1c);
                        }
                        if rnd(ctx, 2) == 0 && !(dec && rnd(ctx, 4) == 0) {
                            self.set_run(w, 0xa77);
                        } else {
                            // `+0xCC(0xa65)` then SetRun(0xa39)
                            self.link_to(w, 0xa65);
                            self.set_run(w, 0xa39);
                            let (x, y) = self.wins[w].pos;
                            self.wins[w].pos = (x + if self.coin { 0xe } else { -0xf }, y - 4);
                            // `+0x94(0, g0E6C)` / `+0x98(0, n)`
                            let n = self.traveller_frames(w, 0);
                            self.traveller_place(w, 0, n);
                        }
                        Next::Tail
                    }
                    0xa77 => {
                        self.park_and_close(w);
                        Next::State(0x1c)
                    }
                    0xa39 => {
                        self.set_run_list(w, &[0xa65, 0xa29]);
                        Next::Tail
                    }
                    _ => Next::State(0x1c),
                }
            }
            0xf => {
                match rnd(ctx, 4) {
                    0 | 1 | 2 => self.set_run(w, 0x979),
                    _ => self.set_run(w, 0x993),
                }
                if rnd(ctx, 5) == 0 {
                    return Next::State(0x1c);
                }
                Next::Tail
            }
            0x10 => {
                match run {
                    0x8dc => {
                        if rnd(ctx, 5) == 0 {
                            self.set_run_list(w, &[0x8e2, 0x8f8, 0x905]);
                        } else {
                            self.set_run(w, 0x8dc);
                        }
                    }
                    0x905 => {
                        if rnd(ctx, 2) == 0 {
                            self.set_run_list(w, &[0x912, 0x91b]);
                        } else {
                            self.set_run(w, 0x905);
                        }
                    }
                    _ => return Next::State(0x1c),
                }
                Next::Tail
            }
            0x11 => {
                match run {
                    0x873 | 0x85e | 0x887 | 0x806 => {
                        if rnd(ctx, 40) == 0 {
                            return Next::State(0x1c);
                        }
                        match rnd(ctx, 6) {
                            0 | 1 | 2 => self.set_run(w, 0x806),
                            3 => self.set_run(w, 0x85e),
                            4 => self.set_run(w, 0x887),
                            _ => {
                                if rnd(ctx, 5) == 0 {
                                    self.set_run(w, 0x81d);
                                }
                            }
                        }
                    }
                    0x81d => {
                        if rnd(ctx, 40) == 0 {
                            self.set_run(w, 0x83e);
                        } else {
                            self.set_run(w, 0x831);
                        }
                    }
                    0x831 => {
                        if rnd(ctx, 40) == 0 {
                            self.set_run(w, 0x83e);
                        } else {
                            self.set_run(w, 0x831);
                        }
                        if rnd(ctx, 20) == 0 {
                            self.set_run(w, 0x873);
                        }
                    }
                    _ => return Next::State(0x1c),
                }
                Next::Tail
            }
            0x13 | 0x14 | 0x15 => {
                if !self.wins[w].more {
                    self.wins[w].pos.0 = FAR_PARK_X;
                    self.desc_busy[self.wins[w].desc] = false;
                    return Next::State(0);
                }
                self.sync_home(w);
                // `+0x4C = |(coordinate − sky edge) / step| + 3` with the
                // step `+0xFC(1, run)` of the state's own run (a Point, h in
                // the high word) against the sky rect `g0E6C` {l, t, r, b}:
                // 0x13 x → right over the step's h (@2B7C..2BA4), 0x14 y →
                // bottom over its v (@2DFC..), 0x15 x → left when flipped,
                // else right, over its h (@3084..). Settled 2026-09-29
                // (`voyeur-long-1/2`: every saucer pass edge to edge).
                let (step_run, edge) = match s {
                    0x13 => (0xa47, (SKY.2, 0)),
                    0x14 => (0xa39, (0, SKY.3)),
                    _ => (0x14, (if flip { SKY.0 } else { SKY.2 }, 0)),
                };
                let step = self.run_step(w, step_run);
                let (x, y) = self.wins[w].pos;
                let n = if s == 0x14 { (y - edge.1) / nz(step.1) } else { (x - edge.0) / nz(step.0) };
                let n = n.abs() + 3;
                // `+0x4C = n`, `+0x46 = 0`: the next tick finds the run
                // ended with a repeat left and queues it again; the tick
                // after, it restarts through the linked `fn028A` hand-off.
                self.wins[w].repeat = n;
                if n > 0 {
                    self.wins[w].done = false;
                }
                self.wins[w].more = false;
                Next::Tail
            }
            0x16 => {
                match run {
                    1 => {
                        if rnd(ctx, 2) == 0 {
                            // `+0xCC(0x1b)` first — the saucer is registered
                            // where the 0x1b composite has it relative to the
                            // window — then SetRun(0x14) links onto it
                            // through the shared saucer part
                            self.link_to(w, 0x1b);
                            self.set_run(w, 0x14);
                            let (x, y) = self.wins[w].pos;
                            self.wins[w].pos = (x + if self.coin { -0x12 } else { 0x12 }, y + 2);
                            // `+0x94(0x23, g0E6C)` / `+0x98(0x23, n)`: the
                            // saucer starts where it enters the sky and ends
                            // its flight on the composite's saucer spot
                            let n = self.traveller_frames(w, 0x23);
                            self.traveller_place(w, 0x23, n);
                        } else {
                            self.set_run(w, 1);
                            self.wins[w].deadline += (rnd(ctx, 8000) + 5000) as u64;
                        }
                    }
                    0x14 => self.set_run_list(w, &[0x1b, 0x23, 0x30, 0x6e, 0x71, 0x81]),
                    0x81 => {
                        self.latch = true;
                        self.park_and_close(w);
                        self.set_cell(col, row + 1, 1);
                        return Next::State(0x1c);
                    }
                    _ => {}
                }
                Next::Tail
            }
            0x19 => {
                match run {
                    0x29f => {
                        if rnd(ctx, 3) == 0 && row != 0 && self.cell(col, row - 1) == 0 {
                            self.set_cell(col, row - 1, 2);
                            self.set_run(w, 0xa01);
                            self.wins[w].home.1 += CELL_H / 2 - 1;
                            self.set_pos_home(w);
                            self.wins[w].frame_delay += 0x19;
                            Next::Tail
                        } else {
                            self.idle_19(w, ctx)
                        }
                    }
                    0xe3 => {
                        if rnd(ctx, 30) == 0 && row != 0 && self.cell(col, row - 1) == 0 {
                            self.set_cell(col, row - 1, 2);
                            self.set_run_list(w, &[0x633, 0x64c, 0x66a]);
                            self.wins[w].home.1 += CELL_H / 2 - 1;
                            self.set_pos_home(w);
                            self.wins[w].frame_delay += 0x19;
                            Next::Tail
                        } else {
                            self.idle_19(w, ctx)
                        }
                    }
                    0x66a => {
                        if rnd(ctx, 4) == 0 {
                            self.set_run_list(w, &[0x66a, 0x66e]);
                        } else {
                            self.set_run(w, 0x66a);
                        }
                        Next::Tail
                    }
                    0x66e => {
                        self.wins[w].home.1 += CELL_H / 2;
                        self.set_run(w, 0x684);
                        self.set_pos_home(w);
                        Next::Tail
                    }
                    0x684 => {
                        self.set_cell(col, row - 1, 1);
                        Next::State(0x1c)
                    }
                    0x112 | 0x162 | 0x159 => Next::State(0x1c),
                    0xa01 | 0x72c => {
                        self.set_cell(col, row + 1, 1);
                        Next::State(0x1c)
                    }
                    0xff => {
                        if self.latch && rnd(ctx, 2) == 0 {
                            let side = if flip { 1 } else { -1 };
                            let nc = col + side;
                            if nc >= 0 && nc < self.cols as i32 && self.cell(nc, row) == 0 {
                                self.set_run(w, 0x112);
                                return Next::Tail;
                            }
                        }
                        if row < self.rows as i32 - 1 && self.cell(col, row + 1) == 2 {
                            if rnd(ctx, 2) == 0 {
                                let f = if rnd(ctx, 2) == 0 { 0x698 } else { 0x6cf };
                                self.set_run(w, f);
                                return Next::Tail;
                            }
                        } else if rnd(ctx, 20) == 0 && row < self.rows as i32 - 1 {
                            self.set_cell(col, row + 1, 2);
                            self.set_run_list(w, &[0x6e2, 0x717, 0x72c]);
                            return Next::Tail;
                        }
                        self.idle_19(w, ctx)
                    }
                    _ => self.idle_19(w, ctx),
                }
            }
            0x1a => {
                match run {
                    0xaea => return Next::State(0x1c),
                    0xab1 => {
                        if rnd(ctx, 20) == 0 {
                            self.set_run(w, 0xaea);
                            return Next::Tail;
                        }
                    }
                    _ => {}
                }
                let c = self.wins[w].counter;
                self.wins[w].counter -= 1;
                if c == 0 {
                    return Next::State(0x1c);
                }
                let f = T_1A[self.wins[w].pick].0;
                self.set_run(w, f);
                Next::Tail
            }
            _ => Next::State(0x1c),
        }
    }

    // -----------------------------------------------------------------
    // fn37 / fn38 / fn39

    /// `fn37` @6A80: does the footprint fit at (col,row) on free cells?
    fn fits(&self, d: &Desc, col: i32, row: i32) -> bool {
        if (self.rows as i32) < row + d.rows || (self.cols as i32) < col + d.cols {
            return false;
        }
        for r in row..row + d.rows {
            for c in col..col + d.cols {
                if d.fp[(r - row) as usize][(c - col) as usize] != 0 && self.cell(c, r) > 0 {
                    return false;
                }
            }
        }
        true
    }

    /// `fn38` @6B66: toggle a free cell between dark and lit-empty.
    fn toggle_cell(&mut self, col: i32, row: i32) {
        if self.cell(col, row) >= 2 {
            return;
        }
        if self.cell(col, row) == 0 {
            self.paint_cell(col, row, LIT_EMPTY, false);
            self.set_cell(col, row, 1);
        } else {
            self.paint_cell(col, row, CLOSED_CELL, false);
            self.set_cell(col, row, 0);
        }
    }

    /// `fn39` state 3: the vignette roll, chooser and placement.
    fn relight(&mut self, ctx: &mut Ctx) {
        let n = self.wins.len();
        // free window: fn39 scans g0CEE[1..=n]
        let mut win: i32 = -1;
        for i in 1..=n {
            if !self.busy[i] {
                win = i as i32;
                break;
            }
        }
        let mut pick: i32 = -1;
        let mut tries = 0;
        if win >= 0 {
            let total: i32 = DESCS.iter().map(|d| d.weight as i32).sum();
            // The walk index (`D6`) is zeroed ONCE, before the retry loop
            // (@7008; the retry branch @70E2 goes back to @700C, past it):
            // a retry walks on from the previous pick, not from record 0.
            // Past record 25 the walk reads on through the 0x44-byte stride
            // into the pick tables behind the descriptors (`g0BA2`..,
            // static DATA): "records" 26/27 weigh 20/2500 and their rows
            // (20/5000) never fit, so they are always refused (`OFF_TABLE`).
            let weight = |k: usize| if k < DESCS.len() { DESCS[k].weight as i32 } else { OFF_TABLE[k - DESCS.len()] };
            let mut i = 0usize;
            loop {
                let mut r = rnd(ctx, total);
                while r > 0 {
                    r -= weight(i);
                    i += 1;
                }
                i = i.saturating_sub(1); // GAP: r == 0 from record 0 walks to −1 in the C
                if i < DESCS.len() {
                    let d = &DESCS[i];
                    if !self.desc_busy[i] && d.cols <= self.cols as i32 && d.rows <= self.rows as i32 {
                        pick = i as i32;
                    }
                }
                if tries > CHOOSER_RETRIES {
                    pick = 1;
                }
                tries += 1;
                if pick != -1 {
                    break;
                }
            }
        }
        if tries > CHOOSER_RETRIES {
            pick = -1;
        }
        if pick >= 0 && DESCS[pick as usize].state == 0x16 && self.latch {
            pick = -1;
        }
        if win >= 0 && rnd(ctx, 20) == 0 {
            // December → descriptor 17 (state 0x13): GAP, no calendar in Ctx
        }
        if pick == -1 || win == -1 || pick >= 26 {
            return;
        }
        let d = &DESCS[pick as usize];
        let mut ok = true;
        let (mut col, mut row) = (-1, -1);
        if d.placed {
            col = rnd(ctx, self.cols as i32 - (d.cols - 1));
            row = rnd(ctx, self.rows as i32 - (d.rows - 1));
            if d.pin_x {
                col = if !self.coin { self.cols as i32 - d.cols } else { 0 };
            }
            if d.pin_top {
                row = self.rows as i32 - d.rows;
            }
            let (start_col, start_row) = (col, row);
            match d.state {
                0x12 => {
                    // February only: GAP, no calendar in Ctx
                    ok = false;
                }
                0xd => {
                    col = if !self.coin { self.cols as i32 - d.cols } else { 0 };
                    row = self.rows as i32 - 2;
                    ok = self.fits(d, col, row);
                }
                0xe => {
                    col = if !self.coin { self.cols as i32 - 2 } else { 1 };
                    row = self.rows as i32 - 2;
                    ok = self.fits(d, col, row);
                }
                3 => {
                    col = if !self.coin { self.cols as i32 - d.cols } else { 0 };
                    row = rnd(ctx, 2);
                    ok = self.fits(d, col, row);
                }
                _ => {
                    let (mut sc, mut sr) = (start_col, start_row);
                    loop {
                        if self.fits(d, col, row) {
                            break;
                        }
                        col += 1;
                        if self.cols as i32 - d.cols < col || d.pin_x {
                            col = 0;
                            if d.pin_x {
                                col = if !self.coin { self.cols as i32 - d.cols } else { 0 };
                                sc = col;
                            }
                            row += 1;
                            if self.rows as i32 - d.rows < row {
                                row = 0;
                                if d.pin_top {
                                    row = self.rows as i32 - d.rows;
                                    sr = row;
                                }
                            }
                        }
                        if col == sc && row == sr {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if d.state == 0x16
                && (row + 1 == self.rows as i32 || (row < self.rows as i32 - 1 && self.cell(col, row + 1) != 0))
            {
                ok = false;
            }
            if matches!(d.state, 0x17 | 5 | 0x18) && !self.latch {
                ok = false;
            }
        }
        if !ok {
            return;
        }
        let w = win as usize;
        if d.placed {
            for r in row..row + d.rows {
                for c in col..col + d.cols {
                    if d.fp[(r - row) as usize][(c - col) as usize] != 0 {
                        self.set_cell(c, r, 2);
                        self.owner[r as usize][c as usize] = w as u8;
                    }
                }
            }
            if d.state == 0x16 {
                self.set_cell(col, row + 1, 2);
            }
        }
        self.busy[w] = true;
        self.desc_busy[pick as usize] = true;
        self.go[w] = Some((pick as usize, col, row));
        #[cfg(test)]
        self.log.push(Ev::Start { desc: pick as usize });
    }

    /// `fn27`'s deadline arm and `fn39` states 0/2: the ctor state, the
    /// recast and the scene build. They read the clock, which is the same
    /// for every DoDrawFrame of one tick, so they run once per tick.
    fn begin_tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        if !self.inited {
            self.inited = true;
            let pick = ctx.rng.pct(TAGLINE_PICKS) as u16;
            self.tagline =
                self.pack.strings(STR_TAGLINE_FIRST + pick).first().cloned().unwrap_or_default();
            self.scene_deadline = now + SCENE_RESET_MS;
            self.ctrl_state = 0;
        }
        if now > self.scene_deadline {
            self.recast(now);
        }
        if self.ctrl_state == 0 {
            for b in &mut self.busy {
                *b = false;
            }
            self.build_scene(ctx);
            self.ctrl_state = 3; // states 0 → 2 → 3 with a zero deadline
        }
    }

    /// One DoDrawFrame's pass through `fn39` state 3 (@6ECA..@78C0).
    fn draw_frame(&mut self, ctx: &mut Ctx) {
        // 1. the cell toggle roll
        if rnd(ctx, CELL_ROLL as i32) == 0 {
            let col = rnd(ctx, self.cols as i32);
            let row = rnd(ctx, self.rows as i32);
            let go = self.cell(col, row) == 1 || rnd(ctx, 10) == 0;
            if go {
                self.toggle_cell(col, row);
            }
        }
        // 2. the relight roll
        let busy_count = (1..=self.wins.len()).filter(|&i| self.busy[i]).count();
        if busy_count < self.wins.len() && rnd(ctx, RELIGHT_ROLL as i32) == 0 {
            self.relight(ctx);
        }
        // 3. run every window
        for w in 0..self.wins.len() {
            self.update(w, ctx);
        }
        // ambient siren/horn
        if rnd(ctx, AMBIENT_ROLL as i32) == 0 {
            let id = if rnd(ctx, 2) == 0 { 5 } else { 6 };
            self.play(ctx, id);
        }
    }

    /// `fn36`: the scene build (the drawing parts are `sprites()`).
    fn build_scene(&mut self, ctx: &mut Ctx) {
        self.roll_layout(ctx);
        self.roll_stars(ctx);
        self.grid = [[0; 6]; 6];
        self.owner = [[0; 6]; 6];
        self.paints.clear();
    }

    /// `fn27`'s recast arm + `fn25`: everything back to the ctor state.
    fn recast(&mut self, now: u64) {
        self.wins = (1..=WINDOW_COUNT).map(Win::new).collect();
        self.busy = [false; 6];
        self.go = [None; 6];
        self.desc_busy = [false; 26];
        self.latch = false;
        self.scene_deadline = now + SCENE_RESET_MS;
        self.ctrl_state = 0;
    }
}

impl Module for Voyeur {
    fn name(&self) -> &'static str {
        "Voyeur"
    }

    /// No controls (its TEXT 1000 help says as much).
    fn controls(&self) -> Vec<ControlDef> {
        Vec::new()
    }

    fn set_control(&mut self, _index: usize, _value: i32) {}

    /// `fn27` @569E, then `fn39`'s state machine.
    fn tick(&mut self, ctx: &mut Ctx) {
        self.begin_tick(ctx);
        for _ in 0..DRAW_FRAMES_PER_TICK {
            self.draw_frame(ctx);
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // sky, stars, moon, sky-side furniture, fire escape, wall, roof
        if let Some(f) = self.pack.frame(BASE_OVERLAY, self.sky_frame) {
            let mut x = 0;
            while x < SCREEN_W {
                out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x, y: 0 });
                x += SKY_TILE_W;
            }
        }
        for star in &self.stars {
            let png = self.star_png[star.color as usize].clone();
            out.push(SpriteDraw { flip: false, pal: 0, png, x: star.x, y: star.y });
        }
        if let (Some((mx, my)), Some(f)) = (self.moon, self.pack.frame(BASE_BUILDING, MOON_FRAME)) {
            out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x: mx, y: my });
        }
        for &(series, fno, dx, y) in SKY_SIDE {
            if let Some(f) = self.pack.frame(series, fno) {
                let x = self.sky_side_x(dx, f.w.max(1));
                out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x, y });
            }
        }
        if let Some(f) = self.pack.frame(BASE_BUILDING, ESCAPE_FRAME) {
            let x = self.sky_side_x(0, f.w.max(1));
            for k in -1..=self.rows as i32 {
                let y = self.wall_y + ESCAPE_DY + CELL_H * k;
                out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x, y });
            }
        }
        if let Some(png) = &self.wall_png {
            out.push(SpriteDraw { flip: false, pal: 0, png: png.clone(), x: self.wall_x, y: self.wall_y });
        }
        for &(series, fno, dx, h) in ROOF {
            if let Some(f) = self.pack.frame(series, fno) {
                out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x: self.wall_x + dx, y: self.wall_y - h });
            }
        }
        // the backdrop paints, in paint order
        for p in &self.paints {
            let series = if p.frame == CLOSED_CELL { BASE_WALL } else { BASE };
            if let Some(f) = self.pack.frame(series, p.frame as u32) {
                out.push(SpriteDraw { flip: p.flip, pal: 0, png: f.png.clone(), x: p.x, y: p.y });
            }
        }
        // the live windows: the current frame centred on pos
        for w in &self.wins {
            if w.state == 0 || w.pos.0 >= PARK_X {
                continue;
            }
            let Some(f) = self.pack.frame(BASE, w.cur as u32) else { continue };
            let (fw, fh) = (f.w.max(1), f.h.max(1));
            out.push(SpriteDraw { flip: w.flip, pal: 0, png: f.png.clone(), x: w.pos.0 - fw / 2, y: w.pos.1 - fh / 2 });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The ms clock moves per Mac tick, with `DRAW_FRAMES_PER_TICK`
    /// DoDrawFrames on each (see "Timing"); the windows self-pace in ms.
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    /// The module's two pieces of derived art, rebuilt from their names: the
    /// 1x1 stars ([`star_image`]) and the cropped wall ([`wall_image`]).
    /// Both used to be PNGs this module wrote into the pack.
    fn generated(&self, name: &str) -> Option<Image> {
        let body = name.strip_prefix(engine::GEN_PREFIX)?;
        if body.starts_with("star/") {
            star_image(name)
        } else if body.starts_with("wall/") {
            wall_image(&self.pack, name)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};

    fn load() -> Option<Voyeur> {
        let pack = Pack::load(std::path::Path::new("../assets/voyeur")).ok()?;
        Some(build(pack).expect("build() returned None with series 1000 present"))
    }

    fn ctx_seeded(seed: u64) -> Ctx {
        Ctx {
            rng: RandomLong::new(seed),
            rng15: Random15::new(seed as u32 | 1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    /// Diagnostic (ignored): open-window episode stats at 4 fps over 450 s,
    /// same measure as the voyeur-long-1 golden (median 0.5 s, p90 2.75 s,
    /// 30 episodes ≥ 10 s, 52 % of cell-time lit; 4×3 grid).
    #[test]
    #[ignore]
    fn open_window_episodes() {
        for seed in [1u64, 3, 5] {
            let Some(mut m) = load() else { return };
            let mut c = ctx_seeded(seed);
            let mut pace = Pacer::new(&m);
            let mut seq: Vec<Vec<bool>> = Vec::new();
            let mut next = 0u64;
            while c.now_ms < 450_000 {
                pace.advance(&mut c);
                m.tick(&mut c);
                c.sounds.clear();
                if c.now_ms >= next {
                    next += 250;
                    let mut v = Vec::new();
                    for r in 0..m.rows as i32 {
                        for col in 0..m.cols as i32 {
                            v.push(m.cell(col, r) > 0);
                        }
                    }
                    seq.push(v);
                }
            }
            let cells = seq[0].len();
            let mut segs: Vec<f64> = Vec::new();
            let mut lit = 0usize;
            for k in 0..cells {
                let (mut cur, mut n, mut first) = (seq[0][k], 0usize, true);
                for row in &seq {
                    lit += row[k] as usize;
                    if row[k] == cur { n += 1 } else {
                        if !first && cur { segs.push(n as f64 / 4.0) }
                        first = false; cur = row[k]; n = 1;
                    }
                }
            }
            segs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |p: f64| segs[((segs.len() as f64) * p) as usize];
            println!("seed {seed} grid {}x{}: open episodes {} median {} p75 {} p90 {} max {} >=10s {} lit {:.2}",
                m.cols, m.rows, segs.len(), q(0.5), q(0.75), q(0.9), segs.last().unwrap(),
                segs.iter().filter(|&&s| s >= 10.0).count(), lit as f64 / (cells * seq.len()) as f64);
        }
    }

    #[test]
    fn voyeur_smoke() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(1);
        let mut pace = Pacer::new(&m);
        let mut out = Vec::new();
        let mut drew = false;
        for _ in 0..4000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            out.clear();
            m.sprites(&mut out);
            if !out.is_empty() {
                drew = true;
            }
        }
        assert!(drew, "no sprites over 4000 ticks");
    }

    #[test]
    fn sprites_draw_the_building_facade_before_any_window_lights() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(0x5EED_CAFE);
        m.tick(&mut ctx);
        let mut out = Vec::new();
        m.sprites(&mut out);
        let sky_tiles = (SCREEN_W / SKY_TILE_W) as usize;
        let escapes = m.rows as usize + 2;
        let want = sky_tiles
            + m.stars.len()
            + usize::from(m.moon.is_some())
            + SKY_SIDE.len()
            + escapes
            + 1
            + ROOF.len()
            + m.paints.len();
        assert_eq!(out.len(), want, "facade piece count");
        assert!(out[..sky_tiles].iter().all(|s| s.y == 0));
        let wall = m.wall_png.clone().expect("a wall sprite must exist");
        assert!(
            out.iter().any(|s| s.png == wall && (s.x, s.y) == (m.wall_x, m.wall_y)),
            "the building wall must be blitted at ({},{})",
            m.wall_x,
            m.wall_y
        );
    }

    #[test]
    fn building_rect_is_rolled_and_bottom_flush() {
        let Some(_) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        assert_eq!(CELL_W * MAX_COLS as i32, WALL_W);
        assert_eq!(CELL_H * MAX_ROWS as i32, 430, "the packed wall art is 5 rows");
        let mut seen_rows = std::collections::HashSet::new();
        let mut seen_x = std::collections::HashSet::new();
        for seed in 1..60u64 {
            let mut m = load().expect("pack was loadable a moment ago");
            let mut ctx = ctx_seeded(seed * 7919);
            m.tick(&mut ctx);
            assert_eq!(m.cols, 4, "640 > 480 forces g0D0E = 4");
            assert!((3..=5).contains(&m.rows), "rows {} outside 3..=5", m.rows);
            assert_eq!(m.wall_y + m.rows as i32 * CELL_H, SCREEN_H);
            assert!(m.wall_x == 0 || m.wall_x == SCREEN_W - WALL_W);
            seen_rows.insert(m.rows);
            seen_x.insert(m.wall_x);
        }
        assert_eq!(seen_rows, [3, 4, 5].into_iter().collect(), "all three row counts");
        assert_eq!(seen_x.len(), 2, "both anchors must occur");
    }

    #[test]
    fn sky_gradient_variant_is_rolled_per_scene() {
        let Some(_) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut seen = std::collections::HashSet::new();
        for seed in 1..60u64 {
            let mut m = load().expect("pack was loadable a moment ago");
            let mut ctx = ctx_seeded(seed * 104_729);
            m.tick(&mut ctx);
            assert!(m.sky_frame < SKY_VARIANTS);
            let f = m.pack.frame(BASE_OVERLAY, m.sky_frame).expect("every rolled sky variant must be packed");
            assert_eq!(f.w, SKY_TILE_W, "the sky strip is 8 px wide");
            let mut out = Vec::new();
            m.sprites(&mut out);
            assert!(out.iter().take(4).all(|s| s.png == f.png));
            seen.insert(m.sky_frame);
        }
        assert_eq!(seen, (0..SKY_VARIANTS).collect(), "all three variants must occur");
    }

    #[test]
    fn roof_props_sit_on_the_roofline_of_whatever_rect_was_rolled() {
        let Some(_) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        for &(series, fno, _dx, h) in ROOF {
            let m = load().expect("pack was loadable a moment ago");
            let f = m.pack.frame(series, fno).expect("roof prop must be packed");
            assert_eq!(f.h, h, "ROOF height for {series}/{fno} must match the art");
        }
        for seed in [3u64, 11, 29, 97, 1_009] {
            let mut m = load().expect("pack was loadable a moment ago");
            let mut ctx = ctx_seeded(seed);
            m.tick(&mut ctx);
            let mut out = Vec::new();
            m.sprites(&mut out);
            for &(series, fno, dx, h) in ROOF {
                let png = m.pack.frame(series, fno).unwrap().png.clone();
                let got = out.iter().find(|s| s.png == png).expect("prop drawn");
                assert_eq!((got.x, got.y + h), (m.wall_x + dx, m.wall_y));
            }
        }
    }

    #[test]
    fn sky_side_furniture_mirrors_with_the_anchor() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let esc_w = m.pack.frame(BASE_BUILDING, ESCAPE_FRAME).unwrap().w;
        assert_eq!(esc_w, 48);
        m.coin = false;
        m.wall_x = 0;
        assert_eq!(m.sky_side_x(0, esc_w), WALL_W);
        m.coin = true;
        m.wall_x = SCREEN_W - WALL_W;
        assert_eq!(m.sky_side_x(0, esc_w), SCREEN_W - WALL_W - esc_w);
    }

    #[test]
    fn starfield_matches_the_disasm_loop() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(42);
        m.tick(&mut ctx);
        assert!((200..400).contains(&m.stars.len()));
        let y_bound = (STAR_RECT_BOTTOM - STAR_RECT_TOP) / 3;
        for s in &m.stars {
            assert!(s.x >= STAR_RECT_LEFT && s.x < STAR_RECT_RIGHT);
            assert!(s.y >= STAR_RECT_TOP && s.y < STAR_RECT_TOP + y_bound);
            assert!((0..3).contains(&s.color));
        }
        for (i, want) in STAR_COLORS.iter().enumerate() {
            let img = m.generated(&m.star_png[i]).expect("star pixels");
            assert_eq!((img.w, img.h), (1, 1));
            assert_eq!(&img.rgba[0..3], want.as_slice());
            assert_eq!(img.rgba[3], 0xFF);
        }
    }

    /// Every file under a directory, with its size and mtime — enough to
    /// catch a module writing into the pack.
    fn tree_stat(dir: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, (u64, std::time::SystemTime)> {
        let mut out = std::collections::BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                match e.file_type() {
                    Ok(t) if t.is_dir() => stack.push(p),
                    Ok(_) => {
                        if let Ok(md) = e.metadata() {
                            out.insert(p, (md.len(), md.modified().unwrap_or(std::time::UNIX_EPOCH)));
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        out
    }

    /// THE read-only-bundle property (2026-09-19). The stars and the cropped
    /// wall are rebuilt from their names by `Module::generated`, and running
    /// the module never writes into the pack. Before this commit
    /// `ensure_star_png` / `ensure_wall_png` wrote
    /// `compounds/2000/star_N.png` and `compounds/1007/wall_rN.png` into it;
    /// inside an installed `.saver` the bundle is read-only, both writes
    /// failed, and the scene lost its whole starfield and its building wall.
    #[test]
    fn stars_and_wall_are_generated_and_the_pack_is_never_written() {
        let Some(mut m) = load() else {
            eprintln!("assets/voyeur missing — skipping");
            return;
        };
        let root = m.pack.root().to_path_buf();
        let before = tree_stat(&root);
        let mut ctx = ctx_seeded(42);
        m.tick(&mut ctx);

        for (i, want) in STAR_COLORS.iter().enumerate() {
            let name = &m.star_png[i];
            assert!(engine::is_generated(name), "{name}");
            let img = m.generated(name).expect("star pixels");
            assert_eq!((img.w, img.h), (1, 1), "{name}");
            assert_eq!(img.rgba, vec![want[0], want[1], want[2], 0xFF], "{name}");
        }

        // the full-height wall is packed art; a short scene crops it, and the
        // crop is bottom-flush — the last row of the crop is the last row of
        // the art.
        let full = m.pack.frame(BASE_WALL, WALL_FRAME).expect("wall art").png.clone();
        assert_eq!(wall_name(&m.pack, WALL_ART_ROWS), Some(full.clone()));
        let art = m.pack.image(&full);
        for rows in 1..WALL_ART_ROWS {
            let name = wall_name(&m.pack, rows).expect("a wall name");
            assert!(engine::is_generated(&name), "{name}");
            let img = m.generated(&name).unwrap_or_else(|| panic!("no pixels for {name}"));
            assert_eq!((img.w, img.h), (art.w, rows * CELL_H as u32), "{name}");
            let tail = art.rgba.len() - img.rgba.len();
            assert_eq!(img.rgba, art.rgba[tail..], "{name} is not the BOTTOM of the wall");
        }

        // and the sprite the scene actually emits resolves
        let mut out = Vec::new();
        m.sprites(&mut out);
        for s in out.iter().filter(|s| engine::is_generated(&s.png)) {
            assert!(m.generated(&s.png).is_some(), "unresolvable sprite {}", s.png);
        }

        assert_eq!(before, tree_stat(&root), "the module wrote into the pack");
    }

    #[test]
    fn starfield_is_static_across_ticks() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(7);
        let mut pace = Pacer::new(&m);
        pace.advance(&mut ctx);
        m.tick(&mut ctx);
        let first: Vec<(i32, i32, u8)> = m.stars.iter().map(|s| (s.x, s.y, s.color)).collect();
        for _ in 1..500 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        let later: Vec<(i32, i32, u8)> = m.stars.iter().map(|s| (s.x, s.y, s.color)).collect();
        assert_eq!(first, later, "starfield must stay static within a scene");
    }

    /// The relight roll is 1/500 per Mac tick with no startup wait: over
    /// two minutes the building lights several distinct cells, and every
    /// placement lands on cells that were free.
    #[test]
    fn windows_light_within_the_first_minute_and_only_on_free_cells() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(1);
        let mut pace = Pacer::new(&m);
        let mut first_light = None;
        let mut cells = std::collections::HashSet::new();
        let ticks = 120_000 / 16;
        for _ in 0..ticks {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let mut active = std::collections::HashSet::new();
            for w in &m.wins {
                if w.state != 0 && w.state != 0x1c && DESCS[w.desc].placed {
                    cells.insert((w.col, w.row));
                    assert!(active.insert((w.col, w.row)), "two vignettes on cell ({},{})", w.col, w.row);
                    assert!(m.cell(w.col, w.row) == 2, "a running vignette's cell must be owned");
                }
            }
            if first_light.is_none() && m.wins.iter().any(|w| w.state != 0) {
                first_light = Some(pace.now_ms());
            }
        }
        let t = first_light.expect("nothing lit in two minutes");
        assert!(t < 60_000, "first light at {t} ms");
        assert!(cells.len() >= 3, "only {} distinct cells lit in two minutes", cells.len());
    }

    /// State 0x1c: the window frees itself, the descriptor's busy byte
    /// clears, its cells become lit-free (1) and the sprite parks at x=1000.
    #[test]
    fn a_finished_vignette_parks_and_marks_its_cells_lit_free() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(3);
        let mut pace = Pacer::new(&m);
        pace.advance(&mut ctx);
        m.tick(&mut ctx);
        // hand window 1 descriptor 0 (state 2, a 1x3 strip) at cell (0,0)
        let d = 0usize;
        for c in 0..3 {
            m.set_cell(c, 0, 2);
            m.owner[0][c as usize] = 1;
        }
        m.busy[1] = true;
        m.desc_busy[d] = true;
        m.go[1] = Some((d, 0, 0));
        let mut saw_lit = false;
        let mut finished = false;
        for _ in 0..6000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let w = &m.wins[0];
            if w.state == 2 {
                saw_lit = true;
                assert!(w.pos.0 < PARK_X, "a running vignette must be on screen");
            }
            if saw_lit && w.state == 0 {
                finished = true;
                break;
            }
        }
        assert!(saw_lit, "descriptor 0 never entered state 2");
        assert!(finished, "the vignette never returned to state 0");
        assert!(!m.busy[1] && !m.desc_busy[d], "window/descriptor must be free again");
        assert!(m.wins[0].pos.0 >= PARK_X, "the sprite must be parked off-screen");
        assert!((0..3).all(|c| m.cell(c, 0) == 1), "the footprint must read lit-free");
    }

    /// `fn44`: one sound per `g0E40` stamp (per tick) across the module.
    #[test]
    fn at_most_one_sound_per_tick() {
        let Some(mut m) = load() else {
            eprintln!("voyeur pack missing — skipping");
            return;
        };
        let mut ctx = ctx_seeded(5);
        ctx.now_ms = 16;
        m.play(&mut ctx, 2);
        m.play(&mut ctx, 3);
        assert_eq!(ctx.sounds, vec![SND_BASE + 2]);
        ctx.now_ms = 33;
        m.play(&mut ctx, 3);
        assert_eq!(ctx.sounds, vec![SND_BASE + 2, SND_BASE + 3]);
    }

    /// Diagnostic: one window's state/run/pos changes over the first three
    /// minutes. `cargo test -p app --lib trace_voyeur -- --ignored --nocapture`
    #[ignore]
    #[test]
    fn trace_voyeur() {
        let Some(mut m) = load() else { return };
        let mut ctx = ctx_seeded(1);
        let mut pace = Pacer::new(&m);
        let mut prev: Vec<(u16, i32, i32, i32)> = vec![(0, 0, 0, 0); WINDOW_COUNT];
        let mut lines = 0;
        for _ in 0..(180_000 / 16) {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            let t = pace.now_ms();
            for (i, w) in m.wins.iter().enumerate() {
                let cur = (w.state, w.run_first, w.pos.0, w.pos.1);
                if cur != prev[i] {
                    println!(
                        "t={t:6} win{} st={:#x} run={:#x} cur={:#x} pos=({},{}) cell=({},{}) desc={} busy={:?} grid={:?}",
                        w.idx, w.state, w.run_first, w.cur, w.pos.0, w.pos.1, w.col, w.row, w.desc, &m.busy[1..=WINDOW_COUNT],
                        (0..m.rows as usize).map(|r| m.grid[r][..m.cols as usize].to_vec()).collect::<Vec<_>>()
                    );
                    lines += 1;
                    prev[i] = cur;
                }
            }
            if lines > 600 {
                break;
            }
        }
    }

    /// Hand-off census (per-tick trace, `--ignored --nocapture`): every run
    /// hand-off the windows make, the delta and flip the port applied, and
    /// what the linked `fn028A` → `fn3F2E` model gives from the same start.
    /// "placed" = an absolute SetPos follows in the same tick (moot for
    /// position; the flip still carries).
    #[ignore]
    #[test]
    fn trace_voyeur_handoffs() {
        let Some(_) = load() else { return };
        let mut pairs: std::collections::BTreeMap<(i32, i32), (usize, (i32, i32), (i32, i32), bool, bool)> = Default::default();
        let mut rows = 0;
        let mut parked_flip = 0;
        let mut parked_pairs: std::collections::BTreeMap<(i32, i32), usize> = Default::default();
        let (mut n, mut placed_n, mut diff_n, mut flip_model, mut flip_port, mut max_diff) = (0, 0, 0, 0, 0, 0);
        for seed in 1..=8u64 {
            let mut m = load().unwrap();
            let mut ctx = ctx_seeded(seed);
            let mut pace = Pacer::new(&m);
            while pace.now_ms() < 20 * 60_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                let evs = std::mem::take(&mut m.log);
                for (k, e) in evs.iter().enumerate() {
                    let Ev::Handoff { w, state, from, to, p0, p1, flip0, flip1, model } = e.clone() else { continue };
                    if from > 0 && p0.0 >= PARK_X && model.2 != flip0 {
                        parked_flip += 1;
                        *parked_pairs.entry((from, to)).or_insert(0) += 1;
                    }
                    if from <= 0 || p0.0 >= PARK_X {
                        continue;
                    }
                    n += 1;
                    if seed == 1 && rows < 120 {
                        rows += 1;
                        println!(
                            "t={:7} win{} st={state:#x} {from:#x}->{to:#x} pos ({},{})->({},{}) d=({},{}) flip {flip0}->{flip1} shared={:?}",
                            pace.now_ms(), w + 1, p0.0, p0.1, p1.0, p1.1, p1.0 - p0.0, p1.1 - p0.1, model.3
                        );
                    }
                    let placed = evs[k + 1..].iter().any(|e2| matches!(e2, Ev::Place { w: w2 } if *w2 == w));
                    let port = (p1.0 - p0.0, p1.1 - p0.1);
                    let mdl = (model.0, model.1);
                    if model.2 != flip0 {
                        flip_model += 1;
                    }
                    if flip1 != flip0 {
                        flip_port += 1;
                    }
                    if placed {
                        placed_n += 1;
                    } else if port != mdl {
                        diff_n += 1;
                        max_diff = max_diff.max((port.0 - mdl.0).abs().max((port.1 - mdl.1).abs()));
                    }
                    let ent = pairs.entry((from, to)).or_insert((0, port, mdl, placed, model.2 != flip0));
                    ent.0 += 1;
                }
            }
        }
        println!("parked-start hand-offs whose model toggles the flip: {parked_flip} {parked_pairs:x?}");
        println!("handoffs={n} placed={placed_n} unplaced_pos_diff={diff_n} max_diff={max_diff} model_flip_toggles={flip_model} port_flip_toggles={flip_port}");
        for ((from, to), (c, port, mdl, placed, fl)) in &pairs {
            if port != mdl || *fl {
                println!("{from:#x}->{to:#x} x{c} port={port:?} model={mdl:?} placed={placed} flip={fl}");
            }
        }
    }

    /// Run the scene build, then hand window 1 descriptor `d` at `(col,row)`
    /// with the building anchor `coin`.
    fn start_vignette(seed: u64, coin: bool, d: usize, col: i32, row: i32) -> Option<(Voyeur, Ctx, Pacer)> {
        let mut m = load()?;
        let mut ctx = ctx_seeded(seed);
        let mut pace = Pacer::new(&m);
        pace.advance(&mut ctx);
        // the scene only: a full tick's DoDrawFrames may already have
        // handed window 1 a vignette of its own
        m.begin_tick(&mut ctx);
        m.coin = coin;
        m.wall_x = if coin { SCREEN_W - WALL_W } else { 0 };
        m.busy[1] = true;
        m.desc_busy[d] = true;
        m.go[1] = Some((d, col, row));
        Some((m, ctx, pace))
    }

    /// Every drawn step of window 1 while it is in `state`: (from, to, dx, dy, flip).
    fn drawn_steps(m: &mut Voyeur, ctx: &mut Ctx, pace: &mut Pacer, state: u16, ms: u64) -> Vec<(i32, i32, i32, i32, bool)> {
        let mut out = Vec::new();
        let mut prev: Option<(i32, (i32, i32))> = None;
        let end = pace.now_ms() + ms;
        while pace.now_ms() < end {
            pace.advance(ctx);
            m.tick(ctx);
            m.log.clear();
            let w = &m.wins[0];
            if w.state != state || w.pos.0 >= PARK_X {
                if prev.is_some() && w.state != state {
                    break;
                }
                continue;
            }
            if let Some((f, p)) = prev {
                if (f, p) != (w.cur, w.pos) {
                    out.push((f, w.cur, w.pos.0 - p.0, w.pos.1 - p.1, w.flip));
                }
            }
            prev = Some((w.cur, w.pos));
        }
        out
    }

    /// RATCHET — the saucer's repeat wrap is a linked hand-off. Run 0x14
    /// (art 754, frames 20..25, link marker 19) is the saucer the golden
    /// capture shows (`voyeur-av.mp4` f301..f327, template-matched): EVERY
    /// drawn step moves it 18 px in x, the wrap 25 → 20 included, and the
    /// y bob repeats with period 6 (−2, −1, +2, +1, +2, −2 — the wrap's −2
    /// is the `fn028A` marker link 19 → 20). The old port's "no-delta wrap"
    /// froze it for one step per cycle (dx 0, dy 0). State 0x15 repeats the
    /// run as a free flyer.
    #[test]
    fn the_saucer_moves_18px_every_step_including_the_wrap() {
        for (seed, coin) in [(5u64, false), (6, true)] {
            let Some((mut m, mut ctx, mut pace)) = start_vignette(seed, coin, 19, -1, -1) else { return };
            let steps = drawn_steps(&mut m, &mut ctx, &mut pace, 0x15, 60_000);
            assert!(steps.len() >= 12, "state 0x15 drew only {} steps", steps.len());
            let wraps: Vec<_> = steps.iter().filter(|s| s.0 == 0x19 && s.1 == 0x14).collect();
            assert!(!wraps.is_empty(), "no repeat wrap seen: {steps:?}");
            for &(from, to, dx, dy, flip) in &steps {
                let sign = if flip { -1 } else { 1 };
                assert_eq!(dx, 18 * sign, "{from:#x} -> {to:#x} moved {dx} px (capture: 18 every step)");
                let want_dy = match (from, to) {
                    (0x19, 0x14) => -2,
                    (0x14, 0x15) => -2,
                    (0x15, 0x16) => -1,
                    (0x16, 0x17) => 2,
                    (0x17, 0x18) => 1,
                    (0x18, 0x19) => 2,
                    _ => panic!("unexpected step {from:#x} -> {to:#x}"),
                };
                assert_eq!(dy, want_dy, "{from:#x} -> {to:#x} dy (capture bob)");
            }
        }
    }

    /// RATCHET — state 0x16's saucer (the one the golden capture shows).
    /// `voyeur-av.mp4`, building anchored LEFT (`g0E49` = 0): the saucer is
    /// drawn MIRRORED (template score 0.9 vs unmirrored), comes in from the
    /// RIGHT screen edge and flies LEFT at 18 px per step (f301..f327, ten
    /// steps, none 0) to the cat's window, where the abduction composite
    /// takes over. In the C that is the window flip `!g0E49` (@0B76..0C16),
    /// `+0xCC(0x1b)` + SetRun(0x14) registering the saucer on the composite,
    /// then `+0x94`/`+0x98` (`fn05D4`/`fn0718`) backing it up to where it
    /// enters the sky with a repeat count that flies it exactly back — so
    /// the composite's window lands on the cat's. The old port had
    /// `flip = g0E49` (flew right), no traveller setup (one cycle from the
    /// window), and frame-centre links.
    #[test]
    fn state_16_saucer_enters_from_the_far_edge_and_lands_on_the_window() {
        for (seed, coin) in [(7u64, false), (8, true)] {
            let Some((mut m, mut ctx, mut pace)) = start_vignette(seed, coin, 20, 3, 1) else { return };
            let heading = if coin { 1 } else { -1 };
            let win_rect = |m: &Voyeur, f: i32, pos: (i32, i32), flip: bool| -> Vec<(i32, i32)> {
                let g = m.pack.frame(BASE, f as u32).unwrap();
                g.parts
                    .iter()
                    .filter(|q| q[0] == 784)
                    .map(|q| {
                        let r = part_rel(&m.pack, f, q, flip).unwrap();
                        (pos.0 + r[0], pos.1 + r[1])
                    })
                    .collect()
            };
            let mut cat_window = None;
            let mut saucer: Vec<(i32, (i32, i32))> = Vec::new();
            let mut landed = None;
            let end = pace.now_ms() + 20 * 60_000;
            while pace.now_ms() < end {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                m.log.clear();
                let w = &m.wins[0];
                if w.state != 0x16 {
                    continue;
                }
                assert_eq!(w.flip, !coin, "state 0x16 flip must be !g0E49");
                if w.run_first == 1 && w.cur >= 1 {
                    cat_window = win_rect(&m, w.cur, w.pos, w.flip).first().copied();
                } else if (0x14..=0x19).contains(&w.cur) {
                    if saucer.last().map_or(true, |s| *s != (w.cur, w.pos)) {
                        saucer.push((w.cur, w.pos));
                    }
                } else if w.cur == 0x1b && !saucer.is_empty() {
                    landed = Some(win_rect(&m, w.cur, w.pos, w.flip));
                    break;
                }
            }
            let cat = cat_window.expect("the cat never sat in the window");
            assert!(saucer.len() > 6, "seed {seed}: the saucer flew only {} frames", saucer.len());
            // enters from the far edge: `fn0718` seats it on the last frame
            // still outside the sky (held one tick, unseen); the next one is
            // the first to meet the screen
            let g = m.pack.frame(BASE, 0x14).unwrap();
            let on = |x: i32| x - g.w / 2 < SCREEN_W && x - g.w / 2 + g.w > 0;
            let first = saucer.iter().position(|s| on(s.1 .0)).expect("the saucer never came on screen");
            let l = saucer[first].1 .0 - g.w / 2;
            assert!(!on(saucer[first].1 .0 - 18 * heading), "seed {seed}: saucer appeared at x {l}, not at the far edge");
            assert_eq!(heading < 0, l + g.w > SCREEN_W / 2, "seed {seed}: saucer entered on the anchor's side");
            // 18 px every step along the heading
            for pair in saucer.windows(2) {
                assert_eq!(pair[1].1 .0 - pair[0].1 .0, 18 * heading, "saucer step {pair:?}");
            }
            // and the abduction composite lands on the cat's window
            let landed = landed.expect("the saucer never reached the window");
            assert!(landed.contains(&cat), "seed {seed}: composite windows {landed:?} vs the cat's {cat:?}");
        }
    }

    /// RATCHET — a window vignette's hand-offs keep the window where it is.
    /// State 0x19's idle run 0xff is authored in the bank's third cell
    /// (x 438) and its "someone below" runs 0x698 / 0x6cf / 0x6e2 in the
    /// second (x 339): `fn028A` → `fn3F2E` lines the shared curtain part up,
    /// so the window frame art (784) stays put. The old frame-centre link
    /// threw the whole window one cell (99 px) left and back. (The capture
    /// never shows a window leave its cell; this event is too rare for the
    /// 60 s golden, so the numbers here are the C's.)
    #[test]
    fn window_frames_hold_still_across_every_hand_off() {
        let mut checked = 0;
        for seed in 1..=6u64 {
            let Some(mut m) = load() else { return };
            let mut ctx = ctx_seeded(seed);
            let mut pace = Pacer::new(&m);
            let mut prev: Vec<Option<(i32, (i32, i32), bool, u16)>> = vec![None; WINDOW_COUNT];
            while pace.now_ms() < 20 * 60_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                let placed: Vec<bool> =
                    (0..WINDOW_COUNT).map(|w| m.log.iter().any(|e| matches!(e, Ev::Place { w: w2 } if *w2 == w))).collect();
                m.log.clear();
                for (i, w) in m.wins.iter().enumerate() {
                    let now = (w.cur, w.pos, w.flip, w.state);
                    if let Some((f0, p0, fl0, s0)) = prev[i] {
                        if s0 == w.state && !placed[i] && f0 != w.cur && p0.0 < PARK_X && w.pos.0 < PARK_X {
                            // a hand-off between frames authored in different
                            // bank cells (no window-frame part at the same bank
                            // spot) must still draw one of the new frame's
                            // windows exactly where one of the old frame's was
                            let wins = |f: i32| -> Vec<[i32; 7]> {
                                m.pack.frame(BASE, f as u32).map(|g| g.parts.iter().filter(|q| q[0] == 784).copied().collect()).unwrap_or_default()
                            };
                            let (wa, wb) = (wins(f0), wins(w.cur));
                            let same_bank = wa.iter().any(|a| wb.iter().any(|b| (a[3], a[4]) == (b[3], b[4])));
                            if !wa.is_empty() && !wb.is_empty() && !same_bank {
                                checked += 1;
                                let scr = |f: i32, q: &[i32; 7], p: (i32, i32), fl: bool| {
                                    let r = part_rel(&m.pack, f, q, fl).unwrap();
                                    (p.0 + r[0], p.1 + r[1])
                                };
                                let hit = wa.iter().any(|a| wb.iter().any(|b| scr(f0, a, p0, fl0) == scr(w.cur, b, w.pos, w.flip)));
                                assert!(hit, "window frame moved on {f0:#x} -> {:#x} (state {:#x})", w.cur, w.state);
                            }
                        }
                    }
                    prev[i] = Some(now);
                }
            }
        }
        assert!(checked >= 20, "only {checked} cross-cell hand-offs exercised");
    }

    /// Trace (`--ignored --nocapture`): in every vignette, each frame that
    /// draws window-frame art must draw its windows on the vignette's own
    /// cell grid. 10 seeds × 30 min: 0 off-grid now; the old driver model
    /// (frame-centre links, no flip, no-delta wrap, no `+0xCC`, flip =
    /// `g0E49`) gave 38 031 off-grid window draws.
    #[ignore]
    #[test]
    fn trace_vignette_window_anchor() {
        let mut bad: std::collections::BTreeMap<(u16, i32), usize> = Default::default();
        let mut good = 0;
        for seed in 1..=10u64 {
            let Some(mut m) = load() else { return };
            let mut ctx = ctx_seeded(seed);
            let mut pace = Pacer::new(&m);
            let mut anchor: Vec<Option<(u16, (i32, i32))>> = vec![None; WINDOW_COUNT];
            while pace.now_ms() < 30 * 60_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                m.log.clear();
                for (i, w) in m.wins.iter().enumerate() {
                    if w.state == 0 || w.pos.0 >= PARK_X || w.cur <= 0 {
                        anchor[i] = None;
                        continue;
                    }
                    let Some(g) = m.pack.frame(BASE, w.cur as u32) else { continue };
                    let wins: Vec<(i32, i32)> = g
                        .parts
                        .iter()
                        .filter(|q| q[0] == 784)
                        .map(|q| {
                            let r = part_rel(&m.pack, w.cur, q, w.flip).unwrap();
                            (w.pos.0 + r[0], w.pos.1 + r[1])
                        })
                        .collect();
                    if wins.is_empty() {
                        continue;
                    }
                    match anchor[i] {
                        Some((s, a)) if s == w.state => {
                            if wins.contains(&a) || wins.iter().any(|v| v.1 == a.1 && (v.0 - a.0) % 99 == 0) {
                                good += 1;
                            } else {
                                *bad.entry((w.state, w.cur)).or_insert(0) += 1;
                            }
                        }
                        _ => anchor[i] = Some((w.state, wins[0])),
                    }
                }
            }
        }
        println!("good={good} bad={bad:x?}");
    }

    /// One free-flyer vignette (states 0x13/0x14/0x15) as the screen sees
    /// it: when it was first/last drawn meeting the screen, where, the flip,
    /// the repeat counts the enter (`fn0718`) and the done-branch set.
    #[derive(Clone, Debug, Default)]
    struct Pass {
        state: u16,
        flip: bool,
        coin: bool,
        t_enter: u64,
        t_on: Option<u64>,
        t_off: u64,
        on: (i32, i32),
        off: (i32, i32),
        y_min: i32,
        y_max: i32,
        rep_enter: i32,
        rep_done: Option<i32>,
        /// when the vignette parked (state 0: the descriptor is free again)
        t_park: u64,
    }

    /// Run `ms` of scene time from `seed`, collecting every free-flyer pass.
    fn flyer_passes(seed: u64, ms: u64) -> Vec<Pass> {
        let Some(mut m) = load() else { return Vec::new() };
        let mut ctx = ctx_seeded(seed);
        let mut pace = Pacer::new(&m);
        let mut open: Vec<Option<Pass>> = vec![None; WINDOW_COUNT];
        let mut out = Vec::new();
        while pace.now_ms() < ms {
            pace.advance(&mut ctx);
            let before: Vec<(u16, bool)> = m.wins.iter().map(|w| (w.state, w.more)).collect();
            m.tick(&mut ctx);
            m.log.clear();
            let now = pace.now_ms();
            for i in 0..WINDOW_COUNT {
                let w = &m.wins[i];
                let flyer = matches!(w.state, 0x13..=0x15);
                if flyer && before[i].0 != w.state {
                    if let Some(p) = open[i].take() {
                        out.push(p);
                    }
                    open[i] = Some(Pass { state: w.state, flip: w.flip, coin: m.coin, t_enter: now, rep_enter: w.repeat, y_min: i32::MAX, y_max: i32::MIN, ..Default::default() });
                }
                if !flyer {
                    if let Some(mut p) = open[i].take() {
                        p.t_park = now;
                        out.push(p);
                    }
                    continue;
                }
                let p = open[i].as_mut().unwrap();
                if before[i].1 && !w.more {
                    p.rep_done = Some(w.repeat);
                }
                if w.cur > 0 && w.pos.0 < PARK_X {
                    let (fw, fh) = m.frame_size(w.cur);
                    let (l, t) = (w.pos.0 - fw / 2, w.pos.1 - fh / 2);
                    if l < SCREEN_W && l + fw > 0 && t < SCREEN_H && t + fh > 0 {
                        if p.t_on.is_none() {
                            p.t_on = Some(now);
                            p.on = w.pos;
                        }
                        p.t_off = now;
                        p.off = w.pos;
                        p.y_min = p.y_min.min(w.pos.1);
                        p.y_max = p.y_max.max(w.pos.1);
                    }
                }
            }
        }
        out.extend(open.into_iter().flatten());
        out
    }

    /// RATCHET — the saucer (state 0x15) against `voyeur-long-1/2`: every
    /// one of the 8 golden passes runs edge to edge across the full 640 px
    /// in 4.0–4.2 s, y 156–356, never starting or ending at a window. The
    /// C: `SKY` (the canvas bounds) for `+0x94`, a middle-half start in it,
    /// and the done-branch's `|(x − edge) / step h| + 3` toward the edge
    /// the flip faces. A repeat on the vertical coordinate, or a sky rect
    /// smaller than the screen, stops the saucer short of the far edge.
    #[test]
    fn saucers_cross_edge_to_edge_in_about_four_seconds() {
        let mut n = 0;
        for seed in [1u64, 3, 5, 7, 9, 11] {
            for p in flyer_passes(seed, 30 * 60_000) {
                if p.state != 0x15 {
                    continue;
                }
                let t_on = p.t_on.expect("a saucer never came on screen");
                n += 1;
                let (from_right, to_left) = (p.on.0 > SCREEN_W - 40, p.off.0 < 40);
                let (from_left, to_right) = (p.on.0 < 40, p.off.0 > SCREEN_W - 40);
                if p.flip {
                    assert!(from_right && to_left, "seed {seed}: flipped saucer {p:?} not right → left edge");
                } else {
                    assert!(from_left && to_right, "seed {seed}: saucer {p:?} not left → right edge");
                }
                let dur = p.t_off - t_on;
                assert!((4000..=4300).contains(&dur), "seed {seed}: crossing took {dur} ms (golden 4.0–4.2 s)");
                assert!(p.y_min >= 110 && p.y_max <= 365, "seed {seed}: saucer y {}..{} (golden 156–356)", p.y_min, p.y_max);
                let tail = p.t_park - p.t_off;
                assert!((1000..=2500).contains(&tail), "seed {seed}: parked {tail} ms after leaving (+3 cycles)");
            }
        }
        assert!(n >= 6, "only {n} saucer passes");
    }

    /// RATCHET — the toaster (state 0x14) against `voyeur-long-1/2`: three
    /// dives of 6.3–6.5 s, in at the top right, y rising all the way, out
    /// through the left edge (building right: flip = `!g0E49` = 0). The C
    /// sets its exit repeat on y against the sky's bottom; it leaves by the
    /// side edge it heads for or the bottom, never stopping on screen.
    #[test]
    fn toasters_dive_across_and_out_of_the_sky() {
        let mut n = 0;
        for seed in [1u64, 3, 5, 7, 9, 11] {
            for p in flyer_passes(seed, 30 * 60_000) {
                if p.state != 0x14 {
                    continue;
                }
                let t_on = p.t_on.expect("a toaster never came on screen");
                n += 1;
                let dur = p.t_off - t_on;
                assert!((5000..=6600).contains(&dur), "seed {seed}: dive took {dur} ms (golden 6.3–6.5 s)");
                assert!(p.off.1 > p.on.1 + 150, "seed {seed}: toaster {p:?} did not dive");
                // unflipped heads left (the golden's), flipped heads right
                let (on_x, off_x) = (p.on.0, p.off.0);
                if p.flip {
                    assert!(off_x > on_x, "seed {seed}: flipped toaster flew left {p:?}");
                } else {
                    assert!(off_x < on_x, "seed {seed}: toaster flew right {p:?}");
                }
                let out_side = if p.flip { off_x > SCREEN_W - 40 } else { off_x < 40 };
                assert!(out_side || p.off.1 > SCREEN_H - 40, "seed {seed}: toaster stopped on screen {p:?}");
                let in_edge = p.on.1 < 40 || if p.flip { on_x < 40 } else { on_x > SCREEN_W - 40 };
                assert!(in_edge, "seed {seed}: toaster appeared mid-sky {p:?}");
            }
        }
        assert!(n >= 4, "only {n} toaster dives");
    }

    /// RATCHET — the chooser's retry walks on from its last pick (@7008
    /// zeroes the walk index once; the retry branch @70E2 re-enters at
    /// @700C). With only descriptor 0 free, a relight reaches it only when
    /// the first roll lands on it (r <= 10 of 415, about 2.7 %); a walk
    /// that restarts at record 0 on every retry finds it within 101 tries
    /// about 91 % of the time.
    #[test]
    fn a_chooser_retry_walks_on_from_the_last_pick() {
        let Some(mut m) = load() else { return };
        let mut ctx = ctx_seeded(9);
        let mut pace = Pacer::new(&m);
        pace.advance(&mut ctx);
        m.tick(&mut ctx);
        const N: usize = 400;
        let mut hits = 0;
        for _ in 0..N {
            m.busy = [false; 6];
            m.go = [None; 6];
            m.desc_busy = [true; 26];
            m.desc_busy[0] = false;
            m.grid = [[0; 6]; 6];
            m.relight(&mut ctx);
            if m.go.iter().any(|g| matches!(g, Some((0, _, _)))) {
                hits += 1;
            }
        }
        assert!(hits >= 1 && hits * 10 < N, "descriptor 0 picked {hits} of {N} (C walk: about 2.7 %)");
    }

    /// Trace (`--ignored --nocapture`): every free flyer over many seeds,
    /// against the `voyeur-long-1/2` golden (900 s, 2026-09-29).
    #[ignore]
    #[test]
    fn trace_voyeur_flyers() {
        let mut all = Vec::new();
        // `RandomLong::new` ORs the seed with 1: odd seeds only
        let seeds: Vec<u64> = (0..12).map(|k| 2 * k + 1).collect();
        for &seed in &seeds {
            let ps = flyer_passes(seed, 30 * 60_000);
            for p in &ps {
                if seed <= 3 {
                    println!(
                        "seed {seed} st={:#x} flip={} coin={} enter={} on={:?}@{:?} off={:?}@{} dur={:?} y={}..{} rep {} / {:?}",
                        p.state, p.flip, p.coin, p.t_enter, p.t_on, p.on, p.off, p.t_off,
                        p.t_on.map(|t| p.t_off - t), p.y_min, p.y_max, p.rep_enter, p.rep_done
                    );
                }
            }
            all.extend(ps.into_iter().map(|p| (seed, p)));
        }
        for st in [0x14u16, 0x15] {
            let ps: Vec<&Pass> = all.iter().map(|(_, p)| p).filter(|p| p.state == st && p.t_on.is_some()).collect();
            let durs: Vec<u64> = ps.iter().map(|p| p.t_off - p.t_on.unwrap()).collect();
            let edge = |x: i32| x < 40 || x > SCREEN_W - 40;
            let e2e = ps.iter().filter(|p| edge(p.on.0) && edge(p.off.0)).count();
            let (ymin, ymax) = (ps.iter().map(|p| p.y_min).min(), ps.iter().map(|p| p.y_max).max());
            println!(
                "state {st:#x}: {} passes in {} s; dur min {:?} max {:?} mean {:.0}; edge-to-edge {e2e}; y {ymin:?}..{ymax:?}",
                ps.len(), 12 * 30 * 60,
                durs.iter().min(), durs.iter().max(), durs.iter().sum::<u64>() as f64 / durs.len().max(1) as f64
            );
        }
        let tail: Vec<u64> = all.iter().filter(|(_, p)| p.state == 0x15 && p.t_on.is_some() && p.t_park > 0).map(|(_, p)| p.t_park - p.t_off).collect();
        println!("saucer off-screen → parked (ms): min {:?} max {:?}", tail.iter().min(), tail.iter().max());
        let mut gaps = Vec::new();
        for &seed in &seeds {
            let ps: Vec<&Pass> = all.iter().filter(|(s, p)| *s == seed && p.state == 0x15 && p.t_on.is_some()).map(|(_, p)| p).collect();
            for w in ps.windows(2) {
                gaps.push(w[1].t_on.unwrap() as i64 - w[0].t_off as i64);
            }
        }
        gaps.sort();
        println!("saucer gaps (ms, off→next on), shortest: {:?}", &gaps[..gaps.len().min(10)]);
    }

    /// Every vignette start (`Ev::Start`) over `ms` of one seed: (time ms,
    /// descriptor), and the scene's row count (`g0D0C`, 3..=5).
    fn starts(seed: u64, ms: u64) -> (Vec<(u64, usize)>, u8) {
        let Some(mut m) = load() else { return (Vec::new(), 0) };
        let mut ctx = ctx_seeded(seed);
        let mut pace = Pacer::new(&m);
        let mut out = Vec::new();
        while pace.now_ms() < ms {
            let now = pace.advance(&mut ctx);
            m.tick(&mut ctx);
            for e in m.log.drain(..) {
                if let Ev::Start { desc, .. } = e {
                    out.push((now, desc));
                }
            }
        }
        (out, m.rows as u8)
    }

    /// Odd seeds whose scene is the golden's 4 × 3 (`g0D0C` = 3).
    const SEEDS_3_ROWS: [u64; 6] = [5, 9, 15, 19, 21, 25];

    /// Window vignettes (placed descriptors) vs free flyers (0x13..0x15).
    fn is_flyer(desc: usize) -> bool {
        matches!(DESCS[desc].state, 0x13..=0x15)
    }

    /// Ratchet (2026-09-29): the vignette START RATE against `voyeur-long-1/2`
    /// (900 s, no panel). Measured there: 97–108 window-vignette starts (a
    /// closed shutter giving way to a scene; flyers excluded) = one per
    /// 8.3–9.3 s, and 11 flyers (8 saucers + 3 toasters). The port ran one
    /// DoDrawFrame per Mac tick: 45 window starts per 900 s on these seeds.
    #[test]
    fn vignette_start_rate_matches_the_long_golden() {
        if load().is_none() {
            return;
        }
        // the golden's scene is 4 × 3: odd seeds whose scene rolls 3 rows
        const MS: u64 = 15 * 60_000;
        let (mut win, mut fly) = (0usize, 0usize);
        for seed in SEEDS_3_ROWS {
            let (st, rows) = starts(seed, MS);
            assert_eq!(rows, 3, "seed {seed}");
            for (_, d) in st {
                if is_flyer(d) {
                    fly += 1;
                } else {
                    win += 1;
                }
            }
        }
        let per900 = |n: usize| n as f64 * 900_000.0 / (SEEDS_3_ROWS.len() as u64 * MS) as f64;
        let (w, f) = (per900(win), per900(fly));
        assert!((70.0..=150.0).contains(&w), "window starts per 900 s: {w:.1} (golden 97–108)");
        assert!((3.5..=25.0).contains(&f), "flyers per 900 s: {f:.1} (golden 11)");
    }

    /// Ratchet (2026-09-29): the draw rate itself. In `voyeur-long-1/2` a
    /// cell `fn38` lit (dark → lit-empty → dark, 896 flashes) stays lit
    /// 0.54 s (MLE over the 10 fps sampling; 0.53 / 0.54 s per take), and a
    /// dark stretch lasts ~5 s — the 1-in-10 relight. At one DoDrawFrame
    /// per Mac tick the flash lasted 500 × 12 ticks ≈ 100 s.
    #[test]
    fn lit_empty_flashes_last_half_a_second() {
        let Some(mut m) = load() else { return };
        let mut ctx = ctx_seeded(SEEDS_3_ROWS[0]);
        let mut pace = Pacer::new(&m);
        let mut lit_since: [[Option<u64>; 6]; 6] = [[None; 6]; 6];
        let mut lives = Vec::new();
        while pace.now_ms() < 10 * 60_000 {
            let now = pace.advance(&mut ctx);
            m.tick(&mut ctx);
            m.log.clear();
            assert_eq!(m.rows, 3);
            for r in 0..m.rows as usize {
                for c in 0..m.cols as usize {
                    match (m.grid[r][c], lit_since[r][c]) {
                        (1, None) => lit_since[r][c] = Some(now),
                        (0, Some(t0)) => {
                            lives.push((now - t0) as f64 / 1000.0);
                            lit_since[r][c] = None;
                        }
                        (2, Some(_)) => lit_since[r][c] = None,
                        _ => {}
                    }
                }
            }
        }
        let mean = lives.iter().sum::<f64>() / lives.len().max(1) as f64;
        assert!(lives.len() > 500, "only {} lit-empty flashes in 10 min", lives.len());
        assert!((0.44..=0.66).contains(&mean), "lit-empty flash lives {mean:.2} s (golden 0.54 s)");
    }

    /// Trace: vignette starts per scene time, the inter-start gaps, and how
    /// the flyers share them (`--ignored --nocapture`).
    #[ignore]
    #[test]
    fn trace_voyeur_start_rate() {
        let want_rows: Option<u8> = std::env::var("VOYEUR_ROWS").ok().and_then(|v| v.parse().ok());
        let (mut win, mut ms) = (0usize, 0u64);
        let mut gaps = Vec::new();
        let mut by_state: std::collections::BTreeMap<u16, usize> = Default::default();
        let mut used = Vec::new();
        for seed in (0..48).map(|k| 2 * k + 1) {
            if used.len() == 12 {
                break;
            }
            let (st, rows) = starts(seed, 30 * 60_000);
            if want_rows.is_some_and(|r| r != rows) {
                continue;
            }
            used.push((seed, rows));
            let wt: Vec<u64> = st.iter().filter(|(_, d)| !is_flyer(*d)).map(|(t, _)| *t).collect();
            win += wt.len();
            gaps.extend(wt.windows(2).map(|p| (p[1] - p[0]) as f64 / 1000.0));
            for (_, d) in st {
                *by_state.entry(DESCS[d].state).or_insert(0) += 1;
            }
            ms += 30 * 60_000;
        }
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = |p: f64| gaps[((gaps.len() - 1) as f64 * p) as usize];
        let fl = |s: u16| by_state.get(&s).copied().unwrap_or(0) as f64 * 900_000.0 / ms as f64;
        println!(
            "window starts {win} in {} s: one per {:.2} s; gap p10/25/50/75/90 {:.1}/{:.1}/{:.1}/{:.1}/{:.1} s",
            ms / 1000, ms as f64 / 1000.0 / win as f64, q(0.1), q(0.25), q(0.5), q(0.75), q(0.9)
        );
        println!("per 900 s: saucers {:.1}, toasters {:.1}, 0x13 {:.1}; {by_state:x?}", fl(0x15), fl(0x14), fl(0x13));
        println!("seeds (seed, rows): {used:?}");
    }
}
