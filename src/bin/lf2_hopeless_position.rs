//! For each uncovered file, find positions of hopeless tokens (no variant matches).
//! Specifically, are hopeless tokens clustered at the END (= phantom-extension solvable)
//! or in the middle (= require new rule)?

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn token_eq(a: &Token, b: &LeafToken) -> bool {
    match (a, b) {
        (Token::Literal(x), LeafToken::Literal(y)) => x == y,
        (Token::Match { pos: pa, len: la }, LeafToken::Match { pos: pb, len: lb }) => {
            pa == pb && la == lb
        }
        _ => false,
    }
}

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        ("basic", okumura_lzss::compress_okumura),
        ("no_dummy", okumura_lzss::compress_okumura_no_dummy),
        ("dummy_then_drop", okumura_lzss::compress_okumura_dummy_then_drop),
        ("no_dummy_left_first", okumura_lzss::compress_okumura_no_dummy_left_first),
        ("no_dummy_tail1", okumura_lzss::compress_okumura_no_dummy_tail1),
        ("basic_tail1", okumura_lzss::compress_okumura_basic_tail1),
        ("dummy_then_drop_tail1", okumura_lzss::compress_okumura_dummy_then_drop_tail1),
        ("basic_tail1_full", okumura_lzss::compress_okumura_basic_tail1_full),
        ("no_dummy_tail1_full", okumura_lzss::compress_okumura_no_dummy_tail1_full),
        ("basic_no_init", okumura_lzss::compress_okumura_basic_no_init),
        ("basic_no_init_tail1", okumura_lzss::compress_okumura_basic_no_init_tail1),
        ("dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("combo", okumura_lzss::compress_okumura_combo),
        ("with_tie_strict", |i| okumura_lzss::compress_okumura_with_tie(i, false)),
        ("no_dummy_lit_for_0x20", okumura_lzss::compress_okumura_no_dummy_lit_for_0x20),
    ]
}

fn process_file(path: &std::path::Path) -> Option<(String, usize, Vec<usize>)> {
    let data = fs::read(path).ok()?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w, h).ok()?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    let outputs: Vec<Vec<Token>> = variants().iter().map(|(_, f)| f(input)).collect();

    let total = leaf.len();
    let mut hopeless_positions: Vec<usize> = Vec::new();
    for i in 0..total {
        let mut any_match = false;
        for out in &outputs {
            if i < out.len() && token_eq(&out[i], &leaf[i]) {
                any_match = true;
                break;
            }
        }
        if !any_match {
            hopeless_positions.push(i);
        }
    }

    let name = path.file_name()?.to_str()?.to_string();
    Some((name, total, hopeless_positions))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    eprintln!("processing {} files...", paths.len());

    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    let results: Vec<(String, usize, Vec<usize>)> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                chunk.iter().filter_map(|p| process_file(p)).collect::<Vec<_>>()
            });
            handles.push(h);
        }
        let mut all = Vec::new();
        for h in handles {
            all.extend(h.join().unwrap());
        }
        all
    });

    println!("file\ttotal\thopeless_count\tfirst_hopeless\tlast_hopeless\tin_last_8_tokens");
    for (name, total, hops) in &results {
        if hops.is_empty() {
            continue;
        }
        let first = hops.first().unwrap();
        let last = hops.last().unwrap();
        let in_last_8 = hops.iter().filter(|&&h| h + 8 >= *total).count();
        println!("{}\t{}\t{}\t{}\t{}\t{}", name, total, hops.len(), first, last, in_last_8);
    }

    ExitCode::SUCCESS
}
