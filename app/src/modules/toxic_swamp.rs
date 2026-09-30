//! Toxic Swamp — full-fidelity transcription of the RE spec
//!
//! ## Fidelity lane, 2026-09-29 (Opus 5.5) — Jason: "something was odd with
//! ## the sinking gangster"
//! - **Larry sinks on his own art and lands through 700's ramp.** Measured
//!   per frame on `qemu/toxic-swamp`, `qemu/toxic-swamp-leftcap` and the
//!   2026-09-13 capture (table at `LARRY_SINK_STOP`): the descent is +4 a
//!   frame with a +5 wherever run 679 plays 685 → 686, the landing is
//!   `B, B+4, B+6, B+6, B+7` (700 701 702 703), and the rest bottom varies
//!   run to run (475 / 479 / 477). That is fn51 tick 0x15 (@11DC) read
//!   literally — fn54's links are the only mover, the exit is
//!   `480 − (+0x130)/2 − 10 <= +0x126` on the frame just advanced onto
//!   (threshold pinned to 405..406 by the three captures), enter 0x16 arms
//!   700 with a plain SetRun. The port replaced all of it with an anchor
//!   walked 4 px a tick on a fixed grid, stopped at 474 and snapped +3 onto
//!   a pinned 477: a +7 jump where the original steps +4 +2 0 +1, and the
//!   same spot every run. The anchor, `reanchor` and the three LARRY_SINK_*
//!   constants are gone; Larry is an ordinary placement point. Before/after
//!   on the golden panel, same seed: landing 466 470 → 477 477 477 477 vs
//!   467 471 → 475 477 477 478; decay ladder timing unchanged (860 at
//!   65.9 s, 874 at 70.0 s both), lint jerk 0 / backwards 2.6 % both.
//!   Decay ladder in `qemu/toxic-swamp` (module start ≈ −3.0 s): 766
//!   family from ≈ 34 s module time, 808 ≈ 38.6 s, 835 ≈ 50.6 s, 860
//!   ≈ 56.8 s, 874 ≈ 61 s — inside the existing 57-65 s tableau ratchet.
//! - **Slots 1/2 (958 / 900) are placed, 12-13 minutes above the world.**
//!   The old UNRESOLVED note below said MF_7A7 case 2 writes no placement;
//!   true, but fn51 enters 1/5 (0x8001/0x8005) do: x rolled across the
//!   world, `+0x126 = top − (+0x130)/2`, deadline `(rand%60) s + 12 min`,
//!   sink depth `rand % (+0x130) + (+0x130)/2`. The port already did that
//!   (the "dead hand in open water" was the prose-era port), except that
//!   fn55 cases 1/2 store their box as (h, w) where cases 0/3 store (w, h)
//!   — an ORIGINAL BUG (SKELETON_BOX_SWAPPED). Read (w, h), 958 (75x25)
//!   hangs with its bottom row on screen row 0: 12 lit px for 12 minutes,
//!   which no golden shows (rows 0-2 of all three QEMU captures carry no
//!   static pixel). Read (h, w) as the C does, it sits wholly above
//!   (bottom −24; 900 bottom −7). Port before: 958 bottom 1 / 900 bottom
//!   0; after: −24 / −7. Needs footage: what they do after the 12-minute
//!   drop (sink to 37..111 px, drift off the left) — only a ≥ 13 min
//!   Fish-Only-off take would show it.
//! - **The "pipe builder" builds nothing; it was sitting 10 px inboard.**
//!   Compound 9 and runs 12/31/51 all carry the same pipe + coral at one
//!   bank position (right edge 499); what animates is green sludge puffing
//!   out of the mouth. So in both the port and the goldens the pipe is
//!   whole from the first frame and never grows or slides (port: the
//!   piece's right/bottom edge is 640/474 on every draw for 71 s). The
//!   sludge does play in the goldens too — `-leftcap` bursts at 4.5-8.5,
//!   37.5-41.5, 64.5-68.5, 86.5-90.5 s, `qemu/toxic-swamp` at 2-6.5,
//!   19.5-23, 37-42 s — just beside the region audio-captures.md diffed.
//!   What WAS wrong is where the piece sits: fn55 case 4 places compound
//!   7's centre and then `fn59(1, 7)` links 7 → 9 (+10 px, −3 px; x
//!   negated when flipped). The port centred 9 on the cap, so its flat
//!   end covered the cap's round mouth. Goldens / port before / after:
//!   right 565 / 555 / 565, left −1 / 9 / −1, top 363 / 363 / 363; the
//!   left cap tile itself −1 / 0 / −1 (GAP, see `paint_floor`).
//!
//! ## Installed-saver fix, 2026-09-19 (Opus 5) — Jason on the INSTALLED
//! ## saver: "animation on fish is off, some going backwards, most are
//! ## jerky" / "I never got the dude to actually die even though I set
//! ## Lungs to Thimble"
//! Both defects only bite OUTSIDE the dev tree, which is why the 2026-09-16
//! motion-model pass below could measure itself clean and still be wrong.
//!
//! - **Backwards + jerky: mirroring was a runtime FILE WRITE.** The art
//!   library mirrors a compound at BLIT time — MF_7A8's toggle
//!   (@0x2C10/@0x2CAA) for the reef cap, the `+0x11c` facing word (fn12
//!   @44E0 state 0x1d) for every critter. This port faked that by encoding
//!   a mirrored `m_NNN.png` INTO THE PACK DIRECTORY the first time a
//!   compound was drawn flipped, remembering the id in a `HashSet`, and then
//!   drawing the mirrored art only `if mirrored.contains(&fb_cur)` — while
//!   `blit_x` placed the mirrored BOX unconditionally. The write is
//!   `let Ok(file) = File::create(..) else { return };`: when it fails — a
//!   read-only or sandboxed pack, i.e. exactly an installed `.saver` — every
//!   rightward critter drew its UNMIRRORED art inside a mirrored box. A fish
//!   swimming right while facing left, jittering as the box walked 96 → 255
//!   px underneath it. Measured by rendering the same tick against a
//!   `chmod a-w` pack: all four blue X-ray fish face LEFT while still
//!   travelling right; after the fix that render is pixel-identical to the
//!   writable one. `SpriteDraw.flip` already existed and `compose` honours
//!   it, which is what the original does anyway, so the module now sets the
//!   flag and generates nothing.
//! - **Larry never dies: MG_7B3 is a latch, not an accumulator.** Listing
//!   @0x0106 reads the After Dark ms clock (the same call `DoDrawFrame`
//!   uses for MG_7D3) and @0x010A stores it into MG_7B3; the five Lung Capacity branches then ADD
//!   {30 s, 2, 5, 10, 20 min} to THAT. The port carried only the `+=`, so
//!   the deadline accumulated. `DoDrawFrame` @0x3F9C restarts the module on
//!   ANY control change and the panel's sliders are continuous (the saver's
//!   `sliderMoved` fires per pixel of the drag), so DRAGGING the Lung slider
//!   to Thimble stacked one 30 s band per re-init: measured 3 300 000 ms —
//!   55 minutes — after a 26-step drag, with Larry parked in state 25. The
//!   act of setting the control is what stopped him dying. Now `now + band`,
//!   and at Thimble he reaches the 39 ↔ 40 tableau at 57-65 s over six
//!   seeds against the capture's ~63 s.
//! - **fn19 @676E off-screen** tests the placement point against the world
//!   edges with `+0x134`/`+0x136` — the box of the BIRTH run's first frame,
//!   measured once by fn17 @62C2 (@0x655E) and never re-measured. The port
//!   measured the LIVE frame, which on this pack swells 96 → 255 px as a
//!   trail streams out behind the fish, so a wisp counted as on-screen for
//!   cycles after its body had gone. Latched into `Actor::birth_box`.
//!
//! Panel words (resolved): the slider words once came from a hand-typed
//! table in `app/src/lib.rs` that sat one band high for Lungs and Critters.
//! They now come straight from the pack's `sUnt 1000/1001` rows, whose
//! thresholds are 0/20/40/60/80 — matching `lung_band()`'s `raw/20` (the
//! divide by 20 @0x0118) and the golden capture's own panel (Critters thumb
//! at 61 % of the track reads the fourth word with 9 critters on the reel;
//! Lung thumb at raw ≈ 18 reads the first word on the 30 s clock).
//!
//! ## Motion-model fix, 2026-09-16 (Opus 5) — Jason: "fish animations are
//! ## REAL jerky, some fish blinked across the screen"
//! Three defects, one family: the port advanced the flipbook by summing each
//! frame's OFst offset into the placement point, where §2's model says the
//! offset is part of the frame's own origin and the placement point moves by
//! the frame-to-frame LINK.
//! - **Jerky.** `fn16` @61D4 (and its slot twin `fn54` @2056) is called with
//!   arg 1 at every site (@486A and twins), and its wrap test is
//!   `cur >= last` AFTER the advance — so the run's LAST frame is never
//!   drawn. It is the link marker: it repeats the first frame's art carrying
//!   the loop-back offset (211/227 −112, 615/621 −22, 656/669 −128, 465/479
//!   −157) or the next run's authored origin (525/536 → 538, 538/550 → 552,
//!   552/561 → 563, 591/595 → 597). The port wrapped at `cur > last`, so it
//!   drew the marker and then drew `first` on top of it at the same place —
//!   one frozen duplicated frame every cycle, one commit in seven on the red
//!   fish's 7-frame run 615. And because it took the wrap delta from the
//!   marker's raw `dx` instead of `centre(last) − centre(first)`, a marker
//!   with `dx = 0` lost the whole hand-off: the piranha's 525 hover snapped
//!   +29 px back every cycle and never went anywhere.
//! - **Blinked across the screen.** `fn12` state 0x1d toggles the facing word
//!   `+0x11c` and touches nothing else; the art library mirrors the compound
//!   about the sprite, so the fish turns where it is. This port mirrors about
//!   `ox`, a bank-space anchor a few hundred px away, so the bare toggle threw
//!   the piranha −368 px in one commit at its 0x1d → 0x15 turn (measured on
//!   the trace test). `set_flip` now re-derives `ox` through the frame's
//!   centre, which is the same pixels as "pos unchanged, art mirrored".
//! - **The 2026-09-15 ledger's floating pipe at ~150 s.** `SetRun` does not
//!   move the placement point (the linked form adds `link(old_last, new)`
//!   first, §2), but `arm` used to absorb the blit jump and pin the new run's
//!   TOP onto the old frame's top. The reef builder's growth runs (`fn51`
//!   9 → 10 → 11 → 12 → 9, compounds 9/12/31/51) are authored on one bottom
//!   line — `by + h` = 417 throughout — while `by` steps 292 → 300 and `h`
//!   steps 125 → 117, so pinning tops walked the pipe 8 px up per hand-off,
//!   22 px per builder cycle, ~150 px by t = 150 s.
//! `bl(f) = bx + dx` / `cn(f) = bl + (w/2, h/2)` and `arm_as` are where this
//! lives; the capture-measured `ox + authored bx` blit model is unchanged
//! apart from folding in `dx`, which the old per-frame accumulation was
//! double-counting on every run whose interior frames carry an offset
//! (229/238/249/259/267/303-family/343/353/405/440/513/563/623/629/639/645,
//! and both skeleton drops, whose per-frame `dy` ramps were being integrated
//! into a quadratic).
//!
//! ## Port-from-decompile pass, 2026-09-15 (Fable) — M129_M129.c, private
//! Both machines were re-transcribed from the Ghidra C (totally-twisted
//! docs/decompiled/toxic-swamp): `fn51` @02E0 (machine A, the reef tableau,
//! states 0..0x28) and `fn12` @44E0 (machine B, the fish, states 0..0x25),
//! with the helpers `fn53`/`fn54` (slot steppers), `fn15`/`fn16` (critter
//! steppers), `fn17` @62C2 (birth), `fn18` @6618 (the shoaling test), `fn19`
//! @676E (off-screen), `fn20`/`fn59` (enter-time delta pre-apply), `fn55`
//! @2148 (slot reset by role), `fn57`/`fn58` (pickers), `fn25` @3F6E
//! (DoDrawFrame). What the prose-era transcription had wrong:
//! - every random site is ANSI `rand()` (75 of them; no RandomBelow) and the
//!   enter deadlines are `now + (rand()%n)*1000 + add`, not `rand*1000/n`;
//! - the steppers are per state, not a global pre-tick step: `Loop`
//!   (fn16/fn54, arg 1) restarts the run, `Hold` (fn15, arg 0) parks on the
//!   last frame, `Shot` (fn53) reports `last <= cur`; states 1/5 (A) and 0
//!   (B) do not step at all;
//! - machine B: states 7/9/10 never transition on completion (they only
//!   swim off and re-birth), 2/3/0x19 gate on completion alone, 4/5 also on
//!   the deadline, 0xd takes the shoaling exit on ANY tick a species-4
//!   neighbour is within 70 px, 0x1d TOGGLES the flip, 0x22 nudges x by
//!   ∓2 (no flip), enters 0x1f..0x22 carry `(rand%3|2|2|3)*1000` deadlines;
//! - machine A: 1/5 drop the skeletons in from above the screen at a random
//!   x (sink target `rand%h + h/2`), 4/8 reset the slot (`fn55` again) once
//!   it drifts off the LEFT edge, 9 and 14/15/24/27/30/33/36/39 gate on the
//!   local deadline (not completion), the flopper table's fifth pair is
//!   142/189 and variant 2 always plays 201, the skeleton/flopper pickers
//!   add their 3 s holds only on picks 0/1 and 1/3, enters 3/7/8/10..13
//!   start one frame early, enters 0xe/0xf hold `(rand%10)*1000 + 5`, the
//!   mirrored pipe builder applies the `g0000`/`g0024`/`g0048`/`g005E`
//!   per-frame x corrections, enters 0x1f/0x25 roll their run;
//! - birth: the flip word is reconciled to `+0x11e`, which nothing in M129
//!   writes (fn11 @441E failed to decompile — GAP: rolled once per critter),
//!   the off-screen test is direction-aware and only in the states that
//!   call `+0x138`, the shoaling test only counts species-4 neighbours.
//! Kept from the capture-verified passes: the reef floor tiler, the slot
//! placement approximations, Larry's anchored sink (SUPERSEDED 2026-09-29:
//! the anchor was not the C; see the top section), the mirrored blit model
//! (`ox` + authored `bx`, which is what the capture measured).
//! (totally-twisted docs/behavior/toxic-swamp.md, cross-checked line-by-line
//! against .scratch-toxic/{ann,statesA,statesB}.txt — where the doc's §7.2/§8.1
//! sketches and the annotated listing disagree, the listing wins and the
//! deviation is noted in APPROXIMATIONS). Two Berkeley StateMachines:
//! machine A (MF_7A3, 40 states — the reef tableau) and machine B (MF_7DE,
//! 37 states — per-fish wander), plus the shared engine protocol: every tick
//! ends in a commit whose return is the draw clock (MG_7D3) and
//! `+0x120 = clock + 100` — the 100 ms per-actor floor (@0x1F62/@0x6086),
//! which on After Dark's own clock is a 106.4 ms frame (see the ERRATA).
//!
//! Faithful behavior implemented here:
//! - Critters slider: `count = max(raw*15/100, 1)` (MF_7C6 @0x3894, 1..15)
//! - Lung Capacity: band = raw/20 -> stage deadline += {30 s, 2, 5, 10,
//!   20 min} (MF_7A0 @0x0118); MG_7B3 is LATCHED to the ms clock first
//!   (the clock read @0x0106, stored @0x010A) and the band added to
//!   that, so the deadline is `now + band` at every init — it does NOT
//!   accumulate (see the 2026-09-19 fix note above)
//! - 9 background SLOTS with distinct ROLES: MF_7A7 (RESET world-logic)
//!   switches on the slot index (+0x132, cases @0x21A0..0x2AA6) and dispatches
//!   each slot's entry state: 0 -> 19 (eye-tree), 1 -> 5 (skeleton 958),
//!   2 -> 1 (skeleton 900), 3 -> 21 (Larry the Lawyer), 4 -> 9 (coral/pipe
//!   builder), 5 -> 14 (pipe 96, then the 14<->15 ping-pong), 6/7/8 -> 16/17/18
//!   (static pipes 98/100/102 — their ticks never transition @0x0FC2/0x0FD8/
//!   0x0FEE). This is how the machine reaches the Larry phase; the doc's §7.2
//!   linear reading missed the MF_7A7 dispatch entirely.
//! - Machine A enters/ticks/deadlines/bubbles from statesA.txt: skeletons
//!   900/914/924/958/986/998 (dl rand·1000/60 + 720 000), the reef-building
//!   loop 9 -> 10(12) -> 11(31) -> 12(51) -> 9 (dl rand·1000/20 + 10 000) with
//!   the eye-tree 19(105/152/199) <-> 20(MF_7AA flopper) cycle, then Larry
//!   21(672) .. 40(874) with the five MG_7B3 expiry branches (ticks
//!   24/27/30/33/36: expired -> advance, else loop through the sub-cycle
//!   25/28/31/34/37) and the re-arms at ticks 26/29/32/35, ending in the
//!   endless 39 <-> 40 tableau (tick 40 @0x1EC8 -> 39; no terminal)
//! - Caps-Lock gag is in TICK 9 (@0x0AF2): MG_7D0 >= 6 && rand%2 routes the
//!   coral-builder slot into state 13 (pipe 1046) instead of 10 (the STR# 133
//!   tagline)
//! - Bubbles (snd 1000) fire ONLY in enters 10/11/12/13 — exactly the four
//!   L134_0DA6 calls @0x0B94/0x0C52/0x0D10/0x0DF4 (§6.1); no loop
//! - Machine B: 37 wander states with the concrete per-state sequences
//!   (statesB.txt: 211/229/238/249/259/267/293/303-family/353/343/364/383/395/
//!   405/427/440/450/465+481/498/513/525/538/552/563/571/577/591/597/615/623/
//!   629/639/645/656/1021/1039), deadlines (rand·1000/3 + 1000 family), the
//!   E1/E2 stepper split, MF_7E4 70 px proximity turns (ticks 13/16 only) and
//!   MF_7E5 off-screen re-birth (§8; statesB)
//! - Birth (MF_7E3 @0x62C2, per the disasm): sticky Caps-Lock latch (MG_7E9)
//!   forces species value 4 and is consumed; MG_7D0 >= 6 widens the roll to
//!   rand%8 (tiny-swimmer bucket), else rand%7 — 0-based buckets straight from
//!   the switch at @0x6310 (0 -> state 1, so the doc's §12 quirk #4
//!   "max(species,1)" does NOT exist in the binary; see APPROXIMATIONS)
//! - Caps-Lock watch (@0x4242): sticky latch + 5 s window + frame counter,
//!   counter only moves while the key is down (ORIGINAL BUG kept, same as
//!   Flying Toilets' MG_7D0 freeze quirk)
//! - Fish Only: the per-frame slot-tick skips (@0x3FE2/@0x40E8) are the
//!   *second* gate — the first is at init (@0x3B3A), and it skips building
//!   the slots at all, so there is no reef to freeze (see the reef notes
//!   below). NOTE: the species roll is NOT gated on this control in the
//!   binary (the rand%8 gate is the Caps-Lock counter @0x62E2); spec §8.1's
//!   contrary reading was flagged low-confidence there and is corrected
//!   here per the disasm.
//! - Live control change -> module restart request (DoDrawFrame @0x3F9C);
//!   approximated as an in-place re-init (see APPROXIMATIONS)
//! - Runtime palette (§4 step 7 + §6 step 6): `clut 1200` ("toxy pal") is
//!   pushed at init and re-pushed every frame — wired as `SpriteDraw.pal =
//!   CLUT_TOXY` on every sprite (engine slot-indexed remap = LoadCLUT
//!   recolour-in-place; the packed frames are palette-indexed). clut 1200 is
//!   also this pack's `base_clut` — it IS the palette the 9000/9001/9002 art
//!   is baked in — so the remap resolves to the identity and the frames draw
//!   in their authored colours. That is correct, not a missing feature: the
//!   original is re-pushing the palette it is already displaying. The depth-4
//!   branch (clut 1216) is unreachable here — see APPROXIMATIONS.
//!
//! - THE REEF FLOOR (MF_7A8 @0x2B5E, decoded 2026-09-01 — spec §8.3 calls
//!   this "the non-Fish-Only variant of the avoidance logic"; it is nothing
//!   of the kind, it is the terrain tiler). It runs ONCE, from the first
//!   slot's RESET tick behind the MG_7B2 one-shot gate (@0x0612 tests
//!   MG_7B2, @0x061E calls MF_7A8, @0x0622 clears it), and blits its result
//!   into the canvas background (@0x2E44 canvas->vtbl[0xCC]) — so the whole
//!   floor draws BEHIND every sprite. Algorithm, verbatim:
//!     * rand()%2 (@0x2B6A) picks an end for the pipe cap, compound 7
//!       (corrugated pipe + sand): right → rect {W−w7, H−h7, W, H}
//!       (@0x2B94); left → the art library's mirror is switched on
//!       (vtbl[0x0C] @0x2C10/@0x2CAA) and the rect is {0, H−h7, w7−1, H}
//!       (@0x2C20 — the minus-one @0x2C4E is where the row's 95/191/287
//!       tile grid comes from).
//!     * that rect is stashed in MG_7B0 (@0x2CBA) as the pipe's keep-out box.
//!     * loop @0x2CE6: while the last rect is still inside the world rect,
//!       lay the next tile hard against it — `rand()%2` picks sand strip 1
//!       or 3 (@0x2CFE), then `rand()%3 == 0` diverts ONE tile (D6 latch) to
//!       compound 5, the toilet + tyre (@0x2D16); the toilet's rect is
//!       stashed in MG_7B1 (@0x2E14). Every tile is bottom-aligned to the
//!       world rect (`top = world.bottom − h`, @0x2D62/@0x2DB6).
//!     * because the bounds test at the loop head runs on the tile ALREADY
//!       drawn, the first tile that overhangs is still painted (and
//!       clipped) — which is why the golden reel's row runs past x = 640.
//! - THE REEF ANCHOR (MF_7A0 @0x00AA..0x00FC): MG_7B4 = Point(world.left +
//!   w(1)/2, world.bottom − h(1)/2) — the centre of compound frame 1, the
//!   sand strip, parked in the world's bottom-left corner. Every static reef
//!   piece starts from MG_7B4 and is then moved by the art library's
//!   frame-link delta from frame 1 (MF_7AB @0x306C with applyDy=1), so each
//!   piece keeps its authored `by` offset relative to that tile. On 640x480
//!   that resolves to a single layer offset, `y = by + (SCREEN_H − 423)` =
//!   `by + 57`, where 423 is the shared design bottom of the terrain strips.
//!   Measured against the 2026-09-01 golden reel (toxic-swamp demo, t=150,
//!   frames de-scaled from the 640x508/28px-title-bar capture): seq 1 top
//!   446, seq 3 446, seq 5 413, seq 199 403, seq 100 452, seq 102 447,
//!   seq 96 373 — the first six are exactly `by + 57`, and seq 96 is
//!   `by + 57 + 20`, the extra +20 being MF_7A7 case 5's
//!   `addi.w [A2+0x0126], 0x14` @0x293E.
//! - MF_7A7 cases 4-8 place the dressing: case 4's growing pipe/coral builds
//!   from whichever end MF_7A8 put the pipe cap on (it tests MG_7B0.left
//!   @0x2724) and is mirrored when that end is the left (+0x136 = 1
//!   @0x2742); cases 5-8 re-roll x as `rand % (world.right − w + w/2)`, used
//!   as a CENTRE (@0x2832..@0x285A and twins @0x29C4, @0x2A6A, @0x2B10).
//! - Fish Only skips the reef ENTIRELY at init (the `+0x7A` flag test
//!   @0x3B3A branches to 0x3BA6, over both MF_7A0 and the 9-slot creation loop), so
//!   there is no floor and no dressing to freeze — bare water plus fish.
//!
//! ORIGINAL BUGS kept:
//! - MG_7D0 counter frozen while Caps Lock is up (only touched inside the
//!   caps branch, @0x4242 block)
//! - dead mirror field +0x11E written with the flip but never read (not
//!   replicated; noted)
//!
//! ## ERRATA — golden capture `emu/captures/toxic-swamp.mp4` (2026-09-13,
//! ## critter/Larry pass; 120 s, DEPTH=32, panel unknown but Lung Capacity
//! ## reads as Thimble, ~9-10 critters on screen)
//!
//! Method: crop `1276:958:2:56` and decimate 2:1 in numpy (per the lane
//! brief — never `scale=…:flags=neighbor`, it ghosts 1-px features), then
//! template-match the packed compounds against the frames. Module x =
//! measured x + 1 (the crop drops one column); module y = measured y.
//!
//! - **Critters are NOT born at a random point in the world — they enter
//!   across a side edge.** Over 120 s there are ~46 edge entries and ~43
//!   edge exits and NOT ONE fish ever materialises in open water. The
//!   module's own first frame (video t = 2.833-2.900; the reef floor and all
//!   the dressing are already complete and static) carries no critter at
//!   all, and the first pixels cross the left and right borders one module
//!   frame later. The listing agrees and is more specific than §8.1:
//!   MF_7E3 @0x653E..0x6602 takes the armed sequence's bounds, sets
//!   `+0x12C = rand % (world.bottom − h) + h` — ONE rand, the vertical — and
//!   then sets `+0x12A` with NO roll at all: `world.right + w/2` when
//!   `+0x11C == 0`, `world.left − w/2` otherwise. The placement point is a
//!   centre, so the sprite's near edge starts flush ON the edge it is
//!   swimming away from. `birth` now does exactly that.
//! - **The flip mirrors the art AND the travel.** `dx` is 0 on every
//!   non-wrap frame of every fish run; the entire swim lives in the authored
//!   `bx` column, which walks LEFT (e.g. run 211: 486 → 381 over 16 frames,
//!   with the wrap frame carrying `dx = −112`, i.e. 6.6 px per module
//!   frame). A rightward fish is therefore the mirrored blit of the same
//!   run. Measured on the green eel that enters at the left at t = 3.0:
//!   matching run 656 MIRRORED scores 9.4 mean abs error against 45+
//!   unmirrored, and `blit_x + bx + w` is constant within a run cycle
//!   (583/584 → 711/712 → 839/840 → 967, stepping by the wrap frame's own
//!   `dx = −128`). That is `ox − (bx + w)` with `ox` accumulating `−dx` —
//!   `Actor::mirror_x` + `blit_x`. The port used to draw every critter
//!   unmirrored and swim them all left.
//! - **Authored speeds check out** (px per 106.4 ms frame, art vs capture):
//!   211 gar 6.6 / 6.6, 383 puffer 3.6 / 3.4, 465 white skeleton 10.5 /
//!   ~10.6, 656 green eel 9.14 / 9.7, 615 red fish 3.1 / ~3.
//! - **"Shoaling" is an artefact of simultaneous birth, not a mechanism.**
//!   Because x is a pure function of (armed frame, entry edge), two critters
//!   of the same species born on the SAME tick heading the SAME way hold an
//!   exact x lockstep with a constant Δy. The capture's two green eels
//!   (born at init, entering left at t = 3.0) share an identical x extent
//!   frame for frame for their whole life — t = 4.7 both x 8..171, t = 5.4
//!   both x 73..237 — at Δy = 247, because species 6 is state 35 and state
//!   35 has no transitions at all. The two tan puffers that enter at the
//!   right at t = 4.3 do the same at Δy = 50 until their state machines take
//!   different random branches at t ≈ 6.8 and drift apart. Mid-run births
//!   are staggered and solitary: of the ~37 post-init edge entries, none is
//!   a simultaneous same-species pair. So there is NO leader/follower link
//!   and no formation code to port — MF_7A7's "shoal cohesion" (spec §8.3)
//!   is the 9-case slot dispatch, and MF_7E4's 70 px test (ticks 13/16
//!   only) is all the neighbour logic there is.
//! - **Species mix observed** (7 buckets, caps-lock never pressed, so
//!   bucket 7 is unreachable and indeed compound 1021 never appears):
//!   0/211 many-eyed gar, 1/383 tan spiky puffer, 2/465 white skeletal
//!   wisp (rare), 3/498 blue X-ray fish, 4/525 grey piranha, 5/615 bloody
//!   red fish, 6/656 green eel. Entry y-centres span 22..440 — the whole
//!   screen height, reef band included. 2-4 of a species on screen at once
//!   is routine and is just the birthday paradox over ~9-10 critters.
//! - **(SUPERSEDED 2026-09-29 — the numbers stand, the reading does not:
//!   474 is compound 700 drawn on the exit point, not a sink stop 6 px
//!   short, and 477 is this run's rest, not a constant.)**
//!   **Larry: 4 px per FRAME, and he stops 3 px lower than the port had
//!   him.** 2-D template fit of runs 672/679/690/700, x pinned at the drawn
//!   left 466: bottom walks 26 30 34 38 … (t = 3.5-6.1) and ends
//!   462 466 470 474 | 476 477 477 477 …, i.e. the sink is +4 per module
//!   frame (37.8 px/s = 4 px / 105.8 ms, which is the MacTick frame, NOT
//!   100 ms), the grid is bottom ≡ 2 (mod 4) so @0x2690 starts him 2 px
//!   below the world top, and the sink exits at 474 — 6 px short of the
//!   world bottom, not the spec's 10. The last 3 px are compound 700's own
//!   design-bottom ramp (284 286 286 287 …) as state 22 takes over. He is
//!   still at 477 at t = 17, 30, 60, 90 and 119 s: he lands once, never
//!   leaves, never re-enters.
//! - **Larry's decay show runs on schedule.** After landing at t ≈ 15.6 he
//!   holds the 672-764 family (states 22/24/25) until t ≈ 34.7 — one
//!   Thimble window (30 s) after module start — then steps through
//!   766-806 (26-31) at 34.7-49 s, 808-833 (32-34) at 50-58 s, 835-856
//!   (35-37) at 58-62 s, and 860-897 (38/39/40) from t ≈ 63 s to the end.
//!   The five MG_7B3 expiry branches and the endless 39 ↔ 40 tableau are
//!   both confirmed.
//! - **The reef floor is exact.** MF_7A8 put the cap at the RIGHT this run:
//!   compound 7 fits at x 544 (= 640 − 96) top 363 (= 480 − 117), and the
//!   row walks left on the 96 px grid — 448, 352, 256, 160, 64, −32 — with
//!   compound 5 (toilet + tyre) taking the 256 slot at top 413 (= 480 − 67)
//!   and sand strips 1/3 elsewhere at top 446 (= 480 − 34). Seven tiles, the
//!   last one overhanging at −32, exactly what `paint_floor` produces. The
//!   dressing is exact too: compound 96 at top 373 (= by 296 + 57 + the
//!   case-5 nudge 20), 98 at 447, 102 at 447, 199 at 403, the pipe builder's
//!   9/31 at 363 — every one `by + 57`, and the builder sits on the capped
//!   (right) end. Only the rolled x values differ, as they must.
//! - **Animated non-fish, complete list:** the anemone's 14 ↔ 15 ping-pong
//!   (compound 96 ↔ run 64-94, the two red tendrils waving at x ≈ 347), the
//!   eye-tree's 19 ↔ 20 flop, the pipe/coral builder's growth run on the cap
//!   end, and Larry. **There is no bubble sprite**: snd 1000 is audio only
//!   and the pack's 110 runs contain nothing smaller than compound 1021
//!   (50x18). The 1-3 px specks drifting in the water cluster around moving
//!   sprites and the reef and are consistent with encoder mosquito noise.
//! - **(RESOLVED 2026-09-29 — see the top section: the enters DO place
//!   them, 12-13 minutes above the world, and the golden is right that
//!   nothing shows.)** **UNRESOLVED — machine-A slots 1 and 2 (compounds
//!   900 and 958) are invisible in the capture.** MF_7A7 case 2 @0x24B4 arms 0x384 = 900 and
//!   measures its bounds into +0x12E/+0x130 exactly like Larry's case 3, but
//!   writes NO world placement, so the port falls back to jitter over the
//!   authored (288, 96) and parks a dead hand in open water. 120 s of
//!   capture show nothing there or anywhere else (a static sprite would be
//!   in the median background; it is not). Cases 1-3 stay flagged in
//!   APPROXIMATIONS; the next capture should sit on a Fish-Only-off run long
//!   enough to see whether these two ever surface.
//!
//! ## ERRATA — golden capture `emu/captures/toxic-swamp.mp4` (2026-09-12,
//! ## tick-quantization sweep)
//!
//! - **The 100 ms actor floor is a deadline on the Mac tick grid.** §6/§8's
//!   commit epilogue arms `+0x120 = clock + 100` (@0x1F62, @0x6086) and the
//!   engine clock `MG_7D3` is the After Dark ms clock
//!   (`TickCount()*16.625`), so an actor's next step is the first Mac tick
//!   at or past +100 ms — it is a floor, not a 100 ms metronome. Measured:
//!   a Rayleigh periodogram of the screen-change events in
//!   `toxic-swamp.mp4` over t = 20-45 s (232 events above a 45 %-of-median
//!   floor, crop 56 px + halve to module px) peaks at **P = 107.52 ms,
//!   R = 0.371**; R at a flat 100 ms is 0.045. So the port's 100 ms shell
//!   tick was ~7 % fast. The module now rides `TickClock::MacTick` and
//!   `SM_FLOOR_MS` quantizes itself to **106.40 ms** — see below.
//! - Side effect, and it is the faithful one: `caps_watch` and the critter
//!   census now run at the draw rate rather than at the actor rate, which is
//!   where the disasm puts them (@0x4242 sits in `DoDrawFrame`, outside both
//!   machine passes). Every state-machine commit still sits behind
//!   `Actor::next_tick`.
//! - **RESOLVED 2026-09-12 (engine lane).** The sweep's note here said the
//!   real Mac would give 7 ticks (116.375 ms) for a 100 ms delay and that
//!   the capture's 107.5 sat between 102 and that. Wrong premise:
//!   `Resource.f4724()` is the TRUNCATED integer `TickCount()*16.625`
//!   (`(t<<4) + (t*0xA006>>16)` = 16, 33, 49, 66, 83, 99, 116 …), so
//!   `now + 100` tested with `>=` fires after 6 ticks six times in ten and
//!   7 the rest — mean **106.40 ms**, which IS the ~107 family the captures
//!   measure (this module 107.5, phlegm-boy 106.7, mikes 106.5, chameleon
//!   106.6-106.9). Nothing between 102 and 116 needs inventing.

