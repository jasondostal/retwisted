# The macOS screensaver

`tools/build_saver.sh [slug] [--install]` turns **every** retwisted module
into one real `.saver` bundle, installed under `~/Library/Screen Savers/`
and selectable in System Settings. The module that runs is picked in the
Options… sheet; the positional slug is only the *default* selection, for a
machine with nothing stored. Default: `bungee-roulette`. Default bundle
name: `Retwisted` (working title — `BUNDLE_NAME=Foo` overrides it, and the
script has the name in one variable).

```
tools/build_saver.sh                 # -> target/saver/Retwisted.saver
tools/build_saver.sh --install       # ... and into ~/Library/Screen Savers/
tools/build_saver.sh shock-clocks --install    # same 13 modules, different default
```

All thirteen packs ride along in `Contents/Resources/assets/<slug>/`, which
is what makes the bundle **91 MB** (83 MB of it art and sound). The script
prints how many packs it shipped and names any catalogued module it could
not find a local pack for. The sheet now says the same thing at the other
end: such a module is listed in the panel's module list as "Name (no pack)",
dimmed, and cannot be chosen, and if it was the *default* the view falls through to the
first module that does have art.

## First run: the pack-less bundle (2026-09-29)

Distribution is bring-your-own-originals, ScummVM-style: the shippable
bundle carries **no Berkeley art** and rips the user's own download on first
run, in-process, with the Rust ripper (`ripper/`, see `docs/ripper.md`).

```
tools/build_saver.sh --no-packs      # the shippable bundle, 3.6 MB; fails if a pack file leaks in
BUNDLE_NAME="Retwisted First Run" BUNDLE_ID=com.retwisted.saver.firstrun \
    tools/build_saver.sh --no-packs  # side by side with an installed dev build
```

A `BUNDLE_NAME` other than `Retwisted` also gets its own Objective-C
principal class (`<Name>SaverView`, patched into a copy of the source) and
Swift module (`<Name>Saver`): legacyScreenSaver loads every saver it shows
into ONE process, and two bundles both defining `RetwistedSaverView` would
collide (one class wins, arbitrarily). Bundle id → its own
`ScreenSaverDefaults` and its own packs dir, so a side-by-side build starts
genuinely empty and cannot touch the installed one's settings.

**Where packs live — `PackStore`, the one place it is decided.** Per module,
the first root that has the pack:

1. `<Application Support>/<bundle id>/assets` — what the rip writes.
   `FileManager` resolves Application Support per process, so in the host it
   is the appex's container:
   `~/Library/Containers/com.apple.ScreenSaver.Engine.legacyScreenSaver/Data/Library/Application Support/com.retwisted.saver/assets`;
   for the unsandboxed headless tools it is
   `~/Library/Application Support/com.retwisted.saver/assets`.
   `RTW_PACKS_DIR` overrides it (tools/tests only; the host never sets it).
2. `Contents/Resources/assets` — the dev build's bundled packs (the default
   `build_saver.sh` keeps bundling them, so the dev/install flow is unchanged).

Per module rather than per root, so a one-module rip next to a full dev
bundle adds instead of hiding twelve modules.

**The rip ABI** (`saver/src/rip.rs`, header section "first-run ripping"):

| call | does |
| --- | --- |
| `rtw_rip_start(input, assets_root)` | start ripping on a worker thread; NULL for a NULL/empty argument |
| `rtw_rip_poll(job, &step, &total)` | `RTW_RIP_RUNNING` 0 / `_DONE` 1 / `_FAILED` 2, plus progress |
| `rtw_rip_message(job)` | progress line / one-line summary / human-worded error (one scratch slot) |
| `rtw_rip_module_count(job)` | packs a DONE job installed |
| `rtw_rip_free(job)` | release; on a RUNNING job = cancel (it finishes, then deletes its temp dir, installs nothing) |

Poll-based because `twistedrip::rip`'s progress callback fires on the thread
running the rip; the worker only writes a shared state and the panel polls
it from a 0.1 s main-run-loop timer (`.common` modes). **A half-rip never
looks like a pack:** the rip goes to `<root>/../.rip-<pid>-…`, a sibling, and
each finished `<slug>/` and `_shared/` then moves in with one rename
(`renamex_np(RENAME_SWAP)` when replacing, so the old pack is complete until
the instant the new one is). Leftover `.rip-*` dirs older than an hour are
swept on the next start. An input that unpacks but holds no modules (the
Flying Toilets demo) fails rather than "succeeding" with nothing. Error
wording lives in Rust (`rip::human_error`): "That file isn't a Totally
Twisted download…", "…uses a compression method the ripper doesn't support…
expand it with The Unarchiver and choose the folder", "…password-protected",
"…looks damaged or incomplete", "…wasn't allowed to read or write…".

**The panel.** No packs: every module listed dimmed, none selected (no
"(no pack)" suffix — fourteen of them said less than the empty state does);
the right column is **Welcome** + what to do + a default **Locate…** button;
the help pane says the originals are usually found on Macintosh Garden
(floppy `After_Dark_-_Totally_Twisted.sit`, hybrid CD `totallytwistedcd.sit`)
and names the drop folder; OK writes nothing. Locate… → `NSOpenPanel` as a
sheet on the panel (files `.sit/.hqx/.iso/.bin` + folders, starts in
Downloads) → the column becomes **Reading your files…** with a 1-bit
progress bar and the ripper's current step; OK disabled; Cancel/close cancels
the job. Done → packs re-read, the list lights up, the shown module becomes
selectable (or the first ripped one, if the fallback was not in the input),
the summary heads the help pane, and every view in the host is told
(`settingsChanged`) — a pack-less view builds its runtime on its next frame
and cycles its timer onto the module's rate. The packs are installed whatever
button then closes the panel. Failed → **That didn't work** + the message +
Locate… again; clicking a module (when there are packs) returns to its
controls. With packs present, **Locate…** sits beside Defaults to re-rip or
replace.

**The saver with no packs** draws "Open Options to locate your Totally
Twisted files", grey Geneva centred on black, silent (no runtime exists), at
a 1 s frame interval.

### Sandbox: what is known, what only the real host can prove

Known headlessly: the appex holds `files.user-selected.read-only` — exactly
the entitlement that makes a sandboxed process's `NSOpenPanel` the powerbox
and grants read access to what the user picks (the panel also calls
`startAccessingSecurityScopedResource` for the life of the job) — and a
read-only exception for `/`; its container's Application Support is its own
to write. The whole flow minus the panel itself is proven by
`saver_sheet_shot --locate` (below).

Only the real host can prove: that the remote open panel actually appears as
a sheet on an Options sheet inside `legacyScreenSaver` (a ViewBridge-hosted
parent) and comes back with a URL; that the rip then reads a file in
`~/Downloads` (TCC-protected); and the container path above. Fallbacks, in
order: **Use Shared Folder** — the panel offers the first `.sit/.hqx/.iso/.bin`
(or sub-folder) in `/Users/Shared/Retwisted/`, which is inside the `/`
read exception and outside TCC's protected folders; or the CLI straight into
the container (Terminal may ask for "access data from other apps"):

