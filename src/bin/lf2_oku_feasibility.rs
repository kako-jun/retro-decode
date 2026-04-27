//! 奥村 LZSS 枠内で Leaf token 列が再現可能か検証する。
//!
//! 各ファイルの Leaf token 列を 1 つずつ追い、各位置で:
//!   1. 現 ring と入力で「最大マッチ長 max_len」と「max_len を達成する全 pos 集合」を線形スキャンで列挙
//!   2. Leaf token が Literal なら max_len <= THRESHOLD であるべき
//!   3. Leaf token が Match{pos, len} なら len == max_len かつ pos ∈ max_pos_set
//!
//! すべての token がこの条件を満たすファイル = 奥村 LZSS の greedy + 任意 tie 規則で
//! 原理的に再現可能。満たさない = どんな tie / dummy 配置でも届かない (奥村枠外)。
//!
//! このカウントが 224 と一致 = dummy 軸が真の天井 / 大きい = 別変種で届く可能性。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{F, N};

const THRESHOLD: usize = 2;
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

/// ring の現在状態と入力で「最長マッチ長 max_len」と「max_len を達成する全 pos」を返す。
///
/// 奥村と同等にするため、入力 F バイトを ring[r..r+F] に仮想 preload した状態で探索する。
/// これにより self-extending match (pos=r-1 で len=F の wrap マッチ等) が検出できる。
fn naive_max_match(ring: &[u8; N], r: usize, input_remaining: &[u8]) -> (usize, Vec<usize>) {
    let max_check_len = input_remaining.len().min(F);
    if max_check_len == 0 {
        return (0, Vec::new());
    }

    // ring に input を r から重ねた仮想 byte 取得関数
    let virt = |idx: usize| -> u8 {
        let m = idx & (N - 1);
        let dist = (m + N - r) & (N - 1);
        if dist < max_check_len {
            input_remaining[dist]
        } else {
            ring[m]
        }
    };

    let first_byte = input_remaining[0];
    let mut best_len = 0usize;
    let mut best_positions: Vec<usize> = Vec::new();

    for p in 0..N {
        if p == r {
            continue;
        }
        if virt(p) != first_byte {
            continue;
        }
        let mut len = 1usize;
        while len < max_check_len {
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

#[derive(Default, Debug, Clone)]
struct FileResult {
    feasible: bool,
    fail_token_idx: Option<usize>,
    fail_reason: Option<&'static str>,
}

fn check_file(input: &[u8], leaf: &[LeafToken]) -> FileResult {
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;

    for (idx, t) in leaf.iter().enumerate() {
        if input_pos >= input.len() {
            // leaf が入力長を越えてもまだ token を出している = decoder/encoder 矛盾
            return FileResult {
                feasible: false,
                fail_token_idx: Some(idx),
                fail_reason: Some("input exhausted"),
            };
        }
        let remaining = &input[input_pos..];
        let (max_len, max_positions) = naive_max_match(&ring, r, remaining);

        match t {
            LeafToken::Literal(b) => {
                // Literal を選ぶには「max_len <= THRESHOLD」が条件
                if max_len > THRESHOLD {
                    return FileResult {
                        feasible: false,
                        fail_token_idx: Some(idx),
                        fail_reason: Some("Literal selected but max_len > THRESHOLD"),
                    };
                }
                if input[input_pos] != *b {
                    return FileResult {
                        feasible: false,
                        fail_token_idx: Some(idx),
                        fail_reason: Some("Literal byte mismatch"),
                    };
                }
                ring[r] = *b;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
            LeafToken::Match { pos, len } => {
                let l = *len as usize;
                let p = *pos as usize;

                // greedy: len == max_len が必須
                if l != max_len {
                    return FileResult {
                        feasible: false,
                        fail_token_idx: Some(idx),
                        fail_reason: Some("Match len != max_len (not greedy)"),
                    };
                }
                // tie: pos が max_len 候補集合に含まれること
                if !max_positions.contains(&p) {
                    return FileResult {
                        feasible: false,
                        fail_token_idx: Some(idx),
                        fail_reason: Some("Match pos not in max_positions"),
                    };
                }
                // 中身検証 (defensive)
                for i in 0..l {
                    if ring[(p + i) & (N - 1)] != input[input_pos + i] {
                        return FileResult {
                            feasible: false,
                            fail_token_idx: Some(idx),
                            fail_reason: Some("Match content mismatch"),
                        };
                    }
                }
                // ring 進行
                for i in 0..l {
                    ring[(r + i) & (N - 1)] = input[input_pos + i];
                }
                r = (r + l) & (N - 1);
                input_pos += l;
            }
        }
    }

    if input_pos != input.len() {
        return FileResult {
            feasible: false,
            fail_token_idx: None,
            fail_reason: Some("input not fully consumed"),
        };
    }

    FileResult {
        feasible: true,
        fail_token_idx: None,
        fail_reason: None,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: lf2_oku_feasibility <input_dir>");
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

    let mut feasible: u64 = 0;
    let mut infeasible: u64 = 0;
    let mut errors: u64 = 0;
    let mut feasible_files: Vec<String> = Vec::new();
    let mut fail_reason_count: std::collections::BTreeMap<&'static str, u64> =
        std::collections::BTreeMap::new();
    let mut fail_offset_buckets: std::collections::BTreeMap<&'static str, u64> =
        std::collections::BTreeMap::new();

    let total = files.len();
    let mut processed = 0u64;
    for path in &files {
        processed += 1;
        if processed % 50 == 0 {
            eprintln!("progress: {}/{}", processed, total);
        }
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let (w, h, payload_start) = match parse_lf2(&data) {
            Some(x) => x,
            None => {
                errors += 1;
                continue;
            }
        };
        let decoded = match decompress_to_tokens(&data[payload_start..], w, h) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let leaf = decoded.tokens;
        let input = decoded.ring_input;

        if leaf.is_empty() {
            errors += 1;
            continue;
        }

        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();

        let res = check_file(&input, &leaf);
        if res.feasible {
            feasible += 1;
            feasible_files.push(label);
        } else {
            infeasible += 1;
            if let Some(reason) = res.fail_reason {
                *fail_reason_count.entry(reason).or_insert(0) += 1;
                let bucket = match res.fail_token_idx {
                    Some(i) if i < 5 => "early[0..5)",
                    Some(i) if i < 50 => "mid[5..50)",
                    Some(i) if i < 500 => "deep[50..500)",
                    Some(_) => "very_deep[500+)",
                    None => "no_idx",
                };
                *fail_offset_buckets.entry(bucket).or_insert(0) += 1;
            }
        }
    }

    println!();
    println!("=== 奥村 LZSS feasibility (greedy + 任意 tie) ===");
    println!(
        "feasible: {}/{} ({:.2}%)",
        feasible,
        total,
        feasible as f64 * 100.0 / total as f64
    );
    println!("infeasible: {}/{}  errors: {}", infeasible, total, errors);
    println!();
    println!("=== fail reason distribution ===");
    for (k, v) in &fail_reason_count {
        println!("  {:<50} {}", k, v);
    }
    println!();
    println!("=== fail offset bucket ===");
    for (k, v) in &fail_offset_buckets {
        println!("  {:<20} {}", k, v);
    }

    // CSV 出力
    let csv = std::env::var("LF2_FEASIBILITY_CSV")
        .unwrap_or_else(|_| "/tmp/lf2_feasibility.csv".into());
    if let Ok(mut f) = fs::File::create(&csv) {
        use std::io::Write;
        let _ = writeln!(f, "label,feasible");
        for path in &files {
            let label = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            let v = if feasible_files.contains(&label) {
                "1"
            } else {
                "0"
            };
            let _ = writeln!(f, "{},{}", label, v);
        }
        eprintln!("CSV written to {}", csv);
    }

    ExitCode::SUCCESS
}
