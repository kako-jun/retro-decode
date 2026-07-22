//! Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則) — 診断専用。
//!
//! 残り17本それぞれについて、`compress_okumura_eof_retie_last_candidates`
//! で最終トークンのタイ候補集合を正しく (overlap 込みで) 再列挙し、実際の
//! Leaf 選択 (元ファイルの最終トークン) がその集合内でどんな統計的位置に
//! あるかを機械的に出力する。距離・絶対pos・write_tick に加え、画像幅
//! (width) を使った行方向の周期性仮説も検証する。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_last_candidates, TaxBase,
};
use retro_decode::formats::toheart::verify_harness;

const N: i32 = 4096;

fn consumed_len(tokens: &[LeafToken], up_to: usize) -> usize {
    tokens[..up_to]
        .iter()
        .map(|t| match t {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        })
        .sum()
}

const FILES: &[(&str, TaxBase)] = &[
    ("C0508.LF2", TaxBase::Basic),
    ("C0509.LF2", TaxBase::Basic),
    ("C050A.LF2", TaxBase::Basic),
    ("C0511.LF2", TaxBase::Basic),
    ("C0518.LF2", TaxBase::Basic),
    ("C0805.LF2", TaxBase::Fill00),
    ("C080D.LF2", TaxBase::Fill00),
    ("C1201.LF2", TaxBase::NoDummy),
    ("C1205.LF2", TaxBase::NoDummy),
    ("C1709.LF2", TaxBase::Basic),
    ("C1E03.LF2", TaxBase::Basic),
    ("C1E05.LF2", TaxBase::Basic),
    ("C1E06.LF2", TaxBase::Basic),
    ("C1E0A.LF2", TaxBase::Basic),
    ("C1E13.LF2", TaxBase::Basic),
    ("C1E16.LF2", TaxBase::Basic),
    ("C1E1A.LF2", TaxBase::Basic),
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);

    println!(
        "name,width,height,residual,leaf_pos,leaf_dist,n_candidates,rank_by_dist_asc,rank_by_dist_desc,\
rank_by_pos_asc,rank_by_pos_desc,rank_by_writetick_asc,rank_by_writetick_desc,\
median_dist,leaf_dist_minus_median,consumed,\
n_candidates_dist_multiple_of_width,leaf_dist_mod_width,leaf_dist_div_width,\
raw_pos,leaf_minus_raw_pos_wrapped,raw_pos_minus_1_in_set,raw_pos_plus_1_in_set,\
rank_by_dist_to_raw_pos_asc"
    );

    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let n = leaf.tokens.len();
        let consumed = consumed_len(&leaf.tokens, n - 1);
        let leaf_last = leaf.tokens[n - 1];
        let (leaf_pos, _leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        let dump = compress_okumura_eof_retie_last_candidates(&leaf.ring_input, *base);
        let (r, len, raw_pos, cands) = match dump {
            Some(d) => d,
            None => {
                println!("{}: NO_RETIE_FIRING", name);
                continue;
            }
        };

        let leaf_entry = cands.iter().find(|(p, _, _)| *p == leaf_pos);
        let leaf_dist = ((r - leaf_pos) & (N - 1)) as i32;

        let mut by_dist = cands.clone();
        by_dist.sort_by_key(|(_, d, _)| *d);
        let mut by_pos = cands.clone();
        by_pos.sort_by_key(|(p, _, _)| *p);
        let mut by_tick = cands.clone();
        by_tick.sort_by_key(|(_, _, t)| *t);

        let rank_dist_asc = by_dist.iter().position(|(p, _, _)| *p == leaf_pos);
        let rank_dist_desc = rank_dist_asc.map(|i| by_dist.len() - 1 - i);
        let rank_pos_asc = by_pos.iter().position(|(p, _, _)| *p == leaf_pos);
        let rank_pos_desc = rank_pos_asc.map(|i| by_pos.len() - 1 - i);
        let rank_tick_asc = by_tick.iter().position(|(p, _, _)| *p == leaf_pos);
        let rank_tick_desc = rank_tick_asc.map(|i| by_tick.len() - 1 - i);

        let median_dist = if !by_dist.is_empty() {
            by_dist[by_dist.len() / 2].1
        } else {
            0
        };

        let width = w as i32;
        let n_mult_width = cands.iter().filter(|(_, d, _)| d % width == 0).count();

        let fmt = |o: Option<usize>| o.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_string());

        // raw_pos (retie 発火前の生 BST 勝者) との関係。
        let wrap_diff = {
            let raw = ((leaf_pos - raw_pos + N / 2).rem_euclid(N)) - N / 2;
            raw
        };
        let pos_in_set = |target: i32| {
            let t = target.rem_euclid(N);
            cands.iter().any(|(p, _, _)| *p == t)
        };
        let raw_minus_1_in_set = pos_in_set(raw_pos - 1);
        let raw_plus_1_in_set = pos_in_set(raw_pos + 1);

        let dist_to_raw = |p: i32| -> i32 {
            let d = (p - raw_pos).rem_euclid(N);
            d.min(N - d)
        };
        let mut by_dist_raw: Vec<(i32, i32)> = cands.iter().map(|(p, _, _)| (*p, dist_to_raw(*p))).collect();
        by_dist_raw.sort_by_key(|(_, d)| *d);
        let rank_dist_raw_asc = by_dist_raw.iter().position(|(p, _)| *p == leaf_pos);

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            w,
            h,
            len,
            leaf_pos,
            leaf_dist,
            cands.len(),
            fmt(rank_dist_asc),
            fmt(rank_dist_desc),
            fmt(rank_pos_asc),
            fmt(rank_pos_desc),
            fmt(rank_tick_asc),
            fmt(rank_tick_desc),
            median_dist,
            leaf_dist - median_dist,
            consumed,
            n_mult_width,
            leaf_dist % width,
            leaf_dist / width,
            raw_pos,
            wrap_diff,
            raw_minus_1_in_set,
            raw_plus_1_in_set,
            fmt(rank_dist_raw_asc),
        );
        if leaf_entry.is_none() {
            eprintln!(
                "WARNING: {} leaf_pos={} not found among {} candidates (r={}, len={})",
                name,
                leaf_pos,
                cands.len(),
                r,
                len
            );
        }
    }
}
