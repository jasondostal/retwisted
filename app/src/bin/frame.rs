//! frame: headless module renderer — runs a module N ticks and writes the
//! final composed 640×480 frame (plus optional text pass) to a PNG. The
//! golden-frame comparison harness (Basilisk II side-by-sides) and a quick
//! eyeball check for engine work both hang off this.
//!
//! Usage: cargo run -p app --bin frame -- [-c N=V ...] [--trace T.jsonl]
//!        [--trace-every MS] <slug> [ticks] [out.png]
//!        (assets/<slug> must exist; default 500 ticks, out.png)
//!
//! `--trace` additionally writes a motion trace: one JSON line per sampled
//! instant holding every sprite the module would draw then, plus the snd ids
//! it fired since the previous line. `tools/motion_lint.py` reads that and
//! prints step/jerk/teleport/stuck numbers — the mechanical motion gate a
//! fix lane has to clear before it may claim a module animates. Static PNGs
//! cannot show jerk; this can.

use app::{compose, modules, ImageCache, SIM_PIXELS};
use engine::{Ctx, Pack, Random15, RandomLong, SpriteDraw, SCREEN_H, SCREEN_W};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// Wall-clock derived from elapsed sim time, starting at the harness's
/// fixed 12:00:00 boot time. Previously `local_hms` was pinned to
/// `(12, 0, 0)` for an entire render regardless of `now_ms`, so
/// shock-clocks' hour/minute-change gags (and anything else keyed off the
/// clock advancing) could never fire in a headless render. This has no
/// bearing on the fixed RNG seeds `main` sets up — only wall time moves.
fn local_hms_at(now_ms: u64) -> (u8, u8, u8) {
    let total_secs = 12 * 3600 + now_ms / 1000;
    (
        ((total_secs / 3600) % 24) as u8,
        ((total_secs / 60) % 60) as u8,
        (total_secs % 60) as u8,
    )
}

/// Minimal JSON string escape. Compound paths are ASCII (`compounds/9000/
/// c_098.png`), but a trace that silently emits invalid JSON would be worse
/// than useless — the lint would blame the module for a quoting bug.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// PNG width/height straight out of the IHDR header — 24 bytes off disk, no
/// decode. Sprite draws carry only a path; the lint wants the box size to
/// tell "the sprite moved" from "the art changed size under a fixed origin".
fn png_dims(path: &Path) -> Option<(u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let rd = |o: usize| u32::from_be_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    Some((rd(16), rd(20)))
}

/// One motion-trace writer: samples on a fixed ms grid and carries the
/// sounds fired between samples.
struct Tracer<W: Write> {
    out: W,
    every_ms: u64,
    next_ms: u64,
    pending_sounds: Vec<u32>,
    dims: HashMap<String, Option<(u32, u32)>>,
    lines: usize,
}

impl<W: Write> Tracer<W> {
    fn new(out: W, every_ms: u64) -> Self {
        Tracer {
            out,
            every_ms: every_ms.max(1),
            next_ms: 0,
            pending_sounds: Vec::new(),
            dims: HashMap::new(),
            lines: 0,
        }
    }

    /// Sound ids fired this tick, held until the next sample line.
    fn collect(&mut self, ctx: &mut Ctx) {
        // Trace lines carry plain snd ids (the queue flag is a shell detail).
        self.pending_sounds.extend(ctx.sounds.drain(..).map(engine::snd_id));
    }

    fn due(&self, now_ms: u64) -> bool {
        now_ms >= self.next_ms
    }

    fn line(&mut self, now_ms: u64, sprites: &[SpriteDraw], root: Option<&Path>) -> String {
        let mut s = format!("{{\"t\":{now_ms},\"sprites\":[");
        for (i, sp) in sprites.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let dims = match root {
                Some(root) => *self
                    .dims
                    .entry(sp.png.clone())
                    .or_insert_with(|| png_dims(&root.join(&sp.png))),
                None => None,
            };
            s.push_str(&format!(
                "{{\"png\":\"{}\",\"x\":{},\"y\":{},\"flip\":{},\"pal\":{}",
                esc(&sp.png),
                sp.x,
                sp.y,
                sp.flip,
                sp.pal
            ));
            if let Some((w, h)) = dims {
                s.push_str(&format!(",\"w\":{w},\"h\":{h}"));
            }
            s.push('}');
        }
        s.push_str("],\"sounds\":[");
        for (i, id) in self.pending_sounds.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&id.to_string());
        }
        s.push_str("]}");
        s
    }

    fn sample(&mut self, now_ms: u64, sprites: &[SpriteDraw], root: Option<&Path>) {
        let l = self.line(now_ms, sprites, root);
        writeln!(self.out, "{l}").expect("write trace line");
        self.pending_sounds.clear();
        self.lines += 1;
        // catch up past any grid slots a long tick period skipped, so the
        // sample times stay on the grid instead of drifting with tick_ms.
        while self.next_ms <= now_ms {
            self.next_ms += self.every_ms;
        }
    }
}

