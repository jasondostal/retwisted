//! Module registry. Each Totally Twisted module lives in its own file and
//! exposes `make(pack: Pack) -> Option<Box<dyn Module>>` (None = not yet
//! implemented). DO NOT add cross-module code here — one file per module.

use engine::{Module, Pack};

pub mod bungee_roulette;
pub mod chameleon;
pub mod coming_soon;
pub mod flying_toilets;
pub mod frankenscreen;
pub mod message_mayhem;
pub mod mikes_so_called_life;
pub mod mime_hunt;
pub mod mowin_boris;
pub mod phlegm_boy;
pub mod shock_clocks;
pub mod toxic_swamp;
pub mod voyeur;

/// Slugs in control-panel order; each must match an assets/<slug>/ pack dir.
pub const SLUGS: &[&str] = &[
    "bungee-roulette",
    "chameleon",
    "coming-soon",
    "flying-toilets",
    "frankenscreen",
    "message-mayhem",
    "mikes-so-called-life",
    "mime-hunt",
    "mowin-boris",
    "phlegm-boy",
    "shock-clocks",
    "toxic-swamp",
    "voyeur",
];

pub fn make(slug: &str, pack: Pack) -> Option<Box<dyn Module>> {
    match slug {
        "bungee-roulette" => bungee_roulette::make(pack),
        "chameleon" => chameleon::make(pack),
        "coming-soon" => coming_soon::make(pack),
        "flying-toilets" => flying_toilets::make(pack),
        "frankenscreen" => frankenscreen::make(pack),
        "message-mayhem" => message_mayhem::make(pack),
        "mikes-so-called-life" => mikes_so_called_life::make(pack),
        "mime-hunt" => mime_hunt::make(pack),
        "mowin-boris" => mowin_boris::make(pack),
        "phlegm-boy" => phlegm_boy::make(pack),
        "shock-clocks" => shock_clocks::make(pack),
        "toxic-swamp" => toxic_swamp::make(pack),
        "voyeur" => voyeur::make(pack),
        _ => None,
    }
}

/// Test pacing helper — the module's own tick grid, driven exactly the way
/// the three shells drive it: a tick counter in, `Ctx::now_ms` out. Tests
/// must never add a fixed millisecond step of their own, because on
/// [`engine::TickClock::MacTick`] there isn't one (16.625 ms, truncated to
/// the original's integer `Resource.f4724()` grid).
#[cfg(test)]
#[derive(Clone, Copy)]
pub struct Pacer {
    clock: engine::TickClock,
    tick: u64,
}

#[cfg(test)]
impl Pacer {
    pub fn new(m: &dyn Module) -> Pacer {
        Pacer { clock: m.clock(), tick: 0 }
    }

    /// Advance one shell tick; returns the module's new `now_ms`.
    pub fn advance(&mut self, ctx: &mut engine::Ctx) -> u64 {
        self.tick += 1;
        ctx.now_ms = self.clock.now_ms(self.tick);
        ctx.now_ms
    }

    pub fn now_ms(&self) -> u64 {
        self.clock.now_ms(self.tick)
    }

    /// How many ticks a `now + delay` gate takes to fire from tick `from`,
    /// with the module's own comparison (`>=` unless `strict`).
    pub fn ticks_for(&self, delay: u64, strict: bool) -> u64 {
        self.ticks_for_at(self.tick.max(1), delay, strict)
    }

    pub fn ticks_for_at(&self, from: u64, delay: u64, strict: bool) -> u64 {
        let target = self.clock.now_ms(from) + delay;
        let mut n = 1;
        while {
            let t = self.clock.now_ms(from + n);
            if strict { t <= target } else { t < target }
        } {
            n += 1;
        }
        n
    }

    /// Mean period in ms of a `now + delay` gate over `frames` firings —
    /// the number to compare against a capture, since on the Mac grid a
    /// 100 ms gate is not a fixed number of ticks.
    pub fn mean_period_ms(&self, delay: u64, strict: bool, frames: u32) -> f64 {
        let mut t = 1u64;
        let start = self.clock.now_ms(t);
        for _ in 0..frames {
            t += self.ticks_for_at(t, delay, strict);
        }
        (self.clock.now_ms(t) - start) as f64 / f64::from(frames)
    }
}
