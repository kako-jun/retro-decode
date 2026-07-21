//! Stage 12-15 Step 3 (Issue #14 脈1 Prong B): 「書込み時挿入」フル変種
//! (`compress_okumura_clip_writetime_descending` / `..._plus1_writetime_descending`)
//! の byte-exact 一致数を、既存 203/522 union (stage10_5 の8モード) に重ねて
//! 計測する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_15_verify -- <LF2_DIR> [--out-prefix PREFIX]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行
//!     (name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,clip_wtd,plus1_wtd,best_mode)
//!   stderr: サマリ (各モード一致数、既存203 union、新union、増分ファイル一覧)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_clip_no_bootstrap_v2,
    compress_okumura_clip_no_bootstrap_v3, compress_okumura_clip_writetime_descending,
    compress_okumura_plus1_no_bootstrap_v1, compress_okumura_plus1_no_bootstrap_v2,
    compress_okumura_plus1_no_bootstrap_v3, compress_okumura_plus1_writetime_descending,
    compress_okumura_tail_plus1,
};
use retro_decode::formats::toheart::verify_harness::{self, matches};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-prefix PREFIX]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_prefix = String::from(".local_data/stage12_15/verify");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out-prefix" => {
                if let Some(v) = args.get(i + 1) {
                    out_prefix = v.clone();
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }
    if let Some(parent) = PathBuf::from(&out_prefix).parent() {
        fs::create_dir_all(parent).ok();
    }

    let files: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, None) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };

    println!(
        "name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,clip_wtd,plus1_wtd,best_mode"
    );

    let mode_names = [
        "clip", "plus1", "clip_v1", "plus1_v1", "clip_v2", "plus1_v2", "clip_v3", "plus1_v3",
        "clip_wtd", "plus1_wtd",
    ];
    let mut matched_lists: Vec<Vec<String>> = vec![Vec::new(); mode_names.len()];
    let mut union203: Vec<String> = Vec::new(); // 既存8モード (stage10_5 union) = 現行203
    let mut new_union: Vec<String> = Vec::new(); // 既存8モード + wtd2モード

    let mut total = 0usize;
    let mut errors = 0usize;

    for path in &files {
        let decoded = match verify_harness::load_and_decode(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);
                errors += 1;
                continue;
            }
        };
        total += 1;
        let name = decoded.name.clone();
        let orig = decoded.payload.as_slice();
        let ring_input = &decoded.ring_input;

        let flags = [
            matches(ring_input, orig, compress_okumura),
            matches(ring_input, orig, compress_okumura_tail_plus1),
            matches(ring_input, orig, compress_okumura_clip_no_bootstrap_v1),
            matches(ring_input, orig, compress_okumura_plus1_no_bootstrap_v1),
            matches(ring_input, orig, compress_okumura_clip_no_bootstrap_v2),
            matches(ring_input, orig, compress_okumura_plus1_no_bootstrap_v2),
            matches(ring_input, orig, compress_okumura_clip_no_bootstrap_v3),
            matches(ring_input, orig, compress_okumura_plus1_no_bootstrap_v3),
            matches(ring_input, orig, compress_okumura_clip_writetime_descending),
            matches(ring_input, orig, compress_okumura_plus1_writetime_descending),
        ];

        let mut best_mode = "none";
        for (idx, &f) in flags.iter().enumerate() {
            if f {
                matched_lists[idx].push(name.clone());
                if best_mode == "none" {
                    best_mode = mode_names[idx];
                }
            }
        }
        if flags[0..8].iter().any(|&f| f) {
            union203.push(name.clone());
        }
        if flags.iter().any(|&f| f) {
            new_union.push(name.clone());
        }

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            orig.len(),
            flags[0] as u8,
            flags[1] as u8,
            flags[2] as u8,
            flags[3] as u8,
            flags[4] as u8,
            flags[5] as u8,
            flags[6] as u8,
            flags[7] as u8,
            flags[8] as u8,
            flags[9] as u8,
            best_mode
        );
    }

    for (idx, name) in mode_names.iter().enumerate() {
        let out_path = format!("{}_matched_{}.txt", out_prefix, name);
        if let Ok(mut f) = fs::File::create(&out_path) {
            for n in &matched_lists[idx] {
                let _ = writeln!(f, "{}", n);
            }
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_union203.txt", out_prefix)) {
        for n in &union203 {
            let _ = writeln!(f, "{}", n);
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_new_union.txt", out_prefix)) {
        for n in &new_union {
            let _ = writeln!(f, "{}", n);
        }
    }
    let newly_added: Vec<&String> = new_union.iter().filter(|n| !union203.contains(n)).collect();
    if let Ok(mut f) = fs::File::create(format!("{}_newly_added.txt", out_prefix)) {
        for n in &newly_added {
            let _ = writeln!(f, "{}", n);
        }
    }

    eprintln!("---");
    eprintln!("files: {} (errors {})", total, errors);
    for (idx, name) in mode_names.iter().enumerate() {
        eprintln!("{:14}: {}/{}", name, matched_lists[idx].len(), total);
    }
    eprintln!("union203 (既存8モード, 現行baseline) : {}/{}", union203.len(), total);
    eprintln!("new_union (既存8 + wtd2)            : {}/{}", new_union.len(), total);
    eprintln!("増分 (new_union - union203)        : {}", newly_added.len());
    for n in &newly_added {
        eprintln!("  NEW: {}", n);
    }

    ExitCode::SUCCESS
}
