//! StuffIt fork decompressors: method 13 ("LZ+Huffman", classic and
//! StuffIt 5) and method 15 (Arsenic: arithmetic coding + BWT + MTF).
//!
//! Vendored, decoder-side only, from the `stuffit` crate 0.3.1
//! (https://github.com/benletchford/stuffit-rs), whose own archive parser
//! misreads both StuffIt 5 headers here (see container/sit.rs) and whose
//! dependency tree (rayon, flate2, encoding_rs, md5, thiserror, libc) is
//! far heavier than these two decoders. Changes: error type, visibility,
//! the two public entry points below; the decoding logic is untouched.
//!
//! Original code: Copyright (c) 2025 Ben Letchford, MIT License --
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit
//! persons to whom the Software is furnished to do so, subject to the
//! following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(clippy::all, dead_code)]

use crate::Error;

/// Method 13 (LZ77 + Huffman), identical in classic and StuffIt 5 archives.
pub fn sit13(data: &[u8], uncomp_len: usize) -> Result<Vec<u8>, Error> {
    Sit13Decoder::new(data).decompress(uncomp_len)
}

/// Method 15 (Arsenic), StuffIt 5 only.
pub fn arsenic(data: &[u8], uncomp_len: usize) -> Result<Vec<u8>, Error> {
    SitArsenicDecoder::new(data).decompress(uncomp_len)
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit_buf: u64,
    bits_in_buf: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    fn fill_buf(&mut self) {
        while self.bits_in_buf <= 56 && self.pos < self.data.len() {
            self.bit_buf |= (self.data[self.pos] as u64) << self.bits_in_buf;
            self.pos += 1;
            self.bits_in_buf += 8;
        }
    }

    // Low-bit-first reading used by SIT13. Classic streams may omit trailing
    // zero bits, so reads past the compressed fork are explicitly zero-filled;
    // the decoder's declared uncompressed length bounds total output.
    fn read_bits_le(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        self.fill_buf();
        let res = (self.bit_buf & ((1u64 << n) - 1)) as u32;
        let available = n.min(self.bits_in_buf);
        self.bit_buf >>= available;
        self.bits_in_buf -= available;
        res
    }

    fn read_bits_le_checked(&mut self, n: u32) -> Option<u32> {
        if n == 0 {
            return Some(0);
        }
        self.fill_buf();
        if self.bits_in_buf < n {
            return None;
        }
        let res = (self.bit_buf & ((1u64 << n) - 1)) as u32;
        self.bit_buf >>= n;
        self.bits_in_buf -= n;
        Some(res)
    }

    fn skip_bits_le(&mut self, mut n: u32) -> bool {
        while n > 0 {
            self.fill_buf();
            if self.bits_in_buf == 0 {
                return false;
            }
            let take = n.min(self.bits_in_buf);
            self.bit_buf >>= take;
            self.bits_in_buf -= take;
            n -= take;
        }
        true
    }

    fn read_bit_le(&mut self) -> bool {
        self.read_bits_le(1) != 0
    }

    fn read_bit_be(&mut self) -> bool {
        if self.bits_in_buf == 0 {
            if self.pos < self.data.len() {
                self.bit_buf = self.data[self.pos] as u64;
                self.pos += 1;
                self.bits_in_buf = 8;
            } else {
                return false;
            }
        }
        let res = (self.bit_buf & (1 << (self.bits_in_buf - 1))) != 0;
        self.bits_in_buf -= 1;
        res
    }

    fn read_byte(&mut self) -> Option<u8> {
        self.read_bits_le_checked(8).map(|byte| byte as u8)
    }
}

// --- HuffmanDecoder ---

struct HuffmanDecoder {
    tree: Vec<[i32; 2]>,
}

impl HuffmanDecoder {
    fn from_lengths(lengths: &[i32], num_symbols: usize) -> Self {
        let mut tree = vec![[i32::MIN, i32::MIN]];
        let mut code = 0u32;

        for length in 1i32..=32 {
            for (i, &len) in lengths.iter().enumerate().take(num_symbols) {
                if len == length {
                    let mut node = 0;
                    for bit_pos in (0..length).rev() {
                        let bit = ((code >> bit_pos) & 1) as usize;
                        if tree[node][bit] == i32::MIN {
                            tree[node][bit] = tree.len() as i32;
                            tree.push([i32::MIN, i32::MIN]);
                        }
                        node = tree[node][bit] as usize;
                    }
                    tree[node][0] = i as i32;
                    tree[node][1] = i as i32;
                    code += 1;
                }
            }
            code <<= 1;
        }
        Self { tree }
    }

    fn from_explicit_codes(codes: &[u32], lengths: &[i32], num_symbols: usize) -> Self {
        let mut tree = vec![[i32::MIN, i32::MIN]];
        for i in 0..num_symbols {
            let length = lengths[i];
            if length <= 0 {
                continue;
            }
            let code = codes[i];
            let mut node = 0;
            for bit_pos in 0..length {
                let bit = ((code >> bit_pos) & 1) as usize;
                if tree[node][bit] == i32::MIN {
                    tree[node][bit] = tree.len() as i32;
                    tree.push([i32::MIN, i32::MIN]);
                }
                node = tree[node][bit] as usize;
            }
            tree[node][0] = i as i32;
            tree[node][1] = i as i32;
        }
        Self { tree }
    }

    fn decode_le(&self, reader: &mut BitReader) -> i32 {
        let mut node = 0;
        loop {
            if self.tree[node][0] == self.tree[node][1] {
                return self.tree[node][0];
            }
            let bit = reader.read_bits_le(1) as usize;
            if bit >= 2 {
                return -1;
            }
            let next = self.tree[node][bit];
            if next == i32::MIN {
                return -1;
            }
            node = next as usize;
        }
    }
}

// --- BitWriter ---
struct Sit13Decoder<'a> {
    reader: BitReader<'a>,
}

