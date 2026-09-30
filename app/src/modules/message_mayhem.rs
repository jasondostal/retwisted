//! Message Mayhem — full-fidelity transcription of the RE spec
//! (totally-twisted docs/behavior/message-mayhem.md; every constant is
//! disasm-cited there). Two scenes, exactly as the original's DoBlank/
//! DoDrawFrame style split (§5/§6):
//!
//! - **Style == 1 → "Bathroom Wall"** (§5 branch A, §6 style-A path,
//!   §7 pipeline): tile wall (series 1000), the `Pens` 500 vector stroke
//!   writer (§11), grout-joke hijack after round 0 (§9.2), tier+1 messages
//!   per wall on a tier×10 s beat, then the wall is repainted (ERRATUM 3),
//!   and the pen squeak deadline clock+500+random(250) ms (§7.3, ERRATUM 4).
//! - **else → "Aortal Squirt"** — the DEFAULT scene (ERRATUM 1) — (§8):
//!   the man's 8-state SM with sound events 71→2001 ohh2, 36/53→2002
//!   spray, 92→2000 bodyfall, all gated by the 1000 ms rerun guard
//!   (§8, §9.4); his severed artery writes the chosen message with the
//!   **series-3000 blood-drip glyph sprites** — a completely different
//!   writer from the wall's (ERRATUM 6).
//!
//! ## 2026-09-19 — THE MAN IS NOW A PORT, NOT A RECONSTRUCTION
//!
//! Everything below this heading about the Aortal Squirt man was written
//! from the prose spec and the video, and most of the *mechanism* in it was
//! wrong. The scene is now a function-by-function port of `M129 fn61 @0420`
//! and the three classes the controller ctor `fn30 @1DE2` builds around it
//! (see the "Aortal Squirt — THE MAN" section further down for the objects,
//! the state table and the frame driver). **The Bathroom Wall / `Pens` 500
//! stroke writer is untouched** — it keeps its verified 40 ms beat behind a
//! gate in `tick`, because the module itself moved to `TickClock::MacTick`.
//!
//! What the decompile killed, in order of damage:
//!
//! * **The glyph beat.** There is no `GLYPH_MS`, no "one glyph every 1.4 s",
//!   and no left-to-right pen. `fn66 @0DAA` lays the WHOLE message out
//!   up front — every character's x and y — and `fn67 @132C` stamps a
//!   character into the backdrop when the **blood jet's tip reaches it**.
//!   The tip is the part whose art id is 87, which exists only on frames
//!   39..45 and 57..63 (the two spray runs) and marches +9 px per frame.
//!   The neck really is the write cursor, and the cursor's *bottom* is what
//!   decides which LINE is currently writable.
//! * **The walk-out / wander second pass** (`ERRATUM 10`, 334 / 328 / 212 px
//!   legs) was invented. The C's line return is: state 2 keeps walking right
//!   until `fn64 @0D2A` says he is off the field, bumps the row by one line
//!   pitch and goes to state 3; state 3 HIDES him for 3 000 ms; state 4 puts
//!   him back at `screenW + 0x19` walking LEFT until he reaches the next
//!   line's first character. The "second pass" the capture shows on a
//!   one-line message is state 5 — the death walk-in from `screenW + 0x3C`.
//! * **The row.** Not `line_top + 17` off a rolled box. `fn66 @1206` rolls
//!   `RandomBelow(cand.h − (lines+1)·lineH − 100) + 100` and the glyph line
//!   sits at `row − height(frame 36)/2 − height('A')/2`. The 17 px the two
//!   captures disagreed about is half a capital A; the row itself is
//!   **re-rolled every cycle**, so both captures were right.
//! * **The layout box.** No `rand(w/4)+10` origin. `fn66 @0F84` draws one of
//!   four candidate line widths (`W/2, 2W/3, 3W/4, 4W/5`) without
//!   replacement until the message fits that candidate's height, jitters by
//!   `RandomBelow((W − lineW)/2)` and adds a flat 50 px margin.
//! * **The frame clock.** `fn63 @0CF2` is `due = now + 0x5A`, re-armed from
//!   `now` and tested `>=`. On After Dark's own 16.625 ms grid that lands on
//!   ~99.8 ms — the period the capture's periodogram measured. The flat
//!   `MAN_FRAME_MS = 100` was the quantization, not the constant.
//! * **The 117..149 compounds are real sprites, not junk.** Run 117..130 is
//!   the neck spurt: `fn74 @1770` pins its channel-1 marker to the man's
//!   channel-1 marker and only shows it while he is in one of the five runs
//!   that carry one (1, 15, 28, 71, 80) — during the spray runs his own art
//!   carries the jet. Run 139..149 is one blood drip; `fn78 @1A0E` drops one
//!   down a random channel of an already-written glyph, and `fn67 @1580`
//!   adds a drip to the live set on every ODD character.
//! * **No blood-pool sprite** — CONFIRMED. The pool is baked into the fall
//!   run 88..110 and the corpse 112..113.
//! * **The walk loop never stops** — CONFIRMED, and it is the module's only
//!   `+0x7C` immediate (§10.1): state 1 re-arms run 15 every time it
//!   finishes and only leaves on `char_x[ci] <= pos.x`.
//!
//! ### GAPs (open, not filled)
//!
//! * `GAP(jet rect)`: the C turns the jet point into a rect through the
//!   sequence's `+0x98`, which is library code this decompile does not
//!   cover. The port uses art 87's own part rect carried onto the drawn
//!   frame; that reproduces the +9 px/frame cursor the pack's part table
//!   shows, but the rect's exact height is unverified.
//! * `GAP(show/hide)`: `+0x2C` / `+0x30` / `+0x70` are modelled as
//!   Hide / Show / Show. The call sites all read that way (state 3 hides for
//!   three seconds, `fn74` hides the spurt, `fn78` hides a spent drip) but
//!   the library slots were not decoded.
//! * `GAP(fn67 call rate)`: `fn37 @2DB2` installs the man on the sprite list
//!   and `fn38 @2DC6` calls `fn67` from the list's draw; `fn33` runs that
//!   draw twice per frame. The port calls it once. The writer is
//!   position-gated, so the difference is bounded by its 3-character
//!   lookahead.
//! * **Flip — RESOLVED 2026-09-26 (letter-boundary jerk).** The flip
//!   comes from the LIBRARY, as suspected: every run hand-off is L132
//!   `fn028A @028A` → `fn0D3C @0D3C` → L135 `fn3F2E @3F2E`, which keeps
//!   the first part the two frames share on screen AND XORs the sprite's
//!   `+0x3C` flip with the two parts' own flip flags. Frames 43..49 and
//!   58..64 carry their body parts authored mirrored, so the spray-B →
//!   run 1 hand-off (64 → 1) flips the man — run 1 plays mirrored and he
//!   walks FORWARD — spray A stays mirrored, and 49 → 28 flips him back.
//!   `fn61`'s state-1 ENTER clearing the flip is the reset for a new
//!   victim. The port used to apply no delta and no flip at a hand-off:
//!   the feet slid +33 px on every 64 → 1 (the jerk Jason saw) and +9 on
//!   every 49 → 28. Capture check (template match of bank-2000 frames in
//!   `message-mayhem-av.mp4`): mirrored 1..10 / 38 / 39 after every spray
//!   B, top-lefts equal to this port's frame for frame (constant layout
//!   offset), no slide at either hand-off. Per-letter travel stays +90 px,
//!   the capture's figure. The in-run link is `fn3DDC @3DDC` with its
//!   flip rounding; part rects (jet cursor, spurt pin) are laid out under
//!   the flip as L135 `fn3BD6 @3BD6` does.
//! * `GAP(jet rect under flip)`: the jet cursor uses art 87's part rect
//!   mirrored with the man (the `fn3BD6` layout every part query goes
//!   through); `+0x68`/`+0x98` themselves are still undecoded (see
//!   `GAP(jet rect)`).
//! * `GAP(fn35 custom text)`: the user's own message (`fn35 @2B12`, control
//!   value 0) still falls back to `CUSTOM_FALLBACK`; unchanged.
//! * `GAP(style roll rng)`: `fn36 @2D7C`'s "Random" style pick is
//!   `p_RandomBelow` = `Ctx::rng`, but `latch()` still rolls it on `rng15`.
//!   Left alone — it is the scene selector, outside this lane.
//! * Original quirk kept: `fn66 @1004`'s candidate shuffle writes
//!   `order[picked] = order[n]` instead of `order[rolled] = order[n]`.
//!
//! The pack was repacked so `meta.json` carries the compound part tables
//! (`[art, channel, flip, l, t, r, b]`); the jet, the marker attach and the
//! drip placement all read them.
//!
//! Faithful quirks preserved (§9): ink accumulates untouched across the
//! rounds of a wall (1), grout hijack incl. "Andrew A gives good header
//! files!" (2), the squeak is a deadline not a per-stroke event (3),
//! sound rerun guard (4), missing-glyph→space drop (5).
//!
//! STR# 500/501 are read from the pack via `pack.strings` (ripped verbatim
//! into meta.json; matching fn34's `fn_48E0('STR#', 500/501, idx, …)`, §7.1),
//! with the compiled copies as fallback if a pack ever ships without them.
//!
//! ## Two writers, one per scene (ERRATUM 6)
//!
//! The module ships **two** unrelated text renderers and the style branch
//! picks exactly one:
//!
//! | scene | writer | ink |
//! |---|---|---|
//! | Bathroom Wall | `Pens` 500 vector strokes (§11), fn27/fn25/fn19 | `clut 601` slot `base+v`, base = `3·rand(3)` (+9 in endgame) |
//! | Aortal Squirt | series-3000 **blood-drip glyph sprites** (§2) | baked into the art — `#aa0000`, no palette roll at all |
//!
//! §4's art/clut split is the proof, and it is per-branch, not per-depth:
//! the wall arm loads bank **1000 only** and `clut 601 "Bathroom **Pens**
//! 256"`, so the man scene never even has a pen ramp in memory; the man arm
//! loads banks **2000 + 3000** and `clut 500 "Aortal"`, so the wall never
//! has the glyph bank. Running one scene's writer in the other is not a
//! colour bug, it is a whole missing subsystem.
//!
//! ## The pen-stroke writer (spec §11) — Bathroom Wall only
//!
//! The big hand-drawn letters are NOT sprites. `Pens` 500 is a 45-glyph
//! vector stroke font (`[...]` polylines, `{[-]cx,cy,...}` arc runs about a
//! shared centre, `<NNN>` advance) that fn27 (`0x59D6`) walks one dot per
//! call, three calls per frame (§11.4). fn25 (`0x483C`) interpolates a
//! segment into the 127-entry lookahead buffer — straight for `[`, an
//! Archimedean sweep (angle AND radius lerped) for `{` — and fn19
//! (`0x38C4`) stamps a 5×5 brush of palette indices per dot with a
//! darken-only accumulator (§11.6). This module reproduces all of it.
//!
//! ### Asset plumbing
//!
//! The stroke data is Berkeley-derived, so it cannot live in repo source
//! (the repo ships zero Berkeley assets; `assets/` is gitignored). The
//! module loads `pens500.json` from the **pack root** at build() time — the
//! machine-readable dump `scripts/pens_extract.py` produces in the
//! totally-twisted RE repo (§11.9): 45 glyphs × {advance, groups}, the three
//! `DATA 129` brush tables, and the `clut 601` pen ramps.
//!
//! **TODO (packer, not this file): `tools/pack_assets.py` owes a step that
//! emits `pens500.json` into each Message Mayhem pack by running
//! `scripts/pens_extract.py` over `Message Mayhem_Pens_500.bin`.** Until
//! that lands, a pack built by the old packer simply has no `pens500.json`;
//! the module then **WARN-falls back** to the legacy series-3000 compound
//! stamping below so such packs still run (badly, but they run).
//!
//! ### Ink surface mechanism
//!
//! fn19 writes *pixels* into the offscreen canvas, not sprites, and the
//! darken-only rule needs read-back — so the writer needs a persistent
//! pixel surface. The engine's module contract only carries `SpriteDraw`
//! (a packed png placed at x/y) and this task may not touch `engine/`, so
//! the surface is built **inside this module**: a 640×480 byte buffer of
//! `clut 601` slot indices (`0xFF` = untouched), run-length encoded per row
//! each time it changes and emitted as one `SpriteDraw` per horizontal run.
//! The runs reference a small ladder of solid 1×W strips (18 slots × 12
//! widths, ~200 files of ~120 bytes) that build() generates once into
//! `<pack root>/_pens/`. Both shells cache images by path, so the strips
//! are read once and every later frame is pure blitting. Emitting one
//! whole-screen png per frame instead would defeat that cache (it is keyed
//! by name and never invalidated) and churn 1.2 MB per tick.
//!
//! **The strips are GENERATED, not written (2026-09-19).** Until that date
//! build() wrote those ~200 files into `<pack root>/_pens/` and the sprites
//! named them by path, behind a guarded create-the-file-or-give-up. In
//! the dev tree the write succeeds; inside an installed `.saver` the bundle
//! is read-only, the write fails, and the whole pen writer was disabled —
//! the module fell back to the legacy series-3000 glyph stamper, i.e. the
//! installed screensaver quietly showed a DIFFERENT Bathroom Wall from the
//! one the dev tree showed. Toxic-swamp's mirrored frames were the same
//! class of bug ("the fish swim backwards"). The strips now travel as
//! `engine::GEN_PREFIX` names resolved by `Module::generated`, cached in
//! exactly the same `(name, pal)` slot a packed compound gets, so the
//! blit-once-read-many property above is unchanged and nothing writes into
//! the pack at all.
//!
//! ## SPEC ERRATA (binary wins — verified against
//! ## `Message Mayhem_CODE_129_DynaMessy.txt`)
//! 1. **Style values are 1-based menu marks, folded by `fn36`.** `0x2D66`
//!    subtracts 1 from the Style mark; only then is the result compared. `mVal 1001`
//!    default **1** therefore latches `[this+0x12A] = 0` → the `!= 1` arm →
//!    the **Aortal Squirt man scene is the factory default**, not the
//!    Bathroom Wall. The "Random" mark folds to 3 and rolls `rand(2)`
//!    (the range 2 is loaded at `0x2D7C`), i.e. 0 or 1 — never a third value. Spec §1.1
//!    left the mapping open (§10 Q1); the fold pins it.
//!    Same fold on `mVal 1002` (`0x2DA0`): default 3 → `[0x12C] = 2`.
//!    **CORRECTED 2026-09-13** — this line used to finish "→ STR# 500
//!    string 2 (Gone for the Day)" and that is one fold short: `fn34`
//!    subtracts **again** at `0x2AA6` before indexing STR# 500, so the
//!    string index is `mVal − 2` = 1 = **"Out to Lunch"**. See the
//!    CONTRADICTIONS note.
//! 2. **The write rect is re-rolled EVERY round, not only on round 0.**
//!    Spec §6 step 2 reads "if `this+0x132 == 0` … pick the write origin"
//!    and omits the else-arm. The `+0x132` test at `0x2778`, branching to `0x2816` when zero, in fact
//!    sends **round 0** to the random-box roll (`0x2816`: left/top =
//!    `rand(w/4)+10` / `rand(h/4)+10` via OffsetRect, right/bottom =
//!    `rand(w/4)+w/2-20` / `rand(h/4)+h/2-20`) and **round > 0** to a
//!    *different* roll at `0x2780` that snaps the box to the tile grid
//!    (`[this+0x12E] + 8` cell pitch at `0x27C6`, `[this+0x130] - 6` at
//!    `0x2802`) with right/bottom pinned to the cached screen size. Both
//!    arms fall into the same `fn23` call at `0x28A4`. Writing every
//!    message at one origin — the visible bug this fixes — was a misreading
//!    of that missing else-arm.
//! 3. **The wall IS washed, on a cycle boundary.** `0x2736` calls the
//!    module's own `vtable+0x04` = **DoBlank** and clears `[0x136]`, and
//!    DoBlank's `0x2664` sets `[this+0x132] = 0` while `0x242A..0x25F8`
//!    repaints the entire tile grid. §9.1's "wall never cleaned" holds
//!    *within* a cycle only. The cycle ends when
//!    `[0x114] + tier*10000 < now` **and** `round > tier` (`0x26F0`,
//!    `0x2700`); `[this+0x114]` is a completion timestamp (`0x2912`,
//!    `0x2638`), not an accumulator, so **tier*10000 ms is the pause
//!    between messages** and a wall carries **tier + 1** messages
//!    (`0x293A..0x294C`: `round <= tier` → `[0x136] = 0`, another round).
//!    `0xDBBA0` (900 000 ms = 15 min) gates a *toggle* of `[0x13A]`
//!    (`0x2722` seq/neg), which adds 9 to the pen index at `0x28A0`.
//! 4. **The engine clock is milliseconds, not 60 Hz Mac ticks.** Every
//!    deadline in this module and its siblings is compared against the same
//!    `Resource.fn_4724` value: 900 000 (15 min), tier*10 000 (10–30 s),
//!    1 000 (the sound rerun guard), 500+rand(250) (the squeak). Read as
//!    ticks those are 4.2 h / 2.8–8.3 min / 16.7 s / 8–12 s — absurd for a
//!    pen squeak. mime-hunt §6.4/§10 documents the identical call as ms
//!    (`* 0x03E8`). The previous 1000/60 conversion inflated every deadline
//!    16.7× and is why the module was effectively silent.
//! 5. **Tiles are mostly plain.** `0x2554..0x25C8`: each cell draws frame
//!    **1** unless `rand(7) == 5`, in which case it pops a *distinct*
//!    variant out of a 13-entry shuffle bag holding {1,3,…,27}
//!    (`0x242E..0x2450` builds it, `0x25B4..0x25C2` swaps the used entry to
//!    the tail and decrements the count). Rolling a random variant for
//!    every cell — the old behaviour — made every tile cracked.
//! 6. **§10 Q8 is answered, and its premise was wrong: the `Pens` writer
//!    does NOT run in the Aortal scene.** Q8 read the golden capture's
//!    aortal text as "big pen strokes behind the man". `full-reel.mp4`
//!    t=187–208 at 32-bit says otherwise — those letterforms are the
//!    **series-3000 blood-drip glyph bitmaps**, stamped one per beat as the
//!    artery sprays. Four independent checks, all measured on the reel
//!    resampled to the device 640×480 (screen area = image rows 56..1013,
//!    cols 2..1277, i.e. exactly 2×):
//!      a. *Letterform.* The reel's `M Y C H I L D ' S` are blobby,
//!         variable-width and carry drip tails. `assets/message-mayhem/
//!         compounds/3000/c_077.png` (= `M`) &c. are those glyphs,
//!         pixel-for-pixel. The wall's strokes at t=178 are uniform 1-px-
//!         cored pen paths — a different alphabet entirely.
//!      b. *Colour.* Packed 3000 art is a single flat `#aa0000`
//!         (170,0,0). Reel aortal ink measures **(145,0,1)** —
//!         170 × 0.853, exactly the reel's known ≈0.84× video darkening.
//!         Nothing in `clut 601` is near it: the blood *pen* ramp is
//!         `870d0d/943032/a2585a`, three shades, not one flat tone.
//!      c. *Metrics.* Cap height 25 px / 33 px cell, matching the pack's
//!         33–39 px glyph canvases at scale 1. Advancing by the compound
//!         **png width** (not the ink width) reproduces the reel's
//!         `MY CHILD'S` run to within 3 px over 258 px — the sidebearings
//!         are baked into the canvases.
//!      d. *The art/clut split (§4.5/§4.6) forbids the alternative.* Bank
//!         3000 is loaded only on the man arm; `clut 601 "Bathroom Pens"`
//!         only on the wall arm. The man scene has no pen ramp to draw
//!         with and the wall has no glyphs to stamp.
//!    The commit that built the real `Pens` writer (`0af2f94`) demoted the
//!    glyph stamping to a "LEGACY FALLBACK … the original has no glyph
//!    sprites" and pointed both branches at fn27. That is what put green
//!    `clut 601` strokes on the aortal black field. The glyph writer is
//!    restored here as the man scene's own renderer.
//! 7. **The struct's `message` default contradicted `controls()`.** Commit
//!    914990f moved the ControlDef default to popup index 1 ("Out to
//!    Lunch"), read straight off the live After Dark 3.0d panel, but left
//!    the field initialiser on the old fold-arithmetic guess of 2. Nothing
//!    that does not call `set_control(2, …)` — every headless render, the
//!    whole test suite — was writing "Gone for the Day". CONFIRMED
//!    2026-09-13 from the resource and the listing, independently of any
//!    panel frame — see the CONTRADICTIONS note.
//! 8. **The man walks, and his neck is the write cursor.** `full-reel.mp4`
//!    t=189–208 measured in device pixels: he enters at the left edge
//!    (grey bbox x = 0 at t=189), is at x≈187 when the first glyph lands
//!    (t≈194.5) and at x≈395 when the last does (t≈205) — **208 px of
//!    travel against 222 px of glyph advance over the same span**. That
//!    ratio is the whole mechanism: §8 state 1's `vtable+0x88(-75)` walk
//!    runs the *walk-in*, and state 2's `+0x898` walk offset then advances
//!    once per glyph, because the artery doing the writing is attached to
//!    a man who is walking away from what he wrote. Two consequences the
//!    2026-09-01 pass had backwards:
//!      a. *The walk-in is a loop, not a run.* The packed 1..11 stride
//!         lasts 1.3 s; the reel's walk-in lasts ≈5.5 s, i.e. four
//!         strides. `0x05CA` re-enters itself via `vtable+0x7C(15)` and
//!         only calls `SM.setState(2)` when fn64's walk test passes, so
//!         the completion test is a *position*, not a frame count.
//!      b. *The spray states loop until the message is finished.* The reel
//!         sprays continuously t≈194–206, 12 s against the 3.1 s the
//!         packed 36..49 + 53..64 runs last, and only folds once
//!         `MY CHILD'S` is complete. Playing them once meant the victim
//!         bled out after two glyphs and the screen was washed before
//!         anyone could read the message — which is what the 600-tick
//!         headless render was showing.
//! 9. **The space advances by its own canvas width, 9 px — not 12.**
//!    `mayhem-death.mp4` f1900 (the finished `OUT TO LUNCH`, frozen while
//!    the corpse lies there) has ink-left columns
//!    `142 168 198 | 240 270 | 306 330 360 390 420`. Rebuilding the pen from
//!    the packed canvas widths (O 27, U 28, T 32, L 25, N 31, C 30, H 32)
//!    hits every one of those to ±1 px if and only if ASCII 32 advances by
//!    the blank space compound's own 9 px width. ERRATUM 6c's "advance by
//!    the png width" is therefore literal — it covers the space too, and the
//!    hand-picked `SPACE_W = 12` was pushing the tail of every message right.
//! 10. **The man does not die where he wrote: there is a SECOND PASS.**
//!    `mayhem-death.mp4` cycle A (30 fps): the last glyph lands at f920 and
//!    he does not fold until f1630, 23.7 s later. In between he **keeps
//!    walking right** at his ordinary 4 px/frame, clean off the right edge
//!    (last pixel f1168, 250 px of walk-off), is gone for 126 frames, then
//!    **re-enters from the right one body-row lower** — pass 1 spans rows
//!    43..167, pass 2 rows 127..251, a flat 84 px drop — and staggers 244 px
//!    left (to x 392 at f1478) and 212 px back right before the collapse.
//!    He falls at x 507. The 2026-09-01 model folded him the instant the
//!    message finished, in place, on the writing row.
//! 11. **There is no blood-pool sprite. The pool is the corpse art.**
//!    The lying run's own blood pixel counts step
//!    `438 438 438 463 492 458 458 458 478 478 478 478 503 537 503 503`
//!    across compounds 95..110, and the pool in `mayhem-death.mp4` f1660..
//!    f1702 reproduces that ladder one frame for one frame before freezing
//!    dead for the remaining 10.4 s. So the pool grows because the run
//!    advances, is anchored under the corpse's neck by construction, and is
//!    finished at frame 110. The 117..149 "spatter/drip" compounds are a
//!    separate arterial jet the scene never draws here — at their packed
//!    origin (bx 425..536, by 55..68) the port was painting them 400 px away
//!    from the body, in mid-air.
//! 12. **The tier pause is timed from the corpse, not from the last glyph.**
//!    The panel in this capture reads Duration = *short* (thumb hard left)
//!    → tier 1 → a 10 000 ms pause. The tableau holds from f1702 to the
//!    blank at f2014 — **10.4 s** — against 36 s since the message finished.
//!    Then 49 frames (1.6 s) of blank screen while the next victim walks in
//!    from off the left edge, and the cycle repeats: 58.3 s end to end.
//!
//! ## BUGS DELIBERATELY KEPT (§11.8)
//! 1. **fn24 cannot parse a minus sign.** `0x4688`/`0x474C` seed the
//!    accumulator with `font[cursor] - '0'` and never test for `'-'`, so an
//!    authored `-5` parses as **−25** and `-1` as **−29**. Eight `[` groups
//!    are affected — the top-left serifs of `B D F P R`, `D`'s bottom tail
//!    and `Y`'s final point — and the result is the pronounced leftward
//!    tick those letters wear in the golden capture. `pens500.json` bakes
//!    the bug into each group's `points`; the authored intent sits beside it
//!    in `points_as_authored` and this module **never reads that key**.
//! 2. **The duplicate-point skip** (§11.1): when a group's next point equals
//!    the current one after scaling, fn27 (`0x5F84`) parses one more pair
//!    instead of drawing a zero-length segment, so `A V W Z` and the digit
//!    `4` draw their apparently separate legs as one unbroken pen path.
//!
//! ## APPROXIMATIONS
//! - **Wall ink RGB binding (spec §10 Q7) — now CONFIRMED, not a guess.**
//!   `full-reel.mp4` t=178.5 resampled to device pixels puts exactly three
//!   tones in the wall message: `(36,84,24)`, `(48,96,36)` and
//!   `(108,132,108)`, i.e. `clut 601` base-6 `285d1a / 3a692e / 798f76`
//!   read out literally, level 0 core and level 1/2 rim. So brush level
//!   *v* → `clut 601` slot `base + v` is right, the sub-table base offset
//!   is zero, and the 2026-08-30 "white core / 0x444346 rim" really was
//!   the broken 8-bit capture path. §11.7's derivation below stands.
//!   (For contrast, the aortal glyphs in the same reel are a single flat
//!   tone with no rim at all — see ERRATUM 6b. A three-level brush cannot
//!   produce that, which is the tell that they are not pen strokes.)
//! - **Ink RGB binding derivation (spec §11.7).** §11.7 pins the
//!   *derivation* of the base index — `[this+0x134] + ([this+0x13A] ? 9 : 0)`
//!   with `[0x134] = 3 × rand(3)` at depth ≥ 8 (`0x2644`, `0x28A0`) — and the
//!   6-pens × 3-shades layout of `clut 601 "Bathroom Pens 256"`, but not the
//!   sub-table's base offset in the live 256-entry device palette. We bind
//!   brush level *v* straight to `clut 601` slot `base + v`, read from the
//!   pack's own `palettes["601"]` (the JSON's `ink.ramps` is the fallback).
//!   That reproduces the traced *structure* — three ink levels, level 3
//!   transparent, darken-only accumulation, and the solid (dark-cored, bases
//!   0/3/6) versus hollow (light-cored, bases 9/12/15) alternation the
//!   `[0x13A]` toggle drives. The roll is genuine and it is the wall's
//!   alone: the reel's wall message is green (base 6), while other captures
//!   show solid dark-red (base 3) and hollow light-cored messages (the
//!   toggle's 9/12/15 half) on the same wall. The man scene never consults
//!   it — its ink is baked into the series-3000 art.
//! - **Runtime ink cells (§2 art note, fn71/GWorld) skipped**: the original
//!   also rendered two 17×17 bitmaps at runtime — 2000-art 123 (the "current
//!   word" cell near the man's head) and 3000-art 79 (four trailing ink
//!   cells per glyph) — via offscreen GWorlds + `RLE.fn_0656`. Sprites
//!   can't be rasterized engine-side, so the message exists exactly once:
//!   as pen ink on the wall, as glyph sprites in the man scene.
//! - **Vertical fit is not enforced.** fn23 (§11.3 step 2) measures the
//!   per-word scale against the box *width* only and word-wraps at the
//!   chosen scale; a message with more lines than the box is tall simply
//!   runs past its bottom edge, as the original did.
//! - **bVal 1003 "Edit Custom" dialog omitted** — the engine has no dialog
//!   plumbing (spec §10 Q5: likely engine-side anyway). Custom message
//!   falls back to a constant ("Wash me!"); the original read the
//!   "Messyges Custom" prefs file (fn35) and errored via STR# 1001.
//! - **Aortal SM states 1–8 (§8)** are choreographed over the packed 2000
//!   sequences in frame order; the 16 sub-sprites (fn62, §10 Q4 UNCERTAIN)
//!   are skipped. The walk (vtable+0x88) and the walk-complete test are now
//!   reproduced — see ERRATUM 8.
//! - **The Aortal scene's message text**: DoDrawFrame's `!= 1` arm pushes
//!   `this+4` (the text buffer) alongside the man object at
//!   `0x2988..0x2996` — the text IS pumped every frame there, but through
//!   the glyph stamper, not fn27 (ERRATUM 6). Placement, now measured off
//!   `mayhem-death.mp4` rather than guessed: one glyph every 1.4 s, left to
//!   right, from the **rolled** write origin, advancing by each compound's
//!   png width (the space included, ERRATUM 9), drawn *under* the man, who
//!   walks along it (ERRATUM 8). The man's absolute row is no longer the
//!   pack's `by` — it is `line_top + 17`, and `line_top` is the rolled box
//!   top, so the whole scene moves every cycle (capture: line_top 24 with
//!   the man at 43, then 29 / 48).
//! - **The aortal write origin's x roll is not pinned.** We reuse §6's
//!   round-0 `rand(w/4) + 10` (→ 10..169), but the three captures put it at
//!   138 (`mayhem-death` cycle A), ≈159 (`full-reel`) and **210**
//!   (`mayhem-death` cycle B). 210 is outside that range, so the aortal arm
//!   must roll wider than the wall's round-0 box. A capture with more cycles
//!   would pin it; the y roll (line_top 20 / 24 / 29) is equally unpinned.
//! - *(superseded 2026-09-26)* "The man is never mirrored" — he is: see
//!   "Flip — RESOLVED" above; the library's shared-part hand-off flips him.
//! - **The second pass is reproduced as measured distances, not mechanism.**
//!   ERRATUM 10's walk-out / drop / wander legs are driven by the capture's
//!   px figures (334 out, 328 left, 212 right). §8's states 6/7/8 almost
//!   certainly encode this differently — `vtable+0x88(-75)` is a *signed*
//!   walk speed, so the turn is probably a sign flip plus a row bump — but
//!   one capture cannot separate the two readings.
//! - fn34's STR# index (§1, UNCERTAIN) is resolved by the `- 1` fold at
//!   `0x2DA0`: popup index i (= raw − 1) is the 1-based STR# 500 index for
//!   1..10, 0 is Custom, 11 (raw 12..14 → mark 13) is Random.
//! - **Wall fallback** (`pens500.json` absent): the wall borrows the man
//!   scene's glyph stamper so a pack from the not-yet-updated packer still
//!   runs. *That* is unfaithful — the wall really is pen strokes — and it
//!   WARNs. The man scene's use of the stamper is not a fallback; it is
//!   the scene's actual renderer.
//!
//! ## ERRATA — tick-quantization sweep, 2026-09-12 (no code change)
//!
//! - **The flat 100 ms sprite frame is RE-CONFIRMED, and the Mac-tick grid
//!   does NOT apply here.** The sweep re-measured `mayhem-death.mp4`
//!   independently of the frame-number spot checks at `MAN_FRAME_MS`: over
//!   t = 70-95 s (206 screen-change events, crop 56 px + halve to module px)
//!   the Rayleigh periodogram peaks at **P = 99.82 ms with R = 0.591**, with
//!   R = 0.029 at the 7-tick 116.375 ms the `f4724` quantization would
//!   predict for a 100 ms delay. So `MAN_FRAME_MS = 100` stays exactly as it
//!   is and `tick_ms` stays at the 40 ms default (100 is an even 2.5 of it,
//!   and the deadline accumulates rather than restarting from `now`).
//! - **All three families are the same clock** (engine lane, 2026-09-12).
//!   `Resource.f4724()` is the TRUNCATED integer `TickCount()*16.625`
//!   (`(t<<4) + (t*0xA006>>16)` = 16, 33, 49, 66, 83, 99, 116 …), and on
//!   that grid a 100 ms delay gives: **100.0 ms** if the deadline
//!   ACCUMULATES (`due += 100` — here, shock-clocks, mime-hunt), **106.4 ms**
//!   if it is re-armed from `now` and tested `>=` (mikes, phlegm,
//!   toxic-swamp, chameleon), and **116.4 ms** if that compare is strict
//!   (frankenscreen). So this module keeps its flat 100 — but the reason is
//!   the accumulate, not an exemption from the Mac grid, and the 116.375
//!   figure above is what a *strict* gate predicts, not what `f4724`
//!   predicts for every 100 ms delay.
//!
//! ## CONTRADICTIONS settled 2026-09-13 — the shipped Message default
//!
//! ERRATUM 1 derived "Gone for the Day" from the fold; ERRATUM 7 read "Out
//! to Lunch" off a panel. **ERRATUM 7 is right, and the fold agrees with it
//! once the second subtraction is counted.** ERRATUM 1 was off by one.
//!
//! * `ripped/message-mayhem/Message Mayhem_mVal_1002_Message.bin` is two
//!   bytes, `00 03`. `MENU 1002` ships **14** items — 1 `Custom`,
//!   2 `-`, 3..12 the ten STR# 500 built-ins, 13 `-`, 14 `Random` — so mark
//!   3 *is literally the string* "Out to Lunch". (Same check on the Style
//!   popup: `mVal 1001` = 1, `MENU 1001` item 1 = `Aortal Squirt`, which is
//!   what ERRATUM 1 already concluded and what the capture's panel shows.)
//! * There are **two** `- 1` folds on the path, not one:
//!   `fn36 @0x2D9A-0x2DA6` stores the Message control value minus 1 at
//!   `+0x12C`, and then `fn34 @0x2A94-0x2AA8` takes that field and, when it
//!   is non-zero, subtracts 1 again before the `GetIndString(500, …)` at
//!   `@0x2AB4`. So the STR# 500 index is
//!   `mVal − 2` = **1** = "Out to Lunch". The second fold is what skips the
//!   `Custom` slot, and it is why the whole mapping closes: mark 12
//!   (American by Birth) ⇒ idx 10, mark 14 (Random) ⇒ `[0x12C]` = 13, the
//!   literal `@0x2A82` compares against; mark 1 and the mark-2 separator
//!   both fall into the `D3 == 0` custom arm at `@0x2ACC`.
//! * **Panel frames in these captures are live preferences, not defaults.**
//!   `emu/captures/message-mayhem.mp4` at t = 1 s has the setup pane open
//!   with Message = **REDRUM**, Style = Aortal Squirt, Duration = short.
//!   ERRATUM 7's "Out to Lunch" comes from a different (2026-08-30) panel
//!   read; it happens to be right, but a panel read is not evidence of a
//!   shipped default — mime-hunt's CyberMood fell into the same trap.
//!
//! No behaviour change: `message: 1` and `default: 1` were already correct.
//! Regression: `message_default_is_out_to_lunch_by_both_folds`.

