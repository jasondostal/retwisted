//! Mike's So-called Life — full-fidelity transcription of the RE spec
//!
//! ## Port-from-decompile pass, 2026-09-15 (Fable) — M129_M129.c, private
//! Read against the Ghidra C (totally-twisted docs/decompiled/mikes-so-called-
//! life): `fn72` @00F2 (Mike's state handler: enter/exit hooks + the update
//! path), `fn74` @0D0E (ChooseNextState — the windows dumped from the A5
//! globals match `DATA_WINDOWS`), `fn75` (the schedule), `fn77` @16A4 (enter:
//! run lookup in `g0264` + `fn76` re-place), `fn76` @1598 (position = room
//! rect centre + the run's link to the room compound), `fn02` @1D52 (room
//! change: the new room rect at a RANDOM offset inside the screen — the
//! placement rule the plan wanted off code — plus the prop show/hide set),
//! `fn45` @308E (DoDrawFrame on the 100 ms floor), `fn21`/`fn22` (the 16-slot
//! cue queue), `fn55`–`fn58` (the TV: rand%3 channel, runs 0x4ea/0x4ef),
//! `fn62`–`fn64` (the 1-in-128 `(Random15() & 0x7F) == 0x45` roll — which
//! is `g094E`'s, a kitchen sprite, NOT Mike's; see the fix lane below).
//! What changed in this pass:
//! - **the per-state run is a table**, `g0264` (94 pairs, `STATE_RUN`) —
//!   the "undecoded handler bodies" the presentation map guessed at;
//! - **rooms change on state EXITS** (`fn02` from the exit hooks of
//!   0x13/0x38/0x3f/0x5c → hall, 0x19/0x1f/0x43 → kitchen, 0x21/0x2c/0x39/
//!   0x3a/0x44/0x47/0x48 → living), not per state;
//! - the invented `travel_chain` walk synthesis is gone: `fn77` is SetRun +
//!   re-place, and the authored runs (walks included) are the itinerary.
//! GAP: the prop show/hide set per room was not re-derived this pass — the
//! prop list stays as the earlier passes had it. (The per-state cue lists
//! were: see the fix lane below.)
//!
//! ## Fix lane, 2026-09-26 (Opus) — Jason's eye: "blinked to the back
//! ## standing up then blinked into the chair … weird sound"
//! - **In-room teleports** were all one invention: a "1-in-128 surprise pose"
//!   that drew art 456 (state 93's `g0264` run) at its own world spot for
//!   ~1 s — 99–102 px out and back, 13× per 120 s. The roll it claimed to
//!   port is `fn62` @185A, the update of `g094E` (vtbl g058A, built next to
//!   Mike in `fn43`, close run 0xB44): when that sprite is open and its run
//!   is done, 1-in-128 per frame closes it. Mike's handler never draws 456
//!   outside state 93. Pose, its Feh! and its per-tick draw are gone.
//!   GAP(g094E): `fn61`–`fn64` not ported, so the roll is not made.
//! - **Cue spam** (1009 + 1012 ~100× each at a 416 ms gap): `entry_cues`
//!   queued Scratching + Filter Sweep on every ENTER of 22 (0x16) — which
//!   re-enters itself every 4-frame run through the 4 s dwell — and 0x16
//!   has no ENTER hook at all. Replaced by `ENTER_CUES`, the listing's own
//!   `fn21` push lists for all 31 hooked states (@038E..@0BE8). And the
//!   index BASE was wrong: `fn17` @3950 builds the channels from `g0A88`, so
//!   channel 0 is Burp and 2 is Pong (`CHANNELS`); the port read @0xA90.
//!   The pump (`fn22` @3BBC) plays a slot whose counter IS 0 and then
//!   decrements, so delay `d` sounds on the `d+1`-th frame (was `max(d,1)`).
//!   Golden `mikes-so-called-life.wav` (matched filter): 1009 ×5 at 2-frame
//!   spacing = exactly ENTER 0x28's (7@8,10,12,14,16); 1023 ×3 = ENTER 0x37.
//! - **The telephone rang by itself** (every 20–50 s, 30 s answer window,
//!   setting +0x11C). Nothing in the C does that: rings are channel 14 in
//!   ENTER 0x1A/0x1B/0x34/0x38, and `g0936`'s 8-slot queue (`fn30` @3E90 from
//!   0x1A/0x1B/0x34) swaps the static phone for the 12-frame ringing run
//!   0x7C7 (`fn29` @3DCE / `fn39` @417E / `fn41` @429A). +0x11C is Mike's
//!   own flag: set on EXIT 0x37, cleared on EXIT 0x2F/0x36.
//!   Quirk kept: `fn40` places the ringing phone in the CURRENT room's
//!   coordinates and `fn02` never hides `g0936`, so a ring outside the hall
//!   draws the hall-table phone's spot in whatever room is up.
//! - GAPs still open: exit 0x4010/0x4011 → `fn55(g094A)` (TV) and the
//!   ENTER 0x18/0x1F/0x21/0x24/0x25/0x27/0x28/0x29/0x42/0x49 prop
//!   show/hide calls (`+0x2C`/`+0x30`/`+0x58` on g092E/g093A/g093E/g0932/
//!   g0946/g095A) are not ported; ENTER 0x01's `+0x2C` and update 1 are the
//!   hidden idle state. Channels 13/16/22/23 are id 0: queued, silent.
//! (totally-twisted docs/behavior/mikes-so-called-life.md; every constant
//! below is either disasm-cited there or byte-verified against
//! `ripped/mike's-so-called-life/Mike's So-called Life_DATA_129_DynaMikes.bin`).
//!
//! Faithful behavior implemented here:
//! - The 95-state `CSpriteMike` activity machine: `ChooseNextState`
//!   (fn74_0D12) transition table with the EXACT weighted-random windows
//!   (DATA 129 @0x001A, 149 words, multiplicity = weight — byte-verified)
//!   and the exact (window offset, count, repeatOK) triples re-verified
//!   against the fn74 switch push sites. Where the §5.2 prose table and the
//!   disasm disagree, the disasm wins: 27/28/77/78 use window count 7
//!   (@0x122A, spec says 8), 47/54 use count 3 (@0x1356, spec says 4),
//!   53 joins 55/59 on window 0xFE (@0x1408, spec routes it to 66), and
//!   18/91 are a constant → 36 (@0x1184, spec lumps them into window 0x62)
//! - Dwell semantics: 4000 ms (state 22), 5000 ms (11/14/35), 3000 ms for
//!   the TV group {49,80..95}; the +0x11E flag is set each tick while
//!   22/11/14/35 dwell (@0x1058/0x10D2/0x111E/0x1290) and on TV expiry
//!   (@0x13B8)
//! - The scripted 127-word day schedule (DATA 129 @0x0166, verbatim, §5.4),
//!   selected by the vestigial `controlValues(3)` read — which, exactly as
//!   in the shipped product, reads a control index that does not exist and
//!   therefore NEVER RUNS (§1, §8.1). The "Activity Level" slider exists
//!   and does nothing, per Berkeley's own help text
//! - MySoundPlayer: `fn21` 16-slot cue queue, drop-when-full with the
//!   shipped "List full!" diagnostic, `fn22` pump last in the frame, 25
//!   channels from `g0A88` (`CHANNELS`), per-state ENTER lists
//!   (`ENTER_CUES`) — see the fix lane above
//! - RingingPhoneSprite `g0936`: 8-slot delay queue fed by ENTER 0x1A/0x1B/
//!   0x34, ringing run 0x7C7 in place of the static phone
//! - Fixed 100 ms master tick with skip-if-late gate (§6, §8.4)
//! - Original bugs kept: the dead Activity Level control (§8.1), debug
//!   printfs on the unknown-state path (§8.2), sound queues silently
//!   dropping when full (§8.8)
//!
//! ## APPROXIMATIONS
//! The spec decodes the planner, schedule, sound table and a sample of
//! handlers exactly, but the 95 per-state handler bodies (which sequence
//! each state plays, and when each cue fires) are explicitly listed as an
//! open question (§9 "Per-state narration"). Approximations made:
//! - State → art/scene presentation map (§5.1 groups + §3.1 catalogue):
//!   hallway states get scene compound 5 + idle/walk runs, kitchen states
//!   scene 10, mirror/TV states scene 1. Sequence *choices* per state are
//!   plausible group-level picks, not decoded handler reads.
//! - The state change is committed when the current run completes (engine
//!   `vtbl+0x1C` commit semantics were not decoded; standard Library-4.0
//!   sprite machines defer to sequence end). The TV exit after the 3 s
//!   dwell (handlers undecoded, but W(0x00C0) contains only TV states, so
//!   the undecoded handler must read +0x11E to leave) is approximated as a
//!   transition to hallway state 2.
//! - Rendering: the original blits compounds into a 408×199 scene canvas
//!   (§2.2 g0962). All packed frames carry OFtb world bounds (`bx/by`)
//!   sharing one coordinate space (living-room scene 1 sits at world
//!   (104,148)); we draw everything in that space, canvas origin at screen
//!   (128,128) per the DoSetUp `0x80` note (§4 step 2). Palette/clut
//!   switching (2001/2002/2003) needs runtime recolor the engine lacks;
//!   scenes are used as authored.
//! - Prop/critter roster and initial sequences are exact (§2.2); which
//!   prop is which beyond the tree/phone/bugs is UNCERTAIN (§9), so the
//!   small props are assigned to rooms by world-bounds overlap.
//! - Because `present` guesses ONE run per state, two consecutive activities
//!   can be drawn a room apart, and cutting between them is a teleport. The
//!   original has no such problem — its handlers name sequences that already
//!   chain end-to-start — so `travel_chain` reconstructs the missing link by
//!   walking the pack's own locomotion runs (`WALK_RUNS`) from where Mike is
//!   standing to where the new activity starts. Which runs those are, and
//!   which room each may be played in, is eyes-on-the-compounds curation, not
//!   a decoded table.
//!
//! ## ERRATA — golden capture `emu/captures/mikes-long.mp4` (2026-09-12,
//! ## tick-quantization sweep)
//!
//! - **The 100 ms master tick is a deadline on the Mac tick grid, not a
//!   100 ms wall-clock beat.** §6/§8.4's gate is `g0910 = now + 100` with
//!   skip-if-late, and `now` is `Resource.fn_4724() = TickCount()*16.625`
//!   (§0 table), so the tick lands on the first Mac tick at or past +100 ms
//!   — never at exactly 100. Measured: a Rayleigh periodogram of the
//!   screen-change events in `mikes-long.mp4` over t = 20-45 s (110 events
//!   above a 45 %-of-median floor, crop 56 px + halve to module px) peaks at
//!   **P = 106.50 ms, R = 0.656**; R at a flat 100 ms is 0.096 and at the
//!   real Mac's 7 ticks = 116.375 ms it is 0.014. So a flat 100 ms is ruled
//!   out, and the port now ticks on a 17 ms grid where the unchanged `100`
//!   quantizes to 6 ticks = **102 ms**.
//! - The whole `tick` body sits behind that gate, so moving the shell tick
//!   changes the master period and nothing else: the sound-slot delays (§6,
//!   "sound delays are in ticks") still decrement once per master frame.
//! - An integer `tick_ms` cannot be 16.625: 17 gets the 3-tick quantization
//!   of a 40 ms delay right but gives 6 ticks here where the real Mac gives
//!   7. The capture sits between the two. See `flying_toilets.rs` for the
//!   sweep's full note.