use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, TickClock, SpriteDraw, SCREEN_H, SCREEN_W,
};
use std::collections::HashMap;

const BASE: u32 = 9000;
const SND_BUBBLES: u32 = 1000;

/// Compound 1 — the sand strip whose bounds define the reef anchor MG_7B4
/// (MF_7A0 @0x00AA passes 1 to the art library's vtbl +0x18).
const FLOOR_REF: u32 = 1;
/// Compound 7 — corrugated pipe on a sand strip; MF_7A8's end cap, and the
/// bounds MF_7A7 case 4 measures for the growing pipe (@0x26E6).
const FLOOR_END: u32 = 7;
/// MF_7A8 @0x2CFE: `rand()%2` — non-zero picks strip 1, zero picks strip 3.
const FLOOR_SAND: [u32; 2] = [3, 1];
/// MF_7A8 @0x2D16: `rand()%3 == 0` swaps ONE tile for the toilet + tyre.
const FLOOR_JUNK: u32 = 5;
/// MF_7A7 cases 5-8 (@0x27CE/0x295A/0x2A00/0x2AA6): the static dressing —
/// anemone mound, weed clump, anemone mound, weed clump.
const DRESSING: [u32; 4] = [96, 98, 100, 102];
/// MF_7A7 case 5 @0x293E: `addi.w [A2 + 0x0126], 0x14` — the big anemone
/// alone sits 20 px lower than the reef anchor puts it.
const DRESSING_Y_NUDGE: i32 = 20;

/// Machine-A tick 21 (@0x11DC) is the SINK, and it has no sink of its own:
/// the Loop stepper (fn54 @2056) moves `+0x126` by the art's frame links and
/// nothing else, so Larry falls exactly as fast as his descent runs are
/// authored — each frame's design bottom `by + h + dy` climbs +4 per frame
/// (672 → 199 … 215, 679 → 215 … 239 **244** 248, 690 → 252 … 280), the
/// one irregular step being 679's 239 → 244. The only exit is
/// `world.bottom − (+0x130)/2 − 10 <= +0x126` (@0x1292..@0x12B6), where
/// `+0x130` is compound 672's height measured ONCE by MF_7A7 case 3 (@0x2550)
/// and `+0x126` is the placement point — the CENTRE of whatever frame the
/// stepper just advanced onto. This is the constant in that test
/// (the constant −10 @0x12AA).
///
/// **Re-verified 2026-09-29 against three goldens** (per-frame NCC fit of the
/// block + feet, 20 fps, native 640x480 QEMU captures; the old 1280x1016
/// capture's numbers from its 2026-09-13 fit):
///
/// | capture | descent grid | last sink frame → 700 → 701 702 703 | rest |
/// |---|---|---|---|
/// | `qemu/toxic-swamp` | ≡ 3, one +5 at 163 → 168, then ≡ 0 | 468 → 472 → 474 474 475 | 475 |
/// | `qemu/toxic-swamp-leftcap` | ≡ 3, one +5 at 215 → 220, then ≡ 0 | 472 → 476 → 478 478 479 | 479 |
/// | `toxic-swamp` (2026-09-13) | ≡ 2 | 470 → 474 → 476 477 | 477 |
///
/// All three are that test read literally: with `+0x130/2 = 65` the exit is
/// the first advance whose centre reaches 405, and a centre P draws a 131 px
/// frame's bottom at P + 66 — so 700 lands at 472 (P 406), 474 (P 408) and,
/// via 675 → 676's +12 centre jump, 476 (P 410), then walks its own
/// 284 → 286 → 286 → 287 design ramp. The rest bottom is NOT a constant; it
/// depends on the grid phase the 679 cycles leave behind (475 / 477 / 479
/// observed). The port used to walk a capture-tuned anchor 4 px a tick on a
/// fixed ≡ 2 grid, stop it at 474 and jump +3 onto a pinned 477: one +7 snap
/// where the original does +4, +2, 0, +1, and the same 477 on every run.
const LARRY_SINK_STOP: i32 = 10;
/// The drawn bottom compound 672 starts at. GAP(1 px): @0x2690 sets
/// `+0x126 = world.top − h/2` = −65, which draws 672 (h 131) with its bottom
/// at 1; the 2026-09-13 capture's grid is ≡ 2 from the first measured frame
/// (26, 30, 34 … at t = 0.6 s, before any 679 cycle can shift it), and both
/// QEMU captures read ≡ 3 = 2 + one 679 shift. Kept at the measured 2.
const LARRY_SPAWN_BOTTOM: i32 = 2;

/// §4 step 7 (@0x3A10..0x3A30): palette = `clut 1216` ("Toxic 16 Palette")
/// if screen depth 4 else `clut 1200` ("toxy pal"), pushed when depth is 4
/// or 8. §6 step 6 re-pushes it every frame. The sim renders full-color
/// (the depth-8 branch), so every sprite draws through clut 1200.
const CLUT_TOXY: u16 = 1200;

/// The StateMachine per-actor tick floor: `+0x120 = clock + 100` after every
/// commit (@0x1F62 machine A, @0x6086 machine B).
const SM_FLOOR_MS: u64 = 100;


/// Lung Capacity band additions to MG_7B3 (MF_7A0 @0x0118): raw/20, 0-based.
const LUNG_BANDS: [u64; 5] = [
    30_000, // 0x7530     "Thimble"
    120_000, // 0x1D4C0   "Cup"
    300_000, // 0x493E0   "Quart"
    600_000, // 0x927C0   "Gallon"
    1_200_000, // 0x124F80 "Barrel"
];

/// MF_7A9 @0x2E7C: skeleton-critter picker, rand()%7 -> sequence
/// (immediates @0x2EBA..0x2F1E: 0x2C8/0x2CA/0x2CC/0x2D5/0x2DD/0x2E4/0x2ED).
const SKELETON_PICKS: [u32; 7] = [712, 714, 716, 725, 733, 740, 749];

