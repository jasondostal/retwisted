//! Shock Clocks — faithful transcription of the RE spec
//! (totally-twisted docs/behavior/shock-clocks.md; every constant is
//! disasm-cited there). Three clocks tell REAL time via `ctx.local_hms`:
//! Father Time (RLEP 1100), Rotting Head (RLEP 1200, factory default) and
//! the Shocked Monkey (RLEP 1000 chain).
//!
//! Faithful behavior implemented here:
//! - menu `mVal 1000` 1-based 1–4, default **1 = Father Time**. `mVal 1000`
//!   stores 2, but the After Dark shell folds the stored mark by one before
//!   the module sees it (`GetControlValue(0)` @0x4A66 → `module+0x31C`), so
//!   the factory default resolves to menu item 1. GOLDEN (2026-09-01 DEPTH=32
//!   Basilisk capture): `g_02` shows the freshly-opened Setup panel with the
//!   Type popup reading **"Father Time"**, `g_03` renders Father Time, and
//!   `g_10` shows the open menu with the check on "Shocked Monkey" while the
//!   monkey is on screen — i.e. the module's selection is the menu ITEM
//!   index, and stored `mVal 2` ⇒ item 1. This corrects spec §1.1/§12.7,
//!   which read the raw `mVal` as the selection (the "mark fold" bug class
//!   already seen in sibling modules).
//!   Value 4 = Random → one `Random15() % 3 + 1` roll (@0x4A78–0x4A88:
//!   the divide by 3 keeps the REMAINDER — spec §1.1 prints this as
//!   `/3 + 1`, errata)
//! - `sVal 1001` Drift Speed slider 0–100 default 50; band = raw/20 clamp ≤4;
//!   per-band interval table DATA129+0xB68 = {0,200,100,120,60} **ms**,
//!   amplitude class DATA129+0xB5E = **{0,1,1,2,2}** (§8 prints {1,1,2,2,…};
//!   band 0 is a zero). The band also sets Father Time's open-coat loop length
//!   and his walk length — see `FT_GESTURE_N` / `FT_WALK_STOP_NUMER`.
//! - real-time protocol (§4): fresh hour/minute compared against last-seen
//!   → +0xA6 (hour) / +0xAC (minute) event flags
//! - Father Time speech table (§5): snd 1100 @seq 30, 1103 @37, 1102 @62/69,
//!   1101 @94/103 gated by `Random15()%256 < 48` (18.75 %), shared snd 2000
//!   "Cuckoo" @119 when the state tick counter == 3
//! - **Both clocks display the real time** (§4 "every clock *displays real
//!   Mac time*"), which spec §11 got wrong when it concluded the monkey
//!   "tells time through its behavior choreography, not through rendered
//!   digits" and §2/§6 when they called the Rotting Head's dial "painted
//!   art". The golden capture disproves both — see `lcd_*` (monkey) and
//!   `hand_*` (Rotting Head) below.
//! - Rotting Head main FSM states 9/11/20 (§6): hour flag → cue sub-seq
//!   +0xB4 → 20 (cuckoo `n = hour%12`, 0 → 12, counted on +0xAA — ORIGINAL
//!   QUIRK §12.2: the head cuckoos 1–12 times); minute flag → 11; snd 1200
//!   "Cuckoo prelude" at state-counter 0 (states 11 AND 20), shared snd 2000
//!   at state-counter 4 (fn_22E8 @0x2352–0x23EC)
//! - **The Rotting Head is a 25-part composite**, decoded from CODE 130's
//!   scene composer `fn @0x01A6` — see `RH_HEAD` / `RH_GUTS` / `RH_DRIPS` /
//!   `ear_next` / `tooth_next` below. The composer's own creation order is
//!   the z-order: two `CSpriteEar` actors (0xAA each, @0x022E/@0x0282), the
//!   head (compound **3**, @0x0312), the neck guts (compound **192**,
//!   @0x0388), `CSpriteMouth` (0xB8 @0x0400), the tooth/flasher parts
//!   (0x4E/0xAE/0xA6×2), **18 drip parts** (0xA8 each, @0x058C–@0x0E8A) and
//!   finally the dial actor (0x88 @0x0F00) on compound 411's anchor. Every
//!   one of those pushes is a literal in the listing; the spec never took
//!   this function apart, so §2/§6's catalogue of the 1200 chain is wrong in
//!   several places (see the errata notes on each constant).
//! - Ear gag: a four-state RING per ear — 43→45→75→77→43 and
//!   87→89→119→121→87 (`CSpriteEar::ChooseNextSequence` @0x0AE4, dispatcher
//!   @0x0AF6–0x0BE6) — with `Random15() & 0x7FF < 8` (0.39 %) rolled only in
//!   the head-worn state; shared snd 30011 "Burst" when the counter hits 21
//!   inside the 29-frame droop. Spec §5 calls these "state pairs toggled by
//!   the gate", which loses the fall-and-return; GOLDEN `g_14` catches the
//!   left ear off the head and lying on its tendon.
//! - CSpriteMouth (@0x1FC6) idles on compound 136 and plays 131 on a
//!   `Random15() & 0x1FF < 16` roll (3.125 %); CSpriteTooth (@0x27A0) runs
//!   two more four-state rings, 138→140→148→160 and 165→167→175→187, gated
//!   the same 8/2048 way at the resting state.
//! - Monkey FSM (§7): idle {146,154,162} — hour flag → `rand%2` → 225/205;
//!   shock window pending → `rand%3` stay idle (table @0x4D8); otherwise
//!   `rand%20` → lazy table @0x4DE = **{146,154,162}×6 + {225,205}**
//!   (ORIGINAL QUIRK §12.3 — kept verbatim, not replaced by rand%3;
//!   CORRECTED 2026-09-13, this line used to end `{146,154}` and that
//!   reading makes the startle unreachable — see `MONKEY_IDLE20`);
//!   startle 225/205 → `rand%2` → 261/245 → snd 1004/1005 (`rand&1`) → 277;
//!   277 → `rand%3` {277,277,282} with a 50 % repeat shock sound; 282 →
//!   cuckoo block (hour%12, 0→12) → `rand%3` back to idle
//! - §12.1's "50-second shock metronome" is FALSIFIED, and 2026-09-12's
//!   re-read of the binary says why twice over: the `rand%20` idle table at
//!   `DATA129+0x4DE` ends in **225, 205** (the startle poses), so the shock is
//!   a flat 1-in-10 roll per idle gesture (`MONKEY_IDLE20`); and `+0xB0` is a
//!   3.0-second SUPPRESSION window, not a countdown, because `fn_4724`
//!   returns TickCount × 16.625 ≈ milliseconds, not ticks
//!   (`MK_SHOCK_WINDOW_MS`). `emu/captures/shock-monkey.mp4` — four complete
//!   gags in 90 s, 4.4 / 7.5 / 16.3 / 26.4 s apart — is what forced the
//!   re-read.
//! - chatter chirps: uniform `Random15()%4` → snd 1000–1003 at the
//!   (seq, counter) pairs of the `@0x192A` dispatcher — **154:[2],
//!   162:[5,8,18,24,30,33]**, 225:[5,9,11], 205:[5,11], 261:[4,8,10,14],
//!   245:[4,10], and 146 never chirps. CORRECTED 2026-09-13: §7 (and the
//!   port) had every row shifted one gesture up, which parked the six-counter
//!   row on the 7-frame seq 154 where counters 8–33 are unreachable — see
//!   `chatter_counters`
//! - the monkey's machine is dressed with five electric arcs and three spark
//!   runs (`MK_ARCS`) plus the hang bar (`MK_BAR`) — spec §2 filed the arc
//!   compounds under "knobs/gauge pieces (art 80–88)" and "tiny wire ends",
//!   errata; they are blue-white discharges, and the golden has one lit in
//!   two of its four monkey frames. **Each of the five arcs strikes with
//!   shared snd 30002 "zap_spark"** — the composer registers it per part the
//!   way the Rotting Head's drips register 30005, and the three long spark
//!   runs are registered silent. NEW 2026-09-13, see `SND_ZAP_SPARK`
//! - out-of-table FSM states log-and-continue like the original's
//!   print-and-continue assert stub (§12.4)
//!
//! ## APPROXIMATIONS
//! - Every part is drawn at its packed compound bounds (bx/by) plus the
//!   frame's own link offset (dx/dy) on the shared field, rather than through
//!   per-part canvases. The dx/dy are the frame's OWN offsets, not deltas to
//!   accumulate — see the note on `Part`. (Father Time and the monkey's
//!   pose are the exceptions: both carry the Library's frame-centre `pos`,
//!   see their `set_run`s.)
//! - §5 attributes CSpriteEar (states 43/45/75/77 …) to Father Time, but
//!   those compound sequences only exist in series 1200 (RLEP 1100 has no
//!   seqs 43/45/75/77/87/89/119/121) and the CODE 130 RH composer is what
//!   `new`s the two 0xAA ear actors, so the ears belong to the Rotting Head,
//!   where the art lives.
//! - gCSpriteFlasher's sequences were never resolved (§5 UNCERTAIN) —
//!   omitted. The two candidates §5/§11 floated turn out to be two DIFFERENT
//!   mechanisms, both now pinned off `emu/captures/shock-monkey.mp4`
//!   (2026-09-12): the monkey's colon blinks on its own 1.02 s / 50 % beat
//!   that runs straight through the shock gag, while the LIT LCD segments are
//!   recoloured by the shock FSM itself — a linear ramp toward black from the
//!   startle, then pure white for 3.1 s from the leaving-282 edge. See `Lcd`.
//!   SUPERSEDED 2026-09-19/29: `gCSpriteFlasher` is FATHER TIME's class (its
//!   ChooseNextSequence is `fn_1250`), not the LCD; the recolour is IMPLEMENTED
//!   from `fn_2C3A`/`fn_2C9A`/`fn_2D70`/`fn_2DB6` (`Lcd`) and re-measured
//!   against `shock-monkey-fullscreen.mp4` — see
//!   `monkey_lcd_matches_the_fullscreen_golden`. Nothing is omitted.
//! - The drip and machine-arc arm gates are invented (`DRIP_GATE`,
//!   `ARC_GATE`); rates are pinned to what the golden frames show.
//!   SUPERSEDED 2026-09-29 for the machine: the arcs and sparks are one
//!   class whose DoDrawFrame `fn_6F8A` is now transcribed (`DRESS_GATE`:
//!   8/1024 per 100 ms frame, sparks loop). The drips are `new`ed by the
//!   CODE 130 composer as the SAME class (`M129_fn167`/`fn169`, `+0xA0 = 0`,
//!   snd `0x7535`, priority 0), so `DRIP_GATE` is resolvable the same way —
//!   left alone here because no Rotting Head golden exists to pin it. The monkey
//!   machine's ambient blue arcs ARE the port's reading: `shock-monkey.mp4`
//!   has them lighting in 3-frame bursts all through the capture, independent
//!   of the gag. The monkey's own "recolour" during the shock needs no code —
//!   `RLEP 1000` seq **277** is packed as the blue electric SKELETON with the
//!   arcs baked in, and the port already draws it (capture: 277 is the only
//!   pose that template-matches at NCC 0.95, at f113–121 / 324–327 / 630–643 /
//!   757–841).
//! - **Drift (§8) is a bouncing GLIDE, and it is now implemented** — see
//!   `step_drift`, `DRIFT_INTERVAL_MS`, `assembly`. The 2026-09-12 lane left
//!   this measured-but-unchanged; the disasm settles all of it. `@0x0118` is
//!   the module's drift callback: one `fn_0610` random placement on the first
//!   fire, then a constant ±amp² step on BOTH axes every band interval,
//!   reflecting off the screen rect inset by half the assembly
//!   (`@0x0254–0x0364`). Band 0 ("Still") cancels that cue and instead
//!   re-rolls the position once every 180 s (`@0x2ACC`). Three further
//!   findings: the interval table is in **milliseconds** (band 2 = 1 px per
//!   100 ms, exactly what `shock-monkey.mp4` measures over all 900 of its
//!   frames); the amplitude table starts with a **zero**; and the drift is
//!   **per-control** — 120 s of band-2 Father Time in `shock-hour.mp4` do not
//!   translate a pixel while a band-2 monkey glides 10 px/s, so only the
//!   monkey's glue registers the `module+0x52` assembly rect the drift
//!   carries. The port's old "teleport to a fresh random offset every band
//!   interval" turns out to be band 0's behaviour generalised to every band.
//! - Father Time's main FSM and his walk step (`fn_0CE8`; reposition + 0x32=50
//!   added to the +0xA6 accumulator, @0x0C92) are not in the spec, so both are
//!   taken off `emu/captures/shock-clocks.mp4` instead: he runs the 19
//!   series-1100 sequences IN ORDER (`FT_POOL`) and the walk locomotes +76 px
//!   per ten-frame cycle (`FT_WALK_*`). Speech gating "counter flag == 0" is
//!   rendered as fire-once-on-entry (counter == 0); seq 119 fires at
//!   counter == 3.
//!
//!   `emu/captures/shock-hour.mp4` (2026-09-12, 120 s, six complete chain
//!   passes and a 10:00:00 hour rollover) closes the three questions that pass
//!   left open:
//!   - **the chain wrap** — 135 hands back to the walk, and the walk keeps its
//!     x: the capture's ten walks start at ax = −176, 140, 152, −176, 140,
//!     −176, 65, 152, −176, 65, stepping +76 per cycle, and only reset to the
//!     entry offset after he has walked off the right edge. The port used to
//!     teleport him back to −176 on every pass (`FT_WALK_WRAP_AX`).
//!   - **the minute flag** — no visible branch. The 09:59:00 rollover lands at
//!     t = 30.0 s while he is mid-walk and the walk runs straight through it.
//!     Consuming the flag with no effect is CONFIRMED, not assumed — and the
//!     2026-09-12 disasm pass goes one better: **Father Time has no minute
//!     flag at all.** §5's `@0x0FD4–0x0FFC` is the HOUR latch (`FatherTime::
//!     on_minute`), and `fn_1250` tests nothing but `+0xA6`.
//!   - **seq 119 is the hour gag, not a chain gesture** — ten cuckoos at
//!     10:00, zero in the other 102 s. See `FT_CUCKOO_SEQ`.
//!
//!   BOTH of that pass's remaining approximations — the walk's cycle COUNT and
//!   the open-coat phase length — are RESOLVED (2026-09-12) by disassembling
//!   `CSpriteFather::ChooseNextSequence` = **`fn_1250` @`0x1250`**, which the
//!   spec never took apart. It is a branching state machine, not the ring the
//!   port walked with an index (`ft_next`). The walk length is a geometric
//!   roll at `Random15() % 256 < 96 − 16·band` per cycle
//!   (`FT_WALK_STOP_NUMER`), and the open-coat stretch is a counted loop over
//!   a SIX-entry gesture table run `DATA129+0x318[band]` = {13,9,6,4,3} times
//!   and exited through a two-entry table that ping-pongs (`FT_GESTURE_N`).
//!   That reproduces the capture's 57/80/72/46/78/169-frame spread and its
//!   12.5 s hour→cuckoo delay without a timer anywhere.
//! - §4's rolling {h,m,s,day,month,year,dow} copy (with the leap-day
//!   decrement quirk) only feeds actor compares against freshly-read time;
//!   per §11's own note the displayed behavior is identical when actors read
//!   real time directly, so the accumulator and the leap-day bug are not
//!   reproduced (nothing displays the date).
//! - **Nothing in this module is measured in 60 Hz ticks.** The helper that
//!   converted them is gone (2026-09-12): the CueAnim compound-frame delays
//!   are ms, §8's drift table is ms (`DRIFT_INTERVAL_MS`), and the monkey's
//!   `+0xB0` deadline is ms because `Resource.fn_4724` hands back
//!   TickCount × 16.625 (`MK_SHOCK_WINDOW_MS`). The CueAnim
//!   compound-frame delays are that same unit
//!   — milliseconds, see TICK_MS/FT_FRAME_MS and the golden's measured
//!   3-capture-frames-per-compound-frame cadence. **Read off the listings
//!   2026-09-19: the CueAnim delay is 100 for gCSpriteFlasher (`@0x0E80`),
//!   CSpriteEar (`@0x0956`), CSpriteMouth (`@0x21D6`) and CSpriteRightEye,
//!   and 120 for CSpriteMonkey (`@0x1730`) and its window re-cue
//!   (`@0x18C2`)** — §9's "120 (FT helpers)" had the classes swapped.
//!   The descriptor-swap helpers fn_2C3E/fn_2D74/fn_2DBA ARE implemented now
//!   (see `Lcd`); they are the monkey's LCD recolour.
//! - §1 found no control-change restart path (UNCERTAIN): controls take
//!   effect live here (menu change rebuilds the actor, like a module recycle).
//! - The RH +0xB4/+0xB0 pending sub-seq ids are not traced to specific
//!   compounds. `fn_2412`'s three states 9/11/20 ARE compound-sequence ids
//!   for the eye part (spec §6 gets this right by accident), so hour events
//!   cue seq 20 — the four eye-open stages followed by compounds 24–37, the
//!   red eyeball shooting out of the socket on its bloody stalk and back —
//!   and minute events cue seq 11, the eight-frame blink. Spec §2 calls
//!   11–23 "mouth-opening stages … the cuckoo door" and 24–41 "the red
//!   eyeball sliding out": the first half is errata, they are the RIGHT EYE
//!   (hence the class name CSpriteRightEye and its assert on this
//!   dispatcher), and 24–41 is the tail of that same run. The real mouth is
//!   the 131–190 family.
//! - The eye's idle state and run is **9**, not 11 — the ctor writes
//!   `+0xA2 = 9` and Init pushes `+0xA2` into SetRun (`RH_EYE_REST`). The
//!   2026-09-12 note below this line picked 11 on a template match; 11 is the
//!   first frame of the eight-frame blink the machine's state 11 plays, so a
//!   golden frame caught mid-blink matches it.
//! - "List full!" (§12.5) cannot occur: the engine sound queue is per-tick.
//!
//!
//! ## 2026-09-19 — ported from the decompile (`docs/decompiled/shock-clocks/`)
//!
//! The classes, from the assert strings at `DATA129+0x188/0x331/0x510/0x6A3/
//! 0x835/0x9CB`, with their vtables decoded out of `A5_globals.bin`:
//!
//! | class | clock | ctor | main vtbl | state vtbl | DoDrawFrame (+0x134) | ChooseNextSequence (**+0x138**) |
//! |---|---|---|---|---|---|---|
//! | gCSpriteFlasher | Father Time | `fn_0DF4` @0x0DF0 | `g036E` | `g04BA` | `fn_0FB2` | **`fn_1250`** |
//! | CSpriteMonkey | Shocked Monkey | `fn_16BA` @0x16B6 | `g054C` | `g0698` | `fn_185E` | **`fn_1A7A`** |
//! | CSpriteRightEye | Rotting Head | `fn_2156` @0x2152 | `g0874` | `g09C0` | `fn_22E8` | **`fn_240E`** |
//! | CSpriteEar ×2 | Rotting Head | `fn_08FE` @0x08FA | `g01C0` | `g030C` | `fn_0A74` | **`fn_0AE4`** |
//! | CSpriteMouth | Rotting Head | `fn_1E4A` @0x1E4A | `g06DE` | `g082A` | (base) | **`fn_1FC6`** |
//! | CSpriteTooth ×2 | Rotting Head | `fn_2622` @0x261E | `g0A06` | `g0B52` | (base) | **`fn_2790`** |
//!
//! Corrections the decompile forced, beyond the per-item notes below:
//!
//! - **ChooseNextSequence is vtable slot +0x138, not +0x148.** +0x148 is the
//!   base `fn_572E`, StartStateSequence: `SetRun(+0xA2)`, `+0xA4 = 0`, then
//!   vtbl+0x144 (reposition). The base tick `fn_5648` (@`0x5644`) calls +0x148
//!   when the armed flag `+0xA0` is clear, +0x138 when the run-end flag
//!   `+0x46` is set, then vtbl+0x84 to advance and `+0xA4 += 1`.
//! - **Which hand-off model (§7.1).** The base vtable `g0F8A` binds
//!   `+0x7C = L132 fn_0204` and `+0x108 = fn_028A` — frankenscreen's and
//!   mime-hunt's LINKED SetRun. But it does not matter here: every
//!   StartStateSequence tail-calls vtbl+0x144, and the base `fn_56DA`
//!   (@`0x56DA`) re-pins the part through the SCENE's vtbl+0x94 (the monkey
//!   scene's `fn_397A` resolves the anchor against compound `0x11D` = 285),
//!   so no travel survives a hand-off. Two classes override +0x144 and
//!   therefore DO travel: gCSpriteFlasher (`fn_1594`, repositions only while
//!   stopped — hence the walk carrying its x) and CSpriteEar (`fn_0CE8`,
//!   centre + the slide accumulator). `run_hand_offs_are_continuous` pins it.
//!   SUPERSEDED IN PART 2026-09-29 (linked-SetRun audit): the conclusion
//!   holds for POSITION but the reason given was loose on two counts. (a)
//!   The re-pin is per RUN, not one anchor: `fn_56DA` pushes `+0xA2`, the
//!   run id just set (@56E6), into the scene's `fn_397A`, so each run starts
//!   at its own authored spot in the compound-285 composite (not "the"
//!   anchor). (b) "Does not matter" was not true of the whole hand-off —
//!   `fn3F2E`'s MIRROR toggle is not undone by the re-pin. It only happens
//!   not to fire: in series 1000 the first part shared across every
//!   reachable hand-off carries flip bit 0 on both sides. The monkey now
//!   transcribes the whole sequence (`ShockedMonkey::set_run`: register,
//!   re-pin; `pos` = frame centre with the `fn3DDC` link in-run; `flip`
//!   drawn), and `monkey_hand_offs_re_pin_and_never_mirror` pins it to
//!   `shock-monkey-fullscreen.mp4` (every clean pose match at its authored
//!   spot, none mirrored; Father Time's keep-the-move model would put 277
//!   at (−15, −11)). No visible change for this pack.
//! - **`gCSpriteFlasher` is Father Time, not the monkey's LCD.** The
//!   2026-09-12 header guessed the LCD recolour was "the unresolved
//!   gCSpriteFlasher"; the assert string at `DATA129+0x331` names `fn_1250`,
//!   which is §3.2's open-coat machine. The recolour is `fn_2C3A`/`fn_2C9A`/
//!   `fn_2D70`/`fn_2DBA` on the sequence object (see `Lcd`).
//! - **The clock type is built by one of three builders** hanging off
//!   `module+0x324`'s vtable slot +0x04: `fn_3204` (Father Time), `M130
//!   fn_01A2` (Rotting Head), `fn_3C48` (Shocked Monkey). `TClockMod::SetUp`
//!   is `fn_4998`.
//!
//! ## GAPs (open, not invented)
//!
//! - **chirp rate — SETTLED 2026-09-29, and the "ruled out" below was wrong.**
//!   The long silences ARE `fn_59A0`: its start pass compares each queued
//!   cue's PRIORITY word against the loudest clip still sounding and discards
//!   the loser (see `SoundPlayer`). Chirps are priority 0, the arcs' 0.98 s
//!   zap_spark 1, Shock1/2 2 — so every zap silences the chirp bed for a
//!   second. The capture proves it directly: 0 of 237 chirps start inside
//!   one of the 87 zap clips, against 68 if independent. The 2026-09-19
//!   replay modelled the one-start-per-pass throttle but not the priority
//!   word, which is why it only moved the rate a quarter of the way.
//!   Numbers (12 odd seeds × 297 s, golden = `snd-correlate.py` r ≥ 0.9 /
//!   ≥ 0.55):
//!
//!   | | golden | port before | port after |
//!   |---|---|---|---|
//!   | chirps / s | 0.80 / 0.84 | 1.03 | **0.74** |
//!   | gap median / mean / max (s) | 0.65 / 1.18 / 7.66 | 0.72 / 0.97 / 5.9 | 0.75 / 1.35 / 8.3 |
//!   | gaps > 3 s | 26 | 6.1 | **22.7** |
//!   | chirps starting under a zap | **0** | 87 | **0** |
//!
//!   Pinned by `monkey_chirps_are_discarded_under_a_louder_cue`. RESIDUAL:
//!   the port now sits ~8 % under the capture; start passes land every
//!   120 ms on the port's 40 ms Idle (GAP(Idle cadence) at `SoundPlayer`) —
//!   at a 20 ms Idle the port reads 0.77 / s — so part of it is that.
//!   SUPERSEDED text, kept for the record: "`fn_185E`'s dispatcher, its rows
//!   and the `rand%20` table stay byte-identical. **The `fn_59A0` busy model
//!   is ruled out as the explanation** … moves the chirps 318 -> 300 (1.25x
//!   -> 1.18x) … Still left out rather than shipped as a guess."
//! - **shock-gag rate — AUTHENTIC, no change (2026-09-29).** The C queues one
//!   shock at the zap (`@0x1BF6`) and one more per `{277, 277, 282}` repeat
//!   (`@0x1C84`), i.e. 1 + Geom(1/3) = **3.0** per gag on average (sd 2.4).
//!   The capture's 20 in 8 gags (2.5) is 0.6 standard errors from that; the
//!   old "port 11 in 6 (1.83), under-firing" was a six-gag sample — over 12
//!   seeds × 297 s the port queues 2.96 per gag (115 gags) and, after the
//!   priority gate, starts 2.75 (122 gags). `snd 30002 zap_spark`'s "118 vs
//!   87 (1.36x)" was a METRIC mismatch, not a rate: the correlator merges
//!   peaks inside half a clip (0.49 s), and counting the port's zaps the same
//!   way gives 86 before and 89 after the `fn_6F8A` transcription
//!   (`DRESS_GATE`), against the capture's 87–89.
//! - **the ear slide extent** (`EAR_SLIDE_LIMIT`) — the original compares
//!   against the scene rect's width through the scene's vtbl+0x98; the port
//!   has no per-assembly rect for the Rotting Head.
//! - **the eye's `+0xB4`/`+0xB0` children** — `fn_2156` zeroes two LONGS and
//!   `fn_240E` pokes them through the scene's vtbl+0x58. They are child
//!   objects the M130 composer installs, not sub-sequence ids; nothing is
//!   wired to them here, and nothing in the capture shows what they are.
//! - **`fn_4998`'s control indices** — `L130 fn_3B6E` is called three times
//!   with a dropped short (the control index) and the port keeps the
//!   2026-09-12 reading (0 = Type, 1 = Drift Speed).
//! - **the drip gate** (`DRIP_GATE`) remains the port's invention; the arc
//!   gate is transcribed (`DRESS_GATE`), and the drips are the same class.
//! - **the drift is still the port's own stepper**; `fn_0118` was read in
//!   2026-09-12 and the capture re-measured on 2026-09-19 (below).
//!
//! ## CAPTURE MEASUREMENTS (2026-09-19, `emu/captures/qemu/shock-clocks.mp4`)
//!
//! 1200 frames at 10 fps, phase-correlated frame to frame over a ±3 px
//! search on the 520×400 interior: the whole scene translates by exactly
//! (−1,+1) or (+1,−1) per 100 ms — 558 of the first 599 steps, the rest
//! (±2,±2) or (0,0) from capture-timing jitter — and never more than 2 px on
//! either axis. The assembly bbox's left edge spans 0..245 and reverses at
//! both ends and nowhere else; 640 − 395 = 245. That is band 2's (100 ms,
//! amp 1) and it is what `drift_glides_diagonally_and_bounces` pins. With the
//! drift removed there is no residual sprite motion in the capture at all,
//! which is `run_hand_offs_are_continuous`'s invariant.
//!
//! ## ERRATA — tick-quantization sweep, 2026-09-12 (no code change)
//!
//! - **The 100 ms clock-animation period is CONFIRMED; leave `TICK_MS` at
//!   40.** `emu/captures/shock-hour.mp4` over t = 25-50 s (164 screen-change
//!   events, crop 56 px + halve to module px) gives a Rayleigh peak at
//!   **P = 101.42 ms with R = 0.684** — the CueAnim delay-100 period for
//!   Father Time / Rotting Head, flat, with nothing at the 116.375 ms the
//!   `f4724` grid would predict (R = 0.052). The `anim_acc` accumulator
//!   already averages exactly 100 ms whatever `TICK_MS` is, so the sweep
//!   changed nothing here.
//! - Still open: whether the 120 delay (monkey zap, `@0x18C2`) is likewise
//!   flat or quantizes to 8 ticks = 133 ms. `shock-monkey.mp4` never holds
//!   the zap long enough for a periodogram; a capture that sits on a single
//!   monkey zap-recover cycle for 20 s would settle it.
//!
//! ## GOLDEN AUDIO — `emu/captures/qemu/`, 2026-09-13 and 2026-09-19
//!
//! First bit-exact guest audio for this module was `shock-clocks.wav`
//! (2026-09-13): 119.12 s against a 120 s wall clock (wav/wall = 0.993, so
//! `t × 20` IS the video frame), panel **Type = Shocked Monkey, Drift Speed =
//! Slow** — but in Demo mode, with the Setup panel still on screen. Three
//! takes on 2026-09-19 close it out and correct two of its findings:
//! **`shock-monkey-fullscreen`** (300 s, no panel, `wavmap`-corrected) and
//! **`shock-father-time{,-take1,-hour}`** (390 s of Father Time, the `-hour`
//! take armed to span guest 18:00:00). All correlated with
//! `scripts/emu/snd-correlate.py` against the `expanded/` rips; the `.wavmap`
//! matters for burst-to-burst spacing, not for gaps inside a burst. Full
//! tables in `docs/emulator/audio-captures.md`. Everything below is the
//! 2026-09-13 capture unless it says otherwise.
//!
//! - **`snd 30002 zap_spark` is a Shock Clocks cue and it was missing.**
//!   40 firings, r = 0.62–0.81, and the disassembly says exactly which part
//!   plays it: the five chassis arcs. IMPLEMENTED — see `SND_ZAP_SPARK` and
//!   `ARC_GATE`, which the 40 firings also re-rate (64/2048 → 17/2048).
//!   (SUPERSEDED 2026-09-29: the gate is `fn_6F8A`'s 8/1024 on a 100 ms
//!   frame — `DRESS_GATE`.)
//! - **The chatter table was shifted one gesture** — FIXED, see
//!   `chatter_counters`. The capture's six-chirp bursts are the counter
//!   deltas 3/10/6/6/3 × 120 ms and only fit the 42-frame seq 162.
//! - **`snd 1005 Shock2` DOES fire** — the line above used to read "did NOT
//!   fire" off this 119 s capture, and the 300 s full-screen take
//!   (`shock-monkey-fullscreen`, 2026-09-19) corrects it. Over 300 s there
//!   are 20 shock cues and **11 of them are Shock2**, with the loser sitting
//!   at exactly the 0.65-0.71 Shock1-vs-Shock2 cross-match this note was
//!   right about (263.94 s: 1005 r = 0.941 vs 1004 r = 0.709; 277.24 s:
//!   0.943 vs 0.708). Nine are Shock1. A 9/11 split over 20 draws is the
//!   `rand & 1` at `@0x1BFA-0x1C22` / `@0x1C84-0x1CB8`, which the port
//!   already plays — no code change, the record was wrong.
//! - **Father Time's sounds now have a golden** — SETTLED 2026-09-19 by
//!   `shock-father-time{,-take1,-hour}`, 390 s of Demo run at Type = Father
//!   Time / Drift Speed = Slow, 78 cue firings. He plays **exactly four voice
//!   lines plus the hour cuckoo and nothing else**: 1100 Hey You, 1101 Hmm,
//!   1102 Laugh2, 1103 Wanna know and shared 2000 Cuckoo. The chirp bed,
//!   Shock1/Shock2 and `30002 zap_spark` are absent (best r = 0.05-0.24) —
//!   they are the Shocked Monkey's, which the mirror-image monkey take
//!   confirms from the other side.
//!   - **`snd 1200 Cuckoo prelude` is NOT Father Time's** (best r = 0.184
//!     across all three takes, the rollover included). It is the Rotting
//!     Head's, states 11 and 20, exactly as `fn_22E8` @`0x2352-0x23EC` has
//!     it and as the port already plays it. 1200 still has no audio evidence
//!     of its own and needs a Rotting Head rollover capture.
//!     RE-CHECKED 2026-09-29 against the listing: the trigger matches (state
//!     11 counter 0 and state 20 counter 0, 2000 at state 20 counter 4, all
//!     priority 2), and it is NOT hour-only — state 11 is the MINUTE blink,
//!     so every minute rollover plays one 1200. The capture that settles it:
//!     `shock-rotting-head-hour` — Type = Rotting Head, Drift Speed Slow,
//!     full screen via the `cmd-ctl-s` sleep hot key, audio on (`-av.mp4` +
//!     `.wav` + `.wavmap`), ~150 s armed at HH:58:30 guest time so it spans
//!     one plain minute rollover and an hour whose `hour % 12` is small
//!     (18:00 → six rounds). Predicted by
//!     `rotting_head_cuckoo_prelude_follows_fn_22e8`: 1200 at HH:59:00; six
//!     1200 → 2000 pairs from HH+1:00:00, 2000 ~0.4 s after each prelude,
//!     rounds ~2.3 s apart (seq 20 is 22 frames at 100 ms + the state-9
//!     frame); then one more 1200 for the deferred minute blink.
//!   - **the hour cuckoo is settled**: the `-hour` take spans guest 18:00:00
//!     and fires 2000 **six** times, `hour % 12` = 6, first at 18:00:07.0 and
//!     1.18 s apart (sd 0.11) against a 0.397 s clip — no overlap. That is
//!     `FT_CUCKOO_SEQ`'s 11-frame run at 100 ms a frame, and the 7.0 s onset
//!     delay is the 94 ⇄ 103 ping-pong, not a timer (see `FT_GESTURE_N`).
//!   - **a cue pre-empts, it does not mix**: 2 of the 78 firings start while
//!     the previous is still sounding, both 1102 -> 1102, and the first clip
//!     is CUT (hour take 142.52 s scores r = 0.650 against the full 748 ms
//!     Laugh2 but r = 0.966 against just its first 552 ms). One channel,
//!     last cue wins — which the engine already models. Do not add mixing.
//!   - **the cadence**: every run opens with 1100, then 1103 at 0.545-0.777 s
//!     (11 instances, mean 0.68) or straight to the laugh; 1102 fires 1-4
//!     times with self-gaps 0.55-2.16 s (min 552 ms, median 1.27, 16 of 18
//!     inside 0.75-2.16); 1101's 18.75 % roll closes some runs. Run to run is
//!     ~20.8 s. All of that is what `FT_WALK_EDGE_MARGIN` was corrected
//!     against and what the two `father_time_*` ratchets hold.
//! - RESIDUAL, still unfixed but re-sized: the port chirps **~1.25× too
//!   often**, not the 1.5× this note used to claim. The full-screen run this
//!   note asked for was taken on 2026-09-19 (`shock-monkey-fullscreen`,
//!   300 s, panel Esc'd via the `cmd-ctl-s` sleep hot key) and the panel
//!   really was eating chirps: see the chirp-rate GAP above for the new
//!   numbers, the ruled-out `fn_59A0` busy model, and the dispersion mismatch
//!   that is what actually remains. SUPERSEDED 2026-09-29: the dispersion was
//!   `fn_59A0`'s priority gate after all — see the chirp-rate entry.

use engine::l135::{self, FrameBox, LinkModel, Registration};
use engine::{ControlDef, ControlKind, Ctx, Image, Module, Pack, SpriteDraw};

// series bases (RLEP chains)
const BASE_MONKEY: u32 = 1000;
const BASE_FT: u32 = 1100;
const BASE_RH: u32 = 1200;

// snd resource ids (§10; packed as sounds/<id>.wav)
const SND_CHIRP1: u32 = 1000; // Chirp1..4 = 1000..1003
const SND_SHOCK1: u32 = 1004;
const SND_SHOCK2: u32 = 1005;
const SND_HEY_YOU: u32 = 1100;
const SND_HMM: u32 = 1101;
const SND_LAUGH2: u32 = 1102;
const SND_WANNA_KNOW: u32 = 1103;
const SND_CUCKOO_PRELUDE: u32 = 1200;
const SND_CUCKOO: u32 = 2000; // shared
const SND_BURST: u32 = 30011; // shared

// The 60 Hz TickCount → ms helper this file used to carry is GONE (2026-09-12).
// Nothing in the module is measured in 60 Hz ticks: the CueAnim delays are
// milliseconds (see below), §8's drift table is milliseconds
// (`DRIFT_INTERVAL_MS`), and the monkey's `+0xB0` deadline is milliseconds too
// because `Resource.fn_4724` scales TickCount by 16.625 before returning it
// (`MK_SHOCK_WINDOW_MS`). Every "ticks" in the spec's tables is a ms.

