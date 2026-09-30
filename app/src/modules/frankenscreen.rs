//! FrankenScreen — ported function by function from `M129_M129.c`
//! (`docs/decompiled/frankenscreen/`, tt private repo). 149 functions; the
//! whole module is here except the eight GAPs listed at the bottom.
//!
//! Everything in this header is checkable against THIS module's C. Where a
//! claim comes off the raw listing instead it says so
//! (`ripped/frankenscreen/FrankenScreen_CODE_129_DynaFrank.txt`).
//!
//! ## Objects (ctor `fn06` @0A58 → `fn08` @0E82)
//!
//! * **Three `RLESequence` art banks** (`fn31` @2B56): resource ids
//!   `1000 + i*1000` for i = 0..2 → series **1000 / 2000 / 3000**, kept in
//!   `g03DC[3]`. Every sprite binds to exactly one of them (`fn79` @3C7A
//!   arg 2 is the bank index), and so does every backdrop stamp. This is
//!   what settles the old header's "§13 open question: which series draws
//!   which id" — it was never ambiguous, the C names the bank.
//! * **One sound bank** (`fn29` @2A3A): `cmid` 30 loaded first, then
//!   `fn62` @735C with (30001, 30002, 30004, 30005, 30006, 30007, −1) —
//!   read off the listing at `00002ABA`, where the decompiler drops the
//!   varargs.
//! * **One "pair" object** `g0004` (26 bytes, `fn145` @head / reset
//!   `fn148` @004E): `+4` current direction, `+8` target direction, `+0xC`
//!   the msg-5 member, `+0x10` the msg-6 member, `+0x14` the re-roll
//!   deadline, `+0x18` "a member is mid-run". It makes the two eyes look
//!   the same way at the same time (`fn153` @0182, `fn151` @0108,
//!   `fn152` @0158).
//! * **A 3×3 placement grid** `+0x10` (`fn141` @022C): nine 18-byte cells
//!   covering the screen, `W/3 × H/3` each.
//! * **Up to 64 sprites** `g01C8[64]`, four classes:
//!
//! | class | ctor | Init `+0x168` | DoFrame `+0x13C` | Region `+0x138` | msgs |
//! |---|---|---|---|---|---|
//! | A | `fn91` @4036 | `fn93` @4084 | `fn98` @4578 | `fn103` @4F16 | 1, 5, 6, 7 |
//! | B | `fn104` @5132 | `fn106` @5174 | `fn110` @54C4 | `fn114` @5980 | 2 |
//! | C | `fn115` @5B30 | `fn117` @5B72 | `fn122` @6048 | `fn120` @5E3C | 3 |
//! | D | `fn126` @6448 | `fn128` @6490 | `fn133` @6804 | `fn131` @6756 | 9..16 |
//!
//! The vtables (`g0684` A, `g07F0` B, `g095C` C, `g0AC8` D, base `g051C`)
//! were decoded from `emu/ghidra/frankenscreen/blocks/A5_globals.bin` at
//! M129 data + the slot offset. Only the `+0x134..+0x168` tail is
//! module-specific; everything below that is the stock `L132_Resource`
//! CompoundSprite.
//!
//! ## Library semantics used (all `L132_Resource`, all verified in this pack)
//!
//! * `+0x7C` = `fn0204` → `fn028A` = **SetRun(id)**, linked. All four class
//!   vtables and the base bind it (`+0x7C` = L132 `fn0204`, `+0x108` =
//!   `fn028A`, `+0xCC` = `fn0D3C`, `+0xD0` = `fn0DF4`, `+0xF0` = `fn1186`,
//!   decoded from `A5_globals.bin` 2026-09-29). The hand-off goes current
//!   frame → marker (`fn1186`: `id − 1` if that record exists, else `id`)
//!   through L135 **`fn3F2E @3F2E`** — the first part the two frames share
//!   keeps its screen position, and the sprite flip is XORed with the two
//!   parts' flip bits — then marker → `id` by `fn3DDC`. *(Superseded: "the
//!   pre-step composes to a single `pos += centre(newFirst) −
//!   centre(currentFrame)`". That was the wrong model; in this pack it
//!   happens to give the same numbers on every hand-off the machines can
//!   make — see "Hand-off audit" below.)*
//! * `+0x80` = `fn0240` = SetRun(id₀) then **queue** id₁… (16-slot ring at
//!   `+0x60`, `fn153A`), vararg list terminated by −1.
//! * `+0x84` = `fn0316` = the per-frame advance: clear `+0x46`; if a queued
//!   run is pending (`+0x4E`) start it; unless `+0x48` (just-set-a-run) skip
//!   one; else `pos += link(f, f+1); f++` while `first <= f < last`; when
//!   the run has run out either pop the queue into `+0x4E` or set
//!   `+0x46 = 1` — **"my sequence has finished"**, the flag every class
//!   machine gates on.
//! * `+0x70` = `fn01E2` = SetFrame(id) — a bare frame, no run. Class A's
//!   `fn93` and `fn81` @3E46 use it for the opening pose, which is why the
//!   sprite sits still until its first act.
//! * `+0x88` = `fn04C8` = SetPos. **`pos` is the frame CENTRE**: `fn97`
//!   @4434 builds the sprite rect as `(x − w/2, y − h/2, +w, +h)`, and
//!   `fn79` @3C7A (listing `00003D22`) stores x at `+0x9E` and y at
//!   `+0xA0`. The old header had that pair the other way round.
//! * In-run `link(from,to)` = L135 `fn3DDC @3DDC`: the difference of the
//!   frame rects' mid-points, `(flip + l + r) >> 1`, x negated while
//!   flipped. Unflipped it equals `centre(to) − centre(from)` (port-plan
//!   §2); flipped it can differ by 1 px on odd widths.
//!
//! ## Hand-off audit (2026-09-29, the `fn028A` bug class)
//!
//! The shock-clocks / message-mayhem fixes found ports that treated the
//! linked hand-off as a centre link. This port did too, but here it is
//! **behaviour-neutral**: an exhaustive census
//! (`frankenscreen_handoff_census`, `#[ignore]`) of every frame a machine's
//! runs can show × every run the same machine can start — 35 416 pairs,
//! both flips — finds that every pair shares a part (except the 8 out of
//! class-D msg 12's frame 348, whose part table is empty: no move, and
//! the centres agree there too), every shared part is registered exactly
//! where the frame centres are, and no shared part's flip bit differs
//! (0 flip toggles). The only disagreements are 1 px
//! `fn3DDC` flip roundings in class-D msg 11's runs, which are never
//! flipped. A 120 s and a 10 min `frame --trace` are byte-identical
//! before and after the transcription.
//!
//! * **The nose "creep" (TODO: "class B link drift ~27 px") is not a
//!   drift.** The drip runs (bank 1000 195..200, bank 3000 239..241 /
//!   246..248) grow the frame downward from a fixed top, so `pos` — the
//!   frame CENTRE — dips 27 / 29 px and the hand-off back to the rest pose
//!   lifts it again; the drawn art never moves. Capture
//!   (`frankenscreen.mp4`, masked template match at 5 fps): the bank-1000
//!   nose sits at top-left (455, 327) in all 135 samples of its 0–27 s
//!   life, the bank-3000 nose at (426, 30) in all 135 of its (drip frames
//!   included). Ratchet `the_nose_holds_its_top_across_a_life`.
//! * Run ids are OFst `frameNum`s as written — no ±1 (library40-api §10.1).
//!   All 118 ids this module names resolve to a live block in the pack.
//!
//! ## Frame driver (`fn15` @131E, DoDrawFrame)
//!
//! Every Mac tick: `fn14` @1290 re-latches Coherency/Blemishes/Lifespan
//! (`GetControlValue(0/1/2)`, listing `00001296`/`000012A8`/`000012B6`),
//! then Caps Lock or `g03D0 < now − built_at` or `1 800 000 < now − start`
//! tears the creature down and rebuilds it. `+0x18` paints the backdrop
//! (`fn20` @1710). Then, **gated on `g01BA < now` — a STRICT compare,
//! re-armed `now + 100`** — `fn153` runs the eye pair and every sprite's
//! `+0x13C` runs once. On After Dark's truncated `TickCount()*16.625`
//! clock a strict `now + 100` lands on **7 Mac ticks = 116.4 ms flat**,
//! which is the period `frankenscreen.mp4` measures. Music replays while
//! the channel is idle and `g03C8 > 0` (or 99).
//!
//! ## Class A — the eyes / mouths (`fn93` @4084, `fn101` @46CC, `fn102` @4E5C)
//!
//! `fn93` rolls a **kind** `+0xBC = RandomBelow(600)/100` ∈ 0..5 (the
//! `g03D4 != 8` sequential path is dead — `fn14` hard-sets `g03D4 = 8`)
//! and maps it to (bank, opening frame):
//!
//! | kind | bank | open | machine |
//! |---|---|---|---|
//! | 0 | 1000 | 3 | `fn101` |
//! | 1 | 1000 | 121 | `fn101` |
//! | 2 | 2000 | 19 | `fn102` (and `+0xBE = 0`) |
//! | 3 | 2000 | 28 | `fn101` |
//! | 4 | 3000 | 9 | `fn101` |
//! | 5 | 3000 | 108 | `fn101` |
//!
//! `+0xBE` is the **direction**, 0..4, opening at 4. `fn99` @45C8 rolls a
//! new one (`RandomBelow(50)/10`, re-rolled while it equals the current),
//! `fn100` @462C is the act gate (`+0x46` set, then a one-shot
//! `+0xAA = now + RandomBelow(3000)` ms deadline), and `fn101` looks the
//! pair (previous direction, new direction) up per kind in `A_TURNS` — a
//! 5 × 5 × 5 table of run ids read straight out of @46CC..@4E56. Kind 2
//! has only two poses (`fn102`: runs 19 and 7).
//!
//! msgs 5 and 6 register with the pair object (`fn150` @00B0) and then use
//! `fn152`/`g0004+8` instead of their own timer, so they turn together;
//! msg 6 also swaps directions 0↔1 (@46FC) and flips its art (`+0x3C ^= 1`
//! at @4258). msg 1's region is the whole screen; 5/6/7 get a point at
//! 30 % / 70 % / 50 % of the width and 20 % of the height, inflated by
//! ±10/40/80/200/400 px by Coherency (`fn103` @4F16).
//!
//! ## Class B — the nose (`fn106` @5174, `fn111`/`fn112`/`fn113`)
//!
//! kind = `RandomBelow(300)/100`; (bank, open, `+0xAE`) =
//! (1000, 193, 239) / (2000, 335, 341) / (3000, 234, 234). Region: a point
//! at 50 % W / 45 % H, same Coherency inflation (`fn114` @5980). Act
//! deadline `now + RandomBelow(10000 | 7000 | 7000)` (`fn109` @5468).
//! Each machine opens with a 20 % `RandomBelow(100) < 20` → **snd 30007**
//! (Sniff) that is independent of what it then does.
//!
//! ## Class C — the mouth (`fn117` @5B72, `fn123`/`fn124`/`fn125`)
//!
//! kind = `RandomBelow(300)/100`; (bank, open, `+0xAE`) =
//! (1000, 254, 292) / (2000, 277, 297) / (3000, 189, 218). Region: 50 % W /
//! 80 % H. Act deadline `now + RandomBelow(5000)` for every kind
//! (`fn121` @5FEC).
//!
//! ## Class D — the extras (`fn128` @6490, `fn133` @6804)
//!
//! Eight messages, each a fixed run in a fixed bank, and a `+0xBC` kind
//! that is 1 (a **static** part: `fn133`'s else branch just re-SetRuns
//! `+0xBE` whenever the run ends) for six of them and 0 (a real machine,
//! `fn134` @687A / `fn135` @694A) for msgs 11 and 12:
//!
//! | msg | bank | run | `+0xA6` | note |
//! |---|---|---|---|---|
//! | 9 | 1000 | 321 | 2 | static |
//! | 10 | 1000 | 251 | 0 | static, the only class-D grid part |
//! | 11 | 2000 | 350 (`+0xAE`) | 2 | `fn134`: 51 % → 361, else 352, + snd 30004 |
//! | 12 | 2000 | 348 (`+0xAE`) | 0 | `fn135`: 90 % → [347, 348] |
//! | 13 | 1000 | 315 | 2 | static |
//! | 14 | 1000 | 318 | 2 | static |
//! | 15 | 3000 | 272 | 2 | static |
//! | 16 | 3000 | 274 | 2 | static |
//!
//! ## Script (`fn33` @2CC0 → `fn34` @2D60, `fn36` @2F00, `fn37` @2F92, `fn38` @31A6)
//!
//! 64 slots at `g02C8`, filled per build:
//!
//! 1. `fn34`: clear to −1, copy the static list `g0008` = **[1, 1, 2, 3]**
//!    (dumped from A5 globals; the `g001C`/`g003C`/`g0058` variants belong
//!    to the dead `g03D4 ∈ 1..6` path). The keyboard-egg latch appends a 9.
//! 2. `fn36`: **Blemishes/8** extra events, each `11 + RandomBelow(60)/10`
//!    → 11..16, written into the first free slot. (The old header's
//!    `RandomBelow(i*10)/10` was invented.)
//! 3. `fn37`: `[0, 1, 3, 5, 8][Coherency−1]` rounds of *add or remove*.
//!    `r2 = RandomBelow(100) < 33` **REMOVES** a slot (a 9..16 if
//!    `r1 = RandomBelow(100) > 39`, else a 1..3 at a stride of
//!    `RandomBelow(5)+1`); otherwise it ADDS `9 + RandomBelow(80)/10`
//!    (9 → 10) or `1 + RandomBelow(30)/10`. The old header only ever added.
//! 4. `fn38`: with probability 50 %, if exactly ONE slot still holds a 1
//!    (`fn41` @3312) it becomes **7** (the two-eyes-in-one part); otherwise
//!    the 1st and 2nd slots holding a 1 (`fn40` @32C2) become 5 and 6 in a
//!    coin-flipped order. The arg order came off the listing at
//!    `00003202`/`00003216`: value first, ordinal second.
//!
//! ## Spawn (`fn24` @2058) and placement (`fn26` @24E8)
//!
//! `fn07` @0E40 gives the memory budget from the screen depth: **187 488**
//! (< 9 or > 16 bpp… see the quirk below), 365 000 (9..16), 720 000 (> 16).
//! Each sprite costs `fn0038(unionRect, depth) + classSize` (196 A / 190 B /
//! 190 C / 204 D) and spawning stops when the next one does not fit. On a
//! 32-bit screen that is **four A/B/C parts and nothing else** — which is
//! exactly what `frankenscreen.mp4` shows at t = 15 s and t = 75 s: two
//! eyes, a nose and a mouth on a stitched skin, never more.
//!
//! `fn26` then hands every `+0xA2` sprite (A, B, C and msg 10) a free grid
//! cell and every `+0xA4` sprite (the other class-D msgs) a non-overlapping
//! spot via `fn27` @2810 / `fn28` @28DE (8 random tries inside its own
//! region); a sprite that gets neither is destroyed on the spot.
//!
//! ## Backdrop (`fn20` @1710)
//!
//! `RandomBelow(40)/10` picks the skin, and the bank comes with it:
//!
//! | roll | bank | tile | loop 1 (`Blem/10 + RandomBelow(20)`) | loop 2 (`Blem/15`) |
//! |---|---|---|---|---|
//! | 0 | 1000 | 312 green | — | — |
//! | 1 | 1000 | 310 pink | 324 / 326 / 328 | — |
//! | 2 | 3000 | 252 orange | 254 / 256 / 258 | 268 |
//! | 3 | 3000 | 260 purple | 262 / 264 / 266 | 270 |
//!
//! Loop 3 always runs: `Blem/10 + RandomBelow(10)` stitches
//! `370 + 2·(RandomBelow(90)/10)` from **bank 2000**. Every stamp is at a
//! plain `(RandomBelow(W), RandomBelow(H))` — there is no overlap
//! rejection for blemishes, and the old port's `find_free_spot` call there
//! was invented. This kills three prose-era claims at once: loop 1 fires
//! on three of the four skins (not "the skin picks a family"), loop 2 is a
//! **single fixed id per skin** and is skipped on BOTH pink and green, and
//! the series assignment (310/312 → 1000, 252/260 → 3000, stitches → 2000)
//! is the C's, not a reel measurement.
//!
//! ## Original quirks kept
//!
//! * `fn07`'s depth test nests a "d above 16" arm inside its "d below 17"
//!   branch (small below 9, medium to 16, large from 17) — that inner arm
//!   is dead. Kept as written.
//! * `fn36` writes only into slots > 0 (`0 < fn39()`), so slot 0 can never
//!   take a blemish message even when it is free.
//! * `fn28` offsets the *test* rect by the rolled point but stores the
//!   point as the new absolute position.
//! * `fn113`'s state ladder has no else arm: an unlisted `+0xAE` leaves the
//!   target as whatever the register held. `+0xAE` opens at 234, which is
//!   listed, so it never bites.
//! * Coherency makes the creature *less* animated (`fn96` @43C2:
//!   100/50/25/12/6 %) while adding more parts (`fn37`).
//!
//! ## GAPs — found in the C, not ported, not invented
//!
//! * `GAP(fn142 @03CC)` — the grid-cell allocator. Ghidra: "Cannot properly
//!   adjust input varnodes". `fn143` @06C0 (which walks all nine cells from
//!   a random start in a random direction) and `fn141`'s 18-byte cell
//!   record are ported; the allocator itself is modelled as "the cell must
//!   be free; mark it used; place the sprite inside it".
//! * `GAP(fn52 @6E3C)` — the sequence/frame bounds helper, also a failed
//!   decompile. Every `+0x60`/`+0x18`/`+0x1C` rect in the module goes
//!   through it. Substituted with the pack's own frame geometry.
//! * `GAP(p_Sprite_MemSizeEstimate)` — library, no source. Modelled as
//!   `rowBytes(w, depth) * h + 0x5C` over the banks' largest frame. The
//!   *count* it yields (4 parts at every depth) matches the capture, but
//!   the bytes are not the original's.
//! * `GAP(fn0240 first arg)` — the decompiler types `fn0240`'s second
//!   parameter as a word at `A6+0x0C` while every caller pushes longs, so
//!   the id handed to the opening `+0x7C` is either the run or its high
//!   word (0). Ported as "SetRun(ids[0]) then queue the rest"; the other
//!   reading costs a blank frame per transition, which the capture does
//!   not show.
//! * `GAP(fn130 @664C)` — class D's custom compound rect (`+0xB4`), used as
//!   the bind rect in `fn79`. Needs `fn52`. Sprites bind to the whole bank.
//! * `GAP(fn82 @3E8E / fn83 @3EAC)` — the show / attach-children pass for
//!   `+0xA6 == 0` and `+0xA6 == 2`. Affects z-order and parenting only.
//! * `GAP(fn43 @3354)` — the 8-bit CLUT cross-fade that plays snd 30002.
//!   Gated on `g03F0 == 8`; the shell composes at 32 bpp, so it is
//!   unreachable and 30002 never fires. (The old port pushed it on every
//!   rebuild.)
//! * `GAP(B/O/C easter egg, fn16 @154A / fn17 @157E / fn18 @15A2)` — typing
//!   B, O, C extends a timer and latches `g03CC`, which appends msg 9 to
//!   the next script. The engine surfaces no key events, only Caps Lock.
//! * `GAP(fn19 @1650)` — the 30-minute `+0x2C` path that burns the live
//!   sprites into the backdrop instead of re-rolling the skin. Modelled as
//!   an ordinary rebuild.
//!
//! ## Sounds — an independent check on the whole decode
//!
//! `docs/emulator/audio-captures.md`'s 120 s FrankenScreen timeline counts
//! **moan2 ×12, drip_slither ×7, kiss ×4, Sniff ×4**, and records that
//! **30002 zap_spark and 30004 spit never fire** (best r < 0.2 over the
//! whole capture). The port cues each of those from exactly one place and
//! reproduces the shape without being tuned to it:
//!
//! * 30006 moan2 — class C only, on the three `fn123`/`fn124`/`fn125`
//!   transitions that carry it. The mouth is the loudest feature; it is.
//! * 30005 drip_slither — class B only (`fn111`'s two short branches,
//!   `fn112`'s 319/327 targets, `fn113`'s 236/243 chains).
//! * 30001 kiss — one site, `fn124`'s `next == 0x10A` (class C kind 1, the
//!   lips). The manifest's four kiss hits all read "lips".
//! * 30007 Sniff — one site, the 20 % roll every class-B machine opens
//!   with. The manifest's note at t = 76.59 s is "hairy nostrils on screen
//!   — the sniff matches the feature"; class B *is* the nose.
//! * 30002 — `fn43` only, behind `g03F0 == 8`. Unreachable at 32 bpp, so
//!   it never fires. (The old port pushed it on every rebuild.)
//! * 30004 spit — `fn134` only, i.e. class D msg 11, which the memory
//!   budget usually refuses. Never fired in the capture either.
//!
//! A 120 s census (`census_frankenscreen_sounds`, `#[ignore]`d) lands on
//! slither 12 / moan 10 / sniff 8 and no 30002 at all.
//!
//! ## Music
//!
//! `cmid` 30 "Horror Cue", 31.20 s, loaded by `fn29` and replayed by
//! `fn15`'s tail while `g03C8` (Music slider / 20, or 99 for ever) lasts.
//! Unchanged from the 2026-09-13 audio pass; it rides `Module::music`, not
//! the pre-empting `Ctx::sounds` channel.