impl<'a> Sit13Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            reader: BitReader::new(data),
        }
    }

    fn decompress(&mut self, uncomp_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = Vec::with_capacity(uncomp_len);
        if uncomp_len == 0 {
            return Ok(output);
        }

        let first_byte = self
            .reader
            .read_byte()
            .ok_or_else(|| Error::Container("Unexpected end of SIT13 stream".into()))?;
        let code = (first_byte >> 4) as usize;

        let (first_code, second_code, offset_code) = if code == 0 {
            let metacode = HuffmanDecoder::from_explicit_codes(&META_CODES, &META_CODE_LENGTHS, 37);
            let first = self.alloc_and_parse_code(321, &metacode)?;
            let second = if (first_byte & 0x08) != 0 {
                HuffmanDecoder {
                    tree: first.tree.clone(),
                }
            } else {
                self.alloc_and_parse_code(321, &metacode)?
            };
            let offset_size = (first_byte & 0x07) as usize + 10;
            let offset = self.alloc_and_parse_code(offset_size, &metacode)?;
            (first, second, offset)
        } else if code < 6 {
            let idx = code - 1;
            (
                HuffmanDecoder::from_lengths(FIRST_CODE_LENGTHS[idx], 321),
                HuffmanDecoder::from_lengths(SECOND_CODE_LENGTHS[idx], 321),
                HuffmanDecoder::from_lengths(OFFSET_CODE_LENGTHS[idx], OFFSET_CODE_SIZES[idx]),
            )
        } else {
            return Err(Error::Container(format!("Invalid SIT13 code: {}", code)));
        };

        let mut current_huffman = &first_code;
        while output.len() < uncomp_len {
            let val = current_huffman.decode_le(&mut self.reader);
            if val < 0 {
                break;
            }
            if val < 256 {
                output.push(val as u8);
                current_huffman = &first_code;
            } else if val < 320 {
                current_huffman = &second_code;
                let mut length = (val - 256 + 3) as usize;
                if val == 318 {
                    length = (self.reader.read_bits_le(10) + 65) as usize;
                } else if val == 319 {
                    length = (self.reader.read_bits_le(15) + 65) as usize;
                }

                let bit_len = offset_code.decode_le(&mut self.reader);
                if bit_len < 0 {
                    break;
                }
                let offset = if bit_len == 0 {
                    1
                } else if bit_len == 1 {
                    2
                } else {
                    (1 << (bit_len - 1)) + self.reader.read_bits_le(bit_len as u32 - 1) + 1
                } as usize;

                if offset > output.len() {
                    break;
                }
                for _ in 0..length {
                    if output.len() >= uncomp_len {
                        break;
                    }
                    let b = output[output.len() - offset];
                    output.push(b);
                }
            } else {
                break;
            }
        }

        Ok(output)
    }

    fn alloc_and_parse_code(
        &mut self,
        num_codes: usize,
        metacode: &HuffmanDecoder,
    ) -> Result<HuffmanDecoder, Error> {
        alloc_and_parse_huffman_code(&mut self.reader, num_codes, metacode)
    }
}

// Standalone helper for use by Method 3 (Huffman) and Method 13 (SIT13)
fn alloc_and_parse_huffman_code(
    reader: &mut BitReader,
    num_codes: usize,
    metacode: &HuffmanDecoder,
) -> Result<HuffmanDecoder, Error> {
    let mut lengths = vec![0i32; num_codes];
    let mut length = 0i32;
    let mut i = 0;
    while i < num_codes {
        let val = metacode.decode_le(reader);
        if val < 0 {
            return Err(Error::Container("Invalid meta code".into()));
        }
        match val {
            31 => length = -1,
            32 => length += 1,
            33 => length -= 1,
            34 => {
                if reader.read_bit_le() {
                    lengths[i] = length;
                    i += 1;
                }
            }
            35 => {
                let mut count = reader.read_bits_le(3) as usize + 2;
                while count > 0 && i < num_codes {
                    lengths[i] = length;
                    i += 1;
                    count -= 1;
                }
            }
            36 => {
                let mut count = reader.read_bits_le(6) as usize + 10;
                while count > 0 && i < num_codes {
                    lengths[i] = length;
                    i += 1;
                    count -= 1;
                }
            }
            _ => length = val + 1,
        }
        if i < num_codes {
            lengths[i] = length;
            i += 1;
        }
    }
    Ok(HuffmanDecoder::from_lengths(&lengths, num_codes))
}

// --- StuffIt 3 (Huffman) Implementation ---
//
// Tree format (XADStuffItHuffmanHandle.m from The Unarchiver):
//   Read bits big-endian (MSB first).
//   bit=1 → leaf node; next 8 bits (BE) are the symbol value.
//   bit=0 → internal node; recursively build 0-branch then 1-branch.
// Symbols are decoded using the same BE bit order.

struct ArithmeticModel {
    first_symbol: u16,
    num_symbols: usize,
    frequencies: Vec<u16>,
    total_frequency: u32,
    increment: u16,
    limit: u32,
}

impl ArithmeticModel {
    fn new(first_symbol: u16, num_symbols: usize, increment: u16, limit: u32) -> Self {
        Self {
            first_symbol,
            num_symbols,
            frequencies: vec![increment; num_symbols],
            total_frequency: num_symbols as u32 * increment as u32,
            increment,
            limit,
        }
    }

    fn reset(&mut self) {
        self.total_frequency = self.num_symbols as u32 * self.increment as u32;
        self.frequencies.fill(self.increment);
    }

    fn update(&mut self, sym_idx: usize) {
        self.frequencies[sym_idx] += self.increment;
        self.total_frequency += self.increment as u32;
        if self.total_frequency > self.limit {
            self.total_frequency = 0;
            for f in &mut self.frequencies {
                *f = (*f + 1) >> 1;
                self.total_frequency += *f as u32;
            }
        }
    }
}

struct ArithmeticDecoder<'a> {
    reader: BitReader<'a>,
    range: u32,
    code: u32,
}