// ---------------------------------------------------------------------------
// Compound-frame delays (§3 "CueAnim", CODE 130 cue list `fn @0x18C8` + its
// pump `fn @0x156C`)
//
// Every actor class arms itself in its constructor with one CODE 130
// animation cue, `CueAnim(obj, 1, 3, delay, 2)`. Decoding the list record
// (24 bytes; the insert @`0x18C8` stores the args as `+0 obj`, `+4 word 1`,
// `+6 long 3` = mode, `+0xA word delay`, `+0xC long 2` = channel) and the
// pump's mode-3 arm (@`0x1666–0x16CA`) settles what `delay` does: the pump
// computes `(now − base) / delay` and `(prev − base) / delay` — both
// unsigned divides by the record's `+0x0A` — and fires the object's `vtbl+0x04`
// (@`0x17EE`) only when the two buckets differ. So mode 3 is "step this
// object once every `delay` units of the engine clock at `[A5+0x30]`".
//
// The three constructors, each loading the delay constant immediately
// before its `CueAnim` push block:
//
// | actor | ctor | constant @ | delay |
// |---|---|---|---|
// | Father Time (`+0xA2 = 1`, the walk) | `fn_0DF4` | @`0x0E80` | **100** |
// | Shocked Monkey (`+0xA2 = 0x92 = 146`) | `fn_16BA` | @`0x1730` | **120** |
// | Rotting Head / CSpriteRightEye | `fn_2156` | @`0x21D6` | **100** |
//
// ERRATA vs §3/§12.6, which lumps these as "observed delays 60 / 100 / 120,
// exact unit UNCERTAIN" and files `fn_16BA` under "FT helpers": `fn_16BA`
// initialises `+0xA2 = 146` and clears the `+0xB0` deadline **long**, so it
// is `CSpriteMonkey`'s constructor, not Father Time's; and `@0x0E80` holds
// 100, not the 120 the spec's table pins there. The remaining sites are the
// ear helper `fn_08FA` @`0x0956` (100), `gCSpriteFlasher` `fn_1250` @`0x1324`
// (100), the monkey's zap cue @`0x18C2` (120) and its recover cue @`0x1DA2`
// (60).
//
// APPROXIMATION — the UNIT. `[A5+0x30]` is not written anywhere in these two
// CODEs, so the disasm cannot pin it. Read as **milliseconds**, which is what
// the same Berkeley Library-4.0 sprite engine calls this quantity in the
// sibling module: Voyeur's `g0E50 = 60` is documented as the "default
// compound-frame delay 60 ms" (voyeur §8, `MF_7D6` @`0x53DE`) and Shock
// Clocks' delays are the same 60/100/120 family. The alternative reading —
// 60 Hz `TickCount` — would put Father Time's walk cycle at 0.6 fps, so it is
// not this field; and it is not the unit of the monkey's `+0xB0` deadline
// either, whatever §12.1 says — see `MK_SHOCK_WINDOW_MS`, `fn_4724` returns
// milliseconds too.
//
// The port had been advancing one compound frame per 25 Hz module tick,
// i.e. 2.5–3× the packed rate for all three clocks; USER REPORT
// (2026-09-01 live test): "shock clocks is WAY off — [Father Time is] way
// sped up / hyper speeded. monkey and head are a little too fast too."
const TICK_MS: u64 = 40;
const SCREEN_W: i32 = 640;
const SCREEN_H: i32 = 480;

// ---------------------------------------------------------------------------
// Drift (§8) — the per-band tables, re-read from `DATA129` on 2026-09-12.
//
// | band | `sUnt 1001` label | `+0xB68` interval | `+0xB5E` amplitude | step |
// |---|---|---|---|---|
// | 0 | Still | **0** | **0** | cue cancelled |
// | 1 | Very Slow | 200 | 1 | 1 px / 200 ms = 5 px/s |
// | 2 | Slow | 100 | 1 | 1 px / 100 ms = 10 px/s |
// | 3 | Moderate | 120 | 2 | 4 px / 120 ms = 33 px/s |
// | 4 | Fast | 60 | 2 | 4 px / 60 ms = 67 px/s |
//
// TWO ERRATA vs §8, both from reading the raw words rather than the spec's
// transcription:
//
// 1. The amplitude table is `{0, 1, 1, 2, 2}`, not `{1, 1, 2, 2, 2}` — band 0
//    is a zero, which is what makes "Still" still. (Words `0x0B5E..0x0B67`;
//    the spec printed the first four and an ellipsis.)
// 2. §8's "non-monotone `Moderate = 120 > Slow = 100` — deliberate pairing or
//    original typo, UNCERTAIN" is neither: it is monotone once the amplitude
//    is folded in, because the step is `amp²` (the direction word is
//    renormalised to ±amp at `@0x02A0` and then multiplied by amp again at
//    `@0x02DA`). The speed ladder is 0 / 5 / 10 / 33 / 67 px/s.
//
// The UNIT is milliseconds, the same reading as the CueAnim compound-frame
// delays above, and the capture nails it: `shock-monkey.mp4` glides the
// monkey assembly exactly **1 px per 100 ms**, which is band 2's row — and
// band 2 is the factory slider 50. The old 60 Hz-tick reading would have made
// that row 1 px per 1.67 s.
const DRIFT_INTERVAL_MS: [u64; 5] = [0, 200, 100, 120, 60];
const DRIFT_AMP: [i32; 5] = [0, 1, 1, 2, 2];
/// Band 0's own cadence: `0x0002BF20` = 180 000 of `fn_4724`'s milliseconds
/// (`@0x29E6` at setup, `@0x2ADA` on every re-arm). Three minutes.
const DRIFT_STILL_HOP_MS: u64 = 180_000;

const FT_FRAME_MS: u64 = 100;
const RH_FRAME_MS: u64 = 100;
const MK_FRAME_MS: u64 = 120;
/// The monkey's RECOVER cue: leaving 282, `fn_1A7A` re-arms its own CueAnim
/// record with delay **60** (loaded @`0x1DA2`, the same
/// `CueAnim(obj, 1, 3, delay, 2)` push block as the ctor's 120 @`0x1730`),
/// and `fn_185E`'s window-expiry arm puts it back to 120 (@`0x18C2`). The
/// CODE 130 insert (@`0x18C4`) UPDATES the record in place when obj and word
/// match, so for the 3.0 s `+0xB0` window the gorilla animates at double
/// speed.
///
/// GOLDEN (`shock-monkey-fullscreen.mp4`, 10 fps, per-frame pose match on
/// the 162 frames that no other run shares): inside the eight white-LCD
/// windows seq 162 advances **1.68** compound frames per 100 ms (56 steps),
/// outside them **0.85** (426 steps) — 60 ms and 118 ms a frame. The port ran
/// 120 ms straight through (0.83 in both). Pinned by
/// `monkey_recovers_at_double_speed_inside_the_white_window`.
///
/// GAP(CueAnim bucket phase): the pump fires on a change of
/// `(now − base) / delay` (CODE 130 @`0x1666`–`0x16CA`); after the delay
/// changes, the port just keeps its accumulator and re-reads the period, so
/// the first frame at the new rate can land up to one module tick early.
const MK_RECOVER_MS: u64 = 60;

// ---------------------------------------------------------------------------
// Code-drawn dial hands (§6 correction + §5 correction)
//
// Neither dial is static painted art: both clocks build a runtime hand actor
// out of a `0x88`-byte object holding three 16-byte {point count, length,
// half-width} records at `actor+0x0C`. No hand sprite exists anywhere in
// either RLEP chain because the hands are rasterised, which is why the
// earlier art searches never turned one up.
//
// **Rotting Head** (CODE 130 composer `fn @0x01A6`): `RLEP 1200` compound
// **411** is a 1×1 fully transparent frame parked at `bx=218, by=56` — a pure
// anchor marker, and exactly the pixel where the head art (`c_001.png`)
// paints the hub dot. The composer fetches its position (frame 411 pushed
// @`0x0EAC`, sprite vtbl+0x94 @`0x0EBE`) and builds a rect of **±36**
// around it (@`0x0ECC–0x0EFE`). Records @`0x0F90–0x10C2`: {3,18,3} {3,31,2}
// {3,34,1}. GOLDEN cross-check (`g_12`/`g_13`/`g_14`): a fat tapered hand
// ~19 px at 46–48°, a 2–3 px hand ~31 px at 223–225°, and a 1 px hand ~34 px
// that SWEEPS between frames (78° → 98° → 101°) — at the capture's 1:37:1x
// exactly `hour = (h%12)*30 + m*0.5 = 48.5°`, `minute = m*6 = 222°`,
// `second = s*6`.
//
// **Father Time** (CODE 129 composer `fn_3204` — previously unread; this is
// what USER REPORT 2026-09-01 saw as "a solid white circle for a clock").
// After loading `RLEP 1100` (@`0x326C`, `0x044C`) and installing the 0xBA
// figure actor (@`0x32E2`), the composer asks the sprite for the placement
// rect of **compound 62's art part 27** — 62 loaded @`0x3358` and
// 27 @`0x335C`, pushed ahead of the out-Rect into sprite
// vtbl+0x98 @`0x3384`. `OFtb 1100` compound 62 channel 8 is `art 27 @ (215,
// 166, 248, 198)`, and art 27 is `RLEP 1102 f_026`, the 33×32 **white dial
// face with tick marks and no hands** painted on his belly. The composer then
// normalises that rect to its own origin (@`0x3386–0x33A2`), takes the centre
// (@`0x33A6–0x33E6`: `(right+left)/2`, `(bottom+top)/2` → **(16, 16)**, i.e.
// compound-space **(231, 182)** — the measured centre of the painted disc is
// (231, 181.5), so this lands on the hub dot), and computes
// `D4 = (bottom − top)/2 − 3 = 13` (@`0x33FA–0x3408`). It news the same
// `0x88` actor (@`0x340A`) and fills the three records:
//
// | record | `+0x00` points | `+0x02` length | `+0x04` half-width |
// |---|---|---|---|
// | 0 hour | 1 (@`0x349A`) | `D4/2` = **6** (@`0x34FE`) | 2 (@`0x3572`) |
// | 1 minute | 1 (@`0x34BA`) | `D4−3` = **10** (@`0x3528`) | 1 (@`0x3596`) |
// | 2 second | 1 (@`0x34DC`) | `D4` = **13** (@`0x354E`) | 1 (@`0x35BA`) |
//
// Point count **1** here, not the head's 3: Father Time's hands are plain
// lines, the head's are tapered triangles.
const RH_HAND_HOUR: Hand = Hand { len: 18.0, halfw: 3.0, points: 3 };
const RH_HAND_MIN: Hand = Hand { len: 31.0, halfw: 2.0, points: 3 };
const RH_HAND_SEC: Hand = Hand { len: 34.0, halfw: 1.0, points: 3 };
const RH_DIAL: Dial = Dial {
    base: BASE_RH,
    hub: (218, 56),
    // half-extent of the composer's dial rect (@`0x0ECC`, the constant 36)
    rad: 36,
    hands: [RH_HAND_HOUR, RH_HAND_MIN, RH_HAND_SEC],
};
const FT_DIAL: Dial = Dial {
    base: BASE_FT,
    hub: (231, 182),
    // the normalised rect's own centre, (16, 16) of a 33×32 face
    rad: 16,
    hands: [
        Hand { len: 6.0, halfw: 2.0, points: 1 },
        Hand { len: 10.0, halfw: 1.0, points: 1 },
        Hand { len: 13.0, halfw: 1.0, points: 1 },
    ],
};

/// The `RLEP 1100` compounds that actually carry the dial face (art 27):
/// **52–117 and 129**. Everything below 52 is the coat CLOSED (no dial on
/// screen at all) and 119–128 swing the face open for the cuckoo — those draw
/// arts 28/29/30/31, the gearworks and the bird, and no white face. The
/// composer installs the hand actor unconditionally, so this range is the
/// port's stand-in for whatever hides it; without it the hands float on his
/// buttoned coat.
///
/// PACKED-ART PROOF (2026-09-12): art 27 is baked into the compound bitmaps,
/// so the range can be read straight off them. Counting near-white pixels per
/// `assets/shock-clocks/compounds/1100/c_NNN.png` gives **723–748** for every
/// compound in 52–117 and for 129 (the dial disc plus his white hair), **96–
/// 137** for 119–128 (hair only — the cuckoo covers the dial) and **62–73**
/// (hair only) for all of 1–51 and 131–139. The boundaries are exact.
const FT_DIAL_COMPOUNDS: std::ops::RangeInclusive<u32> = 52..=117;
const FT_DIAL_COMPOUND_CUCKOO_END: u32 = 129;

// ---------------------------------------------------------------------------
// Father Time walks (§5 `fn_0CE8`, measured off GOLDEN
// `emu/captures/shock-clocks.mp4` t≈7.7–16.4 s)
//
// That capture is 1280×1016 @30 fps: a 640×480 screen at 2× under a 56-row
// Basilisk title bar, so module px = capture px / 2 and module y =
// (capture y − 56) / 2. Every number below is a direct read of it.
//
// **Cadence.** The figure's bounding box changes every THREE capture frames
// (39 changes across capture frames 41–158, 117 frames ⇒ 3.08 frames each).
// 3/30 s = 100 ms — an independent confirmation of `FT_FRAME_MS` above, and
// of the millisecond reading of the CueAnim delay.
//
// **The walk cycle is `RLEP 1100` sequence 1, ten frames, and it LOCOMOTES.**
// Matching the capture's per-frame widths against the packed compounds'
// (69, 58, 50, 58, 72, 60, 50, 52, 66, 76) identifies the run exactly, and the
// right-hand edge pins the actor offset: c_010's art ends at compound x 263
// and lands at module x 87.5 / 163.5 / 239.5 / 315.5 on four successive
// cycles ⇒ **ax = −176, −100, −24, +52**. So the intra-cycle motion is baked
// into the compound `bx` (108 → 188) and the actor adds a flat **+76 px per
// completed cycle** — position is constant within a cycle and steps at the
// wrap. Four cycles, 40 frames, 4.00 s (capture t 7.77 → 11.77), 76 px/s =
// 2.53 px per 30 fps capture frame.
//
// The port had him standing still ("does not locomote", old APPROXIMATIONS
// note) at a centred `ax = 114`, which is why the headless render is one
// motionless sprite.
const FT_WALK_SEQ: u32 = 1;
/// First cycle's offset: c_001's art (compound x 108–176) ends exactly on the
/// left screen edge, i.e. he steps on from off-field.
const FT_WALK_ENTER_AX: i32 = -176;
/// The stride `fn3F2E` produces at every 10 → 1 hand-off (registration on
/// the head, art 37) — derived by `FatherTime::set_run`, named for the tests.
#[cfg_attr(not(test), allow(dead_code))]
const FT_WALK_STEP: i32 = 76;
/// How far a walk resumes from where the previous one stopped, in compound
/// origin x: the chain's registrations 10→13 (−24), 15/19→21 (+5) and
/// 139→1 (+107) — every other hand-off registers at 0. Derived, not tuned:
/// `FatherTime::set_run` produces it; this names it for the tests.
/// GOLDEN `shock-father-time.mp4`: the walk stopping at c_010 left 88
/// (t 56.4 s, ax −100) resumes at c_001 left **96** (t 64.0 s, ax −12).
#[cfg(test)]
const FT_WALK_RESUME_DX: i32 = -24 + 5 + 107;
/// Where the figure stands once the walk is done. GOLDEN-pinned on the thing
/// that matters — the dial. The white disc measures module x 253–283.5,
/// y 156–185.5 (capture frames 250–295); art 27's compound rect is
/// (215, 166, 248, 198) with a 1 px black rim, so `ax = 253 − 216 = 36`,
/// `ay = 156 − 167 = −11`, and the code-drawn hands land on the hub dot.
///
/// RESOLVED 2026-09-26: the capture's per-sequence spread (ax = 28 for seqs
/// 13/17, 33 for 21 and 30/37 after a walk ending at 52) is not a per-sequence
/// MoveTo — it is `fn028A`'s shared-part registration (−24 at 10→13, +5 at
/// →21), and the dial reading for 51 onward is the open coat drawn MIRRORED.
/// `FatherTime::pos` carries all of it; `FT_AX`/`FT_AY` is now only the
/// origin the entry offset and the tests are quoted against.
const FT_AX: i32 = 36;
const FT_AY: i32 = -11;

/// One dial-hand record: `{+0x02 length, +0x04 half-width, +0x00 point count}`.
/// A 3-point hand is a triangle (tapers to the tip); a 1-point hand is a line
/// of constant width.
struct Hand {
    len: f32,
    halfw: f32,
    points: u32,
}

/// One code-drawn dial: the series it belongs to, its hub in compound space,
/// the half-extent of the cached hand canvas, and the hour/minute/second
/// records.
struct Dial {
    base: u32,
    hub: (i32, i32),
    rad: i32,
    hands: [Hand; 3],
}

/// Generated-sprite name for one dial hand: `gen:hand/<series>/<kind><idx>`.
/// The name is the whole recipe — [`ShockClocks::generated`] gets nothing
/// else — so it carries the series (which picks the dial record) and the
/// discrete angle index.
fn hand_name(dial: &Dial, kind: char, idx: u32) -> String {
    format!("{}hand/{}/{kind}_{idx:03}", engine::GEN_PREFIX, dial.base)
}

/// Rasterise one hand (the engine blits images and has no line primitive, so
/// the original's vector hands become one cached image per discrete angle).
///
/// Until 2026-09-19 this wrote the result into
/// `<pack>/compounds/<base>/hands/<kind>_NNN.png` and the sprite named that
/// path; inside an installed `.saver` the bundle is read-only, the write
/// failed, and the clocks simply had no hands. Same pixels, no write: the
/// image is handed straight to `compose`, which caches it under the name
/// exactly as it caches a packed compound.
///
/// The shaft is rasterised by distance-to-axis so that the 1-px-wide second
/// hand stays continuous instead of dropping out sub-pixel.
fn hand_image(dial: &Dial, angle_deg: f32, hand: &Hand) -> Image {
    let n = (dial.rad * 2 + 1) as u32;
    let c = dial.rad as f32;
    let a = angle_deg.to_radians();
    // Clock convention: 0° = 12 o'clock, angles increase clockwise.
    let (dx, dy) = (a.sin(), -a.cos());
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    for py in 0..n {
        for px in 0..n {
            let (vx, vy) = (px as f32 - c, py as f32 - c);
            // projection along the hand axis, clamped to the shaft
            let t = (vx * dx + vy * dy).clamp(0.0, hand.len);
            let (ex, ey) = (vx - t * dx, vy - t * dy);
            let dist = (ex * ex + ey * ey).sqrt();
            // a 3-point hand tapers from `halfw` at the hub to a point at the
            // tip; a 1-point hand is a constant-width line. Never thinner than
            // half a pixel, so the shaft cannot break up.
            //
            // On the 1-point hands the record reads as the hand's FULL width,
            // hence the halving. GOLDEN (capture frames 256–292, dial pixels
            // read at module resolution): Father Time's hour hand is 2 px
            // across — three pixels per scanline at 48°, and a band of
            // perpendicular width W cuts a horizontal run of W/sin 42° = 1.49W
            // — while his minute and second hands are a single pixel per
            // scanline. Taking the records {2, 1, 1} as half-widths made the
            // hour hand a 5 px wedge and buried the minute hand inside it.
            let w = if hand.points >= 3 {
                (hand.halfw * (1.0 - t / hand.len)).max(0.5)
            } else {
                (hand.halfw * 0.5).max(0.5)
            };
            if dist <= w {
                let o = ((py * n + px) * 4) as usize;
                rgba[o + 3] = 0xFF; // pure black, like the art's ink
            }
        }
    }
    Image { w: n, h: n, rgba }
}

/// Resolve a `gen:hand/...` name back to its pixels. Hour index is in
/// half-degree units (720 over 12 h); minute and second are 6°/unit — the
/// same angles [`hand_sprites`] names.
fn hand_from_name(name: &str) -> Option<Image> {
    let rest = name.strip_prefix(&format!("{}hand/", engine::GEN_PREFIX))?;
    let (base, leaf) = rest.split_once('/')?;
    let base: u32 = base.parse().ok()?;
    let dial = [&RH_DIAL, &FT_DIAL].into_iter().find(|d| d.base == base)?;
    let (kind, idx) = leaf.split_once('_')?;
    let idx: u32 = idx.parse().ok()?;
    let (hi, deg) = match kind {
        "h" => (0usize, idx as f32 * 0.5),
        "m" => (1, idx as f32 * 6.0),
        "s" => (2, idx as f32 * 6.0),
        _ => return None,
    };
    Some(hand_image(dial, deg, &dial.hands[hi]))
}

/// The three hand sprites for a wall-clock reading. Hour resolves at the
/// original's continuous half-degree-per-minute rate (720 distinct
/// positions over 12 h); minute and second at 6°/unit.
fn hand_sprites(dial: &Dial, (h, m, s): (u8, u8, u8)) -> Vec<String> {
    let hi = (h as u32 % 12) * 60 + m as u32; // 0..719, half-degree units
    vec![
        hand_name(dial, 'h', hi),
        hand_name(dial, 'm', m as u32),
        hand_name(dial, 's', s as u32),
    ]
}

/// Blit a dial's hands, anchored so the hand canvas' centre lands on the hub.
fn draw_hands(dial: &Dial, hands: &[String], ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
    for png in hands {
        out.push(SpriteDraw {
            flip: false,
            pal: 0,
            png: png.clone(),
            x: ax + dial.hub.0 - dial.rad,
            y: ay + dial.hub.1 - dial.rad,
        });
    }
}

// ---------------------------------------------------------------------------
// The Shocked Monkey's LCD (§7 / §11 correction)
//
// Spec §11 wrote the 39×69 series-1000 compounds 7–35 off as "plain sequence
// dressing … no code maps digits into them". They are the digits. Decoding
// the packed art (segment probe over each compound) gives:
//
// | compound run | frames | reads | placed at (`bx + dx`) |
// |---|---|---|---|
// | 3 | 1 | the unlit ghost panel `1 8 : 8 8` | 94, 148 |
// | 5 | 1 | the lit leading "1" (hour tens) | 94, 150 |
// | 7..16 | 10 | digits **0–9** (hour units) | 115, 148 |
// | 37 | 1 | the lit colon | 164, 164 |
// | 18..24 | 7 | digits **0–6** (minute tens) | 115 + dx 64 = 179, 148 |
// | 26..35 | 10 | digits **0–9** (minute units) | 115 + dx 118 = 233, 148 |
//
// GOLDEN `g_06`/`g_07`/`g_08`/`g_09` (capture wall clock 1:36 AM, menu bar in
// `g_02` confirms): the panel reads "1:36" — hour-tens cell dark, hour-units
// "1", minutes "3" "6", i.e. 12-hour with no leading zero. Every lit pixel
// count matches these five slots exactly. The colon is dark in `g_06/07/08`
// and lit in `g_09`, so it blinks (UNCERTAIN: duty cycle — modelled as a 1 Hz
// toggle; a good candidate for the unresolved `gCSpriteFlasher` of §5/§11).
const LCD_GHOST: u32 = 3;
const LCD_HOUR_TENS: u32 = 5;
const LCD_HOUR_UNITS: u32 = 7;
const LCD_COLON: u32 = 37;
const LCD_MIN_TENS: u32 = 18;
const LCD_MIN_UNITS: u32 = 26;

// ---------------------------------------------------------------------------
// The LCD's palette — what §5's unresolved "something recolours or overdraws
// them" and §7's descriptor swaps actually are.
//
// GOLDEN `emu/captures/shock-monkey.mp4` (2026-09-12, 90 s of Shocked Monkey,
// four complete shock gags at t = 7.8 / 28.7 / 59.5 / 72.1 s). Sampling the
// modal colour of the lit segments inside the panel rect each frame gives a
// three-state cycle, identical all four times:
//
// | phase | lit-segment colour | starts | ends |
// |---|---|---|---|
// | rest | `(252,255,187)` = the packed `(255,255,194)` | — | the startle |
// | **ramp** | linear fade toward black, ~2.85 RGB units per 100 ms | the frame he enters startle pose 225/205 (f78 / f287 / f595 / f721) | the last shock frame |
// | **white** | `(252,252,252)`, i.e. pure white | 3 frames after the last 277 frame — pose 282 is two frames long, so this is the *leaving 282* edge (f124 / f331 / f646 / f844) | 31 / 30 / 31 / 31 frames later |
//
// Two independent ramps pin the rate: shock 1 falls 252 → 122 over 45 module
// frames (2.89/frame) and shock 4 — whose 277 loop ran 85 frames — falls
// 252 → 73 over 64 (2.80/frame) and keeps going, so the ramp does NOT clamp;
// it just gets cut short by however long the twitch lasts. Full travel to
// black is therefore 255 / 2.85 ≈ 89 module frames ≈ **8.8 s**.
//
// That is also what the **"18:88 all-on flash"** is: there is no all-segments
// frame in the art. At the bottom of the ramp the LIT segments have faded to
// roughly the unlit ghost panel's own `(38,35,35)`, so the whole `1 8 : 8 8`
// ghost reads as uniformly lit for a beat. Artifact of the ramp, not a frame.
//
// ERRATA vs the module header's "the descriptor-swap helpers
// (fn_2C3E/fn_2D74/fn_2DBA) have no visible counterpart here": they do. §7
// step 1 calls `fn_2D74(…, table@0x4C6)` when the shock window is consumed —
// one idle sequence before the startle — and leaving 282 (@`0x1D78–0x1D86`)
// calls `fn_2C3E`/`fn_2DBA`. Those are the two edges the capture puts the ramp
// start and the white flash on. Modelled off the startle rather than the
// window consumption because that is the edge the capture can see.
//
// ERRATA vs §11/§5's `gCSpriteFlasher` guess: the blinking colon and the
// yellow→white recolour are NOT the same mechanism. The colon toggles on its
// own 1.02 s / 50 % cycle right through the white phase (capture: the lit
// pixel count steps 1745 ⇄ 1793 at f1, 11, 22, 32, 42, … regardless of the
// gag), while the recolour is edge-driven by the shock FSM.
/// The packed lit-segment colour of every `RLEP 1000` digit compound — the
/// only pixels the recolour touches (the ghost segments are `(38,35,35)`).
///
/// It is also, exactly, the colour the original ARMS the ramp with:
/// `DATA129+0x4C6` is the `RGBColor {0xFFFF, 0xFFFF, 0xC2C2}` that
/// `fn_2C3E` (@`0x2C3A`) copies into the sequence's `+0x126` (the live
/// colour) and `+0x120` (the base) — `0xFFFF>>8 = 255`, `0xC2C2>>8 = 194`.
const LCD_LIT: [u8; 3] = [255, 255, 194];
/// The ramp's per-frame step and floors, read off `fn_2C9A` (@`0x2C9A`), which
/// the module runs once per monkey animation frame while the ramp is armed:
///
/// ```text
/// r: if r < 0x2926 { r = 0x2626 } else { r -= 0x300 }
/// g: if g < 0x2623 { g = 0x2323 } else { g -= 0x300 }
/// b: if b < 0x2623 { b = 0x2323 } else { b -= 0x300 }
/// ```
///
/// `0x300` of `0xFFFF` is 2.99 8-bit units per frame, and the floors
/// `(0x26,0x23,0x23) = (38,35,35)` are the UNLIT ghost panel's own colour —
/// which is why the bottom of the ramp reads as an "18:88 all-on flash" in
/// the capture and why the 2026-09-12 lane measured 2.85 units/frame over a
/// 45-frame window. The fitted `LCD_FADE_MS = 8800` / `LCD_FADE_STEPS = 16`
/// pair is gone: the ramp is now stepped, per channel, as the original steps
/// it, so red reaches its floor after (255−38)/2.99 = 72 monkey frames
/// (8.7 s at `MK_FRAME_MS`) and blue after 53 (6.4 s).
const LCD_RAMP_STEP: u16 = 0x0300;
const LCD_RAMP_ARM: [u16; 3] = [0xFFFF, 0xFFFF, 0xC2C2];
const LCD_RAMP_FLOOR: [u16; 3] = [0x2626, 0x2323, 0x2323];
const LCD_RAMP_TEST: [u16; 3] = [0x2926, 0x2623, 0x2623];
/// How long the white holds after the monkey leaves pose 282.
///
/// The SAME `+0xB0` long the spec called a "shock metronome" and the port
/// called a suppression window: `fn_1A7A`'s 282 arm stamps
/// `RandomSeedFromTicks() + 0xBB8` (@`0x1D7C`), and `fn_185E`'s head
/// (@`0x186E–0x18D4`) fires `fn_2D74(seq, DATA129+0x4C6)` when it expires —
/// `SetColorRemap(white, white)`, an IDENTITY remap, i.e. the recolour is
/// switched OFF and the packed pale yellow comes back. So one constant does
/// both jobs: for 3.0 s after the gag the LCD is white AND the idle roll
/// cannot start another shock.
///
/// GOLDEN (`shock-monkey.mp4`) measured the white phase at 30/31/31/31 module
/// frames = 3.6–3.7 s against the binary's 3.0 s; the difference is the cue
/// bucket (`fn_18C4` kind 3 only samples the clock once per 120 ms frame) plus
/// the frames the leaving-282 edge itself takes. Constant taken from the C.
/// SUPERSEDED 2026-09-29: the "30/31 module frames" were 100 ms frames (the
/// 900-frame subsample of that 90 s take — the same "module frame" the drift
/// note below measures 1 px per), i.e. 3.0–3.1 s, not 120 ms frames and not
/// 3.6–3.7 s. The full-screen take holds white
/// 30/31/30/30/30/30/30/31 frames over its eight gags — the binary's 3.0 s
/// exactly (and the monkey runs at 60 ms through it, `MK_RECOVER_MS`).
const MK_SHOCK_WINDOW_MS: u64 = 3_000;

/// The LCD palette state machine, `fn_2C3A`/`fn_2C9A`/`fn_2D70`/`fn_2DBA`.
///
/// ERRATA vs the 2026-09-12 header, which filed this under "the unresolved
/// `gCSpriteFlasher` of §5/§11". `gCSpriteFlasher` is **Father Time** — the
/// assert string at `DATA129+0x331` names `fn_1250`, the open-coat machine
/// (§3.2's `fn_1250`) — and it has nothing to do with the monkey's LCD. The
/// recolour is three plain helpers on the SEQUENCE object, driven from the
/// monkey's own handler and chooser:
///
/// | edge | call | effect |
/// |---|---|---|
/// | idle chooser picks 225 / 205 (`@0x1BA4`) | `fn_2C3E(seq, @0x4C6, 1)` | arm: live = base = white |
/// | every monkey frame while armed (`@0x1A5C`) | `fn_2C9A(seq)` | step the live colour down, `SetColorRemap(live, base)` |
/// | leaving 282 (`@0x1DC2`, `@0x1DCC`) | `fn_2C3E(seq,0,0)` then `fn_2DBA(seq, @0x4C6)` | disarm, then `SetColorRemap(black, white)` — the white flash |
/// | `+0xB0` expires (`@0x18A4`) | `fn_2D74(seq, @0x4C6)` | `SetColorRemap(white, white)` = identity — back to pale |
#[derive(Clone, Copy, PartialEq, Debug)]
enum Lcd {
    /// The packed pale yellow — no remap installed.
    Rest,
    /// Armed: the live colour, stepped once per monkey frame by `fn_2C9A`.
    Ramp([u16; 3]),
    /// `SetColorRemap(black, white)` — held until `+0xB0` expires.
    White,
}

impl Lcd {
    /// `fn_2C9A` (@`0x2C9A`), one step, per channel.
    fn step(self) -> Self {
        match self {
            Lcd::Ramp(mut c) => {
                for i in 0..3 {
                    c[i] = if c[i] < LCD_RAMP_TEST[i] {
                        LCD_RAMP_FLOOR[i]
                    } else {
                        c[i] - LCD_RAMP_STEP
                    };
                }
                Lcd::Ramp(c)
            }
            other => other,
        }
    }

    /// The 8-bit tint the lit segments carry, or `None` for the packed art.
    fn tint(self) -> Option<[u8; 3]> {
        match self {
            Lcd::Rest => None,
            Lcd::White => Some([0xFF, 0xFF, 0xFF]),
            Lcd::Ramp(c) => Some([(c[0] >> 8) as u8, (c[1] >> 8) as u8, (c[2] >> 8) as u8]),
        }
    }
}

/// Generated-sprite name for one recoloured LCD cell:
/// `gen:lcd/<tag>/<source compound path>`. `tag` is the tint in hex; the
/// source path is everything after it, so [`lcd_from_name`] can split on the
/// FIRST `/` and hand the rest back to the pack.
fn lcd_name(png: &str, tint: [u8; 3]) -> String {
    format!(
        "{}lcd/t{:02x}{:02x}{:02x}/{png}",
        engine::GEN_PREFIX,
        tint[0],
        tint[1],
        tint[2]
    )
}

/// Recolour one LCD compound's lit segments: `tint` replaces [`LCD_LIT`] and
/// everything else (the ghost segments, the bezel) is copied through.
///
/// Until 2026-09-19 the result was written to
/// `<pack>/compounds/1000/lcd/<leaf>_<tag>.png` and the sprite named that
/// path, so inside a read-only `.saver` bundle the recolour silently never
/// happened and the monkey's display stayed pale through the whole shock
/// ramp and the white flash. Same pixels, no write.
fn lcd_from_name(pack: &Pack, name: &str) -> Option<Image> {
    let rest = name.strip_prefix(&format!("{}lcd/", engine::GEN_PREFIX))?;
    let (tag, png) = rest.split_once('/')?;
    let tag = tag.strip_prefix('t')?;
    let tint = [
        u8::from_str_radix(tag.get(0..2)?, 16).ok()?,
        u8::from_str_radix(tag.get(2..4)?, 16).ok()?,
        u8::from_str_radix(tag.get(4..6)?, 16).ok()?,
    ];
    let img = pack.image(png);
    let mut rgba = img.rgba;
    for px in rgba.chunks_exact_mut(4) {
        if px[0] == LCD_LIT[0] && px[1] == LCD_LIT[1] && px[2] == LCD_LIT[2] {
            px[0] = tint[0];
            px[1] = tint[1];
            px[2] = tint[2];
        }
    }
    Some(Image { w: img.w, h: img.h, rgba })
}

/// The art to blit for one LCD cell under the current palette state: the
/// packed compound at rest, a generated recolour otherwise.
fn lcd_art(png: &str, lcd: Lcd) -> String {
    match lcd.tint() {
        None => png.to_string(),
        Some(t) => lcd_name(png, t),
    }
}

/// One sprite part: current compound-sequence id (+0xA2 state), frame index
/// (+0x9E), per-state tick counter (+0xA4) and the armed flag (+0xA0).
///
/// `len` is carried explicitly because the drip parts (§ RH_DRIPS) are cued on
/// a run that STARTS one past a packed sequence's `first` — the composer hands
/// the engine `(firstCompound + 1, frameCount)` and the compound at `first`
/// itself is the part's `itsFirstFrame` (the class asserts name that field:
/// "CSpriteEar::ChooseNextSequence: itsFirstFrame is bogus!"). `pack.seq()`
/// only keys on `first`, so a run start of e.g. 195 has to carry its own
/// length; `pack.frame()` resolves any absolute compound number.
///
/// `dx/dy` are the frame's OWN link offsets, not deltas to accumulate. The
/// packed runs prove it: RLEP 1200 seq 45 walks `dy = 0,1,6,13,24,36,48` and
/// then RESETS to 0 while `by` jumps 73 → 132, and seq 26 of the 1000 chain
/// repeats `dx = 118` on all ten digit frames. Summing them (as an earlier
/// pass did via `cum_dx`) sent the monkey's minute-units cell off-screen at
/// 118 × 10 and would have flung the ear gag across the field; it only ever
/// looked right because nothing but frame 0 was reached.
struct Part {
    seq: u32,
    len: u32,
    frame: u32,
    counter: u32,
    playing: bool,
}

impl Part {
    fn new() -> Self {
        Part { seq: 0, len: 0, frame: 0, counter: 0, playing: false }
    }

    /// (Re)arm on a packed compound sequence (looked up by its `first`).
    fn cue(&mut self, pack: &Pack, base: u32, seq: u32) {
        let len = pack.seq(base, seq).map(|s| s.frames.len() as u32).unwrap_or(0);
        self.cue_run(seq, len);
    }

    /// (Re)arm on an explicit compound run — used for the drip parts, whose
    /// run start is not a packed `first`.
    fn cue_run(&mut self, start: u32, len: u32) {
        self.seq = start;
        self.len = len;
        self.frame = 0;
        self.counter = 0;
        self.playing = len > 0;
    }

    fn idle(&mut self) {
        self.playing = false;
    }

    /// Advance one frame (base tick fn_5648: vtbl+0x84 step, then +0xA4 += 1).
    /// Returns true when the run finished (vtbl+0x13C "sequence finished?").
    fn step(&mut self) -> bool {
        self.counter += 1;
        if !self.playing {
            return true;
        }
        self.frame += 1;
        if self.frame >= self.len {
            self.frame = self.len.saturating_sub(1);
            return true;
        }
        false
    }

    fn draw(&self, pack: &Pack, base: u32, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        if !self.playing {
            return;
        }
        draw_compound(pack, base, self.seq + self.frame, ax, ay, out);
    }
}

// ---------------------------------------------------------------------------
// Father Time (menu 1; RLEP 1100) — §5

/// The series-1100 compound-sequence chain — all 19 decoded seqs, and Father
/// Time walks them IN ORDER.
///
/// GOLDEN (`emu/captures/shock-clocks.mp4`): identifying each capture frame by
/// its compound's packed size and `by` gives, without a gap, `1`×4 walk cycles
/// (t 7.77–11.77) → `13` → `17` (6 frames, 11.77–12.37) → `21` (4 frames,
/// 12.40–12.77) → `30` + `37` + c_051 (20 frames, 12.77–14.83) → the coat
/// flash `51` onward. Seven consecutive transitions in this table's own order;
/// a uniform draw over 19 entries reproduces that with probability ~1e-9.
///
/// This replaces the old uniform `rng.pct(19)` pick, which left the coat shut
/// — and so the dial invisible — 18 gestures out of 19. ERRATA vs the read
/// above: the capture's single pass goes `21` → `30` directly (its 30.7
/// measured compound frames match 30 without seq 26, not the 33 with it), so
/// the port plays 0.3 s of standing pose the original skipped there. One pass
/// is not enough to delete a packed sequence, so 26 stays in the chain.
///
/// GOLDEN #2 (`emu/captures/shock-hour.mp4`, 2026-09-12, 120 s of Father Time)
/// confirms the in-order read across **six** complete passes — the closed-coat
/// compounds template-match at NCC 0.998, giving `135` → `1`×N → `13` → `17` →
/// `21` → `26` → `30` → `37` → `51` every time (e.g. f136–f185 = 13.6–18.5 s,
/// f265–f368 = 26.5–36.8 s) — and settles the two things the first pass could
/// not: the chain WRAP (see `FatherTime::pos`) and seq **119**, which is
/// NOT a free-running chain entry (see `FT_CUCKOO_SEQ`).
///
/// SUPERSEDED 2026-09-12 (disasm): the order above is real but the chain is
/// NOT a ring walked with an index — it is a branching state machine,
/// `CSpriteFather::ChooseNextSequence` = **`fn_1250`** (`@0x1250–0x1596`,
/// CODE 129), which the spec never disassembled. See `ft_next`. Three of the
/// "in-order" steps are actually coin flips that happened to fall the same way
/// in the capture's passes (13→17 at 37.5 %, 21→26 at 18.75 %, 30→37 at
/// 62.5 %), and the open-coat stretch is a counted LOOP, not a straight run —
/// which is exactly the 57 / 80 / 72 / 46 / 78 / 169-frame spread the capture
/// measures. The array survives only as the catalogue of the 19 packed seqs.
#[allow(dead_code)] // the catalogue; `ft_next` is the machine
const FT_POOL: [u32; 19] = [
    1, 13, 17, 21, 26, 30, 37, 51, 58, 62, 69, 82, 88, 94, 103, 112, 119, 131, 135,
];