use engine::l135::{self, FrameBox, LinkModel};
use engine::{ControlDef, ControlKind, Ctx, Image, Module, Pack, SpriteDraw, SCREEN_H, SCREEN_W};

// series bases (§2)
const S_TILE: u32 = 1000;
const S_MAN: u32 = 2000;
const S_FONT: u32 = 3000;

// snd resource ids (§2.2)
const SND_SQUEAK1: u32 = 1001;
const SND_SQUEAK2: u32 = 1002;
const SND_BODYFALL: u32 = 2000; // Aortal Bodyfall — event 92
const SND_OHH2: u32 = 2001; // Aortal ohh2 — event 71
const SND_SPRAY: u32 = 2002; // Aortal Spray 2 — events 36/53

// (§7.3, 0x5E8A/0x5E9C) squeak deadline: clock + 500 + random(250) ms.
// ERRATUM 4: the engine clock is ms, so this is a squeak every 0.5–0.75 s
// while the pen is moving — not the 8–12 s the tick reading implied.
const SQUEAK_BASE_MS: u64 = 500;
const SQUEAK_RAND_MS: u32 = 250;
// (§8, 0x04A4) sound rerun guard: clock + 1000 ms
const SND_GUARD_MS: u64 = 1000;

// Series-3000 blood-glyph layout (the Aortal writer; also the wall's
// fallback when pens500.json is missing). ERRATUM 6c: the compound canvases
// carry their own sidebearings — advancing by the png width with NO extra
// letter spacing reproduces the reel's `MY CHILD'S` run (device x 164..422,
// 258 px) to within 3 px, where advancing by the *ink* width plus a
// constant gap came out 60 px short. So there is no CHAR_SPACING.
const GLYPH_LINE_H: i32 = 40; // tallest 3000 canvas is 39 px
/// Fallback only — the real advance for ASCII 32 is the packed space
/// canvas's own width (9 px in the shipped pack), see `space_w`.
/// ERRATUM 9 (`mayhem-death.mp4`): measuring the finished `OUT TO LUNCH`
/// at f1900 gives ink-left columns 142/168/198/240/270/306/330/360/390/420
/// for `O U T T O L U N C H`; reconstructing the pen from the packed canvas
/// widths (O 27, U 28, T 32, L 25, N 31, C 30, H 32) reproduces every one of
/// them to ±1 px **only if the space advances by 9** — the space compound's
/// own canvas width. A 12 px space puts every glyph after the first space
/// 3 px right and after the second 6 px right.
const SPACE_W_FALLBACK: i32 = 9;
/// Per-glyph beat, measured. `mayhem-death.mp4` cycle A: the `O` of
/// `OUT TO LUNCH` lands at f460 and the `H` at f920 (30 fps) — 11 character
/// advances in 15.33 s, i.e. **1.4 s each**. The old 24-tick (960 ms)
/// figure came from eyeballing the reel and ran the artery 45 % fast.
const GLYPH_MS: u64 = 1400;

// (0x26F0/0x2920) per-message pause = tier × 10 000 ms
const ROUND_PAUSE_MS: u64 = 10_000;
// (0x2710) 0xDBBA0 = 900 000 ms: the endgame-flag toggle period
const ENDGAME_PERIOD_MS: u64 = 900_000;
// (0x28A0) endgame bumps the pen index by 9
const ENDGAME_PEN_BUMP: i32 = 9;

const MAX_INK: usize = 2000; // glyph-stamper safety cap

// STR# resource ids read via pack.strings (fn34, §7.1)
const STR_BUILTINS: u16 = 500;
const STR_GROUT: u16 = 501;

// §11.4: DoDrawFrame runs the sub-phase loop three times per frame (0x2756),
// so the writer lays down at most three dots per frame.
const DOTS_PER_FRAME: usize = 3;

/// The layout box `fn23` lays the message into — QuickDraw-atypical
/// {left, top, right, bottom} order, exactly the stack rect at `[A6-0x0C]`
/// in DoDrawFrame (`0x251A..0x2550` proves the field order).
#[derive(Clone, Copy)]
struct Box2 {
    l: i32,
    t: i32,
    r: i32,
    b: i32,
}

/// ERRATUM 2, round 0 (`0x2816..0x288C`): a box sized
/// `rand(w/4) + w/2 - 20` × `rand(h/4) + h/2 - 20` anchored at (0,0), then
/// `OffsetRect` by `(rand(w/4) + 10, rand(h/4) + 10)`.
fn roll_box_fresh(ctx: &mut Ctx) -> Box2 {
    let (w, h) = (SCREEN_W, SCREEN_H);
    let bw = ctx.rng.pct((w / 4) as u32) as i32 + w / 2 - 20;
    let bh = ctx.rng.pct((h / 4) as u32) as i32 + h / 2 - 20;
    let l = ctx.rng.pct((w / 4) as u32) as i32 + 10;
    let t = ctx.rng.pct((h / 4) as u32) as i32 + 10;
    Box2 { l, t, r: (l + bw).min(w - 8), b: (t + bh).min(h - 8) }
}

/// ERRATUM 2, grout rounds (`0x2780..0x2814`): the box is re-placed on the
/// tile grid — left off the `[this+0x12E] + 8` cell pitch, top off
/// `[this+0x130] - 6` — with right/bottom pinned to the cached screen size
/// (`0x27D2`, `0x280E`). Grout jokes land in the grout.
fn roll_box_grout(ctx: &mut Ctx, cell_w: i32, cell_h: i32) -> Box2 {
    let cw = cell_w.max(16);
    let ch = cell_h.max(16);
    let cols = ((SCREEN_W - cw) / cw).max(1) as u32;
    let rows = ((SCREEN_H - ch) / ch).max(1) as u32;
    let l = cw + 8 + cw * ctx.rng.pct(cols) as i32;
    let t = (ch - 6 + ch * ctx.rng.pct(rows) as i32).max(8);
    Box2 { l: l.min(SCREEN_W - 120), t: t.min(SCREEN_H - GLYPH_LINE_H - 8), r: SCREEN_W - 8, b: SCREEN_H - 8 }
}

/// The prose-era man constants (MAN_FRAME_MS / MAN_PEN_DX / MAN_ENTER_DX /
/// MAN_WALK_STEP / MAN_PASS_DROP / the wander legs / `aortal_box`) are gone.
/// None of them exists in the C: the man's cadence is `fn63 @0CF2`'s 0x5A
/// gate, his motion is the sequence links, and his row comes out of the
/// layout `fn66 @0DAA`, not a rolled box. See the Aortal Squirt section.


/// series-3000 compound frameNum == ASCII code; None = missing art → the
/// char is dropped as a space (§9.5, fn66 @0x0E06).
fn glyph_frame(c: u8) -> Option<u32> {
    let f = c as u32;
    const MISSING: [u32; 8] = [35, 37, 42, 43, 47, 60, 61, 62];
    if (32..=90).contains(&f) && !MISSING.contains(&f) {
        Some(f)
    } else {
        None
    }
}

/// 1-based string `idx` of a STR# list read from the pack, clamped to the
/// list's last entry (the original's lists are always full; a shorter one
/// repeats its tail), or `""` when the pack carries no such list — the
/// round then writes nothing rather than panicking.
fn str_item(list: &[String], idx: usize) -> String {
    list.get(idx.saturating_sub(1).min(list.len().saturating_sub(1))).cloned().unwrap_or_default()
}

/// fn35 custom message fallback (original: "Messyges Custom" prefs file).
const CUSTOM_FALLBACK: &str = "Wash me!";

// ===========================================================================
// A very small JSON reader.
//
// `pens500.json` has to be parsed at build() time and the `app` crate does
// not depend on serde (and this change may not touch Cargo.toml), so this is
// a ~90-line recursive-descent reader over exactly the value grammar the
// extraction emits. It is deliberately permissive: anything malformed yields
// `None` and the module falls back to the legacy glyph path.
// ===========================================================================

mod json {
    #[derive(Debug, Clone)]
    pub enum J {
        Null,
        Bool(bool),
        Num(f64),
        Str(String),
        Arr(Vec<J>),
        Obj(Vec<(String, J)>),
    }

    const NOTHING: &[J] = &[];
    const NO_PAIRS: &[(String, J)] = &[];

    impl J {
        pub fn get(&self, k: &str) -> Option<&J> {
            match self {
                J::Obj(v) => v.iter().find(|(n, _)| n == k).map(|(_, x)| x),
                _ => None,
            }
        }
        pub fn arr(&self) -> &[J] {
            match self {
                J::Arr(v) => v,
                _ => NOTHING,
            }
        }
        pub fn pairs(&self) -> &[(String, J)] {
            match self {
                J::Obj(v) => v,
                _ => NO_PAIRS,
            }
        }
        pub fn num(&self) -> f64 {
            match self {
                J::Num(n) => *n,
                _ => 0.0,
            }
        }
        pub fn int(&self) -> i32 {
            self.num() as i32
        }
        pub fn text(&self) -> &str {
            match self {
                J::Str(s) => s,
                _ => "",
            }
        }
        pub fn is_true(&self) -> bool {
            matches!(self, J::Bool(true))
        }
        /// `arr()[i]` without panicking on a short array.
        pub fn at(&self, i: usize) -> &J {
            self.arr().get(i).unwrap_or(&J::Null)
        }
    }

    pub fn parse(src: &str) -> Option<J> {
        let b: Vec<char> = src.chars().collect();
        let mut i = 0usize;
        let v = value(&b, &mut i)?;
        Some(v)
    }

    fn ws(b: &[char], i: &mut usize) {
        while *i < b.len() && b[*i].is_whitespace() {
            *i += 1;
        }
    }

    fn value(b: &[char], i: &mut usize) -> Option<J> {
        ws(b, i);
        match *b.get(*i)? {
            '{' => {
                *i += 1;
                let mut out = Vec::new();
                ws(b, i);
                if *b.get(*i)? == '}' {
                    *i += 1;
                    return Some(J::Obj(out));
                }
                loop {
                    ws(b, i);
                    let k = string(b, i)?;
                    ws(b, i);
                    if *b.get(*i)? != ':' {
                        return None;
                    }
                    *i += 1;
                    let v = value(b, i)?;
                    out.push((k, v));
                    ws(b, i);
                    match *b.get(*i)? {
                        ',' => *i += 1,
                        '}' => {
                            *i += 1;
                            return Some(J::Obj(out));
                        }
                        _ => return None,
                    }
                }
            }
            '[' => {
                *i += 1;
                let mut out = Vec::new();
                ws(b, i);
                if *b.get(*i)? == ']' {
                    *i += 1;
                    return Some(J::Arr(out));
                }
                loop {
                    out.push(value(b, i)?);
                    ws(b, i);
                    match *b.get(*i)? {
                        ',' => *i += 1,
                        ']' => {
                            *i += 1;
                            return Some(J::Arr(out));
                        }
                        _ => return None,
                    }
                }
            }
            '"' => Some(J::Str(string(b, i)?)),
            't' => lit(b, i, "true").then_some(J::Bool(true)),
            'f' => lit(b, i, "false").then_some(J::Bool(false)),
            'n' => lit(b, i, "null").then_some(J::Null),
            _ => {
                let start = *i;
                while *i < b.len() && "+-0123456789.eE".contains(b[*i]) {
                    *i += 1;
                }
                if *i == start {
                    return None;
                }
                b[start..*i].iter().collect::<String>().parse::<f64>().ok().map(J::Num)
            }
        }
    }

    fn lit(b: &[char], i: &mut usize, want: &str) -> bool {
        let n = want.chars().count();
        if b.len() < *i + n || b[*i..*i + n].iter().collect::<String>() != want {
            return false;
        }
        *i += n;
        true
    }