const ARITH_BITS: u32 = 26;
const ARITH_ONE: u32 = 1 << (ARITH_BITS - 1);
const ARITH_HALF: u32 = 1 << (ARITH_BITS - 2);

impl<'a> ArithmeticDecoder<'a> {
    fn new(mut reader: BitReader<'a>) -> Self {
        let mut code = 0;
        for _ in 0..ARITH_BITS {
            code = (code << 1) | (reader.read_bit_be() as u32);
        }
        Self {
            reader,
            range: ARITH_ONE,
            code,
        }
    }

    fn next_symbol(&mut self, model: &mut ArithmeticModel) -> u16 {
        let freq = self.code / (self.range / model.total_frequency);
        let mut cumulative = 0;
        let mut n = 0;
        while n < model.num_symbols - 1 {
            if cumulative + model.frequencies[n] as u32 > freq {
                break;
            }
            cumulative += model.frequencies[n] as u32;
            n += 1;
        }

        let sym_size = model.frequencies[n] as u32;
        let sym_tot = model.total_frequency;

        let renorm_factor = self.range / sym_tot;
        let low_incr = renorm_factor * cumulative;
        self.code -= low_incr;
        if cumulative + sym_size == sym_tot {
            self.range -= low_incr;
        } else {
            self.range = sym_size * renorm_factor;
        }

        while self.range <= ARITH_HALF {
            self.range <<= 1;
            self.code = (self.code << 1) | (self.reader.read_bit_be() as u32);
        }

        let res = model.first_symbol + n as u16;
        model.update(n);
        res
    }

    fn read_bit_string(&mut self, model: &mut ArithmeticModel, n: u32) -> u32 {
        let mut res = 0;
        for i in 0..n {
            if self.next_symbol(model) != 0 {
                res |= 1 << i;
            }
        }
        res
    }
}

// --- Arithmetic Encoder (inverse of ArithmeticDecoder) ---

struct SitArsenicDecoder<'a> {
    decoder: ArithmeticDecoder<'a>,
}

impl<'a> SitArsenicDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            decoder: ArithmeticDecoder::new(BitReader::new(data)),
        }
    }

    fn decompress(&mut self, uncomp_len: usize) -> Result<Vec<u8>, Error> {
        let mut output = Vec::with_capacity(uncomp_len);
        let mut initial_model = ArithmeticModel::new(0, 2, 1, 256);

        if self.decoder.read_bit_string(&mut initial_model, 8) != 'A' as u32 {
            return Err(Error::Container("Invalid Arsenic signature (A)".into()));
        }
        if self.decoder.read_bit_string(&mut initial_model, 8) != 's' as u32 {
            return Err(Error::Container("Invalid Arsenic signature (s)".into()));
        }

        let block_bits = self.decoder.read_bit_string(&mut initial_model, 4) + 9;
        let block_size = 1 << block_bits;

        let mut selector_model = ArithmeticModel::new(0, 11, 8, 1024);
        let mut mtf_models = [
            ArithmeticModel::new(2, 2, 8, 1024),
            ArithmeticModel::new(4, 4, 4, 1024),
            ArithmeticModel::new(8, 8, 4, 1024),
            ArithmeticModel::new(16, 16, 4, 1024),
            ArithmeticModel::new(32, 32, 2, 1024),
            ArithmeticModel::new(64, 64, 2, 1024),
            ArithmeticModel::new(128, 128, 1, 1024),
        ];

        while output.len() < uncomp_len {
            if self.decoder.next_symbol(&mut initial_model) != 0 {
                break;
            }

            let randomized = self.decoder.next_symbol(&mut initial_model) != 0;
            let transform_index_start =
                self.decoder.read_bit_string(&mut initial_model, block_bits) as usize;

            let mut block = Vec::with_capacity(block_size);
            // Move-to-front list. Fetching index `symbol` and moving it to
            // the head is a memmove of `symbol` bytes -- and MTF exists
            // precisely so that `symbol` is usually tiny.
            let mut mtf: [u8; 256] = std::array::from_fn(|index| index as u8);

            loop {
                let sel = self.decoder.next_symbol(&mut selector_model);
                if sel <= 1 {
                    let mut zero_state = 1;
                    let mut zero_count = 0;
                    let mut current_sel = sel;
                    while current_sel < 2 {
                        if current_sel == 0 {
                            zero_count += zero_state;
                        } else {
                            zero_count += 2 * zero_state;
                        }
                        zero_state *= 2;
                        current_sel = self.decoder.next_symbol(&mut selector_model);
                    }
                    let sym = mtf[0];
                    for _ in 0..zero_count {
                        block.push(sym);
                    }
                    if current_sel == 10 {
                        break;
                    }
                    let symbol = if current_sel == 2 {
                        1
                    } else {
                        self.decoder
                            .next_symbol(&mut mtf_models[current_sel as usize - 3])
                            as usize
                    };
                    let val = mtf[symbol];
                    mtf.copy_within(0..symbol, 1);
                    mtf[0] = val;
                    block.push(val);
                } else if sel == 10 {
                    break;
                } else {
                    let symbol = if sel == 2 {
                        1
                    } else {
                        self.decoder.next_symbol(&mut mtf_models[sel as usize - 3]) as usize
                    };
                    let val = mtf[symbol];
                    mtf.copy_within(0..symbol, 1);
                    mtf[0] = val;
                    block.push(val);
                }
            }

            // A block is at most 2^block_bits <= 2^24 symbols; a longer one
            // (corrupt stream) cannot be walked with the packed transform
            // below and is treated like a corrupt start index.
            if transform_index_start >= block.len() || block.len() > (1 << 24) {
                break;
            }

            selector_model.reset();
            for m in &mut mtf_models {
                m.reset();
            }

            // Inverse transform table, one u32 per position: the successor
            // index in the high 24 bits (blocks hold at most 2^24 symbols)
            // and the byte found there in the low 8, so the random walk
            // below touches one cache line per output byte instead of two.
            let mut transform = vec![0u32; block.len()];

            // Optimized 4-way parallel histogram to reduce cache conflicts
            let mut counts0 = [0usize; 256];
            let mut counts1 = [0usize; 256];
            let mut counts2 = [0usize; 256];
            let mut counts3 = [0usize; 256];

            let chunks = block.chunks_exact(4);
            let remainder = chunks.remainder();
            for chunk in chunks {
                counts0[chunk[0] as usize] += 1;
                counts1[chunk[1] as usize] += 1;
                counts2[chunk[2] as usize] += 1;
                counts3[chunk[3] as usize] += 1;
            }
            for &b in remainder {
                counts0[b as usize] += 1;
            }

            // Merge counts
            let mut counts = [0usize; 256];
            for i in 0..256 {
                counts[i] = counts0[i] + counts1[i] + counts2[i] + counts3[i];
            }

            // Compute prefix sums
            let mut sum = 0usize;
            let mut start_pos = [0usize; 256];
            for i in 0..256 {
                start_pos[i] = sum;
                sum += counts[i];
            }

            // Build transform vector
            let mut current_pos_in_counts = start_pos;
            for (i, &b) in block.iter().enumerate() {
                transform[current_pos_in_counts[b as usize]] = ((i as u32) << 8) | u32::from(b);
                current_pos_in_counts[b as usize] += 1;
            }

            let mut byte_count = 0;
            let mut idx = transform_index_start;
            let mut count = 0;
            let mut last = 0u8;
            let mut repeat = 0;
            let mut rand_idx = 0;
            let mut rand_val = RANDOMIZATION_TABLE[0] as usize;

            while (byte_count < block.len() || repeat > 0) && output.len() < uncomp_len {
                if repeat > 0 {
                    output.push(last);
                    repeat -= 1;
                } else {
                    let entry = transform[idx];
                    idx = (entry >> 8) as usize;
                    let mut b = entry as u8;

                    if randomized && rand_val == byte_count {
                        b ^= 1;
                        rand_idx = (rand_idx + 1) & 255;
                        rand_val += RANDOMIZATION_TABLE[rand_idx] as usize;
                    }
                    byte_count += 1;

                    if count == 4 {
                        count = 0;
                        if b == 0 {
                            continue;
                        }
                        repeat = (b - 1) as usize;
                        output.push(last);
                    } else {
                        if b == last {
                            count += 1;
                        } else {
                            count = 1;
                            last = b;
                        }
                        output.push(b);
                    }
                }
            }
        }

        Ok(output)
    }
}

