//! Stage 14-6 (Issue #14 脈: ⑲続々 奥村EOFドレインループの忠実再現)。
//!
//! 司令塔仮説「InsertNode は EOF 近傍で縮んでいく実効長 (`f_bound`) で
//! 挿入される」を、残る未解決 13 本に対して search_extra を掃引しながら
//! 直接検証する。2つの観点:
//!
//! (A) 生の勝者 (`raw_pos`、タイ再選定なしの素の BST 探索結果) が
//!     search_extra のいずれかの値で leaf_pos と一致するか (13/13 なら
//!     「原典を正しく写せば出る」で完結する)
//! (B) f_bound 縮小後の in_tree 候補集合でブロック分割したとき、
//!     Stage 14-4/14-5 の block_end-1 不変条件がなお成立するか、また
//!     ブロック数・正解ブロックの単純基準 (size/dist/write_tick) 順位が
//!     search_extra によって変わるか
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_fbound_retie_probe, TaxBase,
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

const SEARCH_EXTRA_RANGE: std::ops::RangeInclusive<i32> = -3..=15;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);

    println!("=== (A) raw_pos一致サーチ (search_extra を -3..=5 で掃引) ===");
    println!("name,search_extra,r,len,raw_pos,leaf_pos,raw_pos_eq_leaf_pos");

    let mut any_raw_hit: Vec<(String, i32)> = Vec::new();

    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (_w, _h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(
            &data[ps..],
            _w,
            _h,
        )
        .unwrap();
        let n = leaf.tokens.len();
        let leaf_last = leaf.tokens[n - 1];
        let (leaf_pos, _leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        for search_extra in SEARCH_EXTRA_RANGE {
            let dump =
                compress_okumura_eof_fbound_retie_probe(&leaf.ring_input, *base, search_extra);
            let (r, len, raw_pos, _cands) = match dump {
                Some(d) => d,
                None => continue,
            };
            let hit = raw_pos == leaf_pos;
            if hit {
                any_raw_hit.push((name.to_string(), search_extra));
            }
            println!(
                "{},{},{},{},{},{},{}",
                name, search_extra, r, len, raw_pos, leaf_pos, hit
            );
        }
    }

    println!();
    println!("=== (A) まとめ: raw_pos が leaf_pos に一致した (name, search_extra) ===");
    if any_raw_hit.is_empty() {
        println!("(なし。0/{} で raw_pos 一致なし)", FILES.len());
    } else {
        for (name, se) in &any_raw_hit {
            println!("{} search_extra={}", name, se);
        }
    }

    println!();
    println!("=== (B) in_tree ブロック分割 (search_extra を -3..=5 で掃引) ===");
    println!(
        "name,search_extra,n_in_tree,n_blocks,leaf_in_tree,leaf_block_start,leaf_block_end,\
block_size,is_block_end_minus1,leaf_rank_by_size_desc,leaf_rank_by_dist_asc,leaf_rank_by_tick_desc"
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

        for search_extra in SEARCH_EXTRA_RANGE {
            let dump =
                compress_okumura_eof_fbound_retie_probe(&leaf.ring_input, *base, search_extra);
            let (r, _len, _raw_pos, cands) = match dump {
                Some(d) => d,
                None => continue,
            };

            let in_tree: Vec<_> = cands.iter().filter(|c| c.in_tree).collect();
            let mut positions: Vec<i32> = in_tree.iter().map(|c| c.pos).collect();
            positions.sort_unstable();
            if positions.is_empty() {
                println!("{},{},0,0,false,,,,,,,", name, search_extra);
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

            let leaf_in_tree = positions.contains(&leaf_pos);
            let leaf_block = blocks.iter().find(|(s, e)| leaf_pos >= *s && leaf_pos <= *e);

            match leaf_block {
                Some((bs, be)) => {
                    let is_end_minus1 = leaf_pos == be - 1;

                    let mut by_size: Vec<(i32, i32)> = blocks.clone();
                    by_size.sort_by_key(|(s, e)| -(e - s + 1));
                    let rank_size = by_size.iter().position(|(s, e)| s == bs && e == be);

                    let mask = 4095i32;
                    let dist_of = |p: i32| (r - p) & mask;
                    let mut by_dist: Vec<(i32, i32, i32)> = blocks
                        .iter()
                        .map(|(s, e)| {
                            let d = (*s..=*e).map(|p| dist_of(p)).min().unwrap();
                            (*s, *e, d)
                        })
                        .collect();
                    by_dist.sort_by_key(|(_, _, d)| *d);
                    let rank_dist = by_dist.iter().position(|(s, e, _)| s == bs && e == be);

                    let mut by_tick: Vec<(i32, i32, u32)> = blocks
                        .iter()
                        .map(|(s, e)| {
                            let mt = in_tree
                                .iter()
                                .filter(|c| c.pos >= *s && c.pos <= *e)
                                .map(|c| c.write_tick)
                                .max()
                                .unwrap_or(0);
                            (*s, *e, mt)
                        })
                        .collect();
                    by_tick.sort_by_key(|(_, _, t)| std::cmp::Reverse(*t));
                    let rank_tick = by_tick.iter().position(|(s, e, _)| s == bs && e == be);

                    println!(
                        "{},{},{},{},{},{},{},{},{},{},{},{}",
                        name,
                        search_extra,
                        positions.len(),
                        blocks.len(),
                        leaf_in_tree,
                        bs,
                        be,
                        be - bs + 1,
                        is_end_minus1,
                        rank_size.map(|v| v.to_string()).unwrap_or_default(),
                        rank_dist.map(|v| v.to_string()).unwrap_or_default(),
                        rank_tick.map(|v| v.to_string()).unwrap_or_default(),
                    );
                }
                None => {
                    println!(
                        "{},{},{},{},false,,,,,,,",
                        name,
                        search_extra,
                        positions.len(),
                        blocks.len()
                    );
                }
            }
        }
    }
}
