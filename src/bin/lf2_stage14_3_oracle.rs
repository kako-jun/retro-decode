//! Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討) — BST 非依存オラクル。
//!
//! ring 内容は「どのマッチを選ぶか」に依存せず、常に実入力バイト列だけで
//! 決まる (`slot = (r_init + k) mod N` に `input[k]` が書かれる)。この事実を
//! 使い、BST 探索を経由せず、全候補位置に対する最長一致を直接計算して
//! Leaf の選んだ (pos,len) が「有効な候補」かどうか、また我々の BST が
//! 見つけた候補と比べて何が違うかを機械的に確認する。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::verify_harness;

const N: usize = 4096;
const F: usize = 18;

fn consumed_len(tokens: &[LeafToken], up_to: usize) -> usize {
    tokens[..up_to]
        .iter()
        .map(|t| match t {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        })
        .sum()
}

const FILES: &[(&str, u8)] = &[
    ("C0182.LF2", 0x20),
    ("C0183.LF2", 0x20),
    ("C040E.LF2", 0x20),
    ("C040F.LF2", 0x20),
    ("C0410.LF2", 0x20),
    ("C0411.LF2", 0x20),
    ("C0508.LF2", 0x20),
    ("C0509.LF2", 0x20),
    ("C050A.LF2", 0x20),
    ("C0511.LF2", 0x20),
    ("C0518.LF2", 0x20),
    ("C0805.LF2", 0x00),
    ("C080D.LF2", 0x00),
    ("C1002.LF2", 0x20),
    ("C1201.LF2", 0x20),
    ("C1205.LF2", 0x20),
    ("C1709.LF2", 0x20),
    ("C1E03.LF2", 0x20),
    ("C1E05.LF2", 0x20),
    ("C1E06.LF2", 0x20),
    ("C1E0A.LF2", 0x20),
    ("C1E13.LF2", 0x20),
    ("C1E16.LF2", 0x20),
    ("C1E1A.LF2", 0x20),
];

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);

    for (name, fill) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let input_len = leaf.ring_input.len();
        let n = leaf.tokens.len();
        let leaf_last = leaf.tokens[n - 1];
        let consumed = consumed_len(&leaf.tokens, n - 1);
        let residual = input_len - consumed;

        let (leaf_pos, leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(_) => (-1, -1),
        };

        let r_init: i64 = (N - F) as i64;
        let r_final = ((r_init + consumed as i64) % N as i64) as usize;

        // `slot` に最後に書かれた実データの input index を返す (`consumed` バイト
        // 処理済み時点)。ring は N バイトごとに一周するため、consumed > N の
        // ファイルでは同じ slot が複数回上書きされている — 「最初の書込み」
        // ではなく「直近の書込み」(最大の k < consumed) を取る必要がある。
        let get_byte = |slot: usize| -> u8 {
            let base = ((slot as i64 - r_init).rem_euclid(N as i64)) as usize;
            if base >= consumed {
                return *fill;
            }
            let laps = if consumed > base {
                (consumed - 1 - base) / N
            } else {
                0
            };
            let k = base + laps * N;
            leaf.ring_input[k]
        };
        // 実データのみ (phantom を含めない、真の意味で有効な) 一致長。
        let real_match_len = |p: usize| -> usize {
            let mut j = 0usize;
            while j < residual {
                let a = leaf.ring_input[consumed + j];
                let b = get_byte((p + j) % N);
                if a != b {
                    break;
                }
                j += 1;
            }
            j
        };
        // phantom (ring残骸/fill) まで含めた F バイト一致長 (我々の BST の raw と同じ定義)。
        let phantom_match_len = |p: usize| -> usize {
            let mut j = 0usize;
            while j < F {
                let a = get_byte((r_final + j) % N);
                let b = get_byte((p + j) % N);
                if a != b {
                    break;
                }
                j += 1;
            }
            j
        };

        // 全候補 (既に書かれている = k < consumed のスロット) を走査。
        let mut best_real = (0usize, 0usize); // (len, pos) 最長優先・pos最小
        let mut best_phantom = (0usize, 0usize);
        let mut candidates_at_leaf_len_real: Vec<usize> = Vec::new();
        for p in 0..N {
            let k = ((p as i64 - r_init).rem_euclid(N as i64)) as usize;
            if k >= consumed {
                continue; // 未書込み
            }
            let rl = real_match_len(p);
            if rl > best_real.0 {
                best_real = (rl, p);
            }
            if rl == residual {
                candidates_at_leaf_len_real.push(p);
            }
            let pl = phantom_match_len(p);
            if pl > best_phantom.0 {
                best_phantom = (pl, p);
            }
        }

        let leaf_pos_u = leaf_pos as usize;
        let leaf_real_len_at_pos = if leaf_pos >= 0 {
            real_match_len(leaf_pos_u)
        } else {
            0
        };
        let leaf_phantom_len_at_pos = if leaf_pos >= 0 {
            phantom_match_len(leaf_pos_u)
        } else {
            0
        };

        // タイ集合中の距離統計 (r_final - p mod N)。
        let dist = |p: usize| -> usize { (r_final + N - p) % N };
        let dists: Vec<usize> = candidates_at_leaf_len_real.iter().map(|&p| dist(p)).collect();
        let min_dist = dists.iter().min().copied().unwrap_or(0);
        let max_dist = dists.iter().max().copied().unwrap_or(0);
        let leaf_dist = if leaf_pos >= 0 { dist(leaf_pos_u) } else { 0 };
        let min_pos = candidates_at_leaf_len_real.iter().min().copied().unwrap_or(0);
        let max_pos = candidates_at_leaf_len_real.iter().max().copied().unwrap_or(0);
        let n_at_min_dist = dists.iter().filter(|&&d| d == min_dist).count();
        let n_at_max_dist = dists.iter().filter(|&&d| d == max_dist).count();

        println!(
            "  tie_stats: min_dist={} max_dist={} leaf_dist={} min_pos={} max_pos={} leaf_pos={} n_at_min_dist={} n_at_max_dist={}",
            min_dist, max_dist, leaf_dist, min_pos, max_pos, leaf_pos_u, n_at_min_dist, n_at_max_dist
        );

        println!(
            "{}: residual={} leaf=(pos={},len={}) leaf_real_match_at_pos={} leaf_phantom_match_at_pos={} \
             best_real=(len={},pos={}) best_phantom=(len={},pos={}) n_candidates_at_residual_real={} \
             leaf_pos_in_residual_real_tie_set={}",
            name,
            residual,
            leaf_pos,
            leaf_len,
            leaf_real_len_at_pos,
            leaf_phantom_len_at_pos,
            best_real.0,
            best_real.1,
            best_phantom.0,
            best_phantom.1,
            candidates_at_leaf_len_real.len(),
            candidates_at_leaf_len_real.contains(&leaf_pos_u),
        );
    }
}
