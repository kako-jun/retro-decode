//! Stage 14-5 (Issue #14 脈: ⑲続) — 診断専用。
//!
//! raw_pos を起点に木構造上の in-order 前任/後続方向へ辿り、leaf_pos に
//! 到達するまでの hop 数を調べる。「正解ノードは raw_pos の木構造上の
//! 近傍にいる」という構造仮説の検証。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_probe_inorder_neighbors, TaxBase,
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

    println!("name,r,raw_pos,leaf_pos,succ_hops_to_leaf,pred_hops_to_leaf,succ_chain_head,pred_chain_head");

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

        let dump = compress_okumura_eof_retie_probe_inorder_neighbors(&leaf.ring_input, *base, 500);
        let (r, _len, raw_pos, succ, pred) = match dump {
            Some(d) => d,
            None => {
                println!("{}: NO_RETIE_FIRING", name);
                continue;
            }
        };

        let succ_hop = succ.iter().position(|&p| p == leaf_pos).map(|i| i + 1);
        let pred_hop = pred.iter().position(|&p| p == leaf_pos).map(|i| i + 1);
        let fmt = |o: Option<usize>| o.map(|v| v.to_string()).unwrap_or_else(|| "NA".to_string());

        println!(
            "{},{},{},{},{},{},{:?},{:?}",
            name,
            r,
            raw_pos,
            leaf_pos,
            fmt(succ_hop),
            fmt(pred_hop),
            &succ[..succ.len().min(5)],
            &pred[..pred.len().min(5)],
        );
        let _ = h;
    }
}
