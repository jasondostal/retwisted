//! Coming Soon! — full-fidelity transcription of the RE spec
//! (totally-twisted docs/behavior/coming-soon.md; every constant is
//! disasm-cited there). Parody late-night movie trailers: an eyeballs-mascot
//! salesman hawks 19 fake software products, one poster compound each.
//!
//! Faithful behavior implemented here:
//! - Controls (§2): Pace slider (default 22 "Mellow"), Show popup
//!   (Random/In Order), dead control slot #2 (always stored as 0 into
//!   `g03EA`, whose one reader — fn168's gesture roll — therefore never fires),
//!   Music slider (default 22 "Once")
//! - Pace → trailer dwell `g0052[bucket]/2` = {10000,5000,2500,1250,0} ms
//!   (§2.1); ORIGINAL BUG kept: bucket 4 reads the zero word past the table,
//!   so "Fast" = 0 dwell and the crawl-line roll becomes RandomBelow(1)
//! - Show → playlist order (§2.2): g03EE = raw-1; 0 → random path
//!   RandomBelow(19) + skip-loop, nonzero → in-order scan from 0; shipped
//!   default raw 0 → g03EE = -1 → In Order out of the box (§13.3, bug kept)
//! - Music → repeat count g03F2 = raw/20 (§2.3); ORIGINAL BUG kept: the
//!   "≥99 → -1" branch tests the DIVIDED value and can never fire, so
//!   DriveYouCrazy (99) == Four Times (§13.1)
//! - Master state machine (fn149 @0x0CBE, transcribed 2026-09-29 — see
//!   the g03EC ERRATUM): +1000 ms title card (@0x110A); state 2 ends on
//!   the card's msg 4 (type-on done) and its EXIT re-arms a full dwell
//!   (@0x153A, overwriting the dead +2000 of @0x149E); state 3 rolls
//!   g03EC = RandomBelow(5-bucket)==0 (@0x165E); state 4 waits one more
//!   dwell (g03EC clear) or the mascot's walk-off (g03EC set) before state 0,
//!   which re-stages only when g03EC is set
//! - Playlist (§2.2/§6): 19 trailers, once-per-cycle flags, full cycle reset
//! - Poster compounds (§7): art series 20000, movie i → compound frameNum
//!   POSTER_FNO[i] (29…138), the compound bounds CENTRED on the stage point
//!   (STAGE_CX, STAGE_CY) — golden-measured, see LAYOUT below
//! - Mascot (§8): art series 10000, 29 sequence runs stepped frame by frame;
//!   voice events at frame thresholds 17→slot 6, 200→slot 0,
//!   {126,168,237,278,318,357,390}→slot 1+RandomBelow(5)
//! - Voice queue (§10 f1BAE): 7 slots = snd 10000+i; request dropped while
//!   the previous voice is further than 180 ms from its end; new deadline =
//!   now + duration − 180
//! - Mascot gesture machine (fn168 @0x22CA states 0-0xE, chooser fn167
//!   @0x2154, stepper fn166 @0x1F64) — see the GESTURE ERRATUM. (The spec's
//!   "eyeball/lip-sync picker f2158" is fn167, and it picks gestures.)
//! - Text crawl (§9): STR# 500 runtime-rendered through the engine
//!   TextDraw API with the `#FR0/#FR1`, `#Jc/#Jl/#Jr`, `#R` markup
//!   semantics — title card (item 1) from the title phase on, bullets
//!   (items 2–12) typed on in one run (see the crawl errata; the old "one
//!   per g03EC cycle" is SUPERSEDED); justification is sticky
//!   across `#R`, preserving the §13.4 typos (#Jl Conspiracy card,
//!   #Jr Cross Dressing card)
//! - Click-to-skip (§5 states 2/3/4, f41E0; §13.6): held left button
//!   polled per tick → substate=1, deadline=0, fast-forwarding the trailer
//! - Runtime palette (§6/Appendix B): per-movie clut 20000+i applied to the
//!   poster compound via SpriteDraw.pal (LoadCLUT recolour in place; clut
//!   ids 20000..20018 verified in the pack meta.json). This pack has NO
//!   `base_clut`, so the engine leaves the poster colours alone — see
//!   APPROXIMATIONS.
//! - AD message line (§3 @0x0756): boot tagline STR# 128+RandomBelow(5)
//!   pushed to the message line, rendered along the bottom edge each frame
//!
//! ## LAYOUT (Basilisk DEPTH=32 golden capture 2026-09-01,
//! `d32-coming-soon/g_02..g_11`; an earlier 8-bit capture of the same demo
//! is unusable for colour — Basilisk's broken 8-bit palette path rendered
//! the field white — but agrees with this one on every measurement below
//! except a whole-stage vertical offset, see STAGE_CY)
//!
//! The golden shows the trailer as ONE CARD: a 358×300 coloured panel with
//! the title across its top, a column of bullet lines with an accent tick
//! beside each, the poster art, and the BERKELEY SYSTEMS logo, all on the
//! black field, with the mascot spotlight low on the left.
//! Everything about that card is recoverable from the `OFtb 20000` compound
//! part tables (`scripts/oftb_compose.py`), which are in ABSOLUTE design
//! coordinates — the crawl was never "floating text over a poster", the
//! poster compound *is* the page layout:
//!
//! | channel | art | what it is |
//! |---|---|---|
//! | ch1  | 87 | the card panel — always 358×300 |
//! | ch3/ch4/ch5/ch6 | 85/87 | decorative blobs / the spotlight ellipse behind the poster art |
//! | ch8  | 88 | **the title text block** |
//! | ch9  | 88 | **the bullet text block** |
//! | ch11/ch12/ch13 | 1/2/3-7 | BERKELEY SYSTEMS logo + EDUTAINMENT strip |
//! | ch17 | per-movie | the poster art (bank 20001+m) |
//!
//! Verified against the capture: for the Alien card the ch8 rect maps to
//! screen (141,55)-(499,126) and the golden's two title lines occupy
//! y 58..113 inside it, both centred on x 320 = the card's centre; the ch9
//! rect maps to (154,142)-(331,331) and the golden's first bullet starts at
//! (155, 144) with its last at y 308. Same fit on the Tippy card, and the
//! same fit again in the 8-bit capture. So `POSTER_TITLE_BOX` /
//! `POSTER_BULLET_BOX` below are the original's own text rects, not a guess.
//! Golden bullet metrics: 15 px line pitch, +7 px extra between items.
//!
//! Compound placement: the compound bounds are CENTRED on the stage point,
//! and the stage point is ROLLED PER TRAILER by the placement block at
//! listing @0x0E68-0x0F8C. RULE RECOVERED 2026-09-19 from the decompile —
//! the note that used to stand here ("per-trailer, but no rule is
//! recoverable from four samples", pinning `STAGE_CY = 197`) is retired.
//! See `ComingSoon::roll_stage_point` for the line-by-line port; in short:
//!
//!   the poster sequence's own bounds rect (`g03BA->GetBounds`, vtbl+0x1C
//!   @0x0E76) is normalised to the origin (@0x0E80), moved to the stage's
//!   top-left (@0x0E90), and — if it then fits STRICTLY inside the stage
//!   (@0x0EC4, @0x0ECE) — offset again by `RandomBelow(slack)` in each axis
//!   (@0x0F1C-0x0F42). The stage point is the centre of the page where it
//!   landed (@0x0F4C-0x0F88). A page that does not fit falls back to the
//!   centre of the stage (@0x0ED4).
//!
//! The rect is the bank-wide bound of series 20000 — the bounds call takes
//! no frame argument, and the card sprite's ctor (@0x2D42) makes the same
//! call before any frameNum exists — which the pack measures at
//! (2,18)-(640,407) = **638 x 389**. On the 640x480 stage that gives
//! `x = 319 + RandomBelow(2)` and `y = 194 + RandomBelow(91)`, and THAT is
//! why the two halves of the old note looked so different:
//!
//! * **Horizontal "centring" was the rule's two-pixel range all along.**
//!   Every trailer in every capture reads centre 320 because the page is
//!   638 wide on a 640 stage. Re-measured 2026-09-13 on `coming-soon.mp4`
//!   (crop `1276:958:2:56`, decimate 2:1): the 358-wide card panel spans
//!   x 140-497 at t = 30 and x 139-496 at t = 43. The one-pixel
//!   disagreement the old note wrote off as threshold slop is the roll.
//! * **Vertically the rule spans 194..284**, which covers the run's three
//!   trailers (197, 197, 254 — the third's 358x300 card spans y 104-403)
//!   and the reel's 229. The 8-bit capture's 289 is five pixels outside it;
//!   that capture is already known to carry a whole-stage vertical offset
//!   (see the header of this section), so it does not falsify the range.
//!   The arithmetic the old note worked out stays correct — Tippy's 388x369
//!   compound lands its card at (156,13) and the Alien's 358x300 at
//!   (141,47) — it is just `(stage_cx - w/2, stage_cy - h/2)` at the
//!   golden's stage point (320,197) rather than at a constant.
//!
//! Per-movie `clut 20000+m` turns out to be the CARD DESIGN PALETTE, not an
//! opaque accent table: slot 0 = the panel colour, slots 5/6 = the text ink,
//! slot 7 = the tick/accent. Confirmed on all 19 — Tippy is rose panel /
//! cream ink / red tick, the Alien is navy / grey / slate, and movie 19
//! "The White Saver" is slot 0 = (249,249,249), i.e. the joke card really is
//! a blank white one. That is now the source of the crawl's colours.
//!
//! ## APPROXIMATIONS (engine capability gaps, documented per the brief)
//!
//! 1. **Text rendering font (new approximation class).** The original drew
//!    the crawl through the class-3 glyph machinery via styled text draw
//!    (Resource.f4172, §9) — proportional Mac fonts at unrecovered sizes
//!    and colors. Ours is the engine's 8×8 bitmap font: wider per
//!    character (~8 px/char against the original's ~5), non-ASCII (™)
//!    renders as '?'. Text is now laid out INSIDE the ch8/ch9 rects and
//!    word-wrapped to them, so the block position and hierarchy match the
//!    golden even though the face does not; a long bullet takes ~1.7× the
//!    golden's line count and the tail is clipped at the block bottom.
//!    The §14.6 reveal itself is no longer a guess — it is a left-to-right
//!    per-glyph type-on and we do that (see the 2026-09-12 errata), but we
//!    meter it in SOURCE characters, not pixels, so the wider engine glyph
//!    does not stretch the trailer. The AD message line's on-screen region
//!    is host UI; we draw ours along the bottom edge of the sim screen.
//! 2. **MIDI: no longer an approximation.** The tune is real and packed —
//!    see MUSIC ERRATUM 1. `TUNE_MS` survives only as the fallback for a
//!    pack built before the music section existed.
//! 3. ~~**Mascot sequence selection** is not in the spec; sequences are
//!    stepped in pack order~~ SUPERSEDED 2026-09-29: transcribed from fn168
//!    (GESTURE ERRATUM). The L135 link (fn166's erase/link/draw through
//!    vtbl+0x68/+0x78/+0x6C) is kept as the fixed design offset of
//!    `place_mascot`: all 297 series-10000 records share the design
//!    top-left (47,44), so the link never moves the box.
//! 4. **Silhouette series 13000/13010 are the bullet tick's draw-on frames**
//!    (§14.4, see the errata). Both are black-baked single-colour art and the
//!    engine has no recolour path for them (no `base_clut`), so the growing
//!    slash is not animated; the tick is drawn once, at the golden's size and
//!    offset, in `clut[7]`, with the engine font's `/` standing in for the
//!    27×25 brush stroke.
//! 5. **Missing art ids 84–88 — THE CARD PANEL IS ONE OF THEM.** §7's
//!    "known residual" (art ids 84–88, 124, 125, 144, 145 exist in no RLEP
//!    bank in the rip) is not a cosmetic gap on unused frameNums: **art 87
//!    is the ch1 card panel and art 85 is the ellipse/blob behind the
//!    poster art, on every one of the 19 poster layouts**. The packed
//!    compound PNGs are therefore the poster art + logo alone, floating on
//!    transparency — c_029 is 388×369 with only 8768 opaque pixels. Nothing
//!    in the pack can stand in for the panel: the module's only drawing
//!    primitives are `SpriteDraw` (a packed png at a point — no fill, no
//!    scale) and `TextDraw` (drawn AFTER all sprites, so a glyph-tiled
//!    "panel" would paint over the poster art), and the largest fully
//!    opaque image in the whole pack is 100×100. **So the card renders
//!    without its panel**: the art, the logo and the crawl land in exactly
//!    the golden's positions, on white instead of on rose/navy. Fixing this
//!    needs either art 87/88 recovered by a re-rip (see the spec's UNCERTAIN
//!    note about a second RLEP family) or a filled-rect primitive in the
//!    engine — both outside this change's scope.
//! 5b. **Crawl ink falls back where the missing panel would hide it.** The
//!    original is `clut[5]` ink on a `clut[0]` panel, and `ink()` uses slot
//!    5 — cream on Tippy, tan-grey on the Aliens, exactly the golden. But
//!    two movies (Cyber Ed, Cross Dressing Lawyers) ink in pure black, which
//!    is legible only *because* of the panel; without it they would vanish
//!    into the black field. So `ink()` walks slot 5 → slot 0 → slot 7 and
//!    takes the first that separates from the field, then
//!    `contrast_ink()`. When art 87 arrives, drop the fallback chain and
//!    draw slot 5 on a slot-0 panel unconditionally.
//!    Compounds for frameNums 5/10/15/20/24 compose with empty part lists
//!    for the same reason; we simply never select those frameNums (they are
//!    not among the 19 poster layouts).
//! 6. ~~Deadline hand-off between states 2→3→4 is UNCERTAIN~~ SUPERSEDED
//!    2026-09-29: transcribed from fn149 (see the g03EC ERRATUM). What is
//!    still approximate is state 2's trigger: the card sprite (fn174) is not
//!    ported, so its msg 4 is modelled as "the source-character type-on
//!    budget is spent" (`type_on_done`).
//! 7. **Per-movie clut recolour is a no-op** (§6/Appendix B). `SpriteDraw
//!    .pal` is still set faithfully to 20000+movie, but the pack declares no
//!    `base_clut`, so the engine has no slot space to remap through and the
//!    poster draws in its baked colours. Why: a LoadCLUT is only meaningful
//!    against the device palette the art was rendered in, and here that is
//!    `clut 10000` ("Final 256 all", 208 entries — 172 of the 174 distinct
//!    poster colours come from it), which is a SORTED colour list, so its
//!    index carries no device-slot meaning. The per-movie cluts are 10-entry
//!    accent tables, and each poster is per-movie ART (bank 20001+m) that
//!    already carries the movie's colours. Pass-through is the honest
//!    reading; anything else is a guess. (Before the slot-index fix this
//!    zipped the 24-colour marquee CTAB against the 10-entry movie clut and
//!    sprayed rust/orange over the posters and the Berkeley logo.)
//! 8. **Mascot black backing: NOT an approximation — it is authentic.**
//!    291 of the 294 92×140 series-10000 compounds carry a solid black
//!    block below the spotlight circle (rows 86+). On the black field it is
//!    invisible, and the DEPTH=32 golden confirms it: the mascot reads as a
//!    bare spotlight disc with nothing under it. No masking, colour-key or
//!    crop is needed. (The 8-bit capture's white field made this look like
//!    a mask bug; it was not.)
//!
//! ## MUSIC ERRATA — 2026-09-13 QEMU audio pass
//!
//! 1. **The Music control plays the SHARED bank's `cmid` 20 "Coming
//!    Soon.5", not the module's own `cmid` 10 / `cmid` 410.** §10 and
//!    §14.3 both assume the module file's own pair, and the module really
//!    does carry them (`ripped/coming-soon/Coming Soon_cmid_10.midi`,
//!    "Rhumba Data", 6 tracks, two tempos, 74.40 s). They are not what
//!    sounded. Against `emu/captures/qemu/coming-soon.wav` (60 s, wav ≈
//!    wall, panel Music = Once), an offline render of the shared `cmid` 20
//!    scores **46.0 %** strongest-partial agreement over 63 windows at a
//!    single global lag, and a render of "Rhumba Data" scores **7.5 %**.
//!    The winning lag, +34.70 s into a two-play render, is 1.70 s into
//!    play 2 — the same +1.70 s offset the windows t = 0–20 s (f0–f400)
//!    and t = 20–36 s (f400–f720) independently fit, with 90.5 % of 74
//!    onsets inside 80 ms. §14.3's "which of `cmid` 10 / `cmid` 410 plays
//!    when" has the answer "neither, in a stock Music = Once run".
//!
//!    A further tell, independent of the audio: "Rhumba Data" selects GM
//!    programs 71 (clarinet) and 73 (flute), and the shared bank has no
//!    `INST` for either — two of its five voices would be silent.
//!
//! 2. **The tune is 33.00 s, not 30.** `TUNE_MS` was a guess; the packed
//!    SMF's end-of-track is the real number and `tune_ms()` reads it.
//!
//! 3. **A music replay must not clear `ctx.sounds`.** The old state-2
//!    replay did, on the "single music channel" reading. It is not a
//!    single channel: the MDRV synth is its own, and the capture shows the
//!    tune running unbroken across the salesman's voice cues at t = 4.84
//!    (f97), 13.80 (f276) and 19.01 (f380) — and the tune's own kick drum
//!    sounding at t = 12.82 (f256), between two of them. Clearing the sfx
//!    queue on a replay would have eaten a voice line that happened to
//!    land on the same tick.
//!
//! ## ERRATA — tick-quantization, 2026-09-12 capture wave (FIXED; the
//! 80 ms module gate it introduced is SUPERSEDED by the GESTURE ERRATUM —
//! the 5 ticks are fn168's 75 ms IDLE delay, and the entry run is 100 ms)
//!
//! - **The mascot steps every 83 ms = exactly 5 Mac ticks, not every 40.**
//!   `Resource.f4724()` is `TickCount()*16.625` (§0 table) and §11 says
//!   pacing is "entirely deadline-driven against `Resource.f4724`", so any
//!   delay D fires on the first tick at or past D — and **83.125 ms is 5
//!   ticks**, the quantization of an 80 ms delay (the same 5-tick figure the
//!   bungee lane found for its bounce/snap phases). Measured on
//!   `emu/captures/coming-soon.mp4`: whole-screen over t = 30-50 s the
//!   Rayleigh periodogram peaks at **83.28 ms, R = 0.229** with R = 0.012 at
//!   a flat 100; the sweep's spotlight box (module px 18,265-110,355) gave
//!   **83.10 ms, R = 0.569**. Two independent windows, both 5.00 ticks.
//! - The port advanced §8's mascot sequence once per 40 ms shell tick —
//!   **2x fast**. §8 tables the voice-event frame thresholds but no
//!   per-frame delay, so the 80 ms in `MS_FRAME` is recovered from the
//!   capture, not the disasm. The deadline-driven state machine (§5's
//!   1000/2000 ms and the Pace dwells) is unaffected — those are big enough
//!   that one tick of quantization is under 2 %.
//!
//! ## ERRATA — crawl/scene pass, 2026-09-12 capture wave
//!
//! Evidence below is `emu/captures/coming-soon.mp4` (45 s, DEPTH=32,
//! 2026-09-01; panel Pace = Mellow, Show = "Random", Music = Once) and the
//! `full-reel.mp4` Coming Soon segment (t = 63-67 s). Video frame f = 30 fps,
//! so t = f/30; module px = capture px / 2 after `crop=1276:958:2:56`.
//!
//! - **§14.6 ANSWERED: the crawl does not scroll — it TYPES ON.** Every line
//!   is laid out at its FINAL justified position and then filled in left to
//!   right, glyph by glyph. Tippy's title line is anchored at module x 189
//!   from the very first frame it exists (f205, only the `T` inked) and its
//!   right edge walks 201 → 264 → 318 → 380 → 424 → 444 → 480 over f205-f211,
//!   which are exactly the right edges of glyphs 1, 5, 9, 13, 15, 16 and 18 of
//!   "Tippy the Tapeworm". No line ever moves. Rates: the `#FR0` title runs
//!   36 glyphs in 400 ms (f205-f217) = **~90 glyphs/s**; the `#FR1` bullets run
//!   146 glyphs in 3.200 s (Tippy, f217-f313) and 226 glyphs in 4.867 s
//!   (the Aliens, t 21.933-26.800) = **~46 glyphs/s**. `CRAWL_TITLE_CPS` /
//!   `CRAWL_BULLET_CPS`.
//! - **The bullets are NOT revealed one per dwell cycle; they are a random
//!   SUBSET of the 11, all typed in one unbroken run at the head of the
//!   trailer.** Four cards on film: Tippy = items {1,2,6} (f614), the Aliens =
//!   {1,3,8,11} (f1000), Chernobyl = {1,6,11} (f1300) and — the case that kills
//!   "line 1 always leads" — the reel's Tippy = **{2,4,6}** (reel t≈66 s).
//!   Counts 3/4/3/3 against 11 slots is p ≈ 0.30, i.e. §5's own
//!   `RandomBelow(5 - paceBucket) == 0` = 1/4 at Mellow, rolled **per line
//!   slot** rather than per dwell cycle. The bullet ink area grows on very
//!   nearly every video frame from the card's first frame to the last glyph
//!   (no plateau longer than 2 frames) and is then frozen for 10.1 s (Tippy)
//!   / 11.4 s (Aliens). So the port's "one line per g03EC cycle, 5 s apart"
//!   is FALSIFIED; the rolls are hoisted to `start_movie` here.
//! - **State 1 (the "title card phase") draws NOTHING.** The stage between
//!   trailers is the bare field plus the mascot for 1.033 s (f615-f645) and
//!   1.067 s (f1147-f1178) — that is §5's `+1000 ms` @0x110A, and the card
//!   (panel, poster, logo AND the first title glyph) all appear together on
//!   the first frame of state 2. The port used to draw the card from state 1.
//! - **The "ghost head" is a real DROP SHADOW and the golden has it.** The
//!   module used to claim (in `sprites()` and in the single-emission test)
//!   that the doubled salesman was a rip defect absent from the golden. It is
//!   not: at f614 the mascot's spotlight carries a hard black head-and-shoulder
//!   silhouette up-and-right of the salesman. totally-twisted `51349d6` traced
//!   it to the compound walker's `this+0x11E` latch (set for chanNum 3..8,
//!   XORed into the blit flags → MASKED blit) and re-composed the pack, so the
//!   packed art is now correct **and authentic**. Single-emission still holds
//!   (one disc on screen, always).
//! - **The mascot is not always on stage, and the stage is re-placed per
//!   trailer — RULE NOW RECOVERED for the card, still open for the mascot.**
//!   The spotlight sits at module bbox (23,266)-(111,351) for trailers 1 and
//!   2 (t 5.767-33.100), then vanishes completely for 5.03 s (t 33.200-
//!   38.233 — nothing but field outside the card) and comes back at
//!   (526,298)-(613,381) for trailer 3. Card compound centres are y 197,
//!   y 197, **y 254** in this run and y 229 in the reel (Tippy panel at
//!   (155,45) there vs (156,13) here), against the 8-bit capture's 289.
//!   The CARD half is now ported from the decompile (@0x0E68-0x0F8C, see
//!   `roll_stage_point`) and the measured centres fall inside the rule's
//!   194..284 band. ~~The MASCOT half is NOT ported … `MASCOT_DX/DY` stays
//!   a pin … GAP(mascot side/anchor)~~ SUPERSEDED 2026-09-29: vtbl+0x94 is
//!   L135 `fn4488` (a frame's channel rect with the frame centred on a
//!   point), the four `f3EBE` rects are frame 5's ch1/ch2 (the left/right
//!   spotlight zones) and the poster's ch1 (the card), and the side coin +
//!   hang rule is ported as `place_mascot` — side 1 is the RIGHT zone and
//!   draws MIRRORED. It reproduces all three QEMU placements to the pixel,
//!   including the first one's +24 px (see `place_mascot`). The mascot now
//!   leaves and returns only through the g03EC walk-off (ERRATUM below), so
//!   "the mascot jumps sides while the card's x does not move" is the coin
//!   on a re-stage.
//! - ~~The re-place is gated on `g03EC`, and the port's `g03EC` is already
//!   diverged … GAP(g03EC gating)~~ SUPERSEDED 2026-09-29 by the g03EC
//!   ERRATUM below: the gate is now ported and the port no longer rolls on
//!   every trailer.
//! - **§14.4 (13000/13010 role) LARGELY ANSWERED: they are the bullet tick.**
//!   `OFtb 20000` frameNum **1** (the entry §7 notes appears twice, at both
//!   ends of the table) is a **27×25** compound at design bounds (311,221) —
//!   art id 2 from bank 20000, the one art §7 lists but assigns to no channel.
//!   That is the slash: measured on Tippy's card the ticks are 27 px wide and
//!   25 px tall (f614, e.g. y 94-118 at x 293-319), in `clut[7]` crimson
//!   (181,11,52) ≈ the pack's (204,35,62) × the encoder's 0.84. Series 13000
//!   (all h = 33, w 3..27, design by = 223) and 13010 (all h = 15, w 3..13,
//!   design by = 233) are the SAME slash at the SAME design anchor in growing
//!   widths — the tick's own draw-on animation, not stage shadows. We draw the
//!   static tick at the golden's size/offset; the growth animation is not
//!   modelled (no recolour path for a black-baked sprite — APPROXIMATIONS #7).
//! - Panel/ink/accent from `clut 20000+m` re-confirmed to the pixel: Tippy's
//!   panel reads (134,84,90) on film against the pack's slot 0 (153,102,102),
//!   ink (252,255,200) vs slot 5 (255,255,204), tick (181,11,52) vs slot 7
//!   (204,35,62) — all the 0.84 encoder curve. The Aliens' panel bbox is
//!   x 141-498, y 47-346 = 358×300 at (141,47), i.e. `poster_origin()`
//!   exactly, at that trailer's rolled stage point (320,197).
//! - **Shipped-default Show bug CONFIRMED on film.** The panel in the capture
//!   reads "Show: Random" (f60) and the module still plays **in order** —
//!   Tippy, then the Aliens, then Chernobyl (movies 0,1,2). That is §13.3:
//!   mVal 1001 ships 0, `g03EE = raw - 1 = -1`, so the nonzero/in-order arm
//!   runs while the popup displays its first item.
//! - ~~UNRESOLVED: what sets the trailer length.~~ ANSWERED 2026-09-29 by
//!   the g03EC ERRATUM: type-on + two dwells (5 s + 5 s at Mellow), plus
//!   the mascot's walk-off when g03EC is set. 13.667 s = 3.567 s of type-on
//!   + 10.1; the 16.700 s card was a walk-off trailer (its mascot vanished).
//!
//! ## g03EC ERRATUM — 2026-09-29, fn149 @0x0CBE against the QEMU golden
//!
//! `g03EC` is not "type one more crawl line" (the RE spec's reading, which
//! the port carried as `crawl_flag` and looped state 4 → 3 on). In the C it
//! is **"re-stage on the next state 0"**: the ctor sets it (@0x0C26), every
//! state-3 exit clears it and re-rolls it with `RandomBelow(5 - bucket) ==
//! 0` (@0x163C-0x1674), and it picks between two ends of a trailer:
//!
//! * CLEAR — state 3 arms one more dwell (@0x16BA), state 4 waits it out and
//!   drops to state 0 (@0x1846), which fires at once and starts the next
//!   trailer on the OLD stage point (@0x0F9E) with the mascot still up.
//! * SET — state 3 sends the mascot msg 6 (@0x16A8); the mascot finishes its
//!   run, plays runs 0x6B and 0x1A5, parks off-stage and answers msg 5
//!   (fn168 state 2 @0x29EC-0x2A2E, 0xC @0x2AC4, 0xD @0x2BD2 — the
//!   @0x2563/0x2619/0x2679 once cited here were not fn168's listing
//!   addresses); state 4 then arms ONE more dwell and
//!   drops to state 0 (@0x17F4), which waits it out with the old card up,
//!   re-rolls the point (@0x0E68) and calls the mascot back with msg 1
//!   (@0x1112-0x113A), which restarts it on run 6 — whose frame 17 is the
//!   "Coming Soon!" line.
//!
//! State 2 ends on the card's msg 4 (its bullet table exhausted, fn174
//! @0x3D08), not on a deadline, and its EXIT re-arms a full dwell (@0x153A)
//! over the +2000 of @0x149E, which nothing ever reads. Measured on
//! `emu/captures/qemu/coming-soon.mp4` (20 fps, Pace Mellow, 1 frame = 50 ms):
//!
//! | | golden | port before | port after |
//! |---|---|---|---|
//! | re-stage on a trailer the mascot stayed through | never (3 of 3 kept: cy 279; 251, 251) | always | never |
//! | mascot walk-offs per trailer end | 2 of 5 (t 7.65, 30.85) | 0 (never leaves) | 33 of 104 (8 seeds × 200 s; RandomBelow(4) = 1/4) |
//! | walk-off → old card clears | 4.95 s, 5.00 s | — | 5.05 s, 5.10 s |
//! | card up past its type-on (mascot stays) | 10.0-10.1 s (type-on ≤ 0.10 s) | 7.25 s, or 12.4 s with one extra 3↔4 loop | 10.0-10.1 s (test-pinned 9.90-10.25) |
//! | card clears → next card | 1.00-1.05 s | 1.15 s | 1.05-1.10 s |
//! | "Coming Soon!" (10006) after the mascot's return | 1.15 s, 1.14 s | at random run wraps | 0.91 s — 1.05-1.06 s since the GESTURE ERRATUM |
//!
//! Since the GESTURE ERRATUM dropped the 80 ms module gate (seed 0x5EED_CAFE,
//! 200 s): walk-off → old card clears **5.00 s** (twice), card clears → next
//! card **1.01 s** (11 of 11), the mascot back 16 ms after the card clears.
//!
//! The QEMU guest types a whole card in ≤ 2 frames; the Basilisk capture
//! types at ~46 glyphs/s. The port meters Basilisk's rate, so its cards stand
//! 1-5 s longer than QEMU's — the type-on is CPU-speed, not the port's.
//!
//! ## OPEN QUESTIONS (state of the ledger, 2026-09-29)
//!
//! CLOSED:
//! - *Per-trailer stage point.* Ported from @0x0E68-0x0F8C; `STAGE_CY = 197`
//!   and `STAGE_CX = SCREEN_W/2` are gone. See `roll_stage_point`.
//! - *`g03EC` gating of the re-place* and *trailer length* — the g03EC
//!   ERRATUM above.
//! - *Which rect `g03BA` vtbl+0x1C returns.* The vtable (`M129_g00AA`, shared
//!   by `g03BA` and `g03BE`) binds +0x1C to L135 `fn3A2E`, which copies the
//!   sequence's own bounds rect (`*(this+0x62)+4..+10`) — no frame argument,
//!   i.e. the bank-wide bound read here (638x389).
//! - *Mascot side/anchor rule* (@0x2412-0x2618) — `place_mascot`; golden
//!   placements (525,341) mirrored, (21,322), (21,294) reproduced exactly.
//!   Side FREQUENCY is untested against footage: the golden has 3 placements
//!   (R, L, L); a capture with ~20 walk-offs (~5 min at Pace Mellow) would
//!   measure it.
//!
//! - *Mascot frame delay* and *mascot state machine* (the old
//!   `GAP(mascot state machine)`) — the GESTURE ERRATUM below.
//!
//! STILL OPEN:
//! - *Idle-gesture rate and the unheard voice slots.* The C gives 1 in 32
//!   per idle run end and makes slots 0, 2, 4 and 5 reachable; the golden
//!   has 2 idle gestures in ~24 run ends and never plays 0/2/4/5. Both are
//!   one-sample counts the C does not contradict (see the erratum). A ~5 min
//!   Pace-Mellow QEMU capture WITH audio would measure the rate and hear the
//!   four slots.
//! - GAP(mail 2): fn168 states 2 and 0xE honour a mail 2 (walk straight
//!   off, @0x29A6 / @0x2B44), but nothing in fn149 posts 2 with `g03DC` bit
//!   2 — its only 2 goes to the card. Ported as written; never fires.
//! - *Object update order.* The mascot reads its mail at the top of its own
//!   handler (@0x2318); the port delivers it at the end of the posting tick.
//!
//! ## GESTURE ERRATUM — 2026-09-29, fn168 states 0-0xE against the QEMU golden
//!
//! fn168 (@0x22CA) is a 15-state machine on the L135 state object. Every
//! stepping state opens with its OWN frame gate (`this+0xE4 <= now` →
//! re-arm `now + this+0x134`); fn166 (@0x1F64) draws one frame, fires that
//! frame's voice event and reports the run's end (the next frameNum is no
//! record — fn441A through vtbl+0x8C). Transcribed as `mascot_tick`:
//!
//! | state | what it does | delay |
//! |---|---|---|
//! | 0 | off stage; msg 1 → side coin, hang, gate open, SetState(1) (@0x23DA-0x2688) | — |
//! | 1 | ENTER draws run 6 (@0x268C) and UPDATE overdraws 7 at once; at the run end SetState(fn167()) — straight into a gesture, NO run 0x6B | 100 ms |
//! | 2 | ENTER draws run 0x42 (@0x2968); at each run end: mail 6 → queue 0x6B, then 0xC (@0x29EC); else `RandomBelow(32) == 0` (or `g03EA`) → queue 0x6B, then fn167's gesture, delay 100 (@0x2A34-0x2A88); else `RandomBelow(2)` → 0x5D / 0x4E, first frame drawn in the same frame (@0x2A8E) | 75 ms |
//! | 3,4,5,7,8,9,10,0xB | queue `MASCOT_GESTURES`' runs, return to 2 via 0xE (@0x26FC-0x2964) | inherited |
//! | 0xC | queue 0x1A5, then 0xD (@0x2AC4) | — |
//! | 0xD | when `g03DC` is free: msg 5 to the controller, park at ±10000, SetState(0) (@0x2BD2) | — |
//! | 0xE | ENTER pops and draws (@0x2B00); at each run end the next queued run (drawn at once) or SetState(`this+0xCA`) (@0x2B24) | inherited |
//!
//! fn167 (@0x2154) — the spec's "eyeball/lip-sync picker" — picks the
//! GESTURE: `RandomBelow(100)`, `RandomBelow(30)`; the 1-in-30 or a pending
//! mail 6 answers 2; otherwise the ladder <10→3, <20→4, <30→5, <40→0xB,
//! <50→7, <60→8, <70→9, <80→10, <100→0xB, skipping the gesture it picked
//! last (`this+0x122`). The voice frames sit one per 3-run gesture (its
//! middle run: 126, 168, 237, 278, 318, 357, 390 → slot 1+RandomBelow(5)),
//! 200 in gesture 5's lone run (slot 0, "Wow!") and 17 in the entry run
//! (slot 6, "Coming Soon!") — so the salesman speaks once per gesture and
//! once per call-on, not once per lap of the bank.
//!
//! Golden: `qemu/coming-soon.mp4`, every 20 fps frame of the three mascot
//! placements matched against all 297 series-10000 records (best mean |Δ|;
//! the idle pair 0x5D/0x4E and a few neutral poses are near-ties, the run
//! sequence is not), audio from `-av.mp4` (audio-captures.md "Coming
//! Soon"). It reads exactly as the C:
//!
//! * t 0.00 entry run (frame 57 — it began before the capture) → gesture 7
//!   (216/236/254; 237 = 10003 at 4.84) → 0x42 → idle → 0x6B → 0x1A5 off.
//! * 12.65 back on FRAME 7 (6 never shows) → 10006 at 13.80 → gesture 0xB
//!   (380/389/409; 390 = 10001 at 19.01) with no 0x6B in front → 0x42 →
//!   0x5D/0x4E × ~10 → 0x6B → 0x1A5 off at 30.35.
//! * 35.85 back on frame 7 → 10006 at 36.99 → gesture 4 (155/167/185; 168
//!   = 10001 at 42.51) → idle × 5 → 0x6B → gesture 8 (267/277/296; 278 =
//!   10003 at 50.06) → 0x42 → idle × 6 → 0x6B → gesture 10 (347/356/373;
//!   357 = 10001 at 58.31).
//! * Rates: the entry run's 54 steps (+2 ticks into the gesture) take
//!   5.75 s / 5.70 s = **105-106 ms a step** (100 ms on the Mac grid); an
//!   idle run end to end is 11 × 83 ms (75 ms = 5 ticks, the Basilisk
//!   periodogram's 83.1). The audio trails the video ~0.1 s (390 / 278 show
//!   0.06-0.11 s before their cues), so 10006's 1.15 / 1.14 s after the
//!   mascot shows is ~1.05 s frame to frame = 10 × 105 ms.
//!
//! | | golden (60 s) | port before | port after |
//! |---|---|---|---|
//! | voice cues | 7 | 20 on seed 0x5EED_CAFE; 17.9/min over 16 seeds | 5 on 0x5EED_CAFE; 4.7/min over 16 seeds (pinned 3-11) |
//! | cue slots | {1, 3, 6} | {0,1,2,3,4,6} — every voice frame, every lap | {2, 4, 6} on 0x5EED_CAFE; 10006 only on the entry run, 1-5 only in a gesture's middle run |
//! | "Wow!" (slot 0) | 0 | 2.06/min | 0.44/min (only gesture 5; pinned < 1) |
//! | 10006 | once per call-on, 1.15 / 1.14 s after (audio clock) | every 24.6 s lap; 0.91 s after a call-on | once per call-on, 1.048 / 1.064 s after (pinned 1.00-1.20) |
//! | entry-run step | 105-106 ms | 83 ms | 6-7 ticks, 106.5 ms mean on 0x5EED_CAFE (pinned 103-108) |
//! | idle step | 83 ms | 83 ms | 83 ms |
//! | gesture sequence | entry → gesture; idle → 0x6B → gesture; idle → 0x6B → 0x1A5 | runs in pack order | same grammar as the golden |
//!
//! Golden 7 cues vs 4.7/min: the golden rolled 2 idle gestures in ~24 run
//! ends (1/32 each: P(≥ 2) ≈ 0.17) and walked off twice in 5 trailer ends
//! (expected 1.25), both one-sample luck the C does not rule out. The old
//! `MS_FRAME` module gate is gone with this: the controller runs every tick
//! and the mascot gates itself.

