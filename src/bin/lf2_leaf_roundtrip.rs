//! Leaf token を decode → re-frame で元 LF2 payload を再現できるかテスト。
//! framer の末尾 phantom padding 等の仕様を検証する。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

/// Token list → LF2 payload bytes (既存 to_lf2_bytes_okumura の framing と同じ)
fn frame_payload(tokens: &[LeafToken]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let flag_pos = out.len();
        out.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                LeafToken::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    out.push(b ^ 0xff);
                }
                LeafToken::Match { pos, len } => {
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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <dir_or_file>", args[0]);
        return ExitCode::FAILURE;
    }
    let path = PathBuf::from(&args[1]);
    let files: Vec<PathBuf> = if path.is_dir() {
        let mut v: Vec<PathBuf> = fs::read_dir(&path).unwrap()
            .filter_map(|e| e.ok()).map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2")).unwrap_or(false))
            .collect();
        v.sort();
        v
    } else {
        vec![path]
    };

    let mut n_total = 0;
    let mut n_match = 0;
    let mut n_pad_diff = 0; // 末尾 padding 差
    let mut diff_examples: Vec<(String, usize, usize)> = Vec::new();
    for p in &files {
        let data = match fs::read(p) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { continue; }
        let w = u16::from_le_bytes([data[12], data[13]]);
        let h = u16::from_le_bytes([data[14], data[15]]);
        let cc = data[0x16] as usize;
        let ps = 0x18 + cc * 3;
        let dec = match decompress_to_tokens(&data[ps..], w, h) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let new_payload = frame_payload(&dec.tokens);
        let orig_payload = &data[ps..];
        n_total += 1;
        if new_payload == orig_payload {
            n_match += 1;
        } else {
            // どこから違うか?
            let common_len = new_payload.len().min(orig_payload.len());
            let mut first_diff = common_len;
            for i in 0..common_len {
                if new_payload[i] != orig_payload[i] {
                    first_diff = i;
                    break;
                }
            }
            // 末尾 padding 差: 共通部分は一致するが長さが違う
            if first_diff == common_len && new_payload.len() != orig_payload.len() {
                n_pad_diff += 1;
            }
            if diff_examples.len() < 5 {
                diff_examples.push((
                    p.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string(),
                    first_diff,
                    orig_payload.len() as isize as usize - new_payload.len() as isize as usize,
                ));
            }
        }
    }
    println!("=== Rust leaf round-trip ===");
    println!("total: {}", n_total);
    println!("byte-exact match: {}", n_match);
    println!("size-only diff (= padding-only): {}", n_pad_diff);
    println!("real diff: {}", n_total - n_match - n_pad_diff);
    println!();
    println!("examples (first 5 diffs):");
    for (name, first, _diff_size) in diff_examples.iter() {
        println!("  {}: first_diff_at={}", name, first);
    }
    ExitCode::SUCCESS
}