/// MF_7AA @0x2F2E: flopper picker, rand()%5 -> (tree A stage, tree B stage).
/// The slot's variant word (+0x134) toggles which of the twin eye-trees
/// grows this round (@0x2F7C tst +0x134 / paired immediates 0x6B/0x9A,
/// 0x77/0xA6, 0x79/0xA8, …).
const FLOPPER_STAGES: [(u32, u32); 5] = [
    (107, 154),
    (119, 166),
    (121, 168),
    (128, 175),
    (142, 189),
];

/// MF_7A7 RESET dispatch (@0x242C/0x249E/0x250C/0x26A8/0x27B8/0x2944/0x29E6/
/// 0x2A8C/0x2B32): slot role (index, stored in +0x132) -> machine-A entry
/// state. Case 0 also spawns the eye-tree family (105/152/199) before
/// dispatching 19; cases 5-8 arm pipes 96/98/100/102 before 14/16/17/18.
const ROLE_ENTRY: [u8; 9] = [19, 5, 1, 21, 9, 14, 16, 17, 18];

/// Machine B enter table (statesB.txt): state -> (seq, dl_div, dl_add).
/// dl_div == 0 -> no local deadline set by this enter (transition is
/// stepper-completion only). Deadlines are `now + rand*1000/div + add`.
const B_ENTER: [(u32, u32, u64); 38] = {
    let mut t = [(0u32, 0u32, 0u64); 38];
    t[1] = (211, 3, 1000);
    t[2] = (229, 3, 1000);
    t[3] = (238, 3, 1000);
    t[4] = (249, 3, 1000);
    t[5] = (259, 3, 1000);
    t[6] = (267, 0, 0);
    t[7] = (293, 0, 0);
    t[8] = (303, 0, 0); // picker 303/313/323/333 handled in the arm
    t[9] = (353, 0, 0);
    t[10] = (343, 0, 0);
    t[11] = (364, 0, 0);
    t[12] = (383, 3, 1000);
    t[13] = (395, 3, 1000);
    t[14] = (405, 0, 0);
    t[15] = (427, 0, 0);
    t[16] = (440, 3, 2000);
    t[17] = (450, 0, 0);
    t[18] = (465, 0, 0);
    t[19] = (498, 4, 3000);
    t[20] = (513, 4, 3000);
    t[21] = (525, 0, 0);
    t[22] = (525, 0, 0);
    t[23] = (538, 3, 2000);
    t[24] = (552, 0, 0);
    t[25] = (563, 3, 2000);
    t[26] = (571, 0, 0);
    t[27] = (577, 0, 0);
    t[28] = (591, 0, 0);
    t[29] = (597, 0, 0);
    t[30] = (615, 5, 2000);
    // enters 0x1f..0x22: fn20 pre-apply + `(rand%3|2|2|3)*1000` with no base
    t[31] = (623, 3, 0);
    t[32] = (629, 2, 0);
    t[33] = (639, 2, 0);
    t[34] = (645, 3, 0);
    t[35] = (656, 0, 0);
    t[36] = (1021, 4, 3000);
    t[37] = (1039, 4, 3000);
    t
};

/// MF_7E3 @0x6310 switch: species value (0-based!) -> birth state.
const SPECIES_BIRTH_STATE: [u8; 8] = [1, 12, 18, 19, 22, 30, 35, 36];

// ---------------------------------------------------------------------------
// Actors

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Slot,
    Critter,
}

#[derive(Clone, Copy)]
struct Actor {
    /// Slot vs critter (the original's two CFront classes; kept for parity
    /// with MF_7A2/MF_7DD even though the shared script doesn't branch on it).
    #[allow(dead_code)]
    kind: Kind,
    /// StateMachine state word (+0x96).
    state: u8,
    /// Flipbook (+0x128/+0x12A/+0x12C machine A, movement run +0x12E family
    /// machine B): first/last/cur compound-frame ids.
    fb_first: u32,
    fb_last: u32,
    fb_cur: u32,
    /// Set when the run wraps (MF_7A5/MF_7A6/MF_7E1/MF_7E2 completion signal:
    /// the steppers return 1 once `+0x12C >= +0x12A`, @0x2036/@0x2110).
    fb_done: bool,
    /// Stage offset over the authored compound placement (bx/by): birth
    /// randomization + cumulative link deltas.
    ox: i32,
    oy: i32,
    /// Flip flag (+0x11C critters, +0x136 slots). (+0x11E mirror written too
    /// in the original but never read — dead field, quirk #5; not replicated.)
    flip: bool,
    /// Draw this actor's compound mirrored — the art library's vtbl[0x0C]
    /// toggle. Set for every flipped critter and for the reef's growing pipe
    /// when MF_7A8 parked the pipe cap at the left edge (MF_7A7 case 4
    /// @0x2742). It goes straight into `SpriteDraw.flip`, which mirrors at
    /// blit time exactly as the original does.
    mirror: bool,
    /// Mirror this actor's *motion* as well as its art: the blit x becomes
    /// `ox − (bx + w)` instead of `bx + ox`, so the run's authored leftward
    /// `bx` walk reads as rightward travel. Set for every flipped CRITTER
    /// (`flip` ⇒ `mirror` ⇒ `mirror_x`); never for the reef's growing pipe,
    /// which is mirrored art at an explicitly-computed `ox`.
    ///
    /// Measured (toxic-swamp.mp4 t = 4.6-8.6, green eel entering from the
    /// left): matching run 656's frames MIRRORED beats unmirrored 9.4 vs
    /// 45+ mean abs error, and `blit_x + bx + w` is constant within a run
    /// cycle (583/584 → 711/712 → 839/840 → 967, stepping by the wrap
    /// frame's own `dx` = −128). That is exactly `ox − (bx + w)` with `ox`
    /// accumulating `−dx`, which `step_flipbook` already does for `flip`.
    mirror_x: bool,
    /// Slots: role/index (+0x132, MF_7A7 case selector) and eye-variant
    /// (+0x134). Critters: species (+0x120).
    variant: i32,
    /// `(w, h)` of the compound MF_7A7 case 3 measured ONCE into
    /// `+0x12E/+0x130` (@0x2550, compound 672) — the box the x roll @0x25A4
    /// and tick 21's exit test @0x12A2 are phrased in. Never re-measured.
    ref_box: (i32, i32),
    /// The skeleton drops' sink target (`+0x4d`, enters 1/5): `rand % h + h/2`.
    sink_to: i32,
    /// Local deadline (+0x138/+0x13A); 0 = none set.
    deadline: u64,
    /// Next-eligible tick (+0x120): clock + 100 (the SM floor).
    next_tick: u64,
    /// Slots: the fn55 role (the slot index, +0x132).
    role: i32,
    /// Critters: `+0x134`/`+0x136` — the (w, h) of the BIRTH run's first
    /// frame, measured once by `fn17` @62C2 (the `vtbl[0x18]` GetBounds over
    /// `+0x12e`, @0x655E: `+0x134 = right − left`, `+0x136 = bottom − top`).
    /// `fn19` @676E tests the placement point against the world edges with
    /// THIS half-box, not the box of whatever frame is on screen, and it is
    /// never re-measured when a later state arms a different run. Matters
    /// because half this pack's fish runs swell their box 96 → 255 px as a
    /// trail streams out behind them: measuring live, a wisp's growing trail
    /// kept it "on screen" for cycles after its body had gone.
    birth_box: (i32, i32),
    /// Critters: the `+0x11e` flip word every birth reconciles to (GAP: rolled
    /// once per critter object, see `birth`).
    home_flip: bool,
    /// `cur = first − 1` enters: the next step shows `first` without advancing.
    hold_first: bool,
}

impl Actor {
    fn new(kind: Kind) -> Self {
        Actor {
            kind,
            state: 0,
            // 0 = "never armed". Frame numbering starts at 1, so this is a
            // safe sentinel, and it is what keeps the first `arm` from
            // absorbing a placement jump off a frame the actor never wore.
            fb_first: 0,
            fb_last: 0,
            fb_cur: 0,
            fb_done: false,
            ox: 0,
            oy: 0,
            flip: false,
            mirror: false,
            mirror_x: false,
            variant: 0,
            ref_box: (0, 0),
            sink_to: 0,
            deadline: 0,
            next_tick: 0,
            role: 0,
            birth_box: (0, 0),
            home_flip: false,
            hold_first: false,
        }
    }
}

/// One painted reef-floor tile. MF_7A8 blits these into the canvas
/// background (@0x2E44), so they are emitted before every sprite and never
/// animate: the golden reel's floor is pixel-identical at t=146 and t=160.
#[derive(Clone)]
struct FloorTile {
    png: String,
    /// MF_7A8's art-library flip toggle (vtbl[0x0C] @0x2C10/@0x2CAA) for the
    /// left-end pipe cap. Handed to `SpriteDraw.flip`, which mirrors the
    /// blit — the same thing the toggle does — instead of writing a
    /// pre-mirrored PNG next to the pack.
    flip: bool,
    x: i32,
    y: i32,
}

pub struct ToxicSwamp {
    pack: Pack,
    slots: Vec<Actor>,
    critters: Vec<Actor>,
    /// The MF_7A8 tile row, painted once per init.
    floor: Vec<FloorTile>,
    /// True when MF_7A8 put the pipe cap at the LEFT edge — i.e. MG_7B0.left
    /// is 0, which is exactly what MF_7A7 case 4 branches on (@0x2724).
    floor_pipe_left: bool,
    /// SCREEN_H − bottom(compound 1): the reef layer's bottom-anchor offset
    /// (the `+57` the golden reel measures). Resolved at init.
    reef_dy: i32,
    /// Pixel size of the compounds the floor/dressing placement needs;
    /// `Pack::image` hits the disk, so resolve each once.
    dims: HashMap<u32, (i32, i32)>,
    // controls (raw values, exactly as GetControlValue returns them)
    critters_raw: i32, // 0..100 slider
    lung_raw: i32,     // 0..100 slider
    fish_only: bool,   // xVal 1002
    // latched snapshot (module+0x76/0x78/0x7A) for the change detector
    latch: (i32, i32, i32),
    changed: bool,
    /// MG_7B3 — Larry's stage deadline. LATCHED to the clock at every init
    /// and then offset by the Lung Capacity band (listing @0x0106/@0x010A);
    /// ticks 0x1a/0x1d/0x20/0x23 re-arm it to `now + rand%5 s + 3 s`.
    stage_deadline: u64,
    /// MG_7D0 — Caps-Lock frame counter; MG_7E9 — sticky latch;
    /// module+0x72 — 5 s window.
    caps_count: u32,
    caps_latch: bool,
    caps_window: u64,
    started: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    if !pack.meta.series.contains_key(&BASE.to_string()) {
        return None;
    }
    Some(Box::new(ToxicSwamp::new(pack)))
}

