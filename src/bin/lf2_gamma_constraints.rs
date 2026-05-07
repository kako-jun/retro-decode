//! γ DP の M1+M2+M3: 単一 LF2 から target token 列・順方向 sanity simulation・
//! 「同じバイト列を出す全 (pos, len) 候補集合 = γ constraints の最小単位」を CSV で吐く。
//!
//! Issue: kako-jun/retro-decode#2 [Epic] LF2 バイナリ一致プロジェクト
//!
//! 観察: pos の選択は next ring に影響しない (どの合法 pos も同じバイトを書く)。
//! ⇒ per-token で legal pos 集合を独立に列挙でき、DP backtrack 不要。
//! ⇒ 後段 M4 で 522 ファイル分の constraints を集合演算して encoder ルールを発見する。
//!
//! Usage: cargo run --release --bin lf2_gamma_constraints -- <file.LF2> [out.csv]

use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken, MatchCandidate,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const RING_SIZE: usize = 0x1000;
const INITIAL_RING_POS: usize = 0x0fee;

struct Lf2Header {
    width: u16,
    height: u16,
    payload_start: usize,
}

fn parse_header(data: &[u8]) -> Result<Lf2Header> {
    if data.len() < 0x18 {
        return Err(anyhow!("LF2 too small: {} bytes", data.len()));
    }
    if &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("not LF2 (magic mismatch)"));
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16] as usize;
    let payload_start = 0x18 + color_count * 3;
    if data.len() <= payload_start {
        return Err(anyhow!(
            "no payload (palette ends at 0x{:x}, file size {})",
            payload_start,
            data.len()
        ));
    }
    Ok(Lf2Header {
        width,
        height,
        payload_start,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ERROR: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!(
            "usage: {} <file.LF2> [out.csv]",
            args.first().map(|s| s.as_str()).unwrap_or("lf2_gamma_constraints")
        ));
    }
    let in_path = Path::new(&args[1]);
    let data = fs::read(in_path).with_context(|| format!("read {}", in_path.display()))?;
    let hdr = parse_header(&data)?;
    let dec = decompress_to_tokens(&data[hdr.payload_start..], hdr.width, hdr.height)
        .with_context(|| format!("decompress_to_tokens for {}", in_path.display()))?;

    let mut writer: Box<dyn Write> = if let Some(p) = args.get(2) {
        Box::new(BufWriter::new(File::create(p)?))
    } else {
        Box::new(BufWriter::new(std::io::stdout().lock()))
    };

    writeln!(
        writer,
        "token_idx,input_pos,ring_r,leaf_kind,leaf_pos,leaf_len,n_legal,legal_pos"
    )?;

    let input = &dec.ring_input;
    let mut ring = [0x20u8; RING_SIZE];
    let mut r: usize = INITIAL_RING_POS;
    let mut s: usize = 0;

    let mut n_match_tokens: u64 = 0;
    let mut n_unique_pos: u64 = 0;
    let mut n_multi_pos: u64 = 0;
    let mut sum_n_legal: u64 = 0;
    let mut max_n_legal: u32 = 0;

    for (idx, tok) in dec.tokens.iter().enumerate() {
        let cands: Vec<MatchCandidate> =
            enumerate_match_candidates_with_writeback(&ring, input, s, r);

        let (kind, lpos, llen, legal_pos): (char, u32, u32, Vec<u16>) = match *tok {
            LeafToken::Literal(b) => ('L', b as u32, 1, Vec::new()),
            LeafToken::Match { pos, len } => {
                let legal: Vec<u16> = cands
                    .iter()
                    .filter(|c| c.len == len)
                    .map(|c| c.pos)
                    .collect();
                ('M', pos as u32, len as u32, legal)
            }
        };

        let n_legal = legal_pos.len() as u32;
        let legal_str = if legal_pos.is_empty() {
            String::new()
        } else {
            let mut s = String::with_capacity(legal_pos.len() * 5);
            for (i, p) in legal_pos.iter().enumerate() {
                if i > 0 {
                    s.push(';');
                }
                s.push_str(&p.to_string());
            }
            s
        };
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{}",
            idx, s, r, kind, lpos, llen, n_legal, legal_str
        )?;

        if kind == 'M' {
            // 末端切り詰め: nominal len > remaining bytes の場合、decoder が
            // total_pixels で break したためトークンの記録 len > 実出力 len。
            // この最終マッチでは constraint 分析意味なし、スキップ。
            let end_truncated = s + llen as usize > input.len();
            if !end_truncated && !legal_pos.contains(&(lpos as u16)) {
                return Err(anyhow!(
                    "BUG: leaf-chosen pos {} not in legal set at token {} (input_pos={}, r={}, len={}, in_len={})",
                    lpos, idx, s, r, llen, input.len()
                ));
            }
            n_match_tokens += 1;
            sum_n_legal += n_legal as u64;
            if n_legal > max_n_legal {
                max_n_legal = n_legal;
            }
            if n_legal == 1 {
                n_unique_pos += 1;
            } else {
                n_multi_pos += 1;
            }
        }

        let len = match *tok {
            LeafToken::Literal(_) => 1,
            LeafToken::Match { len, .. } => len as usize,
        };
        for _ in 0..len {
            ring[r] = input[s];
            r = (r + 1) & 0x0fff;
            s += 1;
        }
    }

    writer.flush()?;

    let mean_n_legal = if n_match_tokens > 0 {
        sum_n_legal as f64 / n_match_tokens as f64
    } else {
        0.0
    };
    eprintln!(
        "{}: tokens={} input_bytes={} match={} unique_pos={} multi_pos={} mean_n_legal={:.3} max_n_legal={}",
        in_path.display(),
        dec.tokens.len(),
        s,
        n_match_tokens,
        n_unique_pos,
        n_multi_pos,
        mean_n_legal,
        max_n_legal,
    );

    Ok(())
}
