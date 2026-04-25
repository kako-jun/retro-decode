//! Side-by-side first-divergence inspector for the LF2 LZSS encoder.
//!
//! Pass 1: scan all .LF2 in input dir, compute (filename, first_divergence_token_index).
//! Pass 2: for the 10 earliest-divergence files, dump deep state (last 5 matching tokens,
//! the divergence token, ring buffer near r/leaf_pos/oku_pos, lookahead bytes,
//! all match candidates with markers, and BST nodes).
//!
//! Usage:
//!     cargo run --release --bin lf2_first_div_inspect -- <input_dir>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken, MatchCandidate,
};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_inspect, format_bst_dump, OkumuraSnapshot, Token as OkuToken, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

struct Header {
    width: u16,
    height: u16,
    payload_start: usize,
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
    Ok(Header {
        width,
        height,
        payload_start,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UniToken {
    Literal(u8),
    Match { pos: u16, len: u8 },
}

impl From<LeafToken> for UniToken {
    fn from(t: LeafToken) -> Self {
        match t {
            LeafToken::Literal(b) => UniToken::Literal(b),
            LeafToken::Match { pos, len } => UniToken::Match { pos, len },
        }
    }
}
impl From<OkuToken> for UniToken {
    fn from(t: OkuToken) -> Self {
        match t {
            OkuToken::Literal(b) => UniToken::Literal(b),
            OkuToken::Match { pos, len } => UniToken::Match { pos, len },
        }
    }
}

fn fmt_uni(t: UniToken) -> String {
    match t {
        UniToken::Literal(b) => format!("Literal(0x{:02x})", b),
        UniToken::Match { pos, len } => format!("Match{{pos=0x{:03x}, len={}}}", pos, len),
    }
}

fn first_divergence(leaf: &[LeafToken], oku: &[OkuToken]) -> Option<usize> {
    let n = leaf.len().min(oku.len());
    for i in 0..n {
        let lu: UniToken = leaf[i].into();
        let ou: UniToken = oku[i].into();
        if lu != ou {
            return Some(i);
        }
    }
    if leaf.len() != oku.len() {
        Some(n)
    } else {
        None
    }
}

fn ring_row_str(text_buf: &[u8], row_base: usize) -> String {
    let mut hex = String::new();
    let mut asc = String::new();
    for col in 0..16 {
        let idx = row_base + col;
        if idx >= text_buf.len() {
            hex.push_str("   ");
            asc.push(' ');
            continue;
        }
        let b = text_buf[idx];
        hex.push_str(&format!("{:02x} ", b));
        let c = if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' };
        asc.push(c);
    }
    format!("{} | {}", hex.trim_end(), asc)
}

fn dump_ring_rows(text_buf: &[u8; N + F - 1], rows_of_interest: &[usize]) -> String {
    use std::collections::BTreeSet;
    let mut wanted: BTreeSet<usize> = BTreeSet::new();
    for &row in rows_of_interest {
        let row_idx = row / 16;
        for d in [-1i32, 0, 1] {
            let r = row_idx as i32 + d;
            if r >= 0 && (r as usize) * 16 < N {
                wanted.insert(r as usize);
            }
        }
    }
    let mut s = String::new();
    let mut last: Option<usize> = None;
    for &row in &wanted {
        if let Some(prev) = last {
            if row != prev + 1 {
                s.push_str("    ...\n");
            }
        }
        let base = row * 16;
        let line = ring_row_str(text_buf, base);
        s.push_str(&format!("    ring[0x{:03x}] = {}\n", base, line));
        last = Some(row);
    }
    s
}

fn dump_lookahead(text_buf: &[u8; N + F - 1], r: i32) -> String {
    let r = r as usize;
    let mut hex = String::new();
    let mut asc = String::new();
    for i in 0..F {
        let idx = r + i;
        if idx >= text_buf.len() {
            break;
        }
        let b = text_buf[idx];
        hex.push_str(&format!("{:02x} ", b));
        let c = if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' };
        asc.push(c);
    }
    format!("text_buf[r..r+F] = {}| {}", hex, asc)
}

fn process_file(path: &PathBuf) -> anyhow::Result<Option<(usize, Vec<LeafToken>, Vec<OkuToken>, Vec<u8>, Header)>> {
    let data = fs::read(path)?;
    let hdr = parse_header(&data)?;
    let payload = &data[hdr.payload_start..];
    let leaf_decode = decompress_to_tokens(payload, hdr.width, hdr.height)?;
    let oku_tokens = compress_okumura(&leaf_decode.ring_input);
    let div = first_divergence(&leaf_decode.tokens, &oku_tokens);
    Ok(div.map(|i| (i, leaf_decode.tokens, oku_tokens, leaf_decode.ring_input, hdr)))
}

fn deep_dump(name: &str, leaf_tokens: &[LeafToken], oku_tokens: &[OkuToken], input: &[u8], div_idx: usize) {
    println!("=== {} first_divergence_at_token={} ===", name, div_idx);

    // Last 5 tokens before divergence
    println!();
    println!("Last 5 tokens before divergence (should match):");
    println!("    token  leaf                              oku");
    let start = div_idx.saturating_sub(5);
    for i in start..div_idx {
        let lu: UniToken = leaf_tokens.get(i).copied().map(Into::into).unwrap_or(UniToken::Literal(0));
        let ou: UniToken = oku_tokens.get(i).copied().map(Into::into).unwrap_or(UniToken::Literal(0));
        let mark = if lu == ou { "OK" } else { "DIFF" };
        println!("    {:5}  {:32}  {:32}  {}", i, fmt_uni(lu), fmt_uni(ou), mark);
    }

    // Get inspection snapshot at div_idx
    let snap: OkumuraSnapshot = compress_okumura_inspect(input, div_idx);

    let leaf_tok: UniToken = leaf_tokens.get(div_idx).copied().map(Into::into)
        .unwrap_or(UniToken::Literal(0xFE));
    let oku_tok: UniToken = oku_tokens.get(div_idx).copied().map(Into::into)
        .unwrap_or(UniToken::Literal(0xFE));

    let r = snap.r;
    let s = snap.s;
    let mask = (N as i32) - 1;

    println!();
    println!("Token {}:", div_idx);
    let leaf_dist = match leaf_tok {
        UniToken::Match { pos, .. } => Some((r - pos as i32) & mask),
        _ => None,
    };
    let oku_dist = match oku_tok {
        UniToken::Match { pos, .. } => Some((r - pos as i32) & mask),
        _ => None,
    };
    print!("    leaf chose: {}", fmt_uni(leaf_tok));
    if let Some(d) = leaf_dist { println!("    distance from r=0x{:03x}", d); } else { println!(); }
    print!("    oku  chose: {}", fmt_uni(oku_tok));
    if let Some(d) = oku_dist { println!("    distance from r=0x{:03x}", d); } else { println!(); }

    // Ring buffer state
    println!();
    println!("Ring buffer state:");
    println!("    r = 0x{:03x} (= row {} col {})", r as u16, r / 16, r % 16);
    println!("    s = 0x{:03x}", s as u16);
    let mut interest = vec![r as usize];
    if let UniToken::Match { pos, .. } = leaf_tok {
        interest.push(pos as usize);
        println!("    leaf_pos = 0x{:03x} (row {} col {})", pos, pos / 16, pos % 16);
    }
    if let UniToken::Match { pos, .. } = oku_tok {
        interest.push(pos as usize);
        println!("    oku_pos  = 0x{:03x} (row {} col {})", pos, pos / 16, pos % 16);
    }
    println!();
    print!("{}", dump_ring_rows(&snap.text_buf, &interest));

    // Lookahead
    println!();
    println!("Lookahead at r:");
    println!("    {}", dump_lookahead(&snap.text_buf, r));

    // Build a temporary ring snapshot from text_buf for candidate enumeration.
    // text_buf is N+F-1 bytes; the canonical 4096-byte ring is text_buf[0..N].
    let mut ring = [0u8; N];
    ring.copy_from_slice(&snap.text_buf[..N]);

    let candidates = enumerate_match_candidates_with_writeback(&ring, input, s as usize, r as usize);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let cutoff = max_len.saturating_sub(2);

    let mut filtered: Vec<&MatchCandidate> = candidates.iter()
        .filter(|c| c.len >= cutoff)
        .collect();
    // Sort by (len desc, distance asc)
    let dist_of = |pos: u16| -> u32 { ((r as i32 - pos as i32) & mask) as u32 };
    filtered.sort_by_key(|c| (std::cmp::Reverse(c.len), dist_of(c.pos)));

    println!();
    println!("Match candidates (len >= {}, sorted by len desc / distance asc, max 30):", cutoff);
    println!("    rank  pos    distance  len   note");
    let mut shown = 0usize;
    for (idx, c) in filtered.iter().enumerate() {
        if shown >= 30 { println!("    ... ({} more)", filtered.len() - shown); break; }
        let d = dist_of(c.pos);
        let mut note = String::new();
        if let UniToken::Match { pos, len } = leaf_tok {
            if c.pos == pos && c.len == len { note.push_str(" <- LEAF"); }
        }
        if let UniToken::Match { pos, len } = oku_tok {
            if c.pos == pos && c.len == len {
                if note.is_empty() { note.push_str(" <- OKU"); }
                else { note.push_str(" <- OKU(==LEAF?)"); }
            }
        }
        println!("    {:4}  0x{:03x}  0x{:04x}    {:3}  {}", idx + 1, c.pos, d, c.len, note);
        shown += 1;
    }

    // Note: it's possible LEAF chose a literal or a token not in the candidate set.
    if let UniToken::Literal(b) = leaf_tok {
        println!("    (LEAF chose Literal(0x{:02x}); not present in match candidates)", b);
    } else if let UniToken::Match { pos, len } = leaf_tok {
        let in_set = candidates.iter().any(|c| c.pos == pos && c.len == len);
        if !in_set {
            println!("    (!! LEAF's Match{{pos=0x{:03x}, len={}}} NOT in candidate set !!)", pos, len);
        }
    }
    if let UniToken::Match { pos, len } = oku_tok {
        let in_set = candidates.iter().any(|c| c.pos == pos && c.len == len);
        if !in_set {
            println!("    (!! OKU's Match{{pos=0x{:03x}, len={}}} NOT in candidate set !!)", pos, len);
        }
    }

    // BST dump
    println!();
    print!("{}", format_bst_dump(&snap, 50));
    println!();
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

    eprintln!("Pass 1: scanning {} files for first divergence...", entries.len());

    // Pass 1
    struct DivEntry {
        name: String,
        path: PathBuf,
        div_idx: usize,
        leaf: Vec<LeafToken>,
        oku: Vec<OkuToken>,
        input: Vec<u8>,
        _hdr: Header,
    }
    let mut divs: Vec<DivEntry> = Vec::new();
    let mut identical = 0usize;
    let mut errored = 0usize;
    for path in &entries {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        match process_file(path) {
            Ok(None) => { identical += 1; }
            Ok(Some((i, leaf, oku, input, hdr))) => {
                divs.push(DivEntry {
                    name,
                    path: path.clone(),
                    div_idx: i,
                    leaf,
                    oku,
                    input,
                    _hdr: hdr,
                });
            }
            Err(e) => {
                eprintln!("  err {}: {}", name, e);
                errored += 1;
            }
        }
    }

    divs.sort_by_key(|d| (d.div_idx, d.name.clone()));
    eprintln!("Identical: {}, Divergent: {}, Errored: {}", identical, divs.len(), errored);
    eprintln!();

    let top_n = divs.len().min(10);
    eprintln!("Top {} earliest-divergence files:", top_n);
    for d in divs.iter().take(top_n) {
        eprintln!("  {}: token {}", d.name, d.div_idx);
    }
    eprintln!();

    // Print the top-10 list also to stdout (it's part of the report)
    println!("### TOP {} EARLIEST DIVERGENCE ###", top_n);
    for d in divs.iter().take(top_n) {
        println!("{}: token {}", d.name, d.div_idx);
    }
    println!();

    // Pass 2: deep dumps
    for d in divs.iter().take(top_n) {
        let _ = d.path; // unused after this point
        deep_dump(&d.name, &d.leaf, &d.oku, &d.input, d.div_idx);
    }

    ExitCode::SUCCESS
}
