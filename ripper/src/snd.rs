//! 'snd ' (+ Berkeley 'sndS' delta companion) -> WAV bytes. Port of
//! tools/twistedrip/snd.py: plain resources go through resource_dasm's
//! decode_snd_data + serialize_wav (32-bit float WAV + 'smpl' chunk);
//! ones with an sndS go through the LIB40_Sound delta expander (8-bit PCM).
//! The codec documentation lives in snd.py.

use crate::util::{pyslice, Be};
use crate::Error;

include!("mace_tables.rs");

const MACE_TABLE_1: [i32; 8] = [-0x0D, 0x08, 0x4C, 0xDE, 0xDE, 0x4C, 0x08, -0x0D];
const MACE_TABLE_3: [i32; 4] = [-0x12, 0x8C, 0x8C, -0x12];

fn parse_snd_command_header(d: &[u8]) -> Result<(usize, u16), Error> {
    if d.len() < 4 {
        return Err(Error::Decode(
            "snd doesn't even contain a format code".into(),
        ));
    }
    let format = d.u16_at(0)?;
    let mut channels = 1;
    let (num_commands, mut offset) = match format {
        1 => {
            let count = d.u16_at(2)?;
            let mut offset = 4;
            match count {
                0 => {}
                1 => {
                    let id = d.u16_at(offset)?;
                    let flags = d.u32_at(offset + 2)?;
                    if id != 5 {
                        return Err(Error::Decode("snd data format is not sampled".into()));
                    }
                    channels = if flags & 0x40 != 0 { 2 } else { 1 };
                    offset += 6;
                }
                _ => return Err(Error::Decode("snd has multiple data formats".into())),
            }
            (d.u16_at(offset)? as usize, offset + 2)
        }
        2 => (d.u16_at(4)? as usize, 6),
        f => return Err(Error::Unsupported(format!("snd format {f}"))),
    };
    if num_commands == 0 {
        return Err(Error::Decode("snd contains no commands".into()));
    }
    offset += num_commands * 8;
    Ok((offset, channels))
}

fn decode_mace3(data: &[u8], num_channels: u16) -> Result<Vec<f32>, Error> {
    #[derive(Clone, Copy, Default)]
    struct Ch {
        index: i32,
        level: i32,
    }
    fn read_table2(ch: &mut Ch, value: usize, table: usize) -> i32 {
        let row = ((ch.index & 0x7F0) >> 4) as usize;
        let (t1, current): (&[i32], i32) = if table == 1 {
            let cur = if value < 2 {
                MACE_TABLE_4[row][value]
            } else {
                -1 - MACE_TABLE_4[row][2 * 2 - value - 1]
            };
            (&MACE_TABLE_3, cur)
        } else {
            let cur = if value < 4 {
                MACE_TABLE_2[row][value]
            } else {
                -1 - MACE_TABLE_2[row][2 * 4 - value - 1]
            };
            (&MACE_TABLE_1, cur)
        };
        let ni = ch.index + t1[value] - (ch.index >> 5);
        ch.index = if ni >= 0 { ni } else { 0 };
        current
    }
    let nch = if num_channels == 2 { 2 } else { 1 };
    let mut chans = vec![Ch::default(); nch];
    let mut out = Vec::with_capacity(data.len() * 3);
    let frame = 2 * nch;
    let mut i = 0;
    while i < data.len() {
        if i + frame > data.len() {
            return Err(Error::Decode(
                "odd number of bytes remaining in MACE3 stream".into(),
            ));
        }
        for ch in chans.iter_mut() {
            for _ in 0..2 {
                let v = data[i] as usize;
                i += 1;
                let values = [v & 7, (v >> 3) & 3, v >> 5];
                for (l, &val) in values.iter().enumerate() {
                    let cur = read_table2(ch, val, l);
                    let mut s = cur + ch.level;
                    if s > 0x7FFF {
                        s = 0x7FFF;
                    } else if s < -0x8000 {
                        s = -0x7FFF;
                    }
                    out.push(s as f32 / 32768.0);
                    ch.level = s - (s >> 3);
                }
            }
        }
    }
    Ok(out)
}

pub struct Decoded {
    pub sample_rate: u32,
    pub num_channels: u16,
    pub base_note: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub samples: Vec<f32>,
}

#[inline]
fn u8_sample(b: u8) -> f32 {
    (b as f32 - 128.0) / 128.0
}

