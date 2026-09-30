//! BinHex 4.0 (`.hqx`): 6-bit text encoding + RLE90 over a small header and
//! the two forks, each followed by a CRC-16/XMODEM. Public format (RFC 1741).

use super::MacFile;
use crate::util::{mac_roman, Be};
use crate::Error;

const ALPHABET: &[u8; 64] = b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";
const MARKER: &[u8] = b"(This file must be converted with BinHex";

pub fn is_binhex(data: &[u8]) -> bool {
    find(data, MARKER).is_some()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub fn decode(text: &[u8]) -> Result<MacFile, Error> {
    let m = find(text, MARKER).ok_or_else(|| Error::Container("not BinHex 4.0".into()))?;
    let mut lut = [0xFFu8; 256];
    for (i, &c) in ALPHABET.iter().enumerate() {
        lut[c as usize] = i as u8;
    }
    // The payload starts at the first ':' after the marker line.
    let rest = &text[m + MARKER.len()..];
    let colon = rest
        .iter()
        .position(|&c| c == b':')
        .ok_or_else(|| Error::Container("BinHex: no start colon".into()))?;
    let mut bits: u32 = 0;
    let mut nbits = 0;
    let mut packed = Vec::with_capacity(rest.len() * 3 / 4);
    for &c in &rest[colon + 1..] {
        if c == b':' {
            break;
        }
        if matches!(c, b'\r' | b'\n' | b' ' | b'\t') {
            continue;
        }
        let v = lut[c as usize];
        if v == 0xFF {
            return Err(Error::Container(format!("BinHex: bad character {c:#04x}")));
        }
        bits = (bits << 6) | v as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            packed.push((bits >> nbits) as u8);
        }
    }
    // RLE90: 0x90 n repeats the previous byte n-1 more times; 0x90 0 is a literal 0x90.
    let mut out = Vec::with_capacity(packed.len());
    let mut i = 0;
    while i < packed.len() {
        let b = packed[i];
        i += 1;
        if b == 0x90 {
            let n = *packed.get(i).ok_or(Error::Truncated)?;
            i += 1;
            if n == 0 {
                out.push(0x90);
            } else {
                let prev = *out
                    .last()
                    .ok_or_else(|| Error::Container("BinHex: RLE with no previous byte".into()))?;
                for _ in 1..n {
                    out.push(prev);
                }
            }
        } else {
            out.push(b);
        }
    }

    let d = &out[..];
    let nlen = d.u8_at(0)? as usize;
    let name = mac_roman(d.bytes_at(1, nlen)?);
    let h = 1 + nlen + 1; // name, version byte
    let mut file_type = [0u8; 4];
    file_type.copy_from_slice(d.bytes_at(h, 4)?);
    let mut creator = [0u8; 4];
    creator.copy_from_slice(d.bytes_at(h + 4, 4)?);
    let dlen = d.u32_at(h + 10)? as usize;
    let rlen = d.u32_at(h + 14)? as usize;
    let hend = h + 18;
    if crc16(&d[..hend]) != d.u16_at(hend)? {
        return Err(Error::Container("BinHex: header CRC mismatch".into()));
    }
    let ds = hend + 2;
    let data = d.bytes_at(ds, dlen)?.to_vec();
    if crc16(&data) != d.u16_at(ds + dlen)? {
        return Err(Error::Container("BinHex: data fork CRC mismatch".into()));
    }
    let rs = ds + dlen + 2;
    let rsrc = d.bytes_at(rs, rlen)?.to_vec();
    if rlen > 0 && crc16(&rsrc) != d.u16_at(rs + rlen)? {
        return Err(Error::Container(
            "BinHex: resource fork CRC mismatch".into(),
        ));
    }
    Ok(MacFile {
        path: vec![name],
        file_type,
        creator,
        data,
        rsrc,
    })
}
