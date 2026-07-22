//! Stage 14-5 (Issue #14 脈: ⑲続 EOF巨大tie ブロック選択規則の特定)。
//!
//! `lf2_stage14_4_block_characterize` と同じブロック分割 (ring 絶対位置が
//! 連続する候補の塊) を、`written[]` 全候補ではなく**実際に BST に生存して
//! いる候補 (`in_tree`) だけ**に対して行う。仮説1 (置換セマンティクス) が
//! ブロック選定則そのものを単純化するかどうかを見る。
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

    println!(
        "name,leaf_pos,r,raw_pos,n_in_tree,n_blocks,block_start,block_end,block_size,\
offset_from_block_end,offset_from_block_start,block_contains_raw_pos,block_rank_by_size_desc,\
block_closest_to_r,leaf_dist_to_r,block_max_write_tick_is_leaf_block"
    );

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
        let (r, _len, raw_pos, cands) = match dump {
            Some(d) => d,
            None => continue,
        };

        let in_tree: Vec<_> = cands.iter().filter(|c| c.in_tree).collect();
        let mut positions: Vec<i32> = in_tree.iter().map(|c| c.pos).collect();
        positions.sort_unstable();
        if positions.is_empty() {
            println!("{}: no in_tree candidates (unexpected)", name);
            continue;
        }

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

        let leaf_block = blocks
            .iter()
            .find(|(s, e)| leaf_pos >= *s && leaf_pos <= *e);
        let raw_block_contains = blocks
            .iter()
            .any(|(s, e)| raw_pos >= *s && raw_pos <= *e && *s <= leaf_pos && leaf_pos <= *e);

        // ブロックをサイズ降順に並べたときの leaf_block の順位。
        let mut by_size: Vec<(i32, i32)> = blocks.clone();
        by_size.sort_by_key(|(s, e)| -(e - s + 1));
        let leaf_block_size_rank =
            leaf_block.and_then(|(bs, be)| by_size.iter().position(|(s, e)| s == bs && e == be));

        // r に最も近い (back distance 最小) ブロックが leaf のブロックか。
        let mask = 4095i32;
        let dist = |p: i32| (r - p) & mask;
        let mut by_closest: Vec<(i32, i32, i32)> = blocks
            .iter()
            .map(|(s, e)| {
                let d = blocks_min_dist(*s, *e, r, mask);
                (*s, *e, d)
            })
            .collect();
        by_closest.sort_by_key(|(_, _, d)| *d);
        let closest_block = by_closest.first().copied();
        let block_closest_to_r_is_leaf = match (leaf_block, closest_block) {
            (Some((ls, le)), Some((cs, ce, _))) => *ls == cs && *le == ce,
            _ => false,
        };

        // write_tick 最大のブロック (最も新しく書かれたノードを含むブロック) が leaf のブロックか。
        let mut block_max_tick: Vec<(i32, i32, u32)> = Vec::new();
        for (bs, be) in &blocks {
            let mt = in_tree
                .iter()
                .filter(|c| c.pos >= *bs && c.pos <= *be)
                .map(|c| c.write_tick)
                .max()
                .unwrap_or(0);
            block_max_tick.push((*bs, *be, mt));
        }
        block_max_tick.sort_by_key(|(_, _, t)| std::cmp::Reverse(*t));
        let newest_block = block_max_tick.first().copied();
        let leaf_block_is_newest = match (leaf_block, newest_block) {
            (Some((ls, le)), Some((ns, ne, _))) => *ls == ns && *le == ne,
            _ => false,
        };

        if let Some((bs, be)) = leaf_block {
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                name,
                leaf_pos,
                r,
                raw_pos,
                positions.len(),
                blocks.len(),
                bs,
                be,
                be - bs + 1,
                be - leaf_pos,
                leaf_pos - bs,
                raw_block_contains,
                leaf_block_size_rank
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "NA".into()),
                block_closest_to_r_is_leaf,
                dist(leaf_pos),
                leaf_block_is_newest,
            );
        } else {
            println!("{}: leaf_pos not found in any in_tree block", name);
        }
        let _ = w;
    }
}

fn blocks_min_dist(s: i32, e: i32, r: i32, mask: i32) -> i32 {
    // ブロック [s,e] 内で r に最も近い (back distance 最小) 位置までの距離。
    let mut best = i32::MAX;
    let mut p = s;
    while p <= e {
        let d = (r - p) & mask;
        if d < best {
            best = d;
        }
        p += 1;
    }
    best
}
