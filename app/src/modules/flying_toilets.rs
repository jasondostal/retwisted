//! Flying Toilets — full-fidelity transcription of the RE spec
//!
//! ## Port-from-decompile check, 2026-09-15 (Fable) — M129_M129.c, private
//! Read against the Ghidra C (totally-twisted docs/decompiled/flying-toilets):
//! `fn01` @0612 ctor (Crowd/20 → 3/5/8/10/12 entities, tagline
//! `128 + rand()%13`, six sound channels, `+0x40 = now`, `+0x44 = now+10000`,
//! `+0x4c = 1`), `fn04` @0F10 DoDrawFrame (controls re-read, entity Runs,
//! composite, then the scheduler: flap bed `rand()%3` on `+0x40 < now`, the
//! plunger pop three frames after `g025E` arms, the modifier-key flush and
//! `g0260` counter, the gag on `+0x44 < now`), `fn24` @147C the entity
//! handler (0 = `fn25` spawn anywhere, 1 = `fn26` re-identify + edge spawn,
//! 2 = `fn27` the 40 ms flight step with the link advance, fast/overlap
//! fallbacks (x−2,y+3 then x−2,y+1), the `rand()%100` plunge roll and the
//! `fn30` off-screen test that uses the WIDTH for the bottom edge), `fn28`
//! @1B24 identity (`rand()%100`: <45 occupant toilet, <90 TP roll, else the
//! plunger; Aaron is run 0x1e until `g0260 >= 6`, then 0x41; box = union
//! rect + 6; `fast = rand()%2 == 0`; a further `rand()%2` into the unused
//! `+0xd4`), `fn29` @1DFC the box overlap. The machine below already matched
//! the C function for function; what changed in this pass:
//! - every one of the 16 random sites is ANSI `rand()` (`ctx.rng15.below`),
//!   not the library's RandomBelow — the plan's "16 rand sites";
//! - the `+0xd4` roll is taken so the stream stays in step with the C.
//! - Draw/box model: the C advances `pos` by the frame link (`+0x78`) and
//!   draws the frame centred on it; per cycle the links sum to zero, so the
//!   port's "anchor + (bx − union centre)" placement is the same picture.
//! (totally-twisted docs/behavior/flying-toilets.md; every constant is
//! disasm-cited there). THE EXEMPLAR MODULE: match this structure and this
//! level of fidelity, including deliberate reproduction of original bugs.
//!
//! Faithful behavior implemented here:
//! - Crowd slider banding {3,5,8,10,12} (band = raw/20)
//! - identity rolls: 45% winged toilet (occupant sub-table), 45% TP roll
//!   (paper sub-table), 10% plunger; re-rolled at every edge respawn
//! - flight: -2/+1 per 40 ms tick, x3 fast; 50% fast at spawn; 1% per-step
//!   speed re-roll; 1-in-200 plunger plunge (seq 55 once -> idle 60)
//! - fast-mode in-flight collision fallback steps (-2,+3) then (-2,+1)
//! - ORIGINAL BUG kept: bottom despawn tests the WIDTH, not height
//! - sound: continuous random flap bed (Mix1/Flap1/Flap2), plunger Pop with
//!   the 3-frame delay + single-global collapse, periodic gag (Flush or
//!   Gastro when Rude Sounds), first at +10 s then every 30-119 s
//! - Caps Lock: immediate Flush ALL; MG_7D0 counter arms the Aaron+PIG
//!   variant (seq 65) at >= 6, with the freeze quirk (counter only moves
//!   while Caps Lock is down, so a tapped-in pig stays until a long hold
//!   outlives the 5 s window)
//!
//! ## ERRATA — golden capture `emu/captures/toilets.mp4` (2026-09-12,
//! ## tick-quantization sweep)
//!
//! - **§8 "40 ms" is the delay, not the frame period.** §6.3 gates the
//!   flight step on the entity's `+0xD0` deadline, re-armed to `now + 40`
//!   (@0x181E/@0x1832), and `now` is the After Dark ms clock
//!   `Resource.f4724() = TickCount() * 16.625`, so it only ever takes the
//!   values `16.625 k`. A delay of 40 therefore fires on the **3rd Mac
//!   tick = 49.875 ms**, not on the next 40 ms boundary — the same finding
//!   the bungee lane made (`bungee_roulette.rs` ERRATA §2.3).
//!   Measured on `toilets.mp4` (1280x1016 @ 30 fps, cropped 56 px and
//!   halved to module px): the isolated toilet-paper roll that enters at
//!   video frame 600 drops out of fast mode at f639 and then runs
//!   **f640 → f838: x 284.86 → 28.92 (-255.94 px) and y 231.58 → 359.71
//!   (+128.13 px)**. Normal flight is exactly (-2, +1) px per step, so that
//!   is 127.97 / 128.13 steps in 198 video frames = **1.545 video frames =
//!   51.5 ms per step**. The port used to step on its own 40 ms shell tick
//!   — 28 % too fast. The module now runs on `TickClock::MacTick` (paced at
//!   16.625 ms, `now_ms` truncated exactly as `Resource.f4724()` does) with
//!   the disasm's 40 left alone, so the step is 3 ticks = **49.875 ms**.
//!   The first pass approximated the grid with `tick_ms = 17` (51 ms).
//! - The entity deadline itself was missing: the port stepped every shell
//!   tick and leaned on `tick_ms` to be the period. `Entity::next_due` is
//!   now the real `entity[0xD0]`, so the 40 quantizes on its own and the
//!   per-draw-frame work (§7's sound scheduler, the `MG_7CF` 3-*draw*-frame
//!   plunger delay, the Caps-Lock counter) runs at the draw rate the spec
//!   puts it at instead of at the flight rate.
//!
//! ## ERRATA — golden capture `emu/captures/toilets.mp4` (2026-09-13,
//! ## first full fidelity lane for this module)
//!
//! `toilets.mp4` is 1800 frames, 1280x1016 @ 30 fps, DEPTH=32. Crop
//! `1276:958:2:56` and decimate 2:1 in numpy for true module pixels. The
//! After Dark panel is legible at **f50–f105** and the module runs
//! **f106 (blank) / f108 (first sprites) → f1570**; after f1570 the
//! recording caught the host desktop, not the guest. Panel settings for
//! this run, read off f60: **Crowd = "Not Pretty", Paper = Random,
//! Occupant = Random, Rude Sounds on.**
//!
//! 1. **The field is BLACK.** `docs/emulator/setup.md`'s 2026-08-29 note
//!    ("Flying Toilets draws on a WHITE field") is FALSIFIED. Every one of
//!    the 1462 module frames has an exactly-black background at all four
//!    corners and in every gap between sprites; mean frame luminance is
//!    3–13/255 and never rises. The white reading came from the 8-bit rig
//!    and does not survive DEPTH=32. `meta.json`'s `field: [0,0,0]` is
//!    right; nothing in the port needed changing, but the "one verified
//!    non-black field" claim must not be carried forward.
//! 2. **Palette: no CLUT remap at 32-bit.** §5's palette is pushed to the
//!    canvas only at depth 4 or 8 (@0x08D2), so at DEPTH=32 the art's own
//!    colours are used verbatim. Sampling the idle plunger (`c_060`)
//!    against f500 gives (221,0,0)→(194,0,0) and (204,153,102)→(189,138,93)
//!    — a flat ~0.88x encoder darkening with no hue rotation. The port's
//!    direct-colour `pal: 0` path is correct.
//! 3. **The flight step is 50.0–50.65 ms, not 51.5.** Two independent
//!    measurements, both tighter than the first sweep's:
//!    * *Wing-flap recurrence.* A 6-frame run advances one compound frame
//!      per flight step, and frame 4 of every run is 7–8 px taller than the
//!      other five (`c_004` is `by 27 h 102` vs `by 34 h 95`). That tall
//!      frame recurs every **9.107 / 9.136 / 9.107 video frames** on three
//!      isolated winged toilets (tracks f114–378, f1060–1191, f1130–1394;
//!      29+12+29 peaks) = **303.6 ms per 6-step cycle = 50.6 ms/step**.
//!    * *Position.* Two clean, fully-on-screen TP-roll traverses
//!      (f1216–1569, f1358–1569) give 50.72 and 50.60 ms/step from x and
//!      50.97/51.16 from y; the plunger speed-change tracks below give
//!      -1.33 px/video-frame normal = **50.1 ms/step**.
//!    `TickClock::MacTick` + a 40 ms delay = 3 ticks = **49.875 ms**, i.e.
//!    1.5 % fast against the emulator — well inside Basilisk's timing slop
//!    and vastly better than the old 40 ms shell tick. CONFIRMED; the
//!    earlier "51.5 ms" figure was inflated by centroid drift and by frames
//!    where the sprite was clipped at an edge.
//! 4. **`DoDrawFrame` runs the sound scheduler LAST** (§8: entities @0x0FAC,
//!    composite @0x0FF2, sounds @0x1006). The port called `sound_tick`
//!    first, so a plunge armed by the entity loop was not seen by `MG_7CF`
//!    until the next draw frame and the Pop landed 3 draw frames later
//!    instead of 2. Fixed; `plunger_pop_lands_two_draw_frames_after_the_plunge`
//!    ratchets it. (Sound could not be verified from *this* capture —
//!    Basilisk's audio is garbage and `toilets.mp4` has no audio track at
//!    all. **Superseded 2026-09-13** by the QEMU golden audio below, which
//!    verifies every §7 cue except the Caps-Lock branch; the Pop's
//!    draw-frame count is the one claim that capture still cannot settle.)
//! 5. **Entry side is re-rolled per placement attempt.** @0x168E is the
//!    50-attempt counter and @0x1696 the side coin flip, so the flip is
//!    inside the loop (the same shape as state 0's @0x1552/@0x155A), and
//!    §9's pseudocode agrees. The port rolled the side once outside the
//!    loop. Fixed.
//! 6. **Population, entry edges, size classes — all CONFIRMED.** Over
//!    f108–f1570 the blob count per frame peaks at exactly **10** and never
//!    11 (mean 8.94, the rest partly off-screen), matching Crowd
//!    "Not Pretty" → band 3 → 10. Of 49 tracked edge entries, **26 came in
//!    at the right edge and 23 at the top** (50/50 per §6.2); **zero**
//!    entered from the left or bottom, confirming `MG_7D3 == 0` and the
//!    dead mirror branches. Three size classes and nothing else: winged
//!    toilet 105–126 x 94–107, TP roll 41 x 17, plunger 41 x 26 — read as
//!    the packed unions (107x95…126x107, 43x18, 42x27) minus 1–2 px of
//!    threshold shrink. **CORRECTED 2026-09-13: there is no threshold
//!    shrink.** Re-measured on the decimated frame (crop `1276:958:2:56`,
//!    `a[::2,::2]`, no `flags=neighbor`) the on-screen blobs are the packed
//!    boxes to the pixel — see item 9 and the CONTRADICTIONS note. The
//!    size-class *conclusion* stands; the 1–2 px was the old neighbour-scale
//!    pipeline. There is no depth layering: sprites are drawn in a flat
//!    entity order with no scaling.
//! 7. **Flight geometry: -2/+1 normal, -6/+3 fast, exact 2:1 slope.**
//!    Fully-on-screen traverses give slope 2.000 / 2.014 / 2.017. Normal
//!    entities run **-1.33 px/video-frame**, fast ones **-4.0** — a clean
//!    3.0x. Fast vertical growth is visible on top entries: a fast plunger
//!    (f1256+) emerges 3 px per step (6,6,9,12,12,15,18,…) against a normal
//!    one's 1 px (f608+: 6,6,7,8,8,9,10,…).
//! 8. **The 1 %-per-step speed re-roll is real and was caught twice.**
//!    Plunger track f233–521 runs at -4.0 px/vf until ~f309 and then drops
//!    to -1.33 for the rest of its life; plunger track f608–916 runs at
//!    -1.33 until ~f863 then jumps to -4.0. Roughly a quarter of tracked
//!    entities are fast at any instant, which is what 50 % fast at spawn
//!    plus a 3x traverse speed predicts.
//! 9. **The plunger plunge (55→56→57→60) was filmed.** `c_057` is 29 px
//!    tall against `c_055`/`c_060`'s 27 and `c_056`'s 26, and two plunger
//!    tracks show exactly one 29-px frame: **f241** (track f233–521) and
//!    **f1343** (track f1306–1395). **CORRECTED 2026-09-13** — the series
//!    those sit in is flat **27**, not "an otherwise flat 26-px series";
//!    26 belongs to exactly one frame, `c_056`, the squash. See the
//!    CONTRADICTIONS note.
//!    Both plungers were already fast when it fired, consistent with
//!    @0x1A80 re-rolling `entity[0xD8]` unconditionally. The art is a
//!    squash (56) then a splat-lines pose (57), not a tumble.
//! ## CONTRADICTIONS settled 2026-09-13 — plunger frame heights
//!
//! Item 9 gave three per-frame heights *and* called the series "otherwise
//! flat 26 px", which cannot both be true. The pack settles it and the
//! capture confirms the pack to the pixel.
//!
//! **Packed** (`assets/flying-toilets/meta.json`, series 9000 — these are the
//! authoritative numbers):
//!
//! | run | frames | packed w×h |
//! |---|---|---|
//! | 40 / 45 / 50 (TP rolls) | c_0x0, c_0x1, c_0x2 | 43×18, 43×18, 42×19 |
//! | **55 (the plunge)** | c_055, c_056, c_057 | **42×27, 43×26, 43×29** |
//! | **60 (idle plunger)** | c_060, c_061, c_062 | **42×27, 42×27, 42×27** |
//!
//! So the plunger baseline is **27**, packed, in both runs. 26 is one frame —
//! `c_056`, the squash — and 29 is one frame, `c_057`, the splat pose. The
//! "otherwise flat 26 px" reading is FALSIFIED.
//!
//! **On screen**: identical, with no shrink. Re-measuring `toilets.mp4` on
//! the decimated frame around the f241 plunge, connected components of the
//! descending plunger read
//!
//! ```text
//! f236-f238  42x27  288 px    c_060/061/062 (idle) or c_055
//! f239-f240  43x26  287 px    c_056
//! f241       43x29  302 px    c_057
//! f242+      42x27  288 px    back to run 60
//! ```
//!
//! and the packed PNGs carry **288 / 287 / 302** non-background pixels in
//! those same boxes — a frame-exact identification, not a size match. The TP
//! rolls come out the same way (43×18 at 410–414 px, 42×19 at 418 px against
//! `c_050`/`c_051`/`c_052`'s 411/413/418). Item 6's "41 x 17 / 41 x 26 minus
//! 1–2 px of threshold shrink" is therefore an artefact of the deprecated
//! `scale=…:flags=neighbor` pipeline, the same one that inflated boris'
//! blades and chameleon's puddle; on the decimated frame there is nothing to
//! subtract.
//!
//! No behaviour change — §3's `BOX` table already derives from the packed
//! 42×27 union (`+6` ⇒ the 48×33 the suite asserts). Pinned by
//! `the_plunge_frames_are_27_26_29_and_the_idle_run_is_flat_27`.
//!
//! 10. **TP rolls do not unroll.** Frames 50/51/52 (and 40–42, 45–47) are a
//!    3-pose streamer flutter over a fixed-size roll — the roll never grows
//!    a longer tail. The sweep's "TP roll y +128 px" is just 128 normal
//!    steps of the (-2,+1) drift, reproduced here as f640→f838.
//! 11. **"Dumpin' Dan" is not an event.** The golden-gap item filed as
//!    "toilet newspaper readers" has no timing to it: Dan is Occupant
//!    variant 1 (compound run 20–25, union 122x103, `c_023` 124x106), rolled
//!    per entity inside `MF_7CA` at every edge respawn, and with
//!    Occupant = Random he shows up in about a quarter of winged toilets
//!    with no entry cue, no dwell and no exit of his own. He rides, flaps
//!    and drifts exactly like Nobody/Bessie/Aaron. Confirmed against f130
//!    (bald head, blue shorts, "BUNGEE COWS! MORE!" paper — the same paper
//!    art Bessie reads; only Aaron's says "I SAW ELVIS!"), byte-for-byte the
//!    port's `c_020` composition.
//!
//! ### Still open after this pass
//! - **Identity mix.** The capture's on-screen composition leans lighter
//!   than 45/45/10: total lit ink 17.6 k px/frame against the port's 24.6 k,
//!   and an erosion-core toilet count of 4.04/frame against the port's 5.97,
//!   both of which back-solve to a winged-toilet share near 30 % rather than
//!   45 %. It is NOT enough to move a disasm-cited constant: 48 s of one
//!   Demo run is only ~45 independent identity rolls, so 14 toilets where 20
//!   are expected is 1.8 sigma, and the port sample is one seed whose opening
//!   cohort happened to be toilet-heavy. `r < 45` / `r < 90` (@0x1B38,
//!   @0x1C5C) stays. A 5-minute capture, or three Demo runs, would settle it.
//! - **The pig (run 65–70) and every Caps-Lock path** are untouched by this
//!   capture — nobody held Caps Lock. Unverifiable without a keyed capture.
//! - ~~**All sound.**~~ SETTLED 2026-09-13 — see the next block.
//!
//! ## ERRATA — QEMU golden AUDIO, 2026-09-13
//! ## (`emu/captures/qemu/flying-toilets.wav`, 60 s, and
//! ##  `emu/captures/qemu/flying-toilets-long.wav`, 300 s, 3000 frames at a
//! ##  measured 10.0 fps, wav = wall to 1.8 ms so `frame = round(t x 10)`;
//! ##  panel exactly as the 60 s run: Crowd "Not Pretty", Paper Random,
//! ##  Occupant Random, **Rude Sounds ON**)
//!
//! §7 is no longer disasm-only. Correlation is `snd-correlate.py` against
//! `ripped/flying-toilets/expanded/` + `ripped/shared-twisted-sound/`.
//!
//! 1. **The flap bed is `now + Duration`, and `Duration` is the EXPANDED
//!    sample — the port's clip lengths were 2x too short.** Over the 300 s
//!    run the bed fires 1281 times (Flap 1 x442, Mix 1 x421, Flap 2 x418 —
//!    `RandomLong() % 3`, 0.5 sigma off a third each) with r = 0.98-0.999, and
//!    the interval from one flap onset to the next has its mode *exactly* at
//!    the length of the clip that just played: **210.3 / 175.8 / 253.4 ms**
//!    (medians, n = 441 / 418 / 421 — the packed WAV lengths to 0.1 ms).
//!    §7's table quotes 0.124 / 0.097 / 0.082 s, which are the **raw** rips
//!    of the sndS-compressed resources — half-length MACE payloads that
//!    `pack_assets.py` does not even ship. Constants corrected; the module
//!    used to flap at ~2x the original rate.
//! 2. **`snd 30008` Pop: 24 firings in 300 s, `MS_POP` = 250 ms exact.**
//!    The Pop starts on the *sample boundary* of the flap it follows
//!    (t = 103.762 = the Flap 2 at 103.586 + 175.8 ms; likewise 286.237 and
//!    293.678) and the bed resumes exactly 249-250 ms later — i.e. the 1995
//!    channel *queues*: a command issued mid-clip starts when that clip ends.
//! 3. **Gastro 1007 and Flush 30000 DO fire — §7.4's schedule is CONFIRMED.**
//!    audio-captures.md's "never fired … a genuine miss" was a 60 s sampling
//!    artefact. In the 300 s run the periodic gag fires **4 times**:
//!
//!    | wav t | frame | snd | r | flap bed suppressed |
//!    |---|---|---|---|---|
//!    | 8.94 | f89 | 30000 Flush ALL | 0.846 | 8.82 → 11.74 (2.92 s) |
//!    | 116.15 | f1161 | 30000 Flush ALL | 0.840 | 115.90 → 118.99 (3.09 s) |
//!    | 195.17 | f1952 | 1007 Gastro 11k | 0.996 over its first 184 ms | 194.82 → 195.35 |
//!    | 291.36 | f2914 | 1007 Gastro 11k | 0.997 over its first 184 ms | 290.99 → 291.54 |
//!
//!    Intervals 107.2 / 79.0 / 96.2 s — all inside §7.4's `30 000 +
//!    (rand % 90) * 1000`; 2 Flush + 2 Gastro is the Rude-Sounds coin flip;
//!    and the first one lands at capture t = 8.94 with the Demo press ~1 s
//!    before frame 0, i.e. at **init + 10 s** (@0x0B00's `0x2710`). Nothing
//!    in §7.4 is occupant-, Caps-Lock- or roll-gated: it is just slow.
//! 4. **`snd 1007`'s rip is 1.486 s but only its first 184 ms is the
//!    resource.** Correlated against truncations of the rip, both Gastro
//!    firings hold r = 0.996 / 0.997 flat out to 184 ms and break at 190
//!    (0.986), 200 (0.966), 300 (0.834), 1486 ms (0.573); the rip's own level
//!    steps down 6 dB at ~0.29 s. The flap bed comes back **0.184 s** after
//!    the Gastro onset — the same number twice, to the millisecond — which is
//!    item 2's queueing rule with `Duration(1007) = 184`. `MS_GASTRO` is now
//!    184, not 1486. (Flush behaves the same way: audible 2.800 / 2.844 s,
//!    the sample's own length, against the disasm's hard-coded 2750 — the
//!    channel finishes the buffer, so 2750 stays.)
//! 5. **The Pop's *draw-frame* count is NOT settled by this rig.** All 5 pops
//!    in the 60 s capture sit a constant 8.5-10.0 video frames after the
//!    plunging plunger's `c_057` splat frame (e.g. pop at wav 45.396 = f908,
//!    `43x29` blob at f900; 46.875 = f938 against f929), and a draw frame is
//!    ~50 ms = 1 video frame, so 2 vs 3 draw frames is 50 ms inside a
//!    constant ~0.4 s audio-vs-video offset this capture cannot calibrate.
//!    Item 4 of the 2026-09-13 errata (2 draw frames, from the @0x0FAC /
//!    @0x1006 order) stands on the disasm; the audio neither confirms nor
//!    contradicts it. What the audio *does* settle: the Pop is one-per-plunge
//!    and `MG_7CF` really is a single global (t = 20.03/20.33 and
//!    227.21/227.52 are back-to-back pops 0.30 s apart — a second plunge
//!    re-arming after the first cleared it).
//! 6. **Still unverified by audio:** §7.3's Caps-Lock flush and the `MG_7D0`
//!    pig counter. `capture` may not touch input (After Dark dismisses on any
//!    key event), so a Caps-Lock run needs the key held *before* Demo.