//! ## ERRATA — golden capture `emu/captures/mikes-long.mp4` (2026-09-12,
//! ## 180 s, DEPTH=32 — scene canvas, rooms, props, activities)
//!
//! Geometry used throughout: `crop=1276:958:2:56,scale=638:479:flags=neighbor`
//! on the 1280×1016 capture gives exact module pixels, with module x =
//! measured x + 1. Measurements below are the static residue left after
//! subtracting the packed scene compound from every sampled frame of a room's
//! on-screen period, so they name compounds, not impressions.
//!
//! - **The scene canvas MOVES.** §4 step 2 reads `g0966` initialized to the
//!   screen offset `0x80`; that is only its initial value. The 408×199 panel
//!   is re-placed at a fresh, apparently uniform-random spot on the 640×480
//!   screen every time Mike changes room. Module-pixel top-lefts:
//!   living (194,134) @3.23 s, hallway (15,69) @10.37 s, kitchen (101,193)
//!   @31.27 s, living (206,266) @65.30 s, hallway (81,7) @127.43 s; plus
//!   `mikes-life.mp4` living (197,20) @4.0 s and hallway (73,41) @30.60 s.
//!   Seven draws, seven spots, both living visits different and both hallway
//!   visits different — so neither fixed nor per-room — and every one inside
//!   0..=232 × 0..=281, which is exactly `screen − panel`.
//! - **The world origin is the CURRENT scene's own bounds, not compound 1's.**
//!   Compound 1 is at world (104,148), 5 at (143,148), 10 at (167,149), and
//!   whichever is current lands at the canvas origin. Four confirmations, one
//!   per room-prop: hall plant `c_2074` (world 517,267) measured at
//!   hall-relative (374..401, 119..193); hall telephone `c_2072` (326,303) at
//!   (183..202, 155..164); kitchen pinboard `c_854` (441,279) at
//!   kitchen-relative (274..328, 131..186); living desk chair `c_1094`
//!   (412,288) at living-relative (308..332, 140..192). Compound 10 is
//!   360×198, not 408×199 — the kitchen does not fill the canvas.
//! - **Room changes are rare: five in 180 s.** Living 3.23-10.37, hallway
//!   10.37-31.27, kitchen 31.27-65.30, living 65.30-127.43, hallway
//!   127.43-180+ — dwells 7.1 / 20.9 / 34.0 / 62.1 / 52.6+ s, median 34 s,
//!   and never two of the same room back to back. Giving all 95 states a
//!   hard room made the port cut 16 times in the same 180 s (median dwell
//!   5.0 s); activities with no fixture of their own now inherit Mike's room.
//! - **The purple room is the HALLWAY and the fixture in its left doorway is
//!   a clawfoot BATHTUB, not a toilet.** It is baked into compound 5 itself
//!   (§3's "bathroom left, bedroom right"), the blue panel on the right is
//!   the BEDROOM DOOR and not a shower curtain, and the whole tub family
//!   lands on it: `seq 179/190/265` are 77×97 at world (233,245) =
//!   hall-relative (90..166, 97..193), the door opening. 127.4-180 s is Mike
//!   bathing there twice — in the water 134.8-148.6 s and 158.6-170.6 s, out
//!   in a green towel by the hall table 152.6-157.6 s and 174.6-179.6 s.
//! - **The Christmas tree (`c_581`, g092E's DoSetUp sequence) is never
//!   drawn.** Its world box (489,241)+46×100 is living-relative 385..430 —
//!   over the right-hand radiator and half off the canvas — hall-relative
//!   346..391, kitchen-relative 322..367. The capture covers 69 s of living
//!   room, 74 s of hallway and 34 s of kitchen, and all three boxes are
//!   empty in all three rooms. §2.1's per-sprite draw is gated on
//!   `this->0x54`, so the roster can hold a sprite the module never shows.
//! - **`c_1094` is not the "wall telephone" of §3.1** — it is the living
//!   room's swivel DESK CHAIR (grey seat, black post and base) in front of
//!   the Mac desk, and the living room's ONLY static overlay in both of its
//!   periods. The telephone is `c_2072`, on the hall side table. `c_1443` is
//!   the Mac's SCREEN: living-relative (301..318, 121..134) lands exactly on
//!   the blank monitor compound 1 paints, and 70-110 s shows a small white
//!   shape swimming across it.
//! - **`c_2438`-`c_2494` is a DAYDREAM, not a fly.** It is a 57-frame
//!   one-shot: an 8×6 speck at world (297,268) balloons to 62×48 with a
//!   FLYING TOASTER inside it and drifts up-left on cumulative dx/dy to
//!   finish 77×60 at world (149,103). One play in 180 s, in the kitchen —
//!   on screen 54.4-60.4 s (≈5.7 s = 57 frames at one per master tick),
//!   toaster legible at 57.8 s, at kitchen-relative (92..157, 71..111) then
//!   (68..138) as it drifts. The previous pass pinned it to frame 2438 to
//!   stop a giant toaster cycling; the toaster is real, the cycling was the
//!   bug. `seq 2224` by contrast IS the flies — its frames are two- and
//!   three-pixel specks inside wide, otherwise-transparent boxes.
//! - **The TV block is one long sit.** 68.5-122.5 s is 54 uninterrupted
//!   seconds in the pink armchair at living-relative 149..176, with the TV
//!   screen re-drawing behind him (content changes at 78.5 / 83.5 / 94.5 /
//!   105.5 / 116.5 s) and a newspaper from ~105 s; he stands up at 122.5 s
//!   and walks off to the right, and the room cuts at 127.43 s.
//! - **Walking is ≈5.8 px per master frame.** The 122.5-127.1 s living-room
//!   walk covers 221 px in 4.03 s (54.8 px/s) with a 5-frame stride cycle;
//!   the port's median moving step is 5 px/frame. Measured px/frame only —
//!   the frame cadence itself is another lane's (see the tick note above).
//! - **`full-reel.mp4` has NO Mike's segment.** `docs/emulator/setup.md`'s
//!   segment map lists Mike's So-called Life under "never demoed in this
//!   capture"; `mikes-long.mp4` and `mikes-life.mp4` are the only goldens.
//!
use engine::{ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TickClock};

const BASE: u32 = 1000;

// ---------------------------------------------------------------------------
// DATA 129 verbatim tables (byte-verified against the ripped resource)

/// §5.2 weighted-random windows, DATA 129 @0x001A, 149 words. Window
/// contents are state ids; multiplicity is the weight (ORIGINAL BUG kept
/// as a feature: §8.6 — e.g. channel 84 appears 4× in the TV window).
const DATA_WINDOWS: [u16; 149] = [
    2, 3, 4, 4, 10, 13, 15, 16, 16, 22, 22, 22, 22, 23, 7, 7, //
    7, 9, 9, 9, 17, 22, 11, 11, 12, 14, 14, 14, 21, 18, 19, 20, //
    4, 13, 15, 22, 51, 51, 52, 60, 62, 63, 65, 8, 22, 22, 67, 68, //
    27, 77, 77, 77, 77, 78, 79, 24, 24, 25, 25, 25, 25, 26, 28, 30, //
    32, 32, 35, 35, 43, 45, 44, 44, 44, 46, 39, 42, 57, 58, 58, 73, //
    74, 75, 75, 80, 80, 81, 82, 84, 84, 84, 84, 86, 87, 88, 80, 80, //
    81, 82, 84, 84, 84, 84, 86, 87, 88, 90, 90, 90, 91, 92, 93, 94, //
    28, 29, 53, 53, 54, 54, 56, 59, 59, 59, 59, 60, 61, 62, 65, 65, //
    65, 40, 41, 80, 80, 82, 84, 88, 90, 93, 83, 83, 83, 83, 87, 89, //
    89, 83, 83, 83, 87,
];

/// §5.4 the scripted day, DATA 129 @0x0166 — 126 activity words + the `1`
/// end-of-day marker that resets the cursor (byte-verified; the spec's
/// prose "120-word" is loose). Dead code in the shipped product (§8.5);
/// shipped here as the hidden "schedule mode".
const DAY_SCHEDULE: [u16; 127] = [
    80, 81, 82, 84, 86, 88, 87, 89, 83, 94, 85, 95, 80, 91, 36, 37, //
    80, 90, 4, 2, 3, 13, 11, 12, 16, 22, 16, 22, 16, 22, 16, 23, //
    8, 14, 21, 15, 20, 62, 60, 61, 65, 64, 80, 92, 51, 28, 77, 78, //
    27, 77, 79, 26, 68, 49, 80, 92, 51, 29, 24, 34, 35, 30, 31, 48, //
    74, 73, 75, 76, 40, 46, 72, 69, 6, 5, 10, 15, 19, 52, 66, 67, //
    48, 73, 75, 76, 41, 42, 39, 43, 38, 55, 59, 53, 56, 50, 66, 67, //
    47, 58, 70, 9, 7, 17, 7, 22, 15, 18, 36, 37, 80, 92, 51, 29, //
    25, 48, 73, 75, 76, 41, 39, 43, 38, 55, 59, 54, 57, 64, 1,
];

/// MySoundPlayer channel table (`g091A`), built by `fn17` @3950: one
/// channel per four-byte row of **A5 `g0A88`** (= DATA 129 @0xA88), in
/// order, until the negative terminator. Channel `i` is row `i`; `fn18`
/// @3A14 is the `i -> channel` lookup `fn22`'s pump plays through. The second
/// word of each row is the `LoadSound` flag (0 for the shared 30000-series
/// ids, 1 for the module's own), not a delay. 0 = no sound loaded (the
/// channel exists and plays nothing).
///
/// FIX 2026-09-26: the earlier passes indexed the cue lists from @0xA90,
/// i.e. two rows late — every `fn21` index the C pushes names the row two
/// earlier than the port played. Burp (30009) and Flush ALL (30000) are
/// simply channels 0 and 1, not side-band "direct" sounds. Pinned by
/// `the_channel_table_starts_at_0xa88`.
const CHANNELS: [u32; 25] = [
    30009, // 0  shared Burp
    30000, // 1  shared Flush ALL
    1004,  // 2  Pong
    1005,  // 3  Hmm
    1006,  // 4  Humming
    1007,  // 5  Click
    1008,  // 6  Drawer Sliding
    1009,  // 7  Scratching
    0,     // 8  —
    1011,  // 9  Chuckle
    1012,  // 10 Dreamy Filter Sweep
    0,     // 11 —
    1014,  // 12 Mike New Tub
    0,     // 13 —
    1016,  // 14 Phone Ringing
    1017,  // 15 Dial Tone
    0,     // 16 —
    30007, // 17 shared Sniff
    1020,  // 18 Placeholder
    1021,  // 19 Feh!
    1022,  // 20 Tsk Tsk Tsk
    1023,  // 21 Fart 2
    0,     // 22 —
    0,     // 23 —
    1027,  // 24 Fart Underwater
];

/// `fn72` @00F2's ENTER hooks (`0x8000|state`), transcribed off the listing
/// push sites @038E..@0BE8: every `fn21` @3AF2 call is (channel, delay in
/// master frames). States not listed queue nothing on enter. Read against
/// the listing, not the decompile (the decompile scrambles the three pushed
/// shorts): first push = channel, middle = 0, last = delay.
const ENTER_CUES: &[(u16, &[(u8, i16)])] = &[
    (0x05, &[(0, 2)]),                                  // @038E
    (0x10, &[(5, 3)]),                                  // @03A2
    (0x11, &[(5, 3)]),                                  // @03A2
    (0x1A, &[(14, 0), (15, 15)]),                       // @041C
    (0x1B, &[(14, 0), (14, 45), (14, 90), (15, 128)]),  // @044C
    (0x1C, &[(12, 20), (13, 77)]),                      // @04B8
    (0x1D, &[(1, 10)]),                                 // @04E0
    (0x24, &[(2, 15), (2, 25), (2, 38)]),               // @0520
    (0x28, &[(7, 8), (7, 10), (7, 12), (7, 14), (7, 16)]), // @05DA
    (0x2A, &[(7, 1), (7, 3), (7, 5), (7, 7), (7, 9), (7, 11)]), // @0670
    (0x2F, &[(4, 0)]),                                  // @06E0
    (0x30, &[(4, 0)]),                                  // @06E0
    (0x34, &[(14, 5)]),                                 // @076C
    (0x37, &[(21, 13), (21, 18), (21, 23)]),            // @07A6
    (0x38, &[(14, 0)]),                                 // @07EE
    (0x39, &[(17, 13)]),                                // @0802
    (0x3D, &[(10, 1)]),                                 // @0828
    (0x42, &[(15, 2)]),                                 // @083E
    (0x45, &[(4, 2)]),                                  // @088A
    (0x46, &[(4, 2)]),                                  // @088A
    (0x47, &[(10, 0), (3, 53), (9, 71)]),               // @08A0
    (0x48, &[(10, 0), (3, 53), (9, 71)]),               // @08A0
    (0x49, &[(17, 31), (17, 34)]),                      // @08E8
    (0x4A, &[(10, 0), (17, 21), (17, 27), (19, 31), (20, 35), (20, 37), (20, 39), (20, 41)]), // @0952
    (0x4D, &[(13, 0)]),                                 // @09E4
    (0x4E, &[(24, 0), (9, 26), (13, 29)]),              // @09F8
    (0x51, &[(21, 5), (21, 7), (9, 8), (21, 11), (21, 13)]), // @0A30
    (0x55, &[
        (6, 2), (5, 7), (5, 9), (5, 11), (5, 13), (5, 15), (5, 17), (5, 19),
        (5, 35), (5, 37), (5, 39), (5, 41), (5, 43), (5, 45), (5, 47), (6, 61),
    ]),                                                 // @0A8E
    (0x56, &[(7, 2), (7, 4)]),                          // @0BC2
    (0x58, &[(4, 0)]),                                  // @0BB0
    (0x59, &[(3, 7)]),                                  // @0BE8
];

