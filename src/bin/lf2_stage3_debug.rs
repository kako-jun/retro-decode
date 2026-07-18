//! Stage 3 デバッグ: 新 variant のトークン列と Leaf 実トークン列の最初の相違点を
//! 特定し、その時点の候補集合・age・tie 状況を表示する (Issue #14)。
//!
//! usage: cargo run --release --bin lf2_stage3_debug -- <FILE.LF2>

use std::env;
use std::fs;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_rank1_minage, Token, F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

/// 1 ファイルの最初の相違を分類して 1 行出力する。
/// class:
///   MATCH        = トークン列完全一致
///   TIE_SUBF     = max_len<F の tie (leaf/mine とも max_len 候補) → rank-1 規則の外れ
///   TIE_F        = max_len==F の tie で相違 → min-age 実装のバグ疑い
///   LEN_DIFF     = leaf len != mine len (greedy/hopeless/tail 系)
///   KIND_DIFF    = Literal vs Match の種別相違
///   LEAF_NOT_CAND= leaf の (pos,len) が列挙候補に無い (hopeless/tail overrun)
fn summarize(path: &std::path::Path) {
    let name = path.file_name().unwrap().to_str().unwrap();
    let data = fs::read(path).unwrap();
    if &data[0..8] != LF2_MAGIC {
        println!("{},PARSE_FAIL,,", name);
        return;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let ps = 0x18 + (data[0x16] as usize) * 3;
    let decoded = match decompress_to_tokens(&data[ps..], width, height) {
        Ok(d) => d,
        Err(_) => {
            println!("{},DECODE_FAIL,,", name);
            return;
        }
    };
    let input = &decoded.ring_input;
    let mine = compress_okumura_rank1_minage(input);

    let mut diff_idx = None;
    for (i, (a, b)) in decoded.tokens.iter().zip(mine.iter()).enumerate() {
        let same = match (a, b) {
            (LeafToken::Literal(x), Token::Literal(y)) => x == y,
            (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                p1 == p2 && l1 == l2
            }
            _ => false,
        };
        if !same {
            diff_idx = Some(i);
            break;
        }
    }
    let Some(di) = diff_idx else {
        println!("{},MATCH,{},", name, decoded.tokens.len());
        return;
    };

    // teacher forcing 再現
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for tok in decoded.tokens.iter().take(di) {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            ring[r] = input[input_pos];
            r = (r + 1) & (N - 1);
            input_pos += 1;
        }
    }
    let candidates = enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);

    let class = match (&decoded.tokens[di], &mine[di]) {
        (LeafToken::Match { pos: lp, len: ll }, Token::Match { len: ml, .. }) => {
            let leaf_in = candidates.iter().any(|c| c.pos == *lp && c.len == *ll);
            if !leaf_in {
                "LEAF_NOT_CAND"
            } else if ll != ml {
                "LEN_DIFF"
            } else if *ll as usize == F {
                "TIE_F"
            } else if *ll == max_len {
                "TIE_SUBF"
            } else {
                "LEN_DIFF"
            }
        }
        (LeafToken::Literal(_), Token::Literal(_)) => "KIND_DIFF", // 値違いは無いはず
        _ => "KIND_DIFF",
    };
    println!(
        "{},{},{},{}",
        name,
        class,
        di,
        input_pos
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args[1] == "--summary" {
        // dir 内全ファイルの最初の相違を1行分類で出す
        let mut files: Vec<_> = fs::read_dir(&args[2])
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("LF2"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        if let Some(n) = args.get(3).and_then(|v| v.parse::<usize>().ok()) {
            files.truncate(n);
        }
        for f in files {
            summarize(&f);
        }
        return ExitCode::SUCCESS;
    }
    if args[1] == "--vsbasic" {
        // 各ファイルで Basic とのトークン相違数 (= override が実際に選択を
        // 変えた箇所数の下限) を集計する
        let mut files: Vec<_> = fs::read_dir(&args[2])
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("LF2"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        if let Some(n) = args.get(3).and_then(|v| v.parse::<usize>().ok()) {
            files.truncate(n);
        }
        let mut files_diff = 0usize;
        let mut total = 0usize;
        for f in &files {
            let data = fs::read(f).unwrap();
            if &data[0..8] != LF2_MAGIC {
                continue;
            }
            let width = u16::from_le_bytes([data[12], data[13]]);
            let height = u16::from_le_bytes([data[14], data[15]]);
            let ps = 0x18 + (data[0x16] as usize) * 3;
            let Ok(decoded) = decompress_to_tokens(&data[ps..], width, height) else {
                continue;
            };
            total += 1;
            let a = retro_decode::formats::toheart::okumura_lzss::compress_okumura(
                &decoded.ring_input,
            );
            let b = compress_okumura_rank1_minage(&decoded.ring_input);
            let ndiff = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count()
                + a.len().abs_diff(b.len());
            if ndiff > 0 {
                files_diff += 1;
                println!("{},{}", f.file_name().unwrap().to_str().unwrap(), ndiff);
            }
        }
        eprintln!("files={} files_with_diff_vs_basic={}", total, files_diff);
        return ExitCode::SUCCESS;
    }
    let data = fs::read(&args[1]).expect("read");
    assert_eq!(&data[0..8], LF2_MAGIC);
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16] as usize;
    let ps = 0x18 + colors * 3;
    let decoded = decompress_to_tokens(&data[ps..], width, height).expect("decode");
    let input = &decoded.ring_input;

    let mine = compress_okumura_rank1_minage(input);

    // 最初の相違 token index
    let mut diff_idx = None;
    for (i, (a, b)) in decoded.tokens.iter().zip(mine.iter()).enumerate() {
        let same = match (a, b) {
            (LeafToken::Literal(x), Token::Literal(y)) => x == y,
            (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                p1 == p2 && l1 == l2
            }
            _ => false,
        };
        if !same {
            diff_idx = Some(i);
            break;
        }
    }
    let Some(di) = diff_idx else {
        println!("no token diff (len leaf={} mine={})", decoded.tokens.len(), mine.len());
        return ExitCode::SUCCESS;
    };

    // teacher forcing で di 直前まで shadow ring を再現
    let mut ring = [0x20u8; N];
    let mut write_tick = [u32::MAX; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for tok in decoded.tokens.iter().take(di) {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            ring[r] = input[input_pos];
            write_tick[r] = input_pos as u32;
            r = (r + 1) & (N - 1);
            input_pos += 1;
        }
    }

    let candidates = enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let n_max = candidates.iter().filter(|c| c.len == max_len).count();

    println!(
        "first diff at token {} (input_pos {}, ring r 0x{:03x})",
        di, input_pos, r
    );
    println!("  leaf: {:?}", decoded.tokens[di]);
    println!("  mine: {:?}", mine[di]);
    println!("  enumerated max_len={} n_max={}", max_len, n_max);
    for c in candidates.iter().filter(|c| c.len == max_len) {
        let ps2 = (c.pos as usize) & 0x0fff;
        let age = if write_tick[ps2] == u32::MAX {
            u32::MAX
        } else {
            (input_pos as u32).saturating_sub(write_tick[ps2])
        };
        let dist = (r + 0x1000 - c.pos as usize) & 0x0fff;
        let is_leaf = matches!(decoded.tokens[di], LeafToken::Match { pos, len } if pos == c.pos && len == c.len);
        let is_mine = matches!(mine[di], Token::Match { pos, len } if pos == c.pos && len == c.len);
        println!(
            "    cand pos=0x{:03x} len={} dist={} age={} {}{}",
            c.pos,
            c.len,
            dist,
            if age == u32::MAX { -1i64 } else { age as i64 },
            if is_leaf { "<= LEAF " } else { "" },
            if is_mine { "<= MINE" } else { "" },
        );
    }
    ExitCode::SUCCESS
}