    fn string(b: &[char], i: &mut usize) -> Option<String> {
        ws(b, i);
        if *b.get(*i)? != '"' {
            return None;
        }
        *i += 1;
        let mut out = String::new();
        loop {
            let c = *b.get(*i)?;
            *i += 1;
            match c {
                '"' => return Some(out),
                '\\' => {
                    let e = *b.get(*i)?;
                    *i += 1;
                    out.push(match e {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'u' => {
                            let hex: String = b.get(*i..*i + 4)?.iter().collect();
                            *i += 4;
                            char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
                        }
                        other => other, // covers \" \\ \/
                    });
                }
                other => out.push(other),
            }
        }
    }
}

// ===========================================================================
// `Pens` 500 — the stroke font (§11.1) and its runtime tables
// ===========================================================================

/// Where the extraction lands inside a pack (see the header's asset note).
const PENS_JSON: &str = "pens500.json";
/// Name prefix of the solid ink strips the ink surface is drawn with.
/// These are GENERATED (`engine::GEN_PREFIX`), not packed and not written:
/// `Module::generated` below rebuilds a strip from its own name. Until
/// 2026-09-19 build() wrote ~200 PNGs into `<pack root>/_pens/` and named
/// them by path, which is exactly the read-only-bundle bug the header's
/// "Ink surface mechanism" note now describes.
const STRIP_DIR: &str = "gen:pen/";
/// Strip width ladder, descending — a run is covered greedily.
const STRIP_W: [i32; 12] = [64, 48, 32, 24, 16, 12, 8, 6, 4, 3, 2, 1];
/// `clut 601` has 18 live entries: six pens × three shades (§11.7).
const RAMP_SLOTS: usize = 18;
/// Ink-surface sentinel for "no ink here".
const NO_INK: u8 = 0xFF;

/// One `[...]` polyline or `{...}` arc run of a glyph. `pts` is the
/// bug-faithful point list (§11.8.1) — never `points_as_authored`.
struct PenGroup {
    arc: bool,
    /// `'-'` immediately after `'{'`: reverse-sweep flag, not a minus sign.
    reverse: bool,
    /// Arc centre in font units; y is measured DOWN from the top of the em
    /// box, the opposite of point y (§11.1).
    centre: (i32, i32),
    pts: Vec<(i32, i32)>,
}

struct PenGlyph {
    /// `<NNN>` advance width in font units (fn21/fn22).
    advance: i32,
    groups: Vec<PenGroup>,
}

struct PenFont {
    glyphs: Vec<(char, PenGlyph)>,
    /// `DATA 129` brush tables, index `bx*5 + by`, 3 = transparent (§11.6):
    /// 0 = scale < 0.15, 1 = 0.15..=0.7, 2 = scale > 0.7.
    brush: [[u8; 25]; 3],
    /// `clut 601` slot -> RGB (§11.7); see APPROXIMATIONS / §10 Q7.
    ramp: [[u8; 3]; RAMP_SLOTS],
}

impl PenFont {
    fn glyph(&self, c: char) -> Option<&PenGlyph> {
        self.glyphs.iter().find(|(k, _)| *k == c).map(|(_, g)| g)
    }

    /// fn22 (`0x3D84`): fold-then-look-up. Unknown characters return 0 and
    /// are dropped as spaces (§9.5).
    fn advance(&self, c: char) -> i32 {
        self.glyph(fold(c)).map(|g| g.advance).unwrap_or(0)
    }

    /// fn23 `0x45CE..0x4656`: brush table by scale.
    fn brush_for(&self, scale: f64) -> &[u8; 25] {
        if scale < 0.15 {
            &self.brush[0]
        } else if scale > 0.7 {
            &self.brush[2]
        } else {
            &self.brush[1]
        }
    }
}

/// fn22's `'a'..'z' → -0x20` uppercase fold (`0x5B02`).
fn fold(c: char) -> char {
    if c.is_ascii_lowercase() {
        (c as u8 - 0x20) as char
    } else {
        c
    }
}

/// SANE's round-to-nearest-EVEN extended→short conversion
/// (`Required1.fn_0336`), which is what every `round(...)` in §11 means.
fn rnd(v: f64) -> i32 {
    let fl = v.floor();
    if (v - fl - 0.5).abs() < f64::EPSILON {
        // exact .5 — pick the even neighbour
        let a = fl as i64;
        return (if a % 2 == 0 { a } else { a + 1 }) as i32;
    }
    v.round() as i32
}

/// fn25's atan with the explicit quadrant fixup (`0x4EDC..0x50D0`).
fn quad(dx: f64, dy: f64) -> f64 {
    let t = (dy.abs() / dx.abs()).atan();
    if dx < 0.0 {
        if dy < 0.0 {
            std::f64::consts::PI + t
        } else {
            std::f64::consts::PI - t
        }
    } else if dy < 0.0 {
        2.0 * std::f64::consts::PI - t
    } else {
        t
    }
}

fn hex_rgb(s: &str) -> Option<[u8; 3]> {
    if s.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&s[0..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..6], 16).ok()?,
    ])
}

/// Load `<pack root>/pens500.json` (§11.9). `None` → the legacy glyph path.
fn load_pen_font(pack: &Pack) -> Option<PenFont> {
    let raw = std::fs::read_to_string(pack.root().join(PENS_JSON)).ok()?;
    let j = json::parse(&raw)?;

    // --- the three DATA 129 brush tables, keyed by the extraction's
    // `0x????_scale_...` names.
    let brushes = j.get("brushes")?;
    let mut brush = [[3u8; 25]; 3];
    for (slot, frag) in ["lt_0.15", "0.15_to_0.7", "gt_0.7"].iter().enumerate() {
        let table = brushes.pairs().iter().find(|(k, _)| k.contains(frag)).map(|(_, v)| v)?;
        let cells = table.arr();
        if cells.len() < 25 {
            return None;
        }
        for n in 0..25 {
            brush[slot][n] = cells[n].int() as u8;
        }
    }

    // --- ink ramps. The pack's own clut 601 wins (it is the same resource,
    // already slot-addressed); the extraction's copy is the fallback.
    let mut ramp = [[0u8; 3]; RAMP_SLOTS];
    let mut got_ramp = false;
    if let Some(p601) = pack.meta.palettes.get("601") {
        if (0..RAMP_SLOTS).all(|s| p601.contains_key(&(s as u16))) {
            for s in 0..RAMP_SLOTS {
                ramp[s] = p601[&(s as u16)];
            }
            got_ramp = true;
        }
    }
    if !got_ramp {
        let ramps = j.get("ink")?.get("ramps")?;
        for (base, shades) in ramps.pairs() {
            let Ok(b) = base.parse::<usize>() else { continue };
            for (n, sh) in shades.arr().iter().enumerate() {
                if let (Some(rgb), true) = (hex_rgb(sh.text()), b + n < RAMP_SLOTS) {
                    ramp[b + n] = rgb;
                }
            }
        }
    }

    // --- the glyphs themselves
    let mut glyphs = Vec::new();
    for (key, gv) in j.get("glyphs")?.pairs() {
        let Some(ch) = key.chars().next() else { continue };
        let advance = gv.get("advance").map(|v| v.int()).unwrap_or(0);
        let mut groups = Vec::new();
        for grp in gv.get("groups").map(|v| v.arr()).unwrap_or(&[]) {
            let arc = grp.get("kind").map(|v| v.text() == "arc").unwrap_or(false);
            let reverse = grp.get("sweep_reverse").map(|v| v.is_true()).unwrap_or(false);
            let centre = grp
                .get("arc_centre")
                .map(|v| (v.at(0).int(), v.at(1).int()))
                .unwrap_or((0, 0));
            // BUG KEPT (§11.8.1): `points`, never `points_as_authored`.
            let pts: Vec<(i32, i32)> = grp
                .get("points")
                .map(|v| v.arr())
                .unwrap_or(&[])
                .iter()
                .map(|p| (p.at(0).int(), p.at(1).int()))
                .collect();
            groups.push(PenGroup { arc, reverse, centre, pts });
        }
        glyphs.push((ch, PenGlyph { advance, groups }));
    }
    if glyphs.is_empty() {
        return None;
    }
    Some(PenFont { glyphs, brush, ramp })
}

/// Generated-sprite name of the strip for (`clut 601` slot, width index).
/// The name carries both numbers because it is the only thing
/// [`MessageMayhem::generated`] gets back.
fn strip_path(slot: usize, wi: usize) -> String {
    format!("{STRIP_DIR}s{:02}_w{:02}", slot, STRIP_W[wi])
}

/// Rebuild one ink strip from its name: `gen:pen/sNN_wWW` is a 1-row run of
/// `WW` pixels of `clut 601` slot `NN`. Byte-for-byte what the old
/// `write_ink_strips` encoded into `_pens/sNN_wWW.png`.
fn strip_image(ramp: &[[u8; 3]; RAMP_SLOTS], name: &str) -> Option<Image> {
    let (slot, w) = name.strip_prefix(STRIP_DIR)?.split_once("_w")?;
    let slot: usize = slot.strip_prefix('s')?.parse().ok()?;
    let w: u32 = w.parse().ok()?;
    let c = *ramp.get(slot)?;
    let mut rgba = Vec::with_capacity(w as usize * 4);
    for _ in 0..w {
        rgba.extend_from_slice(&[c[0], c[1], c[2], 0xFF]);
    }
    Some(Image { w, h: 1, rgba })
}

// ===========================================================================
// The ink surface (fn19's canvas)
// ===========================================================================

/// A 640×480 buffer of `clut 601` slot indices, `NO_INK` where untouched —
/// the offscreen canvas fn19 stamps into, plus the row run-lengths the
/// module hands the shell as sprites.
struct InkSurface {
    px: Vec<u8>,
    dirty: bool,
    /// (strip-path index, x, y) per run; path index = slot*12 + width index.
    strips: Vec<(usize, i32, i32)>,
}

impl InkSurface {
    fn new() -> Self {
        InkSurface {
            px: vec![NO_INK; (SCREEN_W * SCREEN_H) as usize],
            dirty: false,
            strips: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.px.iter_mut().for_each(|p| *p = NO_INK);
        self.strips.clear();
        self.dirty = false;
    }

    /// fn19's inner pixel write (`0x3B26..0x3C52`) — the darken-only rule.
    ///
    /// * level 0 (the core) always paints;
    /// * a lighter level never overwrites a darker one;
    /// * a fringe landing on an equal fringe promotes one step darker, which
    ///   is what fills the body of a stroke in as the dots overlap.
    fn put(&mut self, x: i32, y: i32, base: usize, w: u8) {
        if x < 0 || y < 0 || x >= SCREEN_W || y >= SCREEN_H {
            return; // clip against (0,[A5+4]) x (0,[A5+6]) — 0x3B26
        }
        let i = (y * SCREEN_W + x) as usize;
        let cur = self.px[i];
        let lvl = if cur == NO_INK { None } else { Some(cur % 3) };
        let out: u8 = match (w, lvl) {
            (0, _) => 0,           // 0x3C52: the core always paints
            (_, Some(0)) => return, // already the darkest
            (1, Some(1)) => 0,     // fringe over fringe -> one step darker
            (1, Some(2)) => 1,     // never lighten
            (1, None) => 1,
            (2, Some(1)) => return, // 1 is already darker than 2
            (2, Some(2)) => 1,      // 0x3BE2: promote
            (2, None) => 2,
            _ => return,
        };
        let slot = (base + out as usize) as u8;
        if slot != cur {
            self.px[i] = slot;
            self.dirty = true;
        }
    }

    /// Run-length encode the surface into strip placements.
    fn rebuild(&mut self) {
        self.strips.clear();
        for y in 0..SCREEN_H {
            let row = &self.px[(y * SCREEN_W) as usize..((y + 1) * SCREEN_W) as usize];
            let mut x = 0i32;
            while x < SCREEN_W {
                let v = row[x as usize];
                if v == NO_INK {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < SCREEN_W && row[x as usize] == v {
                    x += 1;
                }
                let (mut px, mut run) = (start, x - start);
                while run > 0 {
                    let wi = STRIP_W.iter().position(|&w| w <= run).unwrap_or(STRIP_W.len() - 1);
                    self.strips.push((v as usize * STRIP_W.len() + wi, px, y));
                    px += STRIP_W[wi];
                    run -= STRIP_W[wi];
                }
            }
        }
        self.dirty = false;
    }
}

// ===========================================================================
// fn23/fn24/fn25/fn26/fn27 — the writer
// ===========================================================================

/// The `this+0x124` writer object (§11.2), minus the resource plumbing.
struct PenWriter {
    text: Vec<char>,     // +0x10, with fn23's '\r' wraps already injected
    ti: usize,           // +0x18 message cursor
    scale: f64,          // +0x540
    jitter: i32,         // +0x24
    jx: i32,             // +0x26
    jy: i32,             // +0x28
    pen_x: i32,          // +0x1E
    pen_y: i32,          // +0x20
    left: i32,           // +0x22
    slot_base: usize,    // +0x14 ink base palette index
    // font cursor, decomposed: which glyph / group / point we are on.
    ch: Option<char>,    // +0x1A == 0 means "between glyphs"
    gi: usize,
    pi: usize,
    new_stroke: bool,    // +0x30
    arc: bool,           // +0x2E inverted
    reverse: bool,       // +0x32
    p0: (i32, i32),      // +0x34
    p1: (i32, i32),      // +0x38
    cx: f64,             // +0x538
    cy: f64,             // +0x53C
    pts: Vec<(f64, f64)>, // +0x140, 127 entries of two singles
    dots_left: i32,      // +0x1C, counts down to -1
    complete: bool,
}

impl PenWriter {
    /// fn23 (`0x3DEA`): scale fit, word wrap, jitter range, pen origin.
    fn new(font: &PenFont, msg: &str, b: Box2, slot_base: usize) -> PenWriter {
        let chars: Vec<char> = msg.chars().collect();
        let box_w = (b.r - b.l).max(40) as f64;

        // §11.3 step 2: measure every word, take the SMALLEST per-word scale,
        // clamp 1.0 high (0x4308) and 0.1 low (0x438E).
        let mut scale = 1.0f64;
        let mut word = 0i32;
        for &c in chars.iter().chain(std::iter::once(&' ')) {
            if c == ' ' || c == '\r' || c == '\n' {
                if word > 0 {
                    scale = scale.min(box_w / word as f64);
                }
                word = 0;
            } else {
                word += font.advance(c);
            }
        }
        let scale = scale.clamp(0.1, 1.0);

        // §11.3 step 3: re-walk at the chosen scale, overwriting the
        // separating space with '\r' when a line would overflow (0x446C).
        let space_px = font.advance(' ') as f64 * scale;
        let mut text: Vec<char> = Vec::with_capacity(chars.len());
        let mut line = 0.0f64;
        let mut i = 0usize;
        while i < chars.len() {
            let start = i;
            while i < chars.len() && chars[i] != ' ' {
                i += 1;
            }
            let w: f64 = chars[start..i].iter().map(|&c| font.advance(c) as f64).sum::<f64>() * scale;
            if line > 0.0 && line + w > box_w {
                if text.last() == Some(&' ') {
                    text.pop();
                }
                text.push('\r');
                line = 0.0;
            }
            text.extend_from_slice(&chars[start..i]);
            line += w;
            if i < chars.len() {
                text.push(' ');
                line += space_px;
                i += 1;
            }
        }

        PenWriter {
            text,
            ti: 0,
            scale,
            // §11.3 step 4 (0x44C2, 0x451A)
            jitter: if scale < 0.15 { 1 } else { rnd(scale * 10.0).max(3) },
            jx: 0,
            jy: 0,
            pen_x: b.l,
            pen_y: rnd(scale * 50.0 + b.t as f64 - 100.0), // §11.3 step 5
            left: b.l,
            slot_base,
            ch: None,
            gi: 0,
            pi: 0,
            new_stroke: false,
            arc: false,
            reverse: false,
            p0: (0, 0),
            p1: (0, 0),
            cx: 0.0,
            cy: 0.0,
            pts: vec![(0.0, 0.0); 128],
            dots_left: -1,
            complete: false,
        }
    }

    /// `jitter/2 - rand(jitter)` (`0x5B60`, `0x5966`, `0x5A0A`).
    fn wobble(&self, ctx: &mut Ctx) -> i32 {
        self.jitter / 2 - ctx.rng.pct(self.jitter.max(1) as u32) as i32
    }

    /// fn24 (`0x4674`): font unit × scale, truncated. No sign handling — the
    /// bug that produces the serif ticks lives one level up, in the parsed
    /// values themselves (§11.8.1).
    fn sc(&self, v: i32) -> i32 {
        (v as f64 * self.scale) as i32
    }

    fn scaled(&self, p: (i32, i32)) -> (i32, i32) {
        (self.sc(p.0), self.sc(p.1))
    }

    /// fn26 (`0x58DE`): end of a glyph.
    fn advance_pen(&mut self, adv: i32, space: bool, ctx: &mut Ctx) {
        self.pen_x += rnd(adv as f64 * self.scale);
        if space {
            self.pen_x += self.wobble(ctx); // 0x5966 word-gap wobble
        }
        if self.scale < 0.15 {
            self.pen_x += 1; // 0x5986 rounding compensation
        }
    }

    /// fn25 (`0x483C`): fill the lookahead buffer BACKWARDS — `pts[n]` is the
    /// segment start, `pts[0]` the end — because fn27 counts `dots_left` down.
    fn interpolate(&mut self) {
        let dx = self.p1.0 - self.p0.0;
        let dy = self.p1.1 - self.p0.1;
        if !self.arc {
            // ---- polyline branch (0x484C)
            let n = (dx.abs().max(dy.abs()) - 1).clamp(2, 126);
            let (sx, sy) = (dx as f64 / n as f64, dy as f64 / n as f64);
            // y flip: 100 - P0.y  (0x49FC, 0x4A3E)
            self.pts[n as usize] =
                ((self.p0.0 + self.jx) as f64, (100 - self.p0.1 + self.jy) as f64);
            for i in (0..n as usize).rev() {
                self.pts[i] = (self.pts[i + 1].0 + sx, self.pts[i + 1].1 - sy);
            }
            self.dots_left = n;
        } else {
            // ---- arc branch (0x4B42): angle AND radius lerped, so the
            // segment is an Archimedean-spiral arc, not a circular one.
            let m = dx.abs().max(dy.abs());
            let n = (m + m / 5).clamp(2, 126); // +20% (0x4B5A..0x4BD6)
            let s100 = self.scale * 100.0;
            let (jx, jy) = (self.jx as f64, self.jy as f64);
            let x0 = self.p0.0 as f64 + jx;
            let y0 = s100 - self.p0.1 as f64 + jy;
            let x1 = self.p1.0 as f64 + jx;
            let y1 = s100 - self.p1.1 as f64 + jy;
            let mut adx = x0 - self.cx;
            if adx == 0.0 {
                adx = 0.001;
            }
            let mut bdx = x1 - self.cx;
            if bdx == 0.0 {
                bdx = 0.001;
            }
            let th0 = quad(adx, self.cy - y0);
            let mut th1 = quad(bdx, self.cy - y1);
            if self.reverse {
                if th1 > th0 {
                    th1 -= 2.0 * std::f64::consts::PI; // 0x5386
                }
            } else if th1 < th0 {
                th1 += 2.0 * std::f64::consts::PI; // 0x53F8
            }
            let dth = (th1 - th0) / n as f64;
            let r0 = (x0 - self.cx).hypot(y0 - self.cy);
            let r1 = (x1 - self.cx).hypot(y1 - self.cy);
            let dr = (r1 - r0) / n as f64;
            // the centre is neither flipped nor offset — which is exactly why
            // centre y reads as "down from the top of the em box" (§11.5).
            let yoff = 100.0 - s100;
            let (mut th, mut r) = (th0, r0);
            for i in (0..=n as usize).rev() {
                self.pts[i] = (self.cx + r * th.cos(), self.cy - r * th.sin() + yoff);
                th += dth;
                r += dr;
            }
            self.dots_left = n - 1; // 0x58A8: the start point is already inked
        }
    }

    /// fn19 (`0x38C4`): stamp the 5×5 (or 3×3) brush for one dot.
    fn stamp(&self, font: &PenFont, ink: &mut InkSurface, x: f64, y: f64) {
        let bx0 = rnd(x - 1.0) + self.pen_x; // 0x393C then 0x3AB6/0x3AEC
        let by0 = rnd(y - 1.0) + self.pen_y;
        let brush = font.brush_for(self.scale);
        let n: i32 = if self.scale >= 0.15 { 5 } else { 3 };
        for bx in 0..n {
            for by in 0..n {
                let w = brush[(bx * 5 + by) as usize]; // stride is ALWAYS 5
                if w == 3 {
                    continue; // 3 == transparent
                }
                ink.put(bx0 + bx, by0 + by, self.slot_base, w);
            }
        }
    }

    /// fn27 (`0x59D6`) — ONE dot per call. Returns true when the message is
    /// exhausted (fn27's return 1).
    fn step(
        &mut self,
        font: &PenFont,
        ink: &mut InkSurface,
        ctx: &mut Ctx,
        squeak_deadline: &mut u64,
    ) -> bool {
        if self.complete {
            return true;
        }

        // ---- between glyphs (font cursor == 0)
        while self.ch.is_none() {
            let Some(&raw) = self.text.get(self.ti) else {
                self.complete = true;
                return true; // 0x5AF0: '\0'
            };
            if raw == '\r' || raw == '\n' {
                // 0x5A06: x back to the left margin plus jitter, y down a line
                self.pen_x = self.left + self.wobble(ctx);
                self.pen_y += rnd(125.0 * self.scale);
                self.ti += 1;
                continue;
            }
            self.ti += 1;
            let c = fold(raw); // 0x5B1C
            let Some(g) = font.glyph(c) else { continue }; // fn20 -> 0: drop as space
            if g.groups.is_empty() {
                // 0x5B44: font[cursor] == '<' — a zero-stroke glyph, i.e. space
                let adv = g.advance;
                self.advance_pen(adv, c == ' ', ctx);
                continue;
            }
            self.ch = Some(c);
            self.gi = 0;
            self.pi = 0;
            self.new_stroke = true;
            self.jx = self.wobble(ctx); // 0x5B60
            self.jy = self.wobble(ctx);
            // 0x5B98 — overwritten by the arc centre for '{' groups
            self.cx = (g.advance / 2) as f64 * self.scale + self.jx as f64;
            self.cy = 50.0 * self.scale + self.jy as f64;
        }

        let ch = self.ch.expect("glyph selected above");
        let Some(glyph) = font.glyph(ch) else {
            self.ch = None;
            return false;
        };

        // ---- head of a stroke group (0x5C94)
        if self.new_stroke {
            // 0x5CA0: a squeak fires unconditionally at every stroke head,
            // re-arming the deadline WITHOUT the random part.
            ctx.sounds.push(if ctx.rng.pct(2) == 0 { SND_SQUEAK1 } else { SND_SQUEAK2 });
            *squeak_deadline = ctx.now_ms + SQUEAK_BASE_MS;
            self.new_stroke = false;
            let g = &glyph.groups[self.gi];
            self.arc = g.arc;
            if g.arc {
                // 0x5CF0: the '{' group's FIRST pair is the arc centre, and a
                // leading '-' was consumed by fn27 as a reverse-sweep flag.
                self.reverse = g.reverse;
                let c = self.scaled(g.centre);
                self.cx = (c.0 + self.jx) as f64;
                self.cy = (c.1 + self.jy) as f64;
            }
            if g.pts.len() < 2 {
                self.pi = g.pts.len();
                self.dots_left = -1;
            } else {
                self.p0 = self.scaled(g.pts[0]); // 0x5E16
                self.p1 = self.scaled(g.pts[1]);
                self.pi = 2;
                self.interpolate();
            }
        }

        // ---- lay one dot (0x5E54)
        if self.dots_left >= 0 {
            if ctx.now_ms > *squeak_deadline {
                ctx.sounds.push(if ctx.rng.pct(2) == 0 { SND_SQUEAK1 } else { SND_SQUEAK2 });
                *squeak_deadline =
                    ctx.now_ms + SQUEAK_BASE_MS + ctx.rng.pct(SQUEAK_RAND_MS) as u64;
            }
            let p = self.pts[self.dots_left as usize];
            self.stamp(font, ink, p.0, p.1);
            self.dots_left -= 1;
            return false;
        }

        // ---- advance the parser (0x5EF6)
        let g = &glyph.groups[self.gi];
        if self.pi >= g.pts.len() {
            // the group's ']' / '}'
            self.gi += 1;
            if self.gi >= glyph.groups.len() {
                // font[cursor] == '<' -> the glyph is finished. NB fn26 is
                // called with ']' / '}' here, never a space, so the word-gap
                // wobble cannot fire on this path.
                let adv = glyph.advance;
                self.advance_pen(adv, false, ctx);
                self.ch = None;
            } else {
                self.new_stroke = true;
            }
        } else {
            // another point in the same group (0x5F5C)
            self.p0 = self.p1;
            self.p1 = self.scaled(g.pts[self.pi]);
            self.pi += 1;
            // BUG KEPT (§11.8.2): a duplicate point consumes one more pair
            // rather than drawing a zero-length segment.
            if self.p0 == self.p1 && self.pi < g.pts.len() {
                self.p1 = self.scaled(g.pts[self.pi]);
                self.pi += 1;
            }
            self.interpolate();
        }
        false
    }
}

// --- Aortal Squirt man SM (§8) --------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Pick, // this+0x136 == 0: choose text (fn34) and (re)start
    Writing,
    /// `[this+0x138] != 0` — the write finished; DoDrawFrame idles here
    /// until `[0x114] + tier*10000 < now` (0x2920).
    Done,
}

/// One stamped series-3000 blood-drip glyph (the Aortal writer's output).
#[derive(Clone, Copy)]
struct Placed {
    fno: u32,
    x: i32,
    y: i32,
}

// === Aortal Squirt — THE MAN ===============================================
//
// Ported function-by-function from the decompile. Objects, from the
// controller ctor `fn30 @1DE2` and the man's own init `fn60 @01DE`:
//
// | object | ctor | handler | what |
// |---|---|---|---|
// | the man | `fn57 @head` | **`fn61 @0420`** | 8 states, size 0x8A2, owns the rest |
// | neck spurt | `fn72 @1706` | `fn74 @1770` | loops run 117 pinned to the man |
// | 16 blood drips | `fn75 @1980` | `fn78 @1A0E` | run 139 down an already-written glyph |
// | seq bank 2000 | `fn68 @15A4` + `fn70 @15E0(123)` | draw `fn71 @1610` | the man's art |
// | seq bank 3000 | `fn68 @15A4` + `fn70 @15E0(79)` | — | the glyph font, `frameNum == ASCII` |
//
// `fn70 @15E0`'s second argument is the **art id the draw skips** (`fn71
// @1610` compares `part.art` against `+0x11E`): 123 in bank 2000 and 79 in
// bank 3000 are invisible registration markers. The packed compounds already
// omit them — compound 117 is 17×17 and fully transparent — but their part
// rects are what `fn74` pins the spurt with.
//
// Frame driver: `fn33 @2692` runs the man's state machine behind
// `fn63 @0CF2` (`due = now + 0x5A`, re-armed from `now`, tested `>=`), and
// every live drip behind `fn79 @1C6E` (the same 0x5A). On the Mac tick grid
// a 90 ms re-armed gate lands on ~99.8 ms, which is the period the capture
// measured — so the module moved onto `TickClock::MacTick` (§2) and the flat
// `MAN_FRAME_MS = 100` is gone.

/// `fn63 @0CF2` / `fn79 @1C6E`: `+0x170`/`+0x124 = now + 0x5A`.
const GATE_MS: u64 = 90;

// Runs of bank 2000, all of them OFst block starts (§10.1: ids are frameNums
// as written). `fn61`'s SetRun / SetRunList arguments, verbatim.
const RUN_WALK_L: i32 = 0x01; // 1..11,   net centre −45 px
const RUN_WALK_R: i32 = 0x0F; // 15..25,  net +45
const RUN_TURN: i32 = 0x1C; //   28..31,  net +16
const RUN_SPRAY_A: i32 = 0x24; // 36..49,  net +33, jet art in 39..45
const RUN_SPRAY_B: i32 = 0x35; // 53..64,  net +41, jet art in 57..63
const RUN_STAGGER: i32 = 0x47; // 71..75
const RUN_RECOVER: i32 = 0x50; // 80..83
const RUN_FALL: i32 = 0x58; //   88..110
const RUN_CORPSE: i32 = 0x70; // 112..113
const RUN_SPURT: i32 = 0x75; // 117..130, the neck spurt's own loop
const RUN_DRIP: i32 = 0x8B; //  139..149, one blood drip falling

/// `fn67 @132C` reads the position of the part whose **art** is 87 — the
/// blood jet's tip, which exists only in frames 39..45 and 57..63. Its right
/// edge is the write cursor's x and its bottom the cursor's y.
const JET_ART: i32 = 87;
/// Channel of the invisible registration marker `fn74 @1902` pins on.
const MARK_CH: i32 = 1;
/// `fn60 @02A8`: sixteen drip sprites.
const N_DRIPS: usize = 16;
/// `fn61 @055C`: a fresh victim is placed here and walks right.
const MAN_ENTER_X: i32 = -75;
/// `fn61 @0830` / `@09EE`: re-entry from the right, `g0620 + 0x19 / + 0x3C`.
const MAN_RETURN_X: i32 = 0x19;
const MAN_DEATH_X: i32 = 0x3C;
/// `fn61 @0B86`: everything is parked here when the tableau ends.
const PARK: i32 = -200;
/// `fn60 @0254`: the line pitch is the height of glyph `'p'` plus ten.
const LINE_PAD: i32 = 10;
/// `fn66 @0DAA` word table cap and the per-line left margin it adds.
const MAX_WORDS: usize = 0x40;
const LINE_MARGIN: i32 = 0x32;
/// `fn66 @1206`: the row is `RandomBelow(slack) + 100`.
const ROW_BASE: i32 = 100;
/// `fn66 @0DC2`: the first line wrap is short by this much.
const FIRST_WRAP_FUDGE: i32 = 14;

// --------------------------------------------------------------------------
// The Library 4.0 frame stepper the three classes share
// (L132 `fn0316 @0316` advance, `fn028A @028A` SetRun, `fn0240 @0240` the
// `-1`-terminated run list, `fn04C8 @04C8` SetPos).
// --------------------------------------------------------------------------

/// The OFst block containing `f`, as `(first, last)` — §10.1: resolve an id
/// to the block that contains it and enter the run AT that id.
fn run_span(pack: &Pack, base: u32, f: i32) -> (i32, i32) {
    for s in pack.series(base) {
        let first = s.first as i32;
        let last = first + s.frames.len() as i32 - 1;
        if f >= first && f <= last {
            return (first, last);
        }
    }
    (f, f)
}

/// L135 `fn3A70`: `centre(f) = (bx+dx+w/2, by+dy+h/2)` in bank space.
#[cfg(test)]
fn centre(pack: &Pack, base: u32, f: i32) -> Option<(i32, i32)> {
    if f <= 0 {
        return None;
    }
    pack.frame(base, f as u32).map(|g| FrameBox::of(g).centre())
}

fn part_by_art(pack: &Pack, base: u32, f: i32, art: i32) -> Option<[i32; 7]> {
    let g = pack.frame(base, f.max(0) as u32)?;
    g.parts.iter().find(|p| p[0] == art).copied()
}

fn part_by_chan(pack: &Pack, base: u32, f: i32, ch: i32) -> Option<[i32; 7]> {
    let g = pack.frame(base, f.max(0) as u32)?;
    g.parts.iter().find(|p| p[1] == ch).copied()
}

/// The frame's L135 geometry: the pack record at `f` (clamped to 0).
fn frame_box(pack: &Pack, base: u32, f: i32) -> Option<FrameBox> {
    pack.frame(base, f.max(0) as u32).map(FrameBox::of)
}

/// The bank rect of a frame, `{l, t, r, b}` — what the sequence's `+0x18`
/// hands back (`fn60 @026A`, `fn65 @0D6A`, `fn67 @140E`).
fn bank_rect(pack: &Pack, base: u32, f: i32) -> Option<[i32; 4]> {
    frame_box(pack, base, f).map(|g| g.rect())
}

/// L135 `fn3BD6 @3BD6`'s part layout relative to the frame centre
/// ([`FrameBox::part_rel`]).
fn part_rel(pack: &Pack, base: u32, f: i32, part: &[i32; 7], flip: bool) -> Option<[i32; 4]> {
    Some(frame_box(pack, base, f)?.part_rel(part, flip))
}

/// L135 `fn3DDC @3DDC`, the in-run link `from → to` ([`l135::link`], the
/// transcribed flip-bit rounding). No move when either record is missing.
fn link(pack: &Pack, base: u32, from: i32, to: i32, flip: bool) -> Option<(i32, i32)> {
    Some(l135::link(&frame_box(pack, base, from)?, &frame_box(pack, base, to)?, flip, LinkModel::FN3DDC))
}

/// L135 **`fn3F2E @3F2E`** ([`l135::register`]), the shared-part
/// registration a hand-off links through (`fn0D3C @0D3C`). No record or no
/// common part: no move, no flip change. Returns `(dx, dy, flip')`.
fn shared_link(pack: &Pack, base: u32, cur: i32, new: i32, flip: bool) -> (i32, i32, bool) {
    let (Some(ga), Some(gb)) = (pack.frame(base, cur.max(0) as u32), pack.frame(base, new.max(0) as u32))
    else {
        return (0, 0, flip);
    };
    match l135::register(&FrameBox::of(ga), &ga.parts, &FrameBox::of(gb), &gb.parts, flip) {
        Some(r) => (r.delta.0, r.delta.1, r.flip),
        None => (0, 0, flip),
    }
}

#[derive(Clone)]
struct Spr {
    /// `+0x40/+0x42` — the CENTRE of the current frame (§2).
    x: i32,
    y: i32,
    frame: i32, // +0x3A
    run: i32,   // +0x44
    /// The tail of a `fn0240 @0240` run list.
    queue: std::collections::VecDeque<i32>,
    /// `+0x4E` — the run this tick handed over to; it starts next tick, so
    /// the finished run's last frame is on screen for exactly one tick.
    pending: Option<i32>,
    fresh: bool, // +0x48: a SetRun landed, do not advance this tick
    done: bool,  // +0x46: the run list ran out on this tick
    flip: bool,  // +0x3C
    shown: bool,
}

impl Spr {
    fn new() -> Spr {
        Spr {
            x: 0,
            y: 0,
            frame: 0,
            run: 0,
            queue: std::collections::VecDeque::new(),
            pending: None,
            fresh: false,
            done: false,
            flip: false,
            shown: true,
        }
    }

