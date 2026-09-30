//! A byte-exact reimplementation of zlib 1.2.x's `compress2(level=9)`.
//!
//! Why not miniz_oxide / zlib-rs: the parity gate is byte-for-byte against
//! PNGs the Python ripper wrote with `zlib.compress(raw, 9)` (the system
//! zlib), and every other deflate implementation picks different matches
//! and block splits. This follows zlib's own deflate.c / trees.c for the one
//! configuration Python uses -- windowBits 15, memLevel 8, default strategy,
//! level 9 (`deflate_slow`, good 32 / lazy 258 / nice 258 / chain 4096),
//! all input supplied at once with Z_FINISH -- including the parts that are
//! observable in the output: the rolling hash and its insertion quirks, the
//! window slide (which leaves stale bytes that `longest_match` may read past
//! the end of input), TOO_FAR, the 16383-symbol block limit, the
//! stored/static/dynamic block choice, and the Huffman tree builder with its
//! depth tie-breaks, bit-length overflow repair and code-length RLE.
//!
//! zlib is (C) 1995-2022 Jean-loup Gailly and Mark Adler, under the zlib
//! license; this is an independent Rust rendering of its algorithm.

// Kept shaped like the C it mirrors, so it can be checked line by line.
#![allow(clippy::needless_range_loop, clippy::implicit_saturating_sub)]

const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const W_BITS: usize = 15;
const W_SIZE: usize = 1 << W_BITS;
const W_MASK: usize = W_SIZE - 1;
const WINDOW_SIZE: usize = 2 * W_SIZE;
const MEM_LEVEL: usize = 8;
const HASH_BITS: usize = MEM_LEVEL + 7;
const HASH_SIZE: usize = 1 << HASH_BITS;
const HASH_MASK: usize = HASH_SIZE - 1;
const HASH_SHIFT: usize = HASH_BITS.div_ceil(MIN_MATCH);
const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_MATCH + 1;
const MAX_DIST: usize = W_SIZE - MIN_LOOKAHEAD;
const WIN_INIT: usize = MAX_MATCH;
const TOO_FAR: usize = 4096;
const LIT_BUFSIZE: usize = 1 << (MEM_LEVEL + 6);
const SYM_END: usize = (LIT_BUFSIZE - 1) * 3;

// level 9 configuration
const GOOD_MATCH: usize = 32;
const MAX_LAZY: usize = 258;
const NICE_MATCH: usize = 258;
const MAX_CHAIN: usize = 4096;

const LENGTH_CODES: usize = 29;
const LITERALS: usize = 256;
const L_CODES: usize = LITERALS + 1 + LENGTH_CODES;
const D_CODES: usize = 30;
const BL_CODES: usize = 19;
const HEAP_SIZE: usize = 2 * L_CODES + 1;
const MAX_BITS: usize = 15;
const MAX_BL_BITS: usize = 7;
const END_BLOCK: usize = 256;
const REP_3_6: usize = 16;
const REPZ_3_10: usize = 17;
const REPZ_11_138: usize = 18;

const EXTRA_LBITS: [u8; LENGTH_CODES] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const EXTRA_DBITS: [u8; D_CODES] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const EXTRA_BLBITS: [u8; BL_CODES] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3, 7];
const BL_ORDER: [usize; BL_CODES] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

struct Static {
    length_code: [u8; 256],
    dist_code: [u8; 512],
    base_length: [u16; LENGTH_CODES],
    base_dist: [u16; D_CODES],
    ltree_code: [u16; L_CODES + 2],
    ltree_len: [u16; L_CODES + 2],
    dtree_code: [u16; D_CODES],
    dtree_len: [u16; D_CODES],
}

fn bi_reverse(mut code: u32, mut len: usize) -> u32 {
    let mut res = 0u32;
    loop {
        res |= code & 1;
        code >>= 1;
        res <<= 1;
        len -= 1;
        if len == 0 {
            break;
        }
    }
    res >> 1
}