/// `fn72`'s ENTER hooks that also queue the ringing telephone (`g0936`,
/// `fn30` @3E90 — one delay per slot of its 8-slot queue): 0x1A @0432,
/// 0x1B @045E/@047A/@0496, 0x34 @0782. The rings you HEAR are the channel-14
/// cues in `ENTER_CUES` at the same delays; this queue is the picture.
const ENTER_RINGS: &[(u16, &[i16])] = &[
    (0x1A, &[0]),
    (0x1B, &[0, 45, 90]),
    (0x34, &[5]),
];

/// `g0936`'s run, set by `fn38` @4118 in the ctor (0x7C7): the 12-frame
/// ringing telephone, authored on the hall table's phone (`c_2072`).
const RINGING_PHONE_RUN: u32 = 0x7C7;

/// Window reference: word offset in DATA 129 → index into DATA_WINDOWS.
const fn widx(off: u32) -> usize {
    ((off - 0x1A) / 2) as usize
}

// ---------------------------------------------------------------------------
// Dwell constants (§5.2: `this->0x122` deadline pattern)

const DWELL_SLEEP_MS: u64 = 4000; // 0x0FA0 — state 22
const DWELL_IDLE_MS: u64 = 5000; // 0x1388 — states 11/14/35
const DWELL_TV_MS: u64 = 3000; // 0x0BB8 — the 49/80…95 TV group

const TICK_MS: u64 = 100; // §6: fixed master tick, g0910 = now + 100

// ---------------------------------------------------------------------------
// World-space rendering (see APPROXIMATIONS)

/// Each scene compound carries its OWN world bounds — 1 Living Room at
/// (104,148), 5 Hallway at (143,148), 10 Kitchen at (167,149) — and the blit
/// into the 408×199 canvas (§4 step 2) puts whichever one is current at the
/// canvas origin, so the world origin every other sprite is measured against
/// is the CURRENT scene's `bx/by`, not a single fixed pair.
///
/// Three independent confirmations, one per room, from `mikes-long.mp4`
/// (each measured as the static residue after subtracting the packed scene
/// compound from every sampled frame of that room's on-screen period):
/// the hallway plant `c_2074` (world 517,267) sits at hall-relative
/// (374..401, 119..193) — measured (374..401, 119..193); the hallway's
/// telephone `c_2072` (world 326,303) at hall-relative (183..202, 155..164)
/// — measured exactly that; the kitchen's fridge pinboard `c_854` (world
/// 441,279) at kitchen-relative (274..329, 130..185) — measured
/// (274..328, 131..186); and the living room's swivel desk chair `c_1094`
/// (world 412,288) at living-relative (308..332, 140..192) — measured
/// exactly that, in both living-room periods.
const SCENE_LIVING: u32 = 1;
const SCENE_HALL: u32 = 5;
const SCENE_KITCHEN: u32 = 10;

// ---------------------------------------------------------------------------
// Presentation map (APPROXIMATION — see header). state → (scene, art run).
//
// Every run named here has been checked to actually contain Mike. That is not
// a given: five runs in the packed 1000-series (46, 87, 179, 190, 732) are
// nothing but opaque patches of room wallpaper and wainscot with no figure in
// them at all — c_087 through c_176 are 40×97 rectangles of the hallway's
// purple wall, pixel-identical to the region of scene compound 5 they sit on.
// The RE spec's art catalogue (§3.1) reads those ranges off the composed
// sequence GIFs and calls them "Mike standing idle" and "idle/fidget loop", so
// the figure exists upstream; the packed compounds have lost it. PACK-SIDE
// GAP, reported separately — it is not fixable from this file.
//
// Pointing a state at one of those runs makes Mike vanish for as long as the
// state lasts, which the catch-all did: it covered states 5, 7-17 and 21-35 —
// the whole hallway-idling and long-idle/sleep/scratch half of §5.1, the
// states the planner spends most of its time in — with the 91-frame empty
// patch at 87. Mike was simply not on screen for most of a session. The phone
// block had the same shape of bug from the other side: it drew compound 1094,
// which is the wall telephone PROP (25×53, and already drawn every frame by
// the `g0932`-family prop list), not Mike answering it.
/// `None` = the activity carries no room of its own: its run is a cut-out of
/// Mike alone (the `WALK_RUNS` entries tagged `0`), so it plays in whatever
/// room he is already standing in, and the scene canvas does NOT move.
///
/// That distinction is what the room-change rate hangs on. `mikes-long.mp4`
/// changes room exactly FIVE times in 180 s — living 3.23-10.37, hallway
/// 10.37-31.27, kitchen 31.27-65.30, living 65.30-127.43, hallway
/// 127.43-180 (measured as the frame the 408x199 panel jumps to a new screen
/// position; §canvas errata) — i.e. dwells of 7.1 / 20.9 / 34.0 / 62.1 /
/// 52.6+ s. Returning a hard room for every one of the 95 states made the
/// port cut rooms 16 times in the same 180 s with a median dwell of 5.0 s,
/// because the planner's own graph hops between groups several times a
/// minute and the catch-all dragged every idle state back to the hallway.
/// `g0264` (M129 data +0x264): the 94 (state, sequence) pairs `fn77` @16A4
/// looks the entered state up in — THE per-state run the old header listed
/// as undecoded. Decoded from `emu/ghidra/mikes-so-called-life/blocks/
/// A5_globals.bin`. States 28/29/34/77/78 map to runs 87/46/732/179/190,
/// which this pack carries as empty wallpaper patches (pack-side gap noted
/// below): Mike is genuinely off-screen in those states here.
const STATE_RUN: [(u16, u32); 94] = [
    (2, 1249), (3, 1254), (4, 1244), (5, 1950), (6, 1932), (7, 1863),
    (8, 1880), (9, 1856), (10, 1270), (11, 1299), (12, 1310), (13, 1292),
    (14, 1904), (15, 1327), (16, 1229), (17, 1845), (18, 1338), (19, 1647),
    (20, 1693), (21, 1917), (22, 1237), (23, 1962), (24, 592), (25, 684),
    (26, 635), (27, 265), (28, 87), (29, 46), (30, 777), (31, 785),
    (32, 799), (33, 803), (34, 732), (35, 760), (36, 1362), (37, 1455),
    (38, 2903), (39, 2889), (40, 2332), (41, 2308), (42, 2318), (43, 2897),
    (44, 2362), (45, 2607), (46, 2407), (47, 2866), (48, 2256), (49, 1589),
    (50, 2124), (51, 21), (52, 2005), (53, 2794), (54, 2812), (55, 2751),
    (56, 2847), (57, 1022), (58, 2947), (59, 2788), (60, 494), (61, 511),
    (62, 474), (63, 567), (64, 548), (65, 540), (66, 2032), (68, 2099),
    (67, 2175), (69, 1791), (70, 1735), (71, 2636), (72, 2498), (73, 912),
    (74, 861), (75, 2276), (76, 2282), (77, 179), (78, 190), (79, 230),
    (80, 1117), (81, 1625), (82, 1121), (83, 1187), (84, 1126), (85, 1489),
    (86, 1195), (87, 1144), (88, 1096), (89, 1163), (90, 1206), (91, 1472),
    (92, 1563), (93, 456), (94, 1620), (95, 1557),
];

/// `fn77` @16A4: the run for a state. Rooms are NOT a property of the state
/// any more — `fn02` @1D52 changes them on the EXIT of specific states, see
/// `room_after_exit`.
fn present(state: u16) -> (Option<u32>, u32) {
    let run = STATE_RUN
        .iter()
        .find(|(s, _)| *s == state)
        .map(|(_, r)| *r)
        .unwrap_or(803);
    (None, run)
}

/// `fn72`'s exit hooks (`0x4000|state`): leaving 0x13/0x38/0x3f/0x5c moves
/// the scene to the hallway (compound 5), leaving 0x19/0x1f/0x43 to the
/// kitchen (10), leaving 0x21/0x2c/0x39/0x3a/0x44/0x47/0x48 to the living
/// room (1). Nothing else moves the canvas.
fn room_after_exit(state: u16) -> Option<u32> {
    match state {
        0x13 | 0x38 | 0x3f | 0x5c => Some(SCENE_HALL),
        0x19 | 0x1f | 0x43 => Some(SCENE_KITCHEN),
        0x21 | 0x2c | 0x39 | 0x3a | 0x44 | 0x47 | 0x48 => Some(SCENE_LIVING),
        _ => None,
    }
}

// ---------------------------------------------------------------------------

/// One MySoundPlayer slot (§7.1: 16 slots, 6 bytes each: idIdx, ?, delay).
struct CueSlot {
    ch: u8,
    delay: i16,
}

struct Prop {
    scene: u32, // SCENE_* , or 0 = roams everywhere (the fly)
    first: u32,
    last: u32,
    cur: u32,
    /// Frames per master tick (props authored slower than 10 fps tick).
    rate: u32,
    acc: u32,
    /// One-shot: the run plays through once, then the prop is invisible
    /// until `next_ms`. `false` = the ordinary looping prop.
    oneshot: bool,
    /// One-shot only: currently on screen.
    active: bool,
    /// One-shot only: sim time the next play starts.
    next_ms: u64,
}

pub struct MikesSoCalledLife {
    pack: Pack,

    // --- CSpriteMike (g0952) ---
    state: u16,
    first: u32,
    last: u32,
    cur: u32,
    scene: u32,
    /// Top-left of the scene canvas on the 640×480 screen. Re-rolled on every
    /// room change (`place_scene`) — see the header ERRATA.
    canvas: (i32, i32),
    /// World bounds of the compound currently blitted into the canvas; every
    /// other sprite is drawn at `canvas + (bx+dx, by+dy) - scene_origin`.
    scene_origin: (i32, i32),
    placed: bool,
    entered_ms: u64,       // +0x122: when the current state was entered
    keep_stamp: bool,      // +0x11E: next enter keeps +0x122 (dwell re-pick)
    /// +0x11C: set on EXIT of 0x37, cleared on EXIT of 0x2F / 0x36 (`fn72`
    /// @07E0 / @06F4 / @0716); read by `fn74` for 0x1A/0x42 -> 0x43 and
    /// 0x43 -> 0x2F/0x30. It is NOT a phone flag — nothing on the telephone
    /// side writes it.
    flag_11c: bool,
    schedule_cursor: usize, // +0x120

    // --- MySoundPlayer (g091E) ---
    cues: Vec<CueSlot>, // 16 slots

    // --- RingingPhoneSprite (g0936), fn29 @3DCE / fn39 @417E ---
    /// the 8-slot delay queue at +0x98 (`fn30` @3E90 fills it)
    ring_slots: Vec<i16>,
    /// +0x96: a ring started and the static phone `g0942` is hidden
    ring_96: bool,
    /// the static telephone prop `g0942` (c_2072) is hidden (+0x2C) while
    /// the ringing sprite plays in its place
    phone_hidden: bool,
    /// g0936 shown (+0x34), its frame, and its run-done flag (+0x46)
    ring_shown: bool,
    ring_cur: u32,
    ring_done: bool,

    // --- critter g094A variant picker (MOD.f1AD0) ---
    variant: u32,
    variant_roll_tick: u32,

    // props
    props: Vec<Prop>,

    // master gate (§6 g0910)
    gate: u64,

