//! retwisted engine core: asset packs, Berkeley RNGs, the Module contract,
//! bitmap text, and runtime palette remapping.

pub mod font;
pub mod l135;
pub mod music;

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// RNGs

/// Library 4.0 seg-131 `rand()`/`srand()`: the plain ANSI LCG.
/// `seed = seed * 0x41C64E6D + 12345; return (seed >> 16) & 0x7FFF`
/// (decompiled seg 131 @ listing 59A0; the multiply is Runtime fn00CE, a
/// 32x32 -> low 32 product). Earlier ports took the LOW bits and built a
/// "rand()%2 strictly alternates" quirk on top — that was an error of the
/// prose layer, not the binary. Modules that lean on alternation are wrong.
pub struct Random15 {
    seed: u32,
}

impl Random15 {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }
    /// `srand(seed)`.
    pub fn srand(&mut self, seed: u32) {
        self.seed = seed;
    }
    pub fn next(&mut self) -> u16 {
        self.seed = self.seed.wrapping_mul(0x41C6_4E6D).wrapping_add(0x3039);
        ((self.seed >> 16) & 0x7FFF) as u16
    }
    pub fn below(&mut self, n: u16) -> u16 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
}

/// Stand-in for Library 4.0's `RawRandom`/`RandomLong` (Knuth 55-entry
/// lagged-Fibonacci). Statistically fine; TODO: swap in the real generator
/// for bit-authentic replay.
pub struct RandomLong {
    s: u64,
}

impl RandomLong {
    /// xorshift only needs a nonzero state: every nonzero seed is its own
    /// stream (odd seeds unchanged from the old `seed | 1`), 0 gets a constant.
    pub fn new(seed: u64) -> Self {
        Self { s: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed } }
    }
    pub fn next(&mut self) -> u32 {
        self.s ^= self.s << 13;
        self.s ^= self.s >> 7;
        self.s ^= self.s << 17;
        (self.s >> 16) as u32 & 0x7FFF_FFFF
    }
    /// `RandomLong() % n` as the modules write it.
    pub fn pct(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
}

// ---------------------------------------------------------------------------
// Asset packs

#[derive(Deserialize, Clone)]
pub struct FrameRef {
    pub png: String,
    /// Link offset applied when ENTERING this frame (cumulative along a run).
    pub dx: i32,
    pub dy: i32,
    /// Compound bounds origin relative to the series origin.
    pub bx: i32,
    pub by: i32,
    /// Compound bounds size (the PNG's own dimensions) — what the original's
    /// GetBounds-style metrics see (Phlegm Boy's `+0xDE`/`+0xE0`). Packs
    /// written before 2026-09-12 lack these and read back as 0.
    #[serde(default)]
    pub w: i32,
    #[serde(default)]
    pub h: i32,
    /// The compound's part table, `[art, channel, flip bits, l, t, r, b]`
    /// per part in bank space (packs written before 2026-09-14 lack it).
    #[serde(default)]
    pub parts: Vec<[i32; 7]>,
}

#[derive(Deserialize, Clone)]
pub struct Sequence {
    pub first: u32,
    pub frames: Vec<FrameRef>,
}

#[derive(Deserialize, Clone)]
pub struct Meta {
    pub module: String,
    pub field: [u8; 3],
    /// series base (as string in JSON) -> sequences
    pub series: HashMap<String, Vec<Sequence>>,
    /// clut id -> {color-table SLOT -> 8-bit RGB}, from the module's 'clut'
    /// resources. Mac cluts are slot-addressed and may be sparse (they
    /// redefine specific device slots, not a contiguous 0..N run), so the key
    /// is the resource's own index — never the entry's position in the file.
    #[serde(default)]
    pub palettes: HashMap<String, HashMap<u16, [u8; 3]>>,
    /// The clut id whose slots the packed art is baked against: the device
    /// palette in effect when the compounds were rendered. `None` means the
    /// pack does no runtime recolour and `palettes` is informational only.
    #[serde(default)]
    pub base_clut: Option<String>,
    /// The RLEP bank's own CTAB, per series base — dense, indexed by PIXEL
    /// value (`ctab[pixel]`), which is what the compound PNGs were rendered
    /// with. Provenance only: a CTAB position is a pixel index, not a device
    /// palette slot, so this must never be zipped against a clut. Runtime
    /// recolour goes through `base_clut` + `palettes` instead.
    #[serde(default)]
    pub baked: HashMap<String, Vec<[u8; 3]>>,
    /// STR# resource id -> its strings, in order.
    #[serde(default)]
    pub strings: HashMap<String, Vec<String>>,
    /// The module's own help blurb: its `TEXT` 1000 resource, the paragraph
    /// After Dark 3.0's control panel printed under the module's controls
    /// (what it is, and what each knob does). Mac Roman and CR line endings
    /// in the resource; the packers normalise both, so this is plain UTF-8
    /// with LF. Empty on a pack written before 2026-09-19 and on any module
    /// with no `TEXT` 1000.
    #[serde(default)]
    pub help: String,
    /// `sUnt` resource id -> its `(raw value, word)` rows, in resource
    /// order: the words After Dark's control panel printed under a slider
    /// instead of numbers. Empty on a pack ripped before 2026-09-29.
    #[serde(default)]
    pub slider_words: HashMap<String, Vec<(i32, String)>>,
    /// `MENU` resource id -> its item texts in order, `-` separators kept:
    /// the control panel's popup menus. Empty on a pack ripped before
    /// 2026-09-29.
    #[serde(default)]
    pub menus: HashMap<String, Vec<String>>,
    /// After Dark's MIDI background music (pack v4): the module's `cmid`
    /// song(s), the shared bank's `INST` patch map and the `csnd` samples
    /// it names. Empty on a v3 pack and on every module with no Music
    /// control. See [`music`] and `tools/pack_assets.py`'s MUSIC docstring.
    #[serde(default)]
    pub music: music::MusicMeta,
}

