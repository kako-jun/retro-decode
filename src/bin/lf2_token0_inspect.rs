//! Token-0-focused inspector: find LF2 files where Leaf's token 0 is a Match
//! but the no_dummy okumura variant emits a Literal at token 0.
//!
//! For each such file print:
//!   <filename>  leaf_token0=Match{pos=0xXXX, len=YY}   first_input_byte=0xZZ
//!
//! Then print three histograms:
//!   - leaf_pos values (top 20)
//!   - leaf_len values (full)
//!   - first_input_byte values (full)
//!
//! Usage:
//!     cargo run --release --bin lf2_token0_inspect -- <input_dir>

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_no_dummy, Token as OkuToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

struct Header {
    payload_start: usize,
    width: u16,
    height: u16,
}

fn parse_header(data: &[u8]) -> anyhow::Result<Header> {
    if data.len() < 0x18 {
        anyhow::bail!("file too small");
    }
    if &data[0..8] != LF2_MAGIC {
        anyhow::bail!("bad magic");
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16];
    let payload_start = 0x18 + (color_count as usize) * 3;
    if payload_start > data.len() {
        anyhow::bail!("payload past EOF");
    }
    Ok(Header { payload_start, width, height })
}

struct Hit {
    name: String,
    leaf_pos: u16,
    leaf_len: u8,
    first_input_byte: u8,
}

fn process(path: &PathBuf) -> anyhow::Result<Option<Hit>> {
    let data = fs::read(path)?;
    let hdr = parse_header(&data)?;
    let payload = &data[hdr.payload_start..];
    let leaf_decode = decompress_to_tokens(payload, hdr.width, hdr.height)?;
    if leaf_decode.tokens.is_empty() || leaf_decode.ring_input.is_empty() {
        return Ok(None);
    }
    let leaf0 = leaf_decode.tokens[0];
    let nodummy = compress_okumura_no_dummy(&leaf_decode.ring_input);
    let oku0 = nodummy.first().copied();

    let leaf_is_match = matches!(leaf0, LeafToken::Match { .. });
    let oku_is_literal = matches!(oku0, Some(OkuToken::Literal(_)));

    if leaf_is_match && oku_is_literal {
        if let LeafToken::Match { pos, len } = leaf0 {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            return Ok(Some(Hit {
                name,
                leaf_pos: pos,
                leaf_len: len,
                first_input_byte: leaf_decode.ring_input[0],
            }));
        }
    }
    Ok(None)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    if !dir.is_dir() {
        eprintln!("error: {} is not a directory", dir.display());
        return ExitCode::from(2);
    }

    let mut entries: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("read_dir failed: {}", e);
            return ExitCode::from(1);
        }
    };
    entries.sort();
    eprintln!("Scanning {} files...", entries.len());

    let mut hits: Vec<Hit> = Vec::new();
    let mut errored = 0usize;
    for path in &entries {
        match process(path) {
            Ok(Some(h)) => hits.push(h),
            Ok(None) => {}
            Err(e) => {
                eprintln!(
                    "  err {}: {}",
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                    e
                );
                errored += 1;
            }
        }
    }

    eprintln!("Hits: {}, Errored: {}", hits.len(), errored);
    println!("### TOKEN 0 DISAGREEMENTS (Leaf=Match, no_dummy=Literal) total={} ###", hits.len());
    hits.sort_by(|a, b| a.name.cmp(&b.name));
    for h in &hits {
        println!(
            "{}  leaf_token0=Match{{pos=0x{:03x}, len={}}}   first_input_byte=0x{:02x}",
            h.name, h.leaf_pos, h.leaf_len, h.first_input_byte
        );
    }

    // Histograms
    let mut pos_hist: HashMap<u16, usize> = HashMap::new();
    let mut len_hist: HashMap<u8, usize> = HashMap::new();
    let mut byte_hist: HashMap<u8, usize> = HashMap::new();
    for h in &hits {
        *pos_hist.entry(h.leaf_pos).or_insert(0) += 1;
        *len_hist.entry(h.leaf_len).or_insert(0) += 1;
        *byte_hist.entry(h.first_input_byte).or_insert(0) += 1;
    }

    let mut pos_v: Vec<(u16, usize)> = pos_hist.into_iter().collect();
    pos_v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!();
    println!("### leaf_pos histogram (top 20) ###");
    for (pos, count) in pos_v.iter().take(20) {
        println!("  pos=0x{:03x}  count={}", pos, count);
    }
    println!("  (distinct positions: {})", pos_v.len());

    let mut len_v: Vec<(u8, usize)> = len_hist.into_iter().collect();
    len_v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!();
    println!("### leaf_len histogram ###");
    for (len, count) in &len_v {
        println!("  len={}  count={}", len, count);
    }

    let mut byte_v: Vec<(u8, usize)> = byte_hist.into_iter().collect();
    byte_v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!();
    println!("### first_input_byte histogram ###");
    for (b, count) in &byte_v {
        println!("  byte=0x{:02x}  count={}", b, count);
    }

    ExitCode::SUCCESS
}
