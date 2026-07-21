//! Stage 12-11 (Issue #14) 受入検証: 「腐った木」仮説 RotA/RotB variant で、
//! C0313.LF2 + 長尺上位10本を自走エンコードし、木の**構造的**整合性
//! (循環なし・dad/子の相互整合。順序は意図的に破れるので検査対象外) を
//! OKU_DEBUG_TREE_CHECK 経由で検証する。合格条件: 全ファイルで guard発動0件。
//!
//! usage:
//!   OKU_DEBUG_TREE_CHECK=1 cargo run --release --bin lf2_stage12_11_consistency_check -- <DIR>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_clip_rot_a, compress_okumura_clip_rot_b};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16];
    let payload_start = 0x18 + (colors as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);

    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("LF2"))
                .unwrap_or(false)
        })
        .collect();

    let mut sized: Vec<(u64, PathBuf)> = files
        .iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| (m.len(), p.clone())))
        .collect();
    sized.sort_by(|a, b| b.0.cmp(&a.0));
    let mut targets: Vec<PathBuf> = sized.into_iter().take(10).map(|(_, p)| p).collect();
    let c0313 = dir.join("C0313.LF2");
    if !targets.contains(&c0313) {
        targets.insert(0, c0313);
    }
    files = targets;

    eprintln!("=== Stage 12-11 RotA/RotB consistency check: {} files (C0313 + top10 largest) ===", files.len());

    let mut total = 0usize;
    let mut errors = 0usize;
    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errors += 1;
                continue;
            }
        };
        let Some((width, height, ps)) = parse_lf2(&data) else {
            eprintln!("parse fail {}", name);
            errors += 1;
            continue;
        };
        let Ok(decoded) = decompress_to_tokens(&data[ps..], width, height) else {
            eprintln!("decode fail {}", name);
            errors += 1;
            continue;
        };
        total += 1;
        let toks_a = compress_okumura_clip_rot_a(&decoded.ring_input);
        let toks_b = compress_okumura_clip_rot_b(&decoded.ring_input);
        println!(
            "{}: ring_len={} rotA_tokens={} rotB_tokens={}",
            name,
            decoded.ring_input.len(),
            toks_a.len(),
            toks_b.len()
        );
    }
    eprintln!("=== done: {} files processed ({} errors) ===", total, errors);
    eprintln!("(guard発動0件であることを `grep -c WARN` で別途確認すること)");

    ExitCode::SUCCESS
}