/// **The open-coat loop.** `fn_1250`'s shared arm for seqs 51/58/62/69/82/88
/// (`@0x1446–0x14B8`): if the hour flag is up it leaves for `119` at once;
/// otherwise it bumps a gesture counter `+0xA8` and, **while the counter is
/// below `FT_GESTURE_N[band]`**, draws the next gesture uniformly from a
/// six-entry table; once it is at or past it, it leaves through a two-entry
/// table instead. `+0xA8` is cleared on the way through seq `30`
/// (`@0x1408`), so the count is per coat-opening.
///
/// | table | `DATA129` | words |
/// |---|---|---|
/// | loop length, indexed by the **drift band** (`fn_2B16`, @`0x146A`) | `+0x318` | `{13, 9, 6, 4, 3}` |
/// | gesture pool, `Random15() % 6` (@`0x1476–0x1490`) | `+0x322` | `{58, 58, 62, 69, 82, 88}` |
/// | exit pool, `Random15() % 2` (@`0x1498–0x14B2`) | `+0x32E` | `{94, 103}` |
///
/// Note `58` twice — it is 2/6, not 1/6 — and note what the band index means:
/// **the Drift Speed slider also sets how long Father Time fidgets with his
/// coat open.** Faster drift ⇒ fewer gestures.
///
/// GOLDEN (`shock-hour.mp4`): the six open spans measure 57 / 80 / 72 / 46 /
/// 78 / 169 compound frames. Packed lengths are 51 = 6, {58,58,62,69,82,88} =
/// {3,3,6,12,5,5} (mean 5.67), 94 = 103 = 8, 112 = 6, and the open window is
/// compounds 52–117, i.e. `51 + (N−1) gestures + k×(94|103) + 112`. Solving
/// each span for `k` at `N = 6` (band 2) gives k = 1, 5, 4, 1, 5, 16 — all
/// non-negative, mean 5.3 against the 4.0 a ¼-exit geometric predicts. `N = 9`
/// (band 1) is **excluded**: it needs k < 0 for the 46- and 57-frame spans.
/// `N = 13` (band 0, "Still") predicts a 112-frame mean against the measured
/// 84. So the capture was recorded at the factory slider 50 ⇒ band 2 — the
/// same band the monkey capture's 1 px/100 ms drift pins independently (see
/// `DRIFT_INTERVAL_MS`), from a different measurement entirely.
///
/// This is also what sets the hour→cuckoo delay. 94 and 103 do NOT test the
/// hour flag (`@0x14BA`/`@0x14D8`), so a cuckoo that comes due mid-ping-pong
/// waits for `112`. The capture's 10:00:00 rollover at t = 90.0 s lands inside
/// the 169-frame span (the k = 16 outlier, p ≈ 1 %) and the first cuckoo
/// follows at t = 102.5 s — the 12.5 s delay is that ping-pong, not a timer.
const FT_GESTURE_N: [u32; 5] = [13, 9, 6, 4, 3];
const FT_GESTURE_POOL: [u32; 6] = [58, 58, 62, 69, 82, 88];
const FT_GESTURE_EXIT: [u32; 2] = [94, 103];

/// The walk's cycle count. `fn_1250`'s seq-1 arm (`@0x1320–0x1396`): after
/// each completed ten-frame cycle he keeps walking unless a
/// `Random15() % 256 < 96 − 16×band` roll fires, in which case he hands on to
/// `13`. So the count is **geometric**, not fixed — mean `256/(96−16·band)`
/// cycles = 2.7 / 3.2 / 4 / 5.3 / 8 by band.
///
/// GOLDEN (`shock-hour.mp4`): nine non-entry walks measure 1, 5, 4, 5, 3, 1,
/// 5, 3, 2 (mean 3.2, support 1–5) — a geometric sample, and at band 2 (mean
/// 4, see `FT_GESTURE_N` for why band 2) 3.2 is 0.7 σ low for n = 9. The port
/// used to hard-code four, which is the mean and never the spread.
const FT_WALK_STOP_NUMER: u32 = 96;
const FT_WALK_STOP_STEP: u32 = 16;

/// The walk's other gate, ahead of the roll (`@0x1320–0x134E`): while the
/// actor's **HORIZONTAL** position is below 100 or above `screenHeight − 100`
/// he keeps walking unconditionally, roll or no roll. Comparing an `h` against
/// a *height* is the original's own quirk — `@0x1336` subtracts
/// `screen.top (+0x338)` from `screen.bottom (+0x33C)`, not right from left —
/// and it is transcribed, not corrected.
///
/// CORRECTED 2026-09-19 (audio lane). Until today the port read `actor+0x40`'s
/// high word as Mac `Point.v` and substituted the figure's on-screen vertical
/// midpoint, which made the gate dormant — Father Time does not drift
/// (`step_drift`, "Drift is PER-CONTROL"), so a vertical gate on a fixed
/// anchor is a constant, and the walk collapsed to the bare geometric roll of
/// `FT_WALK_STOP_NUMER` (mean 4 cycles at band 2). **The high word is `h`, not
/// `v`**, and the class's own reposition proves it: `fn_1594` @`0x1594` builds
/// the argument to vtbl+`0x88` as
/// `CONCAT22(margin + Random15() % ((0x33E − 0x33A) − 2·margin),
/// −(0x33C − 0x338)/2)` — the **high** half is the one derived from
/// `right − left` (the screen WIDTH) and randomised across it, the low half is
/// the height-derived centring term. So the long at `+0x40` is `(h:v)` with
/// `h` in the high word, and `@0x1326`'s `>> 16` reads `h`.
///
/// That single word is the whole of Jason's 2026-09-19 "fires audio in rounds,
/// chasing each other". `FatherTime::screen_ax` steps `FT_WALK_STEP` = 76 px per
/// cycle from `FT_WALK_ENTER_AX` = −176 and wraps past `FT_WALK_WRAP_AX` = 520,
/// so a lap is the ten positions −176, −100, −24, 52, 128, 204, 280, 356, 432,
/// 508 and only **four of the ten** (128…356) are inside `[100, 380]` where he
/// is even allowed to stop. Expected walk = 10.8 cycles per chain pass instead
/// of 4, i.e. ~6.8 s of silent walking the port was skipping every round.
/// (2026-09-26: the lap is no longer that fixed grid — each chain pass
/// resumes the walk `FT_WALK_RESUME_DX` = +88 px on from where it stopped,
/// see `FatherTime::set_run` — and the cue rates are unchanged by it.)
///
/// GOLDEN (`emu/captures/qemu/shock-father-time*`, 2026-09-19): the audio has
/// 13 gag runs in 270 s of tabulated take — one run per chain pass, since seq
/// 30 is unconditional — for **20.8 s per pass** against the vertical-gate
/// port's 14.1 s, and 15.9 s of silence between a run's last cue and the next
/// run's `1100 Hey You` against the port's 9.3 s. With the `h` gate the port
/// models 19.3 s and 14.3 s, and every per-id count lands inside the capture's
/// ±30 % band (see `father_time_cue_rate_matches_the_audio_golden`). The
/// `shock-hour.mp4` walk lengths 1, 5, 4, 5, 3, 1, 5, 3, 2 are the same story
/// from the video side: a geometric roll cannot make four 5s out of nine
/// samples, four forced cycles out of `−176` can.
const FT_WALK_EDGE_MARGIN: i32 = 100;

/// The hour gag. `RLEP 1100` seq 119 (11 frames, compounds 119–129) swings the
/// dial face open, pops the yellow bird out over the gearworks and shuts it
/// again — the last frame, 129, is the only one of the eleven that shows the
/// white dial (which is why `FT_DIAL_COMPOUND_CUCKOO_END` exists).
///
/// GOLDEN (`emu/captures/shock-hour.mp4`; recording starts 09:58:30, so
/// **10:00:00 is t = 90.0 s** — independently confirmed on the belly dial:
/// tick-subtracting the static pixels over f856–f1024 leaves an hour hand at
/// 301° (10 o'clock = 300°) and a 10-px minute hand at 0°):
///
/// - a tight yellow-bird detector (r>170, g>140, b<90, r−b>110) fires in
///   **exactly ten** runs — onsets f1026, 1037, 1049, 1060, 1071, 1082, 1093,
///   1104, 1116, 1127 — and the white dial disc vanishes for ten frames and
///   returns for one in **ten** cycles of **11 frames** (f1025→f1136,
///   102.5 s → 113.6 s). Eleven frames is seq 119's packed length and ten is
///   `hour % 12` for 10 o'clock. The coat closes (seq 131) on f1137, one frame
///   after the last cuckoo.
/// - the bird detector fires **nowhere else in the other 102 s**, across six
///   complete passes of `FT_POOL`. So 119 is hour-flag driven, exactly like the
///   Rotting Head's state 20 and the monkey's 282 block (`hour % 12`, 0 → 12,
///   §12.2) — it is a chain SLOT that is skipped unless the flag is up.
///
/// The port used to leave 119 in the free-running chain, so it cuckooed once
/// per pass (~every 20 s) and never on the hour. ERRATA vs §5, which lists
/// "119 | flag == 3 | snd 2000" as a plain speech-table row without saying what
/// puts him in 119.
///
/// Onset delay: 90.0 s (the hour) → 102.5 s (first cuckoo) = **12.5 s**, which
/// is the time the chain took to walk the open-coat gestures from the coat
/// flash at 85.6 s to the 119 slot. UNCERTAIN — the port's open-coat run is a
/// single in-order pass of 51/58/62/69/82/88/94/103/112 = 59 compound frames
/// (5.9 s), and the capture's six open spans measure 57, 80, 72, 46, 78 and
/// 169 frames, so the original repeats something in there. Self-similarity
/// clustering of those spans finds only SIX distinct poses, three of them a
/// 2-frame alternation that loops a variable number of times — i.e. the
/// gestures differ by a few pixels and one of them idles. Not enough to name
/// the looping sequence, so the pass stays one-shot and the port's cuckoo
/// lands sooner after the hour than the original's.
const FT_CUCKOO_SEQ: u32 = 119;

/// Where the walk wraps back to the left edge. GOLDEN (`shock-hour.mp4`): the
/// ten walks of the capture start at ax = −176, 140, 152, −176, 140, −176, 65,
/// 152, −176, 65 and step +76 per cycle, so the position is CARRIED between
/// chain passes — it only resets to `FT_WALK_ENTER_AX` after he has walked off
/// the right edge. Two wraps bracket the threshold: the cycle after ax = 459
/// wrapped (f303 → blank f305–f319 → f320 at −176) and so did the one after
/// ax = 444 (f491 → blank → f515 at −176), while ax = 381 → 457 did not. The
/// walk art spans compound x 108…264, so ax = 520 already puts the leading
/// frame's left edge at 628 and the next cycle would be entirely off-screen.
const FT_WALK_WRAP_AX: i32 = 520;

struct FatherTime {
    part: Part,
    /// `+0xAA`: the hour this actor last saw — primed from `GetLocalTime` by
    /// the ctor (`fn_0DF4` @`0x0E30`), so a fresh Father Time does not fire a
    /// spurious cuckoo on his first frame.
    last_hour: u8,
    /// `+0x40`: the actor's position — **the CENTRE of the current frame**
    /// (port-plan §2), here relative to the (`FT_AX`, `FT_AY`) draw origin.
    /// It is moved by exactly two things, both Library code:
    /// the within-run centre link on every advance (`fn0DF4` → `fn3DDC`), and
    /// the shared-part registration on every run hand-off (`fn028A` →
    /// `fn0D3C` → `L135 fn3F2E`) — see `ft_set_run`. Persists across
    /// chain passes; see `FT_WALK_WRAP_AX` for the one place it is reset.
    pos: (i32, i32),
    /// `+0x3C` bit 0: the sprite is drawn mirrored. Toggled by `fn3F2E`
    /// when the part it registers on carries the opposite flip bit in the
    /// incoming frame (24/28 → 30 on, 133 → 135 off) — see `ft_set_run`.
    flip: bool,
    /// +0xA6 hour event: seq 119 is due, `cuckoo_n` times.
    hour_pending: bool,
    /// +0xAA, latched by the tick (`@0x0FD4–0x0FFC`); the 119 arm reads
    /// `hour % 12` off it, 0 → 12.
    cuckoo_n: u32,
    /// +0xAC, cleared on every hour latch: cuckoos already played this hour.
    cuckoo_ct: u32,
    /// +0xA8: gestures played since the coat opened (cleared on seq 30,
    /// `@0x1408`) — see `FT_GESTURE_N`.
    gesture_ct: u32,
    /// The belly dial's hour/minute/second hands for the current reading,
    /// refreshed every animation frame from `ctx.local_hms`.
    hands: Vec<String>,
}

impl FatherTime {
    fn new(pack: &Pack, hour: u8) -> Self {
        let mut part = Part::new();
        part.cue(pack, BASE_FT, FT_WALK_SEQ);
        let mut ft = FatherTime {
            part,
            last_hour: hour,
            pos: (0, 0),
            flip: false,
            hour_pending: false,
            cuckoo_n: 12,
            cuckoo_ct: 0,
            gesture_ct: 0,
            hands: Vec::new(),
        };
        // The first SetRun has no current frame (`+0x3A` == 0), so `fn028A`
        // registers nothing and the walk simply starts at the entry offset.
        ft.place_ax(pack, FT_WALK_ENTER_AX);
        ft
    }

    /// Put the current frame's compound origin at screen `ax` (unmirrored);
    /// the centre follows from the frame's own rect.
    fn place_ax(&mut self, pack: &Pack, ax: i32) {
        let no = self.part.seq + self.part.frame;
        let (cx, cy) = ft_centre(pack, no);
        self.pos = (ax - FT_AX + cx, cy);
    }

    /// The screen x of the current frame's compound origin (bank space) —
    /// the "ax" every capture measurement in this file is quoted in. For the
    /// unmirrored walk it is the old `walk_ax`: −176, −100, −24, 52 …
    fn screen_ax(&self, pack: &Pack) -> i32 {
        let no = self.part.seq + self.part.frame;
        let (cx, _) = ft_centre(pack, no);
        FT_AX + self.pos.0 - cx
    }

    /// Sequence `+0x78` = `L135 fn3DDC` @3DDC: `centre(to) − centre(from)`,
    /// x negated while mirrored. Applied by `fn0DF4` on every advance.
    ///
    /// Pinned model: [`LinkModel::CENTRE_DIFF`] — the plain centre
    /// difference, no flip-bit rounding; a missing record counts as centre
    /// (0, 0) (see `centre`).
    fn link(&mut self, pack: &Pack, from: u32, to: u32) {
        let (a, b) = (frame_box(pack, BASE_FT, from), frame_box(pack, BASE_FT, to));
        let (dx, dy) = l135::link(&a, &b, self.flip, LinkModel::CENTRE_DIFF);
        self.pos.0 += dx;
        self.pos.1 += dy;
    }

    /// `+0x7C` = `L132 fn0204` → `+0x108` = `fn028A` @028A: the LINKED
    /// SetRun this class's vtable binds (`M129_g036E`: `+0x7C` = fn0204,
    /// `+0x108` = fn028A, `+0x148` = `M129_fn108` @572E which calls it with
    /// the id `fn_1250` just stored at `+0xA2`).
    ///
    /// 1. `+0xF0` = `fn1186` @1186: the frame to pass through is `id − 1`
    ///    when that record exists (`+0x82` = 1 after the sprite reset @0058).
    ///    **Series 1100 has no such record for any of its 19 run starts** —
    ///    OFst 1100 skips 11/12, 16, 20, 25, 29, 36, 50, 57, 61, 68, 81, 87,
    ///    93, 102, 111, 118, 130, 134 and 0 — so the "marker" is the run's
    ///    own first frame. The general form is kept anyway.
    /// 2. `+0xCC` = `fn0D3C` @0D3C → sequence `+0x80` = **`L135 fn3F2E`**
    ///    @3F2E: register the CURRENT frame against the marker on the first
    ///    art id both part tables carry (current's table outer, marker's
    ///    inner), so that part keeps its screen position:
    ///    `pos += (partA − centreA) + (centreB − partB)`, each term taken on
    ///    the frame as it is drawn (mirrored while `+0x3C` bit 0 is set). If
    ///    the two parts' flip bits differ the SPRITE's flip is toggled
    ///    between the two terms (`@3F2E`, the XOR of the two `+0xE` words).
    ///    No shared part → no move.
    /// 3. `+0xF4` = `fn1210` / `+0xD0` = `fn0DF4`: marker → first by the
    ///    centre link (0 here, marker == first).
    ///
    /// This is what makes Father Time stop WHERE HE STOPPED. The port used
    /// to draw every non-walk run at the fixed `FT_AX` anchor, so the stop
    /// snapped him to x ≈ 262 from wherever the walk ended (Jason
    /// 2026-09-26: "blinked to the left hand side mid walk"; motion_lint
    /// 143.5 px teleports). The registrations the pack produces are exactly
    /// the capture's numbers: 10 → 1 = **+76** (the walk stride, on the
    /// head, art 37), 10 → 13 = −24 (the capture's "ax 28 after a walk at
    /// 52"), 15/19 → 21 = +5 ("33 for 21"), 24/28 → 30 = 0 **with the
    /// mirror turned on** (art 9, the feet — `shock-father-time.mp4` f611's
    /// open coat template-matches c_058 MIRRORED at NCC 0.966 vs 0.832
    /// plain), 133 → 135 = 0 with it turned off, and 139 → 1 = +107 on the
    /// head, i.e. the walk resumes 88 px on from where it stopped.
    fn set_run(&mut self, pack: &Pack, id: u32) {
        let cur = self.part.seq + self.part.frame;
        let marker = l135::marker_of(id as i32, |f| f >= 1 && pack.frame(BASE_FT, f as u32).is_some()) as u32;
        self.register(pack, cur, marker);
        self.part.cue(pack, BASE_FT, id);
        if marker != id {
            self.link(pack, marker, id);
        }
    }

    /// `L135 fn3F2E` @3F2E — see `set_run` and `register_shared_part`.
    fn register(&mut self, pack: &Pack, a: u32, b: u32) {
        if let Some(r) = register_shared_part(pack, BASE_FT, a, b, &mut self.flip) {
            self.pos.0 += r.delta.0;
            self.pos.1 += r.delta.1;
        }
    }

    /// **Father Time has no minute flag.** §5 reads `@0x0FD4–0x0FFC` as one;
    /// that block is the HOUR latch (`+0xAA = hour`, `+0xA6 = 1`,
    /// `+0xAC = 0`, exactly the monkey's `@0x18DA`), and `fn_1250` never
    /// tests anything but `+0xA6`. The latch now lives in `tick`, where the
    /// original has it — `fn_0FB2` is the class's DoDrawFrame, so the event
    /// is raised on the actor's own 100 ms frame, not on the 40 ms module
    /// grid the 2026-09-12 port dispatched it from.

    fn tick(&mut self, ctx: &mut Ctx, pack: &Pack, band: usize) {
        let (hour, _, _) = ctx.local_hms;
        // ---- the belly dial tells real time (§4) — see FT_DIAL ----
        self.hands = hand_sprites(&FT_DIAL, ctx.local_hms);

        // `gCSpriteFlasher::DoDrawFrame` = **fn_0FB2** @`0x0FB2`. The hour
        // latch is the handler's own (@`0x0FD8`): on a new hour it stores
        // the hour in `+0xAA`, sets `+0xA6` and clears `+0xAC`. There is no minute test
        // anywhere in the class — see `on_minute`.
        if hour != self.last_hour {
            self.last_hour = hour;
            self.hour_pending = true;
            self.cuckoo_ct = 0;
            self.cuckoo_n = match hour % 12 {
                0 => 12,
                n => u32::from(n),
            };
        }
        // The speech table (@`0x1026`–`0x1096`), keyed on `+0xA2` and the
        // run's own counter `+0xA4`. ERRATA: the counters are **1** and **4**,
        // not the 0 and 3 the 2026-09-12 port used — `fn_0FB2` runs BEFORE
        // the base tick increments `+0xA4`, so counter 1 is the run's second
        // frame, which is where the original speaks.
        if self.part.counter == 1 {
            match self.part.seq {
                30 => ctx.sounds.push(SND_HEY_YOU),     // @0x105A, snd 0x44C
                37 => ctx.sounds.push(SND_WANNA_KNOW),  // @0x1064, snd 0x44F
                62 | 69 => ctx.sounds.push(SND_LAUGH2), // @0x106E, snd 0x44E
                94 | 103 => {
                    // Random15() % 256 < 48 = 18.75 % (@0x1080–0x1088)
                    if ctx.rng.pct(256) < 48 {
                        ctx.sounds.push(SND_HMM); // snd 0x44D
                    }
                }
                _ => {}
            }
        }
        if self.part.seq == FT_CUCKOO_SEQ && self.part.counter == 4 {
            // @0x1096: counter == 4. Once per 119 pass, so the ten passes of
            // the 10 o'clock hour in `shock-hour.mp4` are ten "Cuckoo"s.
            ctx.sounds.push(SND_CUCKOO);
        }

        let before = self.part.seq + self.part.frame;
        let done = self.part.step();
        let after = self.part.seq + self.part.frame;
        if !done && after != before {
            // `+0x84` = `fn0316` → `+0xD0` = `fn0DF4`: the centre link
            self.link(pack, before, after);
        }
        if done {
            // ChooseNextSequence. The base tick `fn_5648` reaches it whenever
            // vtbl+0x13C says the sequence finished, so he never stalls: the
            // GOLDEN's 8.7 s of Father Time has no held pose anywhere, every
            // sequence runs straight into the next (see FT_POOL). What USER
            // REPORT 2026-09-01 "way sped up / hyper speeded" was seeing is
            // the RATE — the port ran one compound frame per 40 ms module
            // tick instead of the CueAnim's 100 ms, 2.5× too fast — not a
            // missing pause.
            //
            let h = self.screen_ax(pack);
            let next = self.choose_next(ctx, band, h);
            // `M129_fn108` @572E: the linked SetRun. The walk's +76 px per
            // cycle is NOT an actor step — it is `fn3F2E` registering the
            // head (art 37) of c_010 onto c_001 (see `set_run`).
            self.set_run(pack, next);
            if next == FT_WALK_SEQ && self.screen_ax(pack) >= FT_WALK_WRAP_AX {
                // GAP(off-screen re-entry): the class's `+0x144` = `fn_1594`
                // @1594 re-places an actor its `+0x34` calls invisible after
                // every SetRun, but the argument it builds does not read as a
                // left-edge re-entry and the capture's is one; the port keeps
                // the capture-measured wrap (see `FT_WALK_WRAP_AX`).
                self.place_ax(pack, FT_WALK_ENTER_AX);
            }
        }
    }

    /// `CSpriteFather::ChooseNextSequence` — **`fn_1250`** @`0x1250`, CODE 129,
    /// transcribed arm for arm. Every `Random15() % 256 >= K` below is the
    /// listing's divide-by-256, keep-remainder, compare-to-K idiom; `band` is
    /// `fn_2B16`'s drift band, fetched once at @`0x1258`.
    ///
    /// | seq | @ | next |
    /// |---|---|---|
    /// | 1 (walk) | `0x1320` | within `FT_WALK_EDGE_MARGIN` of an edge → 1; else hour → 13; else `rand%256 < 96−16·band` → 13, else 1 |
    /// | 13 | `0x1398` | hour → 21; else `rand%256 >= 96` → 21 (62.5 %), else 17 |
    /// | 17 | `0x13C8` | 21 |
    /// | 21 | `0x13D0` | hour → 30; else `rand%256 >= 48` → 30 (81.25 %), else 26 |
    /// | 26 | `0x1400` | 30 |
    /// | 30 | `0x1408` | clears `+0xA8`; hour → 51; else `rand%256 >= 160` → 51 (37.5 %), else 37 |
    /// | 37 | `0x143E` | 51 |
    /// | 51/58/62/69/82/88 | `0x1446` | the open-coat loop — see `FT_GESTURE_N` |
    /// | 94 | `0x14BA` | `rand%256 >= 64` → 103 (75 %), else 112 |
    /// | 103 | `0x14D8` | `rand%256 >= 64` → 94 (75 %), else 112 |
    /// | 112 | `0x14F4` | hour → 119, else 131 |
    /// | 119 | `0x150C` | 131, unless the hour still owes cuckoos → 119 |
    /// | 131 | `0x1552` | 135 |
    /// | 135 | `0x1558` | 1 |
    /// | other | `0x155E` | assert stub, print and continue (§12.4) |
    fn choose_next(&mut self, ctx: &mut Ctx, band: usize, h: i32) -> u32 {
        // every roll in fn_1250 is `p_Random15()`, the seg-132 lagged-Fibonacci
        // (`Ctx::rng`) — shock-clocks never calls ANSI `rand()`.
        let r = |ctx: &mut Ctx| ctx.rng.pct(256);
        match self.part.seq {
            FT_WALK_SEQ => {
                // `@0x1326`: `(actor+0x40) >> 16`, read by 7dc70f1 as the
                // actor's `h` and fed the walk's compound-origin x (the old
                // `walk_ax`, now `screen_ax`) — kept exactly as calibrated
                // against the audio golden. `@0x1336` compares it against the
                // screen HEIGHT less the margin; see `FT_WALK_EDGE_MARGIN`.
                // GAP(walk-stop gate operand): `shock-father-time.mp4`
                // t 55.5–56.5 s stops a walk at ax = −100 (c_001 left 8,
                // c_010 left 88, c_013 left 102 — NCC 0.99), which this
                // reading forbids; the operand is not settled.
                let clear =
                    h >= FT_WALK_EDGE_MARGIN && h <= SCREEN_H - FT_WALK_EDGE_MARGIN;
                if !clear {
                    FT_WALK_SEQ
                } else if self.hour_pending {
                    13
                } else {
                    let stop = FT_WALK_STOP_NUMER
                        .saturating_sub(FT_WALK_STOP_STEP * band as u32);
                    if r(ctx) < stop { 13 } else { FT_WALK_SEQ }
                }
            }
            13 => {
                if self.hour_pending || r(ctx) >= 96 {
                    21
                } else {
                    17
                }
            }
            17 => 21,
            21 => {
                if self.hour_pending || r(ctx) >= 48 {
                    30
                } else {
                    26
                }
            }
            26 => 30,
            30 => {
                // `@0x1408`: the gesture counter is cleared on the way OUT of
                // seq 30, so `FT_GESTURE_N` counts gestures per coat-opening.
                self.gesture_ct = 0;
                if self.hour_pending || r(ctx) >= 160 {
                    51
                } else {
                    37
                }
            }
            37 => 51,
            51 | 58 | 62 | 69 | 82 | 88 => {
                if self.hour_pending {
                    // the open-coat arm is the ONLY fast path to the cuckoo;
                    // it does not consume a gesture (@0x1446, before the bump)
                    FT_CUCKOO_SEQ
                } else {
                    self.gesture_ct += 1;
                    if self.gesture_ct < FT_GESTURE_N[band] {
                        FT_GESTURE_POOL[ctx.rng.pct(6) as usize]
                    } else {
                        FT_GESTURE_EXIT[ctx.rng.pct(2) as usize]
                    }
                }
            }
            // the 94 ⇄ 103 ping-pong: NEITHER arm tests the hour flag, which
            // is what makes the cuckoo late (see `FT_GESTURE_N`)
            94 => {
                if r(ctx) >= 64 {
                    103
                } else {
                    112
                }
            }
            103 => {
                if r(ctx) >= 64 {
                    94
                } else {
                    112
                }
            }
            112 => {
                if self.hour_pending {
                    FT_CUCKOO_SEQ
                } else {
                    131
                }
            }
            FT_CUCKOO_SEQ => {
                if !self.hour_pending {
                    return 131;
                }
                self.cuckoo_ct += 1;
                if self.cuckoo_ct >= self.cuckoo_n.max(1) {
                    self.hour_pending = false;
                    131
                } else {
                    FT_CUCKOO_SEQ
                }
            }
            131 => 135,
            135 => FT_WALK_SEQ,
            other => {
                eprintln!("ShockClocks: CSpriteFather state {other} out of table, continuing");
                FT_WALK_SEQ
            }
        }
    }


    /// L135 `fn16BC`: blit the current frame at `pos − (w/2, h/2)`, mirrored
    /// while `flip`. `ax`/`ay` is the assembly origin (`FT_AX`, `FT_AY`).
    fn draw(&self, pack: &Pack, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        if !self.part.playing {
            return;
        }
        let no = self.part.seq + self.part.frame;
        let Some(f) = pack.frame(BASE_FT, no) else { return };
        let left = ax + self.pos.0 - (f.w >> 1);
        let top = ay + self.pos.1 - (f.h >> 1);
        out.push(SpriteDraw { flip: self.flip, pal: 0, png: f.png.clone(), x: left, y: top });
        // the hands only exist while the coat is open on the white face.
        // The dial is drawn by code at its hub (compound space), so map the
        // hub through the frame as drawn — mirrored about the frame rect.
        if FT_DIAL_COMPOUNDS.contains(&no) || no == FT_DIAL_COMPOUND_CUCKOO_END {
            let hub_x = if self.flip { f.bx + f.bx + f.w - FT_DIAL.hub.0 } else { FT_DIAL.hub.0 };
            let ox = left - f.bx + (hub_x - FT_DIAL.hub.0);
            draw_hands(&FT_DIAL, &self.hands, ox, top - f.by, out);
        }
    }
}

// ---------------------------------------------------------------------------
// Rotting Head (menu 2; RLEP 1200) — §6

/// The head base. **Compound 3**, not compound 1: the CODE 130 scene composer
/// (`fn @0x01A6`) `new`s a 62-byte part at `@0x02D4` and initialises it with
/// the value 3 (@`0x0312`) before adding it to the scene, then a second
/// 62-byte part initialised with `0xC0 = 192` (@`0x0388`) — the neck guts.
/// Those two are the only always-visible art the composer installs.
///
/// Compounds 1 and 3 are byte-identical apart from **156 pixels at the left
/// eye**: compound 1 draws it as a plain closed slit, compound 3 as the RED
/// WINKING slit. Every RH frame of the golden (`g_12`–`g_15`) shows the wink,
/// so compound 3 is the base and compound 1 (the full 240×511 sheet, which
/// also carries the loose eyeball and the two fallen ears parked at the bottom
/// of its canvas) is the `RLEP 1200` art sheet, never blitted whole. Both
/// leave the right eye socket and the mouth as HOLES for the part sprites.
const RH_HEAD: u32 = 3;
/// The dangling neck guts (composer @`0x0388`, `0xC0`). 97×83 at bx=161,
/// by=186 — the full tube bundle the golden shows below the chin. The
/// individual tubes (compounds 194 / 202 / 210 / 218 / 226 / 234 / 248 / 262 /
/// 281 / 295 / 309) are sub-pieces of this same bundle and are the drip parts'
/// `itsFirstFrame`s, not separate dressing.
const RH_GUTS: u32 = 192;
/// `CSpriteRightEye`'s resting state, and its run: **9**, a one-frame block
/// (40×27 at bx=240). `fn_2156`'s ctor writes `+0xA2 = 9` (@`0x2178`) and
/// `fn_223E`'s Init pushes `+0xA2` into SetRun (@`0x2276`) — the `0x14` two
/// instructions earlier feeds GetBounds, not the run.
///
/// ERRATA vs the 2026-09-12 port, which cued a one-frame run on compound
/// **11** because 11 template-matched the golden better than 9. 11 is the
/// first frame of the eight-frame BLINK run the machine's state 11 plays,
/// so a golden frame caught mid-blink matches it; the machine's rest is 9.
const RH_EYE_REST: u32 = 9;

/// The 18 drip parts, verbatim from the composer's `(startCompound, count)`
/// pushes (@`0x058C`–`0x0E8A`; every group also pushes shared snd `0x7535` =
/// 30005). Each start is `packedSequenceFirst + 1`, and the count matches the
/// packed run length minus one, all 18 of them:
///
///   195/6→seq 194(7)  203/6→202(7)  211/6→210(7)  219/6→218(7)  227/6→226(7)
///   235/12→234(13)  249/12→248(13)  263/17→262(18)  282/12→281(13)
///   296/12→295(13)  310/12→309(13)  324/12→323(13)  338/12→337(13)
///   352/10→351(11)  364/10→363(11)  376/10→375(11)  388/10→387(11)
///   400/10→399(11)
///
/// Three shapes of drip: a 9×16 blob that falls in six steps off the five neck
/// tubes; a 4×5 bead that swells to 4×58 and lets go, at eight points around
/// the guts and jaw; and an 8×6 → 8×199 thread that runs off the brow, the
/// tongue, the lip and the eyeball. GOLDEN: `g_12` has exactly one lit (the
/// red bead below the guts), `g_14` has three (a tear off the eyeball, one off
/// the tongue tip, one below the guts).
const RH_DRIPS: [(u32, u32); 18] = [
    (195, 6),
    (203, 6),
    (211, 6),
    (219, 6),
    (227, 6),
    (235, 12),
    (249, 12),
    (263, 17),
    (282, 12),
    (296, 12),
    (310, 12),
    (324, 12),
    (338, 12),
    (352, 10),
    (364, 10),
    (376, 10),
    (388, 10),
    (400, 10),
];
/// Shared snd `0x7535` pushed by every drip group (@`0x05E6` …).
const SND_DRIP: u32 = 30005;

/// APPROXIMATION: the drips' arm gate is behind an unresolved indirect call in
/// the composer, so the module's own house rate is reused — 8/2048 (0.39 %),
/// the documented CSpriteEar/CSpriteTooth gate (§9). At 25 Hz that idles each
/// part ~10 s and keeps ≈0.7 of the 18 running at once, which is what the
/// golden shows (1 lit in `g_12`, 3 in `g_14`).
///
/// It is rolled on `ctx.rng`, the module's only generator, like the ear
/// and tooth gates. `Random15` is an LCG sampled on its LOW bits, so bit *k*
/// has period 2^(k+1); polling 18 (or 8) idle parts every tick makes the draw
/// stride constant, which freezes the bottom bits and turns `& 0x7FF < 8` into
/// "a burst of drips for a few seconds, then none, ever". The original never
/// hits that because its gates only run when a part's sequence *ends*. Faithful
/// where the disasm pins the generator, decorrelated where it does not.
const DRIP_GATE: u32 = 8;

/// One `CSpriteEar` (`fn_08FE` ctor @`0x08FA`). The two ears are built with
/// the same class and told apart by one ctor word (`fn_09BE` @`0x09BA`:
/// `0` → state `0x57` = 87, non-zero → `0x2B` = 43).
struct Ear {
    part: Part,
    /// `+0xA6`: the slide's per-step velocity, `+= 50` on every ground frame
    /// (`fn_0C14` @`0x0C10`), cleared when the ear is gone.
    vel: i32,
    /// `+0xA8`: the slide's accumulated offset. `fn_0CE8` (@`0x0CE4`,
    /// vtbl+0x144) takes the scene centre through the scene's vtbl+0x94 and
    /// then does `add.w [A6-0x1A], D0` (@`0x0D34`) — the SECOND word of the
    /// returned `Point`, i.e. **h**, the x. So the ear does not keep falling
    /// once it is off the head: the 29-frame droop run carries the drop in
    /// its own art (packed `dy` 0,1,6,13,24,36,48 with `by` jumping 73→132)
    /// and the ground state then SLIDES it sideways, accelerating, until it
    /// is past the assembly's width (`fn_0C14` @`0x0C4C` compares against the
    /// scene rect's width). The 2026-09-12 port had no fall/slide at all.
    slide_x: i32,
}

/// One `CSpriteTooth` (`fn_2622` ctor @`0x261E`). Same trick: one ctor word
/// picks the ring (`fn_26D6` @`0x26D2`: `0` → `0xA5` = 165, else `0x8A` = 138).
/// `enabled` is the library's `+0x9E` flag, which `CSpriteMouth` drives
/// through vtbl+0x140 — while it is clear the tooth is frozen and its
/// `vtbl+0x34` gate (`fn_5554` @`0x5550`) reads 0, so it cannot leave rest.
struct Tooth {
    part: Part,
    enabled: bool,
}

struct RottingHead {
    /// `CSpriteRightEye` — the head's main actor. `+0xA2` is one of 9 (rest),
    /// 11 (the blink) or 20 (the cuckoo), and the state IS the compound run.
    eye: Part,
    hour_flag: bool,   // +0xA6
    last_hour: u8,     // +0xA8
    cuckoo_count: u32, // +0xAA
    minute_flag: bool, // +0xAC
    last_minute: u8,   // +0xAE
    mouth: Part,       // CSpriteMouth (136 ⇄ 131)
    teeth: [Tooth; 2], // rings 138→140→148→160 and 165→167→175→187
    ears: [Ear; 2],    // rings 43→45→75→77 and 87→89→119→121
    drips: Vec<Part>,
    /// The dial's hour/minute/second hand sprites for the current reading,
    /// refreshed every tick from `ctx.local_hms`.
    hands: Vec<String>,
}

