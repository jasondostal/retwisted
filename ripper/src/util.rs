//! Small shared helpers: bounds-checked big-endian reads and Mac OS Roman.

use crate::Error;

/// Bounds-checked big-endian field reads over a byte slice. Every read
/// returns `Err(Error::Truncated)` instead of panicking, which is what
/// Python's `struct.error` / `IndexError` turn into on this side.
pub trait Be {
    fn u8_at(&self, off: usize) -> Result<u8, Error>;
    fn u16_at(&self, off: usize) -> Result<u16, Error>;
    fn i16_at(&self, off: usize) -> Result<i16, Error>;
    fn u32_at(&self, off: usize) -> Result<u32, Error>;
    fn bytes_at(&self, off: usize, len: usize) -> Result<&[u8], Error>;
}

impl Be for [u8] {
    #[inline]
    fn u8_at(&self, off: usize) -> Result<u8, Error> {
        self.get(off).copied().ok_or(Error::Truncated)
    }
    #[inline]
    fn u16_at(&self, off: usize) -> Result<u16, Error> {
        let b = self.bytes_at(off, 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    #[inline]
    fn i16_at(&self, off: usize) -> Result<i16, Error> {
        Ok(self.u16_at(off)? as i16)
    }
    #[inline]
    fn u32_at(&self, off: usize) -> Result<u32, Error> {
        let b = self.bytes_at(off, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    #[inline]
    fn bytes_at(&self, off: usize, len: usize) -> Result<&[u8], Error> {
        let end = off.checked_add(len).ok_or(Error::Truncated)?;
        self.get(off..end).ok_or(Error::Truncated)
    }
}

/// Python-style slice: `data[start:end]`, clamped, never fails.
#[inline]
pub fn pyslice(data: &[u8], start: usize, end: usize) -> &[u8] {
    let s = start.min(data.len());
    let e = end.min(data.len()).max(s);
    &data[s..e]
}

/// Mac OS Roman 0x80..0xFF -> Unicode, exactly Python's `mac_roman` codec
/// (which maps 0xDB to U+20AC EURO SIGN, the post-1998 Apple table).
const MAC_ROMAN_HIGH: [u16; 128] = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, //
    0x00E0, 0x00E2, 0x00E4, 0x00E3, 0x00E5, 0x00E7, 0x00E9, 0x00E8, //
    0x00EA, 0x00EB, 0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3, //
    0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, //
    0x2020, 0x00B0, 0x00A2, 0x00A3, 0x00A7, 0x2022, 0x00B6, 0x00DF, //
    0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8, //
    0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5, 0x2202, 0x2211, //
    0x220F, 0x03C0, 0x222B, 0x00AA, 0x00BA, 0x03A9, 0x00E6, 0x00F8, //
    0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB, //
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, //
    0x2013, 0x2014, 0x201C, 0x201D, 0x2018, 0x2019, 0x00F7, 0x25CA, //
    0x00FF, 0x0178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02, //
    0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, //
    0x00CB, 0x00C8, 0x00CD, 0x00CE, 0x00CF, 0x00CC, 0x00D3, 0x00D4, //
    0xF8FF, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC, //
    0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD, 0x02DB, 0x02C7, //
];

#[inline]
pub fn mac_roman_char(b: u8) -> char {
    if b < 0x80 {
        b as char
    } else {
        char::from_u32(MAC_ROMAN_HIGH[(b - 0x80) as usize] as u32).unwrap_or('\u{FFFD}')
    }
}

/// Mac OS Roman bytes -> String (total: every byte maps).
pub fn mac_roman(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| mac_roman_char(b)).collect()
}

/// Latin-1 decode (how rsrc.py decodes the 4-byte resource type).
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}
