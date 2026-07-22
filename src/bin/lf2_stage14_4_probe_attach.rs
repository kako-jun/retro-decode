//! Stage 14-4 (Issue #14 脈: ⑲) — 構造上の木挿入位置 (`dad[r]`) 仮説の当たり判定。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{probe_eof_attach_point_last, TaxBase};
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

    println!("name,leaf_pos,strict_gt_pos,attach,attach_minus_1,attach_is_pseudo_root,match_attach,match_attach_minus_1");
    let mut hits_attach = 0;
    let mut hits_attach_m1 = 0;
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

        let probe = probe_eof_attach_point_last(&leaf.ring_input, *base);
        let (_r, _len, strict_pos, attach) = match probe {
            Some(v) => v,
            None => {
                println!("{}: NO_FIRING", name);
                continue;
            }
        };
        let is_pseudo = attach >= N;
        let attach_m1 = ((attach - 1) & (N - 1)) as i32;
        let match_attach = !is_pseudo && attach == leaf_pos;
        let match_attach_m1 = !is_pseudo && attach_m1 == leaf_pos;
        if match_attach {
            hits_attach += 1;
        }
        if match_attach_m1 {
            hits_attach_m1 += 1;
        }
        println!(
            "{},{},{},{},{},{},{},{}",
            name, leaf_pos, strict_pos, attach, attach_m1, is_pseudo, match_attach, match_attach_m1
        );
    }
    println!(
        "# hits attach={}/{}  attach-1={}/{}",
        hits_attach,
        FILES.len(),
        hits_attach_m1,
        FILES.len()
    );
}
