//! After Dark's MIDI background music: SMF decode, sequencer, sample synth.
//!
//! ## What this is a port of
//!
//! Until 2026-09-13 every module with a **Music** control was silent in the
//! port — `mime_hunt.rs` said "NO SONG ASSET", `coming_soon.rs` "No audio is
//! emitted for music", `frankenscreen.rs` "SONG 30 is a 'SONG' resource with
//! no packed WAV". The QEMU golden captures then measured how much that
//! costs: **80 %** of Mime Hunt's audible output, **70 %** of Coming Soon's
//! and **56 %** of FrankenScreen's is that music
//! (`totally-twisted/docs/emulator/audio-captures.md`).
//!
//! The music is not a `snd`. Each tune is a `cmid` resource in the shared
//! Twisted Sound bank, rendered by the bank's `MDRV` "MIDI Synth 3.43
//! 3/23/94" through the bank's own `csnd` sample resources — which is why
//! the captures correlate against `csnd 3201 a.bass C1` and `csnd 4702
//! timpani C2` at r = 0.6–0.7 and against no module `snd` at all.
//!
//! ## The `cmid` format (answering the lane's first question)
//!
//! **Standard SMF, wrapped — not a Berkeley event list.** A `cmid` resource
//! is a big-endian u32 decompressed length followed by SoundMusicSys LZSS
//! over a plain MIDI file; resource_dasm's `decode_cmid` is literally
//! `decompress_soundmusicsys_data`. What comes out is format 1, 480 ticks
//! per quarter note, one `FF 51` set-tempo at tick 0, and then nothing but
//! note on / note off / program change. No controllers, no pitch bend, no
//! aftertouch, no tempo changes — which is why this synth can be small and
//! still be faithful.
//!
//! Note/velocity/duration/tempo encoding is therefore exactly SMF's:
//! 7-bit note and velocity, duration = the gap to the matching note-off (or
//! a note-on with velocity 0, which these files use throughout), and time =
//! `delta_ticks / ppq * tempo_us_per_quarter`.
//!
//! The instrument mapping is the bank's `INST` resources, one per General
//! MIDI program, each a list of key regions naming a `csnd` sample, its
//! `base_note` and a `freq_mult`. Channel 9 is percussion and takes `INST`
//! 127 regardless of program change — that is the GM convention and it is
//! what the songs assume (they never send a program change on channel 9).
//! `tools/pack_assets.py` flattens all of it into `meta.json`'s `music`
//! section; see its MUSIC docstring.
//!
//! ## Synthesis
//!
//! One voice per sounding note, playing a `csnd` sample at
//!
//! ```text
//!   step = 2^((note - base_note)/12) * (sample_rate / out_rate) * freq_mult
//! ```
//!
//! source frames per output frame, looping between the sample's `smpl` loop
//! points while the note is held and fading out linearly over
//! [`RELEASE_MS`] after note-off. That is resource_dasm's `smssynth`
//! reduced to what these four songs actually use: its `src_ratio` is
//! `note_factor * sample_rate_factor / freq_mult` with
//! `note_factor = freq(base)/freq(note)`, and `step` is its reciprocal.
//! Its ADSR defaults (instant attack, no decay, unity sustain, linear
//! release) are the defaults because the `INST` resources carry no
//! envelope — so they are what the original heard too.
//!
//! ## The music is its own channel
//!
//! [`Player`] is deliberately NOT the engine's single pre-empting sfx
//! channel. The original's MDRV owns its own `SndChannel`, and the captures
//! show it: in `mime-hunt.wav` the Bazooka fires at t = 25.37 and 28.28
//! while the tune runs unbroken either side (a.bass at 1.00–4.78, timpani
//! at 12.41–15.22, a.bass again at 53.42–59.29), and in `coming-soon.wav`
//! the music's own kick drum sounds at t = 12.82, between the voice cues at
//! 4.84 and 13.80. A cue that pre-empted the music would have to restart it
//! and none of the captures shows a restart. So: separate mix, no
//! pre-emption in either direction.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Note-off fade, milliseconds. smssynth's `ADSROptions` default release
/// (0.2 s, linear) — which is what the Twisted `INST` resources get, since
/// they define no envelope of their own.
pub const RELEASE_MS: f32 = 200.0;

/// Voices that may sound at once. `cmid` 10 stacks eight tracks of piano,
/// strings, fantasia and horn; 48 covers its worst chord with room to
/// spare, and the cap exists so a decode bug cannot allocate forever.
pub const MAX_VOICES: usize = 48;

/// Render rate. 22254 Hz is the `csnd` samples' own rate (the Mac's
/// 22254.5454 Hz), so notes at their region's base note resample by exactly
/// 1.0 and the correlation rate of 11127 Hz is an exact half of it.
pub const OUT_RATE: u32 = 22254;