/// snd.py `_decode_snd_data`.
pub fn decode_snd_data(d: &[u8]) -> Result<Decoded, Error> {
    let (mut offset, mut num_channels) = parse_snd_command_header(d)?;
    d.bytes_at(offset, 22)?;
    let data_bytes = d.u32_at(offset + 4)? as usize;
    let rate_fixed = d.u32_at(offset + 8)?;
    let mut loop_start = d.u32_at(offset + 12)?;
    let mut loop_end = d.u32_at(offset + 16)?;
    let encoding = d.u8_at(offset + 20)?;
    let base_note_byte = d.u8_at(offset + 21)?;
    offset += 22;
    let sample_rate = rate_fixed >> 16;
    let base_note = if base_note_byte != 0 {
        base_note_byte as u32
    } else {
        0x3C
    };
    let samples = match encoding {
        0x00 => {
            if data_bytes == 0 {
                return Err(Error::Decode("snd contains no samples".into()));
            }
            let n = data_bytes.min(d.len().saturating_sub(offset));
            pyslice(d, offset, offset + n)
                .iter()
                .map(|&b| u8_sample(b))
                .collect()
        }
        0xFE | 0xFF => {
            d.bytes_at(offset, 42)?;
            let num_frames = d.u32_at(offset)? as usize;
            let state_vars = d.u32_at(offset + 26)?;
            let compression_id = d.u16_at(offset + 34)?;
            let mut bits = d.u16_at(offset + 40)? as usize;
            let cb = offset + 42;
            match compression_id {
                3 => {
                    loop_start = loop_start.wrapping_mul(3);
                    loop_end = loop_end.wrapping_mul(3);
                    let size = num_frames * 2 * num_channels as usize;
                    decode_mace3(pyslice(d, cb, cb + size), num_channels)?
                }
                4 => return Err(Error::Unsupported("MACE6 snd".into())),
                0 => {
                    if bits == 0 {
                        bits = (state_vars >> 16) as usize;
                    }
                    if num_channels == 2
                        && num_frames * 2 * (bits / 8) == 2 * d.len().saturating_sub(cb)
                    {
                        num_channels = 1;
                    }
                    let n = num_frames * num_channels as usize * (bits / 8);
                    let sb = pyslice(d, cb, cb + n);
                    match bits {
                        8 => sb.iter().map(|&b| u8_sample(b)).collect(),
                        16 => {
                            if !sb.len().is_multiple_of(2) {
                                return Err(Error::Decode("odd 16-bit sample data".into()));
                            }
                            // snd.py compares an int to b"swot", so 16-bit data is
                            // always read big-endian; reproduced as-is.
                            sb.chunks_exact(2)
                                .map(|c| i16::from_be_bytes([c[0], c[1]]) as f32 / 32768.0)
                                .collect()
                        }
                        b => return Err(Error::Unsupported(format!("snd bits_per_sample {b}"))),
                    }
                }
                c => return Err(Error::Unsupported(format!("snd compression_id {c}"))),
            }
        }
        e => {
            return Err(Error::Decode(format!(
                "unknown encoding for snd data: {e:#04x}"
            )))
        }
    };
    Ok(Decoded {
        sample_rate,
        num_channels,
        base_note,
        loop_start,
        loop_end,
        samples,
    })
}

