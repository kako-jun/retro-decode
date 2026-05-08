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
    basic_tail1: bool,
    dtd_tail1: bool,
    lit_0x20: bool,
    left_first_tail1: bool,
    max_pos_tail1: bool,
    max_dist_tail1: bool,
    min_dist_tail1: bool,
    hash_chain_first: bool,
    hash_chain_best: bool,
    basic_tail1_full: bool,
    no_dummy_tail1_full: bool,
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

    use retro_decode::formats::toheart::naive_scan_lzss::{self, HashMode};
    let p1 = frame_payload(&okumura_lzss::compress_okumura_no_dummy(input));
    let p2 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_tail1(input));
    let p3 = frame_payload(&okumura_lzss::compress_okumura_basic_tail1(input));
    let p4 = frame_payload(&okumura_lzss::compress_okumura_dummy_then_drop_tail1(input));
    let p5 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_lit_for_0x20(input));
    let p6 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_left_first_tail1(input));
    let p7 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_max_pos_tail1(input));
    let p8 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_max_dist_tail1(input));
    let p9 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_min_dist_tail1(input));
    let p10 = frame_payload(&naive_scan_lzss::compress_hash_chain(input, HashMode::FirstMatch));
    let p11 = frame_payload(&naive_scan_lzss::compress_hash_chain(input, HashMode::BestMatch));
    let p12 = frame_payload(&okumura_lzss::compress_okumura_basic_tail1_full(input));
    let p13 = frame_payload(&okumura_lzss::compress_okumura_no_dummy_tail1_full(input));
    Ok(Result1 {
        name: path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string(),
        no_dummy: p1 == *payload,
        tail1: p2 == *payload,
        basic_tail1: p3 == *payload,
        dtd_tail1: p4 == *payload,
        lit_0x20: p5 == *payload,
        left_first_tail1: p6 == *payload,
        max_pos_tail1: p7 == *payload,
        max_dist_tail1: p8 == *payload,
        min_dist_tail1: p9 == *payload,
        hash_chain_first: p10 == *payload,
        hash_chain_best: p11 == *payload,
        basic_tail1_full: p12 == *payload,
        no_dummy_tail1_full: p13 == *payload,
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

    let counts = [
        ("no_dummy", results.iter().filter(|r| r.no_dummy).count()),
        ("tail1", results.iter().filter(|r| r.tail1).count()),
        ("basic_tail1", results.iter().filter(|r| r.basic_tail1).count()),
        ("dtd_tail1", results.iter().filter(|r| r.dtd_tail1).count()),
        ("lit_0x20", results.iter().filter(|r| r.lit_0x20).count()),
        ("left_first_tail1", results.iter().filter(|r| r.left_first_tail1).count()),
        ("max_pos_tail1", results.iter().filter(|r| r.max_pos_tail1).count()),
        ("max_dist_tail1", results.iter().filter(|r| r.max_dist_tail1).count()),
        ("min_dist_tail1", results.iter().filter(|r| r.min_dist_tail1).count()),
        ("hash_chain_first", results.iter().filter(|r| r.hash_chain_first).count()),
        ("hash_chain_best", results.iter().filter(|r| r.hash_chain_best).count()),
        ("basic_tail1_full", results.iter().filter(|r| r.basic_tail1_full).count()),
        ("no_dummy_tail1_full", results.iter().filter(|r| r.no_dummy_tail1_full).count()),
    ];

    let mut union_set: HashSet<&str> = HashSet::new();
    for r in &results {
        if r.no_dummy || r.tail1 || r.basic_tail1 || r.dtd_tail1 || r.lit_0x20 || r.left_first_tail1
            || r.max_pos_tail1 || r.max_dist_tail1 || r.min_dist_tail1 || r.hash_chain_first || r.hash_chain_best
            || r.basic_tail1_full || r.no_dummy_tail1_full {
            union_set.insert(&r.name);
        }
    }

    println!("=== variant bench ===");
    println!("total files: {}", results.len());
    for (n, c) in &counts {
        println!("  {:30} : {}", n, c);
    }
    println!("  {:30} : {} (across these 6 variants only)", "UNION", union_set.len());

    // 各変種の固有 contribution (== match here AND no other in this set matches)
    // Find files matched by EXACTLY one of the new variants (max_pos/max_dist/min_dist/hash_*)
    let new_variants_only: Vec<&Result1> = results.iter().filter(|r| {
        let any_old = r.no_dummy || r.tail1 || r.basic_tail1 || r.dtd_tail1 || r.lit_0x20 || r.left_first_tail1;
        let any_new = r.max_pos_tail1 || r.max_dist_tail1 || r.min_dist_tail1 || r.hash_chain_first || r.hash_chain_best;
        any_new && !any_old
    }).collect();
    println!("\n=== matched ONLY by new tie/hash variants (not by old 6) ===");
    println!("  count: {}", new_variants_only.len());
    for r in new_variants_only.iter().take(20) {
        let mut tags = vec![];
        if r.max_pos_tail1 { tags.push("max_pos"); }
        if r.max_dist_tail1 { tags.push("max_dist"); }
        if r.min_dist_tail1 { tags.push("min_dist"); }
        if r.hash_chain_first { tags.push("hc_first"); }
        if r.hash_chain_best { tags.push("hc_best"); }
        println!("    {:14} {}", r.name, tags.join(","));
    }
    ExitCode::SUCCESS
}
