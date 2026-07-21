//! Stage 9 (Issue #14): 末尾緩和 (tail-relaxed) エンコーダ
//! (`compress_okumura_tail_relaxed`) の byte-exact 一致数を実測する。
//! lf2_stage3_verify.rs のコピーで、圧縮関数呼び出しのみ差し替えている。
//!
//! 各 LF2 ファイルについて:
//!   (a) 圧縮ペイロードをトークン列にデコード (`decompress_to_tokens`)
//!   (b) デコード時の ring 書込み順バイト列 (`ring_input`) を新 variant で再圧縮
//!   (c) LF2 framing (flag byte / XOR 0xff / 4bit len) で直列化し、
//!       元の圧縮ペイロードと byte 比較
//!
//! ヘッダ・パレットは元ファイルからそのまま使うため、ペイロード一致 =
//! ファイル全体の byte-exact 一致。
//!
//! usage:
//!   cargo run --release --bin lf2_stage9_verify -- <LF2_DIR> [--limit N] [--out MATCHED_TXT]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行 (name,payload_len,reenc_len,match,first_diff)
//!   stderr: サマリ (一致本数 / 総数)
//!   --out: 一致ファイル名リスト (デフォルト .local_data/stage9_matched.txt)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::compress_okumura_tail_relaxed;
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
    let mut out_path = String::from(".local_data/stage9_matched.txt");
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

    println!("name,payload_len,reenc_len,match,first_diff");

    let mut total = 0usize;
    let mut matched_names: Vec<String> = Vec::new();
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

        let tokens = compress_okumura_tail_relaxed(&decoded.ring_input);
        let reenc = tokens_to_lf2_payload(&tokens);
        let orig = decoded.payload.as_slice();

        // 注: 元ファイル末尾に decoder が消費しない trailing bytes がある場合、
        // 再圧縮ペイロードは短くなり不一致側に倒れる。既存 verify 系
        // (lf2_okumura_bench 等) と同じ割り切りで byte-exact を判定する。
        let is_match = orig == reenc.as_slice();
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
            "{},{},{},{},{}",
            name,
            orig.len(),
            reenc.len(),
            if is_match { 1 } else { 0 },
            first_diff
        );
        if is_match {
            matched_names.push(name);
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
        "byte-exact: {}/{} ({:.2}%)",
        matched_names.len(),
        total,
        if total > 0 {
            matched_names.len() as f64 * 100.0 / total as f64
        } else {
            0.0
        }
    );
    eprintln!("matched list -> {}", out_path);

    ExitCode::SUCCESS
}