pub struct Image {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// Build the runtime LoadCLUT tables, keyed by clut id.
///
/// Berkeley's `LoadCLUT(target)` swaps the *device* colour table: slot `i`
/// now holds `target[i]`. Our compounds are baked to true RGB against
/// `meta.base_clut`, so the same operation is the RGB substitution
///
/// ```text
///   base_clut[slot]  ->  target[slot]      for every slot the target defines
/// ```
///
/// and every other colour passes through untouched — which is exactly how a
/// sparse clut behaves on the Mac: slots it does not redefine keep the
/// device colour they already had.
///
/// Two things this must NOT do, both of which were the old bug:
/// * it must not index a clut by an entry's position in the resource (Mac
///   cluts carry a real slot per entry and may be sparse), and
/// * it must not use a bank CTAB as the slot space. A CTAB position is a
///   PIXEL index into that one bank; different banks in the same pack have
///   different CTABs of different lengths, and a bank CTAB is typically the
///   union of every palette the art can wear (chameleon's bank 1000 holds
///   150 colours = all nine Cham schemes), so CTAB position and clut slot are
///   unrelated.
///
/// A clut that is the base itself, or that reproduces the base colour for
/// every slot it defines, yields an empty table and is left out entirely —
/// `remap()` returns `None` and the caller skips the pixel pass.
fn build_remaps(meta: &Meta) -> HashMap<u16, HashMap<[u8; 3], [u8; 3]>> {
    let mut remaps = HashMap::new();
    let Some(base_id) = meta.base_clut.as_deref() else { return remaps };
    let Some(base) = meta.palettes.get(base_id) else { return remaps };
    for (id, target) in &meta.palettes {
        let Ok(id_num) = id.parse::<u16>() else { continue };
        if id == base_id {
            continue; // loading the base palette back is the identity
        }
        let m = slot_remap(base, target);
        if !m.is_empty() {
            remaps.insert(id_num, m);
        }
    }
    remaps
}

/// The one LoadCLUT substitution rule, shared by the pack's static cluts
/// ([`build_remaps`]) and a module's materialised per-sprite palettes
/// ([`Pack::remap_clut`]): `base[slot] -> target[slot]` for every slot the
/// target defines, identity pairs dropped. Ascending slot order so a base
/// clut that repeats a colour resolves deterministically: the lowest slot
/// holding that colour wins.
fn slot_remap(
    base: &HashMap<u16, [u8; 3]>,
    target: &HashMap<u16, [u8; 3]>,
) -> HashMap<[u8; 3], [u8; 3]> {
    let mut slots: Vec<(&u16, &[u8; 3])> = base.iter().collect();
    slots.sort_by_key(|(s, _)| **s);
    let mut m: HashMap<[u8; 3], [u8; 3]> = HashMap::new();
    for (slot, from) in &slots {
        let Some(to) = target.get(slot) else { continue };
        if *to == **from {
            continue;
        }
        m.entry(**from).or_insert(*to);
    }
    m
}

/// Duration of a packed WAV in ms.
fn wav_duration_ms(path: &Path) -> u64 {
    let Ok(b) = std::fs::read(path) else { return 0 };
    if b.len() < 44 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return 0;
    }
    let mut i = 12usize;
    let mut byte_rate = 0u32;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let n = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        if id == b"fmt " && i + 20 <= b.len() {
            byte_rate = u32::from_le_bytes([b[i + 16], b[i + 17], b[i + 18], b[i + 19]]);
        } else if id == b"data" {
            return if byte_rate > 0 { (n as u64 * 1000) / byte_rate as u64 } else { 0 };
        }
        i += 8 + n + (n & 1);
    }
    0
}

/// First `n` of `items`, then "k" (1-based) for every slot the pack could
/// not fill.
fn pad_items(items: impl Iterator<Item = String>, n: usize) -> Vec<String> {
    let mut out: Vec<String> = items.take(n).collect();
    while out.len() < n {
        out.push((out.len() + 1).to_string());
    }
    out
}

/// A loaded asset pack. Cheap to clone: the parsed `meta.json` and the clut
/// remaps are shared (`Arc`), because every module keeps its own clone of
/// the pack and a host may run several instances of one module in a process
/// (the macOS saver's host keeps three or four views alive) — before this,
/// each clone was a deep copy of up to 3 MB of parsed JSON.
#[derive(Clone)]
pub struct Pack {
    pub meta: Arc<Meta>,
    root: PathBuf,
    /// clut id -> baked-RGB -> clut-RGB (built once at load).
    remaps: Arc<HashMap<u16, HashMap<[u8; 3], [u8; 3]>>>,
    sound_lengths: Arc<Mutex<HashMap<u32, u64>>>,
}

impl Pack {
    pub fn load(dir: &Path) -> std::io::Result<Pack> {
        let meta: Meta = serde_json::from_str(&std::fs::read_to_string(dir.join("meta.json"))?)?;
        Ok(Pack::from_meta(meta, dir))
    }

