//! Render a packed `cmid` song to a WAV, offline — the music lane's
//! adjudication harness.
//!
//! The golden QEMU captures are the only thing that can say whether the
//! port's music is the original's music, and correlating against them needs
//! a file. This renders one straight out of a pack, with no audio device and
//! no window, at the same 22254 Hz the `csnd` samples live at.
//!
//! ```text
//!   cargo run -p engine --example render_song -- <slug> <song id> [out.wav] [plays]
//!   scripts/... snd-correlate / the scratch note-track comparison then runs
//!   against emu/captures/qemu/<slug>.wav
//! ```
//!
//! `plays` renders the tune back to back that many times, which is what a
//! Music = Twice panel asks for and what the mime-hunt capture needs to be
//! compared against past t = 55 s.

use engine::music::{write_wav, Player, OUT_RATE};
use engine::Pack;
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let slug = args.next().expect("usage: render_song <slug> <song id> [out.wav] [plays]");
    let song_id: u32 = args
        .next()
        .and_then(|a| a.parse().ok())
        .expect("usage: render_song <slug> <song id> [out.wav] [plays]");
    let out = args.next().unwrap_or_else(|| "song.wav".into());
    let plays: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(1);

    let dir = Path::new("assets").join(&slug);
    let pack = Pack::load(&dir).unwrap_or_else(|e| panic!("load {dir:?}: {e}"));
    let assets = pack
        .music()
        .unwrap_or_else(|| panic!("pack {slug} carries no music (rebuild it with tools/pack_assets.py)"));
    let song = assets
        .songs
        .get(&song_id)
        .unwrap_or_else(|| panic!("pack {slug} has no song {song_id}"))
        .clone();

    eprintln!(
        "song {song_id} \"{}\": ppq {}, {:.2} bpm, {} events, {} note-ons, \
         last note-on {:.2} s, end-of-track {:.2} s",
        song.name,
        song.ppq,
        60e6 / song.tempo_us as f64,
        song.events.len(),
        song.note_ons(),
        song.last_note_on_ms() / 1000.0,
        song.length_ms / 1000.0,
    );

    let mut p = Player::new(assets.bank.clone(), OUT_RATE);
    let mut pcm = Vec::new();
    for _ in 0..plays.max(1) {
        // No tail between plays: the original re-cues the song and the new
        // attack lands on the old release, which is what a replay sounds
        // like. The last play gets a tail so its release is not cut off.
        pcm.extend(p.render_song(song.clone(), 0.0));
    }
    pcm.extend({
        let mut tail = vec![0.0f32; OUT_RATE as usize / 2];
        p.render(&mut tail);
        tail
    });
    write_wav(Path::new(&out), &pcm, OUT_RATE).expect("write wav");
    eprintln!("wrote {out}: {:.2} s at {OUT_RATE} Hz", pcm.len() as f64 / OUT_RATE as f64);
}
