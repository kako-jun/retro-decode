//! Stage 14-6 (Issue #14 脈: ⑲続々 奥村EOFドレインループの忠実再現)。
//!
//! 司令塔仮説の直接実装 (`compress_okumura_eof_search_bound`、Stage 14-3
//! で実装済みの `f_bound` 縮小 variant) を 522 本全件に適用し、現行
//! union268 (`.local_data/stage12_18/union_all.txt`) への純増・退行の
//! 有無を確認する。Stage 14-3 は24本の near-miss taxonomy に対してのみ
//! (勝者が変わるか) を見ており、522本全件の byte-exact union 評価は
//! 行っていなかった。
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_eof_search_bound, TaxBase};

fn frame_payload(tokens: &[retro_decode::formats::toheart::okumura_lzss::Token]) -> Vec<u8> {
    use retro_decode::formats::toheart::okumura_lzss::Token;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let union_path = PathBuf::from(&args[2]);

    let union_existing: BTreeSet<String> = fs::read_to_string(&union_path)
        .unwrap()
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    eprintln!("existing union: {} files", union_existing.len());

    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    let bases = [TaxBase::Basic, TaxBase::NoDummy, TaxBase::Fill00];
    let search_extras = [0i32, 1, 2, 3];

    let mut net_new: Vec<(String, Vec<String>)> = Vec::new();
    let mut regressions: Vec<(String, Vec<String>)> = Vec::new();
    let mut total_hits_any = 0u64;

    for p in &paths {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap().to_string();
        let data = fs::read(p).unwrap();
        if data.len() < 0x18 || &data[0..8] != b"LEAF256\0" {
            continue;
        }
        let w = u16::from_le_bytes([data[12], data[13]]);
        let h = u16::from_le_bytes([data[14], data[15]]);
        let cc = data[0x16] as usize;
        let ps = 0x18 + cc * 3;
        let dec = match retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(
            &data[ps..],
            w,
            h,
        ) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("WARN decode {}: {}", name, e);
                continue;
            }
        };
        let original_payload = &data[ps..];

        let mut hit_variants = Vec::new();
        for base in bases {
            for extra in search_extras {
                let toks = compress_okumura_eof_search_bound(&dec.ring_input, base, extra);
                let payload = frame_payload(&toks);
                if payload == *original_payload {
                    hit_variants.push(format!("{:?}_extra{}", base, extra));
                }
            }
        }

        if !hit_variants.is_empty() {
            total_hits_any += 1;
            if !union_existing.contains(&name) {
                net_new.push((name, hit_variants));
            }
        } else if union_existing.contains(&name) {
            // 既存 union268 所属ファイルが、この variant 群では一致しなかった
            // ケース (regression チェック用の記録。既存 union は他 variant
            // 由来なので regression にはならないが、参考情報として残す)。
            regressions.push((name, vec!["no hit under any f_bound variant".to_string()]));
        }
    }

    println!("files hit by >=1 f_bound variant (of 522): {}", total_hits_any);
    println!(
        "net NEW files (not already in union268): {}",
        net_new.len()
    );
    for (name, vs) in &net_new {
        println!("  {} -> {:?}", name, vs);
    }
    println!(
        "union268 files NOT hit by any f_bound variant (informational, not a regression): {}",
        regressions.len()
    );
}