    /// A pack over an already-parsed `meta` whose files live under `dir` —
    /// what [`Pack::load`] does after reading `meta.json`. Lets a test run a
    /// module against a pack with pieces of its metadata removed.
    pub fn from_meta(meta: Meta, dir: &Path) -> Pack {
        Pack {
            remaps: Arc::new(build_remaps(&meta)),
            meta: Arc::new(meta),
            root: dir.to_path_buf(),
            sound_lengths: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Remap table for a clut id: baked RGB -> recolored RGB. `None` when the
    /// clut changes nothing (see [`build_remaps`]); the caller then draws the
    /// compound unmodified.
    pub fn remap(&self, clut: u16) -> Option<&HashMap<[u8; 3], [u8; 3]>> {
        self.remaps.get(&clut)
    }

    /// Remap table for a module-materialised palette (see [`DYN_PAL`]):
    /// `clut` is slot -> RGB exactly like an entry of `meta.palettes`, and
    /// the table is built by the same slot rule as [`Pack::remap`]'s. Empty
    /// when the pack declares no `base_clut` or the palette changes nothing.
    pub fn remap_clut(&self, clut: &HashMap<u16, [u8; 3]>) -> HashMap<[u8; 3], [u8; 3]> {
        let base = self.meta.base_clut.as_deref().and_then(|b| self.meta.palettes.get(b));
        match base {
            Some(base) => slot_remap(base, clut),
            None => HashMap::new(),
        }
    }

    /// The module's help blurb (`TEXT` 1000), or `""` if the pack has none.
    /// The macOS saver's control panel shows it; nothing in the engine reads
    /// it, so it is carried, not interpreted.
    pub fn help(&self) -> &str {
        &self.meta.help
    }

    /// STR# strings by resource id.
    pub fn strings(&self, id: u16) -> &[String] {
        self.meta
            .strings
            .get(&id.to_string())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// `sUnt` slider words by resource id: `(raw value, word)` rows, or an
    /// empty slice when the pack has none.
    pub fn slider_words(&self, id: u16) -> &[(i32, String)] {
        self.meta
            .slider_words
            .get(&id.to_string())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// `MENU` items by resource id, separators (`-`) included, or an empty
    /// slice when the pack has none.
    pub fn menu(&self, id: u16) -> &[String] {
        self.meta
            .menus
            .get(&id.to_string())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Popup items for a control whose panel menu is `MENU id`, always
    /// exactly `n` long: a pack without that menu (or with a shorter one)
    /// fills the gap with the 1-based item number, so the control's raw
    /// range never depends on the pack. `drop_separators` removes the `-`
    /// rows first, for a module that folds them out of its numbering.
    pub fn popup_items(&self, id: u16, n: usize, drop_separators: bool) -> Vec<String> {
        let items = self.menu(id).iter().filter(|s| !(drop_separators && s.as_str() == "-"));
        pad_items(items.cloned(), n)
    }

    /// Popup items taken from the words of `sUnt id` (a control the
    /// original drew as a slider and this port draws as a popup), padded
    /// the same way as [`Pack::popup_items`].
    pub fn popup_items_from_slider_words(&self, id: u16, n: usize) -> Vec<String> {
        pad_items(self.slider_words(id).iter().map(|(_, w)| w.clone()), n)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Sequences of one series; panics with a clear message if absent.
    pub fn series(&self, base: u32) -> &[Sequence] {
        self.meta
            .series
            .get(&base.to_string())
            .unwrap_or_else(|| panic!("pack has no series {base}"))
    }

    /// Sequence within a series whose run starts at `first` (OFst frameNum).
    pub fn seq(&self, base: u32, first: u32) -> Option<&Sequence> {
        self.series(base).iter().find(|s| s.first == first)
    }

    /// Length of the run starting at `first` (the specs' `SeqLen`).
    pub fn seq_len(&self, base: u32, first: u32) -> u32 {
        self.seq(base, first).map(|s| s.frames.len() as u32).unwrap_or(1)
    }

    /// Frame by absolute frameNum within a series.
    pub fn frame(&self, base: u32, fno: u32) -> Option<&FrameRef> {
        self.series(base).iter().find_map(|s| {
            (s.first..s.first + s.frames.len() as u32)
                .contains(&fno)
                .then(|| &s.frames[(fno - s.first) as usize])
        })
    }

    pub fn image(&self, rel: &str) -> Image {
        let f = std::fs::File::open(self.root.join(rel)).expect("compound png");
        let mut reader = png::Decoder::new(f).read_info().expect("png info");
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).expect("png frame");
        buf.truncate(info.buffer_size());
        Image { w: info.width, h: info.height, rgba: buf }
    }

    /// Path of a packed sound by snd resource id, if present.
    pub fn sound(&self, id: u32) -> Option<PathBuf> {
        let p = self.root.join("sounds").join(format!("{id}.wav"));
        p.exists().then_some(p)
    }

    /// Duration of a packed sound in milliseconds (the sound-bank busy model).
    pub fn sound_ms(&self, id: u32) -> u64 {
        if let Ok(guard) = self.sound_lengths.lock() {
            if let Some(&ms) = guard.get(&id) {
                return ms;
            }
        }
        let Some(p) = self.sound(id) else { return 0 };
        let ms = wav_duration_ms(&p);
        if let Ok(mut guard) = self.sound_lengths.lock() {
            guard.insert(id, ms);
        }
        ms
    }

    /// Whether the pack carries song `id` (a `cmid` tune). Modules use this
    /// the way they use [`Pack::sound`]: assert the asset before claiming
    /// the cue, so a v3 pack degrades to the old silence instead of
    /// panicking.
    pub fn has_song(&self, id: u32) -> bool {
        self.meta.music.songs.contains_key(&id.to_string())
    }

    /// End-of-track of song `id` in milliseconds, straight off the packed
    /// SMF — the real thing, where the modules used to carry a guess
    /// (coming soon's `TUNE_MS = 30_000` against a 33.00 s tune,
    /// frankenscreen's "nominal 60 s song length" against a 31.20 s one).
    /// A module asks this instead of hard-coding a length so the replay
    /// cadence follows the asset.
    pub fn song_length_ms(&self, id: u32) -> Option<u64> {
        self.meta.music.songs.get(&id.to_string()).map(|s| s.length_ms)
    }

    /// Decode this pack's music: songs + the shared instrument bank.
    /// `None` when the pack has none. Costs a few MB of samples, so the
    /// shell does it once per module load, not per tick.
    pub fn music(&self) -> Option<music::MusicAssets> {
        music::MusicAssets::load(&self.root, &self.meta.music)
    }
}

// ---------------------------------------------------------------------------
// Module contract

pub const SCREEN_W: i32 = 640;
pub const SCREEN_H: i32 = 480;

/// One drawable: a packed png placed at screen coordinates.
///
/// `pal` selects a runtime clut swap (chameleon §2.1/§6.6): 0 = the baked
/// palette; otherwise a clut id from `meta.palettes`, applied as the
/// slot-indexed substitution `base_clut[slot] -> clut[slot]` (see
/// [`build_remaps`]) — Berkeley's LoadCLUT recolour-in-place. A clut that
/// redefines nothing relative to the pack's base palette is a no-op, so a
/// module may set `pal` faithfully even where the recolour is invisible
/// (toxic-swamp re-pushes its own boot palette every frame).
/// Marker prefix for a sprite the MODULE supplies as pixels instead of the
/// pack supplying it as a file.
///
/// Several modules draw art the rip does not contain: message-mayhem's solid
/// ink strips, shock-clocks' code-drawn dial hands and its recoloured LCD
/// cells, voyeur's 1x1 stars and its cropped wall. They used to materialise
/// that art by WRITING PNGs into the pack directory at build()/draw time and
/// then naming them by path. In the dev tree the write succeeds, so it looked
/// fine; inside an installed `.saver` the bundle is read-only and every such
/// write fails, whereupon the module silently drew nothing (or, for
/// message-mayhem, fell back to a different scene). Toxic-swamp's mirrored
/// frames were the same bug and were what "the fish swim backwards" turned
/// out to be.
///
/// So generated art now travels as a NAME, never a path: a [`SpriteDraw::png`]
/// beginning with `gen:` is resolved by [`Module::generated`] rather than by
/// opening a file. The name is the whole argument — it must encode everything
/// needed to rebuild those pixels — because it is also the cache key, so the
/// image is decoded/rasterised exactly once per `(name, pal)` just like a
/// packed compound.
pub const GEN_PREFIX: &str = "gen:";

/// First [`SpriteDraw::pal`] value of the module-owned DYNAMIC palette
/// range. A `pal` at or above this is not a clut id: it is a handle the
/// compose pass resolves through [`Module::dyn_palette`] on every draw,
/// because the colours behind it change while the handle does not.
///
/// Why it exists: chameleon's camouflage (`M130_fn17` @057E) interpolates
/// each lizard's 20-entry colour table one step per ~500 ms, and every
/// lizard carries its own table (its own RLE sequence's colour lookup), so
/// a sprite's colours are neither a pack clut nor shared between sprites —
/// a clut id cannot name them. No pack clut id reaches this range (the
/// highest in use is coming-soon's 20000 + movie).
pub const DYN_PAL: u16 = 0xFF00;

/// Whether a [`SpriteDraw::pal`] is a [`DYN_PAL`] handle rather than a
/// clut id.
#[inline]
pub fn is_dyn_pal(pal: u16) -> bool {
    pal >= DYN_PAL
}

/// Whether a [`SpriteDraw::png`] names module-generated pixels (see
/// [`GEN_PREFIX`]) rather than a file inside the pack.
#[inline]
pub fn is_generated(png: &str) -> bool {
    png.starts_with(GEN_PREFIX)
}

#[derive(Clone, Debug)]
pub struct SpriteDraw {
    pub png: String,
    pub x: i32,
    pub y: i32,
    /// 0 = baked colours; a clut id = that clut's LoadCLUT substitution;
    /// at or above [`DYN_PAL`] = a module-owned per-sprite palette handle.
    pub pal: u16,
    /// Mirror the image left-to-right about its own box (the RLE bank's
    /// per-frame flip bit, toggled through the sequence's `+0x0C` slot).
    pub flip: bool,
}

impl Default for SpriteDraw {
    fn default() -> Self {
        SpriteDraw { png: String::new(), x: 0, y: 0, pal: 0, flip: false }
    }
}

/// Whether modules draw their STR# quip captions / AD message-line text.
/// OFF is the verified original behaviour: the Basilisk golden captures
/// (2026-08-30, Phlegm Boy at max Behavior for 45 s, Message Mayhem demos)
/// show NO caption text on screen even while gags visibly fire — the STR#
/// strings and caption machinery exist in the binaries but the modules
/// never render them in a normal run. Kept as a switch, not deleted: the
/// machinery is real, and if a later capture shows a mode that does draw
/// them (After Dark's message-line feature?) flip it or make it a control.
/// How much of the Mac's display gamma to apply when presenting a frame:
/// 1.0 = exactly what the QEMU goldens show, 0.0 = the raw clut colours.
/// Jason's call (2026-09-29): match the capture, tunable by eye.
pub const DISPLAY_GAMMA_STRENGTH: f32 = 1.0;

/// The Mac video driver's gamma table, per channel, sampled at the sixteen
/// `0x11` steps (clut value -> DAC value). Read off lossless QEMU q800
/// screendumps of the standard system clut (Finder): 0x44 -> 0x66 etc. It
/// is Apple's Std Gamma correction for a 2.5-gamma CRT, close to a power
/// curve of exponent 0.69. The art and every clut in the pack stay RAW —
/// this is applied once, after compose and after any LoadCLUT remap, so
/// the exact-RGB substitutions still match.
const DISPLAY_GAMMA_POINTS: [u8; 16] = [
    0x00, 0x27, 0x3f, 0x54, 0x66, 0x77, 0x87, 0x96, 0xa5, 0xb3, 0xc0, 0xcd, 0xda, 0xe7, 0xf3,
    0xff,
];

/// The 256-entry gamma table at `strength` (0 = identity, 1 = the Mac's):
/// linear between the sixteen measured points, so every `0x11` step is
/// exact and in-between values (5-bit direct colour, blended cluts) land
/// within a count of the curve.
pub fn display_gamma_table(strength: f32) -> [u8; 256] {
    let mut t = [0u8; 256];
    for (v, slot) in t.iter_mut().enumerate() {
        let seg = (v / 17).min(14);
        let lo = f32::from(DISPLAY_GAMMA_POINTS[seg]);
        let hi = f32::from(DISPLAY_GAMMA_POINTS[seg + 1]);
        let g = lo + (hi - lo) * ((v - seg * 17) as f32 / 17.0);
        let out = v as f32 + strength * (g - v as f32);
        *slot = out.round().clamp(0.0, 255.0) as u8;
    }
    t
}

/// [`display_gamma_table`] at [`DISPLAY_GAMMA_STRENGTH`], built once.
pub fn display_gamma() -> &'static [u8; 256] {
    static T: std::sync::OnceLock<[u8; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| display_gamma_table(DISPLAY_GAMMA_STRENGTH))
}

pub const SHOW_QUIP_CAPTIONS: bool = false;

/// Caption ink that reads against a module's field colour: black on light
/// fields, white on dark ones. After Dark blanks the screen to black, so
/// most modules caption in light-on-dark; the engine font stands in for the
/// original Mac text draw and its ink must contrast the field the same way.
pub fn contrast_ink(field: [u8; 3]) -> [u8; 3] {
    // Rec. 601 luma, integer form.
    let luma = 299 * field[0] as u32 + 587 * field[1] as u32 + 114 * field[2] as u32;
    if luma >= 128_000 { [0, 0, 0] } else { [255, 255, 255] }
}

/// One filled rectangle, drawn after the field clear and before sprites.
/// Stands in for QuickDraw PaintRect fills the original modules used for
/// solid-colour panels (Coming Soon's per-movie card, clut 20000+m slot 0).
#[derive(Clone, Copy)]
pub struct RectDraw {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub color: [u8; 3],
}

/// One text run drawn with the engine's 8x8 bitmap font (bit 0 = leftmost
/// pixel of each glyph row; 8 px advance per char before scaling).
#[derive(Clone)]
pub struct TextDraw {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub color: [u8; 3],
    pub scale: u32,
}

pub enum ControlKind {
    /// Berkeley slider: raw 0..100 with labeled feel; modules map raw
    /// values exactly as the disasm does.
    Slider { min: i32, max: i32 },
    /// Popup menu of labeled choices. `items[n]` is the raw value
    /// `base + n`: most modules fold the Mac menu's 1-based item number down
    /// to a 0-based index in `set_control` (message-mayhem's Style: "raw 3 →
    /// popup index 2"), but some keep the original numbering — bungee
    /// roulette's Jumper is the 1-based `mVal 1000` menu the disasm holds,
    /// and `derive_buckets` does `jumper_sel.clamp(1, 6) - 1`.
    ///
    /// The base is part of the *definition* because everything outside the
    /// module — the player's title bar, the saver's settings sheet, the C
    /// ABI's `rtw_control_min` — has only the def to go on, and guessing
    /// wrong ticks the wrong item and runs the wrong choice. Build one with
    /// [`ControlKind::popup`] / [`ControlKind::popup_based`].
    Popup { items: Vec<String>, base: i32 },
    Checkbox,
}

impl ControlKind {
    /// A 0-based popup (what most modules' `set_control` expects).
    pub fn popup<I: IntoIterator<Item = S>, S: Into<String>>(items: I) -> ControlKind {
        ControlKind::popup_based(0, items)
    }

    /// A popup whose first item is the raw value `base`.
    pub fn popup_based<I: IntoIterator<Item = S>, S: Into<String>>(
        base: i32,
        items: I,
    ) -> ControlKind {
        ControlKind::Popup { items: items.into_iter().map(Into::into).collect(), base }
    }
}

impl ControlDef {
    /// Inclusive legal raw range: sliders carry their own, a checkbox is
    /// 0/1, a popup runs `base ..= base + items - 1`.
    pub fn range(&self) -> (i32, i32) {
        match &self.kind {
            ControlKind::Slider { min, max } => (*min, *max),
            ControlKind::Checkbox => (0, 1),
            ControlKind::Popup { items, base } => {
                (*base, *base + (items.len() as i32 - 1).max(0))
            }
        }
    }
}

pub struct ControlDef {
    pub name: String,
    pub kind: ControlKind,
    pub default: i32,
}

// ---------------------------------------------------------------------------
// Time base

/// One Mac tick in microseconds. After Dark's clock is the 60.15 Hz VBL
/// counter, which every module spec writes as 16.625 ms.
pub const MAC_TICK_US: u64 = 16_625;

/// After Dark's millisecond clock, `Resource.f4724()`, at tick `t`.
///
/// `CODE 132 @0x4724` computes `TickCount() * 16.625` in *integer*
/// arithmetic — `(t << 4) + (t * 0xA006 >> 16)` (bungee-roulette.md §0) — so
/// the value a module stores in a deadline and compares against is the
/// truncation of 16.625 t, never a fraction:
///
/// ```text
///   t:  1   2   3   4   5   6    7    8    9
///  ms: 16  33  49  66  83  99  116  133  149
/// ```
///
/// That truncation is not a detail, it is the observable: a `now + 100`
/// deadline compared with `>=` lands 6 ticks later six times in ten and 7
/// the rest, a 5-frame cycle averaging **106.4 ms** — which is the ~106.5 ms
/// period the 2026-09-12 captures measure for mikes / phlegm-boy /
/// toxic-swamp / chameleon. The same deadline compared with `>` lands on 7
/// ticks flat = **116.4 ms**, which is frankenscreen's measured 116.8. A
/// float clock would collapse both families onto 116.4.
///
/// (`0xA006 / 65536 = 0.625061`, so the original's clock also gains ~1 ms
/// per 4.5 minutes against a true 16.625 grid. Reproduced, it is free.)
pub fn mac_clock_ms(ticks: u64) -> u64 {
    (ticks << 4) + ((ticks.wrapping_mul(0xA006)) >> 16)
}

/// The grid a module's ticks sit on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TickClock {
    /// A plain millisecond period: tick `t` is at `t * ms`. The engine
    /// default (40 ms = 25 Hz) and what a module that does not gate on
    /// `Resource.f4724` wants.
    Millis(u64),
    /// After Dark's own clock: tick `t` puts `Ctx::now_ms` at
    /// [`mac_clock_ms(t)`](mac_clock_ms) and runs 16.625 ms of wall time
    /// later. Modules on this clock keep their disasm'd delay constants in
    /// milliseconds and let the grid quantize them.
    MacTick,
}

impl TickClock {
    /// `Ctx::now_ms` at tick `t` — what the module's deadlines compare to.
    pub fn now_ms(self, tick: u64) -> u64 {
        match self {
            TickClock::Millis(ms) => tick.saturating_mul(ms.max(1)),
            TickClock::MacTick => mac_clock_ms(tick),
        }
    }

    /// Wall-clock offset of tick `t` from module start, in microseconds.
    /// This is the *pacing* grid and is exact; `now_ms` is what the module
    /// sees, and on the Mac clock that is the truncation of this.
    pub fn at_us(self, tick: u64) -> u64 {
        match self {
            TickClock::Millis(ms) => tick.saturating_mul(ms.max(1) * 1000),
            TickClock::MacTick => tick.saturating_mul(MAC_TICK_US),
        }
    }

    /// Nominal tick period in microseconds (host timer interval).
    pub fn period_us(self) -> u64 {
        match self {
            TickClock::Millis(ms) => ms.max(1) * 1000,
            TickClock::MacTick => MAC_TICK_US,
        }
    }
}

/// Per-tick services handed to a module.
pub struct Ctx {
    pub rng: RandomLong,
    pub rng15: Random15,
    /// snd resource ids fired this tick; the shell plays sounds/<id>.wav.
    pub sounds: Vec<u32>,
    /// Caps Lock state (several modules count it for easter eggs).
    pub caps_lock: bool,
    /// Milliseconds since module start (the engine clock modules compare
    /// deadlines against).
    pub now_ms: u64,
    /// Local wall-clock (hour, minute, second) — Shock Clocks tells REAL time.
    pub local_hms: (u8, u8, u8),
    /// Pointer position in sim coordinates (640×480 space).
    pub mouse: (i32, i32),
    /// Left mouse button held (Coming Soon click-to-skip, §5/§13.6).
    pub mouse_down: bool,
}

pub trait Module {
    fn name(&self) -> &'static str;
    fn controls(&self) -> Vec<ControlDef>;
    fn set_control(&mut self, index: usize, value: i32);
    fn tick(&mut self, ctx: &mut Ctx);
    fn sprites(&self, out: &mut Vec<SpriteDraw>);
    /// Filled rects drawn after the field clear, before sprites (PaintRect
    /// panels — see RectDraw).
    fn rects(&self, out: &mut Vec<RectDraw>) {
        let _ = out;
    }
    /// Text runs drawn after sprites (captions, crawls — STR# passes).
    fn texts(&self, out: &mut Vec<TextDraw>) {
        let _ = out;
    }
    /// Field (background) color.
    fn field(&self) -> [u8; 3];
    /// Tick period in ms (most modules: 40 = 25 Hz; engine floor is 100 ms
    /// for the StateMachine modules — the spec for each module says).
    ///
    /// Only meaningful for modules on [`TickClock::Millis`]; a module that
    /// overrides [`Module::clock`] with `MacTick` does not use it.
    fn tick_ms(&self) -> u64 {
        40
    }
    /// The clock this module's ticks ride. Default: the plain `tick_ms`
    /// grid. Override with [`TickClock::MacTick`] for a module whose gates
    /// compare against `Resource.f4724()` — which is every module that
    /// carries disasm'd millisecond delay constants.
    fn clock(&self) -> TickClock {
        TickClock::Millis(self.tick_ms().max(1))
    }

    /// What the module wants the MUSIC channel doing — `None` for silence,
    /// `Some((song, play))` for "`cmid` song `song` should be sounding, and
    /// this is play number `play`".
    ///
    /// This is deliberately not a `Ctx::sounds` entry. Two reasons:
    ///
    /// * **It is a different channel.** After Dark's `MDRV` synth owns its
    ///   own `SndChannel`; the captures show cues and music overlapping with
    ///   no pre-emption in either direction (see [`music`]'s module docs).
    ///   `Ctx::sounds` *is* the single pre-empting channel, so music must
    ///   not travel through it.
    /// * **It is a state, not an event.** Every module with a Music control
    ///   already keeps its own play counter (mime hunt's `+0x1C` against the
    ///   §8 band, coming soon's `g03F2`, frankenscreen's `MG_3C8`) — the
    ///   original calls `Sound.fn_0C9C(song, …)` once per play. Exposing
    ///   "(song, plays started)" lets the shell restart the tune exactly
    ///   when the module counts a new play and never otherwise, and lets a
    ///   headless render read the same state without an audio device.
    ///
    /// The default is silence, which is correct for the ten modules that
    /// have no Music control.
    fn music(&self) -> Option<(u32, u32)> {
        None
    }

    /// What looped sound the module wants playing on the main channel —
    /// `None` for silence, `Some(id)` for sound `id` looped.
    ///
    /// One-shot cues (`Ctx::sounds`) pre-empt the loop while they play,
    /// and the loop resumes when the cue finishes.
    fn loop_sound(&self) -> Option<u32> {
        None
    }

    /// Pixels for one module-generated sprite (see [`GEN_PREFIX`]).
    ///
    /// The compose pass calls this for a [`SpriteDraw`] whose `png` starts
    /// with `gen:`, and ONLY on a cache miss: the result is stored in the
    /// same `(name, pal)` cache slot a packed compound would occupy, so a
    /// generated sprite costs one rasterise for the life of the cache and
    /// every later frame is a plain blit. `name` is therefore the only input
    /// — it has to spell out the whole recipe — and the answer must be a
    /// pure function of it (plus the module's immutable pack/font tables).
    ///
    /// `None` means "draw nothing", the same degradation a missing compound
    /// gets. The default is `None`: most modules generate no art.
    fn generated(&self, name: &str) -> Option<Image> {
        let _ = name;
        None
    }

    /// The materialised palette behind a [`DYN_PAL`] handle: clut slot ->
    /// 8-bit RGB, the same shape as an entry of `meta.palettes`, applied as
    /// the LoadCLUT substitution `base_clut[slot] -> palette[slot]`
    /// ([`Pack::remap_clut`]). Asked on EVERY draw of a sprite carrying the
    /// handle — the answer is the palette as of the current tick, so it is
    /// never cached. `None` draws the art in its baked colours. The default
    /// is `None`: only a module that emits `DYN_PAL` handles overrides it.
    fn dyn_palette(&self, pal: u16) -> Option<HashMap<u16, [u8; 3]>> {
        let _ = pal;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The textbook ANSI sequence for `srand(1)` — what seg 131 actually does.
    #[test]
    fn random15_is_ansi_rand() {
        let mut r = Random15::new(1);
        let a: Vec<u16> = (0..10).map(|_| r.next()).collect();
        assert_eq!(a, [16838, 5758, 10113, 17515, 31051, 5627, 23010, 7419, 16212, 4086]);
    }

    /// The original's integer `TickCount()*16.625`. The truncation is the
    /// whole point — see [`mac_clock_ms`].
    #[test]
    fn mac_clock_is_the_originals_integer_grid() {
        let want = [0, 16, 33, 49, 66, 83, 99, 116, 133, 149, 166];
        for (t, &ms) in want.iter().enumerate() {
            assert_eq!(mac_clock_ms(t as u64), ms, "tick {t}");
            assert_eq!(TickClock::MacTick.now_ms(t as u64), ms);
        }
        // pacing is the exact grid, not the truncation
        assert_eq!(TickClock::MacTick.at_us(7), 7 * 16_625);
        assert_eq!(TickClock::MacTick.period_us(), 16_625);
        assert_eq!(TickClock::Millis(40).now_ms(3), 120);
        assert_eq!(TickClock::Millis(40).at_us(3), 120_000);
    }

    /// How many Mac ticks a `now + delay` gate takes to fire, from tick `t0`.
    fn ticks_for(delay: u64, strict: bool, t0: u64) -> u64 {
        let target = mac_clock_ms(t0) + delay;
        let mut n = 1;
        while if strict { mac_clock_ms(t0 + n) <= target } else { mac_clock_ms(t0 + n) < target } {
            n += 1;
        }
        n
    }

    /// The three period families the 2026-09-12 captures measure, all out of
    /// one clock. This is the test the whole time base exists for.
    #[test]
    fn frame_gate_uses_the_mac_tick_grid() {
        // 40 ms -> 3 ticks = 49.875 ms, everywhere on the grid.
        // (bungee dive 50.39 / corpse fall 48.74, toilets flight 51.5)
        for t0 in 1..200 {
            assert_eq!(ticks_for(40, false, t0), 3, "40 ms from tick {t0}");
            assert_eq!(ticks_for(80, false, t0), 5, "80 ms from tick {t0}");
        }
        // 100 ms with `>=` is NOT a fixed tick count: it cycles 7,6,7,6,6 —
        // 32 ticks per 5 frames = 106.40 ms. That is the ~106.5 ms family
        // (mikes 106.5, phlegm 106.7, chameleon 107.1, toxic 107.5).
        let mut t = 1u64;
        let mut n = Vec::new();
        for _ in 0..20 {
            let k = ticks_for(100, false, t);
            n.push(k);
            t += k;
        }
        assert!(n.iter().any(|&k| k == 6) && n.iter().any(|&k| k == 7), "{n:?}");
        let total: u64 = n[5..].iter().sum();
        let mean_ms = total as f64 / (n.len() - 5) as f64 * 16.625;
        assert!((mean_ms - 106.40).abs() < 0.01, "100 ms >= gate: {mean_ms} ms");
        // The same delay compared with `>` is 7 ticks flat = 116.375 ms —
        // frankenscreen's measured 116.82.
        for t0 in 1..200 {
            assert_eq!(ticks_for(100, true, t0), 7, "100 ms strict from tick {t0}");
        }
    }

    #[test]
    fn popup_range_comes_from_the_def() {
        let zero = ControlDef {
            name: "Style".into(),
            kind: ControlKind::popup(["A", "B", "C"]),
            default: 0,
        };
        assert_eq!(zero.range(), (0, 2));
        let one = ControlDef {
            name: "One-based".into(),
            kind: ControlKind::popup_based(1, ["X", "Y"]),
            default: 1,
        };
        assert_eq!(one.range(), (1, 2));
        let slider =
            ControlDef { name: "s".into(), kind: ControlKind::Slider { min: 0, max: 90 }, default: 40 };
        assert_eq!(slider.range(), (0, 90));
        let check = ControlDef { name: "c".into(), kind: ControlKind::Checkbox, default: 1 };
        assert_eq!(check.range(), (0, 1));
    }

    fn meta_from(json: &str) -> Meta {
        serde_json::from_str(json).expect("meta")
    }

    /// The remap must pair colours by clut SLOT, and must survive sparse
    /// cluts (slot 7 defined, slots 1/2 not) — a positional zip would pair
    /// slot 5's colour with slot 0's replacement.
    #[test]
    fn remap_is_slot_indexed_and_sparse_safe() {
        let meta = meta_from(
            r#"{"module":"m","field":[0,0,0],"series":{},
                "base_clut":"1500",
                "palettes":{
                  "1500":{"0":[51,255,0],"5":[0,85,0],"7":[1,2,3]},
                  "1504":{"0":[255,102,255],"5":[153,0,204]}
                }}"#,
        );
        let m = build_remaps(&meta);
        let r = m.get(&1504).expect("clut 1504 remaps something");
        assert_eq!(r.get(&[51, 255, 0]), Some(&[255, 102, 255]), "slot 0");
        assert_eq!(r.get(&[0, 85, 0]), Some(&[153, 0, 204]), "slot 5");
        // slot 7 is not redefined by 1504: the colour passes through.
        assert_eq!(r.get(&[1, 2, 3]), None, "undefined slot must pass through");
        assert_eq!(r.len(), 2);
    }

    /// Loading the base palette back, or a clut that repeats it, changes
    /// nothing — no table at all, so the draw path skips the pixel pass.
    #[test]
    fn remap_identity_cluts_are_dropped() {
        let meta = meta_from(
            r#"{"module":"m","field":[0,0,0],"series":{},
                "base_clut":"1200",
                "palettes":{
                  "1200":{"0":[1,1,1],"1":[2,2,2]},
                  "1216":{"0":[1,1,1],"1":[2,2,2]},
                  "1300":{"1":[9,9,9]}
                }}"#,
        );
        let m = build_remaps(&meta);
        assert!(m.get(&1200).is_none(), "base clut is the identity");
        assert!(m.get(&1216).is_none(), "clut equal to the base is the identity");
        assert_eq!(m.get(&1300).and_then(|r| r.get(&[2, 2, 2])), Some(&[9, 9, 9]));
    }

    /// No base palette declared = no runtime recolour; `palettes` alone must
    /// never be enough to invent one (that was the old positional-zip bug).
    #[test]
    fn remap_needs_a_base_clut() {
        let meta = meta_from(
            r#"{"module":"m","field":[0,0,0],"series":{},
                "baked":{"10000":[[1,1,1],[2,2,2]]},
                "palettes":{"20000":{"0":[7,7,7],"1":[8,8,8]}}}"#,
        );
        assert!(build_remaps(&meta).is_empty());
    }

    /// A module-materialised palette goes through the SAME slot rule as a
    /// pack clut: slot-paired against `base_clut`, identity pairs dropped,
    /// undefined slots passing through.
    #[test]
    fn display_gamma_hits_the_measured_points() {
        let t = display_gamma_table(1.0);
        for (i, &want) in DISPLAY_GAMMA_POINTS.iter().enumerate() {
            assert_eq!(t[i * 17], want, "0x{:02x}", i * 17);
        }
        assert_eq!(t[0x44], 0x66, "the toxic pipe grey the capture shows");
        assert!(t.windows(2).all(|w| w[0] <= w[1]), "monotonic");
    }

    #[test]
    fn display_gamma_strength_zero_is_raw() {
        let t = display_gamma_table(0.0);
        assert!(t.iter().enumerate().all(|(v, &g)| g as usize == v));
    }

    #[test]
    fn remap_clut_uses_the_slot_rule() {
        let meta = meta_from(
            r#"{"module":"m","field":[0,0,0],"series":{},
                "base_clut":"1500",
                "palettes":{"1500":{"0":[51,255,0],"5":[0,85,0],"7":[1,2,3]}}}"#,
        );
        let pack = Pack {
            remaps: Arc::new(build_remaps(&meta)),
            meta: Arc::new(meta),
            root: PathBuf::new(),
            sound_lengths: Arc::new(Mutex::new(HashMap::new())),
        };
        let clut: HashMap<u16, [u8; 3]> =
            [(0, [60, 250, 1]), (5, [0, 85, 0])].into_iter().collect();
        let r = pack.remap_clut(&clut);
        assert_eq!(r.get(&[51, 255, 0]), Some(&[60, 250, 1]), "slot 0 recoloured");
        assert_eq!(r.len(), 1, "slot 5 is the identity, slot 7 undefined");
        assert!(is_dyn_pal(DYN_PAL) && !is_dyn_pal(20_000 + 99));
    }
}