fn main() {
    // `-c N=V` / `--control N=V` (repeatable, anywhere) overrides control N
    // before the first tick — e.g. `-c 1=1` renders Message Mayhem's Bathroom
    // Wall branch, which the panel default (Aortal Squirt) never reaches.
    let mut controls: Vec<(usize, i32)> = Vec::new();
    let mut positional: Vec<String> = Vec::new();
    let mut trace_path: Option<String> = None;
    let mut trace_every: u64 = 100;
    // `--raw`: skip the display gamma, for comparing against pack colours
    let mut raw_colour = false;
    let mut raw = std::env::args().skip(1);
    while let Some(a) = raw.next() {
        if a == "--trace" {
            trace_path = Some(raw.next().expect("--trace needs a path"));
            continue;
        }
        if let Some(rest) = a.strip_prefix("--trace=") {
            trace_path = Some(rest.to_string());
            continue;
        }
        if a == "--raw" {
            raw_colour = true;
            continue;
        }
        if a == "--trace-every" {
            trace_every = raw
                .next()
                .and_then(|v| v.parse().ok())
                .expect("--trace-every needs ms");
            continue;
        }
        if let Some(rest) = a.strip_prefix("--trace-every=") {
            trace_every = rest.parse().expect("--trace-every needs ms");
            continue;
        }
        let spec = if a == "-c" || a == "--control" {
            raw.next()
        } else if let Some(rest) = a.strip_prefix("--control=") {
            Some(rest.to_string())
        } else {
            positional.push(a);
            continue;
        };
        let spec = spec.expect("-c needs N=V");
        let (n, v) = spec.split_once('=').expect("control override is N=V");
        controls.push((n.parse().expect("control index"), v.parse().expect("control value")));
    }
    let mut args = positional.into_iter();
    let slug = args
        .next()
        .expect("usage: frame [-c N=V ...] [--trace T.jsonl] <slug> [ticks] [out.png]");
    let ticks: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(500);
    let out = args.next().unwrap_or_else(|| "out.png".into());

    let dir = std::path::Path::new("assets").join(&slug);
    let pack = Pack::load(&dir).unwrap_or_else(|e| panic!("load {dir:?}: {e}"));
    let pack2 = pack.clone();
    let mut m = modules::make(&slug, pack).unwrap_or_else(|| panic!("no module {slug}"));
    for (n, v) in &controls {
        m.set_control(*n, *v);
    }

    // RTW_SEED / RTW_SEED15 override the fixed streams (replaying a player
    // launch: the player uses 0x5EED_CAFE + n and a clock-seeded Random15).
    let env_seed = |k: &str, d: u64| {
        std::env::var(k).ok().and_then(|v| {
            let v = v.trim_start_matches("0x");
            u64::from_str_radix(v, 16).ok()
        }).unwrap_or(d)
    };
    let mut ctx = Ctx {
        rng: RandomLong::new(env_seed("RTW_SEED", 0x5EED_CAFE)),
        rng15: Random15::new(env_seed("RTW_SEED15", 0xC0FFEE) as u32),
        sounds: Vec::new(),
        caps_lock: false,
        now_ms: 0,
        local_hms: (12, 0, 0),
        mouse: (320, 240),
        mouse_down: false,
    };
    let mut sim = vec![0u32; SIM_PIXELS];
    let mut cache = ImageCache::new();
    let mut sprites: Vec<engine::SpriteDraw> = Vec::new();

    let mut tracer = trace_path.as_ref().map(|p| {
        let f = std::fs::File::create(p).unwrap_or_else(|e| panic!("create {p}: {e}"));
        Tracer::new(std::io::BufWriter::new(f), trace_every)
    });
    let root = dir.clone();

    // The module's own grid: `Millis(tick_ms)` for most, the Mac tick
    // (16.625 ms, truncated to ms exactly as `Resource.f4724()` does) for
    // the modules that gate on it. Same clock the player and the saver run.
    let clock = m.clock();
    for i in 0..ticks {
        ctx.now_ms = clock.now_ms(i + 1);
        ctx.local_hms = local_hms_at(ctx.now_ms);
        m.tick(&mut ctx);
        if let Some(t) = tracer.as_mut() {
            t.collect(&mut ctx);
            if t.due(ctx.now_ms) {
                sprites.clear();
                m.sprites(&mut sprites);
                t.sample(ctx.now_ms, &sprites, Some(&root));
            }
        }
        if i + 1 == ticks {
            // compose the final frame — the same pass the player shell and
            // the macOS screensaver draw through (`app::compose`).
            compose(&pack2, &mut cache, m.as_ref(), &mut sim);
            if !raw_colour {
                app::present_gamma(&mut sim);
            }
            sprites.clear();
            m.sprites(&mut sprites);
        }
    }
    write_png(&out, SCREEN_W as u32, SCREEN_H as u32, &sim);
    let traced = tracer.as_ref().map(|t| t.lines).unwrap_or(0);
    if let Some(mut t) = tracer {
        t.out.flush().expect("flush trace");
    }
    print!("{slug}: {ticks} ticks -> {out} ({} sprites)", sprites.len());
    match trace_path {
        Some(p) => println!(" [trace {p}: {traced} lines @ {trace_every} ms]"),
        None => println!(),
    }
}

