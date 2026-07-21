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

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_dummy_all, compress_okumura_clip_no_dummy_any,
    compress_okumura_plus1_no_dummy_all, compress_okumura_plus1_no_dummy_any,
    compress_okumura_tail_plus1,
};
use retro_decode::formats::toheart::verify_harness::{self, matches};

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

    let files: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, limit) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };

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