// ---------------------------------------------------------------------------
// Pack metadata (meta.json "music", written by tools/pack_assets.py)

#[derive(Deserialize, Clone, Debug)]
pub struct SongMeta {
    pub file: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ppq: u32,
    /// End-of-track in ms, as the packer measured it.
    #[serde(default)]
    pub length_ms: u64,
    #[serde(default)]
    pub note_ons: u32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct SampleMeta {
    pub file: String,
    pub rate: u32,
    pub base_note: u8,
    /// Loop points in FRAMES. The packer converts them out of the `smpl`
    /// chunk's byte offsets; 0/0 means the sample does not loop.
    #[serde(default)]
    pub loop_start: usize,
    #[serde(default)]
    pub loop_end: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RegionMeta {
    pub key_low: u8,
    pub key_high: u8,
    pub base_note: i32,
    #[serde(default = "one")]
    pub freq_mult: f32,
    pub sample: String,
}

fn one() -> f32 {
    1.0
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct MusicMeta {
    #[serde(default)]
    pub songs: HashMap<String, SongMeta>,
    /// GM program -> key regions.
    #[serde(default)]
    pub instruments: HashMap<String, Vec<RegionMeta>>,
    #[serde(default)]
    pub samples: HashMap<String, SampleMeta>,
}

// ---------------------------------------------------------------------------
// SMF decode

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    NoteOn { ch: u8, note: u8, vel: u8 },
    NoteOff { ch: u8, note: u8 },
    Program { ch: u8, prog: u8 },
}

/// A decoded `cmid`: absolute-time events in millisecond order.
#[derive(Clone, Debug)]
pub struct Song {
    pub name: String,
    pub ppq: u32,
    /// Microseconds per quarter note at tick 0. These songs carry exactly
    /// one set-tempo; a second one would be a decode surprise worth
    /// hearing about, so [`decode_smf`] reports it in `tempo_changes`.
    pub tempo_us: u32,
    pub tempo_changes: usize,
    /// `(t_ms, event)`, stable-sorted by time. Note-offs precede note-ons at
    /// the same instant so a re-struck note releases before it re-attacks.
    pub events: Vec<(f64, Event)>,
    /// End-of-track, milliseconds — includes the trailing rest, which is
    /// part of the tune's length for replay purposes.
    pub length_ms: f64,
}

impl Song {
    pub fn note_ons(&self) -> usize {
        self.events
            .iter()
            .filter(|(_, e)| matches!(e, Event::NoteOn { .. }))
            .count()
    }

    /// Time of the last note-on, milliseconds — what audio-captures.md
    /// quotes per tune ("`cmid` 40's last note-on is at 14.14 s").
    pub fn last_note_on_ms(&self) -> f64 {
        self.events
            .iter()
            .rev()
            .find(|(_, e)| matches!(e, Event::NoteOn { .. }))
            .map(|(t, _)| *t)
            .unwrap_or(0.0)
    }
}

struct Reader<'a> {
    d: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8, String> {
        let b = *self.d.get(self.p).ok_or("truncated")?;
        self.p += 1;
        Ok(b)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(((self.u8()? as u16) << 8) | self.u8()? as u16)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(((self.u16()? as u32) << 16) | self.u16()? as u32)
    }
    /// MIDI variable-length quantity.
    fn vlq(&mut self) -> Result<u32, String> {
        let mut v = 0u32;
        for _ in 0..4 {
            let b = self.u8()?;
            v = (v << 7) | (b & 0x7F) as u32;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("overlong VLQ".into())
    }
}

/// Decode a standard MIDI file (what a `cmid` decompresses to).
///
/// Handles running status, meta events, sysex, and the note-on-with-zero-
/// velocity spelling of note-off that these files use. Tempo is applied as a
/// map so a multi-tempo file would still come out right, even though the
/// four Twisted tunes each carry exactly one.
pub fn decode_smf(data: &[u8]) -> Result<Song, String> {
    let mut r = Reader { d: data, p: 0 };
    if r.u32()? != 0x4D54_6864 {
        return Err("not an SMF (no MThd)".into());
    }
    let hlen = r.u32()?;
    let _fmt = r.u16()?;
    let ntrk = r.u16()?;
    let div = r.u16()?;
    if div & 0x8000 != 0 {
        return Err("SMPTE division not supported".into());
    }
    let ppq = div.max(1) as u32;
    r.p = 8 + hlen as usize;

    // Pass 1: absolute-TICK events per track, plus the tempo map.
    let mut raw: Vec<(u64, usize, Event)> = Vec::new();
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    let mut name = String::new();
    let mut end_tick = 0u64;
    let mut seq = 0usize;
    for _ in 0..ntrk {
        if r.u32()? != 0x4D54_726B {
            return Err("bad track header (no MTrk)".into());
        }
        let tlen = r.u32()? as usize;
        let end = (r.p + tlen).min(data.len());
        let mut t = 0u64;
        let mut running: Option<u8> = None;
        while r.p < end {
            t += r.vlq()? as u64;
            let peek = *r.d.get(r.p).ok_or("truncated event")?;
            let st = if peek < 0x80 {
                running.ok_or("data byte with no running status")?
            } else {
                r.p += 1;
                if peek < 0xF0 {
                    running = Some(peek);
                }
                peek
            };
            match st {
                0xFF => {
                    let mt = r.u8()?;
                    let ln = r.vlq()? as usize;
                    let body = r.d.get(r.p..r.p + ln).ok_or("truncated meta")?;
                    r.p += ln;
                    match mt {
                        0x51 if ln == 3 => {
                            tempos.push((t, u32::from_be_bytes([0, body[0], body[1], body[2]])));
                        }
                        // Track name: track 0's is the tune's own title
                        // ("Mime Hunt", "Dawn Cue"), which is how the songs
                        // were matched to their modules.
                        0x03 if name.is_empty() => {
                            name = body.iter().map(|&b| b as char).collect();
                        }
                        _ => {}
                    }
                }
                0xF0 | 0xF7 => {
                    let ln = r.vlq()? as usize;
                    r.p = (r.p + ln).min(end);
                }
                _ => {
                    let ch = st & 0x0F;
                    let hi = st & 0xF0;
                    let n = if hi == 0xC0 || hi == 0xD0 { 1 } else { 2 };
                    let args = r.d.get(r.p..r.p + n).ok_or("truncated event")?.to_vec();
                    r.p += n;
                    let ev = match hi {
                        // velocity 0 is a note-off; these files use it
                        0x90 if args[1] > 0 => Some(Event::NoteOn { ch, note: args[0], vel: args[1] }),
                        0x90 | 0x80 => Some(Event::NoteOff { ch, note: args[0] }),
                        0xC0 => Some(Event::Program { ch, prog: args[0] }),
                        _ => None,
                    };
                    if let Some(ev) = ev {
                        raw.push((t, seq, ev));
                        seq += 1;
                    }
                }
            }
        }
        end_tick = end_tick.max(t);
        r.p = end;
    }

    // Pass 2: ticks -> ms through the tempo map.
    tempos.sort_by_key(|(t, _)| *t);
    let tempo_us = tempos.first().map(|(_, u)| *u).unwrap_or(500_000);
    let ms_at = |tick: u64| -> f64 {
        let mut ms = 0.0f64;
        let mut prev_tick = 0u64;
        let mut us = 500_000f64;
        for (tt, tu) in &tempos {
            if *tt >= tick {
                break;
            }
            ms += (*tt - prev_tick) as f64 / ppq as f64 * us / 1000.0;
            prev_tick = *tt;
            us = *tu as f64;
        }
        ms + (tick - prev_tick) as f64 / ppq as f64 * us / 1000.0
    };

    // Note-off before note-on at the same instant (`ord` below), then
    // original order — a stable sort keeps a chord's notes in file order.
    let mut events: Vec<(f64, Event)> = Vec::with_capacity(raw.len());
    raw.sort_by(|a, b| {
        let ord = |e: &Event| match e {
            Event::Program { .. } => 0,
            Event::NoteOff { .. } => 1,
            Event::NoteOn { .. } => 2,
        };
        a.0.cmp(&b.0).then(ord(&a.2).cmp(&ord(&b.2))).then(a.1.cmp(&b.1))
    });
    for (t, _, e) in raw {
        events.push((ms_at(t), e));
    }
    Ok(Song {
        name,
        ppq,
        tempo_us,
        tempo_changes: tempos.len(),
        events,
        length_ms: ms_at(end_tick),
    })
}

// ---------------------------------------------------------------------------
// Instrument bank

pub struct Sample {
    pub data: Vec<f32>,
    pub rate: u32,
    pub loop_start: usize,
    pub loop_end: usize,
}

pub struct Region {
    pub key_low: u8,
    pub key_high: u8,
    pub base_note: u8,
    pub freq_mult: f32,
    pub sample: usize,
}

/// The `MDRV` patch map: GM program -> key regions -> `csnd` samples.
pub struct Bank {
    pub samples: Vec<Sample>,
    pub instruments: HashMap<u8, Vec<Region>>,
}

impl Bank {
    /// The region that covers `note` in `prog`, honouring GM's channel-9
    /// drum rule (program 127 is the drum map and no song ever selects it
    /// with a program change).
    fn region(&self, prog: u8, note: u8) -> Option<&Region> {
        let regs = self.instruments.get(&prog)?;
        regs.iter()
            .find(|r| (r.key_low..=r.key_high).contains(&note))
            // A note outside every region still has to sound: the original's
            // synth clamps to the nearest patch rather than dropping it.
            .or_else(|| {
                regs.iter().min_by_key(|r| {
                    if note < r.key_low { r.key_low - note } else { note.saturating_sub(r.key_high) }
                })
            })
    }
}

/// Minimal RIFF/WAVE reader for the `csnd` rips: mono, either IEEE float32
/// (what resource_dasm writes — format tag 3) or PCM 8/16-bit.
fn read_wav(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let d = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" {
        return Err(format!("{}: not a RIFF/WAVE", path.display()));
    }
    let le16 = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]);
    let le32 = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let (mut fmt, mut chans, mut rate, mut bits) = (1u16, 1u16, 22254u32, 16u16);
    let mut data: Option<(usize, usize)> = None;
    let mut o = 12usize;
    while o + 8 <= d.len() {
        let id = &d[o..o + 4];
        let sz = le32(o + 4) as usize;
        let body = o + 8;
        if body + sz > d.len() {
            break;
        }
        if id == b"fmt " && sz >= 16 {
            fmt = le16(body);
            chans = le16(body + 2).max(1);
            rate = le32(body + 4);
            bits = le16(body + 14);
        } else if id == b"data" {
            data = Some((body, sz));
        }
        o = body + sz + (sz & 1);
    }
    let (off, sz) = data.ok_or_else(|| format!("{}: no data chunk", path.display()))?;
    let mut out = Vec::new();
    match (fmt, bits) {
        (3, 32) => {
            for c in d[off..off + sz].chunks_exact(4) {
                out.push(f32::from_le_bytes([c[0], c[1], c[2], c[3]]));
            }
        }
        (1, 16) => {
            for c in d[off..off + sz].chunks_exact(2) {
                out.push(i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0);
            }
        }
        (1, 8) => {
            for &b in &d[off..off + sz] {
                out.push((b as f32 - 128.0) / 128.0);
            }
        }
        _ => return Err(format!("{}: unsupported wav fmt {fmt} / {bits} bit", path.display())),
    }
    // Mono is what every csnd is; fold anything else so a re-rip that
    // changes shape degrades to "quieter", not "chipmunk".
    if chans > 1 {
        let n = chans as usize;
        out = out.chunks(n).map(|c| c.iter().sum::<f32>() / n as f32).collect();
    }
    Ok((out, rate))
}

