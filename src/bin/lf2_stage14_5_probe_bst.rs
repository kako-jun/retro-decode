//! Stage 14-5 (Issue #14 脈: ⑲続 EOF巨大tie ブロック選択規則の特定) — 診断専用。
//!
//! 残る13本それぞれについて、EOF巨大tie発火時点の候補集合を
//! `compress_okumura_eof_retie_probe_stage14_5` で再列挙し、司令塔の
//! 2大仮説を直接検証する:
//!   1. 置換セマンティクス — 候補のうち実際に BST に生存 (`in_tree`) して
//!      いるものは何件か。leaf_pos は生存しているか。
//!   2. 拡張比較長 — 候補のうち len+1 バイト目まで実内容が一致
//!      (`survives_extended`) するものは何件か。leaf_pos は生き残るか。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_probe_stage14_5, TaxBase,
};
use retro_decode::formats::toheart::verify_harness;

const N: i32 = 4096;

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

    println!(
        "name,r,len,raw_pos,n_candidates,n_in_tree,leaf_pos,leaf_in_set,leaf_in_tree,\
n_survives_extended,leaf_survives_extended,n_in_tree_and_extended,leaf_in_tree_and_extended"
    );

    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let n_tok = leaf.tokens.len();
        let leaf_last = leaf.tokens[n_tok - 1];
        let (leaf_pos, _leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        let dump = compress_okumura_eof_retie_probe_stage14_5(&leaf.ring_input, *base);
        let (r, len, raw_pos, cands) = match dump {
            Some(d) => d,
            None => {
                println!("{}: NO_RETIE_FIRING", name);
                continue;
            }
        };

        let leaf_entry = cands.iter().find(|c| c.pos == leaf_pos);
        let n_in_tree = cands.iter().filter(|c| c.in_tree).count();
        let n_extended = cands.iter().filter(|c| c.survives_extended).count();
        let n_in_tree_ext = cands
            .iter()
            .filter(|c| c.in_tree && c.survives_extended)
            .count();

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            r,
            len,
            raw_pos,
            cands.len(),
            n_in_tree,
            leaf_pos,
            leaf_entry.is_some(),
            leaf_entry.map(|c| c.in_tree).unwrap_or(false),
            n_extended,
            leaf_entry.map(|c| c.survives_extended).unwrap_or(false),
            n_in_tree_ext,
            leaf_entry
                .map(|c| c.in_tree && c.survives_extended)
                .unwrap_or(false),
        );

        // 生存木内候補・拡張生存候補がどちらも十分小さければ、位置一覧を
        // stderr にダンプして目視できるようにする (block_end-1 との突合用)。
        if n_in_tree <= 10 {
            let mut in_tree_pos: Vec<i32> =
                cands.iter().filter(|c| c.in_tree).map(|c| c.pos).collect();
            in_tree_pos.sort_unstable();
            eprintln!(
                "  {} in_tree candidates ({}): {:?} (leaf_pos={}, r={})",
                name,
                in_tree_pos.len(),
                in_tree_pos,
                leaf_pos,
                r
            );
        }
        if n_extended <= 10 {
            let mut ext_pos: Vec<i32> = cands
                .iter()
                .filter(|c| c.survives_extended)
                .map(|c| c.pos)
                .collect();
            ext_pos.sort_unstable();
            eprintln!(
                "  {} survives_extended candidates ({}): {:?} (leaf_pos={}, r={})",
                name,
                ext_pos.len(),
                ext_pos,
                leaf_pos,
                r
            );
        }
        let _ = N;
        let _ = h;
    }
}
