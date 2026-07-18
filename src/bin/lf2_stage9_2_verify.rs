//! Stage 9-2 (Issue #14): 末尾 remaining+1 クリップエンコーダ
//! (`compress_okumura_tail_plus1`) の byte-exact 一致数を実測する。
//! lf2_stage9_verify.rs のコピーで、圧縮関数呼び出しのみ差し替えている。
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
//!   --out: 一致ファイル名リスト (デフォルト .local_data/stage9_2_matched.txt)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_tail_plus1, Token};

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

/// トークン列を LF2 圧縮ペイロードに直列化する
/// (`Lf2Image::to_lf2_bytes_okumura` の framing と同一)。
fn tokens_to_lf2_payload(tokens: &[Token]) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let flag_pos = compressed.len();
        compressed.push(0); // placeholder

        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    compressed.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let encoded_pos = (pos as usize) & 0x0fff;
                    let encoded_len = ((len as usize) - 3) & 0x0f;
                    let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                    let lower = ((encoded_pos >> 4) & 0xff) as u8;
                    compressed.push(upper ^ 0xff);
                    compressed.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }

        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    compressed
}

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
    let mut out_path = String::from(".local_data/stage9_2_matched.txt");
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

    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("LF2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };
    files.sort();
    if let Some(n) = limit {
        files.truncate(n);
    }

    println!("name,payload_len,reenc_len,match,first_diff");

    let mut total = 0usize;
    let mut matched_names: Vec<String> = Vec::new();
    let mut errors = 0usize;

    for path in &files {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errors += 1;
                continue;
            }
        };
        let (width, height, ps) = match parse_lf2(&data) {
            Some(x) => x,
            None => {
                eprintln!("parse fail {}", name);
                errors += 1;
                continue;
            }
        };
        let decoded = match decompress_to_tokens(&data[ps..], width, height) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("decode fail {}: {}", name, e);
                errors += 1;
                continue;
            }
        };
        total += 1;

        let tokens = compress_okumura_tail_plus1(&decoded.ring_input);
        let reenc = tokens_to_lf2_payload(&tokens);
        let orig = &data[ps..];

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