impl ToxicSwamp {
    fn new(pack: Pack) -> ToxicSwamp {
        ToxicSwamp {
            pack,
            slots: Vec::new(),
            critters: Vec::new(),
            floor: Vec::new(),
            floor_pipe_left: false,
            reef_dy: 0,
            dims: HashMap::new(),
            critters_raw: 50,
            lung_raw: 50,
            fish_only: false,
            latch: (50, 50, 0),
            changed: false,
            stage_deadline: 0,
            caps_count: 0,
            caps_latch: false,
            caps_window: 0,
            started: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared actor helpers (associated fns: they only touch the actor + pack, so
// the tick loops can work on local copies without aliasing self).

/// Resolve a compound-frame id to the (first, last) of its OFst run.
/// Ids quoted by the spec that are not run starts (150, 781, 712, 714 …)
/// resolve to the enclosing run, or the next run when the id falls in a gap
/// of the global frame numbering.
fn resolve_run(pack: &Pack, id: u32) -> (u32, u32) {
    let series = pack.series(BASE);
    let mut next: Option<(u32, u32)> = None;
    for s in series {
        let first = s.first;
        let last = first + s.frames.len() as u32 - 1;
        if (first..=last).contains(&id) {
            return (first, last);
        }
        if first > id && next.is_none() {
            next = Some((first, last));
        }
    }
    next.unwrap_or((1, 1))
}

/// Arm a run on an actor (`SetRun`), in the two forms §2 names.
///
/// The library's placement point is `pos = centre(cur) + ox`, and `SetRun`
/// does not move it — it only changes which frame is drawn under it, so the
/// blit becomes `pos − w_new/2`. In this port's `bl + ox` terms that is
/// `ox += centre(old_cur) − centre(new_first)`. The LINKED form (`fn59`
/// @306C for slots, `fn20` @6828 for critters, both called with (1,1), which
/// push two frame numbers into the sequence's `+0x78` link) adds
/// `pos += link(old_last, new)` first, which cancels the `new` term:
/// `ox += centre(old_cur) − centre(old_last)`.
///
/// Both forms are hand-offs the ART carries: every run in this pack ends on a
/// link marker whose `bl` is the next run's authored origin (525/536 → 538,
/// 538/550 → 552, 591/595 → 597, and the whole fish chain), and the reef
/// builder's growth runs (`fn51` 9 → 10 → 11 → 12 → 9, compounds
/// 9/12/31/51) are all authored on ONE bottom line, `by + h` = 417.
///
/// This used to absorb the blit jump instead — pin the new run's first frame
/// onto wherever the old frame was drawn. For the fish chain that is within a
/// pixel of the same thing, but the builder's runs end on their real last
/// frame and step `by` 292 → 300 while `h` steps 125 → 117, so pinning TOPS
/// walked the growing pipe 8 px up the screen at every hand-off — 22 px per
/// builder cycle, ~150 px by t = 150 s. That is the pipe piece floating
/// mid-water in the 2026-09-15 ledger note.
///
/// A never-armed actor (`fb_cur == 0`) has no point to carry: it lands on
/// whatever placement its spawn computed. Before that was fixed, every fresh
/// slot and critter inherited the jump off the placeholder frame 1 — compound
/// 1 is the sand strip at (115, 389) — which buried Larry in the reef floor
/// on his first frame and dropped every newborn fish ~389 px too low.
fn arm_as(pack: &Pack, a: &mut Actor, seq_id: u32, linked: bool) {
    let (first, last) = resolve_run(pack, seq_id);
    let spawn_at = seq_id.clamp(first, last);
    let anchor = if linked { a.fb_last } else { spawn_at };
    if a.fb_cur != 0 {
        if let (Some(fc), Some(fa)) = (pack.frame(BASE, a.fb_cur), pack.frame(BASE, anchor)) {
            let (cx, cy) = cn(fc);
            let (ax, ay) = cn(fa);
            a.ox += if a.flip { ax - cx } else { cx - ax };
            a.oy += cy - ay;
        }
    }
    a.fb_first = spawn_at;
    a.fb_last = last;
    a.fb_cur = spawn_at;
    a.fb_done = false;
}

/// The plain `SetRun`.
fn arm(pack: &Pack, a: &mut Actor, seq_id: u32) {
    arm_as(pack, a, seq_id, false);
}

/// A frame's authored blit origin, `bx + dx` — the compound rect origin plus
/// the frame's OWN OFst offset. This is the whole of the §2 sprite model
/// expressed in pack terms: the library's `centre(f)` is `bl(f) + (w/2, h/2)`
/// and `Draw` blits at `pos − (w/2, h/2)`, so while `pos` tracks `centre(cur)`
/// — which is exactly what the `link(d6, d6+1)` advance maintains — the blit
/// is `bl(cur) + K` with K CONSTANT for the whole run. The frame offsets are
/// therefore a lookup, never an accumulator: the pre-2026-09-16 port summed
/// `dx` into `ox` on every advance, which double-counted every run whose
/// interior frames carry an offset (229/238/249/259/267/303-family/343/353/
/// 405/440/513/563/623/629/639/645, and both skeleton drops).
fn bl(f: &engine::FrameRef) -> (i32, i32) {
    (f.bx + f.dx, f.by + f.dy)
}

/// Library `centre(f)` in bank space (L135 `fn3A70`).
fn cn(f: &engine::FrameRef) -> (i32, i32) {
    (f.bx + f.dx + f.w / 2, f.by + f.dy + f.h / 2)
}

/// Where `frame` actually blits for this actor. Unmirrored actors hang off
/// the authored `bl`; `mirror_x` actors are reflected about the placement
/// point, which is what turns each critter run's authored leftward walk
/// into rightward travel (see `Actor::mirror_x`).
fn blit_x(f: &engine::FrameRef, a: &Actor) -> i32 {
    if a.mirror_x {
        a.ox - (bl(f).0 + f.w)
    } else {
        bl(f).0 + a.ox
    }
}

/// The vertical twin. The flip never mirrors y (the C only negates the x
/// half of the link, @61A0/@6266).
fn blit_y(f: &engine::FrameRef, a: &Actor) -> i32 {
    bl(f).1 + a.oy
}

/// ANSI `rand() % n` — every one of the module's 75 random sites (the C has
/// no RandomBelow call at all).
fn rnd(ctx: &mut Ctx, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    ctx.rng15.below(n.min(0x7fff) as u16) as u32
}

/// The universal enter-block deadline: `now + (rand() % div) * 1000 + add`
/// (divide, scale by 1000, add `g03E6`, add a constant). `div == 0` = none.
fn deadline(ctx: &mut Ctx, div: u32, add: u64) -> u64 {
    if div == 0 {
        return 0;
    }
    ctx.now_ms + (rnd(ctx, div) as u64) * 1000 + add
}

/// The three steppers the handlers call, on the port's motion model (the
/// frame's own OFst delta is spent when the frame is ENTERED, ±x by flip):
/// - `Loop`: fn16 @61D4 (critters, arg 1) / fn54 @2056 (slots, arg 1) — past
///   the last frame the run restarts at `first` and reports one cycle.
/// - `Hold`: fn15 @60F2 (critters, arg 0) — past the last frame the run
///   parks ON `last` and reports done (a one-shot that keeps its end pose).
/// - `Shot`: fn53 @1F90 (slots) — reports `last <= cur` after the advance,
///   never resets (the state transitions on the first report).
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Loop,
    Hold,
    Shot,
}

fn step(pack: &Pack, a: &mut Actor, kind: Step) -> bool {
    if a.hold_first {
        // `cur = first − 1` enters: the first advance lands on `first`
        a.hold_first = false;
        return kind == Step::Shot && a.fb_cur >= a.fb_last;
    }
    let (first, last) = (a.fb_first, a.fb_last);
    let nxt = a.fb_cur + 1;
    match kind {
        // fn16 @61D4 / fn54 @2056, arg 1 (@486A pushes 1 for
        // state 1 and every other Loop site): the wrap test is
        // `cur >= last` AFTER the advance, so **`last` is never drawn**.
        // Every run's last frame is its link marker — it repeats the first
        // frame's art carrying the loop-back offset (211/227 −112, 615/621
        // −22, 656/669 −128), or it carries the next run's authored origin
        // (525/536 → 538, 538/550 → 552, 552/561 → 563, 591/595 → 597).
        // Drawing it was symptom #1: one motionless duplicated frame per
        // cycle (1 in 7 for the red fish's run 615), and for a marker whose
        // own `dx` is 0 the fish then snapped back the marker's whole
        // offset (run 525's hover, +29 px a cycle).
        //
        // `pos` has advanced onto `last` by the time the wrap fires, so the
        // restart on `first` carries `centre(last) − centre(first)`
        // (@621E's `link(last, first)` push pair, x negated by the flip).
        Step::Loop => {
            if nxt >= last {
                if first != last {
                    if let (Some(fl), Some(ff)) =
                        (pack.frame(BASE, last), pack.frame(BASE, first))
                    {
                        let (lx, ly) = cn(fl);
                        let (fx, fy) = cn(ff);
                        a.ox += if a.flip { fx - lx } else { lx - fx };
                        a.oy += ly - fy;
                    }
                }
                a.fb_cur = first;
                a.fb_done = true;
                return true;
            }
            a.fb_cur = nxt;
            false
        }
        // fn15 @60F2, arg 0: the one-shot. `last` IS drawn, the run parks on
        // it, and the finishing call applies no delta (@618C).
        Step::Hold => {
            if nxt > last {
                a.fb_cur = last;
                a.fb_done = true;
                return true;
            }
            a.fb_cur = nxt;
            false
        }
        // fn53 @1F90: never resets; reports `last <= cur` after the advance.
        Step::Shot => {
            a.fb_cur = nxt.min(last);
            if a.fb_cur >= last {
                a.fb_done = true;
                return true;
            }
            false
        }
    }
}

/// Centre of the actor's current frame on screen.
fn centre(pack: &Pack, a: &Actor) -> (i32, i32) {
    match pack.frame(BASE, a.fb_cur) {
        Some(f) => (blit_x(f, a) + f.w / 2, blit_y(f, a) + f.h / 2),
        None => (a.ox, a.oy),
    }
}

/// Put the current frame's centre at (cx, cy).
fn set_centre(pack: &Pack, a: &mut Actor, cx: i32, cy: i32) {
    if let Some(f) = pack.frame(BASE, a.fb_cur) {
        let (bx, by) = bl(f);
        a.ox = if a.mirror_x { cx + f.w / 2 + bx } else { cx - f.w / 2 - bx };
        a.oy = cy - f.h / 2 - by;
    }
}

/// `fn19` @676E: the critter is off-screen. Direction-aware — a leftward
/// (unflipped) fish is off once its right edge passes the LEFT screen edge,
/// a flipped one once its left edge passes the RIGHT edge; both once the
/// sprite has left the screen vertically. State 0 counts as off.
fn off_screen(pack: &Pack, a: &Actor) -> bool {
    if a.state == 0 {
        return true;
    }
    if pack.frame(BASE, a.fb_cur).is_none() {
        return false;
    }
    let (cx, cy) = centre(pack, a);
    // @0x6786..0x67E2 reads +0x134/+0x136 — the BIRTH run's box — not the
    // live frame's. Fall back to the live frame only for an actor that never
    // went through `birth` (the slots, which never call fn19).
    let (bw, bh) = match a.birth_box {
        (0, 0) => pack.frame(BASE, a.fb_cur).map(|f| (f.w, f.h)).unwrap_or((0, 0)),
        b => b,
    };
    let (w2, h2) = (bw / 2, bh / 2);
    let horiz = if !a.flip { 0 < cx + w2 } else { cx - w2 < SCREEN_W };
    let vert = 0 < cy + h2 && cy - h2 < SCREEN_H;
    !(horiz && vert)
}

/// Turn the critter around. The C only toggles the facing word `+0x11c`
/// (@fn12 state 0x1d) — it never touches the placement point, and the art
/// library mirrors the compound about the drawn sprite, so the fish turns
/// WHERE IT IS. This port mirrors about `ox`, which is a bank-space anchor
/// several hundred px away from the fish, so a bare toggle threw it clean
/// across the screen (symptom #2: measured −368 px in one tick on the
/// piranha's 0x1d → 0x15 turn). Re-deriving `ox` through the frame's centre
/// is the same pixels as "pos unchanged, art mirrored".
fn set_flip(pack: &Pack, a: &mut Actor, flip: bool) {
    if a.flip == flip {
        return;
    }
    let (cx, cy) = centre(pack, a);
    a.flip = flip;
    a.mirror = flip;
    a.mirror_x = flip;
    set_centre(pack, a, cx, cy);
}

/// `fn18` @6618: another SPECIES-4 critter within 70 px on both axes of the
/// placement points (strict `< 0x46`); called from ticks 0xd and 0x10.
fn neighbor_within_70(pack: &Pack, critters: &[Actor], idx: usize) -> bool {
    let Some(me) = critters.get(idx) else {
        return false;
    };
    let (mx, my) = centre(pack, me);
    critters.iter().enumerate().any(|(j, other)| {
        if j == idx || other.variant != 4 {
            return false;
        }
        let (ox, oy) = centre(pack, other);
        (mx - ox).abs() < 70 && (my - oy).abs() < 70
    })
}

/// Slot enter helper: arm `seq`, optionally pre-apply its first frame's
/// delta (`fn59(1,1)`), optionally start one frame early (`cur = first − 1`),
/// optionally the bubbles burst (the four L134 calls in enters 10–13).
fn a_spawn(pack: &Pack, a: &mut Actor, ctx: &mut Ctx, seq: u32, pre: bool, early: bool, bubbles: bool) {
    arm_as(pack, a, seq, pre);
    a.hold_first = early;
    if bubbles {
        ctx.sounds.push(SND_BUBBLES);
    }
}

// ---------------------------------------------------------------------------
// Machine A — the tableau script (fn51 @02E0, states 0..0x28)

/// The mirrored pipe builder's per-frame x corrections (`g0000`/`g0024`/
/// `g0048`/`g005E`, read as `x += T[cur − first + 1]` in ticks 10–13 while
/// `cur − first` is below 0x12/0x12/0xb/0x25).
const NUDGE_10: [i32; 18] = [1, 0, -1, 0, 1, -1, 1, -1, 0, 0, 1, -1, 1, 0, 0, -1, 1, -1];
const NUDGE_11: [i32; 18] = [1, -1, 0, 0, 1, 0, 0, -1, 0, 0, 1, -1, 1, 0, 0, -1, 1, -1];
const NUDGE_12: [i32; 11] = [1, -1, 0, 0, 1, 0, 0, -1, 1, 0, -1];
const NUDGE_13: [i32; 37] = [
    0, 1, 0, -1, 0, 1, -1, 1, -1, 0, 0, 1, 0, 0, 0, 0, -1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    -1, 1, -1, 1, -1, 0,
];

/// Skeleton drop (enters 1 and 5, fn51 0x8001/0x8005): a random x across the
/// world, the sprite entirely above the top edge, a 12-13 minute deadline and
/// the sink target `rand % h + h/2` (`+0x4d`) — all phrased in the slot's
/// measured box `+0x12E/+0x130`, which for these two slots holds (h, w), not
/// (w, h): see `SKELETON_BOX_SWAPPED`. So the "width" the x roll uses is the
/// compound's height, and the "height" that parks it above the screen and
/// rolls the sink depth is its width.
fn skeleton_drop(pack: &Pack, a: &mut Actor, ctx: &mut Ctx) {
    let (bw, bh) = a.ref_box; // +0x12E, +0x130 as fn55 cases 1/2 left them
    let (bw, bh) = (bw.max(1), bh.max(1));
    let cx = rnd(ctx, (SCREEN_W - bw).max(1) as u32) as i32 + bw / 2;
    set_centre(pack, a, cx, -bh / 2);
    a.deadline = deadline(ctx, 60, 720_000);
    a.sink_to = rnd(ctx, bh as u32) as i32 + bh / 2;
}

/// ORIGINAL BUG kept — fn55 cases 1/2 (@0x2486..0x249A for 958, @0x24F4..
/// 0x2508 for 900) store the art library's bounds with the two differences
/// the other way round from cases 0/3 (@0x2276..0x228E, @0x2566..0x257E):
/// `+0x12E = bottom − top`, `+0x130 = right − left`. Case 3's order is the
/// right one — tick 21's exit reads `+0x130` as Larry's HEIGHT, and all
/// three goldens land him on exactly that threshold (see LARRY_SINK_STOP).
/// For the skeletons it means `+0x126 = top − w/2`: 958 (75x25) starts
/// centred at y −37, its bottom row at −24. Read the right way round, 958's
/// bottom row sits on screen row 0 — 12 lit pixels for the 12 minutes the
/// slot waits in state 5 — and no golden shows them (rows 0-2 of
/// `qemu/toxic-swamp`, `-leftcap` and `-leftcap-t12` carry no static pixel).
const SKELETON_BOX_SWAPPED: bool = true;

fn a_enter(pack: &Pack, a: &mut Actor, ctx: &mut Ctx, state: u8) {
    a.state = state;
    a.fb_done = false;
    a.deadline = 0;
    match state {
        1 => {
            a_spawn(pack, a, ctx, 900, false, false, false);
            skeleton_drop(pack, a, ctx);
        }
        3 => a_spawn(pack, a, ctx, 914, false, true, false),
        4 => a_spawn(pack, a, ctx, 924, false, false, false),
        5 => {
            a_spawn(pack, a, ctx, 958, false, false, false);
            skeleton_drop(pack, a, ctx);
        }
        7 => a_spawn(pack, a, ctx, 986, false, true, false),
        8 => a_spawn(pack, a, ctx, 998, false, true, false),
        9 => a_spawn(pack, a, ctx, 9, true, false, false),
        10 => a_spawn(pack, a, ctx, 12, true, true, true),
        11 => a_spawn(pack, a, ctx, 31, true, true, true),
        12 => a_spawn(pack, a, ctx, 51, true, true, true),
        13 => a_spawn(pack, a, ctx, 1046, true, true, true),
        // enter 0xe: run 0x60 pinned to ONE frame (`last = 0x60`); enter 0xf:
        // run 0x40; both `dl = (rand%10)*1000 + 5`
        14 => {
            a_spawn(pack, a, ctx, 96, true, false, false);
            a.fb_last = a.fb_first;
            a.deadline = deadline(ctx, 10, 5);
        }
        15 => {
            a_spawn(pack, a, ctx, 64, true, false, false);
            a.deadline = deadline(ctx, 10, 5);
        }
        16 => a_spawn(pack, a, ctx, 98, false, false, false),
        17 => a_spawn(pack, a, ctx, 100, false, false, false),
        18 => a_spawn(pack, a, ctx, 102, false, false, false),
        // enter 0x13: the eye-tree family by the slot's `+0x134` variant
        19 => {
            let seq = match a.variant {
                0 => 105,
                1 => 152,
                _ => 199,
            };
            a_spawn(pack, a, ctx, seq, true, false, false);
            a.deadline = deadline(ctx, 3, 3000);
        }
        // enter 0x14: fn58 @2F2E, the flopper picker — variant 2 always plays
        // 201; else rand%5 over the twin-tree pairs, +3 s hold on picks 1/3
        20 => {
            let r = rnd(ctx, 5) as usize; // consumed even for variant 2
            let seq = if a.variant == 2 {
                201
            } else {
                let pair = FLOPPER_STAGES[r];
                if a.variant == 0 { pair.0 } else { pair.1 }
            };
            a_spawn(pack, a, ctx, seq, true, false, false);
            if a.variant != 2 && (r == 1 || r == 3) {
                a.deadline = ctx.now_ms + 3000;
            }
        }
        21 => a_spawn(pack, a, ctx, 672, false, false, false),
        22 => a_spawn(pack, a, ctx, 700, false, false, false),
        24 => {
            a_spawn(pack, a, ctx, 710, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        // enter 0x19: fn57 @2E7C, the skeleton picker; +3 s on picks 0/1
        25 => {
            let r = rnd(ctx, 7) as usize;
            a_spawn(pack, a, ctx, SKELETON_PICKS[r], true, false, false);
            if r < 2 {
                a.deadline = ctx.now_ms + 3000;
            }
        }
        26 => a_spawn(pack, a, ctx, 766, true, false, false),
        27 => {
            a_spawn(pack, a, ctx, 776, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        28 => a_spawn(pack, a, ctx, 778, true, false, false),
        29 => a_spawn(pack, a, ctx, 785, true, false, false),
        30 => {
            a_spawn(pack, a, ctx, 795, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        31 => {
            let seq = match rnd(ctx, 3) {
                0 => 797,
                1 => 799,
                _ => 801,
            };
            a_spawn(pack, a, ctx, seq, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        32 => a_spawn(pack, a, ctx, 808, true, false, false),
        33 => {
            a_spawn(pack, a, ctx, 826, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        34 => a_spawn(pack, a, ctx, 828, true, false, false),
        35 => a_spawn(pack, a, ctx, 835, true, false, false),
        36 => {
            a_spawn(pack, a, ctx, 845, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        37 => {
            let seq = if rnd(ctx, 2) == 0 { 853 } else { 847 };
            a_spawn(pack, a, ctx, seq, true, false, false);
            a.deadline = deadline(ctx, 2, 1000);
        }
        38 => a_spawn(pack, a, ctx, 860, true, false, false),
        39 => {
            a_spawn(pack, a, ctx, 872, true, false, false);
            a.deadline = deadline(ctx, 5, 1000);
        }
        40 => a_spawn(pack, a, ctx, 874, true, false, false),
        _ => {}
    }
}

/// What a slot tick asks for.
enum ANext {
    Stay,
    State(u8),
    /// back to state 0 = `fn55` re-dispatch of the slot's role
    Reset,
}

/// Machine A tick (fn51's update branches). `stage` is `g024A`, MG_7B3, the
/// Lung-Capacity stage deadline; ticks 0x1a/0x1d/0x20/0x23 re-arm it.
fn a_tick(pack: &Pack, stage: &mut u64, caps_count: u32, a: &mut Actor, ctx: &mut Ctx) -> ANext {
    let now = ctx.now_ms;
    let dl = a.deadline != 0 && a.deadline <= now;
    let single = a.fb_first == a.fb_last;
    let nudge = |a: &mut Actor, t: &[i32], limit: i32| {
        let k = a.fb_cur as i32 - a.fb_first as i32;
        if a.mirror && k < limit {
            let i = (k + 1) as usize;
            if i < t.len() {
                a.ox += t[i];
            }
        }
    };
    match a.state {
        1 | 5 => {
            if dl {
                ANext::State(a.state + 1)
            } else {
                ANext::Stay
            }
        }
        2 | 6 => {
            let done = step(pack, a, Step::Loop);
            let (_, cy) = centre(pack, a);
            if done && a.sink_to <= cy {
                ANext::State(a.state + 1)
            } else {
                ANext::Stay
            }
        }
        3 | 7 => {
            if step(pack, a, Step::Loop) {
                ANext::State(a.state + 1)
            } else {
                ANext::Stay
            }
        }
        // ticks 4/8 (@0x0826..0x0844): off the LEFT edge once
        // `x + (+0x12E)/2 − 3 <= world.left` — `+0x12E` being the swapped
        // box's h (SKELETON_BOX_SWAPPED)
        4 | 8 => {
            step(pack, a, Step::Loop);
            let (cx, _) = centre(pack, a);
            if cx + a.ref_box.0 / 2 - 3 <= 0 {
                ANext::Reset
            } else {
                ANext::Stay
            }
        }
        9 => {
            step(pack, a, Step::Loop);
            if dl {
                if caps_count > 5 && rnd(ctx, 2) != 0 {
                    ANext::State(13)
                } else {
                    ANext::State(10)
                }
            } else {
                ANext::Stay
            }
        }
        10 => {
            nudge(a, &NUDGE_10, 0x12);
            if step(pack, a, Step::Shot) { ANext::State(11) } else { ANext::Stay }
        }
        11 => {
            nudge(a, &NUDGE_11, 0x12);
            if step(pack, a, Step::Shot) { ANext::State(12) } else { ANext::Stay }
        }
        12 | 13 => {
            if a.state == 12 {
                nudge(a, &NUDGE_12, 0xb);
            } else {
                nudge(a, &NUDGE_13, 0x25);
            }
            if step(pack, a, Step::Shot) {
                a.deadline = deadline(ctx, 20, 10_000);
                // the enter of 9 clears the deadline; carry it across
                let d = a.deadline;
                a_enter(pack, a, ctx, 9);
                a.deadline = d;
                ANext::Stay
            } else {
                ANext::Stay
            }
        }
        14 => {
            step(pack, a, Step::Loop);
            if dl { ANext::State(15) } else { ANext::Stay }
        }
        15 => {
            let done = step(pack, a, Step::Loop);
            if done && dl { ANext::State(14) } else { ANext::Stay }
        }
        16 | 17 | 18 => {
            step(pack, a, Step::Loop);
            ANext::Stay
        }
        19 => {
            let done = step(pack, a, Step::Loop);
            if done && dl { ANext::State(20) } else { ANext::Stay }
        }
        20 | 25 | 31 => {
            let back = match a.state {
                20 => 19,
                25 => 24,
                _ => 30,
            };
            let go = if single {
                step(pack, a, Step::Loop) && dl
            } else {
                step(pack, a, Step::Shot)
            };
            if go { ANext::State(back) } else { ANext::Stay }
        }
        // tick 0x15 @11DC: THE SINK — Larry falls in from above on 672 (the
        // run's own +4 ramp carries him: fn54 moves `+0x126` by the frame
        // links and nothing else), every completed cycle re-rolls the skin
        // (rand%3 == 0 → 679/690 by a coin, else 672) with a plain SetRun
        // that keeps the placement point, and the only exit is
        // `world.bottom − (+0x130)/2 − 10 <= +0x126` → 22, tested on the
        // centre of the frame just advanced onto (see LARRY_SINK_STOP).
        21 => {
            if step(pack, a, Step::Loop) {
                let seq = if rnd(ctx, 3) == 0 {
                    if rnd(ctx, 2) == 0 { 679 } else { 690 }
                } else {
                    672
                };
                arm(pack, a, seq);
            }
            let (_, cy) = centre(pack, a);
            let (_, ref_h) = a.ref_box;
            if SCREEN_H - ref_h / 2 - LARRY_SINK_STOP <= cy {
                ANext::State(22)
            } else {
                ANext::Stay
            }
        }
        22 => {
            if step(pack, a, Step::Shot) { ANext::State(24) } else { ANext::Stay }
        }
        // the five stage-expiry branches: the Loop stepper runs, the local
        // deadline gates, MG_7B3 picks the sub-cycle or the advance
        24 | 27 | 30 | 33 | 36 => {
            step(pack, a, Step::Loop);
            if dl {
                let expired = *stage <= now;
                let s = a.state;
                ANext::State(if expired { s + 2 } else { s + 1 })
            } else {
                ANext::Stay
            }
        }
        28 | 34 | 37 => {
            if step(pack, a, Step::Shot) { ANext::State(a.state - 1) } else { ANext::Stay }
        }
        26 | 29 | 32 | 35 => {
            if step(pack, a, Step::Shot) {
                *stage = deadline(ctx, 5, 3000);
                ANext::State(a.state + 1)
            } else {
                ANext::Stay
            }
        }
        38 => {
            if step(pack, a, Step::Shot) { ANext::State(39) } else { ANext::Stay }
        }
        39 => {
            step(pack, a, Step::Loop);
            if dl { ANext::State(40) } else { ANext::Stay }
        }
        40 => {
            if step(pack, a, Step::Shot) { ANext::State(39) } else { ANext::Stay }
        }
        _ => ANext::Stay,
    }
}

// ---------------------------------------------------------------------------
// Machine B — the per-fish wander script (fn12 @44E0, states 0..0x25)

/// Machine B enter: the state's run (+0x12e/+0x130/+0x132) and, where the
/// C sets one, the local deadline `+0x13a`. Enters 0xc/0x10/0x1f..0x22 spend
/// the run's first frame delta first (`fn20(1,1)`).
fn b_enter(pack: &Pack, a: &mut Actor, ctx: &mut Ctx, state: u8) {
    a.state = state;
    a.fb_done = false;
    let (seq, div, add) = B_ENTER[state as usize];
    // enter 8: the four-way nested coin (0x14d/0x143/0x139/0x12f)
    let seq = if state == 8 {
        if rnd(ctx, 2) == 0 {
            if rnd(ctx, 2) == 0 { 333 } else { 323 }
        } else if rnd(ctx, 2) == 0 {
            313
        } else {
            303
        }
    } else {
        seq
    };
    let linked = matches!(state, 0xc | 0x10 | 0x1f | 0x20 | 0x21 | 0x22);
    arm_as(pack, a, seq, linked);
    a.deadline = deadline(ctx, div, add);
}

/// `fn17` @62C2 — critter birth. Species: the sticky modifier-key latch
/// forces 4 (and is consumed); `g03DC >= 6` widens the roll to rand%8
/// (the tiny-swimmer bucket), else rand%7. The flip word `+0x11c` is
/// RECONCILED to `+0x11e`, which nothing in M129 writes (fn11 @441E, the
/// ctor's attach helper, failed to decompile — GAP): the port rolls it once
/// per critter object (`home_flip`), so a slot always re-enters from the
/// same side and a state-0x1d turn is undone at the next birth. Position:
/// `y = rand % (bottom − h) + h/2`, `x = right + w/2` (flip 0, swims left)
/// or `left − w/2` (flip 1).
fn birth(pack: &Pack, caps_latch: &mut bool, caps_count: u32, a: &mut Actor, ctx: &mut Ctx) {
    let species: i32 = if *caps_latch {
        *caps_latch = false;
        4
    } else if caps_count >= 6 {
        rnd(ctx, 8) as i32
    } else {
        rnd(ctx, 7) as i32
    };
    a.variant = species;
    let state = SPECIES_BIRTH_STATE[species.clamp(0, 7) as usize];
    a.flip = a.home_flip;
    a.mirror = a.flip;
    a.mirror_x = a.flip;
    a.deadline = 0;
    b_enter(pack, a, ctx, state);
    if let Some(f) = pack.frame(BASE, a.fb_cur) {
        let h = f.h.max(1);
        let w = f.w.max(1);
        // @0x655E: +0x134/+0x136 are taken here, ONCE, and fn19 reads them
        // for the rest of the critter's life.
        a.birth_box = (w, h);
        let cy = rnd(ctx, (SCREEN_H - h).max(1) as u32) as i32 + h / 2;
        let cx = if !a.flip { SCREEN_W + w / 2 } else { -w / 2 };
        set_centre(pack, a, cx, cy);
    }
}

/// What a critter tick asks for.
enum BNext {
    Stay,
    State(u8),
    /// `+0x138` off-screen → SetState(0) → `fn17` births again next tick
    Rebirth,
}

/// Machine B tick (fn12's update branches). The stepper and the off-screen
/// test are per state, exactly where the C calls them.
fn b_tick(pack: &Pack, critters: &[Actor], idx: usize, a: &mut Actor, ctx: &mut Ctx) -> BNext {
    let now = ctx.now_ms;
    let dl = a.deadline <= now;
    let off = |pack: &Pack, a: &Actor| off_screen(pack, a);
    match a.state {
        1 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl {
                if rnd(ctx, 2) == 0 {
                    BNext::State(3)
                } else if rnd(ctx, 2) != 0 {
                    BNext::State(8)
                } else {
                    BNext::State(2)
                }
            } else {
                BNext::Stay
            }
        }
        2 => {
            if step(pack, a, Step::Loop) {
                if rnd(ctx, 2) != 0 { BNext::State(1) } else { BNext::State(3) }
            } else {
                BNext::Stay
            }
        }
        3 => {
            if step(pack, a, Step::Hold) {
                if rnd(ctx, 2) != 0 { BNext::State(5) } else { BNext::State(4) }
            } else {
                BNext::Stay
            }
        }
        4 | 5 => {
            let done = step(pack, a, Step::Loop);
            if done && dl { BNext::State(a.state + 1) } else { BNext::Stay }
        }
        6 => {
            if step(pack, a, Step::Loop) { BNext::State(7) } else { BNext::Stay }
        }
        7 | 9 | 10 => {
            step(pack, a, Step::Loop);
            if off(pack, a) { BNext::Rebirth } else { BNext::Stay }
        }
        8 => {
            if step(pack, a, Step::Loop) {
                return BNext::State(1);
            }
            if off(pack, a) { BNext::Rebirth } else { BNext::Stay }
        }
        0xb => {
            if step(pack, a, Step::Hold) { BNext::State(0xc) } else { BNext::Stay }
        }
        0xc => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl { BNext::State(0xd) } else { BNext::Stay }
        }
        0xd => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl {
                if rnd(ctx, 2) != 0 { BNext::State(0xe) } else { BNext::State(0xb) }
            } else if neighbor_within_70(pack, critters, idx) {
                BNext::State(0xe)
            } else {
                BNext::Stay
            }
        }
        0xe => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Hold) { BNext::State(0xf) } else { BNext::Stay }
        }
        0xf => {
            if step(pack, a, Step::Hold) { BNext::State(0x10) } else { BNext::Stay }
        }
        0x10 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl && !neighbor_within_70(pack, critters, idx) {
                BNext::State(0x11)
            } else {
                BNext::Stay
            }
        }
        0x11 => {
            if step(pack, a, Step::Hold) { BNext::State(0xb) } else { BNext::Stay }
        }
        // 0x12: the white skeleton's terminal — re-arms 481/465 by a coin,
        // leaves only by swimming off
        0x12 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Loop) {
                let seq = if rnd(ctx, 2) == 0 { 481 } else { 465 };
                arm(pack, a, seq);
            }
            BNext::Stay
        }
        0x13 | 0x14 | 0x24 | 0x25 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl {
                let s = a.state;
                BNext::State(match s {
                    0x13 => 0x14,
                    0x14 => 0x13,
                    0x24 => 0x25,
                    _ => 0x24,
                })
            } else {
                BNext::Stay
            }
        }
        0x15 | 0x16 => {
            if step(pack, a, Step::Loop) { BNext::State(0x17) } else { BNext::Stay }
        }
        0x17 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl {
                if rnd(ctx, 3) == 0 { BNext::State(0x1d) } else { BNext::State(0x18) }
            } else {
                BNext::Stay
            }
        }
        0x18 => {
            if step(pack, a, Step::Loop) { BNext::State(0x19) } else { BNext::Stay }
        }
        0x19 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Loop) {
                if rnd(ctx, 2) != 0 { BNext::State(0x1a) } else { BNext::State(0x16) }
            } else {
                BNext::Stay
            }
        }
        0x1a => {
            if step(pack, a, Step::Loop) { BNext::State(0x1b) } else { BNext::Stay }
        }
        0x1b => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Loop) { BNext::State(0x1c) } else { BNext::Stay }
        }
        0x1c => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Loop) {
                if rnd(ctx, 2) != 0 { BNext::State(0x1d) } else { BNext::State(0x16) }
            } else {
                BNext::Stay
            }
        }
        // 0x1d: the turn — toggles the flip word and the mirror bit, back
        // to 0x15
        0x1d => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            if step(pack, a, Step::Loop) {
                set_flip(pack, a, !a.flip);
                BNext::State(0x15)
            } else {
                BNext::Stay
            }
        }
        0x1e => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            let done = step(pack, a, Step::Loop);
            if done && dl { BNext::State(0x1f) } else { BNext::Stay }
        }
        0x1f | 0x20 | 0x21 => {
            let done = step(pack, a, Step::Loop);
            if done && dl { BNext::State(a.state + 1) } else { BNext::Stay }
        }
        // 0x22: a ±2 px x nudge against the flip, back to 0x1e
        0x22 => {
            let done = step(pack, a, Step::Loop);
            if done && dl {
                a.ox += if !a.flip { -2 } else { 2 };
                BNext::State(0x1e)
            } else {
                BNext::Stay
            }
        }
        0x23 => {
            if off(pack, a) {
                return BNext::Rebirth;
            }
            step(pack, a, Step::Loop);
            BNext::Stay
        }
        _ => BNext::Stay,
    }
}