use engine::l135::{self, FrameBox, LinkModel};
use engine::{
    ControlDef, ControlKind, Ctx, Module, Pack, SpriteDraw, TextDraw, TickClock, SCREEN_H, SCREEN_W,
};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants

/// `fn31` @2B56: `resId = 1000 + i * 1000`, kept in `g03DC[i]`.
const BANKS: [u32; 3] = [1000, 2000, 3000];

// sound resource ids, from `fn29`'s preload list (listing 00002ABA)
const SND_KISS: u32 = 30001; // 0x7531
#[allow(dead_code)] // GAP(fn43): only the 8-bit CLUT flash cues it
const SND_ZAP: u32 = 30002; // 0x7532 — fn43 only (8-bit), see GAP
const SND_SPIT: u32 = 30004; // 0x7534
const SND_SLITHER: u32 = 30005; // 0x7535
const SND_MOAN: u32 = 30006; // 0x7536
const SND_SNIFF: u32 = 30007; // 0x7537
const SONG_HORROR_CUE: u32 = 30;

/// `fn24` @2058 per-class object sizes (`operator_new` args).
const SIZE_A: i32 = 0xC4;
const SIZE_B: i32 = 0xBE;
const SIZE_C: i32 = 0xBE;
const SIZE_D: i32 = 0xCC;

/// `fn96` @43C2: P(the coherency roll fires), by Coherency 1..5.
const COH_PCT: [u32; 5] = [100, 50, 25, 12, 6];
/// `fn37` @2F92: rounds of add-or-remove, by Coherency 1..5.
const SCRIPT_ROUNDS: [u32; 5] = [0, 1, 3, 5, 8];
/// `fn103`/`fn114`/`fn120`: region inflation in px, by Coherency 1..5.
const REGION_PAD: [i32; 5] = [10, 40, 80, 200, 400];

/// `g0008`, dumped from `A5_globals.bin` at M129 data + 8: the static
/// script every Coherency setting uses.
const SCRIPT_BASE: [i32; 4] = [1, 1, 2, 3];

/// `fn93` @4204: class-A kind → (bank index, opening frame).
const A_KINDS: [(usize, i32); 6] = [(0, 3), (0, 121), (1, 19), (1, 28), (2, 9), (2, 108)];

/// `fn101` @46CC: `[previous direction][new direction][kind slot]` → run id.
/// Kind slot 0..4 is kinds 0, 1, 3, 4, 5 (kind 2 runs `fn102` instead).
/// Where the C folds a direction into the "no turn" arm — (0,0), (1,1),
/// (2,2), (3,3) and (4,0) — the row is the direction-4 row.
const A_TURNS: [[[i32; 5]; 5]; 5] = [
    // prev 0
    [
        [0x17, 0x85, 0x44, 0x39, 0x9A], // d0 → shares the d4 arm
        [0x1C, 0x88, 0x4E, 0x3E, 0x9D],
        [0x25, 0x90, 0x61, 0x46, 0xA3],
        [0x2A, 0x8C, 0x6B, 0x4A, 0xA7],
        [0x17, 0x85, 0x44, 0x39, 0x9A],
    ],
    // prev 1
    [
        [0x34, 0x96, 0x80, 0x53, 0xAF],
        [0x2F, 0x93, 0x76, 0x4E, 0xAB], // d1 → shares the d4 arm
        [0x3D, 0x9B, 0x93, 0x5B, 0xB5],
        [0x42, 0x9E, 0x9D, 0x5F, 0xB9],
        [0x2F, 0x93, 0x76, 0x4E, 0xAB],
    ],
    // prev 2
    [
        [0x5A, 0xAA, 0xC4, 0x1E, 0x7F],
        [0x55, 0xAD, 0xCE, 0x22, 0x83],
        [0x47, 0xA2, 0xA7, 0x13, 0x74], // d2 → shares the d4 arm
        [0x4C, 0xA5, 0xB1, 0x17, 0x78],
        [0x47, 0xA2, 0xA7, 0x13, 0x74],
    ],
    // prev 3
    [
        [0x72, 0xBC, 0xFF, 0x31, 0x91],
        [0x6D, 0xB8, 0xF5, 0x35, 0x95],
        [0x64, 0xB3, 0xE2, 0x2A, 0x8B],
        [0x5F, 0xB0, 0xD8, 0x26, 0x87], // d3 → shares the d4 arm
        [0x5F, 0xB0, 0xD8, 0x26, 0x87],
    ],
    // prev 4
    [
        [0x03, 0x7F, 0x1C, 0x09, 0x6C], // d0 → shares the d4 arm
        [0x08, 0x82, 0x30, 0x0E, 0x70],
        [0x0D, 0x7C, 0x26, 0x01, 0x64],
        [0x12, 0x79, 0x3A, 0x05, 0x68],
        [0x03, 0x7F, 0x1C, 0x09, 0x6C],
    ],
];

/// `fn106` @5174: class-B kind → (bank, opening frame, opening `+0xAE`).
const B_KINDS: [(usize, i32, i32); 3] =
    [(0, 0xC1, 0xEF), (1, 0x14F, 0x155), (2, 0xEA, 0xEA)];
/// `fn109` @5468: class-B act delay ceiling by kind.
const B_ACT_MS: [u32; 3] = [10_000, 7_000, 7_000];

/// `fn117` @5B72: class-C kind → (bank, opening frame, opening `+0xAE`).
const C_KINDS: [(usize, i32, i32); 3] =
    [(0, 0xFE, 0x124), (1, 0x115, 0x129), (2, 0xBD, 0xDA)];
/// `fn121` @5FEC: class-C act delay ceiling — 5000 for every kind.
const C_ACT_MS: u32 = 5_000;

/// `fn128` @6490 / `fn130` @664C: msg → (bank, run `+0xBE`, `+0xA6`,
/// `+0xBC` kind, grid?, no-overlap?).
struct DSpec {
    msg: i32,
    bank: usize,
    run: i32,
    attach: i32,
    kind: i32,
    grid: bool,
    spaced: bool,
}
const D_SPECS: [DSpec; 8] = [
    DSpec { msg: 9, bank: 0, run: 0x141, attach: 2, kind: 1, grid: false, spaced: true },
    DSpec { msg: 10, bank: 0, run: 0xFB, attach: 0, kind: 1, grid: true, spaced: false },
    DSpec { msg: 11, bank: 1, run: 0x15E, attach: 2, kind: 0, grid: false, spaced: true },
    DSpec { msg: 12, bank: 1, run: 0x15C, attach: 0, kind: 0, grid: false, spaced: false },
    DSpec { msg: 13, bank: 0, run: 0x13B, attach: 2, kind: 1, grid: false, spaced: true },
    DSpec { msg: 14, bank: 0, run: 0x13E, attach: 2, kind: 1, grid: false, spaced: true },
    DSpec { msg: 15, bank: 2, run: 0x110, attach: 2, kind: 1, grid: false, spaced: true },
    DSpec { msg: 16, bank: 2, run: 0x112, attach: 2, kind: 1, grid: false, spaced: true },
];

