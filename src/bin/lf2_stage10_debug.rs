//! Stage 10 偵察 (Issue #14): Stage 9-2d の per-file フォールバック勝ちモード
//! (Clip or Plus1) を Sim として使い、Leaf との最初の divergence を再分類する。
//!
//! Stage 9 (rank1_minage ベース) の KIND_DIFF は Sim が旧来の Basic 系だった
//! ため、Stage 9-2d で修正された 32 本 (fixed32) の一部が KIND_DIFF に紛れ込んで
//! いた。本ツールは「まず Clip (Basic) で再圧縮・byte-exact なら Sim=Clip、
//! でなければ Sim=Plus1」という Stage 9-2d と同じ選択規則で Sim を決め、
//! Leaf トークン列との最初の相違点を分類する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage10_debug -- --summary <DIR> [N]
//!   cargo run --release --bin lf2_stage10_debug -- --detail <DIR> <names.txt>
//!
//! --summary: 全ファイルの class を1行ずつ出力 (Stage 9 の class 定義を踏襲)
//! --detail : 指定ファイルの初 divergence 点の詳細 (Leaf/Sim 双方のトークン、
//!            remaining、pre-cap raw 長、リング座標) を出力

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_tail_plus1_traced, TailTraceStep, Token, F,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

/// Stage 9-2d と同じ選択規則: Clip が byte-exact ならそれを Sim に、
/// でなければ Plus1 を Sim にする。戻り値は (sim_tokens, trace, mode)。
/// trace は Plus1 側の trace (Clip の場合は raw==capped として同一視できるよう
/// Plus1 側を計算して代用する: Clip 選択時も Plus1 の trace は取れるが、
/// Clip 選択時の実際の出力は clip_tokens を使う)
fn pick_sim(
    ring_input: &[u8],
    orig_payload: &[u8],
) -> (Vec<Token>, Vec<TailTraceStep>, &'static str) {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    let (plus1_tokens, trace) = compress_okumura_tail_plus1_traced(ring_input);
    if orig_payload == clip_reenc.as_slice() {
        (clip_tokens, trace, "clip")
    } else {
        (plus1_tokens, trace, "plus1")
    }
}

