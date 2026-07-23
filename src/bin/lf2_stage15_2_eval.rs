//! Stage 15-2 (Issue #14 脈: 既存3規則の全域再tie変種)
//!
//! `compress_okumura_global_retie` の6変種 (3規則 × EOF側の扱い2通り、
//! base=TaxBase::Basic固定) を522本全件に適用し、既存union268
//! (`.local_data/stage12_18/union_all.txt`) への純増を求める。
//! 新しい比較規則は一切追加していない (既存 `EofTieRule` 3種のみ)。
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_basic_global_closest_dist_eof_default,
    compress_okumura_basic_global_closest_dist_eof_same,
    compress_okumura_basic_global_closest_to_raw_eof_default,
    compress_okumura_basic_global_closest_to_raw_eof_same,
    compress_okumura_basic_global_farthest_dist_eof_default,
    compress_okumura_basic_global_farthest_dist_eof_same, Token,
};

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        (
            "global_closest_dist_eof_default",
            compress_okumura_basic_global_closest_dist_eof_default as fn(&[u8]) -> Vec<Token>,
        ),
        (
            "global_closest_dist_eof_same",
            compress_okumura_basic_global_closest_dist_eof_same,
        ),
        (
            "global_farthest_dist_eof_default",
            compress_okumura_basic_global_farthest_dist_eof_default,
        ),
        (
            "global_farthest_dist_eof_same",
            compress_okumura_basic_global_farthest_dist_eof_same,
        ),
        (
            "global_closest_to_raw_eof_default",
            compress_okumura_basic_global_closest_to_raw_eof_default,
        ),
        (
            "global_closest_to_raw_eof_same",
            compress_okumura_basic_global_closest_to_raw_eof_same,
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
    let out_dir = PathBuf::from(&args[3]);

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
    // per-variant: 一致したファイル名一覧
    let mut hit_lists: Vec<(&'static str, Vec<String>)> =
        vs.iter().map(|(n, _)| (*n, Vec::new())).collect();
    let mut net_new: Vec<(String, Vec<&'static str>)> = Vec::new();
    let mut any_hit_files: BTreeSet<String> = BTreeSet::new();

    for p in &paths {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap().to_string();
        let data = fs::read(p).unwrap();
        if data.len() < 0x18 || &data[0..8] != b"LEAF256\0" {
            eprintln!("WARN skip {}: not LEAF256", name);
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
        for (idx, (vname, f)) in vs.iter().enumerate() {
            let toks = f(&dec.ring_input);
            let payload = frame_payload(&toks);
            if payload == *original_payload {
                hit_variants.push(*vname);
                hit_lists[idx].1.push(name.clone());
            }
        }
        if !hit_variants.is_empty() {
            any_hit_files.insert(name.clone());
            if !union_existing.contains(&name) {
                net_new.push((name.clone(), hit_variants));
            }
        }
    }

    fs::create_dir_all(&out_dir).unwrap();

    println!("=== per-variant hit counts (of {}) ===", paths.len());
    for (name, files) in &hit_lists {
        println!("{} -> {} hits", name, files.len());
        let out_path = out_dir.join(format!("hits_{}.txt", name));
        fs::write(&out_path, files.join("\n") + "\n").unwrap();
    }

    println!(
        "\nfiles hit by >=1 new variant (of {}): {}",
        paths.len(),
        any_hit_files.len()
    );
    println!(
        "net NEW files (not already in union{}): {}",
        union_existing.len(),
        net_new.len()
    );
    let mut delta_lines = Vec::new();
    for (name, vs) in &net_new {
        println!("  {} -> {:?}", name, vs);
        delta_lines.push(format!("{} -> {:?}", name, vs));
    }
    fs::write(
        out_dir.join("union_delta.txt"),
        delta_lines.join("\n") + "\n",
    )
    .unwrap();

    // 本命4本 (V31/V32/C0E01/C0E02) の判定を明示出力
    println!("\n=== 本命4本の判定 ===");
    for target in ["V31.LF2", "V32.LF2", "C0E01.LF2", "C0E02.LF2"] {
        let hit_by: Vec<&str> = hit_lists
            .iter()
            .filter(|(_, files)| files.iter().any(|f| f == target))
            .map(|(n, _)| *n)
            .collect();
        println!("{} -> hit by: {:?}", target, hit_by);
    }
}