```
cargo run -p ripper --release -- After_Dark_-_Totally_Twisted.sit --out \
  "$HOME/Library/Containers/com.apple.ScreenSaver.Engine.legacyScreenSaver/Data/Library/Application Support/com.retwisted.saver/assets"
```

Headless verification (a temp packs dir keeps it out of `~/Library`):

```
export RTW_PACKS_DIR=$(mktemp -d)/assets
xcrun swift tools/saver_shot.swift "target/saver/Retwisted First Run.saver" /tmp/empty.png 5
xcrun swift tools/saver_sheet_shot.swift "target/saver/Retwisted First Run.saver" /tmp/sheet.png \
    --locate ~/working/totally-twisted/original/totallytwistedcd.sit --module mowin-boris \
    --live /tmp/live.png --frames 150
```

Results 2026-09-29: the empty view draws the note; the empty sheet lists 14
dimmed rows with the Welcome column and Locate…; `--locate` of the CD `.sit`
ran `0/15 inflating CD image … 13/15 voyeur` → `done All 13 modules are
ready.` in 3.8 s (floppy 3.1 s), the list lit up with the rip's own
faceplate, and the vending (pack-less) view came up running bungee at the
Mac-tick rate; a junk file → `failed That file isn't a Totally Twisted
download…`, nothing installed, no temp left. `saver/tests/rip.rs` drives the
ABI: floppy rip vs `../assets` (every file, parity rules), re-rip over the
top, refusals, cancel, NULLs.

## Shape of it

```
app/src/lib.rs          modules + compose()  ── the one draw path
   ├── app/src/main.rs            winit player
   ├── app/src/bin/frame.rs       headless PNG renderer
   └── saver/src/lib.rs           C ABI (rtw_*) ── staticlib
          └── saver/macos/RetwistedSaverView.swift   ScreenSaverView
```

The compose pass (field clear → rects → sprites with the palette remap →
texts → 640×480 0RGB) used to be copy-pasted in `main.rs` and `bin/frame.rs`.
It now lives in `app::compose` and all three front ends call it, so the
screensaver draws exactly what the golden-frame tool draws — verified: after
the refactor `frame` produces byte-identical PNGs for bungee-roulette,
chameleon, shock-clocks and message-mayhem.

### The C ABI

`saver/include/retwisted_saver.h` is the contract; `saver/src/lib.rs`
implements it; `saver/tests/abi.rs` drives it for 600 frames and asserts the
field gets painted over, no top byte is ever set, sounds drain, and nothing
panics. Since the bundle can run any of the thirteen it also walks the whole
catalogue: `rtw_create` from the assets **root** for every slug, 100 ticks
of every module with its defaults applied, and a destroy/create chain in the
shape of the sheet's OK button (`abi_runtimes_can_be_swapped_one_for_another`).