// ---------------------------------------------------------------------------
// Module

impl ToxicSwamp {
    /// §4.3 / MF_7C6 @0x3894: count = max(raw*15/100, 1), range 1..=15.
    fn critter_count(&self) -> usize {
        (self.critters_raw * 15 / 100).clamp(1, 15) as usize
    }

    /// §1 Lung Capacity band = raw/20 (0-based), stacks onto MG_7B3.
    fn lung_band(&self) -> u64 {
        LUNG_BANDS[(self.lung_raw / 20).clamp(0, 4) as usize]
    }

    // -----------------------------------------------------------------------
    // Initialization (MF_7C6 §4)

    fn init(&mut self, ctx: &mut Ctx) {
        self.floor.clear();
        self.slots.clear();
        // §4 @0x3B3A: the `+0x7A` flag test branching to 0x3BA6 — with Fish Only
        // checked the module jumps clean over MF_7A0 *and* the 9-slot
        // creation loop, so there is no stage timer, no reef floor and no
        // dressing at all. (Spec §7.3/§6 read this as a per-frame freeze of
        // the slot ticks; the init skip is the stronger, earlier gate.)
        if !self.fish_only {
            // §4.9 / MF_7A0: the stage timer is LATCHED to the clock and then
            // the Lung Capacity band is added — the clock read @0x0106
            // (the same ms-clock read `DoDrawFrame` uses for MG_7D3) with
            // @0x010A storing it into MG_7B3, and the
            // five `+= {30 s, 2, 5, 10, 20 min}` branches right after.
            //
            // It is NOT an accumulator. The prose era read the five `+=`
            // sites without the assignment above them and the port carried
            // that over as `stage_deadline += band`, which is harmless on the
            // first init and fatal afterwards: DoDrawFrame @0x3F9C restarts
            // the module on ANY control change, and the saver's sliders are
            // continuous, so one drag of either slider re-inits the module
            // dozens of times and stacked 30 s onto Larry's first expiry each
            // time. That is "I set Lungs to Thimble and he never died":
            // setting the control is what pushed his death past the horizon.
            self.stage_deadline = ctx.now_ms + self.lung_band();

            // MF_7A0 @0x00AA: MG_7B4, the reef anchor, is built from
            // compound 1's bounds and the world rect's bottom-left corner.
            self.reef_dy = self.reef_anchor();
            // The first slot's RESET tick paints the floor once (@0x0612
            // MG_7B2 gate -> MF_7A8), before MF_7A7 places that slot.
            self.paint_floor(ctx);

            // §4.10 + MF_7A7: 9 background slots; each slot's RESET
            // world-logic case (its index, +0x132) spawns its piece and
            // dispatches its entry state (@0x242C/0x249E/0x250C/0x26A8/
            // 0x27B8/0x2944/0x29E6/0x2A8C/0x2B32). Role 0 is the eye-tree,
            // role 3 is Larry, role 4 builds the pipe, roles 5-8 are the
            // static reef dressing.
            let mut slots = Vec::with_capacity(9);
            for (i, &entry) in ROLE_ENTRY.iter().enumerate() {
                let mut a = Actor::new(Kind::Slot);
                a.role = i as i32; // +0x132 role selector
                self.slot_entry(&mut a, ctx, entry);
                slots.push(a);
            }
            self.slots = slots;
        }

        // §4.11: `count` critters, each birthed (MF_7E3).
        let count = self.critter_count();
        let mut critters = Vec::with_capacity(count);
        for _ in 0..count {
            let mut a = Actor::new(Kind::Critter);
            a.home_flip = rnd(ctx, 2) == 0; // fn11 @441E (GAP), once per critter
            birth(&self.pack, &mut self.caps_latch, self.caps_count, &mut a, ctx);
            critters.push(a);
        }
        self.critters = critters;

        self.latch = (self.critters_raw, self.lung_raw, self.fish_only as i32);
    }

    // -----------------------------------------------------------------------
    // The reef (MF_7A0's anchor, MF_7A8's floor, MF_7A7's dressing)

    /// Pixel size of one compound, cached (`Pack::image` decodes from disk).
    fn dim(&mut self, seq: u32) -> (i32, i32) {
        if let Some(d) = self.dims.get(&seq) {
            return *d;
        }
        let png = self.pack.frame(BASE, seq).map(|f| f.png.clone());
        let d = png
            .map(|p| {
                let im = self.pack.image(&p);
                (im.w as i32, im.h as i32)
            })
            .unwrap_or((0, 0));
        self.dims.insert(seq, d);
        d
    }

    /// MF_7A0 @0x00AA..0x00FC: MG_7B4 = (world.left + w(1)/2,
    /// world.bottom − h(1)/2) — compound 1 parked in the world's bottom-left
    /// corner. Reduced to the layer offset that MG_7B4 plus MF_7AB's
    /// frame-link deltas produce for every reef piece: `by + (SCREEN_H −
    /// bottom(1))`, which is `+57` on a 640x480 screen.
    fn reef_anchor(&mut self) -> i32 {
        let (_, h) = self.dim(FLOOR_REF);
        let by = self.pack.frame(BASE, FLOOR_REF).map(|f| f.by).unwrap_or(0);
        SCREEN_H - (by + h)
    }

    /// Append one painted floor tile, mirrored when MF_7A8 has the art
    /// library's flip toggle on.
    fn push_tile(&mut self, seq: u32, x: i32, y: i32, mirror: bool) {
        let Some(f) = self.pack.frame(BASE, seq) else { return };
        self.floor.push(FloorTile { png: f.png.clone(), flip: mirror, x, y });
    }

    /// MF_7A8 @0x2B5E — paint the reef floor. See the module header for the
    /// address-by-address reading; the shape is: drop the pipe cap
    /// (compound 7) against one end of the world rect, picked by `rand()%2`
    /// and mirrored when that end is the left, then walk tiles away from it
    /// until one crosses the far edge. Every tile is bottom-aligned to the
    /// world rect, so the whole row's bottom is the screen bottom.
    fn paint_floor(&mut self, ctx: &mut Ctx) {
        let (w7, h7) = self.dim(FLOOR_END);
        if w7 <= 0 {
            return;
        }
        // @0x2B6A: rand()%2 — non-zero anchors the cap at the right edge.
        let right_end = rnd(ctx, 2) != 0;
        self.floor_pipe_left = !right_end;
        let (mut left, mut right) = if right_end {
            // @0x2B94: rect = {W − w7, H − h7, W, H}, drawn unmirrored.
            let l = SCREEN_W - w7;
            self.push_tile(FLOOR_END, l, SCREEN_H - h7, false);
            (l, SCREEN_W)
        } else {
            // @0x2C20: flip on, rect = {0, H − h7, w7 − 1, H}. That `− 1`
            // (the minus-one @0x2C4E) is what puts the golden reel's tile
            // grid on 95/191/287/383 instead of 96/192/288/384.
            // GAP(L135 mirrored blit into a rect 1 px narrower than the
            // art): the goldens put the flipped cap at −1..95 (NCC 0.94 at
            // −1 vs 0.70 at 0, `qemu/toxic-swamp-leftcap`) — flush with the
            // rect's right edge, the one the tile grid walks on from.
            self.push_tile(FLOOR_END, w7 - 1 - w7, SCREEN_H - h7, true);
            (0, w7 - 1)
        };
        // MG_7B0 = that rect (@0x2CBA) — the pipe's keep-out box, and the
        // flag MF_7A7 case 4 reads to pick its end.
        let mut used_junk = false; // D6, the one-shot toilet latch
        loop {
            // @0x2CE6/@0x2CF2: the test runs on the tile already painted, so
            // the first overhanging tile still gets drawn (and clipped).
            if left < 0 || right > SCREEN_W {
                break;
            }
            let mut seq = FLOOR_SAND[(rnd(ctx, 2) != 0) as usize]; // @0x2CFE
            // @0x2D16: both rolls are consumed every pass, in this order.
            if rnd(ctx, 3) == 0 && !used_junk {
                used_junk = true;
                seq = FLOOR_JUNK;
            }
            let (w, h) = self.dim(seq);
            if w <= 0 {
                break;
            }
            if right_end {
                right = left;
                left -= w;
            } else {
                left = right;
                right += w;
            }
            self.push_tile(seq, left, SCREEN_H - h, false);
        }
    }

    /// Place a reef piece the way MF_7A7 cases 5-8 do: keep the reef anchor's
    /// vertical (`by + reef_dy`, plus the case-5 nudge) and re-roll x as
    /// `rand % (world.right − w + w/2)`, used as the sprite's CENTRE.
    fn reef_place(&mut self, a: &mut Actor, ctx: &mut Ctx, extra_y: i32) {
        let seq = a.fb_first;
        let (w, _) = self.dim(seq);
        let span = (SCREEN_W - w + w / 2).max(1) as u32;
        let xc = rnd(ctx, span) as i32;
        let bx = self.pack.frame(BASE, seq).map(|f| bl(f).0).unwrap_or(0);
        a.ox = xc - w / 2 - bx;
        a.oy = self.reef_dy + extra_y;
    }

