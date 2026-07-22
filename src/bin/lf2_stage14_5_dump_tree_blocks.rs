//! Stage 14-5 (Issue #14 脈: ⑲続) — 診断専用。
//!
//! 指定ファイルの EOF巨大tie発火時点で、実際に BST に生存している候補
//! (`in_tree`) をブロック分割し、各ブロックの全属性 (start/end/size/
//! min-dist-to-r/max-write-tick) を一覧出力する。「正解ブロックがどれか」
//! を手作業で見極めるための生データダンプ。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_probe_stage14_5, TaxBase,
};
use retro_decode::formats::toheart::verify_harness;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let name = &args[2];
    let base = match args[3].as_str() {
        "basic" => TaxBase::Basic,
        "no_dummy" => TaxBase::NoDummy,
        "fill00" => TaxBase::Fill00,
        other => panic!("unknown base {other}"),
    };

    let path = dir.join(name);
    let data = fs::read(&path).unwrap();
    let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
    let leaf = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
        .unwrap();
    let n = leaf.tokens.len();
    let leaf_last = leaf.tokens[n - 1];
    let (leaf_pos, _leaf_len) = match leaf_last {
        LeafToken::Match { pos, len } => (pos as i32, len as i32),
        LeafToken::Literal(_) => (-1, -1),
    };

    let dump = compress_okumura_eof_retie_probe_stage14_5(&leaf.ring_input, base).unwrap();
    let (r, len, raw_pos, cands) = dump;
    println!(
        "# {} r={} len={} raw_pos={} leaf_pos={}",
        name, r, len, raw_pos, leaf_pos
    );

    let in_tree: Vec<_> = cands.iter().filter(|c| c.in_tree).collect();
    let mut positions: Vec<i32> = in_tree.iter().map(|c| c.pos).collect();
    positions.sort_unstable();

    let mut blocks: Vec<(i32, i32)> = Vec::new();
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

    let mask = 4095i32;
    println!("block_start,block_end,size,min_dist_to_r,max_write_tick,min_write_tick,contains_leaf,contains_raw_pos");
    for (bs, be) in &blocks {
        let members: Vec<_> = in_tree
            .iter()
            .filter(|c| c.pos >= *bs && c.pos <= *be)
            .collect();
        let min_dist = members.iter().map(|c| (r - c.pos) & mask).min().unwrap();
        let max_tick = members.iter().map(|c| c.write_tick).max().unwrap();
        let min_tick = members.iter().map(|c| c.write_tick).min().unwrap();
        let contains_leaf = leaf_pos >= *bs && leaf_pos <= *be;
        let contains_raw = raw_pos >= *bs && raw_pos <= *be;
        println!(
            "{},{},{},{},{},{},{},{}",
            bs,
            be,
            be - bs + 1,
            min_dist,
            max_tick,
            min_tick,
            contains_leaf,
            contains_raw
        );
    }
    let _ = h;
}
