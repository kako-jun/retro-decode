//! Stage 11-3 (Issue #14) 観測専用ツール。
//!
//! Stage 11-2 で見つかった「v1 が第一 divergence を解決した後にぶつかる
//! 第二 divergence」(35本) + C0205 (第一点のまま未解決) の計36本について、
//! v1 (per-file で Clip/Plus1 の良い方 + bootstrap帯reject) を適用した状態で
//! その target token index における Leaf/Sim の相違を Stage 10 と同じ観点で
//! 全数分類する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage11_3_debug -- <DIR> <targets.csv>
//!       (targets.csv: "file,target_ti" ヘッダ付き)
//!   cargo run --release --bin lf2_stage11_3_debug -- --dump <DIR> <FILE.LF2> <ti>
//!       (1点の詳細ダンプ; C120x 系の共通第二点用)

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_plus1_no_bootstrap_v1,
    compress_okumura_tail_plus1_traced, Token, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const BOOTSTRAP_DUMMY_LO: usize = N - F - F; // 4060
const BOOTSTRAP_DUMMY_HI: usize = N - F - 1; // 4077

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16];
    let payload_start = 0x18 + (colors as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn tokens_to_lf2_payload(tokens: &[Token]) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let flag_pos = compressed.len();
        compressed.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    compressed.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let encoded_pos = (pos as usize) & 0x0fff;
                    let encoded_len = ((len as usize) - 3) & 0x0f;
                    let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                    let lower = ((encoded_pos >> 4) & 0xff) as u8;
                    compressed.push(upper ^ 0xff);
                    compressed.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }
        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    compressed
}