/// resource_dasm serialize_wav: 32-bit float PCM + an always-present 'smpl'.
pub fn serialize_wav_f32(dec: &Decoded) -> Vec<u8> {
    let ch = dec.num_channels as u32;
    let data_size = dec.samples.len() as u32 * ch * 4;
    let byte_rate = ch * dec.sample_rate * 4;
    let block_align = (ch * 4) as u16;
    let smpl_size = 0x3Cu32;
    let file_size = data_size + 28 + 8 + 8 + smpl_size;
    let mut o = Vec::with_capacity(file_size as usize + 8);
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&file_size.to_le_bytes());
    o.extend_from_slice(b"WAVEfmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&3u16.to_le_bytes());
    o.extend_from_slice(&dec.num_channels.to_le_bytes());
    o.extend_from_slice(&dec.sample_rate.to_le_bytes());
    o.extend_from_slice(&byte_rate.to_le_bytes());
    o.extend_from_slice(&block_align.to_le_bytes());
    o.extend_from_slice(&32u16.to_le_bytes());
    let period = 1_000_000_000u32.checked_div(dec.sample_rate).unwrap_or(0);
    o.extend_from_slice(b"smpl");
    for v in [
        smpl_size,
        0,
        0,
        period,
        dec.base_note,
        0,
        0,
        0,
        1,
        0x18,
        0,
        0,
        dec.loop_start.wrapping_mul(4),
        dec.loop_end.wrapping_mul(4),
        0,
        0,
    ] {
        o.extend_from_slice(&v.to_le_bytes());
    }
    o.extend_from_slice(b"data");
    o.extend_from_slice(&data_size.to_le_bytes());
    for s in &dec.samples {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

fn snd_sample_header(b: &[u8]) -> Result<(usize, usize, f64), Error> {
    let fmt = b.u16_at(0)?;
    let mut o = match fmt {
        1 => 4 + b.u16_at(2)? as usize * 6,
        2 => 4,
        f => return Err(Error::Decode(format!("unknown snd format {f}"))),
    };
    let ncmd = b.u16_at(o)? as usize;
    o += 2;
    let mut hdr = None;
    for i in 0..ncmd {
        let cmd = b.u16_at(o + i * 8)?;
        b.bytes_at(o + i * 8, 8)?;
        let p2 = b.u32_at(o + i * 8 + 4)? as usize;
        if matches!(cmd & 0x7FFF, 0x50 | 0x51 | 0x80 | 0x81) {
            hdr = Some(p2);
        }
    }
    let hdr = hdr.ok_or_else(|| Error::Decode("no buffer/sound command".into()))?;
    b.bytes_at(hdr, 22)?;
    let ln = b.u32_at(hdr + 4)? as usize;
    let rate = b.u32_at(hdr + 8)? as f64 / 65536.0;
    Ok((hdr + 22, ln, rate))
}

/// LIB40_Sound sym 0x0636, the sndS delta expander.
fn expand_snds(src: &[u8], dst_len: usize, mode: u16) -> Vec<u8> {
    let mut dst = Vec::with_capacity(dst_len);
    let (mut acc1, mut acc2): (u8, u8) = (0, 0x80);
    let n = src.len();
    let mut i = 0;
    'outer: while i < n && dst.len() < dst_len {
        let h = src[i];
        i += 1;
        let nbyte = (h & 0x1F) as usize + 1;
        let bits = (h >> 5) as i32 + 1;
        let w = 1u32 << bits;
        let mask = ((w & 0xFF) as u8).wrapping_sub(1);
        let sign = ((w >> 1) & 0xFF) as u8;
        let signext = !mask;
        let mut bitbuf: u8 = 0;
        let mut bitcount: i32 = 0;
        for _ in 0..nbyte {
            if dst.len() >= dst_len {
                break;
            }
            if i >= n {
                break 'outer;
            }
            let mut cur = src[i] as u32;
            i += 1;
            let mut pos = -bitcount;
            while pos < 8 {
                let mut v = ((((cur << bitcount) & 0xFF) as u8) | bitbuf) & mask;
                if 8 - pos < bits {
                    bitbuf = v;
                    bitcount = 8 - pos;
                    break;
                }
                if v & sign != 0 {
                    v |= signext;
                }
                if dst.len() >= dst_len {
                    break;
                }
                if mode == 1 {
                    acc1 = acc1.wrapping_add(v);
                    acc2 = acc2.wrapping_add(acc1);
                } else {
                    acc2 = acc2.wrapping_add(v);
                }
                dst.push(acc2);
                cur = (cur >> (bits - bitcount)) & 0xFF;
                bitbuf = 0;
                bitcount = 0;
                pos += bits;
            }
        }
    }
    dst
}

fn write_wav_pcm8(samples: &[u8], rate: f64) -> Vec<u8> {
    let n = samples.len() as u32;
    let r = rate.round_ties_even() as u32;
    let mut o = Vec::with_capacity(44 + samples.len());
    o.extend_from_slice(b"RIFF");
    o.extend_from_slice(&(36 + n).to_le_bytes());
    o.extend_from_slice(b"WAVEfmt ");
    o.extend_from_slice(&16u32.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&r.to_le_bytes());
    o.extend_from_slice(&r.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&8u16.to_le_bytes());
    o.extend_from_slice(b"data");
    o.extend_from_slice(&n.to_le_bytes());
    o.extend_from_slice(samples);
    o
}

pub fn to_wav(snd: &[u8], snds: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    if let Some(s) = snds {
        if s.len() != 12 {
            return Err(Error::Decode(format!(
                "sndS is {} bytes, expected 12",
                s.len()
            )));
        }
        let version = s.u16_at(0)?;
        let mode = s.u16_at(2)?;
        let expanded = s.u32_at(4)? as usize;
        if version != 1 {
            return Err(Error::Decode(format!("unsupported sndS version {version}")));
        }
        let (off, ln, rate) = snd_sample_header(snd)?;
        let decoded = expand_snds(pyslice(snd, off, off + ln), expanded, mode);
        return Ok(write_wav_pcm8(&decoded, rate));
    }
    Ok(serialize_wav_f32(&decode_snd_data(snd)?))
}
