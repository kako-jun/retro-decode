//! Issue kako-jun/retro-decode#4 用 first-diff デバッガ。
//!
//! LF2 ペイロードを Leaf のトークン列にデコードし、同じ入力バイト列を
//! 奥村 lzss.c 二分木版移植で再エンコードしたトークン列と比較し、
//! **最初に食い違ったトークン位置**で停止する。そのトークン位置での
//! ring buffer 状態に対する全マッチ候補を列挙して、Leaf の選択が
//! 候補集合に含まれるか切り分ける。
//!
//! モード A (単一ファイル詳細):
//!     cargo run --release --bin lf2_first_diff -- <file.LF2>
//!
//! モード B (ヒストグラム集計):
//!     cargo run --release --bin lf2_first_diff -- --histogram <input_dir>
//!
//! モード C (全決定点データセット生成):
//!     cargo run --release --bin lf2_first_diff -- --full-dataset <input_dir> <output.csv>
//!
//! ヒストグラムモードの出力:
//!     - stdout: 1 行 1 ファイルの CSV（発散のあったファイルのみ）
//!     - stderr: 集計サマリ
//!
//! 全決定点モードの出力:
//!     - stdout: CSV フォーマット（ヘッダ行 + 各決定点）
//!     - stderr: 処理進捗
//!
//! ヘッダ行 (CSV):
//!     filename,token_index,leaf_choice_index,
//!     num_candidates,max_candidate_len,
//!     image_x,image_y,ring_r,prev_token_kind,
//!     candidate_0_distance,candidate_0_length,...,candidate_N_distance,candidate_N_length

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken, MatchCandidate,
};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, Token as OkuToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

struct Header {
    width: u16,
    height: u16,
    color_count: u8,
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
        anyhow::bail!("payload_start past EOF");
    }
    Ok(Header {
        width,
        height,
        color_count,
        payload_start,
    })
}

/// LeafToken と奥村 Token の突き合わせに必要な統一表現。
#[derive(Debug, Clone, Copy)]
struct UniToken {
    is_match: bool,
    /// リテラルなら pixel、マッチなら 0 (未使用)
    lit: u8,
    /// マッチなら pos、リテラルなら 0
    pos: u16,
    /// マッチなら len、リテラルなら 1
    len: u8,
}

impl From<LeafToken> for UniToken {
    fn from(t: LeafToken) -> Self {
        match t {
            LeafToken::Literal(b) => UniToken {
                is_match: false,
                lit: b,
                pos: 0,
                len: 1,
            },
            LeafToken::Match { pos, len } => UniToken {
                is_match: true,
                lit: 0,
                pos,
                len,
            },
        }
    }
}

impl From<OkuToken> for UniToken {
    fn from(t: OkuToken) -> Self {
        match t {
            OkuToken::Literal(b) => UniToken {
                is_match: false,
                lit: b,
                pos: 0,
                len: 1,
            },
            OkuToken::Match { pos, len } => UniToken {
                is_match: true,
                lit: 0,
                pos,
                len,
            },
        }
    }
}

/// トークン 1 個ぶん ring buffer に書き込み、write pointer `r` と
/// 入力消費位置 `s` を進める。LF2 デコーダ側の実装と同じ。
fn apply_token(ring: &mut [u8; 0x1000], r: &mut usize, s: &mut usize, input: &[u8], t: &UniToken) {
    if t.is_match {
        let mut copy_pos = t.pos as usize;
        for _ in 0..t.len {
            let pixel = ring[copy_pos];
            ring[*r] = pixel;
            *r = (*r + 1) & 0x0fff;
            copy_pos = (copy_pos + 1) & 0x0fff;
            *s += 1;
            // input は消費するだけ（この関数では使わないが、一貫性のため *s を進める）
        }
        let _ = input; // unused
    } else {
        ring[*r] = t.lit;
        *r = (*r + 1) & 0x0fff;
        *s += 1;
    }
}