| call | does |
| --- | --- |
| `rtw_module_count()` | how many modules this build has (13) |
| `rtw_module_slug(i)` | module `i`'s slug — the `rtw_create` argument and the settings-key namespace |
| `rtw_module_name(i)` | module `i`'s display name: the original's title ("Mowin' Boris") |
| `rtw_module_index(slug)` | that slug's index, or -1 — validates a slug read back out of the plist |
| `rtw_pack_exists(slug, assets_dir)` | does this bundle carry that module's art? one `stat`, no runtime |
| `rtw_create(slug, assets_dir)` | build a runtime; `assets_dir` may be the pack dir or a root containing `<slug>/` |
| `rtw_destroy(rt)` | free it |
| `rtw_set_control(rt, i, v)` | set control `i` to the module's own raw value |
| `rtw_control_count(rt)` | how many controls the module has (0 ⇒ no Options… button) |
| `rtw_control_kind(rt, i)` | 0 slider, 1 popup, 2 checkbox; -1 for a bad index |
| `rtw_control_min/max(rt, i)` | inclusive raw range (checkbox 0/1; popup: see below) |
| `rtw_control_default(rt, i)` | the module's factory value |
| `rtw_control_item_count(rt, i)` | popup items (0 for the other kinds) |
| `rtw_control_name(rt, i)` | control label, UTF-8 |
| `rtw_control_item(rt, i, n)` | popup item `n`'s label; it is the raw value `min + n` |
| `rtw_control_band_count(rt, i)` | how many `sUnt` words a slider has (0 for anything else) |
| `rtw_control_band_value(rt, i, b)` | the raw value word `b` sits at, ascending |
| `rtw_control_band_label(rt, i, b)` | word `b` — "One", "Hundreds!" |
| `rtw_control_band_for(rt, i, v)` | which word raw `v` falls under; -1 when there are none |
| `rtw_tick_ms(rt)` | tick period in ms, rounded DOWN (16 on the Mac tick) |
| `rtw_tick_us(rt)` | tick period in µs → `animationTimeInterval` |
| `rtw_tick(rt, now_ms, h, m, s, mx, my, down, caps)` | run the ticks due at host time `now_ms`; returns how many ran (0..4) |
| `rtw_pixels(rt)` | pointer to the composed 640×480 0RGB frame |
| `rtw_field(rt, rgb)` | the module's field colour, for letterboxing |
| `rtw_next_sound(rt)` | drain one fired sound as an absolute `.wav` path, NULL when empty |
| `rtw_loop_sound(rt)` | the looping sound the module wants (boris's mower buzz) as a `.wav` path, NULL for none |
| `rtw_music_open(rt)` | open the module's MUSIC channel (decodes the instrument bank); NULL when its pack has none |
| `rtw_music_rate()` | 22254 — mono float PCM rate of the music channel |
| `rtw_music_render(m, out, n)` | pull `n` frames of music PCM; thread-safe against `rtw_tick` (call it from the audio thread) |
| `rtw_music_set_volume(m, v)` | synth gain 0..1 (0.4, as the shells) |
| `rtw_music_playing(m)` | is a song sounding — logs and tests |
| `rtw_music_close(m)` | release the handle; stop pulling audio first |
| `rtw_width()` / `rtw_height()` | 640 / 480 |
| `rtw_randomizer_default()` | the Randomizer's factory Default Duration, 60 (`sVal 503`) |
| `rtw_randomizer_band_count/value/label/for` | its slider words (`sUnt 503`), shaped like the `rtw_control_band_*` calls; static strings, no runtime |
| `rtw_randomizer_seconds(v)` | seconds a module runs at Default Duration `v` (`rsVl 503`); -1 = Forever |

The three *control* string calls (`rtw_control_name`, `rtw_control_item`,
`rtw_control_band_label`) share **one scratch slot** in the runtime:
each invalidates the last returned pointer, exactly like `rtw_next_sound`.
Copy the string before the next call (the Swift side does, immediately).
The four **catalogue** calls are the exception — their strings are static,
built once per process and never invalidated, because the sheet reads all
thirteen names in a loop before any runtime exists.

The catalogue is deliberately runtime-free and pack-free. The sheet has to
fill its module list before anything is loaded, and building thirteen
`Pack`s to read thirteen titles would be a visible stall on opening
Options…. It comes from `app::MODULE_TITLES` — slug plus the title the
module's own `Module::name()` reports, with an `app` test
(`module_titles_are_the_modules_own_names`) and the registry's `SLUGS`
keeping the table honest, so this is not the wrapper growing a second
opinion about the modules the way the old `popup_base()` table did.

`rtw_pack_exists` is the separate, cheap question — "is there a `meta.json`
under this root?" — that the panel asks once per module *with* the assets
path in hand, so that a catalogued module the bundle has no art for can be
dimmed instead of offered. It is not part of the catalogue for exactly
the reason above, and it resolves the path the way `rtw_create` will (pack
dir or root) because both go through one `pack_root()`.

Three deliberate asymmetries:

* **`now_ms` does not become the module's clock.** It only decides how many
  ticks are due. The module's `Ctx::now_ms` is *derived from the tick count*
  on the module's own grid (`engine::TickClock`), like the player shell and
  `bin/frame`, because modules gate on `now < deadline` and a jittery clock
  makes those gates swallow ticks. A stall drops its backlog past 4 ticks
  rather than fast-forwarding, and moves the epoch — the tick counter, and
  so the module's clock, never jumps.

  Two grids exist. `TickClock::Millis(ms)` is the plain one (40 ms for most
  modules). `TickClock::MacTick` is After Dark's own: ticks are paced
  16.625 ms apart and `now_ms` is `Resource.f4724()` = the TRUNCATED integer
  `TickCount()*16.625`, computed with the original's `(t<<4) +
  (t*0xA006>>16)` — 16, 33, 49, 66, 83, 99, 116 … Modules on it keep their
  disasm'd millisecond delay constants and let the grid quantize them, which
  is where the captures' 49.9 / 83.1 / 100.0 / 106.4 / 116.4 ms frame
  periods come from. **This is why the ABI grew `rtw_tick_us`:** 16.625 ms
  is not a whole number of milliseconds, and `animationTimeInterval` driven
  from the rounded 16 or 17 runs those modules 3.8 % fast or 2.3 % slow.
* **Every entry point is `catch_unwind`-wrapped.** A panic unwinding across
  the ABI aborts the host — and the host is `legacyScreenSaver.appex`, where
  an abort is a crash report and a dead screensaver.
* **Popup raw values are not always 0-based — and the def now says which.**
  Most modules fold the Mac menu item number down to an index (message
  mayhem: "raw 3 → popup index 2"), but bungee roulette's Jumper keeps the
  original's 1-based `mVal 1000` — `derive_buckets` does
  `jumper_sel.clamp(1, 6) - 1`, so **Cow is raw 3**, which is what
  `tests/abi.rs` has asserted since the wrapper landed. Guessing 0 would put
  the wrong item in the sheet and run the jumper next door.
  `ControlKind::Popup` therefore carries a `base` alongside its items
  (`ControlKind::popup(..)` = 0-based, `popup_based(1, ..)` = bungee's), and
  `rtw_control_min` reports it straight off the def. The wrapper's old
  slug-keyed `popup_base()` exception table is gone, and the player shell
  (`app/src/main.rs`) reads the base too — its title bar used to show
  bungee's Jumper off by one.

## The configure sheet (Options…)

System Settings shows an **Options…** button for a saver whose view returns
`hasConfigureSheet = true`, and presents whatever `configureSheet` hands
back as a sheet on its own window. Ours is built in code from the ABI — no
xib, no Xcode, nothing to keep in step but the header.

Since 2026-09-19 it is a recreation of **After Dark 3.0's own control
panel**, not a generic form. Geometry comes from that `cdev`'s DITL 5000
("General") in the private RE checkout, at scale 2:

| DITL 5000 item | original rect | what it is |
|---|---|---|
| 0 | (1,1)-(305,35) | faceplate banner, full panel width |
| 1-3 | (5,52)-(126,69) | "Sleep After [ 5 ] minutes" |
| 11 | (5,76)-(141,184) | the scrolling module list |
| 10 | (6,188)-(140,204) | the row under the list |
| 5 | (161,53)-(294,69) | right-column header (the module's title) |
| 4 | (176,75)-(291,91) | the one *enabled* custom item (the Demo button) |
| 6-9 | (161, 99/121/144/166), 133x16 | four control slots, 22 pt pitch |
| 12 | (161,187)-(294,203) | bottom-right slot |

Four of those are deliberately absent, and the reasons are in the source:
items 1-3 (macOS owns the idle timer — a screen saver cannot set it), item 4
(no runtime to demo into; the host's Preview is that button), the sound-level
nubbin from the in-module sub-panel (`PICT` 888 "GroovyNubbin" — it sets a
per-module volume this build does not have, see *Not done*), and the OG
list's "Multi-Module" special (`PICT` 140), a mode that does not exist yet.
The other special, **Randomizer**, is in — see below.

So: banner, module list on the left (pack-less modules listed but dimmed and
unselectable), the selected module's controls in the right column, its
`TEXT` 1000 blurb in a pane under both, and Defaults / Cancel / OK. Four
slots is not a guess — it is the most controls any Totally Twisted module
has, so the panel is a fixed size and nothing resizes when you click a name.

Widget look comes from the QEMU captures of the in-module control sub-panel
(`ref/panel-*.png`), not from taste. A slider is a 1 px rule with a small
knob riding on it and ONE line under it: the control's name at the left, the
band word for the knob's position at the right — `Drift Speed … Very Slow`,
`Blemishes … CraterFace`. Berkeley's own label text, verbatim, colon or no
colon. A popup is a framed white box with a hard 1-bit drop shadow and no
disclosure triangle at all. The list inverts its selected row and the scroll
bars are 1-bit. Ink and paper are a dynamic pair: black on white in Light
Mode, INVERTED in Dark Mode rather than turned grey, which is what a 1-bit
screen does.

The banner art is `assets/_shared/faceplate.png` — the Twisted Faceplate
file's `PICT` 128, packed once for the whole collection (see
`docs/ripper.md`) and drawn at an integer scale with interpolation off, the
same rule the saver's own blit follows. A bundle built from a pack tree that
predates it draws a plain fallback instead.

Three AppKit traps this cost, all of which look like a broken panel and none
of which the compiler says anything about:

* **Type must not be scaled with the geometry.** An original 72 dpi pixel IS
  an AppKit point, and a point is already two device pixels on every display
  this runs on. Doubling the rectangles AND the font sizes renders the
  panel's 9 pt Geneva at 36 device pixels. `TwistedTheme.scale` (2) and
  `TwistedTheme.text` (1.3) are separate for this reason.
* **Geneva has no bold face and `NSFontManager` will not synthesise one.**
  `convert(_:toHaveTrait: .boldFontMask)` hands the plain font straight back
  — no error, no nil — so every "Chicago" string was silently drawing in the
  bold *system* font. The bold is faked with a negative `.strokeWidth`,
  which means that text has to be drawn as an attributed string; a bare
  `NSFont` cannot carry it. (Chicago itself has not shipped since Mac OS 9.
  Geneva is still in `/System/Library/Fonts`.)
* **An `NSCell` draws into its control view's coordinate system, and
  `NSButton`'s is FLIPPED.** A drop shadow computed with y-up therefore
  lands as a thick black bar *above* the popup. Pass `controlView.isFlipped`
  to anything that offsets.

A slider is labelled the way After Dark labelled one, from the original's own
`sUnt` words — Jumps runs `One … Hundreds!` and reads "More" at 50, Equipment
`Safe … Phurkpt` (sic, that is the string in the resource). AD's panels
never printed a number on a slider at all, and neither does this one. The
words come over the ABI
(`rtw_control_band_*`) from the user's own pack: the ripper copies each
module's `sUnt` rows into `meta.json` `slider_words` (and its popup `MENU`s
into `menus`), and `app::slider_bands` reads them at runtime — nothing
Berkeley-authored is compiled in (2026-09-29; the hand-typed
`SLIDER_BANDS` table is gone). A control with no words, or a pack ripped
before 2026-09-29, falls back to the raw value. The raw value is still in the log line.

Two things about that table, because it is the shape of thing that rots:
it belongs on `engine::ControlKind::Slider`, next to the range it labels,
the way popup items already live on the def — the lane that added it was
scoped out of `engine/`, so it sits beside `MODULE_TITLES` instead, with
`slider_bands_match_the_modules_own_sliders` refusing to let an entry name a
non-slider, a threshold outside that slider's range, or a slider with no
words at all. And the bands are *display* bands: the module's own bucket
arithmetic is its own (bungee's Jumps is `raw/20`, phlegm boy's Behavior a
seven-way gate) and where the two disagree the panel says what the original's
panel said.

A module the catalogue has but this bundle ships no pack for is listed
**dimmed and unselectable, titled "Name (no pack)"** — not hidden. Hiding it
would make the list's row numbers stop matching the catalogue the titles came
from, and would quietly answer "this build does not have Mowin' Boris" when the truth
is "this bundle was built on a machine with no Mowin' Boris art". Listed
dead says both, and the old failure — pick it, get a black screen and one
`rtw_create failed` line in the log — is not reachable from the UI. The
selection ladder behind it is stored → `RTWModuleSlug` → compiled-in
fallback → the first catalogued module that *has* a pack, so a bundle built
without the default module's art still comes up running something.

**Defaults** resets the visible module's widgets to `ControlDef::default`
and leaves the list selection alone (it is a choice, not a setting).
**Cancel** throws every working copy away, **OK** writes and applies.

### Changing module in the sheet

Picking a module **rebuilds the control slots in place**: the content view
is replaced (the window is NOT resized — the panel is a fixed four slots
tall) because a module's
`ControlDef`s are its own and a stale `NSSlider` still wired to control 3 of
the module you just left would write into the new one. Nothing is loaded
eagerly — the first time you select a module the sheet builds one *scratch*
runtime, reads its defs and destroys it (`RetwistedSaverView.controls(for:
assets:)`), then caches them. That scratch runtime is the only moment two
runtimes exist at once and it is never ticked, never drawn and never allowed
to make a sound.

Two things this cost a rebuild to get right:

* **The height is a *content* height.** Handing it to `setFrame` instead of
  `setContentSize` shrinks the content area by the title bar, and everything
  laid out from the top — the header — slides out of the window. It looks
  fine in an off-screen shot of the view and wrong in the real sheet, so
  `saver_sheet_shot --module` now asserts the content view and the window's
  content rect agree.
* **Nothing may grow off-screen.** The chrome is 175 pt and the biggest
  control set in the catalogue is four rows, so the tallest sheet (Mowin'
  Boris) is 333 pt and the shortest (Voyeur) 175 pt.

Edits are kept in a **per-slug working copy**, so switching modules back and
forth inside one sheet session loses nothing, and OK writes every module you
touched — not just the selected one.

### Where the values live

`ScreenSaverDefaults(forModuleWithName: <bundle identifier>)`: one string
key for the selection, plus one key per control namespaced by slug.

```
module                      = shock-clocks   # which module runs
bungee-roulette.control.0   = 3      # Jumper  = Cow  (1-based popup, see above)
bungee-roulette.control.1   = 50     # Jumps
bungee-roulette.control.2   = 50     # Equipment
shock-clocks.control.0      = 0      # Type = Father Time
shock-clocks.control.1      = 50     # Drift Speed
```

The identifier, not the display name, because that is the string macOS
keys the saver off and it survives renaming the bundle; the slug prefix so
the thirteen modules keep thirteen sets of values instead of one scrambled
one. Values are **clamped to the control's range on read** — a stale plist
from an older build must not push a junk raw value into a module — and the
`module` key is **validated against `rtw_module_index`**, falling back to
Info.plist's `RTWModuleSlug` when it names something this build does not
have. (`RTWModuleSlug` is all that key means now: the default selection.)

The view reads them in `init(frame:isPreview:)` and pushes them through
`rtw_set_control` *before the first tick*, so the settings thumbnail, the
Preview button and the real screensaver all run the same settings. Modules
re-read their controls through `set_control` and may ask to restart
themselves (bungee does, §1.4) — that is the original's behaviour, and it is
left alone.

### What OK does to a running view

OK posts an in-process `com.retwisted.saver.settingsChanged`. **Every** view
in the host answers it, not just the one that vended the sheet: the System
Settings pane preview, the Screen Saver sheet's live thumbnail and an open
Preview are separate instances of this class (see the host-behaviour section
below), and a module chosen in Options… that changed only one of them would
look broken.

Each view then either re-applies the values (same module) or **swaps the
runtime**:

1. `silence()` — stop the static `AVAudioPlayer` if this view owns it, so
   the module you left cannot keep screaming over the one you picked;
2. `rt = nil` *before* `rtw_destroy`, so nothing can tick or draw through a
   dangling pointer;
3. `rtw_create` the new slug from the same assets root, re-read field
   colour, `rtw_tick_us`, the control set, push the stored values in before
   the first tick, and reset the host epoch.

There is never a moment when two runtimes are tickable, and a dormant or
off-screen view stays dormant across the swap — `reloadSelection()` does not
clear `dormant`, only the host's own `startAnimation()` does.

### Host gotchas specific to the sheet

* **The sheet runs inside `legacyScreenSaver.appex`, not System Settings.**
  So does the write. That appex is sandboxed, which means its
  `ScreenSaverDefaults` plist lands in its **container**, not in
  `~/Library/Preferences/ByHost`:

  ```
  ~/Library/Containers/com.apple.ScreenSaver.Engine.legacyScreenSaver/Data/\
      Library/Preferences/ByHost/com.retwisted.saver.<uuid>.plist
  ```

  Looking for the file in the obvious place and not finding it proves
  nothing. `tools/saver_sheet_shot.swift` run from a terminal writes the
  *non*-container copy — same code path, different file.
* **End the sheet, do not just hide it.** The host presents with
  `beginSheet`, so the dismissal is `window.sheetParent?.endSheet(window,
  returnCode:)`; an `orderOut` alone leaves System Settings modal with no
  sheet on screen, i.e. frozen. The fallback for a host that used the old
  `NSApp.beginSheet` pair is `-[NSApplication endSheet:]`, sent dynamically
  because it has been deprecated since 10.10 and would otherwise warn on
  every build. `saver_sheet_shot --modal` asserts `attachedSheet == nil`
  after the button is hit, which is that whole failure mode in one line.
* **Nothing but the view retains the sheet.** The host borrows the window
  and drops it; if the controller dies the buttons silently do nothing. The
  view holds it and lets go in the sheet's own close callback.
* **`configureSheet` is built fresh every time it is asked for.** A window
  that has already been ended cannot be shown again.
* **After a real screensaver session, the sheet can land on a hidden parent**
  (Tahoe 26.x, 2026-09-26). The host rebuilds the pane thumbnail on a hidden,
  non-main `NSServiceViewControllerWindow` and `beginSheet`s onto it: the
  sheet is "visible" at x=0, cannot become key, and Options… looks dead until
  Settings is relaunched. Not a stale view — one view alive, parent is its
  own window. `rescueIfOrphaned` polls until the sheet is attached; if it
  cannot become key, it ends it on the parent and shows a fresh panel as a
  standalone `.modalPanel` window. Log line: `Options sheet orphaned`.

### The Randomizer

After Dark 3.0's module list carried a **Randomizer** entry above the
modules (`STR#` 143 item 1). It is the first row of the panel's list here,
and choosing it stores `module = randomizer` under the same key scheme.

What the original's Randomizer offered, from the AD 3.0 resources in the
private rip (`ripped/after-dark-3.0/`): its own panel had **New… / Edit… /
Delete…** buttons (`bVal` 504/502/501) for *named* Randomizer settings — each
a list of modules with a per-module Duration, played Random or In Order
(`DLOG`/`DITL` 129 "Randomizer Setup", `MENU` 500 "Order", `sUnt` 500) — and
exactly **one** control on the panel itself: the **Default Duration** slider,
`sVal 503` = 60, labelled by `sUnt 503` (Short, 15 sec., 30 sec., 1 min., 2
min., 5 min., 10 min., 30 min., 45 min., 1 hour, 1 h. 30 m., 2 h., 6 h.,
Forever) and turned into seconds by `rsVl 503`. This build has no named
settings, so its Randomizer is "every module this bundle has art for, random
order", and **Default Duration is its whole panel** — transcribed into
`saver/src/lib.rs` (`RANDOMIZER_WORDS`, `RANDOMIZER_SECONDS`) and served over
`rtw_randomizer_*`, pack-free and runtime-free. The sheet synthesises it as a
normal slider (`RetwistedControl.randomizer`), so it looks and behaves exactly
like a module's, and it persists as `randomizer.control.0`.

* **Between two `rsVl` rows** the floor rule is used — the same one the words
  use (`app::slider_band_index`) — so the word under the knob and the time a
  module runs can never disagree. How AD itself read between rows is not
  settled from the resources; with the floor rule the factory 60 is
  "30 min." = 1800 s. "Short" (0) sits below `rsVl`'s first row and gets its
  15 s.
* **The help pane** is our own sentence, not `TEXT` 500 — Berkeley's words
  stay out of the source.
* **Behaviour** (`RetwistedSaverView`):
  - picks a random module from `packedSlugs` — never a pack-less one — and,
    whenever there is a choice, never the one already running;
  - at construction (one pick), on every `startAnimation()` of a view that
    has already animated (a new session / restarted preview; a just-built
    view is not given a second pick), and every Default Duration of host time
    since that module's open (Forever = only at session start);
  - every swap goes through the ONE swap path (`swap(to:)`, which OK uses
    too): `close()` — audio first, music cut — then `open()`, stored
    controls for the new slug pushed in before its first tick, timer cycled
    for the new rate. One runtime alive at a time;
  - OK while already randomizing keeps the module that is up and re-times it
    to the new Default Duration; OK on a module leaves Randomizer mode.
* The construction log line says `randomizer=off|<n>s|forever`, and each
  rotation logs `module a -> b (randomizer)`.

Headless check (15 s rotation, previews are silent):

```
xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver /tmp/s.png \
    --reset --modal --module Randomizer --set 0=0 --ok
xcrun swift tools/saver_shot.swift target/saver/Retwisted.saver /tmp/r.png 1100 preview
/usr/bin/log show --last 1m --info --predicate 'subsystem == "com.retwisted.saver"' | grep randomizer
```

## Verifying without waiting for the screen to blank

`tools/saver_shot.swift` loads a built bundle exactly the way the system
does (CFBundle → `principalClass` → `init(frame:isPreview:)`), runs it for N
frames and writes what the view actually drew:

```
xcrun swift tools/saver_shot.swift target/saver/Retwisted.saver /tmp/shot.png 300
xcrun swift tools/saver_shot.swift target/saver/Retwisted.saver /tmp/prev.png 250 preview
```

This catches the whole class of "the bundle is wrong" failures (principal
class not found, staticlib not linked, assets missing from Resources, the
frame upside down or with R and B swapped) without System Settings in the
loop.

**Real time has to pass.** The harness sleeps one `animationTimeInterval`
between frames on purpose: a tight loop advances the host clock by
microseconds, `rtw_tick` finds no ticks due, and you get a black PNG that
looks exactly like a broken saver.

`tools/saver_sheet_shot.swift` is its sibling for the *settings*: it loads
the bundle the same way, asks the view for its `configureSheet`,
screenshots the sheet off-screen, optionally works the widgets and hits a
button, and then builds a **second, fresh view** — which reads the stored
values back exactly like the host does — and screenshots what that one
animates.

```
# what the sheet looks like, on factory defaults
xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver \
    /tmp/sheet.png --reset

# pick a Cow, OK it, and prove a fresh view comes up as a cow
xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver \
    /tmp/sheet.png --modal --pick Cow --ok --run /tmp/cow.png --frames 400

# switch module: --module drives the module LIST (DITL 5000 item 11), the
# control slots beside it are rebuilt, OK swaps the runtime in the LIVE view
# (--live) and a fresh view reads it back (--run)
xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver \
    /tmp/sheet.png --reset --modal --module shock-clocks --ok \
    --live /tmp/live.png --run /tmp/run.png --frames 400

# … and that the settings thumbnail honours it too
xcrun swift tools/saver_sheet_shot.swift target/saver/Retwisted.saver \
    /tmp/sheet.png --preview --run /tmp/thumb.png --frames 260
```

It also prints, every run, the module list row by row — with `[disabled]`
against any module this bundle ships no pack for — the module's blurb, and
each slider's label line, current word and raw range. A list scrolled past
its tenth row and a closed popup are both illegible in a screenshot:

```
modules: 0=Bungee Roulette (no pack) [disabled], 1=Chameleon *, 2=Coming Soon!, …
slider rtw.control.1: 0..100 = 50 "More" ends ["One", "Hundreds!"]
```

Flags: `--module <slug|Title>` (takes either spelling, and matches the
"(no pack)" suffix too — pointed at a disabled module it asserts the sheet
*refuses* to switch; runs before the others, since picking a module replaces
every widget below it),
`--pick <item>`, `--set <index>=<raw>`, `--defaults`, `--ok`, `--cancel`,
`--modal` (present on a host window and assert the sheet actually ends),
`--reset` (clear every module's stored values *and* the selection),
`--live <png>` (the view that vended the sheet, after OK — the in-place
runtime swap), `--run <png> --frames n` (a fresh view, reading the store
back), `--preview`, `--dark`, `--warm <n>` (animate the vending view first and print its audio state before and after OK).

**Screenshotting AppKit off-screen has two traps**, both of which produce a
PNG that looks like a broken sheet rather than a broken harness.
`cacheDisplay` draws the view and nothing behind it, so the window
background is missing; on a machine in Dark Mode that is white label text
composited onto white — an empty sheet with a few grey bezels, which is
precisely what the first run of this tool produced. Fill
`windowBackgroundColor` behind the rep, and pin the window's appearance
(the tool uses `.aqua` unless `--dark`) so the shot is the same on any
machine.

## macOS gotchas, all of which cost time

**Third-party savers do not run in their own process.** Sonoma and later
(this is Tahoe, macOS 26.4) load them into
`/System/Library/Frameworks/ScreenSaver.framework/PlugIns/legacyScreenSaver.appex`.
Consequences:

* That appex is **sandboxed** (`com.apple.security.app-sandbox`). It holds a
  read-only temporary exception for `/`, so reading the asset pack out of
  the bundle is fine. The one place it can *write* is its own container —
  which is where the first-run rip puts the packs (see *First run* below).
  Full entitlement list (Tahoe 26.4, `codesign -d --entitlements -`):
  app-sandbox, `files.user-selected.read-only`, `files.bookmarks.app-scope`,
  `assets.pictures.read-only`, `temporary-exception.files.absolute-path.read-only
  = /`, network client/server, `cs.disable-library-validation`, a few
  mach-lookup exceptions.
* It has `com.apple.security.cs.disable-library-validation`, which is the
  only reason an **ad-hoc** signature (`codesign --sign -`) works. Without a
  signature at all the bundle is killed on load on Apple Silicon, so the
  build script always signs.
* Logs come out under that process, not yours. Filter by subsystem:
  `log show --last 5m --info --predicate 'subsystem == "com.retwisted.saver"'`,
  or Console.app with `subsystem:com.retwisted.saver`. The view logs one
  `notice` line per construction with slug, size, rate, `isPreview`, the
  resolved control values and the assets path — if `rtw_create` failed, it
  logs that instead.

**Caching is aggressive, in two places.** `legacyScreenSaver` keeps the
dlopened bundle for the life of its process, and System Settings caches the
saver list and its thumbnail. Reinstalling over a running one gets you the
old code, silently. `--install` therefore `pkill`s `legacyScreenSaver`,
`legacyScreenSaver-x86_64`, `ScreenSaverEngine` and System Settings (they all
relaunch on demand). Reopen System Settings after installing.

**`NSPrincipalClass` is an Objective-C runtime name.** A plain Swift class
gets a mangled name (`_$s14RetwistedSaver…`) that the loader will never find,
and the failure mode is that the saver simply does not appear in the list —
no error anywhere. Hence `@objc(RetwistedSaverView)` on the class and
`NSPrincipalClass = RetwistedSaverView` in Info.plist. The two must match
exactly.

**The executable must be a Mach-O *bundle*, not a dylib.** `swiftc
-emit-library` alone emits `MH_DYLIB`; adding `-dynamiclib` and `-bundle`
together is a linker error. The working incantation is
`swiftc -emit-library -Xlinker -bundle`, which still picks up the Swift
runtime link flags. Check with `otool -hv …/Contents/MacOS/Retwisted` —
filetype must read `BUNDLE`.

**The bundle carries 13 packs, ~17 600 files.** `codesign` seals all of
them, so a build is a few seconds slower than it was and `--install`
`ditto`s 91 MB into `~/Library/Screen Savers/`. Nothing about that is
load-bearing; it is just what "all the modules" costs.

**Other Info.plist keys that matter:** `CFBundlePackageType = BNDL`,
`CFBundleExecutable` matching the actual file name in `Contents/MacOS/`,
`NSHighResolutionCapable` (without it you get a blurry 1x upscale on
Retina), and `LSMinimumSystemVersion`. `RTWModuleSlug` is ours — the *default*
module, what the view runs when nothing is stored or the stored slug is not
in this build. Everything else about which module runs comes from the
`module` key in `ScreenSaverDefaults`.

**Keep the bundle identifier stable.** macOS keys the selected-screensaver
preference off the bundle id; change it and your saver reverts to "not
selected" and the old id lingers in the list until the caches are flushed.
`com.retwisted.saver` is it.

**No Xcode required.** The Command Line Tools SDK ships
`ScreenSaver.framework`, so `xcrun swiftc` against
`$(xcrun --show-sdk-path)` is enough.

**arm64 only.** The script builds for `$(uname -m)-apple-macos13.0`. A
universal saver would need `cargo build --target x86_64-apple-darwin` as
well plus `lipo`; nothing here assumes one architecture, it just isn't
built.

## Using it

System Settings → **Screen Saver** → scroll to **Other** → **Retwisted**.
Hit *Preview* for a full-screen run. The thumbnail in the list is the same
view with `isPreview = true`. Which of the thirteen modules it runs is an
**Options…** choice, not a separate entry in the saver list — there is one
`Retwisted` and it can be any of them.

Sound is deliberately **muted when `isPreview` is true** — the settings
thumbnail would otherwise scream at you — and plays through a single
`AVAudioPlayer` otherwise, stopped and replaced on every new sound, which is
the original's single pre-empting `SndChannel` (and what `App::play_sounds`
does with one rodio `Sink`).

### Music

Four modules have **music** — the `cmid` songs After Dark's MDRV synth plays
through the shared Twisted Sound instrument bank (`engine::music`): Mime
Hunt (song 10), Coming Soon (20), FrankenScreen (30), Mowin' Boris (40, the
dawn cue). It is a **second channel**, as in the original and the player
shell: it never pre-empts the sfx and is never pre-empted by them.

