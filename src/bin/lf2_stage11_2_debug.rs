//! Stage 11-2 (Issue #14) 観測専用ツール。
//!
//! 49 本の KIND_DIFF divergence 点を「現位置の実書込み域候補 (L0_excl) の
//! 有無」で分類する:
//!   α: L0_excl >= 3 (実候補があるのに Leaf は literal) — 候補の pos/age/
//!      挿入時刻からの距離を出す
//!   β: L0_excl < 3 (実候補なし、正当な literal。Sim だけダミー系を拾った)
//!      — v1 (bootstrap帯reject) を当てたときの Sim の再選択をトレースする
//!
//! usage:
//!   cargo run --release --bin lf2_stage11_2_debug -- <DIR> <names.txt>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_clip_no_bootstrap_v1, compress_okumura_plus1_no_bootstrap_v1,
    compress_okumura_tail_plus1_traced, Token, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const MAX_LEN: usize = 18;
const BOOTSTRAP_DUMMY_LO: usize = N - 18 - 18; // 4060
const BOOTSTRAP_DUMMY_HI: usize = N - 18 - 1; // 4077

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

/// 勝ちモード (clip か plus1) も返す
fn pick_sim(ring_input: &[u8], orig_payload: &[u8]) -> (Vec<Token>, bool /* is_clip */) {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    if orig_payload == clip_reenc.as_slice() {
        (clip_tokens, true)
    } else {
        (compress_okumura_tail_plus1_traced(ring_input).0, false)
    }
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

/// brute-force で最長 match の (pos, len) を返す (実書込み域のみ、excl 規約)。
/// 複数候補が同着なら最小 pos を返す (単に決定的にするため)。
fn best_written_match(
    ring: &[u8; N],
    write_tick: &[u32; N],
    input: &[u8],
    s: usize,
) -> Option<(usize, usize)> {
    if s >= input.len() {
        return None;
    }
    let max_len_by_input = (input.len() - s).min(MAX_LEN);
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

fn build_snapshots(
    leaf_tokens: &[LeafToken],
    ring_input: &[u8],
) -> Vec<([u8; N], [u32; N], usize)> {
    let mut ring = [0x20u8; N];
    let mut write_tick = [u32::MAX; N];
    let mut r: usize = N - 18;
    let mut input_pos: usize = 0;
    let mut out = Vec::with_capacity(leaf_tokens.len());

    for tok in leaf_tokens {
        out.push((ring, write_tick, input_pos));
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
    out
}

fn find_divergence(leaf_tokens: &[LeafToken], sim_tokens: &[Token]) -> Option<usize> {
    for (i, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
        let same = match (a, b) {
            (LeafToken::Literal(x), Token::Literal(y)) => x == y,
            (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                p1 == p2 && l1 == l2
            }
            _ => false,
        };
        if !same {
            return Some(i);
        }
    }
    None
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <DIR> <names.txt>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let names: Vec<String> = fs::read_to_string(&args[2])
        .expect("read names")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    println!(
        "file,group,div_ti,input_pos,l0_excl,cand_pos,cand_age,cand_dist_to_div,v1_base,v1_token_kind,v1_token_len,v1_token_pos,v1_in_band,v1_class,second_div"
    );

    for name in &names {
        let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
            println!("{},PARSE_OR_DECODE_FAIL,-,-,-,-,-,-,-,-,-,-,-,-", name);
            continue;
        };
        let (sim_tokens, is_clip) = pick_sim(&ring_input, &orig_payload);
        let Some(di) = find_divergence(&leaf_tokens, &sim_tokens) else {
            println!("{},NO_DIFF,-,-,-,-,-,-,-,-,-,-,-,-", name);
            continue;
        };

        let snapshots = build_snapshots(&leaf_tokens, &ring_input);
        let (ring, write_tick, input_pos) = &snapshots[di];

        let best = best_written_match(ring, write_tick, &ring_input, *input_pos);
        let (l0_excl, cand_pos, cand_age, cand_dist): (usize, i64, i64, i64) = match best {
            Some((pos, len)) => {
                let wt = write_tick[pos];
                let age = if wt == u32::MAX {
                    -1
                } else {
                    (*input_pos as u32).saturating_sub(wt) as i64
                };
                let dist = if wt == u32::MAX {
                    -1
                } else {
                    (*input_pos as i64) - (wt as i64)
                };
                (len, pos as i64, age, dist)
            }
            None => (0, -1, -1, -1),
        };

        let group = if l0_excl >= 3 { "alpha" } else { "beta" };

        if group == "alpha" {
            println!(
                "{},alpha,{},{},{},{},{},{},-,-,-,-,-,-,-",
                name, di, input_pos, l0_excl, cand_pos, cand_age, cand_dist
            );
            continue;
        }

        // beta: v1 (bootstrap帯reject) を当てて同じ div_ti で Sim が何を選ぶか
        let v1_tokens = if is_clip {
            compress_okumura_clip_no_bootstrap_v1(&ring_input)
        } else {
            compress_okumura_plus1_no_bootstrap_v1(&ring_input)
        };
        let (v1_kind, v1_len, v1_pos): (&str, i64, i64) = match v1_tokens.get(di) {
            Some(Token::Literal(_)) => ("Literal", -1, -1),
            Some(Token::Match { pos, len }) => ("Match", *len as i64, *pos as i64),
            None => ("EOF", -1, -1),
        };
        let v1_in_band = v1_kind == "Match"
            && (v1_pos as usize & (N - 1)) >= BOOTSTRAP_DUMMY_LO
            && (v1_pos as usize & (N - 1)) <= BOOTSTRAP_DUMMY_HI;

        let leaf_kind = matches!(leaf_tokens[di], LeafToken::Literal(_));
        let v1_matches_leaf = leaf_kind && v1_kind == "Literal";

        let v1_class = if v1_matches_leaf {
            "saved_literal_matches_leaf"
        } else if v1_kind == "Match" && v1_in_band {
            "still_in_band_match(bug?)"
        } else if v1_kind == "Match" {
            "picked_different_real_or_dummy_elsewhere"
        } else {
            "other"
        };

        // v1_matches_leaf のとき、全ファイルで見た「v1 と leaf の最初の相違点」
        // (= 今回の di より後にあるはずの第二の divergence、無ければファイル
        // 全体が v1 で救われている) を確認する。
        let second_div = if v1_matches_leaf {
            find_divergence(&leaf_tokens, &v1_tokens)
        } else {
            Some(di)
        };
        let second_div_str = match second_div {
            None => "FULL_MATCH".to_string(),
            Some(d) if d == di => "SAME_POINT_STILL_DIFF".to_string(),
            Some(d) => d.to_string(),
        };

        println!(
            "{},beta,{},{},{},-,-,-,{},{},{},{},{},{},{}",
            name,
            di,
            input_pos,
            l0_excl,
            if is_clip { "clip" } else { "plus1" },
            v1_kind,
            v1_len,
            v1_pos,
            v1_in_band,
            v1_class,
            second_div_str
        );
    }

    ExitCode::SUCCESS
}