    /// L132 `fn04C8 @04C8`: store the centre and re-place the sprite.
    fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// `+0xD0` = L132 `fn0DF4 @0DF4` inside a run: step to `f`, moving `pos`
    /// by the sequence's `+0x7C` → `+0x78` = L135 `fn3DDC @3DDC` link. This
    /// is the only thing that moves a sprite within a run.
    fn advance(&mut self, pack: &Pack, base: u32, f: i32) {
        if self.frame != 0 {
            if let Some((dx, dy)) = link(pack, base, self.frame, f, self.flip) {
                self.x += dx;
                self.y += dy;
            }
        }
        self.frame = f;
    }

    /// `+0x70` = L132 `fn01E2 @01E2`: seat a frame outright — no link.
    fn set_frame(&mut self, f: i32) {
        self.frame = f;
    }

    /// `+0x108` = L132 **`fn028A @028A`**, the run hand-off. Two links:
    ///
    /// 1. `+0xF0` = `fn1186 @1186` maps the run id to the frame to pass
    ///    through — `id − 1` when that record exists (`+0x82` is 1 after
    ///    the `fn0058` reset). Bank 2000 has no such lead-in records (every
    ///    run id this module names is a block start with a hole before
    ///    it), so here the marker is the run's own first frame.
    /// 2. `+0xCC` = `fn0D3C @0D3C` links the CURRENT frame to it through
    ///    L135 **`fn3F2E @3F2E`**: the first part id the two part tables
    ///    share keeps its screen position, AND the sprite's flip bit is
    ///    XORed with the difference of the two parts' own flip flags
    ///    (`+0xE` of the part record) — `+0x80` is 1 after the reset, so
    ///    this runs on every hand-off.
    /// 3. `+0xD0` = `fn0DF4` then links marker → first by `fn3DDC` (0 here).
    ///
    /// Step 2 is the whole letter cycle: spray run 53 ends on frame 64,
    /// whose body parts (31, 8) are authored mirrored; run 1 carries them
    /// unmirrored, so the hand-off flips the man and he walks run 1 —
    /// the "walk left" run — mirrored, i.e. forward, keeping his feet
    /// where they were. The capture shows exactly that (mirrored frames
    /// 1..6 and 38/39 after every spray B). Applying no delta instead
    /// slid the feet +33 px right on every letter (Jason, 2026-09-26:
    /// "jerky at the end of writing a letter").
    fn start_run(&mut self, pack: &Pack, base: u32, r: i32) {
        if self.frame != 0 {
            let marker = l135::marker_of(r, |f| f >= 1 && pack.frame(base, f as u32).is_some());
            let (dx, dy, flip) = shared_link(pack, base, self.frame, marker, self.flip);
            self.x += dx;
            self.y += dy;
            self.flip = flip;
            self.frame = marker;
            self.advance(pack, base, r);
        }
        self.frame = r;
        self.run = r;
        self.fresh = true;
        self.pending = None;
    }

    /// `+0x7C` = L132 `fn0204 @0204`: clear the queue, `fn028A`.
    fn set_run(&mut self, pack: &Pack, base: u32, r: i32) {
        self.queue.clear();
        self.start_run(pack, base, r);
    }

    /// `+0x80` = `fn0240 @0240`: `+0x7C` the first id, queue the rest.
    fn set_runs(&mut self, pack: &Pack, base: u32, rs: &[i32]) {
        self.set_run(pack, base, rs[0]);
        self.queue.extend(rs[1..].iter().copied());
    }

    /// L132 `fn0316 @0316`. Frames run `first..=last` inclusive; the tick
    /// that lands on `last` is also the tick that reports the run finished,
    /// so the handler always sees the last frame drawn (§7.1's link marker
    /// trap does not apply — this stepper wraps on the value it just set).
    fn tick(&mut self, pack: &Pack, base: u32) {
        self.done = false;
        // `+0x4E`: the run the last tick dequeued starts now, through the
        // same linked `fn028A` hand-off as an immediate `+0x7C`.
        if let Some(r) = self.pending.take() {
            self.start_run(pack, base, r);
        }
        let (first, last) = run_span(pack, base, self.run);
        if self.fresh {
            self.fresh = false;
        } else if self.frame >= first && self.frame < last {
            let n = self.frame + 1;
            self.advance(pack, base, n);
        }
        if self.frame >= last || self.frame < first {
            match self.queue.pop_front() {
                Some(r) => self.pending = Some(r),
                None => self.done = true,
            }
        }
    }

    /// Screen top-left of the current frame: `pos − (w>>1, h>>1)`.
    fn draw_at(&self, pack: &Pack, base: u32) -> Option<(i32, i32)> {
        let g = pack.frame(base, self.frame.max(0) as u32)?;
        Some((self.x - (g.w >> 1), self.y - (g.h >> 1)))
    }

    /// A part's rect on the screen, `{l, t, r, b}`, mirrored with the
    /// sprite's flip the way L135 `fn3BD6 @3BD6` lays a compound out.
    fn part_screen(&self, pack: &Pack, base: u32, part: &[i32; 7]) -> Option<[i32; 4]> {
        let r = part_rel(pack, base, self.frame, part, self.flip)?;
        Some([r[0] + self.x, r[1] + self.y, r[2] + self.x, r[3] + self.y])
    }

    /// `+0x24`: the sprite's screen rect, `{l, t, r, b}`.
    fn screen_rect(&self, pack: &Pack, base: u32) -> Option<[i32; 4]> {
        let g = pack.frame(base, self.frame.max(0) as u32)?;
        let (x, y) = (self.x - (g.w >> 1), self.y - (g.h >> 1));
        Some([x, y, x + g.w, y + g.h])
    }
}

/// `fn64 @0D2A`: 0 while any of the sprite is still within the field.
fn off_screen(s: &Spr, pack: &Pack, base: u32) -> bool {
    match s.screen_rect(pack, base) {
        Some(r) => !(r[0] < SCREEN_W && r[2] > 0),
        None => true,
    }
}

/// `+0x34` (L135 `fn51BA`): shown AND still on the field.
fn sprite_live(s: &Spr, pack: &Pack, base: u32) -> bool {
    s.shown && !off_screen(s, pack, base)
}

// --------------------------------------------------------------------------

/// The man: `fn61 @0420` plus the layout `fn66 @0DAA` writes into him.
#[derive(Clone)]
struct Man {
    s: Spr,
    state: i32,   // state sub-object +4
    prev: i32,    // +6
    entered: bool, // +0xE
    msg: Vec<u8>,     // +0x89A
    char_x: Vec<i32>, // +0x194
    char_y: Vec<i32>, // +0x394
    drawn: Vec<bool>, // +0x594
    ci: usize,        // +0x894, the character the artery is on
    line_h: i32,      // +0x190
    row: i32,         // +0x18C, the man's centre row
    fudge: i32,       // +0x898
    armed: bool,      // +0x8A0
    gag: bool,        // +0x89E
    drips: usize,     // +0x192
    due: u64,         // +0x170
    snd_gate: u64,    // +0x17C
    rest_at: u64,     // +0x174
    hold_ms: u64,     // +0x178
    pause_until: u64, // +0x180
}

impl Man {
    fn new() -> Man {
        Man {
            s: Spr::new(),
            state: 0,
            prev: 0,
            entered: true,
            msg: Vec::new(),
            char_x: Vec::new(),
            char_y: Vec::new(),
            drawn: Vec::new(),
            ci: 0,
            line_h: 40,
            row: ROW_BASE,
            fudge: FIRST_WRAP_FUDGE,
            armed: false,
            gag: false,
            drips: 0,
            due: 0,
            snd_gate: 0,
            rest_at: 0,
            hold_ms: ROUND_PAUSE_MS,
            pause_until: 0,
        }
    }
    fn stop_x(&self) -> i32 {
        self.char_x.get(self.ci).copied().unwrap_or(0)
    }
}

/// The neck spurt and each blood drip: a sprite with its own two-state
/// machine (`fn74 @1770`, `fn78 @1A0E`).
#[derive(Clone)]
struct Minor {
    s: Spr,
    state: i32,
    entered: bool,
    due: u64, // the drip's +0x124
}

impl Minor {
    fn new() -> Minor {
        Minor { s: Spr::new(), state: 0, entered: true, due: 0 }
    }
}

// ---------------------------------------------------------------------------

pub struct MessageMayhem {
    pack: Pack,

    // raw controls (§1; fn36 @0x2D24 latches these)
    duration: i32, // sVal 1000, 0..100, default 50
    style: i32,    // mVal 1001: 0 Aortal, 1 Bathroom Wall, 2 Random(=menu 3)
    message: i32,  // mVal 1002: 0 custom, 1..10 built-ins, 11 random(=menu 13)

    // latched (fn36)
    tier: i32,           // this+0x128: <34→1, <67→2, else 3
    style_resolved: i32, // this+0x12A: 1 = wall-writer, else = man SM
    latched: bool,

    // wall grid (DoBlank §5 branch A)
    tiles: Option<Vec<(u32, i32, i32)>>,
    cell: (i32, i32), // this+0x12E/0x130: the tile cell pitch (0x2620/0x262C)

    // writer scene (§6/§7/§11)
    phase: Phase,
    round: i32,        // this+0x132
    endgame: bool,     // this+0x13A
    done_ms: u64,      // this+0x114: timestamp of the last completed write
    endgame_ms: u64,   // this+0x118: last endgame toggle (0x2732)
    pen: i32,          // this+0x134 pen index (+9 in endgame, 0x28A0)
    bounds: Box2,      // the fn23 layout box (ERRATUM 2)
    squeak_deadline_ms: u64, // obj+0x3C (§7.3)
    startup_squeak: bool,    // §4 step 9: init-time squeak
    /// DoBlank's `0x2644` pen roll happens in BOTH style branches (§5); the
    /// wall re-rolls it every cycle, the man scene only ever gets this one.
    blanked: bool,

    /// `Pens` 500 (§11), the Bathroom Wall writer. `None` → the wall falls
    /// back to the glyph stamper (WARN).
    pens: Option<PenFont>,
    /// Path table for the generated ink strips, `slot*12 + width index`.
    strip_paths: Vec<String>,
    ink_px: InkSurface,
    writer: Option<PenWriter>,

    // --- series-3000 blood-glyph stamper state (Aortal; wall fallback) ----
    text: Vec<char>,
    text_idx: usize,
    pen_x: i32,
    line_top: i32,
    word_begin: Option<(usize, i32)>,
    ink: Vec<Placed>,
    /// Wall clock at which the next glyph is stamped (GLYPH_MS apart).
    next_glyph_ms: u64,
    /// ASCII 32's advance, read from the packed space canvas (ERRATUM 9).
    space_w: i32,
    /// series-3000 glyph metrics: (frameNum, advance = the compound's png
    /// width — the canvas carries its own sidebearings, ERRATUM 6c)
    font: Vec<(u32, i32)>,

    // STR# 500/501 via pack.strings (fn34, §7.1); constants as fallback
    builtins: Vec<String>,
    grout: Vec<String>,