* **The module decides; the runtime obeys.** `Runtime::pump_music` runs after
  every tick and does exactly what the shell's `App::pump_music` does: a
  change of the module's `(song, plays started)` state restarts the tune from
  the top, `None` stops it, no change touches nothing. So mime hunt's Music
  bands (raw < 5 = no plays), coming soon's replay counter, frankenscreen's
  Music latch and boris's once-per-dawn cue are the modules' own gates, not
  the saver's.
* **ABI shape.** `rtw_music_open(rt)` returns a *separate* host-owned handle
  (an `Arc` of the synth + songs; the runtime keeps only a `Weak`), so the
  audio thread pulling `rtw_music_render` and the main thread calling
  `rtw_tick` only meet at one mutex — the player shell's `MusicStream`
  arrangement. The bank is decoded on open, not at `rtw_create`: only the
  one view allowed to make noise ever pays for it (250 KB–1.1 MB of samples).
  A song the module is already in when the channel opens starts from the top.
  A handle that outlives its runtime renders silence.
* **Swift side.** One `AVAudioEngine` + `AVAudioSourceNode` per process (like
  the static sfx player), owned by one view. It is opened past **the same gate
  as the sfx** — `!isPreview && isOnScreen && isRealRun` (a real session, on
  a shielding-level window) and not dormant — and torn down by `silence()`,
  i.e. on every occasion the sfx are: `stopAnimation`, the stop/unlock
  broadcasts (dormant), System Settings quitting, leaving the window, hiding,
  and `close()` before a module swap destroys the runtime. Teardown order:
  engine stopped, then the handle closed. Volume 0.4, the shells' default.