/// `byte_offset_at_token` は `token_index` 番目のトークンを読み始める時点で
/// 圧縮ペイロード側がどれだけ進んでいるか。flag byte の読み込み位置を含む。
///
/// LF2 のペイロードは 8 トークン単位で flag byte が先頭に来るので、
/// token i の直前の flag byte を含めて数える。
fn byte_offset_at_token(tokens: &[LeafToken], token_index: usize) -> usize {
    let mut offset = 0usize;
    for (i, t) in tokens.iter().take(token_index).enumerate() {
        if i % 8 == 0 {
            offset += 1; // flag byte
        }
        match t {
            LeafToken::Literal(_) => offset += 1,
            LeafToken::Match { .. } => offset += 2,
        }
    }
    // token_index 自身の flag byte（同じ 8 トークンブロック）も
    // 既にカウント済み（i=token_index-(token_index%8) のとき）
    // 新しい 8-ブロックの先頭 token のときは上のループでカウントしていない
    if token_index % 8 == 0 {
        offset += 1;
    }
    offset
}

struct DivergenceInfo {
    token_index: usize,
    byte_offset: usize,
    /// input 位置 s（= x,y の元）
    s: usize,
    ring: Box<[u8; 0x1000]>,
    ring_r: usize,
    leaf: UniToken,
    oku: UniToken,
}

fn find_first_divergence(
    leaf_tokens: &[LeafToken],
    oku_tokens: &[OkuToken],
    input: &[u8],
) -> Option<DivergenceInfo> {
    let mut ring = Box::new([0x20u8; 0x1000]);
    let mut r: usize = 0x0fee;
    let mut s_leaf: usize = 0;

    let n = leaf_tokens.len().min(oku_tokens.len());
    for i in 0..n {
        let lt: UniToken = leaf_tokens[i].into();
        let ot: UniToken = oku_tokens[i].into();

        let same = lt.is_match == ot.is_match
            && lt.lit == ot.lit
            && lt.pos == ot.pos
            && lt.len == ot.len;

        if !same {
            return Some(DivergenceInfo {
                token_index: i,
                byte_offset: byte_offset_at_token(leaf_tokens, i),
                s: s_leaf,
                ring,
                ring_r: r,
                leaf: lt,
                oku: ot,
            });
        }

        // 一致していれば ring を更新
        let mut s = s_leaf;
        apply_token(&mut *ring, &mut r, &mut s, input, &lt);
        s_leaf = s;
    }

    // 長さが違う場合（片方が長い）
    if leaf_tokens.len() != oku_tokens.len() {
        // 短い方の末尾まで一致した後の余剰トークン
        let i = n;
        if i < leaf_tokens.len() {
            let lt: UniToken = leaf_tokens[i].into();
            let dummy_ot = UniToken {
                is_match: false,
                lit: 0xFE,
                pos: 0xFFFF,
                len: 0,
            };
            return Some(DivergenceInfo {
                token_index: i,
                byte_offset: byte_offset_at_token(leaf_tokens, i),
                s: s_leaf,
                ring,
                ring_r: r,
                leaf: lt,
                oku: dummy_ot,
            });
        }
        if i < oku_tokens.len() {
            let ot: UniToken = oku_tokens[i].into();
            let dummy_lt = UniToken {
                is_match: false,
                lit: 0xFE,
                pos: 0xFFFF,
                len: 0,
            };
            return Some(DivergenceInfo {
                token_index: i,
                byte_offset: byte_offset_at_token(leaf_tokens, i),
                s: s_leaf,
                ring,
                ring_r: r,
                leaf: dummy_lt,
                oku: ot,
            });
        }
    }
    None
}

struct AnalyzeResult {
    token_match: bool,
    byte_match: bool,
    divergence: Option<DivergenceResult>,
}

