//! Stage 9-2d (Issue #14): Clip / Plus1 フォールバック合成 variant の
//! byte-exact 一致数を実測する。
//! lf2_stage9_2_verify.rs のコピーで、圧縮ロジックを
//! 「まず Clip (Basic) で再圧縮・不一致なら Plus1 で再挑戦」の
//! フォールバックに差し替えている。
//!
//! Stage 9-2c で「broken30 はタイ/候補選択差ではなく、Leaf が実際に
//! Clip=remaining と Clip=remaining+1 の2実装亜種を使い分けている」と判定した
//! ことを受け、per-file に「どちらの規則で一致したか」を出力する。
//!
//! 各 LF2 ファイルについて:
//!   (a) 圧縮ペイロードをトークン列にデコード (`decompress_to_tokens`)
//!   (b) デコード時の ring 書込み順バイト列 (`ring_input`) を Clip (Basic) で
//!       再圧縮・byte 比較。一致すれば mode=clip
//!   (c) 不一致なら Plus1 で再圧縮・byte 比較。一致すれば mode=plus1
//!   (d) どちらも不一致なら mode=none (元の Clip 側の first_diff を報告)
//!
//! usage:
//!   cargo run --release --bin lf2_stage9_2d_verify -- <LF2_DIR> [--limit N] [--out MATCHED_TXT]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行 (name,payload_len,reenc_len,match,mode,first_diff)
//!   stderr: サマリ (一致本数 / 総数、mode 別内訳)
//!   --out: 一致ファイル名リスト (デフォルト .local_data/stage9_2d_matched.txt)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, compress_okumura_tail_plus1};
use retro_decode::formats::toheart::verify_harness::{self, tokens_to_lf2_payload};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <lf2_dir> [--limit N] [--out matched.txt]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut limit: Option<usize> = None;
    let mut out_path = String::from(".local_data/stage9_2d_matched.txt");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                limit = args.get(i + 1).and_then(|v| v.parse().ok());
                i += 2;
            }
            "--out" => {
                if let Some(v) = args.get(i + 1) {
                    out_path = v.clone();
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let files: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, limit) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };

    println!("name,payload_len,reenc_len,match,mode,first_diff");

    let mut total = 0usize;
    let mut matched_names: Vec<String> = Vec::new();
    let mut errors = 0usize;
    let mut n_clip = 0usize;
    let mut n_plus1 = 0usize;

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

        // まず Clip (Basic) を試す
        let clip_tokens = compress_okumura(&decoded.ring_input);
        let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
        let clip_match = orig == clip_reenc.as_slice();

        let (is_match, mode, reenc) = if clip_match {
            (true, "clip", clip_reenc)
        } else {
            // 不一致なら Plus1 で再挑戦
            let plus1_tokens = compress_okumura_tail_plus1(&decoded.ring_input);
            let plus1_reenc = tokens_to_lf2_payload(&plus1_tokens);
            if orig == plus1_reenc.as_slice() {
                (true, "plus1", plus1_reenc)
            } else {
                (false, "none", clip_reenc)
            }
        };

        let first_diff = if is_match {
            String::from("-")
        } else {
            let ml = orig.len().min(reenc.len());
            (0..ml)
                .find(|&k| orig[k] != reenc[k])
                .unwrap_or(ml)
                .to_string()
        };
        println!(
            "{},{},{},{},{},{}",
            name,
            orig.len(),
            reenc.len(),
            if is_match { 1 } else { 0 },
            mode,
            first_diff
        );
        if is_match {
            matched_names.push(name.clone());
            match mode {
                "clip" => n_clip += 1,
                "plus1" => n_plus1 += 1,
                _ => {}
            }
        }
    }

    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Ok(mut f) = fs::File::create(&out_path) {
        for n in &matched_names {
            let _ = writeln!(f, "{}", n);
        }
    } else {
        eprintln!("warn: failed to write {}", out_path);
    }

    eprintln!("---");
    eprintln!("files     : {} (errors {})", total, errors);
    eprintln!(
        "byte-exact: {}/{} ({:.2}%)  clip={} plus1={}",
        matched_names.len(),
        total,
        if total > 0 {
            matched_names.len() as f64 * 100.0 / total as f64
        } else {
            0.0
        },
        n_clip,
        n_plus1
    );
    eprintln!("matched list -> {}", out_path);

    ExitCode::SUCCESS
}