const META_CODES: [u32; 37] = [
    0x5d8, 0x058, 0x040, 0x0c0, 0x000, 0x078, 0x02b, 0x014, 0x00c, 0x01c, 0x01b, 0x00b, 0x010,
    0x020, 0x038, 0x018, 0x0d8, 0xbd8, 0x180, 0x680, 0x380, 0xf80, 0x780, 0x480, 0x080, 0x280,
    0x3d8, 0xfd8, 0x7d8, 0x9d8, 0x1d8, 0x004, 0x001, 0x002, 0x007, 0x003, 0x008,
];
const META_CODE_LENGTHS: [i32; 37] = [
    11, 8, 8, 8, 8, 7, 6, 5, 5, 5, 5, 6, 5, 6, 7, 7, 9, 12, 10, 11, 11, 12, 12, 11, 11, 11, 12, 12,
    12, 12, 12, 5, 2, 2, 3, 4, 5,
];

const RANDOMIZATION_TABLE: [u16; 256] = [
    0xee, 0x56, 0xf8, 0xc3, 0x9d, 0x9f, 0xae, 0x2c, 0xad, 0xcd, 0x24, 0x9d, 0xa6, 0x101, 0x18,
    0xb9, 0xa1, 0x82, 0x75, 0xe9, 0x9f, 0x55, 0x66, 0x6a, 0x86, 0x71, 0xdc, 0x84, 0x56, 0x96, 0x56,
    0xa1, 0x84, 0x78, 0xb7, 0x32, 0x6a, 0x3, 0xe3, 0x2, 0x11, 0x101, 0x8, 0x44, 0x83, 0x100, 0x43,
    0xe3, 0x1c, 0xf0, 0x86, 0x6a, 0x6b, 0xf, 0x3, 0x2d, 0x86, 0x17, 0x7b, 0x10, 0xf6, 0x80, 0x78,
    0x7a, 0xa1, 0xe1, 0xef, 0x8c, 0xf6, 0x87, 0x4b, 0xa7, 0xe2, 0x77, 0xfa, 0xb8, 0x81, 0xee, 0x77,
    0xc0, 0x9d, 0x29, 0x20, 0x27, 0x71, 0x12, 0xe0, 0x6b, 0xd1, 0x7c, 0xa, 0x89, 0x7d, 0x87, 0xc4,
    0x101, 0xc1, 0x31, 0xaf, 0x38, 0x3, 0x68, 0x1b, 0x76, 0x79, 0x3f, 0xdb, 0xc7, 0x1b, 0x36, 0x7b,
    0xe2, 0x63, 0x81, 0xee, 0xc, 0x63, 0x8b, 0x78, 0x38, 0x97, 0x9b, 0xd7, 0x8f, 0xdd, 0xf2, 0xa3,
    0x77, 0x8c, 0xc3, 0x39, 0x20, 0xb3, 0x12, 0x11, 0xe, 0x17, 0x42, 0x80, 0x2c, 0xc4, 0x92, 0x59,
    0xc8, 0xdb, 0x40, 0x76, 0x64, 0xb4, 0x55, 0x1a, 0x9e, 0xfe, 0x5f, 0x6, 0x3c, 0x41, 0xef, 0xd4,
    0xaa, 0x98, 0x29, 0xcd, 0x1f, 0x2, 0xa8, 0x87, 0xd2, 0xa0, 0x93, 0x98, 0xef, 0xc, 0x43, 0xed,
    0x9d, 0xc2, 0xeb, 0x81, 0xe9, 0x64, 0x23, 0x68, 0x1e, 0x25, 0x57, 0xde, 0x9a, 0xcf, 0x7f, 0xe5,
    0xba, 0x41, 0xea, 0xea, 0x36, 0x1a, 0x28, 0x79, 0x20, 0x5e, 0x18, 0x4e, 0x7c, 0x8e, 0x58, 0x7a,
    0xef, 0x91, 0x2, 0x93, 0xbb, 0x56, 0xa1, 0x49, 0x1b, 0x79, 0x92, 0xf3, 0x58, 0x4f, 0x52, 0x9c,
    0x2, 0x77, 0xaf, 0x2a, 0x8f, 0x49, 0xd0, 0x99, 0x4d, 0x98, 0x101, 0x60, 0x93, 0x100, 0x75,
    0x31, 0xce, 0x49, 0x20, 0x56, 0x57, 0xe2, 0xf5, 0x26, 0x2b, 0x8a, 0xbf, 0xde, 0xd0, 0x83, 0x34,
    0xf4, 0x17,
];