    /// Slot priming: MF_7A7 case N (spawn piece + dispatch entry state).
    /// Case 0 spawns the eye-tree family with rand%3 into +0x134 (@0x21A0)
    /// before dispatching 19; cases 4-8 arm their reef piece and place it.
    fn slot_entry(&mut self, a: &mut Actor, ctx: &mut Ctx, entry: u8) {
        match a.role {
            // Case 0 @0x21A0: the eye-tree. It grows out of the sand, so it
            // takes the reef anchor and a rolled x like the rest of the
            // dressing (golden reel t=150: compound 199 at top 403 = by+57).
            0 => {
                a.variant = (rnd(ctx, 3)) as i32; // eye-tree variant +0x134
                a_enter(&self.pack, a, ctx, entry);
                self.reef_place(a, ctx, 0);
            }
            // Case 4 @0x26BE: the pipe/coral builder. It arms compound 9 but
            // measures compound 7 — the floor's cap — and branches on
            // MG_7B0.left (@0x2724), so it always builds from the end MF_7A8
            // capped, mirrored when that is the left (+0x136 = 1 @0x2742,
            // and the x centre gets the minus-two nudge @0x2774).
            //
            // The placement point is compound 7's centre on the capped end —
            // `x = right − w7/2` (or `left + w7/2`, flipped), `y = bottom −
            // h7/2 − 1` — and then `fn59(1, 7)` @306C links it FROM 7 TO 9:
            // `pos += centre(9) − centre(7)`, dx negated while flipped, which
            // is (+10, −3) in this pack; the left end then takes `x −= 2`.
            // So 9 lands at 565..640 on the right and −1..74 on the left,
            // both top 363 — exactly where `qemu/toxic-swamp` (right) and
            // `-leftcap` / `-leftcap-t12` (left) fit it (NCC 0.97). The port
            // centred 9 on the cap and dropped the link: 10 px inboard,
            // with the pipe's flat end drawn over the cap's round mouth.
            4 => {
                a_enter(&self.pack, a, ctx, entry);
                let (w7, h7) = self.dim(FLOOR_END);
                let left = self.floor_pipe_left;
                let (lx, ly) = match (
                    self.pack.frame(BASE, FLOOR_END),
                    self.pack.frame(BASE, a.fb_cur),
                ) {
                    (Some(f7), Some(f9)) => (cn(f9).0 - cn(f7).0, cn(f9).1 - cn(f7).1),
                    _ => (0, 0),
                };
                let cx = if left { w7 / 2 - lx - 2 } else { SCREEN_W - w7 / 2 + lx };
                let cy = SCREEN_H - h7 / 2 - 1 + ly;
                a.flip = left;
                a.mirror = left;
                set_centre(&self.pack, a, cx, cy);
                a.deadline = ctx.now_ms + 5000; // fn55 case 4: `+0x138 = now + 5000`
            }
            // Cases 1/2 @0x2446/@0x24B4: arm 958 / 900 and measure it into
            // +0x12E/+0x130 — with the two differences swapped
            // (SKELETON_BOX_SWAPPED). No placement here: enters 5/1 do it.
            1 | 2 => {
                let seq = if a.role == 1 { 958 } else { 900 };
                let (w, h) = self.dim(seq);
                a.ref_box = if SKELETON_BOX_SWAPPED { (h, w) } else { (w, h) };
                a_enter(&self.pack, a, ctx, entry);
            }
            // Case 3 @0x2522: Larry the Lawyer — the tied-up mobster. He is
            // NOT reef: @0x2550 measures compound 672's bounds into
            // +0x12E/+0x130, @0x25A4 rolls `x = w/2 + rand % (right − w)` as
            // the centre, and @0x2690 sets `+0x126 = world.top − h/2` — he
            // starts ENTIRELY ABOVE the screen and state 21 sinks him onto
            // the sand. (The rect-rejection re-roll @0x2582..0x268C is
            // approximated the same way cases 0/5-8 are.) From here on he is
            // an ordinary placement point: the steppers' links and the
            // enters' SetRuns move him, exactly as they move every slot.
            3 => {
                a_enter(&self.pack, a, ctx, entry); // arms 672
                let (w, h) = self.dim(a.fb_first);
                // +0x12E/+0x130, measured once and never re-measured.
                a.ref_box = (w, h);
                let span = (SCREEN_W - w).max(1) as u32;
                let cx = w / 2 + rnd(ctx, span) as i32;
                // GAP(1 px, see LARRY_SPAWN_BOTTOM): the centre that puts
                // 672's drawn bottom on the captures' grid.
                set_centre(&self.pack, a, cx, LARRY_SPAWN_BOTTOM - h + h / 2);
            }
            // Cases 5-8 @0x27CE/0x295A/0x2A00/0x2AA6: the static dressing.
            5..=8 => {
                let idx = (a.role - 5) as usize;
                arm(&self.pack, a, DRESSING[idx]);
                a_enter(&self.pack, a, ctx, entry);
                let nudge = if a.role == 5 { DRESSING_Y_NUDGE } else { 0 };
                self.reef_place(a, ctx, nudge);
            }
            _ => {
                a_enter(&self.pack, a, ctx, entry);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Caps-Lock watch (DoDrawFrame @0x4242)

    fn caps_watch(&mut self, ctx: &mut Ctx) {
        if !ctx.caps_lock {
            // ORIGINAL BUG kept: MG_7D0 is only touched inside the caps
            // branch — releasing the key freezes the count indefinitely.
            return;
        }
        self.caps_latch = true; // MG_7E9 sticky
        if self.caps_count == 0 {
            self.caps_window = ctx.now_ms + 5_000; // module+0x72, @0x425A
        }
        if ctx.now_ms >= self.caps_window {
            self.caps_count = 0;
        } else {
            self.caps_count += 1;
        }
    }
}

impl Module for ToxicSwamp {
    fn name(&self) -> &'static str {
        "Toxic Swamp"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Critters".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            ControlDef {
                name: "Lung Capacity".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            ControlDef {
                name: "Fish Only".into(),
                kind: ControlKind::Checkbox,
                default: 0,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.critters_raw = value.clamp(0, 100),
            1 => self.lung_raw = value.clamp(0, 100),
            2 => self.fish_only = value != 0,
            _ => {}
        }
        self.changed = true;
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        // DoDrawFrame step 1: re-read controls; any change -> restart
        // (@0x3F9C; Button() gate not reproducible — see APPROXIMATIONS).
        if !self.started || self.changed {
            self.started = true;
            self.changed = false;
            self.init(ctx);
            return;
        }

        // Slots (machine A) — frozen entirely while Fish Only is set
        // (@0x3FE2/@0x40E8: both slot sm passes skipped): bare floor +
        // swimming fish only.
        if !self.fish_only {
            for i in 0..self.slots.len() {
                if ctx.now_ms < self.slots[i].next_tick {
                    continue;
                }
                self.slots[i].next_tick = ctx.now_ms + SM_FLOOR_MS;
                let mut a = self.slots[i];
                match a_tick(&self.pack, &mut self.stage_deadline, self.caps_count, &mut a, ctx) {
                    ANext::State(s) => a_enter(&self.pack, &mut a, ctx, s),
                    ANext::Reset => {
                        let entry = ROLE_ENTRY[a.role.clamp(0, 8) as usize];
                        self.slot_entry(&mut a, ctx, entry);
                    }
                    ANext::Stay => {}
                }
                self.slots[i] = a;
            }
        }

        // Critters (machine B) run regardless of Fish Only.
        let count = self.critter_count();
        while self.critters.len() < count {
            let mut a = Actor::new(Kind::Critter);
            a.home_flip = rnd(ctx, 2) == 0;
            birth(&self.pack, &mut self.caps_latch, self.caps_count, &mut a, ctx);
            self.critters.push(a);
        }
        self.critters.truncate(count);

        for i in 0..self.critters.len() {
            if ctx.now_ms < self.critters[i].next_tick {
                continue;
            }
            self.critters[i].next_tick = ctx.now_ms + SM_FLOOR_MS;
            let mut a = self.critters[i];
            match b_tick(&self.pack, &self.critters, i, &mut a, ctx) {
                BNext::State(s) => b_enter(&self.pack, &mut a, ctx, s),
                // `+0x138` off-screen → SetState(0) → fn17 births again
                BNext::Rebirth => {
                    birth(&self.pack, &mut self.caps_latch, self.caps_count, &mut a, ctx);
                }
                BNext::Stay => {}
            }
            self.critters[i] = a;
        }

        // Caps-Lock watch runs after the machines (disasm order @0x4242).
        self.caps_watch(ctx);
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // The reef floor first: MF_7A8 blits its tile row into the canvas
        // background (@0x2E44) before a single sprite is composited, so
        // everything else — dressing, Larry, fish — draws over it. The
        // golden reel confirms the sprite order too (t=160: a skeletal fish
        // passes IN FRONT of the anemone's tendrils).
        for t in &self.floor {
            out.push(SpriteDraw {
                flip: t.flip,
                pal: CLUT_TOXY,
                png: t.png.clone(),
                x: t.x,
                y: t.y,
            });
        }
        for a in self.slots.iter().chain(self.critters.iter()) {
            let Some(f) = self.pack.frame(BASE, a.fb_cur) else {
                continue;
            };
            out.push(SpriteDraw {
                // The art library mirrors the compound at BLIT time (the
                // vtbl[0x0C] toggle the facing coin sets); `compose` does the
                // same with this flag. `blit_x` already places the mirrored
                // box, so the two must never disagree — see the header's
                // "backwards fish" note.
                flip: a.mirror,
                pal: CLUT_TOXY, // §4 step 7 / §6 step 6: LoadCLUT(clut 1200) every frame
                png: f.png.clone(),
                x: blit_x(f, a),
                y: blit_y(f, a),
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The Mac tick grid. `+0x120 = clock + 100` is a *floor* on the
    /// `TickCount()*16.625` clock, not a 100 ms metronome, so the shell
    /// ticks finer and `SM_FLOOR_MS` quantizes to 6 ticks = 102 ms.
    /// `toxic-swamp.mp4` measures 107.5 ms and rules a flat 100 out
    /// (R = 0.371 vs 0.045). See the header ERRATA.
    /// `+0x120 = clock + 100` is a deadline against `TickCount()*16.625`,
    /// so the shell rides that grid and `SM_FLOOR_MS` quantizes itself —
    /// 6 ticks six times in ten and 7 the rest, averaging 106.4 ms, which is
    /// toxic-swamp.mp4's measured 107.5. Do not set this back to a plain
    /// `Millis(SM_FLOOR_MS)`.
    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }
}

// ---------------------------------------------------------------------------
// ## APPROXIMATIONS
//
// - **Runtime palette depth branch (clut 1216)** — §4 step 7 picks clut 1216
//   ("Toxic 16 Palette") on a 4-bit screen; the engine sim has no depth
//   concept (always the full-color/depth-8 analog), so the depth-4 branch is
//   unreachable and every frame draws through clut 1200. The remap itself is
//   faithful: clut 1200 is the pack's `base_clut`, so LoadCLUT(1200) is the
//   identity — the art already wears toxy pal. (Before the slot-index fix
//   this path zipped clut 1200 positionally against ONE bank's CTAB and
//   washed the whole swamp neon blue/purple.)
// - **Module restart on control change (F_438 @0x3F9C)** — approximated as
//   an in-place re-init; the mouse Button() gate and the engine "restart
//   me" message do not exist here. MG_7B3 is deliberately NOT reset, so
//   the Lung-Capacity stacking quirk (#2) still accumulates.
// - **Machine A spawn positions** — the enters compute real world-rect
//   placements in the binary (x = w/2 + rand, y from +0x134 + rand, via
//   MG_7D4 scratch); approximated with small per-spawn jitter over the
//   authored compound placements (bx/by are the original stage positions:
//   terrain across the bottom, Larry centered at bx≈287).
// - **MF_7A7 world-logic internals** — cases 0 and 4-8 are now transcribed
//   (reef anchor + rolled x + entry state); what is still approximated is
//   the *rejection loop* those cases wrap the x roll in (@0x22C2..0x2428,
//   @0x285E..0x2936: build the trial rect, reject and re-roll while it
//   overlaps MG_7B0 — the pipe cap's box — or MG_7B1 — the toilet tile's).
//   Here the first roll is taken, so a dressing piece can land on top of the
//   pipe or the toilet where the original would have shuffled it clear.
//   Cases 1-3 (the two skeleton actors and Larry) are transcribed too
//   (2026-09-29): enters 1/5 and case 3 place them; only the same rejection
//   re-roll is skipped.
// - **Reef piece x drift** — MF_7A7's x is the sprite's CENTRE and the
//   original re-anchors every re-arm through MF_7AB's frame-link delta
//   (@0x306C), so a piece whose growth stages change width stays pinned to
//   the artwork's own join. Here the initial centre is placed exactly and
//   later re-arms absorb the placement jump instead, which drifts the
//   growing pipe a few px from the reel (measured: compound 31 left edge
//   −1 in the reel vs −5 here). Vertically there is no drift: the anchor
//   reduces to an exact per-frame `by + 57`.
// - **Compound mirroring** — MF_7A8's art-library mirror toggle
//   (@0x2C10/@0x2CAA), the pipe slot's +0x136 and every critter's +0x11c all
//   ride `SpriteDraw.flip`, which `compose` mirrors at blit time exactly as
//   the art library does. Nothing is generated. (Until 2026-09-19 this
//   encoded an `m_NNN.png` into the pack instead and fell back to the
//   UNMIRRORED frame whenever that write failed — see the fix note at the
//   top of the file.)
// - **The floor is a sprite list, not a background blit** — MF_7A8 composites
//   its tiles into the canvas *once* (@0x2E44) and the sprite engine then
//   draws over that. Here the tiles are re-emitted every frame as the first
//   entries of the sprite list, which paints identically (they never move —
//   the reel's floor is pixel-identical at t=146 and t=160) at the cost of
//   ~8 extra blits per frame.
// - **Stepper A vs B (MF_7A5/MF_7A6)** — both collapse to "advance the run,
//   apply per-frame deltas, signal done on wrap"; A5's wrap-in-place and
//   A6's reset-to-first differ only in what the following enter re-arms
//   anyway.
// - **Link-delta motion** — within-run movement comes from the authored
//   per-frame placements; the packed (dx,dy) link offsets are applied
//   cumulatively (±x by flip) as the run advances, and re-arms absorb the
//   placement jump so re-armed variants stay put (the MF_7AB/MF_7E6
//   link-delta behavior).
// - **MF_7E1 (vertical) vs MF_7E2 (horizontal) steppers** — both collapse
//   to "advance the run, apply per-frame deltas" here; the vertical/
//   horizontal axis split is not visible in the authored placement data.
// - **DoDrawFrame's second SM pass (§6 step 5)** — the 2× catch-up is a
//   no-op against the +0x120 = clock + 100 gate; the single 100 ms floor
//   here is behaviorally equivalent.
// - **Off-screen test (MF_7E5)** — now the real sprite rect (the packed
//   `w`/`h` make it free), because edge birth needs it: an anchor-point test
//   with slop re-births a fish the tick it is born. Packs without `w`/`h`
//   still fall back to the old point test.
// - **Terminal 0x0601 recycle** — machine A never reaches the 0x0FFF state
//   in the annotated script (tick 40 loops back to 39 @0x1EC8), so the
//   "recycle" reply never fires from the tableau; the shell keeps the
//   module running until closed. Machine B's tick-37 terminal bits
//   (mv … 0 @0x6130) are likewise approximated by the 36<->37 wander.
// - **Spec §8.1 vs binary** — the doc's `max(species,1)` re-roll and its
//   "Fish Only widens the roll to 8" reading do not match the disasm
//   (@0x6310 handles species 0 directly -> state 1; the %8 gate is the
//   Caps-Lock counter MG_7D0 >= 6). The binary is followed; §11 had
//   already flagged the Fish-Only reading as low-confidence.
// - **Spec §7.2 table vs statesA.txt** — the doc's sequence column drifts
//   from the annotated listing through the Larry section (doc: 781/793/
//   803/816/820/826/843|853/860/872 at states 24-40; binary: 710/795/826/
//   828/835/845/860/872/874 at 24-40) and misses the MF_7A7 role dispatch
//   entirely. statesA.txt (raw listing addresses) is followed throughout;
//   the doc's bubbles-on-24/40 and the linear 9->19 progression are not
//   in the binary and are not reproduced.
// - **Critter movement axis** — fish enter from a side edge and drift by
//   the authored `bx` column plus the wrap frame's link delta; the original
//   clamps to a world rect ≥400×300 derived from the screen — same effect
//   at 640×480.
// - **GetBounds granularity at birth** — MF_7E3 @0x6554 asks the art library
//   for the bounds of `+0x12e`, the armed run's FIRST frame; the port uses
//   that frame's packed `w`/`h`, which is the same rect up to the pack's own
//   rounding. Those numbers are latched into `Actor::birth_box` and are what
//   fn19 @676E tests, as in the C.
// - **Mirrored critters** — a flipped critter sets `SpriteDraw.flip` and
//   `blit_x` reflects the placement arithmetic about the placement point. A
//   flip that changes mid-life (machine-B ticks 29 and 34) re-arms through
//   `arm` with the NEW flip, so the sprite snaps about its placement point
//   rather than staying pixel-continuous across the turn.
// - **Machine-A slots 1 and 2** — see the ERRATA: MF_7A7 cases 1-2 arm
//   958/900 and measure their bounds but write no placement, so these two
//   keep the jitter-over-authored-placement approximation and float in open
//   water, which the 2026-09-12 capture never shows.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};

    fn mk_ctx(now: u64) -> Ctx {
        Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: now,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    /// Tick-quantization sweep, 2026-09-12. `+0x120 = clock + 100`
    /// (@0x1F62/@0x6086) is a deadline on the After Dark ms clock
    /// `TickCount()*16.625`, so an actor steps on the first Mac tick at or
    /// past +100 ms. `toxic-swamp.mp4` rules a flat 100 ms out: the Rayleigh
    /// periodogram of its screen-change events over t = 20-45 s peaks at
    /// 107.52 ms with R = 0.371, against R = 0.045 at 100 ms exactly.
    /// The shell therefore ticks on the Mac grid and `SM_FLOOR_MS` is left
    /// alone; anything that puts `tick_ms` back to 100 has to fail here.
    #[test]
    fn actor_floor_lands_on_the_mac_tick_grid() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut m = ToxicSwamp::new(pack);
        assert_eq!(m.clock(), TickClock::MacTick, "+0x120 is an f4724 deadline");
        let clock = m.clock();
        let pace = Pacer::new(&m);
        // The truncated Mac clock makes `+100` land 6 ticks later six times
        // in ten and 7 the rest — mean 106.40 ms, which is the capture's
        // 107.5. A flat 100 ms shell is what the capture ruled out.
        let step = pace.mean_period_ms(SM_FLOOR_MS, false, 100);
        assert!(
            (step - 106.40).abs() < 0.5,
            "actor step averages {step} ms; the grid says 106.40 and \
             toxic-swamp.mp4 measures 107.5"
        );

        // and a critter really only commits on that grid
        let mut fired: Vec<u64> = Vec::new();
        let mut due = 0u64;
        for i in 0..120u64 {
            let mut ctx = mk_ctx(clock.now_ms(i));
            m.tick(&mut ctx);
            let Some(c) = m.critters.first() else { continue };
            if c.next_tick != due {
                due = c.next_tick;
                fired.push(ctx.now_ms);
            }
        }
        assert!(fired.len() > 10, "no critter ever stepped");
        let gaps: Vec<u64> = fired[1..].windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            gaps.iter().any(|&g| g <= 100) && gaps.iter().any(|&g| g >= 116),
            "the step must alternate 6- and 7-tick frames: {gaps:?}"
        );
        for &g in &gaps {
            assert!(
                (99..=100).contains(&g) || (116..=117).contains(&g),
                "critter stepped {g} ms apart; the grid allows only 99/100 or 116/117"
            );
        }
    }

    #[test]
    fn toxic_swamp_ticks_500_without_panicking_and_draws() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let Some(mut m) = make(pack) else {
            panic!("make() returned None for a packed series");
        };
        let mut now: u64 = 0;
        let mut out: Vec<SpriteDraw> = Vec::new();
        let mut drew = false;
        let clock = m.clock();
        let mut k = 0u64;
        for _ in 0..500 {
            let mut ctx = mk_ctx(now);
            k += 1;
            now = clock.now_ms(k);
            m.tick(&mut ctx);
            out.clear();
            m.sprites(&mut out);
            if !out.is_empty() {
                drew = true;
            }
        }
        assert!(
            drew,
            "module produced no sprites over 500 ticks (50 s simulated)"
        );
    }

    #[test]
    fn toxic_swamp_fish_only_drops_the_reef_and_keeps_critters() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut m = ToxicSwamp::new(pack);
        m.set_control(2, 1); // Fish Only on
        let mut now: u64 = 0;
        let mut out: Vec<SpriteDraw> = Vec::new();
        let mut drew = false;
        let clock = m.clock();
        let mut k = 0u64;
        for _ in 0..300 {
            let mut ctx = mk_ctx(now);
            k += 1;
            now = clock.now_ms(k);
            m.tick(&mut ctx);
            out.clear();
            m.sprites(&mut out);
            if !out.is_empty() {
                drew = true;
            }
        }
        assert!(drew, "Fish Only run drew nothing (fish expected)");
        // @0x3B3A: the Fish Only branch jumps over MF_7A0 *and* the 9-slot
        // creation loop, so nothing ever calls MF_7A8 — bare water.
        assert!(m.floor.is_empty(), "Fish Only must not paint a reef floor");
        assert!(m.slots.is_empty(), "Fish Only must not build the tableau slots");
        assert!(!m.critters.is_empty(), "Fish Only must still swim critters");
    }

    /// Run one tick so init() has painted the floor and placed the slots.
    fn booted() -> Option<ToxicSwamp> {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return None;
        };
        let mut m = ToxicSwamp::new(pack);
        m.tick(&mut mk_ctx(0));
        Some(m)
    }

    fn frame_size(pack: &Pack, seq: u32) -> (i32, i32) {
        let f = pack.frame(BASE, seq).expect("packed compound");
        let im = pack.image(&f.png);
        (im.w as i32, im.h as i32)
    }

    #[test]
    fn toxic_swamp_reef_floor_tiles_the_full_width_on_the_screen_bottom() {
        let Some(m) = booted() else { return };
        assert!(
            m.floor.len() >= 7,
            "MF_7A8 lays a pipe cap plus a tile per 96 px of world; got {}",
            m.floor.len()
        );
        // Every tile is bottom-aligned to the world rect (@0x2D62/@0x2DB6:
        // `top = MG_7D4.bottom − h`), which on a 640x480 screen is the
        // screen bottom. The golden reel's sand strips top out at 446 =
        // 480 − 34, and the toilet tile at 413 = 480 − 67.
        let mut spans: Vec<(i32, i32)> = Vec::new();
        for t in &m.floor {
            let f = m
                .pack
                .series(BASE)
                .iter()
                .flat_map(|s| s.frames.iter())
                .find(|f| f.png == t.png)
                .expect("tile png belongs to the pack");
            let im = m.pack.image(&f.png);
            assert_eq!(
                t.y + im.h as i32,
                SCREEN_H,
                "floor tile {} is not bottom-anchored (top {}, h {})",
                t.png,
                t.y,
                im.h
            );
            spans.push((t.x, t.x + im.w as i32));
        }
        // ...and the row is gapless from at-or-left-of 0 to past the right
        // edge (the loop head tests the tile already drawn, so the first
        // overhanging tile is still painted — the reel's row runs past 640).
        spans.sort();
        assert!(spans[0].0 <= 0, "floor starts at x={} (should reach 0)", spans[0].0);
        let mut reach = spans[0].1;
        for &(l, r) in &spans[1..] {
            assert!(l <= reach, "gap in the floor row at x={l} (previous ends {reach})");
            reach = reach.max(r);
        }
        assert!(reach >= SCREEN_W, "floor row stops at x={reach}, short of {SCREEN_W}");
    }