struct DivergenceResult {
    info: DivergenceInfo,
    candidates: Vec<MatchCandidate>,
    leaf_in_candidates: bool,
    is_tail_overrun: bool,
    max_candidate_len: u8,
    same_len_different_pos_count: usize,
    longer_than_leaf_count: usize,
}

fn encode_okumura_tokens(tokens: &[OkuToken]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let block = &tokens[i..tokens.len().min(i + 8)];
        let mut flag = 0u8;
        for (bit, token) in block.iter().enumerate() {
            if matches!(token, OkuToken::Literal(_)) {
                flag |= 1 << (7 - bit);
            }
        }
        out.push(flag ^ 0xff);
        for token in block {
            match token {
                OkuToken::Literal(b) => out.push(*b ^ 0xff),
                OkuToken::Match { pos, len } => {
                    let upper = ((len - 3) & 0x0f) | (((pos & 0x0f) as u8) << 4);
                    let lower = ((pos >> 4) & 0xff) as u8;
                    out.push(upper ^ 0xff);
                    out.push(lower ^ 0xff);
                }
            }
        }
        i += block.len();
    }
    out
}

fn analyze(data: &[u8]) -> anyhow::Result<(AnalyzeResult, Header)> {
    let hdr = parse_header(data)?;
    let payload = &data[hdr.payload_start..];
    let leaf_decode = decompress_to_tokens(payload, hdr.width, hdr.height)?;
    let oku_tokens = compress_okumura(&leaf_decode.ring_input);

    let byte_match = payload == encode_okumura_tokens(&oku_tokens);
    let token_match = leaf_decode.tokens.len() == oku_tokens.len()
        && leaf_decode
            .tokens
            .iter()
            .zip(oku_tokens.iter())
            .all(|(l, o)| {
                let lu: UniToken = (*l).into();
                let ou: UniToken = (*o).into();
                lu.is_match == ou.is_match
                    && lu.lit == ou.lit
                    && lu.pos == ou.pos
                    && lu.len == ou.len
            });

    if token_match {
        return Ok((
            AnalyzeResult {
                token_match: true,
                byte_match,
                divergence: None,
            },
            hdr,
        ));
    }

    let div = find_first_divergence(&leaf_decode.tokens, &oku_tokens, &leaf_decode.ring_input);
    let div = match div {
        Some(d) => d,
        None => {
            // ここに来るのは同じ長さ・同じ内容なはず。ここに落ちたら内部矛盾
            return Ok((
                AnalyzeResult {
                    token_match: true,
                    byte_match,
                    divergence: None,
                },
                hdr,
            ));
        }
    };

    // 候補列挙
    let candidates = enumerate_match_candidates_with_writeback(
        &*div.ring,
        &leaf_decode.ring_input,
        div.s,
        div.ring_r,
    );

    let leaf_in_candidates = if div.leaf.is_match {
        candidates.iter().any(|c| c.pos == div.leaf.pos && c.len == div.leaf.len)
    } else {
        // リテラルなら「候補集合に含まれる」という問いは不適切。false 扱い
        false
    };
    let remaining = leaf_decode.ring_input.len().saturating_sub(div.s);
    let is_tail_overrun = div.leaf.is_match && (div.leaf.len as usize) > remaining;
    let max_candidate_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let same_len_different_pos_count = if div.leaf.is_match {
        candidates
            .iter()
            .filter(|c| c.len == div.leaf.len && c.pos != div.leaf.pos)
            .count()
    } else {
        0
    };
    let longer_than_leaf_count = if div.leaf.is_match {
        candidates.iter().filter(|c| c.len > div.leaf.len).count()
    } else {
        candidates.len() // リテラル選択時に 3+ のマッチが取れたなら全部「より長い」
    };

    Ok((
        AnalyzeResult {
            token_match: false,
            byte_match,
            divergence: Some(DivergenceResult {
                info: div,
                candidates,
                leaf_in_candidates,
                is_tail_overrun,
                max_candidate_len,
                same_len_different_pos_count,
                longer_than_leaf_count,
            }),
        },
        hdr,
    ))
}