Headless, `saver_shot` prints the view's `rtwAudioState` (music open/none,
engine running, song playing) and `saver_sheet_shot --warm N` prints it
before and after OK, which is the swap-cuts-the-music check:

```
audio before OK: slug=mowin-boris music=open engine=true playing=true
audio after OK:  slug=mime-hunt   music=none engine=false playing=false
audio live:      slug=mime-hunt   music=open engine=true playing=true
```

(Headless there is no window, so a non-preview view counts as the real run
and really plays — keep those runs short.)

### Options… — and picking a module

System Settings → **Screen Saver** → **Retwisted** → **Options…**. The top
row is **Module**: pick one of the thirteen and the controls under it change
to that module's. Then set them, and hit **OK**. The thumbnail restarts on
the new module; so does an open Preview and the pane's big preview.
Reinstalling the bundle does *not* clear the stored values — the keys live
under the bundle id, which is stable — so your per-module settings and your
choice of module both survive a rebuild.

Each module keeps its own settings, so switching away and back is
non-destructive. Cancel abandons everything, including the module change.
A module greyed out as "(no pack)" is one this build catalogues but this
bundle has no art for — rebuild with `tools/pack_assets.py` for that slug
and it becomes selectable.

To watch what the host thinks it read, the construction log line carries the
resolved settings, and a swap logs the transition:

