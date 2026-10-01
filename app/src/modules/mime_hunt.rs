//! Mime Hunt — ported function by function from `M129_M129.c` + `M130_M130.c`
//! (`docs/decompiled/mime-hunt/`, tt private repo). Two code segments: 146
//! functions in M129 (controller, reticle, weapons, sound bank, decal pool)
//! and 39 in M130 (the mime class and its 100-state machine). Everything in
//! this header is checkable against THIS module's C; where a claim comes off
//! the raw listing instead it says so (`ripped/mime-hunt/Mime Hunt_CODE_129_DynaMime1.txt`
//! / `…_CODE_130_DynaMime2.txt`).
//!
//! ## AMBIENT MODE — observed 2026-09-29
//!
//! *(Corrected.)* This header used to say `emu/captures/qemu/mime-hunt.mp4`
//! was recorded with Caps Lock ON. It was not: nothing touches input during
//! a rig capture, and in that capture the crosshair wanders the whole
//! screen and fires the Bazooka on its own — that is the ambient half.
//! Two panel-free 300 s ambient takes now exist (tt `mime-hunt-ambient`,
//! Manic; `mime-hunt-ambient-benevolent`) and pin the wanderer: it covers
//! the whole field, sits on an edge 16–21 % of the time and in a corner
//! ~1 %, and is never still. Benevolent fires nothing in 300 s (`fn23`).
//! The interactive half (reticle follows the pointer, click fires) is
//! still what `fn131`/`fn132` hand to; neither touches the wanderer, so
//! leaving interactive resumes it where it was, not at the pointer.
//! The rest of the ambient half — the fire schedules, the 10 s grace and
//! the 5-minute recycle — is still only as good as the C reading and
//! keeps its UNOBSERVED flags below.
//!
//! The rig's step rate is NOT the port's: on QEMU the reticle decorrelates
//! within one 100 ms frame (median jump 320–390 px), because After Dark
//! calls `DoDrawFrame` as fast as the host allows and every call steps the
//! wanderer `mimes + 2` times. The port steps once per MacTick, so it
//! wanders at ≤ 54 px per 100 ms. Occupancy (where it spends its time) is
//! rate-free and is what the ratchet pins.
//!
//! ## Objects (factory `fn101` @2A0C → ctor `fn106` @2A40 → `fn108` @2F1E)
//!
//! * **Two `RLESequence` art banks** (`fn137` @4C86): resource 1000 into
//!   `g02A8` and 2000 into `g02AC`. `g02A8` also gets
//!   `(*vtbl+0x9C)(1)` = `fn55` @56F2, which stores 1 in the sequence's
//!   `+0x11E` "skip this channel" slot that `fn56` @558A honours when it
//!   draws a compound. Bank **2000 is the live mime**; bank **1000 is the
//!   reticle (2, 6..12), the impact decals (16..46) and every death
//!   animation (52..260)**.
//! * **A sound bank** `g0524` (`fn135` @4B74, ctor `fn89` @64E8, 256 slots)
//!   preloaded by `fn94` @67E4 with (1000, 2000, 5000, −1) — the arg list
//!   read off `fn92` @674C's caller at listing `00002C8E`. Music is
//!   `cmid` 10 on its own channel.
//! * **A crosshair triple** built by `fn108`:
//!   `+0x22` a 4-byte *mouse* source (`fn31` @2298, vtbl `g00A6`:
//!   `+4` = `fn34` reset, `+8` = `fn35` @22DC GetMouse→global),
//!   `+0x26` a 0x6C-byte *wanderer* (`fn10` @1212, vtbl `g00B2`:
//!   `+4` = `fn13` @124A place, `+8` = `fn17` @1570 step-and-read), and
//!   `+0x1E` the *reticle* itself (`fn36` @2348, vtbl `g00BE`:
//!   `+4` `fn39` @2398 init, `+8` `fn40` @2436 update, `+0xC` `fn41` @25B0
//!   damage, `+0x10` `fn42` @263C draw, `+0x14` `fn43` @2672 blit).
//!   `fn40`/`fn44` pick the wanderer when `g02A6 == 0` (ambient) and the
//!   mouse when it is 1 (interactive) — that flag IS Caps Lock.
//! * **An 8-slot decal pool** `g0512` (`fn61` @57D4 / `fn64` @58EE), each
//!   slot a 0x2E-byte object over its own offscreen. See the GAP list.
//! * **Up to 4 mimes** `g0232[20]`, count `g0230` (`fn115` @378C allocates
//!   `0x1A8` bytes each and binds them with `M130_fn11` @028E).
//!
//! ## Controls (`fn122` @3BF4, `fn124` @3CCC, `fn123` @3C3A)
//!
//! | idx | resource | read in | meaning |
//! |---|---|---|---|
//! | 0 | `sVal 1000` "# of Mimes", 0..90, default 40 | `fn122` | `g0230 = fn110()*raw/100 + 1`, capped at `fn110()` |
//! | 1 | `mVal 1001` → `MENU 1000` "Weapon", default 1 | `fn124` | raw, plus one when above 1 |
//! | 2 | `mVal 1002` → `MENU 1003` "CyberMood", default 1 | `fn124` | `g00D6 = raw` |
//! | 3 | `sVal 1003` "Music", 0..95, default 30 | `fn123` | play-count band |
//!
//! `MENU 1000` is Shotgun / Bazooka / MimeSlayer / `-` / Random, and the
//! bump-by-one-above-1 at listing `00003D46` is what turns the menu
//! item into the weapon id: **1 → 1 Shotgun, 2 → 3 Bazooka, 3 → 4
//! MimeSlayer, 4 → 5 (the separator, unreachable from the menu), 5 → the
//! Random branch** which rolls `RandomBelow(3)+1` and applies the same
//! skip, i.e. {1, 3, 4}, re-armed `now + 5000 + RandomBelow(20000)`.
//! `g029A == 2` is therefore DEAD as shipped (no menu item reaches it) and
//! so is the keyboard shortcut for it — `fn125`'s keyDown arm handles key
//! codes 0x12/0x14/0x15 ('1'/'3'/'4') and deliberately skips 0x13 ('2').
//! Ported anyway, exactly as written.
//!
//! `fn124` runs **once per frame in ambient mode only** (`fn130` @45C4
//! calls it first thing, only while the Caps Lock flag `g02A6` is clear), so a weapon or mood change made
//! while Caps Lock is down does not take until you let go. `fn123` runs
//! once, from the constructor via `fn135` — the Music band is latched at
//! build time and never re-read.
//!
//! Music bands (`fn123`, listing `00003C3A`): `< 5 → 0`, `5..24 → 1`,
//! `25..49 → 2`, `50..74 → 3`, `75..94 → 4`, `>= 95 → 99` (Forever) —
//! exactly the `sUnt 1003` tick labels Never/Once/Twice/Three/Four/Forever.
//!
//! ## Library semantics used (all `L132_Resource`, vtbl `M130_g00AA`
//! decoded from `emu/ghidra/mime-hunt/blocks/A5_globals.bin`)
//!
//! The mime binds the same slots frankenscreen does — **not** boris's:
//!
//! * `+0x7C` = `fn0204` @0204 → `+0x108` = `fn028A` @028A = **SetRun(id)**,
//!   then clear the queue and `+0x46`. `fn028A` is **two links through a
//!   marker frame**, and getting that wrong is the difference between a
//!   mime that walks and one that snaps back — see "The run hand-off"
//!   below.
//! * `+0x80` = `fn0240` = SetRun(id₀) then queue id₁… . The list is
//!   **longs**, −1 terminated (the long −1 pushed at listing `0000432E`).
//! * `+0x84` = `fn0316` = the per-frame advance; raises `+0x46`
//!   ("my run finished") when the run runs out with an empty queue.
//! * `+0x88` = `fn04C8` = SetPos. `fn04C8` stores its **second** short at
//!   `+0x40` and its first at `+0x42`, and `M130_fn32`'s call pushes
//!   (y, x) — so `+0x40 = x`, `+0x42 = y`, and `fn30` @41A0 reads that
//!   pair back as the point it offsets a rect by. **pos is the frame
//!   CENTRE** (port-plan §2).
//! * `+0xFC` = `fn12DE` = "net displacement of run N", called as
//!   `(this, 1, runId)` — `fn30` uses it to ask *would run N still fit in
//!   my cell*, and state 0xE stashes the answer in `+0x178`.
//! * `+0xE8` = `M130_fn34` @47C8 → `fn10FE` = the rect of one compound
//!   CHANNEL on the drawn frame; a negative channel yields an empty rect.
//! * `L132 fn4640(rect, point)` → `fn410C` **centres** the rect on the
//!   point (it is not OffsetRect). Every blast box and the reticle box go
//!   through it.
//! * Run ids are OFst `frameNum`s as written — no ±1 (library40-api
//!   §10.1; mime-hunt's OFst 1000 and 2000 have no lead-in records). A run
//!   resolves to the block CONTAINING the id and is entered AT the id:
//!   e.g. state 0's 0x14C = 332 enters block 331..338 at 332.
//!
//! ## The run hand-off — `fn028A` @028A, the thing that makes mimes walk
//!
//! A hand-off is NOT `pos += centre(newRun) − centre(currentFrame)`. The C
//! routes it through the new run's **lead-in record**:
//!
//! 1. `+0xF0` = `L132 fn1186` @1186 maps the new run id to the frame to
//!    pass through. `+0x82` is hard-set to 1 by the sprite reset
//!    (`L132` @0070), so whenever `fn441A` says the record `id − 1` exists
//!    it returns **`id − 1`** — the OFst lead-in, i.e. the LINK MARKER.
//!    **97 of the 98 run ids this module names are `blockStart + 1`**, so
//!    the marker is there for every one of them but `0x6F4` (1780, which
//!    is a one-frame run at the end of block 1778).
//! 2. `+0xCC` = `L132 fn0D3C` @0D3C links the CURRENT frame to that marker
//!    through the sequence's `+0x80` = **L135 `fn3F2E` @3F2E** — the
//!    *shared-part* registration, not a centre difference. It finds the
//!    first art id both frames' part tables carry and moves `pos` so that
//!    part keeps its screen position:
//!    `delta = (partA − centreA) + (centreB − partB)`. No shared part → no
//!    delta. Then it seats `+0x3A` on the marker.
//! 3. `+0xF4` = `fn1210` @1210 / `+0xD0` = `fn0DF4` @0DF4 link the marker
//!    to the run's first frame through `+0x7C` → `+0x78` = **L135 `fn3DDC`
//!    @3DDC**, the plain `centre(to) − centre(from)` (verified: `fn3DDC`
//!    takes `(l+r)>>1` of two `+0x18` frame-bounds fetches and negates x
//!    under the flip).
//!
//! Because the marker is authored *at the run's own origin*, step 3 is
//! exactly 0 for 64 of the 98 ids and ≤ 8 px for the rest — all of the work
//! is step 2, which registers the outgoing pose against the incoming one.
//! **A run's travel is therefore KEPT.** Bank 2000's walk block 706..727
//! slides the box from x = 324 to x = 215 (the mime walks 107 px left) and
//! the block the machine goes to next carries its marker 729 with an OFst
//! `dx` of +106 and *the identical part table as frame 727* — so the
//! shared-part link is 0, the centre link 729 → 730 is 0, and the mime
//! simply keeps walking. Skipping the marker and linking 727 → 730 by
//! centres gives +106 px instead: a snap back that the 60 s golden capture
//! does not contain (largest frame-to-frame centroid step over four mimes:
//! 16 px, zero over 25 px, while mimes cover 137–197 px spans). Ratchet:
//! `run_hand_offs_are_continuous`, which measures 20 px worst — the
//! *largest authored within-block centre step in the whole of bank 2000*
//! (frames 805 → 806). The hand-offs add nothing on top of the art.
//!
//! `fn30` @41A0's cell check is consistent with this: it asks whether a
//! run's displacement leaves the mime inside its 210 px cell precisely
//! because the displacement is permanent.
//!
//! **Step 2 also FLIPS the mime** (2026-09-29 audit). `fn3F2E` XORs the
//! sprite's `+0x3C` flip with the two shared parts' own flip bits (part
//! record `+0x4`) and lays each part out the `fn3BD6` way under its own
//! flip before differencing. State 6's run 342 ends on frame **343**, the
//! standing pose authored MIRRORED (art 27 flag 1; the marker's art 27 is
//! native), so every hand-off out of 343 mirrors the sprite — that is the
//! whole of the turn: state 6 toggles only the module's `+0x146`, the
//! library turns the art. The port used to leave the flip alone: the mime
//! snapped back to its native facing one frame after turning and slid
//! 19 px (the mirrored part registered as if native). In the 60 s capture
//! 89 of 213 standing-pose sightings are mirrored (template match of
//! c_346 both ways), some for 2.5 s unbroken; the old port drew 1 of 901.
//! 52 of 1729 hand-offs per 60 000 ticks flip (all out of 343); none has a
//! shared part whose vertical bit differs. The in-run link is `fn3DDC`
//! with its flip rounding, and part / channel rects (the hit resolver's
//! input, via `fn4488` → `fn3BD6`) are mirrored with the sprite.
//! Ratchet: `the_turn_hands_off_mirrored`.
//!
//! ## State-object override — the model differs from every other module
//!
//! The mime's state sub-object lives at **`+0x92`** (vtbl `M130_g01FA`),
//! and slot `+0x1C` (SetState) is **`M129_fn60` @5706**, a module override
//! of the stock `L135 fn55B6`. The stock one sets the "pending" flag and
//! lets the next `Run` fire the ENTER; `fn60` instead calls `+0x20`
//! (fire ENTER) itself, **synchronously**, at the end. So in this module
//! `SetState(s)` = exit(old), store, **enter(new) right now**, and
//! `L135 fn5340` (Run) only ever delivers the UPDATE. That is why a mime
//! can walk three states deep inside one tick.
//!
//! *Quirk kept*: `fn5340` clears the pending flag **after** calling the
//! enter hook, so a `SetState` made from inside an ENTER loses the new
//! state's own ENTER. Only enter 0x801B and 0x8031 can do it.
//!
//! Message encoding (`M130_fn29` @0A2A): the handler takes one 16-bit
//! word — `0x8000|s` enter, `0x4000|s` exit, bare `s` update, `0xFFF`
//! dispose. **Every exit returns immediately** (the chain falls through to
//! a return for any word with either top bit set), and so does every *unlisted*
//! enter, which is why ~40 of the 100 states keep whatever run they
//! inherited. `fn29` burns one `RandomBelow(100)` at the top of EVERY
//! message, used or not.
//!
//! ## The mime machine (`M130_fn29` @0A2A, 100 states)
//!
//! Compact form. `r` = the `RandomBelow(100)` drawn at entry; `fin` =
//! `+0x46`; `SetRun` ids are series **2000** decimal unless the state is in
//! the death chain. Enter rows show the run the state arms; update rows
//! show where it goes.
//!
//! | s | enter → run | update |
//! |---|---|---|
//! | 0 | 332 | fin → 5 |
//! | 2 | — | r<50 → 6, else 3 |
//! | 3 | — | neighbour busy && r<15 → 0x88; reticle on me: r<25 stay, r≥50 stay, else 0x75; else r<25→4, <50→7, <75→8, else 9 |
//! | 4 | wait 2..1002 ms; 340 | wait, fin → 10 |
//! | 5 | wait 2..1002 ms; 340 | wait, fin → 2 |
//! | 6 | 343 (+ face toggle) | fin → 3 |
//! | 7/8/9 | 346 / 367 / 371 | fin → 10 |
//! | 10 | — | r<75 → 3, else 0xB |
//! | 0xB | — | reticle on me && 25≤r<75 → 0x75; r<33 → 0xC; <66 → 0xD; else 0x3B |
//! | 0xC | — | r<25 → 0x40, <50 → 0x4D, <75 → 0x62, else 0x55 |
//! | 0xD | — | r<33 → 0x2D, <66 → 0x21, else 0xE |
//! | 0xE | — | `+0x178 = netMove(0x1B)`; → 0xF |
//! | 0xF..0x11 | 556 / 571 / 612·615·619·632 by r | chain 0xF→0x10→0x11→0x12 |
//! | 0x12 | — | been-here && r<50 && fits(10,10,707) → 0x13; else mark, → 0x14 |
//! | 0x13 | 707 | fin → 0x1D |
//! | 0x14 | — | r<50 → 0x11, else 0x15 |
//! | 0x15 | 651 | fin → reps = r/10+2, → 0x16 |
//! | 0x16 | 765 (r<50) / 676 | fin → 0x17 |
//! | 0x17 | — | r<33 → 0x1B; 33..65 && fits(10,10,784) → 0x18; else 0x19 |
//! | 0x18 | 784 | fin → 0x1D |
//! | 0x19 | — | ++rep < target → 0x17, else 0x1A |
//! | 0x1A | 695 | fin → 0x11 |
//! | 0x1B | fits(10,10,803) ? 803 : 828 | fin → r<50 ? 0x5D : 0 |
//! | 0x1D/0x1E | 730 / 742 | chain → 0x1E → 0x1F |
//! | 0x1F | — | r<25 && fits(10,10,784) → 0x18; <50 → 0x1B; else 0x20 |
//! | 0x20 | 761 | fin → 0x16 |
//! | 0x21 | — | fits(10,30,1300) && fits(10,30,44) → 0x22, else 0 |
//! | 0x22..0x25 | 1231 / — / 1262·1287 / — | 0x22 fin→0x23; 0x23 reps=r/10+1 →0x24; 0x24 fin→0x25; 0x25 ++rep<target→0x24 else fits(2,2,1300)?0x26:0x2C |
//! | 0x26..0x29 | 1300 / 1376·1388·1419·1440 / 1396 | 0x26 fin→0x27; 0x27 fin→0x28; 0x28 r<50 → (fits(10,10,1396)?0x29:0x2C) else 0x27; 0x29 fin, r<50→0x27 else 0x2B |
//! | 0x2B | fits(10,10,1461) ? 1461 : →0x2C | fin: r<25→0, <50→0x5E, <75→0x5D, else 0x5C |
//! | 0x2C | 1542 | same four-way as 0x2B |
//! | 0x2D..0x32 | — /556/872/—/891·894/— | walk-and-turn cycle, reps = r/10+1 |
//! | 0x33..0x3A | 922/—/951/981·1042/1009/981·1042/1072/1126 | the long "business" loop |
//! | 0x3B..0x3F | 411/416·429/—/—/454 | reps = r/10+2 loop, ends → 0xD |
//! | 0x40..0x4C | 1568 / … / 2011 | the two big idle suites (see the code) |
//! | 0x55..0x5A | 5 / 190 / 224 / 170·136 / 190·224 / 237 | |
//! | 0x5B | — | r<25 → 0x5C, <50 → 0x5D, <75 → 0x5E, else 0 |
//! | 0x5C/0x5D | 850 / 3174 | fin → 0 |
//! | 0x5E | 1948 | fin → r<80 ? 0 : 0x60 |
//! | 0x5F/0x60/0x61 | 1948 / 340 / 340 | 0x60 and 0x61 FLIP the mime (`+0x14C`) |
//! | 0x62..0x70 | 2259 / 2518·2552·2587 / 2887·2906 / 2720 / 2816 / 2926 / 2663·2601 | |
//! | 0x72 | delay by weapon, `+0x154 = now + d` | fin && dying → dead, → 0x73 |
//! | 0x73 | — | if weapon == 5 restore `g0296` — **terminal** |
//! | 0x74 | see below | the extra-hit / bazooka hold |
//! | 0x75..0x87 | the "shot at, flinch" suite: 3046/3066/3071/3091/3121/3206/3232/3248/3140 | |
//! | 0x88/0x89 | 2223 / kill-run dependent | |
//! | 0x8A | — | **terminal**: the MimeSlayer blank |
//!
//! ## Death (`M130_fn33` @46B0 → 0x72 → `fn26` @0926 → `fn31` @4272)
//!
//! A hit routes through `fn33`: if the mime is already dead and not in
//! 0x74/0x89 it goes to **0x74**; otherwise, if it is not already dying,
//! not in 0x72, not hidden and not dead, it records the hit part, the side
//! (blast centre left or right of the mime), the weapon, and goes to
//! **0x72**. Entering 0x72 stamps `+0x154 = now + d`, d by weapon:
//! Bazooka 950, Shotgun 100, weapon 2 → 0, weapon 5 → 250, MimeSlayer →
//! a DebugStr and nothing. `fn26` (every tick from `fn130`) sees that
//! deadline expire, **re-binds the sprite to bank 1000** and calls `fn31`,
//! which picks the death run from (weapon, hit part):
//!
//! | weapon | hit part | runs (bank 1000) |
//! |---|---|---|
//! | 1 / 2 | 1 head | 50 %: 99, else 73 |
//! | 1 / 2 | 2 body | 52 |
//! | 1 / 2 | 3 arm | 155 |
//! | 1 / 2 | 4 leg | 130 |
//! | 3 Bazooka | any | 50 %: [224, 254], else [184, 205] |
//! | 4 MimeSlayer | any | nothing — `fn128`/`fn39` handle it |
//!
//! and coin-flips the facing first. Entering **0x74** then queues the
//! corpse triple for that run — 73 → [88, 96], 99 → [116, 127], 52 → [61],
//! 155 → [167, 177], 130 → [141, 150] — which is exactly the
//! `[fall][corpse-idle][corpse-still]` layout series 1000 is built in. For
//! the bazooka (or `+0x134` ∈ {184, 224}) 0x74 instead holds `now + 950`
//! and then goes to **0x89**, which queues [210, 205] or [260, 254] (and
//! for a gun death, restores the position stashed at `fn31` time before
//! queueing [184, 205]).
//!
//! `fn35` @4826 decides the hit part by **compound channel**, using the
//! pack's own `parts` table: art ids 110..136 = the head (code 1), 1..12 =
//! the body (code 2), 13..29 = the two legs, 30..46 and 47..109 = the two
//! arm segments (code 3); code 4 is the leg-span path through `fn36`
//! @4DB0, which walks a quarter-step line from the body centre toward the
//! leg box four times looking for the blast. A Bazooka skips all of it and
//! returns 1 for any overlap; a mime that is already dying or dead tests
//! its whole frame rect instead.
//!
//! ## Weapons and firing (`fn125` @3D68, `fn127` @415E, `fn128` @43CA)
//!
//! | `g029A` | menu | snd | refire | blast box | reticle box |
//! |---|---|---|---|---|---|
//! | 1 | Shotgun | 1000 | 1000 ms | 32×32 | 32×32 |
//! | 2 | (dead) | — | 450 ms | 24×24 | 24×24 |
//! | 3 | Bazooka | 2000 | 2500 ms | 64×64 | 64×64 |
//! | 4 | MimeSlayer | 5000 | 1000 ms | 48×48 | 48×48 |
//! | 5 | (separator / secret) | — | — | uninitialised (see quirks) | 24×24 |
//!
//! The boxes are the four A5 rects at `g0110`/`g0118`/`g0120`/`g0128`
//! (M129) and `g0008`/`g0010`/`g0018`/`g0020` (M130), dumped from
//! `A5_globals.bin`. Weapon 4 plays its sound BEFORE the shot resolves and
//! every other weapon after it (`fn125` @3F84 vs @3FC4).
//!
//! `fn127` centres the blast box on the shot point, walks the mimes, and
//! for weapon 4 defers to **`fn128`**: play 5000, run a full
//! damage/draw/blit pass over every mime and the reticle, then
//! `M130_fn39` @53DA on the one that was hit — union the rect, redraw,
//! call `fn146` @0388 (the dither dissolve, a failed decompile) and hide
//! the mime, mark it dead and `SetState(0x8A)`, which is a terminal state
//! with no run. That is the "dither dissolve + blank" the port-plan notes.
//!
//! ## CyberMood — the ambient gun (`fn23` @1E62 … `fn30` @2220) UNOBSERVED
//!
//! | `g00D6` | `MENU 1003` | wander speed / re-roll | fires? |
//! |---|---|---|---|
//! | 1 | Benevolent | 0.05 px, every 8 frames | **never** (`fn23` returns 0) |
//! | 2 | Bored | 0.20 px, every 4 | `now + 15000 + rnd(15000)` |
//! | 3 | Psycho | 0.65 px, every 4 | `now + 100 + rnd(750)` |
//! | 4 | Sadistic | 0.65 px, every 4 | `now + 3000 + rnd(20000)` |
//! | 5 | Manic | alternates 1 and 3 | alternates with the phase |
//!
//! Mood 4 also runs a stalk: the first frame the reticle sits on a live
//! mime (`fn30`) arms `+0x62 = now + 10000 + rnd(15000)`; while that window
//! is open `fn15` drops the speed to 0.05 and `fn22` clamps the velocity
//! to ±0.4. Mood 5 flips `+0x6A` on a `fn16` timer (`now + 4000 + rnd(20000)`
//! calm, `now + 20000 + rnd(40000)` manic) and only fires in the manic half.
//!
//! The wanderer itself (`fn21` @1B36) is a two-axis integrator in SANE
//! 80-bit extended: `v += a * 0.3` clamped to ±5.0, `pos += v * 0.3`,
//! `a` re-rolled every `+0x54` frames by `fn19` @17AC — for x,
//! `r < 33 → −speed`, `< 66 → +speed`, `80..89 → −v/2`, `90..93 → −v`,
//! else 0; for y, `r < 33 → −speed`, `< 66 → 0`, else `+speed`. The three
//! constants 0.3 / 5.0 / −5.0 were read out of the code segment's literal
//! pool at `00211D24` / `00211D2E` / `00211D38`. Before each step `fn20`
//! @1964 steers off the walls: within 64 px of an edge (84 px while the
//! re-roll period is 7) the acceleration on that axis is forced to
//! `±speed` pointing inward — every step, overriding the roll. Position is
//! then clamped to the play field by `fn18` @15E8 with the velocity left
//! alone. The decompile dropped every compare in `fn20` and it had been
//! ported as "clamp the acceleration to ±speed"; with no inward push the
//! velocity random-walks with nothing to turn it, so the reticle parked
//! in a corner for minutes (pinned by
//! `ambient_crosshair_does_not_park_on_the_walls`).
//!
//! ## Layout (`fn109` @30EA, `fn110`..`fn114`)
//!
//! `rows = min(H / (mimeH + 32), 3)`, `cols = W / (mimeW + 32)`, and the
//! population ceiling is `min(rows*cols, 4)` — **four mimes, ever**. The
//! grid picks a row count (1, 2 or 3) and then splits the population
//! across those rows with one of two A5 tables:
//!
//! * `g0164` (3 rows × 10 splits): 9→[3,3,3] 8→[3,3,2] 7→[3,2,2] or
//!   [3,3,1] 6→[3,2,1] or [2,2,2] 5→[2,2,1] 4→[2,1,1] 3→[1,1,1] 2→[1,1,0]
//! * `g01A0` (2 rows × 6): 6→[3,3] 5→[3,2] 4→[3,1] or [2,2] 3→[2,1] 2→[1,1]
//!
//! Each mime then gets a cell **210 px wide** plus a share of the row's
//! slack (`RandomBelow(slack)` per mime, the remainder to the last), and
//! `M130_fn32` @4564 drops it at the cell's horizontal centre with a
//! vertical jitter of `±RandomBelow(cellH − mimeH)/2`. A horizontal jitter
//! is rolled and thrown away — an RNG consumer, kept. Mimes are chained
//! `+0x142`/`+0x144` into a left-right neighbour list that state 3 probes.
//!
//! ## Frame driver (`fn130` @45C4, DoDrawFrame, every Mac tick)
//!
//! 1. ambient only: `fn124` re-reads Weapon + CyberMood.
//! 2. music: while the channel is idle and the play count is under the
//!    band (or the band is 99) start `cmid` 10 and count it.
//! 3. `fn129` @44D0 decides a rebuild: the Mimes control changed, or all
//!    mimes are dead and the 60 s `+0x36` timer since has expired, or
//!    ambient and `now − +0x14 > g029E` (300 000 ms), or interactive and
//!    `+0x3A + 300000 < now`. A rebuild tears down and re-lays the grid.
//! 4. the Caps Lock edge enters/leaves interactive (`fn131`/`fn132`), and
//!    interactive also drops out after 120 s with no event.
//! 5. every mime runs `M130_fn38` @5162 — "is the reticle box over me" —
//!    and the OR of those is what the reticle animation reads.
//! 6. gated on `g028A < now` (ambient: `now + 10000` at build) **or**
//!    interactive: `fn125` fires, then `fn45` @28BA steps the reticle art.
//! 7. every mime: `fn26` (death-art swap) then its state UPDATE.
//! 8. gated on `g0286 + 90 < now` — a STRICT compare, re-armed to `now` —
//!    every mime advances one run frame and the decal pool ticks. On the
//!    truncated `TickCount()*16.625` clock that is **6 ticks = 99.75 ms**
//!    flat, the ~100 ms family.
//!
//! `fn45`: the reticle frame is `2` with no live mime under it, and
//! `6, 7, … 12` cycling while there is, re-armed `now + 0x21` = 33 ms on a
//! strict compare, which the truncated clock turns into 2 or 3 Mac ticks,
//! **44.9 ms mean** — its own gate, ahead of the 99.75 ms frame gate.
//!
//! ## Original quirks kept
//!
//! * `g029A == 5` has no blast-box arm in `fn127`, so the secret
//!   MimeSlayer (below) shoots with whatever rect the stack held. Ported
//!   as an empty box: it hits nothing. `fn125` answers weapon 5 with a
//!   DebugStr and no sound.
//! * The secret: `fn126` @3FCA wants the pointer inside the top-left 64×64
//!   corner and then the bottom-right 64×64 corner, each within 30 s; with
//!   that latch set, the next head shot swaps `g029A` to 5 and stashes the
//!   old weapon in `g0296`, which state 0x73 restores on the next death.
//! * `M130_fn32` rolls and discards a horizontal placement jitter.
//! * `fn23`'s CyberMood 1 arm returns 0 unconditionally — Benevolent never
//!   shoots, so an ambient Benevolent run is four mimes miming forever.
//! * `M130_fn23` computes its delay as `RandomBelow(hi−lo)*1000 + lo`
//!   (listing `00000890`), so the (4, 2) call sites give 2 ms or 1002 ms,
//!   not the 2–4 s the shape suggests.
//! * `L135 fn5340` clears the pending flag after the enter hook, losing
//!   the ENTER of a state set from inside another ENTER.
//! * `fn109`'s row slack can go negative when a row of 3 lands on a field
//!   narrower than 630 px; `RandomBelow` of a negative is whatever the
//!   library does. Clamped at 0 here.
//! * *(superseded 2026-09-29 — the drift was the missing `fn3F2E` flip and
//!   a flip-blind `fn12DE`; see "Step 2 also FLIPS the mime". With both,
//!   60 000 ticks give worst cell overrun 6 px, worst screen overrun 6 px,
//!   12 clipped samples of ~240 000.)* **Mimes drift.** The only restoring force on a walk is the 0x60/0x61
//!   facing flip (which negates the link dx); `fn30` gates every run with
//!   more than 40 px of net travel, but the small ungated ones (556, 571,
//!   872, 1072, 1126, 2011, 3140 — all ≤ 25 px) accumulate and nothing
//!   recentres. Measured over 60 000 ticks with four mimes: worst cell
//!   overrun 136 px, worst screen overrun 75 px (box edge; centre −46),
//!   9 % of samples clipped by an edge, every mime returning inside its
//!   cell. Pinned by `the_grid_caps_at_four_and_the_cell_check_contains_the_walk`.
//!   Worth an eye — but there is no containment in the C to port, and
//!   per-minute the port travels LESS than the capture (≈ 85 px/60 s vs the
//!   capture's 137–197 px).
//! * The ambient crosshair has **no wall bounce**: `fn18` @15E8 clamps the
//!   position and leaves the velocity, so a reticle that overshoots the
//!   `fn20` margin leans on the edge until the inward push has turned its
//!   velocity — 16–44 ticks at most in long runs. The rig shows the same
//!   short leans (longest edge run 0.8 s, never still > 0.2 s).
//! * *(worth an eye)* On the rig the drawn Bazooka reticle reaches the
//!   screen edge (art centre 22 px in, art 44 px). The port clamps the
//!   64 px `RETICLE_BOX` into the screen, so its art stops 32 px in.
//!   `fn40` clamps the `+0x16` rect `fn39` got from its caller, which is
//!   not obviously the weapon box — not chased in this pass.
//!
//! ## GAPs — found in the C, not ported, not invented
//!
//! * `GAP(fn146 @0388)` — the MimeSlayer **dither dissolve** itself:
//!   "Cannot properly adjust input varnodes". `fn128`/`M130_fn39` around
//!   it are ported, so the mime is redrawn, hidden, marked dead and parked
//!   in the terminal 0x8A. The dissolve pattern and the 700 ms blank are
//!   not drawn — the mime simply disappears.
//! * `GAP(fn139 @4E50 → fn66 @5AEC → fn77 @5EFE)` — the pellet spray.
//!   Read off the listing (8 marks for Shotgun, 4 for weapon 2, at
//!   `RandomBelow(w/h)` inside the blast box, kind 1 on a miss and
//!   `RandomBelow(20)/10 + 2` on a hit) and ported, but the art the pool
//!   draws depends on `p_RLE_Helper`'s index space, which is library code
//!   we do not have: `fn64` caches 8 frames by index 0..7 and `fn82`'s
//!   ladder shows pairs (1,2) (3,4) (5,6). Read as OFst *ordinals* those
//!   are the eight small impact compounds 16/20/24/28/36/39/42/45, which
//!   is what this port draws; read as raw frameNums they would land on the
//!   reticle art, which cannot be right.
//! * `GAP(fn06 @0F22)` — the interactive crosshair's per-weapon spring.
//!   Ghidra lost every `_Pack4_FP68K` argument; only the three constants
//!   per weapon survive (Shotgun/5 0.02/0.85/1.30, weapon 2 0.02/0.93/0.60,
//!   Bazooka 0.07/0.30/2.50, MimeSlayer 0.02/0.60/1.50). The reticle
//!   follows the pointer exactly here.
//! * `GAP(fn41DC / fn4210 / fn4234)` — After Dark's "did the user do
//!   something" probes. Modelled as the Caps Lock edge, which is the
//!   documented control (`TEXT 1000`, `STR# 128` items 17..20).
//! * `GAP(fn126 @3FCA)` — the corner code. Nothing in either decompiled
//!   segment calls it, so `+0x4A` never leaves 0 and the secret MimeSlayer
//!   swap in `fn127` is unreachable as shipped. Both halves are ported; the
//!   detector is simply never driven. Its only possible caller is one of
//!   the failed decompiles (`ModuleMain`, `fn143` @0188, `fn146` @0388).
//! * `GAP(fn125 keyDown / Tab)` — key codes 0x12/0x14/0x15 re-arm the gun
//!   and 0x30 (Tab) forces a rebuild. `Ctx` surfaces no key events.
//! * `GAP(fn85 @62C2 / fn86 @6362)` — the `STR# 128` quip drawn into the
//!   canvas at build. `SHOW_QUIP_CAPTIONS` is off engine-wide.
//! * `GAP(fn134 @4A76)` — `LoadCLUT(1000)` at 8 bpp / `LoadCLUT(2000)` at
//!   4 bpp. The shell composes at 32 bpp, so neither fires.
//! * `GAP(fn113/fn114 bounds)` — the two sequences' `+0x1C` GetBounds.
//!   Substituted with the pack's largest frame per bank, which is what
//!   sets `rows`/`cols`.
//! * `+0xFC = fn12DE @12DE` — "how far does run N move me". *(GAP
//!   superseded 2026-09-29.)* The C zeroes `+0x40/+0x42`, seats `+0x3A` on
//!   `fn1210(run)` (the id), and links it to `fn1260(run)` (the block's
//!   last record) in ONE `fn0DF4` step — `fn3DDC` under the sprite's
//!   current flip. The port does exactly that now; it used to ignore the
//!   flip, so a mirrored mime's cell check asked about the wrong direction.
//! * *(superseded 2026-09-29)* ~~`GAP(fn3F2E flip handling)` — "both
//!   frames at a hand-off carry the sprite's single flip state"~~: wrong.
//!   The part flip XOR changes the SPRITE's flip; now ported (see "Step 2
//!   also FLIPS the mime").
//! * `GAP(vertical flip)` — `fn3F2E` toggles `+0x3C` bit 1 on a vertical
//!   part-flag difference too. No hand-off this module makes has one, and
//!   `SpriteDraw` has no vertical flip, so bit 1 is not carried.
//! * `GAP(fn35 leg-span rect)` — the composite box `fn35` builds out of
//!   the two leg rects and the body centre has its vertical extent elided
//!   by the decompiler; the union of the two leg rects is used.
//!
//! ## Sounds
//!
//! `docs/emulator/audio-captures.md`'s 60 s timeline finds snd **2000
//! Bazooka twice** (t = 25.37, 28.28 — 2.91 s apart against the 2500 ms
//! refire) and explicitly **never** 1000 or 5000, which is correct for a
//! Bazooka run. 80 % of the loud time is `cmid` 10, which now rides
//! `Module::music`.

