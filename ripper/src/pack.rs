//! Build assets/<slug>/ (meta.json v4, compounds/, sounds/, music/,
//! pens500.json) and assets/_shared/faceplate.png from parsed resources.
//! Port of tools/twistedrip/pack.py -- including its per-module tables and
//! every dict-ordering detail that shows up in meta.json's bytes.
//!
//! Building is split from writing: `build()` returns the meta document plus
//! a list of output files whose PNGs are still raw RGBA, and `write_all()`
//! deflates and writes them on all cores (the level-9 deflate is most of a
//! rip's time; the bytes are identical whichever thread produces them).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::decode::rlep::{self, Image};
use crate::decode::{clut, oftb, pens, pict};
use crate::json::{dumps_indent1, Json, Obj};
use crate::rsrc::ResMap;
use crate::util::{mac_roman, Be};
use crate::{cmid, inst, snd, Error};

/// meta.json `base_clut` per slug (pack.py BASE_CLUT; rationale there).
fn base_clut(slug: &str) -> Option<&'static str> {
    match slug {
        "chameleon" => Some("1500"),
        "toxic-swamp" => Some("1200"),
        _ => None,
    }
}

/// Which shared-bank `cmid` songs each pack carries (pack.py SONGS).
fn songs(slug: &str) -> &'static [i16] {
    match slug {
        "mime-hunt" => &[10],
        "coming-soon" => &[20],
        "frankenscreen" => &[30],
        "mowin-boris" => &[40],
        _ => &[],
    }
}

pub const SHARED_DIR: &str = "_shared";
pub const FACEPLATE_PICT_ID: i16 = 128;
pub const FACEPLATE_SOURCES: [&str; 2] = ["Twisted Faceplate", "After Dark 3.0"];
pub const HELP_TEXT_ID: i16 = 1000;

pub enum Content {
    Bytes(Vec<u8>),
    Png(Image),
}

/// Files (relative to the pack's parent dir) and directories to create.
#[derive(Default)]
pub struct Output {
    pub dirs: Vec<PathBuf>,
    pub files: Vec<(PathBuf, Content)>,
}

// ---------------------------------------------------------------------------
// SMF scan + WAV metadata (pack.py smf_scan / wav_sample_meta).
// ---------------------------------------------------------------------------

pub struct SmfInfo {
    pub ppq: u16,
    pub length_ms: i64,
    pub note_ons: i64,
    pub programs: BTreeSet<u8>,
    pub name: String,
}