```
log show --last 5m --info --predicate 'subsystem == "com.retwisted.saver"'
… bungee-roulette up — 640x480 @25Hz preview=true controls=Jumper=Cow Jumps=50 Equipment=50 …
… module bungee-roulette -> shock-clocks
… shock-clocks up — 640x480 @25Hz preview=false controls=Type=Father Time Drift Speed=50 …
```

## What a long run costs

Measured on this tree (release, M5 Pro), because the ledger for `0fc83c3`
guessed at it and guessed wrong.

**Creating a runtime loads exactly one pack.** It always did — the ledger's
"every pack is loaded at create time" is not what the code does, and
`abi_create_needs_only_the_selected_modules_pack` now proves it structurally
rather than by stopwatch: each runtime is built from a root that physically
contains only its own pack (a temp dir of symlinks), so needing a
neighbour's art would be a NULL, not a slow frame. What create costs, per
module, from a warm page cache:

| | create | RSS after create |
| --- | --- | --- |
| lightest (flying-toilets) | 0.05 ms | +0.03 MB |
| typical (chameleon, shock-clocks, voyeur) | 0.2–1.3 ms | +1–2 MB |
| heaviest (message-mayhem, bungee, mikes) | 5–10 ms | +1–2 MB |

The heavy three decode a few compounds in their constructors. Building a
scratch runtime for **all thirteen** — what clicking down the whole module
list would cost — is **28 ms and 4.6 MB** all told, so the sheet's
create-defs-destroy-per-module scheme needs nothing cleverer.

