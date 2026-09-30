//! Synthetic-input tests: no Berkeley data needed, always run.

use twistedrip::container::{binhex, sit};
use twistedrip::decode::{pens, rlep};
use twistedrip::json::dumps_indent1;
use twistedrip::{cmid, rsrc, snd};

#[test]
fn rlep_opcodes() {
    // skip 3, one c16[1]; end row; run of c16[5] x2; copy row 0; end sprite.
    let rows = rlep::decode_rows(&[0x31, 0x13, 0x10, 0x52, 0x02, 0x10, 0x30, 0x00, 0x00, 0x00]);
    assert_eq!(rows.len(), 4);
    assert_eq!((rows[0][0].x, rows[0][0].px.clone()), (3, vec![1]));
    assert_eq!((rows[1][0].x, rows[1][0].px.clone()), (0, vec![5, 5]));
    assert_eq!(rows[2][0].px, vec![1]);
    assert!(rows[3].is_empty());
    // Truncated mid-opcode: the run in progress is dropped, like IndexError.
    assert!(rlep::decode_rows(&[0x52]).is_empty());
    // A kept partial row when the stream runs out after a complete run.
    let r = rlep::decode_rows(&[0x13, 0x52]);
    assert_eq!(r.len(), 1);
    // Render: out-of-table index -> magenta; rect widens the image.
    let ctab = vec![[0, 0, 0], [10, 20, 30]];
    let img = rlep::render(
        &rlep::decode_rows(&[0x13, 0x73, 0x00]),
        &ctab,
        Some((0, 0, 2, 6)),
    )
    .unwrap();
    assert_eq!((img.w, img.h), (6, 2));
    assert_eq!(&img.rgba[0..8], &[10, 20, 30, 255, 255, 0, 255, 255]);
}

#[test]
fn cmid_lzss_literals_and_backref() {
    let mut s = vec![0xFF];
    s.extend_from_slice(b"abcdefgh");
    s.extend_from_slice(&[0x00, 0x0F, 0xF8]);
    assert_eq!(cmid::lzss_decompress(&s).unwrap(), b"abcdefghabc");
    let mut with_len = 11u32.to_be_bytes().to_vec();
    with_len.extend_from_slice(&s);
    assert_eq!(cmid::decode(&with_len).unwrap(), b"abcdefghabc");
    assert!(cmid::lzss_decompress(&[0x00, 0x00, 0x10]).is_err()); // underflow
}

#[test]
fn snds_expander_first_order() {
    // Format-2 snd: one bufferCmd pointing at a standard header at 14.
    let mut s = vec![0, 2, 0, 0, 0, 1, 0x80, 0x51, 0, 0, 0, 0, 0, 14];
    s.extend_from_slice(&0u32.to_be_bytes()); // ptr
    s.extend_from_slice(&3u32.to_be_bytes()); // len (header byte + 2 data)
    s.extend_from_slice(&0x56EE_8BA3u32.to_be_bytes()); // 22254.54 Hz
    s.extend_from_slice(&[0; 8]);
    s.extend_from_slice(&[0, 60]);
    s.extend_from_slice(&[0xE1, 0x01, 0xFF]); // 8-bit deltas: +1, -1
    let snds = [0, 1, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0];
    let wav = snd::to_wav(&s, Some(&snds)).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 22255);
    assert_eq!(&wav[44..], &[0x81, 0x80]);
}