pub fn smf_scan(data: &[u8]) -> Result<SmfInfo, Error> {
    let bad = || Error::Decode("malformed SMF".into());
    if data.get(..4) != Some(b"MThd") {
        return Err(Error::Decode("not an SMF".into()));
    }
    let hlen = data.u32_at(4)? as usize;
    let ntrk = data.u16_at(10)? as usize;
    let div = data.u16_at(12)?;
    if div & 0x8000 != 0 {
        return Err(Error::Decode("SMTPE division not supported".into()));
    }
    let mut o = 8 + hlen;
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    let mut max_tick = 0u64;
    let mut note_ons = 0i64;
    let mut programs = BTreeSet::new();
    let mut name = String::new();
    for _ in 0..ntrk {
        if data.get(o..o + 4) != Some(b"MTrk") {
            return Err(Error::Decode("bad track header".into()));
        }
        let tlen = data.u32_at(o + 4)? as usize;
        let body = crate::util::pyslice(data, o + 8, o + 8 + tlen);
        o += 8 + tlen;
        let (mut p, mut t) = (0usize, 0u64);
        let mut running: Option<u8> = None;
        let at = |p: usize| body.get(p).copied().ok_or_else(bad);
        while p < body.len() {
            let mut dt = 0u64;
            loop {
                let b = at(p)?;
                p += 1;
                dt = (dt << 7) | (b & 0x7F) as u64;
                if b & 0x80 == 0 {
                    break;
                }
            }
            t += dt;
            let mut st = at(p)?;
            if st < 0x80 {
                st = running.ok_or_else(bad)?;
            } else {
                p += 1;
                if st < 0xF0 {
                    running = Some(st);
                }
            }
            if st == 0xFF {
                let mt = at(p)?;
                p += 1;
                let mut ln = 0usize;
                loop {
                    let b = at(p)?;
                    p += 1;
                    ln = (ln << 7) | (b & 0x7F) as usize;
                    if b & 0x80 == 0 {
                        break;
                    }
                }
                let payload = crate::util::pyslice(body, p, p + ln);
                p += ln;
                if mt == 0x51 && ln == 3 {
                    let v = payload.iter().fold(0u32, |a, &b| (a << 8) | b as u32);
                    tempos.push((t, v));
                } else if mt == 0x03 && name.is_empty() {
                    name = mac_roman(payload);
                }
            } else if st == 0xF0 || st == 0xF7 {
                let mut ln = 0usize;
                loop {
                    let b = at(p)?;
                    p += 1;
                    ln = (ln << 7) | (b & 0x7F) as usize;
                    if b & 0x80 == 0 {
                        break;
                    }
                }
                p += ln;
            } else {
                let (hi, ch) = (st & 0xF0, st & 0x0F);
                let n = if hi == 0xC0 || hi == 0xD0 { 1 } else { 2 };
                let args = crate::util::pyslice(body, p, p + n);
                p += n;
                if hi == 0x90 && *args.get(1).ok_or_else(bad)? > 0 {
                    note_ons += 1;
                    if ch == 9 {
                        programs.insert(127);
                    }
                } else if hi == 0xC0 && ch != 9 {
                    programs.insert(*args.first().ok_or_else(bad)?);
                }
            }
        }
        max_tick = max_tick.max(t);
    }
    tempos.sort();
    let (mut ms, mut prev, mut us) = (0.0f64, 0u64, 500000.0f64);
    for (tt, tu) in tempos {
        if tt >= max_tick {
            break;
        }
        ms += (tt - prev) as f64 / div as f64 * us / 1000.0;
        prev = tt;
        us = tu as f64;
    }
    ms += (max_tick - prev) as f64 / div as f64 * us / 1000.0;
    Ok(SmfInfo {
        ppq: div,
        length_ms: ms.round_ties_even() as i64,
        note_ons,
        programs,
        name,
    })
}

/// (rate, base_note, loop_start, loop_end) of a csnd/snd WAV, loop points
/// back in frames.
pub fn wav_sample_meta(wav: &[u8]) -> (u32, u32, u32, u32) {
    let (mut rate, mut bits, mut chans) = (22254u32, 32u32, 1u32);
    let (mut base, mut ls, mut le) = (60u32, 0u32, 0u32);
    let mut o = 12usize;
    while o + 8 <= wav.len() {
        let cid = &wav[o..o + 4];
        let sz = u32::from_le_bytes(wav[o + 4..o + 8].try_into().unwrap()) as usize;
        let body = crate::util::pyslice(wav, o + 8, o + 8 + sz);
        let le32 = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let le16 = |b: &[u8], i: usize| u16::from_le_bytes(b[i..i + 2].try_into().unwrap()) as u32;
        if cid == b"fmt " && body.len() >= 16 {
            chans = le16(body, 2);
            rate = le32(body, 4);
            bits = le16(body, 14);
        } else if cid == b"smpl" && body.len() >= 36 {
            base = le32(body, 12);
            let nloops = le32(body, 28);
            if nloops >= 1 && body.len() >= 60 {
                ls = le32(body, 44);
                le = le32(body, 48);
            }
        }
        o += 8 + sz + (sz & 1);
    }
    let frame = ((bits / 8) * chans.max(1)).max(1);
    (rate, base, ls / frame, le / frame)
}

// ---------------------------------------------------------------------------
// Strings, help, palettes.
// ---------------------------------------------------------------------------