use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, TickClock, SpriteDraw, SCREEN_H, SCREEN_W,
};

const BASE: u32 = 9000;

// snd resource ids (packed as sounds/<id>.wav)
const SND_MIX1: u32 = 1003;
const SND_FLAP1: u32 = 1001;
const SND_FLAP2: u32 = 1002;
const SND_GASTRO: u32 = 1007;
const SND_POP: u32 = 30008; // shared
const SND_FLUSH: u32 = 30000; // shared
// §7's `module[0x40] = now + Duration(snd)` — these are `Duration`, and the
// QEMU golden audio says `Duration` is the length of the **sndS-expanded**
// sample, i.e. the byte-identical WAV `pack_assets.py` packs into
// `assets/flying-toilets/sounds/<id>.wav`. The old values were the *raw*
// (still-MACE-compressed) rips, which are half as long, so the port ran its
// flap bed at ~2x the original's rate. See ERRATA 2026-09-13 (QEMU audio) §1.
// Pinned by `the_flap_bed_periods_are_the_packed_sample_lengths`.
const MS_MIX1: u64 = 253; // snd 1003, 2820 frames @ 11127 Hz = 253.438 ms
const MS_FLAP1: u64 = 210; // snd 1001, 2340 frames = 210.299 ms
const MS_FLAP2: u64 = 176; // snd 1002, 1956 frames = 175.789 ms
const MS_POP: u64 = 250; // snd 30008, 2777 frames = 249.573 ms
/// snd 1007. **NOT the 1.486 s the rip is** — see ERRATA 2026-09-13 (QEMU
/// audio) §3: the capture plays exactly the rip's first 184 ms and then the
/// flap bed returns, twice, to the millisecond.
const MS_GASTRO: u64 = 184;
const MS_FLUSH: u64 = 2750; // hard-coded in the original @0x112C


