//! Pairwise candidate dataset for LF2 ML approach (AI 路線 3 周目の前段).
//!
//! 各 LF2 ファイルを Leaf decode 出力で ring 駆動しながら、各 token 位置で
//! `enumerate_match_candidates_with_writeback` を呼んで全合法 (pos, len) 候補を
//! 取得し、最大長同点候補 (>= 2) の token について 1 行 1 候補で CSV 出力する。
//!
//! 各行は「この候補が Leaf 選択か (is_leaf=1/0)」のラベル付き。GBDT の binary
//! classification としてそのまま学習させられる。
//!
//! セッション 298 のコンテキスト: 奥村 LZSS の枠内 16+ 変種を尽くして天井 224
//! を確定。3 周目の AI 路線では「特徴量豊富化 + pairwise learning to rank」で
//! 19% 止まりだった過去の特徴量不足を解消する。

use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};

const N: usize = 4096;
const F: usize = 18;
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

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

/// 同最大長候補がこの数を超える token はスキップ。理由: ring 全体が同一バイトで
/// 埋まる初期段階等で 4000 候補が同点になる場合があり、出力サイズが爆発する一方で
/// 「全候補が等価」ゆえに弁別力ゼロ。これらは別経路 (集計 1 行) で扱う設計余地。
const N_MAX_CAP: usize = 32;

/// 1 ファイル分の pairwise 行を CSV に書き出す。返値は (出力行数, max-tie token 数, skipped-hugec token 数)。
fn process_file(
    label: &str,
    leaf: &[LeafToken],
    input: &[u8],
    out: &mut impl Write,
) -> std::io::Result<(u64, u64, u64)> {
    let mut ring = [0x20u8; 0x1000];
    let mut r: usize = N - F;

    let mut input_pos: usize = 0;
    let mut rows = 0u64;
    let mut tie_tokens = 0u64;
    let mut skipped = 0u64;

    let mut prev_kind: char = 'N'; // None
    let mut prev_len: u8 = 0;

    for (token_idx, tok) in leaf.iter().enumerate() {
        let candidates =
            enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);

        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
        let n_cands = candidates.len();
        let n_max = candidates.iter().filter(|c| c.len == max_len).count();

        let (leaf_kind, leaf_pos, leaf_len_v) = match tok {
            LeafToken::Literal(b) => ('L', *b as u32, 1u32),
            LeafToken::Match { pos, len } => ('M', *pos as u32, *len as u32),
        };

        // 学習対象: Leaf が Match を選び、かつ同最大長候補が 2..=N_MAX_CAP の token
        if leaf_kind == 'M' && n_max >= 2 && max_len >= 3 {
            if n_max > N_MAX_CAP {
                skipped += 1;
            } else {
                tie_tokens += 1;
                for c in candidates.iter().filter(|c| c.len == max_len) {
                    let dist = (r + 0x1000 - c.pos as usize) & 0x0fff;
                    let is_leaf = (leaf_pos as u16 == c.pos
                        && leaf_len_v as u8 == c.len)
                        as u8;
                    writeln!(
                        out,
                        "{},{},{},{},{},{},{},{},{},{},{},{}",
                        label,
                        token_idx,
                        r,
                        n_cands,
                        n_max,
                        max_len,
                        c.pos,
                        c.len,
                        dist,
                        is_leaf,
                        prev_kind,
                        prev_len,
                    )?;
                    rows += 1;
                }
            }
        }

        // ring を Leaf の実バイト列で進める (シミュレーション側の発散を排除し、
        // candidate 列挙が常に Leaf が見たであろう ring 状態に対するものになる)
        let l = token_len(tok);
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            let b = input[input_pos];
            ring[r] = b;
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }

        prev_kind = leaf_kind;
        prev_len = leaf_len_v as u8;
    }

    Ok((rows, tie_tokens, skipped))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lf2_pairwise_dataset <input_dir_or_file> [output_csv]");
        return ExitCode::from(2);
    }
    let path = PathBuf::from(&args[1]);
    let out_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/tmp/lf2_pairwise.csv".to_string());

    let files: Vec<PathBuf> = if path.is_dir() {
        let mut v: Vec<PathBuf> = match fs::read_dir(&path) {
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
                eprintln!("failed to read dir {:?}: {}", path, e);
                return ExitCode::from(1);
            }
        };
        v.sort();
        v
    } else {
        vec![path]
    };

    let f = match fs::File::create(&out_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to create {}: {}", out_path, e);
            return ExitCode::from(1);
        }
    };
    let mut writer = BufWriter::new(f);

    if let Err(e) = writeln!(
        writer,
        "file,token_idx,ring_r,n_cands,n_max,max_len,cand_pos,cand_len,cand_dist,is_leaf,prev_kind,prev_len"
    ) {
        eprintln!("write error: {}", e);
        return ExitCode::from(1);
    }

    let total = files.len();
    let mut processed = 0u64;
    let mut errors = 0u64;
    let mut total_rows = 0u64;
    let mut total_tie_tokens = 0u64;
    let mut total_skipped = 0u64;

    let start = Instant::now();
    for (i, fpath) in files.iter().enumerate() {
        let data = match fs::read(fpath) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let (w, h, ps) = match parse_lf2(&data) {
            Some(x) => x,
            None => {
                errors += 1;
                continue;
            }
        };
        let decoded = match decompress_to_tokens(&data[ps..], w, h) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let label = fpath
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        match process_file(&label, &decoded.tokens, &decoded.ring_input, &mut writer) {
            Ok((rows, ties, skipped)) => {
                total_rows += rows;
                total_tie_tokens += ties;
                total_skipped += skipped;
                processed += 1;
            }
            Err(e) => {
                eprintln!("write error on {}: {}", label, e);
                errors += 1;
            }
        }
        if (i + 1) % 50 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            eprintln!(
                "progress: {}/{} ({:.1}s elapsed, {} rows, {} tie, {} skipped_huge)",
                i + 1,
                total,
                elapsed,
                total_rows,
                total_tie_tokens,
                total_skipped
            );
        }
    }

    eprintln!(
        "done: processed={} errors={} total_rows={} tie_tokens={} skipped_huge={} elapsed={:.1}s csv={}",
        processed,
        errors,
        total_rows,
        total_tie_tokens,
        total_skipped,
        start.elapsed().as_secs_f64(),
        out_path
    );

    ExitCode::SUCCESS
}
