//! retwisted app library: the module registry plus the one compose path
//! every front end draws through.
//!
//! There are three front ends now — the winit player (`bin/app`, i.e.
//! `main.rs`), the headless golden-frame renderer (`bin/frame`), and the
//! macOS screensaver (`saver/`, over a C ABI). They used to each carry
//! their own copy of "clear the field, paint rects, blit sprites with the
//! palette remap, draw texts". Three copies meant three chances to drift,
//! and a screensaver that quietly disagrees with the frame tool is a
//! screensaver you cannot golden-test. So the compose pass lives here and
//! they all call [`compose`].
//!
//! Nothing about the draw order is new: field clear → rects → sprites →
//! texts, exactly as `main.rs` and `bin/frame.rs` did it.

pub mod modules;

use engine::{Image, Module, Pack};
use std::collections::HashMap;

/// Every module, in control-panel order: slug, then the After Dark title
/// the module's own [`engine::Module::name`] reports.
///
/// Why a table instead of `modules::make(slug, pack).name()`: a host that
/// wants to *list* the modules — the screensaver's Module popup — has no
/// pack in hand yet, and loading thirteen `Pack`s to read thirteen strings
/// is a second of I/O to fill one menu. The table is kept honest by
/// `module_titles_are_the_modules_own_names` below, which builds every
/// module it has a pack for and compares.
pub const MODULE_TITLES: &[(&str, &str)] = &[
    ("bungee-roulette", "Bungee Roulette"),
    ("chameleon", "Chameleon"),
    ("coming-soon", "Coming Soon!"),
    ("flying-toilets", "Flying Toilets"),
    ("frankenscreen", "FrankenScreen"),
    ("message-mayhem", "Message Mayhem"),
    ("mikes-so-called-life", "Mike's So-called Life"),
    ("mime-hunt", "Mime Hunt"),
    ("mowin-boris", "Mowin' Boris"),
    ("phlegm-boy", "Phlegm Boy"),
    ("shock-clocks", "Shock Clocks"),
    ("toxic-swamp", "Toxic Swamp"),
    ("voyeur", "Voyeur"),
];

/// The display title for a slug, or `None` if it is not a module.
pub fn module_title(slug: &str) -> Option<&'static str> {
    MODULE_TITLES.iter().find(|(s, _)| *s == slug).map(|(_, t)| *t)
}

/// The `sUnt` resource id holding slider `index`'s words.
///
/// AD's control panels never printed a number on a slider. Each `sVal`
/// resource had an `sUnt` sibling holding a list of (position, string)
/// rows, and the panel spelled the words out under the track, with the band
/// name for the current setting shown as you dragged. The rippers carry
/// those rows into the pack (`meta.json` `slider_words`), so the words come
/// from the user's own originals; all that is compiled in is which resource
/// belongs to which control. Every module numbers its `sVal`/`sUnt` pairs
/// `1000 + control index`, except mike's so-called life, whose one slider is
/// `sVal`/`sUnt` 128.
pub fn slider_words_id(slug: &str, index: usize) -> u16 {
    match slug {
        "mikes-so-called-life" => 128,
        _ => 1000 + index as u16,
    }
}

/// The `sUnt` words for control `index` of a module, read from its pack:
/// `(raw threshold, word)` rows, thresholds ascending. Empty for every
/// control that is not a slider (popups, checkboxes — mowin' boris ships
/// `sUnt` rows for two controls this port draws as popups, and those stay
/// wordless here), for a module with no controls, and for a pack ripped
/// before the rippers carried `sUnt`.
///
/// Not every band boundary is a *behaviour* boundary: the module's own
/// bucket arithmetic is its own (bungee's Jumps is `raw/20`, phlegm boy's
/// Behavior is a seven-way gate), and where the two disagree the `sUnt`
/// table is what the original's panel said, which is what a panel should
/// say.
pub fn slider_bands<'p>(
    pack: &'p Pack,
    slug: &str,
    controls: &[engine::ControlDef],
    index: usize,
) -> &'p [(i32, String)] {
    match controls.get(index).map(|c| &c.kind) {
        Some(engine::ControlKind::Slider { .. }) => {
            pack.slider_words(slider_words_id(slug, index))
        }
        _ => &[],
    }
}