use engine::l135::{self, FrameBox, LinkModel};
use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TickClock, SCREEN_H, SCREEN_W,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants

/// `fn137` @4C86: `g02AC` — the live mime.
const BANK_MIME: u32 = 2000;
/// `fn137` @4C86: `g02A8` — reticle, decals and every death animation.
const BANK_KILL: u32 = 1000;

/// `fn109` @30EA: the authored cell pitch.
const CELL_W: i32 = 210;
/// `fn118` @39BE: the play field is at least this big.
const MIN_FIELD_W: i32 = 512;
const MIN_FIELD_H: i32 = 384;
/// `g0108` = (0, 0, 32, 32): the margin `fn113`/`fn114` add to the mime box.
const BOX_PAD: i32 = 32;

/// `g0110` / `g0118` / `g0120` / `g0128` (M129 A5 data) — the blast boxes
/// for weapons 1..4. Weapon 5 has none (see quirks).
const BLAST: [i32; 5] = [32, 24, 64, 48, 0];
/// `M130_g0008` / `g0010` / `g0018` / `g0020` — the reticle boxes. Weapons
/// 2 and 5 share the 24×24 one (`fn38` @5162).
const RETICLE_BOX: [i32; 5] = [32, 24, 64, 48, 24];

/// `fn125` @3D68: refire interval in ms by weapon.
const REFIRE_MS: [u64; 5] = [1000, 450, 2500, 1000, 0];
/// `fn125` @3F2A: the cue each weapon plays; 0 = none.
const WEAPON_SND: [u32; 5] = [1000, 0, 2000, 5000, 0];

const SONG_MIME_HUNT: u32 = 10;

/// `fn106` @2A40: `g029E`, the ambient life of one grid.
const AMBIENT_LIFE_MS: u64 = 300_000;
/// `fn129` @44D0: the quiet beat after the last mime dies.
const ALL_DEAD_MS: u64 = 60_000;
/// `fn108` @2F1E: `g028A = now + 10000` — the ambient startup grace.
const AMBIENT_GRACE_MS: u64 = 10_000;
/// `fn130` @46BA: interactive drops out after this long with no event.
const INTERACTIVE_IDLE_MS: u64 = 120_000;
/// `fn129` @4520: interactive rebuild after this long.
const INTERACTIVE_LIFE_MS: u64 = 300_000;
/// `fn130` @47C4: the mime/decal advance gate, STRICT.
const ADVANCE_GATE_MS: u64 = 90;
/// `fn45` @2920: the reticle art gate.
const RETICLE_GATE_MS: u64 = 0x21;
/// `fn126` @3FCA: the corner-code box and its window.
const CORNER: i32 = 0x40;
const CORNER_WINDOW_MS: u64 = 30_000;

/// `g0164` — three-row population splits, indexed by `fn109`'s `f8`.
const ROWS3: [[i32; 3]; 10] = [
    [3, 3, 3],
    [3, 3, 2],
    [3, 2, 2],
    [3, 3, 1],
    [3, 2, 1],
    [2, 2, 2],
    [2, 2, 1],
    [2, 1, 1],
    [1, 1, 1],
    [1, 1, 0],
];
/// `g01A0` — two-row population splits.
const ROWS2: [[i32; 2]; 6] = [[3, 3], [3, 2], [3, 1], [2, 2], [2, 1], [1, 1]];

/// `fn35` @4826 part groups: (lo art, hi art). Head, body, legs, arm A,
/// arm B — the ranges the four `fn37` @507E calls scan.
const PART_HEAD: (i32, i32) = (0x6E, 0x88);
const PART_BODY: (i32, i32) = (0x01, 0x0C);
const PART_LEG: (i32, i32) = (0x0D, 0x1D);
const PART_ARM_A: (i32, i32) = (0x1E, 0x2E);
const PART_ARM_B: (i32, i32) = (0x2F, 0x6D);

/// The eight small impact compounds of bank 1000 that `fn64` @58EE caches
/// by index — see GAP(fn139).
const DECAL_IDS: [i32; 8] = [16, 20, 24, 28, 36, 39, 42, 45];

