//! INST instruments and csnd compressed samples (port of
//! tools/twistedrip/inst.py: resource_dasm's decode_INST +
//! generate_json_for_INST, and decode_csnd).

use crate::cmid::lzss_decompress;
use crate::rsrc::ResMap;
use crate::snd;
use crate::util::Be;
use crate::Error;

pub struct Region {
    pub key_low: i64,
    pub key_high: i64,
    pub base_note: i64,
    pub snd_id: i16,
}

pub struct Inst {
    pub base_note: i64,
    pub use_sample_rate: bool,
    pub regions: Vec<Region>,
}

pub fn decode_inst(d: &[u8]) -> Result<Inst, Error> {
    d.bytes_at(0, 14)?;
    let snd_id = d.i16_at(0)?;
    let header_base_note = d.u16_at(2)? as i64;
    let flags1 = d.u8_at(5)?;
    let flags2 = d.u8_at(6)?;
    let n = d.u16_at(12)? as usize;
    let constant_pitch = flags2 & 0x40 != 0;
    let use_sample_rate = flags1 & 0x08 != 0;
    let region_base = if constant_pitch {
        0x3C
    } else {
        header_base_note
    };
    let mut regions = Vec::new();
    if n == 0 {
        regions.push(Region {
            key_low: 0,
            key_high: 0x7F,
            base_note: region_base,
            snd_id,
        });
    } else {
        for k in 0..n {
            let o = 14 + k * 8;
            d.bytes_at(o, 4)?;
            regions.push(Region {
                key_low: d.u8_at(o)? as i64,
                key_high: d.u8_at(o + 1)? as i64,
                base_note: region_base,
                snd_id: d.i16_at(o + 2)?,
            });
        }
    }
    Ok(Inst {
        base_note: header_base_note,
        use_sample_rate,
        regions,
    })
}

pub struct Resolved {
    pub key_low: i64,
    pub key_high: i64,
    pub base_note: i64,
    /// "csnd" or "snd" -- the resource_dasm dump-name type pack.py keys on.
    pub sample_type: &'static str,
    pub snd_id: i16,
    /// None when exactly 1.0 (inst.py omits the key then).
    pub freq_mult: Option<f64>,
}

pub fn resolve_regions(inst: &Inst, bank: &ResMap) -> Result<Vec<Resolved>, Error> {
    let shift = if inst.regions.len() > 1 && inst.base_note != 0 {
        inst.base_note - 0x3C
    } else {
        0
    };
    let mut out = Vec::new();
    for r in &inst.regions {
        let (rtype, ty) = if bank.contains("csnd", r.snd_id) {
            ("csnd", "csnd")
        } else if bank.contains("snd ", r.snd_id) {
            ("snd ", "snd")
        } else {
            return Err(Error::Decode(format!(
                "INST region references missing snd/csnd {}",
                r.snd_id
            )));
        };
        let res = bank.get(rtype, r.snd_id).unwrap();
        let bytes = if rtype == "csnd" {
            csnd_decompress(&res.data)?
        } else {
            res.data.clone()
        };
        let dec = snd::decode_snd_data(&bytes)?;
        let snd_base = dec.base_note as i64;
        let base_note = if r.base_note != 0 && snd_base != 0 {
            r.base_note + snd_base - 0x3C
        } else if r.base_note != 0 {
            r.base_note
        } else if snd_base != 0 {
            snd_base
        } else {
            0x3C
        };
        let mut fm = 1.0f64;
        if !inst.use_sample_rate {
            fm *= 22050.0 / dec.sample_rate as f64;
        }
        out.push(Resolved {
            key_low: r.key_low + shift,
            key_high: r.key_high + shift,
            base_note,
            sample_type: ty,
            snd_id: r.snd_id,
            freq_mult: if fm != 1.0 { Some(fm) } else { None },
        });
    }
    Ok(out)
}

/// csnd: SoundMusicSys LZSS, then per-sample-type delta decoding.
pub fn csnd_decompress(d: &[u8]) -> Result<Vec<u8>, Error> {
    let ts = d.u32_at(0)?;
    let st = ts >> 24;
    if st > 3 && st != 0xFF {
        return Err(Error::Decode("invalid csnd sample type".into()));
    }
    let size = (ts & 0x00FF_FFFF) as usize;
    if st != 0xFF {
        let sb = if st == 2 { 2 } else { st as usize + 1 };
        if !size.is_multiple_of(sb) {
            return Err(Error::Decode(
                "decompressed size is not a multiple of frame size".into(),
            ));
        }
    }
    let mut out = lzss_decompress(&d[4..])?;
    if out.len() < size {
        return Err(Error::Decode(
            "decompression did not produce enough data".into(),
        ));
    }
    out.truncate(size);
    let word = |o: &Vec<u8>, i: usize| u16::from_be_bytes([o[2 * i], o[2 * i + 1]]);
    let put =
        |o: &mut Vec<u8>, i: usize, v: u16| o[2 * i..2 * i + 2].copy_from_slice(&v.to_be_bytes());
    match st {
        0 if !out.is_empty() => {
            let mut s = out[0];
            for b in out.iter_mut().skip(1) {
                s = s.wrapping_add(*b);
                *b = s;
            }
        }
        2 if out.len() >= 2 => {
            let n = out.len() / 2;
            let mut s = word(&out, 0);
            for i in 1..n {
                s = s.wrapping_add(word(&out, i));
                put(&mut out, i, s);
            }
        }
        1 if out.len() >= 2 => {
            let (mut s0, mut s1) = (out[0], out[1]);
            let mut i = 2;
            while i < out.len() {
                s0 = s0.wrapping_add(out[i]);
                out[i] = s0;
                s1 = s1.wrapping_add(out[i + 1]);
                out[i + 1] = s1;
                i += 2;
            }
        }
        3 if out.len() >= 4 => {
            let n = out.len() / 2;
            let (mut s0, mut s1) = (word(&out, 0), word(&out, 1));
            let mut i = 2;
            while i < n {
                s0 = s0.wrapping_add(word(&out, i));
                put(&mut out, i, s0);
                s1 = s1.wrapping_add(word(&out, i + 1));
                put(&mut out, i + 1, s1);
                i += 2;
            }
        }
        _ => {}
    }
    Ok(out)
}

pub fn csnd_to_wav(d: &[u8]) -> Result<Vec<u8>, Error> {
    let bytes = csnd_decompress(d)?;
    Ok(snd::serialize_wav_f32(&snd::decode_snd_data(&bytes)?))
}