/// A pack's music: the decoded songs plus the shared instrument bank.
pub struct MusicAssets {
    pub bank: Arc<Bank>,
    pub songs: HashMap<u32, Arc<Song>>,
}

impl MusicAssets {
    /// Load from a pack root and its `meta.json` music section. Returns
    /// `None` when the pack has no music (every pack built before v4, and
    /// every module that has no Music control).
    pub fn load(root: &Path, meta: &MusicMeta) -> Option<MusicAssets> {
        if meta.songs.is_empty() {
            return None;
        }
        let mut samples = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for (key, s) in &meta.samples {
            match read_wav(&root.join(&s.file)) {
                Ok((data, rate)) => {
                    let n = data.len();
                    index.insert(key.as_str(), samples.len());
                    samples.push(Sample {
                        data,
                        // trust the file's own header over meta.json
                        rate: if rate > 0 { rate } else { s.rate },
                        loop_start: s.loop_start.min(n),
                        loop_end: s.loop_end.min(n),
                    });
                }
                Err(e) => eprintln!("music: {e}"),
            }
        }
        let mut instruments = HashMap::new();
        for (prog, regs) in &meta.instruments {
            let Ok(prog) = prog.parse::<u8>() else { continue };
            let regs: Vec<Region> = regs
                .iter()
                .filter_map(|r| {
                    Some(Region {
                        key_low: r.key_low,
                        key_high: r.key_high,
                        base_note: r.base_note.clamp(0, 127) as u8,
                        freq_mult: if r.freq_mult > 0.0 { r.freq_mult } else { 1.0 },
                        sample: *index.get(r.sample.as_str())?,
                    })
                })
                .collect();
            if !regs.is_empty() {
                instruments.insert(prog, regs);
            }
        }
        let mut songs = HashMap::new();
        for (id, s) in &meta.songs {
            let Ok(id) = id.parse::<u32>() else { continue };
            match std::fs::read(root.join(&s.file))
                .map_err(|e| e.to_string())
                .and_then(|d| decode_smf(&d))
            {
                Ok(song) => {
                    songs.insert(id, Arc::new(song));
                }
                Err(e) => eprintln!("music: song {id}: {e}"),
            }
        }
        if songs.is_empty() || instruments.is_empty() {
            return None;
        }
        Some(MusicAssets { bank: Arc::new(Bank { samples, instruments }), songs })
    }
}