/// `fn21` @1B36's SANE literals, read out of the code segment's pool at
/// `00211D24` / `00211D2E` / `00211D38`.
const WANDER_STEP: f64 = 0.3;
const WANDER_VMAX: f64 = 5.0;
/// `fn22` @1D3E: the Sadistic stalk clamp.
const STALK_VMAX: f64 = 0.4;

// ---------------------------------------------------------------------------
// Geometry

/// One packed frame's bank-space box: the compound rect (`bx,by,w,h`)
/// shifted by the frame's own OFst offset, plus its part table.
#[derive(Clone)]
struct Geom {
    bx: i32,
    by: i32,
    w: i32,
    h: i32,
    dx: i32,
    dy: i32,
    parts: Vec<[i32; 7]>,
}

impl Geom {
    fn frame_box(&self) -> FrameBox {
        FrameBox { bx: self.bx, by: self.by, w: self.w, h: self.h, dx: self.dx, dy: self.dy }
    }
}

type Rect = (i32, i32, i32, i32); // (l, t, r, b) — the module's own order

fn rect_empty(r: Rect) -> bool {
    r.2 <= r.0 || r.3 <= r.1
}

/// `L132 fn432E` @432E — intersection, empty when they do not meet.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    if rect_empty(a) || rect_empty(b) {
        return None;
    }
    let r = (a.0.max(b.0), a.1.max(b.1), a.2.min(b.2), a.3.min(b.3));
    (!rect_empty(r)).then_some(r)
}