fn parse_str_list(d: &[u8]) -> Vec<String> {
    if d.len() < 2 {
        return vec![];
    }
    let count = u16::from_be_bytes([d[0], d[1]]) as usize;
    let mut out = Vec::new();
    let mut o = 2;
    for _ in 0..count {
        if o >= d.len() {
            break;
        }
        let ln = d[o] as usize;
        o += 1;
        out.push(mac_roman(crate::util::pyslice(d, o, o + ln)));
        o += ln;
    }
    out
}

fn build_strings(module: &ResMap) -> Obj {
    let mut o = Obj::new();
    for r in module.of_type("STR#") {
        o.set(
            r.id.to_string(),
            Json::Arr(parse_str_list(&r.data).into_iter().map(Json::Str).collect()),
        );
    }
    o
}

/// pack.py `_parse_sunt`: u16 count, then unpadded rows of (i16 value,
/// Pascal string) -> `[[value, word], ...]`.
fn parse_sunt(d: &[u8]) -> Vec<Json> {
    if d.len() < 2 {
        return vec![];
    }
    let count = u16::from_be_bytes([d[0], d[1]]) as usize;
    let mut out = Vec::new();
    let mut o = 2;
    for _ in 0..count {
        if o + 3 > d.len() {
            break;
        }
        let value = i16::from_be_bytes([d[o], d[o + 1]]) as i64;
        let ln = d[o + 2] as usize;
        o += 3;
        out.push(Json::Arr(vec![
            Json::Int(value),
            Json::Str(mac_roman(crate::util::pyslice(d, o, o + ln))),
        ]));
        o += ln;
    }
    out
}

fn build_slider_words(module: &ResMap) -> Obj {
    let mut o = Obj::new();
    for r in module.of_type("sUnt") {
        o.set(r.id.to_string(), Json::Arr(parse_sunt(&r.data)));
    }
    o
}

/// pack.py `_parse_menu`: skip the 14-byte header and the title, then
/// Pascal-string items (each followed by 4 attribute bytes) up to a zero
/// length byte. Separators ('-') are kept.
fn parse_menu(d: &[u8]) -> Vec<Json> {
    if d.len() < 15 {
        return vec![];
    }
    let mut o = 15 + d[14] as usize;
    let mut out = Vec::new();
    while o < d.len() && d[o] != 0 {
        let ln = d[o] as usize;
        out.push(Json::Str(mac_roman(crate::util::pyslice(d, o + 1, o + 1 + ln))));
        o += 1 + ln + 4;
    }
    out
}

fn build_menus(module: &ResMap) -> Obj {
    let mut o = Obj::new();
    for r in module.of_type("MENU") {
        o.set(r.id.to_string(), Json::Arr(parse_menu(&r.data)));
    }
    o
}

fn build_help(module: &ResMap) -> String {
    match module.get("TEXT", HELP_TEXT_ID) {
        None => String::new(),
        Some(r) => mac_roman(&r.data).replace("\r\n", "\n").replace('\r', "\n"),
    }
}

fn rgb(c: &[u8; 3]) -> Json {
    Json::Arr(c.iter().map(|&v| Json::Int(v as i64)).collect())
}

