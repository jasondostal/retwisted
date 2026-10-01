// C ABI for the retwisted module runtime (see ../src/lib.rs).
//
// This header is hand-written and is the bridging header the Swift
// ScreenSaverView compiles against; keep it in step with lib.rs.

#ifndef RETWISTED_SAVER_H
#define RETWISTED_SAVER_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque runtime handle. Create it once, destroy it once.
typedef struct RtwRuntime RtwRuntime;

// Frame geometry: always 640x480, the original's field.
uint32_t rtw_width(void);
uint32_t rtw_height(void);

// --- the module catalogue ------------------------------------------------
//
// Enough to offer a CHOICE of module with no runtime and no asset pack
// loaded: how many this build has, and per index a slug (what rtw_create
// takes, and what the host should namespace its stored settings with) and a
// display name (the original After Dark title). Indices run in
// control-panel order.
//
// Unlike every other string here these pointers are STATIC — built once per
// process, never invalidated by a later call. The host may hold them.
uint32_t rtw_module_count(void);
const char *rtw_module_slug(uint32_t index);
const char *rtw_module_name(uint32_t index);

// Index of `slug`, or -1 if this build has no such module. Use it to
// validate a slug read back out of stored settings before creating with it.
int32_t rtw_module_index(const char *slug);

// Does `assets_dir` carry `slug`'s art? The catalogue above is compiled in,
// the packs are copied into the bundle at build time, and a module can be in
// the one and not the other — picking such a module used to get a black
// screen and one line in the log. Deliberately NOT part of the catalogue:
// those four calls must stay pack-free and pathless (the popup is filled
// before the host has decided anything). This is the separate, cheap
// question, asked once per module with the assets root in hand, and it
// resolves `assets_dir` exactly the way rtw_create will (pack dir or root).
bool rtw_pack_exists(const char *slug, const char *assets_dir);

// Create a runtime for `slug`, loading its asset pack from `assets_dir`
// (either the pack directory itself or a root containing <slug>/).
// Returns NULL on failure.
RtwRuntime *rtw_create(const char *slug, const char *assets_dir);

// Destroy a runtime. NULL-safe.
void rtw_destroy(RtwRuntime *rt);

// Set control `index` to `value` (module-defined raw values).
void rtw_set_control(RtwRuntime *rt, int32_t index, int32_t value);

// --- control introspection, enough to build a settings UI blind ----------
//
// Kind codes for rtw_control_kind.
#define RTW_CONTROL_SLIDER 0
#define RTW_CONTROL_POPUP 1
#define RTW_CONTROL_CHECKBOX 2

// How many controls the module has (0 = offer no configure sheet).
uint32_t rtw_control_count(const RtwRuntime *rt);

// Kind of control `index`: RTW_CONTROL_*, or -1 for a bad index.
int32_t rtw_control_kind(const RtwRuntime *rt, int32_t index);

// Inclusive legal raw range, and the module's factory default. A checkbox
// is 0/1; a popup runs min..max across its items, item n being min + n
// (the base is NOT always 0: it comes from the module's own ControlDef —
// bungee roulette's Jumper is the original's 1-based Mac menu).
int32_t rtw_control_min(const RtwRuntime *rt, int32_t index);
int32_t rtw_control_max(const RtwRuntime *rt, int32_t index);
int32_t rtw_control_default(const RtwRuntime *rt, int32_t index);

// Number of popup items (0 for sliders and checkboxes).
uint32_t rtw_control_item_count(const RtwRuntime *rt, int32_t index);

// Display name of the control, and the label of popup item `item`. UTF-8,
// NULL for a bad index. Both share ONE scratch slot in the runtime: each
// call invalidates the previous string, so copy it before the next call.
const char *rtw_control_name(RtwRuntime *rt, int32_t index);
const char *rtw_control_item(RtwRuntime *rt, int32_t index, int32_t item);