fn uni_kind_str(t: &UniToken) -> &'static str {
    if t.is_match {
        "match"
    } else {
        "lit"
    }
}

fn print_single(path: &Path) -> anyhow::Result<()> {
    let data = fs::read(path)?;
    let (res, hdr) = analyze(&data)?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");

    println!("File: {}", name);
    println!("Image: {}x{}, color_count={}", hdr.width, hdr.height, hdr.color_count);
    println!("Token match: {}", if res.token_match { "YES" } else { "NO" });
    println!("Byte match:  {}", if res.byte_match { "YES" } else { "NO" });

    let div = match res.divergence {
        None => {
            println!("(no divergence)");
            return Ok(());
        }
        Some(d) => d,
    };

    let x = div.info.s % (hdr.width as usize);
    let y = div.info.s / (hdr.width as usize);

    println!("First divergence at token index: {}", div.info.token_index);
    println!("  Byte offset in payload: {}", div.info.byte_offset);
    println!("  Image position: x={}, y={}", x, y);
    println!("  Ring buffer pos (r): 0x{:04x}", div.info.ring_r);
    println!();
    let leaf_desc = if div.info.leaf.is_match {
        format!(
            "Match {{ pos: 0x{:03x}, len: {} }}",
            div.info.leaf.pos, div.info.leaf.len
        )
    } else {
        format!("Literal(0x{:02x})", div.info.leaf.lit)
    };
    let oku_desc = if div.info.oku.is_match {
        format!(
            "Match {{ pos: 0x{:03x}, len: {} }}",
            div.info.oku.pos, div.info.oku.len
        )
    } else {
        format!("Literal(0x{:02x})", div.info.oku.lit)
    };
    println!("Leaf's token:   {}", leaf_desc);
    println!("Okumura token:  {}", oku_desc);
    println!();
    println!("Candidates at this position (len 3..=18): {} total", div.candidates.len());

    const MAX_PRINT: usize = 40;
    let shown: Vec<&MatchCandidate> = div.candidates.iter().take(MAX_PRINT).collect();
    for c in &shown {
        let mut marker = String::new();
        if div.info.leaf.is_match && c.pos == div.info.leaf.pos && c.len == div.info.leaf.len {
            marker.push_str("  <-- Leaf's choice");
        }
        if div.info.oku.is_match && c.pos == div.info.oku.pos && c.len == div.info.oku.len {
            marker.push_str("  <-- Okumura's choice");
        }
        println!("  pos=0x{:03x} len={}{}", c.pos, c.len, marker);
    }
    if div.candidates.len() > MAX_PRINT {
        println!("  ... ({} more)", div.candidates.len() - MAX_PRINT);
    }

    println!();
    println!(
        "Leaf's choice in candidate set ((pos,len) exact): {}",
        if div.leaf_in_candidates { "YES" } else { "NO" }
    );
    if div.is_tail_overrun {
        println!("Tail overrun: YES (Leaf len extends past EOF; decoder clamps at image end)");
    }
    if div.info.leaf.is_match && div.info.oku.is_match {
        println!(
            "Same position as Okumura? {}",
            if div.info.leaf.pos == div.info.oku.pos { "YES" } else { "NO" }
        );
        println!(
            "Same length as Okumura?   {}",
            if div.info.leaf.len == div.info.oku.len { "YES" } else { "NO" }
        );
    }
    println!("Max candidate len: {}", div.max_candidate_len);
    println!(
        "Longer candidate than Leaf's choice: {} (count)",
        div.longer_than_leaf_count
    );

    Ok(())
}