/// `CSpriteEar::ChooseNextSequence` — **`fn_0AE4`** @`0x0AE0`, transcribed.
/// Two independent four-state RINGS, one per ear; only the head-worn state
/// rolls a gate, and the ground state is where the fall lives:
///
/// | state | @ | what | next |
/// |---|---|---|---|
/// | 43 / 87 | `0x0AF6` / `0x0B8C` | ear on the head | `Random15() & 0x7FF < 8` → 45 / 89, else stay |
/// | 45 / 89 | `0x0B6C` / `0x0BCC` | the 29-frame droop | 75 / 119 |
/// | 75 / 119 | `0x0B50` / `0x0BAC` | falling: `fn_0C14` steps it down | 77 / 121 on landing |
/// | 77 / 121 | `0x0B7C` / `0x0BDC` | the 9-frame slide back on | 43 / 87 |
///
/// (the 2026-09-12 port had the ring right but no fall — 75/119 handed on
/// unconditionally, so the ear never left the temple.)
const EAR_RING: [[u32; 4]; 2] = [[43, 45, 75, 77], [87, 89, 119, 121]];
/// `CSpriteTooth::ChooseNextSequence` — **`fn_2790`** @`0x2790`.
/// `138 →(gate) 140 → 148 → 160 → 138` and `165 →(gate) 167 → 175 → 187 → 165`.
const TOOTH_RING: [[u32; 4]; 2] = [[138, 140, 148, 160], [165, 167, 175, 187]];
/// The gate both classes roll at their resting state: `Random15() & 0x7FF < 8`
/// (@`0x0B36`/`0x0B94`, @`0x27C8`/`0x28B4`) = 8/2048 = 0.39 % per run end.
const RING_GATE: u32 = 8;
/// `fn_0C14` (@`0x0C10`): each ground frame adds 50 to the ear's velocity.
const EAR_SLIDE_STEP: i32 = 50;
/// `CSpriteMouth`'s open roll — `Random15() & 0x1FF < 0x10` (@`0x2036`),
/// 16/512 = 3.125 % per rest frame, AND both teeth must be at rest
/// (`fn_2926` @`0x2922`).
const MOUTH_GATE: u32 = 0x10;
const MOUTH_REST: u32 = 136; // ctor +0xA2 = 0x88
const MOUTH_OPEN: u32 = 131; // init GetBounds id 0x83, and the other state

impl RottingHead {
    fn new(pack: &Pack, hour: u8, minute: u8) -> Self {
        let mut rh = RottingHead {
            eye: Part::new(),
            hour_flag: false,
            // `fn_2156` (@0x2152) primes +0xA8/+0xAE from GetLocalTime.
            last_hour: hour,
            cuckoo_count: 0,
            minute_flag: false,
            last_minute: minute,
            mouth: Part::new(),
            teeth: [
                Tooth { part: Part::new(), enabled: true },
                Tooth { part: Part::new(), enabled: true },
            ],
            ears: [
                Ear { part: Part::new(), vel: 0, slide_x: 0 },
                Ear { part: Part::new(), vel: 0, slide_x: 0 },
            ],
            drips: (0..RH_DRIPS.len()).map(|_| Part::new()).collect(),
            hands: Vec::new(),
        };
        rh.eye.cue(pack, BASE_RH, RH_EYE_REST);
        rh.mouth.cue(pack, BASE_RH, MOUTH_REST);
        for i in 0..2 {
            rh.teeth[i].part.cue(pack, BASE_RH, TOOTH_RING[i][0]);
            rh.ears[i].part.cue(pack, BASE_RH, EAR_RING[i][0]);
        }
        rh
    }

    /// `CSpriteRightEye::ChooseNextSequence` — **`fn_240E`** @`0x240E`.
    ///
    /// | state | @ | next |
    /// |---|---|---|
    /// | 9 (rest) | `0x2422` | hour flag → 20; else minute flag → clear it, 11; else STAY on 9 |
    /// | 11 (blink) | `0x2496` | 9 |
    /// | 20 (cuckoo) | `0x24CA` | count one cuckoo, clear the hour flag when counted out, → 9 |
    /// | other | `0x2596` | assert stub, print and continue (§12.4) |
    ///
    /// So the head does NOT sit in 20 chiming: it alternates 9 ⇄ 20 until the
    /// count is met, exactly as Father Time alternates 119 ⇄ 119.
    ///
    /// **ORIGINAL QUIRK.** The count is `hour % 12`, and the zero test is on
    /// the HOUR, not on the remainder (`0x24DE` tests the hour field `+0xA8`). At
    /// midnight `hour = 0` → `n = 12`; but at NOON `hour = 12` → `12 % 12 = 0`
    /// and the hour is non-zero, so `n` stays **0**, `0 <= count` holds on the
    /// first pass and the head cuckoos exactly ONCE at 12:00. Kept (§12.2).
    fn eye_next(&mut self) -> u32 {
        match self.eye.seq {
            9 => {
                if self.hour_flag {
                    20
                } else if self.minute_flag {
                    self.minute_flag = false;
                    11
                } else {
                    9
                }
            }
            11 => 9,
            20 => {
                if self.hour_flag {
                    let mut n = u32::from(self.last_hour) % 12;
                    if self.last_hour == 0 {
                        n = 12;
                    }
                    self.cuckoo_count += 1;
                    if n <= self.cuckoo_count {
                        self.hour_flag = false;
                    }
                }
                9
            }
            other => {
                eprintln!("ShockClocks: CSpriteRightEye state {other} out of table, continuing");
                9
            }
        }
    }

    /// How far the ear slides before the ground state hands on. The original
    /// compares `+0xA6 + pos.h` against the scene rect's WIDTH (`fn_0C14`
    /// @`0x0C4C`) through the scene's vtbl+0x98.
    ///
    /// GAP(ear slide extent): the port has no per-assembly scene rect for the
    /// Rotting Head (`assembly()` only knows the monkey's 395x184), so the
    /// packed head width stands in. The ear comes back either way; only how
    /// far off the temple it gets is approximate.
    const EAR_SLIDE_LIMIT: i32 = 240;

    fn tick(&mut self, ctx: &mut Ctx, pack: &Pack) {
        let (hour, minute, _) = ctx.local_hms;
        // ---- the dial tells real time (§4) — see RH_DIAL ----
        self.hands = hand_sprites(&RH_DIAL, ctx.local_hms);

        // ---- `CSpriteRightEye::DoDrawFrame` = fn_22E8 @0x22E4 ----
        //
        // THE MINUTE-FLAG BRANCH (open since 2026-09-12). The latch is an
        // if/ELSE, not two independent tests (@0x22F8-0x2332): the minute
        // flag can only rise on a frame where the HOUR did not change. On the
        // hour the minute event is swallowed, which is why the head never
        // blinks and cuckoos on the same frame. (2026-09-29: swallowed for
        // THAT frame only — `+0xAE` is left stale, so the flag rises on the
        // next frame and the blink plays after the hour rounds; the port
        // does the same, see `rotting_head_cuckoo_prelude_follows_fn_22e8`.)
        if hour != self.last_hour {
            self.last_hour = hour;
            self.hour_flag = true;
            self.cuckoo_count = 0;
        } else if minute != self.last_minute {
            self.last_minute = minute;
            self.minute_flag = true;
        }
        // the two cue rows, both on the run's own counter (@0x2352-0x23EC)
        if self.eye.seq == 11 && self.eye.counter == 0 {
            ctx.sounds.push(SND_CUCKOO_PRELUDE); // snd 1200
        }
        if self.eye.seq == 20 {
            if self.eye.counter == 0 {
                ctx.sounds.push(SND_CUCKOO_PRELUDE);
            }
            if self.eye.counter == 4 {
                ctx.sounds.push(SND_CUCKOO); // shared snd 2000
            }
        }
        if self.eye.step() {
            let next = self.eye_next();
            self.eye.cue(pack, BASE_RH, next);
        }

        // ---- `CSpriteMouth::ChooseNextSequence` = fn_1FC6 @0x1FB6 ----
        //
        // 131 arms both teeth (vtbl+0x140 with 1, @0x1FFC/@0x201C) and hands
        // on to 136; 136 rolls 3.125 % and, only if BOTH teeth are at rest,
        // disarms them and hands on to 131.
        if self.mouth.step() {
            let next = if self.mouth.seq == MOUTH_OPEN {
                for t in &mut self.teeth {
                    t.enabled = true;
                }
                MOUTH_REST
            } else if self.mouth.seq == MOUTH_REST {
                if (ctx.rng.next() & 0x1FF) < MOUTH_GATE
                    && self.teeth.iter().enumerate().all(|(i, t)| t.part.seq == TOOTH_RING[i][0])
                {
                    for t in &mut self.teeth {
                        t.enabled = false;
                    }
                    MOUTH_OPEN
                } else {
                    MOUTH_REST
                }
            } else {
                eprintln!("ShockClocks: CSpriteMouth state {} out of table, continuing",
                          self.mouth.seq);
                MOUTH_REST
            };
            self.mouth.cue(pack, BASE_RH, next);
        }

        // ---- CSpriteTooth × 2 ----
        for i in 0..2 {
            if !self.teeth[i].enabled {
                // vtbl+0x140(0) stops the sequence; fn_5554's gate then reads
                // 0 and the ring cannot advance.
                continue;
            }
            if self.teeth[i].part.step() {
                let ring = TOOTH_RING[i];
                let cur = self.teeth[i].part.seq;
                let next = match ring.iter().position(|&x| x == cur) {
                    Some(0) => {
                        if (ctx.rng.next() & 0x7FF) < RING_GATE { ring[1] } else { ring[0] }
                    }
                    Some(k) => ring[(k + 1) % 4],
                    None => {
                        eprintln!("ShockClocks: CSpriteTooth state {cur} out of table, continuing");
                        ring[0]
                    }
                };
                self.teeth[i].part.cue(pack, BASE_RH, next);
            }
        }

        // ---- CSpriteEar × 2 ----
        for i in 0..2 {
            let ring = EAR_RING[i];
            // `fn_0CE8` (vtbl+0x144, @0x0CE4) runs every frame: the slide's
            // velocity accumulates into the draw offset.
            self.ears[i].slide_x += self.ears[i].vel;
            // the burst (fn_0A74 @0x0A70): states 45 / 89, counter 21
            if self.ears[i].part.seq == ring[1] && self.ears[i].part.counter == 21 {
                ctx.sounds.push(SND_BURST); // shared snd 30011, id 0x753B
            }
            if !self.ears[i].part.step() {
                continue;
            }
            let cur = self.ears[i].part.seq;
            let next = match ring.iter().position(|&x| x == cur) {
                Some(0) => {
                    if (ctx.rng.next() & 0x7FF) < RING_GATE { ring[1] } else { ring[0] }
                }
                Some(1) => ring[2],
                Some(2) => {
                    // `fn_0C14` @0x0C10 — one slide step per ground frame.
                    self.ears[i].vel += EAR_SLIDE_STEP;
                    if self.ears[i].slide_x + self.ears[i].vel > Self::EAR_SLIDE_LIMIT {
                        self.ears[i].vel = 0;
                        self.ears[i].slide_x = 0;
                        ring[3]
                    } else {
                        ring[2]
                    }
                }
                Some(_) => ring[0],
                None => {
                    eprintln!("ShockClocks: CSpriteEar state {cur} out of table, continuing");
                    ring[0]
                }
            };
            self.ears[i].part.cue(pack, BASE_RH, next);
        }

        // ---- the 18 drip parts ----
        for (i, (start, count)) in RH_DRIPS.iter().enumerate() {
            let d = &mut self.drips[i];
            if d.playing {
                if d.step() {
                    d.idle();
                }
            } else if ctx.rng.pct(2048) < DRIP_GATE {
                d.cue_run(*start, *count);
                ctx.sounds.push(SND_DRIP);
            }
        }
    }

    fn draw(&self, pack: &Pack, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        // Composer creation order = z-order. The two ears go in FIRST
        // (@0x022E/@0x0282, two 0xAA actors) and the head over them: their
        // art is fully opaque with a black surround, so drawing an ear last
        // punches a black box through the temple.
        for e in &self.ears {
            e.part.draw(pack, BASE_RH, ax + e.slide_x, ay, out);
        }
        draw_compound(pack, BASE_RH, RH_HEAD, ax, ay, out);
        draw_compound(pack, BASE_RH, RH_GUTS, ax, ay, out);
        self.mouth.draw(pack, BASE_RH, ax, ay, out);
        for d in &self.drips {
            d.draw(pack, BASE_RH, ax, ay, out);
        }
        self.eye.draw(pack, BASE_RH, ax, ay, out);
        for t in &self.teeth {
            t.part.draw(pack, BASE_RH, ax, ay, out);
        }
        // the dial's hands, anchored on compound 411's marker
        draw_hands(&RH_DIAL, &self.hands, ax, ay, out);
    }
}


// ---------------------------------------------------------------------------
// Shocked Monkey (menu 3; RLEP 1000 chain) — §7

/// **This is the shock trigger.** The `rand % 20` idle table at
/// `DATA129+0x4DE` (`@0x1B5E–0x1B76`), read straight out of the binary on
/// 2026-09-12:
///
/// ```text
/// 146 154 162 146 154 162 146 154 162 146 154 162 146 154 162 146 154 162 225 205
/// ```
///
/// ERRATA vs §7/§12.3, which prints it as `{146,154,162}×6 + {146,154}` and
/// files the odd tail under "ORIGINAL QUIRK — a lazy hand-rolled weighted
/// pick". The tail is not 146/154: it is **225 and 205**, the two startle
/// poses. So every idle gesture the monkey finishes carries a flat **2/20 =
/// 10 %** chance of starting a shock on its own, and there is no metronome
/// anywhere in the FSM. §12.1's "50-second shock cadence" — and the port's
/// `MK_SHOCK_MS` fit to the capture's mean — were both filling a hole that was
/// never there.
///
/// CONTRADICTION SETTLED 2026-09-13 — and the losing reading's origin is now
/// known. This header carried both tables at once; a fresh dump of
/// `ripped/shock-clocks/Shock Clocks_DATA_129_DynaTClocks1.bin` shows why.
/// The three monkey tables **overlap** in one 44-byte run:
///
/// ```text
/// 0x04D4  00E1 00CD                     <- startle pair,  {225, 205}   (rand%2)
/// 0x04D8  0092 009A 00A2                <- idle triple,   {146,154,162} (rand%3)
/// 0x04DE  0092 009A 00A2 … 00A2 00E1 00CD  <- the rand%20 table, 20 words
/// 0x0502  00E1 00CD 0105 00F5 0115 0115 011A   <- the gag chain
/// ```
///
/// Read the 20 words from **0x04D8** instead of 0x04DE — three words, six
/// bytes early — and you get `{146,154,162}×6 + {146,154}` exactly, because
/// the run of triples ends at 0x0500 and 0x04FC/0x04FE are the last 146/154
/// inside it. That is §7/§12.3's table, and it is an off-by-six-bytes base
/// address, not a different table. From the cited 0x04DE the last two words
/// are 0x0502/0x0504 = **225, 205**. Pinned by `monkey_idle20_is_the_0x4de_
/// window_not_the_0x4d8_one`.
///
/// GOLDEN cross-check (`shock-monkey.mp4`): idle gestures are 7/7/42 frames
/// (146/154/162) at 120 ms, mean 0.98 s once the `rand%3` restart pick is
/// folded in, so a 1/10 exit is ~9.8 s of idling per gag plus the suppression
/// window and the gag itself — against the capture's measured 4.4 / 7.5 /
/// 16.3 / 26.4 s, mean 13.6 s. A metronome cannot produce that spread.
/// The four sibling pick tables the same run of `DATA 129` holds — see
/// `MONKEY_IDLE20` for the overlap that made the spec misread the 20-word one.
/// `+0x4D4` startle pair, `+0x4D8` idle triple, `+0x506` zap pair,
/// `+0x50A` twitch triple.
const MK_STARTLE: [u32; 2] = [225, 205];
const MK_IDLE3: [u32; 3] = [146, 154, 162];
const MK_ZAP: [u32; 2] = [261, 245];
const MK_TWITCH: [u32; 3] = [277, 277, 282];
/// The run every Init actually starts on. `fn_17B8` (@`0x17B4`) looks like it
/// cues 277 — 0x0115 is pushed @`0x17D6` — but that push feeds the
/// sequence's `GetBounds` (`vtbl+0x70`, @`0x17EC`) to size the drift rect.
/// The SetRun three instructions later (@`0x1804`–`0x1818`) pushes
/// **`[A0+0xA2]`**, the state the ctor stored: `0x92` = **146**. Same shape in
/// every Init in the module — Flasher `0x0F58` pushes `+0xA2` = 1 after a
/// GetBounds on 51, the eye `0x223E` after one on 20 — so none of the
/// "bounds" ids are run ids.
const MK_INIT_RUN: u32 = 146;

const MONKEY_IDLE20: [u32; 20] = [
    146, 154, 162, 146, 154, 162, 146, 154, 162, 146, 154, 162, 146, 154, 162, 146, 154,
    162, 225, 205,
];

/// Chatter (seq, counter) table §7, transcribed from the dispatcher itself
/// (@`0x192A`–`0x19F8`, CODE 129) rather than from §7's prose.
///
/// **CORRECTED 2026-09-13 — the whole table was shifted one gesture.** The
/// dispatcher is a chain of compares of `+0xA2` against sequence literals,
/// and the literals are hex: `0x9A` = **154** takes the single "counter
/// equals 2" arm (@`0x1960`)
/// and `0xA2` = **162** takes the six-counter arm (@`0x1970`). **146 (`0x92`)
/// is not in the dispatcher at all — the hanging idle never chirps.** The port
/// had 146 → [2] and 154 → [5,8,18,24,30,33], i.e. every row moved up one.
///
/// That mattered, because seq 154 is only **7** frames long: counters 8, 18,
/// 24, 30 and 33 were unreachable and five of the module's six busiest chirps
/// could never fire. Seq 162 is **42** frames, which is exactly the room the
/// table needs.
///
/// GOLDEN (`emu/captures/qemu/shock-clocks.wav`, 2026-09-13, Shocked Monkey,
/// 119.12 s, 84 chirps): the chirps arrive in six-chirp BURSTS whose internal
/// gaps are the counter deltas 3 / 10 / 6 / 6 / 3 times `MK_FRAME_MS`, i.e.
/// 0.36 / 1.20 / 0.72 / 0.72 / 0.36 s —
///
/// | burst | wav t of the six chirps | gaps (s) |
/// |---|---|---|
/// | 1 | 3.18 3.54 4.62 5.35 5.89 6.30 | 0.36 1.07 0.74 0.54 0.42 |
/// | 2 | 71.68 72.06 72.97 73.67 74.34 74.62 | 0.38 0.91 0.70 0.67 0.28 |
/// | 3 | 86.49 86.79 87.74 88.35 88.99 89.18 | 0.30 0.95 0.61 0.64 0.19 |
///
/// — a 3.0–3.4 s span that cannot fit inside a 7-frame (0.84 s) gesture and
/// fits comfortably inside a 42-frame (5.04 s) one. Pinned by
/// `monkey_chatter_table_is_the_0x1906_dispatcher`.
fn chatter_counters(seq: u32) -> &'static [u32] {
    match seq {
        // 146 (0x92): no arm in the dispatcher — the hanging idle is silent.
        154 => &[2], // 0x9A @0x1960
        162 => &[5, 8, 18, 24, 30, 33], // 0xA2 @0x1970
        225 => &[5, 9, 11], // 0xE1 @0x199C
        205 => &[5, 11], // 0xCD @0x19B6
        261 => &[4, 8, 10, 14], // 0x105 @0x19CA
        245 => &[4, 10], // 0xF5 @0x19EA
        _ => &[],
    }
}

/// The machine dressing spec §2 called "knobs/gauge pieces (art 80–88)" and
/// "tiny wire ends". They are the machine's ELECTRIC ARCS: five 3-frame
/// blue-white discharges welded to fixed points on the chassis, plus three
/// long runs of 5×4 sparks along its foot. Their packed placement (`bx + dx`,
/// `by + dy`) puts them at
///
/// | run | frames | at | what the golden shows |
/// |---|---|---|---|
/// | 39 | 3 | (55, 112) | left flank, mid |
/// | 43 | 3 | (263, 87) | right flank, where the coil cable leaves for the monkey's helmet |
/// | 47 | 3 | (55, 134) | left flank, low |
/// | 51 | 3 | (291, 158) | right flank, low |
/// | 55 | 3 | (55, 219) | bottom left corner, over the cable gland |
/// | 59 / 84 / 108 | 24 / 23 / 37 | (109/134/159, 245) | sparks under the chassis |
///
/// GOLDEN: `g_06` is explained to a 0.26 % residual by machine + LCD + pose
/// 146 **plus compound 55**, and `g_09` by the same plus **compound 43** —
/// i.e. one arc lit in two of the four monkey frames, with the monkey idle
/// both times, so the arcing is ambient and not gated on the zap (`g_07`
/// catches the actual shock and has no arc at all).
const MK_ARCS: [u32; 8] = [39, 43, 47, 51, 55, 59, 84, 108];

/// **The five arcs are what plays shared `snd 30002 zap_spark`** — found
/// 2026-09-13, disasm + golden audio, and it is the module's missing cue.
///
/// The monkey's scene composer (CODE 129 `@0x3D2A`–`@0x4340`) builds every
/// dressing part with the same eight-argument call, whose second word is a
/// snd id. Read the five arc blocks and the three spark blocks side by side:
///
/// | part | compound push | snd word | site |
/// |---|---|---|---|
/// | spark run 59 | `59` @`0x3F2C` | `-1` @`0x3F38` | — |
/// | spark run 84 | @`0x3FAC` | `-1` @`0x3FB8` | — |
/// | spark run 108 | @`0x402C` | `-1` @`0x4038` | — |
/// | arc 39 | @`0x40AC` | **`0x7532`** | @`0x40B6` |
/// | arc 43 | @`0x412C` | **`0x7532`** | @`0x4136` |
/// | arc 47 | @`0x41AC` | **`0x7532`** | @`0x41B6` |
/// | arc 51 | @`0x422C` | **`0x7532`** | @`0x4236` |
/// | arc 55 | @`0x42AC` | **`0x7532`** | @`0x42B6` |
///
/// `0x7532` = 30002. It is the identical registration the Rotting Head's 18
/// drip groups use for `0x7535` = 30005 (`SND_DRIP`, CODE 130 @`0x05E6` …),
/// which the port already plays — so an arm arms the sound with the run, and
/// the three spark runs, armed with −1, are **silent**.
///
/// GOLDEN AUDIO (`emu/captures/qemu/shock-clocks.wav`, Shocked Monkey,
/// 119.12 s): 30002 correlates **40** times at r = 0.62–0.81 against the
/// expanded rip, never below 0.6 elsewhere, and is not a sub-match of
/// anything in the module's own bank (vs Chirp1 0.058, Chirp4 0.046,
/// Shock1 0.032). Firings: 7.58 (f152), 10.24 (f205), 14.03 (f281), 15.79
/// (f316), 17.59 (f352) … 113.90 (f2278). The r ceiling of 0.81 is the
/// 0.98 s sample being cut short on the single SndChannel by the next chirp.
///
/// GOLDEN VIDEO (`shock-clocks.mp4`, 2400 frames at 20 fps): registering each
/// frame against the last (the assembly glides 1 px / 100 ms) and diffing in
/// assembly coordinates lights up exactly the five lightning-bolt shapes plus
/// the spark strip, and their onsets track the audio one-for-one — e.g. wav
/// 10.24 → arc at f209, 14.03 → f287, 15.79 → f319, 17.59 → f346, 19.91 →
/// f403, 33.03 → f667, 36.13 → f729, 60.54 → f1209, 95.77 → f1919. 27 of the
/// 40 zaps sit within a second of a measured arc onset in the four arc boxes
/// that can be isolated from the monkey; the rest are the fifth arc.
const SND_ZAP_SPARK: u32 = 30002;

/// **The machine dressing is one Library-side class, and its DoDrawFrame is
/// now transcribed** (2026-09-29). All eight parts — the five arcs and the
/// three spark runs — are `new`ed as the same 0xA8-byte sprite: ctor
/// `fn_6E2C` (@`0x6E2C`), init `fn_6EEC` (@`0x6EEC`), DoDrawFrame
/// `fn_6F8A` (@`0x6F8A`, vtable `g1456` +0x134 — decoded from
/// `A5_globals.bin`), start `fn_7082` (@`0x7082`). What the composer's
/// eight-word init pushes (@`0x3F2C`… / @`0x40AC`…) lands in:
///
/// | field | arcs 39/43/47/51/55 | sparks 59/84/108 | meaning |
/// |---|---|---|---|
/// | `+0x9E` | run | run | the compound run |
/// | `+0xA0` | **0** | **1** | LOOP flag: never hide, restart at run end |
/// | `+0xA2` | `0x7532` | `-1` | snd queued by `fn_7082`, `-1` = silent |
/// | `+0xA6` | **1** | 0 | the snd's PRIORITY word in the `fn_5898` slot |
///
/// and `fn_6F8A` is, per frame: hidden → (unless looping) `Random15() %
/// 0x400 < 8` else nothing (@`0x6FF2`–`0x7008`); on a hit `fn_7082` (SetRun,
/// re-pin, Show, queue the snd) then advance. Shown → if the run-end flag
/// `+0x46` is up, a one-shot HIDES and returns, a looper restarts through
/// `fn_7082`; then advance.
///
/// SUPERSEDES `ARC_GATE = 17` of 2048 (fitted to 40 zaps in 119 s) and
/// `SPARK_GATE = 64` of 2048 (invented): the arc gate is **8 / 1024**, and the
/// three spark runs are never gated at all — `+0xA0 = 1` loops them for the
/// life of the scene. GOLDEN (`shock-monkey-fullscreen.mp4`, 429 frames
/// sampled across 300 s): each of the three spark tracks under the chassis
/// holds a lit 5-px dot in **100 %** of frames, moving along its track
/// frame to frame; the gated port had each idle ~57 % of the time.
///
/// The parts also run on their OWN CueAnim record, delay **100**
/// (100 loaded @`0x6E82` in the ctor), not on the monkey's 120/60 — see
/// `DRESS_FRAME_MS`.
const DRESS_GATE: u32 = 8;
/// `Random15() % DRESS_GATE_MOD < DRESS_GATE` (the divide by 0x400 @`0x6FFE`).
const DRESS_GATE_MOD: u32 = 0x400;
/// The dressing parts' own CueAnim delay (`fn_6E2C` @`0x6E82`).
const DRESS_FRAME_MS: u64 = 100;

/// `(loops, snd)` for one dressing run — the `+0xA0` / `+0xA2` words the
/// composer pushes. See `SND_ZAP_SPARK` for the sites.
fn dressing_args(run: u32) -> (bool, Option<u32>) {
    match run {
        39 | 43 | 47 | 51 | 55 => (false, Some(SND_ZAP_SPARK)),
        _ => (true, None),
    }
}
/// The bar the gorilla hangs from (124×20 at bx=312, by=174; the compound the
/// monkey glue fetches as `0xF3` @`0x3872`/`0x3EA6` when it measures the
/// assembly). The idle poses paint it into their own art, but the zap poses
/// (245/261/277) drop it, so it has to be drawn underneath: GOLDEN `g_07`
/// (pose 261) is missing exactly this bar and adding it takes the residual
/// from 2.11 % to 1.49 %.
const MK_BAR: u32 = 243;

/// How long the shock window runs before the gag fires again.
///
/// §7/§12.1 read `+0xB0 = RandomSeedFromTicks() + 0xBB8` (@`0x1D7C`) as 3000
/// 60 Hz ticks = **50 s**, and the port kept that as an ORIGINAL BUG. GOLDEN
/// `emu/captures/shock-monkey.mp4` (2026-09-12) falsifies the 50 s by a factor
/// of ~3.7: four complete gags in 90 s, startle poses entered at t = 7.8 /
/// 28.7 / 59.5 / 72.1 s with a fifth starting at 88.8 s, and **4.4 / 7.5 /
/// 16.3 / 26.4 s** from leaving 282 (the white-flash edge, see `Lcd`) to the
/// next startle — mean **13.6 s**. The wall clock only crosses one minute in
/// that capture (12:44 → 12:45 at t ≈ 45 s) and no hour, so none of the four
/// is the hour branch.
///
/// Two things are wrong with the spec reading, and this constant only repairs
/// the second:
///
/// 1. `+0xB0` is cleared by the constructor (`fn_16BA`) and re-armed only on
///    leaving 282 — which is only reachable *through* a shock. Under both the
///    port's "fire when the deadline is reached" and §7's literal "fire while
///    the deadline is still ahead", a freshly built monkey therefore never
///    shocks at all until the hour rolls over. The capture's monkey shocks
///    every ~14 s. So the window is armed at construction here too.
/// 2. The interval is the capture's mean, not 3000 ticks.
///
/// RESOLVED 2026-09-12 — the re-read this note asked for happened, and both
/// halves fell out of the binary:
///
/// 1. `DATA129+0x4DE` **does** carry 225/205 in its last two slots, so the
///    shock is a per-idle-pick roll at 1/10 (see `MONKEY_IDLE20`). `+0xB0` is
///    not the trigger at all; it is a SUPPRESSION window — while it is
///    pending, `ChooseNextSequence` takes the `rand%3` arm at `@0x1B3E` and
///    the monkey cannot startle. That is also why a cold-start monkey shocks
///    fine: the ctor clearing `+0xB0` leaves him *unsuppressed*.
/// 2. The 3000 is real and the unit is **milliseconds**, not 60 Hz ticks.
///    `+0xB0 = fn_4724() + 0xBB8`, and `Resource.fn_4724` (`@0x4724`, LIB40
///    Resource) is not `TickCount` — it is `TickCount` scrambled through
///    `((lo16 × 0xA006) >> 16) + hi16 × 0xA006 + (tc << 4)`, which for any
///    TickCount under 18 minutes is exactly `tc × 16.625`. One 60 Hz tick is
///    16.667 ms, so `fn_4724` counts milliseconds to within 0.25 %. §12.1's
///    "3000 ticks = 50 s" is therefore **3000 ms = 3.0 s**, a factor of
///    16.6 out — the single biggest constant error in the spec.
///
/// Mean interval under the repaired reading: 3.0 s suppressed + ~9.8 s of
/// 1/10 idling + ~2 s of gag ≈ 15 s, against the capture's 13.6 s mean over
/// four samples spread 4.4…26.4 s. The old fitted `MK_SHOCK_MS = 13_600`
/// metronome is gone.
/// (the constant itself lives with the LCD model, above.)

struct ShockedMonkey {
    pose: Part,
    /// `+0x40`: the pose sprite's position — **the CENTRE of the current
    /// frame** in bank space (port-plan §2), drawn relative to the clock
    /// anchor (which carries the drift). Moved by the `fn3DDC` centre link on
    /// every advance, by `fn3F2E` on every hand-off, and then RE-PINNED by
    /// `fn_56DA` at the end of every hand-off — see `set_run`.
    pos: (i32, i32),
    /// `+0x3C` bit 0: the pose is drawn mirrored. Cleared by Init (`fn_17B4`
    /// @17B4, after its SetRun) and toggled only by `fn3F2E` — which, with
    /// this pack, never happens (see `set_run`).
    flip: bool,
    /// The hang bar, `+0xAC`. `fn_3C48` (@`0x3C48`, the clock-type-3 builder)
    /// `new`s a 0x3E-byte sequence on compound **243**, inits it with
    /// `fn_6A74(bar, 243, scene)` (@`0x3EB4`) and stores it in the monkey's
    /// `+0xAC` (@`0x3EC8`). The monkey's chooser then pokes it: leaving
    /// 225/205 calls `vtbl+0x30` (restart, @`0x1BC0`), leaving 282 calls
    /// `vtbl+0x2C` (stop, @`0x1D4E`).
    ///
    /// ERRATA: the 2026-09-12 port read `+0xAC` as the monkey's MINUTE flag
    /// (the generic layout's meaning, and CSpriteRightEye's) and wrote a
    /// `minute_flag` that restarted the startle pose. `fn_185E` never sets a
    /// minute flag — its latch (@`0x18DA`) only compares the HOUR — so the
    /// monkey has no minute event at all, and `+0xAC` here is a pointer.
    bar: bool,
    arcs: Vec<Part>,
    /// `+0x46` of each dressing part: the advance ran off the end of the run
    /// on the previous frame (the part still shows its last frame).
    arc_ended: Vec<bool>,
    hour_flag: bool,   // +0xA6
    last_hour: u8,     // +0xA8
    cuckoo_count: u32, // +0xAA
    /// `+0xB0`: the white-LCD / shock-suppression window's expiry in ms — see
    /// `MK_SHOCK_WINDOW_MS`. `None` = clear, which is both "not suppressed"
    /// and "no recolour pending".
    deadline: Option<u64>,
    /// Wall clock latched each tick and rendered into the LCD cells.
    hms: (u8, u8, u8),
    /// The LCD palette (`Lcd`), stepped once per monkey frame by `fn_2C9A`.
    lcd: Lcd,
    /// The monkey's own CueAnim delay: `MK_FRAME_MS`, or `MK_RECOVER_MS`
    /// while the `+0xB0` window runs.
    delay: u64,
}

impl ShockedMonkey {
    /// `fn_16BA` (@`0x16B6`) primes `+0xA8` from `GetLocalTime` — so a fresh
    /// monkey does NOT see a spurious hour change on its first frame. The
    /// 2026-09-12 port left it 0 and every 12:00 test start fired the hour gag.
    fn new(pack: &Pack, hour: u8) -> Self {
        let mut pose = Part::new();
        pose.cue(pack, BASE_MONKEY, MK_INIT_RUN);
        ShockedMonkey {
            // Init's SetRun has no current frame (`+0x3A` == 0) so nothing
            // registers, and the StartStateSequence re-pin puts the run where
            // the composite authors it.
            pos: mk_centre(pack, MK_INIT_RUN),
            flip: false,
            pose,
            // `fn_6A74` adds the bar to the scene at build time and nothing
            // hides it until the first gag, so it starts drawn.
            bar: true,
            arcs: (0..MK_ARCS.len()).map(|_| Part::new()).collect(),
            arc_ended: vec![false; MK_ARCS.len()],
            hour_flag: false,
            last_hour: hour,
            cuckoo_count: 0,
            // `fn_16BA` clears `+0xB0`: a fresh monkey is UNSUPPRESSED, so
            // the 1/10 idle roll can shock him straight away.
            deadline: None,
            hms: (12, 0, 0),
            lcd: Lcd::Rest,
            delay: MK_FRAME_MS, // fn_16BA's CueAnim @0x1730
        }
    }

    /// `CSpriteMonkey::ChooseNextSequence` — **`fn_1A7A`** @`0x1A76`, CODE 129,
    /// arm for arm. `+0xA2` is the state; every pick is `Random15()` indexing
    /// one of four `DATA 129` tables:
    ///
    /// | table | at | words |
    /// |---|---|---|
    /// | startle pair, `% 2` | `+0x4D4` | `{225, 205}` |
    /// | idle triple, `% 3` | `+0x4D8` | `{146, 154, 162}` |
    /// | idle twenty, `% 20` | `+0x4DE` | `{146,154,162}×6 + {225, 205}` |
    /// | zap pair, `% 2` | `+0x506` | `{261, 245}` |
    /// | twitch triple, `% 3` | `+0x50A` | `{277, 277, 282}` |
    ///
    /// | state | @ | next |
    /// |---|---|---|
    /// | 146 / 154 / 162 | `0x1AA0` | hour → startle pair; else suppressed → idle triple; else idle twenty. Landing on 225/205 ARMS the LCD ramp (`fn_2C3E`, @`0x1BA4`) |
    /// | 225 / 205 | `0x1BB0` | restart the bar (`+0xAC` vtbl+0x30) → zap pair |
    /// | 261 / 245 | `0x1BE8` | `rand&1` → snd 1005 / 1004, → 277 |
    /// | 277 | `0x1C44` | twitch triple; if it stays on 277, another `rand&1` shock sound |
    /// | 282 | `0x1CC0` | **if the hour still owes cuckoos, straight back to the zap pair** — the monkey's hour gag is repeated ZAPS. Otherwise stop the bar, pick an idle, stamp `+0xB0 = now + 3000`, disarm the ramp and flash the LCD white |
    /// | other | `0x1DD0` | assert stub, print and continue (§12.4) |
    ///
    /// ERRATA vs the 2026-09-12 port, which had (a) no re-zap — it counted
    /// cuckoos in a per-tick block outside the chooser, so the hour gag was
    /// invisible; (b) the suppression window armed on EVERY exit from 282,
    /// including the ones that owe another cuckoo; (c) a `minute_flag`
    /// restart on 225/205 (see `bar`).
    fn choose_next(&mut self, ctx: &mut Ctx, now: u64) -> u32 {
        match self.pose.seq {
            146 | 154 | 162 => {
                let next = if self.hour_flag {
                    MK_STARTLE[ctx.rng.pct(2) as usize]
                } else if self.deadline.is_some() {
                    MK_IDLE3[ctx.rng.pct(3) as usize]
                } else {
                    MONKEY_IDLE20[ctx.rng.pct(20) as usize]
                };
                if matches!(next, 225 | 205) {
                    // @0x1BA4: fn_2C3E(seq, DATA129+0x4C6, 1) — arm the ramp
                    // at the packed lit colour.
                    self.lcd = Lcd::Ramp(LCD_RAMP_ARM);
                }
                next
            }
            225 | 205 => {
                self.bar = true; // +0xAC vtbl+0x30 @0x1BC0
                MK_ZAP[ctx.rng.pct(2) as usize]
            }
            261 | 245 => {
                // @0x1BF6: rand&1 → 0x3ED (1005) else 0x3EC (1004)
                ctx.sounds.push(if ctx.rng.next() & 1 == 0 { SND_SHOCK2 } else { SND_SHOCK1 });
                277
            }
            277 => {
                let next = MK_TWITCH[ctx.rng.pct(3) as usize];
                if next == 277 {
                    // @0x1C8C: the repeat twitch fires another shock sound
                    ctx.sounds.push(if ctx.rng.next() & 1 == 0 { SND_SHOCK2 } else { SND_SHOCK1 });
                }
                next
            }
            282 => {
                if self.hour_flag {
                    // @0x1CC4: n = hour % 12, 12 when the remainder is 0
                    let mut n = u32::from(self.last_hour) % 12;
                    if n == 0 {
                        n = 12;
                    }
                    self.cuckoo_count += 1;
                    if self.cuckoo_count < n {
                        // …and go round the gag again. No bar stop, no
                        // window, no recolour — the arm below is skipped.
                        return MK_ZAP[ctx.rng.pct(2) as usize];
                    }
                    self.hour_flag = false;
                }
                self.bar = false; // +0xAC vtbl+0x2C @0x1D4E
                let next = MK_IDLE3[ctx.rng.pct(3) as usize];
                // @0x1D7C: +0xB0 = fn_4724() + 0xBB8
                self.deadline = Some(now + MK_SHOCK_WINDOW_MS);
                // @0x1DC2 fn_2C3E(seq,0,0) then @0x1DCC fn_2DBA(seq, +0x4C6):
                // disarm the ramp, then SetColorRemap(black, white).
                self.lcd = Lcd::White;
                // @0x1DA2: re-cue this sprite at delay 60 for the window.
                self.delay = MK_RECOVER_MS;
                next
            }
            other => {
                eprintln!("ShockClocks: CSpriteMonkey state {other} out of table, continuing");
                MK_IDLE3[0]
            }
        }
    }