// --- slider end labels (After Dark's `sUnt` words) -----------------------
//
// AD's control panels never printed a number on a slider: every sVal had an
// sUnt sibling holding (position, word) rows, and the panel spelled them out
// under the track -- "One" at one end, "Hundreds!" at the other. A host that
// can only read min and max renders "0" and "100", which is the one thing
// the original never showed.
//
// Shaped like the popup-item calls: how many words, the raw value each sits
// at (ascending, so word 0 is the low end), and the word itself. Words share
// the SAME one scratch slot as rtw_control_name/rtw_control_item -- copy
// before the next call. 0 words for every control the original labelled with
// nothing (popups, checkboxes).
uint32_t rtw_control_band_count(const RtwRuntime *rt, int32_t index);
int32_t rtw_control_band_value(const RtwRuntime *rt, int32_t index, int32_t band);
const char *rtw_control_band_label(RtwRuntime *rt, int32_t index, int32_t band);

// Which word raw `value` falls under: the last tick at or below it, and word
// 0 for anything below the first tick (several tables start at 10 or 20 while
// the slider starts at 0, and the module's own arithmetic puts those values
// in the first band too). -1 when the control has no words. The rule lives
// here so two hosts cannot round it differently.
int32_t rtw_control_band_for(const RtwRuntime *rt, int32_t index, int32_t value);

// The module's tick period in ms, ROUNDED DOWN (16 for a module on After
// Dark's 16.625 ms Mac tick). Prefer rtw_tick_us for the host timer.
uint32_t rtw_tick_ms(const RtwRuntime *rt);

// The module's tick period in MICROseconds — use it as
// animationTimeInterval. 40000 for most modules; 16625 for the modules that
// ride After Dark's Mac tick (TickCount()*16.625), which is not a whole
// number of milliseconds.
uint32_t rtw_tick_us(const RtwRuntime *rt);

// Advance the sim to `now_ms` (host monotonic clock). `hour/minute/second`
// are the local wall clock; `mouse_x/y` are sim coordinates (640x480) or
// (-1,-1) when the pointer is outside the field. Returns ticks run (0..4).
uint32_t rtw_tick(RtwRuntime *rt, uint64_t now_ms, uint8_t hour, uint8_t minute,
                  uint8_t second, int32_t mouse_x, int32_t mouse_y,
                  bool mouse_down, bool caps_lock);

// The composed frame: width*height uint32_t in 0RGB (0x00RRGGBB). Valid
// until the next rtw_tick / rtw_set_control / rtw_destroy.
const uint32_t *rtw_pixels(RtwRuntime *rt);

// The module's field (background) colour, written to rgb[0..3].
void rtw_field(const RtwRuntime *rt, uint8_t *rgb);

// Drain one fired sound as an absolute .wav path, or NULL when the queue is
// empty. Valid until the next rtw_next_sound call.
const char *rtw_next_sound(RtwRuntime *rt);

// Whether the sound the last rtw_next_sound returned QUEUES behind the one
// playing (back to back, gapless) instead of pre-empting it.
bool rtw_sound_queued(const RtwRuntime *rt);

// Whether the composed frame differs from the last one this returned true
// for (true on the first call). Redraw only then.
bool rtw_frame_changed(RtwRuntime *rt);

// The currently active looping sound as an absolute .wav path, or NULL when
// no loop is sounding. Valid until the next rtw_loop_sound call.
const char *rtw_loop_sound(RtwRuntime *rt);

// --- the Randomizer ------------------------------------------------------
//
// After Dark 3.0's "Randomizer" list entry: run a random module, move on to
// another every Default Duration. Its panel's one control is the Default
// Duration slider — sVal 503 (factory 60), words sUnt 503, seconds rsVl 503.
// Everything here is pack-free and runtime-free; strings are STATIC.
int32_t rtw_randomizer_default(void);
uint32_t rtw_randomizer_band_count(void);  // slider range: band 0 .. last band
int32_t rtw_randomizer_band_value(int32_t band);
const char *rtw_randomizer_band_label(int32_t band);
int32_t rtw_randomizer_band_for(int32_t value);
// Seconds each module runs for at `value`; -1 = Forever.
int32_t rtw_randomizer_seconds(int32_t value);

