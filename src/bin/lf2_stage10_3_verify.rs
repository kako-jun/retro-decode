//! Stage 10-3 (Issue #14): 4モード {Clip, Plus1} x {dummy可, dummy不可(RejectAny)}
//! の byte-exact 一致数を実測し、per-file にどのモードで一致したか (該当なし
//! なら none) を出す。union (いずれかのモードで一致) も算出する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage10_3_verify -- <LF2_DIR> [--limit N]
//!       [--out-prefix .local_data/stage10_3]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行
//!     (name,payload_len,match_clip,match_plus1,match_clip_nodummy,match_plus1_nodummy,best_mode)
//!   stderr: サマリ (各モード一致数・union)
//!   <out-prefix>_matched_{clip,plus1,clip_nodummy,plus1_nodummy,union}.txt: 一致ファイル名リスト

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_dummy_all, compress_okumura_clip_no_dummy_any,
    compress_okumura_plus1_no_dummy_all, compress_okumura_plus1_no_dummy_any,
    compress_okumura_tail_plus1, Token,
};

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

fn tokens_to_lf2_payload(tokens: &[Token]) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let flag_pos = compressed.len();
        compressed.push(0);
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

fn matches(ring_input: &[u8], orig: &[u8], f: impl Fn(&[u8]) -> Vec<Token>) -> bool {
    let toks = f(ring_input);
    let reenc = tokens_to_lf2_payload(&toks);
    orig == reenc.as_slice()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <lf2_dir> [--limit N] [--out-prefix PREFIX]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut limit: Option<usize> = None;
    let mut out_prefix = String::from(".local_data/stage10_3");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                limit = args.get(i + 1).and_then(|v| v.parse().ok());
                i += 2;
            }
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

    println!(
        "name,payload_len,match_clip,match_plus1,match_clip_nodummy,match_plus1_nodummy,best_mode"
    );

    let mut total = 0usize;
    let mut errors = 0usize;
    let mut m_clip: Vec<String> = Vec::new();
    let mut m_plus1: Vec<String> = Vec::new();
    let mut m_clip_nd: Vec<String> = Vec::new();
    let mut m_plus1_nd: Vec<String> = Vec::new();
    let mut m_clip_nd_all: Vec<String> = Vec::new();
    let mut m_plus1_nd_all: Vec<String> = Vec::new();
    let mut m_union: Vec<String> = Vec::new();

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
        let orig = &data[ps..];

        let is_clip = matches(&decoded.ring_input, orig, compress_okumura);
        let is_plus1 = matches(&decoded.ring_input, orig, compress_okumura_tail_plus1);
        let is_clip_nd = matches(
            &decoded.ring_input,
            orig,
            compress_okumura_clip_no_dummy_any,
        );
        let is_plus1_nd = matches(
            &decoded.ring_input,
            orig,
            compress_okumura_plus1_no_dummy_any,
        );
        let is_clip_nd_all = matches(
            &decoded.ring_input,
            orig,
            compress_okumura_clip_no_dummy_all,
        );
        let is_plus1_nd_all = matches(
            &decoded.ring_input,
            orig,
            compress_okumura_plus1_no_dummy_all,
        );
        if is_clip_nd_all {
            m_clip_nd_all.push(name.clone());
        }
        if is_plus1_nd_all {
            m_plus1_nd_all.push(name.clone());
        }

        let best_mode = if is_clip {
            "clip"
        } else if is_plus1 {
            "plus1"
        } else if is_clip_nd {
            "clip_nodummy"
        } else if is_plus1_nd {
            "plus1_nodummy"
        } else {
            "none"
        };

        if is_clip {
            m_clip.push(name.clone());
        }
        if is_plus1 {
            m_plus1.push(name.clone());
        }
        if is_clip_nd {
            m_clip_nd.push(name.clone());
        }
        if is_plus1_nd {
            m_plus1_nd.push(name.clone());
        }
        if is_clip || is_plus1 || is_clip_nd || is_plus1_nd || is_clip_nd_all || is_plus1_nd_all {
            m_union.push(name.clone());
        }

        println!(
            "{},{},{},{},{},{},{}",
            name,
            orig.len(),
            is_clip as u8,
            is_plus1 as u8,
            is_clip_nd as u8,
            is_plus1_nd as u8,
            best_mode
        );
    }

    for (suffix, list) in [
        ("clip", &m_clip),
        ("plus1", &m_plus1),
        ("clip_nodummy", &m_clip_nd),
        ("plus1_nodummy", &m_plus1_nd),
        ("clip_nodummy_all", &m_clip_nd_all),
        ("plus1_nodummy_all", &m_plus1_nd_all),
        ("union", &m_union),
    ] {
        let out_path = format!("{}_matched_{}.txt", out_prefix, suffix);
        if let Some(parent) = std::path::Path::new(&out_path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        if let Ok(mut f) = fs::File::create(&out_path) {
            for n in list {
                let _ = writeln!(f, "{}", n);
            }
        }
    }

    eprintln!("---");
    eprintln!("files: {} (errors {})", total, errors);
    eprintln!("clip           : {}/{}", m_clip.len(), total);
    eprintln!("plus1          : {}/{}", m_plus1.len(), total);
    eprintln!("clip_nodummy   : {}/{}", m_clip_nd.len(), total);
    eprintln!("plus1_nodummy  : {}/{}", m_plus1_nd.len(), total);
    eprintln!("clip_nodummy_all : {}/{}", m_clip_nd_all.len(), total);
    eprintln!("plus1_nodummy_all: {}/{}", m_plus1_nd_all.len(), total);
    eprintln!("union          : {}/{}", m_union.len(), total);

    ExitCode::SUCCESS
}