**The decoded-sprite cache was the real growth.** `app::compose` decodes a
compound the first time it is drawn and keeps it, keyed by `(png, clut)`.
Over 20 000 frames (13 min of a 40 ms module, 5½ of a Mac-tick one) that
carried the process to:

| | before | after the cap |
| --- | --- | --- |
| chameleon (recolours — every compound × every clut) | 130 MB RSS, still climbing | **35 MB** |
| voyeur (16 MB pack) | 90 MB | **44 MB** |
| mikes-so-called-life | 16 MB | 17 MB |
| bungee-roulette | 8 MB | 8 MB |

So `saver/src/lib.rs` caps the cache at **24 MB of decoded RGBA** and drops
it whole when it is passed; the next compose re-decodes what is on screen
*now*, which is the working set rather than the accumulated history.
Chameleon crosses the cap about every 700 ticks (~25 s) and pays 2.3 ms for
the refill frame against its 16.6 ms budget; voyeur pays 11.7 ms against
60 ms. Neither is a dropped frame. An LRU would spread the cost, but
`compose` owns the map and cannot report hits, and a bounded clear leaves
the player shell and `bin/frame` byte-identical — they live for seconds and
want the cache they have.

RSS does not fall when the runtime is destroyed: that is the allocator
holding pages, not a leak — the next module's decodes land in them.

### Lazy, shared packs (2026-09-29)

What was actually loaded, and when, before this pass:

| path | loaded |
| --- | --- |
| catalogue (`rtw_module_*`) | nothing — static table, never a pack |
| `packedSlugs` / `rtw_pack_exists` | one `stat` of `meta.json` per module |
| `rtw_create` | ONE pack (parse `meta.json`, build clut remaps) + the module constructor — but **per view**, in `init`, for every view the host builds (3–4 per process, usually the same module), each with its own parse, its own deep copy of `Meta` in every `Pack` clone, and its own sprite cache (up to 24 MB each) |
| dormant views | kept all of it forever (the host never tears views down) |
| sheet: `controls(for:)` | a whole scratch runtime per module clicked, per sheet — module constructors are not free (chameleon ~35–55 ms, phlegm boy ~22, bungee ~12–17, boris ~13) |
| sheet: `helpText` | Foundation-parses the whole `meta.json` for one string (mime hunt 3.2 MB: ~14 ms) per click |

What it does now:

* **`Pack` is cheap to clone** (`engine`): `meta` and the remaps are `Arc`s.
  Every module keeps a clone; it used to be a deep copy. No module code
  changed — field access through the `Arc` is transparent.
* **One pack per process, shared** (`saver/src/lib.rs` `acquire_pack`): a
  registry keyed by canonical pack root. The first runtime of a pack loads
  it; others share the parse AND one decoded-sprite cache (one 24 MB cap for
  the process, not one per view). `rtw_destroy` sweeps entries nothing uses,
  so a pack is loaded exactly while some module on it is instantiated.
  `abi_pack_is_shared_by_live_runtimes_and_freed_with_the_last` proves the
  share, the free, and that the probe never loads. (`set_cache_cap`, the
  test hook, gives that runtime a private cache.)
* **Views build lazily.** `init` only decides the module; the runtime is
  built by `startAnimation()` (before `super`, so the timer gets the module's
  rate), the first `animateOneFrame`, or the first `draw` — whichever comes
  first. A view the host builds and never shows loads nothing.
* **Dormant views let go.** Stop/unlock broadcasts and System Settings
  quitting now `close()` the runtime (audio first). `startAnimation()`
  rebuilds it — fresh, as After Dark started a module fresh on every
  activation; under the Randomizer it is a new pick. A settings change that
  reaches a view with no runtime just records the choice.