// ---------------------------------------------------------------------------
// Sequencer + synth

struct Voice {
    sample: usize,
    pos: f64,
    step: f64,
    loop_start: f64,
    loop_end: f64,
    amp: f32,
    ch: u8,
    note: u8,
    /// `None` while held; `Some(frames remaining)` once released.
    release: Option<f32>,
    /// Monotonic stamp, so voice stealing takes the oldest.
    age: u64,
}

/// The music channel: one song at a time, rendered as its own mix.
///
/// This is NOT the sfx channel — see the module docs. Nothing here stops or
/// is stopped by `Ctx::sounds`.
pub struct Player {
    bank: Arc<Bank>,
    out_rate: u32,
    song: Option<Arc<Song>>,
    /// Index of the next event to fire.
    next: usize,
    /// Output frames rendered since the song started.
    frame: u64,
    voices: Vec<Voice>,
    /// Current program per MIDI channel. Channel 9 is pinned to the drum
    /// map (GM program 127) and program changes on it are ignored.
    program: [u8; 16],
    age: u64,
    volume: f32,
    /// Set while the module wants silence; distinct from "song ran out".
    stopped: bool,
}

impl Player {
    pub fn new(bank: Arc<Bank>, out_rate: u32) -> Player {
        Player {
            bank,
            out_rate: out_rate.max(1),
            song: None,
            next: 0,
            frame: 0,
            voices: Vec::new(),
            program: [0; 16],
            age: 0,
            volume: 1.0,
            stopped: true,
        }
    }