    // controls
    activity_level: i32, // sVal 1000 raw — read by the code, never used (§1)
    control_slot3: i32,  // the controlValues(3) target: never written (§1)
    started: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

/// Concrete constructor — the tests drive the machine directly (the trait
/// object is not downcastable; engine::Module has no `any` supertrait).
fn build(pack: Pack) -> Option<MikesSoCalledLife> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    let prop = |scene: u32, first: u32, rate: u32| -> Prop {
        let last = first + pack.seq_len(BASE, first) - 1;
        Prop {
            scene,
            first,
            last,
            cur: first,
            rate,
            acc: 0,
            oneshot: false,
            active: true,
            next_ms: 0,
        }
    };
    Some(MikesSoCalledLife {
        // §2.2 cast, initial sequences per DoSetUp @0x2448–0x2D30 (exact).
        props: vec![
            // g092E's DoSetUp sequence is 581, the decorated Christmas tree
            // (46x100 at world 489,241) — and the tree is NEVER DRAWN. Its
            // world box puts it at living-relative 385..430 (over and past
            // the right-hand radiator, half of it off the 408 px canvas),
            // hall-relative 346..391 and kitchen-relative 322..367.
            // `mikes-long.mp4` covers all three rooms — living 3.2-10.4 s
            // and 65.3-127.4 s (69 s), hallway 10.4-31.3 s and 127.4-180 s
            // (74 s), kitchen 31.3-65.3 s (34 s) — and subtracting the
            // packed scene compound from every sampled frame leaves NOTHING
            // at any of those three boxes in any room: the living room's
            // only static overlay is the desk chair below, the hallway's are
            // the plant and the telephone, the kitchen's is the pinboard.
            // §2.1's per-sprite draw is gated on `this->0x54`, so the roster
            // can hold a sprite the module never shows; the tree is one.
            // It stays out of the draw list, roster note kept here.
            //
            // c_1094 is NOT the "wall telephone" of §3.1 — it is the living
            // room's swivel DESK CHAIR, a grey seat on a black post and
            // base, standing in front of the Mac desk. Confirmed as the
            // living room's one static overlay in BOTH living-room periods
            // of `mikes-long.mp4`, at living-relative (308..332, 140..192),
            // which is exactly its world box (412,288)+25x53. The actual
            // telephone is c_2072, in the hallway (see `present`).
            prop(SCENE_LIVING, 1094, 0),  // desk chair (static)
            // c_1443 is the Mac's SCREEN: 18x14 at world (405,269) =
            // living-relative (301..318, 121..134), which lands exactly on
            // the blank monitor `c_001` paints there. `mikes-long.mp4`
            // 70-110 s shows a small white shape swimming across that screen
            // while the packed compound's screen is solid black.
            prop(SCENE_LIVING, 1443, 4),  // g0932 Mac screen saver (18×14)
            // g092A (10×22) and g0942 (20×10) are the HALLWAY table's lamp
            // flame and telephone, not living-room furniture. Composited into
            // each of the three scenes at their own world bounds, both land
            // exactly on the hall's side table, and the golden capture's
            // hallway (emu/captures/mikes-life.mp4 @40 s) shows precisely that
            // table with a lit lamp and a black rotary phone on it, while the
            // golden living room (d32-mikes/g_07.png) has plain dresser top
            // where the module was drawing them. This also makes §7.2 read
            // straight: RingingPhoneSprite "wiggles the g0942 prop" because
            // g0942 IS the ringing telephone, jiggling on its table.
            prop(SCENE_HALL, 2077, 4),  // g092A hall lamp flame (10×22)
            prop(SCENE_HALL, 2072, 0),  // g0942 hall telephone (20×10)
            prop(SCENE_KITCHEN, 2224, 4), // g0926 prop (15×7)
            // g0922 is the DAYDREAM: a 57-frame one-shot thought bubble
            // that starts as an 8x6 speck at world (297,268), balloons to
            // 62x48 with a FLYING TOASTER inside it, then drifts up and to
            // the left on cumulative dx/dy and ends 77x60 at world
            // (149,103). It is neither a fly nor a loop.
            //
            // `mikes-long.mp4` shows exactly one play in 180 s, in the
            // kitchen: the bubble appears at 54.4 s beside Mike's head at
            // kitchen-relative (92..157, 71..111) — the run's frames 3-32
            // box, (85..146, 71..118) — the toaster is legible inside it at
            // 57.8 s, it has drifted up-left to kitchen-relative (68..138)
            // by then, and it is gone by 60.4 s. 57 frames at one frame per
            // master tick is 5.7 s, which is the length measured.
            //
            // The previous pass pinned first==last==2438 to stop a giant
            // toaster cycling across the scene ten times a second; the
            // capture says the toaster is real and the cycling was the bug.
            // Playing it as a one-shot with a long gap gets both.
            Prop {
                scene: 0,
                first: 2438,
                last: 2438 + 56,
                cur: 2438,
                rate: 1,
                acc: 0,
                oneshot: true,
                active: false,
                // don't open the session on a daydream
                next_ms: 45_000,
            },
            // g093A is not a "fly/bug sprite" — compound 854 is a 56×56
            // OPAQUE green pinboard with two notes on it, and it was set to
            // roam every room (`scene: 0`), so it sat on top of the living
            // room's oval mirror and blocked the hallway's right-hand doorway
            // in every frame. Both goldens rule those rooms out (g_07.png has
            // the mirror clean; the capture's hallway has an open doorway), so
            // it goes to the one room neither golden covers.
            prop(SCENE_KITCHEN, 854, 0),  // g093A green pinboard (56×56)
            // g093E is the hallway's potted plant: scene 5 paints the pot,
            // compound 2074 the foliage above it (it matches no scene's
            // pixels, so it is furniture drawn on top). Roaming, it grew a
            // second plant in the living room and the kitchen.
            prop(SCENE_HALL, 2074, 0),    // g093E hall plant (28×75)
        ],
        pack,
        state: 0,
        first: 87,
        last: 87,
        cur: 87,
        scene: SCENE_LIVING,
        canvas: (128, 128),
        scene_origin: (104, 148),
        placed: false,
        entered_ms: 0,
        keep_stamp: false,
        flag_11c: false,
        schedule_cursor: 0,
        cues: Vec::with_capacity(16),
        ring_slots: Vec::with_capacity(8),
        ring_96: false,
        phone_hidden: false,
        ring_shown: false,
        ring_cur: RINGING_PHONE_RUN,
        ring_done: true,
        variant: 2072,
        variant_roll_tick: 0,
        gate: 0,
        // §1: sVal 1000 "Activity Level", default 0x0032 = 50 (Listless).
        activity_level: 50,
        control_slot3: 0,
        started: false,
    })
}

impl MikesSoCalledLife {
    /// §1/§8.1: DoSetUp @0x22F4 reads `controlValues(3)` — control index 3
    /// of a ONE-control module — clamps `>0 → 1 else 0` into MOD.g090E, and
    /// the tick handler branches schedule-vs-random on it (@0x0C86–0x0C9E).
    /// Index 3 reads nothing, so g090E is always 0 and the scripted day is
    /// dead code. The slider itself is never consulted. ORIGINAL BUG kept.
    fn schedule_mode(&self) -> bool {
        self.control_slot3 > 0
    }

    /// §5.2 fn73_0CD6 PickFromTable: `next = window[Random15() % count]`;
    /// the no-repeat calls re-roll while the pick equals the current state.
    fn pick(&self, ctx: &mut Ctx, off: u32, count: usize, avoid_current: bool) -> u16 {
        let w = &DATA_WINDOWS[widx(off)..widx(off) + count];
        loop {
            let next = w[ctx.rng15.below(count as u16) as usize];
            if !avoid_current || next != self.state {
                return next;
            }
        }
    }

    /// §5.4 MOD.f1556: walk `this->0x120` through DAY_SCHEDULE; a value of
    /// 1 is the end-of-day marker that resets the cursor (@0x156E–0x1576).
    fn schedule_next(&mut self) -> u16 {
        let mut v = DAY_SCHEDULE[self.schedule_cursor];
        self.schedule_cursor += 1;
        if v == 1 {
            self.schedule_cursor = 0;
            v = DAY_SCHEDULE[0];
            self.schedule_cursor = 1;
        }
        v
    }

    /// §5.2 fn74_0D12 ChooseNextState — the planner, verbatim switch.
    fn choose_next_state(&mut self, ctx: &mut Ctx) -> u16 {
        if self.schedule_mode() {
            return self.schedule_next();
        }
        let now = ctx.now_ms;
        let s = self.state;
        match s {
            // fn74 dwell cases: while `now < +0x122 + N` re-pick the same
            // state and set +0x11E so the re-enter keeps the stamp
            // (@0x1058/@0x10D2/@0x111E/@0x1290); once expired, move on.
            22 => {
                if now < self.entered_ms + DWELL_SLEEP_MS {
                    self.keep_stamp = true;
                    22
                } else {
                    self.pick(ctx, 0x001E, 12, false)
                }
            }
            11 => {
                if now < self.entered_ms + DWELL_IDLE_MS {
                    self.keep_stamp = true;
                    11
                } else {
                    // @0x10F0 pushes repeatOK = 1 → repeat allowed
                    self.pick(ctx, 0x0046, 3, false)
                }
            }
            14 => {
                if now < self.entered_ms + DWELL_IDLE_MS {
                    self.keep_stamp = true;
                    14
                } else {
                    self.pick(ctx, 0x004C, 4, true) // clr.w @0x113C
                }
            }
            35 => {
                if now < self.entered_ms + DWELL_IDLE_MS {
                    self.keep_stamp = true;
                    35
                } else {
                    // @0x12B0 pushes repeatOK = 1 → repeat allowed
                    self.pick(ctx, 0x0098, 5, false)
                }
            }
            // the TV group @0x13A0: for 3 s after sitting down the pick is
            // the 11-entry channel window (0xC0, repeat allowed, +0x11E kept
            // so the 3 s runs from the FIRST sit); after that the 18-entry
            // window (0xD6, clr.w @0x13CC → no repeat) that also gets up
            // (90..94). The earlier port had the two branches swapped.
            49 | 80 | 81 | 82 | 84 | 86 | 88 | 95 => {
                if now < self.entered_ms + DWELL_TV_MS {
                    self.keep_stamp = true;
                    self.pick(ctx, 0x00C0, 11, false)
                } else {
                    self.pick(ctx, 0x00D6, 18, true)
                }
            }

            // constant transitions
            4 => 2,
            6 => 5,
            8 => 14,
            13 => 11,
            24 => 34,
            25 | 31 => 48,
            30 => 31,
            32 => 33,
            33 | 68 | 71 => 49,
            34 => 35,
            // @0x1184 (cases 18, 91): loads 36 — straight to the mirror
            36 => 37,
            37 | 64 => 95,
            38 => 55,
            43 => 38,
            44 | 72 => 69,
            45 => 71,
            46 => 72,
            50 | 52 => 66, // @0x13DC / @0x1400: loads 66 (52 only — spec §5.2 lumps 53 in)
            56 => 50,
            57 => 64,
            58 => 70,
            65 => 64,
            69 => 6,
            70 => 7,
            73 => 75,
            74 => 73,
            75 => 76,
            85 => 95,
            94 => 85,

            // 26/66 (0x1A/0x42) -> 67 when +0x11C is set; see `flag_11c`
            // — the earlier "ring flag" reading was wrong
            26 | 66 => {
                if self.flag_11c {
                    67
                } else {
                    self.pick(ctx, 0x0076, 2, true)
                }
            }
            67 => {
                if self.flag_11c {
                    47
                } else {
                    48
                }
            }

            // weighted windows (offset, count, repeatOK). Counts/flags are
            // taken from the DISASM push sites (table address, count, flag),
            // which corrects three transcription slips in the §5.2 prose
            // table: 27/28/77/78 push count 7 (not 8, @0x122A), 47/54 push
            // count 3 (not 4, @0x1356), and 53 routes to window 0xFE with
            // 55/59 (not to 66, @0x0F6C→0x1408).
            2 => self.pick(ctx, 0x001A, 2, false),
            3 | 5 | 10 | 12 | 21 | 90 => self.pick(ctx, 0x001E, 12, false),
            7 | 9 | 17 => self.pick(ctx, 0x0036, 8, false),
            15 => self.pick(ctx, 0x0054, 3, true),
            16 => self.pick(ctx, 0x005A, 4, true),
            18 | 91 => 36, // mirror — see constant above (@0x1184)
            19 | 63 | 92 => self.pick(ctx, 0x0062, 3, true),
            20 | 93 => self.pick(ctx, 0x0068, 4, true),
            23 => self.pick(ctx, 0x0070, 3, true),
            27 | 28 | 77 | 78 => self.pick(ctx, 0x007A, 7, false), // @0x122A: 7
            29 | 79 => self.pick(ctx, 0x0088, 8, true),
            39 => self.pick(ctx, 0x00A2, 2, true),
            40 => self.pick(ctx, 0x00A6, 4, true),
            41 | 42 => self.pick(ctx, 0x00AE, 2, false),
            47 | 54 => self.pick(ctx, 0x00B2, 3, true), // @0x1356: 3
            53 | 55 | 59 => self.pick(ctx, 0x00FE, 9, false),
            48 => self.pick(ctx, 0x00B8, 4, true),
            51 => self.pick(ctx, 0x00FA, 2, true),
            60 | 61 | 62 => self.pick(ctx, 0x0110, 6, true),
            76 => self.pick(ctx, 0x011C, 2, true),
            83 => self.pick(ctx, 0x0120, 7, true),
            87 => self.pick(ctx, 0x012E, 7, false),
            89 => self.pick(ctx, 0x013C, 4, true),

            // ORIGINAL BUG kept (§8.2): the debug printf ships in release.
            _ => {
                eprintln!("CSpriteMike::ChooseNextState({})", s);
                s
            }
        }
    }

