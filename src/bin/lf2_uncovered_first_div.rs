//! 271 uncovered LF2 ファイルそれぞれの no_dummy_tail1 vs leaf 最初の divergence を分析。
//!
//! 出力: 各 uncovered ファイルの first divergence の (token_idx, leaf_token, ours_token,
//! leaf_len, ours_len, n_candidates_at_leaf_len) を集計。
//! 仮説検証: divergence が特定の cand_len 範囲に集中しているか？

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{
    self, Token,
};

const N: usize = 4096;
const F: usize = 18;
const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn token_eq(a: &Token, b: &LeafToken) -> bool {
    match (a, b) {
        (Token::Literal(x), LeafToken::Literal(y)) => x == y,
        (Token::Match { pos: pa, len: la }, LeafToken::Match { pos: pb, len: lb }) => {
            pa == pb && la == lb
        }
        _ => false,
    }
}

fn analyze_file(path: &std::path::Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w, h).ok()?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    // Use the *best* of no_dummy_tail1 and basic_tail1 (max first divergence).
    let nd = okumura_lzss::compress_okumura_no_dummy_tail1(input);
    let bs = okumura_lzss::compress_okumura_basic_tail1(input);
    let dt = okumura_lzss::compress_okumura_dummy_then_drop_tail1(input);
    let lf_first = okumura_lzss::compress_okumura_no_dummy_left_first_tail1(input);

    let first_div_for = |toks: &[Token], leaf: &[LeafToken]| -> usize {
        let n = toks.len().min(leaf.len());
        for i in 0..n {
            if !token_eq(&toks[i], &leaf[i]) {
                return i;
            }
        }
        n
    };

    let div_nd = first_div_for(&nd, leaf);
    let div_bs = first_div_for(&bs, leaf);
    let div_dt = first_div_for(&dt, leaf);
    let div_lf = first_div_for(&lf_first, leaf);

    let (_best_div, ours, best_name): (usize, &Vec<Token>, &str) = {
        let cands: [(usize, &Vec<Token>, &str); 4] = [
            (div_nd, &nd, "no_dummy"),
            (div_bs, &bs, "basic"),
            (div_dt, &dt, "dummy_drop"),
            (div_lf, &lf_first, "left_first"),
        ];
        let mut best: (usize, &Vec<Token>, &str) = cands[0];
        for c in &cands[1..] {
            if c.0 > best.0 {
                best = (c.0, c.1, c.2);
            }
        }
        best
    };

    let n = leaf.len().min(ours.len());
    let mut first_div: Option<usize> = None;
    for i in 0..n {
        if !token_eq(&ours[i], &leaf[i]) {
            first_div = Some(i);
            break;
        }
    }
    let div_idx = first_div?;
    let _ = best_name;

    // Compute n_candidates at leaf's max_len at this token
    let mut ring = [0x20u8; 0x1000];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for i in 0..div_idx {
        let l = match &leaf[i] {
            LeafToken::Literal(_) => 1,
            LeafToken::Match { len, .. } => *len as usize,
        };
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            let b = input[input_pos];
            ring[r] = b;
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }
    }

    let cands = enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);
    let max_len = cands.iter().map(|c| c.len).max().unwrap_or(0);
    let n_max = cands.iter().filter(|c| c.len == max_len).count();

    let leaf_t = match &leaf[div_idx] {
        LeafToken::Literal(b) => format!("Lit({})", b),
        LeafToken::Match { pos, len } => format!("M({},{})", pos, len),
    };
    let ours_t = match &ours[div_idx] {
        Token::Literal(b) => format!("Lit({})", b),
        Token::Match { pos, len } => format!("M({},{})", pos, len),
    };

    let leaf_len = match &leaf[div_idx] {
        LeafToken::Literal(_) => 1u32,
        LeafToken::Match { len, .. } => *len as u32,
    };

    let name = path.file_name()?.to_str()?.to_string();
    Some(format!(
        "{}\t{}\t{}\t{}\tleaf_len={}\tmax_avail_len={}\tn_max_cands={}\tbest_variant={}",
        name, div_idx, leaf_t, ours_t, leaf_len, max_len, n_max, best_name
    ))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    println!("file\ttok_idx\tleaf\tours\tleaf_len\tmax_avail_len\tn_max_cands");
    for p in &paths {
        if let Some(line) = analyze_file(p) {
            println!("{}", line);
        }
    }
    ExitCode::SUCCESS
}
