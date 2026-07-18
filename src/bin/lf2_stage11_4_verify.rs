//! Stage 11-4 (Issue #14): v1 の write_tick 偽陽性を補正した v4
//! (`DummyMode::RejectPureBootstrap`) を追加し、既存の全モード
//! (Clip/Plus1/v1/v2/v3/v4) で 522 本フル計測・union を再計算する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage11_4_verify -- <LF2_DIR> [--out-prefix PREFIX]
//!
//! 出力:
//!   stdout: 1 ファイル 1 行
//!     (name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,clip_v4,plus1_v4,best_mode)
//!   stderr: サマリ (各モード一致数、union、既存197/203基準からの増分)

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_clip_no_bootstrap_v2,
    compress_okumura_clip_no_bootstrap_v3, compress_okumura_clip_no_bootstrap_v4,
    compress_okumura_plus1_no_bootstrap_v1, compress_okumura_plus1_no_bootstrap_v2,
    compress_okumura_plus1_no_bootstrap_v3, compress_okumura_plus1_no_bootstrap_v4,
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
        eprintln!("usage: {} <lf2_dir> [--out-prefix PREFIX]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_prefix = String::from(".local_data/stage11_4");
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

    println!(
        "name,payload_len,clip,plus1,clip_v1,plus1_v1,clip_v2,plus1_v2,clip_v3,plus1_v3,clip_v4,plus1_v4,best_mode"
    );

    let mode_names = [
        "clip", "plus1", "clip_v1", "plus1_v1", "clip_v2", "plus1_v2", "clip_v3", "plus1_v3",
        "clip_v4", "plus1_v4",
    ];
    let mut matched_lists: Vec<Vec<String>> = vec![Vec::new(); mode_names.len()];
    let mut union: Vec<String> = Vec::new();
    let mut baseline197: Vec<String> = Vec::new(); // clip || plus1
    let mut baseline203: Vec<String> = Vec::new(); // clip||plus1||v1||v2||v3 (Stage10-5 union)

    let mut total = 0usize;
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
        let orig = &data[ps..];
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
            matches(ring_input, orig, compress_okumura_clip_no_bootstrap_v4),
            matches(ring_input, orig, compress_okumura_plus1_no_bootstrap_v4),
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
        if flags[0..8].iter().any(|&f| f) {
            baseline203.push(name.clone());
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
    if let Ok(mut f) = fs::File::create(format!("{}_baseline203.txt", out_prefix)) {
        for n in &baseline203 {
            let _ = writeln!(f, "{}", n);
        }
    }

    eprintln!("---");
    eprintln!("files: {} (errors {})", total, errors);
    for (idx, name) in mode_names.iter().enumerate() {
        eprintln!("{:10}: {}/{}", name, matched_lists[idx].len(), total);
    }
    eprintln!("baseline197 (clip||plus1)             : {}/{}", baseline197.len(), total);
    eprintln!("baseline203 (clip..v3, Stage10-5 union): {}/{}", baseline203.len(), total);
    eprintln!("union (all 10 modes, incl v4)          : {}/{}", union.len(), total);

    ExitCode::SUCCESS
}