#[test]
fn pens_sign_quirk_and_json() {
    // A synthetic font (0x8E is Mac Roman e-acute, escaped by ensure_ascii).
    let doc = pens::decode(b"A:[1,2,-5,3]{-7,8,1,1,1,1,2,2}<50>\x8e:<1>\0").unwrap();
    let s = dumps_indent1(&doc);
    assert!(s.contains("\"raw_neg_bug\": true"));
    assert!(
        s.contains("\"points\": [\n      [\n       1,\n       2\n      ],\n      [\n       -25,")
    );
    assert!(s.contains("\"\\u00e9\": {"));
    // Length + FNV-1a of CPython's json.dumps(pens.decode(...), indent=1).
    let h = s.bytes().fold(0xcbf29ce484222325u64, |x, b| {
        (x ^ b as u64).wrapping_mul(0x100000001b3)
    });
    assert_eq!((s.len(), h), (3005, 0xb1fe4c6b504e2457));
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

#[test]
fn binhex_roundtrip() {
    let (data, rsrc) = (b"data fork".to_vec(), b"resource fork bytes".to_vec());
    let mut raw = vec![4];
    raw.extend_from_slice(b"Test\0TEXTttxt\0\0");
    raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
    raw.extend_from_slice(&(rsrc.len() as u32).to_be_bytes());
    let c = crc16(&raw);
    raw.extend_from_slice(&c.to_be_bytes());
    for fork in [&data, &rsrc] {
        raw.extend_from_slice(fork);
        raw.extend_from_slice(&crc16(fork).to_be_bytes());
    }
    let alphabet = b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr";
    let mut text = b"(This file must be converted with BinHex 4.0)\r:".to_vec();
    let mut bits = 0u32;
    let mut n = 0;
    for (i, &b) in raw.iter().enumerate() {
        bits = (bits << 8) | b as u32;
        n += 8;
        while n >= 6 {
            n -= 6;
            text.push(alphabet[((bits >> n) & 63) as usize]);
        }
        if i % 40 == 39 {
            text.push(b'\r');
        }
    }
    if n > 0 {
        text.push(alphabet[((bits << (6 - n)) & 63) as usize]);
    }
    text.push(b':');
    let f = binhex::decode(&text).unwrap();
    assert_eq!(f.path, vec!["Test".to_string()]);
    assert_eq!(&f.file_type, b"TEXT");
    assert_eq!((f.data, f.rsrc), (data, rsrc));
}

fn tiny_fork() -> Vec<u8> {
    // Two 'TEST' resources (id 7 named "hi", id -2 unnamed) + one 'snd '.
    let datas: [&[u8]; 3] = [b"seven", b"minus two", b"s"];
    let mut data = Vec::new();
    let mut offs = Vec::new();
    for d in datas {
        offs.push(data.len() as u32);
        data.extend_from_slice(&(d.len() as u32).to_be_bytes());
        data.extend_from_slice(d);
    }
    let mut map = vec![0u8; 28];
    let tl = 28u16;
    map.extend_from_slice(&1u16.to_be_bytes()); // 2 types
    map.extend_from_slice(b"TEST");
    map.extend_from_slice(&1u16.to_be_bytes());
    map.extend_from_slice(&(2 + 16u16).to_be_bytes());
    map.extend_from_slice(b"snd ");
    map.extend_from_slice(&0u16.to_be_bytes());
    map.extend_from_slice(&(2 + 16 + 24u16).to_be_bytes());
    for (id, name_off, off) in [
        (7i16, 0u16, offs[0]),
        (-2, 0xFFFF, offs[1]),
        (128, 0xFFFF, offs[2]),
    ] {
        map.extend_from_slice(&id.to_be_bytes());
        map.extend_from_slice(&name_off.to_be_bytes());
        map.extend_from_slice(&off.to_be_bytes());
        map.extend_from_slice(&[0; 4]);
    }
    let nl = map.len() as u16;
    map.extend_from_slice(b"\x02hi");
    map[24..26].copy_from_slice(&tl.to_be_bytes());
    map[26..28].copy_from_slice(&nl.to_be_bytes());
    let mut fork = vec![0u8; 256];
    let data_off = 256u32;
    let map_off = data_off + data.len() as u32;
    fork[0..4].copy_from_slice(&data_off.to_be_bytes());
    fork[4..8].copy_from_slice(&map_off.to_be_bytes());
    fork[8..12].copy_from_slice(&(data.len() as u32).to_be_bytes());
    fork[12..16].copy_from_slice(&(map.len() as u32).to_be_bytes());
    fork.extend_from_slice(&data);
    fork.extend_from_slice(&map);
    fork
}

#[test]
fn rsrc_parse_order_names_and_sniff() {
    let fork = tiny_fork();
    let m = rsrc::parse(&fork).unwrap();
    let got: Vec<(String, i16, String, Vec<u8>)> = m
        .iter()
        .map(|r| (r.rtype.clone(), r.id, r.name.clone(), r.data.clone()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("TEST".into(), 7, "hi".into(), b"seven".to_vec()),
            ("TEST".into(), -2, "".into(), b"minus two".to_vec()),
            ("snd ".into(), 128, "".into(), b"s".to_vec()),
        ]
    );
    // 'snd ' is an asset marker, so this fork counts as module content.
    assert!(twistedrip::ingest::is_asset_fork(&fork));
    assert!(rsrc::parse(&[]).unwrap().is_empty());
}

#[test]
fn classic_sit_store_method() {
    let rsrc_fork = tiny_fork();
    let data = b"hello".to_vec();
    let mut a = b"SIT!".to_vec();
    a.extend_from_slice(&1u16.to_be_bytes());
    let total = 22 + 112 + rsrc_fork.len() + data.len();
    a.extend_from_slice(&(total as u32).to_be_bytes());
    a.extend_from_slice(b"rLau\x02");
    a.resize(22, 0);
    let mut h = vec![0u8; 112];
    h[2] = 5;
    h[3..8].copy_from_slice(b"Thing");
    h[66..70].copy_from_slice(b"ADgm");
    h[70..74].copy_from_slice(b"ADrk");
    h[84..88].copy_from_slice(&(rsrc_fork.len() as u32).to_be_bytes());
    h[88..92].copy_from_slice(&(data.len() as u32).to_be_bytes());
    h[92..96].copy_from_slice(&(rsrc_fork.len() as u32).to_be_bytes());
    h[96..100].copy_from_slice(&(data.len() as u32).to_be_bytes());
    a.extend_from_slice(&h);
    a.extend_from_slice(&rsrc_fork);
    a.extend_from_slice(&data);
    assert!(sit::is_classic(&a));
    let list = sit::list_classic(&a).unwrap();
    assert_eq!(list.len(), 1);
    let f = list[0].unpack(&a).unwrap();
    assert_eq!(f.path, vec!["Thing".to_string()]);
    assert_eq!((f.data, f.rsrc), (data, rsrc_fork));
}