    /// `fn21` @3AF2 CueSound: a negative delay is refused with the shipped
    /// diagnostic; otherwise the first free slot (+0x22 == -1) of 16 takes
    /// (channel, delay); full = dropped with "List full!" (§8.8).
    fn cue(&mut self, ch: u8, delay: i16) {
        if delay < 0 {
            eprintln!("MySoundPlayer::CueSound() called with aFrameDelay < 0!");
            return;
        }
        if self.cues.len() >= 16 {
            eprintln!("MySoundPlayer::CueSound(): List full!");
            return;
        }
        self.cues.push(CueSlot { ch, delay });
    }

    /// `fn22` @3BBC, the pump, last thing in `fn45`'s frame: a live slot
    /// whose counter IS 0 plays its channel (`fn18` @3A14 lookup), then
    /// every live counter is decremented — so delay `d` sounds on the
    /// `d+1`-th pump, the same frame for `d = 0`, and the slot frees itself
    /// at -1.
    fn pump_sounds(&mut self, ctx: &mut Ctx) {
        let mut due = Vec::new();
        self.cues.retain_mut(|slot| {
            if slot.delay == 0 {
                due.push(CHANNELS[slot.ch as usize]);
            }
            slot.delay -= 1;
            slot.delay >= 0
        });
        for snd in due {
            if snd != 0 && self.pack.sound(snd).is_some() {
                ctx.sounds.push(snd);
            }
        }
    }

    /// `fn72`'s ENTER hooks: the per-state cue lists (`ENTER_CUES`) and the
    /// ringing-telephone queue (`ENTER_RINGS`). A dwell re-pick of the same
    /// state (the chooser returning 22 while 22 is still dwelling) is a
    /// full exit+enter in the C and fires these again — harmless, because
    /// none of the dwell states (0x0B/0x0E/0x16/0x23) has a hook.
    fn enter_hooks(&mut self, new: u16) {
        if let Some((_, list)) = ENTER_CUES.iter().find(|(s, _)| *s == new) {
            for &(ch, delay) in list.iter() {
                self.cue(ch, delay);
            }
        }
        if let Some((_, list)) = ENTER_RINGS.iter().find(|(s, _)| *s == new) {
            for &d in list.iter() {
                // fn30 @3E90: first free (-1) slot of 8; none free = dropped
                if self.ring_slots.len() < 8 && d >= 0 {
                    self.ring_slots.push(d);
                }
            }
        }
    }

    /// `fn72`'s EXIT hooks that touch Mike's own fields: +0x11C.
    /// (0x402F, like ENTER 0x36, also arms `g094E`'s close flag when it is
    /// open — GAP(g094E): the kitchen sprite's `fn61`-`fn64` are not ported.)
    fn exit_hooks(&mut self, old: u16) {
        match old {
            0x2F | 0x36 => self.flag_11c = false, // @06F4 / @0716
            0x37 => self.flag_11c = true,         // @07E0
            _ => {}
        }
    }

    /// Put a run on screen from its first frame.
    fn start_run(&mut self, art: u32) {
        self.first = art;
        self.last = art + self.pack.seq_len(BASE, art) - 1;
        self.cur = art;
    }

    /// SetState: EXIT(old) then ENTER(new), `fn72` @00F2.
    fn transition(&mut self, ctx: &mut Ctx, new: u16) {
        let old = self.state;
        // exit(old) — fn72's `0x4000|state` hooks: fn02 @1D52 moves the
        // scene for a handful of states; every other transition keeps the
        // canvas where it is.
        self.exit_hooks(old);
        let scene = room_after_exit(old).unwrap_or(self.scene);
        if scene != self.scene || !self.placed {
            self.place_scene(ctx, scene);
        }
        // enter(new): the hook body first, then the common 0x8000 tail
        // @0C2A — stamp +0x122 unless +0x11E asks to keep it, then fn77:
        // table lookup, SetRun, fn76 re-place. No walk is synthesised: the
        // authored runs (walks included) are the itinerary.
        self.state = new;
        self.enter_hooks(new);
        if self.keep_stamp {
            self.keep_stamp = false;
        } else {
            self.entered_ms = ctx.now_ms;
        }
        let (_, art) = present(new);
        self.start_run(art);
    }

    /// `g0936` RingingPhoneSprite's frame: `fn29` @3DCE then `fn39` @417E.
    /// Runs after Mike in `fn45`'s order, so a delay-0 slot queued by
    /// Mike's ENTER this frame fires this frame.
    fn phone_tick(&mut self) {
        // fn29: run finished with +0x96 up -> the static phone comes back
        if self.ring_done && self.ring_96 {
            self.ring_96 = false;
            self.phone_hidden = false; // g0942 +0x30
        }
        let mut fire = false;
        self.ring_slots.retain_mut(|d| {
            if *d == 0 {
                fire = true;
            }
            *d -= 1;
            *d >= 0
        });
        if fire {
            // +0x96 = 1, hide g0942, fn41: SetRun(0x7C7) + fn40 re-place
            // (room centre + link(room frame -> run), i.e. world coords in
            // WHATEVER room is current — fn02 never hides g0936; quirk kept)
            // + show.
            self.ring_96 = true;
            self.phone_hidden = true;
            self.ring_cur = RINGING_PHONE_RUN;
            self.ring_done = false;
            self.ring_shown = true;
        }
        // fn39: shown -> advance; once done, +0x94 is 0 (fn38's ctor arg)
        // so the sprite hides instead of replaying
        if self.ring_shown {
            if !self.ring_done {
                let last = RINGING_PHONE_RUN + self.pack.seq_len(BASE, RINGING_PHONE_RUN) - 1;
                if self.ring_cur < last {
                    self.ring_cur += 1;
                } else {
                    self.ring_done = true;
                }
            } else {
                self.ring_shown = false;
            }
        }
    }

    /// Critter g094A variant picker, MOD.f1AD0: 1-in-3 gate, then a random
    /// variant different from the current one.
    ///
    /// Draws from `Random15`, like every other random in this module — §0's
    /// linkage table lists exactly one generator for `DynaMikes`
    /// (`Resource.fn_3AF0` = `Random15`), seeded once from `sEEd 0` via
    /// `RandomSeedFromTicks` (§4 step 8). There is no second stream, and
    /// routing these draws to the harness's `RandomLong` was not just
    /// unfaithful, it was load-bearing: `Random15` is the Berkeley LCG whose
    /// low bit alternates on every draw (`rand() % 2` strictly alternates —
    /// the engine's own doc comment says so), so a tick that always spends the
    /// SAME number of draws pins every `% 2` pick to one answer forever.
    /// State 2's window is `{2, 3}` with repeat allowed, so with a constant
    /// two draws per tick (the 1-in-128 roll plus the planner's pick) the
    /// planner re-picked state 2 on every tick of a ten-minute run and Mike
    /// never left the hallway walk. Putting the critter's periodic 1-in-3
    /// gate back on the shared stream restores a varying draw count, which is
    /// what the original has anyway — its 95 undecoded handler bodies (§9) are
    /// `Random15` callers too.
    fn critter_tick(&mut self, ctx: &mut Ctx) {
        self.variant_roll_tick += 1;
        if self.variant_roll_tick >= 30 {
            self.variant_roll_tick = 0;
            if ctx.rng15.below(3) == 0 {
                const VARIANTS: [u32; 3] = [854, 2072, 2074];
                loop {
                    let v = VARIANTS[ctx.rng15.below(3) as usize];
                    if v != self.variant {
                        self.variant = v;
                        break;
                    }
                }
            }
        }
    }

    fn prop_tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        for p in &mut self.props {
            if p.oneshot {
                if !p.active {
                    if now >= p.next_ms {
                        p.active = true;
                        p.cur = p.first;
                        p.acc = 0;
                    }
                    continue;
                }
                p.acc += 1;
                if p.acc >= p.rate.max(1) {
                    p.acc = 0;
                    p.cur += 1;
                    if p.cur > p.last {
                        p.active = false;
                        p.cur = p.first;
                        // one play per 180 s capture; keep it rare.
                        p.next_ms = now + 60_000 + (ctx.rng15.below(120) as u64) * 1000;
                    }
                }
                continue;
            }
            if p.rate == 0 || p.last == p.first {
                continue;
            }
            p.acc += 1;
            if p.acc >= p.rate {
                p.acc = 0;
                p.cur += 1;
                if p.cur > p.last {
                    p.cur = p.first;
                }
            }
        }
    }

    /// World position of a compound: bounds origin PLUS the frame's own link
    /// offset.
    ///
    /// `dx/dy` is the OFtb link offset the compositor applies when the run
    /// enters this frame, and for every sequence that moves Mike across a room
    /// it is where the travel actually lives: the walk cycles re-use the same
    /// handful of poses over and over, so `bx` cycles too, and `dx` steps up by
    /// a cycle's width each time round. Reading `bx` alone therefore replays
    /// a walk as a stutter that snaps backwards every five or six frames
    /// (`seq 2005`: `bx` jumps 138 px between adjacent frames, `bx+dx` never
    /// more than 13), and it turns the 3-frame armchair loop `seq 1121`
    /// (`bx` = 444, 357, 444 · `dx` = -87, 0, -87) into Mike flicking 87 px
    /// sideways ten times a second while he is supposed to be *sitting still*.
    /// That is the "disappearing and reappearing all over his rooms" report:
    /// not the state machine (it commits a handful of times a minute) but the
    /// draw call, dropping half of every frame's coordinate. §6: "walking
    /// sequences carry cumulative link offsets that span the full 408 px room
    /// (compounds 2005/2124/2636)."
    fn world_xy(&self, f: &engine::FrameRef) -> (i32, i32) {
        (
            self.canvas.0 + f.bx + f.dx - self.scene_origin.0,
            self.canvas.1 + f.by + f.dy - self.scene_origin.1,
        )
    }

    /// Re-place the 408×199 scene canvas at a fresh random spot on the
    /// 640×480 screen, and re-anchor the world origin on the new scene's own
    /// bounds. Called on every room change — see `roll_canvas`.
    fn place_scene(&mut self, ctx: &mut Ctx, scene: u32) {
        self.scene = scene;
        let (origin, size) = match self.pack.frame(BASE, scene) {
            Some(f) => {
                let img = self.pack.image(&f.png);
                ((f.bx + f.dx, f.by + f.dy), (img.w as i32, img.h as i32))
            }
            None => ((104, 148), (408, 199)),
        };
        self.scene_origin = origin;
        let sx = (engine::SCREEN_W - size.0).max(0) as u16;
        let sy = (engine::SCREEN_H - size.1).max(0) as u16;
        self.canvas = (
            ctx.rng15.below(sx + 1) as i32,
            ctx.rng15.below(sy + 1) as i32,
        );
        self.placed = true;
    }

    fn draw_frame(&self, fno: u32, out: &mut Vec<SpriteDraw>) {
        let Some(f) = self.pack.frame(BASE, fno) else { return };
        let (x, y) = self.world_xy(f);
        out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x, y });
    }
}