    pub fn out_rate(&self) -> u32 {
        self.out_rate
    }

    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
    }

    /// Start `song` from the top, cutting anything already sounding. This is
    /// `Sound.fn_0C9C(<song>, …)` — a play, not a resume.
    pub fn play(&mut self, song: Arc<Song>) {
        self.song = Some(song);
        self.next = 0;
        self.frame = 0;
        self.voices.clear();
        self.program = [0; 16];
        self.program[9] = 127;
        self.stopped = false;
    }

    /// Silence the music channel immediately (module teardown / Music Off).
    pub fn stop(&mut self) {
        self.song = None;
        self.voices.clear();
        self.stopped = true;
    }

    /// True while the song still has events to fire or voices sounding.
    pub fn is_playing(&self) -> bool {
        !self.stopped
            && self.song.as_ref().is_some_and(|s| {
                self.next < s.events.len() || !self.voices.is_empty() || self.position_ms() < s.length_ms
            })
    }

    /// Milliseconds into the current song.
    pub fn position_ms(&self) -> f64 {
        self.frame as f64 * 1000.0 / self.out_rate as f64
    }

    fn note_on(&mut self, ch: u8, note: u8, vel: u8) {
        let prog = if ch == 9 { 127 } else { self.program[ch as usize & 15] };
        let bank = self.bank.clone();
        let Some(r) = bank.region(prog, note) else { return };
        let s = &bank.samples[r.sample];
        // See the module docs: this is smssynth's src_ratio, inverted.
        let step = 2f64.powf((note as f64 - r.base_note as f64) / 12.0)
            * (s.rate as f64 / self.out_rate as f64)
            * r.freq_mult as f64;
        let sample = r.sample;
        if self.voices.len() >= MAX_VOICES {
            if let Some((i, _)) = self.voices.iter().enumerate().min_by_key(|(_, v)| v.age) {
                self.voices.remove(i);
            }
        }
        self.age += 1;
        self.voices.push(Voice {
            sample,
            pos: 0.0,
            step,
            loop_start: s.loop_start as f64,
            // smssynth loops whenever loop_end > 0; a zero end is "one shot".
            loop_end: if s.loop_end > s.loop_start { s.loop_end as f64 } else { 0.0 },
            amp: vel as f32 / 127.0,
            ch,
            note,
            release: None,
            age: self.age,
        });
    }

    fn note_off(&mut self, ch: u8, note: u8) {
        let frames = RELEASE_MS / 1000.0 * self.out_rate as f32;
        for v in self.voices.iter_mut() {
            if v.ch == ch && v.note == note && v.release.is_none() {
                v.release = Some(frames);
            }
        }
    }

    /// Render `out.len()` mono frames of the music channel, overwriting
    /// `out`. Silence when nothing is playing — the caller can keep a sink
    /// open on this forever.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let Some(song) = self.song.clone() else { return };
        let bank = self.bank.clone();
        let ms_per_frame = 1000.0 / self.out_rate as f64;
        let release_frames = RELEASE_MS / 1000.0 * self.out_rate as f32;
        for slot in out.iter_mut() {
            // Fire everything due at or before this frame's timestamp.
            let now = self.frame as f64 * ms_per_frame;
            while self.next < song.events.len() && song.events[self.next].0 <= now {
                let (_, ev) = song.events[self.next];
                self.next += 1;
                match ev {
                    Event::Program { ch, prog } => {
                        if ch != 9 {
                            self.program[ch as usize & 15] = prog;
                        }
                    }
                    Event::NoteOn { ch, note, vel } => {
                        self.note_off(ch, note);
                        self.note_on(ch, note, vel);
                    }
                    Event::NoteOff { ch, note } => self.note_off(ch, note),
                }
            }
            let mut acc = 0.0f32;
            let mut i = 0;
            while i < self.voices.len() {
                let v = &mut self.voices[i];
                let data = &bank.samples[v.sample].data;
                let idx = v.pos as usize;
                if idx + 1 >= data.len() {
                    self.voices.remove(i);
                    continue;
                }
                let frac = (v.pos - idx as f64) as f32;
                let s = data[idx] * (1.0 - frac) + data[idx + 1] * frac;
                let env = match v.release {
                    None => 1.0,
                    Some(left) => (left / release_frames).clamp(0.0, 1.0),
                };
                acc += s * v.amp * env;
                v.pos += v.step;
                if v.loop_end > 0.0 && v.pos >= v.loop_end {
                    v.pos = v.loop_start + (v.pos - v.loop_end);
                }
                if let Some(left) = v.release.as_mut() {
                    *left -= 1.0;
                    if *left <= 0.0 {
                        self.voices.remove(i);
                        continue;
                    }
                }
                i += 1;
            }
            // Headroom: eight simultaneous voices at full velocity would
            // clip a naive sum, and the original's ASC mixer clipped rather
            // than wrapped. Scale, then hard-clip as it did.
            *slot = (acc * self.volume * 0.25).clamp(-1.0, 1.0);
            self.frame += 1;
        }
        if self.next >= song.events.len() && self.voices.is_empty() {
            // Song over. Keep `song` so `position_ms` stays meaningful to a
            // module counting replay time; `is_playing` goes false once the
            // trailing rest is past.
        }
    }

    /// Render the whole song to mono f32 at the player's rate. Used by the
    /// offline renderer (`engine/examples/render_song.rs`) and the
    /// correlation harness; `tail_ms` keeps releases from being cut off.
    pub fn render_song(&mut self, song: Arc<Song>, tail_ms: f64) -> Vec<f32> {
        let total = ((song.length_ms + tail_ms) / 1000.0 * self.out_rate as f64) as usize;
        self.play(song);
        let mut out = vec![0.0f32; total];
        // One block per call keeps the event loop identical to live playback.
        for chunk in out.chunks_mut(1024) {
            self.render(chunk);
        }
        out
    }
}