/// §6.3 @0x1832: `entity[0xD0] = now + 40` — the flight-step delay, left
/// exactly as the disasm has it so the Mac-tick grid quantizes it.
const FLIGHT_DELAY_MS: u64 = 40;

#[derive(Clone, Copy, PartialEq)]
enum State {
    SpawnAnywhere,
    SpawnEdge,
    Fly,
}

struct Entity {
    state: State,
    first: u32,
    last: u32,
    cur: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fast: bool,
    ax: i32,
    ay: i32,
    /// `entity[0xD0]` — the §6.3 next-flight-step deadline (@0x181E gate,
    /// @0x1832 re-arm). Also armed at park time (@0x1DDE).
    next_due: u64,
}

pub struct FlyingToilets {
    pack: Pack,
    entities: Vec<Entity>,
    // controls (raw values, exactly as GetControlValue returns them)
    crowd: i32,    // 0..100 slider
    paper: i32,    // 0-based: 0 White 1 Pastel 2 CowPrint 3 Random (MG_7D2)
    occupant: i32, // 0-based: 0 Nobody 1 Dan 2 Bessie 3 Aaron 4 Random (MG_7D1)
    rude: bool,
    // sound scheduler (module globals in the original)
    flap_deadline: u64, // module+0x40
    gag_deadline: u64,  // module+0x44
    chan_free: bool,    // module+0x4C
    pop_countdown: u32, // MG_7CF
    caps_count: u32,    // MG_7D0
    caps_window: u64,   // module+0x48
    started: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

fn build(pack: Pack) -> Option<FlyingToilets> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    Some(FlyingToilets {
        pack,
        entities: Vec::new(),
        crowd: 50,
        paper: 0,
        occupant: 0,
        rude: true,
        flap_deadline: 0,
        gag_deadline: 0,
        chan_free: true,
        pop_countdown: 0,
        caps_count: 0,
        caps_window: 0,
        started: false,
    })
}