use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, TickClock, SpriteDraw, TextDraw, SCREEN_H, SCREEN_W,
};

// ---------------------------------------------------------------------------
// Constants (all disasm-cited in the spec)

/// Poster art series (§7): 19 movie compounds, frameNums below.
const BASE_POSTER: u32 = 20000;
/// Mascot art series (§8).
const BASE_MASCOT: u32 = 10000;

/// §7: per-movie poster compound frameNum (movie 0-based index → OFst frameNum).
const POSTER_FNO: [u32; 19] = [
    29, 35, 42, 49, 56, 63, 70, 76, 81, 86, 92, 97, 103, 109, 115, 121, 127, 133, 138,
];

/// §2.1: `g0052` = {20000,10000,5000,2500,0} ms, halved at every use site.
/// Indexed by pace bucket 0..4; bucket 5 (raw 100) reads the zero word past
/// the table — both collapse to 0 (ORIGINAL BUG kept, §13.2).
const PACE_DWELL_RAW: [u64; 5] = [20_000, 10_000, 5_000, 2_500, 0];

/// §11: trailer start → title card, +1000 ms (@0x110A).
const MS_TITLE: u64 = 1_000;
/// §11: poster phase hold (substate 0), +2000 ms (@0x149E).
const MS_POSTER_HOLD: u64 = 2_000;
/// §11: voice gap — new deadline = now + duration − 180 (@0x1C2A).
const MS_VOICE_GAP: u64 = 180;