    #[test]
    fn toxic_swamp_reef_anchor_is_the_sand_strip_on_the_screen_bottom() {
        let Some(mut m) = booted() else { return };
        let (_, h1) = frame_size(&m.pack, FLOOR_REF);
        let by1 = m.pack.frame(BASE, FLOOR_REF).unwrap().by;
        // MF_7A0 @0x00AA..0x00FC reduced to the layer offset: on 640x480
        // that is 480 − (389 + 34) = 57, the value every reef piece in the
        // 2026-09-01 golden reel sits at (seq 1 top 446, seq 5 413,
        // seq 199 403, seq 100 452, seq 102 447).
        assert_eq!(m.reef_dy, SCREEN_H - (by1 + h1));
        assert_eq!(m.reef_dy, 57, "640x480 reef anchor drifted from the reel");

        // Roles 5-8 are the static dressing; role 5 alone carries MF_7A7's
        // `addi.w [A2 + 0x0126], 0x14` (@0x293E).
        for role in 5..=8usize {
            let a = m.slots[role];
            let want = m.reef_dy + if role == 5 { DRESSING_Y_NUDGE } else { 0 };
            assert_eq!(
                a.oy, want,
                "dressing slot {role} (compound {}) drew at oy {} not {want}",
                a.fb_first, a.oy
            );
            let f = m.pack.frame(BASE, a.fb_cur).expect("armed compound");
            let (w, _) = frame_size(&m.pack, a.fb_cur);
            let x = f.bx + a.ox;
            assert!(
                x + w > 0 && x < SCREEN_W,
                "dressing slot {role} rolled off-screen at x={x} (w {w})"
            );
        }
        let _ = m.dim(FLOOR_REF);
    }

    #[test]
    fn toxic_swamp_reef_floor_draws_behind_every_actor() {
        let Some(m) = booted() else { return };
        let mut out: Vec<SpriteDraw> = Vec::new();
        m.sprites(&mut out);
        assert!(out.len() > m.floor.len(), "no actors emitted after the floor");
        // MF_7A8 blits the row into the canvas background (@0x2E44) before a
        // single sprite composites, and the reel confirms the rest of the
        // order (t=160: a skeletal fish passes in front of the anemone).
        for (i, t) in m.floor.iter().enumerate() {
            assert_eq!(out[i].png, t.png, "sprite {i} is not the floor tile");
            assert_eq!((out[i].x, out[i].y), (t.x, t.y));
        }
    }

    #[test]
    fn toxic_swamp_pipe_builder_shares_the_floor_caps_end() {
        // MF_7A7 case 4 branches on MG_7B0.left (@0x2724), the rect MF_7A8
        // left behind, so the pipe/coral piece always sits on the painted
        // pipe cap — mirrored when that cap is at the left (+0x136 = 1
        // @0x2742). Rewritten 2026-09-29: this used to pin the piece's
        // centre on the cap's centre, which is the port's old placement,
        // not the capture's. Pinned now to the goldens (NCC fits, all three
        // QEMU captures): cap 7 at 544..640 / −1..95, piece 9 at 565..640 /
        // −1..74, both top 363, and static from the first frame.
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut seen_left = false;
        let mut seen_right = false;
        for seed in 1..40u64 {
            let mut m = ToxicSwamp::new(pack.clone());
            let mut ctx = mk_ctx(0);
            ctx.rng = RandomLong::new(seed);
            ctx.rng15 = Random15::new(seed as u32);
            m.tick(&mut ctx);
            let cap = m.floor.first().expect("floor cap");
            let a = m.slots[4];
            let f = m.pack.frame(BASE, a.fb_cur).expect("armed compound");
            let (left, top) = (blit_x(f, &a), blit_y(f, &a));
            assert_eq!(top, 363, "pipe piece top off the goldens");
            if m.floor_pipe_left {
                seen_left = true;
                assert_eq!(cap.x, -1, "left cap: goldens fit it at -1..95");
                assert!(a.mirror, "left-end pipe must draw mirrored");
                assert_eq!(left, -1, "left-end pipe piece: goldens fit it at -1..74");
            } else {
                seen_right = true;
                assert_eq!(cap.x, 544, "right cap: goldens fit it at 544..640");
                assert!(!a.mirror, "right-end pipe must draw unmirrored");
                assert_eq!(left, 565, "right-end pipe piece: goldens fit it at 565..640");
            }
        }
        assert!(seen_left && seen_right, "coin flip never took both ends");
    }