/// `fn20` @1766: `RandomBelow(40)/10` → (bank, tile id, loop-1 family,
/// loop-2 id).
const SKINS: [(usize, i32, &[i32], Option<i32>); 4] = [
    (0, 312, &[], None),                      // green
    (0, 310, &[324, 326, 328], None),         // pink
    (2, 252, &[254, 256, 258], Some(268)),    // orange
    (2, 260, &[262, 264, 266], Some(270)),    // purple
];
/// `fn20` @1CA2: the nine stitch compounds, bank 2000.
const STITCHES: [i32; 9] = [370, 372, 374, 376, 378, 380, 382, 384, 386];

/// `fn15` @144A: the sprite pass is gated `g01BA < now`, re-armed
/// `now + 100`. Strict, so on MacTick it lands on 7 ticks = 116.4 ms.
const DRAW_GATE_MS: u64 = 100;
/// `fn15` @13A4: the absolute 30-minute reset (`+0x22`).
const HARD_RESET_MS: u64 = 1_800_000;

// startup caption (`fn22` @1EF4 from the ctor @0BC8 → `fn21` @1E54, which
// does MoveTo(left+15, top+25), TextFont(0), ForeColor(30)).
const CAPTION_STR: u16 = 128;
const CAPTION_X: i32 = 16;
const CAPTION_Y: i32 = 17;

// ---------------------------------------------------------------------------
// Geometry

/// One packed frame's bank-space box: the compound rect (`bx,by,w,h`)
/// shifted by the frame's own OFst offset. GAP(fn52) substitute.
type Geom = FrameBox;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Class {
    A,
    B,
    C,
    D,
}

/// One backdrop stamp laid down by `fn20` (tile, pustule or stitch).
#[derive(Clone)]
struct Stamp {
    bank: usize,
    id: i32,
    x: i32,
    y: i32,
}

/// A sprite. Field comments are byte offsets into the C object.
#[derive(Clone)]
struct Sprite {
    class: Class,
    msg: i32,       // +0x9A
    bank: usize,    // which g03DC the fn79 bind picked
    kind: i32,      // +0xBC
    x: i32,         // +0x9E
    y: i32,         // +0xA0
    region: (i32, i32, i32, i32), // +0x92..+0x98, (l, t, r, b)
    flip: bool,     // +0x3C bit 0
    frame: i32,     // +0x3A
    first: i32,     // +0x44
    last: i32,      // first − 1 + len(first)
    fresh: bool,    // +0x48
    finished: bool, // +0x46
    queue: Vec<i32>,  // +0x60 ring
    next_run: i32,    // +0x4E
    armed: bool,    // +0xB0
    act_due: u64,   // +0xAA
    state_run: i32, // +0xAE
    dir: i32,       // +0xBE (class A) / the static run (class D)
    paired: bool,   // +0xC2
    grid: bool,     // +0xA2
    spaced: bool,   // +0xA4
    attach: i32,    // +0xA6
}

/// `g0004` — the object that keeps the two eyes in step.
#[derive(Clone, Default)]
struct Pair {
    cur: i32,        // +4
    target: i32,     // +8
    member_a: Option<usize>, // +0xC  (msg 5)
    member_b: Option<usize>, // +0x10 (msg 6)
    due: u64,        // +0x14
    pending: bool,   // +0x18
}

pub struct FrankenScreen {
    pack: Pack,
    geom: HashMap<(usize, i32), Geom>,
    block_end: HashMap<(usize, i32), i32>,
    /// The largest frame extent per bank — the `+0x1C` substitute the
    /// memory estimator unions (GAP).
    bank_max: [(i32, i32); 3],

    // controls, raw as GetControlValue returns them
    coh: i32,       // popup index 0..4; g01C2 = index + 1
    blem: i32,      // g03CE
    life_raw: i32,  // banded to g03D0 by fn14
    music_raw: i32, // g03CA
    music_count: u32, // g03C8

    // state
    sprites: Vec<Sprite>,
    backdrop: Vec<Stamp>,
    skin: usize,
    pair: Pair,
    script: [i32; 64],
    cells: [bool; 9],
    need_paint: bool, // +0x18
    built_at: u64,    // +0x1E
    started_at: u64,  // +0x22
    hard_reset: bool, // +0x2C
    draw_due: u64,    // g01BA
    egg_latch: bool,  // g03CC — see the GAP list
    started: bool,
    caption: Option<String>,
    painted: bool,
    music_plays: u32,
    music_next_ok: u64,
}

pub fn make(pack: Pack) -> Option<Box<dyn Module>> {
    build(pack).map(|m| Box::new(m) as Box<dyn Module>)
}

/// Concrete constructor — the tests drive the module directly.
fn build(pack: Pack) -> Option<FrankenScreen> {
    for base in BANKS {
        if !pack.meta.series.contains_key(&base.to_string()) {
            return None;
        }
    }
    let mut geom = HashMap::new();
    let mut block_end = HashMap::new();
    let mut bank_max = [(0i32, 0i32); 3];
    for (b, base) in BANKS.iter().enumerate() {
        for seq in pack.series(*base) {
            let end = seq.first as i32 + seq.frames.len() as i32 - 1;
            for (i, f) in seq.frames.iter().enumerate() {
                let id = seq.first as i32 + i as i32;
                geom.insert(
                    (b, id),
                    Geom { bx: f.bx, by: f.by, w: f.w, h: f.h, dx: f.dx, dy: f.dy },
                );
                block_end.insert((b, id), end);
                bank_max[b].0 = bank_max[b].0.max(f.w);
                bank_max[b].1 = bank_max[b].1.max(f.h);
            }
        }
    }
    Some(FrankenScreen {
        pack,
        geom,
        block_end,
        bank_max,
        coh: 2,
        blem: 40,
        life_raw: 0,
        music_raw: 22,
        music_count: music_count_for(22),
        sprites: Vec::new(),
        backdrop: Vec::new(),
        skin: 0,
        pair: Pair::default(),
        script: [-1; 64],
        cells: [false; 9],
        need_paint: true,
        built_at: 0,
        started_at: 0,
        hard_reset: false,
        draw_due: 0,
        egg_latch: false,
        started: false,
        caption: None,
        painted: false,
        music_plays: 0,
        music_next_ok: 0,
    })
}

/// `fn13` @121E: `g03CA = GetControlValue(3)`; ≥ 99 → 99 ("'Til Ya Drop"),
/// else `min(raw / 20, 4)`.
fn music_count_for(raw: i32) -> u32 {
    if raw >= 99 {
        99
    } else {
        (raw / 20).clamp(0, 4) as u32
    }
}

/// `fn14` @12C2: Lifespan raw → ms (< 20 → 30 s, < 40 → 2 m, < 60 → 5 m,
/// < 80 → 10 m, else 1 h; × 1000).
fn lifespan_ms(raw: i32) -> u64 {
    let s: u64 = match raw {
        r if r < 20 => 30,
        r if r < 40 => 120,
        r if r < 60 => 300,
        r if r < 80 => 600,
        _ => 3600,
    };
    s * 1000
}

/// `fn07` @0E40 — the memory budget, by screen depth. The `16 < d` arm
/// inside the `d < 17` branch is dead in the original; kept as written.
fn budget_for(depth: i32) -> i32 {
    if depth < 0x11 {
        if depth < 9 || 0x10 < depth {
            0x2DE60
        } else {
            0x591C8
        }
    } else {
        720_000
    }
}

/// The shell composes at 32 bpp (`g03F0 = Canvas_DrawEntry()`).
const DEPTH: i32 = 32;

impl FrankenScreen {
    // ---- bank helpers (GAP(fn52) substitutes) -----------------------------

    fn g(&self, bank: usize, id: i32) -> Option<&Geom> {
        self.geom.get(&(bank, id))
    }

    /// L135 `fn3A70`: `centre(f) = (bx+dx+w/2, by+dy+h/2)`. Only the tests
    /// use it now (the superseded centre-difference model they measure the
    /// linked hand-off against).
    #[cfg(test)]
    fn centre(&self, bank: usize, id: i32) -> Option<(i32, i32)> {
        self.g(bank, id).map(|g| g.centre())
    }

    /// Sequence `+0x78` = L135 `fn3DDC @3DDC`, the in-run link `from → to`
    /// ([`l135::link`], the transcribed flip-bit rounding: unflipped exactly
    /// `centre(to) − centre(from)`; flipped, 1 px off it wherever the two
    /// widths' parities differ). No move when either id is unknown.
    /// GAP(fn3DDC seq flag): `fn3DDC` XORs the flip it is handed with the
    /// sequence object's own `+0x3C` bits; nothing this module calls sets
    /// those, so they are taken as 0 (`LinkModel::seq_flip`).
    fn link(&self, bank: usize, from: i32, to: i32, flip: bool) -> (i32, i32) {
        let (Some(a), Some(b)) = (self.g(bank, from), self.g(bank, to)) else {
            return (0, 0);
        };
        l135::link(a, b, flip, LinkModel::FN3DDC)
    }

    /// L135 `fn3BD6 @3BD6`'s part layout, relative to the frame centre
    /// ([`FrameBox::part_rel`]).
    #[cfg(test)]
    fn part_rel(&self, bank: usize, id: i32, part: &[i32; 7], flip: bool) -> Option<[i32; 4]> {
        Some(self.g(bank, id)?.part_rel(part, flip))
    }

    /// L135 **`fn3F2E @3F2E`** ([`l135::register`]), the shared-part
    /// registration a hand-off links through (`fn0D3C @0D3C`): the first
    /// part the two frames share keeps its screen position, and the sprite's
    /// flip is XORed with the two parts' own flip flags. Part tables come off
    /// the pack, the boxes off the module's map. No record or no common part:
    /// no move, no flip change. Returns `(dx, dy, flip', shared art id)`
    /// (−1: none).
    fn shared_link(&self, bank: usize, cur: i32, new: i32, flip: bool) -> (i32, i32, bool, i32) {
        let base = BANKS[bank];
        let (Some(ga), Some(gb)) =
            (self.pack.frame(base, cur.max(0) as u32), self.pack.frame(base, new.max(0) as u32))
        else {
            return (0, 0, flip, -1);
        };
        let (Some(ba), Some(bb)) = (self.g(bank, cur), self.g(bank, new)) else {
            return (0, 0, flip, -1);
        };
        match l135::register(ba, &ga.parts, bb, &gb.parts, flip) {
            Some(r) => (r.delta.0, r.delta.1, r.flip, r.art),
            None => (0, 0, flip, -1),
        }
    }

    /// `+0xF0` = L132 `fn1186 @1186`: the frame a hand-off to run `id`
    /// passes through — `id − 1` when that record exists (`+0x82` is 1
    /// after the sprite reset), else `id` itself.
    fn marker_of(&self, bank: usize, id: i32) -> i32 {
        l135::marker_of(id, |f| f >= 1 && self.g(bank, f).is_some())
    }

    /// `first − 1 + len(first)` (L135 `fn4456`): walk to the end of the
    /// OFst block that CONTAINS `first` — never an exact-`first` match
    /// (library40-api §10.1).
    fn last_of(&self, bank: usize, first: i32) -> i32 {
        self.block_end.get(&(bank, first)).copied().unwrap_or(first)
    }

    fn size(&self, bank: usize, id: i32) -> (i32, i32) {
        self.g(bank, id).map(|g| (g.w, g.h)).unwrap_or((0, 0))
    }

    // ---- sprite ops (all L132_Resource) ------------------------------------

    /// `+0x108` = L132 **`fn028A @028A`** — start run `id`. Does NOT clear
    /// the queue (only `fn0204` does). When a frame is showing:
    ///
    /// 1. `+0xF0` = `fn1186` picks the marker ([`Self::marker_of`]);
    /// 2. `+0xCC` = `fn0D3C` links the CURRENT frame onto it through L135
    ///    `fn3F2E` ([`Self::shared_link`]): the first part the two frames
    ///    share keeps its screen position, and the sprite's flip is XORed
    ///    with the two parts' own flip bits;
    /// 3. `+0xD0` = `fn0DF4` steps marker → `id` by `fn3DDC` (0 when the
    ///    marker is `id` itself).
    ///
    /// Superseded 2026-09-29: this used to compose to a plain
    /// `centre(id) − centre(frame)`. In THIS pack the two agree on every
    /// hand-off the machines can make (see the header's "Hand-off audit");
    /// the transcription changes no drawn frame.
    fn set_run_inner(&mut self, i: usize, id: i32) {
        let (bank, frame, flip) = {
            let s = &self.sprites[i];
            (s.bank, s.frame, s.flip)
        };
        if frame != 0 {
            let mk = self.marker_of(bank, id);
            let (dx, dy, flip2, _) = self.shared_link(bank, frame, mk, flip);
            let (lx, ly) = self.link(bank, mk, id, flip2);
            let s = &mut self.sprites[i];
            s.x += dx + lx;
            s.y += dy + ly;
            s.flip = flip2;
        }
        let last = self.last_of(bank, id);
        let s = &mut self.sprites[i];
        s.frame = id;
        s.first = id;
        s.last = last;
        s.fresh = true;
        s.next_run = -1;
    }

    /// `+0x7C` = `fn0204`: clear the queue, then `fn028A`.
    fn set_run(&mut self, i: usize, id: i32) {
        self.sprites[i].queue.clear();
        self.set_run_inner(i, id);
        self.sprites[i].finished = false;
    }

    /// `+0x80` = `fn0240(this, id₀, id₁, …, −1)`: SetRun the first, queue
    /// the rest. See GAP(fn0240 first arg).
    fn play(&mut self, i: usize, ids: &[i32]) {
        self.set_run(i, ids[0]);
        self.sprites[i].queue.extend_from_slice(&ids[1..]);
    }

    /// `+0x70` = `fn01E2`: show one frame with no run behind it. The C
    /// leaves `+0x44` at 0, so the next `+0x84` immediately reports the
    /// sequence finished — which is why a fresh part holds its opening
    /// pose until its first act.
    fn set_frame(&mut self, i: usize, id: i32) {
        let s = &mut self.sprites[i];
        s.frame = id;
        s.first = 0;
        s.last = 0;
        s.fresh = false;
        s.finished = true;
    }