    /// `CSpriteMonkey::DoDrawFrame` — **`fn_185E`** @`0x185E`, CODE 129.
    /// Head, then the time latch, then the chatter dispatcher, then the base
    /// tick `fn_5648`.
    fn tick(&mut self, ctx: &mut Ctx, pack: &Pack, hour: u8, now: u64) {
        self.hms = ctx.local_hms;

        // ---- 1. the window (@0x186E–0x18D4). When it expires the module
        // calls fn_2D74(seq, +0x4C6) = SetColorRemap(white, white), an
        // IDENTITY remap — the recolour comes off and the packed pale yellow
        // returns — re-cues at delay 120 and RETURNS, skipping the rest of the
        // frame entirely (@0x18D4, returning 0). ----
        if let Some(d) = self.deadline {
            if now >= d {
                self.deadline = None;
                self.lcd = Lcd::Rest;
                // @0x18C2: back on the ctor's 120.
                self.delay = MK_FRAME_MS;
                return;
            }
        }

        // ---- 2. the hour latch (@0x18DA). There is no minute branch. ----
        if hour != self.last_hour {
            self.last_hour = hour;
            self.hour_flag = true;
            self.cuckoo_count = 0;
        }

        // ---- 3. chatter (@0x1906–0x1A32): uniform Random15()%4 chirp ----
        if chatter_counters(self.pose.seq).contains(&self.pose.counter) {
            ctx.sounds.push(SND_CHIRP1 + ctx.rng.pct(4));
        }

        // ---- 4. fn_2C9A (@0x1A5C): step the LCD ramp once per frame ----
        self.lcd = self.lcd.step();

        // ---- 5. base tick fn_5648 → ChooseNextSequence on run end ----
        let before = self.pose.seq + self.pose.frame;
        if self.pose.step() {
            let next = self.choose_next(ctx, now);
            // fn_1A7A's tail (@0x1DD8) → vtbl+0x148 = fn_572E
            self.set_run(pack, next);
        } else {
            // `+0x84` = `L132 fn0316` → `+0xD0` = `fn0DF4`: the centre link
            // (`L135 fn3DDC`), x negated while mirrored. The one-shot's
            // finishing step (above) applies none.
            let after = self.pose.seq + self.pose.frame;
            // Pinned model: `LinkModel::CENTRE_DIFF`, as Father Time's.
            let (a, b) = (frame_box(pack, BASE_MONKEY, before), frame_box(pack, BASE_MONKEY, after));
            let (dx, dy) = l135::link(&a, &b, self.flip, LinkModel::CENTRE_DIFF);
            self.pos.0 += dx;
            self.pos.1 += dy;
        }
    }

    /// One frame of every dressing part — `fn_6F8A` (@`0x6F8A`) per part, on
    /// the parts' own 100 ms CueAnim (`DRESS_FRAME_MS`). See `DRESS_GATE`.
    fn tick_dressing(&mut self, ctx: &mut Ctx, pack: &Pack) {
        for (i, run) in MK_ARCS.iter().enumerate() {
            let (loops, snd) = dressing_args(*run);
            let a = &mut self.arcs[i];
            if !a.playing {
                // @0x6FF2: a one-shot rolls to arm; a looper always starts
                if !loops && ctx.rng.pct(DRESS_GATE_MOD) >= DRESS_GATE {
                    continue;
                }
            } else if !self.arc_ended[i] {
                // shown and mid-run: advance
                self.arc_ended[i] = a.step();
                continue;
            } else if !loops {
                // @0x6FC4: run over, one-shot → Hide and return
                a.idle();
                self.arc_ended[i] = false;
                continue;
            }
            // fn_7082 (@0x7082): SetRun + Show + queue the snd (then the
            // advance lands on the run's first frame, which `cue` shows)
            a.cue(pack, BASE_MONKEY, *run);
            self.arc_ended[i] = false;
            if let Some(s) = snd {
                ctx.sounds.push(s);
            }
        }
    }

    /// `M129_fn108` @572E, StartStateSequence, as `CSpriteMonkey`'s vtable
    /// `M129_g054C` binds it — the LINKED SetRun (`+0x7C` = `L132 fn0204`,
    /// `+0x108` = `fn028A`, `+0xF0` = `fn1186`, `+0xCC` = `fn0D3C`, exactly
    /// Father Time's slots) followed by the base re-pin (`+0x144` =
    /// `M129_fn107` @56DA; the monkey does NOT override it, Father Time does).
    ///
    /// 1. `fn1186`: the marker is `id − 1` when that record exists. Series
    ///    1000 has none before any of the nine pose runs (145, 153, 161, 204,
    ///    224, 244, 260, 276, 281 are all holes), so it is the run's own first
    ///    frame.
    /// 2. `fn0D3C` → `L135 fn3F2E`: register the outgoing frame on the marker
    ///    through their first shared part (`register_shared_part`) — `pos`
    ///    moves, and the mirror bit toggles if that part's flip bit differs.
    /// 3. `fn_56DA`: ask the SCENE (`module+0x320`, the monkey's `g0CFA`) for
    ///    the run's anchor — it pushes `+0xA2`, the id just set (@56E6), into
    ///    the scene's `+0x94` = `M129_fn72` @397A, which resolves it against
    ///    compound `0x11D` = 285 (the plate the whole composite is authored
    ///    on) through the scene's `+0x78` — and `SetPosition` (`+0x88` =
    ///    `L132 fn04C8`, an absolute store to `+0x40/+0x42`) there. So step
    ///    2's move is OVERWRITTEN on every hand-off and each run starts at its
    ///    authored spot in the composite; only step 2's mirror toggle
    ///    survives.
    ///    GAP(fn_397A anchor math): the `p_Sequence_PointRectOp` call and the
    ///    inflate-by-8 centring around it are not transcribed; the anchor is
    ///    taken as the incoming frame's authored bank centre, which is what
    ///    the capture measures (below).
    ///
    /// What the registration asks for, per hand-off the machine can make
    /// (`fn3F2E` on the pack): idle → idle/startle 0 or (0,+1) on the head
    /// (art 6); startle → zap **(+12,+1)**; zap → 277 and 277 → 282 **no
    /// shared part**; 277 → 277 (−2,−2) on art 8; 282 → idle **(−12,−2)**;
    /// 282 → zap 0. **No hand-off toggles the mirror** — every shared part
    /// carries flip bit 0 on both sides (series 1000's only flipped parts are
    /// the arms of 245/277/282, never the first shared one).
    ///
    /// GOLDEN (`shock-monkey-fullscreen.mp4`, all eight gags, masked-SSD
    /// template match of every series-1000 pose compound, plain AND mirrored,
    /// against the machine-relative position): see
    /// `monkey_hand_offs_re_pin_and_never_mirror`. Keeping step 2's move (the
    /// Father Time model) would draw 277 at (−15,−11) from its authored spot
    /// and creep the monkey (−1,−3) per gag; the capture has 277 at (0,0).
    fn set_run(&mut self, pack: &Pack, id: u32) {
        let cur = self.pose.seq + self.pose.frame;
        let marker = l135::marker_of(id as i32, |f| f >= 1 && pack.frame(BASE_MONKEY, f as u32).is_some()) as u32;
        if let Some(r) = register_shared_part(pack, BASE_MONKEY, cur, marker, &mut self.flip) {
            self.pos.0 += r.delta.0;
            self.pos.1 += r.delta.1;
        }
        self.pose.cue(pack, BASE_MONKEY, id);
        // (`fn1210`/`fn0DF4` marker → first is a no-op: marker == first.)
        // fn_56DA: the re-pin.
        self.pos = mk_centre(pack, id);
    }

    fn draw(&self, pack: &Pack, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        // base: compound 1, the 286×184 lab machine (§2) — its LCD window is
        // an empty black hole in the art; the panel + digits fill it.
        draw_seq_frame(pack, BASE_MONKEY, 1, 0, ax, ay, out);
        self.draw_lcd(pack, ax, ay, out);
        draw_compound(pack, BASE_MONKEY, MK_BAR, ax, ay, out);
        // the pose: its frame rect centred on `pos`, mirrored about that
        // rect while `flip` (L135 `fn16BC` / `fn3BD6`)
        if self.pose.playing {
            if let Some(f) = pack.frame(BASE_MONKEY, self.pose.seq + self.pose.frame) {
                out.push(SpriteDraw {
                    flip: self.flip,
                    pal: 0,
                    png: f.png.clone(),
                    x: ax + self.pos.0 - (f.w >> 1),
                    y: ay + self.pos.1 - (f.h >> 1),
                });
            }
        }
        for a in &self.arcs {
            a.draw(pack, BASE_MONKEY, ax, ay, out);
        }
    }

    /// HH:MM in 12-hour form with no leading zero, exactly as the golden
    /// shows it (`g_06` = "1:36" with the hour-tens cell dark), in whatever
    /// palette state the shock gag has the panel in (`Lcd`).
    fn draw_lcd(&self, pack: &Pack, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        let (h, m, s) = self.hms;
        let h12 = match h % 12 {
            0 => 12,
            n => n,
        };
        // The unlit ghost cells, always on and never recoloured — the capture
        // keeps `1 8 : 8 8` at its own dark grey through both the ramp and the
        // white flash; it is the LIT segments that move.
        draw_seq_frame(pack, BASE_MONKEY, LCD_GHOST, 0, ax, ay, out);
        if h12 >= 10 {
            self.draw_cell(pack, LCD_HOUR_TENS, 0, ax, ay, out);
        }
        self.draw_cell(pack, LCD_HOUR_UNITS, u32::from(h12 % 10), ax, ay, out);
        if s % 2 == 0 {
            // 1 Hz, 50 % duty — GOLDEN `shock-monkey.mp4`: the panel's lit
            // pixel count steps 1745 ⇄ 1793 (the colon is 48 px) at f1, 11,
            // 22, 32, 42, 52, 62, 72, 82, 92, 103, … i.e. 10.2 module frames,
            // and it keeps that beat right through the shock's white phase.
            self.draw_cell(pack, LCD_COLON, 0, ax, ay, out);
        }
        self.draw_cell(pack, LCD_MIN_TENS, u32::from(m / 10), ax, ay, out);
        self.draw_cell(pack, LCD_MIN_UNITS, u32::from(m % 10), ax, ay, out);
    }

    /// One lit LCD cell, recoloured for the current palette state.
    fn draw_cell(&self, pack: &Pack, seq: u32, idx: u32, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
        let Some(f) = pack.frame(BASE_MONKEY, seq + idx) else { return };
        out.push(SpriteDraw {
            flip: false,
            pal: 0,
            png: lcd_art(&f.png, self.lcd),
            x: ax + f.bx + f.dx,
            y: ay + f.by + f.dy,
        });
    }
}

// ---------------------------------------------------------------------------
// MySoundPlayer — the module's sound list (`module+0x32C`), transcribed
// 2026-09-29. Every cue in the module goes through it; nothing plays a snd
// directly.
//
// `fn_5898` (@`0x5898`) QUEUES: it takes the first free 12-byte slot of 16
// (free = word `+6` is −1) and stores `{id, word, PRIORITY, delay}`. The
// module's call sites push:
//
// | cue | site | priority |
// |---|---|---|
// | monkey chirps 1000–1003 | `fn_185E` @`0x1A1C`–`0x1A20` | **0** |
// | monkey Shock1/Shock2 1004/1005 | `fn_1A7A` @`0x1C0C` / @`0x1CA2` | **2** |
// | machine arcs' 30002 zap_spark | `fn_7082` via `+0xA6` = 1 (@`0x40BA` …) | **1** |
// | Father Time 1100–1103, 2000 | `fn_0FB2` (@`0x1240`) | 0 |
// | Rotting Head 1200 / 2000 | `fn_22E8` (@`0x2352`–`0x23EC`) | 2 |
// | ear Burst 30011 | `fn_0A74` | 1 |
// | drips 30005 | `fn_7082` via `+0xA6` = 0 | 0 |
//
// (the `word` and `delay` fields are 0 at every site, so `fn_59A0`'s
// repeat-slot loop and its delay countdown never run here and are not
// modelled.)
//
// `fn_59A0` (@`0x59A0`) PLAYS, once per module Idle and BEFORE the Idle pumps
// the CueAnim list (@`0x5362`–`0x53AC`, then `M130 fn23`), so a cue queued by
// a frame is heard on a later Idle. Start passes are throttled: the Idle
// passes `allowStart` only when `now >= module+0x340`, and then sets
// `+0x340 = now + 100` (`SND_START_GATE_MS`). On a start pass:
//
// 1. the highest priority among slots still SOUNDING (`now < +8`) is found;
// 2. among queued, never-started slots the highest priority is picked (ties
//    go to the LAST slot), and it starts only if its priority is >= (1)'s;
//    starting stamps `+8 = now + the clip's length` and pre-empts the channel;
// 3. every other never-started slot, and every finished one, is FREED.
//
// So a cue that cannot start on the first start pass after it was queued is
// thrown away, never delayed. That is what the chirp bed's long silences are:
// a chirp (priority 0) queued while a 0.98 s zap_spark (priority 1) or a
// Shock (2) is sounding is discarded. GOLDEN (`shock-monkey-fullscreen.wav`,
// `snd-correlate.py` r >= 0.9 chirps / >= 0.6 zaps): the 87 zap clips cover
// 85 s of the 294.6 s take and **0** of the 237 chirps start inside one,
// against 68 if the two were independent; zaps start inside other zaps 14
// times (equal priority pre-empts) and inside a Shock 0 times (3.4
// independent). The 2026-09-19 "fn_59A0 busy model" replay that moved the
// chirps only 1.25x -> 1.18x modelled the one-start-per-pass throttle but
// not the priority word.
//
// GAP(SoundRateConvert): the stamped length is `SoundRateConvert` of the
// resource (`@0x5B8E`), not transcribed; taken as the packed clip's length
// (`Pack::sound_ms`), which is what the capture's clip lengths are.
// GAP(Idle cadence): the original's Idle rate is the shell's; the port's is
// `TICK_MS`, so start passes land every 120 ms rather than every ~100–117 ms.

/// `module+0x340 = now + 100` (the Idle @`0x5364`–`0x537A`).
const SND_START_GATE_MS: u64 = 100;
/// `fn_5898`'s list: 16 slots (the bound 0x10 @`0x58BC`).
const SND_SLOTS: usize = 16;

#[derive(Clone, Copy, Debug)]
struct SndSlot {
    id: u32,
    prio: i16,
    /// `+8`: when the clip stops sounding; 0 = queued, never started.
    end: u64,
}

struct SoundPlayer {
    slots: [Option<SndSlot>; SND_SLOTS],
    /// `module+0x340`.
    next_start: u64,
}

impl SoundPlayer {
    fn new() -> Self {
        SoundPlayer { slots: [None; SND_SLOTS], next_start: 0 }
    }

    /// `fn_5898`: first free slot; a full list drops the cue (the `@0x598A`
    /// "list full" message).
    fn queue(&mut self, id: u32, prio: i16) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.is_none()) {
            *slot = Some(SndSlot { id, prio, end: 0 });
        }
    }

    /// One Idle's `fn_59A0` call; started cues go to `out`.
    fn service(&mut self, pack: &Pack, now: u64, out: &mut Vec<u32>) {
        if now < self.next_start {
            // allowStart = 0: nothing observable (no repeat slots here).
            return;
        }
        self.next_start = now + SND_START_GATE_MS;
        // 1. the loudest priority still sounding (starting from 0)
        let sounding = self
            .slots
            .iter()
            .flatten()
            .filter(|s| s.end != 0 && now < s.end)
            .map(|s| s.prio)
            .fold(0i16, i16::max);
        // 2. the best queued slot, ties to the last (the compare is `<=`)
        let mut pick: Option<usize> = None;
        let mut best = 0i16;
        for (i, s) in self.slots.iter().enumerate() {
            if let Some(s) = s {
                if s.end == 0 && best <= s.prio {
                    best = s.prio;
                    pick = Some(i);
                }
            }
        }
        if let Some(i) = pick {
            if sounding <= best {
                let slot = self.slots[i].as_mut().expect("picked slot");
                slot.end = now + pack.sound_ms(slot.id).max(1);
                out.push(slot.id);
            }
        }
        // 3. free the finished and the never-started
        for s in self.slots.iter_mut() {
            if let Some(v) = s {
                if v.end == 0 || v.end <= now {
                    *s = None;
                }
            }
        }
    }
}

/// The priority word each call site pushes into `fn_5898` (table above).
fn cue_priority(clock: u32, id: u32) -> i16 {
    match (clock, id) {
        (3, SND_SHOCK1 | SND_SHOCK2) => 2,
        (3, SND_ZAP_SPARK) => 1,
        (2, SND_CUCKOO_PRELUDE | SND_CUCKOO) => 2,
        (2, SND_BURST) => 1,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------

enum ActiveClock {
    Ft(FatherTime),
    Rh(RottingHead),
    Monkey(ShockedMonkey),
}

pub struct ShockClocks {
    pack: Pack,
    mval_raw: i32, // module+0x31C: raw mVal 1000, 1-based 1..4, default 2
    selection: u32, // module+0x31A: 1..3
    needs_roll: bool,
    built: u32, // selection the active actor was built for
    clock: Option<ActiveClock>,
    drift: i32, // sVal 1001 raw 0..100
    /// §8 drift, the bouncing glide — see `DRIFT_INTERVAL_MS`. `None` until
    /// the first cue fires (or permanently, in the "Still" band).
    /// The pair is the ASSEMBLY's top-left on the 640×480 field, not an offset.
    drift_pos: Option<(i32, i32)>,
    drift_dir: (i32, i32),
    drift_due: u64,
    last_hour: u8,   // +0xA8
    last_minute: u8, // +0xAE
    started: bool,
    /// Compound-frame accumulator: the module grid runs at `TICK_MS`, the
    /// active clock's parts advance one frame per `frame_ms()` (see the
    /// CueAnim table at the top of the file). Seeded at the period so the
    /// very first module tick always produces a frame.
    anim_acc: u64,
    /// Compound frames stepped since the actor was built (test hook for the
    /// rate arithmetic).
    anim_frames: u64,
    /// The monkey machine's dressing parts run on their own CueAnim record
    /// (`DRESS_FRAME_MS`); this is its accumulator.
    dress_acc: u64,
    /// `module+0x32C`, the sound list every cue goes through.
    snd: SoundPlayer,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    Some(Box::new(build(pack)?))
}

fn build(pack: Pack) -> Option<ShockClocks> {
    // all three RLEP chains must be packed
    for base in [BASE_MONKEY, BASE_FT, BASE_RH] {
        if !pack.meta.series.contains_key(&base.to_string()) {
            return None;
        }
    }
    Some(ShockClocks {
        pack,
        // Factory default: Father Time. `mVal 1000` stores 2, but the shell
        // hands the module the menu ITEM index (stored mark − 1) — golden
        // g_02/g_03/g_10, see the module header. Spec §1.1/§12.7 errata.
        mval_raw: 1,
        selection: 1,
        needs_roll: false,
        built: 0,
        clock: None,
        drift: 50,
        drift_pos: None,
        drift_dir: (1, 1),
        drift_due: 0,
        last_hour: 0,
        last_minute: 0,
        started: false,
        anim_acc: FT_FRAME_MS,
        anim_frames: 0,
        dress_acc: DRESS_FRAME_MS,
        snd: SoundPlayer::new(),
    })
}

impl ShockClocks {
    fn build_clock(&mut self, hour: u8, minute: u8) {
        self.built = self.selection;
        self.clock = Some(match self.selection {
            1 => ActiveClock::Ft(FatherTime::new(&self.pack, hour)),
            3 => ActiveClock::Monkey(ShockedMonkey::new(&self.pack, hour)),
            _ => ActiveClock::Rh(RottingHead::new(&self.pack, hour, minute)),
        });
        // seed the actor's last-seen time so the first tick doesn't spam events
        self.last_hour = hour;
        self.last_minute = minute;
        self.anim_acc = self.frame_ms();
        self.anim_frames = 0;
        self.dress_acc = DRESS_FRAME_MS;
    }

    /// The active actor's compound-frame delay, from its constructor's
    /// `CueAnim` (see the table at the top of the file).
    fn frame_ms(&self) -> u64 {
        if let Some(ActiveClock::Monkey(m)) = &self.clock {
            // fn_16BA @0x1730 (120), or the recover cue @0x1DA2 (60)
            return m.delay;
        }
        match self.selection {
            1 => FT_FRAME_MS,  // fn_0DF4 @0x0E80
            3 => MK_FRAME_MS,  // fn_16BA @0x1730
            _ => RH_FRAME_MS,  // fn_2156 @0x21D6
        }
    }

    /// §8 band picker fn_2B16: band = GetControlValue(1)/20, clamp ≤ 4.
    fn drift_band(&self) -> usize {
        (self.drift / 20).clamp(0, 4) as usize
    }

    /// The drifting assembly: `(off_x, off_y, w, h)` — where the assembly rect
    /// `module+0x52` sits relative to the clock's draw anchor, and how big it
    /// is. The drift keeps this rect inside the screen, so it is what sets the
    /// travel limits.
    ///
    /// **`None` for Father Time and the Rotting Head — drift is PER-CONTROL.**
    /// See `step_drift` for the evidence; the short version is that only the
    /// monkey's glue measures an assembly (the `0xF3` = compound 243 fetch at
    /// `@0x3872`/`@0x3EA6`, which is the hang bar — the right edge of the
    /// rect below), and only the monkey is ever seen drifting.
    ///
    /// The monkey's is capture-pinned and exact. `shock-monkey.mp4` reflects
    /// the machine's left edge off x = 0 and x = 244 and its top off y = 2 and
    /// y = 296, and the two spans are `640 − 395 = 245` and `480 − 184 = 296`
    /// wide — so w = **395**, the machine (`bx = 41`, 286 wide) out to the hang
    /// bar's right edge (312 + 124 = 436), and h = **184**, the machine's own
    /// packed height. The original measures the STATIC dressing, not the
    /// gorilla, whose poses reach x = 468. (The y readings sit 1 px low
    /// against the x ones because the machine's top scanline is transparent.)
    fn assembly(&self) -> Option<(i32, i32, i32, i32)> {
        match self.built {
            3 => Some((41, 75, 395, 184)),
            _ => None,
        }
    }

    /// Where the active clock is drawn: the base anchor, or — once drift is
    /// running — wherever the glide has carried the assembly.
    fn anchor(&self) -> (i32, i32) {
        let base = match self.built {
            1 => (FT_AX, FT_AY),
            2 => (102, 100),
            _ => (65, 66),
        };
        match (self.drift_pos, self.assembly()) {
            (Some((x, y)), Some((ox, oy, _, _))) => (x - ox, y - oy),
            _ => base,
        }
    }

    /// §8 drift, rewritten off the disasm (2026-09-12) — see
    /// `DRIFT_INTERVAL_MS` for why the old random-teleport reading was wrong.
    ///
    /// `@0x0118` (the module's `CueAnim` mode-3 callback, re-cued by `fn_2B88`
    /// at `DATA129+0xB68[band]`):
    /// - first fire only (`+0x46`): `fn_0610` @`0x0610` drops the assembly at a
    ///   uniformly random on-screen position (two `Random15()`s, each
    ///   reduced modulo `screen − assembly`, vertical first), then the
    ///   direction is aimed at the screen centre (`@0x0188–0x01CC`) and the
    ///   call returns without moving.
    /// - every later fire: step by the direction, and flip an axis' sign when
    ///   the tentative position leaves the screen rect inset by half the
    ///   assembly (`@0x0254–0x0364`) — a reflection, not a re-roll.
    /// The direction words `+0x4A/+0x4C` are renormalised to ±`+0x48`
    /// (`@0x0262–0x02BA`) and then multiplied by `+0x48` again (`@0x02DA`),
    /// so the step is `amp²` px on BOTH axes — the motion is always 45°.
    ///
    /// GOLDEN (`shock-monkey.mp4`, whole-frame pixel-shift correlation and a
    /// per-frame bbox trace, 900 module frames): the assembly moves **exactly
    /// 1 px in x and 1 px in y every module frame**, all 900 of them, with
    /// `|dx| = |dy| = elapsed frames` at every sampled lag; it reverses x at
    /// f241 (left = 244), f488 (left = 0) and f737 (left = 243), and y at f134
    /// (top = 296), f432 (top = 2) and f730 (top = 296). Independent
    /// half-periods, 244 and 294 frames — a DVD-logo bounce, not a random
    /// walk. The port used to teleport to a fresh random offset every band
    /// interval, which is the "Still" band's behaviour (below) applied to
    /// every band.
    ///
    /// **Band 0 ("Still") is not still.** `fn_2A30` @`0x2ACC`: when the band's
    /// interval word is zero the glide cue is cancelled (`fn_2B88`'s mode-4
    /// arm) and instead the module re-rolls the position through `fn_0610`
    /// every `0x2BF20` = **180 000 ms = 3 minutes** (`@0x29E6`/`@0x2ADA`, the
    /// same `fn_4724` millisecond clock as `MK_SHOCK_WINDOW_MS`). So "Still"
    /// means one random hop every three minutes.
    ///
    /// **Drift is PER-CONTROL — only the monkey moves.** `shock-hour.mp4` is
    /// 120 s of Father Time with zero translation: frame 890 against frame
    /// 1130, 24 s apart, correlates at dx = dy = 0, and the bbox holds one
    /// bottom edge for 308 consecutive frames. Every other measurement in that
    /// same capture pins the slider at band 2 — the open-coat spans
    /// (`FT_GESTURE_N`; band 0's 13 gestures are arithmetically impossible
    /// against a 46-frame span) and the walk counts (`FT_WALK_STOP_NUMER`) —
    /// and `full-reel.mp4` t ≈ 289–293 s shows the module's own Setup panel
    /// with **Drift Speed parked mid-track, readout "Slow"**, i.e. the factory
    /// 50 ⇒ band 2. A band-2 Father Time that does not move while a band-2
    /// monkey glides 10 px/s can only mean the drift has nothing to carry:
    /// the assembly rect `module+0x52` is registered by the monkey glue and
    /// nobody else. UNCERTAIN in mechanism, but the observation is direct on
    /// both sides — see `assembly`.
    fn step_drift(&mut self, ctx: &mut Ctx, band: usize, now: u64) {
        let Some((_, _, w, h)) = self.assembly() else { return };
        let (maxx, maxy) = ((SCREEN_W - w).max(1), (SCREEN_H - h).max(1));
        let interval = DRIFT_INTERVAL_MS[band];
        if interval == 0 {
            // "Still": one fn_0610 re-roll every three minutes (@0x2ACC). The
            // deadline is armed at setup (@0x29E6), so the FIRST hop is three
            // minutes in — the composer's placement holds until then.
            if self.drift_due == 0 {
                self.drift_due = now + DRIFT_STILL_HOP_MS;
            } else if now >= self.drift_due {
                self.drift_pos =
                    Some((ctx.rng.pct(maxx as u32) as i32, ctx.rng.pct(maxy as u32) as i32));
                self.drift_due = now + DRIFT_STILL_HOP_MS;
            }
            return;
        }
        let Some(mut p) = self.drift_pos else {
            let y = ctx.rng.pct(maxy as u32) as i32;
            let x = ctx.rng.pct(maxx as u32) as i32;
            self.drift_pos = Some((x, y));
            self.drift_dir = (
                if x + w / 2 >= SCREEN_W / 2 { -1 } else { 1 },
                if y + h / 2 >= SCREEN_H / 2 { -1 } else { 1 },
            );
            self.drift_due = now + interval;
            return;
        };
        let step = DRIFT_AMP[band] * DRIFT_AMP[band];
        while now >= self.drift_due {
            self.drift_due += interval;
            let (nx, ny) = (p.0 + self.drift_dir.0 * step, p.1 + self.drift_dir.1 * step);
            if ny < 0 || ny >= maxy {
                self.drift_dir.1 = -self.drift_dir.1;
            }
            if nx < 0 || nx >= maxx {
                self.drift_dir.0 = -self.drift_dir.0;
            }
            p = (p.0 + self.drift_dir.0 * step, p.1 + self.drift_dir.1 * step);
        }
        self.drift_pos = Some(p);
    }
}

/// Draw one compound by its absolute number. `bx/by` is the compound bounds
/// origin and `dx/dy` the frame's own link offset; the two are summed, which
/// is what puts the monkey's minute cells at x = 115 + 64 = 179 and
/// 115 + 118 = 233.
fn draw_compound(pack: &Pack, base: u32, no: u32, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
    let Some(f) = pack.frame(base, no) else { return };
    out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x: ax + f.bx + f.dx, y: ay + f.by + f.dy });
}

/// `L135 fn3F2E` @3F2E ([`l135::register`]), the shared-part registration
/// every LINKED SetRun (`L132 fn0204` → `fn028A` → `fn0D3C`) runs from the
/// outgoing frame `a` to the incoming marker `b`: the first part the two
/// share keeps its screen position, and `flip` toggles when the two parts'
/// flip bits differ. No record or no shared part → `None`, no move, no
/// toggle.
///
/// Shared by Father Time (who keeps the move) and the Shocked Monkey (whose
/// `fn_56DA` re-pin overwrites it — see `ShockedMonkey::set_run`).
fn register_shared_part(pack: &Pack, base: u32, a: u32, b: u32, flip: &mut bool) -> Option<Registration> {
    let (fa, fb) = (pack.frame(base, a)?, pack.frame(base, b)?);
    let r = l135::register(&FrameBox::of(fa), &fa.parts, &FrameBox::of(fb), &fb.parts, *flip)?;
    *flip = r.flip;
    Some(r)
}

/// The record's L135 geometry; a missing record reads as the zero box,
/// whose centre is the (0, 0) `centre` has always fallen back to.
fn frame_box(pack: &Pack, base: u32, no: u32) -> FrameBox {
    pack.frame(base, no).map(FrameBox::of).unwrap_or_default()
}

/// `centre(f) = (bx + dx + w/2, by + dy + h/2)` in bank space (port-plan §2,
/// L135 `fn3A70`).
fn centre(pack: &Pack, base: u32, no: u32) -> (i32, i32) {
    frame_box(pack, base, no).centre()
}

/// `centre` for series 1100 (Father Time).
fn ft_centre(pack: &Pack, no: u32) -> (i32, i32) {
    centre(pack, BASE_FT, no)
}

/// `centre` for series 1000 (the Shocked Monkey).
fn mk_centre(pack: &Pack, no: u32) -> (i32, i32) {
    centre(pack, BASE_MONKEY, no)
}

/// Draw frame `idx` of the run that starts at compound `seq`.
fn draw_seq_frame(pack: &Pack, base: u32, seq: u32, idx: u32, ax: i32, ay: i32, out: &mut Vec<SpriteDraw>) {
    draw_compound(pack, base, seq + idx, ax, ay, out);
}

impl Module for ShockClocks {
    fn name(&self) -> &'static str {
        "Shock Clocks"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            // mVal 1000 "Type": menu items 1–4 (Father Time / Rotting Head /
            // Shocked Monkey / Random). Stored mVal is 2, but the shell folds
            // the mark by one, so the module's default selection is item 1 =
            // Father Time (golden g_02: the Setup panel opens on "Father
            // Time"). Our Popup index is 0-based, so raw = index + 1.
            ControlDef {
                name: "Type".into(),
                kind: ControlKind::Popup {
                    // 0-based: set_control folds the menu item number
                    base: 0,
                    // MENU 1000, from the pack: three clocks + Random.
                    items: self.pack.popup_items(1000, 4, false),
                },
                default: 0, // menu item 1 = Father Time
            },
            // sVal 1001 + sUnt 1001: slider 0–100 default 50; ticks 20/40/60/
            // 80/100 = Still / Very Slow / Slow / Moderate / Fast (§1)
            ControlDef {
                name: "Drift Speed".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 50,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => {
                let raw = (value + 1).clamp(1, 4); // back to mVal space
                if raw != self.mval_raw {
                    self.mval_raw = raw;
                    if raw == 4 {
                        // "Random": one Random15()/3 + 1 roll (@0x4A78–0x4A88)
                        self.needs_roll = true;
                    } else {
                        self.selection = raw as u32;
                    }
                }
            }
            1 => self.drift = value.clamp(0, 100),
            _ => {}
        }
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let (hour, minute, _second) = ctx.local_hms;
        let now = ctx.now_ms;

        if !self.started {
            self.started = true;
            self.build_clock(hour, minute);
        }
        if self.needs_roll {
            self.needs_roll = false;
            self.selection = ctx.rng.pct(3) + 1;
        }
        if self.built != self.selection {
            self.build_clock(hour, minute);
        }

        // ---- real-time protocol (§4): fresh time vs last-seen → event flags
        let hour_evt = hour != self.last_hour;
        let minute_evt = minute != self.last_minute;
        if hour_evt {
            self.last_hour = hour;
        }
        if minute_evt {
            self.last_minute = minute;
        }
        if hour_evt || minute_evt {
            match self.clock.as_mut() {
                // The head latches both events inside CSpriteRightEye's own
                // handler (`fn_22E8` @0x22F8) — and as an if/ELSE, so the
                // minute event is swallowed on the hour.
                Some(ActiveClock::Rh(_)) => {}
                // The monkey latches the hour inside its own handler
                // (`fn_185E` @0x18DA) and has no minute event at all.
                Some(ActiveClock::Monkey(_)) => {}
                // gCSpriteFlasher latches the hour in its own handler
                // (`fn_0FB2` @0x0FD8) and has no minute test at all.
                Some(ActiveClock::Ft(_)) => {}
                None => {}
            }
        }

        // ---- the Idle's fn_59A0, BEFORE the CueAnim pump (@0x5366) ----
        self.snd.service(&self.pack, now, &mut ctx.sounds);
        let queued_from = ctx.sounds.len();

        // ---- drift (§8) — the bouncing glide, see `step_drift` ----
        let band = self.drift_band();
        self.step_drift(ctx, band, now);
        // NOTE: the walk's edge gate needs no latch from here any more — it
        // reads the actor's own `h`, which for Father Time is `screen_ax`.
        // See `FT_WALK_EDGE_MARGIN`.