    // --- Aortal Squirt (fn61 @0420 and the two sprite classes it owns) ---
    man: Man,
    /// `fn72 @1706`'s object: the spurt pinned to the man's neck marker.
    spurt: Minor,
    /// `fn75 @1980` × 16 — `fn62 @0C74` runs the first `man.drips` of them.
    drips: Vec<Minor>,
    /// The controller's `+0x136`: cleared when the man reaches state 8, which
    /// is what makes `fn33 @2692` re-lay the message and wash the screen.
    man_running: bool,
    /// The wall runs off `Ctx::now_ms` at the 40 ms beat it was verified on;
    /// the module itself is on `MacTick` for the man's 90 ms gate.
    wall_due: u64,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    make_concrete(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

fn make_concrete(pack: Pack) -> Option<MessageMayhem> {
    // the module needs all three series (§2)
    for base in [S_TILE, S_MAN, S_FONT] {
        if !pack.meta.series.contains_key(&base.to_string()) {
            return None;
        }
    }

    // §11: the pen font, and the solid ink strips its surface is drawn with.
    // WARN: a pack built before tools/pack_assets.py learns to emit
    // pens500.json has neither, and falls through to the legacy glyph path.
    let pens = load_pen_font(&pack);
    let mut strip_paths = Vec::new();
    if pens.is_some() {
        for slot in 0..RAMP_SLOTS {
            for wi in 0..STRIP_W.len() {
                strip_paths.push(strip_path(slot, wi));
            }
        }
    } else {
        eprintln!(
            "message-mayhem: WARN no {PENS_JSON} in the pack — falling back to the \
             legacy series-3000 glyph path (see the module header: pack_assets.py \
             owes a pens_extract.py step)"
        );
    }

    // Glyph metrics for every existing series-3000 compound. ERRATUM 6c:
    // the advance is the compound's full png WIDTH — the canvases carry
    // their own sidebearings — so each glyph is stamped at the raw pen x
    // with no bbox correction.
    // The man scene ALWAYS needs these, so they are no longer built only on
    // the pens-missing path (which is what made the Aortal scene unable to
    // draw anything but pen strokes).
    let mut font = Vec::new();
    for fno in 32..=90u32 {
        if glyph_frame(fno as u8).is_none() {
            continue;
        }
        let Some(f) = pack.frame(S_FONT, fno) else { continue };
        let img = pack.image(&f.png);
        font.push((fno, img.w as i32));
    }

    // The man scene has no other renderer; a pack without bank 3000 art is
    // unplayable rather than merely degraded.
    if font.is_empty() {
        eprintln!("message-mayhem: no series-3000 glyph art in the pack");
        return None;
    }
    // ERRATUM 9: the space advances by its own (blank) canvas width.
    let space_w = font.iter().find(|g| g.0 == 32).map(|g| g.1).unwrap_or(SPACE_W_FALLBACK);

    // STR# 500 "Built in Messages" (1-based idx 1..10) and STR# 501 "Grout
    // jokes" (idx 1..7, §9.2), both from the user's pack.
    let builtins = pack.strings(STR_BUILTINS).to_vec();
    let grout = pack.strings(STR_GROUT).to_vec();

    Some(MessageMayhem {
        pack,
        duration: 50,
        // ERRATUM 1: fn36 folds the menu mark with `- 1`, so mVal 1001's
        // default mark 1 lands on popup index 0 = Aortal Squirt.
        style: 0,
        // ERRATUM 7: the message default must match `controls()` — commit
        // 914990f moved the ControlDef default to popup index 1 ("Out to
        // Lunch", read off the live After Dark 3.0d panel) but left this
        // field on the old fold-arithmetic guess of 2, so every headless
        // render and every `make()` without an explicit `set_control`
        // silently wrote "Gone for the Day" instead.
        message: 1,
        tier: 1,
        style_resolved: 0,
        latched: false,
        tiles: None,
        cell: (130, 122),
        phase: Phase::Pick,
        round: 0,
        endgame: false,
        done_ms: 0,
        endgame_ms: 0,
        pen: 0,
        bounds: Box2 { l: 60, t: 60, r: SCREEN_W - 60, b: SCREEN_H - 60 },
        squeak_deadline_ms: 0,
        startup_squeak: true,
        blanked: false,
        pens,
        strip_paths,
        ink_px: InkSurface::new(),
        writer: None,
        text: Vec::new(),
        text_idx: 0,
        pen_x: 60,
        line_top: 60,
        word_begin: None,
        ink: Vec::new(),
        next_glyph_ms: 0,
        space_w,
        font,
        builtins,
        grout,
        man: Man::new(),
        spurt: Minor::new(),
        drips: (0..N_DRIPS).map(|_| Minor::new()).collect(),
        man_running: false,
        wall_due: 0,
    })
}

impl MessageMayhem {
    /// fn36 (0x2D24): latch duration/style per blank (§1).
    fn latch(&mut self, ctx: &mut Ctx) {
        // fn_3B72(0): <34 → 1, <67 → 2, else 3
        self.tier = if self.duration < 34 {
            1
        } else if self.duration < 67 {
            2
        } else {
            3
        };
        // fn_3B72(1): slot -1 (first blank) → fn_3B60(2) random pick; menu 3
        // (Random) resolves the same way. ==1 → wall, else → man (§1.1).
        // ERRATUM 1: the popup index IS the folded value (raw − 1). Index 2
        // is the "Random" mark, which rolls rand(2) (range loaded @0x2D7C)
        // → 0 (Aortal Squirt) or 1 (Bathroom Wall), and only once, while
        // the slot still reads -1 (0x2D76).
        if !self.latched {
            self.style_resolved = if self.style == 2 {
                ctx.rng15.below(2) as i32
            } else {
                self.style
            };
            self.latched = true;
        }
    }

    /// fn34 (0x2A6C): text source picker (§7.1) + round sequencing (§7.3).
    /// Strings come from STR# 500/501 via pack.strings (see header).
    fn pick_text(&mut self, ctx: &mut Ctx) {
        let s = if self.round != 0 {
            // grout mode: idx = fn_3DCC(7)+1 → random 1..7 (TickCount rand)
            let idx = ctx.rng.pct(7) as usize + 1;
            str_item(&self.grout, idx)
        } else {
            match self.message {
                0 => CUSTOM_FALLBACK.to_string(), // fn35 custom (see APPROXIMATIONS)
                11 => {
                    // Message = "Random": idx = fn_3B60(11) → 0..10
                    let idx = ctx.rng15.below(11) as usize;
                    if idx == 0 {
                        CUSTOM_FALLBACK.to_string() // idx 0 → custom file
                    } else {
                        str_item(&self.builtins, idx)
                    }
                }
                v => str_item(&self.builtins, v as usize),
                // raw v → STR# 500 idx v (1-based)
            }
        };

        // §11.3: fn23 lays the message out for the pen writer — the WALL's
        // writer only. Binding the man scene to it (and so to a `clut 601`
        // pen roll it has no business reading) is what painted green
        // graffiti on the aortal black field; see ERRATUM 6.
        self.writer = if self.style_resolved == 1 {
            let (bounds, base) = (self.bounds, (self.pen.max(0) as usize) % RAMP_SLOTS);
            self.pens.as_ref().map(|f| PenWriter::new(f, &s, bounds, base))
        } else {
            None
        };

        // series-3000 glyph-stamper cursors
        self.text = s.chars().collect();
        self.text_idx = 0;
        self.pen_x = self.bounds.l;
        self.line_top = self.bounds.t;
        self.word_begin = None;
    }

    /// DoBlank §5 branch A: wall grid. ERRATUM 5 — each cell is the plain
    /// tile (frame 1) unless `rand(7) == 5` (0x255A), in which case it pops
    /// a *distinct* variant out of a 13-entry shuffle bag over {1,3,…,27}
    /// (built at 0x242E, drawn-without-replacement at 0x2568..0x25C2).
    fn build_tiles(&mut self, ctx: &mut Ctx) {
        let (tw, th) = (130, 122); // RLEP 1000 tile size (§2)
        self.cell = (tw, th);
        let cols = (SCREEN_W + tw - 1) / tw;
        let rows = (SCREEN_H + th - 1) / th;
        // 0x242E..0x2450: pool[i] = 2i+1 for i in 0..14, then pool[0] = 13
        // (the remaining count) — so 13 pickable variants, 3..27.
        let mut bag: Vec<u32> = (1..14).map(|i| 2 * i + 1).collect();
        let mut v = Vec::with_capacity((cols * rows) as usize);
        for r in 0..rows {
            for c in 0..cols {
                let mut fno = 1; // 0x25C8: the plain tile
                if !bag.is_empty() && ctx.rng15.below(7) == 5 {
                    let k = ctx.rng15.below(bag.len() as u16) as usize;
                    fno = bag.swap_remove(k);
                }
                v.push((fno, c * tw, r * th));
            }
        }
        self.tiles = Some(v);
    }

    /// DoBlank (0x23E8) as DoDrawFrame re-invokes it at 0x2736: repaint the
    /// whole tile grid (which wipes the accumulated ink), reset the round
    /// counter (0x2664) and re-stamp the completion clock (0x2630).
    fn do_blank(&mut self, ctx: &mut Ctx) {
        self.build_tiles(ctx);
        self.ink.clear();
        self.ink_px.clear();
        self.writer = None;
        self.round = 0;
        self.done_ms = ctx.now_ms;
        // 0x2644: depth ≥ 8 → pen index = rand(3) * 3, else 0
        self.pen = ctx.rng.pct(3) as i32 * 3;
        self.phase = Phase::Pick;
    }

    /// §7.3 squeak: `random(2)+1001` when the clock passes obj+0x3C, then
    /// re-arm `+0x3C = clock + 500 + random(250)` (0x5E8A: `#0xFA` = 250).
    /// ERRATUM 4: those are milliseconds. (The §11 pen writer arms the same
    /// deadline from inside fn27.)
    fn squeak(&mut self, ctx: &mut Ctx) {
        if ctx.now_ms >= self.squeak_deadline_ms {
            ctx.sounds.push(if ctx.rng.pct(2) == 0 { SND_SQUEAK1 } else { SND_SQUEAK2 });
            self.squeak_deadline_ms =
                ctx.now_ms + SQUEAK_BASE_MS + ctx.rng.pct(SQUEAK_RAND_MS) as u64;
        }
    }

    /// §11.4: pump fn27 three times per frame — the wall's writer. The man
    /// scene routes to the glyph stamper instead (ERRATUM 6).
    fn write_step(&mut self, ctx: &mut Ctx) {
        if !self.uses_pen_writer() || self.writer.is_none() {
            self.glyph_write_step(ctx);
            return;
        }
        let mut done = false;
        {
            let font = self.pens.as_ref().expect("checked above");
            let w = self.writer.as_mut().expect("checked above");
            for _ in 0..DOTS_PER_FRAME {
                if w.step(font, &mut self.ink_px, ctx, &mut self.squeak_deadline_ms) {
                    done = true;
                    break;
                }
            }
        }
        if done {
            self.finish_round(ctx);
        }
    }

    // --- the series-3000 blood-glyph writer (Aortal scene; §2, ERRATUM 6) --

    /// fn66 word wrap: if the pen passes the layout box's right edge, the
    /// current word drops to the next line.
    fn wrap_word(&mut self, ctx: &mut Ctx) {
        let Some((bi, bx)) = self.word_begin else { return };
        if self.pen_x <= self.bounds.r {
            return;
        }
        let nx = self.new_line_x(ctx);
        let dx = bx - nx;
        let (ny, dy) = if self.line_top + 2 * GLYPH_LINE_H > self.bounds.b {
            (self.bounds.t, self.bounds.t - self.line_top)
        } else {
            (self.line_top + GLYPH_LINE_H, GLYPH_LINE_H)
        };
        self.line_top = ny;
        for k in bi..self.ink.len() {
            self.ink[k].x -= dx;
            self.ink[k].y += dy;
        }
        self.pen_x = nx + (self.pen_x - bx);
        self.word_begin = Some((bi, nx));
    }

    /// '\r' x re-randomization (0x5A0A) inside the layout box.
    fn new_line_x(&mut self, ctx: &mut Ctx) -> i32 {
        let span = ((self.bounds.r - self.bounds.l) / 8).max(1) as u32;
        self.bounds.l + ctx.rng.pct(span) as i32
    }

    /// The Aortal scene's writer (§2 bank 3000, ERRATUM 6): one blood-drip
    /// glyph sprite stamped per beat, left to right, as the artery sprays.
    /// Also stands in for the wall when the pack has no `pens500.json`.
    ///
    /// The reel has no ink on screen until the man actually starts spraying
    /// (t=190 walking, nothing; t=194 first spray, still nothing; t≈196 the
    /// `M` lands), so in the man scene the stamper idles until the SM
    /// reaches `Spray1`. Squeaks stay on the wall: §2.2 splits the two sound
    /// ranges, 1001–1002 (pen squeaks) for the writer and 2000–2002 for the
    /// man, and a severed artery does not squeak like a marker.
    fn glyph_write_step(&mut self, ctx: &mut Ctx) {
        // The Aortal scene no longer comes through here at all — its writer
        // is `fn67 @132C`, driven by the blood jet. This is the wall's
        // fallback for a pack with no `pens500.json`.
        let wall = self.style_resolved == 1;
        if ctx.now_ms < self.next_glyph_ms {
            if wall {
                self.squeak(ctx);
            }
            return;
        }
        self.next_glyph_ms += GLYPH_MS;
        if self.next_glyph_ms <= ctx.now_ms {
            self.next_glyph_ms = ctx.now_ms + GLYPH_MS;
        }

        let Some(&ch) = self.text.get(self.text_idx) else {
            self.finish_round(ctx);
            return;
        };
        self.text_idx += 1;

        let wob = ((self.pen_x as f64 * 0.05).sin() * 2.0).round() as i32;

        if ch == '\r' || ch == '\n' {
            self.line_top += GLYPH_LINE_H;
            self.pen_x = self.new_line_x(ctx);
            self.word_begin = None;
            return;
        }

        let c = fold(ch);
        // chars beyond ASCII 90 (e.g. '™' U+2122) must NOT truncate into the
        // table — check the full code point first.
        let fno = if (c as u32) <= 90 { glyph_frame(c as u8) } else { None };
        match fno {
            None | Some(32) => {
                self.pen_x += self.space_w;
                self.word_begin = None;
            }
            Some(fno) => {
                let w = self.font.iter().find(|g| g.0 == fno).map(|g| g.1).unwrap_or(10);
                if self.word_begin.is_none() {
                    self.word_begin = Some((self.ink.len(), self.pen_x));
                }
                self.ink.push(Placed { fno, x: self.pen_x, y: self.line_top + wob });
                if self.ink.len() > MAX_INK {
                    let drop = self.ink.len() - MAX_INK;
                    self.ink.drain(0..drop);
                    if let Some((bi, bx)) = self.word_begin {
                        self.word_begin = Some((bi.saturating_sub(drop), bx));
                    }
                }
                self.pen_x += w;
                self.wrap_word(ctx);
            }
        }

        if self.text_idx >= self.text.len() {
            self.finish_round(ctx);
        }
    }

    // ----------------------------------------------------------------------

    /// Write complete: `[0x132]++` (0x2902), endgame forces 1000 (0x290E),
    /// `[0x114] = clock` (0x2912) — the pause timer starts now.
    fn finish_round(&mut self, ctx: &mut Ctx) {
        self.round += 1;
        if self.endgame {
            self.round = 1000; // grout hijack forever (§9.2)
        }
        self.done_ms = ctx.now_ms;
        self.phase = Phase::Done;
    }

    /// §6 style-A per-frame pipeline (`0x26E4..0x2954`).
    fn wall_tick(&mut self, ctx: &mut Ctx) {
        if self.tiles.is_none() {
            self.do_blank(ctx);
            self.endgame_ms = ctx.now_ms;
        }
        let now = ctx.now_ms;
        let pause = self.tier as u64 * ROUND_PAUSE_MS;

        // 0x26E4..0x2748: cycle end — the pause has elapsed AND the wall has
        // already carried its tier+1 messages. Toggle the endgame flag on
        // the 900 000 ms period (0x2708..0x2732), then re-blank.
        if now >= self.done_ms + pause && self.round > self.tier {
            if now >= self.endgame_ms + ENDGAME_PERIOD_MS {
                self.endgame = !self.endgame;
                self.endgame_ms = now;
            }
            self.do_blank(ctx);
        }

        match self.phase {
            Phase::Pick => {
                // ERRATUM 2: round 0 → the free random box; grout rounds →
                // the tile-snapped box. Both re-roll, every round.
                self.bounds = if self.round == 0 {
                    roll_box_fresh(ctx)
                } else {
                    roll_box_grout(ctx, self.cell.0, self.cell.1)
                };
                // 0x2890..0x28A4: the pen index handed to fn23, +9 in endgame
                self.pen = (self.pen % ENDGAME_PEN_BUMP)
                    + if self.endgame { ENDGAME_PEN_BUMP } else { 0 };
                self.pick_text(ctx); // sets this+0x136=1, this+0x138=0
                self.phase = Phase::Writing;
            }
            Phase::Writing => self.write_step(ctx),
            Phase::Done => {
                // 0x2920..0x294C: after the pause, if round <= tier the wall
                // takes another message; otherwise the cycle-end block above
                // owns the transition.
                if now >= self.done_ms + pause && self.round <= self.tier {
                    self.phase = Phase::Pick;
                }
            }
        }
    }

    // ======================================================================
    // Aortal Squirt: the man's machine, fn-by-fn
    // ======================================================================

    /// `fn66 @0DAA` — lay the message out. Writes the per-character x/y
    /// tables the artery aims at, picks the line width from a shuffled bag of
    /// four candidates, and rolls the man's row.
    fn man_layout(&mut self, ctx: &mut Ctx) {
        let w = SCREEN_W;
        let h = SCREEN_H;
        // `fn60 @0254`: the line pitch is glyph 'p' plus ten.
        self.man.line_h =
            bank_rect(&self.pack, S_FONT, 0x70).map(|r| r[3] - r[1]).unwrap_or(30) + LINE_PAD;
        self.man.fudge = FIRST_WRAP_FUDGE;

        // @0DE2: a character with no art in bank 3000 becomes a space.
        let mut msg: Vec<u8> = self
            .text
            .iter()
            .map(|c| if (*c as u32) < 256 { *c as u8 } else { b' ' })
            .collect();
        for c in msg.iter_mut() {
            if self.pack.frame(S_FONT, *c as u32).is_none() {
                *c = b' ';
            }
        }
        let len = msg.len();

        // @0E4A: the word table — (width, start index), `start` being the
        // index of the space that ENDED the previous word.
        let gw = |c: u8| -> i32 { self.pack.frame(S_FONT, c as u32).map(|g| g.w).unwrap_or(0) };
        let mut words: Vec<(i32, usize)> = Vec::new();
        let (mut run_w, mut prev_w, mut seg) = (0i32, 0i32, 0usize);
        for (i, &c) in msg.iter().enumerate() {
            if c == b' ' || c == b'\r' {
                if words.len() < MAX_WORDS {
                    words.push((run_w - prev_w, seg));
                } else {
                    let n = words.len() - 1;
                    words[n] = (run_w - prev_w, seg);
                }
                prev_w = run_w;
                seg = i;
            }
            run_w += gw(c);
        }
        if words.len() < MAX_WORDS {
            words.push((run_w - prev_w, seg));
            words.push((0, len));
        } else {
            let n = words.len() - 1;
            words[n] = (run_w - prev_w, seg);
        }
        // `nw` counts the real words; the extra entry is the sentinel.
        let nw = words.len().saturating_sub(1);

        self.man.char_x = vec![0; len];
        self.man.char_y = vec![0; len];
        self.man.drawn = vec![false; len];

        // @0F84: four candidate line boxes, drawn without replacement.
        let cand: Vec<(i32, i32)> =
            (0..4).map(|j| ((j + 1) * (w / (j + 2)), (j + 1) * (h / (j + 2)))).collect();
        let widest = cand[3].0;
        let mut order: Vec<usize> = (0..4).collect();
        let mut n_cand = 4usize;
        let mut lines;
        let mut pick;
        let mut guard = 0;
        loop {
            guard += 1;
            let k = (ctx.rng.pct(n_cand.max(1) as u32) as usize).min(n_cand.saturating_sub(1));
            pick = order[k];
            n_cand = n_cand.saturating_sub(1);
            // QUIRK KEPT (@1004): the removal writes through the PICKED value,
            // not the rolled slot — identical on the first pass (the bag
            // starts as the identity) and skewed afterwards.
            if pick < order.len() {
                order[pick] = order[n_cand];
            }
            let line_w = cand[pick].0;
            let jitter = ctx.rng.pct(((w - line_w) >> 1).max(1) as u32) as i32;
            lines = 0;
            let mut ci = 0usize;
            while ci < nw {
                lines += 1;
                let seg = ci;
                let mut acc = 0i32;
                while ci < nw && words[ci].0 + acc <= line_w {
                    let prev = ci;
                    ci += 1;
                    acc += words[prev].0;
                }
                if acc == 0 {
                    acc = line_w;
                    ci += 1;
                }
                let mut x = (line_w >> 1) + jitter + LINE_MARGIN - (acc >> 1);
                let stop = words.get(ci).map(|v| v.1).unwrap_or(len);
                let mut c = words[seg].1;
                while c < stop && c < len {
                    self.man.char_x[c] = x;
                    x += gw(msg[c]);
                    c += 1;
                }
            }
            let done = lines * self.man.line_h < cand[pick].1
                || cand[pick].0 == widest
                || n_cand == 0
                || guard > 8;
            if done {
                break;
            }
        }

        // @11D2: the row, then every character's y.
        self.man.ci = 0;
        let slack = cand[pick].1 - (lines + 1) * self.man.line_h - ROW_BASE;
        self.man.row = if slack < 1 {
            ROW_BASE
        } else {
            ctx.rng.pct(slack as u32) as i32 + ROW_BASE
        };
        // @1226: half the spraying man, then half a capital A.
        let man_h = bank_rect(&self.pack, S_MAN, RUN_SPRAY_A).map(|r| r[3] - r[1]).unwrap_or(125);
        let glyph_h = bank_rect(&self.pack, S_FONT, 0x41).map(|r| r[3] - r[1]).unwrap_or(30);
        let mut top = self.man.row - (man_h >> 1) - (glyph_h >> 1);
        let mut prev_x = 0i32;
        let mut i = 0usize;
        let mut spins = 0;
        while i < len && spins < len * 8 + 64 {
            spins += 1;
            if prev_x < self.man.char_x[i] {
                self.man.char_y[i] = top + ctx.rng.pct(5) as i32;
                prev_x = self.man.char_x[i];
                self.man.drawn[i] = false;
                i += 1;
            } else {
                prev_x = 0;
                top += self.man.line_h;
            }
        }
        self.man.msg = msg;
        self.man.drips = 0;
        self.man.armed = false;
    }

    /// `fn67 @132C`, hooked off the sprite list's draw (`fn37 @2DB2` installs
    /// the man, `fn38 @2DC6` calls this). The blood jet's tip — the part whose
    /// art is 87, present only in frames 39..45 and 57..63 — is the write
    /// cursor; a character is stamped into the backdrop when the cursor
    /// reaches it. The neck really is the cursor.
    fn man_write(&mut self) {
        let run = self.man.s.run;
        if (run != RUN_SPRAY_B && run != RUN_SPRAY_A) || !self.man.armed {
            return;
        }
        let Some(jet) = part_by_art(&self.pack, S_MAN, self.man.s.frame, JET_ART) else { return };
        // @1390: `+0x68(&pt, 87, frame)` then the sequence's `+0x98`.
        // GAP(jet rect): `+0x98` is library code this decompile does not
        // cover; the jet part's own bank rect carried onto the drawn frame
        // gives the +9 px/frame cursor the pack's part table shows.
        // The part rect is laid out under the man's flip (L135 `fn3BD6`) —
        // spray run 36 is drawn mirrored after the 64 → 1 hand-off flips
        // him, and its jet then marches the other way.
        let Some([_, _, cur_r, cur_b]) = self.man.s.part_screen(&self.pack, S_MAN, &jet) else {
            return;
        };
        let start = self.man.ci;
        for j in start..(start + 3).min(self.man.msg.len()) {
            let ch = self.man.msg[j];
            if ch == 0 {
                break;
            }
            let Some(g) = bank_rect(&self.pack, S_FONT, ch as i32) else { continue };
            let gw = g[2] - g[0];
            let cx = self.man.char_x[j];
            // @1420 / @1470: run 53 stamps once the cursor is past the
            // character's right edge; run 36 while it is inside its span.
            let hit = if run == RUN_SPRAY_B {
                gw + cx - 2 <= cur_r
            } else {
                !(cx > cur_r || gw + cx + 2 < cur_r)
            };
            if !hit || self.man.char_y[j] > cur_b + 4 {
                continue;
            }
            // @14F4: move the glyph's bank rect so its top-left lands on
            // (char_x, char_y) and draw it there — permanent ink.
            self.ink.push(Placed { fno: ch as u32, x: cx, y: self.man.char_y[j] });
            if self.ink.len() > MAX_INK {
                let drop = self.ink.len() - MAX_INK;
                self.ink.drain(0..drop);
            }
            self.man.drawn[j] = true;
            if self.man.ci == j {
                self.man.ci += 1;
                // @1580: one drip per ODD character, capped at sixteen.
                if self.man.drips < N_DRIPS {
                    self.man.drips += self.man.ci & 1;
                }
            }
        }
    }

    /// `fn13 @3656` behind `fn61 @0450`'s frame switch: the man's art fires
    /// its own sounds off the frame it is showing, with a 1000 ms re-run
    /// guard on the two that can repeat.
    fn man_frame_sound(&mut self, ctx: &mut Ctx) {
        let id = match self.man.s.frame {
            0x47 => SND_OHH2,
            0x24 | 0x35 => SND_SPRAY,
            0x5C => SND_BODYFALL,
            _ => return,
        };
        if id != SND_BODYFALL && ctx.now_ms <= self.man.snd_gate {
            return;
        }
        ctx.sounds.push(id);
        self.man.snd_gate = ctx.now_ms + SND_GUARD_MS;
    }

    /// L135 `fn55B6 @55B6`: fire the current state's EXIT now, latch the new
    /// state, and flag the ENTER for the next Run.
    fn man_set_state(&mut self, ctx: &mut Ctx, s: i32) {
        let old = self.man.state;
        self.man_msg(ctx, 0x4000_0000 | (0x4000 | old) as u32);
        self.man.prev = old;
        self.man.state = s;
        self.man.entered = true;
    }

    /// L135 `fn5340 @5340`: ENTER (if pending) then UPDATE — and the UPDATE
    /// reads the state field AFTER the enter, so an enter that changes state
    /// hands its own update to the new state. State 8 relies on that.
    fn man_run(&mut self, ctx: &mut Ctx) {
        if self.man.entered {
            self.man.entered = false;
            let s = self.man.state;
            self.man_msg(ctx, 0x8000_0000 | (0x8000 | s) as u32);
        }
        let s = self.man.state;
        self.man_msg(ctx, s as u32);
    }

    /// **`fn61 @0420`** — the man's message handler. States 1..8.
    fn man_msg(&mut self, ctx: &mut Ctx, m: u32) {
        let low = (m & 0xFFFF) as i32;
        let flags = m & 0xC000_0000;
        let p = &self.pack as *const Pack;
        // SAFETY-free shorthand: the pack is never mutated below.
        let pack: &Pack = unsafe { &*p };

        // @0450: the per-frame sound switch runs on UPDATEs only.
        if flags == 0 {
            self.man_frame_sound(ctx);
        }

        match low {
            // ---- state 1: walk in from the left ---------------------------
            0x8001 => {
                self.man.snd_gate = 0;
                self.man.s.shown = true;
                // @0550: `+0x70(1)` seats frame 1 before the place, so the
                // walk-in's run 15 links from frame 1, not the corpse.
                self.man.s.set_frame(RUN_WALK_L);
                self.man.s.set_pos(MAN_ENTER_X, self.man.row);
                // @0578: a leftover flip is cleared; nothing ever sets one.
                if self.man.s.flip {
                    self.man.s.flip = false;
                }
                self.man.s.set_runs(pack, S_MAN, &[RUN_WALK_R]);
                self.man.armed = true;
                self.man.gag = true;
                self.man.due = 0;
            }
            0x01 => {
                self.man.s.tick(pack, S_MAN);
                if self.man.s.done {
                    if self.man.stop_x() <= self.man.s.x {
                        self.man_set_state(ctx, 2);
                        return;
                    }
                    // @064C: the walk is a LOOP with a position test — the
                    // one `+0x7C` immediate in the whole module (§10.1).
                    self.man.s.set_run(pack, S_MAN, RUN_WALK_R);
                }
            }
            // ---- state 2: spray a character, step, spray ------------------
            0x8002 => {
                self.man.s.set_runs(
                    pack,
                    S_MAN,
                    &[RUN_SPRAY_B, RUN_WALK_L, RUN_SPRAY_A, RUN_TURN],
                );
                self.man.gag = false;
                self.man.armed = true;
            }
            0x02 => {
                self.man.s.tick(pack, S_MAN);
                if self.man.s.done {
                    if off_screen(&self.man.s, pack, S_MAN) {
                        // @06C0: next line. The first wrap is short by 14.
                        self.man.row += self.man.line_h - self.man.fudge;
                        self.man.fudge = 0;
                        self.man_set_state(ctx, 3);
                        return;
                    }
                    if self.man.ci != 0 {
                        let i = self.man.ci;
                        let eom = self.man.msg.get(i).copied().unwrap_or(0) == 0;
                        let wrapped = !eom && self.man.char_x[i] < self.man.char_x[i - 1];
                        if eom || wrapped {
                            // nothing left on this line: just walk.
                            self.man.s.set_runs(pack, S_MAN, &[RUN_WALK_R]);
                        } else {
                            self.man.s.set_runs(
                                pack,
                                S_MAN,
                                &[RUN_WALK_R, RUN_SPRAY_B, RUN_WALK_L, RUN_SPRAY_A, RUN_TURN],
                            );
                        }
                    }
                }
            }
            // ---- state 3: hidden for three seconds ------------------------
            0x8003 => {
                self.man.pause_until = ctx.now_ms + 3000;
                // GAP(show/hide): `+0x2C(0)` / `+0x30()` read as Hide / Show
                // at every call site but the library slots are undecoded.
                self.man.s.shown = false;
            }
            0x03 => {
                if ctx.now_ms > self.man.pause_until {
                    self.man.s.shown = true;
                    if self.man.ci < self.man.msg.len() && self.man.row < SCREEN_H {
                        self.man_set_state(ctx, 4);
                    } else {
                        self.man_set_state(ctx, 5);
                    }
                    return;
                }
            }
            // ---- state 4: come back in from the right, walking left -------
            0x8004 => {
                self.man.s.shown = true;
                // @0826: `+0x70(1)` then SetPos, then run 1 — linked from
                // frame 1 onto itself. The flip is NOT cleared here (only
                // state 1's ENTER clears it).
                self.man.s.set_frame(RUN_WALK_L);
                self.man.s.set_pos(SCREEN_W + MAN_RETURN_X, self.man.row);
                self.man.s.set_runs(pack, S_MAN, &[RUN_WALK_L]);
                self.man.gag = false;
            }
            0x04 => {
                self.man.s.tick(pack, S_MAN);
                if self.man.s.done {
                    if self.man.s.x <= self.man.stop_x() {
                        self.man_set_state(ctx, 2);
                        return;
                    }
                    // @08F2: a one-in-three stumble, once per leg.
                    if !self.man.gag && ctx.rng.pct(3) == 2 {
                        self.man.s.set_runs(pack, S_MAN, &[RUN_STAGGER, RUN_RECOVER]);
                        self.man.gag = true;
                    } else {
                        self.man.s.set_run(pack, S_MAN, RUN_WALK_L);
                        self.man.gag = false;
                    }
                }
            }
            // ---- state 5: the last walk in, down one line -----------------
            0x8005 => {
                self.man.row += self.man.line_h;
                let half = bank_rect(pack, S_MAN, RUN_WALK_L)
                    .map(|r| ((r[3] - r[1]) >> 1) + 4)
                    .unwrap_or(66);
                if SCREEN_H < half + self.man.row {
                    self.man.row = SCREEN_H - half;
                }
                self.man.s.set_runs(pack, S_MAN, &[RUN_WALK_L, RUN_WALK_L, RUN_WALK_L]);
                self.man.s.set_pos(SCREEN_W + MAN_DEATH_X, self.man.row);
            }
            0x05 => {
                self.man.s.tick(pack, S_MAN);
                if self.man.s.done {
                    if self.man.s.x < SCREEN_W >> 2 {
                        self.man_set_state(ctx, 6);
                        return;
                    }
                    if self.man.s.x <= (SCREEN_W / 3) * 2 && ctx.rng.pct(3) == 0 {
                        self.man_set_state(ctx, 6);
                        return;
                    }
                    self.man.s.set_run(pack, S_MAN, RUN_WALK_L);
                }
            }
            // ---- state 6: the death beat ----------------------------------
            0x8006 => {
                self.man.s.set_runs(
                    pack,
                    S_MAN,
                    &[
                        RUN_TURN,
                        RUN_WALK_R,
                        RUN_SPRAY_B,
                        RUN_WALK_L,
                        RUN_WALK_L,
                        RUN_STAGGER,
                        RUN_FALL,
                        RUN_CORPSE,
                    ],
                );
            }
            0x06 => {
                self.man.s.tick(pack, S_MAN);
                if self.man.s.done {
                    self.man_set_state(ctx, 7);
                    return;
                }
            }
            // ---- state 7: the frozen tableau ------------------------------
            0x8007 => self.man.rest_at = ctx.now_ms,
            0x07 => {
                if ctx.now_ms > self.man.rest_at + self.man.hold_ms {
                    self.man.s.set_pos(PARK, PARK);
                    self.spurt.s.set_pos(PARK, PARK);
                    for d in self.drips.iter_mut() {
                        d.s.set_pos(PARK, PARK);
                    }
                    self.man.drips = 0;
                    self.man_set_state(ctx, 8);
                    return;
                }
            }
            // ---- state 8: the reset latch ---------------------------------
            0x8008 => {
                self.man.fudge = FIRST_WRAP_FUDGE;
                self.man_set_state(ctx, 1);
                return;
            }
            _ => {
                // Any unhandled UPDATE (state 0 included) falls into state 1.
                if flags == 0 {
                    self.man_set_state(ctx, 1);
                    return;
                }
            }
        }
        // @0C58: every path that does not transition runs the spurt.
        self.spurt_run(ctx);
    }

    // ---- the neck spurt, fn74 @1770 --------------------------------------

    fn spurt_run(&mut self, ctx: &mut Ctx) {
        if self.spurt.entered {
            self.spurt.entered = false;
            let s = self.spurt.state;
            self.spurt_msg(ctx, 0x8000_0000 | (0x8000 | s) as u32);
        }
        let s = self.spurt.state;
        self.spurt_msg(ctx, s as u32);
    }

    fn spurt_set_state(&mut self, ctx: &mut Ctx, s: i32) {
        let old = self.spurt.state;
        self.spurt_msg(ctx, 0x4000_0000 | (0x4000 | old) as u32);
        self.spurt.state = s;
        self.spurt.entered = true;
    }

    fn spurt_msg(&mut self, ctx: &mut Ctx, m: u32) {
        let low = (m & 0xFFFF) as i32;
        let flags = m & 0xC000_0000;
        let p = &self.pack as *const Pack;
        let pack: &Pack = unsafe { &*p };

        let run = self.man.s.run;
        let attached =
            matches!(run, RUN_WALK_L | RUN_WALK_R | RUN_TURN | RUN_STAGGER | RUN_RECOVER);
        // @17A2: no man on screen, no spurt. Same for the spray runs, whose
        // art carries the jet itself.
        if !sprite_live(&self.man.s, pack, S_MAN) || !attached {
            self.spurt.s.shown = false;
            return;
        }
        self.spurt.s.shown = true;
        match low {
            0x8001 => self.spurt.s.set_run(pack, S_MAN, RUN_SPURT),
            0x01 => {
                self.spurt.s.tick(pack, S_MAN);
                if self.spurt.s.done {
                    self.spurt.s.set_run(pack, S_MAN, RUN_SPURT);
                }
            }
            _ => {
                if flags == 0 {
                    self.spurt_set_state(ctx, 1);
                    return;
                }
            }
        }
        if self.spurt.s.frame == 0 {
            return;
        }
        // @18B0: keep the flip in step with the man, then pin channel 1 —
        // the invisible marker — of both frames together.
        if self.man.s.flip != self.spurt.s.flip {
            self.spurt.s.flip = !self.spurt.s.flip;
        }
        let (Some(mp), Some(sp)) = (
            part_by_chan(pack, S_MAN, self.man.s.frame, MARK_CH),
            part_by_chan(pack, S_MAN, self.spurt.s.frame, MARK_CH),
        ) else {
            return;
        };
        // seq `+0x94` gives the man's channel-1 point under HIS flip; seq
        // `+0x84` = L135 `fn4220 @4220` centres the spurt so its own
        // channel-1 part (under its flip) lands on that point.
        let Some(m) = self.man.s.part_screen(pack, S_MAN, &mp) else { return };
        let Some(r) = part_rel(pack, S_MAN, self.spurt.s.frame, &sp, self.spurt.s.flip) else { return };
        self.spurt.s.set_pos(m[0] - r[0], m[1] - r[1]);
    }

    // ---- the blood drips, fn62 @0C74 + fn78 @1A0E ------------------------

    /// `fn62 @0C74`: every live drip that has passed its own 90 ms gate
    /// (`fn79 @1C6E`) runs its machine.
    fn drips_tick(&mut self, ctx: &mut Ctx) {
        for i in 0..self.man.drips.min(N_DRIPS) {
            if ctx.now_ms < self.drips[i].due {
                continue;
            }
            self.drips[i].due = ctx.now_ms + GATE_MS;
            self.drip_run(ctx, i);
        }
    }

    fn drip_run(&mut self, ctx: &mut Ctx, i: usize) {
        if self.drips[i].entered {
            self.drips[i].entered = false;
            let s = self.drips[i].state;
            self.drip_msg(ctx, i, 0x8000_0000 | (0x8000 | s) as u32);
        }
        let s = self.drips[i].state;
        self.drip_msg(ctx, i, s as u32);
    }

    fn drip_set_state(&mut self, ctx: &mut Ctx, i: usize, s: i32) {
        let old = self.drips[i].state;
        self.drip_msg(ctx, i, 0x4000_0000 | (0x4000 | old) as u32);
        self.drips[i].state = s;
        self.drips[i].entered = true;
    }

    fn drip_msg(&mut self, ctx: &mut Ctx, i: usize, m: u32) {
        let low = (m & 0xFFFF) as i32;
        let flags = m & 0xC000_0000;
        let p = &self.pack as *const Pack;
        let pack: &Pack = unsafe { &*p };
        match low {
            0x8001 => {
                // @1A44: pick a character the artery has already written.
                let n = self.man.ci;
                if n == 0 {
                    return;
                }
                let j = (ctx.rng.pct(n as u32) as usize).min(n - 1);
                if !self.man.drawn.get(j).copied().unwrap_or(false) {
                    self.drip_set_state(ctx, i, 1);
                    return;
                }
                // @1A7C: a random one of the glyph's five channels, 4..7.
                let ch = ctx.rng.pct(4) as i32 + 4;
                let g = self.man.msg[j] as i32;
                let (Some(r1), Some(r3)) = (
                    part_by_chan(pack, S_FONT, g, ch),
                    part_by_chan(pack, S_FONT, g, 3),
                ) else {
                    self.drip_set_state(ctx, i, 1);
                    return;
                };
                let x = self.man.char_x[j] + (r1[3] + ((r1[5] - r1[3]) >> 1) - r3[3]) + 5;
                let y = self.man.char_y[j] + (r1[4] + ((r1[6] - r1[4]) >> 1) - r3[4]);
                self.drips[i].s.shown = true;
                self.drips[i].s.set_run(pack, S_MAN, RUN_DRIP);
                self.drips[i].s.set_pos(x, y);
                self.drips[i].due = ctx.now_ms + ctx.rng.pct(3) as u64 * 30;
            }
            0x01 => {
                self.drips[i].s.tick(pack, S_MAN);
                if self.drips[i].s.done {
                    self.drips[i].s.shown = false;
                    self.drip_set_state(ctx, i, 2);
                    return;
                }
            }
            0x02 => {
                self.drips[i].s.set_pos(PARK, PARK);
                self.drip_set_state(ctx, i, 1);
                return;
            }
            _ => {
                if flags == 0 {
                    self.drip_set_state(ctx, i, 1);
                }
            }
        }
    }

    /// **`fn33 @2692`, the `+0x12A != 1` arm** — the whole man scene per
    /// frame. `+0x136` is the "a victim is live" latch; it is cleared when
    /// the man reaches state 8, and that is what washes the screen and lays
    /// the next message out.
    fn aortal_tick(&mut self, ctx: &mut Ctx) {
        // @2960: the tableau hold is the Duration band × 10 000 ms.
        self.man.hold_ms = self.tier as u64 * ROUND_PAUSE_MS;
        if !self.man_running {
            self.round = 0; // @2976: never the grout list on this arm
            self.pick_text(ctx); // fn34 @2A68
            self.man_layout(ctx); // fn66 @0DAA
            self.man_running = true;
            self.man_run(ctx);
        } else {
            self.drips_tick(ctx); // fn62 @0C74
            if ctx.now_ms >= self.man.due {
                self.man.due = ctx.now_ms + GATE_MS; // fn63 @0CF2
                self.man_run(ctx);
            }
        }
        // fn38 @2DC6 off the sprite list's draw. GAP(fn67 call rate): the C
        // runs that draw twice per frame; the writer is position-gated so
        // the difference is bounded by its three-character lookahead.
        self.man_write();
        // @2A0E: state 8 → DoBlank (a bare field wash on this arm) and relay.
        if self.man.state == 8 {
            self.ink.clear();
            self.man_running = false;
        }
    }


    /// True when this scene draws its message with the `Pens` 500 stroke
    /// writer: the Bathroom Wall, and only if the pack shipped the font.
    /// The Aortal scene never does (ERRATUM 6).
    fn uses_pen_writer(&self) -> bool {
        self.style_resolved == 1 && self.pens.is_some()
    }

    /// Wall ink surface -> sprites (see the header's "Ink surface mechanism").
    fn push_pen_ink(&self, out: &mut Vec<SpriteDraw>) {
        for &(pi, x, y) in &self.ink_px.strips {
            if let Some(p) = self.strip_paths.get(pi) {
                out.push(SpriteDraw { flip: false, pal: 0, png: p.clone(), x, y });
            }
        }
    }

    /// Series-3000 blood-drip glyphs at the stamped pen position. `f.bx/f.by`
    /// are the rip-capture absolute positions, not per-glyph offsets, so they
    /// are ignored entirely: the canvas already carries the sidebearings
    /// (ERRATUM 6c) and the stamper placed `g.x` accordingly. The ink colour
    /// is baked into the art (a flat `#aa0000`) — there is no palette roll
    /// and no remap, which is why `pal: 0`.
    fn push_glyph_ink(&self, out: &mut Vec<SpriteDraw>) {
        for g in &self.ink {
            let Some(f) = self.pack.frame(S_FONT, g.fno) else { continue };
            out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x: g.x, y: g.y });
        }
    }