/// Every NAMED clut keyed by slot (pack.py reproduces pack_assets.py's
/// filename-glob accident of skipping nameless cluts; so does this).
fn build_palettes(module: &ResMap) -> Obj {
    let mut out = Obj::new();
    for r in module.of_type("clut") {
        if r.name.is_empty() {
            continue;
        }
        let cl = clut::parse(&r.data);
        if cl.len() >= 2 {
            let mut p = Obj::new();
            for (k, v) in &cl {
                p.set(k.to_string(), rgb(v));
            }
            out.set(r.id.to_string(), p);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Art.
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn frame_json(
    png: String,
    dx: i64,
    dy: i64,
    bx: i64,
    by: i64,
    w: i64,
    h: i64,
    parts: Option<Json>,
) -> Json {
    let mut f = Obj::new();
    f.set("png", png);
    f.set("dx", dx);
    f.set("dy", dy);
    f.set("bx", bx);
    f.set("by", by);
    f.set("w", w);
    f.set("h", h);
    if let Some(p) = parts {
        f.set("parts", p);
    }
    Json::Obj(f)
}

fn seq_json(first: i64, frames: Vec<Json>) -> Json {
    let mut s = Obj::new();
    s.set("first", first);
    s.set("frames", Json::Arr(frames));
    Json::Obj(s)
}

fn pack_bank_frames(data: &[u8], base: i16, slug: &str, out: &mut Output) -> Option<Vec<Json>> {
    let bank = rlep::parse_bank(data);
    if bank.frames.is_empty() {
        return None;
    }
    let cdir = PathBuf::from(slug).join("compounds").join(base.to_string());
    out.dirs.push(cdir.clone());
    let mut frames: Vec<_> = bank.frames.iter().collect();
    frames.sort_by_key(|f| f.0);
    let mut seqs = Vec::new();
    for (fid, rect, stream) in frames {
        let rows = rlep::decode_rows(stream);
        let Some(img) = rlep::render(&rows, &bank.ctab, *rect) else {
            continue;
        };
        let name = format!("f_{fid:03}.png");
        let (w, h) = (img.w as i64, img.h as i64);
        out.files.push((cdir.join(&name), Content::Png(img)));
        seqs.push(seq_json(
            *fid as i64,
            vec![frame_json(
                format!("compounds/{base}/{name}"),
                0,
                0,
                0,
                0,
                w,
                h,
                None,
            )],
        ));
    }
    Some(seqs)
}

#[allow(clippy::too_many_arguments, clippy::map_entry)]
fn pack_series(
    banks: &[(i16, &[u8])],
    cache: &mut HashMap<i16, Arc<oftb::BankArt>>,
    oftb_by_id: &BTreeMap<i16, &[u8]>,
    base: i16,
    ofst: &[u8],
    slug: &str,
    module_name: &str,
    out: &mut Output,
    warnings: &mut Vec<String>,
) -> Result<Vec<Json>, Error> {
    let mut recs = oftb::parse_ofst(ofst)?;
    let art = oftb::load_art_chain(banks, cache, base)?;
    let shadow = oftb::shadow_channels(module_name);
    recs.sort();
    let mut runs: Vec<Vec<(u16, u32, i16, i16)>> = Vec::new();
    for r in recs {
        match runs.last_mut() {
            Some(run) if r.0 as u32 == run.last().unwrap().0 as u32 + 1 => run.push(r),
            _ => runs.push(vec![r]),
        }
    }
    let cdir = PathBuf::from(slug).join("compounds").join(base.to_string());
    out.dirs.push(cdir.clone());
    struct Composed {
        name: String,
        bl: i64,
        bt: i64,
        w: i64,
        h: i64,
        parts: Json,
    }
    let mut composed: HashMap<u16, Composed> = HashMap::new();
    let mut seqs = Vec::new();
    for run in &runs {
        let mut frames = Vec::new();
        for &(fno, off, dx, dy) in run {
            let tb_id = base as i64 + (off >> 16) as i64;
            let Some(tb) = i16::try_from(tb_id).ok().and_then(|id| oftb_by_id.get(&id)) else {
                continue;
            };
            if !composed.contains_key(&fno) {
                let c = oftb::compose_compound(&art, tb, (off & 0xFFFF) as usize, shadow)?;
                for p in &c.parts {
                    if p.art == 0 || !art.contains_key(&(p.art - 1)) {
                        warnings.push(format!("compound {fno}: art {} missing", p.art));
                    }
                }
                let Some(img) = c.image else { continue };
                let name = format!("c_{fno:03}.png");
                let (w, h) = (img.w as i64, img.h as i64);
                out.files.push((cdir.join(&name), Content::Png(img)));
                let parts = Json::Arr(
                    c.parts
                        .iter()
                        .map(|p| {
                            Json::Arr(vec![
                                Json::Int(p.art as i64),
                                Json::Int(p.chan as i64),
                                Json::Int(p.flags as i64),
                                Json::Int(p.rect.0 as i64),
                                Json::Int(p.rect.1 as i64),
                                Json::Int(p.rect.2 as i64),
                                Json::Int(p.rect.3 as i64),
                            ])
                        })
                        .collect(),
                );
                composed.insert(
                    fno,
                    Composed {
                        name,
                        bl: c.bounds.0 as i64,
                        bt: c.bounds.1 as i64,
                        w,
                        h,
                        parts,
                    },
                );
            }
            let c = &composed[&fno];
            frames.push(frame_json(
                format!("compounds/{base}/{}", c.name),
                dx as i64,
                dy as i64,
                c.bl,
                c.bt,
                c.w,
                c.h,
                Some(c.parts.clone()),
            ));
        }
        if !frames.is_empty() {
            seqs.push(seq_json(run[0].0 as i64, frames));
        }
    }
    Ok(seqs)
}

fn build_art(
    slug: &str,
    module_name: &str,
    module: &ResMap,
    out: &mut Output,
    warnings: &mut Vec<String>,
) -> Result<(Obj, Obj), Error> {
    let rlep_by_id: BTreeMap<i16, &[u8]> = module
        .of_type("RLEP")
        .map(|r| (r.id, r.data.as_slice()))
        .collect();
    let oftb_by_id: BTreeMap<i16, &[u8]> = module
        .of_type("OFtb")
        .map(|r| (r.id, r.data.as_slice()))
        .collect();
    let ofst_by_id: BTreeMap<i16, &[u8]> = module
        .of_type("OFst")
        .map(|r| (r.id, r.data.as_slice()))
        .collect();
    let banks: Vec<(i16, &[u8])> = rlep_by_id.iter().map(|(k, v)| (*k, *v)).collect();
    let mut cache = HashMap::new();

    let mut series = Obj::new();
    for (&base, &ofst) in &ofst_by_id {
        let seqs = pack_series(
            &banks,
            &mut cache,
            &oftb_by_id,
            base,
            ofst,
            slug,
            module_name,
            out,
            warnings,
        )?;
        series.set(base.to_string(), Json::Arr(seqs));
    }
    for (&base, &data) in &rlep_by_id {
        if ofst_by_id.contains_key(&base) {
            continue;
        }
        if let Some(seqs) = pack_bank_frames(data, base, slug, out) {
            if !seqs.is_empty() {
                series.set(base.to_string(), Json::Arr(seqs));
            }
        }
    }
    let mut baked = Obj::new();
    for (base_str, _) in &series.0 {
        let Ok(id) = base_str.parse::<i16>() else {
            continue;
        };
        if let Some(data) = rlep_by_id.get(&id) {
            let bank = rlep::parse_bank(data);
            if !bank.ctab.is_empty() {
                baked.set(
                    base_str.clone(),
                    Json::Arr(bank.ctab.iter().map(rgb).collect()),
                );
            }
        }
    }
    Ok((series, baked))
}

// ---------------------------------------------------------------------------
// Sounds + music.
// ---------------------------------------------------------------------------

fn build_sounds(
    slug: &str,
    module: &ResMap,
    shared: &ResMap,
    out: &mut Output,
) -> Result<usize, Error> {
    let sdir = PathBuf::from(slug).join("sounds");
    out.dirs.push(sdir.clone());
    let mut written = BTreeSet::new();
    for bank in [module, shared] {
        for r in bank.of_type("snd ") {
            if written.contains(&r.id) {
                continue;
            }
            let companion = bank.get("sndS", r.id).map(|c| c.data.as_slice());
            let wav = snd::to_wav(&r.data, companion)
                .map_err(|e| Error::Decode(format!("snd {}: {e}", r.id)))?;
            out.files
                .push((sdir.join(format!("{}.wav", r.id)), Content::Bytes(wav)));
            written.insert(r.id);
        }
    }
    Ok(written.len())
}

fn build_music(slug: &str, shared: &ResMap, out: &mut Output) -> Result<Option<Obj>, Error> {
    let ids = songs(slug);
    if ids.is_empty() {
        return Ok(None);
    }
    let mdir = PathBuf::from(slug).join("music");
    let sdir = mdir.join("samples");
    let mut song_obj = Obj::new();
    let mut programs = BTreeSet::new();
    for &sid in ids {
        let Some(res) = shared.get("cmid", sid) else {
            continue;
        };
        let data = cmid::decode(&res.data)?;
        let info = smf_scan(&data)?;
        out.dirs.push(sdir.clone());
        out.files
            .push((mdir.join(format!("song_{sid}.mid")), Content::Bytes(data)));
        let mut s = Obj::new();
        s.set("file", format!("music/song_{sid}.mid"));
        s.set("name", info.name);
        s.set("ppq", info.ppq as i64);
        s.set("length_ms", info.length_ms);
        s.set("note_ons", info.note_ons);
        song_obj.set(sid.to_string(), s);
        programs.extend(info.programs);
    }
    if song_obj.is_empty() {
        return Ok(None);
    }
    let mut inst_by_prog = HashMap::new();
    for r in shared.of_type("INST") {
        inst_by_prog.insert(r.id, inst::decode_inst(&r.data)?);
    }
    let mut instruments = Obj::new();
    let mut samples = Obj::new();
    for prog in programs {
        let Some(decoded) = inst_by_prog.get(&(prog as i16)) else {
            continue;
        };
        let resolved = inst::resolve_regions(decoded, shared)?;
        let mut regions = Vec::new();
        for r in resolved {
            let key = format!("{}_{}", r.sample_type, r.snd_id);
            if samples.get(&key).is_none() {
                let res_type = if r.sample_type == "csnd" {
                    "csnd"
                } else {
                    "snd "
                };
                let Some(res) = shared.get(res_type, r.snd_id) else {
                    continue;
                };
                let wav = if r.sample_type == "csnd" {
                    inst::csnd_to_wav(&res.data)?
                } else {
                    snd::to_wav(&res.data, None)?
                };
                let (rate, wbase, ls, le) = wav_sample_meta(&wav);
                out.dirs.push(sdir.clone());
                out.files
                    .push((sdir.join(format!("{key}.wav")), Content::Bytes(wav)));
                let mut s = Obj::new();
                s.set("file", format!("music/samples/{key}.wav"));
                s.set("rate", rate as i64);
                s.set("base_note", wbase as i64);
                s.set("loop_start", ls as i64);
                s.set("loop_end", le as i64);
                samples.set(key.clone(), s);
            }
            let mut g = Obj::new();
            g.set("key_low", r.key_low);
            g.set("key_high", r.key_high);
            g.set("base_note", r.base_note);
            g.set("freq_mult", r.freq_mult.unwrap_or(1.0));
            g.set("sample", key);
            regions.push(Json::Obj(g));
        }
        if !regions.is_empty() {
            instruments.set(prog.to_string(), Json::Arr(regions));
        }
    }
    let mut m = Obj::new();
    m.set("songs", song_obj);
    m.set("instruments", instruments);
    m.set("samples", samples);
    Ok(Some(m))
}

// ---------------------------------------------------------------------------
// Top level.
// ---------------------------------------------------------------------------

pub struct Built {
    pub meta: Obj,
    pub output: Output,
    pub warnings: Vec<String>,
    pub sounds: usize,
}

/// pack.py `build()`: everything for assets/<slug>/, not yet written. Paths
/// in the returned Output are relative to the assets root.
pub fn build(
    slug: &str,
    module_name: &str,
    module: &ResMap,
    shared_sound: &ResMap,
) -> Result<Built, Error> {
    let mut out = Output::default();
    out.dirs.push(PathBuf::from(slug));
    let mut warnings = Vec::new();
    let (series, baked) = build_art(slug, module_name, module, &mut out, &mut warnings)?;
    let palettes = build_palettes(module);
    let sounds = build_sounds(slug, module, shared_sound, &mut out)?;
    if slug == "message-mayhem" {
        if let Some(r) = module.get("Pens", 500) {
            let doc = pens::decode(&r.data)?;
            out.files.push((
                PathBuf::from(slug).join("pens500.json"),
                Content::Bytes(dumps_indent1(&doc).into_bytes()),
            ));
        }
    }
    let music = build_music(slug, shared_sound, &mut out)?;

    let mut meta = Obj::new();
    meta.set("module", module_name.to_lowercase().replace(' ', "-"));
    meta.set(
        "field",
        Json::Arr(vec![Json::Int(0), Json::Int(0), Json::Int(0)]),
    );
    meta.set("series", series);
    meta.set("palettes", palettes);
    meta.set(
        "base_clut",
        base_clut(slug).map(Json::from).unwrap_or(Json::Null),
    );
    meta.set("baked", baked);
    meta.set("strings", build_strings(module));
    meta.set("help", build_help(module));
    meta.set("slider_words", build_slider_words(module));
    meta.set("menus", build_menus(module));
    if let Some(m) = music {
        meta.set("music", m);
    }
    out.files.push((
        PathBuf::from(slug).join("meta.json"),
        Content::Bytes(dumps_indent1(&Json::Obj(meta.clone())).into_bytes()),
    ));
    Ok(Built {
        meta,
        output: out,
        warnings,
        sounds,
    })
}

/// pack.py `build_faceplate`: the collection-wide banner, or None.
pub fn build_faceplate(resources: &HashMap<String, ResMap>) -> Option<(PathBuf, Image)> {
    for name in FACEPLATE_SOURCES {
        let Some(res) = resources
            .get(name)
            .and_then(|m| m.get("PICT", FACEPLATE_PICT_ID))
        else {
            continue;
        };
        let Ok((w, h, rgba)) = pict::decode(&res.data) else {
            continue;
        };
        return Some((
            PathBuf::from(SHARED_DIR).join("faceplate.png"),
            Image { w, h, rgba },
        ));
    }
    None
}

/// Create dirs, then write every file -- PNGs deflated across all cores.
pub fn write_all(root: &Path, out: Output) -> Result<(), Error> {
    let io = |p: &Path, e: std::io::Error| Error::Io(format!("{}: {e}", p.display()));
    for d in &out.dirs {
        let p = root.join(d);
        std::fs::create_dir_all(&p).map_err(|e| io(&p, e))?;
    }
    let files = out.files;
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(files.len().max(1));
    let first_err: std::sync::Mutex<Option<Error>> = std::sync::Mutex::new(None);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((rel, content)) = files.get(i) else {
                    break;
                };
                let p = root.join(rel);
                let res = match content {
                    Content::Bytes(b) => std::fs::write(&p, b),
                    Content::Png(img) => {
                        std::fs::write(&p, crate::png::write_png(img.w, img.h, &img.rgba))
                    }
                };
                if let Err(e) = res {
                    first_err.lock().unwrap().get_or_insert(io(&p, e));
                    break;
                }
            });
        }
    });
    match first_err.into_inner().unwrap() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// pack.py's `test_parse_sunt_and_menu_synthetic`, on the same made-up bytes.
    #[test]
    fn parse_sunt_and_menu_synthetic() {
        let mut sunt = vec![0, 2, 0, 0, 3];
        sunt.extend_from_slice(b"Low");
        sunt.extend_from_slice(&[0, 50, 4]);
        sunt.extend_from_slice(b"High");
        let row = |v: i64, w: &str| Json::Arr(vec![Json::Int(v), Json::Str(w.into())]);
        assert_eq!(parse_sunt(&sunt), vec![row(0, "Low"), row(50, "High")]);
        assert!(parse_sunt(&[]).is_empty());
        let mut menu = vec![0u8; 14];
        menu.push(5);
        menu.extend_from_slice(b"Title");
        for item in [&b"Alpha"[..], b"-", b"Beta"] {
            menu.push(item.len() as u8);
            menu.extend_from_slice(item);
            menu.extend_from_slice(&[0; 4]);
        }
        menu.push(0);
        let s = |w: &str| Json::Str(w.into());
        assert_eq!(parse_menu(&menu), vec![s("Alpha"), s("-"), s("Beta")]);
        assert!(parse_menu(&[]).is_empty());
    }
}
