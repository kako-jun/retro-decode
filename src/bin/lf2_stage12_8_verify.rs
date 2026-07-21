//! Stage 12-8b (Issue #14): DelSuccessor 系 4 モードの 522本フル計測。
//! {Clip, Plus1} × {DelSuccessor} と {Clip, Plus1} × {DelSuccessor+WriteTimeDescending}
//! を既存 baseline203 (10モード) にクロスし、union 増分・新規一致ファイルの
//! 由来 (旧分類・tailモード) を集計する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_8_verify -- <LF2_DIR> [--out-prefix PREFIX]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_del_successor, compress_okumura_clip_del_successor_wtd,
    compress_okumura_clip_no_bootstrap_v1, compress_okumura_clip_no_bootstrap_v2,
    compress_okumura_clip_no_bootstrap_v3, compress_okumura_plus1_del_successor,
    compress_okumura_plus1_del_successor_wtd, compress_okumura_plus1_no_bootstrap_v1,
    compress_okumura_plus1_no_bootstrap_v2, compress_okumura_plus1_no_bootstrap_v3,
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

fn load_old_classification(path: &str) -> BTreeMap<String, (String, String)> {
    // file,class,mode,di,input_pos
    let mut map = BTreeMap::new();
    if let Ok(content) = fs::read_to_string(path) {
        let mut lines = content.lines();
        lines.next(); // header
        for line in lines {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() < 3 {
                continue;
            }
            map.insert(f[0].to_string(), (f[1].to_string(), f[2].to_string()));
        }
    }
    map
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-prefix PREFIX]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_prefix = String::from(".local_data/stage12_8");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out-prefix" => {
                out_prefix = args[i + 1].clone();
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let old_class = load_old_classification(".local_data/stage10_summary.csv");

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

    // 既存10モード (Predecessor 系、baseline203 = 0..10 の any) + 新規4モード (Successor 系)
    let mode_names = [
        "clip", "plus1", "clip_v1", "plus1_v1", "clip_v2", "plus1_v2", "clip_v3", "plus1_v3",
        "clip_delsucc", "plus1_delsucc", "clip_delsucc_wtd", "plus1_delsucc_wtd",
    ];
    // (tail, del) の由来ラベル (best_mode 判定用)
    let mode_tail = ["Clip", "Plus1", "Clip", "Plus1", "Clip", "Plus1", "Clip", "Plus1", "Clip", "Plus1", "Clip", "Plus1"];
    let mode_del = [
        "Predecessor", "Predecessor", "Predecessor", "Predecessor", "Predecessor", "Predecessor",
        "Predecessor", "Predecessor", "Successor", "Successor", "Successor", "Successor",
    ];

    let mut matched_lists: Vec<Vec<String>> = vec![Vec::new(); mode_names.len()];
    let mut baseline203: Vec<String> = Vec::new();
    let mut new_union: Vec<String> = Vec::new();
    let mut newly_added: Vec<(String, String, String, String)> = Vec::new(); // (file, best_mode, old_class, old_mode)
    let mut best_mode_idx_map: BTreeMap<String, usize> = BTreeMap::new();

    let mut total = 0usize;
    let mut errors = 0usize;

    for path in &files {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string();
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
            matches(ring_input, orig, compress_okumura_clip_del_successor),
            matches(ring_input, orig, compress_okumura_plus1_del_successor),
            matches(ring_input, orig, compress_okumura_clip_del_successor_wtd),
            matches(ring_input, orig, compress_okumura_plus1_del_successor_wtd),
        ];

        let mut best_mode = "none";
        for (idx, &f) in flags.iter().enumerate() {
            if f {
                matched_lists[idx].push(name.clone());
                if best_mode == "none" {
                    best_mode = mode_names[idx];
                    best_mode_idx_map.insert(name.clone(), idx);
                }
            }
        }
        let old_match = flags[0..8].iter().any(|&f| f);
        let any_match = flags.iter().any(|&f| f);
        if old_match {
            baseline203.push(name.clone());
        }
        if any_match {
            new_union.push(name.clone());
        }
        if any_match && !old_match {
            let (oc, om) = old_class.get(&name).cloned().unwrap_or(("?".to_string(), "?".to_string()));
            newly_added.push((name.clone(), best_mode.to_string(), oc, om));
        }
    }

    for (idx, name) in mode_names.iter().enumerate() {
        let out_path = format!("{}_matched_{}.txt", out_prefix, name);
        if let Ok(mut f) = fs::File::create(&out_path) {
            for n in &matched_lists[idx] {
                let _ = writeln!(f, "{}", n);
            }
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_new_union.txt", out_prefix)) {
        for n in &new_union {
            let _ = writeln!(f, "{}", n);
        }
    }
    if let Ok(mut f) = fs::File::create(format!("{}_newly_added.tsv", out_prefix)) {
        writeln!(f, "file\tbest_new_mode\told_class\told_mode").ok();
        for (file, bm, oc, om) in &newly_added {
            writeln!(f, "{}\t{}\t{}\t{}", file, bm, oc, om).ok();
        }
    }

    eprintln!("---");
    eprintln!("files: {} (errors {})", total, errors);
    for (idx, name) in mode_names.iter().enumerate() {
        eprintln!("{:20}: {}/{}", name, matched_lists[idx].len(), total);
    }
    eprintln!("baseline203 (既存10モード any)     : {}/{}", baseline203.len(), total);
    eprintln!("new_union (既存10 + 新規4 any)      : {}/{}", new_union.len(), total);
    eprintln!("増分 (new_union - baseline203)     : {}", new_union.len() as i64 - baseline203.len() as i64);
    eprintln!("---");
    eprintln!("新規一致ファイル ({}本):", newly_added.len());
    for (file, bm, oc, om) in &newly_added {
        eprintln!("  {:12} best_new_mode={:20} old_class={:15} old_mode={}", file, bm, oc, om);
    }
    eprintln!("---");
    // 旧分類ごとの内訳
    let mut by_old_class: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, oc, _) in &newly_added {
        *by_old_class.entry(oc.clone()).or_insert(0) += 1;
    }
    eprintln!("新規一致ファイルの旧分類内訳: {:?}", by_old_class);
    // (tail, del) クロス表 (new_union 全体、best_mode 基準)
    eprintln!("---");
    eprintln!("new_union 全体の (tail, del) best_mode クロス表:");
    let mut cross: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for idx in best_mode_idx_map.values() {
        *cross.entry((mode_tail[*idx], mode_del[*idx])).or_insert(0) += 1;
    }
    for (k, v) in &cross {
        eprintln!("  {:?}: {}", k, v);
    }

    ExitCode::SUCCESS
}