// ~~`MS_FRAME` = 80 ms, one DoDrawFrame gate for the whole module~~
// SUPERSEDED 2026-09-29 (GESTURE ERRATUM): the 83.1 ms the Basilisk capture
// measured on the spotlight (periodogram 83.28 ms R 0.229 / 83.10 ms R 0.569,
// t 30-50 s) is the mascot's OWN idle delay — fn168 ENTER 0x8002 arms 75 ms,
// which is 5 Mac ticks — not a module frame gate. §11 is right that the
// module has no wait of its own: the controller runs every tick and each
// sprite gates itself (fn168 `this+0xE4`/`this+0x134`).
/// §10: 7 voice slots → snd 10000+i (Wow!, Mustbemad, Utiliterrific,
/// Edufantastic, Infomazing, Flavor, Coming Soon).
const VOICE_SND: [u32; 7] = [10_000, 10_001, 10_002, 10_003, 10_004, 10_005, 10_006];
/// Durations (ms) of snd 10000..10006, measured from the packed wavs
/// (the original reads them via Sound.f12DE @0x12DE).
const VOICE_MS: [u64; 7] = [323, 808, 886, 1_168, 936, 889, 1_496];
/// §8: mascot voice-event frame thresholds within the 10000-series runs.
const EV_SLOT0: u32 = 200; // → voice slot 0
const EV_SLOT6: u32 = 17; // → voice slot 6
const EV_RANDOM5: [u32; 7] = [126, 168, 237, 278, 318, 357, 390]; // → slot 1+RandomBelow(5)
/// fn168 ENTER 0x8001 (@0x268C): the run the mascot starts on every time it
/// is called on stage, at a 100 ms frame delay. Its frame 17 is `EV_SLOT6` —
/// "Coming Soon!" (snd 10006) — which is why the golden's two 10006 hits
/// (t 13.80, 36.99) each land ~1.1 s after a re-stage (mascot back at
/// t 12.65, 35.85). See the GESTURE ERRATUM.
const MASCOT_RUN_ENTER: u32 = 6;
const MS_MASCOT_ENTER: u64 = 100;
/// fn168 ENTER 0x8002 (@0x2968): the idle state's first run 0x42, at 75 ms.
const MASCOT_RUN_IDLE: u32 = 0x42;
const MS_MASCOT_IDLE: u64 = 75;
/// State 2's idle re-roll at each run end (@0x2A8E): `RandomBelow(2) == 0`
/// → 0x5D, else 0x4E.
const MASCOT_RUN_IDLE_A: u32 = 0x5D;
const MASCOT_RUN_IDLE_B: u32 = 0x4E;
/// State 2's gesture roll at each idle run end (@0x2A34): `RandomBelow(32)
/// == 0` (or `g03EA` set — never, see `dead2`) plays run 0x6B and then the
/// gesture fn167 picks, at a 100 ms frame delay (@0x2A64).
const MASCOT_GESTURE_ODDS: u32 = 32;
/// Run 0x6B fronts every idle gesture AND the
/// walk-off (state 2 @0x2A0A / @0x2A56); the walk-off's own run 0x1A5 is
/// queued by state 0xC (@0x2AC4).
const MASCOT_RUN_TURN: u32 = 0x6B;
const MASCOT_RUN_EXIT: u32 = 0x1A5;
/// fn168 gesture states 3..=0xB (@0x26FC-0x2964): each queues these runs
/// (fn133) for the queue player 0xE and returns to state 2 (`this+0xCA`).
/// The middle run of every 3-run gesture carries one `EV_RANDOM5` frame;
/// state 5's single run carries `EV_SLOT0` ("Wow!"). There is no state 6.
const MASCOT_GESTURES: [(u8, &[u32]); 8] = [
    (3, &[0x72, 0x7D, 0x8F]),
    (4, &[0x9B, 0xA7, 0xB9]),
    (5, &[0xC4]),
    (7, &[0xD8, 0xEC, 0xFE]),
    (8, &[0x10B, 0x115, 0x128]),
    (9, &[0x133, 0x13D, 0x150]),
    (10, &[0x15B, 0x164, 0x175]),
    (11, &[0x17C, 0x185, 0x199]),
];
/// fn167 @0x2154's ladder on `RandomBelow(100)`: the first rung whose bound
/// the roll is under AND whose gesture is not the last one played wins.
const MASCOT_GESTURE_LADDER: [(u32, u8); 9] = [
    (10, 3),
    (20, 4),
    (30, 5),
    (40, 11),
    (50, 7),
    (60, 8),
    (70, 9),
    (80, 10),
    (100, 11),
];
/// §10's background tune — **the shared bank's `cmid` 20 "Coming Soon.5"**,
/// not the module's own `cmid 10`/`410`. See MUSIC ERRATUM 1.
const SONG_COMING_SOON: u32 = 20;

/// Fallback tune length for a pre-v4 pack that carries no packed song. The
/// real one comes from the asset ([`Pack::song_length_ms`]): `cmid` 20's
/// end-of-track is **33.00 s**, not the 30 s this constant used to guess.
const TUNE_MS: u64 = 33_000;

/// STR# 500 "Item Texts" — 19 movies × 12 lines, read from the user's pack
/// (`Pack::strings(500)`, meta.json `strings.500`). Per movie: item (m)*12
/// is the title card (`#FR0`), items +1..+11 are bullets (`#FR1#Jl…`).
/// Rendered by the §9 markup parser in `texts()`.
const STR_CRAWL: u16 = 500;

/// Boot taglines, STR# 128..132 (§3) — one rolled each boot and pushed to
/// the AD message line (rendered in `texts()`). Each resource carries the
/// full message ("Coming Soon!: <tagline>"); the pick is `RandomBelow(5)`.
const STR_TAGLINE_FIRST: u16 = 128;
const TAGLINE_COUNT: u32 = 5;

// --- stage geometry (see the LAYOUT note in the module header) -------------

/// Poster compound bounds size `(w, h)` per movie, from the `OFtb 20000`
/// part tables. The compound is placed by CENTRING these bounds, so the
/// module needs the size without decoding the png every frame.
const POSTER_WH: [(i32, i32); 19] = [
    (388, 369), (358, 300), (358, 300), (393, 302), (360, 300), (358, 300),
    (358, 300), (358, 300), (388, 300), (382, 300), (358, 309), (380, 300),
    (358, 300), (358, 300), (358, 306), (358, 306), (382, 304), (358, 324),
    (358, 300),
];

/// `ch1` — the card panel rect `(x, y, w, h)` relative to the compound
/// bounds origin. Always 358×300; art 87, MISSING from the rip
/// (APPROXIMATIONS #5), so nothing draws it. Kept because it is the frame
/// the whole layout hangs off, and because it is what a future art-87
/// recovery (or a fill primitive) would target.
#[allow(dead_code)]
const POSTER_CARD: [(i32, i32, i32, i32); 19] = [
    (30, 0, 358, 300), (0, 0, 358, 300), (0, 0, 358, 300), (4, 1, 358, 300),
    (2, 0, 358, 300), (0, 0, 358, 300), (0, 0, 358, 300), (0, 0, 358, 300),
    (30, 0, 358, 300), (0, 0, 358, 300), (0, 5, 358, 300), (22, 0, 358, 300),
    (0, 0, 358, 300), (0, 0, 358, 300), (0, 2, 358, 300), (0, 2, 358, 300),
    (7, 0, 358, 300), (0, 20, 358, 300), (0, 0, 358, 300),
];

/// `ch8` — the TITLE text block `(x, y, w, h)` relative to the compound
/// bounds origin. Note how the heights track the title's line count (71/78
/// for the two-line cards, 101 for the three-line Religious Right card,
/// 31–44 for the one-liners): these really are the text rects.
const POSTER_TITLE_BOX: [(i32, i32, i32, i32); 19] = [
    (30, 8, 358, 71), (0, 8, 358, 71), (0, 12, 358, 37), (162, 23, 182, 37),
    (10, 7, 345, 71), (18, 16, 282, 41), (11, 17, 345, 71), (4, 14, 350, 39),
    (34, 10, 350, 78), (4, 12, 350, 41), (52, 24, 182, 37), (22, 14, 358, 35),
    (16, 18, 342, 31), (16, 12, 336, 69), (1, 15, 356, 34), (8, 11, 314, 101),
    (7, 19, 358, 44), (0, 31, 358, 36), (0, 11, 358, 36),
];

/// `ch9` — the BULLET text block `(x, y, w, h)` relative to the compound
/// bounds origin. Its side varies per movie (left of the poster art on the
/// Aliens card, right of the worm on Tippy's) — which is why the old
/// screen-wide floating crawl could never line up.
const POSTER_BULLET_BOX: [(i32, i32, i32, i32); 19] = [
    (173, 95, 203, 186), (13, 95, 177, 189), (172, 66, 169, 215),
    (174, 65, 177, 220), (178, 94, 171, 190), (167, 71, 175, 213),
    (162, 80, 186, 198), (167, 80, 177, 185), (208, 100, 173, 184),
    (14, 66, 176, 212), (153, 86, 191, 199), (200, 78, 171, 206),
    (12, 69, 184, 209), (9, 89, 184, 154), (172, 67, 181, 174),
    (152, 86, 196, 159), (178, 73, 181, 169), (181, 86, 167, 213),
    (34, 81, 190, 174),
];

/// The stage the placement rule @0x0E68-0x0F8C confines the page to: the
/// draw canvas bounds `[A5]..[A5+6]` (`g0512..g0518`), filled once in the
/// ctor by `SetDrawOrigin()->GetBounds(&g0512)` (@0x0510, again @0x070E).
/// On the 640x480 stage this is the whole screen.
const STAGE: (i32, i32, i32, i32) = (0, 0, SCREEN_W, SCREEN_H); // l, t, r, b

/// The poster-bank frameNum whose two channels are the mascot's anchor
/// zones: `f3EBE` fetches ch1 into `this+0x9C` and ch2 into `this+0xA4`
/// (@0x3F14-0x3F62, both `vtbl+0x94` with frame 5). Frame 5 is 638x154 at
/// design (2,238); ch1 = (2,238)-(136,392) is the left spotlight zone and
/// ch2 = (506,238)-(640,390) the right one.
const MASCOT_ANCHOR_FNO: u32 = 5;

/// §9 frame styles: `#FR0` title cards at double scale, `#FR1` bullets at
/// single scale (the original's font sizes were not recovered — see
/// APPROXIMATIONS #1; these are the two scales that fit the ch8/ch9 rects,
/// the widest title being "The Tao of Tax Evasion" at 22 chars = 352 px in
/// a 358-px block).
const CRAWL_TITLE_SCALE: u32 = 2;
const CRAWL_BULLET_SCALE: u32 = 1;
/// Bullet block metrics, scaled down from the golden's 15 px pitch / +7 px
/// item gap in proportion to the smaller engine glyph.
const BULLET_LINE_H: i32 = 10;
const BULLET_ITEM_GAP: i32 = 4;
/// Bullet text indent inside the ch9 block, leaving room for the accent
/// tick. The original overlaps tick and text (its tick is a thin slash);
/// ours is a glyph, so it gets its own gutter.
const BULLET_INDENT: i32 = 12;
/// The accent tick, golden-measured on Tippy's card (`coming-soon.mp4` f614):
/// a 27×25 slash whose box is x 293-319 against a ch9 block edge at x 299 and
/// whose top sits 14 px above the item's first text line. So it hangs 6 px
/// left of the block and starts a line and a half high — the original really
/// does let the slash cut across the first glyphs. `/` at scale 3 is 24×24,
/// the closest the engine font gets to the brush stroke (see the errata:
/// the stroke is `OFtb 20000` frameNum 1 / series 13000, black-baked, so the
/// real art cannot be recoloured to `clut[7]` here).
const TICK_DX: i32 = -6;
const TICK_DY: i32 = -14;
const TICK_SCALE: u32 = 3;

/// §9 type-on rate for `#FR0` title text, source glyphs per second.
/// Golden: 36 glyphs in 400 ms (`coming-soon.mp4` f205-f217).
const CRAWL_TITLE_CPS: u64 = 90;
/// §9 type-on rate for `#FR1` bullet text, source glyphs per second.
/// Golden: 146 glyphs in 3.200 s (Tippy f217-f313) and 226 in 4.867 s
/// (the Aliens, t 21.933-26.800) — 45.6 and 46.4.
const CRAWL_BULLET_CPS: u64 = 46;
/// §6 bullet slots per movie: `STR# 500` items 2..12 = line numbers 1..=11.
const BULLET_SLOTS: usize = 11;

// ---------------------------------------------------------------------------