    /// MF_7A7 case 3 + machine-A state 21, re-verified 2026-09-29 against
    /// `qemu/toxic-swamp`, `qemu/toxic-swamp-leftcap` and the 2026-09-13
    /// `toxic-swamp` (see LARRY_SINK_STOP for the table). Rewritten: the old
    /// version pinned the port's capture-tuned anchor walk — a fixed ≡ 2 grid,
    /// a stop at 474 and a +3 jump onto a constant 477 — which no capture
    /// shows. What all three show, and what this pins:
    /// - he sinks on his art's own ramp: +4 a frame, except 679's 685 → 686
    ///   (+5; one such step in each QEMU capture, at 163 → 168 / 215 → 220);
    /// - he leaves 21 on the first advance whose centre reaches 405 and is
    ///   drawn as 700 on that point, then walks 700's ramp +2, +2, +3
    ///   (468 → 472 474 474 475, 472 → 476 478 478 479, 470 → 474 476 _ 477:
    ///   +4 into 700 in all three, the h-131 exit);
    /// - the rest bottom depends on the grid phase and is NOT a constant
    ///   (475 / 479 / 477 in the three captures; 474..=485 by the exit rule);
    /// - he never moves off it, and never slides sideways off his art's `bx`.
    #[test]
    fn toxic_swamp_larry_sinks_in_from_above_and_then_stays_put() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut rests = std::collections::BTreeSet::new();
        let mut plus4 = 0;
        for seed in [1u32, 3, 5, 7, 9, 11, 13, 15] {
            let mut m = ToxicSwamp::new(pack.clone());
            let mut ctx = mk_ctx(0);
            ctx.rng15 = Random15::new(seed);
            let mut pace = Pacer::new(&m);
            // (state, compound, drawn left − bx, drawn bottom), one entry per
            // change of what is on screen.
            let mut walk: Vec<(u8, u32, i32, i32)> = Vec::new();
            let mut out: Vec<SpriteDraw> = Vec::new();
            while ctx.now_ms < 25_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                let a = m.slots[3];
                assert_eq!(a.role, 3, "slot 3 is not Larry");
                let f = m.pack.frame(BASE, a.fb_cur).expect("armed compound");
                out.clear();
                m.sprites(&mut out);
                let d = &out[m.floor.len() + 3];
                assert_eq!(d.png, f.png, "slot 3 is not where sprites() puts it");
                let e = (a.state, a.fb_cur, d.x - f.bx, d.y + f.h);
                if walk.last().map(|w| (w.1, w.2, w.3)) != Some((e.1, e.2, e.3)) {
                    walk.push(e);
                }
            }
            assert_eq!(walk[0].3, LARRY_SPAWN_BOTTOM, "seed {seed}: spawn bottom");
            assert!(walk.iter().all(|w| w.2 == walk[0].2), "seed {seed}: Larry slid off his bx");
            let land = walk
                .iter()
                .position(|w| w.1 == 700)
                .unwrap_or_else(|| panic!("seed {seed}: Larry never reached 700"));
            for i in 1..land {
                let dy = walk[i].3 - walk[i - 1].3;
                let want = if walk[i].1 == 686 { 5 } else { 4 };
                assert_eq!(
                    dy, want,
                    "seed {seed}: sink step {} -> {} on {} is {dy} px",
                    walk[i - 1].3, walk[i].3, walk[i].1
                );
            }
            // The exit: the first advance whose CENTRE reaches
            // 480 − 131/2 − 10 = 405; 700 is armed on that point (plain
            // SetRun) and drawn at once. The three captures pin the
            // threshold to 405..=406 (qemu 406 exits, leftcap 398 → 410,
            // 2026-09-13 404 does not).
            let centre_of = |w: &(u8, u32, i32, i32)| {
                let h = m.pack.frame(BASE, w.1).map(|f| f.h).unwrap_or(0);
                w.3 - h + h / 2
            };
            assert!(centre_of(&walk[land - 1]) < 405, "seed {seed}: sank past the exit");
            assert!(centre_of(&walk[land]) >= 405, "seed {seed}: left 21 early");
            if walk[land].3 - walk[land - 1].3 == 4 {
                plus4 += 1;
            }
            let b = walk[land].3;
            let ramp: Vec<(u32, i32)> =
                walk[land + 1..land + 4].iter().map(|w| (w.1, w.3 - b)).collect();
            assert_eq!(
                ramp,
                vec![(701, 2), (702, 2), (703, 3)],
                "seed {seed}: 700's design ramp (284 286 286 287) was not walked"
            );
            let rest = walk[land + 3].3;
            // centre 405..=416 (416 = 675 → 676's +12 from just under 405) + 69
            assert!((474..=485).contains(&rest), "seed {seed}: rest bottom {rest}");
            for w in &walk[land + 3..] {
                assert_eq!(w.3, rest, "seed {seed}: Larry moved off the sand on {}", w.1);
            }
            rests.insert(rest);
        }
        assert!(
            rests.len() > 1,
            "every seed parked Larry on the same bottom {rests:?}; the captures \
             park him on 475, 477 and 479"
        );
        // All three captures step +4 into 700 (the h-131 exits, by far the
        // commonest); taller frames' exits step less (seed 3: +2).
        assert!(plus4 * 2 >= 8, "only {plus4}/8 landings stepped +4 into 700");
    }

    /// Machine-A slots 1/2 (958 / 900) are invisible in every golden: rows
    /// 0-2 of `qemu/toxic-swamp` (60 s), `-leftcap` (100 s) and
    /// `-leftcap-t12` (120 s) carry no static pixel, and nothing else of them
    /// shows. fn51 enters 1/5 park them above the world for 12-13 minutes —
    /// with the ORIGINAL BUG that fn55 cases 1/2 store their box as (h, w)
    /// (SKELETON_BOX_SWAPPED), so 958 hangs at `−w/2`, wholly off screen.
    /// Read as (w, h), 958's bottom row lands on screen row 0 (12 lit px).
    #[test]
    fn toxic_swamp_skeleton_slots_stay_off_screen_while_they_wait() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        for seed in [1u32, 3, 5, 7] {
            let mut m = ToxicSwamp::new(pack.clone());
            let mut ctx = mk_ctx(0);
            ctx.rng15 = Random15::new(seed);
            let mut pace = Pacer::new(&m);
            let mut out: Vec<SpriteDraw> = Vec::new();
            while ctx.now_ms < 120_000 {
                pace.advance(&mut ctx);
                m.tick(&mut ctx);
                if pace.now_ms() % 997 > 17 {
                    continue; // sample ~once a second
                }
                out.clear();
                m.sprites(&mut out);
                for slot in [1usize, 2] {
                    let a = m.slots[slot];
                    assert!(matches!(a.state, 1 | 5), "seed {seed}: slot {slot} left its wait");
                    let d = &out[m.floor.len() + slot];
                    let h = m.pack.frame(BASE, a.fb_cur).map(|f| f.h).expect("armed");
                    assert!(
                        d.y + h <= 0,
                        "seed {seed} t {}: slot {slot} ({}) shows rows down to {}",
                        ctx.now_ms,
                        d.png,
                        d.y + h
                    );
                }
            }
        }
    }

    /// The blitted rect of a critter's current frame: what `sprites()` emits.
    fn crect(m: &ToxicSwamp, a: &Actor) -> (i32, i32, i32, i32) {
        let f = m.pack.frame(BASE, a.fb_cur).expect("armed compound");
        (blit_x(f, a), f.by + a.oy, f.w, f.h)
    }

    /// **The 2026-09-12 capture's headline falsification.** §8.1's "position =
    /// random point in the world" is only half true: `toxic-swamp.mp4` has
    /// ~46 edge entries and ~43 edge exits over 120 s and NOT ONE fish ever
    /// materialises in open water. The module's own first frame (video
    /// t = 2.833-2.900 — the reef is already complete) carries no critter at
    /// all; the first pixels cross the left and right borders one module
    /// frame later. So x is the entry edge and only y is rolled.
    #[test]
    fn toxic_swamp_critters_are_born_against_a_side_edge() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let (mut from_left, mut from_right) = (0, 0);
        for seed in 1..12u64 {
            let mut m = ToxicSwamp::new(pack.clone());
            let mut ctx = mk_ctx(0);
            ctx.rng = RandomLong::new(seed);
            ctx.rng15 = Random15::new(seed as u32);
            m.tick(&mut ctx);
            assert!(!m.critters.is_empty(), "no critters were birthed");
            for (i, a) in m.critters.iter().enumerate() {
                let (x, y, w, h) = crect(&m, a);
                if a.flip {
                    from_left += 1;
                    assert_eq!(x + w, 0, "mirrored critter {i} is not flush off the left");
                    assert!(a.mirror && a.mirror_x, "a flipped critter blits mirrored");
                } else {
                    from_right += 1;
                    assert_eq!(x, SCREEN_W, "plain critter {i} is not flush off the right");
                }
                // @0x65B0..0x65CC `+0x12C = rand % (bottom − h) + h` read as
                // a bottom: every newborn is fully on screen vertically,
                // never clipped. (The capture's entry y-centres run 22..440,
                // the whole screen height, reef band included.)
                assert!(
                    y >= 0 && y + h <= SCREEN_H,
                    "critter {i} was born clipped vertically at y {y}..{}",
                    y + h
                );
                // ...and nothing is on screen on the frame it is born.
                assert!(
                    off_screen(&m.pack, a) || x + w == 0 || x == SCREEN_W,
                    "critter {i} materialised in open water at ({x}, {y})"
                );
            }
        }
        assert!(
            from_left > 0 && from_right > 0,
            "the flip coin never took both edges ({from_left} L / {from_right} R)"
        );
    }

    /// The flip is not cosmetic. Measured on `toxic-swamp.mp4` t = 4.6-8.6
    /// (the green eel that enters from the left): matching run 656 MIRRORED
    /// beats unmirrored by 9.4 vs 45+ mean abs error, and `blit_x + bx + w`
    /// holds constant within a run cycle (583/584 → 711/712 → 839/840 → 967,
    /// stepping by the wrap frame's own dx = −128). So a flipped critter both
    /// draws mirrored AND travels the opposite way at the same authored rate.
    #[test]
    fn toxic_swamp_flipped_critters_swim_the_other_way() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let m = ToxicSwamp::new(pack);
        let clock = m.clock();
        // two green eels (species 6 -> state 35, the one species whose script
        // has no transitions at all), one each way, born on the same tick.
        let mut ctx = mk_ctx(0);
        let mut pair: Vec<Actor> = Vec::new();
        for flip in [true, false] {
            let mut a = Actor::new(Kind::Critter);
            a.flip = flip;
            a.mirror = flip;
            a.mirror_x = flip;
            b_enter(&m.pack, &mut a, &mut ctx, 35);
            let f = m.pack.frame(BASE, a.fb_cur).expect("armed");
            a.ox = if flip { f.bx } else { SCREEN_W - f.bx };
            a.oy = 200 - f.by;
            pair.push(a);
        }
        let start: Vec<i32> = pair.iter().map(|a| crect(&m, a).0).collect();
        let mut xs: Vec<Vec<i32>> = vec![Vec::new(), Vec::new()];
        for k in 1..40u64 {
            ctx.now_ms = clock.now_ms(k);
            for (j, a) in pair.iter_mut().enumerate() {
                // state 0x23 only loops its run; nothing here goes off-screen
                let _ = b_tick(&m.pack, &[], 0, a, &mut ctx);
                xs[j].push(crect(&m, a).0);
            }
        }
        // run 656's wrap frame carries dx = −128 over 14 frames: 9.14 px per
        // module frame, and the capture measures 9.71 on a 100 ms sampling
        // grid (= 9.7 * 1.064 / 1.064). Opposite signs, equal magnitude.
        let dr = xs[0].last().unwrap() - start[0];
        let dl = xs[1].last().unwrap() - start[1];
        assert!(dr > 200, "the mirrored eel must swim RIGHT; it moved {dr}");
        assert!(dl < -200, "the plain eel must swim LEFT; it moved {dl}");
        assert!(
            (dr + dl).abs() <= 4,
            "the two directions must travel the same distance ({dr} vs {dl})"
        );
        // and each step is monotone — no per-frame teleport back to the
        // run's authored start (the bug a naive `bx + ox` mirror would give)
        for j in 0..2 {
            for i in 1..xs[j].len() {
                let d = xs[j][i] - xs[j][i - 1];
                assert!(
                    d.abs() <= 30,
                    "eel {j} jumped {d} px in one frame at step {i}"
                );
            }
        }
    }

    /// Shoaling, as the capture actually produces it: two critters of the
    /// same species born on the SAME tick and heading the SAME way hold an
    /// exact x lockstep with a constant Δy, because a critter's x is a pure
    /// function of (armed frame, entry edge) — `dx` is 0 on every non-wrap
    /// frame of every fish run, so the whole leftward walk lives in the
    /// authored `bx` column.
    ///
    /// `toxic-swamp.mp4`: the two green eels that enter at the left at
    /// t = 3.0 share an identical x extent frame for frame for their whole
    /// life (t = 4.7 x 8..171 and x 8..171; t = 5.4 x 73..237 and x 73..237)
    /// at a constant Δy = 247; the two tan puffers that enter at the right at
    /// t = 4.3 do the same at Δy = 50 until their state machines take
    /// different random branches at t ≈ 6.8. No leader/follower link, no
    /// formation code — just simultaneous birth plus a deterministic run.
    #[test]
    fn toxic_swamp_same_species_same_tick_births_shoal_in_lockstep() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let m = ToxicSwamp::new(pack);
        let clock = m.clock();
        let mut ctx = mk_ctx(0);
        let mut shoal: Vec<Actor> = Vec::new();
        for y in [40, 287, 400] {
            let mut a = Actor::new(Kind::Critter);
            a.flip = true;
            a.mirror = true;
            a.mirror_x = true;
            b_enter(&m.pack, &mut a, &mut ctx, 35); // species 6, the green eel
            let f = m.pack.frame(BASE, a.fb_cur).expect("armed");
            a.ox = f.bx;
            a.oy = y - f.by;
            shoal.push(a);
        }
        let dy0: Vec<i32> = shoal.iter().map(|a| crect(&m, a).1).collect();
        for k in 1..60u64 {
            ctx.now_ms = clock.now_ms(k);
            for a in shoal.iter_mut() {
                let _ = b_tick(&m.pack, &[], 0, a, &mut ctx);
            }
            let r: Vec<(i32, i32, i32, i32)> = shoal.iter().map(|a| crect(&m, a)).collect();
            for j in 1..r.len() {
                assert_eq!(
                    r[j].0, r[0].0,
                    "frame {k}: the shoal broke x lockstep ({:?})",
                    r.iter().map(|q| q.0).collect::<Vec<_>>()
                );
                assert_eq!(
                    r[j].1 - r[0].1,
                    dy0[j] - dy0[0],
                    "frame {k}: the shoal's Δy drifted"
                );
            }
        }
    }

    /// Do two drawn rects overlap? A swimming critter always covers some of
    /// the pixels it covered on the previous frame — the fastest authored
    /// walk in the pack is 27 px against a 96 px body. Neither the sprite's
    /// centre nor its left edge is usable on its own: a run's `w` can more
    /// than double inside one cycle (the skeletal wisp's 465 goes 96 → 236)
    /// so the centre jumps when the art collapses at the wrap, and the left
    /// edge jumps when it grows.
    fn overlaps(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
    }

    /// One critter sample, taken only on the ticks it committed.
    struct Sample {
        t: u64,
        state: u8,
        run: u32,
        cur: u32,
        /// the drawn rect `sprites()` emits for this frame
        rect: (i32, i32, i32, i32),
        off: bool,
        /// `fn17` @62C2 just re-birthed this critter: it is flush against the
        /// edge it is about to swim away from, so the step that put it there
        /// is a teleport on purpose.
        born: bool,
    }

    /// Drive the module through `Pacer` (never a fixed step — §1 rule 11)
    /// and collect every critter commit.
    fn run_critters(ticks: u64) -> Option<Vec<Vec<Sample>>> {
        let pack = Pack::load(std::path::Path::new("../assets/toxic-swamp")).ok()?;
        let mut m = ToxicSwamp::new(pack);
        let mut ctx = mk_ctx(0);
        let mut pace = Pacer::new(&m);
        let mut out: Vec<Vec<Sample>> = Vec::new();
        let mut due: Vec<u64> = Vec::new();
        for _ in 0..ticks {
            let t = pace.advance(&mut ctx);
            m.tick(&mut ctx);
            for (i, a) in m.critters.iter().enumerate() {
                if out.len() <= i {
                    // first sighting: the birth snapshot, not a commit
                    out.push(Vec::new());
                    due.push(a.next_tick);
                    continue;
                }
                if due[i] == a.next_tick {
                    continue;
                }
                due[i] = a.next_tick;
                let Some(f) = m.pack.frame(BASE, a.fb_cur) else { continue };
                out[i].push(Sample {
                    t,
                    state: a.state,
                    run: a.fb_first,
                    cur: a.fb_cur,
                    rect: (blit_x(f, a), blit_y(f, a), f.w, f.h),
                    off: off_screen(&m.pack, a),
                    born: blit_x(f, a) == SCREEN_W || blit_x(f, a) + f.w == 0,
                });
            }
        }
        Some(out)
    }

    /// **Symptom #2, 2026-09-16 — "some fish blinked across the screen."**
    /// `fn12` @44E0 state 0x1d toggles the facing word `+0x11c` and nothing
    /// else: the C leaves the placement point alone and the art library
    /// mirrors the compound about the sprite, so the fish turns in place.
    /// This port mirrors about `ox`, a bank-space anchor a few hundred px
    /// from the fish, so a bare toggle threw it clean across the screen
    /// (measured −368 px in one commit on the piranha's 0x1d → 0x15 turn).
    /// Nothing a critter does on screen may move it clear of the pixels it
    /// occupied on the previous frame.
    #[test]
    fn toxic_swamp_critters_never_teleport_on_screen() {
        let Some(fish) = run_critters(3000) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut checked = 0;
        for (i, samples) in fish.iter().enumerate() {
            for w in samples.windows(2) {
                // an off-screen sample is the tick `fn19` @676E re-births on,
                // and a birth is *supposed* to jump to the far edge.
                if w[0].off || w[1].off || w[1].born {
                    continue;
                }
                checked += 1;
                assert!(
                    overlaps(w[0].rect, w[1].rect),
                    "fish {i} jumped clear of itself at t={}: {:?} then {:?} \
                     (state {:#x} -> {:#x}, run {} -> {})",
                    w[1].t,
                    w[0].rect,
                    w[1].rect,
                    w[0].state,
                    w[1].state,
                    w[0].run,
                    w[1].run
                );
            }
        }
        assert!(checked > 1000, "only {checked} on-screen commits sampled");
    }

    /// **Symptom #1, 2026-09-16 — "fish animations are REAL jerky."**
    /// `fn16` @61D4 is called with arg 1 everywhere (@486A and
    /// twins), and its wrap test is `cur >= last` AFTER the advance, so the
    /// run's LAST frame is never drawn: it is the link marker, repeating the
    /// first frame's art with the loop-back offset. The port used to draw it
    /// and then draw `first` on top of it at the same place — one frozen
    /// duplicated frame every cycle, which on the red fish's 7-frame run 615
    /// is one commit in seven. A critter must never show the same compound
    /// on two consecutive commits, and each commit is one frame of the C's
    /// `+0x120 = clock + 100` gate on the Mac grid.
    #[test]
    fn toxic_swamp_loop_frames_never_stall() {
        let Some(fish) = run_critters(3000) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut commits = 0;
        for (i, samples) in fish.iter().enumerate() {
            for w in samples.windows(2) {
                if w[1].born {
                    continue;
                }
                commits += 1;
                assert!(
                    w[0].cur != w[1].cur,
                    "fish {i} held compound {} across two commits at t={} \
                     (state {:#x}, run {})",
                    w[0].cur,
                    w[1].t,
                    w[1].state,
                    w[1].run
                );
                // the frame gate itself: `now + 100` on `TickClock::MacTick`
                let gap = w[1].t - w[0].t;
                assert!(
                    (99..=100).contains(&gap) || (116..=117).contains(&gap),
                    "fish {i} committed {gap} ms apart; the C's 100 ms gate on \
                     the Mac grid allows only 99/100 or 116/117"
                );
            }
        }
        assert!(commits > 1000, "only {commits} commits sampled");
    }

    /// **Hand-offs are M129's own `fn3DDC` link, NOT the L132 `fn028A` →
    /// L135 `fn3F2E` shared-part chain** (audit 2026-09-29). Both actor
    /// classes (`g00A8` critter, `g0400` slot) do bind `+0x7C` = `fn0204`,
    /// `+0x108` = `fn028A`, `+0xCC` = `fn0D3C`, but M129 never dispatches
    /// any of those slots (nor `+0x80`/`+0x84`/`+0x94`/`+0x98`/`+0xB0`/
    /// `+0xB4`/`+0xD0`, the other library paths that reach them) on an
    /// actor: the only actor slots it calls are `+0x6C` = L135 `fn16BC`
    /// (seat the frame centred on `pos`, no link), `+0x138` and the `+0x92`
    /// state object. A hand-off writes first/last/cur directly and then
    /// `fn20 @6828` (critters, enters 0xc/0x10/0x1f..0x22) or `fn59 @306C`
    /// (slots) adds the shared sequence's `+0x78` = `fn3DDC` link
    /// `(old last → new first)` to `pos`, x negated by the actor's own flip
    /// word; the sequence flip is 0, so no flip rounding, and nothing
    /// toggles the flip. Over 6 seeds × 30 000 ticks the reachable hand-offs
    /// (546 distinct) match this within the mirrored blit's 1 px width
    /// parity; the `fn3F2E` chain would have moved 130 of them, by up to
    /// 406 px (e.g. 294 → 383, the puffer re-arm), and toggles none.
    ///
    /// Exhaustive over the pack: every frame of every run as the outgoing
    /// frame, every run start as the incoming one, both facings for a
    /// critter. The flipped SLOT (the left-cap pipe builder) is left out on
    /// purpose: `fn53`/`fn54`/`fn59` negate its link dx by `+0x136`, while
    /// this port draws it at the unmirrored authored `bl` — an open,
    /// capture-unverified question (the golden reel capped the right end),
    /// not something to pin.
    #[test]
    fn toxic_swamp_hand_offs_link_old_last_to_new_first() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let runs: Vec<(u32, u32)> = pack
            .series(BASE)
            .iter()
            .map(|s| (s.first, s.first + s.frames.len() as u32 - 1))
            .collect();
        // `pos` as the library holds it: the drawn rect's left + (w >> 1)
        let pos = |a: &Actor| {
            let f = pack.frame(BASE, a.fb_cur).unwrap();
            (blit_x(f, a) + (f.w >> 1), blit_y(f, a) + (f.h >> 1))
        };
        let mut checked = 0usize;
        for &(first, last) in &runs {
            for cur in first..=last {
                for &(new, _) in &runs {
                    for (kind, flip) in [(Kind::Critter, false), (Kind::Critter, true), (Kind::Slot, false)] {
                        let mut a = Actor::new(kind);
                        (a.fb_first, a.fb_last, a.fb_cur) = (first, last, cur);
                        (a.ox, a.oy) = (300, 200);
                        (a.flip, a.mirror, a.mirror_x) = (flip, flip, flip && kind == Kind::Critter);
                        let p0 = pos(&a);
                        arm_as(&pack, &mut a, new, true);
                        let p1 = pos(&a);
                        let (cl, cn_) = (cn(pack.frame(BASE, last).unwrap()), cn(pack.frame(BASE, new).unwrap()));
                        let want = (if flip { cl.0 - cn_.0 } else { cn_.0 - cl.0 }, cn_.1 - cl.1);
                        let got = (p1.0 - p0.0, p1.1 - p0.1);
                        assert_eq!(a.fb_cur, new, "{cur} -> {new} did not seat the run start");
                        assert_eq!(a.flip, flip, "{cur} -> {new} toggled the flip");
                        assert_eq!(got.1, want.1, "{cur} -> {new} y (flip {flip})");
                        assert!(
                            (got.0 - want.0).abs() <= flip as i32,
                            "{cur} -> {new} (last {last}, flip {flip}): moved {got:?}, fn3DDC link {want:?}"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100_000, "only {checked} hand-offs checked");
    }

    /// **Symptom #1's ratchet.** A Loop run's visible cycle is `len − 1`
    /// commits, not `len`: `fn16` @61D4's wrap test is `cur >= last` after
    /// the advance (arg 1, @486A), so the run's last frame is
    /// the link marker and is never drawn. It repeats the first frame's art
    /// carrying the loop-back offset (211/227 −112, 615/621 −22, 656/669
    /// −128) or the next run's authored origin (525/536, 538/550, 591/595),
    /// and that offset rides on the restart — so the swim crosses the wrap
    /// without a stalled frame and without a snap-back.
    #[test]
    fn toxic_swamp_loop_cycle_skips_the_link_marker() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        // (state, run, frames drawn per cycle, the per-commit walk the pack
        // authors for that run — min..=max of `bl` steps, wrap included)
        let cases: [(u8, u32, u32, i32, i32); 5] = [
            (0x1e, 615, 6, -5, -3),   // red fish, the worst stall: 1 in 7
            (1, 211, 16, -7, -7),     // many-eyed gar, a flat 7 px a frame
            (35, 656, 13, -18, -4),   // green eel
            (0x16, 525, 11, -10, 3),  // grey piranha's hover
            (0x17, 538, 12, -8, 2),   // ...and its first swim leg
        ];
        for (state, run, cycle, lo, hi) in cases {
            let mut a = Actor::new(Kind::Critter);
            let mut ctx = mk_ctx(0);
            b_enter(&pack, &mut a, &mut ctx, state);
            assert_eq!(a.fb_first, run, "state {state:#x} arms run {run}");
            let last = a.fb_last;
            let mut seen: Vec<u32> = vec![a.fb_cur];
            let mut xs: Vec<i32> = vec![bl(pack.frame(BASE, a.fb_cur).unwrap()).0 + a.ox];
            for _ in 0..(cycle * 2) {
                step(&pack, &mut a, Step::Loop);
                seen.push(a.fb_cur);
                xs.push(bl(pack.frame(BASE, a.fb_cur).unwrap()).0 + a.ox);
            }
            assert!(
                !seen.contains(&last),
                "run {run}'s link marker {last} was drawn: {seen:?}"
            );
            assert_eq!(
                seen[0], seen[cycle as usize],
                "run {run} should cycle every {cycle} commits: {seen:?}"
            );
            for (k, d) in xs.windows(2).map(|w| w[1] - w[0]).enumerate() {
                assert!(
                    (lo..=hi).contains(&d),
                    "run {run} step {k} moved {d} px; the authored walk is \
                     {lo}..={hi} — a wrap that drops its link offset shows up here"
                );
            }
        }
    }

    /// **The 2026-09-15 ledger's third symptom — "a pipe piece floating
    /// mid-water at ~150 s".** The reef builder (`fn51` states 9..0xd,
    /// compounds 9/12/31/51 and the caps-lock 1046) grows on the end
    /// `MF_7A8` capped and is authored on ONE bottom line: every frame of
    /// every growth run has `by + h` = 417, so with the reef offset it
    /// bottoms out on the sand at 474. `SetRun` does not move the placement
    /// point, so the pipe cannot climb. The old `arm` pinned the new run's
    /// TOP onto the old frame's top instead, and `by` steps 292 -> 300 while
    /// `h` steps 125 -> 117, so the pipe walked 8 px up at every hand-off,
    /// 22 px per builder cycle.
    #[test]
    fn toxic_swamp_reef_dressing_stays_planted_on_the_sand() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut m = ToxicSwamp::new(pack);
        let mut ctx = mk_ctx(0);
        let mut pace = Pacer::new(&m);
        let mut base: Vec<Option<i32>> = Vec::new();
        // ~200 s: the builder has cycled a dozen times by here.
        for _ in 0..12_000u64 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            for (i, a) in m.slots.iter().enumerate() {
                // Larry (role 3) walks on purpose; the two skeleton drops
                // (roles 1/2) sink in from above.
                if matches!(a.role, 1 | 2 | 3) {
                    continue;
                }
                let Some(f) = m.pack.frame(BASE, a.fb_cur) else { continue };
                let bottom = blit_y(f, a) + f.h;
                while base.len() <= i {
                    base.push(None);
                }
                let first = *base[i].get_or_insert(bottom);
                assert!(
                    (bottom - first).abs() <= 4,
                    "reef slot {i} (role {}, run {}, compound {}) drifted to \
                     bottom {bottom} at t={}; it was planted at {first}",
                    a.role,
                    a.fb_first,
                    a.fb_cur,
                    ctx.now_ms
                );
            }
        }
    }

    /// Trace lane (2026-09-16, the "jerky + blinked across the screen" fix):
    /// drive the module through `Pacer` and print one line per critter per
    /// commit — (t, state, run, d6, travel). Flags every on-screen commit
    /// that moves more than 40 px (the teleport) and every commit that holds
    /// the drawn compound (the stall that reads as jerk).
    /// `cargo test -p app -- --ignored --nocapture toxic_swamp_trace`
    #[test]
    #[ignore]
    fn toxic_swamp_trace() {
        let Some(fish) = run_critters(3000) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let (mut jumps, mut stalls) = (0, 0);
        for (i, samples) in fish.iter().enumerate() {
            for s in samples {
                println!(
                    "t={} fish{i} st{:#x} run{} d6={} rect={:?} off={}",
                    s.t, s.state, s.run, s.cur, s.rect, s.off
                );
            }
            for w in samples.windows(2) {
                if !w[0].off && !w[1].off && !overlaps(w[0].rect, w[1].rect) {
                    jumps += 1;
                    println!(
                        "  JUMP  fish {i} t={} {:#x}->{:#x} run {}->{} {:?} {:?}",
                        w[1].t, w[0].state, w[1].state, w[0].run, w[1].run,
                        w[0].rect, w[1].rect
                    );
                }
                if w[0].cur == w[1].cur {
                    stalls += 1;
                    println!(
                        "  STALL fish {i} t={} held {} (state {:#x})",
                        w[1].t, w[0].cur, w[1].state
                    );
                }
            }
        }
        println!("== jumps {jumps}  stalls {stalls}");
    }

    /// MF_7A7 case 3 + machine-A state 21: Larry the Lawyer is dropped in from
    /// above the world and sinks onto the sand at a fixed rate, then never
    /// moves off that spot again. Pins the two things the live test caught:
     /// **Ratchet 1 — Jason, 2026-09-19: "some [fish] going backwards, most
    /// are jerky".** The art library mirrors a compound at BLIT time (the
    /// `vtbl[0x0C]` toggle the facing coin sets, @0x2C10/@0x2CAA for the reef
    /// cap and the critter flip word `+0x11c` everywhere else). This port
    /// used to fake that by WRITING a pre-mirrored `m_NNN.png` into the pack
    /// directory the first time a compound was drawn flipped, and then
    /// drawing it only `if self.mirrored.contains(&fb_cur)`. `blit_x` placed
    /// the MIRRORED box unconditionally, so the moment that file write did
    /// not happen — a read-only or sandboxed pack, which is exactly what an
    /// installed `.saver` is — every rightward critter drew its *unmirrored*
    /// art inside a mirrored box: a fish swimming right while facing left,
    /// jittering as the box width walked 96 -> 255 px underneath it. Both of
    /// Jason's fish symptoms, from one silent `let Ok(file) = ... else
    /// { return };`.
    ///
    /// The invariant that kills it for good: **every png this module emits is
    /// one the pack itself declares**, and mirroring rides `SpriteDraw.flip`,
    /// which `compose` honours. Nothing is generated, so nothing can fail to
    /// be generated.
    #[test]
    fn toxic_swamp_mirroring_rides_the_blit_flag_and_writes_no_art() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let known: std::collections::HashSet<String> = pack
            .series(BASE)
            .iter()
            .flat_map(|s| s.frames.iter())
            .map(|f| f.png.clone())
            .collect();
        let mut m = ToxicSwamp::new(pack);
        let mut ctx = mk_ctx(0);
        let mut pace = Pacer::new(&m);
        let mut out: Vec<SpriteDraw> = Vec::new();
        let mut flipped_seen = 0usize;
        // ~90 s: long enough for both entry edges, several run hand-offs and
        // (on a left-capped floor) the mirrored pipe builder.
        for _ in 0..5_400u64 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            out.clear();
            m.sprites(&mut out);
            for s in &out {
                assert!(
                    known.contains(&s.png),
                    "sprites() emitted {} — not a compound the pack declares. \
                     The module is generating art at runtime again; a pack it \
                     cannot write to will draw the unmirrored frame in a \
                     mirrored box (fish facing backwards).",
                    s.png
                );
                if s.flip {
                    flipped_seen += 1;
                }
            }
        }
        assert!(
            flipped_seen > 0,
            "90 s and not one mirrored draw — the flip never reached SpriteDraw"
        );
    }

    /// **Ratchet 2 — Jason, 2026-09-19: "I never got the dude to actually die
    /// even though I set Lungs to Thimble".** MG_7B3 is latched to the ms
    /// clock and then offset by the Lung band (the clock read @0x0106,
    /// stored @0x010A, then the five `+=` branches). The port
    /// read only the `+=` and accumulated instead. DoDrawFrame @0x3F9C
    /// restarts the module on ANY control change and the panel's sliders are
    /// continuous, so *dragging* the Lung slider to Thimble stacked one 30 s
    /// band per re-init and pushed Larry's first expiry minutes out — the act
    /// of setting the control is what stopped him dying.
    ///
    /// Drag the slider the way the panel does, then hold Thimble and watch
    /// him all the way to the 39 <-> 40 tableau.
    #[test]
    fn toxic_swamp_larry_reaches_the_terminal_tableau_after_a_slider_drag() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut m = ToxicSwamp::new(pack);
        let mut ctx = mk_ctx(0);
        let mut pace = Pacer::new(&m);
        // The drag: `sliderMoved` fires per pixel, so the module sees a run
        // of set_control calls, each with a tick behind it.
        for v in (0..=50).rev().step_by(2) {
            m.set_control(1, v);
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        assert_eq!(m.lung_raw, 0, "the drag ended on Thimble");
        let settled = ctx.now_ms;
        let mut terminal_at: Option<u64> = None;
        // 90 s of module time after the drag. The golden capture (Thimble,
        // 640x480) has Larry in the 860-897 family from t ~ 63 s.
        while ctx.now_ms < settled + 90_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
            if let Some(a) = m.slots.iter().find(|a| a.role == 3) {
                if a.state >= 39 && terminal_at.is_none() {
                    terminal_at = Some(ctx.now_ms - settled);
                }
            }
        }
        let larry = m.slots.iter().find(|a| a.role == 3).expect("Larry's slot");
        let at = terminal_at.unwrap_or_else(|| {
            panic!(
                "Larry never reached the 39/40 tableau in 90 s at Thimble \
                 (stopped in state {}, stage deadline {} vs now {}) — the \
                 stage timer is stacking across re-inits again",
                larry.state, m.stage_deadline, ctx.now_ms
            )
        });
        assert!(
            (30_000..90_000).contains(&at),
            "Larry reached the tableau at {at} ms; the capture's Thimble run \
             gets there at ~63 s and cannot beat the 30 s band"
        );
        assert!(
            matches!(larry.state, 39 | 40),
            "tick 40 @0x1EC8 returns to 39 — the tableau is endless, not a \
             terminal state (Larry sat in {})",
            larry.state
        );
    }

    /// **Ratchet 3.** The latch itself, stated once: MG_7B3 after an init is
    /// `now + band`, never `previous + band`. Two inits, two bands, and the
    /// second must not remember the first.
    #[test]
    fn toxic_swamp_stage_timer_latches_to_the_clock_at_every_init() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/toxic-swamp")) else {
            eprintln!("assets/toxic-swamp missing; skipping");
            return;
        };
        let mut m = ToxicSwamp::new(pack);
        let mut ctx = mk_ctx(0);
        let mut pace = Pacer::new(&m);
        m.set_control(1, 0); // Thimble: band 0 = 30 s
        pace.advance(&mut ctx);
        m.tick(&mut ctx);
        let first = m.stage_deadline;
        assert_eq!(first, ctx.now_ms + 30_000, "first init latched the clock");
        // Run a while, then change a control — the *other* one, because
        // @0x3F9C restarts on any of the three.
        while ctx.now_ms < 20_000 {
            pace.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        m.set_control(0, 60);
        pace.advance(&mut ctx);
        m.tick(&mut ctx);
        assert_eq!(
            m.stage_deadline,
            ctx.now_ms + 30_000,
            "the re-init stacked onto the old deadline ({first}) instead of \
             latching the clock"
        );
    }
}