    fn push_ink(&self, out: &mut Vec<SpriteDraw>) {
        if self.uses_pen_writer() {
            self.push_pen_ink(out);
        } else {
            self.push_glyph_ink(out);
        }
    }
}

impl Module for MessageMayhem {
    fn name(&self) -> &'static str {
        "Message Mayhem"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            // sVal 1000 + sUnt 1000 (the words come from the pack)
            ControlDef {
                name: "Duration".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
            // MENU 1001 (from the pack): 0 Aortal Squirt, 1 Bathroom Wall,
            // 2 '-', 3 Random. The separator is dropped (raw 3 → popup
            // index 2; resolved via fn_3B60(2)-shaped pick, §1.1).
            ControlDef {
                name: "Style".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    items: self.pack.popup_items(1001, 3, true),
                },
                // ERRATUM 1: mVal 1001 default mark 1 → folded 0 = Aortal
                // Squirt. The man is the default scene.
                default: 0,
            },
            // MENU 1002 (from the pack) ships 14 items: 1 Custom, 2 '-',
            // 3..12 the ten STR# 500 built-ins (abbreviated for the menu),
            // 13 '-', 14 Random. Both separators are dropped here, so popup
            // index 1..10 == STR# 500 idx 1..10 and popup index 11 == the
            // Random mark 14 (`[0x12C]` = 13, the literal `fn34 @0x2A82`
            // tests).
            ControlDef {
                name: "Message".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    items: self.pack.popup_items(1002, 12, true),
                },
                // mVal 1002 = 3 ⇒ MENU item 3, literally "Out to Lunch" —
                // and the arithmetic agrees once BOTH folds are applied
                // (`fn36 @0x2DA0` −1, then `fn34 @0x2AA6` −1 ⇒ STR# 500
                // idx 1). ERRATUM 1's "Gone for the Day" stopped a fold
                // short. See the CONTRADICTIONS note.
                default: 1,
            },
            // bVal 1003 "Edit Custom" omitted — no dialog plumbing (§10 Q5)
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.duration = value.clamp(0, 100),
            1 => {
                self.style = value.clamp(0, 2);
                self.latched = false; // re-latch style on next blank (fn36)
            }
            2 => self.message = value.clamp(0, 11),
            _ => {}
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        self.latch(ctx); // fn36 per blank/frame

        // DoBlank 0x2644: at depth ≥ 8 the pen index is `rand(3) * 3` — one
        // of the pencil / blood / green ramps of clut 601 (§11.7) — else 0.
        // The roll runs in both style branches, but only the wall READS it
        // (fn23 at 0x28A4, on the `==1` arm): `clut 601` is "Bathroom Pens",
        // and §4.6 only loads it on that arm. Rolling it for the man scene
        // too was harmless; letting the man scene *draw* with it was not
        // (ERRATUM 6). `do_blank` re-rolls per wall cycle — which is why the
        // wall's messages vary between the green, blood and pencil ramps and
        // between solid and hollow (the 0x13A toggle's 9/12/15 half) — while
        // the man scene, having no re-blank and no pen writer, never changes
        // ink at all.
        if !self.blanked {
            self.blanked = true;
            self.pen = ctx.rng.pct(3) as i32 * 3;
        }

        // §4 step 9: init-time startup squeak (fn11 sound-range once). §2.2
        // splits the ranges: 1001–1002 is the pen, so it is the wall's.
        if self.startup_squeak {
            self.startup_squeak = false;
            if self.style_resolved == 1 {
                ctx.sounds.push(if ctx.rng.pct(2) == 0 { SND_SQUEAK1 } else { SND_SQUEAK2 });
            }
            self.squeak_deadline_ms =
                ctx.now_ms + SQUEAK_BASE_MS + ctx.rng.pct(SQUEAK_RAND_MS) as u64;
        }

        if self.style_resolved == 1 {
            // The wall was verified on the 40 ms beat and is left on it; the
            // module's own clock is now `MacTick` so the man's `fn63 @0CF2`
            // 90 ms gate quantizes the way the original's does.
            if ctx.now_ms >= self.wall_due {
                self.wall_due = ctx.now_ms + 40;
                self.wall_tick(ctx); // style A: tile wall + writer
            }
        } else {
            self.aortal_tick(ctx); // style B: fn33 @2692's man arm
        }