/// Write mono f32 samples as a 16-bit PCM WAV — the validation harness's
/// input, and the only reason the engine writes audio at all.
pub fn write_wav(path: &Path, samples: &[f32], rate: u32) -> std::io::Result<()> {
    let n = samples.len();
    let data_len = (n * 2) as u32;
    let mut d: Vec<u8> = Vec::with_capacity(44 + n * 2);
    d.extend(b"RIFF");
    d.extend((36 + data_len).to_le_bytes());
    d.extend(b"WAVEfmt ");
    d.extend(16u32.to_le_bytes());
    d.extend(1u16.to_le_bytes()); // PCM
    d.extend(1u16.to_le_bytes()); // mono
    d.extend(rate.to_le_bytes());
    d.extend((rate * 2).to_le_bytes());
    d.extend(2u16.to_le_bytes());
    d.extend(16u16.to_le_bytes());
    d.extend(b"data");
    d.extend(data_len.to_le_bytes());
    for s in samples {
        d.extend(((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-built SMF: two tracks, running status, a note-off spelled as
    /// note-on velocity 0, one tempo. Everything the `cmid` files use.
    fn tiny_smf() -> Vec<u8> {
        let mut trk0: Vec<u8> = Vec::new();
        trk0.extend([0x00, 0xFF, 0x03, 0x04]);
        trk0.extend(b"Tune");
        trk0.extend([0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]); // 500000 us/qn
        trk0.extend([0x00, 0xFF, 0x2F, 0x00]);
        let mut trk1: Vec<u8> = Vec::new();
        trk1.extend([0x00, 0xC0, 0x20]); // program 32 on ch 0
        trk1.extend([0x00, 0x90, 0x3C, 0x40]); // note on C4
        trk1.extend([0x83, 0x60, 0x3E, 0x40]); // running status, +480t: note on D4
        trk1.extend([0x00, 0x3C, 0x00]); // running status: note OFF C4 (vel 0)
        trk1.extend([0x83, 0x60, 0x80, 0x3E, 0x00]); // +480t note off D4
        trk1.extend([0x00, 0xFF, 0x2F, 0x00]);
        let mut d: Vec<u8> = Vec::new();
        d.extend(b"MThd");
        d.extend(6u32.to_be_bytes());
        d.extend(1u16.to_be_bytes());
        d.extend(2u16.to_be_bytes());
        d.extend(480u16.to_be_bytes());
        for t in [trk0, trk1] {
            d.extend(b"MTrk");
            d.extend((t.len() as u32).to_be_bytes());
            d.extend(t);
        }
        d
    }

    #[test]
    fn smf_decode_handles_running_status_and_zero_velocity_offs() {
        let s = decode_smf(&tiny_smf()).expect("decode");
        assert_eq!(s.name, "Tune");
        assert_eq!(s.ppq, 480);
        assert_eq!(s.tempo_us, 500_000);
        assert_eq!(s.note_ons(), 2);
        assert_eq!(s.last_note_on_ms(), 500.0);
        assert_eq!(s.length_ms, 1000.0);
        assert_eq!(s.events[0], (0.0, Event::Program { ch: 0, prog: 32 }));
        assert_eq!(s.events[1], (0.0, Event::NoteOn { ch: 0, note: 0x3C, vel: 0x40 }));
        // note-off sorts before note-on at the same instant
        assert_eq!(s.events[2], (500.0, Event::NoteOff { ch: 0, note: 0x3C }));
        assert_eq!(s.events[3], (500.0, Event::NoteOn { ch: 0, note: 0x3E, vel: 0x40 }));
        assert_eq!(s.events[4], (1000.0, Event::NoteOff { ch: 0, note: 0x3E }));
    }

    #[test]
    fn smf_decode_rejects_junk() {
        assert!(decode_smf(b"not a midi file at all").is_err());
        assert!(decode_smf(&[]).is_err());
    }

    /// A one-sample-per-region bank of unit impulses: makes the synth's
    /// pitch maths observable without any real audio.
    fn test_bank() -> Arc<Bank> {
        let mut data = vec![0.0f32; 64];
        data[0] = 1.0;
        let mut instruments = HashMap::new();
        instruments.insert(
            32,
            vec![Region { key_low: 0, key_high: 127, base_note: 60, freq_mult: 1.0, sample: 0 }],
        );
        Arc::new(Bank {
            samples: vec![Sample { data, rate: 22254, loop_start: 0, loop_end: 0 }],
            instruments,
        })
    }

    #[test]
    fn an_octave_up_doubles_the_sample_step() {
        let mut p = Player::new(test_bank(), 22254);
        p.play(Arc::new(decode_smf(&tiny_smf()).unwrap()));
        p.program[0] = 32;
        p.note_on(0, 60, 127);
        assert!((p.voices[0].step - 1.0).abs() < 1e-9, "base note plays at 1.0");
        p.note_on(0, 72, 127);
        assert!((p.voices[1].step - 2.0).abs() < 1e-9, "an octave up is 2.0");
        p.note_on(0, 48, 127);
        assert!((p.voices[2].step - 0.5).abs() < 1e-9, "an octave down is 0.5");
        // half the output rate halves every step
        let mut q = Player::new(test_bank(), 11127);
        q.play(Arc::new(decode_smf(&tiny_smf()).unwrap()));
        q.program[0] = 32;
        q.note_on(0, 60, 127);
        assert!((q.voices[0].step - 2.0).abs() < 1e-9);
    }

    #[test]
    fn a_rendered_song_sounds_and_then_stops() {
        let song = Arc::new(decode_smf(&tiny_smf()).unwrap());
        let mut p = Player::new(test_bank(), 22254);
        // 300 ms of tail: the last note-off lands exactly at end-of-track,
        // and its 200 ms release is still part of the tune.
        let pcm = p.render_song(song.clone(), 300.0);
        assert_eq!(pcm.len(), (1.3 * 22254.0) as usize);
        assert!(pcm.iter().any(|s| s.abs() > 0.0), "something sounded");
        assert!(!p.is_playing(), "the song is over once the release has run out");
        // ... and it is still playing at end-of-track itself, because the
        // note held to the final tick has not released yet.
        let mut q = Player::new(test_bank(), 22254);
        q.render_song(song, 0.0);
        assert!(q.is_playing(), "a note held at end-of-track is still sounding");
        p.stop();
        let mut buf = vec![1.0f32; 16];
        p.render(&mut buf);
        assert!(buf.iter().all(|s| *s == 0.0), "stop() silences the channel");
    }

    /// The ratchet. Every packed `cmid` decodes to exactly the tune the
    /// 2026-09-13 capture pass measured: the track name the SMF carries,
    /// the tempo, the note-on count, the end-of-track and — the number
    /// audio-captures.md actually quotes per tune — the last note-on.
    ///
    /// Those four last-note-on values (51.58 / 32.50 / 30.90 / 14.14 s) are
    /// the doc's, independently derived; reproducing them to the millisecond
    /// is what says the SMF decode, the tempo map and the tick->ms
    /// conversion are all right.
    #[test]
    fn every_packed_song_decodes_to_the_measured_tune() {
        // (pack slug, song id, name, bpm, note-ons, last note-on s, end s)
        let want: [(&str, u32, &str, f64, usize, f64, f64); 4] = [
            ("mime-hunt", 10, "Mime Hunt", 128.0, 664, 51.58, 54.84),
            ("coming-soon", 20, "Coming Soon.5", 240.0, 565, 32.50, 33.00),
            ("frankenscreen", 30, "Horror Cue", 100.0, 538, 30.90, 31.20),
            ("mowin-boris", 40, "Dawn Cue", 67.5, 66, 14.14, 16.00),
        ];
        let mut checked = 0;
        for (slug, id, name, bpm, notes, last, end) in want {
            let dir = std::path::Path::new("../assets").join(slug);
            let Ok(meta) = std::fs::read_to_string(dir.join("meta.json")) else {
                eprintln!("pack {slug} missing — skipping");
                continue;
            };
            let meta: crate::Meta = serde_json::from_str(&meta).expect("meta");
            let Some(assets) = MusicAssets::load(&dir, &meta.music) else {
                eprintln!("pack {slug} has no music (rebuild with tools/pack_assets.py)");
                continue;
            };
            let s = assets.songs.get(&id).unwrap_or_else(|| panic!("{slug}: no song {id}"));
            assert_eq!(s.name, name, "{slug} song {id} track name");
            assert_eq!(s.ppq, 480, "{slug}: every Twisted cmid is 480 ppq");
            assert_eq!(s.tempo_changes, 1, "{slug}: one set-tempo");
            assert!((60e6 / s.tempo_us as f64 - bpm).abs() < 0.01, "{slug} tempo");
            assert_eq!(s.note_ons(), notes, "{slug} note-ons");
            assert!(
                (s.last_note_on_ms() / 1000.0 - last).abs() < 0.01,
                "{slug} last note-on: {} s, audio-captures.md says {last}",
                s.last_note_on_ms() / 1000.0
            );
            assert!(
                (s.length_ms / 1000.0 - end).abs() < 0.01,
                "{slug} end-of-track: {} s, want {end}",
                s.length_ms / 1000.0
            );
            // First event is always the channel's program change at t = 0,
            // last is a note-off — these files never end on a hanging note.
            assert!(matches!(s.events[0], (t, Event::Program { .. }) if t == 0.0), "{slug} first event");
            assert!(
                matches!(s.events.last(), Some((_, Event::NoteOff { .. }))),
                "{slug} last event is a note-off"
            );
            // The patch map must cover every program the song selects, or
            // whole tracks go silent (Coming Soon's own "Rhumba Data" wants
            // GM 71/73, which this bank does NOT have — that is one of the
            // reasons it is not the tune that plays).
            for (_, e) in &s.events {
                if let Event::Program { ch, prog } = e {
                    assert!(
                        *ch == 9 || assets.bank.instruments.contains_key(prog),
                        "{slug}: song selects GM program {prog} with no INST"
                    );
                }
            }
            // …and the drum map whenever the song plays channel 9 at all
            // (Horror Cue does not — it is piano/strings/choir only).
            let drums = s
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::NoteOn { ch: 9, .. }));
            assert_eq!(
                drums,
                assets.bank.instruments.contains_key(&127),
                "{slug}: channel-9 use and a packed drum map must agree"
            );
            checked += 1;
        }
        assert!(checked > 0, "no packs with music — run tools/pack_assets.py");
    }

    /// Channel 9 is the drum map no matter what a program change says —
    /// the songs play percussion there and never select program 127.
    #[test]
    fn channel_nine_is_percussion() {
        let mut instruments = HashMap::new();
        let mut data = vec![0.0f32; 8];
        data[0] = 1.0;
        instruments.insert(
            127,
            vec![Region { key_low: 36, key_high: 36, base_note: 36, freq_mult: 1.0, sample: 0 }],
        );
        let bank = Arc::new(Bank {
            samples: vec![Sample { data, rate: 22254, loop_start: 0, loop_end: 0 }],
            instruments,
        });
        let mut p = Player::new(bank, 22254);
        p.play(Arc::new(decode_smf(&tiny_smf()).unwrap()));
        p.program[9] = 127;
        p.note_on(9, 36, 100);
        assert_eq!(p.voices.len(), 1, "ch 9 note 36 found the drum map");
        p.voices.clear();
        // a program change on ch 9 must not move it off the drum map
        p.note_on(0, 36, 100);
        assert!(p.voices.is_empty(), "ch 0 has no program 127 patch");
    }
}