impl Module for MikesSoCalledLife {
    fn name(&self) -> &'static str {
        "Mike's So-called Life"
    }

    /// §1: exactly one control, `sVal 1000` "Activity Level", slider raw
    /// 0..100, default 0x0032 = 50. Tick labels from `sUnt 128`:
    /// 0 Dull · 20 Boring · 30 Tedious · 40 Tiresome · 50 Listless (default)
    /// 60 Saurian · 70 Languorous · 80 Well-Fed Dog · 90 Hypnotized
    /// Butterfly. TEXT 1000: "Activity Level: Frankly, this control does
    /// absolutely nothing." (§8.1) — it exists, it is read into
    /// `activity_level` for parity with the disasm, and it does nothing.
    fn controls(&self) -> Vec<ControlDef> {
        vec![ControlDef {
            name: "Activity Level".into(),
            kind: ControlKind::Slider { min: 0, max: 100 },
            default: 50,
        }]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            // stored, never consulted — exactly like the original (§1)
            0 => self.activity_level = value.clamp(0, 100),
            _ => {}
        }
    }

    /// §6 DoDrawFrame (MOD.f3092): master gate `now < g0910 → return`,
    /// else `g0910 = now + 100` — a fixed 100 ms tick (§8.4). Per tick:
    /// Mike steps, critters/props tick, then the MySoundPlayer pump.
    fn tick(&mut self, ctx: &mut Ctx) {
        // §6 master gate (skip-if-late)
        if ctx.now_ms < self.gate {
            return;
        }
        self.gate = ctx.now_ms + TICK_MS;

        if !self.started {
            self.started = true;
            // prime Mike via the StartStateSequence wrappers (§4 step 5);
            // day opens in the living room
            self.transition(ctx, 80);
        }

        // fn45 @308E order: Mike's state object first, then g094E, g094A,
        // g0936 (the telephone), the props, the draw, and the pump last.
        //
        // REMOVED 2026-09-26: the "1-in-128 surprise pose" (art 456 drawn for
        // ~1 s + Feh! on `(Random15() & 0x7F) == 0x45`). That roll is `fn62`
        // @185A, the update of `g094E` (vtbl g058A, a KITCHEN sprite whose
        // close run is 0xB44) — not Mike's. Nothing in Mike's handler draws
        // 456 outside state 93 (its `g0264` run), and 456 drawn at its own
        // world spot was every in-room teleport Jason saw ("blinked to the
        // back standing up, then back into the chair"). Feh! (channel 19) is
        // queued only by ENTER 0x4A. GAP(g094E): fn61–fn64 not ported.
        //
        // REMOVED 2026-09-26: the self-scheduled phone (a ring every 20–50 s
        // with a 30 s answer window driving +0x11C). The C rings ONLY from
        // Mike's ENTER hooks 0x1A/0x1B/0x34 (channel-14 cues + g0936's queue);
        // +0x11C is Mike's own flag (see `exit_hooks`).

        // fn72's update path @0C7A: while the run is still going, tick it;
        // once it has played out, choose the next state (fn75 schedule if
        // g090E, else fn74) and SetState it — exit(old) then enter(new),
        // which restarts the run even when the pick is the state Mike is in.
        // fn72's update path @0C7A: while the run is still going, tick it;
        // once it has played out, choose the next state (fn75 schedule if
        // g090E, else fn74) and SetState it — exit(old) then enter(new),
        // which restarts the run even when the pick is the state Mike is in.
        if self.cur < self.last {
            self.cur += 1;
        } else {
            let next = self.choose_next_state(ctx);
            self.transition(ctx, next);
        }

        self.phone_tick();
        self.prop_tick(ctx);
        self.critter_tick(ctx);
        self.pump_sounds(ctx); // §6 step 6: MOD.f3BC0 pump, last
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // scene canvas g0962 (§2.2): compounds 1 / 5 / 10
        self.draw_frame(self.scene, out);

        // props of the current room (§2.2 roster; assignments approximated)
        for p in &self.props {
            if p.scene != 0 && p.scene != self.scene {
                continue;
            }
            if p.oneshot && !p.active {
                continue;
            }
            // g0942, the static telephone, is hidden while g0936 rings
            if self.phone_hidden && p.first == 2072 {
                continue;
            }
            self.draw_frame(p.cur, out);
        }

        // g0936 the ringing telephone (run 0x7C7), world coords in the
        // current room
        if self.ring_shown {
            self.draw_frame(self.ring_cur, out);
        }

        // Mike (g0952)
        self.draw_frame(self.cur, out);

        // Variant critter g094A. The picker itself is real (MOD.f1AD0), but
        // DoSetUp gives g094A no initial sequence — §2.2 lists one for every
        // other cast member and none for this one — so the three sequences it
        // cycles through here (854 / 2072 / 2074) are borrowed from the OTHER
        // critters' documented initials. Drawn unconditionally, that meant a
        // green pinboard, a telephone and a potted plant popping into
        // whichever room Mike happened to be in, on top of the copy the prop
        // list was already drawing in the room each belongs to. Draw the
        // variant only in its own room; where it duplicates the prop it is a
        // no-op, and everywhere else the room stops sprouting furniture.
        if self.props.iter().any(|p| p.first == self.variant && p.scene == self.scene)
            && !(self.phone_hidden && self.variant == 2072)
        {
            self.draw_frame(self.variant, out);
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// §6/§8.4's master tick is `g0910 = now + 100` on the `fn_4724`
    /// (`TickCount()*16.625`, integer) clock. On the real grid a `>= 100`
    /// gate fires on the 7,6,7,6,6 cycle = 106.40 ms mean, which is
    /// `mikes-long.mp4`'s measured 106.5 (R = 0.656 vs 0.096 for a flat
    /// 100). The old `tick_ms = 17` approximation gave 102. See the header
    /// ERRATA and engine `TickClock`.
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }
}

// ---------------------------------------------------------------------------
// Visual smoke test (headless): pack → make() → 500 ticks, no panics,
// sprites produced.

#[cfg(test)]
mod tests {
    use super::*;

    /// Advance `ctx` by one shell tick on the module's clock (the Mac tick
    /// grid — see `clock()`), the way every shell drives it. Tick index is
    /// recovered from `now_ms` exactly (mac_clock_ms(t) ∈ [16.625t−1, 16.625t]).
    fn tick(ctx: &mut Ctx) {
        let t = (ctx.now_ms as f64 / 16.625).round() as u64 + 1;
        ctx.now_ms = TickClock::MacTick.now_ms(t);
    }
    use engine::{Random15, RandomLong};

    #[test]
    fn mikes_so_called_life_smoke() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let Some(mut module) = make(pack) else {
            panic!("make() returned None although series 1000 exists");
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
        let mut produced = 0;
        for _ in 0..500 {
            tick(&mut ctx);
            module.tick(&mut ctx);
            let mut out = Vec::new();
            module.sprites(&mut out);
            produced += out.len();
        }
        assert!(produced > 0, "no sprites produced in 500 ticks");
        eprintln!("mikes-so-called-life: 500 ticks ok, {produced} sprites drawn");
    }

    /// CONTRADICTIONS 2026-09-13, item 9: the cue table lives at
    /// `DATA 129 @0xA90` with the two shared rows at `@0xA88`/`@0xA8C` and
    /// the `0xFFFF` terminator at `@0xAEC`. Neither §7.1's `@0x008A`/`@0x0080`
    /// nor the old byte-verified `@0xA92`/`@0xA8A`/`@0xA8E` is right; the
    /// latter set is uniformly +2, naming each row's delay word instead of
    /// its id word. This test re-checks the layout from the raw bytes so the
    /// citation and the constant cannot drift apart again.
    #[test]
    fn the_channel_table_starts_at_0xa88() {
        // The image lives in the RE repo, which the port does not depend on;
        // skip cleanly when it is not checked out beside us.
        let p = std::path::Path::new(
            "../../totally-twisted/ripped/mike's-so-called-life/\
             Mike's So-called Life_DATA_129_DynaMikes.bin",
        );
        let Ok(d) = std::fs::read(p) else {
            eprintln!("DATA_129 image not available — skipping");
            return;
        };
        let w = |off: usize| u16::from_be_bytes([d[off], d[off + 1]]);
        // fn17 @3950: one channel per 4-byte row from g0A88, in order, until
        // the first negative id word
        for (i, &want) in CHANNELS.iter().enumerate() {
            let off = 0xA88 + i * 4;
            assert_eq!(u32::from(w(off)), want, "channel {i} at {off:#X}");
            // second word = LoadSound flag: 0 shared, 1 module-local
            assert_eq!(w(off + 2), u16::from(want < 30_000), "flag {i} at {:#X}", off + 2);
        }
        assert!((w(0xA88 + CHANNELS.len() * 4) as i16) < 0, "terminator");
        // the ring cue the phone states push is channel 14 = Phone Ringing,
        // the dial tone after it channel 15 — the base is right
        assert_eq!((CHANNELS[14], CHANNELS[15]), (1016, 1017));
    }