/// Which band `raw` falls in: the last threshold at or below it, and the
/// first band for anything under the first threshold (several tables start
/// at 10 or 20 while the slider starts at 0 — AD's panel still showed the
/// first word there, and the module's own arithmetic agrees: frankenscreen
/// Lifespan `< 20` is "Brief", whose tick is at 10). `None` only when the
/// control has no words.
pub fn slider_band_index<S>(bands: &[(i32, S)], raw: i32) -> Option<usize> {
    if bands.is_empty() {
        return None;
    }
    Some(bands.iter().rposition(|(at, _)| raw >= *at).unwrap_or(0))
}

pub const SIM_W: usize = engine::SCREEN_W as usize;
pub const SIM_H: usize = engine::SCREEN_H as usize;
/// Length of the composed frame buffer, in `u32` pixels.
pub const SIM_PIXELS: usize = SIM_W * SIM_H;

/// Decoded compound PNGs, keyed by (relative path, clut id). The clut id is
/// part of the key because the palette remap is baked into the decoded
/// pixels — the same compound under two cluts is two images.
pub type ImageCache = HashMap<(String, u16), Image>;

/// 0RGB, which is what softbuffer wants and what the PNG writer and the
/// screensaver's CGImage (byteOrder32Little + noneSkipFirst) both read back.
#[inline]
pub fn pack_rgb(c: [u8; 3]) -> u32 {
    (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

/// Decode one compound and apply the runtime LoadCLUT remap (chameleon
/// §2.1/§6.6): an exact per-colour substitution, colours outside the baked
/// palette passing through. `None` when the PNG is missing or undecodable —
/// a missing compound must degrade to "that sprite is not drawn", never to
/// a dead front end.
fn load_image(pack: &Pack, png: &str, pal: u16) -> Option<Image> {
    let f = std::fs::File::open(pack.root().join(png)).ok()?;
    let mut reader = png::Decoder::new(f).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some(remap_image(pack, Image { w: info.width, h: info.height, rgba: buf }, pal))
}

/// Apply the runtime LoadCLUT remap in place. Split out of [`load_image`] so
/// module-generated art (see [`engine::GEN_PREFIX`]) goes through the exact
/// same palette pass a packed compound does — the cache key carries `pal`
/// either way, so the two kinds of image must mean the same thing by it.
fn remap_image(pack: &Pack, img: Image, pal: u16) -> Image {
    if pal != 0 {
        if let Some(rm) = pack.remap(pal) {
            return remap_pixels(img, rm);
        }
    }
    img
}

/// The per-colour substitution itself, shared by the cached clut path and
/// the per-draw [`engine::DYN_PAL`] path.
fn remap_pixels(mut img: Image, rm: &HashMap<[u8; 3], [u8; 3]>) -> Image {
    for px in img.rgba.chunks_exact_mut(4) {
        if let Some(c) = rm.get(&[px[0], px[1], px[2]]) {
            px[0] = c[0];
            px[1] = c[1];
            px[2] = c[2];
        }
    }
    img
}

/// Compose the module's current frame into `out` (0RGB, 640×480, row-major).
///
/// `out` must hold at least [`SIM_PIXELS`] pixels; anything past that is
/// left alone (the screensaver hands us its own buffer, sized exactly).
/// `cache` is caller-owned so a front end can drop it when it swaps module
/// or restarts — the decoded images belong to the pack, not to the frame.
/// Present a composed frame the way the Mac's video driver did: every
/// channel through the display gamma table ([`engine::display_gamma`]).
/// Call it after [`compose`] — compose repaints the whole buffer each
/// frame, so this in-place pass never compounds. Tools comparing against
/// raw pack colours skip it (`frame --raw`).
pub fn present_gamma(buf: &mut [u32]) {
    let t = engine::display_gamma();
    for px in buf.iter_mut() {
        let r = t[((*px >> 16) & 0xff) as usize] as u32;
        let g = t[((*px >> 8) & 0xff) as usize] as u32;
        let b = t[(*px & 0xff) as usize] as u32;
        *px = (*px & 0xff00_0000) | (r << 16) | (g << 8) | b;
    }
}

pub fn compose(pack: &Pack, cache: &mut ImageCache, m: &dyn Module, out: &mut [u32]) {
    assert!(
        out.len() >= SIM_PIXELS,
        "compose needs a {SIM_PIXELS}-pixel buffer, got {}",
        out.len()
    );
    let out = &mut out[..SIM_PIXELS];

    out.fill(pack_rgb(m.field()));

    // rects: QuickDraw PaintRect panels, after the clear and before sprites
    let mut rects: Vec<engine::RectDraw> = Vec::new();
    m.rects(&mut rects);
    for r in &rects {
        let c = pack_rgb(r.color);
        for y in r.y.max(0)..(r.y + r.h).min(SIM_H as i32) {
            for x in r.x.max(0)..(r.x + r.w).min(SIM_W as i32) {
                out[y as usize * SIM_W + x as usize] = c;
            }
        }
    }

    let mut sprites: Vec<engine::SpriteDraw> = Vec::new();
    m.sprites(&mut sprites);
    for s in &sprites {
        // A `DYN_PAL` handle is not a clut: its colours are the module's
        // palette as of THIS draw (chameleon's camouflage fade), so they
        // cannot live in the decode cache. The art is cached in its baked
        // colours (`pal` 0) and recoloured per draw below.
        let dyn_pal = if engine::is_dyn_pal(s.pal) { Some(m.dyn_palette(s.pal)) } else { None };
        let pal = if dyn_pal.is_some() { 0 } else { s.pal };
        let key = (s.png.clone(), pal);
        if !cache.contains_key(&key) {
            // `gen:` names module-generated pixels, not a file in the pack
            // (see `engine::GEN_PREFIX`). Either way the decode/rasterise
            // happens once and lands in the same cache slot, so a generated
            // sprite costs a packed one's blit from the second frame on —
            // and nothing has to write a PNG into a read-only bundle to get
            // there, which is what the old path did.
            let img = if engine::is_generated(&s.png) {
                m.generated(&s.png).map(|img| remap_image(pack, img, pal))
            } else {
                load_image(pack, &s.png, pal)
            };
            let Some(img) = img else { continue };
            cache.insert(key.clone(), img);
        }
        let recoloured;
        let img = match dyn_pal.flatten().map(|clut| pack.remap_clut(&clut)) {
            Some(rm) if !rm.is_empty() => {
                let src = &cache[&key];
                recoloured = remap_pixels(Image { w: src.w, h: src.h, rgba: src.rgba.clone() }, &rm);
                &recoloured
            }
            _ => &cache[&key],
        };
        for y in 0..img.h as i32 {
            let sy = s.y + y;
            if !(0..SIM_H as i32).contains(&sy) {
                continue;
            }
            for x in 0..img.w as i32 {
                let sx = s.x + x;
                if !(0..SIM_W as i32).contains(&sx) {
                    continue;
                }
                let ix = if s.flip { img.w as i32 - 1 - x } else { x };
                let p = ((y as u32 * img.w + ix as u32) * 4) as usize;
                if img.rgba[p + 3] >= 128 {
                    out[sy as usize * SIM_W + sx as usize] = (img.rgba[p] as u32) << 16
                        | (img.rgba[p + 1] as u32) << 8
                        | img.rgba[p + 2] as u32;
                }
            }
        }
    }

    // text pass (captions, crawls — STR# passes) after sprites
    let mut texts: Vec<engine::TextDraw> = Vec::new();
    m.texts(&mut texts);
    for t in &texts {
        engine::font::draw_text_u32(
            out,
            SIM_W as u32,
            SIM_H as u32,
            &t.text,
            t.x,
            t.y,
            t.color,
            t.scale,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::{ControlDef, Ctx, Image, RectDraw, SpriteDraw, TextDraw};
    use std::path::Path;

    /// A module that needs no pack art: one rect, one text, no sprites.
    struct Swatch;
    impl Module for Swatch {
        fn name(&self) -> &'static str {
            "Swatch"
        }
        fn controls(&self) -> Vec<ControlDef> {
            Vec::new()
        }
        fn set_control(&mut self, _i: usize, _v: i32) {}
        fn tick(&mut self, _c: &mut Ctx) {}
        fn sprites(&self, _out: &mut Vec<SpriteDraw>) {}
        fn rects(&self, out: &mut Vec<RectDraw>) {
            // deliberately overhangs the right/bottom edge: the clip must
            // hold, and must not wrap onto the next row.
            out.push(RectDraw { x: 620, y: 470, w: 100, h: 100, color: [255, 0, 0] });
        }
        fn texts(&self, out: &mut Vec<TextDraw>) {
            out.push(TextDraw {
                text: "HI".into(),
                x: 4,
                y: 4,
                color: [0, 255, 0],
                scale: 1,
            });
        }
        fn field(&self) -> [u8; 3] {
            [0, 0, 17]
        }
    }

    /// [`MODULE_TITLES`] exists so a host can build a menu without packs;
    /// this is the price of that shortcut being paid. Every slug in the
    /// registry must be in the table, in the same order, with the title the
    /// module itself reports. A pack that is missing locally is skipped —
    /// the slug/order half of the check still runs.
    #[test]
    fn module_titles_are_the_modules_own_names() {
        let slugs: Vec<&str> = MODULE_TITLES.iter().map(|(s, _)| *s).collect();
        assert_eq!(slugs, modules::SLUGS, "MODULE_TITLES drifted from SLUGS");
        for (slug, title) in MODULE_TITLES {
            let Ok(pack) = Pack::load(Path::new("../assets").join(slug).as_path()) else {
                continue;
            };
            let Some(m) = modules::make(slug, pack) else { continue };
            assert_eq!(&m.name(), title, "{slug}: table title vs Module::name()");
        }
    }
    /// [`slider_words_id`] is the one compiled-in fact about the `sUnt`
    /// words: which resource belongs to which control. This holds it to the
    /// packs, both ways. Every slider of every module the tree has a
    /// (current) pack for must find words, ascending and inside the slider's
    /// own range — so adding a slider whose id rule is wrong fails here
    /// instead of shipping a sheet labelled "0" and "100" — and every `sUnt`
    /// the pack carries must belong to some control of that module.
    #[test]
    fn slider_bands_match_the_modules_own_sliders() {
        use engine::ControlKind;
        let mut checked_modules = 0usize;
        let mut stale = 0usize;
        for (slug, _) in MODULE_TITLES {
            let Ok(pack) = Pack::load(Path::new("../assets").join(slug).as_path()) else {
                continue;
            };
            let Some(m) = modules::make(slug, pack.clone()) else { continue };
            let controls = m.controls();
            if pack.meta.slider_words.is_empty() && controls.iter().any(|c| matches!(c.kind, ControlKind::Slider { .. })) {
                stale += 1; // ripped before the rippers carried `sUnt`
                continue;
            }
            checked_modules += 1;
            for (i, c) in controls.iter().enumerate() {
                let bands = slider_bands(&pack, slug, &controls, i);
                let ControlKind::Slider { .. } = c.kind else {
                    assert!(bands.is_empty(), "{slug} control {i} ({}) is not a slider but has sUnt words", c.name);
                    continue;
                };
                let (lo, hi) = c.range();
                assert!(!bands.is_empty(), "{slug} slider {i} ({}) has no sUnt words in the pack", c.name);
                let mut last = i32::MIN;
                for (at, word) in bands {
                    assert!(*at > last, "{slug} {}: thresholds must ascend at {at}", c.name);
                    assert!(
                        (lo..=hi).contains(at),
                        "{slug} {}: threshold {at} outside the slider's {lo}..={hi}",
                        c.name
                    );
                    assert!(!word.is_empty(), "{slug} {}: empty word at {at}", c.name);
                    last = *at;
                }
                for v in lo..=hi {
                    let n = slider_band_index(bands, v).expect("non-empty");
                    assert!(n < bands.len(), "{slug} {}: raw {v} -> band {n}", c.name);
                }
            }
            for id in pack.meta.slider_words.keys() {
                let id: u16 = id.parse().expect("numeric sUnt id");
                assert!(
                    (0..controls.len()).any(|i| slider_words_id(slug, i) == id),
                    "{slug}: sUnt {id} belongs to no control"
                );
            }
        }
        if stale > 0 {
            eprintln!("{stale} packs predate sUnt ripping — re-rip to check them");
        }
        // Guard the guard: a tree with no packs would pass vacuously.
        assert!(checked_modules + stale >= 10, "only {} modules built — packs missing?", checked_modules + stale);
    }

    /// The floor rule, spelled out on synthetic words: below the first tick
    /// you are still in the first band, and every tick opens its own.
    #[test]
    fn slider_band_index_floors_below_the_first_tick() {
        let bands = [(10, "a"), (20, "b"), (40, "c"), (60, "d"), (80, "e")];
        assert_eq!(slider_band_index(&bands, 0), Some(0), "below the first tick is band 0");
        assert_eq!(slider_band_index(&bands, 9), Some(0));
        assert_eq!(slider_band_index(&bands, 10), Some(0));
        assert_eq!(slider_band_index(&bands, 59), Some(2));
        assert_eq!(slider_band_index(&bands, 100), Some(4));
        let top_only = [(0, "a"), (99, "b"), (100, "c")];
        assert_eq!(slider_band_index(&top_only, 100), Some(2), "a 100-only band");
        assert_eq!(slider_band_index(&top_only, 99), Some(1));
        assert_eq!(slider_band_index::<&str>(&[], 3), None, "no words, no band");
    }

    /// The words the saver shows are the pack's `sUnt` rows verbatim, and a
    /// control with none (voyeur has no controls; a popup has no words; a
    /// pack without `slider_words` has none at all) yields an empty list.
    #[test]
    fn slider_bands_are_the_packs_sunt_rows() {
        let Ok(pack) = Pack::load(Path::new("../assets/bungee-roulette")) else {
            eprintln!("assets/bungee-roulette missing — skipping");
            return;
        };
        let m = modules::make("bungee-roulette", pack.clone()).expect("make");
        let controls = m.controls();
        assert!(slider_bands(&pack, "bungee-roulette", &controls, 0).is_empty(), "Jumper is a popup");
        assert!(slider_bands(&pack, "bungee-roulette", &controls, 9).is_empty(), "no such control");
        assert_eq!(slider_bands(&pack, "bungee-roulette", &controls, 1), pack.slider_words(1001));
        assert_eq!(slider_bands(&pack, "bungee-roulette", &controls, 2), pack.slider_words(1002));
        let mut meta = (*pack.meta).clone();
        meta.slider_words.clear();
        let bare = Pack::from_meta(meta, pack.root());
        assert!(slider_bands(&bare, "bungee-roulette", &controls, 1).is_empty());
        assert_eq!(slider_words_id("mikes-so-called-life", 0), 128);
        assert_eq!(slider_words_id("shock-clocks", 1), 1001);
    }

    /// A module whose only sprite is generated: a 2x2 red block at (10, 20),
    /// counting how often the engine asks for its pixels.
    struct Gen {
        calls: std::cell::Cell<u32>,
    }

    impl Module for Gen {
        fn name(&self) -> &'static str {
            "Gen"
        }
        fn controls(&self) -> Vec<ControlDef> {
            Vec::new()
        }
        fn set_control(&mut self, _i: usize, _v: i32) {}
        fn tick(&mut self, _c: &mut Ctx) {}
        fn sprites(&self, out: &mut Vec<SpriteDraw>) {
            out.push(SpriteDraw { png: "gen:block/ff0000".into(), x: 10, y: 20, ..Default::default() });
            out.push(SpriteDraw { png: "gen:nope".into(), x: 0, y: 0, ..Default::default() });
        }
        fn field(&self) -> [u8; 3] {
            [0, 0, 0x11]
        }
        fn generated(&self, name: &str) -> Option<Image> {
            self.calls.set(self.calls.get() + 1);
            let hex = name.strip_prefix("gen:block/")?;
            let c = [
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ];
            let mut rgba = Vec::new();
            for _ in 0..4 {
                rgba.extend_from_slice(&[c[0], c[1], c[2], 0xFF]);
            }
            Some(Image { w: 2, h: 2, rgba })
        }
    }

    /// `gen:` sprites are drawn from the module's own pixels, land in the
    /// same cache as packed compounds, and are asked for exactly once — the
    /// property the whole hook exists for (no per-frame rasterise, and no
    /// PNG written into the pack to get a path to name).
    #[test]
    fn compose_draws_and_caches_generated_sprites() {
        let Some(pack) = any_pack() else {
            eprintln!("assets/bungee-roulette missing — skipping");
            return;
        };
        let m = Gen { calls: std::cell::Cell::new(0) };
        let mut cache = ImageCache::new();
        let mut buf = vec![0u32; SIM_PIXELS];
        compose(&pack, &mut cache, &m, &mut buf);
        assert_eq!(buf[SIM_W * 20 + 10], 0x00FF_0000, "generated pixels blitted");
        assert_eq!(buf[SIM_W * 21 + 11], 0x00FF_0000);
        assert_eq!(buf[SIM_W * 22 + 12], 0x00_0011, "and only its own 2x2 box");
        assert_eq!(buf[0], 0x00_0011, "a generated sprite that returns None draws nothing");
        assert_eq!(m.calls.get(), 2, "one ask per distinct name on the first frame");
        assert!(cache.contains_key(&("gen:block/ff0000".to_string(), 0)));
        compose(&pack, &mut cache, &m, &mut buf);
        compose(&pack, &mut cache, &m, &mut buf);
        assert_eq!(
            m.calls.get(),
            4,
            "the hit is cached; only the None sprite is re-asked (it caches nothing, \
             exactly like a missing compound)"
        );
    }

    fn any_pack() -> Option<Pack> {
        Pack::load(Path::new("../assets/bungee-roulette")).ok()
    }

    #[test]
    fn compose_clears_fills_and_clips() {
        let Some(pack) = any_pack() else {
            eprintln!("assets/bungee-roulette missing — skipping");
            return;
        };
        let mut cache = ImageCache::new();
        let mut buf = vec![0u32; SIM_PIXELS];
        compose(&pack, &mut cache, &Swatch, &mut buf);
        assert_eq!(buf[SIM_W * 200 + 300], 0x00_0011, "field clear");
        assert_eq!(buf[SIM_W * 479 + 639], 0x00FF_0000, "rect corner");
        assert_eq!(buf[SIM_W * 470 + 619], 0x00_0011, "rect clipped on the left edge");
        assert_eq!(buf[SIM_W * 471], 0x00_0011, "rect must not wrap to the next row");
        assert!(
            buf[..SIM_W * 12].iter().any(|&p| p == 0x00_FF00),
            "text pass ran"
        );
    }

    /// The real thing: build the shipped module and compose a frame. Guards
    /// the screensaver's whole reason for existing — a frame that comes out
    /// entirely field-coloured means nothing is being drawn.
    #[test]
    fn compose_draws_bungee_sprites() {
        let Some(pack) = any_pack() else {
            eprintln!("assets/bungee-roulette missing — skipping");
            return;
        };
        let Some(mut m) = modules::make("bungee-roulette", pack.clone()) else {
            panic!("bungee-roulette module missing")
        };
        let mut ctx = Ctx {
            rng: engine::RandomLong::new(0x5EED_CAFE),
            rng15: engine::Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut cache = ImageCache::new();
        let mut buf = vec![0u32; SIM_PIXELS];
        let field = pack_rgb(m.field());
        let mut drew = false;
        let clock = m.clock();
        for i in 1..=400u64 {
            ctx.now_ms = clock.now_ms(i);
            m.tick(&mut ctx);
            compose(&pack, &mut cache, m.as_ref(), &mut buf);
            if buf.iter().any(|&p| p != field) {
                drew = true;
                break;
            }
        }
        assert!(drew, "400 ticks of bungee-roulette drew nothing but field");
    }
}