fn classify(
    leaf_tokens: &[LeafToken],
    sim_tokens: &[Token],
    ring_input: &[u8],
) -> (
    &'static str, // class
    usize,        // di
    usize,        // input_pos at di
) {
    use retro_decode::formats::toheart::okumura_lzss::N;
    let mut di = None;
    for (i, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
        let same = match (a, b) {
            (LeafToken::Literal(x), Token::Literal(y)) => x == y,
            (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                p1 == p2 && l1 == l2
            }
            _ => false,
        };
        if !same {
            di = Some(i);
            break;
        }
    }
    let Some(di) = di else {
        return ("MATCH", leaf_tokens.len(), ring_input.len());
    };

    // teacher forcing で di 直前まで shadow ring を再現
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for tok in leaf_tokens.iter().take(di) {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        for _ in 0..l {
            if input_pos >= ring_input.len() {
                break;
            }
            ring[r] = ring_input[input_pos];
            r = (r + 1) & (N - 1);
            input_pos += 1;
        }
    }
    let candidates = enumerate_match_candidates_with_writeback(&ring, ring_input, input_pos, r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);

    let class = match (&leaf_tokens[di], &sim_tokens[di]) {
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
        (LeafToken::Literal(_), Token::Literal(_)) => "KIND_DIFF",
        _ => "KIND_DIFF",
    };
    (class, di, input_pos)
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} --summary <DIR> [N] | --detail <DIR> <names.txt>",
            args[0]
        );
        return ExitCode::from(2);
    }

    if args[1] == "--summary" {
        let mut files: Vec<_> = fs::read_dir(&args[2])
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("LF2"))
                    .unwrap_or(false)
            })
            .collect();
        files.sort();
        if let Some(n) = args.get(3).and_then(|v| v.parse::<usize>().ok()) {
            files.truncate(n);
        }
        println!("file,class,mode,di,input_pos");
        for f in &files {
            let name = f.file_name().unwrap().to_str().unwrap().to_string();
            let dir = PathBuf::from(&args[2]);
            let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, &name) else {
                println!("{},PARSE_OR_DECODE_FAIL,-,-,-", name);
                continue;
            };
            let (sim_tokens, _trace, mode) = pick_sim(&ring_input, &orig_payload);
            let (class, di, input_pos) = classify(&leaf_tokens, &sim_tokens, &ring_input);
            println!("{},{},{},{},{}", name, class, mode, di, input_pos);
        }
        return ExitCode::SUCCESS;
    }

    if args[1] == "--detail" {
        let dir = PathBuf::from(&args[2]);
        let names: Vec<String> = fs::read_to_string(&args[3])
            .expect("read names")
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        println!(
            "file,mode,div_ti,input_pos,file_remaining,buf_len,leaf_kind,leaf_len,leaf_pos,leaf_ring,sim_kind,sim_len,sim_pos,sim_ring,sim_raw_len,leaf_next_kind,leaf_next_len,leaf_next_pos,leaf_next_ring"
        );
        for name in &names {
            let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
                println!(
                    "{},PARSE_OR_DECODE_FAIL,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-",
                    name
                );
                continue;
            };
            let (sim_tokens, trace, mode) = pick_sim(&ring_input, &orig_payload);

            let mut di = None;
            for (i, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
                let same = match (a, b) {
                    (LeafToken::Literal(x), Token::Literal(y)) => x == y,
                    (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                        p1 == p2 && l1 == l2
                    }
                    _ => false,
                };
                if !same {
                    di = Some(i);
                    break;
                }
            }
            let Some(di) = di else {
                println!("{},{},NO_DIFF,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-", name, mode);
                continue;
            };

            // input_pos: di 番目のトークンが始まる時点での実入力消費バイト数
            // (leaf/sim は 0..di で完全一致するので teacher forcing で正しく再現できる)
            let mut input_pos: usize = 0;
            for tok in leaf_tokens.iter().take(di) {
                let l = match tok {
                    LeafToken::Literal(_) => 1usize,
                    LeafToken::Match { len, .. } => *len as usize,
                };
                input_pos += l;
            }
            let file_remaining = ring_input.len().saturating_sub(input_pos);

            // buf_len: エンコーダ内部の先読みバッファ充填数 (最大 F=18 で飽和する
            // ため、tail 近傍でない限り file_remaining と一致しない。参考値)
            let ts = trace.get(di);
            let buf_len = ts.map(|t| t.remaining).unwrap_or(usize::MAX);
            let sim_raw_len = ts.map(|t| t.raw_match_length).unwrap_or(-1);

            let (leaf_kind, leaf_len, leaf_pos): (&str, i64, i64) = match &leaf_tokens[di] {
                LeafToken::Literal(_) => ("Literal", -1, -1),
                LeafToken::Match { pos, len } => ("Match", *len as i64, *pos as i64),
            };
            let (sim_kind, sim_len, sim_pos): (&str, i64, i64) = match &sim_tokens[di] {
                Token::Literal(_) => ("Literal", -1, -1),
                Token::Match { pos, len } => ("Match", *len as i64, *pos as i64),
            };
            let leaf_ring = if leaf_pos >= 0 {
                format!("0x{:03x}", leaf_pos & 0x0fff)
            } else {
                "-".to_string()
            };
            let sim_ring = if sim_pos >= 0 {
                format!("0x{:03x}", sim_pos & 0x0fff)
            } else {
                "-".to_string()
            };

            // 次のトークンで Leaf が何をしているか (パターン(i)の追跡: Sim が
            // 見つけた一致を1つ後で使い直すか)
            let (leaf_next_kind, leaf_next_len, leaf_next_pos): (&str, i64, i64) =
                match leaf_tokens.get(di + 1) {
                    Some(LeafToken::Literal(_)) => ("Literal", -1, -1),
                    Some(LeafToken::Match { pos, len }) => ("Match", *len as i64, *pos as i64),
                    None => ("EOF", -1, -1),
                };
            let leaf_next_ring = if leaf_next_pos >= 0 {
                format!("0x{:03x}", leaf_next_pos & 0x0fff)
            } else {
                "-".to_string()
            };

            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                name,
                mode,
                di,
                input_pos,
                file_remaining,
                buf_len,
                leaf_kind,
                leaf_len,
                leaf_pos,
                leaf_ring,
                sim_kind,
                sim_len,
                sim_pos,
                sim_ring,
                sim_raw_len,
                leaf_next_kind,
                leaf_next_len,
                leaf_next_pos,
                leaf_next_ring
            );
            let _ = orig_payload;
        }
        return ExitCode::SUCCESS;
    }

    eprintln!("unknown mode {}", args[1]);
    ExitCode::from(2)
}