        if self.ink_px.dirty {
            self.ink_px.rebuild();
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        if self.style_resolved == 1 {
            // tile wall backdrop (DoBlank §5 branch A)
            if let Some(tiles) = &self.tiles {
                for &(fno, x, y) in tiles {
                    if let Some(f) = self.pack.frame(S_TILE, fno) {
                        out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x, y });
                    }
                }
            }
            self.push_ink(out); // pen ink over the tiles
        } else {
            // The man scene's backdrop is the bare field (§5 branch B: the
            // `0x2670` arm only calls engine vtable+0x14 — no grid, no tiles;
            // the reel's aortal span is a flat black screen). The blood
            // glyphs go down first: the reel at t=208 has the man's head
            // overlapping the trailing `S`, and he is in front of it.
            self.push_glyph_ink(out);
            // No blood-pool sprite exists: the pool is baked into the fall
            // run 88..110 and the corpse 112..113. The three sprites are the
            // man, the neck spurt pinned to him (`fn74 @1770`) and the live
            // drips (`fn78 @1A0E`); `pos` is the frame CENTRE, so each blits
            // at `pos − (w>>1, h>>1)`.
            let mut push = |s: &Spr| {
                if !s.shown || s.frame == 0 {
                    return;
                }
                let Some(f) = self.pack.frame(S_MAN, s.frame.max(0) as u32) else { return };
                let (Some((x, y)), _) = (s.draw_at(&self.pack, S_MAN), ()) else { return };
                out.push(SpriteDraw { flip: s.flip, pal: 0, png: f.png.clone(), x, y });
            };
            for d in self.drips.iter().take(self.man.drips.min(N_DRIPS)) {
                push(&d.s);
            }
            push(&self.spurt.s);
            push(&self.man.s);
        }
    }

    /// §2: every module rides After Dark's own clock. `fn63 @0CF2` re-arms
    /// `now + 0x5A` and tests `>=`, which on the 16.625 ms grid lands on the
    /// ~99.8 ms the capture's periodogram measured — so the 90 in the code is
    /// the real constant and the 100 the prose pinned was the quantization.
    fn clock(&self) -> engine::TickClock {
        engine::TickClock::MacTick
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The ink strips (`gen:pen/sNN_wWW`), rebuilt from the name. See
    /// [`strip_image`] and the header's "Ink surface mechanism".
    fn generated(&self, name: &str) -> Option<Image> {
        strip_image(&self.pens.as_ref()?.ramp, name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::path::Path;

    fn ctx() -> Ctx {
        Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    fn pack() -> Option<Pack> {
        Pack::load(Path::new("../assets/message-mayhem")).ok()
    }

    fn run(style: i32) {
        let Some(pack) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let Some(mut m) = make(pack) else {
            eprintln!("make() returned None — skipping");
            return;
        };
        m.set_control(1, style);
        let mut ctx = ctx();
        let mut pacer = Pacer::new(m.as_ref());
        let mut out: Vec<SpriteDraw> = Vec::new();
        let mut drew = false;
        let mut sounds = 0usize;
        let mut texts_seen = 0usize;
        while pacer.now_ms() < 40_000 {
            pacer.advance(&mut ctx);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            sounds += ctx.sounds.len();
            out.clear();
            m.sprites(&mut out);
            if !out.is_empty() {
                drew = true;
            }
            let mut t = Vec::new();
            m.texts(&mut t);
            texts_seen += t.len();
        }
        assert!(drew, "style {style}: no sprites in 40 s");
        // The wall's squeak deadline is 500+rand(250) ms; the man's artery
        // sprays on every run 36/53 behind a 1000 ms guard. Either way the
        // scene has to be audible inside forty seconds.
        assert!(sounds >= 5, "style {style}: only {sounds} sounds in 40 s");
        // the message renders exactly once, as ink — no engine-font overlay.
        assert_eq!(texts_seen, 0, "style {style}: text overlay is back");
    }

    #[test]
    fn smoke_bathroom_wall() {
        run(1); // Bathroom Wall: tiles + writer + grout hijack
    }

    #[test]
    fn smoke_aortal_squirt() {
        run(0); // Aortal Squirt: the man SM
    }

    /// ERRATUM 1: `fn36`'s `- 1` fold means mVal 1001's default mark 1
    /// resolves to 0 — the Aortal Squirt man scene, not the tile wall. The
    /// user-visible symptom of getting this wrong was "there's no guy at
    /// all on the screen".
    #[test]
    fn default_style_is_the_aortal_man_scene() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let m = make(p).expect("make");
        let style = &m.controls()[1];
        assert_eq!(style.default, 0, "Style default must fold to Aortal Squirt");
        // and the man must actually be on screen from the first frame
        let Some(p) = pack() else { return };
        let mut m = make(p).expect("make");
        let mut ctx = ctx();
        let mut pacer = Pacer::new(m.as_ref());
        // `fn33 @2976`'s first pass lays the message out and runs the man
        // once; state 0's update only sets state 1, so his first frame lands
        // on the next beat. He is on screen well inside a second either way.
        for _ in 0..8 {
            pacer.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        let mut out = Vec::new();
        m.sprites(&mut out);
        assert!(
            out.iter().any(|s| s.png.contains("/2000/")),
            "no series-2000 man sprite on the default style"
        );
    }

    /// CONTRADICTIONS 2026-09-13: the shipped Message default is
    /// "Out to Lunch". `mVal 1002` = 3 is a 1-based MENU 1002 item number and
    /// item 3 is that string; the fold arithmetic agrees once **both**
    /// subtractions are counted (`fn36 @0x2DA0` then `fn34 @0x2AA6`), giving
    /// STR# 500 idx `mVal − 2` = 1. ERRATUM 1's "Gone for the Day" applied
    /// only the first fold.
    #[test]
    fn message_default_is_out_to_lunch_by_both_folds() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let builtins = p.strings(STR_BUILTINS).to_vec();
        let menu = p.menu(1002).to_vec();
        let m = make_concrete(p).expect("make");

        // The two folds, as the listing has them.
        let mval = 3i32; // Message Mayhem_mVal_1002_Message.bin == 00 03
        let field_12c = mval - 1; // fn36 @0x2DA0
        let str500_idx = field_12c - 1; // fn34 @0x2AA6, guarded by != 0
        assert_eq!(str500_idx, 1);
        // MENU item mVal (1-based) is the menu's spelling of STR# 500 idx
        // `mVal − 2`: the two folds land on the item the panel shows…
        assert_eq!(builtins.len(), 10, "STR# 500 carries ten built-ins");
        if menu.len() >= mval as usize {
            assert_eq!(menu[mval as usize - 1], builtins[str500_idx as usize - 1]);
            // …and one fold short lands on the NEXT built-in — the losing
            // reading.
            assert_ne!(menu[mval as usize - 1], builtins[field_12c as usize - 1]);
        } else {
            eprintln!("pack predates MENU ripping — skipping the menu half");
        }

        // …and the port must ship that, from both the ControlDef and the
        // field initialiser (ERRATUM 7's bug was the two disagreeing).
        let defs = Module::controls(&m);
        assert_eq!(defs[2].default, 1, "ControlDef default");
        assert_eq!(m.message, 1, "struct field default");
        let ControlKind::Popup { items, .. } = &defs[2].kind else { panic!("popup") };
        // the Random mark closes the mapping: MENU item 14 ⇒ [0x12C] = 13
        assert_eq!(items.len(), 12, "14 MENU items less the two separators");
        if !menu.is_empty() {
            assert_eq!(items[m.message as usize], builtins[0], "popup default = STR# 500 idx 1");
            assert_eq!(&items[11], menu.last().unwrap(), "popup 11 = the menu's last (Random) item");
        }
    }

    /// A pack with no STR# 500/501 (or no MENUs) must still build and run:
    /// the rounds write empty messages and the popups fall back to item
    /// numbers, never a panic.
    #[test]
    fn missing_strings_degrade_to_empty_messages() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let mut meta = (*p.meta).clone();
        meta.strings.clear();
        meta.menus.clear();
        meta.slider_words.clear();
        let bare = Pack::from_meta(meta, p.root());
        for style in [0, 1] {
            let mut m = make_concrete(bare.clone()).expect("make");
            m.set_control(1, style);
            m.set_control(2, 3);
            let defs = Module::controls(&m);
            let ControlKind::Popup { items, .. } = &defs[2].kind else { panic!("popup") };
            assert_eq!(items.len(), 12);
            assert_eq!(items[0], "1");
            let mut ctx = ctx();
            let mut pacer = Pacer::new(&m);
            for _ in 0..3000 {
                pacer.advance(&mut ctx);
                m.tick(&mut ctx);
                ctx.sounds.clear();
            }
            m.pick_text(&mut ctx);
            m.round = 1000; // grout mode
            m.pick_text(&mut ctx);
        }
    }

    /// ERRATUM 2: every round re-rolls the layout box. The old code only
    /// re-rolled on round 0, so message after message piled up on the same
    /// origin into an illegible ink blot.
    #[test]
    fn each_round_writes_at_a_fresh_origin() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let mut m = make_concrete(p).expect("make");
        m.set_control(0, 0); // Duration short → tier 1 → 10 s between rounds
        m.set_control(1, 1); // Bathroom Wall
        let mut ctx = ctx();
        let mut origins: Vec<(i32, i32)> = Vec::new();
        let mut rounds = 0;
        // the stroke writer lays 3 dots/frame, so a message is thousands of
        // ticks long — give it room to finish two rounds.
        for _ in 0..40_000 {
            ctx.now_ms += m.tick_ms();
            let before = m.round;
            m.tick(&mut ctx);
            if m.round != before {
                rounds += 1;
            }
            if matches!(m.phase, Phase::Writing) {
                let o = (m.bounds.l, m.bounds.t);
                if origins.last() != Some(&o) {
                    origins.push(o);
                }
            }
        }
        assert!(rounds >= 2, "only {rounds} rounds completed");
        origins.sort_unstable();
        origins.dedup();
        assert!(
            origins.len() >= 2,
            "every round wrote at the same origin {origins:?} — the pileup bug is back"
        );
    }

    /// ERRATUM 4: the engine clock is ms, so the writing pen squeaks often
    /// and the Aortal man's three sound events (2002 spray, 2001 ohh2, 2000
    /// bodyfall) all clear the 1000 ms rerun guard within one cycle.
    ///
    /// §2.2 splits the two sound ranges by scene — fn11 plays 1001–1002 for
    /// arg 1 and 2000–2002 otherwise — so the squeaks are the wall's pen and
    /// the man scene must be silent of them. (A pen squeak over the Aortal
    /// scene was the audible half of the same confusion ERRATUM 6 fixes.)
    fn sound_census(style: i32, secs: u32) -> (std::collections::BTreeSet<u32>, usize) {
        let Some(p) = pack() else { return (Default::default(), usize::MAX) };
        let mut m = make(p).expect("make");
        m.set_control(1, style);
        let mut ctx = ctx();
        let mut pacer = Pacer::new(m.as_ref());
        let mut seen: std::collections::BTreeSet<u32> = Default::default();
        let mut squeaks = 0usize;
        while pacer.now_ms() < secs as u64 * 1000 {
            pacer.advance(&mut ctx);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            for &s in &ctx.sounds {
                seen.insert(s);
                if s == SND_SQUEAK1 || s == SND_SQUEAK2 {
                    squeaks += 1;
                }
            }
        }
        (seen, squeaks)
    }

    #[test]
    fn the_module_is_audible_on_the_millisecond_clock() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let (seen, squeaks) = sound_census(0, 60); // Aortal Squirt
        for id in [SND_SPRAY, SND_OHH2] {
            assert!(seen.contains(&id), "man-SM sound {id} never fired in 60 s");
        }
        // NB 2000 (bodyfall, event 92) is NOT asserted. At the capture's
        // measured 100 ms sprite frame the collapse run 71..75 takes 500 ms
        // and frame 92 lands 400 ms into the lying run — 900 ms after event
        // 71 armed §9.4's single 1000 ms rerun guard (`this+0x17C`, one
        // field for all three man sounds per §8/`0x04A4`), so the guard eats
        // it. The old 120 ms frame put the same event at 1.08 s and it slid
        // through. See the header's open questions.
        assert_eq!(squeaks, 0, "the man scene squeaked like a marker {squeaks}x (§2.2)");

        let (seen, squeaks) = sound_census(1, 60); // Bathroom Wall
        assert!(squeaks >= 10, "only {squeaks} pen squeaks in 60 s at the wall");
        assert!(
            seen.iter().all(|&s| s == SND_SQUEAK1 || s == SND_SQUEAK2),
            "the wall played a man-scene sound: {seen:?}"
        );
    }

    /// series-3000 font map (the Aortal writer; also the wall's fallback when `pens500.json` is
    /// missing): id 10 = '.' (spec §2); '!' = 033; missing art drops as space.
    #[test]
    fn legacy_font_map_spot_checks() {
        assert_eq!(glyph_frame(b'!'), Some(33));
        assert_eq!(glyph_frame(b'.'), Some(46));
        assert_eq!(glyph_frame(b'0'), Some(48));
        assert_eq!(glyph_frame(b'Z'), Some(90));
        assert_eq!(glyph_frame(b'#'), None); // missing art 35
        assert_eq!(glyph_frame(b'~'), None); // outside the table
    }

    // --- §11 stroke writer -------------------------------------------------

    #[test]
    fn json_reader_handles_the_shapes_pens500_uses() {
        let j = json::parse(
            r#"{"a": 1, "b": [1, -2, 3.5], "c": {"kind": "arc", "sweep_reverse": true},
                "\"": null, "d": "x\tyA"}"#,
        )
        .expect("parse");
        assert_eq!(j.get("a").unwrap().int(), 1);
        assert_eq!(j.get("b").unwrap().at(1).int(), -2);
        assert_eq!(j.get("c").unwrap().get("kind").unwrap().text(), "arc");
        assert!(j.get("c").unwrap().get("sweep_reverse").unwrap().is_true());
        assert!(j.get("\"").is_some(), "escaped key");
        assert_eq!(j.get("d").unwrap().text(), "x\tyA");
    }

    /// SANE rounds half to EVEN; Rust's `f64::round` rounds half away from
    /// zero. Every `round(...)` in §11 goes through `rnd`.
    #[test]
    fn rnd_is_round_half_to_even() {
        assert_eq!(rnd(2.5), 2);
        assert_eq!(rnd(3.5), 4);
        assert_eq!(rnd(-2.5), -2);
        assert_eq!(rnd(-1.5), -2);
        assert_eq!(rnd(2.6), 3);
        assert_eq!(rnd(-2.6), -3);
    }

    fn pen_font() -> Option<PenFont> {
        load_pen_font(&pack()?)
    }

    #[test]
    fn pens_500_loads_with_the_grammar_intact() {
        let Some(f) = pen_font() else {
            eprintln!("no pens500.json — skipping");
            return;
        };
        assert_eq!(f.glyphs.len(), 45, "45 glyphs (§11.1)");
        // space is a zero-stroke glyph — fn27 sees '<' immediately.
        let sp = f.glyph(' ').expect("space glyph");
        assert!(sp.groups.is_empty());
        assert_eq!(sp.advance, 35);
        // O is one arc run about the dead centre of the em box (§11.1).
        let o = f.glyph('O').expect("O");
        assert_eq!(o.advance, 93);
        assert_eq!(o.groups.len(), 1);
        assert!(o.groups[0].arc);
        assert_eq!(o.groups[0].centre, (40, 50));
        assert_eq!(o.groups[0].pts.len(), 7);
        // the large brush's cross-section through the core is 2 0 0 0 2
        // (§11.6): brush[bx*5 + 2] for bx 0..4.
        let large: Vec<u8> = (0..5).map(|bx| f.brush[2][bx * 5 + 2]).collect();
        assert_eq!(large, vec![2, 0, 0, 0, 2]);
        // clut 601: six pens x three shades, blood at base 3 (§11.7)
        assert_eq!(f.ramp[3], [0x87, 0x0d, 0x0d]);
        assert_eq!(f.ramp[5], [0xa2, 0x58, 0x5a]);
    }

    /// BUG KEPT (§11.8.1): fn24 cannot parse a minus sign, so `-5` reads as
    /// −25 and `-1` as −29. Those are the leftward serif ticks on B/D/F/P/R
    /// and Y's tail — visible in the golden capture. A port that "fixes"
    /// this will not match.
    #[test]
    fn the_fn24_sign_bug_is_preserved() {
        let Some(f) = pen_font() else {
            eprintln!("no pens500.json — skipping");
            return;
        };
        // B's and R's top-left serif start off the left of the em box.
        for (ch, want) in [('B', -25), ('R', -29)] {
            let g = f.glyph(ch).expect("glyph");
            let serif = g.groups[1].pts[0];
            assert_eq!(serif.0, want, "{ch} serif start x (fn24 sign bug)");
        }
        // Y's final point drops to y = −29.
        let y = f.glyph('Y').expect("Y");
        let last = *y.groups.last().unwrap().pts.last().unwrap();
        assert_eq!(last.1, -29, "Y tail y (fn24 sign bug)");
        // every affected glyph reaches left of x = 0 or below y = 0
        let bugged: Vec<char> = ['B', 'D', 'F', 'P', 'R', 'Y']
            .into_iter()
            .filter(|&c| {
                f.glyph(c)
                    .map(|g| g.groups.iter().any(|gr| gr.pts.iter().any(|p| p.0 < -4 || p.1 < -4)))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(bugged, vec!['B', 'D', 'F', 'P', 'R', 'Y']);
    }

    /// Drive the writer directly and check the raster: large letters, the
    /// 5-pixel stroke cross-section, and the serif tick reaching left of the
    /// pen origin.
    #[test]
    fn the_stroke_writer_paints_large_letters_with_a_rim_core_rim_profile() {
        let Some(f) = pen_font() else {
            eprintln!("no pens500.json — skipping");
            return;
        };
        let mut ink = InkSurface::new();
        let mut ctx = ctx();
        let mut deadline = 0u64;
        // scale 1.0: the box is wide enough that the fit does not shrink it.
        let b = Box2 { l: 40, t: 160, r: 620, b: 400 };
        let mut w = PenWriter::new(&f, "REDRUM", b, 3);
        assert!(w.scale > 0.7, "expected a near-1.0 fit, got {}", w.scale);
        let mut n = 0;
        while !w.step(&f, &mut ink, &mut ctx, &mut deadline) && n < 200_000 {
            n += 1;
        }
        // bounding box of the ink
        let (mut x0, mut x1, mut y0, mut y1) = (SCREEN_W, -1, SCREEN_H, -1);
        let mut painted = 0usize;
        for y in 0..SCREEN_H {
            for x in 0..SCREEN_W {
                if ink.px[(y * SCREEN_W + x) as usize] != NO_INK {
                    painted += 1;
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
            }
        }
        assert!(painted > 3000, "only {painted} ink pixels for REDRUM");
        let h = y1 - y0 + 1;
        assert!(h >= 80, "letters only {h} px tall — should span a tile row");
        assert!(x1 - x0 > 300, "REDRUM only {} px wide", x1 - x0);
        // the R's serif tick reaches left of the pen origin (fn24 sign bug)
        assert!(x0 < b.l, "no ink left of the margin ({x0} >= {})", b.l);

        // Cross-section: scanning a row across a stroke must cross a lighter
        // rim, a darker core, and a lighter rim again — the `1 0 0 0 1` the
        // 2/0/0/0/2 brush profile settles into once the fringes have
        // overlapped and promoted (§11.6). Count maximal inked runs that read
        // rim / core / rim and are stroke-width, not blob-width.
        let lvl = |x: i32, y: i32| -> Option<u8> {
            let v = ink.px[(y * SCREEN_W + x) as usize];
            (v != NO_INK).then(|| v % 3)
        };
        let mut profiles = 0;
        for y in y0..=y1 {
            let mut x = x0;
            while x <= x1 {
                if lvl(x, y).is_none() {
                    x += 1;
                    continue;
                }
                let start = x;
                while x <= x1 && lvl(x, y).is_some() {
                    x += 1;
                }
                let run: Vec<u8> = (start..x).map(|i| lvl(i, y).unwrap()).collect();
                let n = run.len();
                if (3..=9).contains(&n)
                    && run[0] > 0
                    && run[n - 1] > 0
                    && run[1..n - 1].iter().any(|&v| v == 0)
                {
                    profiles += 1;
                }
            }
        }
        assert!(profiles > 20, "only {profiles} rim/core/rim cross-sections");
        // every painted pixel is inside the pen's own 3-slot ramp
        for &v in ink.px.iter() {
            if v != NO_INK {
                assert!((3..6).contains(&v), "slot {v} outside the blood ramp");
            }
        }
    }

    /// The wall writer must draw the message as pen ink spread across the
    /// wall, at the pen origin — not the old series-3000 compound cluster.
    #[test]
    fn wall_ink_is_pen_strokes_spread_from_the_pen_origin() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let Some(mut m) = make(p) else { return };
        m.set_control(1, 1); // Bathroom Wall
        let mut ctx = ctx();
        for _ in 0..900 {
            ctx.now_ms += m.tick_ms();
            m.tick(&mut ctx);
        }
        let mut out = Vec::new();
        m.sprites(&mut out);
        let ink: Vec<&SpriteDraw> = out.iter().filter(|s| s.png.starts_with(STRIP_DIR)).collect();
        assert!(!ink.is_empty(), "no pen ink after 900 ticks");
        assert!(
            out.iter().all(|s| !s.png.contains("/3000/")),
            "series-3000 compound glyphs are back — the stroke writer is not running"
        );
        let xs: Vec<i32> = ink.iter().map(|s| s.x).collect();
        let ys: Vec<i32> = ink.iter().map(|s| s.y).collect();
        let (x0, x1) = (*xs.iter().min().unwrap(), *xs.iter().max().unwrap());
        let (y0, y1) = (*ys.iter().min().unwrap(), *ys.iter().max().unwrap());
        assert!(x1 - x0 > 60, "ink x-range too narrow ({x0}..{x1})");
        assert!(y1 - y0 > 40, "ink y-range too short ({y0}..{y1}) — letters too small");
    }

    /// Run a style until it has laid ink, and return the sprite list.
    fn sprites_after(style: i32, ticks: u32) -> Vec<SpriteDraw> {
        let Some(p) = pack() else { return Vec::new() };
        let Some(mut m) = make(p) else { return Vec::new() };
        m.set_control(1, style);
        let mut ctx = ctx();
        let mut pacer = Pacer::new(m.as_ref());
        while pacer.now_ms() < ticks as u64 * 40 {
            pacer.advance(&mut ctx);
            m.tick(&mut ctx);
        }
        let mut out = Vec::new();
        m.sprites(&mut out);
        out
    }

    /// Every file under a directory, with its size and mtime — enough to
    /// catch a module writing into the pack.
    fn tree_stat(dir: &Path) -> std::collections::BTreeMap<std::path::PathBuf, (u64, std::time::SystemTime)> {
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

    /// THE read-only-bundle property (2026-09-19). The Bathroom Wall's ink
    /// strips are generated from their own names — `Module::generated`
    /// answers for every one the wall emits — and building and running the
    /// module leaves the pack directory byte-for-byte untouched. Before this
    /// commit build() wrote ~200 PNGs into `<pack>/_pens/`, and when that
    /// write failed (an installed `.saver` is read-only) the wall silently
    /// became the legacy glyph stamper instead.
    #[test]
    fn ink_strips_are_generated_and_the_pack_is_never_written() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let root = p.root().to_path_buf();
        let before = tree_stat(&root);
        let Some(mut m) = make(p) else { panic!("message-mayhem module missing") };
        m.set_control(1, 1); // Bathroom Wall
        let mut ctx = ctx();
        let mut pacer = Pacer::new(m.as_ref());
        let mut names: std::collections::BTreeSet<String> = Default::default();
        while pacer.now_ms() < 1200 * 40 {
            pacer.advance(&mut ctx);
            m.tick(&mut ctx);
            let mut out = Vec::new();
            m.sprites(&mut out);
            names.extend(out.iter().map(|s| s.png.clone()));
        }
        let gen: Vec<&String> = names.iter().filter(|n| engine::is_generated(n)).collect();
        assert!(!gen.is_empty(), "the wall laid no generated ink");
        for n in gen {
            let img = m.generated(n).unwrap_or_else(|| panic!("no pixels for {n}"));
            // 1 x W solid, alpha-opaque: what write_ink_strips used to encode
            let w: u32 = n.rsplit("_w").next().unwrap().parse().unwrap();
            assert_eq!((img.w, img.h), (w, 1), "{n}");
            assert_eq!(img.rgba.len(), w as usize * 4, "{n}");
            let px = &img.rgba[0..4];
            assert_eq!(px[3], 0xFF, "{n}");
            assert!(img.rgba.chunks_exact(4).all(|c| c == px), "{n} is not solid");
        }
        assert_eq!(before, tree_stat(&root), "the module wrote into the pack");
    }

    /// ERRATUM 6, the regression this lane exists for: the Aortal scene
    /// writes with the series-3000 blood-drip glyph sprites, whose ink is
    /// baked into the art. It must never emit a `clut 601` pen strip —
    /// that is what put green graffiti on the man's black field.
    #[test]
    fn the_aortal_scene_writes_in_blood_glyphs_not_pen_strokes() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let out = sprites_after(0, 4000);
        let glyphs: Vec<&SpriteDraw> = out.iter().filter(|s| s.png.contains("/3000/")).collect();
        assert!(!glyphs.is_empty(), "the artery wrote nothing in 4000 ticks");
        assert!(
            out.iter().all(|s| !s.png.starts_with(STRIP_DIR)),
            "the man scene emitted clut-601 pen ink — the green-graffiti bug is back"
        );
        // and the ink goes down before the man, who walks in front of it
        let first_ink = out.iter().position(|s| s.png.contains("/3000/"));
        let first_man = out.iter().position(|s| s.png.contains("/2000/"));
        assert!(first_man.is_some(), "no man sprite");
        assert!(first_ink < first_man, "blood glyphs must be drawn behind the man");
    }

    /// The two scenes must not share a backdrop either: the wall is a tile
    /// grid (§5 branch A), the man scene is the bare field (§5 branch B only
    /// calls engine vtable+0x14). "Merged the aorta guy and the bathroom
    /// scenes and got neither right" was the user-visible symptom.
    #[test]
    fn each_scene_keeps_its_own_backdrop() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let wall = sprites_after(1, 1200);
        assert!(
            wall.iter().filter(|s| s.png.contains("/1000/")).count() >= 20,
            "the Bathroom Wall lost its tile grid"
        );
        assert!(
            wall.iter().all(|s| !s.png.contains("/2000/")),
            "the man wandered into the Bathroom Wall"
        );
        let man = sprites_after(0, 4000);
        assert!(
            man.iter().all(|s| !s.png.contains("/1000/")),
            "the tile wall leaked into the Aortal scene"
        );
    }

    /// Ink-ramp binding, per style. The wall's strokes index `clut 601`
    /// (`_pens/` strips named by slot) and the roll only ever selects a
    /// pen base, never a colour outside the clut. The man's glyphs carry a
    /// single flat `#aa0000` baked in — measured in the reel at t=208 as
    /// (145,0,1) = 170 x the ≈0.85 video gamma, with NO second tone, which
    /// is why they cannot be three-level pen strokes (ERRATUM 6b).
    #[test]
    fn ink_ramp_is_bound_per_style() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        // The wall draws from the clut-601 slot ladder…
        let wall = sprites_after(1, 1200);
        let slots: std::collections::BTreeSet<usize> = wall
            .iter()
            .filter(|s| s.png.starts_with(STRIP_DIR))
            .filter_map(|s| {
                (0..RAMP_SLOTS).find(|&i| {
                    (0..STRIP_W.len()).any(|w| strip_path(i, w) == s.png)
                })
            })
            .collect();
        assert!(!slots.is_empty(), "the wall laid no clut-601 ink");
        // one pen = three consecutive slots from a 3-aligned base
        let base = *slots.iter().next().unwrap() / 3 * 3;
        assert!(
            slots.iter().all(|&s| (base..base + 3).contains(&s)),
            "wall ink spans more than one pen ramp: {slots:?} (base {base})"
        );

        // …and the man's ink is the packed art, one flat tone, no ramp.
        let man = sprites_after(0, 4000);
        let mut tones: std::collections::BTreeSet<[u8; 3]> = Default::default();
        for s in man.iter().filter(|s| s.png.contains("/3000/")) {
            assert_eq!(s.pal, 0, "blood glyphs must not be remapped");
            let img = p.image(&s.png);
            for px in img.rgba.chunks_exact(4) {
                if px[3] > 200 {
                    tones.insert([px[0], px[1], px[2]]);
                }
            }
        }
        assert!(!tones.is_empty(), "no blood-glyph pixels");
        assert_eq!(tones.len(), 1, "blood glyphs are not one flat tone: {tones:?}");
        let t = *tones.iter().next().unwrap();
        assert!(
            t[0] > 120 && t[1] < 40 && t[2] < 40,
            "the artery is not writing in blood: {t:?}"
        );
    }

    // ---- Aortal Squirt ratchets (three minimum, §5) ---------------------

    /// Drive the man scene on the Pacer and collect a transition trace.
    fn man_trace(secs: u64, message: i32) -> (MessageMayhem, Vec<(u64, i32, i32, i32, i32, i32)>) {
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0); // Aortal Squirt
        m.set_control(2, message);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut trace = Vec::new();
        let (mut ls, mut lr) = (-1i32, -1i32);
        while pacer.now_ms() < secs * 1000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            if m.man.state != ls || m.man.s.run != lr {
                ls = m.man.state;
                lr = m.man.s.run;
                trace.push((c.now_ms, m.man.state, m.man.s.run, m.man.s.frame, m.man.s.x, m.man.s.y));
            }
        }
        (m, trace)
    }

    /// RATCHET 1 — every run the port names is a real OFst block start in the
    /// pack, and the state machine actually enters all of them (§5 item 2).
    /// Reverting any `SetRun` id to a prose-era guess fails this.
    #[test]
    fn every_named_run_ships_and_is_entered() {
        let Some(p) = pack() else {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        };
        let named = [
            RUN_WALK_L, RUN_WALK_R, RUN_TURN, RUN_SPRAY_A, RUN_SPRAY_B, RUN_STAGGER, RUN_RECOVER,
            RUN_FALL, RUN_CORPSE, RUN_SPURT, RUN_DRIP,
        ];
        for r in named {
            let (first, last) = run_span(&p, S_MAN, r);
            assert_eq!(first, r, "run {r} is not an OFst block start (§10.1)");
            assert!(last > first || r == RUN_CORPSE, "run {r} has no frames");
            assert!(p.frame(S_MAN, r as u32).is_some(), "run {r} has no art");
        }
        // the jet marker only exists on the spraying frames — that is what
        // makes `fn67`'s cursor test fire exactly there
        for f in 36..=49 {
            let has = part_by_art(&p, S_MAN, f, JET_ART).is_some();
            assert_eq!(has, (39..=45).contains(&f), "jet art on frame {f}");
        }
        let (_, trace) = man_trace(180, 1);
        let runs: std::collections::BTreeSet<i32> = trace.iter().map(|t| t.2).collect();
        for r in [RUN_WALK_R, RUN_SPRAY_B, RUN_WALK_L, RUN_SPRAY_A, RUN_TURN] {
            assert!(runs.contains(&r), "run {r} never played; saw {runs:?}");
        }
    }

    /// RATCHET 2 — the machine gets around: states 1..8 all visited, the
    /// message actually gets written, and the cycle restarts (§5 item 3).
    #[test]
    fn the_machine_gets_around_and_writes_the_message() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        // message 4 is long enough to wrap, which is the only way state 4 runs
        let (m, trace) = man_trace(400, 4);
        let states: std::collections::BTreeSet<i32> = trace.iter().map(|t| t.1).collect();
        for s in 1..=8 {
            assert!(states.contains(&s), "state {s} never ran; saw {states:?}");
        }
        // he wrote something, and he wrote it in the right order
        assert!(m.ink.len() + m.man.ci > 0, "the artery never wrote a character");
        // and the layout put the ink on the man's row, not at a rolled box
        let man_h = bank_rect(&m.pack, S_MAN, RUN_SPRAY_A).map(|r| r[3] - r[1]).unwrap();
        let glyph_h = bank_rect(&m.pack, S_FONT, 0x41).map(|r| r[3] - r[1]).unwrap();
        let want_top = m.man.row - (man_h >> 1) - (glyph_h >> 1);
        let first_y = *m.man.char_y.first().expect("laid out");
        assert!(
            (first_y - want_top).abs() <= m.man.line_h * 8 + 5,
            "line 0 at {first_y} is nowhere near the man's head {want_top}"
        );
        // the reset really recycles: state 8 clears the latch
        assert!(
            trace.iter().filter(|t| t.1 == 1).count() >= 2,
            "the scene never started a second victim"
        );
    }

    /// RATCHET 3 — the walk is continuous. Inside a run every step moves the
    /// man by exactly the `fn3DDC` link (flip-aware); across every run
    /// hand-off the part the two frames share stays where it was on screen
    /// (`fn028A` → `fn3F2E`). The 2026-09-26 jerk was the second one
    /// failing: frame 64 → 1 applied no delta and slid his feet 33 px on
    /// every letter. A walk cycle still travels the run's own stride.
    #[test]
    fn the_man_never_teleports_between_frames() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut prev: Option<(Spr, i32)> = None;
        let mut steps = 0usize;
        let mut handoffs = 0usize;
        let mut cycles = 0usize;
        let mut cycle_x0: Option<i32> = None;
        let mut last_state = -1i32;
        while pacer.now_ms() < 240_000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            let here = m.man.s.clone();
            // states 1/4/5/7 place him with SetPos; compare only against the
            // last DIFFERENT drawing, and only within one state
            if let Some((q, _)) = prev.as_ref() {
                if (q.frame, q.x, q.y, q.flip) == (here.frame, here.x, here.y, here.flip) {
                    continue;
                }
            }
            let placed = prev.as_ref().map_or(true, |p| p.1 != m.man.state);
            if m.man.state != last_state {
                last_state = m.man.state;
                cycle_x0 = None;
            }
            if let Some((q, _)) = prev.as_ref().filter(|_| !placed) {
                if q.run == here.run && here.frame == q.frame + 1 {
                    let (dx, dy) = link(&m.pack, S_MAN, q.frame, here.frame, q.flip).unwrap();
                    assert_eq!(
                        (here.x - q.x, here.y - q.y),
                        (dx, dy),
                        "frame {} -> {} did not move by its link at {}ms",
                        q.frame,
                        here.frame,
                        c.now_ms
                    );
                    steps += 1;
                } else if here.frame != q.frame && here.frame == here.run && q.frame > 0 {
                    // a hand-off: the first shared part must not move. A loop
                    // restart inside one UPDATE (state 1/4/5's `+0x7C`) first
                    // advances onto the run's last frame, undrawn — replay it.
                    let mut q = q.clone();
                    let (first, last) = run_span(&m.pack, S_MAN, q.run);
                    if q.frame >= first && q.frame < last {
                        let n = q.frame + 1;
                        q.advance(&m.pack, S_MAN, n);
                    }
                    let (ga, gb) = (
                        m.pack.frame(S_MAN, q.frame as u32).unwrap(),
                        m.pack.frame(S_MAN, here.frame as u32).unwrap(),
                    );
                    if let Some((pa, pb)) =
                        ga.parts.iter().find_map(|a| gb.parts.iter().find(|b| b[0] == a[0]).map(|b| (a, b)))
                    {
                        let a = q.part_screen(&m.pack, S_MAN, pa).unwrap();
                        let b = here.part_screen(&m.pack, S_MAN, pb).unwrap();
                        assert!(
                            (a[0] - b[0]).abs() <= 1 && a[1] == b[1],
                            "hand-off {} -> {} slid part {} from {:?} to {:?} at {}ms",
                            q.frame,
                            here.frame,
                            pa[0],
                            a,
                            b,
                            c.now_ms
                        );
                        handoffs += 1;
                    }
                }
                if m.man.state == 1 && q.run == RUN_WALK_R && here.frame == RUN_WALK_R
                    && q.frame != RUN_WALK_R
                {
                    if let Some(x0) = cycle_x0 {
                        let (first, last) = run_span(&m.pack, S_MAN, RUN_WALK_R);
                        let want = centre(&m.pack, S_MAN, last).unwrap().0
                            - centre(&m.pack, S_MAN, first).unwrap().0;
                        assert_eq!(here.x - x0, want, "walk stride at {}ms", c.now_ms);
                        cycles += 1;
                    }
                    cycle_x0 = Some(here.x);
                }
            }
            prev = Some((here, m.man.state));
        }
        assert!(steps > 500, "only {steps} in-run frame steps sampled");
        assert!(handoffs > 100, "only {handoffs} hand-offs sampled");
        assert!(cycles >= 3, "only {cycles} walk cycles sampled");
    }

    /// RATCHET 4 (2026-09-26 jerk) — the letter boundary. Spray run 53 ends
    /// on frame 64, whose body parts are authored mirrored; `fn3F2E` XORs
    /// the part flags into the sprite, so run 1 plays MIRRORED (he walks
    /// forward) with his feet planted, and the 49 → 28 hand-off flips him
    /// back. The capture's template match shows mirrored 1..10 and 38/39
    /// after every spray B, and top-lefts that match this port's frame for
    /// frame. Reverting to a no-delta hand-off slides the feet +33 px here.
    #[test]
    fn the_letter_boundary_flips_him_in_place() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut prev: Option<Spr> = None;
        let (mut letters, mut backs) = (0usize, 0usize);
        while pacer.now_ms() < 120_000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            let here = m.man.s.clone();
            if let Some(q) = prev.as_ref() {
                let tl = |s: &Spr| s.draw_at(&m.pack, S_MAN).unwrap();
                if q.frame == 64 && here.frame == RUN_WALK_L {
                    assert!(!q.flip && here.flip, "64 -> 1 did not flip him at {}ms", c.now_ms);
                    // frame 64 and frame 1 share the body; same screen box edge
                    assert_eq!(tl(q), tl(&here), "64 -> 1 moved him at {}ms", c.now_ms);
                    letters += 1;
                }
                if q.frame == 49 && here.frame == RUN_TURN {
                    assert!(q.flip && !here.flip, "49 -> 28 did not flip him back at {}ms", c.now_ms);
                    assert_eq!(tl(q), tl(&here), "49 -> 28 moved him at {}ms", c.now_ms);
                    backs += 1;
                }
            }
            prev = Some(here);
        }
        assert!(letters >= 8 && backs >= 8, "only {letters} letter boundaries / {backs} turns");
    }

    /// The cadence: `fn63 @0CF2` re-arms `now + 0x5A` and tests `>=`, which
    /// on the Mac grid is the ~99.8 ms the capture's periodogram found. A
    /// flat 90 (or the prose's 100 on a 40 ms grid = 120) fails this.
    #[test]
    fn the_man_steps_on_the_mac_quantized_ninety() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut steps = Vec::new();
        let (mut lf, mut lt) = (-1i32, 0u64);
        while pacer.now_ms() < 60_000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            if m.man.s.frame != lf {
                if lf >= 0 && m.man.state == 2 {
                    steps.push(c.now_ms - lt);
                }
                lf = m.man.s.frame;
                lt = c.now_ms;
            }
        }
        steps.retain(|d| *d < 200);
        assert!(steps.len() > 30, "only {} frame steps sampled", steps.len());
        let mean = steps.iter().sum::<u64>() as f64 / steps.len() as f64;
        assert!(
            (96.0..104.0).contains(&mean),
            "mean sprite period {mean:.1} ms, capture says ~99.8"
        );
    }

    /// `fn67 @132C`: nothing is written before the artery is open. The blood
    /// jet's marker art exists only on the spraying frames, so ink can only
    /// appear while the man is in run 36 or 53.
    #[test]
    fn no_blood_before_the_jet_marker_is_on_screen() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut n = 0usize;
        while pacer.now_ms() < 120_000 {
            pacer.advance(&mut c);
            let before = m.ink.len();
            m.tick(&mut c);
            if m.ink.len() > before {
                n += 1;
                assert!(
                    m.man.s.run == RUN_SPRAY_A || m.man.s.run == RUN_SPRAY_B,
                    "ink laid down during run {} at {}ms",
                    m.man.s.run,
                    c.now_ms
                );
                assert!(m.man.armed, "ink before the +0x8A0 arm");
            }
        }
        assert!(n > 0, "no ink in two minutes");
    }

    /// The neck spurt (`fn74 @1770`) is pinned to the man's channel-1 marker
    /// and hides whenever he is not in one of the five runs that carry one.
    #[test]
    fn the_spurt_rides_the_neck_marker() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut checked = 0usize;
        while pacer.now_ms() < 120_000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            if !m.spurt.s.shown || m.spurt.s.frame == 0 {
                continue;
            }
            let (Some(mp), Some(sp)) = (
                part_by_chan(&m.pack, S_MAN, m.man.s.frame, MARK_CH),
                part_by_chan(&m.pack, S_MAN, m.spurt.s.frame, MARK_CH),
            ) else {
                continue;
            };
            let a = m.man.s.part_screen(&m.pack, S_MAN, &mp).unwrap();
            let b = m.spurt.s.part_screen(&m.pack, S_MAN, &sp).unwrap();
            let (a, b) = ((a[0], a[1]), (b[0], b[1]));
            assert_eq!(a, b, "spurt marker adrift from the man's");
            checked += 1;
        }
        assert!(checked > 20, "the spurt never showed ({checked} samples)");
    }

    /// The sound events are keyed off the FRAME the art is showing, with the
    /// 1000 ms re-run guard on the two repeatable ones (`fn61 @0450`).
    #[test]
    fn the_man_sounds_on_his_own_frames() {
        if pack().is_none() {
            eprintln!("assets/message-mayhem missing — skipping");
            return;
        }
        let p = pack().expect("pack");
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut seen = std::collections::BTreeSet::new();
        let mut last_spray = 0u64;
        while pacer.now_ms() < 400_000 {
            pacer.advance(&mut c);
            c.sounds.clear();
            m.tick(&mut c);
            for s in c.sounds.iter() {
                seen.insert(*s);
                assert!(
                    [SND_SPRAY, SND_OHH2, SND_BODYFALL].contains(s),
                    "the man scene fired a wall sound {s}"
                );
                if *s == SND_SPRAY {
                    assert!(
                        last_spray == 0 || c.now_ms - last_spray > SND_GUARD_MS,
                        "spray re-fired inside the 1000 ms guard"
                    );
                    last_spray = c.now_ms;
                }
            }
        }
        assert!(seen.contains(&SND_SPRAY), "the artery never sprayed; saw {seen:?}");
        assert!(seen.contains(&SND_BODYFALL), "no body fall; saw {seen:?}");
    }

    /// The per-tick trace, for eyeballing hand-offs (§5's ignored trace).
    #[test]
    #[ignore]
    fn trace_the_man() {
        if pack().is_none() {
            return;
        }
        let (m, trace) = man_trace(300, 4);
        for (t, s, r, f, x, y) in trace {
            println!("t={t:>7}ms state={s} run={r:>3} frame={f:>3} pos=({x:>5},{y:>4})");
        }
        println!(
            "msg={:?} ci={} row={} line_h={} ink={} drips={}",
            String::from_utf8_lossy(&m.man.msg),
            m.man.ci,
            m.man.row,
            m.man.line_h,
            m.ink.len(),
            m.man.drips
        );
    }

    /// The man's grey-body centroid x inside frame `f`'s art (the same mask
    /// the capture measurement uses: neutral grey, mid luminance). This is
    /// what the eye follows, independent of how wide the jet makes the frame.
    fn body_cx(p: &Pack, f: i32) -> f64 {
        let Some(g) = p.frame(S_MAN, f.max(0) as u32) else { return 0.0 };
        let img = p.image(&g.png);
        let (mut sx, mut n) = (0f64, 0f64);
        for y in 0..img.h {
            for x in 0..img.w {
                let i = ((y * img.w + x) * 4) as usize;
                let px = &img.rgba[i..i + 4];
                if px[3] == 0 {
                    continue;
                }
                let (r, gg, b) = (px[0] as i32, px[1] as i32, px[2] as i32);
                let mx = r.max(gg).max(b);
                let mn = r.min(gg).min(b);
                let mean = (r + gg + b) as f64 / 3.0;
                if mx - mn < 30 && mean > 70.0 && mean < 215.0 {
                    sx += x as f64;
                    n += 1.0;
                }
            }
        }
        if n > 0.0 { sx / n } else { img.w as f64 / 2.0 }
    }

    /// The man's feet in frame `f`'s art: `(min x, max x)` of the grey
    /// pixels in the bottom 20 rows of the figure — a planted foot is the
    /// sharpest "did he slide" landmark the capture offers.
    fn feet_x(p: &Pack, f: i32) -> (i32, i32) {
        let Some(g) = p.frame(S_MAN, f.max(0) as u32) else { return (0, 0) };
        let img = p.image(&g.png);
        let grey = |x: u32, y: u32| {
            let i = ((y * img.w + x) * 4) as usize;
            let px = &img.rgba[i..i + 4];
            let (r, gg, b) = (px[0] as i32, px[1] as i32, px[2] as i32);
            let mean = (r + gg + b) as f64 / 3.0;
            px[3] != 0 && r.max(gg).max(b) - r.min(gg).min(b) < 30 && mean > 50.0 && mean < 215.0
        };
        let mut yb = 0;
        for y in 0..img.h {
            for x in 0..img.w {
                if grey(x, y) {
                    yb = y;
                }
            }
        }
        let (mut lo, mut hi) = (i32::MAX, i32::MIN);
        for y in yb.saturating_sub(19)..=yb {
            for x in 0..img.w {
                if grey(x, y) {
                    lo = lo.min(x as i32);
                    hi = hi.max(x as i32);
                }
            }
        }
        (lo, hi)
    }

    /// Per-tick trace of the man across letter boundaries: every tick his
    /// frame, run or state changes — state, run, frame, centre, the delta
    /// applied since the last line, the drawn top-left, and the letter index.
    #[test]
    #[ignore]
    fn trace_the_man_per_tick() {
        let Some(p) = pack() else { return };
        let mut m = make_concrete(p).expect("module");
        m.set_control(1, 0);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut last: Option<(i32, i32, i32, i32, i32)> = None;
        while pacer.now_ms() < 30_000 {
            pacer.advance(&mut c);
            m.tick(&mut c);
            let s = &m.man.s;
            let here = (m.man.state, s.run, s.frame, s.x, s.y);
            let moved = last.map_or(true, |q| q != here);
            if moved {
                let (dx, dy) = last.map_or((0, 0), |q| (s.x - q.3, s.y - q.4));
                let tl = s.draw_at(&m.pack, S_MAN).unwrap_or((0, 0));
                let w = m.pack.frame(S_MAN, s.frame.max(0) as u32).map_or(0, |g| g.w);
                let (mut cx, (mut fl, mut fh)) = (body_cx(&m.pack, s.frame), feet_x(&m.pack, s.frame));
                if s.flip {
                    // the art is drawn mirrored inside its own rect
                    cx = (w - 1) as f64 - cx;
                    (fl, fh) = (w - 1 - fh, w - 1 - fl);
                }
                let bx = tl.0 as f64 + cx;
                println!(
                    "t={:>6} st={} run={:>3} f={:>3} pos=({:>4},{:>4}) d=({:>4},{:>3}) tl=({:>4},{:>4}) body={:>6.1} feet={}..{} flip={} ci={}",
                    c.now_ms, here.0, here.1, here.2, here.3, here.4, dx, dy, tl.0, tl.1, bx,
                    tl.0 + fl, tl.0 + fh, s.flip as u8, m.man.ci
                );
            } else {
                println!("t={:>6} (hold f={})", c.now_ms, s.frame);
            }
            last = Some(here);
        }
    }
}