struct Identity {
    first: u32,
    last: u32,
    w: i32,
    h: i32,
    ax: i32,
    ay: i32,
    /// `+0xd8 = rand()%2 == 0` (fn28 @1D9E)
    fast: bool,
}

impl FlyingToilets {
    /// Crowd -> entity count, @0x06A2: band = raw/20 -> {3,5,8,10,12}.
    fn max_entities(&self) -> usize {
        match self.crowd / 20 {
            0 => 3,
            1 => 5,
            2 => 8,
            3 => 10,
            _ => 12,
        }
    }

    /// §6.1 MF_7CA PickSequence.
    fn pick_identity(&self, ctx: &mut Ctx) -> Identity {
        let r = ctx.rng15.below(100) as u32;
        let (first, last);
        if r < 45 {
            let occ = if self.occupant == 4 { ctx.rng15.below(4) as i32 } else { self.occupant };
            first = match occ {
                0 => 1,
                1 => 20,
                2 => 10,
                _ => {
                    // Aaron; the pig cameo arms at MG_7D0 >= 6
                    if self.caps_count >= 6 && self.pack.seq(BASE, 65).is_some() {
                        65
                    } else {
                        30
                    }
                }
            };
            last = first + self.pack.seq_len(BASE, first) - 1;
        } else if r < 90 {
            let pap = if self.paper == 3 { ctx.rng15.below(3) as i32 } else { self.paper };
            first = match pap {
                0 => 50,
                1 => 45,
                _ => 40,
            };
            last = first + self.pack.seq_len(BASE, first) - 1;
        } else {
            first = 60; // idle plunger; SeqLen deliberately not consulted
            last = 60;
        }

        // union bounds over the run -> collision box (+6 like @0x1D8A)
        let seq = self.pack.seq(BASE, first).expect("identity sequence");
        let (mut x1, mut y1, mut x2, mut y2) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for f in &seq.frames {
            let img = self.pack.image(&f.png);
            x1 = x1.min(f.bx);
            y1 = y1.min(f.by);
            x2 = x2.max(f.bx + img.w as i32);
            y2 = y2.max(f.by + img.h as i32);
        }
        // fn28 @1D9E..1DBC: the speed coin, then the unused `+0xd4` roll
        let fast = ctx.rng15.below(2) == 0;
        let _d4 = ctx.rng15.below(2);
        Identity {
            first,
            last,
            w: (x2 - x1) + 6,
            h: (y2 - y1) + 6,
            ax: (x1 + x2) / 2,
            ay: (y1 + y2) / 2,
            fast,
        }
    }

    fn overlaps(&self, x: i32, y: i32, w: i32, h: i32, skip: usize) -> bool {
        self.entities.iter().enumerate().any(|(i, e)| {
            i != skip
                && e.x > -4000
                && (x - w / 2) < (e.x + e.w / 2)
                && (e.x - e.w / 2) < (x + w / 2)
                && (y - h / 2) < (e.y + e.h / 2)
                && (e.y - e.h / 2) < (y + h / 2)
        })
    }

    fn sound_tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        if !self.started {
            self.started = true;
            self.flap_deadline = now;
            self.gag_deadline = now + 10_000; // first gag @ init+10 s
        }