// --- the music channel ---------------------------------------------------
//
// After Dark's MDRV music (the cmid songs through the instrument bank) is a
// SEPARATE channel: it never pre-empts the sfx above and is never pre-empted
// by them. The runtime starts/stops songs as the module's own Music state
// changes (its gates, its play counts) on every rtw_tick; the host pulls PCM.
typedef struct RtwMusic RtwMusic;

// Open rt's music channel; NULL when the module's pack has no music. Decodes
// the instrument bank, so only open it when you will actually play (the saver
// does it for the real-run view only). Whatever the module is playing now
// starts from the top. Host-owned: close it exactly once. It may outlive rt
// (it renders silence after rtw_destroy).
RtwMusic *rtw_music_open(RtwRuntime *rt);

// Sample rate of rtw_music_render's output (22254 Hz), mono float.
uint32_t rtw_music_rate(void);

// Render `frames` mono float samples into `out` (overwritten). Thread-safe
// against rtw_tick: call it from the audio render thread. NULL m = silence.
void rtw_music_render(const RtwMusic *m, float *out, uint32_t frames);

// Channel gain 0..1 (the player shell's default is 0.4).
void rtw_music_set_volume(const RtwMusic *m, float volume);

// True while a song is sounding. For logs/tests.
bool rtw_music_playing(const RtwMusic *m);

// Release the handle. Stop pulling audio FIRST. NULL-safe.
void rtw_music_close(RtwMusic *m);

// --- first-run ripping ---------------------------------------------------
//
// The shippable .saver carries no asset packs. These run the in-process
// ripper (ripper/, lib twistedrip) on the user's own Totally Twisted
// download — floppy .sit, CD .sit/.iso, .sit.hqx, MacBinary, or a folder of
// expanded module files — on a BACKGROUND thread, and the host polls it from
// its own (main) thread. Poll-based because the ripper's progress callback
// fires on the thread doing the rip; nothing here calls back into the host.
//
// The rip lands in a temp dir BESIDE `assets_root` (its parent's
// `.rip-*`), never inside it, and each finished `<slug>/` (and `_shared/`)
// is then moved into `assets_root` with one atomic rename — so
// rtw_pack_exists never sees a half-written pack. A module the input did
// not contain keeps whatever pack it already had.
typedef struct RtwRip RtwRip;

#define RTW_RIP_RUNNING 0
#define RTW_RIP_DONE 1
#define RTW_RIP_FAILED 2

// Start a rip of `input` into `assets_root` (created if missing). NULL for a
// NULL/empty argument; every failure after that is reported by the job.
RtwRip *rtw_rip_start(const char *input, const char *assets_root);

// RTW_RIP_RUNNING / _DONE / _FAILED (FAILED for a NULL job). `step`/`total`
// (either may be NULL) are the ripper's progress, for a bar.
int32_t rtw_rip_poll(const RtwRip *job, uint32_t *step, uint32_t *total);

// Progress line while running; a one-line summary once DONE; a
// human-worded error ("That file isn't a Totally Twisted download…") once
// FAILED. ONE scratch slot per job: copy before the next call.
const char *rtw_rip_message(RtwRip *job);

// Module packs a DONE job installed (1..13); 0 otherwise.
uint32_t rtw_rip_module_count(const RtwRip *job);

// Release the job. Freeing a RUNNING job cancels it: the worker finishes
// the rip it is in, then deletes its temp dir instead of installing.
// NULL-safe.
void rtw_rip_free(RtwRip *job);

#ifdef __cplusplus
}
#endif

#endif // RETWISTED_SAVER_H