fn pick_sim_v1(ring_input: &[u8], orig_payload: &[u8]) -> Vec<Token> {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    if orig_payload == clip_reenc.as_slice() {
        compress_okumura_clip_no_bootstrap_v1(ring_input)
    } else {
        compress_okumura_plus1_no_bootstrap_v1(ring_input)
    }
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

/// teacher forcing で target_ti 直前までの (ring, write_tick, r, input_pos) を作る。
fn build_state(
    leaf_tokens: &[LeafToken],
    ring_input: &[u8],
    target_ti: usize,
) -> ([u8; N], [u32; N], usize, usize) {
    let mut ring = [0x20u8; N];
    let mut write_tick = [u32::MAX; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;

    for tok in leaf_tokens.iter().take(target_ti) {
        match tok {
            LeafToken::Literal(_) => {
                if input_pos < ring_input.len() {
                    ring[r] = ring_input[input_pos];
                    write_tick[r] = input_pos as u32;
                    r = (r + 1) & (N - 1);
                    input_pos += 1;
                }
            }
            LeafToken::Match { pos, len } => {
                let pos = (*pos as usize) & (N - 1);
                let len = *len as usize;
                for k in 0..len {
                    if input_pos >= ring_input.len() {
                        break;
                    }
                    let src = (pos + k) & (N - 1);
                    let b = ring[src];
                    ring[r] = b;
                    write_tick[r] = input_pos as u32;
                    r = (r + 1) & (N - 1);
                    input_pos += 1;
                }
            }
        }
    }
    (ring, write_tick, r, input_pos)
}

fn best_written_match(
    ring: &[u8; N],
    write_tick: &[u32; N],
    input: &[u8],
    s: usize,
) -> Option<(usize, usize)> {
    if s >= input.len() {
        return None;
    }
    let max_len_by_input = (input.len() - s).min(F);
    if max_len_by_input == 0 {
        return None;
    }
    let mut best: Option<(usize, usize)> = None;
    for pos in 0..N {
        let mut l = 0usize;
        while l < max_len_by_input {
            let slot = (pos + l) & (N - 1);
            if write_tick[slot] == u32::MAX {
                break;
            }
            if ring[slot] != input[s + l] {
                break;
            }
            l += 1;
        }
        if l >= 3 {
            match best {
                Some((_, bl)) if bl >= l => {}
                _ => best = Some((pos, l)),
            }
        }
    }
    best
}

fn pos_class(pos: usize, write_tick: &[u32; N]) -> &'static str {
    let in_band = pos >= BOOTSTRAP_DUMMY_LO && pos <= BOOTSTRAP_DUMMY_HI;
    let written = write_tick[pos] != u32::MAX;
    match (in_band, written) {
        (true, _) => "bootstrap_band",
        (false, false) => "dummy_outside_band",
        (false, true) => "real_written",
    }
}

fn classify_one(dir: &PathBuf, name: &str, target_ti: usize) -> String {
    let Some((leaf_tokens, ring_input, orig_payload)) = load_file(dir, name) else {
        return format!("{},{},PARSE_OR_DECODE_FAIL", name, target_ti);
    };
    let sim_tokens = pick_sim_v1(&ring_input, &orig_payload);
    if target_ti >= leaf_tokens.len() || target_ti >= sim_tokens.len() {
        return format!("{},{},OUT_OF_RANGE", name, target_ti);
    }

    let (ring, write_tick, _r, input_pos) = build_state(&leaf_tokens, &ring_input, target_ti);

    let (leaf_kind, leaf_len, leaf_pos): (&str, i64, i64) = match &leaf_tokens[target_ti] {
        LeafToken::Literal(_) => ("Literal", -1, -1),
        LeafToken::Match { pos, len } => ("Match", *len as i64, *pos as i64),
    };
    let (sim_kind, sim_len, sim_pos): (&str, i64, i64) = match &sim_tokens[target_ti] {
        Token::Literal(_) => ("Literal", -1, -1),
        Token::Match { pos, len } => ("Match", *len as i64, *pos as i64),
    };

    let sim_pos_class = if sim_kind == "Match" {
        pos_class((sim_pos as usize) & (N - 1), &write_tick)
    } else {
        "-"
    };

    // Leaf 側の性格: literal 継続か別位置matchか。実書込み候補があるのに
    // literal なら alpha 型として記録。
    let best = best_written_match(&ring, &write_tick, &ring_input, input_pos);
    let (alpha_pos, alpha_len, alpha_age): (i64, i64, i64) = match best {
        Some((pos, len)) if leaf_kind == "Literal" => {
            let wt = write_tick[pos];
            let age = if wt == u32::MAX {
                -1
            } else {
                (input_pos as u32).saturating_sub(wt) as i64
            };
            (pos as i64, len as i64, age)
        }
        _ => (-1, -1, -1),
    };
    let is_alpha = leaf_kind == "Literal" && alpha_pos >= 0;

    // Stage10 スタイルの分類 (tie/len_diff/leaf_not_cand/kind_diff)
    let candidates =
        enumerate_match_candidates_with_writeback(&ring, &ring_input, input_pos, _r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let class = match (&leaf_tokens[target_ti], &sim_tokens[target_ti]) {
        (LeafToken::Match { pos: lp, len: ll }, Token::Match { len: ml, .. }) => {
            let leaf_in = candidates.iter().any(|c| c.pos == *lp && c.len == *ll);
            if !leaf_in {
                "LEAF_NOT_CAND"
            } else if ll != ml {
                "LEN_DIFF"
            } else if *ll as usize == F {
                "TIE_F"
            } else if *ll == max_len {
                "TIE_SUBF"
            } else {
                "LEN_DIFF"
            }
        }
        (LeafToken::Literal(_), Token::Literal(_)) => "KIND_DIFF_SAME_LITERAL(unexpected)",
        _ => "KIND_DIFF",
    };

    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        name,
        target_ti,
        input_pos,
        class,
        leaf_kind,
        leaf_len,
        leaf_pos,
        sim_kind,
        sim_len,
        sim_pos,
        sim_pos_class,
        is_alpha,
        format!("{}/{}/{}", alpha_pos, alpha_len, alpha_age)
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <DIR> <targets.csv> | --dump <DIR> <FILE.LF2> <ti>",
            args[0]
        );
        return ExitCode::from(2);
    }

    if args[1] == "--dump" {
        let dir = PathBuf::from(&args[2]);
        let name = &args[3];
        let ti: usize = args[4].parse().unwrap();

        let (leaf_tokens, ring_input, orig_payload) = load_file(&dir, name).expect("load");
        let sim_tokens = pick_sim_v1(&ring_input, &orig_payload);

        let (ring, write_tick, r, input_pos) = build_state(&leaf_tokens, &ring_input, ti);
        println!("=== dump {} ti={} input_pos={} r=0x{:03x} ===", name, ti, input_pos, r);
        println!("leaf token [ti]  : {:?}", leaf_tokens[ti]);
        println!("sim(v1) token[ti]: {:?}", sim_tokens[ti]);
        println!(
            "leaf token [ti-1]: {:?}",
            if ti > 0 { Some(&leaf_tokens[ti - 1]) } else { None }
        );
        println!(
            "sim(v1) token[ti-1]: {:?}",
            if ti > 0 { Some(&sim_tokens[ti - 1]) } else { None }
        );

        let candidates =
            enumerate_match_candidates_with_writeback(&ring, &ring_input, input_pos, r);
        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
        println!("max_len={} n_candidates={}", max_len, candidates.len());
        for c in candidates.iter().filter(|c| c.len == max_len) {
            let ps = (c.pos as usize) & (N - 1);
            let wt = write_tick[ps];
            let age = if wt == u32::MAX {
                -1i64
            } else {
                (input_pos as u32).saturating_sub(wt) as i64
            };
            let cls = pos_class(ps, &write_tick);
            println!(
                "  cand pos=0x{:03x} len={} age={} class={}",
                c.pos, c.len, age, cls
            );
        }

        let best = best_written_match(&ring, &write_tick, &ring_input, input_pos);
        println!("best_written_match (excl dummy) = {:?}", best);

        // 帯内候補の有無と v1 reject の実際の動き
        let band_candidates: Vec<_> = candidates
            .iter()
            .filter(|c| {
                let ps = (c.pos as usize) & (N - 1);
                ps >= BOOTSTRAP_DUMMY_LO && ps <= BOOTSTRAP_DUMMY_HI
            })
            .collect();
        println!("band candidates (any len): {:?}", band_candidates);

        return ExitCode::SUCCESS;
    }

    let dir = PathBuf::from(&args[1]);
    let targets_csv = fs::read_to_string(&args[2]).expect("read targets");
    println!(
        "file,target_ti,input_pos,class,leaf_kind,leaf_len,leaf_pos,sim_kind,sim_len,sim_pos,sim_pos_class,is_alpha,alpha_pos_len_age"
    );
    for line in targets_csv.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, ',');
        let name = parts.next().unwrap();
        let ti: usize = parts.next().unwrap().parse().unwrap();
        println!("{}", classify_one(&dir, name, ti));
    }

    ExitCode::SUCCESS
}