        // §7.1 wing-flap ambience: continuous randomized bed
        if now > self.flap_deadline {
            let (id, dur) = match ctx.rng15.below(3) {
                0 => (SND_MIX1, MS_MIX1),
                1 => (SND_FLAP1, MS_FLAP1),
                _ => (SND_FLAP2, MS_FLAP2),
            };
            ctx.sounds.push(id);
            self.flap_deadline = now + dur;
            self.chan_free = true;
        }

        // §7.2 plunger Pop: 3-frame delay, single global (overlaps collapse)
        if self.pop_countdown != 0 && self.chan_free {
            if self.pop_countdown >= 3 {
                ctx.sounds.push(SND_POP);
                self.flap_deadline += MS_POP;
                self.chan_free = false;
                self.pop_countdown = 0;
            } else {
                self.pop_countdown += 1;
            }
        }

        // §7.3 Caps Lock: flush + pig counter (freeze quirk preserved:
        // the counter only moves inside this branch)
        if ctx.caps_lock {
            if self.chan_free {
                ctx.sounds.push(SND_FLUSH);
                self.flap_deadline += MS_FLUSH;
                self.chan_free = false;
            }
            if self.caps_count == 0 {
                self.caps_window = now + 5_000;
            }
            if now >= self.caps_window {
                self.caps_count = 0;
            } else {
                self.caps_count += 1;
            }
        }

        // §7.4 periodic gag
        if now > self.gag_deadline && self.chan_free {
            let pick = if self.rude { ctx.rng15.below(2) } else { 0 };
            if pick == 0 {
                ctx.sounds.push(SND_FLUSH);
                self.flap_deadline += MS_FLUSH;
            } else {
                ctx.sounds.push(SND_GASTRO);
                self.flap_deadline += MS_GASTRO;
            }
            self.gag_deadline = now + (ctx.rng15.below(90) as u64) * 1000 + 30_000;
            self.chan_free = false;
        }
    }
}