    /// The planner must actually walk the 95-state machine (both drivers:
    /// the shipped random mode AND the dead schedule path, driven directly
    /// here since controlValues(3) can never set it — §8.5) and the
    /// MySoundPlayer must emit ids from the §7.1 cue table.
    #[test]
    fn mikes_so_called_life_machine_coverage() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
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
        let mut sounds_fired: Vec<u32> = Vec::new();
        // 20000 shell ticks = ~3300 master frames = ~5.6 minutes of sim time;
        // this asserts the cue fires AT ALL, and the scratch states are a
        // small slice of a 95-state weighted walk.
        for _ in 0..20000 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            sounds_fired.append(&mut ctx.sounds);
        }
        eprintln!("final state {}", m.state);
        assert!(m.state >= 2 && m.state <= 95, "state out of range");
        assert!(!sounds_fired.is_empty(), "no ENTER cue fired in 5.6 minutes");
        assert!(
            sounds_fired.iter().all(|s| CHANNELS.contains(s)),
            "a sound outside the g0A88 channel table fired: {sounds_fired:?}"
        );
        // dead schedule path driven directly (g090E = 1: controlValues(3) > 0,
        // unreachable in the shipped wiring — §5.4/§8.5)
        let mut m2 = build(
            Pack::load(std::path::Path::new("../assets/mikes-so-called-life")).unwrap(),
        )
        .unwrap();
        m2.control_slot3 = 1; // the vestigial branch, exercised for coverage
        let mut seen = std::collections::HashSet::new();
        for _ in 0..2000 {
            tick(&mut ctx);
            m2.tick(&mut ctx);
            seen.insert(m2.state);
        }
        assert!(
            m2.schedule_cursor > 5,
            "schedule cursor stuck at {} (day walk not advancing)",
            m2.schedule_cursor
        );
        assert!(
            seen.len() > 5,
            "schedule mode only reached {seen:?} states — day walk looks stuck"
        );
    }

    /// Mike's sprite must never walk off the end of the run his state
    /// selected.
    ///
    /// The planner runs every master tick (§5.3) but the commit only lands
    /// when the run finishes, and it is completely normal for the pick to be
    /// the state Mike is already in — every repeat-allowed window can return
    /// it, and the dwell cases (@0x1058/0x10D2/0x111E/0x1290) return it by
    /// construction. `Random15` makes that the *common* case here: the
    /// engine's `rand() % n` alternates strictly, and with exactly one
    /// intervening `Random15()` draw per tick (the 1-in-128 reaction roll) a
    /// two-entry window such as state 2's `{2,3}` samples the same slot
    /// every single tick.
    ///
    /// That path used to leave `cur` one past `last` and then keep
    /// incrementing, and because `Pack::frame` resolves any frame number in
    /// the series, Mike marched clean out of his own sequence and on through
    /// unrelated compounds at 10 fps (measured 210 frames past the end of a
    /// 26-frame walk run) — while `cur > last` also stayed true, so the
    /// planner committed a fresh state, scene and room *every tick*. Both
    /// halves of "stuttery and way too fast".
    #[test]
    fn mike_never_runs_past_the_end_of_his_sequence() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        // both the shell seeds and the harness seeds — the Random15
        // alternation makes the failure seed-sensitive
        for (a, b) in [(1u64, 1u32), (0x5EED_CAFE, 0xC0FFEE)] {
            let mut m = build(pack.clone()).unwrap();
            let mut ctx = Ctx {
                rng: RandomLong::new(a),
                rng15: Random15::new(b),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: 0,
                local_hms: (12, 0, 0),
                mouse: (320, 240),
                mouse_down: false,
            };
            for i in 0..6000u64 {
                tick(&mut ctx);
                m.tick(&mut ctx);
                assert!(
                    m.cur >= m.first && m.cur <= m.last,
                    "tick {i}: state {} run {}..={} but cur={}",
                    m.state,
                    m.first,
                    m.last,
                    m.cur
                );
            }
        }
    }

    /// §6's 1-in-128 reaction is a *pose*, not a permanent state. The
    /// countdown was set and never decremented, so the first hit (expected
    /// inside ~13 s at 10 fps) parked Mike on art 456 for the rest of the
    /// session and he stopped animating entirely.
    /// Tick-quantization sweep, 2026-09-12. §6/§8.4's master gate is
    /// `g0910 = now + 100` against `Resource.fn_4724() = TickCount()*16.625`,
    /// so it fires on the first Mac tick at or past +100 ms and can never be
    /// a clean 100. `mikes-long.mp4` rules a flat 100 out: the Rayleigh
    /// periodogram of its screen-change events over t = 20-45 s peaks at
    /// 106.50 ms with R = 0.656, against R = 0.096 at 100 ms exactly.
    /// Ticking the shell on the Mac grid puts the master frame at 6 ticks.
    #[test]
    fn master_tick_lands_on_the_mac_tick_grid() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        assert_eq!(m.clock(), TickClock::MacTick, "mikes gates on fn_4724");
        let mut ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        // the body only runs when the 100 ms gate fires — on the integer
        // Mac clock a `>=` gate lands on the 7,6,7,6,6 cycle = 106.4 ms mean
        let mut ran: Vec<u64> = Vec::new();
        let mut gate = m.gate;
        for _ in 0..1200u64 {
            tick(&mut ctx);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            if m.gate != gate {
                gate = m.gate;
                ran.push(ctx.now_ms);
            }
        }
        assert!(ran.len() > 100, "the master tick never ran");
        let gaps: Vec<u64> = ran.windows(2).map(|w| w[1] - w[0]).collect();
        for g in &gaps {
            assert!((99..=117).contains(g), "master frame gap {g} ms is off the 6/7-tick grid");
        }
        let mean = gaps.iter().sum::<u64>() as f64 / gaps.len() as f64;
        assert!(
            (mean - 106.4).abs() < 1.5,
            "master frame averages {mean:.2} ms; mikes-long.mp4 measures 106.5"
        );
    }

    /// The `g0922` daydream must play its WHOLE 57-frame run, once, and then
    /// go away.
    ///
    /// The spec's art catalogue calls 2438-2494 a uniform "8x6, 57 frames"
    /// fly-buzz run. It is not a fly: the compound balloons from an 8x6 speck
    /// through 62x48 with a flying toaster inside it and drifts up-left on
    /// cumulative dx/dy to finish 77x60 at world (149,103). The previous pass
    /// pinned it to frame 2438 to stop a giant toaster cycling across the
    /// room ten times a second — but `mikes-long.mp4` 54.4-60.4 s shows the
    /// bubble for real, in the kitchen, toaster legible at 57.8 s, once in
    /// 180 s and for ~5.7 s (= 57 frames at one per master tick). So the
    /// toaster is not the bug; looping it was.
    #[test]
    fn the_daydream_bubble_is_a_one_shot_that_plays_its_whole_run() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        {
            let b = m.props.iter().find(|p| p.first == 2438).expect("daydream prop");
            assert!(b.oneshot, "the daydream must be a one-shot, not a loop");
            assert_eq!(b.last, 2494, "the whole 57-frame run must be playable");
        }
        let mut ctx = Ctx {
            rng: RandomLong::new(7),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut gate = m.gate;
        let mut plays = 0u32;
        let mut on = 0u32;
        let mut run_len = 0u32;
        let mut longest = 0u32;
        let mut reached = 2438u32;
        let mut frames = 0u32;
        // 10 minutes of master frames
        while frames < 6000 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            if m.gate == gate {
                continue;
            }
            gate = m.gate;
            frames += 1;
            let b = m.props.iter().find(|p| p.first == 2438).unwrap();
            if b.active {
                if run_len == 0 {
                    plays += 1;
                }
                run_len += 1;
                on += 1;
                reached = reached.max(b.cur);
                longest = longest.max(run_len);
            } else {
                run_len = 0;
            }
        }
        assert!(plays >= 2, "the daydream never replayed ({plays} plays in 600 s)");
        assert!(
            reached >= 2470,
            "the daydream stopped at compound {reached} — it must reach the toaster frames"
        );
        assert!(
            longest <= 60,
            "one play lasted {longest} master frames; the run is 57"
        );
        // one play per ~2-3 min: far from the every-frame loop it replaced
        assert!(
            on * 8 < frames,
            "the daydream was on screen for {on} of {frames} frames — that is a loop"
        );
    }

    /// The decorated Christmas tree (`c_581`, g092E's DoSetUp sequence) is
    /// never drawn. `mikes-long.mp4` covers all three rooms — 69 s of living
    /// room, 74 s of hallway, 34 s of kitchen — and the tree's world box
    /// (489,241)+46x100 is empty in every one of them. Drawing it put a 46 px
    /// Christmas tree over the living room's radiator, half off the canvas.
    #[test]
    fn the_christmas_tree_is_never_drawn() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        assert!(
            !m.props.iter().any(|p| p.first == 581),
            "the Christmas tree is in the draw list"
        );
        let mut ctx = Ctx {
            rng: RandomLong::new(3),
            rng15: Random15::new(99),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        for _ in 0..4000u32 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            let mut out = Vec::new();
            m.sprites(&mut out);
            for d in &out {
                for n in 581..=584u32 {
                    assert!(
                        !d.png.ends_with(&format!("c_{n}.png")),
                        "compound {n} (Christmas tree) was drawn"
                    );
                }
            }
        }
    }

    /// The scene canvas is re-placed at a fresh random spot on the 640x480
    /// screen every time Mike changes room — it is NOT the fixed screen
    /// offset 0x80 that §4 step 2 reads as the initial value.
    ///
    /// `mikes-long.mp4`, panel top-left measured in module pixels (crop the
    /// 56 px title bar, halve): living (194,134) at 3.23 s, hallway (15,69)
    /// at 10.37 s, kitchen (101,193) at 31.27 s, living (206,266) at 65.30 s,
    /// hallway (81,7) at 127.43 s. `mikes-life.mp4` adds (197,20) at 4.0 s
    /// and (73,41) at 30.60 s. Seven draws, seven different spots, both
    /// living-room visits in different places and both hallway visits in
    /// different places — so it is neither fixed nor per-room — and every one
    /// inside 0..=232 x 0..=281, which is exactly `screen - panel`
    /// (640-408, 480-199).
    #[test]
    fn the_scene_canvas_moves_to_a_new_random_spot_on_every_room_change() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        let mut ctx = Ctx {
            rng: RandomLong::new(0x5EED_CAFE),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut seen: Vec<(u32, (i32, i32))> = Vec::new();
        let mut gate = m.gate;
        let mut frames = 0u32;
        while frames < 6000 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            if m.gate == gate {
                continue;
            }
            gate = m.gate;
            frames += 1;
            if seen.last().map(|s| s.1) != Some(m.canvas) {
                seen.push((m.scene, m.canvas));
                // the scene compound must be blitted AT the canvas origin,
                // and the origin every other sprite is measured from is that
                // compound's own world bounds.
                let f = m.pack.frame(BASE, m.scene).expect("scene compound packed");
                let img = m.pack.image(&f.png);
                assert_eq!(
                    m.scene_origin,
                    (f.bx + f.dx, f.by + f.dy),
                    "world origin is not the current scene's own bounds"
                );
                let (w, h) = (img.w as i32, img.h as i32);
                assert!(
                    m.canvas.0 >= 0 && m.canvas.0 + w <= engine::SCREEN_W,
                    "canvas x {} puts a {w} px panel off a {} px screen",
                    m.canvas.0,
                    engine::SCREEN_W
                );
                assert!(
                    m.canvas.1 >= 0 && m.canvas.1 + h <= engine::SCREEN_H,
                    "canvas y {} puts a {h} px panel off a {} px screen",
                    m.canvas.1,
                    engine::SCREEN_H
                );
                let mut out = Vec::new();
                m.sprites(&mut out);
                let first = out.first().expect("scene drawn first");
                assert_eq!(
                    (first.x, first.y),
                    m.canvas,
                    "the scene compound is not drawn at the canvas origin"
                );
            }
        }
        assert!(
            seen.len() >= 4,
            "the canvas moved only {} times in 600 s",
            seen.len()
        );
        for w in seen.windows(2) {
            assert_ne!(w[0].1, w[1].1, "the canvas was re-placed at the same spot");
        }
        // not per-room either: the same room comes back somewhere else
        for room in [SCENE_LIVING, SCENE_HALL, SCENE_KITCHEN] {
            let spots: Vec<_> = seen.iter().filter(|s| s.0 == room).map(|s| s.1).collect();
            if spots.len() >= 2 {
                assert!(
                    spots.windows(2).any(|w| w[0] != w[1]),
                    "room {room} always redraws at the same spot"
                );
            }
        }
    }

    /// A room change is a CUT — the whole panel jumps to a new place on the
    /// screen — and the original makes very few of them.
    ///
    /// `mikes-long.mp4` changes room five times in 180 s: living 3.23-10.37,
    /// hallway 10.37-31.27, kitchen 31.27-65.30, living 65.30-127.43,
    /// hallway 127.43-180+. Dwells 7.1 / 20.9 / 34.0 / 62.1 / 52.6+ s, median
    /// 34 s, shortest 7.1 s. The port used to cut 16 times in the same 180 s
    /// with a median dwell of 5.0 s, because `present` handed every one of
    /// the 95 states a hard room and the planner's graph hops between groups
    /// several times a minute. Activities with no fixture of their own now
    /// inherit Mike's current room (see `present`).
    #[test]
    fn rooms_do_not_flip_faster_than_the_golden() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        // Pooled over seeds: which rooms one 600 s run happens to reach is
        // the RNG stream, not the behaviour (a three-seed "every seed sees
        // all three rooms" pin broke the moment the invented per-tick
        // Random15 draws came out, 2026-09-26).
        let seeds = 20u64;
        let mut all_three = 0;
        let mut dwells: Vec<u64> = Vec::new();
        for a in 1..=seeds {
            let mut m = build(pack.clone()).unwrap();
            let mut ctx = Ctx {
                rng: RandomLong::new(a),
                rng15: Random15::new(a as u32),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: 0,
                local_hms: (12, 0, 0),
                mouse: (320, 240),
                mouse_down: false,
            };
            let mut cuts = 0usize;
            let mut room = 0u32;
            let mut since = 0u64;
            let mut gate = m.gate;
            let mut rooms_seen = std::collections::HashSet::new();
            while ctx.now_ms < 600_000 {
                tick(&mut ctx);
                m.tick(&mut ctx);
                if m.gate == gate {
                    continue;
                }
                gate = m.gate;
                if m.scene != room {
                    if room != 0 {
                        dwells.push(ctx.now_ms - since);
                        cuts += 1;
                    }
                    room = m.scene;
                    since = ctx.now_ms;
                    rooms_seen.insert(room);
                }
            }
            all_three += usize::from(rooms_seen.len() == 3);
            assert!(
                cuts <= 40,
                "seed {a:#x}: {cuts} room cuts in 600 s — the golden makes 5 in 180 s"
            );
        }
        dwells.sort_unstable();
        let median = dwells[dwells.len() / 2];
        eprintln!(
            "{} room changes over {seeds} x 600 s, median dwell {median} ms, {all_three} seeds saw all three rooms",
            dwells.len()
        );
        assert!(
            median >= 9_000,
            "median room dwell {median} ms — the golden's shortest of five is 7100 ms and its median is 34000"
        );
        assert!(all_three * 5 >= seeds as usize * 4, "only {all_three}/{seeds} seeds reached all three rooms in 600 s");
    }

    /// `g0264` covers every state the chooser can hand out, and every run in
    /// it is packed. Five states' runs (28/29/34/77/78 → 87/46/732/179/190)
    /// are wallpaper/door patches without Mike in this pack — a pack-side
    /// gap, not a table error.
    #[test]
    fn every_state_has_a_packed_run() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        for state in 2..=95u16 {
            let (_, run) = present(state);
            assert!(
                STATE_RUN.iter().any(|(s, _)| *s == state),
                "state {state} is not in g0264"
            );
            assert!(pack.frame(BASE, run).is_some(), "state {state}: run {run} not packed");
        }
        // the room-changing exits, per fn02's callers in fn72
        assert_eq!(room_after_exit(0x13), Some(SCENE_HALL));
        assert_eq!(room_after_exit(0x19), Some(SCENE_KITCHEN));
        assert_eq!(room_after_exit(0x21), Some(SCENE_LIVING));
        assert_eq!(room_after_exit(80), None);
    }

    /// Mike must not teleport.
    ///
    /// The planner picks an activity every time a run ends, `present` maps it
    /// to a single run, and that run starts wherever its art was drawn — so
    /// before `travel_chain`, committing a kitchen state put him 192 px across
    /// the room between two consecutive 100 ms ticks (microwave `seq 2751` to
    /// fridge `seq 2308`), and 71 such cuts landed in a ten-minute run. That
    /// is the "teleports around the kitchen" report. Nothing in the golden
    /// does this: every move Mike makes in `emu/captures/mikes-life.mp4` is a
    /// walk run stepping 5-7 px per tick.
    ///
    /// Measured at the bounds CENTRE, not `bx`: the bounds box is tight to the
    /// art, so a standing pose (21-30 px wide) and a mid-stride one (46-68)
    /// differ by up to 25 px of left edge for a man who has not moved.
    /// Crossing a room is a cut; changing pose is not.
    #[test]
    fn mike_walks_between_activities_instead_of_teleporting() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        for (a, b) in [(1u64, 1u32), (0x5EED_CAFE, 0xC0FFEE)] {
            let mut m = build(pack.clone()).unwrap();
            let mut ctx = Ctx {
                rng: RandomLong::new(a),
                rng15: Random15::new(b),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: 0,
                local_hms: (12, 0, 0),
                mouse: (320, 240),
                mouse_down: false,
            };
            let mut widths: std::collections::HashMap<u32, i32> = Default::default();
            let mut prev: Option<(u32, i32)> = None;
            let mut worst = (0i32, 0u64, 0u16);
            for i in 0..6000u64 {
                tick(&mut ctx);
                m.tick(&mut ctx);
                let f = m.pack.frame(BASE, m.cur).expect("current frame is packed");
                let w = *widths
                    .entry(m.cur)
                    .or_insert_with(|| m.pack.image(&f.png).w as i32);
                let at = f.bx + f.dx + w / 2;
                if let Some((scene, was)) = prev {
                    // a room change moves the whole scene canvas; only motion
                    // INSIDE one room has to be continuous
                    if scene == m.scene && (at - was).abs() > worst.0 {
                        worst = ((at - was).abs(), i, m.state);
                    }
                }
                prev = Some((m.scene, at));
            }
            assert!(
                worst.0 <= 80,
                "seed {a:#x}/{b:#x}: Mike jumped {} px in one tick at tick {}                  (state {}) — that is a teleport, not a stride",
                worst.0,
                worst.1,
                worst.2
            );
        }
    }

    /// The telephone must not ring continuously — and it rings only when
    /// Mike's ENTER hooks say so. The old self-scheduled ring (every 20–50 s,
    /// with a 30 s answer window) is gone (2026-09-26): the C rings from
    /// ENTER 0x1B (channel 14 at frames 0/45/90, g0936 queued at the same
    /// delays, dial tone at 128), 0x1A, 0x34 and 0x38 only.
    #[test]
    fn the_telephone_does_not_ring_every_tick() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            eprintln!("pack ../assets/mikes-so-called-life missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
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
        let mut rings = 0usize;
        while ctx.now_ms < 300_000 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            rings += ctx.sounds.iter().filter(|&&s| s == 1016).count();
            ctx.sounds.clear();
        }
        // ENTER 0x1B, driven directly: three rings 45 frames apart, the
        // ringing sprite (run 0x7C7) replacing the static phone each time
        let mut m = build(m.pack.clone()).unwrap();
        m.transition(&mut ctx, 0x1B);
        let (mut at, mut shown) = (Vec::new(), 0);
        for f in 0..140 {
            m.phone_tick();
            m.pump_sounds(&mut ctx);
            if ctx.sounds.drain(..).any(|s| s == 1016) {
                at.push(f);
            }
            shown += usize::from(m.ring_shown);
        }
        assert_eq!(at, vec![0, 45, 90], "ENTER 0x1B ring frames");
        assert!(shown >= 3 * 11, "ringing sprite on screen {shown} frames");
        assert!(!m.phone_hidden, "static phone never came back");
        assert!(
            rings <= 40,
            "snd 1016 played {rings} times in 300 s — the ring deadline is not being advanced"
        );
    }
    #[test]
    #[ignore]
    fn trace_mike() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life"))
        else {
            return;
        };
        let mut m = build(pack).unwrap();
        let mut ctx = Ctx {
            rng: RandomLong::new(0x5EED_CAFE),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        // per-tick dump: t, state, run first/last, cur, drawn centre,
        // canvas, cues fired this tick
        for i in 0..8000u64 {
            tick(&mut ctx);
            m.tick(&mut ctx);
            let fno = m.cur;
            let f = m.pack.frame(BASE, fno).unwrap();
            let img = m.pack.image(&f.png);
            let (x, y) = m.world_xy(f);
            let (cx, cy) = (x + img.w as i32 / 2, y + img.h as i32 / 2);
            eprintln!(
                "i{i} t{} st{} run{}..{} cur{} c({cx},{cy}) canvas{:?} scene{} snd{:?}",
                ctx.now_ms, m.state, m.first, m.last, fno, m.canvas, m.scene, ctx.sounds
            );
            ctx.sounds.clear();
        }
    }

}