* **Per-process caches in the sheet** for control defs (filled by every
  `open()`, so the running module never needs a scratch runtime) and help
  text.

Measured (release, M5 Pro, warm page cache). Live Rust heap for four
runtimes of one module in one process, 300 frames each (a counting allocator
around `rtw_create`/`rtw_tick`/`rtw_pixels`; 1.17 MB of every runtime is its
own 640×480 frame buffer):

| module | 4 live before | 4 live after | 2nd+ `rtw_create` before → after |
| --- | --- | --- | --- |
| mime-hunt | 21.5 MB | 11.4 MB | 3.3 → 0.5 ms |
| voyeur | 18.6 MB | 9.7 MB | 2.8 → 0.1 ms |
| chameleon | 11.1 MB | 7.6 MB | 35 → 29 ms (constructor) |
| bungee-roulette | 11.4 MB | 7.5 MB | 9.3 → 7.5 ms (constructor) |
| mowin-boris | 6.2 MB | 5.2 MB | 7.7 → 6.9 ms |

Three of the four going dormant now frees their runtimes outright (before:
nothing freed, and each kept up to 24 MB of sprites). The worst case for
sprites across the process drops from 24 MB × views to 24 MB.

Whole process, the bundle loaded the host's way, four `isPreview` views of
one module (includes AppKit/CG, so noisier): `init` 5–40 ms → **0.1 ms**
(nothing built); first drawn frame of the first view 124 → 93 ms (mime hunt)
/ 99 → 83 (voyeur) / 119 → 112 (chameleon), later views ~35 → ~30 ms; peak
RSS with four views 71.5 → 62.5 MB (mime hunt), 70.1 → 59.0 (voyeur), 57.0 →
54.1 (chameleon), 55.3 → 53.9 (bungee). Process RSS does NOT fall when views
go dormant — the allocator keeps the pages (above) — but the next module's
allocations land in them instead of growing the process. The cost: a woken
view rebuilds its module, so its first frame is 20–30 ms for most modules and
~95 ms for chameleon (its constructor), once per session start.

## Not done

* The sheet has no live preview of its own.
* Slider *tick marks* are still evenly spaced (`NSSlider` can only do that),
  while the real `sUnt` positions are not — mime hunt's are 0/10/40/70/90.
  The words under the ends and over the thumb are right; the tick spacing is
  decorative.
* `app::SLIDER_BANDS` should be `engine::ControlKind::Slider { words }`.
  It is a table beside the modules instead of on them, which is the shape of
  the `popup_base()` bug that was removed in `0fc83c3`; a test holds it in
  place until `engine/` can be touched.
* `assets/` is gitignored and stays that way — the bundle is assembled from
  your local packs at build time. Nothing Berkeley ever enters the repo.
* Nothing shrinks the 83 MB of packs on DISK: no dedup between modules, no
  lazy fetch, no per-module sub-bundle. (What is in RAM is: packs load on
  instantiation, are shared per process and freed with their last runtime.)
* Module constructors are the remaining per-instance cost (chameleon ~30 ms
  every build). That is module code, out of the saver's reach.

## `sUnt` errata

> **SUPERSEDED 2026-09-29.** The words and ticks now come straight from the
> pack's ripped `sUnt` rows, and those showed the old hand-typed table was
> mis-transcribed in places: message mayhem's Duration ticks are 0/34/67
> (not 34/67/100), shock clocks' Drift Speed ticks run 0–80 (no inferred
> fifth "Fast" band), and the resource spellings are "Langorous", "Manic"
> and Boris Revenge "Rare". Of the notes below, "Phurkpt" and chameleon's
> doubled "Hyper" still stand; the shock-clocks and tick-start notes were
> artefacts of the transcription.

Transcribing the slider words out of the disasm'd `sUnt` tables turned up
four things worth not re-deriving. These are spec notes, not port bugs —
the RE repo's docs are read-only from here.

* **bungee roulette, Equipment 80 = "Phurkpt".** Literally the string in the
  resource (spec §1.3 flags it too). Not a transcription slip, do not "fix".
* **chameleon, Zest 80 and 100 are both "Hyper".** Five distinct words for
  six ticks, so the slider's top fifth is unnamed-by-repetition. The sheet
  prints "Hyper" at both ends of that stretch because the resource does.
* **shock clocks, Drift Speed has four ticks and five bands.** `sUnt 1001`
  carries 20/40/60/80 (Still / Very Slow / Slow / Moderate) and the spec
  names a fifth, "Fast", at the top (§8 and the drift table) — the entry at
  100 is inferred from that, and is the one word in the table not read
  straight off a resource dump.
* **Ticks do not start at 0 everywhere.** frankenscreen's start at 10,
  toxic swamp's and shock clocks' at 20, message mayhem's Duration at 34.
  Anything below the first tick is in the first band — which is also what
  the modules' own arithmetic does (frankenscreen Lifespan `< 20` is the
  30-second "Brief", whose tick is at 10), so the floor rule in
  `rtw_control_band_for` is not a UI convenience.

## legacyScreenSaver host behaviour (Tahoe 26.4, verified 2026-09-12 via `/usr/bin/log`)

Everything below was observed from the view's own log lines (`subsystem ==
"com.retwisted.saver"`, one notice every 5 s per view plus one per sound).
Note: zsh has a `log` builtin — always call `/usr/bin/log show`.

- **Several full-size, `isPreview == false` views per run.** The System
  Settings pane's big preview, the Screen Saver sheet's live thumbnail, the
  Preview button's window and the real run are all separate views, all
  1512×982, all unmuted by default. Each had its own RNG and its own
  AVAudioPlayer → two or three bungee runs interleaving their cues
  ("humans mooing, fish screaming").
- **Views are never torn down.** After dismissal the host keeps calling
  `animateOneFrame` on them (15 % CPU, audio continuing after unlock).
  Their windows keep reporting `isVisible == true`, and the shielding-level
  saver windows report `occlusionState` as *not* visible even while they
  are the only thing on screen — testing occlusion blanks the live saver.
- **Window level does not separate the sheet thumbnail from a real run**:
  both sit on level −2147483625. The pane preview is level 0.
- **Distributed notifications are the reliable signal**:
  `com.apple.screensaver.didstart` on a real run;
  `com.apple.screensaver.{willstop,didstop}` and `com.apple.screenIsUnlocked`
  on dismissal (each delivered to every view in the process). Nothing fires
  when System Settings closes — watch `NSWorkspace.didTerminateApplication`
  for `com.apple.systempreferences` instead.
- **didstart must not wake dormant views**: it resurrected the pane view
  and produced a second run over the real one.

Resulting policy in `RetwistedSaverView`: one static audio channel per
process; a view plays only when `sessionActive && window.level != .normal`,
its `startAnimation()` came no earlier than 5 s before the session's first
start broadcast (the sheet thumbnail shares the real run's level and
broadcasts, and played over it with Settings open — 2026-09-30), and it is
on a visible window; full-screen views go dormant on the stop /
unlock broadcasts and when System Settings quits, and only the host's
`startAnimation()` wakes them; dormant or off-screen views neither tick nor
draw.
