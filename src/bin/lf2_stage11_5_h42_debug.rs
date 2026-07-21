//! Stage 11-5 (Issue #14 脈4): H42 単体の帯内採用 (pos=4074) 再精査。
//!
//! v4 (`DummyMode::RejectPureBootstrap`) は H42 を壊す (baseline203 の一員だが
//! v4 のみ不一致)。原因を特定するため、Leaf 実トークン列で teacher forcing
//! しながら write_tick を追跡し、divergence 点 (Leaf vs v4 の最初の不一致
//! token) の直前状態を出力する。実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage11_5_h42_debug -- <FILE.LF2> [--vs plus1]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_clip_no_bootstrap_v4, compress_okumura_plus1_no_bootstrap_v4, Token, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const BOOTSTRAP_DUMMY_LO: usize = N - F - F; // 4060
const BOOTSTRAP_DUMMY_HI: usize = N - F - 1; // 4077
const INITIAL_LOOKAHEAD_LO: usize = N - F; // 4078
const INITIAL_LOOKAHEAD_HI: usize = N - 1; // 4095

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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <FILE.LF2> [--vs plus1]", args[0]);
        return ExitCode::from(2);
    }
    let path = PathBuf::from(&args[1]);
    let use_plus1 = args.iter().any(|a| a == "--vs" ) && args.iter().any(|a| a == "plus1");

    let data = fs::read(&path).expect("read file");
    let (width, height, ps) = parse_lf2(&data).expect("parse lf2 header");
    let decoded = decompress_to_tokens(&data[ps..], width, height).expect("decode tokens");
    let leaf_tokens = decoded.tokens;
    let ring_input = decoded.ring_input;

    let v4_tokens = if use_plus1 {
        compress_okumura_plus1_no_bootstrap_v4(&ring_input)
    } else {
        compress_okumura_clip_no_bootstrap_v4(&ring_input)
    };

    // divergence 点 (Leaf vs v4)
    let mut di = None;
    for (i, (a, b)) in leaf_tokens.iter().zip(v4_tokens.iter()).enumerate() {
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
        println!("NO_DIFF vs v4 ({})", if use_plus1 { "plus1" } else { "clip" });
        return ExitCode::SUCCESS;
    };

    // teacher forcing: Leaf の実トークンで ring/write_tick を再現 (di 直前まで)
    let mut ring = [0x20u8; N];
    let mut write_tick = [u32::MAX; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    // ブートストラップ由来の「先読み充填」を write_tick なしで模擬 (compress 側と同じ:
    // 最初の F バイトは text_buf[r..r+F-1] に書かれるが write_tick は更新されない)
    for k in 0..F.min(ring_input.len()) {
        ring[(N - F + k) & (N - 1)] = ring_input[k];
    }

    for tok in leaf_tokens.iter().take(di) {
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

    println!("file: {}", path.display());
    println!("divergence token index (di): {}", di);
    println!("input_pos consumed before di (input_pos@di): {}", input_pos);
    println!("ring write pointer r@di: {}", r);
    println!("ring size N: {} , has input_pos exceeded N (ring wrapped at least once): {}", N, input_pos > N);

    match leaf_tokens.get(di) {
        Some(LeafToken::Match { pos, len }) => {
            let pos_u = (*pos as usize) & (N - 1);
            println!("Leaf token[di] = Match {{ pos: {}, len: {} }}", pos_u, len);
            println!(
                "  in bootstrap band [{},{}]: {}",
                BOOTSTRAP_DUMMY_LO,
                BOOTSTRAP_DUMMY_HI,
                pos_u >= BOOTSTRAP_DUMMY_LO && pos_u <= BOOTSTRAP_DUMMY_HI
            );
            println!(
                "  in initial lookahead range [{},{}]: {}",
                INITIAL_LOOKAHEAD_LO, INITIAL_LOOKAHEAD_HI,
                pos_u >= INITIAL_LOOKAHEAD_LO && pos_u <= INITIAL_LOOKAHEAD_HI
            );
            for k in 0..*len as usize {
                let slot = (pos_u + k) & (N - 1);
                let wt = write_tick[slot];
                let wt_str = if wt == u32::MAX { "UNSET".to_string() } else { wt.to_string() };
                println!(
                    "  slot {} (+{}) write_tick={} real_byte(ring_input[input_pos+{}])={:?}",
                    slot,
                    k,
                    wt_str,
                    k,
                    ring_input.get(input_pos + k)
                );
            }
        }
        Some(LeafToken::Literal(b)) => println!("Leaf token[di] = Literal({})", b),
        None => println!("Leaf token[di] = EOF"),
    }

    match v4_tokens.get(di) {
        Some(Token::Match { pos, len }) => println!("v4  token[di] = Match {{ pos: {}, len: {} }}", pos, len),
        Some(Token::Literal(b)) => println!("v4  token[di] = Literal({})", b),
        None => println!("v4  token[di] = EOF"),
    }

    ExitCode::SUCCESS
}