/// Ratchets for the 2026-09-26 fix lane (Jason: "blinked to the back standing
/// up, then blinked into the chair … weird sound").
#[cfg(test)]
mod ratchet_2026_09_26 {
    use super::*;
    use engine::{Random15, RandomLong};

    fn ctx(seed: u32) -> Ctx {
        Ctx {
            rng: RandomLong::new(seed as u64),
            rng15: Random15::new(seed),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    /// Drive `secs` of sim time on the Mac tick grid; call `f` once per
    /// MASTER frame with the sounds that frame emitted.
    fn run(m: &mut MikesSoCalledLife, c: &mut Ctx, secs: u64, mut f: impl FnMut(&MikesSoCalledLife, &Ctx, &[u32])) {
        let mut t = 0u64;
        let mut gate = m.gate;
        while c.now_ms < secs * 1000 {
            t += 1;
            c.now_ms = TickClock::MacTick.now_ms(t);
            m.tick(c);
            if m.gate != gate {
                gate = m.gate;
                let s = std::mem::take(&mut c.sounds);
                f(m, c, &s);
            }
        }
    }

    /// Mike's drawn centre, relative to the scene canvas.
    fn mike_at(m: &MikesSoCalledLife) -> (i32, i32) {
        let f = m.pack.frame(BASE, m.cur).expect("Mike's frame is packed");
        let img = m.pack.image(&f.png);
        let (x, y) = m.world_xy(f);
        (x + img.w as i32 / 2 - m.canvas.0, y + img.h as i32 / 2 - m.canvas.1)
    }

    /// No Mike-only jump inside a room. The 456 "surprise pose" (really
    /// g094E's fn62 roll, not Mike's) drew state 93's run at its own world
    /// spot for ~1 s: 99–102 px out and back, 13× in 120 s on the frame
    /// seed. With it gone, the worst in-room step over 20 seeds × 600 s is
    /// an AUTHORED one (fn76 re-places each run at its own world position).
    #[test]
    fn mike_does_not_teleport_inside_a_room() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life")) else {
            eprintln!("pack missing — skipping");
            return;
        };
        let mut worst = (0f64, 0u32, 0u64, 0u32, 0u32);
        let mut on_456 = 0u64;
        for seed in 1..=20u32 {
            let mut m = build(pack.clone()).unwrap();
            let mut c = ctx(seed);
            let mut prev: Option<(u32, (i32, i32), u32)> = None;
            run(&mut m, &mut c, 600, |m, c, _| {
                if m.cur == 456 && m.state != 93 {
                    on_456 += 1;
                }
                let at = mike_at(m);
                if let Some((scene, p, cur)) = prev {
                    if scene == m.scene {
                        let d = (((at.0 - p.0).pow(2) + (at.1 - p.1).pow(2)) as f64).sqrt();
                        if d > worst.0 {
                            worst = (d, seed, c.now_ms, cur, m.cur);
                        }
                    }
                }
                prev = Some((m.scene, at, m.cur));
            });
        }
        eprintln!("worst in-room step {:.1} px (seed {}, t {} ms, {} -> {})", worst.0, worst.1, worst.2, worst.3, worst.4);
        assert_eq!(on_456, 0, "art 456 drawn outside state 93");
        assert!(
            worst.0 <= 60.0,
            "Mike jumped {:.1} px inside one room (seed {}, t {} ms, frame {} -> {})",
            worst.0, worst.1, worst.2, worst.3, worst.4
        );
    }

    /// Cue spam: snd 1009/1012 fired ~100× each in 53 s at a 416 ms gap —
    /// the invented entry cue re-queued on every dwell re-pick of state 22
    /// (0x16 has NO enter hook in fn72) through a table read two rows late.
    /// Golden `mikes-so-called-life.wav` (104 s, matched filter): 1009 × 5
    /// (one 0x28 burst, 2 frames apart), 1012 × 0.
    #[test]
    fn scratching_and_filter_sweep_do_not_spam() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life")) else {
            eprintln!("pack missing — skipping");
            return;
        };
        let (mut n1009, mut n1012, mut all) = (0u32, 0u32, 0u32);
        let seeds = 20u32;
        for seed in 1..=seeds {
            let mut m = build(pack.clone()).unwrap();
            let mut c = ctx(seed);
            run(&mut m, &mut c, 600, |_, _, s| {
                n1009 += s.iter().filter(|&&x| x == 1009).count() as u32;
                n1012 += s.iter().filter(|&&x| x == 1012).count() as u32;
                all += s.len() as u32;
            });
        }
        let per120 = |n: u32| n as f64 / seeds as f64 / 5.0;
        eprintln!(
            "per 120 s: 1009 {:.2}, 1012 {:.2}, all cues {:.1}",
            per120(n1009), per120(n1012), per120(all)
        );
        assert!(per120(n1009) <= 12.0, "1009 {:.1}/120 s (golden 5, spam was 103)", per120(n1009));
        assert!(per120(n1012) <= 3.0, "1012 {:.1}/120 s (golden 0, spam was 100)", per120(n1012));
        assert!(per120(all) <= 60.0, "{:.1} cues/120 s", per120(all));
    }

    /// Sleeping (0x16) re-enters itself every 4-frame run while the 4 s dwell
    /// runs, and in the C that re-enter is silent.
    #[test]
    fn sleeping_is_silent() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/mikes-so-called-life")) else {
            eprintln!("pack missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        let mut c = ctx(1);
        c.now_ms = 1;
        m.tick(&mut c); // start the machine
        m.cues.clear();
        for _ in 0..5 {
            m.state = 22;
            m.transition(&mut c, 22);
            assert!(m.cues.is_empty(), "ENTER 0x16 queued a cue");
        }
        // and the ones that DO have hooks queue exactly the listing's list
        m.transition(&mut c, 0x28);
        let q: Vec<(u8, i16)> = m.cues.iter().map(|s| (s.ch, s.delay)).collect();
        assert_eq!(q, vec![(7, 8), (7, 10), (7, 12), (7, 14), (7, 16)]);
        // pump: channel 7 is 1009, first one on the 9th pump
        let mut fired = Vec::new();
        for f in 0..20 {
            m.pump_sounds(&mut c);
            if c.sounds.drain(..).any(|x| x == 1009) {
                fired.push(f);
            }
        }
        assert_eq!(fired, vec![8, 10, 12, 14, 16]);
    }
}