fn write_png(path: &str, w: u32, h: u32, sim: &[u32]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().expect("png header");
    let mut raw = Vec::with_capacity((w * h * 4) as usize);
    for &c in sim {
        raw.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 0xFF]);
    }
    writer.write_image_data(&raw).expect("png data");
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::TickClock;

    #[test]
    fn wall_clock_advances_from_noon_and_never_stays_pinned() {
        assert_eq!(local_hms_at(0), (12, 0, 0));
        assert_eq!(local_hms_at(1_500), (12, 0, 1)); // 1.5 s -> 1 s (floor)
        assert_eq!(local_hms_at(60_000), (12, 1, 0)); // 1 minute
        assert_eq!(local_hms_at(3_600_000), (13, 0, 0)); // 1 hour
        assert_eq!(local_hms_at(12 * 3_600_000), (0, 0, 0)); // wraps at midnight
        // the bug this guards against: local_hms pinned at (12,0,0) forever
        assert_ne!(local_hms_at(500_000), (12, 0, 0));
    }

    /// Hand-rolled JSON has to actually be JSON. Values are checked by
    /// substring rather than a parser (app carries no serde_json) — the
    /// shape check below is what guards the overall grammar.
    #[test]
    fn trace_line_is_well_formed() {
        let mut t = Tracer::new(Vec::new(), 100);
        let sprites = vec![
            SpriteDraw { png: "compounds/9000/c_098.png".into(), x: 12, y: -3, pal: 0, flip: false },
            SpriteDraw { png: "compounds/9000/m_636.png".into(), x: 600, y: 470, pal: 129, flip: true },
        ];
        t.pending_sounds = vec![9000, 9001];
        let l = t.line(1_234, &sprites, None);
        assert!(l.starts_with("{\"t\":1234,\"sprites\":["), "{l}");
        assert!(l.contains("\"png\":\"compounds/9000/c_098.png\",\"x\":12,\"y\":-3,\"flip\":false,\"pal\":0"), "{l}");
        assert!(l.contains("\"flip\":true,\"pal\":129"), "{l}");
        assert!(l.ends_with("\"sounds\":[9000,9001]}"), "{l}");
        assert!(!l.contains('\n'), "a trace line is one line");
        assert!(balanced(&l), "{l}");
        // no art on disk to size against -> w/h simply omitted, never faked
        assert!(!l.contains("\"w\":"), "{l}");
    }

    #[test]
    fn empty_sprite_list_still_emits_a_line() {
        let mut t = Tracer::new(Vec::new(), 100);
        let l = t.line(0, &[], None);
        assert_eq!(l, "{\"t\":0,\"sprites\":[],\"sounds\":[]}");
    }

    #[test]
    fn png_paths_with_quotes_are_escaped() {
        let mut t = Tracer::new(Vec::new(), 100);
        let sprites = vec![SpriteDraw { png: "a\"b\\c.png".into(), ..Default::default() }];
        let l = t.line(0, &sprites, None);
        assert!(l.contains("\"png\":\"a\\\"b\\\\c.png\""), "{l}");
        assert!(balanced(&l), "{l}");
    }

    /// The sample grid must not drift when the tick period is coarser than
    /// `--trace-every`: samples land on multiples of the interval.
    #[test]
    fn sample_grid_catches_up_past_skipped_slots() {
        let mut t = Tracer::new(Vec::new(), 100);
        assert!(t.due(0));
        t.sample(0, &[], None);
        assert_eq!(t.next_ms, 100);
        assert!(!t.due(99));
        t.sample(350, &[], None); // one long tick jumped three slots
        assert_eq!(t.next_ms, 400);
        assert_eq!(t.lines, 2);
    }

    /// Sounds are carried between samples and cleared after each line, so a
    /// snd id is reported exactly once, against the sample that follows it.
    #[test]
    fn sounds_are_drained_per_line() {
        let mut t = Tracer::new(Vec::new(), 100);
        let mut ctx = Ctx {
            rng: RandomLong::new(1),
            rng15: Random15::new(1),
            sounds: vec![42],
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (0, 0),
            mouse_down: false,
        };
        t.collect(&mut ctx);
        assert!(ctx.sounds.is_empty(), "collect drains the tick's sounds");
        t.sample(0, &[], None);
        let second = t.line(100, &[], None);
        assert!(second.ends_with("\"sounds\":[]}"), "{second}");
    }

    fn balanced(l: &str) -> bool {
        let (mut depth, mut instr, mut esc) = (0i32, false, false);
        for c in l.chars() {
            if instr {
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    instr = false;
                }
                continue;
            }
            match c {
                '"' => instr = true,
                '{' | '[' => depth += 1,
                '}' | ']' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return false;
            }
        }
        depth == 0 && !instr
    }

    /// End to end against real art when the (gitignored) asset packs are
    /// present: every line the tracer writes for a real module parses, is
    /// on the grid, and carries the PNG box sizes read off disk.
    #[test]
    fn traces_a_real_module_into_well_formed_lines() {
        let dir = Path::new("../assets/mime-hunt");
        let Ok(pack) = Pack::load(dir) else {
            eprintln!("assets/mime-hunt missing — skipping");
            return;
        };
        let mut m = modules::make("mime-hunt", pack).expect("module");
        let mut ctx = Ctx {
            rng: RandomLong::new(0x5EED_CAFE),
            rng15: Random15::new(0xC0FFEE),
            sounds: Vec::new(),
            caps_lock: false,
            now_ms: 0,
            local_hms: (12, 0, 0),
            mouse: (320, 240),
            mouse_down: false,
        };
        let mut t = Tracer::new(Vec::new(), 100);
        let mut sprites = Vec::new();
        let clock = m.clock();
        let per = match clock {
            TickClock::Millis(ms) => ms,
            _ => 17,
        };
        for i in 0..(10_000 / per.max(1)) {
            ctx.now_ms = clock.now_ms(i + 1);
            m.tick(&mut ctx);
            t.collect(&mut ctx);
            if t.due(ctx.now_ms) {
                sprites.clear();
                m.sprites(&mut sprites);
                t.sample(ctx.now_ms, &sprites, Some(dir));
            }
        }
        let text = String::from_utf8(t.out.clone()).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines.len() > 50, "10 s at 100 ms should sample ~100 lines, got {}", lines.len());
        let mut last_t = None::<u64>;
        let mut saw_wh = false;
        for l in &lines {
            assert!(balanced(l), "unbalanced: {l}");
            assert!(l.starts_with("{\"t\":"), "{l}");
            assert!(l.contains("\"sprites\":["), "{l}");
            assert!(l.ends_with("]}"), "{l}");
            let now: u64 = l[5..l.find(',').unwrap()].parse().expect("t is a number");
            if let Some(p) = last_t {
                assert!(now > p, "timestamps must advance: {p} -> {now}");
            }
            last_t = Some(now);
            saw_wh |= l.contains("\"w\":");
        }
        assert!(saw_wh, "real art on disk should give every sprite a w/h");
    }
}