        // ---- actor tick, on the compound-frame grid ----
        //
        // Every part of a clock is one sprite of the same composite, so they
        // all step on the actor's own frame delay; the drift and the §4
        // real-time protocol above stay on the module grid. `TICK_MS` (40) and
        // the delays (100/120) are not commensurate, so the accumulator
        // alternates 2- and 3-tick gaps and averages the delay exactly.
        // The period is re-read every frame: the monkey re-cues itself at
        // 60 / 120 ms from inside its own frame (`MK_RECOVER_MS`).
        self.anim_acc += TICK_MS;
        while self.anim_acc >= self.frame_ms() {
            self.anim_acc -= self.frame_ms();
            self.anim_frames += 1;
            match &mut self.clock {
                Some(ActiveClock::Ft(ft)) => ft.tick(ctx, &self.pack, band),
                Some(ActiveClock::Rh(rh)) => rh.tick(ctx, &self.pack),
                Some(ActiveClock::Monkey(m)) => m.tick(ctx, &self.pack, hour, now),
                None => {}
            }
        }
        // The dressing parts' CueAnim records follow the monkey's in the pump
        // list (the composer `new`s the monkey at `fn_3C48` before the eight
        // `fn_6E2C` parts), so they step after it.
        if let Some(ActiveClock::Monkey(mk)) = &mut self.clock {
            self.dress_acc += TICK_MS;
            while self.dress_acc >= DRESS_FRAME_MS {
                self.dress_acc -= DRESS_FRAME_MS;
                mk.tick_dressing(ctx, &self.pack);
            }
        }
        // Everything the frames above cued goes into the list (`fn_5898`),
        // to be started — or discarded — by a later Idle's `fn_59A0`.
        for id in ctx.sounds.drain(queued_from..).collect::<Vec<_>>() {
            self.snd.queue(id, cue_priority(self.built, id));
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        if !self.started || self.built != self.selection {
            return;
        }
        // Anchors place each composite on the 640×480 field, sized off the
        // golden capture's geometry (the drift slider then wanders from
        // here — the golden's four monkey frames walk the whole assembly
        // (−40, −40) per capture, so the captured positions are drift
        // samples, not the base).
        //
        // Monkey: centre the 427×198 full-scene bounds (compound 285,
        // bx=41, by=75) → ax = (640−427)/2 − 41, ay = (480−198)/2 − 75.
        // Golden g_08 sits at (72, 58), one drift hop from this.
        //
        // Rotting Head: centre the composite's real extent, which is now the
        // ear-to-ear span of the parts, not compound 1's 240×511 sheet —
        // x from the left ear's bx=129 to the right ear's 268+39=307, y from
        // the head's by=11 to the guts' 186+83=269 ⇒ 178×258, so
        // ax = (640−178)/2 − 129, ay = (480−258)/2 − 11.
        //
        // Father Time: `FT_AX`/`FT_AY`, read off the belly dial in the
        // shock-clocks capture — the still frames `g_03`/`g_04` disagreed by
        // 57 px in x precisely because he WALKS (`fn_0CE8`, §5), so they were
        // never usable for this; `FatherTime::pos` now carries that.
        //
        // Once the drift is running those base anchors are gone: the original
        // teleports the assembly to a random spot on its first drift frame and
        // glides from there (`step_drift`), so `anchor()` reads the glide.
        let (ax, ay) = self.anchor();
        match &self.clock {
            Some(ActiveClock::Ft(ft)) => ft.draw(&self.pack, ax, ay, out),
            Some(ActiveClock::Rh(rh)) => rh.draw(&self.pack, ax, ay, out),
            Some(ActiveClock::Monkey(m)) => m.draw(&self.pack, ax, ay, out),
            None => {}
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    /// The engine default, stated rather than inherited: §8's drift is on its
    /// own millisecond deadline (`step_drift`), and the compound-frame
    /// accumulator is driven off `TICK_MS`. The clocks' own
    /// animation rate is NOT this — it is 100 ms (Father Time, Rotting Head) /
    /// 120 ms (Shocked Monkey), see the CueAnim table at the top of the file.
    fn tick_ms(&self) -> u64 {
        TICK_MS
    }

    /// The module's two families of derived art, rebuilt from their names:
    /// the code-drawn dial hands ([`hand_from_name`]) and the recoloured LCD
    /// cells ([`lcd_from_name`]). Both used to be PNGs this module wrote into
    /// the pack, which is why an installed (read-only) `.saver` showed
    /// handless clocks and a monkey panel that never lit.
    fn generated(&self, name: &str) -> Option<Image> {
        let body = name.strip_prefix(engine::GEN_PREFIX)?;
        if body.starts_with("hand/") {
            hand_from_name(name)
        } else if body.starts_with("lcd/") {
            lcd_from_name(&self.pack, name)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn shock_clocks_runs_and_draws() {
        let pack = match Pack::load(Path::new("../assets/shock-clocks")) {
            Ok(p) => p,
            Err(_) => {
                eprintln!("assets/shock-clocks missing — skipping");
                return;
            }
        };
        let Some(mut m) = make(pack) else {
            eprintln!("pack lacks shock-clocks series — skipping");
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
        let mut drew = false;
        for i in 0..1500u64 {
            // cycle all three clocks (+ Random roll) and exercise time changes
            if i % 300 == 0 {
                m.set_control(0, (i / 300 % 4) as i32);
            }
            if i % 97 == 0 {
                ctx.local_hms = ((12 + (i / 3600) % 12) as u8, ((i / 60) % 60) as u8, 0);
            }
            ctx.now_ms += m.tick_ms();
            m.tick(&mut ctx);
            let mut v = Vec::new();
            m.sprites(&mut v);
            if !v.is_empty() {
                drew = true;
            }
        }
        assert!(drew, "sprites() never produced output");
    }

    fn ctx_at(hms: (u8, u8, u8), now_ms: u64) -> Ctx {
        Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms,
            local_hms: hms,
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    fn pack_or_skip() -> Option<Pack> {
        Pack::load(Path::new("../assets/shock-clocks")).ok()
    }

    /// Run one clock for a few ticks at a fixed wall time and return the
    /// packed PNG paths it draws.
    fn drawn_at(type_index: i32, hms: (u8, u8, u8)) -> Vec<String> {
        let Some(pack) = pack_or_skip() else { return Vec::new() };
        let Some(mut m) = make(pack) else { return Vec::new() };
        m.set_control(0, type_index);
        let mut ctx = ctx_at(hms, 0);
        for _ in 0..3 {
            ctx.now_ms += m.tick_ms();
            m.tick(&mut ctx);
        }
        let mut v = Vec::new();
        m.sprites(&mut v);
        v.into_iter().map(|s| s.png).collect()
    }

    /// GOLDEN g_06 (2026-09-01, DEPTH=32): the Shocked Monkey's panel reads
    /// the real wall clock — "1:36" in 12-hour form, hour-tens cell dark.
    /// Spec §11 called compounds 7–35 "plain sequence dressing … no code maps
    /// digits into them"; they are digits 0–9 / 0–6 / 0–9 and this is the
    /// regression guard for that errata.
    #[test]
    fn monkey_lcd_shows_local_hms() {
        let Some(_) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // 13:07 -> "1:07" (h12 = 1, no leading "1" cell), colon lit on the
        // even second.
        let d = drawn_at(2, (13, 7, 4));
        assert!(d.iter().any(|p| p.ends_with("1000/c_003.png")), "ghost panel missing: {d:?}");
        assert!(!d.iter().any(|p| p.ends_with("1000/c_005.png")), "hour-tens lit at 1 o'clock: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_008.png")), "hour units '1' missing: {d:?}"); // seq 7 + 1
        assert!(d.iter().any(|p| p.ends_with("1000/c_037.png")), "colon unlit on an even second: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_018.png")), "minute tens '0' missing: {d:?}"); // seq 18 + 0
        assert!(d.iter().any(|p| p.ends_with("1000/c_033.png")), "minute units '7' missing: {d:?}"); // seq 26 + 7

        // 22:45 -> "10:45": the leading "1" cell lights, hour units 0.
        let d = drawn_at(2, (22, 45, 30));
        assert!(d.iter().any(|p| p.ends_with("1000/c_005.png")), "hour-tens dark at 10 o'clock: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_007.png")), "hour units '0' missing: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_022.png")), "minute tens '4' missing: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_031.png")), "minute units '5' missing: {d:?}");

        // midnight reads as 12, not 0 (the `hour % 12, 0 -> 12` rule).
        let d = drawn_at(2, (0, 0, 1));
        assert!(d.iter().any(|p| p.ends_with("1000/c_005.png")), "midnight lost its leading 1: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("1000/c_009.png")), "midnight hour units != 2: {d:?}");
        assert!(!d.iter().any(|p| p.ends_with("1000/c_037.png")), "colon lit on an odd second: {d:?}");

        // and the panel must actually MOVE with the clock, not sit painted.
        assert_ne!(drawn_at(2, (13, 7, 4)), drawn_at(2, (13, 8, 4)), "LCD frozen across a minute");
    }

    /// THE read-only-bundle property (2026-09-19). Both families of derived
    /// art — the code-drawn dial hands and the recoloured LCD cells — are
    /// rebuilt from their own names by `Module::generated`, and running the
    /// module never writes into the pack. Before this commit `ensure_hand`
    /// and `ensure_lcd` put PNGs in `<pack>/compounds/<base>/hands/` and
    /// `<pack>/compounds/1000/lcd/`; inside an installed `.saver` the bundle
    /// is read-only, both writes failed, and the clocks ran handless with a
    /// panel that never lit.
    #[test]
    fn derived_art_is_generated_and_the_pack_is_never_written() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let root = pack.root().to_path_buf();
        let before = tree_stat(&root);
        let m = build(pack.clone()).expect("shock-clocks module");

        // hands: every angle index either dial can ask for
        for dial in [&RH_DIAL, &FT_DIAL] {
            let n = (dial.rad * 2 + 1) as u32;
            for (kind, idx) in [('h', 719u32), ('m', 59), ('s', 0)] {
                let name = hand_name(dial, kind, idx);
                assert!(engine::is_generated(&name), "{name}");
                let img = m.generated(&name).unwrap_or_else(|| panic!("no pixels for {name}"));
                assert_eq!((img.w, img.h), (n, n), "{name}");
                assert!(img.rgba.chunks_exact(4).any(|p| p[3] == 0xFF), "{name} is blank");
                assert!(
                    img.rgba.chunks_exact(4).all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0),
                    "{name} is not the art's black ink"
                );
            }
        }

        // LCD: the recolour is LCD_LIT -> tint and nothing else moves
        let src = "compounds/1000/c_005.png";
        let base = pack.image(src);
        let tint = [0x12u8, 0x34, 0x56];
        let name = lcd_name(src, tint);
        let img = m.generated(&name).expect("no pixels for the LCD cell");
        assert_eq!((img.w, img.h), (base.w, base.h));
        let mut lit = 0;
        for (a, b) in base.rgba.chunks_exact(4).zip(img.rgba.chunks_exact(4)) {
            if a[0..3] == LCD_LIT[..] {
                assert_eq!(&b[0..3], &tint, "a lit segment kept the packed colour");
                lit += 1;
            } else {
                assert_eq!(a, b, "a non-lit pixel moved");
            }
        }
        assert!(lit > 0, "the cell has no lit segments to recolour");
        // and at rest the module names the packed compound, not a gen: name
        assert_eq!(lcd_art(src, Lcd::Rest), src);

        assert_eq!(before, tree_stat(&root), "the module wrote into the pack");
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

    /// GOLDEN g_12/g_13/g_14: the Rotting Head's forehead dial carries three
    /// real hands (hour/minute/second) that sweep. Spec §2/§6 called it
    /// "painted art" and a prior pass concluded "no hands is correct" — this
    /// is the guard for that errata. Hand geometry is CODE 130
    /// @0x0EAC–0x10C2 (hub = compound 411, lengths 18/31/34).
    #[test]
    fn rotting_head_hands_track_local_hms() {
        let Some(_) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let d = drawn_at(1, (3, 17, 42));
        // hour index is in half-degree units: (3 % 12) * 60 + 17 = 197
        assert!(d.iter().any(|p| p.ends_with("hand/1200/h_197")), "hour hand: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("hand/1200/m_017")), "minute hand: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("hand/1200/s_042")), "second hand: {d:?}");
        // 12-hour fold: 15:17 draws the same hour hand as 3:17
        let pm = drawn_at(1, (15, 17, 42));
        assert_eq!(d, pm, "hour hand did not fold mod 12");
        // every unit moves its own hand
        assert_ne!(d, drawn_at(1, (3, 17, 43)), "second hand frozen");
        assert_ne!(d, drawn_at(1, (3, 18, 42)), "minute hand frozen");
        assert_ne!(d, drawn_at(1, (4, 17, 42)), "hour hand frozen");
    }

    /// Run one clock for `n` ticks at a fixed wall time and return the packed
    /// PNG paths, in draw order.
    fn drawn_after(type_index: i32, hms: (u8, u8, u8), n: u32) -> Vec<String> {
        let Some(pack) = pack_or_skip() else { return Vec::new() };
        let Some(mut m) = make(pack) else { return Vec::new() };
        m.set_control(0, type_index);
        m.set_control(1, 0); // Drift Speed "Still" — anchors stay put
        let mut ctx = ctx_at(hms, 0);
        for _ in 0..n {
            ctx.now_ms += m.tick_ms();
            m.tick(&mut ctx);
        }
        let mut v = Vec::new();
        m.sprites(&mut v);
        v.into_iter().map(|s| s.png).collect()
    }

    fn index_of(v: &[String], leaf: &str) -> Option<usize> {
        v.iter().position(|p| p.ends_with(leaf))
    }

    /// GOLDEN `g_12`–`g_15`: the Rotting Head is a composite, not compound 1.
    /// CODE 130's scene composer (`fn @0x01A6`) installs the head as compound
    /// **3** (@0x0312) and the dangling neck guts as compound
    /// **192** (`0xC0` @0x0388), with the two CSpriteEar actors created FIRST
    /// so the head draws over them. Compound 1 — the 240×511 sheet that also
    /// carries the loose eyeball and the two fallen ears at the bottom of its
    /// canvas — must never be blitted whole.
    #[test]
    fn rotting_head_composite_matches_the_composer() {
        let Some(_) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let d = drawn_at(1, (3, 17, 42));
        assert!(!d.iter().any(|p| p.ends_with("1200/c_001.png")), "compound 1 sheet blitted: {d:?}");
        let head = index_of(&d, "1200/c_003.png").expect(&format!("head compound 3 missing: {d:?}"));
        assert!(index_of(&d, "1200/c_192.png").is_some(), "neck guts (192) missing: {d:?}");
        // ears under the head: their art is fully opaque with a black
        // surround, so drawing one last punches a black box through the temple
        let ear_l = index_of(&d, "1200/c_043.png").expect(&format!("left ear missing: {d:?}"));
        let ear_r = index_of(&d, "1200/c_087.png").expect(&format!("right ear missing: {d:?}"));
        assert!(ear_l < head && ear_r < head, "ears drawn over the head: {d:?}");
        // the socket and the mouth are HOLES in compound 3 — parts fill them
        // the eye part fills the socket with whichever of its three runs it
        // is on (9 rest / 11 blink / 20 cuckoo)
        assert!(
            (9..=41).any(|n| index_of(&d, &format!("1200/c_{n:03}.png")).is_some()),
            "right eye missing: {d:?}"
        );
        assert!(index_of(&d, "1200/c_136.png").is_some(), "mouth (lips + tongue) missing: {d:?}");
        assert!(index_of(&d, "1200/c_138.png").is_some(), "upper tooth missing: {d:?}");
        assert!(index_of(&d, "1200/c_165.png").is_some(), "lower tooth missing: {d:?}");
    }

    /// The CSpriteEar ring (@0x0AE4): 43 → 45 (29-frame droop) → 75 (on the
    /// ground, sliding) → 77 (9-frame climb back) → 43, gate rolled only at
    /// 43, and the ONLY exit from the ground state is the slide running out
    /// (`fn_0C14` @0x0C10). GOLDEN `g_14` has the left ear off the head.
    ///
    /// RATCHET: drive a real head for 5 minutes and assert both ears visit
    /// all four states AND actually leave the temple (`slide_x > 0` at some
    /// point). Reverting to an unconditional hand-off leaves `slide_x`
    /// pinned at 0 and this fails.
    #[test]
    fn ear_ring_walks_all_four_states_and_the_ear_actually_slides_off() {
        let Some(mut m) = pack_or_skip().and_then(build) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        m.set_control(0, 1); // Rotting Head
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut seen = [std::collections::BTreeSet::new(), std::collections::BTreeSet::new()];
        let mut fell = [false, false];
        let mut t = 0u64;
        while t < 300_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms =
                (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            if let Some(ActiveClock::Rh(rh)) = &m.clock {
                for i in 0..2 {
                    seen[i].insert(rh.ears[i].part.seq);
                    fell[i] |= rh.ears[i].slide_x > 0;
                }
            }
        }
        for i in 0..2 {
            let want: std::collections::BTreeSet<u32> = EAR_RING[i].iter().copied().collect();
            assert_eq!(seen[i], want, "ear {i} visited {:?}", seen[i]);
            assert!(fell[i], "ear {i} never left the temple");
        }
    }

    /// The 18 drip parts of the composer (@0x058C–@0x0E8A). Each `(start,
    /// count)` pair must land exactly on a packed run: `start` is one past a
    /// sequence's `first`, and `first + count` is its last frame. If the pack
    /// or the table ever drift apart, the drips silently stop drawing.
    #[test]
    fn drip_runs_line_up_with_the_pack() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        assert_eq!(RH_DRIPS.len(), 18);
        for (start, count) in RH_DRIPS {
            let first = start - 1;
            let len = pack.seq(BASE_RH, first).map(|s| s.frames.len() as u32);
            assert_eq!(
                len,
                Some(count + 1),
                "drip {start}/{count} does not sit on packed run {first}"
            );
            for i in 0..count {
                assert!(pack.frame(BASE_RH, start + i).is_some(), "drip frame {} missing", start + i);
            }
        }
    }

    /// Compounds 39–57 are the machine's electric arcs (spec §2 filed them as
    /// "knobs/gauge pieces"), and 243 is the bar the gorilla hangs from —
    /// GOLDEN `g_06` needs compound 55 and `g_09` needs compound 43 to close
    /// out, and `g_07` (a zap pose, which drops the bar from its own art)
    /// needs 243.
    #[test]
    fn monkey_machine_dressing_is_drawn() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        for run in MK_ARCS {
            assert!(pack.seq(BASE_MONKEY, run).is_some(), "arc run {run} not packed");
        }
        assert!(pack.frame(BASE_MONKEY, MK_BAR).is_some(), "hang bar 243 not packed");
        // the bar is always under the pose; arcs light on their own, so sweep
        // until at least one has fired.
        let d = drawn_at(2, (13, 7, 4));
        assert!(index_of(&d, "1000/c_243.png").is_some(), "hang bar not drawn: {d:?}");
        let lit = (1..=400u32).any(|n| {
            let v = drawn_after(2, (13, 7, 4), n);
            MK_ARCS.iter().any(|r| v.iter().any(|p| p.ends_with(&format!("1000/c_{r:03}.png"))))
        });
        assert!(lit, "no machine arc fired in 400 ticks");
    }

    /// GOLDEN g_02 (Setup panel opens on "Father Time") + g_03 (Father Time
    /// on screen) + g_10 (check on "Shocked Monkey" while the monkey runs):
    /// stored `mVal 1000 = 2` reaches the module as menu item 1. Spec
    /// §1.1/§12.7 read the raw mVal as the selection and made Rotting Head
    /// the default — the mark-fold bug class.
    #[test]
    fn factory_default_is_father_time() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let Some(m) = make(pack) else { return };
        assert_eq!(m.controls()[0].default, 0, "Type popup default is not item 1");
        // and the untouched module must build Father Time (series 1100)
        let Some(pack) = pack_or_skip() else { return };
        let Some(mut m) = make(pack) else { return };
        let mut ctx = ctx_at((12, 0, 0), 0);
        ctx.now_ms += m.tick_ms();
        m.tick(&mut ctx);
        let mut v = Vec::new();
        m.sprites(&mut v);
        assert!(!v.is_empty(), "default clock drew nothing");
        assert!(
            v.iter().all(|s| s.png.contains("/110")),
            "default clock is not Father Time (RLEP 1100): {:?}",
            v.iter().map(|s| &s.png).collect::<Vec<_>>()
        );
    }

    /// Run a clock on the module grid for `ms` of wall time, feeding
    /// `local_hms` forward so the §4 minute/hour events actually fire.
    fn run(type_index: i32, ms: u64) -> Option<(ShockClocks, Vec<String>)> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, type_index);
        m.set_control(1, 0); // Drift Speed "Still"
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
        }
        let mut v = Vec::new();
        m.sprites(&mut v);
        let pngs = v.into_iter().map(|s| s.png).collect();
        Some((m, pngs))
    }

    /// Step the Shocked Monkey on the module grid, sampling (pose seq, LCD
    /// state, the PNG drawn for the minute-units digit) after every tick and
    /// keeping the changes.
    fn mk_trace(ms: u64) -> Option<Vec<(u64, u32, Lcd, String)>> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, 2); // Shocked Monkey
        m.set_control(1, 0); // Drift Speed "Still"
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut out: Vec<(u64, u32, Lcd, String)> = Vec::new();
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            let Some(ActiveClock::Monkey(mk)) = &m.clock else { continue };
            let mut v = Vec::new();
            mk.draw_lcd(&m.pack, 0, 0, &mut v);
            // the minute-units cell is always drawn, whatever the reading
            let cell = v.last().map(|s| s.png.clone()).unwrap_or_default();
            let e = (t, mk.pose.seq, mk.lcd, cell);
            if out.last().map(|p| (p.1, p.2, p.3.clone())) != Some((e.1, e.2, e.3.clone())) {
                out.push(e);
            }
        }
        Some(out)
    }

    /// GOLDEN `emu/captures/shock-monkey.mp4` — the gag §5/§11 left as
    /// "something recolours or overdraws them" and the module header filed as
    /// "LCD white = monkey shock, unimplemented". Sampling the modal colour of
    /// the lit segments inside the panel rect gives, four times over:
    ///
    /// - rest `(252,255,187)` = the packed `(255,255,194)`;
    /// - a linear **ramp** toward black starting on the frame he enters the
    ///   startle pose (f78 / f287 / f595 / f721 vs startles at f78 / f287 /
    ///   f596 / f721), ~2.85 RGB units per 100 ms — two ramps of different
    ///   length (45 and 64 frames) give 2.89 and 2.80, and the long one keeps
    ///   falling to `(73,76,31)`, so it does not clamp;
    /// - pure **white** `(252,252,252)` from three frames after the last 277
    ///   frame — pose 282 is two frames, so that is the leaving-282 edge
    ///   (f124 / f331 / f646 / f844) — held 31 / 30 / 31 / 31 module frames;
    /// - back to the packed pale (f155 / f361 / f677 / f875).
    #[test]
    fn monkey_lcd_ramps_down_then_flashes_white_after_the_shock() {
        // 180 s, not 60: gags are a 1-in-10 roll per idle gesture and the
        // full-screen golden has a 118 s gap between two of its eight, so a
        // 60 s window only ever held one by seed luck.
        let Some(trace) = mk_trace(180_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // the gag has to be reachable at all without waiting for an hour
        let shocked: Vec<_> = trace.iter().filter(|e| e.1 == 277).collect();
        assert!(!shocked.is_empty(), "the monkey never got shocked in 180 s");

        // 1. the ramp is armed on the startle, never before it
        let first_ramp = trace
            .iter()
            .position(|e| matches!(e.2, Lcd::Ramp { .. }))
            .expect("the LCD never started fading");
        assert!(
            matches!(trace[first_ramp].1, 225 | 205),
            "the ramp armed on pose {} instead of the startle",
            trace[first_ramp].1
        );
        assert!(
            trace[..first_ramp].iter().all(|e| e.2 == Lcd::Rest),
            "the LCD moved before the startle"
        );
        // and it actually darkens the drawn art, monotonically
        let levels: Vec<u32> = trace[first_ramp..]
            .iter()
            .take_while(|e| matches!(e.2, Lcd::Ramp { .. }))
            .map(|e| match e.2 { Lcd::Ramp(c) => u32::from(c[0]), _ => unreachable!() })
            .collect();
        assert!(levels.len() > 4, "the ramp produced no distinct levels: {levels:?}");
        assert!(levels.windows(2).all(|w| w[1] <= w[0]), "the ramp brightened: {levels:?}");
        assert!(
            trace[first_ramp..]
                .iter()
                .take_while(|e| matches!(e.2, Lcd::Ramp { .. }))
                .any(|e| e.3.starts_with("gen:lcd/t")),
            "no dimmed digit art was ever drawn"
        );

        // 2. white is armed exactly on leaving 282, and only there
        let w = trace
            .iter()
            .position(|e| matches!(e.2, Lcd::White { .. }))
            .expect("the LCD never went white");
        assert_eq!(trace[w - 1].1, 282, "white did not arm on the 282 edge");
        assert!(trace[w].3.starts_with("gen:lcd/tffffff/"), "white state drew pale art: {}", trace[w].3);

        // 3. it holds MK_SHOCK_WINDOW_MS and then returns to the packed pale art
        let back = trace[w..]
            .iter()
            .find(|e| e.2 == Lcd::Rest)
            .expect("the LCD never came back to pale");
        let held = back.0 - trace[w].0;
        assert!(
            held.abs_diff(MK_SHOCK_WINDOW_MS) <= 2 * MK_FRAME_MS,
            "white held {held} ms, want {MK_SHOCK_WINDOW_MS}"
        );
        assert!(!engine::is_generated(&back.3), "rest state drew derived art: {}", back.3);

        // 4. the colon keeps its own 1 Hz beat right through the white phase
        //    (capture: the lit pixel count steps 1745 ⇄ 1793 every ~10 frames,
        //    gag or no gag)
        let mut m = build(pack_or_skip().unwrap()).unwrap();
        m.set_control(0, 2);
        let mut lit = Vec::new();
        for s in [0u8, 1, 2, 3] {
            let mk = ShockedMonkey { hms: (12, 34, s), lcd: Lcd::White, ..ShockedMonkey::new(&m.pack, 12) };
            let mut v = Vec::new();
            mk.draw_lcd(&m.pack, 0, 0, &mut v);
            lit.push(v.iter().any(|d| d.png.contains("c_037")));
        }
        assert_eq!(lit, vec![true, false, true, false], "the colon stopped blinking under the flash");
    }

    /// The compound-frame rate is the actor's own `CueAnim` delay (Father Time
    /// `fn_0DF4` @0x0E80 = 100, Shocked Monkey `fn_16BA` @0x1730 = 120,
    /// CSpriteRightEye `fn_2156` @0x21D6 = 100), not the 40 ms module grid.
    /// USER REPORT 2026-09-01: the port was running every clock at one frame
    /// per grid tick, i.e. 2.5–3× the packed rate.
    #[test]
    fn compound_frames_run_at_the_ctor_cue_rate() {
        let Some(_) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // 12 s of wall clock, the same for all three, so the counts are
        // directly comparable: 12000/100 = 120 frames, 12000/120 = 100.
        const MS: u64 = 12_000;
        for (ty, period, label) in
            [(0i32, FT_FRAME_MS, "Father Time"), (1, RH_FRAME_MS, "Rotting Head"), (2, MK_FRAME_MS, "Shocked Monkey")]
        {
            let (m, _) = run(ty, MS).unwrap();
            let want = MS / period;
            // the accumulator is seeded at the period so tick 1 always draws a
            // frame — that is the only slack, hence ±1.
            assert!(
                m.anim_frames.abs_diff(want) <= 1,
                "{label}: {} compound frames in {MS} ms, want ~{want} ({period} ms/frame)",
                m.anim_frames
            );
            assert_eq!(m.frame_ms(), period, "{label} frame delay");
        }
        // and the grid itself is still the engine default
        assert_eq!(TICK_MS, 40);
        // the arithmetic the user actually sees: Father Time is 2.5× slower
        // than the old one-frame-per-tick behaviour, the monkey 3×.
        assert_eq!(FT_FRAME_MS * 10, TICK_MS * 25); // 100/40 = 2.5×
        assert_eq!(MK_FRAME_MS, TICK_MS * 3); // 120/40 = 3×
    }

    /// Step one clock on the module grid, sampling the active Father Time's
    /// (sequence, frame, screen ax) after every tick and keeping the changes.
    fn ft_trace(ms: u64) -> Option<Vec<(u32, u32, i32)>> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, 0); // Father Time
        m.set_control(1, 0); // Drift Speed "Still" — no drift in the trace
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut out: Vec<(u32, u32, i32)> = Vec::new();
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            if let Some(ActiveClock::Ft(ft)) = &m.clock {
                let e = (ft.part.seq, ft.part.frame, ft.screen_ax(&m.pack));
                if out.last() != Some(&e) {
                    out.push(e);
                }
            }
        }
        Some(out)
    }

    /// Like `ft_trace` but with the wall clock seeded at `start` (h, m, s) so
    /// an hour or minute rollover is reachable in a handful of seconds.
    /// Returns the (seq, frame, ax) change-trace and every sound fired.
    fn ft_trace_from(start: (u8, u8, u8), ms: u64) -> Option<(Vec<(u32, u32, i32)>, Vec<u32>)> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, 0); // Father Time
        m.set_control(1, 0); // Drift Speed "Still"
        let base = u64::from(start.0) * 3600 + u64::from(start.1) * 60 + u64::from(start.2);
        let mut ctx = ctx_at(start, 0);
        let mut out: Vec<(u32, u32, i32)> = Vec::new();
        let mut sounds = Vec::new();
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = base + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            sounds.extend(ctx.sounds.iter().copied());
            if let Some(ActiveClock::Ft(ft)) = &m.clock {
                let e = (ft.part.seq, ft.part.frame, ft.screen_ax(&m.pack));
                if out.last() != Some(&e) {
                    out.push(e);
                }
            }
        }
        Some((out, sounds))
    }

    /// Count how many separate runs of `seq` the trace contains.
    fn runs_of(trace: &[(u32, u32, i32)], seq: u32) -> usize {
        let mut n = 0;
        let mut prev_frame = u32::MAX;
        let mut inside = false;
        for e in trace {
            if e.0 != seq {
                inside = false;
                continue;
            }
            // a fresh run restarts at frame 0
            if !inside || e.1 < prev_frame {
                n += 1;
            }
            inside = true;
            prev_frame = e.1;
        }
        n
    }

    /// GOLDEN `emu/captures/shock-hour.mp4`, the whole point of that capture:
    /// **Father Time cuckoos on the hour, `hour % 12` times, and at no other
    /// time.** Recording starts 09:58:30 so 10:00:00 is t = 90.0 s; the yellow
    /// bird is on screen in exactly ten runs (onsets f1026, 1037, 1049, 1060,
    /// 1071, 1082, 1093, 1104, 1116, 1127) and the white dial disc vanishes
    /// for ten frames and returns for one across ten 11-frame cycles,
    /// f1025 → f1136. Eleven frames is seq 119's packed length; ten is
    /// `hour % 12` at 10 o'clock. The detector fires nowhere in the capture's
    /// other 102 s, which covers six complete passes of `FT_POOL` — so 119 is
    /// a chain SLOT the hour flag unlocks, not a free-running gesture.
    ///
    /// The port used to keep 119 in the chain unconditionally (a cuckoo every
    /// ~14 s, never on the hour) and never even handed Father Time his hour
    /// event.
    #[test]
    fn father_time_cuckoos_hour_mod_twelve_times_on_the_hour() {
        let Some((quiet, quiet_snd)) = ft_trace_from((12, 0, 0), 60_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // 60 s with no hour rollover: four full chain passes, zero cuckoos.
        assert_eq!(runs_of(&quiet, FT_CUCKOO_SEQ), 0, "cuckooed without an hour event");
        assert!(!quiet_snd.contains(&SND_CUCKOO), "Cuckoo sound without an hour event");
        assert!(quiet.iter().any(|e| e.0 == 131), "the chain never reached the 119 slot");

        // cross 10:00:00 — hour % 12 == 10, exactly the capture's reading.
        let (trace, snd) = ft_trace_from((9, 59, 55), 60_000).unwrap();
        assert_eq!(
            runs_of(&trace, FT_CUCKOO_SEQ),
            10,
            "10 o'clock did not produce ten cuckoo passes: {:?}",
            trace.iter().map(|e| e.0).collect::<Vec<_>>()
        );
        assert_eq!(
            snd.iter().filter(|s| **s == SND_CUCKOO).count(),
            10,
            "snd 2000 did not fire once per cuckoo pass"
        );
        // the ten passes are consecutive and hand straight on to the coat
        // close (capture: seq 131 lands on f1137, one frame after the last).
        let after: Vec<u32> = trace
            .iter()
            .skip_while(|e| e.0 != FT_CUCKOO_SEQ)
            .skip_while(|e| e.0 == FT_CUCKOO_SEQ)
            .map(|e| e.0)
            .collect();
        assert_eq!(after.first(), Some(&131), "the cuckoo did not hand on to the coat close");

        // midnight is the 0 -> 12 case (§12.2), shared with the other clocks.
        let (mid, _) = ft_trace_from((23, 59, 55), 60_000).unwrap();
        assert_eq!(runs_of(&mid, FT_CUCKOO_SEQ), 12, "midnight is not twelve cuckoos");
    }

    /// GOLDEN `shock-hour.mp4`: a minute rollover does NOT divert Father Time.
    /// 09:59:00 lands at t = 30.0 s, mid-walk (f272–f303): the walk runs
    /// through it at its usual +76 px/cycle and the chain order is unchanged.
    /// This is the guard for the `minute_pending` flag staying a no-op.
    #[test]
    fn father_time_minute_rollover_does_not_divert_the_chain() {
        // 11:29:58 -> the 11:30 rollover lands at t = 2 s, inside the window;
        // 11:30:00 -> the next one is at t = 60 s, outside it. Neither crosses
        // an hour, so the only difference is the minute flag.
        let Some((with_minute, _)) = ft_trace_from((11, 29, 58), 30_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let (no_minute, _) = ft_trace_from((11, 30, 0), 30_000).unwrap();
        let seqs = |t: &[(u32, u32, i32)]| {
            let mut v: Vec<u32> = Vec::new();
            for e in t {
                if v.last() != Some(&e.0) {
                    v.push(e.0);
                }
            }
            v
        };
        assert_eq!(
            seqs(&with_minute),
            seqs(&no_minute),
            "the minute flag changed Father Time's gesture chain"
        );
    }

    /// GOLDEN `shock-hour.mp4`: the chain WRAP. Ten walks in that capture
    /// start at ax = −176, 140, 152, −176, 140, −176, 65, 152, −176, 65 and
    /// step +76 per cycle, so a walk resumes where the previous one stopped
    /// instead of teleporting back to the entry offset; the only resets to
    /// −176 come right after he has walked off the right edge (f303 → blank
    /// f305–f319 → f320 at −176; f491 → blank → f515 at −176). The port used
    /// to zero `walk_cycle` on every chain wrap, so he re-entered from off
    /// screen left once per pass and never crossed the field.
    #[test]
    fn father_time_walk_carries_its_position_across_the_chain() {
        let Some((trace, _)) = ft_trace_from((12, 0, 0), 120_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // the ax of each walk cycle, in order (the trace never samples the
        // very first frame, so key on the ax changing rather than frame == 0)
        // (ax, first cycle of a walk?)
        let mut cycles: Vec<(i32, bool)> = Vec::new();
        let mut walking = false;
        for e in &trace {
            if e.0 != FT_WALK_SEQ {
                walking = false;
                continue;
            }
            if !walking || cycles.last().map(|c| c.0) != Some(e.2) {
                cycles.push((e.2, !walking));
            }
            walking = true;
        }
        assert!(cycles.len() >= 12, "not enough walk cycles in 120 s: {cycles:?}");
        assert_eq!(cycles[0].0, FT_WALK_ENTER_AX, "he did not walk on from the left");
        let mut wraps = 0;
        let mut resumes = 0;
        for w in cycles.windows(2) {
            // within a walk: the 76 px stride; across a chain pass: the
            // stop/resume registrations net +88 (see `FT_WALK_RESUME_DX`)
            let step = if w[1].1 { FT_WALK_RESUME_DX } else { FT_WALK_STEP };
            if w[1].0 == FT_WALK_ENTER_AX && w[0].0 + step >= FT_WALK_WRAP_AX {
                wraps += 1;
            } else {
                assert_eq!(w[1].0, w[0].0 + step, "walk cycle {w:?} did not step {step} px");
                resumes += usize::from(w[1].1);
            }
        }
        assert!(resumes >= 1, "no walk resumed after a chain pass: {cycles:?}");
        let cycles: Vec<i32> = cycles.iter().map(|c| c.0).collect();
        assert!(wraps >= 1, "he never crossed the field in 120 s: {cycles:?}");
        // and the first walk is still the capture's four-cycle entry
        assert_eq!(&cycles[..4], &[-176, -100, -24, 52]);
    }

    /// GOLDEN `emu/captures/shock-clocks.mp4` (t 7.7–11.8 s): Father Time
    /// WALKS ON. Sequence 1's ten frames carry the stride in their own packed
    /// `bx` (108 → 188) and the actor adds a flat +76 px at each wrap, so his
    /// offset reads ax = −176, −100, −24, +52 over four successive cycles —
    /// measured off c_010, whose art ends at compound x 263 and lands at
    /// module x 87.5 / 163.5 / 239.5 / 315.5. Four cycles × 10 frames × 100 ms
    /// = 4.00 s, exactly the capture's 7.77 → 11.77. The port had him standing
    /// still at a centred ax = 114.
    #[test]
    fn father_time_walks_on_at_76px_a_cycle() {
        let Some(trace) = ft_trace(6_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let walk: Vec<_> = trace.iter().take_while(|e| e.0 == FT_WALK_SEQ).collect();
        // the initial frame 0 is never sampled (the trace starts after tick 1),
        // so the first cycle contributes frames 1..9 and the rest 0..9. The
        // COUNT is a roll now (see `FT_WALK_STOP_NUMER`), so only the stride
        // is pinned here.
        assert_eq!(walk.len() % 10, 9, "walk ended mid-cycle: {} frames", walk.len());
        assert!(walk.len() >= 9, "no walk at all");
        for (i, e) in walk.iter().enumerate() {
            let cycle = ((i + 1) / 10) as i32; // +1 for the unsampled frame 0
            let want = FT_WALK_ENTER_AX + FT_WALK_STEP * cycle;
            assert_eq!(e.2, want, "walk frame {i} at ax {} want {want}", e.2);
        }
        // and each cycle plays every packed frame, in order
        if walk.len() >= 19 {
            let cycle2: Vec<u32> = walk[9..19].iter().map(|e| e.1).collect();
            assert_eq!(cycle2, (0..10).collect::<Vec<u32>>(), "walk cycle frames");
        }
        // then he stops, on the spot, and hands to seq 13 (@0x138C/@0x1362)
        // — registered on the head (art 37), which puts c_013's compound
        // origin 24 px left of the last cycle's (see `FatherTime::set_run`)
        let after = trace[walk.len()];
        assert_eq!(after.0, 13, "walk did not hand off to seq 13");
        assert_eq!(after.2, walk.last().unwrap().2 - 24, "he did not stop where he stood");
        // the speed the user sees: FT_WALK_STEP px per ten-frame cycle of
        // 10 × FT_FRAME_MS = 1.00 s ⇒ 76 px/s = 2.53 px per 30 fps frame
        assert_eq!(10 * FT_FRAME_MS, 1_000);
        assert_eq!(FT_WALK_STEP, 76);
    }

    /// The sequence of sequences Father Time visits, consecutive duplicates
    /// kept apart by watching the frame counter restart.
    fn ft_seq_runs(trace: &[(u32, u32, i32)]) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        let mut prev: Option<(u32, u32)> = None;
        for e in trace {
            let fresh = match prev {
                Some((s, f)) => s != e.0 || e.1 <= f,
                None => true,
            };
            if fresh {
                out.push(e.0);
            }
            prev = Some((e.0, e.1));
        }
        out
    }

    /// Every transition Father Time makes must be one `fn_1250` allows.
    ///
    /// This replaces `father_time_runs_the_chain_in_order`, which asserted the
    /// chain was `FT_POOL` walked with an index. The disasm says otherwise (see
    /// `ft_next`/`FT_GESTURE_N`): three of the "in-order" steps the capture
    /// showed are coin flips — 13→17 at 37.5 %, 21→26 at 18.75 %, 30→37 at
    /// 62.5 % — and the run 51…88 is a counted loop. A 30 s trace exercises
    /// both skips, which is why the old assertion could not survive.
    #[test]
    fn father_time_walks_the_fn_1250_state_machine() {
        let Some(trace) = ft_trace(60_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let seqs = ft_seq_runs(&trace);
        assert!(FT_POOL.iter().all(|s| *s == 119 || seqs.contains(s)) || seqs.len() > 20);
        let legal = |a: u32| -> Vec<u32> {
            match a {
                1 => vec![1, 13],
                13 => vec![17, 21],
                17 => vec![21],
                21 => vec![26, 30],
                26 => vec![30],
                30 => vec![37, 51],
                37 => vec![51],
                51 | 58 | 62 | 69 | 82 | 88 => {
                    let mut v = FT_GESTURE_POOL.to_vec();
                    v.extend_from_slice(&FT_GESTURE_EXIT);
                    v.push(119);
                    v
                }
                94 => vec![103, 112],
                103 => vec![94, 112],
                112 => vec![119, 131],
                119 => vec![119, 131],
                131 => vec![135],
                135 => vec![1],
                _ => vec![],
            }
        };
        for w in seqs.windows(2) {
            assert!(
                legal(w[0]).contains(&w[1]),
                "illegal fn_1250 transition {} → {} in {seqs:?}",
                w[0],
                w[1]
            );
        }
        // 60 s of chain must reach the coat flash, so the dial is actually
        // on screen in a headless render
        assert!(seqs.contains(&51), "the coat never opened in 60 s: {seqs:?}");
        // and both 2-way skips must actually be taken somewhere in a minute —
        // an in-order walk would never produce either
        assert!(
            seqs.windows(2).any(|w| w == [13, 21]) || seqs.windows(2).any(|w| w == [21, 30]),
            "no probabilistic skip anywhere in 60 s — the chain is still a ring: {seqs:?}"
        );
    }

    /// The open-coat loop runs `FT_GESTURE_N[band]` gestures, so the Drift
    /// Speed slider sets how long the dial stays visible (`DATA129+0x318`,
    /// `@0x146A`). Band 0 = 13 gestures, band 4 = 3.
    #[test]
    fn open_coat_loop_length_follows_the_drift_band() {
        for (raw, band) in [(0, 0usize), (50, 2), (100, 4)] {
            let Some(pack) = pack_or_skip() else {
                eprintln!("assets/shock-clocks missing — skipping");
                return;
            };
            let mut m = build(pack).unwrap();
            m.set_control(0, 0);
            m.set_control(1, raw);
            let mut ctx = ctx_at((12, 0, 0), 0);
            let mut t = 0u64;
            let mut trace: Vec<(u32, u32, i32)> = Vec::new();
            while t < 120_000 {
                t += TICK_MS;
                ctx.now_ms = t;
                let s = 12 * 3600 + t / 1000;
                ctx.local_hms =
                    (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
                m.tick(&mut ctx);
                if let Some(ActiveClock::Ft(ft)) = &m.clock {
                    let e = (ft.part.seq, ft.part.frame, 0);
                    if trace.last() != Some(&e) {
                        trace.push(e);
                    }
                }
            }
            let seqs = ft_seq_runs(&trace);
            // count the gestures in each complete 51…(94|103) stretch
            let mut lens = Vec::new();
            let mut n = 0u32;
            for s in &seqs {
                match s {
                    51 => n = 1,
                    58 | 62 | 69 | 82 | 88 if n > 0 => n += 1,
                    94 | 103 if n > 0 => {
                        lens.push(n);
                        n = 0;
                    }
                    _ => n = 0,
                }
            }
            assert!(!lens.is_empty(), "band {band}: no complete open-coat run in 120 s");
            for l in &lens {
                assert_eq!(
                    *l, FT_GESTURE_N[band],
                    "band {band}: open-coat run of {l} gestures, want {}",
                    FT_GESTURE_N[band]
                );
            }
        }
    }

    /// The walk length is a roll, not a constant. `fn_1250` @`0x136A` exits to
    /// seq 13 on `Random15() % 256 < 96 − 16·band`, so at band 2 the roll is
    /// geometric with p = ¼ — but the roll is only REACHED while the actor's
    /// `h` is inside `[100, 380]` (`FT_WALK_EDGE_MARGIN`), which is 4 of the
    /// 10 positions on a −176…508 lap. Mean ≈ 10.5 cycles, support 1…∞.
    ///
    /// GOLDEN (`shock-hour.mp4`): ten apparent walk starts at ax = −176, 140,
    /// 152, −176, 140, −176, 65, 152, −176, 65 with visible lengths 1, 5, 4,
    /// 5, 3, 1, 5, 3, 2. **Four of those starts are the wrap** (`−176` =
    /// `FT_WALK_ENTER_AX`), i.e. one walk run counted twice across the blank
    /// f305–f319 stretch where he is off the right edge — so the capture holds
    /// SIX walk runs in 120 s, one per chain pass, which is exactly the six
    /// gag runs the 2026-09-19 audio take has. The old expectation (mean ~4,
    /// no positional gate) is what made the port re-run the gag 1.5× too
    /// often; see `FT_WALK_EDGE_MARGIN`.
    #[test]
    fn father_time_walk_length_is_a_roll() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        m.set_control(0, 0);
        m.set_control(1, 50); // factory slider ⇒ band 2, p = 1/4
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut t = 0u64;
        let (mut counts, mut cur, mut walking) = (Vec::new(), 0u32, false);
        let (mut prev_seq, mut prev_frame) = (0u32, u32::MAX);
        // where each walk stopped — the h gate's own invariant
        let (mut ends, mut last_ax) = (Vec::new(), 0i32);
        while t < 600_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            let Some(ActiveClock::Ft(ft)) = &m.clock else { continue };
            if (ft.part.seq, ft.part.frame) == (prev_seq, prev_frame) {
                continue;
            }
            let fresh = ft.part.seq != prev_seq || ft.part.frame <= prev_frame;
            prev_seq = ft.part.seq;
            prev_frame = ft.part.frame;
            if ft.part.seq == FT_WALK_SEQ {
                if fresh {
                    cur += 1;
                }
                last_ax = ft.screen_ax(&m.pack);
                walking = true;
            } else if walking {
                counts.push(cur);
                ends.push(last_ax);
                cur = 0;
                walking = false;
            }
        }
        assert!(counts.len() >= 8, "too few walks in 10 min: {counts:?}");
        // drop the entry walk, which starts mid-cycle from the ctor's cue
        let later = &counts[1..];
        let mean = later.iter().sum::<u32>() as f64 / later.len() as f64;
        assert!(
            (7.0..15.0).contains(&mean),
            "band-2 walk mean {mean:.2}, want ~10.5 (p = 1/4 gated to 4 of 10 lap \
             positions): {later:?}"
        );
        assert!(later.contains(&1), "no one-cycle walk in {later:?} — the roll is not firing");
        assert!(later.iter().any(|c| *c >= 4), "no long walk in {later:?}");
        // the gate, not just the roll: every walk must END with the actor's
        // `h` inside `[100, 380]` — `@0x1326` forbids leaving it anywhere else.
        assert!(
            ends.iter().all(|h| (FT_WALK_EDGE_MARGIN..=SCREEN_H - FT_WALK_EDGE_MARGIN)
                .contains(h)),
            "a walk ended outside the 100..380 window the h gate allows: {ends:?}"
        );
        // …and a faster band makes him walk longer (p = 96−16·4 = 32/256)
        let mut m4 = build(pack_or_skip().unwrap()).unwrap();
        m4.set_control(0, 0);
        m4.set_control(1, 100);
        assert_eq!(m4.drift_band(), 4);
        assert!(FT_WALK_STOP_NUMER - FT_WALK_STOP_STEP * 4 < FT_WALK_STOP_NUMER);
    }

    /// One pose trace of the monkey at a chosen Drift Speed.
    fn mk_poses(ms: u64, drift: i32) -> Option<Vec<(u64, u32)>> {
        mk_poses_seeded(ms, drift, 1)
    }

    fn mk_poses_seeded(ms: u64, drift: i32, seed: u32) -> Option<Vec<(u64, u32)>> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, 2);
        m.set_control(1, drift);
        let mut ctx = ctx_at((12, 0, 0), 0);
        ctx.rng = RandomLong::new(seed.into());
        let mut out: Vec<(u64, u32)> = Vec::new();
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            if let Some(ActiveClock::Monkey(mk)) = &m.clock {
                if out.last().map(|e| e.1) != Some(mk.pose.seq) {
                    out.push((t, mk.pose.seq));
                }
            }
        }
        Some(out)
    }

    /// One sound trace of the monkey: `(t_ms, snd id)` for every cue the
    /// module pushes over `ms` of module time at a chosen Drift Speed and
    /// RNG seed. The wall clock is stepped from 12:00:00 so no hour rolls.
    fn mk_sounds(ms: u64, drift: i32, seed: u32) -> Option<Vec<(u64, u32)>> {
        let mut m = build(pack_or_skip()?)?;
        m.set_control(0, 2);
        m.set_control(1, drift);
        let mut ctx = ctx_at((12, 0, 0), 0);
        ctx.rng = RandomLong::new(seed.into());
        ctx.rng15 = Random15::new(seed | 1);
        let mut out: Vec<(u64, u32)> = Vec::new();
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            for snd in ctx.sounds.drain(..) {
                out.push((t, snd));
            }
        }
        Some(out)
    }

    /// The capture's length, so every rate assertion below is quoted against
    /// the same window the wav is: `emu/captures/qemu/shock-clocks.wav` runs
    /// 119.12 s of a 120 s wall clock (Shocked Monkey, Drift Slow).
    const QEMU_WAV_MS: u64 = 119_120;

    /// GOLDEN AUDIO 2026-09-13 — shared `snd 30002 zap_spark` is the monkey
    /// machine's five chassis arcs, and the port had no cue for it at all.
    ///
    /// Disasm: the scene composer arms each arc with the snd id in the same
    /// argument slot the Rotting Head's drips use for 30005 — `0x7532` for
    /// compounds 39/43/47/51/55 (@`0x40B6`/`0x4136`/`0x41B6`/`0x4236`/
    /// `0x42B6`) and `-1` for the three long spark runs 59/84/108
    /// (@`0x3F38`/`0x3FB8`/`0x4038`). Capture: 40 firings in 119.12 s at
    /// r = 0.62–0.81, first at t = 7.58 s (f152), the one the manifest reads
    /// on screen at t = 15.79 (f316, "monkey below the clock, blue electric
    /// glow and arcs").
    #[test]
    fn monkey_arcs_cue_the_shared_zap_spark() {
        let Some(v) = mk_sounds(QEMU_WAV_MS, 50, 1) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let zaps = v.iter().filter(|(_, s)| *s == SND_ZAP_SPARK).count();
        assert!(zaps > 0, "snd 30002 never fired — the capture hears it 40 times");
        // the five arcs sound, the three spark runs do not
        // (REWRITTEN 2026-09-29: this used to assert the invented per-part
        // gates `ARC_GATE`/`SPARK_GATE`; `fn_6F8A` has one gate, 8/1024, and
        // the sparks are LOOPERS — see `DRESS_GATE`.)
        for run in MK_ARCS {
            let (loops, snd) = dressing_args(run);
            match run {
                39 | 43 | 47 | 51 | 55 => {
                    assert_eq!(snd, Some(SND_ZAP_SPARK), "arc {run} lost its 0x7532");
                    assert!(!loops, "arc {run} is a one-shot (+0xA0 = 0)");
                }
                _ => {
                    assert_eq!(snd, None, "spark run {run} is armed with -1, it is silent");
                    assert!(loops, "spark run {run} loops (+0xA0 = 1)");
                }
            }
        }
        // …and at the capture's rate. 40 heard; five arcs rolling 8/1024 once
        // per DRESS_FRAME_MS queue ~45, so hold the band tight enough that a
        // return to 64/2048 (≈142) fails.
        assert!(
            (25..=70).contains(&zaps),
            "{zaps} zap_sparks in 119.12 s; the capture has 40 (t = 7.58 … 113.90)"
        );
    }

    /// GOLDEN AUDIO 2026-09-13 — the chatter table is the `@0x192A`
    /// dispatcher's, and every row of §7's was one gesture too high.
    ///
    /// The literals compared against `+0xA2` are `0x9A` = 154 (one arm,
    /// counter 2, @`0x1960`), `0xA2` = 162 (six counters, @`0x1970`), `0xE1`
    /// = 225, `0xCD` = 205, `0x105` = 261, `0xF5` = 245 — and `0x92` = 146 is
    /// not compared at all. The consequence the capture sees: seq 154 is 7
    /// frames, so on §7's reading counters 8/18/24/30/33 could never be
    /// reached and five of the six busiest chirps were dead code; seq 162 is
    /// 42 frames and holds the whole row.
    #[test]
    fn monkey_chatter_table_is_the_0x1906_dispatcher() {
        assert_eq!(chatter_counters(146), &[] as &[u32], "146 (0x92) has no dispatcher arm");
        assert_eq!(chatter_counters(154), &[2], "0x9A @0x1960 is the single counter-2 arm");
        assert_eq!(
            chatter_counters(162),
            &[5, 8, 18, 24, 30, 33],
            "0xA2 @0x1970 is the six-counter arm"
        );
        assert_eq!(chatter_counters(225), &[5, 9, 11]);
        assert_eq!(chatter_counters(205), &[5, 11]);
        assert_eq!(chatter_counters(261), &[4, 8, 10, 14]);
        assert_eq!(chatter_counters(245), &[4, 10]);

        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // every counter in a row must be reachable inside its own sequence
        for seq in [154u32, 162, 225, 205, 261, 245] {
            let len = pack.seq(BASE_MONKEY, seq).map(|s| s.frames.len() as u32).unwrap_or(0);
            let last = *chatter_counters(seq).last().unwrap();
            assert!(
                last < len,
                "seq {seq} is {len} frames but the table wants counter {last} — \
                 that is exactly the §7 shift this test guards"
            );
        }
        // and the six-counter row must span the capture's burst: gaps
        // 3/10/6/6/3 x MK_FRAME_MS = 0.36/1.20/0.72/0.72/0.36 s, measured at
        // wav t = 3.18 3.54 4.62 5.35 5.89 6.30 (f64…f126), again at
        // 71.68…74.62 and 86.49…89.18.
        let row = chatter_counters(162);
        let span = (row[row.len() - 1] - row[0]) as u64 * MK_FRAME_MS;
        assert_eq!(span, 3_360, "the 162 burst is 3.36 s wide; the capture measures 3.12");
    }

    /// GOLDEN AUDIO 2026-09-13 — the cue *colour* and the cue *mix*.
    ///
    /// The capture's 84 chirps split 22/18/19/25 over snds 1000–1003, which is
    /// the uniform `Random15() % 4` of `@0x1A0A` (χ² = 1.4 on 3 df); the shock
    /// pair is snd 1004/1005 only, and nothing else in the module's own bank
    /// is ever heard. This pins the id set the monkey may emit, so a future
    /// pass cannot quietly add a Father Time voice line to the monkey's path.
    ///
    /// RESIDUAL (see the module header): the port emits ~1.5× the capture's 84
    /// chirps. The band below is deliberately wide enough to pass that and
    /// tight enough to fail the pre-fix table, which could only reach ~50.
    /// (2026-09-29: with the `fn_59A0` priority gate the port sits at ~88 per
    /// 119 s — this Demo capture's panel ate chirps, so it reads low; the
    /// full-screen rate is pinned by `monkey_chirps_are_discarded_under_a_louder_cue`.)
    #[test]
    fn monkey_chirp_mix_matches_the_capture() {
        let mut chirps = [0u32; 4];
        let mut shocks = 0u32;
        let mut seeds = 0;
        for seed in [1u32, 7, 23, 101, 999] {
            let Some(v) = mk_sounds(QEMU_WAV_MS, 50, seed) else {
                eprintln!("assets/shock-clocks missing — skipping");
                return;
            };
            seeds += 1;
            for (_, s) in &v {
                match *s {
                    1000..=1003 => chirps[(*s - 1000) as usize] += 1,
                    SND_SHOCK1 | SND_SHOCK2 => shocks += 1,
                    SND_ZAP_SPARK => {}
                    other => panic!(
                        "monkey emitted snd {other}; the capture only has 1000-1005 + 30002"
                    ),
                }
            }
        }
        let total: u32 = chirps.iter().sum();
        let per_run = total / seeds;
        assert!(
            (85..=170).contains(&per_run),
            "{per_run} chirps per 119.12 s (capture: 84). Below ~85 means the \
             chatter table has slipped back onto the 7-frame seq 154."
        );
        // uniform rand%4: no colour may take less than a fifth of the bed
        for (i, n) in chirps.iter().enumerate() {
            assert!(
                *n * 5 > total,
                "snd {} is {n}/{total} of the chirp bed; the capture is 22/18/19/25",
                1000 + i
            );
        }
        assert!(shocks > 0, "no Shock1/Shock2 in 119 s; the capture has four");
    }

    /// CONTRADICTION SETTLED 2026-09-13 — the `rand%20` table is the 20-word
    /// window at `DATA129+0x4DE`, and its tail is 225/205. §7/§12.3's
    /// `{146,154,162}×6 + {146,154}` is the same 20 words read from
    /// **0x04D8**, the idle-triple base six bytes earlier. This test pins the
    /// distinction: the port's table must be the 0x4DE window (startle
    /// reachable), and must NOT be the 0x4D8 window (startle unreachable).
    #[test]
    fn monkey_idle20_is_the_0x4de_window_not_the_0x4d8_one() {
        // The overlapping run in DATA_129, words at 0x04D4 .. 0x0504.
        let run: [u32; 28] = [
            225, 205, // 0x04D4  startle pair (rand%2)
            146, 154, 162, // 0x04D8  idle triple (rand%3); the 0x4DE window starts next
            146, 154, 162, 146, 154, 162, 146, 154, 162, // 0x04DE .. 0x04EE
            146, 154, 162, 146, 154, 162, 146, 154, 162, // 0x04F0 .. 0x0500
            225, 205, // 0x0502
            261, 245, 277, // 0x0506 .. the gag chain
        ];
        let at = |byte_off: usize| -> [u32; 20] {
            let i = (byte_off - 0x4D4) / 2;
            run[i..i + 20].try_into().unwrap()
        };
        assert_eq!(MONKEY_IDLE20, at(0x4DE), "the table is the 0x4DE window");
        assert_eq!(
            at(0x4D8),
            [
                146, 154, 162, 146, 154, 162, 146, 154, 162, 146, 154, 162, 146, 154, 162,
                146, 154, 162, 146, 154
            ],
            "the 0x4D8 window is exactly the {{146,154,162}}x6 + {{146,154}} the spec prints"
        );
        assert_ne!(MONKEY_IDLE20, at(0x4D8), "the losing reading crept back");
        // and the consequence that makes the difference matter
        assert_eq!(
            MONKEY_IDLE20.iter().filter(|s| matches!(s, 225 | 205)).count(),
            2,
            "2/20 = the flat 1-in-10 shock; the 0x4D8 window has 0 and can never startle"
        );
    }

    /// The shock has no metronome: it comes out of the `rand % 20` idle table
    /// at `DATA129+0x4DE`, whose last two slots are 225 and 205
    /// (`MONKEY_IDLE20`). Over ten minutes with the wall clock never crossing
    /// an hour, the monkey must still shock repeatedly, at irregular spacing.
    #[test]
    fn monkey_shocks_without_an_hour_or_a_metronome() {
        let Some(trace) = mk_poses(600_000, 0) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let starts: Vec<u64> =
            trace.iter().filter(|e| matches!(e.1, 225 | 205)).map(|e| e.0).collect();
        assert!(starts.len() >= 10, "only {} shocks in 10 min", starts.len());
        let gaps: Vec<u64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = gaps.iter().sum::<u64>() as f64 / gaps.len() as f64 / 1000.0;
        // 3.0 s suppressed + Geom(1/10) idling + the gag itself. RESIDUAL: the
        // idle gestures are 7 / 7 / 42 frames at 120 ms, mean 2.24 s, so ten
        // of them is ~22 s and the modelled mean lands near 30 s against the
        // capture's 13.6 s over four samples (p ≈ 0.14 for four draws that low
        // out of a 30 s exponential). The mechanism is the binary's; the rate
        // wants a 5-minute monkey capture to confirm.
        assert!((5.0..60.0).contains(&mean), "mean shock gap {mean:.1} s: {gaps:?}");
        let spread = gaps.iter().max().unwrap() - gaps.iter().min().unwrap();
        assert!(spread > 8_000, "shock gaps are a metronome (spread {spread} ms): {gaps:?}");
        // The ctor must NOT arm the window, or nothing can shock for 3 s.
        // (REWRITTEN 2026-09-29: this used to read "seed 1's first shock is
        // under 60 s", a stream pin that broke as soon as the dressing parts
        // moved onto their own 100 ms CueAnim and drew from `ctx.rng` at a
        // different cadence. The property itself is "a startle can land
        // inside the first 3 s", so ask exactly that, over enough seeds that
        // the 1-in-10 first pick shows up.)
        let earliest = (0..40u32)
            .filter_map(|k| {
                let v = mk_poses_seeded(3_000, 0, 2 * k + 1)?;
                v.iter().find(|e| matches!(e.1, 225 | 205)).map(|e| e.0)
            })
            .min();
        assert!(
            earliest.is_some_and(|t| t < MK_SHOCK_WINDOW_MS),
            "no startle inside the first 3 s over 40 seeds — is the ctor arming +0xB0?"
        );
    }

    /// Leaving 282 arms a 3.0 s SUPPRESSION window (`MK_SHOCK_WINDOW_MS`), not
    /// a 50 s countdown to the next shock: no startle may begin within it.
    #[test]
    fn monkey_cannot_startle_inside_the_suppression_window() {
        let Some(trace) = mk_poses(600_000, 0) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let mut armed: Option<u64> = None;
        let mut leaving = false;
        let mut checked = 0;
        for (t, seq) in &trace {
            // the entry AFTER a 282 is the pose 282 handed on to, i.e. the
            // frame the window was armed on (@0x1D7C)
            if leaving {
                armed = Some(*t);
                leaving = false;
            }
            if *seq == 282 {
                leaving = true;
            }
            if matches!(seq, 225 | 205) {
                if let Some(a) = armed.take() {
                    checked += 1;
                    assert!(
                        t - a >= MK_SHOCK_WINDOW_MS,
                        "startle at {t} ms, only {} ms after leaving 282",
                        t - a
                    );
                }
            }
        }
        assert!(checked >= 5, "only {checked} armed windows observed");
        assert_eq!(MK_SHOCK_WINDOW_MS, 3_000);
    }

    /// Trace: every monkey transition, plus the census and the chirp rate the
    /// audio manifest can be checked against.
    /// `cargo test -p app -- --ignored --nocapture monkey_trace`
    #[test]
    #[ignore]
    fn monkey_trace() {
        let Some(pack) = pack_or_skip() else { return };
        let mut m = build(pack).unwrap();
        m.set_control(0, 2);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut census: std::collections::BTreeMap<u32, u32> = Default::default();
        let mut chirps = 0u32;
        let mut shocks = 0u32;
        let mut prev = 0;
        let span = 119_120u64; // the QEMU capture's length
        let mut t = 0u64;
        while t < span {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            ctx.sounds.clear();
            m.tick(&mut ctx);
            for s in &ctx.sounds {
                match *s {
                    1000..=1003 => chirps += 1,
                    1004 | 1005 => shocks += 1,
                    _ => {}
                }
            }
            if let Some(ActiveClock::Monkey(mk)) = &m.clock {
                *census.entry(mk.pose.seq).or_default() += 1;
                if mk.pose.seq != prev {
                    println!("{t:>7} ms  pose {:>3}  lcd {:?}", mk.pose.seq, mk.lcd);
                    prev = mk.pose.seq;
                }
            }
        }
        println!("census (module ticks per pose): {census:?}");
        println!(
            "over {:.2} s: {chirps} chirps ({:.3}/s), {shocks} shock cues",
            span as f64 / 1000.0,
            chirps as f64 * 1000.0 / span as f64
        );
        println!("capture (docs/emulator/audio-captures.md): 84 chirps (0.705/s), 6 shock cues");
    }

    /// One monkey run hand-off as the SCREEN shows it, read off `sprites()`
    /// only (no actor internals), plus what `L135 fn3F2E` computes for it.
    #[derive(Debug)]
    struct MkHandOff {
        t: u64,
        /// outgoing compound → incoming run
        from: u32,
        to: u32,
        /// the pose sprite's drawn top-left relative to the machine's
        /// (compound 1), minus the pack's authored bank-space offset
        /// `(bx + dx − 41, by + dy − 75)`: (0, 0) = drawn where the composite
        /// authors it. Before and after the hand-off.
        dev_before: (i32, i32),
        dev_after: (i32, i32),
        /// drawn mirrored, after the hand-off
        flip: bool,
        /// `fn3F2E`'s registration, carried with the C's own mirror bit
        reg: Option<Registration>,
        c_flip: bool,
    }

    /// Drive the monkey (band 2, the capture's Slow) from `start` for `ms`
    /// and log every run hand-off, including re-cues of the same run.
    fn mk_handoffs(start: (u8, u8, u8), ms: u64) -> Option<Vec<MkHandOff>> {
        let pack = pack_or_skip()?;
        let mut m = build(pack.clone())?;
        m.set_control(0, 2);
        m.set_control(1, 50);
        let mut ctx = ctx_at(start, 0);
        let base_s = u64::from(start.0) * 3600 + u64::from(start.1) * 60 + u64::from(start.2);
        let mach = pack.frame(BASE_MONKEY, 1)?.png.clone();
        // (compound no, deviation, drawn flip)
        let observe = |m: &ShockClocks| -> Option<(u32, (i32, i32), bool)> {
            let Some(ActiveClock::Monkey(mk)) = &m.clock else { return None };
            let no = mk.pose.seq + mk.pose.frame;
            let f = m.pack.frame(BASE_MONKEY, no)?;
            let mut v = Vec::new();
            m.sprites(&mut v);
            let ms = v.iter().find(|s| s.png == mach)?;
            let ps = v.iter().find(|s| s.png == f.png)?;
            let dev = (ps.x - ms.x - (f.bx + f.dx - 41), ps.y - ms.y - (f.by + f.dy - 75));
            Some((no, dev, ps.flip))
        };
        let mut out = Vec::new();
        let mut c_flip = false; // fn_17B4 clears +0x3C bit 0 after the first SetRun
        let mut prev = observe(&m);
        let mut prev_counter = 0;
        let mut t = 0u64;
        while t < ms {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = base_s + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            let now = observe(&m);
            let Some(ActiveClock::Monkey(mk)) = &m.clock else { continue };
            if let (Some(p), Some(n)) = (prev, now) {
                // a hand-off = the run was (re)cued this tick: the base tick
                // counts +0xA4 up every frame and SetRun zeroes it
                if mk.pose.counter == 0 && prev_counter > 0 {
                    let reg = register_shared_part(&pack, BASE_MONKEY, p.0, n.0, &mut c_flip);
                    out.push(MkHandOff {
                        t,
                        from: p.0,
                        to: mk.pose.seq,
                        dev_before: p.1,
                        dev_after: n.1,
                        flip: n.2,
                        reg,
                        c_flip,
                    });
                }
            }
            prev = now;
            prev_counter = mk.pose.counter;
        }
        Some(out)
    }

    /// Per-hand-off monkey trace (the METHOD trace for the linked-SetRun
    /// audit): `cargo test -p app mk_handoff_trace -- --ignored --nocapture`.
    /// Crosses 02:00 so the hour gag's 282 → zap re-entries are in it.
    #[test]
    #[ignore]
    fn mk_handoff_trace() {
        let Some(rows) = mk_handoffs((1, 59, 30), 300_000) else { return };
        for r in &rows {
            println!(
                "{:7} c_{:03} -> run {:3}  dev {:?} -> {:?}  drawn flip {:5}  fn3F2E {:?}  C flip {}",
                r.t, r.from, r.to, r.dev_before, r.dev_after, r.flip, r.reg, r.c_flip
            );
        }
        let off = rows.iter().filter(|r| r.dev_after != (0, 0)).count();
        let mism = rows.iter().filter(|r| r.flip != r.c_flip).count();
        println!("{} hand-offs, {off} drawn off the authored spot, {mism} mirror mismatches vs the C", rows.len());
    }

    /// RATCHET (2026-09-29 linked-SetRun audit). `CSpriteMonkey`'s vtable
    /// binds the same linked SetRun as Father Time (`fn0204` → `fn028A` →
    /// `fn3F2E`), but its StartStateSequence then re-pins through `fn_56DA`,
    /// so the registration's move is thrown away and only its mirror toggle
    /// could survive — and no hand-off in series 1000 toggles. See
    /// `ShockedMonkey::set_run`.
    ///
    /// GOLDEN (`shock-monkey-fullscreen.mp4`, the eight gags f590–f2920):
    /// every pose compound, plain and mirrored, masked-SSD-matched against
    /// the frame and measured relative to the machine (compound 1). Every
    /// clean match (masked SSD ≤ 1500, ~2x the median; the 32 rejected are
    /// the codec's ramp after the full-body flash) sits at the pose's
    /// authored spot, (0, 0): idle 146/154/162 ×67, startle 205/225 ×55, zap
    /// 245/261 ×144, 277 ×80, and — RGB, skeleton compounds only —
    /// 277/278/279 ×80 and 282/283 ×22. Mirrored beats plain in none of them.
    /// Over the whole 300 s take (all 3000 frames): 2964 clean, all 2964 at
    /// (0, 0), 0 mirrored. Keeping
    /// `fn3F2E`'s move (Father Time's model) would put 277 at **(−15, −11)**
    /// (zap → 277 shares no part, so the centre holds while the art jumps)
    /// and 282 at (−1, −3), creeping another (−1, −3) per gag.
    #[test]
    fn monkey_hand_offs_re_pin_and_never_mirror() {
        let Some(rows) = mk_handoffs((1, 59, 30), 300_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // the trace must reach every kind of hand-off, including the ones
        // where fn3F2E asks for a move and the ones where it finds no part
        let to = |r: u32| rows.iter().filter(|h| h.to == r).count();
        for r in [146, 154, 162, 205, 225, 245, 261, 277, 282] {
            assert!(to(r) > 0, "no hand-off into run {r} in 300 s");
        }
        assert!(rows.iter().any(|h| h.from == 283 && matches!(h.to, 245 | 261)), "the hour gag never re-zapped");
        let moved = rows.iter().filter(|h| h.reg.is_some_and(|r| r.delta != (0, 0))).count();
        let none = rows.iter().filter(|h| h.reg.is_none()).count();
        assert!(moved > 0 && none > 0, "fn3F2E moved {moved} / found no part {none}");
        for h in &rows {
            // capture: every pose at its authored spot relative to the machine
            assert_eq!(
                h.dev_after,
                (0, 0),
                "c_{:03} -> run {} drew {:?} off its authored spot at t = {} ms (capture: (0, 0); \
                 fn3F2E {:?} must be overwritten by the fn_56DA re-pin)",
                h.from,
                h.to,
                h.dev_after,
                h.t,
                h.reg
            );
            // capture: never mirrored; the C: never toggled
            assert!(!h.c_flip, "fn3F2E toggled the mirror on c_{:03} -> {}", h.from, h.to);
            assert_eq!(h.flip, h.c_flip, "drawn mirror differs from the C's at c_{:03} -> {}", h.from, h.to);
        }
    }

    /// RATCHET — **the monkey's hour gag is repeated ZAPS.** `fn_1A7A`'s 282
    /// arm (@`0x1CC0`) only leaves the gag when `+0xAA` has counted out
    /// `hour % 12`; until then it returns another `{261,245}` pick and jumps
    /// straight to the StartStateSequence tail (@`0x1DD8`), skipping the bar
    /// stop, the `+0xB0` stamp and the LCD flash entirely.
    ///
    /// Drive the clock across 02:00 and assert the monkey runs at least two
    /// consecutive zap cycles with no idle pose between them. Reverting the
    /// arm to "always pick an idle" leaves exactly one and this fails.
    #[test]
    fn monkey_hour_gag_is_repeated_zaps() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        m.set_control(0, 2);
        m.set_control(1, 0);
        // start at 01:59:55 so the hour rolls five seconds in; hour 2 owes
        // two cuckoos.
        let mut ctx = ctx_at((1, 59, 55), 0);
        let mut poses: Vec<u32> = Vec::new();
        let mut t = 0u64;
        while t < 120_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 1 * 3600 + 59 * 60 + 55 + t / 1000;
            ctx.local_hms =
                (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            if let Some(ActiveClock::Monkey(mk)) = &m.clock {
                if poses.last() != Some(&mk.pose.seq) {
                    poses.push(mk.pose.seq);
                }
            }
        }
        // the longest stretch of gag poses with no idle in it
        let mut best = 0;
        let mut run = 0;
        for p in &poses {
            if matches!(p, 225 | 205 | 261 | 245 | 277 | 282) {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
        }
        // one cycle is at most startle + zap + n*277 + 282; two cuckoos means
        // a second 261/245 arrives with no idle between, so the stretch has to
        // contain two 282s.
        let mut twos = 0;
        let mut seen282 = false;
        let mut streak: Vec<u32> = Vec::new();
        for p in &poses {
            if matches!(p, 225 | 205 | 261 | 245 | 277 | 282) {
                streak.push(*p);
            } else {
                if streak.iter().filter(|x| **x == 282).count() >= 2 {
                    twos += 1;
                }
                streak.clear();
            }
            seen282 |= *p == 282;
        }
        assert!(seen282, "the monkey never finished a gag: {poses:?}");
        assert!(best >= 4, "no gag longer than {best} poses");
        assert!(twos >= 1, "the hour never produced back-to-back zaps: {poses:?}");
    }

    /// RATCHET — **the head's minute flag is swallowed on the hour.**
    /// `fn_22E8` (@`0x22F8`) is `if (hour changed) {...} else if (minute
    /// changed) {...}`, so the two events can never be raised on the same
    /// frame. Feed a frame where both change and assert only the hour lands.
    #[test]
    fn head_minute_flag_is_swallowed_on_the_hour() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // (a) the hour rolls: minute rolls with it, and only the hour lands
        let mut rh = RottingHead::new(&pack, 9, 59);
        let mut ctx = ctx_at((10, 0, 0), 0);
        rh.tick(&mut ctx, &pack);
        // the eye's one-frame rest run ends on the same tick, so the chooser
        // consumes the flag immediately: the hour sends it to 20 (cuckoo).
        assert_eq!(rh.eye.seq, 20, "the hour did not start the cuckoo");
        assert!(rh.hour_flag, "the hour was not latched");
        assert!(!rh.minute_flag, "the minute flag rose on the hour");
        assert_eq!(rh.last_minute, 59, "the minute was latched on the hour frame");
        // (b) a plain minute rolls: the minute lands and the hour does not
        let mut rh = RottingHead::new(&pack, 10, 0);
        let mut ctx = ctx_at((10, 1, 0), 0);
        rh.tick(&mut ctx, &pack);
        assert!(!rh.hour_flag);
        // the minute sends it to 11 (the blink) and the chooser clears the
        // flag as it goes, so look at where it landed.
        assert_eq!(rh.eye.seq, 11, "the minute did not start the blink");
        // (c) and the noon quirk: hour%12 == 0 with a non-zero hour leaves the
        // count at 0, so the head cuckoos exactly ONCE at 12:00 (@0x24DE)
        let mut rh = RottingHead::new(&pack, 11, 59);
        rh.last_hour = 12;
        rh.hour_flag = true;
        rh.cuckoo_count = 0;
        rh.eye.cue(&pack, BASE_RH, 20);
        assert_eq!(rh.eye_next(), 9);
        assert!(!rh.hour_flag, "noon cuckooed more than once");
        // …against midnight, which cuckoos twelve times
        let mut rh = RottingHead::new(&pack, 23, 59);
        rh.last_hour = 0;
        rh.hour_flag = true;
        for i in 1..=12 {
            rh.eye.cue(&pack, BASE_RH, 20);
            assert_eq!(rh.eye_next(), 9);
            assert_eq!(rh.hour_flag, i < 12, "midnight stopped after {i}");
        }
    }

    /// MOTION CONTINUITY (§5 ratchet, the mime-hunt shape). Every actor's
    /// drawn top-left, module frame by module frame, with the DRIFT REMOVED —
    /// the drift is the assembly gliding, not a sprite hand-off.
    ///
    /// The bound comes out of the C, not out of a fit. `fn_572E` (@`0x572E`,
    /// vtbl+0x148, the StartStateSequence every `ChooseNextSequence` tail-
    /// calls) does SetRun and then vtbl+**0x144**, and the BASE implementation
    /// of +0x144 is `fn_56DA` (@`0x56DA`): take the scene's centre
    /// (scene vtbl+0x94) and SetPos the part there. So for CSpriteMonkey,
    /// CSpriteMouth, CSpriteRightEye and CSpriteTooth every run starts pinned
    /// to the same point and NOTHING travels between runs, however the
    /// library's `fn_0204`→`fn_028A` linked SetRun (+0x7C, §7.1's second
    /// hand-off model, which this module's base vtable `g0F8A` does bind)
    /// would have moved it — the recentre overwrites it.
    ///
    /// Only two classes override +0x144 and therefore keep travel:
    ///
    /// - **gCSpriteFlasher** — `fn_1594` (@`0x1594`) repositions ONLY when
    ///   vtbl+0x34 says the sprite is not running, i.e. once at construction
    ///   and after a stop. That is why Father Time's walk carries its x
    ///   across chain passes (`FT_WALK_STEP` = 76 px per cycle, golden).
    /// - **CSpriteEar** — `fn_0CE8` (@`0x0CE4`) recentres and then adds the
    ///   accumulated fall `+0xA8`, which grows by `+0xA6` (itself `+= 50` per
    ///   ground frame) — an accelerating drop, up to `EAR_FALL_LIMIT`.
    ///
    /// CAPTURE (`emu/captures/qemu/shock-clocks.mp4`, Shocked Monkey, 1200
    /// frames at 10 fps, phase-correlated frame to frame over a ±3 px search):
    /// the whole scene translates by exactly **(−1,+1) or (+1,−1) per 100 ms**
    /// — 558 of 599 steps in the first 60 s, the rest (±2,±2) or (0,0) from
    /// capture-timing jitter — and never more than 2 px on either axis. The
    /// bbox left reverses at 0 and 245 and nowhere else. That is the drift
    /// (`drift_glides_diagonally_and_bounces` pins it); with the drift removed
    /// there is no residual sprite motion in the capture at all, which is what
    /// this test asserts.
    /// The 1200-series ear compounds: rings 43..85 and 87..129.
    fn is_ear_art(png: &str) -> bool {
        png.rsplit("c_")
            .next()
            .and_then(|t| t.strip_suffix(".png"))
            .and_then(|t| t.parse::<u32>().ok())
            .is_some_and(|n| (43..=85).contains(&n) || (87..=129).contains(&n))
            && png.contains("/1200/")
    }

    /// The drawn origin of one part, the way `Part::draw` computes it.
    fn part_origin(pack: &Pack, base: u32, part: &Part) -> Option<(i32, i32)> {
        if !part.playing {
            return None;
        }
        let f = pack.frame(base, part.seq + part.frame)?;
        Some((f.bx + f.dx, f.by + f.dy))
    }

    /// The largest step the PACK authors for a class: within a run, frame to
    /// frame, and across a hand-off, last frame of one run to first frame of
    /// any other the class can reach.
    ///
    /// Both are authored teleports here. Every run start goes through
    /// `fn_572E` → vtbl+0x144, and the base `fn_56DA` (@`0x56DA`) re-pins the
    /// part through the SCENE's vtbl+0x94 — which is not a naive centre but a
    /// per-compound anchor (the monkey scene's `fn_397A` @`0x397A` resolves it
    /// against compound `0x11D` = 285, the full 427×198 machine plate; the
    /// head scene's is `fn_6214` @`0x6214`). So the original places each part
    /// at its authored spot in the composite exactly as this port's bank-space
    /// placement does, and nothing accumulates between runs. Anything the port
    /// shows ABOVE this ceiling is a link applied twice.
    fn authored_max_step(pack: &Pack, base: u32, runs: &[u32]) -> i32 {
        let org = |f: &engine::FrameRef| (f.bx + f.dx, f.by + f.dy);
        let step = |a: (i32, i32), b: (i32, i32)| (b.0 - a.0).abs().max((b.1 - a.1).abs());
        let mut worst = 0;
        for r in runs {
            let Some(sq) = pack.seq(base, *r) else { continue };
            for w in sq.frames.windows(2) {
                worst = worst.max(step(org(&w[0]), org(&w[1])));
            }
            for r2 in runs {
                let Some(sq2) = pack.seq(base, *r2) else { continue };
                worst = worst.max(step(org(sq.frames.last().unwrap()), org(&sq2.frames[0])));
            }
        }
        worst
    }

    /// Per-tick Father Time trace across the walk-stop beat (the METHOD
    /// trace for the 2026-09-26 "blinked to the left" report): state, frame,
    /// drawn left, centre, flip. `cargo test -p app ft_stop_trace --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn ft_stop_trace() {
        let Some(mut m) = pack_or_skip().and_then(build) else { return };
        m.set_control(0, 0);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut last = None;
        let mut t = 0u64;
        while t < 16_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            m.tick(&mut ctx);
            if let Some(ActiveClock::Ft(ft)) = &m.clock {
                let no = ft.part.seq + ft.part.frame;
                let left = ft.screen_ax(&m.pack) + m.pack.frame(BASE_FT, no).map_or(0, |f| f.bx + f.dx);
                let row = (ft.part.seq, ft.part.frame, left, ft.pos, ft.flip);
                if last != Some(row) {
                    println!(
                        "{t:6} seq {:3} d6 {:2} c_{no:03} left {left:4} pos {:?} flip {}",
                        row.0, row.1, row.3, row.4
                    );
                    last = Some(row);
                }
            }
        }
    }

    /// RATCHET (2026-09-26, Jason: "Father Time may have just blinked to the
    /// left hand side of the screen mid walk"). The port drew every non-walk
    /// run at the fixed `FT_AX` anchor, so a walk that ended anywhere but
    /// ax 52 snapped him to x 262 — motion_lint's 143.5 px teleports at
    /// 46.5 / 177.9 / 199.2 s. `fn028A`'s shared-part registration keeps him
    /// where he stopped. Every number below is `shock-father-time.mp4`'s
    /// (NCC >= 0.98 template matches, 10 fps): c_010 left 88 -> c_013 left
    /// 102 (t 56.4 -> 56.5), c_017 -> c_021 left 102 -> 100, c_139 left 113
    /// -> c_001 left 96 (t 63.9 -> 64.0), and the open coat is drawn
    /// MIRRORED (f611: c_058 flipped NCC 0.966 vs 0.832 plain).
    #[test]
    fn father_time_stops_where_he_stands() {
        let Some(mut m) = pack_or_skip().and_then(build) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        m.set_control(0, 0);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        // (compound, drawn left, flip) at every change
        let mut seen: Vec<(u32, i32, bool)> = Vec::new();
        let mut t = 0u64;
        while t < 300_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            let Some(ActiveClock::Ft(ft)) = &m.clock else { continue };
            let no = ft.part.seq + ft.part.frame;
            let left = ft.screen_ax(&m.pack) + m.pack.frame(BASE_FT, no).map_or(0, |f| f.bx + f.dx);
            if seen.last().map(|e| (e.0, e.1)) != Some((no, left)) {
                seen.push((no, left, ft.flip));
            }
        }
        let (mut stops, mut resumes, mut coats) = (0, 0, 0);
        for w in seen.windows(2) {
            let d = w[1].1 - w[0].1;
            match (w[0].0, w[1].0) {
                (10, 13) => {
                    stops += 1;
                    assert_eq!(d, 14, "walk stop moved him {d} px (capture: +14)");
                }
                (15 | 19, 21) => assert_eq!(d, -2, "->21 moved him {d} px (capture: -2)"),
                (139, 1) => {
                    resumes += 1;
                    assert_eq!(d, -17, "walk resume moved him {d} px (capture: -17)");
                }
                _ => {}
            }
            if (51..=129).contains(&w[1].0) {
                coats += 1;
                assert!(w[1].2, "open coat c_{:03} drawn unmirrored", w[1].0);
            }
        }
        assert!(stops >= 5 && resumes >= 5 && coats > 0, "{stops} stops / {resumes} resumes / {coats}");
    }

    /// Every packed run start of a series.
    fn pack_runs(pack: &Pack, base: u32) -> Vec<u32> {
        pack.series(base).iter().map(|s| s.first).collect()
    }

    /// The largest frame-centre step the pack authors between two adjacent
    /// frames of one run (series 1100 is centre-linked, port-plan §2).
    fn authored_max_centre_step(pack: &Pack, base: u32, runs: &[u32]) -> i32 {
        let mut worst = 0;
        for r in runs {
            let Some(sq) = pack.seq(base, *r) else { continue };
            for w in sq.frames.windows(2) {
                let c = |f: &engine::FrameRef| f.bx + f.dx + (f.w >> 1);
                worst = worst.max((c(&w[1]) - c(&w[0])).abs());
            }
        }
        worst
    }

    #[test]
    fn run_hand_offs_are_continuous() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // ---- Shocked Monkey: one pose part, every run it can reach ----
        let mk_runs: Vec<u32> = MONKEY_IDLE20
            .iter()
            .chain(MK_ZAP.iter())
            .chain(MK_TWITCH.iter())
            .copied()
            .collect();
        let authored = authored_max_step(&pack, BASE_MONKEY, &mk_runs);
        assert!(authored > 0, "the pack authors no motion at all");
        let mut m = build(pack.clone()).unwrap();
        m.set_control(0, 2);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let (mut worst, mut handoffs, mut frames) = (0i32, 0u32, 0u32);
        let mut origins: Vec<((u32, u32), (i32, i32))> = Vec::new();
        let mut prev: Option<(u32, (i32, i32))> = None;
        let mut t = 0u64;
        while t < 300_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            let Some(ActiveClock::Monkey(mk)) = &m.clock else { continue };
            let Some(o) = part_origin(&pack, BASE_MONKEY, &mk.pose) else { continue };
            if let Some((pseq, (px, py))) = prev {
                frames += 1;
                if pseq != mk.pose.seq {
                    handoffs += 1;
                }
                worst = worst.max((o.0 - px).abs().max((o.1 - py).abs()));
            }
            origins.push(((mk.pose.seq, mk.pose.frame), o));
            prev = Some((mk.pose.seq, o));
        }
        assert!(frames > 2_000, "only {frames} monkey frames");
        assert!(handoffs > 100, "only {handoffs} monkey hand-offs");
        // THE INVARIANT: with every run start re-pinned by fn_56DA, a part's
        // drawn origin is a pure function of (run, frame) — no travel is
        // carried across a hand-off and nothing accumulates inside one. A
        // link applied twice shows up here as the same (run, frame) landing
        // in two different places.
        let mut seen: std::collections::HashMap<(u32, u32), (i32, i32)> =
            std::collections::HashMap::new();
        for (k, o) in &origins {
            if let Some(p) = seen.get(k) {
                assert_eq!(
                    p, o,
                    "monkey run {} frame {} drew at {o:?} and earlier at {p:?} — travel is \
                     being carried across a hand-off",
                    k.0, k.1
                );
            } else {
                seen.insert(*k, *o);
            }
        }
        assert!(seen.len() > 100, "only {} distinct (run, frame) pairs", seen.len());
        assert!(
            worst <= authored,
            "the monkey's worst per-frame step is {worst} px, the pack authors {authored}"
        );
        // and it is not a stuck sprite either: the worst step has to be the
        // scale of a real hand-off, not a one-pixel wobble.
        assert!(worst >= 8, "the monkey never actually changed pose: {worst} px");

        // ---- Rotting Head: eye, mouth and both teeth (the ears travel by
        // design and have their own ratchet) ----
        let rh_runs: [&[u32]; 4] = [&[9, 11, 20], &[MOUTH_REST, MOUTH_OPEN], &TOOTH_RING[0], &TOOTH_RING[1]];
        let mut m = build(pack.clone()).unwrap();
        m.set_control(0, 1);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut worst = [0i32; 4];
        let mut prev: [Option<(i32, i32)>; 4] = [None; 4];
        let mut handoffs = 0u32;
        let mut t = 0u64;
        while t < 300_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            let Some(ActiveClock::Rh(rh)) = &m.clock else { continue };
            let parts = [&rh.eye, &rh.mouth, &rh.teeth[0].part, &rh.teeth[1].part];
            for (i, part) in parts.iter().enumerate() {
                let Some(o) = part_origin(&pack, BASE_RH, part) else { continue };
                if let Some((px, py)) = prev[i] {
                    let d = (o.0 - px).abs().max((o.1 - py).abs());
                    if d > 0 {
                        handoffs += 1;
                    }
                    worst[i] = worst[i].max(d);
                }
                prev[i] = Some(o);
            }
        }
        assert!(handoffs > 200, "the head barely moved: {handoffs}");
        for (i, runs) in rh_runs.iter().enumerate() {
            let authored = authored_max_step(&pack, BASE_RH, runs);
            assert!(
                worst[i] <= authored.max(1),
                "head part {i} stepped {} px, the pack authors {authored}",
                worst[i]
            );
        }

        // ---- Father Time: the one actor the original lets travel ----
        let mut m = build(pack).unwrap();
        m.set_control(0, 0);
        m.set_control(1, 50);
        let mut ctx = ctx_at((12, 0, 0), 0);
        // Every run hand-off is `fn028A`'s shared-part registration, so the
        // DRAWN frame's centre may only move by what the pack authors between
        // adjacent frames — the stop, the coat, the resume included. The one
        // exception is the off-screen wrap back to the entry offset.
        let ft_runs: Vec<u32> = pack_runs(&m.pack, BASE_FT);
        let authored = authored_max_centre_step(&m.pack, BASE_FT, &ft_runs);
        let mut prev: Option<(i32, i32)> = None;
        let (mut worst, mut moved, mut wraps) = (0, 0, 0);
        let mut t = 0u64;
        while t < 300_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let sec = 12 * 3600 + t / 1000;
            ctx.local_hms = (((sec / 3600) % 24) as u8, ((sec / 60) % 60) as u8, (sec % 60) as u8);
            m.tick(&mut ctx);
            if let Some(ActiveClock::Ft(ft)) = &m.clock {
                let ax = ft.screen_ax(&m.pack);
                if let Some((p, pax)) = prev {
                    let d = (ft.pos.0 - p).abs();
                    if ax == FT_WALK_ENTER_AX && pax >= FT_WALK_WRAP_AX - FT_WALK_STEP - 24 {
                        wraps += usize::from(d > 0);
                    } else {
                        worst = worst.max(d);
                        moved += usize::from(d > 0);
                    }
                }
                prev = Some((ft.pos.0, ax));
            }
        }
        assert!(moved > 100, "Father Time never moved");
        assert!(wraps >= 1, "he never walked off the right edge");
        assert!(
            worst <= authored,
            "Father Time's drawn centre stepped {worst} px, the pack authors at most {authored}"
        );
    }

    /// §8 drift is a bouncing 45° glide, one `DRIFT_AMP²` step per band
    /// interval, reflecting off the screen rect inset by the assembly.
    ///
    /// GOLDEN (`shock-monkey.mp4`, 900 module frames): 1 px in x AND y every
    /// 100 ms, left reversing at 0 and 244, top at 2 and 296 — i.e. the
    /// assembly's 395×184 rect inside 640×480. The port used to teleport to a
    /// fresh random offset every interval.
    #[test]
    fn drift_glides_diagonally_and_bounces() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let mut m = build(pack).unwrap();
        m.set_control(0, 2); // Shocked Monkey
        m.set_control(1, 50); // factory slider ⇒ band 2 = 1 px / 100 ms
        assert_eq!(m.drift_band(), 2);
        assert_eq!(DRIFT_INTERVAL_MS[2], 100);
        assert_eq!(DRIFT_AMP[2], 1);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let mut path: Vec<(i32, i32)> = Vec::new();
        let mut t = 0u64;
        while t < 120_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            m.tick(&mut ctx);
            if let Some(p) = m.drift_pos {
                if path.last() != Some(&p) {
                    path.push(p);
                }
            }
        }
        assert!(path.len() > 1000, "drift barely moved: {} samples", path.len());
        let (mut turns_x, mut turns_y) = (0, 0);
        for w in path.windows(2) {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            assert_eq!(dx.abs(), 1, "x step {dx}, want ±1");
            assert_eq!(dy.abs(), 1, "y step {dy}, want ±1 (the glide is 45°)");
        }
        for w in path.windows(3) {
            if (w[1].0 - w[0].0) != (w[2].0 - w[1].0) {
                turns_x += 1;
            }
            if (w[1].1 - w[0].1) != (w[2].1 - w[1].1) {
                turns_y += 1;
            }
        }
        assert!(turns_x >= 1 && turns_y >= 1, "never reflected in 120 s");
        let (xs, ys): (Vec<i32>, Vec<i32>) = path.iter().cloned().unzip();
        // 640 − 395 = 245 and 480 − 184 = 296 are the exclusive limits
        assert!(*xs.iter().min().unwrap() >= 0 && *xs.iter().max().unwrap() <= 244);
        assert!(*ys.iter().min().unwrap() >= 0 && *ys.iter().max().unwrap() <= 295);
        // and one of them must actually reach its wall, or the limits are
        // wider than the original's
        assert!(*xs.iter().max().unwrap() == 244 || *ys.iter().max().unwrap() == 295);
    }

    /// Drift is per-control: `shock-hour.mp4` is 120 s of a band-2 Father Time
    /// that never translates a pixel (frame 890 vs 1130 correlate at
    /// dx = dy = 0), while `shock-monkey.mp4`'s band-2 monkey glides 10 px/s.
    /// Only the monkey registers an assembly rect — see `assembly`.
    #[test]
    fn only_the_monkey_drifts() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        for (type_index, base) in [(0i32, (FT_AX, FT_AY)), (1, (102, 100))] {
            let mut m = build(Pack::load(pack.root()).unwrap()).unwrap();
            m.set_control(0, type_index);
            m.set_control(1, 50);
            let mut ctx = ctx_at((12, 0, 0), 0);
            let mut t = 0u64;
            while t < 60_000 {
                t += TICK_MS;
                ctx.now_ms = t;
                m.tick(&mut ctx);
                assert_eq!(m.anchor(), base, "clock {type_index} drifted at t = {t} ms");
            }
        }
    }

    /// The two §8 band tables, read raw out of `DATA129`. Band 0 is a ZERO in
    /// both — §8 printed the amplitude table as `{1,1,2,2,…}` — and the step
    /// is `amp²`, which makes the "non-monotone" interval row monotone in
    /// speed: 0 / 5 / 10 / 33 / 67 px per second.
    #[test]
    fn drift_band_tables_are_monotone_in_speed() {
        assert_eq!(DRIFT_INTERVAL_MS, [0, 200, 100, 120, 60]);
        assert_eq!(DRIFT_AMP, [0, 1, 1, 2, 2]);
        let speed = |b: usize| {
            if DRIFT_INTERVAL_MS[b] == 0 {
                0.0
            } else {
                (DRIFT_AMP[b] * DRIFT_AMP[b]) as f64 * 1000.0 / DRIFT_INTERVAL_MS[b] as f64
            }
        };
        for b in 1..5 {
            assert!(speed(b) > speed(b - 1), "band {b} is not faster than {}", b - 1);
        }
        assert_eq!(speed(2), 10.0); // the capture's 1 px / 100 ms
    }

    /// Father Time's belly dial is code-drawn too — CODE 129's composer
    /// `fn_3204` measures compound 62's art-27 placement (215,166,248,198),
    /// puts the hub at its centre (231,182) and cuts three 1-point hands of
    /// 6 / 10 / 13 px (@0x3358–0x35BA). Without them the module draws the
    /// packed face, which is a blank white disc — USER REPORT 2026-09-01:
    /// "the old dude has a solid white circle for a clock".
    #[test]
    fn father_time_dial_carries_hands() {
        let Some(_) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        // walk the chain until he lands on a coat-open pose. GOLDEN: the coat
        // flashes ~7 s in (capture t 7.77 start, 14.83 c_052) and stays open
        // for the rest of the chain, so a couple of chain passes is plenty.
        let mut open = None;
        let mut closed = None;
        for sec in 1..=40u64 {
            let (m, pngs) = run(0, sec * 1_000).unwrap();
            let no = match &m.clock {
                Some(ActiveClock::Ft(ft)) => ft.part.seq + ft.part.frame,
                _ => unreachable!(),
            };
            if FT_DIAL_COMPOUNDS.contains(&no) || no == FT_DIAL_COMPOUND_CUCKOO_END {
                open.get_or_insert((no, pngs));
            } else {
                closed.get_or_insert((no, pngs));
            }
        }
        let (no, pngs) = open.expect("Father Time never opened his coat in 40 s");
        assert!(
            pngs.iter().any(|p| p.contains("hand/1100/h_")),
            "no hour hand on the open dial (compound {no}): {pngs:?}"
        );
        assert!(pngs.iter().any(|p| p.contains("hand/1100/m_")), "no minute hand: {pngs:?}");
        assert!(pngs.iter().any(|p| p.contains("hand/1100/s_")), "no second hand: {pngs:?}");
        let (no, pngs) = closed.expect("Father Time never closed his coat in 40 s");
        assert!(
            !pngs.iter().any(|p| p.contains("hand/1100/")),
            "hands drawn on the buttoned coat (compound {no}): {pngs:?}"
        );
    }

    /// Both dials read the same wall clock, at the same angles — the hand
    /// canvases differ only in size (FT rad 16, RH rad 36) and taper.
    #[test]
    fn father_time_hands_track_local_hms() {
        let Some(pack) = pack_or_skip() else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let at = |hms| hand_sprites(&FT_DIAL, hms);
        let d = at((3, 17, 42));
        assert!(d.iter().any(|p| p.ends_with("hand/1100/h_197")), "hour hand: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("hand/1100/m_017")), "minute hand: {d:?}");
        assert!(d.iter().any(|p| p.ends_with("hand/1100/s_042")), "second hand: {d:?}");
        assert_eq!(d, at((15, 17, 42)), "hour hand did not fold mod 12");
        assert_ne!(d, at((3, 17, 43)), "second hand frozen");
        assert_ne!(d, at((3, 18, 42)), "minute hand frozen");
        assert_ne!(d, at((4, 17, 42)), "hour hand frozen");
        // the two dials must not share a cache slot: different geometry
        assert!(RH_DIAL.rad != FT_DIAL.rad && RH_DIAL.base != FT_DIAL.base);
        // §5's records: 1-point lines at D4/2, D4-3, D4 with D4 = 32/2 - 3
        let d4 = 32 / 2 - 3;
        assert_eq!(FT_DIAL.hands[0].len as i32, d4 / 2);
        assert_eq!(FT_DIAL.hands[1].len as i32, d4 - 3);
        assert_eq!(FT_DIAL.hands[2].len as i32, d4);
        assert!(FT_DIAL.hands.iter().all(|h| h.points == 1));
        assert!(RH_DIAL.hands.iter().all(|h| h.points == 3));
    }
    // -----------------------------------------------------------------
    // Father Time cue-schedule ratchet — the 2026-09-19 audio golden
    // -----------------------------------------------------------------

    /// One 120 s headless run of Father Time at the panel defaults, driven on
    /// the module's own clock through [`Pacer`] (no fixed stepping), returning
    /// `(now_ms, snd)` for every cue it fires. Wall time is held at noon so no
    /// hour flag is raised — the canonical golden take spans no rollover
    /// either.
    fn ft_cue_log(seed: u64, secs: u64) -> Vec<(u64, u32)> {
        let Some(pack) = pack_or_skip() else { return Vec::new() };
        let Some(mut m) = make(pack) else { return Vec::new() };
        let mut ctx = Ctx {
            rng: RandomLong::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1),
            rng15: Random15::new(seed as u32 | 1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut pace = Pacer::new(m.as_ref());
        let mut out = Vec::new();
        while pace.now_ms() < secs * 1000 {
            let now = pace.advance(&mut ctx);
            m.tick(&mut ctx);
            for s in ctx.sounds.drain(..) {
                out.push((now, s));
            }
        }
        out
    }

    /// GOLDEN `emu/captures/qemu/shock-father-time{,-hour}` (2026-09-19),
    /// 270 s of tabulated cue onsets across the canonical 120 s take and the
    /// 150 s hour take, scaled to 120 s. Father Time plays these four voice
    /// lines and nothing else; the cuckoo is hour-only and absent here.
    const FT_GOLDEN_PER_120S: [(u32, f64); 4] =
        [(SND_HEY_YOU, 5.78), (SND_HMM, 4.00), (SND_LAUGH2, 10.67), (SND_WANNA_KNOW, 4.89)];

    /// **THE cue-schedule ratchet.** Every voice line has to fire at the
    /// capture's rate, within ±30 %, pooled over enough seeds that the
    /// geometric walk and the 94⇄103 ping-pong average out.
    ///
    /// This is what Jason's 2026-09-19 "fires audio in rounds, chasing each
    /// other" measures: before `FT_WALK_EDGE_MARGIN` was corrected to read the
    /// actor's `h`, this test's four rates were 8.6 / 6.1 / 13.9 / 5.3 against
    /// the golden's 5.78 / 4.00 / 10.67 / 4.89 — every gag run arriving 1.5×
    /// too soon because the silent walk between them was a bare geometric
    /// 4 cycles instead of the ~11 the horizontal gate forces.
    #[test]
    fn father_time_cue_rate_matches_the_audio_golden() {
        if pack_or_skip().is_none() {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        }
        const SEEDS: u64 = 48;
        const SECS: u64 = 120;
        let mut counts: HashMap<u32, usize> = HashMap::new();
        for seed in 1..=SEEDS {
            for (_, s) in ft_cue_log(seed, SECS) {
                *counts.entry(s).or_default() += 1;
            }
        }
        // Nothing but the four voice lines: the chirp bed, the shocks, the
        // zap and snd 1200 are the monkey's and the Rotting Head's, and all
        // three takes score them at r <= 0.24.
        let allowed: Vec<u32> = FT_GOLDEN_PER_120S.iter().map(|(id, _)| *id).collect();
        let mut stray: Vec<(u32, usize)> =
            counts.iter().filter(|(id, _)| !allowed.contains(id)).map(|(a, b)| (*a, *b)).collect();
        stray.sort();
        assert!(stray.is_empty(), "Father Time fired cues the capture never has: {stray:?}");

        for (id, want) in FT_GOLDEN_PER_120S {
            let got = counts.get(&id).copied().unwrap_or(0) as f64 / SEEDS as f64;
            assert!(
                got >= want * 0.7 && got <= want * 1.3,
                "snd {id}: {got:.2} per 120 s, golden {want:.2} (+/-30% = {:.2}..{:.2})",
                want * 0.7,
                want * 1.3
            );
        }
    }

    /// The other half of the eye report: the *shape* of the Laugh2 burst.
    ///
    /// GOLDEN: 18 `1102 -> 1102` gaps under 6 s across the three takes —
    /// 0.55, 0.64, 0.75, 0.84, 0.86, 1.00, 1.21, 1.26, 1.28, 1.28, 1.35, 1.37,
    /// 1.40, 1.44, 1.50, 1.54, 1.93, 2.16 s (median 1.27, min 552 ms, 16 of 18
    /// inside 0.75-2.16). A re-cue faster than half a Laugh2 template (374 ms)
    /// would be invisible to the correlator, so 500 ms is the floor this can
    /// assert; the module's own floor is one packed run of seq 62 = 600 ms.
    #[test]
    fn father_time_laugh_bursts_keep_the_captured_spacing() {
        if pack_or_skip().is_none() {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        }
        let mut gaps: Vec<u64> = Vec::new();
        let mut any_two: Option<u64> = None;
        for seed in 1..=48u64 {
            let log = ft_cue_log(seed, 120);
            let mut prev_any: Option<u64> = None;
            let mut prev_laugh: Option<u64> = None;
            for (t, s) in log {
                if let Some(p) = prev_any {
                    any_two = Some(any_two.map_or(t - p, |m: u64| m.min(t - p)));
                }
                prev_any = Some(t);
                if s == SND_LAUGH2 {
                    if let Some(p) = prev_laugh {
                        if t - p < 6_000 {
                            gaps.push(t - p);
                        }
                    }
                    prev_laugh = Some(t);
                }
            }
        }
        assert!(gaps.len() > 100, "not enough Laugh2 pairs to judge: {}", gaps.len());
        gaps.sort_unstable();
        let min = gaps[0];
        let median = gaps[gaps.len() / 2];
        // the capture's own floor, 552 ms, rounded down to the 500 ms the
        // correlator can still resolve
        assert!(min >= 500, "Laugh2 re-cued after {min} ms; capture floor is 552 ms");
        assert!(
            (750..=2_160).contains(&median),
            "Laugh2 median self-gap {median} ms outside the capture's 0.75-2.16 s band"
        );
        // and no two voice lines closer than the capture's 545 ms minimum
        let any = any_two.expect("no cues at all");
        assert!(any >= 500, "two voice lines {any} ms apart; capture minimum is 545 ms");
    }

    /// GOLDEN (`shock-monkey-fullscreen.mp4`) — the recover cue @`0x1DA2`.
    /// Leaving 282 re-cues the monkey at delay 60 until the `+0xB0` window
    /// expires (@`0x18C2` puts 120 back). Measured on the capture over the
    /// eight white-LCD windows: seq 162 steps **1.68** frames per 100 ms
    /// inside the window (≈ 60 ms a frame) and **0.85** outside (≈ 118 ms).
    /// The port used to hold 120 ms through the window.
    #[test]
    fn monkey_recovers_at_double_speed_inside_the_white_window() {
        let Some(mut m) = pack_or_skip().and_then(build) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        m.set_control(0, 2);
        let mut ctx = ctx_at((12, 0, 0), 0);
        ctx.rng = RandomLong::new(3);
        let (mut in_ms, mut in_frames, mut out_ms, mut out_frames) = (0u64, 0u64, 0u64, 0u64);
        let mut t = 0u64;
        while t < 600_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 12 * 3600 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            let white = matches!(&m.clock, Some(ActiveClock::Monkey(mk)) if mk.lcd == Lcd::White);
            let before = m.anim_frames;
            m.tick(&mut ctx);
            ctx.sounds.clear();
            let n = m.anim_frames - before;
            if white {
                in_ms += TICK_MS;
                in_frames += n;
            } else {
                out_ms += TICK_MS;
                out_frames += n;
            }
        }
        assert!(in_frames > 200, "only {in_frames} frames inside a white window in 600 s");
        let (inside, outside) = (in_ms / in_frames, out_ms / out_frames);
        assert!(
            (50..=70).contains(&inside),
            "{inside} ms a frame inside the white window; the capture steps 1.68 frames / 100 ms (~60 ms)"
        );
        assert!(
            (110..=130).contains(&outside),
            "{outside} ms a frame outside it; the capture steps 0.85 frames / 100 ms (~118 ms)"
        );
    }

    /// GOLDEN (`shock-monkey-fullscreen.mp4`) — the three spark runs under the
    /// chassis (59/84/108) are LOOPERS: the composer inits them with `+0xA0 =
    /// 1` (@`0x3F34`/`0x3FB4`/`0x4034`), so `fn_6F8A` never rolls for them and
    /// restarts them at every run end. 429 frames sampled across the 300 s
    /// capture hold a lit dot on all three tracks in every one (100 %),
    /// moving frame to frame. The port gated them at an invented 64/2048 and
    /// they were dark ~57 % of the time. Also pins the parts' own 100 ms
    /// cadence (`DRESS_FRAME_MS`, @`0x6E82`): the 24-frame run 59 takes 2.4 s.
    #[test]
    fn monkey_spark_runs_loop_for_the_life_of_the_scene() {
        let Some(mut m) = pack_or_skip().and_then(build) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        m.set_control(0, 2);
        let mut ctx = ctx_at((12, 0, 0), 0);
        let (mut lit, mut total) = (0u32, 0u32);
        let mut restarts_59 = 0u32;
        let mut prev_59 = 0u32;
        let mut t = 0u64;
        while t < 120_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            m.tick(&mut ctx);
            ctx.sounds.clear();
            let Some(ActiveClock::Monkey(mk)) = &m.clock else { continue };
            if t < 1_000 {
                continue;
            }
            for (i, run) in MK_ARCS.iter().enumerate() {
                if matches!(run, 59 | 84 | 108) {
                    total += 1;
                    lit += u32::from(mk.arcs[i].playing);
                }
                if *run == 59 {
                    if mk.arcs[i].frame < prev_59 {
                        restarts_59 += 1;
                    }
                    prev_59 = mk.arcs[i].frame;
                }
            }
        }
        assert_eq!(lit, total, "a spark run went dark ({lit}/{total} lit); the capture has 100 %");
        // 119 s / (24 frames × 100 ms, +1 for the run-end frame) ≈ 47 loops
        assert!(
            (44..=50).contains(&restarts_59),
            "run 59 looped {restarts_59} times in 119 s; 24 frames at 100 ms is ~47"
        );
    }

    /// GOLDEN AUDIO (`shock-monkey-fullscreen.wav`, 297 s, no panel) — the
    /// `fn_59A0` priority gate. A chirp (priority 0) queued while a 0.98 s
    /// zap_spark (priority 1) or a Shock (2) is still sounding is thrown
    /// away on the next start pass. The capture: 0 of 237 chirps start inside
    /// one of the 87 zap clips (68 if independent); chirps 0.80–0.84 / s;
    /// 26 inter-chirp gaps over 3 s; 87–89 zaps as the correlator separates
    /// them (peaks inside half a clip merge). The ungated port: 87 chirps
    /// inside zaps, 1.03 / s, 6 gaps over 3 s.
    #[test]
    fn monkey_chirps_are_discarded_under_a_louder_cue() {
        const WAV_MS: u64 = 297_080;
        let zap_ms = match pack_or_skip() {
            Some(p) => p.sound_ms(SND_ZAP_SPARK),
            None => {
                eprintln!("assets/shock-clocks missing — skipping");
                return;
            }
        };
        assert!((950..=1000).contains(&zap_ms), "zap_spark is {zap_ms} ms; the rip is 976");
        let (mut chirps, mut inside, mut long_gaps, mut zaps) = (0usize, 0usize, 0usize, 0usize);
        let seeds = [1u32, 3, 5, 7, 9, 11];
        for seed in seeds {
            let v = mk_sounds(WAV_MS, 50, seed).expect("pack");
            let zap_t: Vec<u64> = v.iter().filter(|e| e.1 == SND_ZAP_SPARK).map(|e| e.0).collect();
            let ch: Vec<u64> = v.iter().filter(|e| (1000..=1003).contains(&e.1)).map(|e| e.0).collect();
            chirps += ch.len();
            inside += ch.iter().filter(|&&c| zap_t.iter().any(|&z| z < c && c < z + zap_ms)).count();
            long_gaps += ch.windows(2).filter(|w| w[1] - w[0] > 3_000).count();
            let mut kept: Vec<u64> = Vec::new();
            for z in zap_t {
                if kept.last().is_none_or(|&k| z - k >= zap_ms / 2) {
                    kept.push(z);
                }
            }
            zaps += kept.len();
        }
        let n = seeds.len();
        assert_eq!(inside, 0, "{inside} chirps started under a sounding zap_spark; the capture has 0");
        let rate = chirps as f64 / n as f64 / (WAV_MS as f64 / 1000.0);
        assert!((0.65..=0.95).contains(&rate), "{rate:.3} chirps/s; the capture has 0.80-0.84");
        let gaps = long_gaps / n;
        assert!((15..=40).contains(&gaps), "{gaps} chirp gaps over 3 s per take; the capture has 26");
        let zaps = zaps / n;
        assert!((70..=105).contains(&zaps), "{zaps} separable zap_sparks per take; the capture has 87-89");
    }

    /// GOLDEN (`shock-monkey-fullscreen.mp4`, 2026-09-29 re-measure) — the
    /// LCD recolour (`fn_2C3A`/`fn_2C9A`/`fn_2D70`/`fn_2DB6`) is AUTHENTIC.
    /// Median colour of the lit segments per frame, eight gags:
    ///
    /// - white holds **30 / 31 / 30 / 30 / 30 / 30 / 30 / 31** frames at
    ///   10 fps = 3.0–3.1 s (the 2026-09-12 "3.6–3.7 s" was the older Demo
    ///   capture) — `MK_SHOCK_WINDOW_MS`;
    /// - gag 1's ramp runs f589 → f652 (6.4 s = 53 monkey frames) and reads
    ///   red **96** on the last frame before the white, which is exactly
    ///   `0xFFFF − 53 × 0x300` → `0x60`. Mid-ramp the capture shows a
    ///   STAIRCASE (steps every 0.4–0.9 s) because the original only
    ///   re-renders the panel when something dirties it (colon toggle, an arc
    ///   overlapping the panel); the port redraws every frame, so it shows the
    ///   same ramp without the steps — an engine redraw difference, not a
    ///   recolour one.
    #[test]
    fn monkey_lcd_matches_the_fullscreen_golden() {
        let mut l = Lcd::Ramp(LCD_RAMP_ARM);
        for _ in 0..53 {
            l = l.step();
        }
        assert_eq!(l.tint().map(|t| t[0]), Some(96), "53 fn_2C9A steps; the capture reads red 96");
        let Some(trace) = mk_trace(300_000) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        let mut held = Vec::new();
        for (i, e) in trace.iter().enumerate() {
            if e.2 == Lcd::White && (i == 0 || trace[i - 1].2 != Lcd::White) {
                if let Some(end) = trace[i..].iter().find(|x| x.2 != Lcd::White) {
                    held.push(end.0 - e.0);
                }
            }
        }
        assert!(!held.is_empty(), "no white flash in 300 s");
        for h in &held {
            assert!((2_950..=3_150).contains(h), "white held {h} ms; the capture holds 3.0-3.1 s: {held:?}");
        }
    }

    /// `snd 1200 Cuckoo prelude` — trigger CONFIRMED against the C, NO golden
    /// (needs a Rotting Head capture; see the module header's GOLDEN AUDIO
    /// notes). `fn_22E8` (@`0x2352`–`0x23EC`) queues 1200 at counter 0 of eye
    /// state **11** (the minute blink) and of state **20** (one hour round),
    /// and 2000 at counter 4 of state 20, all at priority 2; `fn_240E` loops
    /// 20 `hour % 12` times (12 when the RAW hour is 0 — @`0x2538` tests the
    /// hour, not the remainder, so 12:00 noon is ONE round) and the latch is
    /// an if/else that leaves `+0xAE` stale on the hour frame, so the
    /// 18:00 minute event is DEFERRED, not lost: it rises on the next frame
    /// and waits in state 9 behind the hour rounds. From 17:58:30 over 150 s
    /// that is: one 1200 at 17:59:00, six 1200 → 2000 pairs from 18:00:00
    /// (2000 0.4 s after its prelude), then the deferred blink's 1200.
    #[test]
    fn rotting_head_cuckoo_prelude_follows_fn_22e8() {
        let Some(mut m) = pack_or_skip().and_then(build) else {
            eprintln!("assets/shock-clocks missing — skipping");
            return;
        };
        m.set_control(0, 1); // Rotting Head
        let mut ctx = ctx_at((17, 58, 30), 0);
        let mut cues: Vec<(u64, u32)> = Vec::new();
        let mut t = 0u64;
        while t < 150_000 {
            t += TICK_MS;
            ctx.now_ms = t;
            let s = 17 * 3600 + 58 * 60 + 30 + t / 1000;
            ctx.local_hms = (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8, (s % 60) as u8);
            m.tick(&mut ctx);
            for snd in ctx.sounds.drain(..) {
                if matches!(snd, SND_CUCKOO_PRELUDE | SND_CUCKOO) {
                    cues.push((t, snd));
                }
            }
        }
        let preludes: Vec<u64> = cues.iter().filter(|c| c.1 == SND_CUCKOO_PRELUDE).map(|c| c.0).collect();
        let cuckoos: Vec<u64> = cues.iter().filter(|c| c.1 == SND_CUCKOO).map(|c| c.0).collect();
        assert_eq!(preludes.len(), 8, "blink + six hour rounds + the deferred blink: {cues:?}");
        assert_eq!(cuckoos.len(), 6, "18:00 is six rounds: {cues:?}");
        // the minute blink at 17:59:00 = t 30 s, the first round at 18:00:00 = 90 s
        assert!((30_000..31_000).contains(&preludes[0]), "{cues:?}");
        assert!((90_000..91_000).contains(&preludes[1]), "{cues:?}");
        assert!(preludes[7] > cuckoos[5], "the 18:00 blink comes after the rounds: {cues:?}");
        for (p, c) in preludes[1..7].iter().zip(&cuckoos) {
            assert!((300..=560).contains(&(c - p)), "2000 {} ms after its prelude: {cues:?}", c - p);
        }
    }
}
