//! Quick single-variant bench for `okumura_no_dummy_tail1`.
//! Reports binary match count and per-file delta vs baseline `okumura_no_dummy`.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

struct Result1 {
    name: String,
    no_dummy: bool,
    tail1: bool,
}

fn process(path: &std::path::Path) -> Result<Result1> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("not LF2"));
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h)?;
    let input = &dec.ring_input;
    let payload = &data[ps..];

    let p1 = frame_payload(&okumura_lzss::compress_okumura_no_dummy(input));
    let p2 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_tail1(input));
    Ok(Result1 {
        name: path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string(),
        no_dummy: p1 == *payload,
        tail1: p2 == *payload,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <dir>", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("read_dir error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    paths.sort();
    eprintln!("found {} LF2 files", paths.len());

    let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(paths.len().max(1));
    let cs = (paths.len() + n - 1) / n;
    let results: Vec<Result1> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(cs) {
            let h = scope.spawn(move || {
                let mut out = Vec::new();
                for p in chunk {
                    match process(p) {
                        Ok(r) => out.push(r),
                        Err(e) => eprintln!("WARN: {}: {}", p.display(), e),
                    }
                }
                out
            });
            handles.push(h);
        }
        let mut all = Vec::new();
        for h in handles {
            all.extend(h.join().unwrap());
        }
        all
    });

    let n_no_dummy = results.iter().filter(|r| r.no_dummy).count();
    let n_tail1 = results.iter().filter(|r| r.tail1).count();
    let n_only_tail1: HashSet<&str> = results.iter().filter(|r| r.tail1 && !r.no_dummy).map(|r| r.name.as_str()).collect();
    let n_only_no_dummy: HashSet<&str> = results.iter().filter(|r| r.no_dummy && !r.tail1).map(|r| r.name.as_str()).collect();

    println!("=== Single-variant bench ===");
    println!("total files            : {}", results.len());
    println!("okumura_no_dummy match : {}", n_no_dummy);
    println!("tail1 match            : {}", n_tail1);
    println!("delta                  : {:+}", n_tail1 as i64 - n_no_dummy as i64);
    println!();
    println!("--- newly matched by tail1 (was failing under no_dummy) [{}] ---", n_only_tail1.len());
    let mut v: Vec<&str> = n_only_tail1.iter().copied().collect();
    v.sort();
    for n in v {
        println!("+ {}", n);
    }
    println!();
    println!("--- regressed (was passing under no_dummy, fails under tail1) [{}] ---", n_only_no_dummy.len());
    let mut v: Vec<&str> = n_only_no_dummy.iter().copied().collect();
    v.sort();
    for n in v {
        println!("- {}", n);
    }
    ExitCode::SUCCESS
}
