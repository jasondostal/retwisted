//! 'cmid' -> Standard MIDI File: u32 BE length, then SoundMusicSys LZSS
//! (port of tools/twistedrip/cmid.py / resource_dasm's decoder).

use crate::util::Be;
use crate::Error;

/// SoundMusicSys LZSS: LSB-first control bits, 1 = literal, 0 = BE u16
/// back-reference {offset = len - (4096 - (p & 0xFFF)), count = (p >> 12) + 3}.
/// A stream that ends mid-control-byte just stops.
pub fn lzss_decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len() * 2);
    let n = data.len();
    let mut pos = 0;
    loop {
        if pos >= n {
            return Ok(out);
        }
        let control = data[pos];
        pos += 1;
        let mut mask = 1u8;
        for _ in 0..8 {
            if control & mask != 0 {
                if pos >= n {
                    return Ok(out);
                }
                out.push(data[pos]);
                pos += 1;
            } else {
                if pos + 1 >= n {
                    return Ok(out);
                }
                let params = ((data[pos] as usize) << 8) | data[pos + 1] as usize;
                pos += 2;
                let back = 4096 - (params & 0x0FFF);
                let count = ((params >> 12) & 0x0F) + 3;
                if back > out.len() {
                    return Err(Error::Decode(format!(
                        "cmid LZSS back-reference underflows output (offset={}, output so far={} bytes)",
                        out.len() as i64 - back as i64,
                        out.len()
                    )));
                }
                // Byte by byte: an overlapping run (back < count) replicates.
                let src = out.len() - back;
                for k in 0..count {
                    let b = out[src + k];
                    out.push(b);
                }
            }
            mask = mask.wrapping_shl(1);
        }
    }
}

pub fn decode(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut size = data.u32_at(0)? as usize;
    if size & 0xFF00_0000 != 0 {
        size &= 0x00FF_FFFF;
    }
    let out = lzss_decompress(&data[4..])?;
    if out.len() != size {
        return Err(Error::Decode(format!(
            "cmid decompression produced {:#x} bytes, expected {size:#x}",
            out.len()
        )));
    }
    Ok(out)
}
