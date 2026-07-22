//! Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則)
//!
//! 新規3variant (Basic/NoDummy/Fill00 × ClosestToRawPos) を522本全件に
//! 適用し、既存 union264 (`.local_data/stage12_18/union_all.txt`) への
//! 純増を求める。
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_basic_eof_closest_to_raw_tie, compress_okumura_fill00_eof_closest_to_raw_tie,
    compress_okumura_no_dummy_eof_closest_to_raw_tie, Token,
};

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        (
            "basic_eof_closest_to_raw_tie",
            compress_okumura_basic_eof_closest_to_raw_tie,
        ),
        (
            "no_dummy_eof_closest_to_raw_tie",
            compress_okumura_no_dummy_eof_closest_to_raw_tie,
        ),
        (
            "fill00_eof_closest_to_raw_tie",
            compress_okumura_fill00_eof_closest_to_raw_tie,
        ),
    ]
}

fn frame_payload(tokens: &[Token]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let flag_pos = out.len();
        out.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    out.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let p = (pos as usize) & 0x0fff;
                    let l = ((len as usize) - 3) & 0x0f;
                    let upper = (l | ((p & 0x0f) << 4)) as u8;
                    let lower = ((p >> 4) & 0xff) as u8;
                    out.push(upper ^ 0xff);
                    out.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }
        out[flag_pos] = flag_byte ^ 0xff;
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let union_path = PathBuf::from(&args[2]);

    let union_existing: BTreeSet<String> = fs::read_to_string(&union_path)
        .unwrap()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    eprintln!("existing union: {} files", union_existing.len());

    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    let vs = variants();
    let mut net_new: Vec<(String, Vec<&'static str>)> = Vec::new();
    let mut total_hits_any = 0u64;

    for p in &paths {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap().to_string();
        let data = fs::read(p).unwrap();
        if data.len() < 0x18 || &data[0..8] != b"LEAF256\0" {
            continue;
        }
        let w = u16::from_le_bytes([data[12], data[13]]);
        let h = u16::from_le_bytes([data[14], data[15]]);
        let cc = data[0x16] as usize;
        let ps = 0x18 + cc * 3;
        let dec = match retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(
            &data[ps..],
            w,
            h,
        ) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("WARN decode {}: {}", name, e);
                continue;
            }
        };
        let original_payload = &data[ps..];
        let mut hit_variants = Vec::new();
        for (vname, f) in &vs {
            let toks = f(&dec.ring_input);
            let payload = frame_payload(&toks);
            if payload == *original_payload {
                hit_variants.push(*vname);
            }
        }
        if !hit_variants.is_empty() {
            total_hits_any += 1;
            if !union_existing.contains(&name) {
                net_new.push((name, hit_variants));
            }
        }
    }

    println!("files hit by >=1 new variant (of 522): {}", total_hits_any);
    println!("net NEW files (not already in union264): {}", net_new.len());
    for (name, vs) in &net_new {
        println!("  {} -> {:?}", name, vs);
    }
}