fn run_histogram(dir: &Path) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort();

    let mut total = 0usize;
    let mut token_perfect = 0usize;
    let mut byte_perfect = 0usize;
    let mut errored = 0usize;
    let mut divergent = 0usize;
    let mut leaf_in = 0usize;
    let mut leaf_out_match = 0usize;
    let mut leaf_out_literal = 0usize;
    let mut tail_overrun = 0usize;

    let mut kind_dist: BTreeMap<(bool, bool), usize> = BTreeMap::new(); // (leaf_is_match, oku_is_match)
    let mut len_delta: BTreeMap<i32, usize> = BTreeMap::new();
    let mut pos_sign: BTreeMap<i8, usize> = BTreeMap::new(); // -1 / 0 / +1
    let mut rows: Vec<String> = Vec::new();

    for path in &entries {
        total += 1;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        let data = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };
        let (res, hdr) = match analyze(&data) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("analyze fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };
        if res.token_match {
            token_perfect += 1;
            if res.byte_match {
                byte_perfect += 1;
            }
            continue;
        }
        let Some(div) = res.divergence else {
            continue;
        };
        divergent += 1;
        let x = div.info.s % (hdr.width as usize);
        let y = div.info.s / (hdr.width as usize);

        let (leaf_pos_s, leaf_len_s) = if div.info.leaf.is_match {
            (format!("0x{:03x}", div.info.leaf.pos), div.info.leaf.len.to_string())
        } else {
            ("-".to_string(), format!("lit:0x{:02x}", div.info.leaf.lit))
        };
        let (oku_pos_s, oku_len_s) = if div.info.oku.is_match {
            (format!("0x{:03x}", div.info.oku.pos), div.info.oku.len.to_string())
        } else {
            ("-".to_string(), format!("lit:0x{:02x}", div.info.oku.lit))
        };

        if div.is_tail_overrun {
            tail_overrun += 1;
        }
        rows.push(format!(
            "{},{},{},{},{},0x{:03x},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            div.info.token_index,
            div.info.byte_offset,
            x,
            y,
            div.info.ring_r,
            uni_kind_str(&div.info.leaf),
            leaf_pos_s,
            leaf_len_s,
            uni_kind_str(&div.info.oku),
            oku_pos_s,
            oku_len_s,
            if div.leaf_in_candidates { 1 } else { 0 },
            if div.is_tail_overrun { 1 } else { 0 },
            div.candidates.len(),
            div.max_candidate_len,
            div.same_len_different_pos_count,
            div.longer_than_leaf_count
        ));

        if div.info.leaf.is_match {
            if div.leaf_in_candidates {
                leaf_in += 1;
            } else {
                leaf_out_match += 1;
            }
        } else {
            leaf_out_literal += 1;
        }

        *kind_dist
            .entry((div.info.leaf.is_match, div.info.oku.is_match))
            .or_insert(0) += 1;

        if div.info.leaf.is_match && div.info.oku.is_match {
            let dlen = div.info.leaf.len as i32 - div.info.oku.len as i32;
            *len_delta.entry(dlen).or_insert(0) += 1;
            let sign = if (div.info.leaf.pos as i32) < (div.info.oku.pos as i32) {
                -1
            } else if div.info.leaf.pos == div.info.oku.pos {
                0
            } else {
                1
            };
            *pos_sign.entry(sign).or_insert(0) += 1;
        }
    }

    println!(
        "# total={},token_perfect={},byte_perfect={},divergent={}",
        total, token_perfect, byte_perfect, divergent
    );
    println!("filename,token_index,byte_offset,x,y,ring_r,leaf_kind,leaf_pos,leaf_len,oku_kind,oku_pos,oku_len,leaf_in_candidates,is_tail_overrun,num_candidates,max_candidate_len,same_len_different_pos_count,longer_than_leaf_count");
    for row in rows {
        println!("{row}");
    }
    eprintln!("---");
    eprintln!("Total files: {}", total);
    eprintln!("Token-perfect: {}", token_perfect);
    eprintln!("Byte-perfect: {}", byte_perfect);
    eprintln!("Divergent: {}", divergent);
    eprintln!("Errors: {}", errored);
    if divergent > 0 {
        let match_divergent = leaf_in + leaf_out_match;
        eprintln!(
            "  Leaf's choice in candidate set (all divergent): {} ({:.1}%)",
            leaf_in,
            100.0 * leaf_in as f64 / divergent as f64
        );
        eprintln!(
            "  Leaf's choice in candidate set (match tokens only): {} / {} ({:.1}%)",
            leaf_in,
            match_divergent,
            if match_divergent > 0 {
                100.0 * leaf_in as f64 / match_divergent as f64
            } else {
                0.0
            }
        );
        eprintln!(
            "  Leaf match not in candidate set: {} ({:.1}%)",
            leaf_out_match,
            100.0 * leaf_out_match as f64 / divergent as f64
        );
        eprintln!(
            "  Leaf literal while Okumura matched: {} ({:.1}%)",
            leaf_out_literal,
            100.0 * leaf_out_literal as f64 / divergent as f64
        );
        eprintln!(
            "  Tail overrun cases: {} ({:.1}%)",
            tail_overrun,
            100.0 * tail_overrun as f64 / divergent as f64
        );
    }
    eprintln!();
    eprintln!("First-diff token kind distribution (leaf / okumura):");
    for ((lm, om), c) in &kind_dist {
        eprintln!(
            "  Leaf={}, Okumura={}: {}",
            if *lm { "match" } else { "literal" },
            if *om { "match" } else { "literal" },
            c
        );
    }
    eprintln!();
    eprintln!("Length delta (Leaf.len - Okumura.len) distribution [match/match only]:");
    for (k, v) in &len_delta {
        eprintln!("  {:+}: {}", k, v);
    }
    eprintln!();
    eprintln!("Position sign (Leaf vs Okumura pos) [match/match only]:");
    for (k, v) in &pos_sign {
        let label = match k {
            -1 => "Leaf smaller",
            0 => "Same pos",
            1 => "Leaf larger",
            _ => "?",
        };
        eprintln!("  {}: {}", label, v);
    }

    Ok(())
}