    /// `+0x84` = `fn0316`, the per-frame advance.
    fn advance(&mut self, i: usize) {
        self.sprites[i].finished = false;
        let next = self.sprites[i].next_run;
        if next != -1 {
            self.set_run_inner(i, next);
        }
        let (bank, first, flip, frame, fresh) = {
            let s = &self.sprites[i];
            (s.bank, s.first, s.flip, s.frame, s.fresh)
        };
        let last = self.last_of(bank, first);
        if fresh {
            self.sprites[i].fresh = false;
        } else if frame < last && frame >= first {
            let (dx, dy) = self.link(bank, frame, frame + 1, flip);
            let s = &mut self.sprites[i];
            s.x += dx;
            s.y += dy;
            s.frame = frame + 1;
        }
        let s = &mut self.sprites[i];
        if s.frame >= last || s.frame < first {
            if s.queue.is_empty() {
                s.finished = true;
            } else {
                s.next_run = s.queue.remove(0);
            }
        }
    }

    /// `fn97`/`fn107`/`fn119`/`fn138` `+0x154` — the drawn rect, centred on
    /// pos: `(x − w/2, y − h/2, +w, +h)`.
    fn rect(&self, i: usize) -> (i32, i32, i32, i32) {
        let s = &self.sprites[i];
        let (w, h) = self.size(s.bank, s.frame);
        (s.x - w / 2, s.y - h / 2, s.x - w / 2 + w, s.y - h / 2 + h)
    }

