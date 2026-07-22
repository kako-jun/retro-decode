//! Stage 12-17 Step 2 (Issue #14 脈1 Prong B 続き): dummy帯保持ハイブリッド
//! (`compress_okumura_clip/plus1_writetime_descending_keepdummy` /
//! `..._ascending_keepdummy`) の byte-exact 一致数を、既存 union
//! (stage12_16 の12モード、207/522) に重ねて計測する。
//!
//! Step 1 (`lf2_stage12_17_offtree_profile`) で、退行711件(WTD)/362件(WTA)の
//! うち173件が「dummy帯 [4060,4077] が一度も挿入されない」ことに起因すると特定。
//! Step 2 deepscan (`lf2_stage12_17_deepscan`) では KeepDummy 導入で退行が
//! WTD 711→427、WTA 362→82 に減る一方、救済も WTD 90→6、WTA 84→0 に激減する
//! トレードオフを確認済み。ここでは実際の byte-exact union への影響を測る。
//!
//! サニティ: 長尺上位10本 + C0313.LF2 を `OKU_DEBUG_TREE_CHECK=1` 相当で先に
//! 流し、毎操作不変条件チェックでクラッシュしないことを確認してから本計測。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_17_verify -- <LF2_DIR> [--out-prefix PREFIX]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_clip_no_bootstrap_v2,
    compress_okumura_clip_no_bootstrap_v3, compress_okumura_clip_writetime_ascending,
    compress_okumura_clip_writetime_ascending_keepdummy, compress_okumura_clip_writetime_descending,
    compress_okumura_clip_writetime_descending_keepdummy, compress_okumura_plus1_no_bootstrap_v1,
    compress_okumura_plus1_no_bootstrap_v2, compress_okumura_plus1_no_bootstrap_v3,
    compress_okumura_plus1_writetime_ascending, compress_okumura_plus1_writetime_ascending_keepdummy,
    compress_okumura_plus1_writetime_descending, compress_okumura_plus1_writetime_descending_keepdummy,
    compress_okumura_tail_plus1,
};
use retro_decode::formats::toheart::verify_harness::{self, matches};

fn run_sanity(dir: &PathBuf) -> bool {
    let files = match verify_harness::list_lf2_files(dir, None) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("sanity: failed to read dir {:?}: {}", dir, e);
            return false;
        }
    };
    let mut sized: Vec<(u64, PathBuf)> = files
        .iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| (m.len(), p.clone())))
        .collect();
    sized.sort_by(|a, b| b.0.cmp(&a.0));
    let mut targets: Vec<PathBuf> = sized.into_iter().take(10).map(|(_, p)| p).collect();
    let c0313 = dir.join("C0313.LF2");
    if !targets.contains(&c0313) && c0313.exists() {
        targets.push(c0313);
    }

    env::set_var("OKU_DEBUG_TREE_CHECK", "1");
    eprintln!("=== Stage 12-17 サニティ: {} files (毎操作不変条件チェック有効) ===", targets.len());
    let mut ok = true;
    for path in &targets {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let decoded = match verify_harness::load_and_decode(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("sanity load fail {}: {}", name, e);
                ok = false;
                continue;
            }
        };
        let ring_input = &decoded.ring_input;
        let _ = compress_okumura_clip_writetime_descending_keepdummy(ring_input);
        let _ = compress_okumura_plus1_writetime_descending_keepdummy(ring_input);
        let _ = compress_okumura_clip_writetime_ascending_keepdummy(ring_input);
        let _ = compress_okumura_plus1_writetime_ascending_keepdummy(ring_input);
        eprintln!("  sanity OK: {} (ring_len={})", name, ring_input.len());
    }
    env::remove_var("OKU_DEBUG_TREE_CHECK");
    eprintln!("=== サニティ完了: {} files, クラッシュ0件 ===", targets.len());
    ok
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-prefix PREFIX]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_prefix = String::from(".local_data/stage12_17/verify");
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

    if !run_sanity(&dir) {
        eprintln!("サニティ失敗、本計測を中止する");
        return ExitCode::from(1);
    }

    let files: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, None) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };

    let mode_names = [
        "clip", "plus1", "clip_v1", "plus1_v1", "clip_v2", "plus1_v2", "clip_v3", "plus1_v3",
        "clip_wtd", "plus1_wtd", "clip_wta", "plus1_wta", "clip_wtd_kd", "plus1_wtd_kd", "clip_wta_kd",
        "plus1_wta_kd",
    ];
    println!("name,payload_len,{},best_mode", mode_names.join(","));

    let mut matched_lists: Vec<Vec<String>> = vec![Vec::new(); mode_names.len()];
    let mut union207: Vec<String> = Vec::new(); // 既存12モード (stage12_16 union) = 現行207
    let mut new_union: Vec<String> = Vec::new(); // 既存12 + KeepDummy4

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
            matches(ring_input, orig, compress_okumura_clip_writetime_ascending),
            matches(ring_input, orig, compress_okumura_plus1_writetime_ascending),
            matches(ring_input, orig, compress_okumura_clip_writetime_descending_keepdummy),
            matches(ring_input, orig, compress_okumura_plus1_writetime_descending_keepdummy),
            matches(ring_input, orig, compress_okumura_clip_writetime_ascending_keepdummy),
            matches(ring_input, orig, compress_okumura_plus1_writetime_ascending_keepdummy),
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
        if flags[0..12].iter().any(|&f| f) {
            union207.push(name.clone());
        }
        if flags.iter().any(|&f| f) {
            new_union.push(name.clone());
        }

        let flag_str: Vec<String> = flags.iter().map(|f| (*f as u8).to_string()).collect();
        println!("{},{},{},{}", name, orig.len(), flag_str.join(","), best_mode);
    }

    for (idx, name) in mode_names.iter().enumerate() {
        let out_path = format!("{}_matched_{}.txt", out_prefix, name);
        if let Ok(mut f) = fs::File::create(&out_path) {
            for n in &matched_lists[idx] {
                let _ = writeln!(f, "{}", n);
            }
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_union207.txt", out_prefix)) {
        for n in &union207 {
            let _ = writeln!(f, "{}", n);
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_new_union.txt", out_prefix)) {
        for n in &new_union {
            let _ = writeln!(f, "{}", n);
        }
    }
    let newly_added: Vec<&String> = new_union.iter().filter(|n| !union207.contains(n)).collect();
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
    eprintln!("union207 (既存12モード, 現行baseline) : {}/{}", union207.len(), total);
    eprintln!("new_union (既存12 + KeepDummy4)       : {}/{}", new_union.len(), total);
    eprintln!("増分 (new_union - union207)           : {}", newly_added.len());
    for n in &newly_added {
        eprintln!("  NEW: {}", n);
    }

    ExitCode::SUCCESS
}