fn run_full_dataset(dir: &Path, output_csv: &Path) -> anyhow::Result<()> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort();

    let mut all_rows: Vec<String> = Vec::new();
    let mut total_files = 0usize;
    let mut total_tokens = 0usize;
    let mut error_files = 0usize;

    for path in &entries {
        total_files += 1;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        eprintln!("[{}/{}] processing: {}", total_files, entries.len(), name);

        let data = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  read error: {}", e);
                error_files += 1;
                continue;
            }
        };

        let hdr = match parse_header(&data) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  header parse error: {}", e);
                error_files += 1;
                continue;
            }
        };

        let payload = &data[hdr.payload_start..];
        let leaf_decode = match decompress_to_tokens(payload, hdr.width, hdr.height) {
            Ok(ld) => ld,
            Err(e) => {
                eprintln!("  decompress error: {}", e);
                error_files += 1;
                continue;
            }
        };

        // 各トークン位置で全決定点を取り出す
        let mut ring = Box::new([0x20u8; 0x1000]);
        let mut r: usize = 0x0fee;
        let mut s: usize = 0;

        for (token_idx, leaf_token) in leaf_decode.tokens.iter().enumerate() {
            let candidates = enumerate_match_candidates_with_writeback(
                &*ring,
                &leaf_decode.ring_input,
                s,
                r,
            );

            // Leaf の選択がどのインデックスを選んだかを判定
            let (leaf_choice_index, num_candidates, max_candidate_len) =
                if let LeafToken::Match {
                    pos: leaf_pos,
                    len: leaf_len,
                } = leaf_token
                {
                    // 候補の中から Leaf の選択と完全マッチするものを探す
                    let choice_idx = candidates
                        .iter()
                        .position(|c| c.pos == *leaf_pos && c.len == *leaf_len)
                        .map(|i| i as i32)
                        .unwrap_or(-1);
                    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
                    (choice_idx, candidates.len(), max_len)
                } else {
                    // リテラル選択時は -1（候補集合内にない）
                    (-1, candidates.len(), 0)
                };

            let x = s % (hdr.width as usize);
            let y = s / (hdr.width as usize);
            let prev_token_kind = if token_idx > 0 {
                match leaf_decode.tokens[token_idx - 1] {
                    LeafToken::Literal(_) => "literal",
                    LeafToken::Match { .. } => "match",
                }
            } else {
                "none"
            };

            // CSV 行を組み立て。候補ごとに distance, length カラムを追加
            let mut row = format!(
                "{},{},{},{},{},{},{},0x{:04x},{}",
                name, token_idx, leaf_choice_index, num_candidates, max_candidate_len, x, y, r,
                prev_token_kind
            );
            for candidate in &candidates {
                // 候補の「距離」: ring write pointer r から見た過去方向の距離（バイト）
                // pos がどのくらい前のデータか。ring buffer wrap-around を考慮
                let pos_usize = candidate.pos as usize;
                let distance = if pos_usize <= r {
                    r - pos_usize  // pos は r より前（過去）
                } else {
                    (0x1000 - pos_usize) + r  // wrap-around: pos は r をまたいで前
                };
                row.push_str(&format!(",{},{}", distance, candidate.len));
            }
            all_rows.push(row);
            total_tokens += 1;

            // リングバッファを更新（次のトークンのため）
            let ut: UniToken = (*leaf_token).into();
            apply_token(&mut *ring, &mut r, &mut s, &leaf_decode.ring_input, &ut);
        }
    }

    // CSV ファイルに書き込み
    let mut csv_content = String::new();
    csv_content.push_str(
        "filename,token_index,leaf_choice_index,num_candidates,max_candidate_len,\
         image_x,image_y,ring_r,prev_token_kind,\
         candidate_0_distance,candidate_0_length,candidate_1_distance,candidate_1_length\n"
    );
    for row in &all_rows {
        csv_content.push_str(&row);
        csv_content.push('\n');
    }

    fs::write(output_csv, csv_content)?;

    eprintln!("---");
    eprintln!("Total files: {}", total_files);
    eprintln!("Total decision points (tokens): {}", total_tokens);
    eprintln!("Errors: {}", error_files);
    eprintln!("Output CSV: {}", output_csv.display());

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage:");
        eprintln!("  {} <file.LF2>                             # モード A: 単一ファイル詳細", args[0]);
        eprintln!("  {} --histogram <input_dir>                # モード B: ヒストグラム", args[0]);
        eprintln!("  {} --full-dataset <input_dir> <output.csv> # モード C: 全決定点データセット", args[0]);
        return ExitCode::from(2);
    }

    if args[1] == "--histogram" {
        if args.len() != 3 {
            eprintln!("usage: {} --histogram <input_dir>", args[0]);
            return ExitCode::from(2);
        }
        let dir = PathBuf::from(&args[2]);
        if !dir.is_dir() {
            eprintln!("error: {} is not a directory", dir.display());
            return ExitCode::from(2);
        }
        if let Err(e) = run_histogram(&dir) {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    if args[1] == "--full-dataset" {
        if args.len() != 4 {
            eprintln!("usage: {} --full-dataset <input_dir> <output.csv>", args[0]);
            return ExitCode::from(2);
        }
        let dir = PathBuf::from(&args[2]);
        let output = PathBuf::from(&args[3]);
        if !dir.is_dir() {
            eprintln!("error: {} is not a directory", dir.display());
            return ExitCode::from(2);
        }
        if let Err(e) = run_full_dataset(&dir, &output) {
            eprintln!("error: {}", e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    let path = PathBuf::from(&args[1]);
    if let Err(e) = print_single(&path) {
        eprintln!("error: {}", e);
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