    fn overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
        a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
    }

    // ---- the eye pair (g0004) ---------------------------------------------

    /// `fn150` @00B0: claim the msg-5 or msg-6 slot. Returns whether the
    /// claim took.
    fn pair_register(&mut self, i: usize, msg: i32) -> bool {
        if msg == 5 && self.pair.member_a.is_none() {
            self.pair.member_a = Some(i);
            return true;
        }
        if msg == 6 && self.pair.member_b.is_none() {
            self.pair.member_b = Some(i);
            return true;
        }
        false
    }

    /// `fn149` @0084: how many members are registered.
    fn pair_count(&self) -> i32 {
        self.pair.member_a.is_some() as i32 + self.pair.member_b.is_some() as i32
    }

    /// `fn153` @0182, run once per draw gate before the sprites.
    fn pair_tick(&mut self, ctx: &mut Ctx) {
        // fn95 @43A0 is `+0x46 == 0` — i.e. "still mid-run".
        let busy = |m: Option<usize>, s: &Vec<Sprite>| match m {
            Some(j) => j < s.len() && !s[j].finished,
            None => false,
        };
        if busy(self.pair.member_a, &self.sprites) || busy(self.pair.member_b, &self.sprites) {
            self.pair.pending = true;
            return;
        }
        if self.pair.pending {
            self.pair.pending = false;
            self.pair.cur = self.pair.target;
            self.pair.due = ctx.now_ms + u64::from(ctx.rng.pct(4000));
        }
        if ctx.now_ms > self.pair.due {
            // fn151 @0108
            let mut t = self.pair.cur;
            while t == self.pair.cur {
                t = (ctx.rng.pct(50) / 10) as i32;
            }
            self.pair.target = t;
        }
    }

    /// `fn152` @0158: a member acts when the target differs and nobody is
    /// mid-run.
    fn pair_go(&self) -> bool {
        self.pair.cur != self.pair.target && !self.pair.pending
    }

    // ---- script generation -------------------------------------------------

    /// `fn39` @328C: the first free slot, or −1.
    fn free_slot(&self) -> i32 {
        self.script.iter().position(|&v| v == -1).map(|i| i as i32).unwrap_or(-1)
    }

    /// `fn40` @32C2: the `ordinal`-th slot holding `value`, or −1.
    fn nth_slot(&self, value: i32, ordinal: i32) -> i32 {
        let mut n = 0;
        for (i, &v) in self.script.iter().enumerate() {
            if v == value {
                n += 1;
                if n == ordinal {
                    return i as i32;
                }
            }
        }
        -1
    }

    /// `fn41` @3312: how many slots hold `value`.
    fn count_slots(&self, value: i32) -> i32 {
        self.script.iter().filter(|&&v| v == value).count() as i32
    }

    fn coherency(&self) -> usize {
        (self.coh.clamp(0, 4) + 1) as usize // g01C2
    }

    /// `fn33` @2CC0 (the `g03D4 == 8` arm, which is the only live one).
    fn make_script(&mut self, ctx: &mut Ctx) {
        // fn34 @2D60
        self.script = [-1; 64];
        for (i, &v) in SCRIPT_BASE.iter().enumerate() {
            self.script[i] = v;
        }
        if self.egg_latch {
            let s = self.free_slot();
            if s >= 0 {
                self.script[s as usize] = 9;
                self.egg_latch = false;
            }
        }
        // fn36 @2F00: Blemishes/8 events, msg 11 + RandomBelow(60)/10
        let n = self.blem.max(0) / 8;
        for _ in 0..n {
            let msg = 11 + (ctx.rng.pct(60) / 10) as i32;
            let s = self.free_slot();
            if s > 0 {
                self.script[s as usize] = msg;
            }
        }
        // fn37 @2F92
        for _ in 0..SCRIPT_ROUNDS[self.coherency() - 1] {
            let r1 = ctx.rng.pct(100);
            let r2 = ctx.rng.pct(100);
            if r2 < 0x21 {
                // remove
                if r1 > 0x27 {
                    for i in 0..64 {
                        let v = self.script[i];
                        if v > 8 && v < 0x11 && v != 9 {
                            self.script[i] = -1;
                            break;
                        }
                    }
                } else {
                    let stride = (ctx.rng.pct(5) + 1) as usize;
                    for i in 0..64 {
                        let v = self.script[i];
                        if i % stride == 0 && v > 0 && v < 4 {
                            self.script[i] = -1;
                            break;
                        }
                    }
                }
            } else {
                // add
                let s = self.free_slot();
                if s >= 0 {
                    let v = if r1 > 0x27 {
                        let m = (ctx.rng.pct(0x50) / 10) as i32 + 9;
                        if m == 9 {
                            10
                        } else {
                            m
                        }
                    } else {
                        (ctx.rng.pct(0x1E) / 10) as i32 + 1
                    };
                    self.script[s as usize] = v;
                }
            }
        }
        // fn38 @31A6
        let single = ctx.rng.pct(100) < 50 && self.count_slots(1) == 1;
        if single {
            let s = self.nth_slot(1, 1);
            if s != -1 {
                self.script[s as usize] = 7;
            }
        } else {
            let a = if ctx.rng.pct(100) <= 50 { 6 } else { 5 };
            let b = if a == 5 { 6 } else { 5 };
            let s1 = self.nth_slot(1, 1);
            let s2 = self.nth_slot(1, 2);
            if s1 != -1 {
                self.script[s1 as usize] = a;
            }
            if s2 != -1 {
                self.script[s2 as usize] = b;
            }
        }
    }

    // ---- build / teardown ---------------------------------------------------

    /// `p_Sprite_MemSizeEstimate` substitute — see GAP.
    fn mem_estimate_at(&self, banks: &[usize], depth: i32) -> i32 {
        let mut w = 0;
        let mut h = 0;
        for &b in banks {
            w = w.max(self.bank_max[b].0);
            h = h.max(self.bank_max[b].1);
        }
        let row_bytes = ((w * depth + 31) / 32) * 4;
        row_bytes * h + 0x5C
    }

    fn mem_estimate(&self, banks: &[usize]) -> i32 {
        self.mem_estimate_at(banks, DEPTH)
    }

    /// `fn08` @0E82: grid, script, pair reset, spawn, placement.
    fn build_creature(&mut self, ctx: &mut Ctx) {
        self.cells = [false; 9]; // fn141 @022C
        self.make_script(ctx); // fn33
        self.pair = Pair { cur: 4, target: 4, ..Pair::default() }; // fn148 @004E
        self.spawn(ctx); // fn24
        self.place(ctx); // fn26
    }

    /// `fn24` @2058.
    fn spawn(&mut self, ctx: &mut Ctx) {
        self.sprites.clear();
        let mut budget = budget_for(DEPTH);
        for slot in 0..64 {
            if budget <= 0 {
                break;
            }
            let msg = self.script[slot];
            let (class, cost) = match msg {
                1 | 5 | 6 | 7 => (Class::A, self.mem_estimate(&[0, 1, 2]) + SIZE_A), // fn94 @4294
                2 => (Class::B, self.mem_estimate(&[0, 1, 2]) + SIZE_B), // fn108 @538C
                3 => (Class::C, self.mem_estimate(&[0, 1, 2]) + SIZE_C), // fn118 @5C3C
                9..=16 => {
                    let b = D_SPECS.iter().find(|d| d.msg == msg).map(|d| d.bank).unwrap_or(0);
                    (Class::D, self.mem_estimate(&[b]) + SIZE_D) // fn129 @6624
                }
                _ => continue,
            };
            if cost >= budget {
                continue;
            }
            budget -= cost;
            let i = self.sprites.len();
            self.sprites.push(Sprite {
                class,
                msg,
                bank: 0,
                kind: 0,
                x: 0,
                y: 0,
                region: (0, 0, SCREEN_W, SCREEN_H),
                flip: false,
                frame: 0,
                first: 0,
                last: 0,
                fresh: false,
                finished: true, // fn0058 sets +0x46 = 1
                queue: Vec::new(),
                next_run: -1,
                armed: false,
                act_due: 0,
                state_run: 0,
                dir: 4,
                paired: false,
                grid: false,
                spaced: false,
                attach: 1,
            });
            match class {
                Class::A => self.init_a(ctx, i, msg),
                Class::B => self.init_b(ctx, i, msg),
                Class::C => self.init_c(ctx, i, msg),
                Class::D => self.init_d(ctx, i, msg),
            }
        }
    }

    /// `fn79` @3C7A: region → clip to screen → random point inside it.
    fn place_random(&mut self, ctx: &mut Ctx, i: usize) {
        let r = self.sprites[i].region;
        let r = (r.0.max(0), r.1.max(0), r.2.min(SCREEN_W), r.3.min(SCREEN_H));
        self.sprites[i].region = r;
        let w = (r.2 - r.0 - 1).max(1);
        let h = (r.3 - r.1 - 1).max(1);
        let dx = ctx.rng.pct(w as u32) as i32;
        let dy = ctx.rng.pct(h as u32) as i32;
        self.sprites[i].x = r.0 + dx;
        self.sprites[i].y = r.1 + dy;
    }

    /// `fn103`/`fn114`/`fn120`: a point at (`px`%, `py`%) of the field,
    /// inflated by Coherency.
    fn point_region(&self, px: i32, py: i32) -> (i32, i32, i32, i32) {
        let x = px * SCREEN_W / 100;
        let y = py * SCREEN_H / 100;
        let p = REGION_PAD[self.coherency() - 1];
        (x - p, y - p, x + p, y + p)
    }

    /// `fn93` @4084.
    fn init_a(&mut self, ctx: &mut Ctx, i: usize, msg: i32) {
        let claimed = if msg == 5 || msg == 6 {
            self.sprites[i].paired = true;
            self.pair_register(i, msg)
        } else {
            false
        };
        let mut kind = (ctx.rng.pct(600) / 100) as i32; // g03D4 == 8 arm
        if self.pair_count() == 2 && claimed {
            // fn96 @43C2 → copy the partner's kind so the eyes match
            if ctx.rng.pct(100) < COH_PCT[self.coherency() - 1] {
                let other = if msg == 5 { self.pair.member_b } else { self.pair.member_a };
                if let Some(j) = other {
                    kind = self.sprites[j].kind;
                }
            }
        }
        let (bank, open) = A_KINDS[kind.clamp(0, 5) as usize];
        {
            let s = &mut self.sprites[i];
            s.kind = kind;
            s.bank = bank;
            s.dir = if kind == 2 { 0 } else { 4 };
            s.grid = true; // ctor fn91 sets +0xA2 = 1
        }
        // fn103 @4F16
        self.sprites[i].region = if msg == 1 {
            (0, 0, SCREEN_W, SCREEN_H)
        } else {
            let px = match msg {
                5 => 30,
                6 => 70,
                _ => 50,
            };
            self.point_region(px, 20)
        };
        self.place_random(ctx, i);
        if msg == 6 {
            self.sprites[i].flip = !self.sprites[i].flip; // @4258
        }
        self.set_frame(i, open);
    }

    /// `fn106` @5174.
    fn init_b(&mut self, ctx: &mut Ctx, i: usize, msg: i32) {
        let kind = (ctx.rng.pct(300) / 100) as i32;
        let (bank, open, ae) = B_KINDS[kind.clamp(0, 2) as usize];
        {
            let s = &mut self.sprites[i];
            s.kind = kind;
            s.bank = bank;
            s.state_run = ae;
            s.grid = true;
        }
        self.sprites[i].region = self.point_region(50, 45); // fn114 @5980
        self.place_random(ctx, i);
        self.start(ctx, i, open, msg);
    }

    /// `fn117` @5B72.
    fn init_c(&mut self, ctx: &mut Ctx, i: usize, msg: i32) {
        let kind = (ctx.rng.pct(300) / 100) as i32;
        let (bank, open, ae) = C_KINDS[kind.clamp(0, 2) as usize];
        {
            let s = &mut self.sprites[i];
            s.kind = kind;
            s.bank = bank;
            s.state_run = ae;
            s.grid = true;
        }
        self.sprites[i].region = self.point_region(50, 80); // fn120 @5E3C
        self.place_random(ctx, i);
        self.start(ctx, i, open, msg);
    }

    /// `fn128` @6490.
    fn init_d(&mut self, ctx: &mut Ctx, i: usize, msg: i32) {
        let Some(d) = D_SPECS.iter().find(|d| d.msg == msg) else { return };
        {
            let s = &mut self.sprites[i];
            s.bank = d.bank;
            s.kind = d.kind;
            s.attach = d.attach;
            s.grid = d.grid;
            s.spaced = d.spaced;
            s.dir = d.run;
            s.state_run = d.run;
            s.region = (0, 0, SCREEN_W, SCREEN_H); // fn131 @6756
        }
        self.place_random(ctx, i);
        if d.kind == 0 {
            // fn81 with no run word of its own (see GAP) — open on +0xAE
            let open = self.sprites[i].state_run;
            self.start(ctx, i, open, msg);
        }
    }

    /// `fn81` @3E46: SetFrame, SetPos, `+0xB0 = 1`, then the class's own
    /// `+0x15C` arms the act deadline.
    fn start(&mut self, ctx: &mut Ctx, i: usize, open: i32, _msg: i32) {
        self.set_frame(i, open);
        self.sprites[i].armed = true;
        self.arm(ctx, i);
    }

    /// `+0x15C` — `fn109` @5468 / `fn121` @5FEC / `fn132` @67B4. Class A's
    /// is `fn72`, an empty stub; it arms lazily inside `fn100` instead.
    fn arm(&mut self, ctx: &mut Ctx, i: usize) {
        let (class, kind, msg) = {
            let s = &self.sprites[i];
            (s.class, s.kind, s.msg)
        };
        let ceiling = match class {
            Class::B => B_ACT_MS[kind.clamp(0, 2) as usize],
            Class::C => C_ACT_MS,
            Class::D => {
                if msg == 11 {
                    10_000
                } else {
                    1_000
                }
            }
            Class::A => return,
        };
        self.sprites[i].act_due = ctx.now_ms + u64::from(ctx.rng.pct(ceiling));
    }

    /// `fn26` @24E8: grid pass, show/attach pass (GAP), spacing pass.
    fn place(&mut self, ctx: &mut Ctx) {
        let cw = SCREEN_W / 3;
        let ch = SCREEN_H / 3;
        let mut dead: Vec<usize> = Vec::new();
        for i in 0..self.sprites.len() {
            if !self.sprites[i].grid {
                continue;
            }
            // fn142(grid, sprite, −1, &pt) — GAP: modelled as "the cell my
            // point already sits in must be free".
            let (x, y) = (self.sprites[i].x, self.sprites[i].y);
            let c0 = ((x / cw.max(1)).clamp(0, 2) * 3 + (y / ch.max(1)).clamp(0, 2)) as usize;
            if !self.cells[c0] {
                self.cells[c0] = true;
                continue;
            }
            // fn143 @06C0: walk all nine from a random start in a random
            // direction, taking the first free one.
            let start = (ctx.rng.pct(900) / 100) as i32;
            let up = ctx.rng.pct(900) / 100 == 0; // fn53 @6ED8
            let mut c = start;
            let mut took = false;
            for _ in 0..9 {
                if c > 8 {
                    c = 0;
                }
                if c < 0 {
                    c = 8;
                }
                if !self.cells[c as usize] {
                    self.cells[c as usize] = true;
                    let (col, row) = (c / 3, c % 3);
                    self.sprites[i].x = col * cw + ctx.rng.pct(cw.max(1) as u32) as i32;
                    self.sprites[i].y = row * ch + ctx.rng.pct(ch.max(1) as u32) as i32;
                    took = true;
                    break;
                }
                c += if up { 1 } else { -1 };
            }
            if !took {
                dead.push(i);
            }
        }
        // GAP(fn82/fn83): the show / attach-children pass.
        for i in 0..self.sprites.len() {
            if !self.sprites[i].spaced || dead.contains(&i) {
                continue;
            }
            // fn27 @2810: do I overlap anybody?
            let mine = self.rect(i);
            let clash = (0..self.sprites.len())
                .any(|j| j != i && !dead.contains(&j) && Self::overlap(mine, self.rect(j)));
            if !clash {
                continue;
            }
            // fn28 @28DE: 8 random tries inside my own region.
            let r = self.sprites[i].region;
            let (rw, rh) = ((r.2 - r.0).max(1), (r.3 - r.1).max(1));
            let mut ok = false;
            for _ in 0..8 {
                let px = ctx.rng.pct(rw as u32) as i32;
                let py = ctx.rng.pct(rh as u32) as i32;
                // the original offsets the TEST rect but stores the raw
                // point as the new position — quirk kept.
                let probe = (mine.0 + px, mine.1 + py, mine.2 + px, mine.3 + py);
                if !(0..self.sprites.len())
                    .any(|j| j != i && !dead.contains(&j) && Self::overlap(probe, self.rect(j)))
                {
                    self.sprites[i].x = px;
                    self.sprites[i].y = py;
                    ok = true;
                    break;
                }
            }
            if !ok {
                dead.push(i);
            }
        }
        dead.sort_unstable();
        dead.dedup();
        for i in dead.into_iter().rev() {
            self.sprites.remove(i);
            if self.pair.member_a == Some(i) {
                self.pair.member_a = None;
            }
            if self.pair.member_b == Some(i) {
                self.pair.member_b = None;
            }
            for m in [&mut self.pair.member_a, &mut self.pair.member_b] {
                if let Some(j) = m {
                    if *j > i {
                        *j -= 1;
                    }
                }
            }
        }
    }

    // ---- the backdrop (fn20 @1710) -----------------------------------------

    fn paint_backdrop(&mut self, ctx: &mut Ctx) {
        self.backdrop.clear();
        // GAP(fn43): the 8-bit CLUT flash + snd 30002 lives behind
        // `g03F0 == 8`, which never holds here.
        let roll = (ctx.rng.pct(40) / 10) as usize;
        self.skin = roll.min(3);
        let (bank, tile, family, loop2) = SKINS[self.skin];
        // tile the field
        let (tw, th) = self.size(bank, tile);
        if tw > 0 && th > 0 {
            let cols = SCREEN_W / tw + 1;
            let rows = SCREEN_H / th + 1;
            for r in 0..rows {
                for c in 0..cols {
                    self.backdrop.push(Stamp { bank, id: tile, x: c * tw, y: r * th });
                }
            }
        }
        let blem = self.blem.max(0);
        // loop 1 @19A8
        if !family.is_empty() {
            let n = blem / 10 + ctx.rng.pct(20) as i32;
            for _ in 0..n {
                let k = (ctx.rng.pct(30) / 10) as usize;
                let id = family[k.min(family.len() - 1)];
                let x = ctx.rng.pct(SCREEN_W as u32) as i32;
                let y = ctx.rng.pct(SCREEN_H as u32) as i32;
                self.backdrop.push(Stamp { bank, id, x, y });
            }
        }
        // loop 2 @1B6E — one fixed id, orange and purple skins only
        if let Some(id) = loop2 {
            let n = blem / 15;
            for _ in 0..n {
                let x = ctx.rng.pct(SCREEN_W as u32) as i32;
                let y = ctx.rng.pct(SCREEN_H as u32) as i32;
                self.backdrop.push(Stamp { bank, id, x, y });
            }
        }
        // loop 3 @1C7C — stitches, always, out of bank 2000
        let n = blem / 10 + ctx.rng.pct(10) as i32;
        for _ in 0..n {
            let id = STITCHES[(ctx.rng.pct(90) / 10) as usize % STITCHES.len()];
            let x = ctx.rng.pct(SCREEN_W as u32) as i32;
            let y = ctx.rng.pct(SCREEN_H as u32) as i32;
            // GAP: the third arg to the bank's draw op is
            // `RandomBelow(30)/10` ∈ 0..2 here and 0 in loops 1/2 — a
            // transform mode we do not model. The roll is consumed so the
            // stream stays in step.
            let _mode = ctx.rng.pct(30) / 10;
            self.backdrop.push(Stamp { bank: 1, id, x, y });
        }
    }

    // ---- per-class DoFrame (+0x13C) ----------------------------------------

    /// `fn99` @45C8: the next direction.
    fn pick_dir(&mut self, ctx: &mut Ctx, i: usize) -> i32 {
        if self.sprites[i].paired {
            return self.pair.target;
        }
        let cur = self.sprites[i].dir;
        let mut d = cur;
        while d == cur {
            d = (ctx.rng.pct(50) / 10) as i32;
        }
        d
    }

    /// `fn100` @462C: the act gate.
    fn act_gate(&mut self, ctx: &mut Ctx, i: usize) -> bool {
        if self.sprites[i].paired {
            return self.pair_go();
        }
        if !self.sprites[i].finished {
            return false;
        }
        if !self.sprites[i].armed {
            self.sprites[i].armed = true;
            self.sprites[i].act_due = ctx.now_ms + u64::from(ctx.rng.pct(3000));
        }
        if ctx.now_ms > self.sprites[i].act_due {
            self.sprites[i].armed = false;
            return true;
        }
        false
    }

    /// `fn101` @46CC.
    fn frame_a_turn(&mut self, ctx: &mut Ctx, i: usize) {
        if self.act_gate(ctx, i) {
            let mut d = self.pick_dir(ctx, i);
            if self.sprites[i].msg == 6 {
                // @46FC: the right eye mirrors left/right
                if d == 1 {
                    d = 0;
                } else if d == 0 {
                    d = 1;
                }
            }
            let (prev, kind) = {
                let s = &self.sprites[i];
                (s.dir, s.kind)
            };
            let slot = match kind {
                0 => 0,
                1 => 1,
                3 => 2,
                4 => 3,
                _ => 4,
            };
            let run = A_TURNS[prev.clamp(0, 4) as usize][d.clamp(0, 4) as usize][slot];
            self.set_run(i, run);
            self.sprites[i].dir = d;
        }
        self.advance(i);
    }

    /// `fn102` @4E5C — kind 2's two-pose machine.
    fn frame_a_pair(&mut self, ctx: &mut Ctx, i: usize) {
        if self.act_gate(ctx, i) {
            let d = self.pick_dir(ctx, i);
            let grp = match d {
                0 | 1 | 3 => 0,
                _ => 1,
            };
            let prev = self.sprites[i].dir;
            if prev == 0 && grp == 1 {
                self.set_run(i, 0x13);
            } else if prev == 1 && grp == 0 {
                self.set_run(i, 7);
            }
            self.sprites[i].dir = grp;
        }
        self.advance(i);
    }

    /// `fn111` @5504 — class B kind 0. No state test: the roll alone picks.
    fn frame_b0(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            if ctx.rng.pct(100) < 20 {
                ctx.sounds.push(SND_SNIFF);
            }
            let next = if (0..=9).contains(&r) {
                self.play(i, &[0xC3, 0xC1]);
                ctx.sounds.push(SND_SLITHER);
                0xC3
            } else if (10..=0x13).contains(&r) {
                self.play(i, &[0xD0, 0xC1]);
                ctx.sounds.push(SND_SLITHER);
                0xD0
            } else if (0x14..=0x1D).contains(&r) {
                self.play(i, &[0xE0, 0xEA, 0xF5]);
                0xE0
            } else if (0x1E..=0x27).contains(&r) {
                self.play(i, &[0xDB, 0xE5, 0xEF]);
                0xDB
            } else {
                -1
            };
            if next != -1 {
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
        }
        self.advance(i);
    }

    /// `fn112` @56B4 — class B kind 1.
    fn frame_b1(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            if ctx.rng.pct(100) < 20 {
                ctx.sounds.push(SND_SNIFF);
            }
            let st = self.sprites[i].state_run;
            let next = if st == 0x155 || st == 0x13F || st == 0x147 {
                if r <= 0x13 {
                    -1
                } else if r <= 0x31 {
                    0x13F
                } else if r <= 0x4F {
                    0x147
                } else {
                    0x14F
                }
            } else if st == 0x14F {
                if r <= 0x18 {
                    -1
                } else {
                    0x155
                }
            } else {
                -1
            };
            if next != -1 {
                self.set_run(i, next);
                if next == 0x13F || next == 0x147 {
                    ctx.sounds.push(SND_SLITHER);
                }
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
        }
        self.advance(i);
    }

    /// `fn113` @581E — class B kind 2.
    fn frame_b2(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            if ctx.rng.pct(100) < 20 {
                ctx.sounds.push(SND_SNIFF);
            }
            let st = self.sprites[i].state_run;
            // quirk: no else arm in the C; +0xAE opens at 0xEA, which is
            // listed, so the fall-through never bites.
            let next = if st == 0xEA || st == 0xEC || st == 0xF3 || st == 0xE1 {
                if r <= 9 {
                    0xEA
                } else if r <= 0x1D {
                    0xEC
                } else if r <= 0x31 {
                    0xF3
                } else {
                    0xE1
                }
            } else {
                -1
            };
            if next != -1 {
                if next == 0xEC || next == 0xF3 {
                    self.play(i, &[next, 0xEA]);
                    ctx.sounds.push(SND_SLITHER);
                } else {
                    self.set_run(i, next);
                }
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
        }
        self.advance(i);
    }

    /// `fn123` @6088 — class C kind 0.
    fn frame_c0(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            let st = self.sprites[i].state_run;
            let mut moan = false;
            let next = if st == 0xFE {
                if r <= 0x20 {
                    -1
                } else if r <= 0x41 {
                    0x10B
                } else {
                    0x124
                }
            } else if st == 0x10B {
                if r <= 0x31 {
                    0x10B
                } else {
                    0x124
                }
            } else if st == 0x124 {
                if r <= 0x20 {
                    moan = true;
                    0xFE
                } else {
                    -1
                }
            } else {
                -1
            };
            if moan {
                ctx.sounds.push(SND_MOAN);
            }
            if next != -1 {
                self.set_run(i, next);
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
        }
        self.advance(i);
    }

    /// `fn124` @61AC — class C kind 1.
    fn frame_c1(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            let st = self.sprites[i].state_run;
            let mut moan = false;
            let next = if st == 0x10A || st == 0x129 || st == 0x12F {
                if r <= 0x13 {
                    -1
                } else if r <= 0x3B {
                    0x10A
                } else if r <= 0x4F {
                    moan = true;
                    0x115
                } else {
                    0x12F
                }
            } else if st == 0x115 || st == 0x122 {
                if r <= 0x1D {
                    -1
                } else if r <= 0x31 {
                    0x11A
                } else {
                    0x129
                }
            } else if st == 0x11A {
                if r <= 0x1D {
                    -1
                } else {
                    0x122
                }
            } else {
                -1
            };
            if moan {
                ctx.sounds.push(SND_MOAN);
            }
            if next != -1 {
                self.set_run(i, next);
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
            if next == 0x10A {
                ctx.sounds.push(SND_KISS);
            }
        }
        self.advance(i);
    }

    /// `fn125` @6324 — class C kind 2.
    fn frame_c2(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            let r = ctx.rng.pct(100) as i32;
            let st = self.sprites[i].state_run;
            let mut moan = false;
            let next = if st == 0xBD {
                if r <= 0x20 {
                    -1
                } else if r <= 0x41 {
                    0xC5
                } else {
                    0xDA
                }
            } else if st == 0xC5 {
                if r <= 0x31 {
                    0xC5
                } else {
                    0xDA
                }
            } else if st == 0xDA {
                if r <= 0x20 {
                    moan = true;
                    0xBD
                } else {
                    -1
                }
            } else {
                -1
            };
            if moan {
                ctx.sounds.push(SND_MOAN);
            }
            if next != -1 {
                self.set_run(i, next);
                self.sprites[i].armed = false;
                self.sprites[i].state_run = next;
            }
        }
        self.advance(i);
    }

    /// `fn134` @687A — class D msg 11.
    fn frame_d11(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due {
            if ctx.rng.pct(100) < 0x33 {
                self.play(i, &[0x169]);
            } else {
                self.play(i, &[0x160]);
            }
            self.sprites[i].armed = false;
            ctx.sounds.push(SND_SPIT);
        }
        self.advance(i);
    }

    /// `fn135` @694A — class D msg 12.
    fn frame_d12(&mut self, ctx: &mut Ctx, i: usize) {
        if self.ready(ctx, i) && ctx.now_ms > self.sprites[i].act_due && ctx.rng.pct(100) < 0x5A {
            self.play(i, &[0x15B, 0x15C]);
            self.sprites[i].armed = false;
        }
        self.advance(i);
    }

    /// The `+0x46 != 0 → arm if needed` preamble every B/C/D machine opens
    /// with (`fn111` @5508, `fn123` @6090, `fn134` @6882).
    fn ready(&mut self, ctx: &mut Ctx, i: usize) -> bool {
        if !self.sprites[i].finished {
            return false;
        }
        if !self.sprites[i].armed {
            self.arm(ctx, i);
            self.sprites[i].armed = true;
        }
        true
    }

    /// `+0x13C` for one sprite.
    fn do_frame(&mut self, ctx: &mut Ctx, i: usize) {
        let (class, kind, msg) = {
            let s = &self.sprites[i];
            (s.class, s.kind, s.msg)
        };
        match class {
            // fn98 @4578
            Class::A => {
                if kind == 2 {
                    self.frame_a_pair(ctx, i)
                } else {
                    self.frame_a_turn(ctx, i)
                }
            }
            // fn110 @54C4
            Class::B => match kind {
                0 => self.frame_b0(ctx, i),
                1 => self.frame_b1(ctx, i),
                _ => self.frame_b2(ctx, i),
            },
            // fn122 @6048
            Class::C => match kind {
                0 => self.frame_c0(ctx, i),
                1 => self.frame_c1(ctx, i),
                _ => self.frame_c2(ctx, i),
            },
            // fn133 @6804
            Class::D => {
                if kind == 0 {
                    if msg == 11 {
                        self.frame_d11(ctx, i)
                    } else {
                        self.frame_d12(ctx, i)
                    }
                } else {
                    if self.sprites[i].finished {
                        let run = self.sprites[i].dir;
                        self.set_run(i, run);
                    }
                    self.advance(i);
                }
            }
        }
    }

    // ---- caption / music ----------------------------------------------------

    /// `fn22` @1EF4 from the ctor @0BC8 — the STR# 128 gag After Dark
    /// paints on the blanked field before the module's first frame.
    fn show_caption(&mut self, ctx: &mut Ctx) {
        let gags = self.pack.strings(CAPTION_STR);
        if gags.is_empty() {
            return;
        }
        let idx = (ctx.rng.pct((gags.len() * 10) as u32) / 10) as usize;
        self.caption = Some(gags[idx.min(gags.len() - 1)].clone());
    }

    /// `fn15` @14BC: replay while the channel is idle and `g03C8` lasts.
    fn music_tick(&mut self, ctx: &mut Ctx) {
        if self.music_count == 0 {
            return;
        }
        if ctx.now_ms >= self.music_next_ok {
            if self.music_count != 99 {
                self.music_count -= 1;
            }
            self.music_plays += 1;
            self.music_next_ok = ctx.now_ms + self.tune_ms();
        }
    }

    fn tune_ms(&self) -> u64 {
        self.pack.song_length_ms(SONG_HORROR_CUE).unwrap_or(31_200)
    }
}