/// `L132 fn416E` @416E — union, skipping empties.
fn union(a: Rect, b: Rect) -> Rect {
    if rect_empty(a) {
        return b;
    }
    if rect_empty(b) {
        return a;
    }
    (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
}

/// `L132 fn4640` @4640 → `fn410C` @410C: centre `r` on `p`.
fn centre_on(r: Rect, p: (i32, i32)) -> Rect {
    let dx = p.0 - (r.0 + r.2) / 2;
    let dy = p.1 - (r.1 + r.3) / 2;
    (r.0 + dx, r.1 + dy, r.2 + dx, r.3 + dy)
}

fn area(r: Rect) -> i32 {
    (r.2 - r.0) * (r.3 - r.1)
}

// ---------------------------------------------------------------------------
// The mime

#[derive(Clone)]
struct Mime {
    // sequence state (L132 CompoundSprite)
    bank: u32,
    x: i32,          // +0x40
    y: i32,          // +0x42
    frame: i32,      // +0x3A
    first: i32,      // +0x44
    fresh: bool,     // +0x48
    finished: bool,  // +0x46
    queue: Vec<i32>, // +0x60 ring
    next_run: i32,   // +0x4E
    flip: bool,      // +0x3C bit 0

    // state sub-object at +0x92
    state: i32, // +0x96
    prev: i32,  // +0x98

    cell: Rect,    // +0x19E..+0x1A4
    hidden: bool,  // +0x152
    dying: bool,   // +0x12C
    dead: bool,    // +0x12E
    over: bool,    // +0x17C — the reticle is on me
    face: i32,     // +0x146
    neigh_prev: i32, // +0x142
    neigh_next: i32, // +0x144

    death_due: u64, // +0x154 — swap to bank 1000 at this ms
    hold_due: u64,  // +0x14C — the 0x74 bazooka hold
    wait_due: u64,  // +0x15C — fn21/fn22/fn23 deadline

    rep: i32,     // +0x160
    rep_n: i32,   // +0x162
    rep_run: i32, // +0x164

    c166: i32, // +0x166 counter (0x48)
    c168: i32, // +0x168 target  (0x40)
    c16a: i32, // +0x16A target  (0x3E)
    c16c: i32, // +0x16C counter (0x3D)
    c16e: i32, // +0x16E counter (0x30/0x32/0x36/0x38)
    c170: i32, // +0x170 target
    c172: i32, // +0x172 target  (0x15/0x23)
    c174: i32, // +0x174 counter (0x19/0x25)
    f176: i32, // +0x176 "been here" latch (0xE/0x12)
    f178: i32, // +0x178 net move of run 0x1B (0xE)

    hit: i32,          // +0x130 — fn35's part code
    side: i32,         // +0x158
    kill_weapon: i32,  // +0x148
    kill_run: i32,     // +0x134
    kill_kind: i32,    // +0x196
    saved: (i32, i32), // +0x136 / +0x138
    splat: (i32, i32), // +0x186 — fn16's point
    f19a: i32,         // +0x19A
}

impl Mime {
    fn new() -> Mime {
        Mime {
            bank: BANK_MIME,
            x: 0,
            y: 0,
            frame: 0,
            first: 0,
            fresh: false,
            finished: false,
            queue: Vec::new(),
            next_run: -1,
            flip: false,
            state: 0,
            prev: 0,
            cell: (0, 0, 0, 0),
            hidden: false,
            dying: false,
            dead: false,
            over: false,
            face: 0,
            neigh_prev: -1,
            neigh_next: -1,
            death_due: 0,
            hold_due: 0,
            wait_due: 0,
            rep: 0,
            rep_n: 0,
            rep_run: 0,
            c166: 0,
            c168: 0,
            c16a: 0,
            c16c: 0,
            c16e: 0,
            c170: 0,
            c172: 0,
            c174: 0,
            f176: 0,
            f178: 0,
            hit: 0,
            side: 0,
            kill_weapon: 0,
            kill_run: 0,
            kill_kind: 0,
            saved: (0, 0),
            splat: (0, 0),
            f19a: 0,
        }
    }
}

/// `g0512`'s 8 slots (`fn61` @57D4 / `fn72` @5D3E).
#[derive(Clone, Copy, Default)]
struct Decal {
    active: bool, // +0x2A
    kind: i32,    // +0x2C
    idx: i32,     // +0x24 — index into DECAL_IDS
    step: i32,    // +0x26
    tick: bool,   // +0x28
    x: i32,
    y: i32,
}

/// The ambient crosshair driver (`fn10` @1212, fields +0x04..+0x6A).
/// Occupancy observed on the rig (see the header); fire gates UNOBSERVED.
#[derive(Clone, Default)]
struct Wanderer {
    x: f64,   // +0x2C
    y: f64,   // +0x36
    vx: f64,  // +0x04
    vy: f64,  // +0x0E
    ax: f64,  // +0x18
    ay: f64,  // +0x22
    speed: f64, // +0x40
    period: i32, // +0x54
    frames: i32, // +0x58
    on_mime: i32, // +0x56
    fire_due: u64, // +0x5A
    burst_end: u64, // +0x5E
    stalk_until: u64, // +0x62
    phase_due: u64, // +0x66
    phase: i32,     // +0x6A
}

// ---------------------------------------------------------------------------

pub struct MimeHunt {
    pack: Pack,
    geom: HashMap<(u32, i32), Geom>,
    block_end: HashMap<(u32, i32), i32>,
    /// GAP(fn113/fn114): the largest frame per bank stands in for the
    /// sequences' GetBounds.
    bank_max: HashMap<u32, (i32, i32)>,
    /// Test-only: one record per `fn028A` hand-off —
    /// (mime, from frame, marker, run id, flip before, flip after, dx, dy).
    #[cfg(test)]
    handoffs: Vec<(usize, i32, i32, i32, bool, bool, i32, i32)>,

    // raw control values
    mimes_raw: i32,
    weapon_raw: i32,
    mood_raw: i32,
    music_raw: i32,

    // latched
    weapon: i32,      // g029A
    saved_weapon: i32, // g0296
    mood: i32,        // g00D6
    music_band: u32,  // g0528
    music_plays: u32, // +0x1C
    music_next_ok: u64,
    weapon_roll_due: u64, // +0x46

    field: Rect, // g02B4..g02BA

    mimes: Vec<Mime>,
    want: i32,      // g0230
    mimes_latch: i32, // g0290 — the control value the grid was built for

    decals: [Decal; 8],

    // the crosshair
    wander: Wanderer,
    ret_pt: (i32, i32),
    ret_rect: Rect, // +0x16
    ret_frame: i32, // +0x2C
    ret_over: bool, // +0x2A
    ret_due: u64,   // +0x26

    interactive: bool, // g02A6
    caps_prev: bool,
    prev_mouse_down: bool,

    fire_due: u64,   // +0x3A
    event_at: u64,   // +0x3E
    corner: i32,     // +0x4A
    corner_due: u64, // +0x4C
    all_dead_due: u64, // +0x36
    built_at: u64,   // +0x14
    live_at: u64,    // g028A — ambient grace
    adv_at: u64,     // g0286
    need_clear: bool, // +0x18
    quit: bool,      // +0x1A
    ret_live: bool,  // the fn130 draw gate

    started: bool,
    cleared: bool,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

/// Concrete constructor — the tests drive the module directly.
fn build(pack: Pack) -> Option<MimeHunt> {
    for base in [BANK_MIME, BANK_KILL] {
        if !pack.meta.series.contains_key(&base.to_string()) {
            return None;
        }
    }
    let mut geom = HashMap::new();
    let mut block_end = HashMap::new();
    let mut bank_max: HashMap<u32, (i32, i32)> = HashMap::new();
    for base in [BANK_MIME, BANK_KILL] {
        let mut mx = (0i32, 0i32);
        for seq in pack.series(base) {
            let end = seq.first as i32 + seq.frames.len() as i32 - 1;
            for (i, f) in seq.frames.iter().enumerate() {
                let id = seq.first as i32 + i as i32;
                geom.insert(
                    (base, id),
                    Geom {
                        bx: f.bx,
                        by: f.by,
                        w: f.w,
                        h: f.h,
                        dx: f.dx,
                        dy: f.dy,
                        parts: f.parts.clone(),
                    },
                );
                block_end.insert((base, id), end);
                mx.0 = mx.0.max(f.w);
                mx.1 = mx.1.max(f.h);
            }
        }
        bank_max.insert(base, mx);
    }
    Some(MimeHunt {
        pack,
        geom,
        block_end,
        bank_max,
        mimes_raw: 40,
        weapon_raw: 1,
        mood_raw: 1,
        music_raw: 30,
        weapon: 1,
        saved_weapon: 1,
        mood: 1,
        music_band: 0,
        music_plays: 0,
        music_next_ok: 0,
        weapon_roll_due: 0,
        field: (0, 0, SCREEN_W, SCREEN_H),
        mimes: Vec::new(),
        want: 0,
        mimes_latch: -1,
        decals: [Decal::default(); 8],
        wander: Wanderer::default(),
        ret_pt: (SCREEN_W / 2, SCREEN_H / 2),
        ret_rect: (0, 0, 0, 0),
        ret_frame: 2,
        ret_over: false,
        ret_due: 0,
        interactive: false,
        caps_prev: false,
        prev_mouse_down: false,
        fire_due: 0,
        event_at: 0,
        corner: 0,
        corner_due: 0,
        all_dead_due: 0,
        built_at: 0,
        live_at: 0,
        adv_at: 0,
        need_clear: true,
        quit: false,
        ret_live: false,
        started: false,
        cleared: false,
        #[cfg(test)]
        handoffs: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// bank helpers

impl MimeHunt {
    fn g(&self, bank: u32, id: i32) -> Option<&Geom> {
        self.geom.get(&(bank, id))
    }

    /// The frame's L135 geometry, off the module's own map.
    fn frame_box(&self, bank: u32, id: i32) -> Option<FrameBox> {
        self.g(bank, id).map(Geom::frame_box)
    }

    /// Sequence `+0x78` = L135 `fn3DDC @3DDC` ([`l135::link`], the
    /// transcribed flip-bit rounding). No move when either id is unknown.
    fn link(&self, bank: u32, from: i32, to: i32, flip: bool) -> (i32, i32) {
        match (self.frame_box(bank, from), self.frame_box(bank, to)) {
            (Some(a), Some(b)) => l135::link(&a, &b, flip, LinkModel::FN3DDC),
            _ => (0, 0),
        }
    }

    /// L135 `fn3BD6 @3BD6`'s layout of one part, relative to the frame
    /// centre ([`FrameBox::part_rel`]).
    fn part_rel(&self, bank: u32, id: i32, part: &[i32; 7], flip: bool) -> Option<[i32; 4]> {
        Some(self.frame_box(bank, id)?.part_rel(part, flip))
    }

    /// `first − 1 + len(first)` (L135 `fn4456`): the end of the OFst block
    /// CONTAINING `first` — never an exact-`first` match (§10.1).
    fn last_of(&self, bank: u32, first: i32) -> i32 {
        self.block_end.get(&(bank, first)).copied().unwrap_or(first)
    }

    fn size(&self, bank: u32, id: i32) -> (i32, i32) {
        self.g(bank, id).map(|g| (g.w, g.h)).unwrap_or((0, 0))
    }

    /// `+0xFC` = `fn12DE(this, 1, run)`: how far a whole run moves the
    /// sprite. The C zeroes pos, seats `+0x3A` on `fn1210(run)` = the run
    /// id itself, and links it to `fn1260(run)` = the block's last frame
    /// in ONE `fn0DF4` step — `fn3DDC` under the sprite's CURRENT flip, so
    /// a mirrored mime asks about the mirrored walk.
    fn net_move(&self, bank: u32, run: i32, flip: bool) -> (i32, i32) {
        self.link(bank, run, self.last_of(bank, run), flip)
    }
}

// ---------------------------------------------------------------------------
// sprite ops (L132_Resource, exactly as frankenscreen binds them)

impl MimeHunt {
    /// The part pair `fn3F2E` registers on ([`l135::first_shared_part`]).
    #[cfg(test)]
    fn first_shared_part(&self, bank: u32, a: i32, b: i32) -> Option<([i32; 7], [i32; 7])> {
        let (ga, gb) = (self.g(bank, a)?, self.g(bank, b)?);
        l135::first_shared_part(&ga.parts, &gb.parts).map(|(pa, pb)| (*pa, *pb))
    }

    /// L135 **`fn3F2E` @3F2E** ([`l135::register`]) — the **shared-part
    /// link** `fn0D3C` runs on every hand-off: the first part the two
    /// frames share keeps its screen position, and the sprite's `+0x3C`
    /// flip is XORed with the two parts' OWN flip bits — a pose whose
    /// shared part is authored mirrored hands off to a native one by
    /// MIRRORING THE SPRITE. No part in common → no move, no flip change.
    /// Returns `(dx, dy, flip')`.
    ///
    /// GAP(vertical flip): the part flags' vertical bit XORs the sprite's
    /// `+0x3C` bit 1 the same way. No hand-off this module makes has a
    /// shared part whose vertical bit differs (trace_mime_hand_offs: 0 of
    /// 1626), and `SpriteDraw` has no vertical flip, so bit 1 is not carried
    /// (`Registration::flag_xor` reports it).
    fn shared_link(&self, bank: u32, a: i32, b: i32, flip: bool) -> (i32, i32, bool) {
        let (Some(ga), Some(gb)) = (self.g(bank, a), self.g(bank, b)) else {
            return (0, 0, flip);
        };
        match l135::register(&ga.frame_box(), &ga.parts, &gb.frame_box(), &gb.parts, flip) {
            Some(r) => (r.delta.0, r.delta.1, r.flip),
            None => (0, 0, flip),
        }
    }

    /// `+0x108` = `fn028A` @028A — start run `id`. **Two steps, and the
    /// first one is the whole story**:
    ///
    /// 1. `+0xF0` = `fn1186` @1186 turns the new run id into the frame to
    ///    pass through: `id − 1` when that record exists (`+0x82` set and
    ///    `fn441A` says valid), i.e. **the block's lead-in record — the
    ///    link marker**. 97 of the 98 run ids this module names are
    ///    `blockStart + 1`, so the marker is there every time but once.
    /// 2. `+0xCC` = `fn0D3C` @0D3C links the CURRENT frame to that marker
    ///    through the sequence's `+0x80` = L135 `fn3F2E` — the shared-part
    ///    registration, not a centre difference — writing BOTH `pos` and
    ///    the sprite's `+0x3C` flip (`+0x80` is 1 after the reset), and
    ///    seats `+0x3A` on the marker.
    /// 3. `+0xF4` = `fn1210` @1210 / `+0xD0` = `fn0DF4` @0DF4 then link the
    ///    marker to the run's first frame through `+0x7C` → `+0x78` =
    ///    `fn3DDC` @3DDC, under the flip step 2 just stored.
    ///
    /// Because the marker is authored at the run's own origin, step 3 is
    /// 0 for 64 of the 98 ids and ≤ 8 px for the rest, and step 2 does the
    /// real work: the outgoing pose and the marker share their parts, so
    /// the sprite does not move. **A run's travel is kept, not undone** —
    /// which is what the 60 s capture shows (mimes walk 137–197 px spans,
    /// no frame-to-frame step over 16 px).
    fn set_run_inner(&mut self, i: usize, id: i32) {
        let (bank, frame, flip) = {
            let m = &self.mimes[i];
            (m.bank, m.frame, m.flip)
        };
        if frame != 0 {
            // `fn441A` here: the record is in the map (no id floor)
            let marker = l135::marker_of(id, |f| self.g(bank, f).is_some());
            let (dx, dy, flip2) = self.shared_link(bank, frame, marker, flip);
            // marker → first under the flip `fn0D3C` just stored
            let (dx2, dy2) = self.link(bank, marker, id, flip2);
            #[cfg(test)]
            self.handoffs.push((i, frame, marker, id, flip, flip2, dx + dx2, dy + dy2));
            let m = &mut self.mimes[i];
            m.x += dx + dx2;
            m.y += dy + dy2;
            m.flip = flip2;
        }
        let m = &mut self.mimes[i];
        m.frame = id;
        m.first = id;
        m.fresh = true;
        m.next_run = -1;
    }

    /// `+0x7C` = `fn0204`: clear the queue, `fn028A`, clear `+0x46`.
    fn set_run(&mut self, i: usize, id: i32) {
        self.mimes[i].queue.clear();
        self.set_run_inner(i, id);
        self.mimes[i].finished = false;
    }

    /// `+0x80` = `fn0240(this, id₀, id₁, …, −1)`.
    fn play(&mut self, i: usize, ids: &[i32]) {
        self.set_run(i, ids[0]);
        self.mimes[i].queue.extend_from_slice(&ids[1..]);
    }

    /// `+0x84` = `fn0316`, the per-frame advance.
    fn advance(&mut self, i: usize) {
        self.mimes[i].finished = false;
        let next = self.mimes[i].next_run;
        if next != -1 {
            self.set_run_inner(i, next);
        }
        let (bank, first, flip, frame, fresh) = {
            let m = &self.mimes[i];
            (m.bank, m.first, m.flip, m.frame, m.fresh)
        };
        let last = self.last_of(bank, first);
        if fresh {
            self.mimes[i].fresh = false;
        } else if frame < last && frame >= first {
            let (dx, dy) = self.link(bank, frame, frame + 1, flip);
            let m = &mut self.mimes[i];
            m.x += dx;
            m.y += dy;
            m.frame = frame + 1;
        }
        let m = &mut self.mimes[i];
        if m.frame >= last || m.frame < first {
            if m.queue.is_empty() {
                m.finished = true;
            } else {
                m.next_run = m.queue.remove(0);
            }
        }
    }

    /// The drawn frame rect, centred on pos (port-plan §2).
    fn rect(&self, i: usize) -> Rect {
        let m = &self.mimes[i];
        let (w, h) = self.size(m.bank, m.frame);
        (m.x - w / 2, m.y - h / 2, m.x - w / 2 + w, m.y - h / 2 + h)
    }

    /// `+0xE8` = `M130_fn34` @47C8 → `fn10FE` → L135 `fn4488` @4488: the
    /// rect of the part on channel `chan`, on the drawn frame.
    fn channel_rect(&self, i: usize, chan: i32) -> Rect {
        let m = &self.mimes[i];
        let Some(g) = self.g(m.bank, m.frame) else { return (0, 0, 0, 0) };
        for p in &g.parts {
            if p[1] == chan {
                return self.part_screen(i, p);
            }
        }
        (0, 0, 0, 0)
    }

    /// One part's rect on the screen, laid out under the mime's flip the
    /// way `fn4488` gets it — through `+0x74` = L135 `fn3BD6 @3BD6`, which
    /// mirrors every part inside the frame rect while the sprite is
    /// flipped. (The port used to hand back the unmirrored rect, so a
    /// mirrored mime was hit-tested as its own reflection.)
    fn part_screen(&self, i: usize, p: &[i32; 7]) -> Rect {
        let m = &self.mimes[i];
        match self.part_rel(m.bank, m.frame, p, m.flip) {
            Some(r) => (r[0] + m.x, r[1] + m.y, r[2] + m.x, r[3] + m.y),
            None => (0, 0, 0, 0),
        }
    }

    /// The rect of the Nth part whose art id lands in `[lo, hi]` — `fn37`
    /// @507E finds that part's channel and `fn4488` turns it into a rect;
    /// the pack's part table carries both, so this is one step.
    fn part_rect(&self, i: usize, range: (i32, i32), nth: i32) -> Rect {
        let m = &self.mimes[i];
        let Some(g) = self.g(m.bank, m.frame) else { return (0, 0, 0, 0) };
        let mut seen = 0;
        for p in &g.parts {
            if p[0] >= range.0 && p[0] <= range.1 {
                seen += 1;
                if seen == nth {
                    return self.part_screen(i, p);
                }
            }
        }
        (0, 0, 0, 0)
    }
}

// ---------------------------------------------------------------------------
// The ambient wanderer — `fn13`..`fn30`. Motion observed; fire gates UNOBSERVED.

impl MimeHunt {
    /// `fn15` @13C4: re-latch the wander speed and re-roll period from the
    /// CyberMood, including mood 4's stalk window and mood 5's phase.
    fn wander_speed(&mut self, now: u64) {
        let (mut speed, mut period) = (0.2_f64, 3);
        match self.mood {
            1 | 5 => {
                speed = 0.05;
                period = 7;
            }
            2 => speed = 0.2,
            3 | 4 => {
                speed = 0.65;
                period = 3;
            }
            _ => {}
        }
        if self.mood == 4 {
            if now < self.wander.stalk_until {
                speed = 0.05;
                period = 7;
            }
        } else if self.mood == 5 {
            if self.wander.phase == 0 {
                speed = 0.05;
                period = 7;
            } else {
                speed = 0.65;
                period = 3;
            }
        }
        self.wander.speed = speed;
        self.wander.period = period;
    }

    /// `fn16` @14F4: re-arm the Manic phase timer.
    fn wander_phase_timer(&mut self, ctx: &mut Ctx, now: u64) {
        self.wander.phase_due = if self.wander.phase == 0 {
            now + 4_000 + u64::from(ctx.rng.pct(20_000))
        } else {
            now + 20_000 + u64::from(ctx.rng.pct(40_000))
        };
    }

    /// `fn19` @17AC: re-roll the acceleration.
    fn wander_roll(&mut self, ctx: &mut Ctx, now: u64) {
        self.wander_speed(now);
        let s = self.wander.speed;
        let r = ctx.rng.pct(100) as i32;
        if (0..=0x20).contains(&r) {
            self.wander.ax = -s;
        } else if (0x21..=0x41).contains(&r) {
            self.wander.ax = s;
        } else if (0x5A..=0x5D).contains(&r) {
            self.wander.ax = -self.wander.ax;
        } else if (0x50..=0x59).contains(&r) {
            self.wander.ax = -self.wander.ax / 2.0;
        } else {
            self.wander.ax = 0.0;
        }
        let r = ctx.rng.pct(100) as i32;
        if (0..=0x20).contains(&r) {
            self.wander.ay = -s;
        } else if (0x21..=0x41).contains(&r) {
            self.wander.ay = 0.0;
        } else {
            self.wander.ay = s;
        }
    }

    /// `fn20` @1964: steer off the walls. Not a clamp — the decompile lost
    /// every compare here, so this is read off the listing: the margin is
    /// 64 px while the re-roll period is ≤ 5 (`cmp`/`ble` @1980) and 84 px
    /// otherwise; inside `field.left + margin` the x acceleration becomes
    /// `+speed` (FOCPX/`bge` @19CE), past `field.right − margin` it becomes
    /// `−speed` (`ble` @1A32, `xori.b #$80` @1A46), and the same for y
    /// against top (@1A9C) and bottom (@1B00). Runs every step, after the
    /// `fn19` roll, so near a wall it overrides the roll. The integrator
    /// still has no velocity damping and `fn18` still only clamps position,
    /// so a fast reticle overshoots into the wall and sits there until the
    /// push turns it — briefly, not forever.
    fn wander_steer(&mut self) {
        let margin = if self.wander.period <= 5 { 64 } else { 84 };
        let s = self.wander.speed;
        if self.wander.x < f64::from(self.field.0 + margin) {
            self.wander.ax = s;
        }
        if self.wander.x > f64::from(self.field.2 - margin) {
            self.wander.ax = -s;
        }
        if self.wander.y < f64::from(self.field.1 + margin) {
            self.wander.ay = s;
        }
        if self.wander.y > f64::from(self.field.3 - margin) {
            self.wander.ay = -s;
        }
    }

    /// `fn21` @1B36: one integrator step. `fn20` @1964 steers off the walls
    /// first, `fn22` @1D3E damps for the stalk.
    fn wander_step(&mut self, ctx: &mut Ctx, now: u64) {
        let n = self.wander.frames;
        self.wander.frames += 1;
        if self.wander.period < n {
            self.wander_roll(ctx, now);
            self.wander.frames = 0;
        }
        self.wander_steer();

        self.wander.vx += self.wander.ax * WANDER_STEP;
        self.wander.vy += self.wander.ay * WANDER_STEP;
        self.wander.vx = self.wander.vx.clamp(-WANDER_VMAX, WANDER_VMAX);
        self.wander.vy = self.wander.vy.clamp(-WANDER_VMAX, WANDER_VMAX);

        // fn22 — the Sadistic stalk clamp
        if self.mood == 4 && self.wander.on_mime != 0 && now < self.wander.stalk_until {
            self.wander.vx = self.wander.vx.clamp(-STALK_VMAX, STALK_VMAX);
            self.wander.vy = self.wander.vy.clamp(-STALK_VMAX, STALK_VMAX);
        }

        self.wander.x += self.wander.vx * WANDER_STEP;
        self.wander.y += self.wander.vy * WANDER_STEP;

        // fn18 @15E8 — clamp into the play field
        self.wander.x = self.wander.x.clamp(f64::from(self.field.0), f64::from(self.field.2));
        self.wander.y = self.wander.y.clamp(f64::from(self.field.1), f64::from(self.field.3));
    }

    /// `fn28` @1FC2: arm the next ambient shot.
    fn wander_arm(&mut self, ctx: &mut Ctx, now: u64) {
        self.wander.burst_end = 0;
        self.wander.fire_due = match self.mood {
            2 => now + 15_000 + u64::from(ctx.rng.pct(15_000)),
            3 => now + 100 + u64::from(ctx.rng.pct(0x2EE)),
            4 => now + 3_000 + u64::from(ctx.rng.pct(20_000)),
            5 => {
                if self.wander.phase == 0 {
                    now + 15_000 + u64::from(ctx.rng.pct(15_000))
                } else {
                    now + 100 + u64::from(ctx.rng.pct(0x2EE))
                }
            }
            _ => 0,
        };
    }

    /// `fn29` @20FC: the (dead) weapon-2 burst window.
    fn wander_burst(&mut self, ctx: &mut Ctx, now: u64) {
        self.wander.burst_end = match self.mood {
            2 => now + 100 + u64::from(ctx.rng.pct(2_000)),
            3 => now + 3_000 + u64::from(ctx.rng.pct(5_000)),
            4 => now + 700 + u64::from(ctx.rng.pct(2_000)),
            5 => {
                if self.wander.phase == 0 {
                    now + 100 + u64::from(ctx.rng.pct(2_000))
                } else {
                    now + 3_000 + u64::from(ctx.rng.pct(5_000))
                }
            }
            _ => return,
        };
    }

    /// `fn24` @1ECE.
    fn wander_should_fire_inner(&mut self, ctx: &mut Ctx, now: u64) -> bool {
        if self.weapon == 2 {
            if self.wander.burst_end == 0 {
                if self.wander.fire_due < now {
                    self.wander_burst(ctx, now);
                    return true;
                }
                false
            } else if now < self.wander.burst_end {
                true
            } else {
                self.wander_arm(ctx, now);
                false
            }
        } else if self.wander.fire_due < now {
            self.wander_arm(ctx, now);
            true
        } else {
            false
        }
    }

    /// `fn23` @1E62 — the CyberMood dispatch. Mood 1 never shoots.
    fn wander_should_fire(&mut self, ctx: &mut Ctx, now: u64) -> bool {
        match self.mood {
            2 | 3 | 4 => self.wander_should_fire_inner(ctx, now),
            5 => {
                // fn27 @1F76
                if self.wander.phase_due < now {
                    self.wander_phase_timer(ctx, now);
                    self.wander.phase = i32::from(self.wander.phase == 0);
                }
                if self.wander.phase == 0 {
                    false
                } else {
                    self.wander_should_fire_inner(ctx, now)
                }
            }
            _ => false,
        }
    }

    /// `fn30` @2220 — Sadistic arms a stalk when the reticle first lands
    /// on a live mime and drops it when the reticle leaves.
    fn wander_on_mime(&mut self, ctx: &mut Ctx, now: u64, live: i32) {
        if self.mood == 4 {
            if self.wander.on_mime == 0 {
                if live == 1 {
                    self.wander.stalk_until = now + 10_000 + u64::from(ctx.rng.pct(15_000));
                    self.wander_arm(ctx, now);
                }
            } else if live == 0 {
                self.wander.stalk_until = 0;
            }
        }
        self.wander.on_mime = live;
    }
}

// ---------------------------------------------------------------------------
// build / teardown

impl MimeHunt {
    /// `fn118` @39BE: the play field, floored at 512×384.
    fn set_field(&mut self) {
        let mut r: Rect = (0, 0, SCREEN_W, SCREEN_H);
        if r.2 - r.0 < MIN_FIELD_W {
            r.2 = r.0 + MIN_FIELD_W;
        }
        if r.3 - r.1 < MIN_FIELD_H {
            r.3 = r.1 + MIN_FIELD_H;
        }
        self.field = r;
    }

    /// GAP(fn113 @364A / fn114 @36DA): the mime box, + the 32 px `g0108`
    /// margin.
    fn mime_box(&self) -> (i32, i32) {
        let a = self.bank_max.get(&BANK_KILL).copied().unwrap_or((0, 0));
        let b = self.bank_max.get(&BANK_MIME).copied().unwrap_or((0, 0));
        (a.0.max(b.0) + BOX_PAD, a.1.max(b.1) + BOX_PAD)
    }

    /// `fn112` @3610 / `fn111` @35E4 / `fn110` @35B0.
    fn grid_shape(&self) -> (i32, i32, i32) {
        let (bw, bh) = self.mime_box();
        let rows = if bh > 0 { ((self.field.3 - self.field.1) / bh).min(3) } else { 1 };
        let cols = if bw > 0 { (self.field.2 - self.field.0) / bw } else { 1 };
        ((rows * cols).min(4).max(1), rows, cols)
    }

    /// `fn122` @3BF4: population from the Mimes control.
    fn population(&self) -> i32 {
        let (cap, _, _) = self.grid_shape();
        (cap * self.mimes_raw / 100 + 1).min(cap)
    }

    /// `fn124` @3CCC: re-read Weapon and CyberMood. Ambient only.
    fn read_weapon_mood(&mut self, ctx: &mut Ctx, now: u64) {
        if self.weapon_raw == 5 {
            if self.weapon_roll_due < now {
                let pick = (ctx.rng.pct(3) as i32) + 1;
                self.weapon_roll_due = now + 5_000 + u64::from(ctx.rng.pct(20_000));
                self.weapon = pick;
                if self.weapon > 1 {
                    self.weapon += 1;
                }
            }
        } else {
            self.weapon = self.weapon_raw;
            if self.weapon > 1 {
                self.weapon += 1;
            }
        }
        self.mood = self.mood_raw;
    }

    /// `fn123` @3C3A: the Music play-count band. Latched once, from the
    /// constructor's `fn135`.
    fn latch_music(&mut self) {
        let raw = self.music_raw;
        self.music_band = if raw < 5 {
            0
        } else if raw < 25 {
            1
        } else if raw < 50 {
            2
        } else if raw < 75 {
            3
        } else if raw < 95 {
            4
        } else {
            99
        };
        self.music_plays = 0;
    }

    /// `fn108` @2F1E: rebuild the crosshair, the mimes and the grid.
    fn rebuild(&mut self, ctx: &mut Ctx, now: u64) {
        self.set_field();
        // fn13 @124A — place the wanderer
        let w = self.field.2 - self.field.0;
        let h = self.field.3 - self.field.1;
        self.wander = Wanderer::default();
        self.wander.x = f64::from(ctx.rng.pct(w.max(1) as u32) as i32);
        self.wander.y = f64::from(ctx.rng.pct(h.max(1) as u32) as i32);
        // fn14 @1328: fn28, then a 50/50 sign on the speed, then fn15
        self.wander_arm(ctx, now);
        self.wander_phase_timer(ctx, now);
        let _sign = ctx.rng.pct(100); // fn14's 0x33 coin — sets ±speed
        self.wander_speed(now);

        self.want = self.population();
        self.mimes_latch = self.mimes_raw;
        self.mimes = (0..self.want).map(|_| Mime::new()).collect();
        self.layout(ctx);

        self.ret_frame = 2;
        self.ret_over = false;
        self.ret_due = 0;
        self.corner = 0;
        self.corner_due = 0;
        self.all_dead_due = 0;
        self.live_at = now + AMBIENT_GRACE_MS;
        self.adv_at = now;
        self.decals = [Decal::default(); 8];
    }

    /// `fn109` @30EA: split the population across rows and hand each mime
    /// a 210 px cell plus a share of the row's slack.
    fn layout(&mut self, ctx: &mut Ctx) {
        let n = self.want;
        if n == 0 {
            return;
        }
        if n == 1 {
            self.mimes[0].cell = self.field;
            self.place(ctx, 0);
            self.mimes[0].neigh_prev = -1;
            self.mimes[0].neigh_next = -1;
            return;
        }
        let (_, rows, _cols) = self.grid_shape();
        let w = self.field.2 - self.field.0;
        let h = self.field.3 - self.field.1;
        let cellcols = w / CELL_W;

        // which row count, and which table row
        let mut nrows = 3;
        let mut two = false;
        if n < 7 && (cellcols > 2 || rows < 3) {
            let r = ctx.rng.pct(100) as i32;
            if r < 0x33 || rows < 3 {
                if rows < 2 {
                    nrows = 1;
                } else {
                    nrows = 2;
                    two = true;
                }
            }
        }
        let mut f8 = 0i32;
        if nrows == 3 {
            f8 = match n {
                9 => 0,
                8 => 1,
                7 => (ctx.rng.pct(100) / 0x32) as i32 + 2,
                6 => {
                    if cellcols < 3 {
                        5
                    } else {
                        (ctx.rng.pct(100) / 0x32) as i32 + 4
                    }
                }
                5 => 6,
                4 => 7,
                3 => 8,
                _ => 9,
            };
        } else if two {
            f8 = match n {
                6 => 0,
                5 => 1,
                4 => {
                    if cellcols < 3 {
                        3
                    } else {
                        (ctx.rng.pct(100) / 0x32) as i32 + 2
                    }
                }
                3 => 4,
                _ => 5,
            };
        }

        let rowh = h / nrows;
        let mut rot = ctx.rng.pct(100) as i32;
        let mut slot = 0usize;
        let mut prev: i32 = -1;
        for li in 0..nrows {
            let cnt = if nrows < 2 {
                n
            } else if nrows == 3 {
                ROWS3[f8.clamp(0, 9) as usize][(rot.rem_euclid(3)) as usize]
            } else {
                ROWS2[f8.clamp(0, 5) as usize][(rot.rem_euclid(2)) as usize]
            };
            if cnt != 0 {
                let mut slack = (w - cnt * CELL_W).max(0);
                let mut acc = 0i32;
                for j in 0..cnt {
                    if slot >= self.mimes.len() {
                        break;
                    }
                    let gap = if j == cnt - 1 {
                        slack
                    } else {
                        let g = ctx.rng.pct(slack.max(1) as u32) as i32;
                        slack -= g;
                        g
                    };
                    let l = self.field.0 + acc + j * CELL_W;
                    let t = self.field.1 + li * rowh;
                    let r = self.field.0 + gap + acc + (j + 1) * CELL_W;
                    let b = self.field.1 + (li + 1) * rowh;
                    acc += gap;
                    self.mimes[slot].cell = (l, t, r, b);
                    self.place(ctx, slot);
                    self.mimes[slot].neigh_prev = prev;
                    self.mimes[slot].neigh_next = -1;
                    if prev != -1 {
                        self.mimes[prev as usize].neigh_next = slot as i32;
                    }
                    prev = slot as i32;
                    slot += 1;
                }
            }
            rot += 1;
        }
    }

    /// `M130_fn32` @4564: drop the mime in the horizontal centre of its
    /// cell with a vertical jitter. The horizontal jitter is rolled and
    /// discarded — kept as an RNG consumer.
    fn place(&mut self, ctx: &mut Ctx, i: usize) {
        let (bw, bh) = self.mime_box();
        let cell = self.mimes[i].cell;
        let cw = cell.2 - cell.0;
        let ch = cell.3 - cell.1;
        if cw - bw > 1 {
            let _ = ctx.rng.pct((cw - bw) as u32); // discarded (quirk)
        }
        let mut jitter = 0i32;
        if ch - bh > 1 {
            jitter = (ctx.rng.pct((ch - bh) as u32) as i32) >> 1;
        }
        let _ = ctx.rng.pct(100); // fn32's first unused roll
        if ctx.rng.pct(100) > 0x32 {
            jitter = -jitter;
        }
        let m = &mut self.mimes[i];
        m.x = cell.0 + cw / 2;
        m.y = jitter + cell.1 + ch / 2;
    }

    /// `fn140` @4F06: every mime dead?
    fn all_dead(&self) -> bool {
        !self.mimes.is_empty() && self.mimes.iter().all(|m| m.dead)
    }

    /// `fn129` @44D0.
    fn wants_rebuild(&mut self, now: u64) -> bool {
        if self.mimes_latch != self.mimes_raw {
            return true;
        }
        if self.all_dead_due == 0 && self.all_dead() {
            self.all_dead_due = now + ALL_DEAD_MS;
        }
        if !self.interactive && now.saturating_sub(self.built_at) > AMBIENT_LIFE_MS {
            self.all_dead_due = 0;
            self.fire_due = now;
            return true;
        }
        if self.interactive {
            if self.quit {
                self.fire_due = now;
                self.all_dead_due = 0;
                self.quit = false;
                return true;
            }
            if self.fire_due + INTERACTIVE_LIFE_MS < now {
                self.fire_due = now;
                self.all_dead_due = 0;
                return true;
            }
        }
        if self.all_dead_due != 0 && now > self.all_dead_due {
            self.fire_due = now;
            self.all_dead_due = 0;
            return true;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// the reticle

impl MimeHunt {
    /// `fn44` @2870: read the crosshair's point source. In ambient that is
    /// the wanderer's `+8` = `fn17` @1570, which **steps the integrator**,
    /// so every `fn44` call moves the crosshair. `fn130` makes one call per
    /// mime (@4714) plus one in `fn125` and one in `fn40`, so the ambient
    /// crosshair advances `mimes + 2` times per frame. Quirk kept.
    fn point(&mut self, ctx: &mut Ctx, now: u64) -> (i32, i32) {
        if self.interactive {
            (ctx.mouse.0.max(0), ctx.mouse.1.max(0))
        } else {
            self.wander_step(ctx, now);
            (self.wander.x.round() as i32, self.wander.y.round() as i32)
        }
    }

    /// `fn40` @2436: the crosshair box, centred on a fresh `fn44` point and
    /// clamped to the canvas.
    fn reticle_update(&mut self, ctx: &mut Ctx, now: u64) {
        let pt = self.point(ctx, now);
        self.ret_pt = pt;
        let s = RETICLE_BOX[(self.weapon.clamp(1, 5) - 1) as usize];
        let mut r = centre_on((0, 0, s, s), pt);
        // fn40's clamp into the canvas
        if r.0 < 0 {
            let d = -r.0;
            r.0 += d;
            r.2 += d;
        }
        if r.1 < 0 {
            let d = -r.1;
            r.1 += d;
            r.3 += d;
        }
        if r.2 > SCREEN_W {
            let d = SCREEN_W - r.2;
            r.0 += d;
            r.2 += d;
        }
        if r.3 > SCREEN_H {
            let d = SCREEN_H - r.3;
            r.1 += d;
            r.3 += d;
        }
        self.ret_rect = r;
    }

    /// `fn45` @28BA: the reticle art. 2 while nothing live is under it,
    /// 6..12 cycling while something is, on a 33 ms gate.
    fn reticle_anim(&mut self, ctx: &mut Ctx, now: u64, over: bool) {
        if self.ret_due < now {
            let was = self.ret_over;
            self.ret_over = over;
            if !self.ret_over {
                self.ret_frame = 2;
            } else if !was {
                self.ret_frame = 6;
            } else {
                self.ret_frame += 1;
                if self.ret_frame > 12 {
                    self.ret_frame = 6;
                }
            }
            self.ret_due = now + RETICLE_GATE_MS;
        }
        let live = i32::from(over);
        self.wander_on_mime(ctx, now, live);
    }

    /// `M130_fn38` @5162: is the weapon's reticle box over this mime?
    fn reticle_over(&mut self, i: usize, pt: (i32, i32)) -> bool {
        let s = RETICLE_BOX[(self.weapon.clamp(1, 5) - 1) as usize];
        let box_ = centre_on((0, 0, s, s), pt);
        let m = &self.mimes[i];
        if m.hidden {
            return false;
        }
        let target = if m.dead { self.channel_rect(i, 1) } else { self.rect(i) };
        intersect(box_, target).is_some()
    }

    /// `fn126` @3FCA — the corner code that unlocks the secret MimeSlayer.
    /// **Nothing in the decompiled C calls it** (see the GAP list), so the
    /// latch it drives never advances and `fn127`'s secret branch is
    /// unreachable. Ported, not wired: inventing a caller would be exactly
    /// the kind of gap-filling this port exists to undo.
    #[allow(dead_code)]
    fn corner_code(&mut self, now: u64) {
        let p = self.ret_pt;
        if self.corner_due < now {
            self.corner = 0;
            self.corner_due = now + CORNER_WINDOW_MS;
            return;
        }
        let f = self.field;
        match self.corner {
            0 => {
                if (f.0..=f.0 + CORNER).contains(&p.0) && (f.1..=f.1 + CORNER).contains(&p.1) {
                    self.corner += 1;
                    self.corner_due = now + CORNER_WINDOW_MS;
                }
            }
            1 => {
                if (f.2 - CORNER..=f.2).contains(&p.0) && (f.3 - CORNER..=f.3).contains(&p.1) {
                    self.corner += 1;
                    self.corner_due = now + CORNER_WINDOW_MS;
                }
            }
            _ => self.corner = 0,
        }
    }
}

// ---------------------------------------------------------------------------
// firing

impl MimeHunt {
    /// `fn125` @3D68 — the trigger. Ambient asks the CyberMood; interactive
    /// takes the mouseDown edge (and holds for the dead weapon 2).
    fn fire_tick(&mut self, ctx: &mut Ctx, now: u64, click: bool) {
        let refire = REFIRE_MS[(self.weapon.clamp(1, 5) - 1) as usize];
        let mut fired = false;
        if !self.interactive {
            if self.wander_should_fire(ctx, now) && self.fire_due + refire < now {
                self.fire_due = now;
                fired = true;
            }
        } else {
            if click && self.fire_due + refire < now {
                self.event_at = now;
                self.fire_due = now;
                fired = true;
            }
        }
        if !fired {
            return;
        }
        // fn125 @3F04 / @3EDE takes a FRESH fn44 point for the shot
        let pt = self.point(ctx, now);
        let snd = WEAPON_SND[(self.weapon.clamp(1, 5) - 1) as usize];
        // weapon 4 cues BEFORE the shot resolves (listing 00003F84)
        if self.weapon == 4 && snd != 0 && self.pack.sound(snd).is_some() {
            ctx.sounds.push(snd);
        }
        self.resolve_shot(ctx, now, pt);
        if self.weapon != 4 && snd != 0 && self.pack.sound(snd).is_some() {
            ctx.sounds.push(snd);
        }
    }

    /// `fn127` @415E.
    fn resolve_shot(&mut self, ctx: &mut Ctx, now: u64, pt: (i32, i32)) {
        let w = self.weapon.clamp(1, 5);
        let s = BLAST[(w - 1) as usize];
        // quirk: weapon 5 has no arm in the C, so it shoots an empty box
        let blast = if s == 0 { (0, 0, 0, 0) } else { centre_on((0, 0, s, s), pt) };
        let spray = w == 1 || w == 2;
        let mut splat_kind = 0i32;
        let mut first_hit: i32 = -1;
        for i in 0..self.mimes.len() {
            if !self.mimes[i].over {
                continue;
            }
            let code = self.hit_test(i, blast);
            if code == 0 {
                continue;
            }
            if code == 1 && self.corner == 2 && now < self.corner_due {
                // the secret: swap to the MimeSlayer and remember the gun
                splat_kind = 0;
                self.saved_weapon = self.weapon;
                self.weapon = 5;
                self.corner = 0;
                self.corner_due = 0;
            } else {
                splat_kind = i32::from(!self.mimes[i].dead);
            }
            if w == 4 {
                if first_hit == -1 {
                    first_hit = i as i32;
                }
            } else {
                self.hit_mime(ctx, i, blast, code, now);
                self.run_state(ctx, i, now);
            }
        }
        if w == 4 && first_hit != -1 {
            self.mime_slayer(ctx, first_hit as usize);
        }
        if spray {
            self.spray(ctx, blast, splat_kind);
        }
    }

    /// `M130_fn35` @4826 — which body part the blast box caught.
    fn hit_test(&self, i: usize, blast: Rect) -> i32 {
        let m = &self.mimes[i];
        if m.hidden || m.death_due != 0 {
            return 0;
        }
        if m.dying || m.dead {
            let whole = self.channel_rect(i, 1);
            return i32::from(intersect(blast, whole).is_some());
        }
        if self.weapon == 3 {
            return 1;
        }
        let mut best = -1i32;
        let mut code = 0i32;
        if let Some(x) = intersect(blast, self.part_rect(i, PART_HEAD, 1)) {
            code = 1;
            best = area(x);
        }
        let body = self.part_rect(i, PART_BODY, 1);
        if let Some(x) = intersect(blast, body) {
            if area(x) > best {
                code = 2;
                best = area(x);
            }
        }
        let leg1 = self.part_rect(i, PART_LEG, 1);
        let leg2 = self.part_rect(i, PART_LEG, 2);
        // GAP(fn35 leg-span rect): the C stretches the two leg boxes to the
        // body's centre line; the vertical extent is elided by the
        // decompiler, so the union of the two legs stands in.
        let cx = body.0 + (body.2 - body.0) / 2;
        let mut span = union(leg1, leg2);
        if !rect_empty(span) {
            span.0 = span.0.min(cx);
            span.2 = span.2.max(cx);
        }
        if let Some(x) = intersect(blast, span) {
            let a = area(x);
            if a > best && self.leg_line(blast, body, leg1) {
                code = 4;
                best = a;
            }
            if a > best && self.leg_line(blast, body, leg2) {
                code = 4;
                best = a;
            }
        }
        for (range, nth) in [(PART_ARM_A, 1), (PART_ARM_A, 2), (PART_ARM_B, 1), (PART_ARM_B, 2)] {
            let r = self.part_rect(i, range, nth);
            if let Some(x) = intersect(blast, r) {
                if area(x) > best {
                    code = 3;
                }
                break;
            }
        }
        code
    }

    /// `M130_fn36` @4DB0 — walk a quarter-step line from the body centre
    /// toward the leg box, four times, looking for the blast.
    fn leg_line(&self, blast: Rect, body: Rect, leg: Rect) -> bool {
        if rect_empty(leg) || rect_empty(body) {
            return false;
        }
        let cx = body.0 + (body.2 - body.0) / 2;
        let dx = (leg.2 - cx) / 4;
        let dy = (leg.3 - body.3) / 4;
        let mut l = cx.min(cx + dx);
        let mut r = cx.max(cx + dx);
        if l == r {
            l -= 1;
            r += 1;
        }
        let mut t = body.3.min(body.3 + dy);
        let mut b = body.3.max(body.3 + dy);
        if t == b {
            t -= 1;
            b += 1;
        }
        for _ in 0..4 {
            if intersect(blast, (l, t, r, b)).is_some() {
                return true;
            }
            l += dx;
            r += dx;
            t += dy;
            b += dy;
        }
        false
    }

    /// `M130_fn33` @46B0.
    fn hit_mime(&mut self, ctx: &mut Ctx, i: usize, blast: Rect, code: i32, now: u64) {
        let (dead, st, dying, hidden) = {
            let m = &self.mimes[i];
            (m.dead, m.state, m.dying, m.hidden)
        };
        if dead && st != 0x74 && st != 0x89 {
            self.set_state(ctx, i, 0x74, now);
            return;
        }
        if dying || st == 0x72 || hidden || dead {
            return;
        }
        // fn16 @042C: a random point in the blast box, clamped to the cell
        let bw = ((blast.2 - blast.0) * 10).max(1);
        let px = blast.0 + (ctx.rng.pct(bw as u32) as i32) / 10;
        let py = blast.1 + (ctx.rng.pct(bw as u32) as i32) / 10;
        let cell = self.mimes[i].cell;
        let m = &mut self.mimes[i];
        m.splat = (px.clamp(cell.0, cell.2), py.clamp(cell.1, cell.3));
        m.hit = code;
        m.kill_weapon = self.weapon;
        let r = self.rect(i);
        let bc = blast.0 + (blast.2 - blast.0) / 2;
        let mc = r.0 + (r.2 - r.0) / 2;
        self.mimes[i].side = i32::from(bc >= mc);
        self.set_state(ctx, i, 0x72, now);
    }

    /// `fn128` @43CA → `M130_fn39` @53DA — the MimeSlayer dissolve.
    fn mime_slayer(&mut self, ctx: &mut Ctx, i: usize) {
        if self.pack.sound(5000).is_some() {
            ctx.sounds.push(5000);
        }
        // GAP(fn146): the dither pattern is not drawn; the mime is hidden.
        let m = &mut self.mimes[i];
        m.hidden = true;
        m.dead = true;
        m.state = 0x8A;
        m.prev = 0;
        m.queue.clear();
    }

    /// `fn139` @4E50 → `fn66` @5AEC → `fn77` @5EFE — the pellet spray.
    /// See GAP(fn139) for the art-index reading.
    fn spray(&mut self, ctx: &mut Ctx, blast: Rect, kind_flag: i32) {
        let n = match self.weapon {
            1 => 8,
            2 => 4,
            _ => 0,
        };
        if n == 0 || rect_empty(blast) {
            return;
        }
        let w = (blast.2 - blast.0).max(1) as u32;
        let h = (blast.3 - blast.1).max(1) as u32;
        for _ in 0..n {
            let px = blast.0 + ctx.rng.pct(w) as i32;
            let py = blast.1 + ctx.rng.pct(h) as i32;
            let Some(slot) = self.decals.iter().position(|d| !d.active) else { break };
            let kind =
                if kind_flag == 0 { 1 } else { (ctx.rng.pct(20) as i32) / 10 + 2 };
            self.decals[slot] = Decal {
                active: true,
                kind,
                idx: 0,
                step: 0,
                tick: false,
                x: px,
                y: py,
            };
        }
    }

    /// `fn82` @611C — the decal ladder: kind 1 shows table slots 1 and 2,
    /// kind 2 shows 3 and 4, kind 3 shows 5 and 6; then the slot frees.
    fn decal_tick(&mut self) {
        for d in self.decals.iter_mut() {
            if !d.active || !d.tick {
                continue;
            }
            if d.step == 2 {
                *d = Decal::default();
                continue;
            }
            if d.step == 0 {
                d.idx = match d.kind {
                    1 => 1,
                    2 => 3,
                    3 => 5,
                    _ => d.idx,
                };
            } else {
                d.idx += 1;
            }
            d.step += 1;
            d.tick = false;
        }
    }
}

// ---------------------------------------------------------------------------
// the mime state machine — M130_fn29 @0A2A

impl MimeHunt {
    /// `M129_fn60` @5706, the module's SetState override: exit the old
    /// state, store, and fire the new state's ENTER **synchronously**.
    fn set_state(&mut self, ctx: &mut Ctx, i: usize, s: i32, now: u64) {
        self.set_state_depth(ctx, i, s, now, 0);
    }

    fn set_state_depth(&mut self, ctx: &mut Ctx, i: usize, s: i32, now: u64, depth: u32) {
        {
            let m = &mut self.mimes[i];
            m.prev = m.state;
            m.state = s;
        }
        if depth < 8 {
            self.handle(ctx, i, 0x8000 | s, now, depth + 1);
        }
    }

    /// `L135 fn5340` — Run: deliver the UPDATE for the current state.
    fn run_state(&mut self, ctx: &mut Ctx, i: usize, now: u64) {
        let s = self.mimes[i].state;
        self.handle(ctx, i, s, now, 0);
    }

    /// `M130_fn30` @41A0 — would run `id` still leave me inside my cell,
    /// inflated by (`ym`, `xm`)?
    fn fits(&self, i: usize, ym: i32, xm: i32, run: i32) -> bool {
        let m = &self.mimes[i];
        let (dx, dy) = self.net_move(m.bank, run, m.flip);
        let (bw, bh) = self.bank_max.get(&BANK_MIME).copied().unwrap_or((0, 0));
        let p = (m.x + dx, m.y + dy);
        let r = centre_on((0, 0, bw, bh), p);
        m.cell.0 + xm < r.0 && r.2 < m.cell.2 - xm && m.cell.1 + ym < r.1 && r.3 < m.cell.3 - ym
    }

    /// `M130_fn28` @09B2 — is the neighbour I am facing busy?
    fn neighbour_busy(&self, i: usize) -> bool {
        let m = &self.mimes[i];
        let n = if m.face == 0 { m.neigh_prev } else { m.neigh_next };
        n >= 0
            && (n as usize) < self.mimes.len()
            && (self.mimes[n as usize].dying || self.mimes[n as usize].dead)
    }

    /// `M130_fn25` @08D4 — advance the repeat counter, re-arming the run.
    fn repeat(&mut self, i: usize) -> bool {
        if !self.mimes[i].finished {
            return false;
        }
        self.mimes[i].rep += 1;
        if self.mimes[i].rep_n < self.mimes[i].rep {
            return true;
        }
        let run = self.mimes[i].rep_run;
        self.set_run(i, run);
        false
    }

    /// `M130_fn24` @08B0.
    fn arm_repeat(&mut self, i: usize, run: i32, n: i32) {
        let m = &mut self.mimes[i];
        m.rep_run = run;
        m.rep_n = n;
        m.rep = 0;
    }

    /// `M130_fn23` @087C: `RandomBelow(hi − lo) * 1000 + lo`.
    fn wait(&mut self, ctx: &mut Ctx, i: usize, now: u64, hi: i32, lo: i32) {
        let d = u64::from(ctx.rng.pct((hi - lo).max(1) as u32)) * 1000 + lo as u64;
        self.mimes[i].wait_due = now + d;
    }

    /// `M130_fn27` @0970 — the facing toggle (`+0x146` and the flip bit).
    fn face_flip(&mut self, i: usize) {
        let m = &mut self.mimes[i];
        m.face = i32::from(m.face == 0);
        m.flip = !m.flip;
    }

    #[allow(clippy::too_many_lines)]
    fn handle(&mut self, ctx: &mut Ctx, i: usize, msg: i32, now: u64, depth: u32) {
        // fn29 burns one RandomBelow(100) on every message, used or not.
        let r = ctx.rng.pct(100) as i32;
        macro_rules! go {
            ($s:expr) => {{
                self.set_state_depth(ctx, i, $s, now, depth);
                return;
            }};
        }
        macro_rules! run {
            ($id:expr) => {{
                self.set_run(i, $id);
                return;
            }};
        }
        macro_rules! fin {
            () => {
                if !self.mimes[i].finished {
                    return;
                }
            };
        }
        macro_rules! wait {
            () => {
                if now < self.mimes[i].wait_due {
                    return;
                }
            };
        }

        if msg & 0x4000 != 0 && msg & 0x8000 == 0 {
            return; // every exit is a no-op
        }
        match msg {
            // ---- enters ----------------------------------------------------
            0x8000 => run!(0x14C),
            0x8005 => {
                self.wait(ctx, i, now, 4, 2);
                run!(0x154)
            }
            0x8004 => {
                self.wait(ctx, i, now, 2, 1);
                run!(0x154)
            }
            0x8006 => {
                self.set_run(i, 0x157);
                // @1310: this arm toggles +0x146 ONLY, not the art flip
                self.mimes[i].face = i32::from(self.mimes[i].face == 0);
                return;
            }
            0x8007 => run!(0x15A),
            0x8008 => run!(0x16F),
            0x8009 => run!(0x173),
            0x800F => run!(0x22C),
            0x8010 => run!(0x23B),
            0x8011 => {
                if r < 0x19 {
                    run!(0x264)
                } else if r < 0x32 {
                    run!(0x267)
                } else if r < 0x4B {
                    run!(0x26B)
                } else {
                    run!(0x278)
                }
            }
            0x8013 => run!(0x2C3),
            0x8015 => run!(0x28B),
            0x8016 => {
                if r < 0x32 {
                    run!(0x2FD)
                } else {
                    run!(0x2A4)
                }
            }
            0x8018 => run!(0x310),
            0x801A => run!(0x2B7),
            0x801B => {
                if self.fits(i, 10, 10, 0x323) {
                    run!(0x323)
                } else {
                    run!(0x33C)
                }
            }
            0x801D => run!(0x2DA),
            0x801E => run!(0x2E6),
            0x8020 => run!(0x2F9),
            0x8022 => run!(0x4CF),
            0x8024 => {
                if r < 0x32 {
                    run!(0x4EE)
                } else {
                    run!(0x507)
                }
            }
            0x8026 => run!(0x514),
            0x8027 => {
                if r < 0x19 {
                    run!(0x560)
                } else if r < 0x32 {
                    run!(0x56C)
                } else if r < 0x4B {
                    run!(0x58B)
                } else {
                    run!(0x5A0)
                }
            }
            0x8029 => run!(0x574),
            0x802B => {
                if self.fits(i, 10, 10, 0x5B5) {
                    run!(0x5B5)
                } else {
                    go!(0x2C)
                }
            }
            0x802C => run!(0x606),
            0x802E => run!(0x22C),
            0x802F => run!(0x368),
            0x8031 => {
                self.mimes[i].wait_due = 0;
                if r < 0x32 {
                    self.wait(ctx, i, now, 3, 1);
                    run!(0x37B)
                } else {
                    run!(0x37E)
                }
            }
            0x8033 => run!(0x39A),
            0x8035 => run!(0x3B7),
            0x8036 | 0x8038 => {
                if r < 0x32 {
                    run!(0x3D5)
                } else {
                    run!(0x412)
                }
            }
            0x8037 => run!(0x3F1),
            0x8039 => run!(0x430),
            0x803A => run!(0x466),
            0x803B => run!(0x19B),
            0x803C => {
                if r < 0x32 {
                    run!(0x1A0)
                } else {
                    run!(0x1AD)
                }
            }
            0x803F => run!(0x1C6),
            0x8040 => {
                self.mimes[i].c168 = (ctx.rng.pct(30) as i32) / 10 + 4;
                self.mimes[i].c166 = 0;
                self.play(i, &[0x620]);
                return;
            }
            0x8042 => {
                let d = u64::from(ctx.rng.pct(3)) * 1000;
                self.mimes[i].wait_due = now + d;
                run!(0x6F0)
            }
            0x8043 => run!(0x6F4),
            0x8044 => {
                if r < 0x32 {
                    run!(0x6F7)
                } else {
                    run!(0x756)
                }
            }
            0x8046 => {
                let n = (ctx.rng.pct(30) as i32) / 10 + 2;
                self.arm_repeat(i, 0x71E, n);
                return;
            }
            0x8047 => run!(0x725),
            0x8049 => run!(0x72B),
            0x804A => {
                let n = (ctx.rng.pct(30) as i32) / 10;
                self.arm_repeat(i, 0x737, n);
                return;
            }
            0x804B => run!(0x749),
            0x804D => run!(0x7DB),
            0x804E => {
                let n = (ctx.rng.pct(20) as i32) / 10 + 2;
                self.arm_repeat(i, 0x7FD, n);
                return;
            }
            0x804F => {
                let n = (ctx.rng.pct(10) as i32) / 10 + 2;
                self.arm_repeat(i, 0x812, n);
                return;
            }
            0x8050 => {
                let n = (ctx.rng.pct(10) as i32) / 10 + 2;
                self.arm_repeat(i, 0x827, n);
                return;
            }
            0x8051 => {
                let n = (ctx.rng.pct(10) as i32) / 10 + 1;
                self.arm_repeat(i, 0x83C, n);
                return;
            }
            0x8052 => run!(0x851),
            0x8053 => {
                let n = (ctx.rng.pct(20) as i32) / 10 + 2;
                self.arm_repeat(i, 0x862, n);
                return;
            }
            0x8054 => run!(0x86D),
            0x8055 => run!(5),
            0x8056 => run!(0xBE),
            0x8057 => run!(0xE0),
            0x8058 => {
                if r < 0x32 {
                    self.play(i, &[0xAA, 0x88]);
                } else {
                    self.play(i, &[0x88, 0xAA]);
                }
                return;
            }
            0x8059 => {
                if r < 0x32 {
                    self.play(i, &[0xBE, 0xE0]);
                } else {
                    self.play(i, &[0xE0, 0xBE]);
                }
                return;
            }
            0x805A => run!(0xED),
            0x805C => run!(0x352),
            0x805D => run!(0xC66),
            0x805E | 0x805F => run!(0x79C),
            0x8060 | 0x8061 => run!(0x154),
            0x8063 => run!(0x8D3),
            0x8064 => {
                if r < 0x21 {
                    run!(0x9D6)
                } else if r < 0x42 {
                    run!(0x9F8)
                } else {
                    run!(0xA1B)
                }
            }
            0x8066 => {
                if r < 0x32 {
                    run!(0xB47)
                } else {
                    run!(0xB5A)
                }
            }
            0x806A => run!(0xAA0),
            0x806C | 0x8088 => run!(0x8AF),
            0x806D => run!(0xB00),
            0x806F => run!(0xB6E),
            0x8070 => {
                if r < 0x32 {
                    run!(0xA67)
                } else {
                    run!(0xA29)
                }
            }
            0x8072 => {
                // the death-art swap deadline, by weapon
                let d: u64 = match self.weapon {
                    3 => 0x3B6,
                    1 => 100,
                    2 => 0,
                    5 => 0xFA,
                    _ => {
                        // weapon 4: a DebugStr and nothing — fn128 owns it
                        return;
                    }
                };
                self.mimes[i].death_due = now + d;
                return;
            }
            0x8074 => {
                let kr = self.mimes[i].kill_run;
                if self.weapon == 3 || kr == 0xB8 || kr == 0xE0 {
                    self.mimes[i].hold_due =
                        if self.weapon == 3 { now + 0x3B6 } else { now };
                    self.mimes[i].f19a = 1;
                    return;
                }
                match kr {
                    0x49 => self.play(i, &[0x58, 0x60]),
                    0x63 => self.play(i, &[0x74, 0x7F]),
                    0x34 => self.play(i, &[0x3D]),
                    0x9B => self.play(i, &[0xA7, 0xB1]),
                    0x82 => self.play(i, &[0x8D, 0x96]),
                    _ => {}
                }
                return;
            }
            0x8077 => run!(0xBE6),
            0x8079 => run!(0xBFA),
            0x807A => run!(0xBFF),
            0x807C => run!(0xC13),
            0x807D => run!(0xC31),
            0x807E => run!(0xC86),
            0x8080 => {
                let n = (ctx.rng.pct(20) as i32) / 10 + 3;
                self.arm_repeat(i, 0xC98, n);
                return;
            }
            0x8082 => run!(0xCA0),
            0x8084 => {
                let n = (ctx.rng.pct(20) as i32) / 10 + 2;
                self.arm_repeat(i, 0xCA8, n);
                return;
            }
            0x8086 => run!(0xCB0),
            0x8087 => run!(0xC44),
            0x8089 => {
                let kr = self.mimes[i].kill_run;
                if kr == 0xB8 {
                    self.play(i, &[0xD2, 0xCD]);
                } else if kr == 0xE0 {
                    self.play(i, &[0x104, 0xFE]);
                } else {
                    let sv = self.mimes[i].saved;
                    if sv.0 != 0 && sv.1 != 0 {
                        self.mimes[i].x = sv.0;
                        self.mimes[i].y = sv.1;
                    }
                    self.play(i, &[0xB8, 0xCD]);
                    self.mimes[i].kill_run = 0xB8;
                }
                return;
            }

            // ---- updates ---------------------------------------------------
            0 => {
                fin!();
                go!(5)
            }
            2 => {
                if r < 0x32 {
                    go!(6)
                } else {
                    go!(3)
                }
            }
            3 => {
                if self.neighbour_busy(i) && r < 0xF {
                    go!(0x88)
                }
                if self.mimes[i].over {
                    if r < 0x19 || r > 0x31 {
                        return;
                    }
                    go!(0x75)
                }
                if r < 0x19 {
                    go!(4)
                } else if r < 0x32 {
                    go!(7)
                } else if r < 0x4B {
                    go!(8)
                } else {
                    go!(9)
                }
            }
            4 => {
                wait!();
                fin!();
                go!(10)
            }
            5 => {
                wait!();
                fin!();
                go!(2)
            }
            6 => {
                fin!();
                go!(3)
            }
            7 | 8 | 9 => {
                fin!();
                go!(10)
            }
            0xA => {
                if r < 0x4B {
                    go!(3)
                } else {
                    go!(0xB)
                }
            }
            0xB => {
                if self.mimes[i].over && (0x19..0x4B).contains(&r) {
                    go!(0x75)
                }
                if r < 0x21 {
                    go!(0xC)
                } else if r < 0x42 {
                    go!(0xD)
                } else {
                    go!(0x3B)
                }
            }
            0xC => {
                if r < 0x19 {
                    go!(0x40)
                } else if r < 0x32 {
                    go!(0x4D)
                } else if r < 0x4B {
                    go!(0x62)
                } else {
                    go!(0x55)
                }
            }
            0xD => {
                if r < 0x21 {
                    go!(0x2D)
                } else if r < 0x42 {
                    go!(0x21)
                } else {
                    go!(0xE)
                }
            }
            0xE => {
                self.mimes[i].f176 = 0;
                let bank = self.mimes[i].bank;
                let d = self.net_move(bank, 0x1B, self.mimes[i].flip);
                self.mimes[i].f178 = d.0;
                go!(0xF)
            }
            0xF => {
                fin!();
                go!(0x10)
            }
            0x10 => {
                fin!();
                go!(0x11)
            }
            0x11 => {
                fin!();
                go!(0x12)
            }
            0x12 => {
                if self.mimes[i].f176 != 0 && r < 0x32 && self.fits(i, 10, 10, 0x2C3) {
                    go!(0x13)
                }
                self.mimes[i].f176 = 1;
                go!(0x14)
            }
            0x13 => {
                fin!();
                go!(0x1D)
            }
            0x14 => {
                if r < 0x32 {
                    go!(0x11)
                } else {
                    go!(0x15)
                }
            }
            0x15 => {
                fin!();
                self.mimes[i].c174 = 0;
                self.mimes[i].c172 = (ctx.rng.pct(20) as i32) / 10 + 2;
                go!(0x16)
            }
            0x16 => {
                fin!();
                go!(0x17)
            }
            0x17 => {
                if r < 0x21 {
                    go!(0x1B)
                }
                if (0x21..0x42).contains(&r) && self.fits(i, 10, 10, 0x310) {
                    go!(0x18)
                }
                go!(0x19)
            }
            0x18 => {
                fin!();
                go!(0x1D)
            }
            0x19 => {
                self.mimes[i].c174 += 1;
                if self.mimes[i].c174 < self.mimes[i].c172 {
                    go!(0x17)
                } else {
                    go!(0x1A)
                }
            }
            0x1A => {
                fin!();
                go!(0x11)
            }
            0x1B => {
                fin!();
                if r < 0x32 {
                    go!(0x5D)
                } else {
                    go!(0)
                }
            }
            0x1D => {
                fin!();
                go!(0x1E)
            }
            0x1E => {
                fin!();
                go!(0x1F)
            }
            0x1F => {
                if r < 0x19 && self.fits(i, 10, 10, 0x310) {
                    go!(0x18)
                }
                if (0x19..0x32).contains(&r) {
                    go!(0x1B)
                }
                go!(0x20)
            }
            0x20 => {
                fin!();
                go!(0x16)
            }
            0x21 => {
                if self.fits(i, 10, 0x1E, 0x514) && self.fits(i, 10, 0x1E, 0x2C) {
                    go!(0x22)
                }
                go!(0)
            }
            0x22 => {
                fin!();
                go!(0x23)
            }
            0x23 => {
                self.mimes[i].c172 = (ctx.rng.pct(20) as i32) / 10 + 1;
                self.mimes[i].c174 = 0;
                go!(0x24)
            }
            0x24 => {
                fin!();
                go!(0x25)
            }
            0x25 => {
                self.mimes[i].c174 += 1;
                if self.mimes[i].c174 < self.mimes[i].c172 {
                    go!(0x24)
                }
                if self.fits(i, 2, 2, 0x514) {
                    go!(0x26)
                }
                let _ = self.fits(i, 10, 10, 0x606); // the C DebugStrs on false
                go!(0x2C)
            }
            0x26 => {
                fin!();
                go!(0x27)
            }
            0x27 => {
                fin!();
                go!(0x28)
            }
            0x28 => {
                if r < 0x32 {
                    if self.fits(i, 10, 10, 0x574) {
                        go!(0x29)
                    }
                    go!(0x2C)
                }
                go!(0x27)
            }
            0x29 => {
                fin!();
                if r < 0x32 {
                    go!(0x27)
                } else {
                    go!(0x2B)
                }
            }
            0x2B | 0x2C => {
                fin!();
                if r < 0x19 {
                    go!(0)
                } else if r < 0x32 {
                    go!(0x5E)
                } else if r < 0x4B {
                    go!(0x5D)
                } else {
                    go!(0x5C)
                }
            }
            0x2D => go!(0x2E),
            0x2E => {
                fin!();
                go!(0x2F)
            }
            0x2F => {
                fin!();
                go!(0x30)
            }
            0x30 => {
                self.mimes[i].c16e = 0;
                self.mimes[i].c170 = (ctx.rng.pct(20) as i32) / 10 + 1;
                go!(0x31)
            }
            0x31 => {
                wait!();
                fin!();
                go!(0x32)
            }
            0x32 => {
                self.mimes[i].c16e += 1;
                if self.mimes[i].c16e < self.mimes[i].c170 {
                    go!(0x30)
                } else {
                    go!(0x33)
                }
            }
            0x33 => {
                fin!();
                go!(0x34)
            }
            0x34 => {
                self.mimes[i].c16e = 0;
                self.mimes[i].c170 = 2;
                go!(0x35)
            }
            0x35 => {
                fin!();
                go!(0x36)
            }
            0x36 => {
                fin!();
                if self.mimes[i].c16e < self.mimes[i].c170 {
                    go!(0x37)
                }
                if r < 0x32 {
                    go!(0x39)
                }
                go!(0x37)
            }
            0x37 => {
                fin!();
                go!(0x38)
            }
            0x38 => {
                fin!();
                self.mimes[i].c16e += 1;
                if self.mimes[i].c16e < self.mimes[i].c170 {
                    return;
                }
                if r < 0x32 {
                    go!(0x35)
                } else {
                    go!(0x3A)
                }
            }
            0x39 => {
                fin!();
                if r < 0x32 {
                    go!(0)
                } else {
                    go!(0x5D)
                }
            }
            0x3A => {
                fin!();
                if r < 0x32 {
                    go!(0)
                } else {
                    go!(0x5E)
                }
            }
            0x3E => {
                self.mimes[i].c16a = (ctx.rng.pct(20) as i32) / 10 + 2;
                self.mimes[i].c16c = 0;
                go!(0x3B)
            }
            0x3B => {
                fin!();
                go!(0x3C)
            }
            0x3C => {
                fin!();
                go!(0x3D)
            }
            0x3D => {
                self.mimes[i].c16c += 1;
                if self.mimes[i].c16c < self.mimes[i].c16a {
                    go!(0x3B)
                } else {
                    go!(0x3F)
                }
            }
            0x3F => {
                fin!();
                go!(0xD)
            }
            0x40 => {
                fin!();
                go!(0x41)
            }
            0x41 => {
                if r < 0x32 {
                    go!(0x42)
                } else {
                    go!(0x43)
                }
            }
            0x42 => {
                wait!();
                fin!();
                go!(0x45)
            }
            0x43 => {
                fin!();
                go!(0x45)
            }
            0x45 => {
                if r < 0x28 {
                    go!(0x41)
                } else {
                    go!(0x44)
                }
            }
            0x44 => {
                fin!();
                go!(0x46)
            }
            0x46 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x47)
            }
            0x47 => {
                fin!();
                go!(0x48)
            }
            0x48 => {
                if self.mimes[i].c168 <= self.mimes[i].c166 {
                    go!(0x4C)
                }
                self.mimes[i].c166 += 1;
                if r < 0x19 {
                    go!(0x49)
                } else {
                    go!(0x44)
                }
            }
            0x49 => {
                fin!();
                go!(0x4A)
            }
            0x4A => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x4B)
            }
            0x4B => {
                fin!();
                go!(0x44)
            }
            0x4C => {
                if r < 0x32 {
                    go!(0x5E)
                } else {
                    go!(0)
                }
            }
            0x4D => {
                fin!();
                go!(0x4E)
            }
            0x4E => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x4F)
            }
            0x4F => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x50)
            }
            0x50 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x51)
            }
            0x51 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x52)
            }
            0x52 => {
                fin!();
                go!(0x53)
            }
            0x53 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x54)
            }
            0x54 => {
                fin!();
                if r < 0x14 {
                    go!(0x4D)
                } else {
                    go!(0x5B)
                }
            }
            0x55 => {
                fin!();
                if r < 0x32 {
                    go!(0x56)
                } else {
                    go!(0x57)
                }
            }
            0x56 | 0x57 => {
                fin!();
                go!(0x58)
            }
            0x58 => {
                fin!();
                go!(0x59)
            }
            0x59 => {
                fin!();
                go!(0x5A)
            }
            0x5A => {
                fin!();
                if r < 0x21 {
                    go!(0x5C)
                } else if r < 0x42 {
                    go!(0x5D)
                } else {
                    go!(0)
                }
            }
            0x5B => {
                if r < 0x19 {
                    go!(0x5C)
                } else if r < 0x32 {
                    go!(0x5D)
                } else if r < 0x4B {
                    go!(0x5E)
                } else {
                    go!(0)
                }
            }
            0x5C | 0x5D | 0x6C | 0x88 => {
                fin!();
                go!(0)
            }
            0x5E => {
                fin!();
                if r < 0x50 {
                    go!(0)
                } else {
                    go!(0x60)
                }
            }
            0x5F => {
                fin!();
                go!(0x61)
            }
            0x60 => {
                fin!();
                self.face_flip(i);
                go!(0x5F)
            }
            0x61 => {
                fin!();
                self.face_flip(i);
                go!(0x5E)
            }
            0x62 => {
                if self.fits(i, 10, 10, 0x8D3) {
                    go!(0x63)
                } else {
                    go!(0)
                }
            }
            0x63 => {
                fin!();
                go!(0x64)
            }
            0x64 => {
                fin!();
                go!(0x65)
            }
            0x65 => {
                if r < 0x46 {
                    go!(0x66)
                } else {
                    go!(0x68)
                }
            }
            0x66 => {
                fin!();
                go!(0x67)
            }
            0x67 => {
                if r < 0x46 {
                    go!(0x70)
                } else {
                    go!(0x68)
                }
            }
            0x68 => {
                if r < 0x1E {
                    go!(0x6A)
                } else {
                    go!(0x69)
                }
            }
            0x69 => {
                if r < 0x1E {
                    go!(0x6D)
                } else {
                    go!(0x6E)
                }
            }
            0x6A => {
                fin!();
                go!(0x69)
            }
            0x6B => {
                if r < 0x1E {
                    go!(0x5E)
                } else if r < 0x3C {
                    go!(0x5D)
                } else if r < 0x5A {
                    go!(0x6C)
                } else {
                    go!(0)
                }
            }
            0x6D => {
                fin!();
                go!(0x6E)
            }
            0x6E => {
                if r < 0x1E {
                    go!(0x64)
                } else {
                    go!(0x6F)
                }
            }
            0x6F => {
                fin!();
                go!(0x6B)
            }
            0x70 => {
                fin!();
                go!(0x68)
            }
            0x72 => {
                fin!();
                if !self.mimes[i].dying {
                    return;
                }
                self.mimes[i].dead = true;
                go!(0x73)
            }
            0x73 => {
                if self.weapon == 5 {
                    self.weapon = self.saved_weapon;
                }
                return;
            }
            0x74 => {
                if self.mimes[i].hold_due == 0 {
                    fin!();
                    go!(0x73)
                }
                if now <= self.mimes[i].hold_due {
                    return;
                }
                self.mimes[i].hold_due = 0;
                go!(0x89)
            }
            0x75 => go!(0x76),
            0x76 => {
                if r < 0x19 {
                    go!(0x77)
                } else if r < 0x32 {
                    go!(0x7E)
                } else if r < 0x4B {
                    go!(0x87)
                } else {
                    go!(0x5D)
                }
            }
            0x77 => {
                fin!();
                go!(0x78)
            }
            0x78 => {
                self.wait(ctx, i, now, 4, 2);
                go!(0x79)
            }
            0x79 => {
                wait!();
                fin!();
                if r < 0x19 {
                    go!(0x7A)
                } else {
                    go!(0x7B)
                }
            }
            0x7A => {
                fin!();
                go!(0x7B)
            }
            0x7B => go!(0x7C),
            0x7C => {
                fin!();
                go!(0x7D)
            }
            0x7D => {
                fin!();
                go!(0)
            }
            0x7E => {
                fin!();
                go!(0x80)
            }
            0x80 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x82)
            }
            0x82 => {
                fin!();
                go!(0x84)
            }
            0x84 => {
                if !self.repeat(i) {
                    return;
                }
                go!(0x86)
            }
            0x86 => {
                fin!();
                go!(0x7D)
            }
            0x87 => {
                fin!();
                go!(0x4F)
            }
            0x89 => {
                fin!();
                go!(0x73)
            }
            0x8A => {}
            _ => {}
        }
    }

    /// `M130_fn26` @0926 — the death-art swap: bind to bank 1000 and let
    /// `fn31` pick the run.
    fn death_swap(&mut self, ctx: &mut Ctx, i: usize, now: u64) {
        if self.mimes[i].death_due == 0 || now < self.mimes[i].death_due {
            return;
        }
        self.mimes[i].death_due = 0;
        self.mimes[i].bank = BANK_KILL;
        self.mimes[i].frame = 0;
        self.death_runs(ctx, i);
    }

    /// `M130_fn31` @4272.
    fn death_runs(&mut self, ctx: &mut Ctx, i: usize) {
        {
            let m = &mut self.mimes[i];
            m.dying = true;
            m.f19a = 0;
            m.kill_kind = 3;
            m.kill_run = -1;
            m.saved = (m.x, m.y);
        }
        let coin = ctx.rng.pct(100) as i32;
        let weapon = self.mimes[i].kill_weapon;
        let hit = self.mimes[i].hit;
        match weapon {
            1 | 2 => match hit {
                1 => {
                    if ctx.rng.pct(100) > 0x32 {
                        self.face_flip(i);
                    }
                    if coin < 0x33 {
                        self.mimes[i].kill_kind = 1;
                        self.play(i, &[99]);
                        self.mimes[i].kill_run = 99;
                    } else {
                        self.mimes[i].kill_kind = 2;
                        self.play(i, &[0x49]);
                        self.mimes[i].kill_run = 0x49;
                    }
                }
                2 => {
                    if ctx.rng.pct(100) > 0x32 {
                        self.face_flip(i);
                    }
                    self.play(i, &[0x34]);
                    self.mimes[i].kill_run = 0x34;
                }
                3 => {
                    let (side, face) = (self.mimes[i].side, self.mimes[i].face);
                    if (side == 1 && face != 0) || (side == 0 && face == 0) {
                        self.face_flip(i);
                    }
                    self.play(i, &[0x9B]);
                    self.mimes[i].kill_run = 0x9B;
                }
                4 => {
                    let (side, face) = (self.mimes[i].side, self.mimes[i].face);
                    if (side == 1 && face != 0) || (side == 0 && face == 0) {
                        self.face_flip(i);
                    }
                    self.play(i, &[0x82]);
                    self.mimes[i].kill_run = 0x82;
                }
                _ => {}
            },
            3 => {
                self.mimes[i].dying = true;
                if ctx.rng.pct(100) > 0x32 {
                    self.face_flip(i);
                }
                if coin < 0x33 {
                    self.play(i, &[0xE0, 0xFE]);
                    self.mimes[i].kill_run = 0xE0;
                } else {
                    self.play(i, &[0xB8, 0xCD]);
                    self.mimes[i].kill_run = 0xB8;
                    self.mimes[i].kill_kind = 2;
                }
            }
            // weapon 4: a DebugStr in the C — fn128/fn39 own the MimeSlayer
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Module

impl Module for MimeHunt {
    fn name(&self) -> &'static str {
        "Mime Hunt"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            // sVal 1000, ticks One/Few/More/Even More/Lots at 0/10/40/70/90
            ControlDef {
                name: "# of Mimes".into(),
                kind: ControlKind::Slider { min: 0, max: 90 },
                default: 40,
            },
            // mVal 1001 -> the MENU resource 1001 (menu ID 1000), from the
            // pack. The separator is a real item: the C maps raw > 1 to
            // raw + 1, so the popup is 1-based and item 4 ("-") is the
            // unreachable weapon 5.
            ControlDef {
                name: "Weapon".into(),
                kind: ControlKind::popup_based(1, self.pack.popup_items(1001, 5, false)),
                default: 1,
            },
            // mVal 1002 -> the MENU resource 1002 (menu ID 1003,
            // "CyberMood"), from the pack; 1-based, default Benevolent
            ControlDef {
                name: "CyberMood".into(),
                kind: ControlKind::popup_based(1, self.pack.popup_items(1002, 5, false)),
                default: 1,
            },
            // sVal 1003, bands in fn123
            ControlDef {
                name: "Music".into(),
                kind: ControlKind::Slider { min: 0, max: 95 },
                default: 30,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.mimes_raw = value.clamp(0, 90),
            1 => self.weapon_raw = value.clamp(1, 5),
            2 => self.mood_raw = value.clamp(1, 5),
            3 => self.music_raw = value.clamp(0, 95),
            _ => {}
        }
    }

    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    fn music(&self) -> Option<(u32, u32)> {
        (self.music_plays > 0 && self.pack.has_song(SONG_MIME_HUNT))
            .then_some((SONG_MIME_HUNT, self.music_plays))
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        let now = ctx.now_ms;
        if !self.started {
            self.started = true;
            // fn106 @2A40: fn135 (which latches the Music band via fn123),
            // fn137, fn124, fn108.
            self.latch_music();
            self.read_weapon_mood(ctx, now);
            self.rebuild(ctx, now);
            self.built_at = now;
            self.fire_due = now;
            self.event_at = now;
            self.caps_prev = ctx.caps_lock;
            self.need_clear = true;
        }

        // 1. ambient only: re-read the Weapon and CyberMood controls
        if !self.interactive {
            self.read_weapon_mood(ctx, now);
        }

        // 2. music
        if now >= self.music_next_ok
            && (self.music_plays < self.music_band || self.music_band == 99)
            && self.pack.has_song(SONG_MIME_HUNT)
        {
            self.music_plays += 1;
            self.music_next_ok = now + self.pack.song_length_ms(SONG_MIME_HUNT).unwrap_or(30_000);
        }

        // 3. rebuild?
        if self.wants_rebuild(now) {
            self.rebuild(ctx, now);
            self.need_clear = true;
        }

        // 4. the Caps Lock edge (GAP(fn41DC)): fn131 / fn132
        if ctx.caps_lock != self.caps_prev {
            self.caps_prev = ctx.caps_lock;
            if !self.interactive {
                self.live_at = now.saturating_sub(1);
                self.interactive = true;
                self.fire_due = now;
                self.event_at = now;
            } else {
                self.interactive = false;
                self.live_at = now.saturating_sub(1);
            }
        }
        if self.interactive && self.event_at + INTERACTIVE_IDLE_MS < now {
            self.interactive = false;
        }
        if ctx.mouse_down || ctx.mouse != (0, 0) {
            // any pointer activity re-arms the interactive idle timer
            if self.interactive && ctx.mouse_down {
                self.event_at = now;
            }
        }

        if self.need_clear {
            self.need_clear = false;
            self.cleared = true;
            self.built_at = now;
        }

        // 5. the reticle box over every mime — one fn44 read per mime
        let mut any_over = false;
        for i in 0..self.mimes.len() {
            let pt = self.point(ctx, now);
            let over = self.reticle_over(i, pt);
            self.mimes[i].over = over;
            any_over |= over;
        }

        // 6. the gun, then the reticle art — both behind the same gate
        let live = self.live_at < now || self.interactive;
        self.ret_live = live;
        if live {
            let click = ctx.mouse_down && !self.prev_mouse_down;
            self.fire_tick(ctx, now, click);
            self.reticle_anim(ctx, now, any_over);
        }
        self.prev_mouse_down = ctx.mouse_down;

        // fn40 @2436 runs in the draw pass (fn130 @486E), after the gun
        self.reticle_update(ctx, now);

        // 7. the death-art swap and the state UPDATE
        for i in 0..self.mimes.len() {
            self.death_swap(ctx, i, now);
            self.run_state(ctx, i, now);
        }

        // 8. the 90 ms STRICT advance gate
        if self.adv_at + ADVANCE_GATE_MS < now {
            self.adv_at = now;
            for d in self.decals.iter_mut() {
                if d.active {
                    d.tick = true;
                }
            }
            for i in 0..self.mimes.len() {
                self.advance(i);
            }
            self.decal_tick();
        }
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        if !self.cleared {
            return;
        }
        for m in &self.mimes {
            if m.hidden {
                continue;
            }
            let Some(f) = self.pack.frame(m.bank, m.frame.max(0) as u32) else { continue };
            let (w, h) = self.size(m.bank, m.frame);
            out.push(SpriteDraw {
                flip: m.flip,
                pal: 0,
                png: f.png.clone(),
                x: m.x - w / 2,
                y: m.y - h / 2,
            });
        }
        for d in &self.decals {
            if !d.active || d.idx <= 0 || d.idx as usize >= DECAL_IDS.len() {
                continue;
            }
            let id = DECAL_IDS[d.idx as usize];
            let Some(f) = self.pack.frame(BANK_KILL, id as u32) else { continue };
            let (w, h) = self.size(BANK_KILL, id);
            out.push(SpriteDraw {
                flip: false,
                pal: 0,
                png: f.png.clone(),
                x: d.x - w / 2,
                y: d.y - h / 2,
            });
        }
        // the reticle, on top: frame +0x2C of bank 1000 in the +0x16 box.
        // fn130 gates the draw phases on the same `g028A < now || g02A6`
        // test the gun uses, so ambient shows no crosshair for 10 s.
        if !self.ret_live {
            return;
        }
        if let Some(f) = self.pack.frame(BANK_KILL, self.ret_frame.max(0) as u32) {
            let (w, h) = self.size(BANK_KILL, self.ret_frame);
            let c = (
                (self.ret_rect.0 + self.ret_rect.2) / 2,
                (self.ret_rect.1 + self.ret_rect.3) / 2,
            );
            out.push(SpriteDraw {
                flip: false,
                pal: 0,
                png: f.png.clone(),
                x: c.0 - w / 2,
                y: c.1 - h / 2,
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }
}

// ---------------------------------------------------------------------------
// tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::path::Path;

    fn pack() -> Option<Pack> {
        let dir = Path::new("../assets/mime-hunt");
        if !dir.join("meta.json").exists() {
            return None;
        }
        Pack::load(dir).ok()
    }

    fn ctx() -> Ctx {
        Ctx {
            rng: RandomLong::new(0x1234_5678),
            rng15: Random15::new(1),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        }
    }

    /// Smoke: 3000 ticks on the module's own grid, something drew and the
    /// mimes actually moved through their machine. Caps Lock goes down
    /// halfway so the interactive half of `fn130` runs too.
    #[test]
    fn mime_hunt_smoke() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut states = std::collections::HashSet::new();
        let mut drew = 0usize;
        for t in 0..3000 {
            if t == 1500 {
                c.caps_lock = true;
            }
            c.mouse_down = t > 1500 && t % 40 == 0;
            m.tick(&mut c);
            for mm in &m.mimes {
                states.insert(mm.state);
            }
            let mut out = Vec::new();
            m.sprites(&mut out);
            drew = drew.max(out.len());
            pacer.advance(&mut c);
        }
        assert!(!m.mimes.is_empty(), "the grid built no mimes");
        assert!(drew > 1, "nothing but the reticle ever drew");
        assert!(states.len() > 5, "the machine never left its opening: {states:?}");
        assert!(m.interactive, "Caps Lock never took the module interactive");
    }

    /// RATCHET 1 — every run the port names must resolve to a live OFst
    /// block in the pack, entered AT the id (library40-api §10.1). This is
    /// the test that catches a transcription slip in the 100-state table.
    #[test]
    fn every_named_run_ships() {
        let Some(p) = pack() else { return };
        let m = build(p).expect("module");
        // series 2000 — the live mime
        let mime_runs: &[i32] = &[
            0x14C, 0x154, 0x157, 0x15A, 0x16F, 0x173, 0x22C, 0x23B, 0x264, 0x267, 0x26B, 0x278,
            0x2C3, 0x28B, 0x2FD, 0x2A4, 0x310, 0x2B7, 0x323, 0x33C, 0x2DA, 0x2E6, 0x2F9, 0x4CF,
            0x4EE, 0x507, 0x514, 0x560, 0x56C, 0x58B, 0x5A0, 0x574, 0x5B5, 0x606, 0x368, 0x37B,
            0x37E, 0x39A, 0x3B7, 0x3D5, 0x412, 0x3F1, 0x430, 0x466, 0x19B, 0x1A0, 0x1AD, 0x1C6,
            0x620, 0x6F0, 0x6F4, 0x6F7, 0x756, 0x71E, 0x725, 0x72B, 0x737, 0x749, 0x7DB, 0x7FD,
            0x812, 0x827, 0x83C, 0x851, 0x862, 0x86D, 5, 0xBE, 0xE0, 0xAA, 0x88, 0xED, 0x352,
            0xC66, 0x79C, 0x8D3, 0x9D6, 0x9F8, 0xA1B, 0xB47, 0xB5A, 0xAA0, 0x8AF, 0xB00, 0xB6E,
            0xA67, 0xA29, 0xBE6, 0xBFA, 0xBFF, 0xC13, 0xC31, 0xC86, 0xC98, 0xCA0, 0xCA8, 0xCB0,
            0xC44,
        ];
        for &id in mime_runs {
            assert!(
                m.g(BANK_MIME, id).is_some(),
                "series 2000 run {id} (0x{id:X}) is not in the pack"
            );
        }
        // series 1000 — the reticle, the decals and the death chain
        let kill_runs: &[i32] = &[
            2, 6, 7, 8, 9, 10, 11, 12, 99, 0x49, 0x34, 0x9B, 0x82, 0xE0, 0xFE, 0xB8, 0xCD, 0x58,
            0x60, 0x74, 0x7F, 0x3D, 0xA7, 0xB1, 0x8D, 0x96, 0xD2, 0x104,
        ];
        for &id in kill_runs {
            assert!(
                m.g(BANK_KILL, id).is_some(),
                "series 1000 run {id} (0x{id:X}) is not in the pack"
            );
        }
        for &id in &DECAL_IDS {
            assert!(m.g(BANK_KILL, id).is_some(), "decal compound {id} is not in the pack");
        }
    }

    /// RATCHET 2 — the machine gets around: 60 000 ticks must visit the
    /// idle spine, the act ladder and at least one death, and no mime may
    /// be parked on a single state for the whole run.
    #[test]
    fn the_machine_gets_around() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(1, 2); // Bazooka
        m.set_control(2, 3); // Psycho — the only mood that shoots often
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut seen = std::collections::HashSet::new();
        let mut kills = 0usize;
        let mut was_dead = vec![false; 4];
        for _ in 0..60_000 {
            m.tick(&mut c);
            for (k, mm) in m.mimes.iter().enumerate() {
                seen.insert(mm.state);
                if k < was_dead.len() {
                    if mm.dead && !was_dead[k] {
                        kills += 1;
                    }
                    was_dead[k] = mm.dead;
                }
            }
            pacer.advance(&mut c);
        }
        assert!(seen.contains(&0), "never reached the idle root: {seen:?}");
        assert!(seen.contains(&3), "never reached the idle chooser: {seen:?}");
        assert!(seen.len() >= 15, "only {} states in 60k ticks: {seen:?}", seen.len());
        assert!(kills > 0, "Psycho never killed anything in 60k ticks");
    }

    /// RATCHET 3 — the two gates, measured by driving the module on its own
    /// grid. `fn130`'s mime/decal advance is `g0286 + 90 < now` re-armed to
    /// `now`, which on the truncated `TickCount()*16.625` clock is 6 ticks =
    /// 99.75 ms flat; `fn45`'s reticle gate is `+0x26 < now` re-armed
    /// `now + 0x21`, which is the mixed 2/3-tick family averaging 44.9 ms.
    /// A `Millis` clock, a different gate constant or a `>=` compare all
    /// land somewhere else.
    #[test]
    fn the_two_gates_land_on_the_mac_grid() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        assert_eq!(m.clock(), TickClock::MacTick, "the module is on After Dark's clock");
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut adv_at = Vec::new();
        let mut ret_at = Vec::new();
        let mut last_adv = u64::MAX;
        let mut last_ret = u64::MAX;
        for t in 0..3000u64 {
            m.tick(&mut c);
            if m.adv_at != last_adv {
                last_adv = m.adv_at;
                adv_at.push(t);
            }
            if m.ret_due != last_ret {
                last_ret = m.ret_due;
                ret_at.push(t);
            }
            pacer.advance(&mut c);
        }
        assert!(adv_at.len() > 20, "the advance gate never fired");
        let steps: Vec<u64> = adv_at.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(steps.iter().all(|&n| n == 6), "advance steps are not 6 ticks: {steps:?}");
        let span = (adv_at[adv_at.len() - 1] - adv_at[1]) as f64;
        let ms = span * 16.625 / (adv_at.len() - 2) as f64;
        assert!((ms - 99.75).abs() < 0.01, "advance period {ms} ms");

        assert!(ret_at.len() > 20, "the reticle gate never fired");
        let rsteps: Vec<u64> = ret_at.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(rsteps.iter().any(|&n| n == 2) && rsteps.iter().any(|&n| n == 3),
                "the reticle gate is not the strict 33 ms family: {rsteps:?}");
        let rms = (ret_at[ret_at.len() - 1] - ret_at[1]) as f64 * 16.625
            / (ret_at.len() - 2) as f64;
        assert!((rms - 44.9).abs() < 1.0, "reticle period {rms} ms");
    }

    /// RATCHET 4 — the weapon fold. `fn124`'s bump of every raw value above 1 is
    /// what maps the 5-item menu (whose 4th item is a separator) onto the
    /// weapon ids the sound and blast tables are indexed by.
    #[test]
    fn the_menu_item_folds_onto_the_weapon_id() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        let mut c = ctx();
        for (raw, want) in [(1, 1), (2, 3), (3, 4), (4, 5)] {
            m.set_control(1, raw);
            m.read_weapon_mood(&mut c, 0);
            assert_eq!(m.weapon, want, "menu item {raw}");
        }
        // the Random item only ever rolls 1, 3 or 4 — never the separator
        m.set_control(1, 5);
        let mut seen = std::collections::HashSet::new();
        let mut now = 1u64;
        for _ in 0..200 {
            m.weapon_roll_due = 0;
            m.read_weapon_mood(&mut c, now);
            seen.insert(m.weapon);
            now += 40_000;
        }
        assert!(seen.iter().all(|w| [1, 3, 4].contains(w)), "Random rolled {seen:?}");
        assert!(seen.len() >= 2, "Random never varied");
        // and the per-weapon tables line up with the sound the capture heard
        assert_eq!(WEAPON_SND[2], 2000, "Bazooka is snd 2000");
        assert_eq!(REFIRE_MS[2], 2500, "Bazooka refires at 2500 ms");
        assert_eq!(BLAST[2], 64, "Bazooka blast box is 64x64");
    }

    /// The Music bands are the `sUnt 1003` ticks, latched once.
    #[test]
    fn music_bands_match_the_slider_ticks() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        for (raw, want) in [(0, 0), (4, 0), (5, 1), (24, 1), (25, 2), (49, 2), (50, 3), (74, 3),
                            (75, 4), (94, 4), (95, 99)]
        {
            m.music_raw = raw;
            m.latch_music();
            assert_eq!(m.music_band, want, "Music {raw}");
        }
    }

    /// Benevolent (the shipping default) never fires — `fn23`'s mood-1 arm
    /// returns 0 unconditionally. This is what makes an out-of-the-box
    /// ambient run four mimes miming at nothing.
    #[test]
    fn benevolent_never_fires() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(2, 1);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        for _ in 0..40_000 {
            m.tick(&mut c);
            pacer.advance(&mut c);
        }
        assert!(c.sounds.is_empty(), "Benevolent fired {:?}", c.sounds);
        assert!(m.mimes.iter().all(|x| !x.dead), "Benevolent killed a mime");
    }

    /// The grid never exceeds four mimes, every one lands inside its own
    /// cell (`fn110` caps at 4, `M130_fn32` centres in the cell), and
    /// `fn30` @41A0 keeps them there while they walk — the travel a run
    /// hand-off now preserves has to stay bounded by something, and the
    /// cell check is that something.
    #[test]
    fn the_grid_caps_at_four_and_the_cell_check_contains_the_walk() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(0, 90); // "Lots"
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        m.tick(&mut c);
        assert!(!m.mimes.is_empty());
        assert!(m.mimes.len() <= 4, "{} mimes", m.mimes.len());
        for mm in &m.mimes {
            assert!(mm.x >= mm.cell.0 && mm.x <= mm.cell.2, "x {} outside {:?}", mm.x, mm.cell);
            assert!(mm.y >= mm.cell.1 && mm.y <= mm.cell.3, "y {} outside {:?}", mm.y, mm.cell);
        }
        // Travel envelope over 60 000 ticks. `fn30` gates only the handful
        // of transitions that ask (0x12, 0x17, 0x1F, 0x21, 0x25, 0x28,
        // 0x2B, 0x62, enter 0x801B) — and every run with more than 40 px of
        // net travel is one of those — but the small ungated runs (−17, −18
        // px) accumulate, and the C has no recentring. (Superseded
        // 2026-09-29: the restoring force the old comment missed is the
        // turn — state 6's frame 343 hands off through `fn3F2E`'s flip XOR,
        // and `fn12DE` asks under that flip. With both the port measures
        // 6 px worst cell/screen overrun; before, 136 / 75 px.) The bounds
        // below are the old loose envelope, kept as a coarse guard.
        let mut worst_cell = 0i32;
        let mut worst_screen = 0i32;
        let mut offscreen_ticks = 0u64;
        let mut xs: Vec<Vec<i32>> = vec![Vec::new(); 4];
        for _ in 0..60_000u64 {
            m.tick(&mut c);
            for mm in &m.mimes {
                let (w, h) = m.size(mm.bank, mm.frame);
                if w == 0 {
                    continue;
                }
                let (l, t, r, b) = (mm.x - w / 2, mm.y - h / 2, mm.x + w / 2, mm.y + h / 2);
                worst_cell = worst_cell.max((mm.cell.0 - l).max(r - mm.cell.2));
                let ov = (-l).max(r - SCREEN_W).max((-t).max(b - SCREEN_H));
                worst_screen = worst_screen.max(ov);
                if ov > 0 {
                    offscreen_ticks += 1;
                }
            }
            for (k, mm) in m.mimes.iter().enumerate() {
                if k < 4 {
                    xs[k].push(mm.x);
                }
            }
            pacer.advance(&mut c);
        }
        for (k, v) in xs.iter().enumerate() {
            if v.is_empty() { continue; }
            println!("  mime{k}: x min {} max {} last {}", v.iter().min().unwrap(),
                     v.iter().max().unwrap(), v[v.len()-1]);
        }
        println!("  off-screen samples: {offscreen_ticks}");
        println!("worst cell overrun {worst_cell} px, worst screen overrun {worst_screen} px");
        assert!(
            worst_screen < 120,
            "a mime wandered {worst_screen} px off the field — more than its own width"
        );
        assert!(
            offscreen_ticks * 5 < 60_000 * m.mimes.len() as u64,
            "mimes spend {offscreen_ticks} of {} samples clipped by an edge",
            60_000 * m.mimes.len() as u64
        );
        for (k, v) in xs.iter().enumerate() {
            if v.is_empty() {
                continue;
            }
            let cell = m.mimes[k].cell;
            assert!(
                v.iter().any(|&x| x > cell.0 && x < cell.2),
                "mime{k} never came back inside its cell"
            );
        }
    }

    /// `#[ignore]`d census: which states 120 000 ticks actually reach.
    #[test]
    #[ignore]
    fn census_mime_hunt_states() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(1, 2);
        m.set_control(2, 5);
        m.set_control(0, 90);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut hist: std::collections::BTreeMap<i32, u32> = Default::default();
        for _ in 0..120_000 {
            m.tick(&mut c);
            for mm in &m.mimes {
                *hist.entry(mm.state).or_default() += 1;
            }
            pacer.advance(&mut c);
        }
        let total: u32 = hist.values().sum();
        println!("{} distinct states, {} samples", hist.len(), total);
        for (s, n) in &hist {
            println!("  0x{s:02X} {n:7} {:5.2}%", 100.0 * f64::from(*n) / f64::from(total));
        }
    }

    /// RATCHET 4 — **run hand-offs are continuous.** 60 000 ticks with
    /// four mimes; no live mime's drawn box may move more than 20 px in one
    /// frame. The golden capture (`emu/captures/qemu/mime-hunt.mp4`, four
    /// mimes at Lots/Bazooka/Manic, white-pixel centroid at 10 fps) has a
    /// largest frame-to-frame step of 16 px and zero steps over 25 px,
    /// while mimes walk continuously across 137–197 px spans. A port that
    /// links a run hand-off straight from the outgoing frame to the new
    /// run's first — skipping `fn1186`'s `id − 1` marker — snaps the mime
    /// back by up to 106 px roughly every 40 s. Kill/respawn is excluded:
    /// the bank-1000 swap and the grid rebuild both re-seat the sprite.
    #[test]
    fn run_hand_offs_are_continuous() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(0, 90); // Lots — four mimes, like the capture
        m.set_control(1, 2); // Bazooka
        m.set_control(2, 5); // Manic
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        // (drawn box CENTRE = pos, bank, in the death chain). The centre is
        // the nearest thing this model has to the capture's white-pixel
        // centroid; the box EDGES move on their own whenever the mime
        // throws an arm out (block 827: w goes 57 -> 106 in two frames, so
        // the left edge steps 30 px while the body does not move at all).
        let mut prev: Vec<(i32, i32, u32, bool)> = Vec::new();
        let mut worst = 0i32;
        let mut worst_in_run = 0i32;
        let mut worst_at = (0u64, 0i32, 0i32);
        let mut hand_offs = 0usize;
        let mut spans: Vec<i32> = Vec::new();
        let mut anchor: Vec<i32> = Vec::new();
        for _ in 0..60_000u64 {
            let before: Vec<i32> = m.mimes.iter().map(|x| x.first).collect();
            m.tick(&mut c);
            let cur: Vec<(i32, i32, u32, bool)> = m
                .mimes
                .iter()
                .map(|x| (x.x, x.y, x.bank, x.dying || x.dead))
                .collect();
            let rebuilt = m.built_at + 200 > c.now_ms;
            if prev.len() == cur.len() && !rebuilt {
                for (k, (a, b)) in prev.iter().zip(cur.iter()).enumerate() {
                    if a.2 != b.2 || a.3 || b.3 {
                        continue; // kill / respawn re-seats the sprite
                    }
                    let d = (a.0 - b.0).abs().max((a.1 - b.1).abs());
                    if k < before.len() && before[k] != m.mimes[k].first {
                        hand_offs += 1;
                        if d > worst {
                            worst = d;
                            worst_at = (c.now_ms, m.mimes[k].first, m.mimes[k].frame);
                        }
                    } else {
                        // In-run steps are the art's own box-centre motion
                        // (the 0x323 lunge moves the centre 20.5 px 805->806
                        // with dx/dy 0); bounded loosely, not by the capture.
                        worst_in_run = worst_in_run.max(d);
                    }
                    if k == 0 {
                        if anchor.is_empty() {
                            anchor.push(b.0);
                        }
                        spans.push(b.0);
                    }
                }
            }
            prev = cur;
            pacer.advance(&mut c);
        }
        let span = spans.iter().copied().max().unwrap_or(0)
            - spans.iter().copied().min().unwrap_or(0);
        assert!(hand_offs > 200, "only {hand_offs} run hand-offs in 60k ticks");
        assert!(
            worst <= 16,
            "a live mime jumped {worst} px at a hand-off at t={} run 0x{:X} frame {} \
             (the capture's largest step is 16 px)",
            worst_at.0,
            worst_at.1,
            worst_at.2
        );
        assert!(worst_in_run <= 26, "in-run step {worst_in_run} px exceeds the art's own motion");
        assert!(span >= 100, "mime 0 never travelled: {span} px of ground covered");
        println!(
            "hand-offs {hand_offs}, worst hand-off step {worst} px (t={} run 0x{:X} frame {}), \
             worst in-run {worst_in_run} px, \
             mime 0 covered {span} px",
            worst_at.0, worst_at.1, worst_at.2
        );
    }

    /// RATCHET 5 — **the turn really turns** (L135 `fn3F2E`'s flip XOR).
    /// State 6 plays run 342 → frame 343, the standing pose with its body
    /// parts authored MIRRORED (part flags bit 0), and toggles only the
    /// module's `+0x146`. The art flip comes from the library: the next
    /// hand-off's first shared part (art 27) is mirrored in 343 and native
    /// in the marker, so `fn3F2E` XORs the sprite's `+0x3C` and registers
    /// the part through the flip. The old port applied no flip — the mime
    /// snapped back to its native facing after one frame and slid 19 px.
    ///
    /// Pinned to `mime-hunt.mp4` (masked-SSD template match of c_346,
    /// native and mirrored, ≤ 0.02, 10 fps): of 213 standing-pose sightings
    /// 89 are mirrored (42 %), with unbroken mirrored stretches of 2.5 s.
    /// The old port drew 1 mirrored of 901 over 8 seeds.
    #[test]
    fn the_turn_hands_off_mirrored() {
        let Some(p) = pack() else { return };
        let mut m = build(p.clone()).expect("module");
        let mut c = ctx();
        m.tick(&mut c);
        assert!(!m.mimes.is_empty());
        // 1. the flip-in-place at 343 -> run 346 (and back again)
        for (from, flip0) in [(0x157, false), (0x157, true)] {
            {
                let mm = &mut m.mimes[0];
                mm.bank = BANK_MIME;
                mm.frame = from;
                mm.first = 0x156;
                mm.flip = flip0;
                mm.x = 300;
                mm.y = 200;
            }
            let g = m.g(BANK_MIME, from).unwrap().clone();
            let part = *g.parts.iter().find(|q| q[0] == 27).unwrap();
            assert_eq!(part[2] & 1, 1, "frame 343's art-27 part is authored mirrored");
            let before = m.part_screen(0, &part);
            m.set_run(0, 0x15A);
            assert_eq!(m.mimes[0].flip, !flip0, "343 -> 346 must toggle the sprite flip");
            let g2 = m.g(BANK_MIME, 0x15A).unwrap().clone();
            let part2 = *g2.parts.iter().find(|q| q[0] == 27).unwrap();
            let after = m.part_screen(0, &part2);
            assert_eq!(before, after, "the shared part moved across the hand-off");
        }
        // 2. the capture's orientation census, over 8 seeds
        let stand: std::collections::HashSet<String> = [
            337, 339, 340, 342, 345, 346, 347, 348, 349, 350, 364, 366, 368, 370, 371, 372, 406,
            407, 408, 410, 411, 491, 492, 532, 547,
        ]
        .iter()
        .filter_map(|&id: &i32| p.frame(BANK_MIME, id as u32).map(|f| f.png.clone()))
        .collect();
        let (mut nat, mut mir) = (0u32, 0u32);
        for seed in 0..8u32 {
            let mut m = build(p.clone()).expect("module");
            m.set_control(0, 90);
            m.set_control(1, 2);
            m.set_control(2, 5);
            let mut c = ctx();
            c.rng = RandomLong::new((0x1234_5678u32 ^ seed.wrapping_mul(0x9E37_79B9)) as _);
            let mut pacer = Pacer::new(&m);
            let mut next = 20_000u64;
            while pacer.now_ms() < 80_000 {
                pacer.advance(&mut c);
                m.tick(&mut c);
                if c.now_ms < next {
                    continue;
                }
                next += 100;
                let mut out = Vec::new();
                m.sprites(&mut out);
                for s in out.iter().filter(|s| stand.contains(&s.png)) {
                    if s.flip {
                        mir += 1;
                    } else {
                        nat += 1;
                    }
                }
            }
        }
        let frac = f64::from(mir) / f64::from((nat + mir).max(1));
        assert!(nat + mir > 200, "only {} standing samples", nat + mir);
        // capture 89/213 = 0.42; a band wide enough for 8 seeds of 60 s
        assert!(
            (0.25..=0.60).contains(&frac),
            "mirrored standing {mir} of {} = {frac:.2}; the capture shows 0.42",
            nat + mir
        );
    }

    /// `#[ignore]`d: the largest per-frame position jump any live mime makes.
    #[test]
    #[ignore]
    fn measure_mime_step() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(0, 90);
        m.set_control(1, 2);
        m.set_control(2, 5);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut prev: Vec<(i32, i32, u32, bool)> = Vec::new();
        let mut worst = 0i32;
        let mut hist: Vec<i32> = Vec::new();
        let mut worst_at = (0u64, 0u32, 0i32);
        for _ in 0..60_000u64 {
            m.tick(&mut c);
            let cur: Vec<(i32, i32, u32, bool)> =
                m.mimes.iter().map(|x| (x.x, x.y, x.bank, x.dying || x.dead)).collect();
            if m.built_at + 100 > c.now_ms {
                prev = cur;
                pacer.advance(&mut c);
                continue;
            }
            if prev.len() == cur.len() {
                for (k, (a, b)) in prev.iter().zip(cur.iter()).enumerate() {
                    if a.2 != b.2 || a.3 || b.3 {
                        continue; // kill / respawn re-anchors the sprite
                    }
                    let d = (a.0 - b.0).abs().max((a.1 - b.1).abs());
                    if d > 0 {
                        hist.push(d);
                    }
                    if d > 40 && d >= worst.min(40) {
                        println!("  jump {d} px at t={} mime{k} run 0x{:X} frame {} state 0x{:X}",
                                 c.now_ms, m.mimes[k].first, m.mimes[k].frame, m.mimes[k].state);
                    }
                    if d > worst {
                        worst = d;
                        worst_at = (c.now_ms, m.mimes[k].first as u32, m.mimes[k].frame);
                    }
                }
            }
            prev = cur;
            pacer.advance(&mut c);
        }
        println!("worst per-tick mime step: {worst} px at t={} run 0x{:X} frame {}",
                 worst_at.0, worst_at.1, worst_at.2);
        println!("steps  >12px: {}  >30px: {}  >60px: {}  of {}",
                 hist.iter().filter(|&&d| d > 12).count(),
                 hist.iter().filter(|&&d| d > 30).count(),
                 hist.iter().filter(|&&d| d > 60).count(),
                 hist.len());
    }

    /// `#[ignore]`d per-hand-off trace (60 000 ticks, Lots/Bazooka/Manic —
    /// the capture's settings): for every `fn028A` hand-off, the first part
    /// `fn3F2E` finds shared between the outgoing frame and the marker, the
    /// XOR of the two parts' own flip bits, the delta applied and the
    /// sprite flip before/after. Prints a census and the first 40 lines.
    #[test]
    #[ignore]
    fn trace_mime_hand_offs() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(0, 90);
        m.set_control(1, 2);
        m.set_control(2, 5);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut lines = 0;
        let (mut n, mut shared, mut x1, mut x2, mut flips) = (0, 0, 0, 0, 0);
        let mut pairs: std::collections::BTreeMap<(i32, i32), u32> = Default::default();
        for _ in 0..60_000u64 {
            m.handoffs.clear();
            m.tick(&mut c);
            let log = std::mem::take(&mut m.handoffs);
            for &(k, from, marker, id, f0, f1, dx, dy) in &log {
                let bank = m.mimes[k].bank;
                n += 1;
                let sp = m.first_shared_part(bank, from, marker).map(|(a, b)| (a[0], a[2] & 3, b[2] & 3));
                if let Some((_, a, b)) = sp {
                    shared += 1;
                    if (a ^ b) & 1 != 0 {
                        x1 += 1;
                        *pairs.entry((from, id)).or_default() += 1;
                    }
                    if (a ^ b) & 2 != 0 {
                        x2 += 1;
                    }
                }
                if f0 != f1 {
                    flips += 1;
                }
                if lines < 40 {
                    lines += 1;
                    println!(
                        "t={:7} mime{k} st 0x{:02X} bank {bank} {from} -> marker {marker} -> run {id} \
                         part {:?} flip {f0}->{f1} delta ({dx},{dy}) pos ({},{})",
                        c.now_ms, m.mimes[k].state, sp, m.mimes[k].x, m.mimes[k].y
                    );
                }
            }
            pacer.advance(&mut c);
        }
        println!(
            "hand-offs {n}, with a shared part {shared}, part h-flip differs {x1}, \
             v-flip differs {x2}, sprite flips applied {flips}"
        );
        for ((a, b), k) in pairs.iter().take(60) {
            println!("  h-xor hand-off {a} -> {b}: {k}");
        }
    }

    /// `#[ignore]`d capture comparison: how often a drawn mime is in the
    /// plain standing pose (the 25 records pixel-identical to c_346)
    /// mirrored vs native, sampled every 100 ms for 60 s over 8 seeds —
    /// the same census `mime-hunt.mp4` gives by template match
    /// (c_346 native/mirrored, masked SSD ≤ 0.02 at 10 fps), plus the
    /// longest unbroken mirrored-standing stretch.
    #[test]
    #[ignore]
    fn census_standing_orientation() {
        const STAND: [i32; 25] = [
            337, 339, 340, 342, 345, 346, 347, 348, 349, 350, 364, 366, 368, 370, 371, 372, 406,
            407, 408, 410, 411, 491, 492, 532, 547,
        ];
        let Some(p) = pack() else { return };
        let stand: std::collections::HashSet<String> = STAND
            .iter()
            .filter_map(|&id| p.frame(BANK_MIME, id as u32).map(|f| f.png.clone()))
            .collect();
        let (mut nat, mut mir) = (0u32, 0u32);
        let mut longest = 0u32;
        for seed in 0..8u32 {
            let mut m = build(p.clone()).expect("module");
            m.set_control(0, 90);
            m.set_control(1, 2);
            m.set_control(2, 5);
            let mut c = ctx();
            c.rng = RandomLong::new((0x1234_5678u32 ^ seed.wrapping_mul(0x9E37_79B9)) as _);
            let mut pacer = Pacer::new(&m);
            let mut next = 20_000u64; // skip the build
            let mut run: std::collections::HashMap<(i32, i32), u32> = Default::default();
            while pacer.now_ms() < 80_000 {
                pacer.advance(&mut c);
                m.tick(&mut c);
                if c.now_ms < next {
                    continue;
                }
                next += 100;
                let mut out = Vec::new();
                m.sprites(&mut out);
                let mut seen: std::collections::HashMap<(i32, i32), u32> = Default::default();
                for s in &out {
                    if !stand.contains(&s.png) {
                        continue;
                    }
                    let key = (s.x / 40, s.y / 40);
                    if s.flip {
                        mir += 1;
                        let k = run.get(&key).copied().unwrap_or(0) + 1;
                        longest = longest.max(k);
                        seen.insert(key, k);
                    } else {
                        nat += 1;
                    }
                }
                run = seen;
            }
        }
        println!(
            "standing samples: native {nat}, mirrored {mir} ({:.0}% mirrored); \
             longest mirrored stretch {longest} samples",
            100.0 * f64::from(mir) / f64::from((nat + mir).max(1))
        );
    }

    /// `#[ignore]`d: where the ambient crosshair goes.
    #[test]
    #[ignore]
    fn trace_ambient_crosshair() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(2, 5);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        for t in 0..12_000u64 {
            m.tick(&mut c);
            if t % 400 == 0 {
                println!("t={:7} ret ({:4},{:4}) v=({:6.2},{:6.2}) a=({:5.2},{:5.2}) sp={:.2}",
                    c.now_ms, m.ret_pt.0, m.ret_pt.1, m.wander.vx, m.wander.vy,
                    m.wander.ax, m.wander.ay, m.wander.speed);
            }
            pacer.advance(&mut c);
        }
    }

    /// Where the ambient crosshair spends its time, measured the way the
    /// rig captures were: the reticle point per tick, "on an edge" when it
    /// is within 24 px of the screen edge (the art is 44 px, so that is
    /// where the drawn reticle touches the edge).
    struct AmbientStats {
        edge: f64,
        corner: f64,
        longest_pin: u32,
        still: f64,
        cells: usize,
    }

    fn ambient_stats(seed: u64, mood: i32, ticks: u64) -> AmbientStats {
        let p = pack().expect("pack");
        let mut m = build(p).expect("module");
        m.set_control(0, 85);
        m.set_control(1, 2);
        m.set_control(2, mood);
        let mut c = ctx();
        c.rng = RandomLong::new(seed);
        let mut pacer = Pacer::new(&m);
        let (mut n, mut edge, mut corner, mut still) = (0u32, 0u32, 0u32, 0u32);
        let (mut pin, mut longest_pin) = (0u32, 0u32);
        let mut prev = (-1, -1);
        let mut cells = std::collections::HashSet::new();
        for t in 0..ticks {
            m.tick(&mut c);
            pacer.advance(&mut c);
            if t < 700 {
                continue; // the 10 s grace: no crosshair yet
            }
            let (x, y) = m.ret_pt;
            let ex = x <= 24 || x >= SCREEN_W - 24;
            let ey = y <= 24 || y >= SCREEN_H - 24;
            n += 1;
            edge += u32::from(ex || ey);
            corner += u32::from(ex && ey);
            let same = (x, y) == prev;
            still += u32::from(same);
            pin = if same && (ex || ey) { pin + 1 } else { 0 };
            longest_pin = longest_pin.max(pin);
            cells.insert((x.clamp(0, SCREEN_W - 1) / 80, y.clamp(0, SCREEN_H - 1) / 80));
            prev = (x, y);
        }
        let n = f64::from(n.max(1));
        AmbientStats {
            edge: f64::from(edge) / n,
            corner: f64::from(corner) / n,
            longest_pin,
            still: f64::from(still) / n,
            cells: cells.len(),
        }
    }

    /// RATCHET — the ambient crosshair does not park on the walls. Pinned
    /// to the 2026-09-29 rig captures (tt `emu/captures/qemu/`):
    /// `mime-hunt-ambient` (Manic) had the reticle on an edge in 16.3 % of
    /// 2917 visible frames and in a corner in 0.7 %, never still for more
    /// than 0.2 s, longest edge run 0.8 s; `mime-hunt-ambient-benevolent`
    /// 21.1 % / 1.3 %, longest edge run 0.7 s. With `fn20` ported as a clamp
    /// the port sat on an edge 87–99 % of the time and in the bottom-right
    /// corner for up to 5739 ticks at a stretch.
    #[test]
    fn ambient_crosshair_does_not_park_on_the_walls() {
        if pack().is_none() {
            return;
        }
        for (mood, cap_edge) in [(5, 0.163), (1, 0.211)] {
            let s = ambient_stats(0x1234_5678, mood, 36_000);
            assert!(
                (cap_edge - 0.10..=cap_edge + 0.10).contains(&s.edge),
                "mood {mood}: on an edge {:.1}% of the time, capture {:.1}%",
                100.0 * s.edge,
                100.0 * cap_edge
            );
            assert!(s.corner < 0.03, "mood {mood}: in a corner {:.1}%", 100.0 * s.corner);
            assert!(
                s.longest_pin < 120,
                "mood {mood}: pinned on a wall for {} ticks",
                s.longest_pin
            );
            assert!(s.still < 0.05, "mood {mood}: still {:.1}% of ticks", 100.0 * s.still);
            assert_eq!(s.cells, 48, "mood {mood}: the reticle never covered the field");
        }
    }

    /// `#[ignore]`d: the ambient crosshair's edge/corner occupancy over long
    /// runs, several seeds, the two moods the rig captured.
    #[test]
    #[ignore]
    fn trace_ambient_occupancy() {
        for mood in [5, 1] {
            for seed in [0x1234_5678u64, 1, 4, 0x55AA, 0xDEAD_BEEF] {
                let s = ambient_stats(seed, mood, 36_000);
                println!(
                    "mood {mood} seed {seed:#x}: edge {:5.1}% corner {:5.1}% longest pin {} ticks \
                     still {:5.1}% cells {}/48",
                    100.0 * s.edge, 100.0 * s.corner, s.longest_pin, 100.0 * s.still, s.cells
                );
            }
        }
    }

    /// `#[ignore]`d trace: one line per mime transition.
    #[test]
    #[ignore]
    fn trace_mime_hunt() {
        let Some(p) = pack() else { return };
        let mut m = build(p).expect("module");
        m.set_control(1, 2);
        m.set_control(2, 3);
        let mut c = ctx();
        let mut pacer = Pacer::new(&m);
        let mut last = vec![-1i32; 8];
        for _ in 0..40_000 {
            m.tick(&mut c);
            for (k, mm) in m.mimes.iter().enumerate() {
                if k < last.len() && last[k] != mm.state {
                    println!(
                        "t={:7} mime{} state 0x{:X} bank {} run 0x{:X} frame {} pos ({},{}) \
                         dying={} dead={} over={}",
                        c.now_ms, k, mm.state, mm.bank, mm.first, mm.frame, mm.x, mm.y,
                        mm.dying, mm.dead, mm.over
                    );
                    last[k] = mm.state;
                }
            }
            pacer.advance(&mut c);
        }
    }
}
