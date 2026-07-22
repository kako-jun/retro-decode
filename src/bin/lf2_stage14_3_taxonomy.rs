//! Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討)
//!
//! near-miss 台帳 (remaining_tokens=1) の24本それぞれについて、最終トークンの
//! 相違を機械的に分類する。`.local_data/stage14_3/final_token_taxonomy.csv` を出力。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie, compress_okumura_tax_trace, EofTieRule, TaxBase, Token,
};
use retro_decode::formats::toheart::verify_harness;

fn frame_payload(tokens: &[Token]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let flag_pos = out.len();
        out.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    out.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let p = (pos as usize) & 0x0fff;
                    let l = ((len as usize) - 3) & 0x0f;
                    let upper = (l | ((p & 0x0f) << 4)) as u8;
                    let lower = ((p >> 4) & 0xff) as u8;
                    out.push(upper ^ 0xff);
                    out.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }
        out[flag_pos] = flag_byte ^ 0xff;
    }
    out
}

/// (file, base) — Stage 14-2 near_miss_ledger.csv の best_variant 由来。
const FILES: &[(&str, TaxBase)] = &[
    ("C0182.LF2", TaxBase::Basic),
    ("C0183.LF2", TaxBase::Basic),
    ("C040E.LF2", TaxBase::Basic),
    ("C040F.LF2", TaxBase::Basic),
    ("C0410.LF2", TaxBase::Basic),
    ("C0411.LF2", TaxBase::Basic),
    ("C0508.LF2", TaxBase::Basic),
    ("C0509.LF2", TaxBase::Basic),
    ("C050A.LF2", TaxBase::Basic),
    ("C0511.LF2", TaxBase::Basic),
    ("C0518.LF2", TaxBase::Basic),
    ("C0805.LF2", TaxBase::Fill00),
    ("C080D.LF2", TaxBase::Fill00),
    ("C1002.LF2", TaxBase::Basic),
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

fn consumed_len(tokens: &[LeafToken], up_to: usize) -> usize {
    tokens[..up_to]
        .iter()
        .map(|t| match t {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        })
        .sum()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let out_path = PathBuf::from(&args[2]);

    let mut rows: Vec<String> = Vec::new();
    rows.push(
        "name,base,leaf_pos,leaf_len,gen_raw_pos,gen_raw_len,gen_capped_len,residual,pos_match,len_diff,eof_phantom,excess,window_r_matches_window_pos_for_excess,window_r_all_0x20,window_r_all_0x00,rle_adjacent,fixed_by_eof_tie_rule,byte_exact_after_fix,note"
            .to_string(),
    );

    for (name, base) in FILES {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
                .unwrap();
        let input_len = leaf.ring_input.len();
        let n = leaf.tokens.len();
        let leaf_last = leaf.tokens[n - 1];
        let consumed_before = consumed_len(&leaf.tokens, n - 1);
        let residual = input_len - consumed_before;

        let (gen_tokens, steps) = compress_okumura_tax_trace(&leaf.ring_input, *base);

        // gen 側の「最終ステップ」= len_residual == residual の steps エントリ
        // (EOF 近傍複数ステップが記録されているので、residual が一致するものを探す)。
        let step = steps.iter().find(|s| s.len_residual == residual);

        let (leaf_pos, leaf_len) = match leaf_last {
            LeafToken::Match { pos, len } => (pos as i32, len as i32),
            LeafToken::Literal(b) => (-1, -(b as i32) - 1), // Literal は pos=-1 で区別
        };

        let mut note = String::new();
        if step.is_none() {
            note.push_str("NO_STEP_FOUND;");
        }
        let (gen_raw_pos, gen_raw_len, gen_capped_len, pos_match, len_diff, eof_phantom, excess,
             excess_match, win_r_all20, win_r_all00) = if let Some(st) = step {
            let raw_pos_ring = (st.raw_pos as u16) & 0x0fff;
            let pos_match = leaf_pos >= 0 && raw_pos_ring as i32 == leaf_pos;
            let len_diff = if leaf_pos >= 0 { leaf_len - st.raw_len } else { i32::MIN };
            let eof_phantom = leaf_pos >= 0 && (leaf_len as usize) > residual;
            let excess = if leaf_pos >= 0 && (leaf_len as usize) > residual {
                (leaf_len as usize) - residual
            } else {
                0
            };
            // excess バイト (window_r[residual..leaf_len)) が window_pos の同じ
            // オフセットと一致するか (leaf の候補位置と gen の raw_pos が同じ場合のみ意味を持つ)
            let excess_match = if pos_match && excess > 0 {
                let lo = residual;
                let hi = (residual + excess).min(F_CONST);
                st.window_r[lo..hi] == st.window_pos[lo..hi]
            } else {
                false
            };
            let tail_region = &st.window_r[residual.min(F_CONST)..];
            let win_r_all20 = !tail_region.is_empty() && tail_region.iter().all(|&b| b == 0x20);
            let win_r_all00 = !tail_region.is_empty() && tail_region.iter().all(|&b| b == 0x00);
            (
                st.raw_pos,
                st.raw_len,
                st.capped_len,
                pos_match,
                len_diff,
                eof_phantom,
                excess,
                excess_match,
                win_r_all20,
                win_r_all00,
            )
        } else {
            (-1, -1, -1, false, i32::MIN, false, 0, false, false, false)
        };

        if gen_tokens.len() != n {
            note.push_str(&format!("gen_token_count={};", gen_tokens.len()));
        }

        // r-1 (直前に書かれたバイト) を候補位置とする RLE 隣接判定。
        let rle_adjacent = if let Some(st) = step {
            let r_minus_1 = ((st.r - 1 + 4096) & 4095) as u16;
            (leaf_pos >= 0) && (leaf_pos as u16 == r_minus_1)
        } else {
            false
        };

        // Stage 14-3 確定版 (ClosestDist/FarthestDist) で byte-exact 反転するか。
        let mut fixed_by = "none";
        let mut byte_exact = false;
        for (rule_name, rule) in [
            ("ClosestDist", EofTieRule::ClosestDist),
            ("FarthestDist", EofTieRule::FarthestDist),
        ] {
            let gen = compress_okumura_eof_retie(&leaf.ring_input, *base, rule);
            let payload = frame_payload(&gen);
            if payload == data[ps..] {
                fixed_by = rule_name;
                byte_exact = true;
                break;
            }
        }

        rows.push(format!(
            "{},{:?},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            base,
            leaf_pos,
            leaf_len,
            gen_raw_pos,
            gen_raw_len,
            gen_capped_len,
            residual,
            pos_match,
            len_diff,
            eof_phantom,
            excess,
            excess_match,
            win_r_all20,
            win_r_all00,
            rle_adjacent,
            fixed_by,
            byte_exact,
            note
        ));
    }

    fs::write(&out_path, rows.join("\n") + "\n").unwrap();
    eprintln!("wrote {}", out_path.display());
}

const F_CONST: usize = 18;