/// trees.c gen_codes over separate code/len arrays.
fn gen_codes(code: &mut [u16], len: &[u16], max_code: usize, bl_count: &[u16; MAX_BITS + 1]) {
    let mut next_code = [0u16; MAX_BITS + 1];
    let mut c: u32 = 0;
    for bits in 1..=MAX_BITS {
        c = (c + bl_count[bits - 1] as u32) << 1;
        next_code[bits] = c as u16;
    }
    for n in 0..=max_code {
        let l = len[n] as usize;
        if l == 0 {
            continue;
        }
        code[n] = bi_reverse(next_code[l] as u32, l) as u16;
        next_code[l] = next_code[l].wrapping_add(1);
    }
}

fn static_tables() -> &'static Static {
    use std::sync::OnceLock;
    static S: OnceLock<Static> = OnceLock::new();
    S.get_or_init(|| {
        let mut s = Static {
            length_code: [0; 256],
            dist_code: [0; 512],
            base_length: [0; LENGTH_CODES],
            base_dist: [0; D_CODES],
            ltree_code: [0; L_CODES + 2],
            ltree_len: [0; L_CODES + 2],
            dtree_code: [0; D_CODES],
            dtree_len: [0; D_CODES],
        };
        let mut length = 0usize;
        let mut code = 0usize;
        while code < LENGTH_CODES - 1 {
            s.base_length[code] = length as u16;
            for _ in 0..(1usize << EXTRA_LBITS[code]) {
                s.length_code[length] = code as u8;
                length += 1;
            }
            code += 1;
        }
        s.length_code[length - 1] = code as u8;
        let mut dist = 0usize;
        code = 0;
        while code < 16 {
            s.base_dist[code] = dist as u16;
            for _ in 0..(1usize << EXTRA_DBITS[code]) {
                s.dist_code[dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }
        dist >>= 7;
        while code < D_CODES {
            s.base_dist[code] = (dist << 7) as u16;
            for _ in 0..(1usize << (EXTRA_DBITS[code] - 7)) {
                s.dist_code[256 + dist] = code as u8;
                dist += 1;
            }
            code += 1;
        }
        let mut bl_count = [0u16; MAX_BITS + 1];
        for n in 0..=143 {
            s.ltree_len[n] = 8;
        }
        for n in 144..=255 {
            s.ltree_len[n] = 9;
        }
        for n in 256..=279 {
            s.ltree_len[n] = 7;
        }
        for n in 280..=287 {
            s.ltree_len[n] = 8;
        }
        bl_count[7] = 24;
        bl_count[8] = 144 + 8;
        bl_count[9] = 112;
        let lens = s.ltree_len;
        gen_codes(&mut s.ltree_code, &lens, L_CODES + 1, &bl_count);
        for n in 0..D_CODES {
            s.dtree_len[n] = 5;
            s.dtree_code[n] = bi_reverse(n as u32, 5) as u16;
        }
        s
    })
}

#[inline]
fn d_code(st: &Static, dist: usize) -> usize {
    if dist < 256 {
        st.dist_code[dist] as usize
    } else {
        st.dist_code[256 + (dist >> 7)] as usize
    }
}

/// A dynamic tree: zlib's ct_data {fc: Freq|Code, dl: Dad|Len} unions kept
/// as two shared arrays, because the builder relies on the aliasing (Len
/// overwrites Dad, Code overwrites Freq).
struct Tree {
    fc: Vec<u16>,
    dl: Vec<u16>,
    max_code: usize,
}

impl Tree {
    fn new(n: usize) -> Self {
        Tree {
            fc: vec![0; n],
            dl: vec![0; n],
            max_code: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum Kind {
    L,
    D,
    Bl,
}

struct BitOut {
    out: Vec<u8>,
    buf: u64,
    nbits: u32,
}

impl BitOut {
    #[inline]
    fn send(&mut self, value: u32, len: usize) {
        self.buf |= (value as u64) << self.nbits;
        self.nbits += len as u32;
        while self.nbits >= 8 {
            self.out.push(self.buf as u8);
            self.buf >>= 8;
            self.nbits -= 8;
        }
    }
    fn windup(&mut self) {
        if self.nbits > 0 {
            self.out.push(self.buf as u8);
        }
        self.buf = 0;
        self.nbits = 0;
    }
}

struct Deflate<'a> {
    input: &'a [u8],
    next_in: usize,
    window: Vec<u8>,
    prev: Vec<u16>,
    head: Vec<u16>,
    ins_h: usize,
    strstart: usize,
    block_start: isize,
    lookahead: usize,
    insert: usize,
    match_start: usize,
    match_length: usize,
    prev_match: usize,
    prev_length: usize,
    match_available: bool,
    high_water: usize,
    // trees
    lt: Tree,
    dt: Tree,
    bt: Tree,
    bl_count: [u16; MAX_BITS + 1],
    heap: [usize; 2 * L_CODES + 1],
    heap_len: usize,
    heap_max: usize,
    depth: [u8; 2 * L_CODES + 1],
    sym_buf: Vec<u8>,
    opt_len: u64,
    static_len: u64,
    bits: BitOut,
}

impl<'a> Deflate<'a> {
    fn new(input: &'a [u8]) -> Self {
        Deflate {
            input,
            next_in: 0,
            window: vec![0; WINDOW_SIZE],
            prev: vec![0; W_SIZE],
            head: vec![0; HASH_SIZE],
            ins_h: 0,
            strstart: 0,
            block_start: 0,
            lookahead: 0,
            insert: 0,
            match_start: 0,
            match_length: MIN_MATCH - 1,
            prev_match: 0,
            prev_length: MIN_MATCH - 1,
            match_available: false,
            high_water: 0,
            lt: Tree::new(HEAP_SIZE),
            dt: Tree::new(2 * D_CODES + 1),
            bt: Tree::new(2 * BL_CODES + 1),
            bl_count: [0; MAX_BITS + 1],
            heap: [0; 2 * L_CODES + 1],
            heap_len: 0,
            heap_max: 0,
            depth: [0; 2 * L_CODES + 1],
            sym_buf: Vec::with_capacity(SYM_END + 3),
            opt_len: 0,
            static_len: 0,
            bits: BitOut {
                out: Vec::new(),
                buf: 0,
                nbits: 0,
            },
        }
    }

    #[inline]
    fn update_hash(h: usize, c: u8) -> usize {
        ((h << HASH_SHIFT) ^ c as usize) & HASH_MASK
    }

    #[inline]
    fn insert_string(&mut self, s: usize) -> usize {
        self.ins_h = Self::update_hash(self.ins_h, self.window[s + MIN_MATCH - 1]);
        let head = self.head[self.ins_h] as usize;
        self.prev[s & W_MASK] = head as u16;
        self.head[self.ins_h] = s as u16;
        head
    }

    fn slide_hash(&mut self) {
        for h in self.head.iter_mut() {
            let m = *h as usize;
            *h = if m >= W_SIZE { (m - W_SIZE) as u16 } else { 0 };
        }
        for p in self.prev.iter_mut() {
            let m = *p as usize;
            *p = if m >= W_SIZE { (m - W_SIZE) as u16 } else { 0 };
        }
    }

    fn fill_window(&mut self) {
        loop {
            let mut more = WINDOW_SIZE - self.lookahead - self.strstart;
            if self.strstart >= W_SIZE + MAX_DIST {
                let n = W_SIZE - more;
                self.window.copy_within(W_SIZE..W_SIZE + n, 0);
                self.match_start = self.match_start.wrapping_sub(W_SIZE);
                self.strstart -= W_SIZE;
                self.block_start -= W_SIZE as isize;
                if self.insert > self.strstart {
                    self.insert = self.strstart;
                }
                self.slide_hash();
                more += W_SIZE;
            }
            let avail = self.input.len() - self.next_in;
            if avail == 0 {
                break;
            }
            let n = avail.min(more);
            let dst = self.strstart + self.lookahead;
            self.window[dst..dst + n].copy_from_slice(&self.input[self.next_in..self.next_in + n]);
            self.next_in += n;
            self.lookahead += n;

            if self.lookahead + self.insert >= MIN_MATCH {
                let mut s = self.strstart - self.insert;
                self.ins_h = self.window[s] as usize;
                self.ins_h = Self::update_hash(self.ins_h, self.window[s + 1]);
                while self.insert > 0 {
                    self.ins_h = Self::update_hash(self.ins_h, self.window[s + MIN_MATCH - 1]);
                    self.prev[s & W_MASK] = self.head[self.ins_h];
                    self.head[self.ins_h] = s as u16;
                    s += 1;
                    self.insert -= 1;
                    if self.lookahead + self.insert < MIN_MATCH {
                        break;
                    }
                }
            }
            if !(self.lookahead < MIN_LOOKAHEAD && self.next_in < self.input.len()) {
                break;
            }
        }
        // WIN_INIT zeroing past the data (so longest_match reads defined bytes).
        if self.high_water < WINDOW_SIZE {
            let curr = self.strstart + self.lookahead;
            if self.high_water < curr {
                let init = (WINDOW_SIZE - curr).min(WIN_INIT);
                self.window[curr..curr + init].fill(0);
                self.high_water = curr + init;
            } else if self.high_water < curr + WIN_INIT {
                let init = (curr + WIN_INIT - self.high_water).min(WINDOW_SIZE - self.high_water);
                let hw = self.high_water;
                self.window[hw..hw + init].fill(0);
                self.high_water += init;
            }
        }
    }

    fn longest_match(&mut self, mut cur_match: usize) -> usize {
        let mut chain_length = MAX_CHAIN;
        let scan = self.strstart;
        let mut best_len = self.prev_length;
        let mut nice_match = NICE_MATCH;
        let limit = if self.strstart > MAX_DIST {
            self.strstart - MAX_DIST
        } else {
            0
        };
        let w = &self.window;
        let mut scan_end1 = w[scan + best_len - 1];
        let mut scan_end = w[scan + best_len];
        if self.prev_length >= GOOD_MATCH {
            chain_length >>= 2;
        }
        if nice_match > self.lookahead {
            nice_match = self.lookahead;
        }
        loop {
            let m = cur_match;
            let skip = w[m + best_len] != scan_end
                || w[m + best_len - 1] != scan_end1
                || w[m] != w[scan]
                || w[m + 1] != w[scan + 1];
            if !skip {
                // scan[2] / match[2] are not compared (equal by the hash).
                let mut len = 3;
                while len < MAX_MATCH && w[scan + len] == w[m + len] {
                    len += 1;
                }
                if len > best_len {
                    self.match_start = cur_match;
                    best_len = len;
                    if len >= nice_match {
                        break;
                    }
                    scan_end1 = w[scan + best_len - 1];
                    scan_end = w[scan + best_len];
                }
            }
            cur_match = self.prev[cur_match & W_MASK] as usize;
            if cur_match <= limit {
                break;
            }
            chain_length -= 1;
            if chain_length == 0 {
                break;
            }
        }
        if best_len <= self.lookahead {
            best_len
        } else {
            self.lookahead
        }
    }

    // ---------------- trees.c ----------------

    fn init_block(&mut self) {
        for n in 0..L_CODES {
            self.lt.fc[n] = 0;
        }
        for n in 0..D_CODES {
            self.dt.fc[n] = 0;
        }
        for n in 0..BL_CODES {
            self.bt.fc[n] = 0;
        }
        self.lt.fc[END_BLOCK] = 1;
        self.opt_len = 0;
        self.static_len = 0;
        self.sym_buf.clear();
    }

    #[inline]
    fn tally_lit(&mut self, c: u8) -> bool {
        self.sym_buf.extend_from_slice(&[0, 0, c]);
        self.lt.fc[c as usize] += 1;
        self.sym_buf.len() == SYM_END
    }

    #[inline]
    fn tally_dist(&mut self, dist: usize, len: usize) -> bool {
        let st = static_tables();
        self.sym_buf
            .extend_from_slice(&[dist as u8, (dist >> 8) as u8, len as u8]);
        let d = dist - 1;
        self.lt.fc[st.length_code[len] as usize + LITERALS + 1] += 1;
        self.dt.fc[d_code(st, d)] += 1;
        self.sym_buf.len() == SYM_END
    }

    fn tree(&mut self, k: Kind) -> &mut Tree {
        match k {
            Kind::L => &mut self.lt,
            Kind::D => &mut self.dt,
            Kind::Bl => &mut self.bt,
        }
    }

    #[inline]
    fn smaller(tree: &Tree, depth: &[u8], n: usize, m: usize) -> bool {
        tree.fc[n] < tree.fc[m] || (tree.fc[n] == tree.fc[m] && depth[n] <= depth[m])
    }

    fn pqdownheap(&mut self, k_kind: Kind, mut k: usize) {
        let tree = match k_kind {
            Kind::L => &self.lt,
            Kind::D => &self.dt,
            Kind::Bl => &self.bt,
        };
        let v = self.heap[k];
        let mut j = k << 1;
        while j <= self.heap_len {
            if j < self.heap_len && Self::smaller(tree, &self.depth, self.heap[j + 1], self.heap[j])
            {
                j += 1;
            }
            if Self::smaller(tree, &self.depth, v, self.heap[j]) {
                break;
            }
            self.heap[k] = self.heap[j];
            k = j;
            j <<= 1;
        }
        self.heap[k] = v;
    }

    fn gen_bitlen(&mut self, kind: Kind) {
        let st = static_tables();
        let (extra, base, max_length, stree): (&[u8], usize, usize, Option<&[u16]>) = match kind {
            Kind::L => (
                &EXTRA_LBITS,
                LITERALS + 1,
                MAX_BITS,
                Some(&st.ltree_len[..]),
            ),
            Kind::D => (&EXTRA_DBITS, 0, MAX_BITS, Some(&st.dtree_len[..])),
            Kind::Bl => (&EXTRA_BLBITS, 0, MAX_BL_BITS, None),
        };
        let heap = self.heap;
        let heap_max = self.heap_max;
        let mut bl_count = [0u16; MAX_BITS + 1];
        let mut opt_len = self.opt_len;
        let mut static_len = self.static_len;
        let tree = self.tree(kind);
        let max_code = tree.max_code;
        let mut overflow: i32 = 0;
        tree.dl[heap[heap_max]] = 0;
        let mut h = heap_max + 1;
        while h < HEAP_SIZE {
            let n = heap[h];
            let mut bits = tree.dl[tree.dl[n] as usize] as usize + 1;
            if bits > max_length {
                bits = max_length;
                overflow += 1;
            }
            tree.dl[n] = bits as u16;
            h += 1;
            if n > max_code {
                continue;
            }
            bl_count[bits] += 1;
            let xbits = if n >= base {
                extra[n - base] as usize
            } else {
                0
            };
            let f = tree.fc[n] as u64;
            opt_len = opt_len.wrapping_add(f * (bits + xbits) as u64);
            if let Some(s) = stree {
                static_len = static_len.wrapping_add(f * (s[n] as usize + xbits) as u64);
            }
        }
        if overflow != 0 {
            loop {
                let mut bits = max_length - 1;
                while bl_count[bits] == 0 {
                    bits -= 1;
                }
                bl_count[bits] -= 1;
                bl_count[bits + 1] += 2;
                bl_count[max_length] -= 1;
                overflow -= 2;
                if overflow <= 0 {
                    break;
                }
            }
            let mut h = HEAP_SIZE;
            let mut bits = max_length;
            while bits != 0 {
                let mut n = bl_count[bits];
                while n != 0 {
                    h -= 1;
                    let m = heap[h];
                    if m > max_code {
                        continue;
                    }
                    if tree.dl[m] as usize != bits {
                        opt_len = opt_len.wrapping_add(
                            (bits as u64)
                                .wrapping_sub(tree.dl[m] as u64)
                                .wrapping_mul(tree.fc[m] as u64),
                        );
                        tree.dl[m] = bits as u16;
                    }
                    n -= 1;
                }
                bits -= 1;
            }
        }
        self.bl_count = bl_count;
        self.opt_len = opt_len;
        self.static_len = static_len;
    }

    fn build_tree(&mut self, kind: Kind) {
        let st = static_tables();
        let (elems, stree): (usize, Option<&[u16]>) = match kind {
            Kind::L => (L_CODES, Some(&st.ltree_len[..])),
            Kind::D => (D_CODES, Some(&st.dtree_len[..])),
            Kind::Bl => (BL_CODES, None),
        };
        let mut max_code: isize = -1;
        self.heap_len = 0;
        self.heap_max = HEAP_SIZE;
        for n in 0..elems {
            let freq = self.tree(kind).fc[n];
            if freq != 0 {
                self.heap_len += 1;
                self.heap[self.heap_len] = n;
                max_code = n as isize;
                self.depth[n] = 0;
            } else {
                self.tree(kind).dl[n] = 0;
            }
        }
        while self.heap_len < 2 {
            let node = if max_code < 2 {
                max_code += 1;
                max_code as usize
            } else {
                0
            };
            self.heap_len += 1;
            self.heap[self.heap_len] = node;
            self.tree(kind).fc[node] = 1;
            self.depth[node] = 0;
            self.opt_len = self.opt_len.wrapping_sub(1);
            if let Some(s) = stree {
                self.static_len = self.static_len.wrapping_sub(s[node] as u64);
            }
        }
        self.tree(kind).max_code = max_code as usize;
        let mut n = self.heap_len / 2;
        while n >= 1 {
            self.pqdownheap(kind, n);
            n -= 1;
        }
        let mut node = elems;
        loop {
            // pqremove
            let n = self.heap[1];
            self.heap[1] = self.heap[self.heap_len];
            self.heap_len -= 1;
            self.pqdownheap(kind, 1);
            let m = self.heap[1];
            self.heap_max -= 1;
            self.heap[self.heap_max] = n;
            self.heap_max -= 1;
            self.heap[self.heap_max] = m;
            let depth_n = self.depth[n];
            let depth_m = self.depth[m];
            {
                let tree = self.tree(kind);
                tree.fc[node] = tree.fc[n].wrapping_add(tree.fc[m]);
                tree.dl[n] = node as u16;
                tree.dl[m] = node as u16;
            }
            self.depth[node] = (if depth_n >= depth_m { depth_n } else { depth_m }) + 1;
            self.heap[1] = node;
            node += 1;
            self.pqdownheap(kind, 1);
            if self.heap_len < 2 {
                break;
            }
        }
        self.heap_max -= 1;
        self.heap[self.heap_max] = self.heap[1];
        self.gen_bitlen(kind);
        let bl_count = self.bl_count;
        let tree = self.tree(kind);
        let mc = tree.max_code;
        let (fc, dl) = (&mut tree.fc, &tree.dl);
        gen_codes(fc, dl, mc, &bl_count);
    }

    fn scan_tree(&mut self, kind: Kind) {
        let max_code = self.tree(kind).max_code;
        let mut prevlen: isize = -1;
        let mut nextlen = self.tree(kind).dl[0] as isize;
        let mut count = 0usize;
        let (mut max_count, mut min_count) = if nextlen == 0 { (138, 3) } else { (7, 4) };
        self.tree(kind).dl[max_code + 1] = 0xFFFF;
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = self.tree(kind).dl[n + 1] as isize;
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                self.bt.fc[curlen as usize] += count as u16;
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.bt.fc[curlen as usize] += 1;
                }
                self.bt.fc[REP_3_6] += 1;
            } else if count <= 10 {
                self.bt.fc[REPZ_3_10] += 1;
            } else {
                self.bt.fc[REPZ_11_138] += 1;
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
    }

    #[inline]
    fn send_bl(&mut self, c: usize) {
        let (code, len) = (self.bt.fc[c], self.bt.dl[c]);
        self.bits.send(code as u32, len as usize);
    }

    fn send_tree(&mut self, kind: Kind, max_code: usize) {
        let mut prevlen: isize = -1;
        let mut nextlen = self.tree(kind).dl[0] as isize;
        let mut count = 0usize;
        let (mut max_count, mut min_count) = if nextlen == 0 { (138, 3) } else { (7, 4) };
        for n in 0..=max_code {
            let curlen = nextlen;
            nextlen = self.tree(kind).dl[n + 1] as isize;
            count += 1;
            if count < max_count && curlen == nextlen {
                continue;
            } else if count < min_count {
                loop {
                    self.send_bl(curlen as usize);
                    count -= 1;
                    if count == 0 {
                        break;
                    }
                }
            } else if curlen != 0 {
                if curlen != prevlen {
                    self.send_bl(curlen as usize);
                    count -= 1;
                }
                self.send_bl(REP_3_6);
                self.bits.send((count - 3) as u32, 2);
            } else if count <= 10 {
                self.send_bl(REPZ_3_10);
                self.bits.send((count - 3) as u32, 3);
            } else {
                self.send_bl(REPZ_11_138);
                self.bits.send((count - 11) as u32, 7);
            }
            count = 0;
            prevlen = curlen;
            if nextlen == 0 {
                max_count = 138;
                min_count = 3;
            } else if curlen == nextlen {
                max_count = 6;
                min_count = 3;
            } else {
                max_count = 7;
                min_count = 4;
            }
        }
    }

    fn build_bl_tree(&mut self) -> usize {
        self.scan_tree(Kind::L);
        self.scan_tree(Kind::D);
        self.build_tree(Kind::Bl);
        let mut max_blindex = BL_CODES - 1;
        while max_blindex >= 3 {
            if self.bt.dl[BL_ORDER[max_blindex]] != 0 {
                break;
            }
            max_blindex -= 1;
        }
        self.opt_len = self
            .opt_len
            .wrapping_add(3 * (max_blindex as u64 + 1) + 5 + 5 + 4);
        max_blindex
    }

    fn send_all_trees(&mut self, lcodes: usize, dcodes: usize, blcodes: usize) {
        self.bits.send((lcodes - 257) as u32, 5);
        self.bits.send((dcodes - 1) as u32, 5);
        self.bits.send((blcodes - 4) as u32, 4);
        for rank in 0..blcodes {
            let l = self.bt.dl[BL_ORDER[rank]];
            self.bits.send(l as u32, 3);
        }
        self.send_tree(Kind::L, lcodes - 1);
        self.send_tree(Kind::D, dcodes - 1);
    }

    fn compress_block(&mut self, dynamic: bool) {
        let st = static_tables();
        let syms = std::mem::take(&mut self.sym_buf);
        let (lcode, llen, dcode, dlen): (&[u16], &[u16], &[u16], &[u16]) = if dynamic {
            (&self.lt.fc, &self.lt.dl, &self.dt.fc, &self.dt.dl)
        } else {
            (&st.ltree_code, &st.ltree_len, &st.dtree_code, &st.dtree_len)
        };
        let bits = &mut self.bits;
        for s in syms.chunks_exact(3) {
            let mut dist = s[0] as usize | ((s[1] as usize) << 8);
            let lc = s[2] as usize;
            if dist == 0 {
                bits.send(lcode[lc] as u32, llen[lc] as usize);
            } else {
                let code = st.length_code[lc] as usize;
                bits.send(
                    lcode[code + LITERALS + 1] as u32,
                    llen[code + LITERALS + 1] as usize,
                );
                let extra = EXTRA_LBITS[code] as usize;
                if extra != 0 {
                    bits.send((lc - st.base_length[code] as usize) as u32, extra);
                }
                dist -= 1;
                let code = d_code(st, dist);
                bits.send(dcode[code] as u32, dlen[code] as usize);
                let extra = EXTRA_DBITS[code] as usize;
                if extra != 0 {
                    bits.send((dist - st.base_dist[code] as usize) as u32, extra);
                }
            }
        }
        bits.send(lcode[END_BLOCK] as u32, llen[END_BLOCK] as usize);
        self.sym_buf = syms;
    }

    fn flush_block(&mut self, last: bool) {
        let stored_len = (self.strstart as isize - self.block_start) as usize;
        let buf_ok = self.block_start >= 0;
        self.build_tree(Kind::L);
        self.build_tree(Kind::D);
        let max_blindex = self.build_bl_tree();
        let mut opt_lenb = self.opt_len.wrapping_add(3 + 7) >> 3;
        let static_lenb = self.static_len.wrapping_add(3 + 7) >> 3;
        if static_lenb <= opt_lenb {
            opt_lenb = static_lenb;
        }
        let last_bit = last as u32;
        if stored_len as u64 + 4 <= opt_lenb && buf_ok {
            // stored block
            self.bits.send(last_bit, 3);
            self.bits.windup();
            let bs = self.block_start as usize;
            let out = &mut self.bits.out;
            out.extend_from_slice(&(stored_len as u16).to_le_bytes());
            out.extend_from_slice(&(!(stored_len as u16)).to_le_bytes());
            out.extend_from_slice(&self.window[bs..bs + stored_len]);
        } else if static_lenb == opt_lenb {
            self.bits.send((1 << 1) + last_bit, 3);
            self.compress_block(false);
        } else {
            self.bits.send((2 << 1) + last_bit, 3);
            let (l, d) = (self.lt.max_code + 1, self.dt.max_code + 1);
            self.send_all_trees(l, d, max_blindex + 1);
            self.compress_block(true);
        }
        self.init_block();
        if last {
            self.bits.windup();
        }
        self.block_start = self.strstart as isize;
    }

    fn run(mut self) -> Vec<u8> {
        self.init_block();
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window();
                if self.lookahead == 0 {
                    break;
                }
            }
            let mut hash_head = 0usize;
            if self.lookahead >= MIN_MATCH {
                hash_head = self.insert_string(self.strstart);
            }
            self.prev_length = self.match_length;
            self.prev_match = self.match_start;
            self.match_length = MIN_MATCH - 1;
            if hash_head != 0
                && self.prev_length < MAX_LAZY
                && self.strstart - hash_head <= MAX_DIST
            {
                self.match_length = self.longest_match(hash_head);
                if self.match_length <= 5
                    && self.match_length == MIN_MATCH
                    && self.strstart - self.match_start > TOO_FAR
                {
                    self.match_length = MIN_MATCH - 1;
                }
            }
            if self.prev_length >= MIN_MATCH && self.match_length <= self.prev_length {
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                let bflush = self.tally_dist(
                    self.strstart - 1 - self.prev_match,
                    self.prev_length - MIN_MATCH,
                );
                self.lookahead -= self.prev_length - 1;
                self.prev_length -= 2;
                loop {
                    self.strstart += 1;
                    if self.strstart <= max_insert {
                        self.insert_string(self.strstart);
                    }
                    self.prev_length -= 1;
                    if self.prev_length == 0 {
                        break;
                    }
                }
                self.match_available = false;
                self.match_length = MIN_MATCH - 1;
                self.strstart += 1;
                if bflush {
                    self.flush_block(false);
                }
            } else if self.match_available {
                let c = self.window[self.strstart - 1];
                if self.tally_lit(c) {
                    self.flush_block(false);
                }
                self.strstart += 1;
                self.lookahead -= 1;
            } else {
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            let c = self.window[self.strstart - 1];
            self.tally_lit(c);
            self.match_available = false;
        }
        self.insert = self.strstart.min(MIN_MATCH - 1);
        self.flush_block(true);
        self.bits.out
    }
}

pub fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    (b << 16) | a
}

/// `zlib.compress(data, 9)`: zlib header, raw deflate, Adler-32.
pub fn zlib_compress9(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0xDA];
    out.extend(Deflate::new(data).run());
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// Reference outputs from CPython `zlib.compress(data, 9)` (zlib 1.2.12).
    #[test]
    fn matches_zlib_level9() {
        assert_eq!(hex(&zlib_compress9(b"")), "78da030000000001");
        assert_eq!(
            hex(&zlib_compress9(b"hello hello hello hello")),
            "78dacb48cdc9c957c8402701680308b1"
        );
        let rgba: Vec<u8> = (0..300)
            .flat_map(|_| [0, 0, 0, 0, 0x10, 0x20, 0x30, 0xff])
            .collect();
        assert_eq!(
            hex(&zlib_compress9(&rgba)),
            "78da63606060105030f8cf304a8fd2a3f4283d4a8fd2a3f4283d4a538506007d609b64"
        );
        let ramp: Vec<u8> = (0..3).flat_map(|_| 0..=255u8).collect();
        assert!(hex(&zlib_compress9(&ramp)).ends_with("ff88f63f00a0627e90"));
    }

    #[test]
    fn adler() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }
}