impl Module for FrankenScreen {
    fn name(&self) -> &'static str {
        "FrankenScreen"
    }

    fn controls(&self) -> Vec<ControlDef> {
        vec![
            ControlDef {
                name: "Coherency".into(),
                kind: ControlKind::Popup {
                    base: 0,
                    // MENU 1000, from the pack: five coherency levels.
                    items: self.pack.popup_items(1000, 5, false),
                },
                default: 2,
            },
            ControlDef {
                name: "Blemishes".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 40,
            },
            ControlDef {
                name: "Lifespan".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 0,
            },
            ControlDef {
                name: "Music".into(),
                kind: ControlKind::Slider { min: 0, max: 100 },
                default: 22,
            },
        ]
    }

    fn set_control(&mut self, index: usize, value: i32) {
        match index {
            0 => self.coh = value.clamp(0, 4),
            1 => self.blem = value.clamp(0, 100),
            2 => self.life_raw = value.clamp(0, 100),
            3 => {
                self.music_raw = value.clamp(0, 100);
                self.music_count = music_count_for(self.music_raw);
            }
            _ => {}
        }
    }

    /// `fn15` @131E.
    fn tick(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            self.built_at = ctx.now_ms;
            self.started_at = ctx.now_ms;
            self.build_creature(ctx); // ctor's fn08
            self.show_caption(ctx); // ctor's fn22
            self.need_paint = true; // ctor sets +0x18 = 1
            self.draw_due = ctx.now_ms + DRAW_GATE_MS;
            // The ctor is not a DoDrawFrame: After Dark paints fn22's gag
            // on the blanked field and the module's first frame only
            // arrives on the next pass. That is the black frame the
            // caption owns (measured 167-233 ms across ten Demo starts,
            // of which this is the module's share).
            return;
        }

        // fn14 re-latches the controls every DoDrawFrame; our set_control
        // already keeps them live.
        let now = ctx.now_ms;
        if ctx.caps_lock
            || lifespan_ms(self.life_raw) < now.saturating_sub(self.built_at)
            || HARD_RESET_MS < now.saturating_sub(self.started_at)
        {
            if HARD_RESET_MS < now.saturating_sub(self.started_at) {
                self.hard_reset = true; // GAP(fn19): modelled as a rebuild
                self.started_at = now;
            }
            self.sprites.clear(); // fn09 @0F5A
            self.build_creature(ctx); // fn08 @0E82
            self.need_paint = true;
            self.built_at = now;
        }

        // GAP: fn16/fn17/fn18 — the B/O/C key egg.

        if self.need_paint {
            self.paint_backdrop(ctx); // fn20 @1710
            self.need_paint = false;
            self.hard_reset = false;
            self.painted = true;
            self.caption = None;
        }

        // fn15 @144A — STRICT compare, re-armed now + 100.
        if self.draw_due < now {
            self.pair_tick(ctx); // fn153 @0182
            for i in 0..self.sprites.len() {
                self.do_frame(ctx, i);
            }
            self.draw_due = now + DRAW_GATE_MS;
        }

        self.music_tick(ctx);
    }

    fn sprites(&self, out: &mut Vec<SpriteDraw>) {
        if !self.painted {
            return;
        }
        for st in &self.backdrop {
            let Some(f) = self.pack.frame(BANKS[st.bank], st.id as u32) else { continue };
            out.push(SpriteDraw { flip: false, pal: 0, png: f.png.clone(), x: st.x, y: st.y });
        }
        for s in &self.sprites {
            let Some(f) = self.pack.frame(BANKS[s.bank], s.frame as u32) else { continue };
            let (w, h) = self.size(s.bank, s.frame);
            out.push(SpriteDraw {
                flip: s.flip,
                pal: 0,
                png: f.png.clone(),
                x: s.x - w / 2,
                y: s.y - h / 2,
            });
        }
    }

    fn field(&self) -> [u8; 3] {
        self.pack.meta.field
    }

    fn clock(&self) -> TickClock {
        TickClock::MacTick
    }

    fn music(&self) -> Option<(u32, u32)> {
        (self.music_plays > 0 && self.pack.has_song(SONG_HORROR_CUE))
            .then_some((SONG_HORROR_CUE, self.music_plays))
    }

    fn texts(&self, out: &mut Vec<TextDraw>) {
        if let Some(text) = &self.caption {
            let mut scale = 2;
            while scale > 1 && engine::font::text_width(text, scale) > SCREEN_W - CAPTION_X {
                scale -= 1;
            }
            out.push(TextDraw {
                text: text.clone(),
                x: CAPTION_X,
                y: CAPTION_Y,
                color: engine::contrast_ink(self.field()),
                scale,
            });
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::Pacer;
    use engine::{Random15, RandomLong};
    use std::path::Path;

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

    fn load() -> Option<FrankenScreen> {
        let pack = Pack::load(Path::new("../assets/frankenscreen")).ok()?;
        build(pack)
    }

    #[test]
    fn frankenscreen_smoke() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(1);
        let clock = m.clock();
        let (mut drew, mut sounded, mut caption) = (false, false, false);
        for i in 1..=3000u64 {
            c.now_ms = clock.now_ms(i);
            c.sounds.clear();
            m.tick(&mut c);
            if !c.sounds.is_empty() {
                sounded = true;
            }
            let mut v = Vec::new();
            m.sprites(&mut v);
            if !v.is_empty() {
                drew = true;
            }
            let mut t = Vec::new();
            m.texts(&mut t);
            if !t.is_empty() {
                caption = true;
            }
        }
        assert!(drew, "sprites() never produced output");
        assert!(sounded, "the module never cued a sound");
        assert!(caption, "the fn22 startup caption never armed");
    }

    /// RATCHET 1 — every run id the port names must resolve to a live OFst
    /// block in the bank the C binds that class to. This is the test that
    /// catches a wrong bank: `A_TURNS` kind slot 2 is bank 2000, slot 3/4
    /// are bank 3000, and a slot shuffle puts the sprite on missing art.
    #[test]
    fn every_named_run_ships_in_its_own_bank() {
        let Some(m) = load() else { return };
        let mut checked = 0;
        // class A openings
        for (kind, (bank, open)) in A_KINDS.iter().enumerate() {
            assert!(
                m.g(*bank, *open).is_some(),
                "class-A kind {kind} opens on {open} in bank {} — no record",
                BANKS[*bank]
            );
            checked += 1;
        }
        // class A turn table: kinds 0,1 → bank 0; 3 → bank 1; 4,5 → bank 2
        let slot_bank = [0usize, 0, 1, 2, 2];
        for (p, row) in A_TURNS.iter().enumerate() {
            for (d, per_kind) in row.iter().enumerate() {
                for (slot, &run) in per_kind.iter().enumerate() {
                    let b = slot_bank[slot];
                    assert!(
                        m.g(b, run).is_some(),
                        "A_TURNS[{p}][{d}][{slot}] = {run:#x} missing in bank {}",
                        BANKS[b]
                    );
                    checked += 1;
                }
            }
        }
        // fn102's two poses (kind 2, bank 2000)
        for run in [0x13, 0x07] {
            assert!(m.g(1, run).is_some(), "fn102 run {run:#x} missing in bank 2000");
            checked += 1;
        }
        // class B / C openings and their machines' targets
        for (kind, (bank, open, ae)) in B_KINDS.iter().enumerate() {
            for id in [*open, *ae] {
                assert!(m.g(*bank, id).is_some(), "class-B kind {kind} run {id:#x} missing");
                checked += 1;
            }
        }
        for (kind, (bank, open, ae)) in C_KINDS.iter().enumerate() {
            for id in [*open, *ae] {
                assert!(m.g(*bank, id).is_some(), "class-C kind {kind} run {id:#x} missing");
                checked += 1;
            }
        }
        for (bank, runs) in [
            (0usize, vec![0xC1, 0xC3, 0xD0, 0xE0, 0xEA, 0xF5, 0xDB, 0xE5, 0xEF]), // fn111
            (1, vec![0x13F, 0x147, 0x14F, 0x155]),                                 // fn112
            (2, vec![0xEA, 0xEC, 0xF3, 0xE1]),                                     // fn113
            (0, vec![0xFE, 0x10B, 0x124]),                                         // fn123
            (1, vec![0x10A, 0x115, 0x11A, 0x122, 0x129, 0x12F]),                   // fn124
            (2, vec![0xBD, 0xC5, 0xDA]),                                           // fn125
            (1, vec![0x15B, 0x15C, 0x15E, 0x160, 0x169]),                          // fn134/fn135
        ] {
            for run in runs {
                assert!(
                    m.g(bank, run).is_some(),
                    "machine run {run:#x} missing in bank {}",
                    BANKS[bank]
                );
                checked += 1;
            }
        }
        // class D statics
        for d in &D_SPECS {
            assert!(
                m.g(d.bank, d.run).is_some(),
                "class-D msg {} run {:#x} missing in bank {}",
                d.msg,
                d.run,
                BANKS[d.bank]
            );
            checked += 1;
        }
        // the backdrop: skin, pustules, stitches — each in the bank fn20 uses
        for (bank, tile, family, loop2) in SKINS {
            assert!(m.g(bank, tile).is_some(), "skin {tile} missing in bank {}", BANKS[bank]);
            checked += 1;
            for &id in family {
                assert!(m.g(bank, id).is_some(), "pustule {id} missing in bank {}", BANKS[bank]);
                checked += 1;
            }
            if let Some(id) = loop2 {
                assert!(m.g(bank, id).is_some(), "loop-2 id {id} missing in bank {}", BANKS[bank]);
                checked += 1;
            }
        }
        for id in STITCHES {
            assert!(m.g(1, id).is_some(), "stitch {id} missing in bank 2000");
            checked += 1;
        }
        assert!(checked > 150, "only {checked} ids checked — the table shrank");
    }

    /// RATCHET 2 — `fn24`'s budget must admit exactly the eyes-nose-mouth
    /// quartet the capture shows, and `fn38` must have turned the script's
    /// two 1s into the 5/6 eye pair (or one 7), so the parts land in the
    /// three regions `fn103`/`fn114`/`fn120` name rather than anywhere on
    /// the field.
    #[test]
    fn the_budget_builds_a_face() {
        let Some(m) = load() else { return };
        // The arithmetic claim first: `fn07`'s three bands, and the fact
        // that budget and estimate scale together so the SAME four parts
        // fit at 8, 16 and 32 bpp.
        assert_eq!(budget_for(8), 0x2DE60, "fn07's < 9 bpp band");
        assert_eq!(budget_for(16), 0x591C8, "fn07's 9..16 bpp band");
        assert_eq!(budget_for(32), 720_000, "fn07's > 16 bpp band");
        for depth in [8, 16, 32] {
            let cost = m.mem_estimate_at(&[0, 1, 2], depth) + SIZE_A;
            let fits = budget_for(depth) / cost;
            assert_eq!(
                fits, 4,
                "at {depth} bpp the budget {} fits {fits} parts of {cost}, not the four \
                 eyes-nose-mouth the capture shows",
                budget_for(depth)
            );
        }
        // …then the behaviour: a creature is never more than that quartet,
        // its parts come out of the base script, and the eyes sit inside
        // fn103's band rather than anywhere on the field.
        let clock = m.clock();
        let mut saw_nose = false;
        let mut saw_mouth = false;
        let mut saw_eye = false;
        for seed in 0..32u64 {
            let Some(mut m) = load() else { return };
            let mut c = ctx(seed);
            c.now_ms = clock.now_ms(1);
            m.tick(&mut c);
            let msgs: Vec<i32> = m.sprites.iter().map(|s| s.msg).collect();
            assert!(
                m.sprites.len() <= 4,
                "seed {seed}: the budget let {} parts through: {msgs:?}",
                m.sprites.len()
            );
            assert!(!m.sprites.is_empty(), "seed {seed}: nothing spawned at all");
            saw_nose |= msgs.contains(&2);
            saw_mouth |= msgs.contains(&3);
            saw_eye |= msgs.iter().any(|&v| v == 5 || v == 6 || v == 7);
            let pad = REGION_PAD[m.coherency() - 1];
            for s in &m.sprites {
                let want = match s.msg {
                    5 => Some((30, 20)),
                    6 => Some((70, 20)),
                    7 => Some((50, 20)),
                    2 => Some((50, 45)),
                    3 => Some((50, 80)),
                    _ => None,
                };
                // fn26's grid can move a part into any free cell, so the
                // claim is about the REGION fn103/fn114/fn120 built, which
                // is what fn79 draws the opening position from.
                let Some((px, py)) = want else { continue };
                let (cx, cy) = (px * SCREEN_W / 100, py * SCREEN_H / 100);
                assert_eq!(
                    s.region,
                    (
                        (cx - pad).max(0),
                        (cy - pad).max(0),
                        (cx + pad).min(SCREEN_W),
                        (cy + pad).min(SCREEN_H)
                    ),
                    "seed {seed}: msg {} got the wrong region for ({cx},{cy}) ±{pad}",
                    s.msg
                );
            }
        }
        assert!(saw_nose, "fn24 never spawned the class-B nose (script slot 2)");
        assert!(saw_mouth, "fn24 never spawned the class-C mouth (script slot 3)");
        assert!(saw_eye, "fn38 never turned a script 1 into an eye message");
    }

    /// RATCHET 3 — `fn20`'s skin roll owns both the bank AND the blemish
    /// families: 310/312 come out of bank 1000, 252/260 out of 3000, the
    /// stitches always out of 2000, loop 1 never fires on the green skin,
    /// and loop 2 is a single fixed id that only the orange and purple
    /// skins get. Drive all four rolls through `Pacer`-style ticks.
    #[test]
    fn the_skin_roll_owns_the_bank_and_the_blemishes() {
        let clock = TickClock::MacTick;
        let mut seen = [false; 4];
        for seed in 0..64u64 {
            let Some(mut m) = load() else { return };
            let mut c = ctx(seed);
            for i in 1..=2u64 {
                c.now_ms = clock.now_ms(i);
                m.tick(&mut c);
            }
            let (bank, tile, family, loop2) = SKINS[m.skin];
            seen[m.skin] = true;
            // every tile stamp is the rolled skin out of the rolled bank
            let tiles: Vec<&Stamp> = m.backdrop.iter().filter(|s| s.id == tile).collect();
            assert!(!tiles.is_empty(), "the field was never tiled");
            // Non-tautological half: every stamp must be a LIVE record in
            // the bank fn20 drew it from. 310/312 only exist in 1000,
            // 252/260/254..270 only in 3000, 370..386 only in 2000, so a
            // family pointed at the wrong skin dies here.
            // ...and every stamp must be the RIGHT art. A bare id lives
            // in several banks as completely different pictures, so the
            // size is the check: the skin tile is 132x119 in exactly one
            // bank, and a pustule is a 12-38 px sphere in exactly one
            // (it is a 122-173 px maw in the other two).
            for st in &m.backdrop {
                let g = m.g(st.bank, st.id);
                assert!(
                    g.is_some(),
                    "backdrop stamped {} out of bank {} — no OFst record",
                    st.id,
                    BANKS[st.bank]
                );
                let (w, h) = m.size(st.bank, st.id);
                if st.id == tile {
                    assert_eq!(
                        (w, h),
                        (132, 119),
                        "skin {tile} out of bank {} is {w}x{h}, not the 132x119 tile",
                        BANKS[st.bank]
                    );
                } else if !STITCHES.contains(&st.id) {
                    assert!(
                        w <= 40 && h <= 40,
                        "blemish {} out of bank {} is {w}x{h} — that is a maw, not a zit",
                        st.id,
                        BANKS[st.bank]
                    );
                }
            }
            assert!(
                tiles.iter().all(|s| s.bank == bank),
                "skin {tile} must come out of bank {}",
                BANKS[bank]
            );
            for st in &m.backdrop {
                if STITCHES.contains(&st.id) {
                    assert_eq!(st.bank, 1, "stitch {} must come out of bank 2000", st.id);
                } else if st.id != tile {
                    assert!(
                        family.contains(&st.id) || Some(st.id) == loop2,
                        "skin {tile} stamped a blemish {} that is not in its family",
                        st.id
                    );
                    assert_eq!(st.bank, bank, "blemish {} left the skin's bank", st.id);
                }
            }
            if family.is_empty() {
                assert!(
                    m.backdrop.iter().all(|s| s.id == tile || STITCHES.contains(&s.id)),
                    "the green skin (312) has no loop-1 family, so only stitches may join it"
                );
            }
            if loop2.is_none() {
                assert!(
                    m.backdrop.iter().all(|s| s.id != 268 && s.id != 270),
                    "loop 2 fired on a skin that has no loop-2 id"
                );
            }
        }
        assert!(seen.iter().filter(|&&v| v).count() >= 3, "the skin roll barely moved");
    }

    /// The draw gate is `g01BA < now`, re-armed `now + 100`, on the
    /// truncated `TickCount()*16.625` clock — 7 Mac ticks flat.
    #[test]
    fn the_draw_gate_is_seven_mac_ticks() {
        let Some(mut m) = load() else { return };
        assert_eq!(m.clock(), TickClock::MacTick);
        let clock = m.clock();
        let mut c = ctx(3);
        let mut fired: Vec<u64> = Vec::new();
        let mut last = u64::MAX;
        for i in 1..=400u64 {
            c.now_ms = clock.now_ms(i);
            m.tick(&mut c);
            if m.draw_due != last {
                last = m.draw_due;
                fired.push(i);
            }
        }
        assert!(fired.len() > 20, "the draw gate never ran");
        let gaps: Vec<u64> = fired[1..].windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|&g| g == 7), "the draw gate must fire every 7 ticks: {gaps:?}");
    }

    /// The whole machine gets around: over a long run every class visits
    /// more than its opening pose and nothing is stuck.
    #[test]
    fn the_machine_gets_around() {
        let Some(mut m) = load() else { return };
        let clock = m.clock();
        let mut c = ctx(0xBEEF);
        // Interminable, so one creature lives through the whole run.
        m.set_control(2, 90);
        let mut frames: HashMap<(Class, usize), std::collections::HashSet<i32>> = HashMap::new();
        let mut builds = 0usize;
        let mut prev = u64::MAX;
        for i in 1..=60_000u64 {
            c.now_ms = clock.now_ms(i);
            c.sounds.clear();
            m.tick(&mut c);
            if m.built_at != prev {
                prev = m.built_at;
                builds += 1;
            }
            for (k, s) in m.sprites.iter().enumerate() {
                frames.entry((s.class, k)).or_default().insert(s.frame);
            }
        }
        assert_eq!(builds, 1, "Interminable must build once in 60 000 ticks, got {builds}");
        assert!(!frames.is_empty(), "no sprites at all");
        let moved = frames.values().filter(|v| v.len() > 1).count();
        assert!(
            moved >= frames.len() - 1,
            "only {moved} of {} parts ever left their opening pose",
            frames.len()
        );
    }

    /// `fn15`'s Lifespan branch really expires and rebuilds, and a longer
    /// band really defers it.
    #[test]
    fn lifespan_expiry_rebuilds_the_creature() {
        let Some(mut m) = load() else { return };
        let clock = m.clock();
        let mut c = ctx(0x5EED);
        assert_eq!(lifespan_ms(m.life_raw), 30_000);
        let ticks = 20 * 60 * 1_000_000 / clock.period_us();
        let mut builds = 0usize;
        let mut prev = u64::MAX;
        for i in 1..=ticks {
            c.now_ms = clock.now_ms(i);
            m.tick(&mut c);
            if m.built_at != prev {
                prev = m.built_at;
                builds += 1;
            }
        }
        assert!((39..=42).contains(&builds), "expected ~41 builds in 20 min, got {builds}");

        let Some(mut m) = load() else { return };
        m.set_control(2, 90);
        let mut c = ctx(0x5EED);
        let mut builds = 0usize;
        let mut prev = u64::MAX;
        for i in 1..=ticks {
            c.now_ms = clock.now_ms(i);
            m.tick(&mut c);
            if m.built_at != prev {
                prev = m.built_at;
                builds += 1;
            }
        }
        assert_eq!(builds, 1, "Interminable must not rebuild inside 20 min");
    }

    /// A part's rect on the screen under the sprite's flip (`fn3BD6`).
    fn part_screen(m: &FrankenScreen, s: &Sprite, part: &[i32; 7]) -> [i32; 4] {
        let r = m.part_rel(s.bank, s.frame, part, s.flip).expect("part of a live frame");
        [r[0] + s.x, r[1] + s.y, r[2] + s.x, r[3] + s.y]
    }

    /// The first part `cur`'s and `new`'s tables share, as the two records.
    fn first_shared(m: &FrankenScreen, bank: usize, cur: i32, new: i32) -> Option<([i32; 7], [i32; 7])> {
        let a = m.pack.frame(BANKS[bank], cur as u32)?;
        let b = m.pack.frame(BANKS[bank], new as u32)?;
        a.parts.iter().find_map(|pa| b.parts.iter().find(|pb| pb[0] == pa[0]).map(|pb| (*pa, *pb)))
    }

    /// RATCHET 4 — the run hand-off is L132 `fn028A` → `fn0D3C` → L135
    /// `fn3F2E` (every class vtable binds `+0x7C` = `fn0204`, `+0x108` =
    /// `fn028A`): the first part the outgoing and incoming frames share
    /// stays where it was on screen; with no shared part the sprite does
    /// not move. The superseded `centre(new) − centre(cur)` model fails
    /// this on every pair picked here. (In this pack the machines' own
    /// hand-offs never hit such a pair — `frankenscreen_handoff_census` —
    /// so this pins the mechanism, not a visible change.)
    #[test]
    fn a_hand_off_keeps_the_shared_part_on_screen() {
        let Some(mut m) = load() else { return };
        let mut c = ctx(1);
        c.now_ms = m.clock().now_ms(1);
        m.tick(&mut c);
        assert!(!m.sprites.is_empty());
        // pick (cur, run) pairs, run a block start with no lead-in record,
        // where the two models disagree
        let mut shared: Vec<(usize, i32, i32)> = Vec::new();
        let mut unshared: Vec<(usize, i32, i32)> = Vec::new();
        for b in 0..3 {
            let firsts: Vec<i32> = m.pack.series(BANKS[b]).iter().map(|q| q.first as i32).collect();
            for &cur in &firsts {
                for &to in &firsts {
                    if cur == to || m.marker_of(b, to) != to {
                        continue;
                    }
                    let (dx, dy, _, art) = m.shared_link(b, cur, to, false);
                    let old = centre_link(&m, b, cur, to, false);
                    if (dx, dy) == old {
                        continue;
                    }
                    if art >= 0 && shared.len() < 200 {
                        shared.push((b, cur, to));
                    } else if art < 0 && unshared.len() < 50 {
                        unshared.push((b, cur, to));
                    }
                }
            }
        }
        assert!(shared.len() >= 50, "only {} disagreeing shared-part pairs", shared.len());
        assert!(!unshared.is_empty(), "no disagreeing unshared pair");
        for &(b, cur, to) in &shared {
            for flip in [false, true] {
                m.sprites[0].bank = b;
                m.sprites[0].flip = flip;
                m.set_frame(0, cur);
                m.sprites[0].x = 300;
                m.sprites[0].y = 200;
                let (pa, pb) = first_shared(&m, b, cur, to).unwrap();
                let before = part_screen(&m, &m.sprites[0], &pa);
                m.set_run(0, to);
                let after = part_screen(&m, &m.sprites[0], &pb);
                assert_eq!(
                    before, after,
                    "bank {} {cur} -> run {to} (flip {flip}): shared art {} moved",
                    BANKS[b], pa[0]
                );
            }
        }
        for &(b, cur, to) in &unshared {
            m.sprites[0].bank = b;
            m.sprites[0].flip = false;
            m.set_frame(0, cur);
            m.sprites[0].x = 300;
            m.sprites[0].y = 200;
            m.set_run(0, to);
            assert_eq!(
                (m.sprites[0].x, m.sprites[0].y),
                (300, 200),
                "bank {} {cur} -> run {to}: no shared part, yet the sprite moved",
                BANKS[b]
            );
        }
    }

    /// RATCHET 5 — the "nose creep" is not a creep. The drip runs (bank
    /// 1000 kind 0: 195..200; bank 3000 kind 2: 239..241 / 246..248) grow
    /// their frame DOWNWARD from a fixed top, so `pos` (the frame centre)
    /// dips 27 / 29 px and the hand-off back to the rest pose lifts it
    /// again; the drawn top-left never moves. Capture
    /// (`frankenscreen.mp4`, masked template match at 5 fps): the bank-1000
    /// nose sits at top-left (455, 327) in all 135 samples of its 0–27 s
    /// life and the bank-3000 nose at (426, 30) in all 135 of its,
    /// drip frames 236..246 included — 0 px drift. Pinned here as: every
    /// drawn top-left of a class-B part over a whole Interminable life is
    /// one point, and its centre never ends a life displaced.
    #[test]
    fn the_nose_holds_its_top_across_a_life() {
        let mut kinds_seen = [false; 3];
        for seed in 0..40u64 {
            let Some(mut m) = load() else { return };
            m.set_control(2, 90);
            let mut c = ctx(seed);
            let mut pacer = Pacer::new(&m);
            let mut tops: HashMap<usize, std::collections::HashSet<(i32, i32)>> = HashMap::new();
            let mut start: HashMap<usize, (i32, i32, i32)> = HashMap::new();
            let mut hand_offs = 0usize;
            let mut prev: Vec<Sprite> = Vec::new();
            while pacer.now_ms() < 300_000 {
                pacer.advance(&mut c);
                m.tick(&mut c);
                for (k, s) in m.sprites.iter().enumerate() {
                    if s.class != Class::B {
                        continue;
                    }
                    let (w, h) = m.size(s.bank, s.frame);
                    tops.entry(k).or_default().insert((s.x - w / 2, s.y - h / 2));
                    start.entry(k).or_insert((s.frame, s.x, s.y));
                    if let Some(p) = prev.get(k) {
                        if p.first != s.first && p.frame != 0 {
                            hand_offs += 1;
                        }
                    }
                }
                prev = m.sprites.clone();
            }
            for (k, t) in &tops {
                let s = &m.sprites[*k];
                kinds_seen[s.kind as usize] = true;
                assert_eq!(
                    t.len(),
                    1,
                    "seed {seed}: class-B kind {} (bank {}) drew at {} top-lefts: {t:?}",
                    s.kind,
                    BANKS[s.bank],
                    t.len()
                );
                // back on the rest frame, the centre is where it started
                let (f0, x0, y0) = start[k];
                if s.frame == f0 {
                    assert_eq!((s.x, s.y), (x0, y0), "seed {seed}: the nose ended its life displaced");
                }
            }
            assert!(hand_offs > 0 || tops.is_empty(), "seed {seed}: the nose never handed off");
        }
        assert!(kinds_seen.iter().all(|&k| k), "not every nose kind came up: {kinds_seen:?}");
    }

    /// The eye pair turns together: `fn153`/`fn152` only let a member act
    /// when the target differs and nobody is mid-run.
    #[test]
    fn the_eye_pair_moves_in_step() {
        let Some(mut m) = load() else { return };
        let clock = m.clock();
        let mut c = ctx(0xEEEE);
        for i in 1..=4000u64 {
            c.now_ms = clock.now_ms(i);
            m.tick(&mut c);
            if m.pair.member_a.is_some() && m.pair.member_b.is_some() {
                // a member may only be mid-run while the pair says pending
                let a = m.pair.member_a.unwrap();
                let b = m.pair.member_b.unwrap();
                if !m.sprites[a].finished || !m.sprites[b].finished {
                    assert!(
                        m.pair.pending || m.pair.cur != m.pair.target,
                        "an eye ran with the pair idle at tick {i}"
                    );
                }
            }
        }
    }

    /// `#[ignore]`d census: the 120 s sound mix, to compare against
    /// `docs/emulator/audio-captures.md`'s FrankenScreen timeline
    /// (moan2 x12, drip_slither x7, kiss x4, Sniff x4; 30002 and 30004
    /// never fire).
    #[test]
    #[ignore]
    fn census_frankenscreen_sounds() {
        let Some(mut m) = load() else { return };
        let clock = m.clock();
        for seed in 0..4u64 {
            let Some(mut m2) = load() else { return };
            std::mem::swap(&mut m, &mut m2);
            let mut c = ctx(seed);
            let mut tally: HashMap<u32, usize> = HashMap::new();
            let ticks = 120 * 1_000_000 / clock.period_us();
            for i in 1..=ticks {
                c.now_ms = clock.now_ms(i);
                c.sounds.clear();
                m.tick(&mut c);
                for &s in &c.sounds {
                    *tally.entry(s).or_default() += 1;
                }
            }
            let mut v: Vec<_> = tally.into_iter().collect();
            v.sort();
            println!("seed {seed}: {v:?}");
        }
    }

    /// `#[ignore]`d trace: one line per transition.
    #[test]
    #[ignore]
    fn trace_frankenscreen() {
        let Some(mut m) = load() else { return };
        let clock = m.clock();
        let mut c = ctx(7);
        let mut last: Vec<(i32, i32, i32)> = Vec::new();
        for i in 1..=6000u64 {
            c.now_ms = clock.now_ms(i);
            c.sounds.clear();
            m.tick(&mut c);
            let cur: Vec<(i32, i32, i32)> =
                m.sprites.iter().map(|s| (s.frame, s.x, s.y)).collect();
            if cur != last {
                for (k, s) in m.sprites.iter().enumerate() {
                    println!(
                        "t={:6} #{k} {:?} msg={} kind={} bank={} f={} run={} pos=({},{}) fin={}",
                        c.now_ms, s.class, s.msg, s.kind, BANKS[s.bank], s.frame, s.state_run,
                        s.x, s.y, s.finished
                    );
                }
                println!("---");
                last = cur;
            }
            if !c.sounds.is_empty() {
                println!("t={:6} snd {:?}", c.now_ms, c.sounds);
            }
        }
    }

    /// What the linked `fn028A` hand-off does from `cur` to run `id`:
    /// `fn3F2E` onto the marker, then `fn3DDC` marker → `id`. Returns
    /// `(dx, dy, flip', shared art id)`.
    fn linked_handoff(m: &FrankenScreen, bank: usize, cur: i32, id: i32, flip: bool) -> (i32, i32, bool, i32) {
        let mk = m.marker_of(bank, id);
        let (dx, dy, f2, art) = m.shared_link(bank, cur, mk, flip);
        let (lx, ly) = m.link(bank, mk, id, f2);
        (dx + lx, dy + ly, f2, art)
    }

    /// The run ids each machine hands between (opening pose first), grouped
    /// per machine: a part only ever hands off inside its own group.
    fn machine_groups() -> Vec<(&'static str, usize, Vec<i32>)> {
        let mut v: Vec<(&'static str, usize, Vec<i32>)> = Vec::new();
        let slot_bank = [0usize, 0, 1, 2, 2];
        let slot_open = [3, 121, 28, 9, 108];
        for slot in 0..5 {
            let mut runs = vec![slot_open[slot]];
            for row in A_TURNS.iter() {
                for per_kind in row.iter() {
                    runs.push(per_kind[slot]);
                }
            }
            v.push(("A fn101", slot_bank[slot], runs));
        }
        v.push(("A fn102", 1, vec![0x13, 0x07]));
        v.push(("B fn111", 0, vec![0xC1, 0xEF, 0xC3, 0xD0, 0xE0, 0xEA, 0xF5, 0xDB, 0xE5]));
        v.push(("B fn112", 1, vec![0x14F, 0x155, 0x13F, 0x147]));
        v.push(("B fn113", 2, vec![0xEA, 0xEC, 0xF3, 0xE1]));
        v.push(("C fn123", 0, vec![0xFE, 0x10B, 0x124]));
        v.push(("C fn124", 1, vec![0x115, 0x129, 0x10A, 0x11A, 0x122, 0x12F]));
        v.push(("C fn125", 2, vec![0xBD, 0xDA, 0xC5]));
        v.push(("D fn134", 1, vec![0x15E, 0x169, 0x160]));
        v.push(("D fn135", 1, vec![0x15C, 0x15B]));
        for d in &D_SPECS {
            if d.kind == 1 {
                v.push(("D static", d.bank, vec![d.run]));
            }
        }
        v
    }

    /// `#[ignore]`d exhaustive census: every frame a machine's runs can end
    /// on (or open with) × every run the same machine can start, both
    /// flips — the centre-difference hand-off vs the linked `fn3F2E` one.
    #[test]
    #[ignore]
    fn frankenscreen_handoff_census() {
        let Some(m) = load() else { return };
        let (mut n, mut diff, mut flips, mut worst, mut unshared) = (0usize, 0usize, 0usize, 0i32, 0usize);
        for (name, b, runs) in machine_groups() {
            for cur in runs.iter().flat_map(|&r| r..=m.last_of(b, r)) {
                for &to in &runs {
                    for flip in [false, true] {
                        let (ox, oy) = centre_link(&m, b, cur, to, flip);
                        let (lx, ly, lf, art) = linked_handoff(&m, b, cur, to, flip);
                        n += 1;
                        let e = (ox - lx).abs().max((oy - ly).abs());
                        unshared += (art < 0) as usize;
                        if lf != flip {
                            flips += 1;
                        }
                        if e != 0 || lf != flip {
                            diff += 1;
                            worst = worst.max(e);
                            if diff <= 40 {
                                println!(
                                    "{name} bank {} {cur:4} -> {to:4} flip {} : centre ({ox},{oy}) linked ({lx},{ly}) flip' {} art {art}",
                                    BANKS[b], flip as u8, lf as u8
                                );
                            }
                        }
                    }
                }
            }
        }
        println!("census: {n} hand-offs, {unshared} share no part, {diff} differ, {flips} flip, worst {worst} px");
        // in-run steps (and the loop-back) under a flip: fn3DDC's rounding
        let (mut steps, mut sdiff) = (0usize, 0usize);
        for (name, b, runs) in machine_groups() {
            for &r in &runs {
                let last = m.last_of(b, r);
                for f in r..last {
                    for flip in [false, true] {
                        steps += 1;
                        if m.link(b, f, f + 1, flip) != centre_link(&m, b, f, f + 1, flip) {
                            sdiff += 1;
                            if sdiff <= 10 {
                                println!("{name} bank {} step {f} -> {} flip {}", BANKS[b], f + 1, flip as u8);
                            }
                        }
                    }
                }
            }
        }
        println!("census: {steps} in-run steps, {sdiff} differ (all flipped)");
    }

    /// The superseded model: `centre(to) − centre(from)`, dx negated flipped.
    fn centre_link(m: &FrankenScreen, bank: usize, from: i32, to: i32, flip: bool) -> (i32, i32) {
        match (m.centre(bank, from), m.centre(bank, to)) {
            (Some(a), Some(b)) => (if flip { a.0 - b.0 } else { b.0 - a.0 }, b.1 - a.1),
            _ => (0, 0),
        }
    }

    /// `#[ignore]`d per-tick trace of every run hand-off (the L132 `fn028A`
    /// bug class): tick, sprite, class/kind, frame → run, pos, the delta the
    /// port applied, the linked `fn3F2E` delta, flip, shared part art id —
    /// plus each part's net travel over one Interminable life.
    /// `cargo test -p app frankenscreen_handoff_trace -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn frankenscreen_handoff_trace() {
        for (seed, life) in [(7u64, 90), (0xBEEF, 90), (0x5EED, 0)] {
            let Some(mut m) = load() else { return };
            m.set_control(2, life);
            let mut c = ctx(seed);
            let mut pacer = Pacer::new(&m);
            let (mut n, mut off, mut worst, mut flips) = (0usize, 0usize, 0i32, 0usize);
            let mut built = u64::MAX;
            let mut origin: Vec<(i32, i32)> = Vec::new();
            let mut span: Vec<(i32, i32, i32, i32)> = Vec::new();
            let mut lines = 0usize;
            while pacer.now_ms() < 600_000 {
                let before = m.sprites.clone();
                pacer.advance(&mut c);
                c.sounds.clear();
                m.tick(&mut c);
                if m.built_at != built {
                    built = m.built_at;
                    origin = m.sprites.iter().map(|s| (s.x, s.y)).collect();
                    span = m.sprites.iter().map(|s| (s.x, s.y, s.x, s.y)).collect();
                    continue;
                }
                for (k, (a, b)) in before.iter().zip(m.sprites.iter()).enumerate() {
                    let sp = &mut span[k];
                    *sp = (sp.0.min(b.x), sp.1.min(b.y), sp.2.max(b.x), sp.3.max(b.y));
                    let step = b.frame == a.frame + 1 && b.first == a.first;
                    if b.frame == a.frame || step || a.frame == 0 {
                        continue;
                    }
                    let (lx, ly, lf, art) = linked_handoff(&m, a.bank, a.frame, b.first, a.flip);
                    let (ax, ay) = (b.x - a.x, b.y - a.y);
                    let err = (ax - lx).abs().max((ay - ly).abs());
                    n += 1;
                    if err != 0 || lf != b.flip {
                        off += 1;
                    }
                    if lf != a.flip {
                        flips += 1;
                    }
                    worst = worst.max(err);
                    if lines < 60 {
                        lines += 1;
                        println!(
                            "t={:7} #{k} {:?}{} {:4} -> run {:4} (mk {:4}) pos ({:4},{:4})->({:4},{:4}) applied ({:4},{:4}) linked ({:4},{:4}) flip {}->{} (linked {}) art {}",
                            c.now_ms, b.class, b.kind, a.frame, b.first, m.marker_of(a.bank, b.first),
                            a.x, a.y, b.x, b.y, ax, ay, lx, ly, a.flip as u8, b.flip as u8, lf as u8, art
                        );
                    }
                }
            }
            println!("seed {seed:#x} life {life}: hand-offs {n}, off-model {off}, worst |applied−linked| {worst} px, linked flips {flips}");
            for (k, s) in m.sprites.iter().enumerate() {
                let o = origin.get(k).copied().unwrap_or((0, 0));
                let sp = span.get(k).copied().unwrap_or((0, 0, 0, 0));
                println!(
                    "  #{k} {:?}{} bank {} net ({},{}) span x {}..{} y {}..{}",
                    s.class, s.kind, BANKS[s.bank], s.x - o.0, s.y - o.1, sp.0 - o.0, sp.2 - o.0, sp.1 - o.1, sp.3 - o.1
                );
            }
        }
    }
}