/// §9 markup parser (@0x683C scans for '#'; command dispatch
/// @0x319A–0x3210): returns (is title card, [(justify, line text)]) with
/// `#FR0/#FR1` selecting the frame style, `#Jc/#Jl/#Jr` the justification
/// (sticky across `#R` breaks — the §13.4 typos), `#R` the line break.
/// Justify codes: 0 left / 1 center / 2 right.
fn parse_crawl(s: &str) -> (bool, Vec<(u8, String)>) {
    let chars: Vec<char> = s.chars().collect();
    let mut title = false;
    let mut just = 0u8;
    let mut lines = vec![(just, String::new())];
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && i + 1 < chars.len() {
            match chars[i + 1] {
                'F' if i + 3 < chars.len() && chars[i + 2] == 'R' => {
                    let f = chars[i + 3];
                    if f == '0' || f == '1' {
                        title = f == '0';
                        i += 4;
                        continue;
                    }
                }
                'J' if i + 2 < chars.len() => {
                    just = match chars[i + 2] {
                        'c' => 1,
                        'r' => 2,
                        _ => 0,
                    };
                    // The current line's stored justification was set (to 0)
                    // when the line was pushed, before this token was seen.
                    // If no text has landed on it yet, retarget it now —
                    // otherwise every card's first line (and any line whose
                    // #J token precedes its text) renders flush-left
                    // regardless of the code, e.g. the #Jc title cards.
                    if lines.last().unwrap().1.is_empty() {
                        lines.last_mut().unwrap().0 = just;
                    }
                    i += 3;
                    continue;
                }
                'R' => {
                    lines.push((just, String::new()));
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        lines.last_mut().unwrap().1.push(chars[i]);
        i += 1;
    }
    (title, lines)
}

// ---------------------------------------------------------------------------

/// Master state machine states (§5, `this+0x98`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)] // 0x8000/0x4000 wipe variants not modeled (§14.1)
enum State {
    /// 0 @0x0DAC — wait dwell; pick + start next movie.
    WaitDwell = 0,
    /// 1 @0x114E — title-card phase.
    Title = 1,
    /// 2 @0x125E — poster + music replay management.
    Poster = 2,
    /// 3 @0x1556 — hold + pace probability roll.
    Hold = 3,
    /// 4 @0x16EA — next-line / exit phase.
    NextLine = 4,
    /// 5 @0x1858 — final/exit phase.
    Exit = 5,
}

/// Mascot sequence run (a maximal run of consecutive 10000-series frameNums).
struct Run {
    first: u32,
    last: u32,
}

pub struct ComingSoon {
    pack: Pack,
    mascot_runs: Vec<Run>,

    // --- controls, raw values exactly as GetControlValue returned them (§2)
    pace_raw: i32,  // control 0, slider 0..100, default 0x16 (22 "Mellow")
    show_raw: i32,  // control 1, popup 1-based (1 Random / 2 In Order); shipped default 0
    dead2: i32,     // control 2 → g03EA: always 0, so fn168's forced gesture (@0x2A40) never fires
    music_raw: i32, // control 3, slider 0..100, default 0x16 (22 "Once")

    // --- control-derived (§2, @0x0AB4–0x0B06)
    pace_bucket: i32, // g03F0 = raw/20
    show_order: i32,  // g03EE = raw-1; 0 → random, nonzero → in order
    music_left: i32,  // g03F2 = raw/20; "≥99 → -1" branch dead (§13.1)

    // --- master state machine (controller object, §5)
    state: State,
    substate: u16, // this+0x9A
    deadline: u64, // this+0x8C, ms

    // --- trailer player (§6)
    played: [bool; 19], // this+0x9C[19] once-per-cycle flags
    cur_movie: usize,   // 0-based; original this+0xD0 is 1-based
    /// `g03EC` — "RE-STAGE on the next state 0". Set to 1 by the controller
    /// ctor (@0x0C26), cleared and re-rolled at every state-3 exit
    /// (`RandomBelow(5 - bucket) == 0`, @0x163C-0x1674), forced to 1 by
    /// state 5 (@0x1978). While it is SET: state 3 sends the mascot msg 6
    /// (walk off), state 4 waits for its msg 5 and arms one more dwell, and
    /// state 0 re-rolls the stage point and calls the mascot back (msg 1).
    /// While it is CLEAR the next trailer starts on the OLD point with the
    /// mascot still up. The port used to call this `crawl_flag` and read it
    /// as "type one more bullet line" — the RE spec's reading, falsified by
    /// the C and by the golden (see the g03EC erratum).
    restage: bool,
    /// Controller mailbox `this+0x98`: what the card (3/4) and the mascot (5)
    /// post through `g03DA`/`g03DC` bit 4 (@0x1006). Only msg 5 is modelled.
    mailbox: u16,
    /// Is a trailer card still standing? The card is erased only when state
    /// 0 fires (@0x0DBE-0x0E1C), so it stays up through state 4 AND through
    /// the extra dwell state 0 waits after the mascot has walked off.
    card_live: bool,
    /// §5's `RandomBelow(5 - paceBucket) == 0` resolved once per line slot at
    /// movie start: the bullet items (1..=11) this trailer will show, in
    /// order. Golden shows all of them typed in one unbroken run, so the roll
    /// cannot be the per-dwell gate the port used to make it (see the errata).
    shown: Vec<usize>,
    /// `Ctx::now_ms` when the card went up (state 2 entry) — the type-on
    /// clock (§9).
    card_at: u64,
    /// The stage point (controller `this+0xC4` / `this+0xC6`), rolled by the
    /// placement block @0x0E68-0x0F8C. See [`ComingSoon::roll_stage_point`].
    stage_cx: i32,
    stage_cy: i32,
    /// Last `Ctx::now_ms` seen by `tick`. `texts()`/`sprites()` get no `Ctx`,
    /// and the type-on needs the clock.
    now: u64,

    // --- sound manager (§10)
    voice_busy_until: u64, // this+0x1E deadline
    next_slot: usize,      // this+0x22

    // --- music (§10; see MUSIC ERRATUM 1)
    music_ends_at: u64,
    music_started: bool,
    /// Plays STARTED. The shell restarts the tune whenever this moves —
    /// see `Module::music`. `music_left` (g03F2) is the original's
    /// remaining-replays counter and stays exactly as the disasm has it.
    music_plays: u32,

    // --- mascot (§8): the fn168 sprite, states 0..=0xE (GESTURE ERRATUM)
    /// fn168 state (`this+0x40` state object). 0 off stage, 1 entry run,
    /// 2 idle, 3..=0xB gestures (queue their runs), 0xC walk-off, 0xD park,
    /// 0xE queue player.
    m_state: u8,
    /// SetState pending: the next Run fires ENTER `0x8000|s` before UPDATE.
    m_enter: bool,
    /// `this+0x11E`: the current run's first frameNum (the wrap target).
    m_first: u32,
    /// `this+0x120`: the next frameNum fn166 draws (0 = none).
    m_next: u32,
    /// `this+0xD4`: the run fn166 last latched; a new `m_first` restarts
    /// `m_next` there (@0x1F8C).
    m_latch: u32,
    /// The frameNum fn166 last DREW — what is on screen. 0 = nothing drawn
    /// since the last call-on.
    m_frame: u32,
    /// `this+0xE4`: when the next frame is due; `this+0x134`: the delay.
    m_due: u64,
    m_delay: u64,
    /// `this+0xEA`: the fn132/133/134 run queue the gesture states fill.
    m_queue: std::collections::VecDeque<u32>,
    /// `this+0xCA`: the state 0xE hands back to when the queue runs dry.
    m_ret: u8,
    /// `this+0x122`: the last gesture fn167 picked (never picked twice running).
    m_gesture: u8,
    /// `this+0x11A`: the mascot's mail word, copied from `g03DA` whenever
    /// `g03DC` bit 2 is set (@0x2318) and kept until something clears it —
    /// 1 = come on stage (state 0), 6 = walk off at the next idle run end
    /// (state 2 @0x29EC), which also makes fn167 answer "no gesture".
    m_mail: u16,
    /// A post to the mascot not yet read (`g03DA` with `g03DC` bit 2) —
    /// delivered at the end of the posting tick (`deliver_mascot_mail`).
    mascot_mail: u16,
    /// fn168 state 0: off stage (pos parked at ±10000 by state 0xD @0x2BD2,
    /// or never called on yet), waiting for msg 1.
    mascot_hidden: bool,
    /// `this+0x11C`: the side coin, 0 = left zone, 1 = right zone (drawn
    /// mirrored — the sequence's mirror toggle vtbl+0x0C is applied for side 1
    /// @0x2670 and undone @0x240E before the next coin).
    mascot_side: u16,
    /// Screen offset of the mascot's design space: frame `f` draws at
    /// `(f.bx, f.by) + mascot_off`. Fixed at each placement (L135 links keep
    /// the frames design-aligned in between).
    mascot_off: (i32, i32),

    /// AD message line (§3): the boot tagline string (STR# 128..132),
    /// rendered along the bottom edge every frame.
    msg_line: String,

    started: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

/// Concrete constructor — `make` boxes it; the layout tests drive it
/// directly so they can pose the state machine (`cur_movie` / `line`)
/// instead of waiting on the §5 deadlines.
fn build(pack: Pack) -> Option<ComingSoon> {
    // The module needs both its poster stage and its mascot art.
    if !pack.meta.series.contains_key(&BASE_POSTER.to_string())
        || !pack.meta.series.contains_key(&BASE_MASCOT.to_string())
    {
        return None;
    }
    // Mascot sequence runs = maximal consecutive frameNum spans (§12).
    let mascot_runs = pack
        .series(BASE_MASCOT)
        .iter()
        .map(|s| Run {
            first: s.first,
            last: s.first + s.frames.len() as u32 - 1,
        })
        .collect();
    Some(ComingSoon {
        pack,
        mascot_runs,
        // Defaults per §2 (sVal 1000 = 0x16, mVal 1001 = 0x0000, sVal 1003 = 0x16).
        pace_raw: 22,
        show_raw: 0,
        dead2: 0,
        music_raw: 22,
        pace_bucket: 1,
        show_order: -1,
        music_left: 1,
        state: State::WaitDwell,
        substate: 0,
        deadline: 0,
        played: [false; 19],
        cur_movie: 0,
        restage: true, // g03EC = 1 in the controller ctor @0x0C26
        mailbox: 0,
        card_live: false,
        shown: Vec::new(),
        card_at: 0,
        // The ctor (@0x0C2A) leaves the point at zero; the first state-0
        // update rolls it before anything draws. Seeding it with the
        // rule's own degenerate-stage fallback (@0x0ED4) keeps the field
        // meaningful if it is ever read first.
        stage_cx: SCREEN_W / 2,
        stage_cy: SCREEN_H / 2,
        now: 0,
        voice_busy_until: 0,
        next_slot: 0,
        music_ends_at: 0,
        music_started: false,
        music_plays: 0,
        // fn165 @0x1E4E zeroes 0x11C, 0x122, 0xD0 and 0x120; the state
        // object starts in state 0.
        m_state: 0,
        m_enter: false,
        m_first: 0,
        m_next: 0,
        m_latch: 0,
        m_frame: 0,
        m_due: 0,
        m_delay: 0,
        m_queue: std::collections::VecDeque::new(),
        m_ret: 0,
        m_gesture: 0,
        m_mail: 0,
        mascot_mail: 0,
        // The mascot sprite starts in fn168 state 0 and only comes on stage
        // when the first (ctor-forced, g03EC = 1) re-stage calls it.
        mascot_hidden: true,
        mascot_side: 0,
        mascot_off: (0, 0),
        msg_line: String::new(),
        started: false,
    })
}

impl ComingSoon {
    /// Per-trailer dwell = `g0052[bucket]/2` (§2.1). Bucket 4 reads the last
    /// entry (0); bucket 5 reads the zero word past the table — the get-or-0
    /// below reproduces both (ORIGINAL BUG kept, §13.2).
    fn dwell(&self) -> u64 {
        PACE_DWELL_RAW
            .get(self.pace_bucket as usize)
            .copied()
            .unwrap_or(0)
            / 2
    }

    /// End-of-track of the packed tune, milliseconds. The asset is the
    /// authority — `cmid` 20 runs 33.00 s — and [`TUNE_MS`] only covers a
    /// pre-v4 pack that has no music section at all.
    fn tune_ms(&self) -> u64 {
        self.pack.song_length_ms(SONG_COMING_SOON).unwrap_or(TUNE_MS)
    }

    /// Control interpreter MOD.f0A90 (§2, @0x0A90–0x0B06): recompute the
    /// derived globals from the raw control values.
    fn read_controls(&mut self) {
        self.pace_bucket = self.pace_raw / 20; // g03F0 @0x0AB4
        self.show_order = self.show_raw - 1; // g03EE @0x0AC6
        self.dead2 = 0; // case 2 stores literal 0 into g03EA (@0x0AD8; read only by fn168 @0x2A40)
        self.music_left = self.music_raw / 20; // g03F2 @0x0AEA
        if self.music_left >= 99 {
            self.music_left = -1; // ORIGINAL BUG kept: tests the divided
                                  // value, can never fire (§13.1, @0x0AF6)
        }
    }

    /// §2.2/§6 pick next movie with the once-per-cycle flag array.
    fn pick_movie(&mut self, ctx: &mut Ctx) -> usize {
        // all 19 seen → clear flags, new cycle (@0x104C–0x1072)
        if self.played.iter().all(|&p| p) {
            self.played = [false; 19];
        }
        let idx = if self.show_order == 0 {
            // random path: RandomBelow(19) @0x107A + skip-loop @0x108E–0x10C4
            let mut i = ctx.rng.pct(19) as usize;
            while self.played[i] {
                i = (i + 1) % 19;
            }
            i
        } else {
            // in-order: scan starts at index 0 @0x1088, first unplayed.
            // Shipped default g03EE = -1 lands here too (§13.3).
            (0..19).find(|&i| !self.played[i]).unwrap_or(0)
        };
        self.played[idx] = true;
        idx
    }

    /// MOD.f3EBE start movie (§6): reset the line table; the original also
    /// registers the poster RLEP chain 20000+index and the per-movie clut —
    /// the clut is applied at draw time via SpriteDraw.pal (§6/Appendix B);
    /// our compounds are pre-composed, so only the recolour remains.
    ///
    /// This is also where the 12-line table (`this+0xFA`, 0x0C stride @0x38DC)
    /// gets filled: §5's pace roll `RandomBelow(5 - paceBucket) == 0` is run
    /// once per bullet slot 1..=11 and the winners are what the crawl types.
    /// The capture forces that reading — four cards show {1,2,6}, {1,3,8,11},
    /// {1,6,11} and {2,4,6}, never a run of consecutive items, and every
    /// winner is typed in one unbroken burst at the head of the trailer.
    fn start_movie(&mut self, ctx: &mut Ctx, movie: usize) {
        self.cur_movie = movie;
        // @0x165E: RandomBelow(5 - bucket) == 0. Bucket 4 ("Fast") gives
        // RandomBelow(1) == 0, i.e. every line — the §13.2 table-overrun bug
        // lands on the same denominator, so both collapse to "show all".
        let denom = (5 - self.pace_bucket).max(1) as u32;
        self.shown = (1..=BULLET_SLOTS)
            .filter(|_| ctx.rng.pct(denom) == 0)
            .collect();
    }

    /// §10 MOD.f1BAE play voice slot with the 180 ms gap rule: if the
    /// previous voice is still further than 180 ms from its end, drop;
    /// otherwise start and set deadline = now + duration − 180.
    fn queue_voice(&mut self, ctx: &mut Ctx, slot: usize) {
        if self.voice_busy_until > ctx.now_ms {
            return; // request dropped (@0x1C2A addi.l #0xFFFFFF4C)
        }
        ctx.sounds.push(VOICE_SND[slot]);
        self.voice_busy_until = ctx.now_ms + VOICE_MS[slot] - MS_VOICE_GAP;
        self.next_slot = slot; // this+0x22
    }

    /// fn167 @0x2154 — the GESTURE chooser (the RE spec read it as an
    /// "eyeball/lip-sync picker"; it is not: its answer is the fn168 state
    /// the caller switches to). `RandomBelow(100)`, then `RandomBelow(30)`;
    /// the 1-in-30 OR a pending walk-off (mail 6) answers 2 (idle, no
    /// gesture); otherwise the first ladder rung the roll is under whose
    /// gesture is not `this+0x122` (the last one) wins and is remembered;
    /// a roll that falls off the ladder answers 2.
    fn choose_gesture(&mut self, ctx: &mut Ctx) -> u8 {
        let n = ctx.rng.pct(100);
        let neutral = ctx.rng.pct(30) == 0;
        if neutral || self.m_mail == 6 {
            return 2;
        }
        for (bound, g) in MASCOT_GESTURE_LADDER {
            if n < bound && self.m_gesture != g {
                self.m_gesture = g;
                return g;
            }
        }
        2
    }

    /// L135 SetState on the mascot's state object: the new state's ENTER
    /// runs at the top of the NEXT Run, then its UPDATE (port-plan §2).
    fn mascot_set_state(&mut self, s: u8) {
        self.m_state = s;
        self.m_enter = true;
    }

    /// Start run `fno` (`this+0x11E` and `this+0x120` both set, as every
    /// caller in fn168 does).
    fn mascot_run(&mut self, fno: u32) {
        self.m_first = fno;
        self.m_next = fno;
    }

    /// fn441A via the sequence's vtbl+0x8C: is `fno` a record of the bank?
    /// A run ends at the first frameNum that is not.
    fn mascot_valid(&self, fno: u32) -> bool {
        self.mascot_runs.iter().any(|r| (r.first..=r.last).contains(&fno))
    }

    /// fn166 @0x1F64 — draw one frame and advance. Latches a new run start
    /// (@0x1F8C), draws `this+0x120` (the L135 link keeps every frame on the
    /// placement's design offset, see `place_mascot`), dispatches the voice
    /// events on the frame just drawn (@0x2034-0x2100), then advances;
    /// returns true when the frame drawn was the run's last (the next
    /// frameNum is no record — the counter is rewound to the run start).
    fn mascot_step(&mut self, ctx: &mut Ctx) -> bool {
        if self.m_next == 0 {
            return false;
        }
        if self.m_first != self.m_latch {
            self.m_latch = self.m_first;
            self.m_next = self.m_first;
        }
        let f = self.m_next;
        self.m_frame = f;
        if f == EV_SLOT6 {
            self.queue_voice(ctx, 6);
        } else if f == EV_SLOT0 {
            self.queue_voice(ctx, 0);
        } else if EV_RANDOM5.contains(&f) {
            let slot = 1 + ctx.rng.pct(5) as usize; // @0x20A4
            self.queue_voice(ctx, slot);
        }
        self.m_next = f + 1;
        if !self.mascot_valid(self.m_next) {
            self.m_next = self.m_first;
            return true;
        }
        false
    }

    /// The frame gate every stepping state opens with: `this+0xE4 <= now`
    /// re-arms `now + this+0x134`.
    fn mascot_due(&mut self, now: u64) -> bool {
        if self.m_due <= now {
            self.m_due = now + self.m_delay;
            true
        } else {
            false
        }
    }

    /// The mascot's Run (L135 fn5340), once per module frame: a pending
    /// SetState fires its ENTER first, then the state's UPDATE — fn168
    /// @0x22CA, states 1..=0xE (state 0's msg 1 is `deliver_mascot_mail`).
    fn mascot_tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        if std::mem::take(&mut self.m_enter) {
            match self.m_state {
                // ENTER 0x8001 @0x268C: the entry run at 100 ms, first frame
                // drawn at once (and at once overdrawn by UPDATE 1, whose
                // gate state 0 left open — so frame 6 never shows).
                1 => {
                    self.m_delay = MS_MASCOT_ENTER;
                    self.mascot_run(MASCOT_RUN_ENTER);
                    self.mascot_step(ctx);
                }
                // ENTER 0x8002 @0x2968: idle run 0x42 at 75 ms.
                2 => {
                    self.m_delay = MS_MASCOT_IDLE;
                    self.mascot_run(MASCOT_RUN_IDLE);
                    self.mascot_step(ctx);
                }
                // ENTER 0x800E @0x2B00: pop the next queued run and draw it.
                0xE => {
                    let r = self.m_queue.pop_front().unwrap_or(0);
                    self.mascot_run(r);
                    self.mascot_step(ctx);
                }
                _ => {}
            }
        }
        match self.m_state {
            // state 1 @0x26AA: step the entry run; at its end fn167 picks
            // what follows — straight into a gesture, no run 0x6B.
            1 => {
                if self.mascot_due(now) && self.mascot_step(ctx) {
                    let g = self.choose_gesture(ctx);
                    self.mascot_set_state(g);
                }
            }
            // state 2 @0x2986: idle.
            2 => {
                if !self.mascot_due(now) {
                    return;
                }
                if self.m_mail == 2 {
                    // @0x29A6. GAP(mail 2): nothing in fn149 ever posts 2
                    // with g03DC bit 2 (its 2 goes to the card), so this arm
                    // is kept only because the C has it.
                    self.m_mail = 0;
                    self.mascot_set_state(0xC);
                } else if self.mascot_step(ctx) {
                    if self.m_mail == 6 {
                        // @0x29EC-0x2A2E: walk off — run 0x6B, then 0xC.
                        self.m_ret = 0xC;
                        self.m_queue.clear(); // fn132
                        self.m_queue.push_back(MASCOT_RUN_TURN);
                        self.mascot_set_state(0xE);
                    } else if ctx.rng.pct(MASCOT_GESTURE_ODDS) == 0 || self.dead2 != 0 {
                        // @0x2A46-0x2A88: run 0x6B, then fn167's gesture, at
                        // 100 ms.
                        self.m_ret = self.choose_gesture(ctx);
                        self.m_queue.push_back(MASCOT_RUN_TURN);
                        self.m_delay = MS_MASCOT_ENTER;
                        self.mascot_set_state(0xE);
                    } else {
                        // @0x2A8E: another idle run, drawn in the same frame.
                        let r = if ctx.rng.pct(2) == 0 {
                            MASCOT_RUN_IDLE_A
                        } else {
                            MASCOT_RUN_IDLE_B
                        };
                        self.mascot_run(r);
                        self.mascot_step(ctx);
                    }
                }
            }
            // gesture states @0x26FC-0x2964: queue the runs, come back to 2.
            3..=0xB => {
                if let Some((_, runs)) = MASCOT_GESTURES.iter().find(|(s, _)| *s == self.m_state) {
                    self.m_queue.extend(runs.iter().copied());
                    self.m_ret = 2;
                    self.mascot_set_state(0xE);
                }
            }
            // state 0xC @0x2AC4: the walk-off run, then park.
            0xC => {
                self.m_queue.push_back(MASCOT_RUN_EXIT);
                self.m_ret = 0xD;
                self.mascot_set_state(0xE);
            }
            // state 0xE @0x2B24: play the queue; empty → `this+0xCA`.
            0xE => {
                if !self.mascot_due(now) {
                    return;
                }
                if self.m_mail == 2 {
                    self.m_mail = 0; // GAP(mail 2), as in state 2
                    self.mascot_set_state(0xC);
                } else if self.mascot_step(ctx) {
                    match self.m_queue.pop_front() {
                        None => self.mascot_set_state(self.m_ret),
                        Some(r) => {
                            self.mascot_run(r);
                            self.mascot_step(ctx);
                        }
                    }
                }
            }
            // state 0xD @0x2BD2: once the g03DC mailbox is free, post msg 5
            // to the controller, park the sprite at ±10000 (drawing the
            // rewound `this+0x120` there) and drop to state 0.
            0xD => {
                if self.mailbox == 0 {
                    self.mailbox = 5;
                    self.m_frame = self.m_next;
                    self.mascot_hidden = true;
                    self.mascot_set_state(0);
                }
            }
            _ => {}
        }
    }

    /// Deliver the mascot's mail (`g03DA` → `this+0x11A`, @0x2318). The C
    /// reads it at the top of the mascot's own handler; the object update
    /// order is not decoded, so the port delivers it at the end of the frame
    /// that posted it. Golden: the mascot is back on the frame the old card
    /// clears (f717) or one 50 ms capture frame later (f252 → f253).
    fn deliver_mascot_mail(&mut self, ctx: &mut Ctx) {
        let mail = std::mem::take(&mut self.mascot_mail);
        if mail == 0 {
            return;
        }
        self.m_mail = mail;
        // fn168 state 0, msg 1 (@0x23DA-0x2688): clear the mail, toss the
        // side coin, hang the sprite in its zone, open the frame gate and
        // SetState(1). Nothing is drawn until ENTER 0x8001 next frame.
        if self.m_state == 0 && self.m_mail == 1 {
            self.m_mail = 0;
            let coin = ctx.rng.pct(2) as u16; // RandomBelow(2) @0x2412
            self.place_mascot(coin);
            self.m_frame = 0;
            self.m_due = 0;
            self.mascot_hidden = false;
            self.mascot_set_state(1);
        }
    }

    /// State 2's exit condition: the card sprite (fn174) posts msg 4 once
    /// its bullet table runs out (@0x3D08) — i.e. when the type-on is done.
    /// Metered in source characters like [`ComingSoon::typed`] (the engine
    /// font clips some bullets at the block bottom; the budget does not
    /// care, so the trailer keeps the original's length).
    fn type_on_done(&self, now: u64) -> bool {
        let chars = |idx: usize| -> usize {
            parse_crawl(&self.crawl_item(idx)).1.iter().map(|(_, t)| t.chars().count()).sum()
        };
        let title = chars(self.cur_movie * 12);
        let bullets: usize = self.shown.iter().map(|&n| chars(self.cur_movie * 12 + n)).sum();
        let (t, b) = self.typed(now, title);
        t >= title && b >= bullets
    }
}

// ---------------------------------------------------------------------------
// Stage geometry + the per-movie card palette

impl ComingSoon {
    /// The page rect the placement rule moves around the stage: the poster
    /// sequence's own bounds, `g03BA->GetBounds(&r)` (vtbl+0x1C @0x0E76),
    /// immediately normalised to the origin by `Resource.f4038` (@0x0E80 —
    /// `r.right -= r.left; r.left = 0; r.bottom -= r.top; r.top = 0`), so
    /// only its SIZE is used.
    ///
    /// `g03BA` is the single 20000-series sequence built at @0x2C4E over the
    /// whole poster bank, and the bounds call takes no frame argument (the
    /// card sprite's ctor @0x2D42 makes the same call before any frameNum is
    /// set), so this is the bank-wide bound, not the current trailer's
    /// compound: the union of every 20000 frame's `bx,by,w,h`, which the pack
    /// puts at (2,18)-(640,407) = **638 x 389**.
    ///
    /// That size is what makes the rule check out against the capture — see
    /// [`ComingSoon::roll_stage_point`].
    fn page_wh(&self) -> (i32, i32) {
        self.series_wh(BASE_POSTER)
    }

    /// `seq->GetBounds()` then `ToOrigin` (vtbl+0x1C = L135 `fn3A2E`, the
    /// sequence's own bank-wide bounds rect; `Resource.f4038`): the SIZE of
    /// the union of every frame of `base`. Poster bank: 638x389. Mascot
    /// bank: 92x144.
    fn series_wh(&self, base: u32) -> (i32, i32) {
        let (mut l, mut t, mut r, mut b) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for seq in self.pack.series(base) {
            for f in &seq.frames {
                l = l.min(f.bx);
                t = t.min(f.by);
                r = r.max(f.bx + f.w);
                b = b.max(f.by + f.h);
            }
        }
        if r <= l || b <= t {
            return (0, 0);
        }
        (r - l, b - t)
    }

    /// vtbl+0x94 = L135 `fn4488`: lay frame `fno` out with its bounds
    /// centred on `pt` (`pt - (w>>1, h>>1)`) and return the screen rect
    /// `(l, t, r, b)` of the first part on channel `chan`.
    fn channel_rect(&self, base: u32, fno: u32, chan: i32, pt: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
        let f = self.pack.frame(base, fno)?;
        let (ox, oy) = (pt.0 - (f.w >> 1), pt.1 - (f.h >> 1));
        let p = f.parts.iter().find(|p| p[1] == chan)?;
        Some((p[3] - f.bx + ox, p[4] - f.by + oy, p[5] - f.bx + ox, p[6] - f.by + oy))
    }

    /// fn168 state 0 on msg 1 (@0x23DA-0x2688): put the mascot on the side
    /// `coin` (= `RandomBelow(2)`, rolled by the caller) picked, at the
    /// current stage point.
    ///
    /// * the anchor zones are frame 5's ch1 (left) / ch2 (right), laid out
    ///   centred on the stage point (`f3EBE` @0x3F14-0x3F62);
    /// * a zone that pokes outside the stage horizontally flips the coin
    ///   (@0x2440-0x245A);
    /// * the mascot's bank bounds (92x144) hang from the zone: centred on its
    ///   x, top at `zone.bottom - h/4` (@0x24FA-0x2532);
    /// * side 0 pushed back inside the stage's left edge, side 1 inside its
    ///   right edge; if the pushed rect then overlaps the card's ch1 rect
    ///   (`this+0x8C`, filled by the same `f3EBE`) it is parked at -10000
    ///   (@0x2532-0x25B0) — neither ever fires on a 640-wide stage;
    /// * the sprite's pos becomes the rect's centre (@0x25B6-0x2618). pos is
    ///   the centre of the frame CURRENT at that moment, and L135's links keep
    ///   the later frames design-aligned to it, so the design offset is fixed
    ///   here through the current frame's own `w>>1, h>>1`.
    ///
    /// That last point is why the first trailer's mascot sits 24 px lower
    /// than every later one: on the first call the sequence is still on the
    /// ctor's frame (a 92x92 record; `h>>1` = 46), while every later call
    /// comes straight from the walk-off's last frame 428 (92x140, 70).
    /// Golden `qemu/coming-soon.mp4`: first placement (right, stage cy 276)
    /// png at (525,341) = `cy + 65`; later ones (left, cy 279 / 251) at
    /// (21,322) / (21,294) = `cy + 43`. (The Basilisk capture's first two
    /// trailers are the same +24-26 px outlier.)
    fn place_mascot(&mut self, coin: u16) {
        let (sl, _st, sr, _sb) = STAGE;
        let pt = (self.stage_cx, self.stage_cy);
        let zone = |s: u16| self.channel_rect(BASE_POSTER, MASCOT_ANCHOR_FNO, s as i32 + 1, pt);
        let mut side = coin;
        if let Some(z) = zone(side) {
            if z.0 < sl || sr < z.2 {
                side = (side == 0) as u16;
            }
        }
        // GAP(anchor frame): a pack without frame 5's part table leaves the
        // mascot where it was.
        let Some(z) = zone(side) else { return };
        let (mw, mh) = self.series_wh(BASE_MASCOT);
        let dx = (z.0 + (z.2 - z.0) / 2) - mw / 2;
        let dy = z.3 - mh / 4;
        let (mut l, mut t, mut r, mut b) = (dx, dy, dx + mw, dy + mh);
        let card = self.channel_rect(BASE_POSTER, POSTER_FNO[self.cur_movie], 1, pt);
        let mut park = false;
        if side == 0 {
            if l < sl {
                (l, r) = (sl, r + (sl - l));
                park = card.is_some_and(|c| c.0 < r);
            }
        } else if sr < r {
            (l, r) = (l - (r - sr), sr);
            park = card.is_some_and(|c| l < c.2);
        }
        if park {
            (l, t, r, b) = (l - 10_000, t - 10_000, r - 10_000, b - 10_000);
        }
        let pos = (l + (r - l) / 2, t + (b - t) / 2);
        // The frame current at placement. GAP(ctor frame): the mascot ctor
        // (fn165 @0x1E4E) leaves the sequence on index 0 of its table, read
        // here as the bank's first record (frameNum 1, 92x92) — which the
        // golden's +24 px first placement bears out.
        let cur = self
            .pack
            .frame(BASE_MASCOT, self.m_frame)
            .or_else(|| self.pack.series(BASE_MASCOT).first().and_then(|s| s.frames.first()));
        if let Some(f) = cur {
            self.mascot_off = (pos.0 - (f.w >> 1) - f.bx, pos.1 - (f.h >> 1) - f.by);
        }
        self.mascot_side = side;
    }

    /// Roll the per-trailer stage point — the state-0 placement block,
    /// listing @0x0E68-0x0F8C (the arm taken while `g03EC != 0`; the
    /// `g03EC == 0` arm @0x0F8E re-clips to the card's own rect and leaves
    /// the point alone).
    ///
    /// ```text
    /// r = g03BA->GetBounds()                     @0x0E76  vtbl+0x1C
    /// r.ToOrigin()                               @0x0E80  Resource.f4038
    /// r.Offset(stage.left, stage.top)            @0x0E90  XRect::Offset
    /// if r.right < stage.right && r.bottom < stage.bottom {   @0x0EC4/@0x0ECE
    ///     r.Offset(RandomBelow(stage.right  - r.right),       @0x0F1C-0x0F2C
    ///              RandomBelow(stage.bottom - r.bottom))      @0x0F32-0x0F42
    ///     pt = (r.left + (r.right  - r.left)/2,               @0x0F4C-0x0F6A
    ///           r.top  + (r.bottom - r.top )/2)               @0x0F6E-0x0F88
    /// } else {                                                @0x0ED4
    ///     pt = centre of the stage rect                       @0x0ED4-0x0F10
    /// }
    /// this+0xC4 = pt.h; this+0xC6 = pt.v
    /// ```
    ///
    /// So the page is dropped at a uniformly random position that keeps it
    /// STRICTLY inside the stage (flush left/top allowed, at least one pixel
    /// of margin right/bottom), and the stage point is that placed page's
    /// centre. On the 640x480 stage with a 638x389 page that is
    /// `x = 319 + RandomBelow(2)` and `y = 194 + RandomBelow(91)`.
    ///
    /// Checked against `emu/captures/qemu/coming-soon.mp4` (20 fps, 1x): the
    /// horizontal range is two pixels wide, which is why every trailer in
    /// every capture reads centre 320 (the 358-wide card at x 140-497 and
    /// x 139-496 — the one-pixel disagreement the old note called measurement
    /// slop IS the roll); and the vertical range 194..284 contains the run's
    /// three trailers (197, 197, 254) and the reel's 229. The 8-bit capture's
    /// 289 sits five pixels outside it, and that capture is already known to
    /// carry a whole-stage vertical offset (see the module note), so it is not
    /// treated as a counter-example.
    ///
    /// `RandomBelow` is the seg-132 generator (`Ctx::rng`), per §2 — this
    /// module is not one of the four ANSI-`rand()` modules.
    fn roll_stage_point(&mut self, ctx: &mut Ctx) {
        let (sl, st, sr, sb) = STAGE;
        let (pw, ph) = self.page_wh();
        // r after ToOrigin + Offset(stage.left, stage.top).
        let (rl, rt, rr, rb) = (sl, st, sl + pw, st + ph);
        if rr < sr && rb < sb {
            let dh = ctx.rng.pct((sr - rr) as u32) as i32;
            let dv = ctx.rng.pct((sb - rb) as u32) as i32;
            let (rl, rt, rr, rb) = (rl + dh, rt + dv, rr + dh, rb + dv);
            self.stage_cx = rl + (rr - rl) / 2;
            self.stage_cy = rt + (rb - rt) / 2;
        } else {
            // @0x0ED4: the page does not fit — centre the stage itself.
            self.stage_cx = sl + (sr - sl) / 2;
            self.stage_cy = st + (sb - st) / 2;
        }
    }

    /// Screen position of the current movie's poster compound: its OFtb
    /// bounds centred on the stage point (see the LAYOUT note).
    fn poster_origin(&self) -> (i32, i32) {
        let (w, h) = POSTER_WH[self.cur_movie];
        (self.stage_cx - w / 2, self.stage_cy - h / 2)
    }

    /// A compound-relative rect `(x, y, w, h)` in screen coordinates.
    fn stage_rect(&self, r: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
        let (ox, oy) = self.poster_origin();
        (ox + r.0, oy + r.1, r.2, r.3)
    }

    /// One slot of the current movie's card palette, `clut 20000+m`
    /// (§6/Appendix B): slot 0 = card panel, slot 5/6 = text ink, slot 7 =
    /// accent. See the LAYOUT note for how that was established.
    fn card_slot(&self, slot: u16) -> Option<[u8; 3]> {
        self.pack
            .meta
            .palettes
            .get(&(BASE_POSTER as usize + self.cur_movie).to_string())
            .and_then(|p| p.get(&slot))
            .copied()
    }

    /// First candidate colour that separates from the field, else the
    /// generic contrast ink (APPROXIMATIONS #5b).
    fn readable(&self, slots: &[u16]) -> [u8; 3] {
        let field = self.field();
        let luma = |c: [u8; 3]| {
            (299 * c[0] as i32 + 587 * c[1] as i32 + 114 * c[2] as i32) / 1000
        };
        let fl = luma(field);
        for &s in slots {
            if let Some(c) = self.card_slot(s) {
                if (luma(c) - fl).abs() >= 60 {
                    return c;
                }
            }
        }
        engine::contrast_ink(field)
    }

    /// Is the trailer card on screen? State 1 (title card) draws a bare
    /// field — the golden's 1.0 s black gap between trailers, see the
    /// errata. State 0 erases the old card only when it FIRES, so while it
    /// waits out the post-walk-off dwell the old card is still up (golden:
    /// mascot gone at t 7.65 / 30.85, card gone at 12.60 / 35.85).
    fn card_up(&self) -> bool {
        match self.state {
            State::Poster | State::Hold | State::NextLine => true,
            State::WaitDwell => self.card_live,
            _ => false,
        }
    }

    /// §9 type-on budget at `now`: how many SOURCE characters of the title and
    /// of the bullet run have been laid down. Metering in source characters
    /// (not pixels, not wrapped lines) keeps the trailer the golden's length
    /// even though the engine's glyph is ~1.6× the original's width
    /// (APPROXIMATIONS #1).
    fn typed(&self, now: u64, title_len: usize) -> (usize, usize) {
        let ms = now.saturating_sub(self.card_at);
        let title = (ms * CRAWL_TITLE_CPS / 1000) as usize;
        if title < title_len {
            return (title, 0);
        }
        // The title finishes at its own rate; the bullets start from there.
        let title_ms = title_len as u64 * 1000 / CRAWL_TITLE_CPS;
        let rest = ms.saturating_sub(title_ms);
        (title_len, (rest * CRAWL_BULLET_CPS / 1000) as usize)
    }

    /// Crawl ink: the movie's own text colour (clut slot 5).
    fn ink(&self) -> [u8; 3] {
        self.readable(&[5, 0, 7])
    }

    /// Bullet tick accent (clut slot 7 — the red slash on Tippy's card).
    fn accent(&self) -> [u8; 3] {
        self.readable(&[7, 5, 0])
    }
}

/// Greedy word wrap to `cols` characters; never returns an empty vec, and
/// a single over-long word is emitted on its own line rather than dropped.
fn wrap(text: &str, cols: usize) -> Vec<String> {
    if cols == 0 {
        return vec![text.to_string()];
    }
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.chars().count() + 1 + word.chars().count() <= cols {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    out.push(line);
    out
}

impl ComingSoon {
    /// STR# 500 fetch — GetIndString(500, (m-1)*12 + n, …) @0x3614/0x390E/
    /// 0x3A0C/0x3D92 (§6), from the pack (`Pack::strings(500)`, meta.json
    /// `strings.500`, 228 items). A missing item is an empty line.
    fn crawl_item(&self, idx: usize) -> String {
        self.pack.strings(STR_CRAWL).get(idx).cloned().unwrap_or_default()
    }
}

impl Module for ComingSoon {
    fn name(&self) -> &'static str {
        "Coming Soon!"
    }

    /// §2: the four live controls. Index 2 is the dead slot — kept so
    /// control indices match the original's f3B72 numbering exactly (§13.3).
    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Pace".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 22, // 0x16 → bucket 1 "Mellow"
            },
            ControlDef {
                name: "Show".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    // MENU 1001, from the pack.
                    items: self.pack.popup_items(1001, 2, false),
                },
                // Shipped mVal 1001 default is 0 → g03EE = -1 → In Order out
                // of the box (§13.3); popup index 1 maps to raw 2 → same path.
                default: 1,
            },
            ControlDef {
                name: "(unused)".into(),
                kind: ControlKind::Checkbox,
                default: 0, // dead control slot: read as 0, stored, never used
            },
            ControlDef {
                name: "Music".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 22, // 0x16 → "Once"
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.pace_raw = value.clamp(0, 100),
            // engine popup values are 0-based; the original control was
            // 1-based (1 Random / 2 In Order) — map back to the raw domain.
            1 => self.show_raw = value.clamp(0, 1) + 1,
            2 => self.dead2 = value, // g03EA; read_controls() zeroes it as the C does
            3 => self.music_raw = value.clamp(0, 100),
            _ => {}
        }
        self.read_controls();
    }

    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    /// §10's music channel — the shared bank's `cmid` 20, on its own mix.
    /// See [`Module::music`] and MUSIC ERRATUM 1.
    fn music(&self) -> Option<(u32, u32)> {
        (self.music_plays > 0 && self.pack.has_song(SONG_COMING_SOON))
            .then_some((SONG_COMING_SOON, self.music_plays))
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        self.now = now; // the draw side has no Ctx; §9's type-on needs it
        // One module frame per Mac tick: no module-wide gate (the old 80 ms
        // `MS_FRAME` gate is SUPERSEDED — it was the mascot's own 75 ms idle
        // delay, now in fn168's `this+0xE4` gate; see the GESTURE ERRATUM).

        // Boot (§3): tagline roll → AD message line, music first play,
        // first movie pickup happens through state 0 with deadline 0.
        if !self.started {
            self.started = true;
            // §3 @0x0756–0x0760: one of STR# 128..132 via RandomBelow(5) is
            // pushed to the AD message line (Required1.f4272). The packed
            // resources carry the full "Coming Soon!: <tagline>" string;
            // rendered in texts(). A pack without it leaves the line empty.
            let tag = ctx.rng.pct(TAGLINE_COUNT) as u16;
            self.msg_line = self
                .pack
                .strings(STR_TAGLINE_FIRST + tag)
                .first()
                .cloned()
                .unwrap_or_default();
            self.music_started = self.music_left != 0;
            self.music_ends_at = now + self.tune_ms();
            if self.music_started {
                self.music_plays += 1; // first play (Sound.f0C9C @0x0D6A)
            }
        }

        self.mascot_tick(ctx);

        // ---- master state machine MOD.f0CC2 (§5) --------------------------
        match self.state {
            State::WaitDwell => {
                // @0x0DAC: wait for deadline. On firing the C zeroes the
                // deadline, the click substate and the mailbox (@0x0DB4) and
                // erases the old card (@0x0DC0-0x0E18).
                if now >= self.deadline {
                    self.deadline = 0;
                    self.substate = 0;
                    self.mailbox = 0;
                    self.card_live = false;
                    // g03EC gates the re-stage (@0x0E1C). SET: re-roll the
                    // stage point (@0x0E68-0x0F8C, placement first, playlist
                    // second — so its two RandomBelow rolls come off
                    // `ctx.rng` ahead of the pick). CLEAR (@0x0F9E): the new
                    // trailer goes up on the OLD point; the C only re-clips
                    // the canvas to the card rect, which draws nothing here.
                    if self.restage {
                        self.roll_stage_point(ctx);
                    }
                    let m = self.pick_movie(ctx); // §2.2 pick next movie
                    self.start_movie(ctx, m); // MOD.f3EBE @0x1106
                    self.deadline = now + MS_TITLE; // +1000 ms @0x110A
                    // @0x1112-0x113A: g03DC = g03EC ? 2 : 0, g03DA = (g03EC != 0)
                    // — on a re-stage the mascot gets msg 1 (come on stage).
                    if self.restage {
                        self.mascot_mail = 1;
                    }
                    self.state = State::Title; // SetState(1) @0x114A
                }
            }
            State::Title => {
                // @0x114E: title-card phase; deadline check @0x121C. Nothing
                // is drawn here — the golden holds a bare field (plus the
                // mascot) for 1.033 s / 1.067 s between trailers and puts the
                // panel, the poster, the logo and the first title glyph on
                // screen together, one frame into state 2. See the errata.
                if now >= self.deadline {
                    self.state = State::Poster; // SetState(2) @0x125A
                    self.card_at = now; // §9 type-on clock starts here
                    self.card_live = true;
                }
            }
            State::Poster => {
                // @0x125E: poster + music. While g03F2 != 0 and the tune is
                // done, restart (Sound.f0C9C @0x12BC) and g03F2-- (@0x12C8).
                if self.music_left != 0 && self.music_started && now >= self.music_ends_at {
                    // The music is its OWN channel (engine::music): a replay
                    // must not touch `ctx.sounds`. The old line here cleared
                    // the sfx queue on every replay, which would have eaten a
                    // salesman voice line that happened to fire on the same
                    // tick — and the capture shows the two coexisting (the
                    // tune runs unbroken across the voice cues at t = 4.84 /
                    // 13.80 / 19.01, and its own kick drum sounds at 12.82).
                    self.music_ends_at = now + self.tune_ms();
                    self.music_plays += 1;
                    if self.music_left > 0 {
                        self.music_left -= 1;
                    }
                }
                // Click-to-skip (§5 state 2, f41E0 @0x12FE; §13.6): the held
                // button is polled per tick → substate=1, deadline=0, and msg
                // 2 to the card, which answers 4 straight away (fn174 state 9).
                if ctx.mouse_down {
                    self.substate = 1;
                    self.deadline = 0;
                }
                // The exit is the CARD's msg 4 in the mailbox (@0x1456) —
                // posted when its type-on runs out of bullets — not a
                // deadline. Then +2000 ms when substate==0 (@0x149E) and
                // SetState(3) (@0x14C0), whose EXIT 0x4002 runs at once and
                // re-arms the deadline to now + g0052[b]/2 when substate==0
                // (@0x153A). ORIGINAL QUIRK kept: the +2000 is overwritten
                // before anything reads it, so state 3 is a full dwell.
                if self.substate == 1 || self.type_on_done(now) {
                    if self.substate == 0 {
                        self.deadline = now + MS_POSTER_HOLD;
                    }
                    // EXIT 0x4002
                    if self.substate == 0 {
                        self.deadline = now + self.dwell();
                    }
                    self.state = State::Hold;
                }
            }
            State::Hold => {
                // @0x1556: hold phase. Click-to-skip (§5 state 3, @0x155E;
                // §13.6): held button → substate=1, deadline=0.
                if ctx.mouse_down {
                    self.substate = 1;
                    self.deadline = 0;
                }
                // Deadline reached @0x163C: g03EC = 0, then (substate 0 only)
                // RandomBelow(5 - bucket) == 0 → g03EC = 1 (@0x165E-0x1674).
                if now >= self.deadline {
                    self.restage = false;
                    if self.substate == 0 {
                        let denom = (5 - self.pace_bucket).max(0) as u32;
                        if ctx.rng.pct(denom) == 0 {
                            self.restage = true; // g03EC = 1 @0x1672
                        }
                    }
                    if !self.restage {
                        // @0x16BA: one more dwell, SetState(4) @0x16E6.
                        if self.substate == 0 {
                            self.deadline = now + self.dwell();
                        }
                        self.state = State::NextLine;
                    } else if self.mascot_mail == 0 {
                        // @0x1690-0x16A8: g03DA = 6, g03DC = 2 — tell the
                        // mascot to walk off — and SetState(4). (Taken only
                        // while that mailbox is free; the port's is.)
                        self.mascot_mail = 6;
                        self.state = State::NextLine;
                    }
                }
            }
            State::NextLine => {
                // @0x16EA. Click-to-skip (§5 state 4; §13.6): held button →
                // substate=1, deadline=0.
                if ctx.mouse_down {
                    self.substate = 1;
                    self.deadline = 0;
                }
                if !self.restage {
                    // g03EC clear (@0x17CA): deadline passed (strictly) →
                    // SetState(0) @0x1846, and state 0 fires on its next
                    // frame because the deadline is already behind it.
                    if now > self.deadline {
                        self.state = State::WaitDwell;
                    }
                } else if self.mailbox == 5 {
                    // g03EC set: wait for the mascot's msg 5 (it has walked
                    // off, @0x17E0), arm ONE MORE dwell and SetState(0)
                    // (@0x17F4-0x181E). State 0 then waits that dwell out
                    // with the old card still standing and the stage empty
                    // of mascot — the golden's 4.95 s / 5.00 s.
                    self.mailbox = 0;
                    self.deadline = now + self.dwell();
                    self.state = State::WaitDwell;
                }
            }
            State::Exit => {
                // @0x1858: state 5. Each card/mascot 4-or-5 in the mailbox
                // bumps substate (@0x192A); at 2 → deadline 0, g03EC = 1,
                // SetState(0) (@0x1958-0x1978). ENTER 0x8005 zeroes substate.
                // GAP(state 5 entry): fn149 never calls SetState(5) and no
                // other caller is decoded, so nothing in this port enters it.
                if self.mailbox == 4 || self.mailbox == 5 {
                    self.substate += 1;
                    self.mailbox = 0;
                }
                if self.substate == 2 {
                    self.deadline = 0;
                    self.restage = true;
                    self.state = State::WaitDwell;
                }
            }
        }

        self.deliver_mascot_mail(ctx);
    }

    fn rects(&self, out: &mut Vec<engine::RectDraw>) {
        // The card panel (OFtb ch1, art 87 — a missing art id in the rip).
        // The golden shows it as a solid per-movie colour, and clut
        // 20000+m slot 0 IS that colour ("The White Saver"'s slot 0 is
        // (249,249,249): the joke card really is blank white). PaintRect
        // stand-in via the engine's RectDraw, drawn under the poster art.
        if self.card_up() {
            if let Some(color) = self.card_slot(0) {
                let (px, py) = self.poster_origin();
                let (cx, cy, cw, ch) = POSTER_CARD[self.cur_movie];
                out.push(engine::RectDraw {
                    x: px + cx,
                    y: py + cy,
                    w: cw,
                    h: ch,
                    color,
                });
            }
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        // Marquee stage + poster compound (§7): one compound per movie,
        // drawn at its packed compound bounds, from the poster phase on
        // (state 2 — state 1 is a blank 1 s, see the errata). The
        // title-card / bullet crawl text renders in texts() (§9).
        //
        // The "doubled mascot" live-test report is CLOSED, and it was never
        // this file's doing: exactly one SpriteDraw is pushed for
        // BASE_MASCOT below, unconditionally, every tick — see
        // `mascot_art_emitted_at_most_once_per_frame`. The second head is
        // the salesman's DROP SHADOW, and it is authentic: the DEPTH=32
        // golden has it too (`coming-soon.mp4` f614 — a hard black
        // head-and-shoulder silhouette up-and-right of him, inside the
        // spotlight). totally-twisted `51349d6` found the mechanism (the
        // compound walker's this+0x11E latch, set for chanNum 3..8 and
        // XORed into the blit flags, selects the RLE segment's MASKED entry
        // — the shadow channels blit as black silhouettes) and re-composed
        // the pack, so compounds/10000 now carries a black shadow instead
        // of a second full-colour head. Nothing to fix here; do not "mask"
        // it away.
        if self.card_up() {
            let (px, py) = self.poster_origin();
            if let Some(f) = self.pack.frame(BASE_POSTER, POSTER_FNO[self.cur_movie]) {
                // §6/Appendix B: start movie registers the per-movie clut
                // 20000+index (Canvas2.f3B34 chain @0x4016). The engine
                // resolves it through the pack's base_clut; this pack has
                // none, so the recolour is a documented no-op — see
                // APPROXIMATIONS #7. clut ids 20000..20018 verified in the
                // pack meta.json `palettes`.
                let pal = BASE_POSTER as u16 + self.cur_movie as u16;
                // The packed bx/by are the compound's DESIGN-space bounds;
                // the stage placement is the centring above, not those.
                out.push(SpriteDraw { flip: false, pal, png: f.png.clone(), x: px, y: py });
            }
        }
        // Mascot (§8): on stage from its first call-on to its walk-off, placed by
        // `place_mascot` (@0x2412-0x2618). Its solid black
        // backing below the spotlight is authentic and vanishes into the
        // black field (APPROXIMATIONS #8).
        if self.mascot_hidden {
            return;
        }
        if let Some(f) = self.pack.frame(BASE_MASCOT, self.m_frame) {
            out.push(SpriteDraw {
                flip: self.mascot_side == 1, // side 1 = mirrored (vtbl+0x0C @0x2670)
                pal: 0,
                png: f.png.clone(),
                x: f.bx + self.mascot_off.0,
                y: f.by + self.mascot_off.1,
            });
        }
        // Series 13000/13010 silhouettes: visual role UNCERTAIN (§14.4) —
        // not drawn.
        let _ = (SCREEN_W, SCREEN_H); // compound bounds are already screen-space
    }

    /// §9 text-crawl render (class-3 markup renderer) + §3 AD message line.
    /// Markup grammar (@0x683C scans for '#'; dispatch @0x319A–0x3210):
    /// `#FR0/#FR1` frame style, `#Jc/#Jl/#Jr` justification (sticky across
    /// `#R` breaks — that is what makes the §13.4 typos whole-card quirks),
    /// `#R` line break.
    fn texts(&self, out: &mut Vec<TextDraw>) {
        // ---- §9 text crawl --------------------------------------------
        // Both blocks live INSIDE the card: the title in the compound's ch8
        // rect, the bullets in its ch9 rect (see the LAYOUT note).
        if self.card_up() {
            let ink = self.ink();
            let accent = self.accent();

            // --- title card (STR# 500 item 1, `#FR0`). Lines are distributed
            // down the ch8 block and justified within it — which is what
            // makes the §13.4 typos read as a left- and a right-shoved card
            // instead of two identical centred ones. Each line is laid out at
            // its FINAL justified x and then typed in left to right (§14.6):
            // the golden's first title line sits at module x 189 from the
            // frame that holds only its `T`.
            let (tx, ty, tw, th) = self.stage_rect(POSTER_TITLE_BOX[self.cur_movie]);
            let (_, tlines) = parse_crawl(&self.crawl_item(self.cur_movie * 12));
            let title_chars: usize = tlines.iter().map(|(_, t)| t.chars().count()).sum();
            let (mut budget, mut bullet_budget) = self.typed(self.now, title_chars);
            let n = tlines.len().max(1) as i32;
            let pitch = th / n;
            let gh = 8 * CRAWL_TITLE_SCALE as i32;
            for (i, (just, text)) in tlines.into_iter().enumerate() {
                let w = engine::font::text_width(&text, CRAWL_TITLE_SCALE);
                let x = match just {
                    1 => tx + (tw - w) / 2, // #Jc
                    2 => tx + tw - w,       // #Jr
                    _ => tx,                // #Jl
                };
                let len = text.chars().count();
                let take = budget.min(len);
                budget -= take;
                if take == 0 {
                    break;
                }
                out.push(TextDraw {
                    text: text.chars().take(take).collect(),
                    x,
                    y: ty + i as i32 * pitch + (pitch - gh) / 2,
                    color: ink,
                    scale: CRAWL_TITLE_SCALE,
                });
                if take < len {
                    break; // still typing this line
                }
            }

            // --- bullets (items 2..12, `#FR1#Jl`): the slots §5's pace roll
            // won at movie start (`shown`), typed in one unbroken run right
            // after the title, each with an accent tick in the block's
            // gutter. Word-wrapped to the block and clipped at its bottom
            // (APPROXIMATIONS #1).
            let (bx, by, bw, bh) = self.stage_rect(POSTER_BULLET_BOX[self.cur_movie]);
            let cols = (((bw - BULLET_INDENT) / 8).max(1)) as usize;
            let mut y = by + 2;
            'items: for &n in &self.shown {
                if bullet_budget == 0 || y + 8 > by + bh {
                    break; // not typed yet, or the block is full
                }
                out.push(TextDraw {
                    text: "/".into(),
                    x: bx + TICK_DX,
                    y: y + TICK_DY,
                    color: accent,
                    scale: TICK_SCALE,
                });
                let raw = self.crawl_item(self.cur_movie * 12 + n);
                let (_, lines) = parse_crawl(&raw);
                for (_just, text) in lines {
                    for piece in wrap(&text, cols) {
                        if y + 8 > by + bh {
                            break 'items;
                        }
                        let len = piece.chars().count();
                        let take = bullet_budget.min(len);
                        bullet_budget -= take;
                        if take == 0 {
                            break 'items;
                        }
                        out.push(TextDraw {
                            text: piece.chars().take(take).collect(),
                            x: bx + BULLET_INDENT,
                            y,
                            color: ink,
                            scale: CRAWL_BULLET_SCALE,
                        });
                        y += BULLET_LINE_H;
                        if take < len {
                            break 'items; // still typing this line
                        }
                    }
                }
                y += BULLET_ITEM_GAP;
            }
        }
        // ---- §3 AD message line ----------------------------------------
        // Required1.f4272 pushes the boot tagline to the host's message
        // line; the region itself is host UI, so we draw ours along the
        // bottom edge of the sim screen (APPROXIMATIONS #1).
        if engine::SHOW_QUIP_CAPTIONS && !self.msg_line.is_empty() {
            out.push(TextDraw {
                text: self.msg_line.clone(),
                x: 4,
                y: SCREEN_H - 10,
                color: engine::contrast_ink(self.field()),
                scale: 1,
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    // No tick_ms override: pacing in the original is entirely
    // deadline-driven against Resource.f4724 with AD calling the draw
    // entry at its usual rate (§11) — the 40 ms default matches.
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{Random15, RandomLong};

    /// §10 MUSIC — SONG 20, and MUSIC ERRATUM 1's identification.
    ///
    /// `emu/captures/qemu/coming-soon.wav` (60 s, wav ≈ wall, panel
    /// Music = Once). An offline render of the shared bank's `cmid` 20
    /// "Coming Soon.5" matches the whole capture at a single global lag of
    /// +34.70 s — 1.70 s into the render's second play, i.e. a real offset
    /// of +1.70 s, the same offset two independent windows (t = 0–20 and
    /// t = 20–36, f0–f400 and f400–f720) agree on. 46.0 % strongest-partial
    /// agreement over 63 windows, 90.5 % of 74 onsets inside 80 ms.
    ///
    /// The module's OWN `cmid` 10 "Rhumba Data" — the tune §10 and §14.3
    /// name — scores **7.5 %** on the same test. It is not what played.
    #[test]
    fn music_is_song_20_and_replays_on_the_real_tune_length() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let has_song = pack.has_song(SONG_COMING_SOON);
        let Some(mut m) = build(pack) else { return };
        // panel default sVal 1003 = 22 -> g03F2 = 22/20 = 1 replay
        assert_eq!(m.music_raw, 22);
        assert_eq!(m.music_left, 1);
        if has_song {
            assert_eq!(
                m.tune_ms(),
                33_000,
                "cmid 20 runs 33.00 s; TUNE_MS used to guess 30 s"
            );
        }
        let clock = Module::clock(&m);
        let mut ctx = Ctx {
            rng: RandomLong::new(5),
            rng15: Random15::new(5),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };
        let mut starts: Vec<u64> = Vec::new();
        let mut last: Option<(u32, u32)> = None;
        for i in 1..=6_000u64 {
            ctx.now_ms = clock.now_ms(i);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            // the music must never ride the single PRE-EMPTING sfx channel
            assert!(
                !ctx.sounds.contains(&SONG_COMING_SOON),
                "SONG 20 was pushed onto the sfx channel at {} ms",
                ctx.now_ms
            );
            let now = Module::music(&m);
            if now != last {
                if let Some((song, _)) = now {
                    assert_eq!(song, SONG_COMING_SOON);
                    starts.push(ctx.now_ms);
                }
                last = now;
            }
        }
        if !has_song {
            assert!(starts.is_empty(), "a pack with no song stays silent");
            return;
        }
        assert!(!starts.is_empty(), "the boot play never started");
        assert!(starts[0] < 200, "play 1 is a boot event, got {} ms", starts[0]);
        // g03F2 = 1 buys exactly one replay, and it waits out the tune.
        assert_eq!(starts.len(), 2, "Music = Once is play + one replay: {starts:?}");
        assert!(
            starts[1] >= 33_000,
            "the replay fired at {} ms, before the 33.00 s tune ran out",
            starts[1]
        );
        assert_eq!(m.music_left, 0, "g03F2 must be spent");
    }

    /// RATCHET (rewritten 2026-09-29) for fn168's per-state frame delays.
    /// The old test pinned EVERY mascot step to one 80 ms / 5-tick module
    /// gate — the wrong model: the 5 ticks the Basilisk capture measured
    /// (`coming-soon.mp4` t 30-50 s, periodogram 83.28 ms R 0.229; spotlight
    /// box 83.10 ms R 0.569) is the IDLE state's own 75 ms delay (ENTER
    /// 0x8002 @0x2968), and the entry run steps at 100 ms (ENTER 0x8001
    /// @0x268C). Golden `qemu/coming-soon.mp4`: the entry run's frames 7..61
    /// (54 steps) plus the two ticks into the first gesture take 5.75 s and
    /// 5.70 s from the mascot's first frame (f253 → 380 at f368; f717 → 155
    /// at f831) = **105-106 ms a step**. Old port: 83 ms for both.
    #[test]
    fn the_mascot_steps_75ms_idle_and_100ms_on_entry() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            return;
        };
        assert_eq!(Module::clock(&build(pack.clone()).unwrap()), TickClock::MacTick);
        let (mut idle, mut entry): (Vec<u64>, Vec<u64>) = (Vec::new(), Vec::new());
        for seed in [1u64, 3, 5, 7] {
            let Some(mut m) = build(pack.clone()) else { return };
            let mut last: Option<(u8, u32, u64)> = None;
            drive(&mut m, seed, 3_610, |m, now| {
                if m.m_frame == 0 || m.mascot_hidden {
                    last = None;
                    return;
                }
                let cur = (m.m_state, m.m_frame);
                if let Some((s, f, t)) = last {
                    // one step inside one run of one state
                    if (s, f + 1) == cur {
                        match s {
                            1 => entry.push(now - t),
                            // not 66 -> 67: ENTER 0x8002 draws 66 on the
                            // gate the gesture armed at 100 ms
                            2 if f != MASCOT_RUN_IDLE => idle.push(now - t),
                            _ => {}
                        }
                    }
                }
                if last.is_none_or(|(s, f, _)| (s, f) != cur) {
                    last = Some((cur.0, cur.1, now));
                }
            });
        }
        assert!(idle.len() > 100 && entry.len() > 50, "idle {} entry {}", idle.len(), entry.len());
        for &g in &idle {
            assert!((83..=84).contains(&g), "idle step {g} ms; 75 ms is 5 Mac ticks");
        }
        for &g in &entry {
            assert!((99..=117).contains(&g), "entry step {g} ms; 100 ms is 6-7 Mac ticks");
        }
        let mean = entry.iter().sum::<u64>() as f64 / entry.len() as f64;
        assert!(
            (103.0..=108.0).contains(&mean),
            "entry step mean {mean:.1} ms; golden 105-106"
        );
    }

    /// Voice cues seen on one 60 s run: (ms, slot, mascot state, frame).
    fn cue_log(pack: &Pack, seed: u64, ticks: u64) -> Vec<(u64, usize, u8, u32)> {
        let Some(mut m) = build(pack.clone()) else { return Vec::new() };
        let clock = Module::clock(&m);
        let mut ctx = Ctx {
            rng: RandomLong::new(seed),
            rng15: Random15::new(seed as u32),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };
        let mut out = Vec::new();
        for t in 1..=ticks {
            ctx.now_ms = clock.now_ms(t);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            for &id in &ctx.sounds {
                if let Some(slot) = VOICE_SND.iter().position(|&v| v == id) {
                    out.push((ctx.now_ms, slot, m.m_state, m.m_frame));
                }
            }
        }
        out
    }

    /// RATCHET for fn168's gesture machine (states 1-0xB, fn167) against
    /// `qemu/coming-soon-av.mp4` (60 s; audio-captures.md "Coming Soon"):
    /// **7 voice cues in 60 s, slots {1, 3, 6} only** — 10006 twice, each
    /// right after a call-on; one slot-1..5 cue per gesture (entry → 0xB,
    /// 4 and 7 straight off the entry run; 8 and 10 off the idle 1-in-32
    /// roll, each fronted by run 0x6B), and no "Wow!" (slot 0 lives only in
    /// gesture 5's run). The old port stepped every run in pack order and so
    /// hit every voice frame once per 24.6 s lap: 20 cues a minute including
    /// slot 0 twice and 10006 at every lap. Pinned here: the rate band, 10006
    /// only on the entry run, slot 0 only in gesture 5, slots 1-5 only in a
    /// gesture's middle run.
    #[test]
    fn the_salesman_speaks_per_gesture_not_per_lap() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            return;
        };
        let seeds = [1u64, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31];
        let (mut total, mut wow) = (0usize, 0usize);
        for seed in seeds {
            for (t, slot, _state, f) in cue_log(&pack, seed, 3_610) {
                total += 1;
                match slot {
                    6 => assert!(
                        (MASCOT_RUN_ENTER..=61).contains(&f),
                        "seed {seed} {t} ms: 10006 off the entry run (frame {f})"
                    ),
                    0 => {
                        wow += 1;
                        assert_eq!(f, EV_SLOT0, "seed {seed} {t} ms: Wow! off frame {f}");
                    }
                    _ => assert!(
                        EV_RANDOM5.contains(&f),
                        "seed {seed} {t} ms: slot {slot} off frame {f}"
                    ),
                }
            }
        }
        let per_min = total as f64 / seeds.len() as f64;
        assert!(
            (3.0..=11.0).contains(&per_min),
            "{per_min:.1} cues a minute; golden 7 (the pack-order lap gave 20)"
        );
        let wow_per_min = wow as f64 / seeds.len() as f64;
        assert!(wow_per_min < 1.0, "{wow_per_min:.2} Wow!s a minute; golden 0 (the lap gave 2)");
    }

    /// RATCHET for the entry run's timing: "Coming Soon!" (10006, frame 17)
    /// sounds exactly once per call-on, ten 100 ms steps after the mascot's
    /// first frame (7 — ENTER 0x8001 draws 6 and UPDATE 1 overdraws it the
    /// same tick). Golden: 10006 at 13.80 / 36.99 s on the audio clock, the
    /// mascot's first frame at 12.65 / 35.85 on video = 1.15 / 1.14 s, which
    /// includes the capture's ~0.1 s audio lag (the gesture cues 390 and 278
    /// sound 0.06-0.11 s after their frames show) — 1.04-1.05 s frame to
    /// frame. The old port, 11 steps of 83 ms from frame 6: 0.914 s.
    #[test]
    fn coming_soon_sounds_ten_entry_steps_after_the_call_on() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            return;
        };
        let mut calls = 0;
        for seed in [1u64, 3, 5, 7, 9, 11, 13, 15] {
            let Some(mut m) = build(pack.clone()) else { return };
            let clock = Module::clock(&m);
            let mut ctx = Ctx {
                rng: RandomLong::new(seed),
                rng15: Random15::new(seed as u32),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: 0,
                local_hms: (12, 0, 0),
                mouse: (-1, -1),
                mouse_down: false,
            };
            let mut shown_at: Option<u64> = None;
            let mut drawn = false;
            let mut heard = 0;
            for t in 1..=12_000u64 {
                ctx.now_ms = clock.now_ms(t);
                ctx.sounds.clear();
                m.tick(&mut ctx);
                let now_drawn = !m.mascot_hidden && m.m_frame != 0;
                if now_drawn && !drawn {
                    assert!(shown_at.is_none() || heard == 1, "seed {seed}: a call-on went unheard");
                    assert_eq!(m.m_frame, 7, "seed {seed}: the entry shows frame {}", m.m_frame);
                    shown_at = Some(ctx.now_ms);
                    heard = 0;
                    calls += 1;
                }
                drawn = now_drawn;
                if ctx.sounds.contains(&VOICE_SND[6]) {
                    let d = ctx.now_ms - shown_at.expect("10006 before any call-on");
                    assert!(
                        (1_000..=1_200).contains(&d),
                        "seed {seed}: 10006 {d} ms after the mascot showed; golden 1040-1050 \
                         (1140-1150 on the lagging audio clock)"
                    );
                    heard += 1;
                }
            }
        }
        assert!(calls >= 16, "only {calls} call-ons");
    }

    #[test]
    fn coming_soon_flows() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = make(pack) else {
            eprintln!("make() returned None — skipping");
            return;
        };
        let tick = m.tick_ms();
        let mut now = 0u64;
        let mut got_sprites = false;
        for _ in 0..500 {
            let mut ctx = Ctx {
                rng: RandomLong::new(1),
                rng15: Random15::new(1),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: now,
                local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
            };
            m.tick(&mut ctx);
            assert!(ctx.sounds.iter().all(|&s| (10_000..=10_006).contains(&s)));
            let mut out = Vec::new();
            m.sprites(&mut out);
            if !out.is_empty() {
                got_sprites = true;
            }
            now += tick;
        }
        assert!(got_sprites, "sprites() never produced output");
    }

    /// §13.4: a `#J` token must retarget the line it lands on, even when
    /// (as with every title card) the token precedes any text on that
    /// line. Before the fix, the first line's justification was hard-coded
    /// to 0 at `lines` construction time, and the `#J` handler updated
    /// only the local `just` variable — never `lines.last_mut()` — so
    /// every `#Jc`/`#Jr` title card rendered as if it were `#Jl`.
    #[test]
    fn parse_crawl_first_line_justification_takes_effect() {
        let (title, lines) = parse_crawl("#FR0#JcA Synthetic Title#Rin \"The Sequel\"");
        assert!(title);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (1, "A Synthetic Title".to_string())); // #Jc = 1
        assert_eq!(lines[1], (1, "in \"The Sequel\"".to_string())); // sticky across #R

        let (_, lines) = parse_crawl("#FR0#JlLeft Title#RSecond Line");
        assert_eq!(lines[0].0, 0); // #Jl = 0 (left)
        assert_eq!(lines[1].0, 0); // sticky

        let (_, lines) = parse_crawl("#FR0#JrRight Title#RSecond Line");
        assert_eq!(lines[0].0, 2); // #Jr = 2 (right)
        assert_eq!(lines[1].0, 2);

        // §13.4's documented original typos — the title cards of movies 6
        // and 13 (STR# 500 items 72 and 156) say #Jl and #Jr where every
        // other card says #Jc — must still take literal effect (kept, not
        // "fixed") now that #J actually does something.
        let Some(crawl) = packed_crawl() else { return };
        for (item, just) in [(72, 0), (156, 2)] {
            let (title, lines) = parse_crawl(&crawl[item]);
            assert!(title, "item {item} is a title card");
            assert!(lines.len() >= 2, "item {item}: {lines:?}");
            assert!(lines.iter().all(|l| l.0 == just), "item {item}: {lines:?}");
        }
    }

    /// Golden geometry, checked without the emulator: the DEPTH=32 capture
    /// (`d32-coming-soon/g_06..g_10`, the Aliens card) puts the card at
    /// screen (141,47), the title block at (141,55)-(499,126) with its
    /// first line starting at y 58, and the bullet block at
    /// (154,142)-(331,331) with its first bullet at (155,144). These are
    /// the numbers the whole layout hangs off; if the centring rule or a
    /// rect drifts, this is what says so.
    #[test]
    fn aliens_card_lands_on_the_golden_rects() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = build(pack) else { return };
        m.cur_movie = 1; // The Alien Abduction Survival Guide
        // The stage point the golden's trailers 1-2 were rolled onto. The
        // roll itself is `stage_point_obeys_the_0EC4_placement_rule`; this
        // test is about what the layout hangs off it.
        m.stage_cx = 320;
        m.stage_cy = 197;

        assert_eq!(m.poster_origin(), (141, 47), "compound origin");
        assert_eq!(
            m.stage_rect(POSTER_TITLE_BOX[1]),
            (141, 55, 358, 71),
            "ch8 title block"
        );
        assert_eq!(
            m.stage_rect(POSTER_BULLET_BOX[1]),
            (154, 142, 177, 189),
            "ch9 bullet block"
        );

        // Tippy's compound is 30 px wider and 69 px taller, so centring it
        // lands its card somewhere else entirely — the golden's (156, 13).
        m.cur_movie = 0;
        let (ox, oy) = m.poster_origin();
        assert_eq!((ox + POSTER_CARD[0].0, oy + POSTER_CARD[0].1), (156, 13));
    }

    /// RATCHET for the per-trailer stage point, listing @0x0E68-0x0F8C.
    ///
    /// The rule drops the poster sequence's own bounds rect ("the page")
    /// uniformly at random inside the stage, strictly inside on the right and
    /// bottom edges, and makes the stage point that page's centre. Three
    /// things are checked, and the pinned constant this replaced fails the
    /// third:
    ///
    /// 1. the page really is the 638x389 bank-wide bound the rule needs for
    ///    its horizontal range to be two pixels wide (the capture's 320);
    /// 2. every rolled point keeps the page inside the stage, and lands in
    ///    the x 319..320 / y 194..284 band that band-limits the capture's
    ///    measured card centres (197, 197, 254 in the run; 229 in the reel);
    /// 3. the point actually MOVES — a fixed point passes (2) trivially.
    #[test]
    fn stage_point_obeys_the_0ec4_placement_rule() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = build(pack) else { return };

        let (pw, ph) = m.page_wh();
        assert_eq!(
            (pw, ph),
            (638, 389),
            "series 20000's union bound is what @0x0E76 hands the rule"
        );

        let mut ctx = Ctx {
            rng: RandomLong::new(7),
            rng15: Random15::new(7),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };
        let (sl, st, sr, sb) = STAGE;
        let mut xs = std::collections::BTreeSet::new();
        let mut ys = std::collections::BTreeSet::new();
        for i in 0..200 {
            m.roll_stage_point(&mut ctx);
            let (cx, cy) = (m.stage_cx, m.stage_cy);
            // The page, back-computed from the point it produced, must sit
            // inside the stage the way @0x0EC4/@0x0ECE demand.
            let (l, t) = (cx - pw / 2, cy - ph / 2);
            assert!(
                l >= sl && t >= st && l + pw < sr && t + ph < sb,
                "roll {i}: page ({l},{t}) {pw}x{ph} escapes the stage {:?}",
                STAGE
            );
            assert!(
                (319..=320).contains(&cx) && (194..=284).contains(&cy),
                "roll {i}: stage point ({cx},{cy}) outside the capture's band"
            );
            // And no trailer's compound may clip, at any rolled point.
            for movie in 0..19 {
                m.cur_movie = movie;
                let (w, h) = POSTER_WH[movie];
                let (ox, oy) = m.poster_origin();
                assert!(
                    ox >= sl && oy >= st && ox + w <= sr && oy + h <= sb,
                    "roll {i}: movie {movie} compound ({ox},{oy}) {w}x{h} clips"
                );
            }
            xs.insert(cx);
            ys.insert(cy);
        }
        // The pinned STAGE_CY = 197 this replaced yields exactly one y.
        assert_eq!(xs.len(), 2, "the 2-wide horizontal roll never used both x");
        assert!(
            ys.len() >= 60,
            "200 rolls produced only {} distinct y — the point is not moving",
            ys.len()
        );
    }

    /// RATCHET for `g03EC`'s polarity and the trailer cadence of states 2-4
    /// (listing @0x0E1C, @0x1456-0x14C0, @0x153A, @0x163C-0x16E6,
    /// @0x17CA-0x1846). Golden = `emu/captures/qemu/coming-soon.mp4` (60 s,
    /// 20 fps, Pace Mellow), per-frame card/mascot bboxes:
    ///
    /// | trailer | card up | card off | mascot | stage point |
    /// |---|---|---|---|---|
    /// | Tippy | (<0) | 12.60 | walks off 7.65 | first |
    /// | Aliens | 13.65 | 23.75 | stays | RE-ROLLED (cy 276 → 279) |
    /// | Chernobyl | 24.75 | 35.85 | walks off 30.85 | kept (279) |
    /// | SimWrath | 36.85 | 46.95 | stays | RE-ROLLED (cy 251) |
    /// | Last Chance | 47.95 | 58.05 | stays | kept (251) |
    /// | (next) | 59.10 | — | — | kept (251) |
    ///
    /// So (a) the point moves ONLY on the trailer after a walk-off; (b) from
    /// the walk-off to the old card clearing is one dwell (4.95 s, 5.00 s);
    /// (c) a trailer the mascot stays through is up 10.10 s, of which the
    /// type-on is ≤ 0.10 s on this (fast) guest — two 5 s dwells after the
    /// card's msg 4. The old port re-rolled on EVERY trailer, never walked
    /// the mascot off, and held a card 2 s + 5 s (+5 s per extra roll).
    #[test]
    fn the_stage_moves_only_after_the_mascot_walks_off() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let (sl, st, sr, sb) = STAGE;
        let (mut walks, mut kept) = (0, 0);
        for seed in [1u64, 3, 5, 7, 9, 11, 13, 15] {
            let Some(mut m) = build(pack.clone()) else { return };
            let mut prev_state = m.state;
            let mut prev_hidden = m.mascot_hidden;
            let mut prev_card = false;
            let mut point: Option<(i32, i32)> = None;
            let mut walked_at: Option<u64> = None;
            let mut typed_at: Option<u64> = None;
            drive(&mut m, seed, 12_000, |m, now| {
                let card = m.card_up();
                // walk-off: the mascot leaves while the card is still up
                if !prev_hidden && m.mascot_hidden {
                    assert!(card, "seed {seed} {now} ms: the mascot left with no card up");
                    walked_at = Some(now);
                }
                if prev_state == State::Poster && m.state == State::Hold {
                    typed_at = Some(now);
                }
                if prev_card && !card {
                    match walked_at {
                        Some(w) => {
                            let d = now - w;
                            assert!(
                                (4_900..=5_200).contains(&d),
                                "seed {seed}: walk-off to card-off {d} ms; golden 4950/5000"
                            );
                        }
                        None => {
                            let d = now - typed_at.expect("card went up without typing");
                            assert!(
                                (9_900..=10_250).contains(&d),
                                "seed {seed}: card held {d} ms past its type-on; golden \
                                 10100 total with <= 100 of type-on"
                            );
                        }
                    }
                }
                if prev_state == State::WaitDwell && m.state == State::Title {
                    let p = (m.stage_cx, m.stage_cy);
                    if let Some(old) = point {
                        if walked_at.is_some() {
                            walks += 1;
                            assert!(m.mascot_mail == 0 && !m.mascot_hidden,
                                "seed {seed} {now} ms: the re-stage did not call the mascot back");
                        } else {
                            kept += 1;
                            assert_eq!(
                                p, old,
                                "seed {seed} {now} ms: the stage moved without a walk-off"
                            );
                            assert!(!m.mascot_hidden, "seed {seed}: the mascot vanished");
                        }
                    }
                    point = Some(p);
                    walked_at = None;
                    typed_at = None;
                }
                if card {
                    let (w, h) = POSTER_WH[m.cur_movie];
                    let (ox, oy) = m.poster_origin();
                    assert!(
                        ox >= sl && oy >= st && ox + w <= sr && oy + h <= sb,
                        "{now} ms: movie {} compound ({ox},{oy}) {w}x{h} off the stage",
                        m.cur_movie
                    );
                }
                prev_state = m.state;
                prev_hidden = m.mascot_hidden;
                prev_card = card;
            });
        }
        // Mellow: RandomBelow(4) == 0 per trailer, so both paths must show up.
        assert!(walks >= 3 && kept >= 3 * walks / 2, "walks {walks}, kept {kept}");
    }

    /// RATCHET for the mascot's side coin and anchor (fn168 @0x2412-0x2618,
    /// zones from `f3EBE` @0x3F14-0x3F62). Golden = `qemu/coming-soon.mp4`,
    /// best-matching pack frame at the tracked png box (mean |Δ| ≈ 14/255 —
    /// the 8-bit palette — against ≥ 18 for any other frame or mirror):
    ///
    /// | placement | stage point (from the card) | mascot png | mirrored |
    /// |---|---|---|---|
    /// | first (Tippy, f0-f152) | (319, 276) | (525, 341) | yes |
    /// | after walk-off 1 (Aliens, f253-f616) | (319, 279) | (21, 322) | no |
    /// | after walk-off 2 (SimWrath, f717-) | (319, 251) | (21, 294) | no |
    ///
    /// The old port pinned every frame at (22, 264), unmirrored.
    #[test]
    fn the_mascot_hangs_under_its_side_coins_zone() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let mascot_pngs: std::collections::HashSet<String> = pack
            .series(BASE_MASCOT)
            .iter()
            .flat_map(|s| s.frames.iter().map(|f| f.png.clone()))
            .collect();
        // (m_frame at the call, stage point, movie, coin) -> golden png, mirror
        let cases = [
            (0u32, (319, 276), 0usize, 1u16, (525, 341), true),
            (428, (319, 279), 1, 0, (21, 322), false),
            (428, (319, 251), 3, 0, (21, 294), false),
        ];
        for (cur, (cx, cy), movie, coin, want, mirrored) in cases {
            let Some(mut m) = build(pack.clone()) else { return };
            m.m_frame = cur;
            m.stage_cx = cx;
            m.stage_cy = cy;
            m.cur_movie = movie;
            m.place_mascot(coin);
            m.mascot_hidden = false;
            // any 92x140 frame of the entry run draws at the same box
            m.m_frame = 17;
            let mut out = Vec::new();
            m.sprites(&mut out);
            let s = out
                .iter()
                .find(|s| mascot_pngs.contains(&s.png))
                .expect("no mascot sprite");
            assert_eq!(
                ((s.x, s.y), s.flip),
                (want, mirrored),
                "call from frame {cur} at stage ({cx},{cy}), coin {coin}"
            );
        }
    }

    /// The coin is a fair `RandomBelow(2)` rolled only when the mascot is
    /// called on (msg 1), so the side can change only across a re-stage and
    /// both sides turn up. (The golden has 3 placements — R, L, L — too few
    /// to measure a rate; a capture with ~20 walk-offs would.)
    #[test]
    fn the_side_coin_is_tossed_only_on_a_call_on() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let (mut right, mut total) = (0, 0);
        for seed in [1u64, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23] {
            let Some(mut m) = build(pack.clone()) else { return };
            let mut prev: Option<(u16, (i32, i32))> = None;
            let mut prev_state = m.state;
            drive(&mut m, seed, 12_000, |m, now| {
                if m.mascot_hidden {
                    prev_state = m.state;
                    return;
                }
                let cur = (m.mascot_side, m.mascot_off);
                let called = prev_state == State::WaitDwell && m.state == State::Title;
                if prev != Some(cur) {
                    assert!(
                        called && (prev.is_none() || m.restage),
                        "seed {seed} {now} ms: the mascot moved without a call-on"
                    );
                    total += 1;
                    right += cur.0 as i32;
                }
                prev = Some(cur);
                prev_state = m.state;
            });
        }
        assert!(total >= 30, "only {total} placements");
        let frac = right as f64 / total as f64;
        assert!((0.3..=0.7).contains(&frac), "right side {right}/{total}");
    }

    /// Every crawl run must land inside its block, for every movie and
    /// every reveal count — this is the regression the old layout had
    /// (title and bullets floated at fixed screen y, wholly outside the
    /// card). The tick glyph is allowed its gutter to the left.
    #[test]
    fn crawl_stays_inside_the_card_text_blocks() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = build(pack) else { return };
        m.state = State::Poster; // the card is only up from the poster phase
        m.card_at = 0;
        // Worst case for the block: the "Fast" pace shows all 11 slots.
        m.shown = (1..=BULLET_SLOTS).collect();
        for movie in 0..19 {
            m.cur_movie = movie;
            // Sweep the type-on instead of the old line counter: every
            // intermediate reveal has to sit inside the blocks too.
            for step in 0..=40 {
                m.now = step * 300;
                let line = step as usize;
                let mut out: Vec<TextDraw> = Vec::new();
                m.texts(&mut out);
                let (tx, ty, tw, th) = m.stage_rect(POSTER_TITLE_BOX[movie]);
                let (bx, by, bw, bh) = m.stage_rect(POSTER_BULLET_BOX[movie]);
                for t in &out {
                    let w = engine::font::text_width(&t.text, t.scale);
                    let h = 8 * t.scale as i32;
                    let in_title = t.y >= ty && t.y + h <= ty + th && t.x >= tx && t.x + w <= tx + tw;
                    let in_bullets = t.y + TICK_DY.abs() >= by
                        && t.y <= by + bh
                        && t.x >= bx + TICK_DX
                        && t.x + w <= bx + bw;
                    assert!(
                        in_title || in_bullets,
                        "movie {movie} line {line}: {:?} at ({},{}) size {}x{} \
                         escaped title {:?} and bullets {:?}",
                        t.text,
                        t.x,
                        t.y,
                        w,
                        h,
                        (tx, ty, tw, th),
                        (bx, by, bw, bh),
                    );
                }
            }
        }
    }

    /// Every title fits its ch8 block at CRAWL_TITLE_SCALE — the widest is
    /// "The Tao of Tax Evasion" (22 chars = 352 px) in a 358-px block. If a
    /// bigger scale is ever tried, this is the wall it hits.
    /// The pack's STR# 500 crawl (228 items), or `None` (with a note) when
    /// the pack is absent.
    fn packed_crawl() -> Option<Vec<String>> {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return None;
        };
        let crawl = pack.strings(STR_CRAWL).to_vec();
        assert_eq!(crawl.len(), 19 * 12, "STR# 500 is 19 movies x 12 lines");
        Some(crawl)
    }

    #[test]
    fn every_title_fits_its_block_at_the_chosen_scale() {
        let Some(crawl) = packed_crawl() else { return };
        for movie in 0..19 {
            let (_, _, tw, th) = POSTER_TITLE_BOX[movie];
            let (_, lines) = parse_crawl(&crawl[movie * 12]);
            assert!(
                th >= lines.len() as i32 * 8 * CRAWL_TITLE_SCALE as i32,
                "movie {movie}: {} title lines do not fit {th} px",
                lines.len()
            );
            for (_, text) in lines {
                let w = engine::font::text_width(&text, CRAWL_TITLE_SCALE);
                assert!(w <= tw, "movie {movie}: {text:?} is {w} px in a {tw} px block");
            }
        }
    }

    /// Test rig: run the module on its own clock, keeping one RNG stream.
    fn drive(m: &mut ComingSoon, seed: u64, ticks: u64, mut f: impl FnMut(&ComingSoon, u64)) {
        let clock = Module::clock(m);
        let mut ctx = Ctx {
            rng: RandomLong::new(seed),
            rng15: Random15::new(seed as u32),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };
        for t in 1..=ticks {
            ctx.now_ms = clock.now_ms(t);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            f(m, ctx.now_ms);
        }
    }

    /// Total source characters the crawl has laid down right now.
    fn crawl_chars(m: &ComingSoon) -> usize {
        let mut out = Vec::new();
        m.texts(&mut out);
        out.iter()
            .filter(|t| t.text != "/")
            .map(|t| t.text.chars().count())
            .sum()
    }

    /// §5 state 1 draws NOTHING. `coming-soon.mp4`: the card clears at f615
    /// (t 20.500) and the next one lands at f646 (21.533) — 1.033 s of bare
    /// field plus the mascot, and again 1.067 s at f1147→f1179. The panel,
    /// the poster, the logo and the first title glyph all arrive together.
    /// The port used to draw the card from state 1, so there was no gap at
    /// all.
    #[test]
    fn the_title_phase_is_a_blank_second() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let mascot_pngs: std::collections::HashSet<String> = pack
            .series(BASE_MASCOT)
            .iter()
            .flat_map(|s| s.frames.iter().map(|f| f.png.clone()))
            .collect();
        let Some(mut m) = build(pack) else { return };
        let mut first_card: Option<u64> = None;
        let mut saw_title_state = false;
        drive(&mut m, 3, 200, |m, now| {
            let mut rects = Vec::new();
            m.rects(&mut rects);
            let mut sprites = Vec::new();
            m.sprites(&mut sprites);
            let poster = sprites.iter().any(|s| !mascot_pngs.contains(&s.png));
            let mascot = sprites.iter().any(|s| mascot_pngs.contains(&s.png));
            let mut texts = Vec::new();
            m.texts(&mut texts);
            match m.state {
                State::WaitDwell | State::Title => {
                    if m.state == State::Title {
                        saw_title_state = true;
                    }
                    assert!(
                        rects.is_empty() && !poster && texts.is_empty(),
                        "state {:?} at {now} ms drew the card: {} rects, poster {poster}, \
                         {} texts",
                        m.state,
                        rects.len(),
                        texts.len()
                    );
                    // the mascot stays on stage through the blank gap — bar
                    // the one Mac tick between a call-on (fn168 state 0
                    // SetState(1), which draws nothing) and ENTER 0x8001's
                    // first frame on the next Run.
                    let calling_on = m.m_state == 1 && m.m_enter;
                    assert!(mascot || calling_on, "the mascot vanished during {:?}", m.state);
                }
                _ => {
                    if first_card.is_none() {
                        first_card = Some(now);
                        assert!(poster, "the poster is late: the card went up without it");
                        assert!(!rects.is_empty(), "the panel is late");
                    }
                }
            }
        });
        assert!(saw_title_state, "never entered the title phase");
        let up = first_card.expect("the card never went up");
        // +1000 ms @0x110A, taken on the 83 ms frame grid; the golden's two
        // measured gaps are 1033 and 1067 ms.
        assert!(
            (1000..=1120).contains(&up),
            "the card went up {up} ms in; the golden says ~1000-1070"
        );
    }

    /// §14.6 ANSWERED. The crawl is a left-to-right per-glyph type-on over a
    /// line that is ALREADY at its final justified position — `coming-soon.mp4`
    /// f205-f211 keeps Tippy's title line anchored at module x 189 while its
    /// right edge walks the glyph boundaries of "Tippy the Tapeworm". Rates
    /// are 36 title glyphs in 400 ms and ~46 bullet glyphs/s.
    #[test]
    fn the_crawl_types_on_without_moving_the_line() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = build(pack) else { return };
        m.cur_movie = 0; // Tippy: "#Jc" title, 18 + 18 chars
        m.state = State::Poster;
        m.card_at = 0;
        m.shown = vec![1, 2, 3]; // "Travel through the intestinal tracts ...", +2 more

        let first_line_x = |m: &ComingSoon| -> Option<i32> {
            let mut out = Vec::new();
            m.texts(&mut out);
            out.first().map(|t| t.x)
        };

        m.now = 0;
        assert_eq!(crawl_chars(&m), 0, "the crawl is already typed at t=0");

        // The centred first line must not shift as it fills in — that is the
        // whole point of the golden's fixed x 189.
        m.now = 1000;
        let settled = first_line_x(&m).expect("no title at all");
        for ms in [60u64, 120, 180, 200] {
            m.now = ms;
            assert_eq!(
                first_line_x(&m),
                Some(settled),
                "the #Jc title line moved while typing at {ms} ms"
            );
        }

        // 36 title glyphs at 90/s = 400 ms, and no bullet before that.
        m.now = 200;
        assert_eq!(crawl_chars(&m), 18, "title line 1 should be done at 200 ms");
        m.now = 399;
        assert!(crawl_chars(&m) < 36, "the title finished early");
        m.now = 400;
        assert_eq!(crawl_chars(&m), 36, "the title should be complete at 400 ms");

        // Bullets start where the title left off, at ~46 glyphs/s.
        let bullets_at = |m: &mut ComingSoon, ms: u64| {
            m.now = ms;
            crawl_chars(m) - 36
        };
        assert_eq!(bullets_at(&mut m, 400), 0, "a bullet beat the title");
        let one_sec = bullets_at(&mut m, 1_400);
        assert!(
            (40..=50).contains(&one_sec),
            "{one_sec} bullet glyphs in the first second; the golden says ~46"
        );
        let two_sec = bullets_at(&mut m, 2_400);
        assert!(
            (86..=96).contains(&two_sec),
            "{two_sec} bullet glyphs in two seconds; the golden says ~92"
        );
    }

    /// §5's pace roll picks the bullet SLOTS at movie start, it does not gate
    /// one reveal per dwell cycle. Four golden cards — Tippy {1,2,6}
    /// (`coming-soon.mp4` f614), the Aliens {1,3,8,11} (f1000), Chernobyl
    /// {1,6,11} (f1300) and the reel's Tippy {2,4,6} (t≈66 s) — are subsets,
    /// never runs, and the last of them does not even start at slot 1. At
    /// Mellow that is `RandomBelow(4) == 0` over 11 slots.
    #[test]
    fn the_pace_roll_picks_a_scattered_subset_of_the_bullets() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let Some(mut m) = build(pack) else { return };
        let mut ctx = Ctx {
            rng: RandomLong::new(0xBEEF),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (-1, -1),
            mouse_down: false,
        };

        // Mellow (the capture's panel): bucket 1 → 1/4 per slot.
        m.pace_bucket = 1;
        let mut total = 0usize;
        let mut led_with_one = 0usize;
        let runs = 400;
        for _ in 0..runs {
            m.start_movie(&mut ctx, 0);
            assert!(
                m.shown.windows(2).all(|w| w[0] < w[1]),
                "the slot list must stay in order: {:?}",
                m.shown
            );
            assert!(
                m.shown.iter().all(|&n| (1..=BULLET_SLOTS).contains(&n)),
                "slot out of range: {:?}",
                m.shown
            );
            if m.shown.first() == Some(&1) {
                led_with_one += 1;
            }
            total += m.shown.len();
        }
        let mean = total as f64 / runs as f64;
        assert!(
            (2.0..=3.6).contains(&mean),
            "{mean:.2} bullets per card at Mellow; 11 slots at 1/4 is 2.75 \
             (the golden's four cards ran 3, 4, 3, 3)"
        );
        // The reel's {2,4,6} card proves slot 1 is not privileged.
        assert!(
            led_with_one < runs,
            "every card led with slot 1; the reel's Tippy card starts at 2"
        );

        // "Fast" (bucket 4) is RandomBelow(1) == 0 — every slot, always.
        m.pace_bucket = 4;
        m.start_movie(&mut ctx, 0);
        assert_eq!(
            m.shown,
            (1..=BULLET_SLOTS).collect::<Vec<_>>(),
            "Fast must show every bullet"
        );

        // Pokey (bucket 0) is the thinnest card: 1/5.
        m.pace_bucket = 0;
        let mut total = 0usize;
        for _ in 0..runs {
            m.start_movie(&mut ctx, 0);
            total += m.shown.len();
        }
        let mean = total as f64 / runs as f64;
        assert!(
            (1.4..=3.0).contains(&mean),
            "{mean:.2} bullets per card at Pokey; 11 slots at 1/5 is 2.2"
        );
    }

    #[test]
    fn wrap_breaks_on_words_and_never_drops_text() {
        assert_eq!(wrap("a bb ccc", 5), vec!["a bb", "ccc"]);
        // A word longer than the column gets its own line rather than
        // being silently discarded (the crawl has "Probe'n'Poke™" and
        // friends).
        assert_eq!(wrap("hi antidisestablishment", 6), vec!["hi", "antidisestablishment"]);
        assert_eq!(wrap("", 10), vec![""]);
        // A synthetic long bullet, and every real one from the pack.
        let mut bullets = vec![
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor".to_string(),
        ];
        bullets.extend(packed_crawl().unwrap_or_default());
        for long in &bullets {
            let joined = wrap(long, 20).join(" ");
            assert_eq!(
                joined.split_whitespace().collect::<Vec<_>>(),
                long.split_whitespace().collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn missing_strings_leave_the_crawl_and_message_line_empty() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let mut meta = (*pack.meta).clone();
        meta.strings.clear();
        meta.menus.clear();
        let bare = Pack::from_meta(meta, pack.root());
        let Some(mut m) = make(bare) else { panic!("make") };
        let mut ctx = Ctx {
            rng: engine::RandomLong::new(3),
            rng15: engine::Random15::new(3),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let clock = m.clock();
        let mut texts = Vec::new();
        for i in 0..20_000u64 {
            ctx.now_ms = clock.now_ms(i + 1);
            m.tick(&mut ctx);
            ctx.sounds.clear();
            texts.clear();
            m.texts(&mut texts);
            // Only the port's own bullet accent tick ("/") may remain.
            let bad = texts.iter().find(|t| !t.text.is_empty() && t.text != "/");
            assert!(bad.is_none(), "tick {i}: {:?}", bad.map(|t| &t.text));
        }
    }

    /// A `#J` token seen after text has already landed on the line (mid-run
    /// switch, not present in the packed STR# 500 data but grammatically
    /// legal) must NOT retroactively rewrite that line — only a still-empty
    /// line is retargeted.
    #[test]
    fn parse_crawl_late_justification_does_not_rewrite_started_line() {
        let (_, lines) = parse_crawl("abc#Jrdef");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], (0, "abcdef".to_string()));
    }

    /// Live-test report: "the coming soon guy seems like he's doubled,
    /// almost like a double stamp". Investigation (2026-09-01) traced the
    /// look to the ripped art itself — every sampled compounds/10000/*.png
    /// (several different sequences) carries a baked second head, and the
    /// same defect is already present in the source rip's own
    /// sequences-10000/*.gif, predating this module. This test locks down
    /// the thing that IS this file's responsibility: `sprites()` must never
    /// push the mascot (art series `BASE_MASCOT`) more than once in a
    /// single frame. Drives every state-machine phase (WaitDwell → Title →
    /// Poster → Hold → NextLine → Exit → back to WaitDwell) plus the
    /// click-to-skip path, since a future change to the phase dispatch is
    /// the realistic way a second, accidental mascot push could sneak in.
    #[test]
    fn mascot_art_emitted_at_most_once_per_frame() {
        let Ok(pack) = Pack::load(std::path::Path::new("../assets/coming-soon")) else {
            eprintln!("pack ../assets/coming-soon missing — skipping");
            return;
        };
        let mascot_pngs: std::collections::HashSet<String> = pack
            .series(BASE_MASCOT)
            .iter()
            .flat_map(|s| s.frames.iter().map(|f| f.png.clone()))
            .collect();
        let Some(mut m) = build(pack) else { return };
        let tick = m.tick_ms();
        let mut now = 0u64;
        let mut seen = [false; 6]; // indexed by State as usize (WaitDwell..Exit)
        for i in 0..2000u64 {
            let mut ctx = Ctx {
                rng: RandomLong::new(0xC0FFEE ^ i),
                rng15: Random15::new(1),
                sounds: Vec::new(),
                caps_lock: false,
                now_ms: now,
                local_hms: (12, 0, 0),
                mouse: (320, 240),
                // hammer the click-to-skip path on a third of ticks — the
                // states most likely to grow a second draw are the ones
                // with extra branches (§5 states 2/3/4, f41E0).
                mouse_down: i % 3 == 0,
            };
            m.tick(&mut ctx);
            seen[m.state as usize] = true;
            let mut out = Vec::new();
            m.sprites(&mut out);
            let mascot_draws = out.iter().filter(|s| mascot_pngs.contains(&s.png)).count();
            assert!(
                mascot_draws <= 1,
                "tick {i} (state {:?}): mascot drawn {mascot_draws} times",
                m.state
            );
            now += tick;
        }
        // Sanity: the drive above actually exercised the whole machine, so
        // "always <= 1" isn't vacuously true from only ever sitting in one
        // state.
        for s in [
            State::WaitDwell,
            State::Title,
            State::Poster,
            State::Hold,
            State::NextLine,
        ] {
            assert!(seen[s as usize], "state {s:?} never reached in 2000 ticks");
        }
    }
}

