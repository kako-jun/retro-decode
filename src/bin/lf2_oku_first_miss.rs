//! 各 LF2 ファイルの「奥村 greedy 違反」最初の token を詳細出力する診断ツール。
//!
//! `lf2_oku_feasibility` が 0/522 になった原因 (Literal selected but max_len > THRESHOLD)
//! を、ファイル別に「max_len, max_pos 数, leaf_token 種別, 直前 3 token, 直後の next_max_len」
//! まで観察し CSV 化する。
//!
//! 仮説判別の材料:
//!   - 全 miss で max_len が一定 (例: 3) なら Leaf min_match_len 違いの可能性
//!   - max_len が散らばり、かつ「次位置 next_max_len > max_len」のケースが多ければ lazy match 採用
//!   - どちらでもなければ、もっと別の隠れ条件 (距離、ring 状態) が支配

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn threshold() -> usize {
    std::env::var("OKU_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
}

fn init_match_allowed() -> bool {
    std::env::var("OKU_INIT_MATCH")
        .map(|s| s != "0")
        .unwrap_or(true)
}

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

fn naive_max_match(
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
    // start 位置: virtual region は不可 (まだ書いていない領域に "未来" 入力を映すのは反則)。
    // extension 位置: virtual region は許容 (self-extending RLE マッチを正しく扱う)。
    let is_valid_start = |idx: usize| -> bool {
        if init_match {
            return true;
        }
        let m = idx & (N - 1);
        written[m]
    };
    let is_valid_ext = |idx: usize| -> bool {
        if init_match {
            return true;
        }
        let m = idx & (N - 1);
        let dist = (m + N - r) & (N - 1);
        if dist < max_check_len {
            return true; // virtual extension OK
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

#[derive(Debug)]
struct MissRecord {
    label: String,
    fail_idx: usize,
    fail_kind: &'static str, // "Literal" or "Match"
    fail_max_len: usize,
    fail_max_positions: usize,
    leaf_match_len: usize, // 0 if Literal
    next_max_len: usize,   // greedy max at input_pos+1 (only if literal output, else 0)
    prev_kinds: String,    // e.g. "L,L,M(3,12)"
    input_pos: usize,
}

fn fmt_prev(prev: &[(char, usize, usize)]) -> String {
    prev.iter()
        .map(|(k, l, d)| {
            if *k == 'L' {
                "L".to_string()
            } else {
                format!("M(l={},d={})", l, d)
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn check_file(input: &[u8], leaf: &[LeafToken], label: &str) -> Option<MissRecord> {
    let thr = threshold();
    let init_match = init_match_allowed();
    let mut ring = [0x20u8; N];
    let mut written = [false; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    let mut prev: Vec<(char, usize, usize)> = Vec::new(); // kind, len, dist (dist=0 for L)

    for (idx, t) in leaf.iter().enumerate() {
        if input_pos >= input.len() {
            return Some(MissRecord {
                label: label.into(),
                fail_idx: idx,
                fail_kind: "InputExhausted",
                fail_max_len: 0,
                fail_max_positions: 0,
                leaf_match_len: 0,
                next_max_len: 0,
                prev_kinds: fmt_prev(&prev),
                input_pos,
            });
        }
        let remaining = &input[input_pos..];
        let (max_len, max_positions) = naive_max_match(&ring, &written, r, remaining, init_match);

        match t {
            LeafToken::Literal(b) => {
                if max_len > thr {
                    // 次位置の max_len も計算
                    let mut ring2 = ring;
                    let mut written2 = written;
                    ring2[r] = *b;
                    written2[r] = true;
                    let r2 = (r + 1) & (N - 1);
                    let next_remaining = if input_pos + 1 < input.len() {
                        &input[input_pos + 1..]
                    } else {
                        &input[input_pos..input_pos]
                    };
                    let (next_max_len, _) =
                        naive_max_match(&ring2, &written2, r2, next_remaining, init_match);
                    return Some(MissRecord {
                        label: label.into(),
                        fail_idx: idx,
                        fail_kind: "Literal",
                        fail_max_len: max_len,
                        fail_max_positions: max_positions.len(),
                        leaf_match_len: 0,
                        next_max_len,
                        prev_kinds: fmt_prev(&prev),
                        input_pos,
                    });
                }
                ring[r] = *b;
                written[r] = true;
                r = (r + 1) & (N - 1);
                input_pos += 1;
                prev.push(('L', 0, 0));
                if prev.len() > 3 {
                    prev.remove(0);
                }
            }
            LeafToken::Match { pos, len } => {
                let l = *len as usize;
                let p = *pos as usize;
                if l != max_len || !max_positions.contains(&p) {
                    return Some(MissRecord {
                        label: label.into(),
                        fail_idx: idx,
                        fail_kind: "Match",
                        fail_max_len: max_len,
                        fail_max_positions: max_positions.len(),
                        leaf_match_len: l,
                        next_max_len: 0,
                        prev_kinds: fmt_prev(&prev),
                        input_pos,
                    });
                }
                for i in 0..l {
                    ring[(r + i) & (N - 1)] = input[input_pos + i];
                    written[(r + i) & (N - 1)] = true;
                }
                r = (r + l) & (N - 1);
                input_pos += l;
                let dist = if p < r {
                    r - p
                } else {
                    r + N - p
                };
                prev.push(('M', l, dist));
                if prev.len() > 3 {
                    prev.remove(0);
                }
            }
        }
    }
    None
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: lf2_oku_first_miss <input_dir>");
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

    let csv_path = std::env::var("LF2_FIRST_MISS_CSV")
        .unwrap_or_else(|_| "/tmp/lf2_first_miss.csv".into());
    let mut csv_lines: Vec<String> = vec![
        "label,fail_idx,fail_kind,fail_max_len,fail_max_positions,leaf_match_len,next_max_len,gain_lazy,prev_kinds,input_pos".into(),
    ];

    let mut max_len_hist: std::collections::BTreeMap<usize, u64> = Default::default();
    let mut lazy_gain_hist: std::collections::BTreeMap<i64, u64> = Default::default();
    let mut kind_hist: std::collections::BTreeMap<&'static str, u64> = Default::default();
    let mut total_misses = 0u64;

    let total = files.len();
    for (i, path) in files.iter().enumerate() {
        if (i + 1) % 50 == 0 {
            eprintln!("progress: {}/{}", i + 1, total);
        }
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let (w, h, payload_start) = match parse_lf2(&data) {
            Some(x) => x,
            None => continue,
        };
        let decoded = match decompress_to_tokens(&data[payload_start..], w, h) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if decoded.tokens.is_empty() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        if let Some(rec) = check_file(&decoded.ring_input, &decoded.tokens, &label) {
            total_misses += 1;
            *max_len_hist.entry(rec.fail_max_len).or_insert(0) += 1;
            *kind_hist.entry(rec.fail_kind).or_insert(0) += 1;
            let gain = rec.next_max_len as i64 - rec.fail_max_len as i64;
            if rec.fail_kind == "Literal" {
                *lazy_gain_hist.entry(gain).or_insert(0) += 1;
            }
            csv_lines.push(format!(
                "{},{},{},{},{},{},{},{},{},{}",
                rec.label,
                rec.fail_idx,
                rec.fail_kind,
                rec.fail_max_len,
                rec.fail_max_positions,
                rec.leaf_match_len,
                rec.next_max_len,
                gain,
                rec.prev_kinds.replace(',', ";"),
                rec.input_pos,
            ));
        }
    }

    if let Ok(mut f) = fs::File::create(&csv_path) {
        use std::io::Write;
        for line in &csv_lines {
            let _ = writeln!(f, "{}", line);
        }
        eprintln!("CSV written to {}", csv_path);
    }

    println!();
    println!("=== first-miss diagnostics ===");
    println!("files with miss: {}/{}", total_misses, total);
    println!();
    println!("=== fail kind ===");
    for (k, v) in &kind_hist {
        println!("  {:<12} {}", k, v);
    }
    println!();
    println!("=== fail_max_len 分布 (全 miss) ===");
    for (k, v) in &max_len_hist {
        println!("  max_len={:<3} {}", k, v);
    }
    println!();
    println!("=== Literal miss の lazy gain (next_max_len - fail_max_len) ===");
    println!("  ※ gain >= 1 が多ければ lazy match 仮説支持");
    for (k, v) in &lazy_gain_hist {
        println!("  gain={:<+3} {}", k, v);
    }
    ExitCode::SUCCESS
}
