//! Stage 14-5 (Issue #14 脈: ⑲続) — 診断専用。
//!
//! 実 BST 生存候補 (`in_tree`) だけに絞った母集団に対して、距離/pos/
//! write_tick の単純基準で leaf_pos の順位を再計算する (ブロック分割を
//! 経由しない、候補単位のランキング)。母集団が数千→数十に縮んだことで
//! 単純基準が機能するようになるかを見る。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_probe_stage14_5, TaxBase,
};
use retro_decode::formats::toheart::verify_harness;

const FILES: &[(&str, TaxBase)] = &[
    ("C0508.LF2", TaxBase::Basic),
    ("C0509.LF2", TaxBase::Basic),
    ("C050A.LF2", TaxBase::Basic),
    ("C0511.LF2", TaxBase::Basic),
    ("C0518.LF2", TaxBase::Basic),
    ("C1205.LF2", TaxBase::NoDummy),
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

    println!("name,n_in_tree,rank_dist_asc,rank_dist_desc,rank_pos_asc,rank_pos_desc,rank_tick_asc,rank_tick_desc");

    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let n = leaf.tokens.len();
        let leaf_last = leaf.tokens[n - 1];
        let (leaf_pos, _leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        let dump = compress_okumura_eof_retie_probe_stage14_5(&leaf.ring_input, *base);
        let (_r, _len, _raw_pos, cands) = match dump {
            Some(d) => d,
            None => continue,
        };
        let in_tree: Vec<_> = cands.iter().filter(|c| c.in_tree).collect();

        let mut by_dist = in_tree.clone();
        by_dist.sort_by_key(|c| c.dist);
        let mut by_pos = in_tree.clone();
        by_pos.sort_by_key(|c| c.pos);
        let mut by_tick = in_tree.clone();
        by_tick.sort_by_key(|c| c.write_tick);

        let rank_dist_asc = by_dist.iter().position(|c| c.pos == leaf_pos);
        let rank_dist_desc = rank_dist_asc.map(|i| by_dist.len() - 1 - i);
        let rank_pos_asc = by_pos.iter().position(|c| c.pos == leaf_pos);
        let rank_pos_desc = rank_pos_asc.map(|i| by_pos.len() - 1 - i);
        let rank_tick_asc = by_tick.iter().position(|c| c.pos == leaf_pos);
        let rank_tick_desc = rank_tick_asc.map(|i| by_tick.len() - 1 - i);
        let fmt = |o: Option<usize>| o.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_string());

        println!(
            "{},{},{},{},{},{},{},{}",
            name,
            in_tree.len(),
            fmt(rank_dist_asc),
            fmt(rank_dist_desc),
            fmt(rank_pos_asc),
            fmt(rank_pos_desc),
            fmt(rank_tick_asc),
            fmt(rank_tick_desc),
        );
        let _ = h;
    }
}
