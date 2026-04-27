//! 奥村 LZSS の枠組みを微調整した複数バリアントで feasibility を測定する。
//!
//! 検証する独立 3 軸:
//!   - threshold: Literal 許容の境界 (max_len <= threshold なら Literal OK)
//!   - init_match: 初期 ring (未書込み slot) を start 位置として許可するか
//!     true  = 標準奥村, false = 未書込み slot から始まるマッチ禁止 (extension は許す)
//!   - lazy: 「次位置の max_len > 現位置 max_len」なら Literal を選んでもよい
//!
//! lazy=true 時の挙動: greedy の代替として「lazy 選択した場合のみ Literal が許容される」
//! を加える。Match 出力時は依然 max_len 一致 + max_positions 帰属。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16];
    let payload_start = 0x18 + (color_count as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

#[derive(Copy, Clone, Debug)]
struct Variant {
    threshold: usize,
    init_match: bool,
    lazy: bool,
}

fn max_match_v(
    ring: &[u8; N],
    written: &[bool; N],
    r: usize,
    input_remaining: &[u8],
    init_match: bool,
) -> (usize, Vec<usize>) {
    let max_check_len = input_remaining.len().min(F);
    if max_check_len == 0 {
        return (0, Vec::new());
    }
    let virt = |idx: usize| -> u8 {
        let m = idx & (N - 1);
        let dist = (m + N - r) & (N - 1);
        if dist < max_check_len {
            input_remaining[dist]
        } else {
            ring[m]
        }
    };
    let is_valid_start = |idx: usize| -> bool {
        if init_match {
            return true;
        }
        written[idx & (N - 1)]
    };
    let is_valid_ext = |idx: usize| -> bool {
        if init_match {
            return true;
        }
        let m = idx & (N - 1);
        let dist = (m + N - r) & (N - 1);
        if dist < max_check_len {
            return true;
        }
        written[m]
    };
    let first_byte = input_remaining[0];
    let mut best_len = 0usize;
    let mut best_positions: Vec<usize> = Vec::new();
    for p in 0..N {
        if p == r {
            continue;
        }
        if !is_valid_start(p) {
            continue;
        }
        if virt(p) != first_byte {
            continue;
        }
        let mut len = 1usize;
        while len < max_check_len {
            if !is_valid_ext(p + len) {
                break;
            }
            if virt(p + len) == input_remaining[len] {
                len += 1;
            } else {
                break;
            }
        }
        if len > best_len {
            best_len = len;
            best_positions.clear();
            best_positions.push(p);
        } else if len == best_len && best_len > 0 {
            best_positions.push(p);
        }
    }
    (best_len, best_positions)
}

fn check_file_v(input: &[u8], leaf: &[LeafToken], v: Variant) -> bool {
    let mut ring = [0x20u8; N];
    let mut written = [false; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;

    for t in leaf.iter() {
        if input_pos >= input.len() {
            return false;
        }
        let remaining = &input[input_pos..];
        let (max_len, max_positions) = max_match_v(&ring, &written, r, remaining, v.init_match);

        match t {
            LeafToken::Literal(b) => {
                let lazy_ok = if v.lazy && max_len > v.threshold && input_pos + 1 < input.len() {
                    let mut ring2 = ring;
                    let mut written2 = written;
                    ring2[r] = *b;
                    written2[r] = true;
                    let r2 = (r + 1) & (N - 1);
                    let next_remaining = &input[input_pos + 1..];
                    let (next_max_len, _) =
                        max_match_v(&ring2, &written2, r2, next_remaining, v.init_match);
                    next_max_len > max_len
                } else {
                    false
                };
                if max_len > v.threshold && !lazy_ok {
                    return false;
                }
                if input[input_pos] != *b {
                    return false;
                }
                ring[r] = *b;
                written[r] = true;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
            LeafToken::Match { pos, len } => {
                let l = *len as usize;
                let p = *pos as usize;
                if l != max_len {
                    return false;
                }
                if !max_positions.contains(&p) {
                    return false;
                }
                for i in 0..l {
                    ring[(r + i) & (N - 1)] = input[input_pos + i];
                    written[(r + i) & (N - 1)] = true;
                }
                r = (r + l) & (N - 1);
                input_pos += l;
            }
        }
    }
    input_pos == input.len()
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: lf2_oku_variants <input_dir>");
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
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

    type Decoded = (String, Vec<u8>, Vec<LeafToken>);
    let mut decoded_files: Vec<Decoded> = Vec::new();
    for path in &files {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let (w, h, payload_start) = match parse_lf2(&data) {
            Some(x) => x,
            None => continue,
        };
        let dec = match decompress_to_tokens(&data[payload_start..], w, h) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if dec.tokens.is_empty() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        decoded_files.push((label, dec.ring_input, dec.tokens));
    }
    eprintln!("decoded files: {}", decoded_files.len());

    let mut variants: Vec<Variant> = Vec::new();
    for &threshold in &[2usize, 3] {
        for &init_match in &[true, false] {
            for &lazy in &[false, true] {
                variants.push(Variant {
                    threshold,
                    init_match,
                    lazy,
                });
            }
        }
    }

    println!();
    println!("=== variant feasibility table ===");
    println!(
        "{:<8} {:<12} {:<6} {:>10} {:>8}",
        "thresh", "init_match", "lazy", "feasible", "%"
    );
    let total = decoded_files.len() as f64;
    for v in &variants {
        let mut feasible = 0u64;
        for (_, input, leaf) in &decoded_files {
            if check_file_v(input, leaf, *v) {
                feasible += 1;
            }
        }
        println!(
            "{:<8} {:<12} {:<6} {:>10} {:>7.2}%",
            v.threshold,
            v.init_match,
            v.lazy,
            feasible,
            feasible as f64 * 100.0 / total
        );
    }
    ExitCode::SUCCESS
}
