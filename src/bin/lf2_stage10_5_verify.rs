//! Stage 10-5 (Issue #14): ブートストラップダミーノード帯 [4060,4077] を狙った
//! predicate 可変の reject variant 3種 (v1/v2/v3) x {Clip, Plus1} = 6モードを
//! 522本フル計測する。
//!
//! - v1 (`RejectBootstrapUnwritten`): 候補位置がダミーノード帯かつ窓が全域
//!   未書込みなら不採用
//! - v2 (`RejectBootstrapUnwrittenLenGt10`): v1 に加えて len>10 のみ不採用
//! - v3 (`RejectBootstrapEdge`): 候補位置が帯の端 (4076/4077) ならそれだけで
//!   不採用 (窓条件なし)
//!
//! usage:
//!   cargo run --release --bin lf2_stage10_5_verify -- <LF2_DIR> [--out-prefix PREFIX]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行
//!     (name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,best_mode)
//!   stderr: サマリ (各モード一致数、union、既存197基準からのfixed/broken)
//!   <out-prefix>_matched_{mode}.txt / <out-prefix>_union.txt

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_clip_no_bootstrap_v2,
    compress_okumura_clip_no_bootstrap_v3, compress_okumura_plus1_no_bootstrap_v1,
    compress_okumura_plus1_no_bootstrap_v2, compress_okumura_plus1_no_bootstrap_v3,
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
    let mut out_prefix = String::from(".local_data/stage10_5");
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

    let files: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, None) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };

    println!(
        "name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,best_mode"
    );

    let mode_names = [
        "clip", "plus1", "clip_v1", "plus1_v1", "clip_v2", "plus1_v2", "clip_v3", "plus1_v3",
    ];
    let mut matched_lists: Vec<Vec<String>> = vec![Vec::new(); mode_names.len()];
    let mut union: Vec<String> = Vec::new();
    let mut baseline197: Vec<String> = Vec::new(); // clip || plus1 (既存の197基準)

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
        if flags.iter().any(|&f| f) {
            union.push(name.clone());
        }
        if flags[0] || flags[1] {
            baseline197.push(name.clone());
        }

        println!(
            "{},{},{},{},{},{},{},{},{},{},{}",
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
    if let Ok(mut f) = fs::File::create(format!("{}_union.txt", out_prefix)) {
        for n in &union {
            let _ = writeln!(f, "{}", n);
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_baseline197.txt", out_prefix)) {
        for n in &baseline197 {
            let _ = writeln!(f, "{}", n);
        }
    }

    eprintln!("---");
    eprintln!("files: {} (errors {})", total, errors);
    for (idx, name) in mode_names.iter().enumerate() {
        eprintln!("{:14}: {}/{}", name, matched_lists[idx].len(), total);
    }
    eprintln!("baseline197 (clip||plus1): {}/{}", baseline197.len(), total);
    eprintln!("union (all 8 modes)      : {}/{}", union.len(), total);

    ExitCode::SUCCESS
}
