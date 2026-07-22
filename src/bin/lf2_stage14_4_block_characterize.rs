//! Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則)
//!
//! 残17本それぞれについて、タイ候補集合を「ring 上の絶対位置が連続する
//! ブロック」に分割し、Leaf が選んだ位置がどのブロックの何番目 (末尾から
//! 何個目) に当たるかを機械的に記録する。`.local_data/stage14_4/
//! block_characterization.csv` を出力する。
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
        "name,leaf_pos,n_blocks,block_start,block_end,block_size,\
offset_from_block_end,offset_from_block_start,block_contains_raw_pos,\
raw_pos,raw_pos_block_size"
    );

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

        let dump = compress_okumura_eof_retie_last_candidates(&leaf.ring_input, *base);
        let (_r, _len, raw_pos, cands) = match dump {
            Some(d) => d,
            None => continue,
        };

        let mut positions: Vec<i32> = cands.iter().map(|(p, _, _)| *p).collect();
        positions.sort_unstable();

        // 連続する ring 絶対位置 (gap==1) でブロック分割。
        let mut blocks: Vec<(i32, i32)> = Vec::new(); // (start, end) inclusive
        let mut start = positions[0];
        let mut prev = positions[0];
        for &p in &positions[1..] {
            if p == prev + 1 {
                prev = p;
            } else {
                blocks.push((start, prev));
                start = p;
                prev = p;
            }
        }
        blocks.push((start, prev));

        let leaf_block = blocks.iter().find(|(s, e)| leaf_pos >= *s && leaf_pos <= *e);
        let raw_block = blocks.iter().find(|(s, e)| raw_pos >= *s && raw_pos <= *e);

        if let Some((bs, be)) = leaf_block {
            let block_contains_raw = raw_pos >= *bs && raw_pos <= *be;
            println!(
                "{},{},{},{},{},{},{},{},{},{},{}",
                name,
                leaf_pos,
                blocks.len(),
                bs,
                be,
                be - bs + 1,
                be - leaf_pos,
                leaf_pos - bs,
                block_contains_raw,
                raw_pos,
                raw_block.map(|(s, e)| (e - s + 1).to_string()).unwrap_or_else(|| "NA".into()),
            );
        } else {
            println!("{}: leaf_pos not found in any block (unexpected)", name);
        }
        let _ = N;
    }
}