impl Module for FlyingToilets {
    fn name(&self) -> &'static str {
        "Flying Toilets"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Crowd".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            ControlDef {
                name: "Paper".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    // MENU 1001, from the pack: three papers + Random.
                    items: self.pack.popup_items(1001, 4, false),
                },
                default: 0,
            },
            ControlDef {
                name: "Occupant".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    // MENU 1002, from the pack: nobody, three occupants,
                    // Random.
                    items: self.pack.popup_items(1002, 5, false),
                },
                default: 0,
            },
            ControlDef {
                name: "Rude Sounds".into(),
                kind: ControlKind::Checkbox,
                default: 1,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.crowd = value.clamp(0, 100),
            1 => self.paper = value.clamp(0, 3),
            2 => self.occupant = value.clamp(0, 4),
            3 => self.rude = value != 0,
            _ => {}
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let max = self.max_entities();
        while self.entities.len() < max {
            let id = self.pick_identity(ctx);
            self.entities.push(Entity {
                state: State::SpawnAnywhere,
                first: id.first,
                last: id.last,
                cur: id.first,
                x: -5000,
                y: -5000,
                w: id.w,
                h: id.h,
                fast: id.fast,
                ax: id.ax,
                ay: id.ay,
                // @0x1DDE: the park path arms the deadline too
                next_due: ctx.now_ms + FLIGHT_DELAY_MS,
            });
        }
        self.entities.truncate(max); // Crowd re-read per frame @0x0F18

        for i in 0..self.entities.len() {
            match self.entities[i].state {
                State::SpawnAnywhere => {
                    for _ in 0..50 {
                        let x = ctx.rng15.below(SCREEN_W as u16) as i32;
                        let y = ctx.rng15.below(SCREEN_H as u16) as i32;
                        let (w, h) = (self.entities[i].w, self.entities[i].h);
                        if !self.overlaps(x, y, w, h, i) {
                            let e = &mut self.entities[i];
                            e.x = x;
                            e.y = y;
                            e.state = State::Fly;
                            break;
                        }
                    }
                }
                State::SpawnEdge => {
                    let id = self.pick_identity(ctx);
                    for _ in 0..50 {
                        // @0x168E is the 50-attempt counter test and
                        // @0x1696 the side coin flip — the flip sits INSIDE the
                        // loop, exactly as state 0's coordinate rolls do
                        // (@0x1552 counter, @0x155A rolls). A blocked right-edge
                        // attempt can therefore retry against the top edge.
                        // fn26 @16A6: `rand()%2 == 0` → the top edge, else the side
                        let top = ctx.rng15.below(2) == 0;
                        let (x, y) = if top {
                            (ctx.rng15.below(SCREEN_W as u16) as i32, -id.h / 2)
                        } else {
                            (SCREEN_W + id.w / 2, ctx.rng15.below(SCREEN_H as u16) as i32)
                        };
                        if !self.overlaps(x, y, id.w, id.h, i) {
                            self.entities[i] = Entity {
                                state: State::Fly,
                                first: id.first,
                                last: id.last,
                                cur: id.first,
                                x,
                                y,
                                w: id.w,
                                h: id.h,
                                fast: id.fast,
                                ax: id.ax,
                                ay: id.ay,
                                next_due: ctx.now_ms + FLIGHT_DELAY_MS,
                            };
                            break;
                        }
                    }
                }
                State::Fly => {
                    // §6.3 MF_7C9 @0x181E: the flight step runs at most once
                    // every 40 ms of the After Dark ms clock. That clock is
                    // `TickCount()*16.625`, so with a Mac-tick shell the 40
                    // quantizes up to 3 ticks on its own — see ERRATA.
                    if ctx.now_ms < self.entities[i].next_due {
                        continue;
                    }
                    self.entities[i].next_due = ctx.now_ms + FLIGHT_DELAY_MS; // @0x1832
                    {
                        let e = &mut self.entities[i];
                        e.cur += 1;
                        if e.cur > e.last {
                            if e.first == 55 {
                                e.first = 60;
                                e.last = 60;
                            }
                            e.cur = e.first;
                        }
                    }
                    let (fast, x0, y0, w, h) = {
                        let e = &self.entities[i];
                        (e.fast, e.x, e.y, e.w, e.h)
                    };
                    let mul = if fast { 3 } else { 1 };
                    let (mut nx, mut ny) = (x0 - 2 * mul, y0 + mul);
                    if fast && self.overlaps(nx, ny, w, h, i) {
                        (nx, ny) = (x0 - 2, y0 + 3);
                        if self.overlaps(nx, ny, w, h, i) {
                            (nx, ny) = (x0 - 2, y0 + 1);
                        }
                    }
                    let plunge_roll = ctx.rng15.below(100) == 0;
                    let fast_roll = plunge_roll && ctx.rng15.below(2) != 0;
                    let e = &mut self.entities[i];
                    e.x = nx;
                    e.y = ny;
                    if plunge_roll {
                        e.fast = fast_roll;
                        if e.fast && e.first == 60 {
                            e.first = 55;
                            e.last = 57;
                            e.cur = 55;
                            self.pop_countdown = 1; // MG_7CF arm
                        }
                    }
                    // despawn: exited left or bottom — ORIGINAL BUG: the
                    // bottom test uses the WIDTH (entity[0xDA]).
                    let e = &mut self.entities[i];
                    if (-e.w / 2) > e.x || (SCREEN_H + e.w / 2) < e.y {
                        e.state = State::SpawnEdge;
                        e.x = -5000;
                        e.y = -5000;
                    }
                }
            }
        }

        // §8 `DoDrawFrame` order (`MF_7B0 @0x0F14`): controls, then the entity
        // state machines (@0x0FAC), then the composite (@0x0FF2), and only THEN
        // the sound scheduler (@0x1006). The scheduler used to run first here,
        // which pushed the `MG_7CF` plunger Pop one whole draw frame late — see
        // ERRATA §4.
        self.sound_tick(ctx);
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        for e in &self.entities {
            if e.x <= -4000 {
                continue;
            }
            let Some(f) = self.pack.frame(BASE, e.cur) else { continue };
            out.push(SpriteDraw {
                flip: false,
                pal: 0,
                png: f.png.clone(),
                x: e.x + f.bx - e.ax,
                y: e.y + f.by - e.ay,
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The Mac tick grid, not the flight period: §6.3's 40 ms delay is a
    /// deadline against `Resource.f4724() = TickCount()*16.625`, so it fires
    /// on the 3rd tick = **49.875 ms**, not on a clean 40 ms boundary. The
    /// old 40 ms shell tick ran the flock 28 % fast.
    ///
    /// `toilets.mp4` measures **50.6 ms** per flight step from the wing-flap
    /// recurrence (a 6-frame run cycles every 9.11 video frames on three
    /// isolated toilets) and 50.1–50.7 ms from position, so the Mac grid is
    /// 1.5 % fast against the emulator. The first sweep's "51.5 ms" was
    /// inflated by centroid drift and edge-clipped frames — see the 2026-09-13
    /// header ERRATA §3. The shell used to approximate the grid with
    /// `tick_ms = 17` (2.3 % slow); `TickClock::MacTick` is the real thing.
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::path::Path;

    fn rig() -> (FlyingToilets, Ctx) {
        let pack = Pack::load(Path::new("../assets/flying-toilets")).expect("pack");
        let m = build(pack).expect("build");
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

    /// Tick-quantization sweep, 2026-09-12, re-measured 2026-09-13. §6.3's
    /// flight delay is `entity[0xD0] = now + 40` against the After Dark ms
    /// clock `Resource.f4724() = TickCount()*16.625`, so it fires on the
    /// **3rd Mac tick**, not on a 40 ms boundary. `toilets.mp4` measures
    /// **50.6 ms** per step (wing-flap recurrence, 9.11 video frames per
    /// 6-frame run on tracks f114-378 / f1060-1191 / f1130-1394) and
    /// 50.1-50.7 ms from position; the old 40 ms shell tick was 28 % fast.
    /// Anything that quietly puts `tick_ms` back to 40 has to fail here.
    #[test]
    fn flight_step_takes_three_mac_ticks() {
        let (mut m, mut ctx) = rig();
        assert_eq!(m.clock(), TickClock::MacTick, "the flight gate is an f4724 deadline");
        let mut pace = Pacer::new(&m);
        for from in 1..200 {
            assert_eq!(
                pace.ticks_for_at(from, FLIGHT_DELAY_MS, false),
                3,
                "the 40 ms flight delay must take 3 Mac ticks (from tick {from})"
            );
        }
        let period = pace.mean_period_ms(FLIGHT_DELAY_MS, false, 60);
        assert!(
            (period - 49.875).abs() < 0.2,
            "flight step is {period} ms; the Mac grid says 49.9, toilets.mp4 50.6"
        );

        // and the entity really steps on that grid, not on every tick
        m.set_control(0, 0); // Crowd band 0 -> 3 entities
        for _ in 0..3 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
        }
        let watched = m
            .entities
            .iter()
            .position(|e| e.state == State::Fly && e.x > -4000)
            .expect("an entity in flight");
        let mut moves: Vec<u64> = Vec::new();
        let mut last = m.entities[watched].x;
        for _ in 0..90 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            if m.entities[watched].state != State::Fly {
                break;
            }
            let x = m.entities[watched].x;
            if x != last {
                moves.push(ctx.now_ms);
                last = x;
            }
        }
        assert!(moves.len() > 8, "entity barely moved ({} steps)", moves.len());
        let per = (moves[moves.len() - 1] - moves[0]) as f64 / (moves.len() - 1) as f64;
        assert!(
            (per - 49.875).abs() < 1.0,
            "entity advanced every {per:.1} ms; the grid says 49.9, the capture 50.6"
        );
    }

    /// Drive the module one shell tick, with the one-shot sound slot held
    /// open so §7.2's `module[0x4C]` gate never masks what we are measuring.
    fn draw_frame(m: &mut FlyingToilets, ctx: &mut Ctx, pace: &mut Pacer) {
        pace.advance(ctx);
        ctx.sounds.clear();
        m.chan_free = true;
        m.tick(ctx);
    }

    /// ERRATA 2026-09-13 §3 + §6/§7. The capture's wing-flap recurrence is the
    /// tightest clock this module has: `c_004` is 7 px taller than the other
    /// five frames of every 6-frame run, and on three isolated winged toilets
    /// (`toilets.mp4` tracks f114-378, f1060-1191, f1130-1394) that tall frame
    /// comes round every 9.11 video frames = 303.6 ms. That is only possible if
    /// the compound frame advances exactly once per flight step, so the cycle
    /// pins both the animation cadence and the Mac-tick step at once.
    #[test]
    fn wing_flap_is_one_art_frame_per_flight_step() {
        let (mut m, mut ctx) = rig();
        m.set_control(0, 60); // the capture's panel: Crowd "Not Pretty"
        m.set_control(2, 0); // Occupant = Nobody, so every toilet is run 1-6
        assert_eq!(
            m.max_entities(),
            10,
            "Crowd 60 is band 3 -> 10 entities; toilets.mp4 peaks at exactly 10 sprites"
        );
        // the rig's 0xC0FFEE ANSI stream deals nine TP rolls in a row and one
        // winged toilet at the bottom-left; seed 1 opens with a toilet
        ctx.rng15 = Random15::new(1);
        let mut pace = Pacer::new(&m);
        for _ in 0..12 {
            draw_frame(&mut m, &mut ctx, &mut pace);
        }
        // the six-frame flyer with the most screen ahead of it (the ANSI
        // rand() stream can park an early pick near the exit edge)
        let watched = m
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.state == State::Fly && e.x > -4000 && e.last == e.first + 5)
            .max_by_key(|(_, e)| e.x + (SCREEN_H - e.y))
            .map(|(i, _)| i)
            .expect("a six-frame winged toilet in flight");
        let first = m.entities[watched].first;
        let mut wraps: Vec<u64> = Vec::new();
        let mut last_cur = m.entities[watched].cur;
        for _ in 0..500 {
            draw_frame(&mut m, &mut ctx, &mut pace);
            let e = &m.entities[watched];
            if e.state != State::Fly || e.first != first {
                break;
            }
            if e.cur != last_cur {
                let want = if last_cur == first + 5 { first } else { last_cur + 1 };
                assert_eq!(e.cur, want, "the compound frame advances by exactly one per flight step");
                if e.cur == first {
                    wraps.push(ctx.now_ms);
                }
                last_cur = e.cur;
            }
        }
        assert!(wraps.len() >= 5, "only {} flap cycles observed", wraps.len());
        let cycle = (wraps[wraps.len() - 1] - wraps[0]) as f64 / (wraps.len() - 1) as f64;
        assert!(
            (cycle - 6.0 * 49.875).abs() < 6.0,
            "flap cycle is {cycle:.1} ms; the Mac grid says 299.25, toilets.mp4 303.6"
        );
    }

    /// ERRATA 2026-09-13 §7. `toilets.mp4` measures -1.33 px per video frame
    /// for normal flight and -4.00 for fast — a clean 3.0x — on an exact 2:1
    /// slope (measured 2.000 / 2.014 / 2.017 over three full traverses).
    #[test]
    fn flight_step_is_minus_two_plus_one_and_three_times_that_when_fast() {
        let (mut m, mut ctx) = rig();
        m.set_control(0, 0); // Crowd band 0 -> 3 entities, plenty of room
        let mut pace = Pacer::new(&m);
        for _ in 0..4 {
            draw_frame(&mut m, &mut ctx, &mut pace);
        }
        let i = m
            .entities
            .iter()
            .position(|e| e.state == State::Fly && e.x > -4000)
            .expect("an entity in flight");
        for (fast, want) in [(false, (-2, 1)), (true, (-6, 3))] {
            let mut moved = 0;
            for _ in 0..120 {
                m.entities[i].fast = fast;
                let e = &m.entities[i];
                let (x0, y0, w, h) = (e.x, e.y, e.w, e.h);
                // §6.3's fast-mode fallback steps only exist when the target
                // overlaps; skip those so this test measures the clean step.
                let blocked = fast && m.overlaps(x0 + want.0, y0 + want.1, w, h, i);
                draw_frame(&mut m, &mut ctx, &mut pace);
                if m.entities[i].state != State::Fly {
                    break;
                }
                let d = (m.entities[i].x - x0, m.entities[i].y - y0);
                if d == (0, 0) || blocked {
                    continue;
                }
                assert_eq!(
                    d, want,
                    "fast={fast}: §6.3's step is (-2,+1) times {}",
                    if fast { 3 } else { 1 }
                );
                moved += 1;
                if moved == 5 {
                    break;
                }
            }
            assert!(moved > 0, "fast={fast}: the entity never took a clean step");
        }
    }

    /// ERRATA 2026-09-13 §6. Of 49 edge entries tracked in `toilets.mp4`,
    /// 26 came in off the right edge and 23 off the top, and **none** from the
    /// left or the bottom — `MG_7D3` is never 1, so the mirror-image branches
    /// are dead. The 50/50 split is @0x1696's coin flip.
    #[test]
    fn respawns_enter_only_at_the_right_edge_or_the_top() {
        let (mut m, mut ctx) = rig();
        m.set_control(0, 100); // Crowd "Phew!" -> 12 entities, fastest churn
        let mut pace = Pacer::new(&m);
        let (mut right, mut top) = (0u32, 0u32);
        let mut before: Vec<State> = Vec::new();
        for _ in 0..6000 {
            before.clear();
            before.extend(m.entities.iter().map(|e| e.state));
            draw_frame(&mut m, &mut ctx, &mut pace);
            for (i, e) in m.entities.iter().enumerate() {
                if before.get(i) != Some(&State::SpawnEdge) || e.state != State::Fly {
                    continue;
                }
                assert!(
                    e.x >= SCREEN_W || e.y < 0,
                    "entity re-entered at ({}, {}); §6.2 only spawns off the right edge \
                     (x = w + box/2) or the top (y = -box/2)",
                    e.x,
                    e.y
                );
                if e.x >= SCREEN_W {
                    right += 1;
                } else {
                    top += 1;
                }
            }
        }
        assert!(right + top > 40, "only {} respawns observed", right + top);
        let share = f64::from(right) / f64::from(right + top);
        assert!(
            (0.35..=0.65).contains(&share),
            "right-edge share is {share:.2} of {} respawns; @0x1696 is a coin flip and \
             toilets.mp4 measured 26 right / 23 top",
            right + top
        );
    }

    /// ERRATA 2026-09-13 §4. `DoDrawFrame` (§8) runs the entity state machines
    /// at @0x0FAC and the sound scheduler at @0x1006, so the draw frame that
    /// arms `MG_7CF` (@0x1AEE) is also the one that bumps it 1 -> 2, and the
    /// Pop lands **two** draw frames later. The port used to call `sound_tick`
    /// first, which stretched that to three.
    #[test]
    fn plunger_pop_lands_two_draw_frames_after_the_plunge() {
        let (mut m, mut ctx) = rig();
        m.set_control(0, 100); // 12 entities -> a plunger turns up sooner
        let mut pace = Pacer::new(&m);
        let mut armed = None;
        for _ in 0..8000 {
            draw_frame(&mut m, &mut ctx, &mut pace);
            if m.entities.iter().any(|e| e.first == 55) {
                armed = Some(m.pop_countdown);
                break;
            }
        }
        let counted = armed.expect("no plunger plunged in 8000 draw frames");
        assert_eq!(
            counted, 2,
            "the draw frame that arms MG_7CF must also bump it (sound_tick runs last)"
        );

        draw_frame(&mut m, &mut ctx, &mut pace);
        assert_eq!(m.pop_countdown, 3, "second draw frame: 2 -> 3, still silent");
        assert!(!ctx.sounds.contains(&SND_POP), "the Pop is one frame early");

        draw_frame(&mut m, &mut ctx, &mut pace);
        assert!(
            ctx.sounds.contains(&SND_POP),
            "§7.2's Pop must fire on the second draw frame after the plunge"
        );
        assert_eq!(m.pop_countdown, 0, "MG_7CF is a single global and clears on fire");
    }

    /// CONTRADICTIONS 2026-09-13, item 10: the plunge run's packed heights
    /// are 27 / 26 / 29 and the idle run is flat **27** — not "an otherwise
    /// flat 26-px series". 26 is one frame, `c_056`, the squash.
    /// `toilets.mp4` confirms the pack frame-for-frame at the f241 plunge
    /// (42x27 288 px → 43x26 287 px → 43x29 302 px → 42x27 288 px, against
    /// the PNGs' own 288 / 287 / 302 non-background pixels), so these are
    /// both the packed numbers and the on-screen ones.
    #[test]
    fn the_plunge_frames_are_27_26_29_and_the_idle_run_is_flat_27() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/flying-toilets")) else {
            return;
        };
        let h = |id: u32| pack.frame(BASE, id).map(|f| (f.w, f.h));
        // run 55 — the plunge
        assert_eq!(h(55), Some((42, 27)), "c_055");
        assert_eq!(h(56), Some((43, 26)), "c_056 — the squash, the ONLY 26");
        assert_eq!(h(57), Some((43, 29)), "c_057 — the splat pose");
        // run 60 — the idle plunger, flat 27
        for id in [60u32, 61, 62] {
            assert_eq!(h(id), Some((42, 27)), "c_{id:03} is the flat idle height");
        }
        // the losing reading, pinned
        let heights: Vec<i32> = [55u32, 56, 57, 60, 61, 62]
            .iter()
            .filter_map(|&id| h(id).map(|(_, hh)| hh))
            .collect();
        assert_eq!(
            heights.iter().filter(|&&hh| hh == 26).count(),
            1,
            "exactly one 26-px plunger frame; the series is not flat 26: {heights:?}"
        );
        assert_eq!(
            heights.iter().filter(|&&hh| hh == 27).count(),
            4,
            "27 is the plunger baseline: {heights:?}"
        );
    }

    /// ERRATA 2026-09-13 §6: `toilets.mp4` shows exactly three on-screen size
    /// classes and nothing between them — winged toilet 105-126 x 94-107,
    /// TP roll 41x17, plunger 41x26 (packed unions less 1-2 px of threshold
    /// shrink). These are the derived collision boxes, §3's `BOX` table.
    #[test]
    fn identity_boxes_are_the_spec_box_table() {
        let (mut m, mut ctx) = rig();
        let mut seen: Vec<(u32, i32, i32)> = Vec::new();
        for occ in 0..4 {
            for pap in 0..3 {
                m.occupant = occ;
                m.paper = pap;
                for _ in 0..300 {
                    let id = m.pick_identity(&mut ctx);
                    let row = (id.first, id.w, id.h);
                    if !seen.contains(&row) {
                        seen.push(row);
                    }
                }
            }
        }
        seen.sort_unstable();
        assert_eq!(
            seen,
            vec![
                (1, 115, 108),  // Nobody
                (10, 128, 113), // Bessie
                (20, 130, 112), // Dumpin' Dan
                (30, 132, 113), // Aaron
                (40, 49, 25),   // TP roll, Cow Print
                (45, 49, 25),   // TP roll, Pastel
                (50, 49, 25),   // TP roll, White
                (60, 48, 33),   // idle plunger
            ],
            "§3's BOX table (union + 6, @0x1D8A/@0x1D9E); the pig run 65 needs MG_7D0 >= 6"
        );
    }

    /// Length in ms of a packed sound, straight out of its RIFF header —
    /// `pack_assets.py` writes the sndS-**expanded** WAV, which is exactly
    /// what §7's `Duration(snd)` measures on the emulator.
    fn packed_ms(id: u32) -> u64 {
        let p = std::path::Path::new("../assets/flying-toilets/sounds").join(format!("{id}.wav"));
        let b = std::fs::read(p).expect("packed sound");
        let u32at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) as u64;
        let u16at = |o: usize| u64::from(u16::from_le_bytes([b[o], b[o + 1]]));
        // walk the RIFF chunks — `1007.wav` is float32 with an `smpl` chunk
        // in front of `data`, so a fixed offset-40 read is wrong for it.
        let (mut rate, mut chans, mut width, mut bytes) = (0u64, 1u64, 1u64, 0u64);
        let mut o = 12;
        while o + 8 <= b.len() {
            let (id, sz) = (&b[o..o + 4], u32at(o + 4) as usize);
            match id {
                b"fmt " => {
                    chans = u16at(o + 10);
                    rate = u32at(o + 12);
                    width = u16at(o + 22) / 8;
                }
                b"data" => bytes = sz as u64,
                _ => {}
            }
            o += 8 + sz + (sz & 1);
        }
        let frames = bytes / (width.max(1) * chans.max(1));
        (frames * 1000 + rate / 2) / rate // ms, rounded
    }

    /// ERRATA 2026-09-13 (QEMU audio) §1. `module[0x40] = now + Duration(snd)`
    /// and `Duration` is the **expanded** sample — the one `pack_assets.py`
    /// ships. `flying-toilets-long.wav` (300 s, 1281 flaps) puts the mode of
    /// the flap-to-flap interval exactly on the clip that just played:
    /// 210.3 ms after Flap 1 (n = 441), 175.8 after Flap 2 (n = 418),
    /// 253.4 after Mix 1 (n = 421). §7's table quotes 97 / 82 / 124 ms, which
    /// are the still-compressed raw rips — half-length, and the reason the
    /// port used to flap at twice the original's rate. Anything that puts the
    /// raw-rip numbers back has to fail here.
    #[test]
    fn the_flap_bed_periods_are_the_packed_sample_lengths() {
        if !std::path::Path::new("../assets/flying-toilets/sounds/1001.wav").exists() {
            return;
        }
        for (id, konst, name) in [
            (SND_FLAP1, MS_FLAP1, "Flap 1"),
            (SND_FLAP2, MS_FLAP2, "Flap 2"),
            (SND_MIX1, MS_MIX1, "Mix 1"),
            (SND_POP, MS_POP, "Pop"),
        ] {
            let packed = packed_ms(id);
            assert_eq!(
                konst, packed,
                "snd {id} {name}: §7's period must be the packed sample ({packed} ms), not {konst}"
            );
            assert!(
                konst > 150,
                "snd {id} {name} is back on the half-length raw rip ({konst} ms)"
            );
        }
        // the gag slots are NOT sample lengths: 2750 is @0x112C's literal and
        // Gastro's rip is 1.486 s of which the emulator plays 184 ms (§3/§4).
        assert_eq!(MS_FLUSH, 2750, "@0x112C/@0x11C4 hard-code 0x0ABE");
        assert_eq!(
            MS_GASTRO, 184,
            "the bed returns 0.184 s after the Gastro onset at wav t = 195.167 AND 291.357; \
             the rip's full 1486 ms is not what plays"
        );
        assert!(
            packed_ms(SND_GASTRO) > 1400,
            "the packed rip really is the long one — MS_GASTRO is deliberately not it"
        );
    }

    /// ERRATA 2026-09-13 (QEMU audio) §3. §7.4's gag is not occupant-gated,
    /// not Caps-Lock-gated and not a rare roll — audio-captures.md's "never
    /// fired in 60 s" was a sampling artefact. `flying-toilets-long.wav` gets
    /// four in 300 s with Rude Sounds ON: Flush at 8.94 s (= init + 10 s, the
    /// Demo press is ~1 s before frame 0), Flush at 116.15, Gastro at 195.17,
    /// Gastro at 291.36 — gaps 107.2 / 79.0 / 96.2 s, and 2 of each from the
    /// coin flip.
    #[test]
    fn the_periodic_gag_fires_at_ten_seconds_then_every_thirty_to_one_nineteen() {
        let (mut m, mut ctx) = rig();
        m.set_control(3, 1); // Rude Sounds ON, as the capture's panel has it
        let mut pace = Pacer::new(&m);
        let mut gags: Vec<(u64, u32)> = Vec::new();
        while pace.now_ms() < 300_000 {
            pace.advance(&mut ctx);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            for id in &ctx.sounds {
                if *id == SND_FLUSH || *id == SND_GASTRO {
                    gags.push((ctx.now_ms, *id));
                }
            }
        }
        assert!(
            (3..=9).contains(&gags.len()),
            "300 s at 30-119 s a gag should give 3-9, got {}: {gags:?}",
            gags.len()
        );
        assert!(
            (9_900..=10_100).contains(&gags[0].0),
            "@0x0B00's 0x2710 puts the first gag at init + 10 s, got {} ms",
            gags[0].0
        );
        for w in gags.windows(2) {
            let gap = w[1].0 - w[0].0;
            assert!(
                (30_000..=119_999).contains(&gap),
                "§7.4's `30000 + (rand % 90) * 1000` bounds the gap; got {gap} ms"
            );
        }
        assert!(
            gags.iter().any(|g| g.1 == SND_GASTRO) && gags.iter().any(|g| g.1 == SND_FLUSH),
            "with Rude Sounds ON @0x1188 is a coin flip; the capture got 2 of each: {gags:?}"
        );

        // and with Rude Sounds OFF it is always the flush (@0x11A0)
        let (mut m, mut ctx) = rig();
        m.set_control(3, 0);
        let mut pace = Pacer::new(&m);
        while pace.now_ms() < 300_000 {
            pace.advance(&mut ctx);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            assert!(
                !ctx.sounds.contains(&SND_GASTRO),
                "Gastro needs Rude Sounds; @0x118C tests module[0x3E] first"
            );
        }
    }
}