const OFFSET_CODE_SIZES: [usize; 5] = [11, 13, 14, 11, 11];

const FIRST_CODE_LENGTHS: [&[i32]; 5] = [
    // FirstCodeLengths_1 from XADStuffIt13Handle.m
    &[
        4, 5, 7, 8, 8, 9, 9, 9, 9, 7, 9, 9, 9, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 9, 9, 10, 10, 9,
        10, 9, 9, 5, 9, 9, 9, 9, 10, 9, 9, 9, 9, 9, 9, 9, 9, 7, 9, 9, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9, 9, 9, 9, 9, 8, 9, 9, 8, 8, 9, 9, 9, 9, 9, 9, 9, 7, 8, 9, 7, 9, 9, 7, 7, 9, 9, 9, 9,
        10, 9, 10, 10, 10, 9, 9, 9, 5, 9, 8, 7, 5, 9, 8, 8, 7, 9, 9, 8, 8, 5, 5, 7, 10, 5, 8, 5, 8,
        9, 9, 9, 9, 9, 10, 9, 9, 10, 9, 9, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10,
        10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10,
        10, 10, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10,
        10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 9, 9, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 9, 10, 9, 5, 6, 5, 5, 8, 9,
        9, 9, 9, 9, 9, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 9, 10, 9, 9, 9, 10, 9, 10, 9, 10, 9, 10, 9, 10, 10, 10, 9, 10, 9,
        10, 10, 9, 9, 9, 6, 9, 9, 10, 9, 5,
    ],
    &[
        4, 7, 7, 8, 7, 8, 8, 8, 8, 7, 8, 7, 8, 7, 9, 8, 8, 8, 9, 9, 9, 9, 10, 10, 9, 10, 10, 10,
        10, 10, 9, 9, 5, 9, 8, 9, 9, 11, 10, 9, 8, 9, 9, 9, 8, 9, 7, 8, 8, 8, 9, 9, 9, 9, 9, 10, 9,
        9, 9, 10, 9, 9, 10, 9, 8, 8, 7, 7, 7, 8, 8, 9, 8, 8, 9, 9, 8, 8, 7, 8, 7, 10, 8, 7, 7, 9,
        9, 9, 9, 10, 10, 11, 11, 11, 10, 9, 8, 6, 8, 7, 7, 5, 7, 7, 7, 6, 9, 8, 6, 7, 6, 6, 7, 9,
        6, 6, 6, 7, 8, 8, 8, 8, 9, 10, 9, 10, 9, 9, 8, 9, 10, 10, 9, 10, 10, 9, 9, 10, 10, 10, 10,
        10, 10, 10, 9, 10, 10, 11, 10, 10, 10, 10, 10, 10, 10, 11, 10, 11, 10, 10, 9, 11, 10, 10,
        10, 10, 10, 10, 9, 9, 10, 11, 10, 11, 10, 11, 10, 12, 10, 11, 10, 12, 11, 12, 10, 12, 10,
        11, 10, 11, 11, 11, 9, 10, 11, 11, 11, 12, 12, 10, 10, 10, 11, 11, 10, 11, 10, 10, 9, 11,
        10, 11, 10, 11, 11, 11, 10, 11, 11, 12, 11, 11, 10, 10, 10, 11, 10, 10, 11, 11, 12, 10, 10,
        11, 11, 12, 11, 11, 10, 11, 9, 12, 10, 11, 11, 11, 10, 11, 10, 11, 10, 11, 9, 10, 9, 7, 3,
        5, 6, 6, 7, 7, 8, 8, 8, 9, 9, 9, 11, 10, 10, 10, 12, 13, 11, 12, 12, 11, 13, 12, 12, 11,
        12, 12, 13, 12, 14, 13, 14, 13, 15, 13, 14, 15, 15, 14, 13, 15, 15, 14, 15, 14, 15, 15, 14,
        15, 13, 13, 14, 15, 15, 14, 14, 16, 16, 15, 15, 15, 12, 15, 10,
    ],
    &[
        6, 6, 6, 6, 6, 9, 8, 8, 4, 9, 8, 9, 8, 9, 9, 9, 8, 9, 9, 10, 8, 10, 10, 10, 9, 10, 10, 10,
        9, 10, 10, 9, 9, 9, 8, 10, 9, 10, 9, 10, 9, 10, 9, 10, 9, 9, 8, 9, 8, 9, 9, 9, 10, 10, 10,
        10, 9, 9, 9, 10, 9, 10, 9, 9, 7, 8, 8, 9, 8, 9, 9, 9, 8, 9, 9, 10, 9, 9, 8, 9, 8, 9, 8, 8,
        8, 9, 9, 9, 9, 9, 10, 10, 10, 10, 10, 9, 8, 8, 9, 8, 9, 7, 8, 8, 9, 8, 10, 10, 8, 9, 8, 8,
        8, 10, 8, 8, 8, 8, 9, 9, 9, 9, 10, 10, 10, 10, 10, 9, 7, 9, 9, 10, 10, 10, 10, 10, 9, 10,
        10, 10, 10, 10, 10, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 9, 10,
        10, 10, 10, 10, 10, 10, 9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 9, 10, 10, 10, 10, 9, 8, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 9,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 9, 10, 10, 10, 10, 10, 10, 9,
        10, 10, 10, 10, 10, 10, 9, 9, 9, 10, 10, 10, 10, 10, 10, 9, 9, 10, 9, 9, 8, 9, 8, 9, 4, 6,
        6, 6, 7, 8, 8, 9, 9, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 7, 10, 10, 10, 7, 10, 10, 7, 7, 7, 7, 7, 6, 7, 10, 7, 7, 10, 7, 7, 7, 6, 7, 6,
        6, 7, 7, 6, 6, 9, 6, 9, 10, 6, 10,
    ],
    &[
        2, 6, 6, 7, 7, 8, 7, 8, 7, 8, 8, 9, 8, 9, 9, 9, 8, 8, 9, 9, 9, 10, 10, 9, 8, 10, 9, 10, 9,
        10, 9, 9, 6, 9, 8, 9, 9, 10, 9, 9, 9, 10, 9, 9, 9, 9, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9,
        9, 9, 9, 10, 10, 9, 7, 7, 8, 8, 8, 8, 9, 9, 7, 8, 9, 10, 8, 8, 7, 8, 8, 10, 8, 8, 8, 9, 8,
        9, 9, 10, 9, 11, 10, 11, 9, 9, 8, 7, 9, 8, 8, 6, 8, 8, 8, 7, 10, 9, 7, 8, 7, 7, 8, 10, 7,
        7, 7, 8, 9, 9, 9, 9, 10, 11, 9, 11, 10, 9, 7, 9, 10, 10, 10, 11, 11, 10, 10, 11, 10, 10,
        10, 11, 11, 10, 9, 10, 10, 11, 10, 11, 10, 11, 10, 10, 10, 11, 10, 11, 10, 10, 9, 10, 10,
        11, 10, 10, 10, 10, 9, 10, 10, 10, 10, 11, 10, 11, 10, 11, 10, 11, 11, 11, 10, 12, 10, 11,
        10, 11, 10, 11, 11, 10, 8, 10, 10, 11, 10, 11, 11, 11, 10, 11, 10, 11, 10, 11, 11, 11, 9,
        10, 11, 11, 10, 11, 11, 11, 10, 11, 11, 11, 10, 10, 10, 10, 10, 11, 10, 10, 11, 11, 10, 10,
        9, 11, 10, 10, 11, 11, 10, 10, 10, 11, 10, 10, 10, 10, 10, 10, 9, 11, 10, 10, 8, 10, 8, 6,
        5, 6, 6, 7, 7, 8, 8, 8, 9, 10, 11, 10, 10, 11, 11, 12, 12, 10, 11, 12, 12, 12, 12, 13, 13,
        13, 13, 13, 12, 13, 13, 15, 14, 12, 14, 15, 16, 12, 12, 13, 15, 14, 16, 15, 17, 18, 15, 17,
        16, 15, 15, 15, 15, 13, 13, 10, 14, 12, 13, 17, 17, 18, 10, 17, 4,
    ],
    &[
        7, 9, 9, 9, 9, 9, 9, 9, 9, 8, 9, 9, 9, 7, 9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 9, 10, 9, 10, 9,
        10, 9, 9, 5, 9, 7, 9, 9, 9, 9, 9, 7, 7, 7, 9, 7, 7, 8, 7, 8, 8, 7, 7, 9, 9, 9, 9, 7, 7, 7,
        9, 9, 9, 9, 9, 9, 7, 9, 7, 7, 7, 7, 9, 9, 7, 9, 9, 7, 7, 7, 7, 7, 9, 7, 8, 7, 9, 9, 9, 9,
        9, 9, 9, 9, 9, 9, 9, 9, 7, 8, 7, 7, 7, 8, 8, 6, 7, 9, 7, 7, 8, 7, 5, 6, 9, 5, 7, 5, 6, 7,
        7, 9, 8, 9, 9, 9, 9, 9, 9, 9, 9, 10, 9, 10, 10, 10, 9, 9, 10, 10, 10, 10, 10, 10, 10, 9,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 9, 10, 10, 10, 9, 9, 10, 9, 9,
        9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9,
        10, 10, 10, 9, 10, 10, 10, 9, 9, 9, 10, 10, 10, 10, 10, 9, 10, 9, 10, 10, 9, 10, 10, 9, 10,
        10, 10, 10, 10, 10, 10, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9,
        10, 10, 10, 10, 10, 10, 10, 9, 10, 9, 10, 9, 10, 10, 9, 5, 6, 8, 8, 7, 7, 7, 9, 9, 9, 9, 9,
        9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 10, 10, 10, 10, 10, 10, 10,
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 9, 10, 10, 5, 10, 8, 9, 8,
        9,
    ],
];
const SECOND_CODE_LENGTHS: [&[i32]; 5] = [
    &[
        4, 5, 6, 6, 7, 7, 6, 7, 7, 7, 6, 8, 7, 8, 8, 8, 8, 9, 6, 9, 8, 9, 8, 9, 9, 9, 8, 10, 5, 9,
        7, 9, 6, 9, 8, 10, 9, 10, 8, 8, 9, 9, 7, 9, 8, 9, 8, 9, 8, 8, 6, 9, 9, 8, 8, 9, 9, 10, 8,
        9, 9, 10, 8, 10, 8, 8, 8, 8, 8, 9, 7, 10, 6, 9, 9, 11, 7, 8, 8, 9, 8, 10, 7, 8, 6, 9, 10,
        9, 9, 10, 8, 11, 9, 11, 9, 10, 9, 8, 9, 8, 8, 8, 8, 10, 9, 9, 10, 10, 8, 9, 8, 8, 8, 11, 9,
        8, 8, 9, 9, 10, 8, 11, 10, 10, 8, 10, 9, 10, 8, 9, 9, 11, 9, 11, 9, 10, 10, 11, 10, 12, 9,
        12, 10, 11, 10, 11, 9, 10, 10, 11, 10, 11, 10, 11, 10, 11, 10, 10, 10, 9, 9, 9, 8, 7, 6, 8,
        11, 11, 9, 12, 10, 12, 9, 11, 11, 11, 10, 12, 11, 11, 10, 12, 10, 11, 10, 10, 10, 11, 10,
        11, 11, 11, 9, 12, 10, 12, 11, 12, 10, 11, 10, 12, 11, 12, 11, 12, 11, 12, 10, 12, 11, 12,
        11, 11, 10, 12, 10, 11, 10, 12, 10, 12, 10, 12, 10, 11, 11, 11, 10, 11, 11, 11, 10, 12, 11,
        12, 10, 10, 11, 11, 9, 12, 11, 12, 10, 11, 10, 12, 10, 11, 10, 12, 10, 11, 10, 7, 5, 4, 6,
        6, 7, 7, 7, 8, 8, 7, 7, 6, 8, 6, 7, 7, 9, 8, 9, 9, 10, 11, 11, 11, 12, 11, 10, 11, 12, 11,
        12, 11, 12, 12, 12, 12, 11, 12, 12, 11, 12, 11, 12, 11, 13, 11, 12, 10, 13, 10, 14, 14, 13,
        14, 15, 14, 16, 15, 15, 18, 18, 18, 9, 18, 8,
    ],
    &[
        5, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 8, 7, 8, 7, 7, 7, 8, 8, 8, 8, 9, 8, 9, 8, 9, 9, 9, 7, 9,
        8, 8, 6, 9, 8, 9, 8, 9, 8, 9, 8, 9, 8, 9, 8, 9, 8, 8, 8, 8, 8, 9, 8, 9, 8, 9, 9, 10, 8, 10,
        8, 9, 9, 8, 8, 8, 7, 8, 8, 9, 8, 9, 7, 9, 8, 10, 8, 9, 8, 9, 8, 9, 8, 8, 8, 9, 9, 9, 9, 10,
        9, 11, 9, 10, 9, 10, 8, 8, 8, 9, 8, 8, 8, 9, 9, 8, 9, 10, 8, 9, 8, 8, 8, 11, 8, 7, 8, 9, 9,
        9, 9, 10, 9, 10, 9, 10, 9, 8, 8, 9, 9, 10, 9, 10, 9, 10, 8, 10, 9, 10, 9, 11, 10, 11, 9,
        11, 10, 10, 10, 11, 9, 11, 9, 10, 9, 11, 9, 11, 10, 10, 9, 10, 9, 9, 8, 10, 9, 11, 9, 9, 9,
        11, 10, 11, 9, 11, 9, 11, 9, 11, 10, 11, 10, 11, 10, 11, 9, 10, 10, 11, 10, 10, 8, 10, 9,
        10, 10, 11, 9, 11, 9, 10, 10, 11, 9, 10, 10, 9, 9, 10, 9, 10, 9, 10, 9, 10, 9, 11, 9, 11,
        10, 10, 9, 10, 9, 11, 9, 11, 9, 11, 9, 10, 9, 11, 9, 11, 9, 11, 9, 10, 8, 11, 9, 10, 9, 10,
        9, 10, 8, 10, 8, 9, 8, 9, 8, 7, 4, 4, 5, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 7, 8, 8, 9, 9, 10,
        10, 10, 10, 10, 10, 11, 11, 10, 10, 12, 11, 11, 12, 12, 11, 12, 12, 11, 12, 12, 12, 12, 12,
        12, 11, 12, 11, 13, 12, 13, 12, 13, 14, 14, 14, 15, 13, 14, 13, 14, 18, 18, 17, 7, 16, 9,
    ],
    &[
        5, 6, 6, 6, 6, 7, 7, 7, 6, 8, 7, 8, 7, 9, 8, 8, 7, 7, 8, 9, 9, 9, 9, 10, 8, 9, 9, 10, 8,
        10, 9, 8, 6, 10, 8, 10, 8, 10, 9, 9, 9, 9, 9, 10, 9, 9, 8, 9, 8, 9, 8, 9, 9, 10, 9, 10, 9,
        9, 8, 10, 9, 11, 10, 8, 8, 8, 8, 9, 7, 9, 9, 10, 8, 9, 8, 11, 9, 10, 9, 10, 8, 9, 9, 9, 9,
        8, 9, 9, 10, 10, 10, 12, 10, 11, 10, 10, 8, 9, 9, 9, 8, 9, 8, 8, 10, 9, 10, 11, 8, 10, 9,
        9, 8, 12, 8, 9, 9, 9, 9, 8, 9, 10, 9, 12, 10, 10, 10, 8, 7, 11, 10, 9, 10, 11, 9, 11, 7,
        11, 10, 12, 10, 12, 10, 11, 9, 11, 9, 12, 10, 12, 10, 12, 10, 9, 11, 12, 10, 12, 10, 11, 9,
        10, 9, 10, 9, 11, 11, 12, 9, 10, 8, 12, 11, 12, 9, 12, 10, 12, 10, 13, 10, 12, 10, 12, 10,
        12, 10, 9, 10, 12, 10, 9, 8, 11, 10, 12, 10, 12, 10, 12, 10, 11, 10, 12, 8, 12, 10, 11, 10,
        10, 10, 12, 9, 11, 10, 12, 10, 12, 11, 12, 10, 9, 10, 12, 9, 10, 10, 12, 10, 11, 10, 11,
        10, 12, 8, 12, 9, 12, 8, 12, 8, 11, 10, 11, 10, 11, 9, 10, 8, 10, 9, 9, 8, 9, 8, 7, 4, 3,
        5, 5, 6, 5, 6, 6, 7, 7, 8, 8, 8, 7, 7, 7, 9, 8, 9, 9, 11, 9, 11, 9, 8, 9, 9, 11, 12, 11,
        12, 12, 13, 13, 12, 13, 14, 13, 14, 13, 14, 13, 13, 13, 12, 13, 13, 12, 13, 13, 14, 14, 13,
        13, 14, 14, 14, 14, 15, 18, 17, 18, 8, 16, 10,
    ],
    &[
        4, 5, 6, 6, 6, 6, 7, 7, 6, 7, 7, 9, 6, 8, 8, 7, 7, 8, 8, 8, 6, 9, 8, 8, 7, 9, 8, 9, 8, 9,
        8, 9, 6, 9, 8, 9, 8, 10, 9, 9, 8, 10, 8, 10, 8, 9, 8, 9, 8, 8, 7, 9, 9, 9, 9, 9, 8, 10, 9,
        10, 9, 10, 9, 8, 7, 8, 9, 9, 8, 9, 9, 9, 7, 10, 9, 10, 9, 9, 8, 9, 8, 9, 8, 8, 8, 9, 9, 10,
        9, 9, 8, 11, 9, 11, 10, 10, 8, 8, 10, 8, 8, 9, 9, 9, 10, 9, 10, 11, 9, 9, 9, 9, 8, 9, 8, 8,
        8, 10, 10, 9, 9, 8, 10, 11, 10, 11, 11, 9, 8, 9, 10, 11, 9, 10, 11, 11, 9, 12, 10, 10, 10,
        12, 11, 11, 9, 11, 11, 12, 9, 11, 9, 10, 10, 10, 10, 12, 9, 11, 10, 11, 9, 11, 11, 11, 10,
        11, 11, 12, 9, 10, 10, 12, 11, 11, 10, 11, 9, 11, 10, 11, 10, 11, 9, 11, 11, 9, 8, 11, 10,
        11, 11, 10, 7, 12, 11, 11, 11, 11, 11, 12, 10, 12, 11, 13, 11, 10, 12, 11, 10, 11, 10, 11,
        10, 11, 11, 11, 10, 12, 11, 11, 10, 11, 10, 10, 10, 11, 10, 12, 11, 12, 10, 11, 9, 11, 10,
        11, 10, 11, 10, 12, 9, 11, 11, 11, 9, 11, 10, 10, 9, 11, 10, 10, 9, 10, 9, 7, 4, 5, 5, 5,
        6, 6, 7, 6, 8, 7, 8, 9, 9, 7, 8, 8, 10, 9, 10, 10, 12, 10, 11, 11, 11, 11, 10, 11, 12, 11,
        11, 11, 11, 11, 13, 12, 11, 12, 13, 12, 12, 12, 13, 11, 9, 12, 13, 7, 13, 11, 13, 11, 10,
        11, 13, 15, 15, 12, 14, 15, 15, 15, 6, 15, 5,
    ],
    &[
        8, 10, 11, 11, 11, 12, 11, 11, 12, 6, 11, 12, 10, 5, 12, 12, 12, 12, 12, 12, 12, 13, 13,
        14, 13, 13, 12, 13, 12, 13, 12, 15, 4, 10, 7, 9, 11, 11, 10, 9, 6, 7, 8, 9, 6, 7, 6, 7, 8,
        7, 7, 8, 8, 8, 8, 8, 8, 9, 8, 7, 10, 9, 10, 10, 11, 7, 8, 6, 7, 8, 8, 9, 8, 7, 10, 10, 8,
        7, 8, 8, 7, 10, 7, 6, 7, 9, 9, 8, 11, 11, 11, 10, 11, 11, 11, 8, 11, 6, 7, 6, 6, 6, 6, 8,
        7, 6, 10, 9, 6, 7, 6, 6, 7, 10, 6, 5, 6, 7, 7, 7, 10, 8, 11, 9, 13, 7, 14, 16, 12, 14, 14,
        15, 15, 16, 16, 14, 15, 15, 15, 15, 15, 15, 15, 15, 14, 15, 13, 14, 14, 16, 15, 17, 14, 17,
        15, 17, 12, 14, 13, 16, 12, 17, 13, 17, 14, 13, 13, 14, 14, 12, 13, 15, 15, 14, 15, 17, 14,
        17, 15, 14, 15, 16, 12, 16, 15, 14, 15, 16, 15, 16, 17, 17, 15, 15, 17, 17, 13, 14, 15, 15,
        13, 12, 16, 16, 17, 14, 15, 16, 15, 15, 13, 13, 15, 13, 16, 17, 15, 17, 17, 17, 16, 17, 14,
        17, 14, 16, 15, 17, 15, 15, 14, 17, 15, 17, 15, 16, 15, 15, 16, 16, 14, 17, 17, 15, 15, 16,
        15, 17, 15, 14, 16, 16, 16, 16, 16, 12, 4, 4, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 8, 9, 9, 9,
        9, 9, 10, 10, 10, 11, 10, 11, 11, 11, 11, 11, 12, 12, 12, 13, 13, 12, 13, 12, 14, 14, 12,
        13, 13, 13, 13, 14, 12, 13, 13, 14, 14, 14, 13, 14, 14, 15, 15, 13, 15, 13, 17, 17, 17, 9,
        17, 7,
    ],
];
const OFFSET_CODE_LENGTHS: [&[i32]; 5] = [
    &[5, 6, 3, 3, 3, 3, 3, 3, 3, 4, 6],
    &[5, 6, 4, 4, 3, 3, 3, 3, 3, 4, 4, 4, 6],
    &[6, 7, 4, 4, 3, 3, 3, 3, 3, 4, 4, 4, 5, 7],
    &[3, 6, 5, 4, 2, 3, 3, 3, 4, 4, 6],
    &[6, 7, 7, 6, 4, 3, 2, 2, 3, 3, 6],
];
