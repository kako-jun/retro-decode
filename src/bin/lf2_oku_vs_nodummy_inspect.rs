//! 奥村 vs no_dummy の token 列を Leaf 真値と side-by-side でダンプ。
//!
//! 「奥村だけ当たる 9 ファイル」「no_dummy だけ当たる 53 ファイル」のパターン観察に使う。
//! 各 token の `奥村==Leaf` / `no_dummy==Leaf` フラグを並置し、divergence の構造を読む。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_no_dummy, Token as OkuToken, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16];
    let payload_start = 0x18 + (color_count as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn token_str_leaf(t: &LeafToken) -> String {
    match t {
        LeafToken::Literal(b) => format!("L({:02X})", b),
        LeafToken::Match { pos, len } => format!("M(p={:04X},l={:02})", pos, len),
    }
}

fn token_str_oku(t: &OkuToken) -> String {
    match t {
        OkuToken::Literal(b) => format!("L({:02X})", b),
        OkuToken::Match { pos, len } => format!("M(p={:04X},l={:02})", pos, len),
    }
}

fn tokens_eq_lo(l: &LeafToken, o: &OkuToken) -> bool {
    match (l, o) {
        (LeafToken::Literal(a), OkuToken::Literal(b)) => a == b,
        (LeafToken::Match { pos: lp, len: ll }, OkuToken::Match { pos: op, len: ol }) => {
            lp == op && ll == ol
        }
        _ => false,
    }
}

fn leaf_step(t: &LeafToken) -> u32 {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as u32,
    }
}

fn oku_step(t: &OkuToken) -> u32 {
    match t {
        OkuToken::Literal(_) => 1,
        OkuToken::Match { len, .. } => *len as u32,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lf2_oku_vs_nodummy_inspect <file.LF2> [max_tokens=80]");
        return ExitCode::from(2);
    }
    let path = PathBuf::from(&args[1]);
    let max_tokens: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);

    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read failed: {}", e);
            return ExitCode::from(1);
        }
    };
    let (w, h, payload_start) = match parse_lf2(&data) {
        Some(x) => x,
        None => {
            eprintln!("parse_lf2 failed");
            return ExitCode::from(1);
        }
    };
    let decoded = match decompress_to_tokens(&data[payload_start..], w, h) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("decompress failed: {:?}", e);
            return ExitCode::from(1);
        }
    };
    let leaf = decoded.tokens;
    let input = decoded.ring_input;

    let oku = compress_okumura(&input);
    let nod = compress_okumura_no_dummy(&input);

    let n = leaf.len().min(oku.len()).min(nod.len()).min(max_tokens);
    println!("File: {}", path.display());
    println!(
        "leaf_total={} oku_total={} nod_total={} input_len={}",
        leaf.len(),
        oku.len(),
        nod.len(),
        input.len()
    );

    // first 18 bytes of input
    print!("input[0..min(36,len)] = ");
    for &b in input.iter().take(36) {
        print!("{:02X} ", b);
    }
    println!();
    println!();

    println!("idx | r_leaf | leaf            | oku             | nod             | L=O L=N O=N");
    println!("----+--------+-----------------+-----------------+-----------------+------------");

    let mut r_leaf: u32 = (N - F) as u32;
    let mask = (N as u32) - 1;

    let mut leaf_eq_oku_count: usize = 0;
    let mut leaf_eq_nod_count: usize = 0;
    let mut oku_eq_nod_count: usize = 0;
    let mut first_div_oku: Option<usize> = None;
    let mut first_div_nod: Option<usize> = None;

    for i in 0..n {
        let lo = tokens_eq_lo(&leaf[i], &oku[i]);
        let ln = tokens_eq_lo(&leaf[i], &nod[i]);
        let on = matches!(
            (&oku[i], &nod[i]),
            (OkuToken::Literal(a), OkuToken::Literal(b)) if a == b
        ) || matches!(
            (&oku[i], &nod[i]),
            (OkuToken::Match { pos: a, len: la }, OkuToken::Match { pos: b, len: lb })
                if a == b && la == lb
        );

        if lo {
            leaf_eq_oku_count += 1;
        } else if first_div_oku.is_none() {
            first_div_oku = Some(i);
        }
        if ln {
            leaf_eq_nod_count += 1;
        } else if first_div_nod.is_none() {
            first_div_nod = Some(i);
        }
        if on {
            oku_eq_nod_count += 1;
        }

        let mark_lo = if lo { "✓" } else { "✗" };
        let mark_ln = if ln { "✓" } else { "✗" };
        let mark_on = if on { "·" } else { "≠" };

        println!(
            "{:3} | {:04X}   | {:15} | {:15} | {:15} | {}   {}   {}",
            i,
            r_leaf,
            token_str_leaf(&leaf[i]),
            token_str_oku(&oku[i]),
            token_str_oku(&nod[i]),
            mark_lo,
            mark_ln,
            mark_on,
        );

        r_leaf = (r_leaf + leaf_step(&leaf[i])) & mask;
    }
    println!();
    println!(
        "summary (first {} tokens): leaf=oku {}/{}  leaf=nod {}/{}  oku=nod {}/{}",
        n, leaf_eq_oku_count, n, leaf_eq_nod_count, n, oku_eq_nod_count, n
    );
    println!(
        "first_div: oku@{:?}  nod@{:?}",
        first_div_oku, first_div_nod
    );

    // ファイル全体の identical 判定
    let oku_full_match = leaf.len() == oku.len() && leaf.iter().zip(oku.iter()).all(|(l, o)| tokens_eq_lo(l, o));
    let nod_full_match = leaf.len() == nod.len() && leaf.iter().zip(nod.iter()).all(|(l, o)| tokens_eq_lo(l, o));
    println!("FULL FILE: identical_oku={} identical_nod={}", oku_full_match, nod_full_match);

    let _ = oku_step; // 念のため (未使用警告抑止)

    ExitCode::SUCCESS
}
