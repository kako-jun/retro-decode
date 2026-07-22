//! Stage 14-4 (Issue #14 脈: ⑲) — `AllowEq` 木再探索仮説の当たり判定プローブ。
//!
//! 24本 (Stage14-3 で確定済みの7本 + 未解決17本) それぞれについて、
//! `probe_eof_retree_allow_eq_last` で最終トークンの `StrictGt` 勝者 (現行既定)
//! と `AllowEq` 勝者を求め、実ファイルの Leaf 選択 pos と比較する。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{probe_eof_retree_allow_eq_last, TaxBase};
use retro_decode::formats::toheart::verify_harness;

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
    ("C0182.LF2", TaxBase::Basic),
    ("C0183.LF2", TaxBase::Basic),
    ("C040E.LF2", TaxBase::Basic),
    ("C040F.LF2", TaxBase::Basic),
    ("C0410.LF2", TaxBase::Basic),
    ("C0411.LF2", TaxBase::Basic),
    ("C1002.LF2", TaxBase::Basic),
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

    println!("name,base,leaf_pos,strict_gt_pos,allow_eq_pos,allow_eq_len,len,match_allow_eq");
    let mut hits = 0;
    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let n = leaf.tokens.len();
        let _consumed = consumed_len(&leaf.tokens, n - 1);
        let leaf_last = leaf.tokens[n - 1];
        let (leaf_pos, _leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        let probe = probe_eof_retree_allow_eq_last(&leaf.ring_input, *base);
        let (_r, len, strict_pos, ae_pos, ae_len) = match probe {
            Some(v) => v,
            None => {
                println!("{}: NO_FIRING", name);
                continue;
            }
        };
        let matched = ae_pos == leaf_pos;
        if matched {
            hits += 1;
        }
        println!(
            "{},{:?},{},{},{},{},{},{}",
            name, base, leaf_pos, strict_pos, ae_pos, ae_len, len, matched
        );
    }
    println!("# hits: {}/{}", hits, FILES.len());
}
