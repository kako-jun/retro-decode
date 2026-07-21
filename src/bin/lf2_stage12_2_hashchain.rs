//! Stage 12-2a (Issue #14 脈3): hash-chain 仮説の安価な生死判定。
//!
//! 受け入れ条件: 候補構造 (3バイトhash + チェイン深さ D 打ち切り) が
//! α群7本 (C1801/H21/H43/H91/S06E/V24/V9E) の「実書込み域にある正当な
//! 同長候補をLeafが見逃した」事例では**全て深さ D を超えて漏れ**、かつ
//! byte-exact 203本 (`.local_data/stage11_4_baseline203_sorted.txt`) で
//! Leaf が実際に採用した全 Match 候補は**深さ D 以内で発見できる**、を
//! 同時に満たす (hash関数, mask, D) の組があるかを数え上げるだけの
//! 観測専用ツール (実装変更なし)。
//!
//! チェインモデル: 「挿入=書込み時」。ring 上の各 written スロット pos の
//! 3バイトキー hash(ring[pos..pos+3]) でバケツ分けし、同バケツ内を
//! write_tick 降順 (最新が rank 1) に並べたときの順位をチェイン深さとする。
//! 未書込み (write_tick 未設定) スロットはチェイン対象外 (真の「挿入」が
//! 起きていないため)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_2_hashchain -- <DIR> \
//!       [--baseline-list PATH] [--out-tsv PATH]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, compress_okumura_tail_plus1, Token, F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

const ALPHA_FILES: [&str; 7] = ["C1801.LF2", "H21.LF2", "H43.LF2", "H91.LF2", "S06E.LF2", "V24.LF2", "V9E.LF2"];

#[derive(Clone, Copy, Debug)]
enum HashFn {
    H1, // ((b0<<4)^(b1<<2)^b2) & mask
    H2, // (((b0<<5)^b1)<<5)^b2 & mask (deflate 風 rolling)
    H3, // ((b0<<8|b1)^(b2<<4)) & mask
}

impl HashFn {
    fn name(&self) -> &'static str {
        match self {
            HashFn::H1 => "h1_shift4_2_0",
            HashFn::H2 => "h2_deflate_roll",
            HashFn::H3 => "h3_b0b1_xor_b2",
        }
    }
    fn raw(&self, b0: u32, b1: u32, b2: u32) -> u32 {
        match self {
            HashFn::H1 => (b0 << 4) ^ (b1 << 2) ^ b2,
            HashFn::H2 => (((b0 << 5) ^ b1) << 5) ^ b2,
            HashFn::H3 => ((b0 << 8) | b1) ^ (b2 << 4),
        }
    }
}

const MASKS: [u32; 3] = [0xFFF, 0x7FF, 0x3FF];
const ALL_HASHES: [HashFn; 3] = [HashFn::H1, HashFn::H2, HashFn::H3];

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

fn pick_sim(ring_input: &[u8], orig_payload: &[u8]) -> Vec<Token> {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    if orig_payload == clip_reenc.as_slice() {
        clip_tokens
    } else {
        compress_okumura_tail_plus1(ring_input)
    }
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

/// pos から始まる len バイト全てが write_tick 済み (真の書込み域) の occurrence を列挙。
fn find_genuine_occurrences(ring: &[u8; N], write_tick: &[u32; N], target: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if target.is_empty() {
        return out;
    }
    'outer: for pos in 0..N {
        for (k, &tb) in target.iter().enumerate() {
            let slot = (pos + k) & (N - 1);
            if ring[slot] != tb || write_tick[slot] == u32::MAX {
                continue 'outer;
            }
        }
        out.push(pos);
    }
    out
}

/// 3バイトhash+maskで、pos のチェイン内順位 (1=最新) を返す。
/// pos 自身が未書込みなら None。
fn chain_rank(ring: &[u8; N], write_tick: &[u32; N], pos: usize, hf: HashFn, mask: u32) -> Option<usize> {
    if write_tick[pos] == u32::MAX {
        return None;
    }
    let target_tick = write_tick[pos];
    let b0 = ring[pos] as u32;
    let b1 = ring[(pos + 1) & (N - 1)] as u32;
    let b2 = ring[(pos + 2) & (N - 1)] as u32;
    let target_hash = hf.raw(b0, b1, b2) & mask;

    let mut newer_count = 0usize;
    for p in 0..N {
        let t = write_tick[p];
        if t == u32::MAX || t <= target_tick {
            continue;
        }
        let c0 = ring[p] as u32;
        let c1 = ring[(p + 1) & (N - 1)] as u32;
        let c2 = ring[(p + 2) & (N - 1)] as u32;
        let h = hf.raw(c0, c1, c2) & mask;
        if h == target_hash {
            newer_count += 1;
        }
    }
    Some(1 + newer_count)
}

struct AlphaResult {
    file: String,
    div_ti: usize,
    input_pos: usize,
    sim_len: u8,
    sim_pos: u16,
    n_genuine_occ: usize,
    // per (hash,mask) -> min rank across genuine occurrences (None if no genuine occ)
    min_rank: Vec<Option<usize>>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--baseline-list PATH] [--out-tsv PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut baseline_list = String::from(".local_data/stage11_4_baseline203_sorted.txt");
    let mut out_tsv = String::from(".local_data/stage12_2_hashchain.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--baseline-list" => {
                if let Some(v) = args.get(i + 1) {
                    baseline_list = v.clone();
                }
                i += 2;
            }
            "--out-tsv" => {
                if let Some(v) = args.get(i + 1) {
                    out_tsv = v.clone();
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let combos: Vec<(HashFn, u32)> = ALL_HASHES
        .iter()
        .flat_map(|&hf| MASKS.iter().map(move |&m| (hf, m)))
        .collect();

    // --- alpha7: 見逃し候補のチェイン順位 (per combo, min across genuine occurrences) ---
    let mut alpha_results: Vec<AlphaResult> = Vec::new();
    for name in ALPHA_FILES.iter() {
        let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
            eprintln!("WARN: failed to load {}", name);
            continue;
        };
        let sim_tokens = pick_sim(&ring_input, &orig_payload);

        let mut di = None;
        for (ti, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
            let same = match (a, b) {
                (LeafToken::Literal(x), Token::Literal(y)) => x == y,
                (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                    p1 == p2 && l1 == l2
                }
                _ => false,
            };
            if !same {
                di = Some(ti);
                break;
            }
        }
        let Some(di) = di else {
            eprintln!("WARN: {} NO_DIFF (unexpected for alpha group)", name);
            continue;
        };

        let mut ring = [0x20u8; N];
        let mut write_tick = [u32::MAX; N];
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
                write_tick[r] = input_pos as u32;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
        }

        let (sim_len, sim_pos): (u8, u16) = match &sim_tokens[di] {
            Token::Match { pos, len } => (*len, *pos),
            Token::Literal(_) => {
                eprintln!("WARN: {} sim token at di is Literal, unexpected", name);
                continue;
            }
        };
        let target: Vec<u8> =
            ring_input[input_pos..(input_pos + sim_len as usize).min(ring_input.len())].to_vec();
        let occurrences = find_genuine_occurrences(&ring, &write_tick, &target);

        let mut min_rank: Vec<Option<usize>> = Vec::with_capacity(combos.len());
        for &(hf, mask) in &combos {
            let mut best: Option<usize> = None;
            for &occ in &occurrences {
                if let Some(rk) = chain_rank(&ring, &write_tick, occ, hf, mask) {
                    best = Some(best.map_or(rk, |b: usize| b.min(rk)));
                }
            }
            min_rank.push(best);
        }

        alpha_results.push(AlphaResult {
            file: name.to_string(),
            div_ti: di,
            input_pos,
            sim_len,
            sim_pos,
            n_genuine_occ: occurrences.len(),
            min_rank,
        });
    }

    // --- 203本 byte-exact: Leaf が実際に採用した全 Match のチェイン順位の最大値 ---
    let baseline_names: Vec<String> = fs::read_to_string(&baseline_list)
        .expect("read baseline list")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut max_needed_rank: Vec<usize> = vec![0; combos.len()];
    let mut skipped_dummy_matches = 0usize;
    let mut total_matches_checked = 0usize;
    let mut worst_examples: Vec<Option<(String, usize, usize)>> = vec![None; combos.len()]; // (file, input_pos, rank)

    for name in &baseline_names {
        let Some((leaf_tokens, ring_input, _orig_payload)) = load_file(&dir, name) else {
            eprintln!("WARN: failed to load baseline {}", name);
            continue;
        };
        let mut ring = [0x20u8; N];
        let mut write_tick = [u32::MAX; N];
        let mut r: usize = N - F;
        let mut input_pos: usize = 0;

        for tok in leaf_tokens.iter() {
            if let LeafToken::Match { pos, len } = tok {
                let pos_u = (*pos as usize) & (N - 1);
                if write_tick[pos_u] != u32::MAX {
                    total_matches_checked += 1;
                    for (ci, &(hf, mask)) in combos.iter().enumerate() {
                        if let Some(rk) = chain_rank(&ring, &write_tick, pos_u, hf, mask) {
                            if rk > max_needed_rank[ci] {
                                max_needed_rank[ci] = rk;
                                worst_examples[ci] = Some((name.clone(), input_pos, rk));
                            }
                        }
                    }
                } else {
                    skipped_dummy_matches += 1;
                }
            }
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            for _ in 0..l {
                if input_pos >= ring_input.len() {
                    break;
                }
                ring[r] = ring_input[input_pos];
                write_tick[r] = input_pos as u32;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
        }
    }

    // --- TSV 出力 (alpha7 detail) ---
    if let Ok(mut f) = fs::File::create(&out_tsv) {
        write!(f, "file\tdiv_ti\tinput_pos\tsim_len\tsim_pos\tn_genuine_occ").ok();
        for &(hf, mask) in &combos {
            write!(f, "\t{}_{:#x}_min_rank", hf.name(), mask).ok();
        }
        writeln!(f).ok();
        for a in &alpha_results {
            write!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{}",
                a.file, a.div_ti, a.input_pos, a.sim_len, a.sim_pos, a.n_genuine_occ
            )
            .ok();
            for r in &a.min_rank {
                write!(f, "\t{}", r.map(|x| x as i64).unwrap_or(-1)).ok();
            }
            writeln!(f).ok();
        }
    }

    // --- 判定サマリ ---
    eprintln!("alpha7 files analyzed: {}/{}", alpha_results.len(), ALPHA_FILES.len());
    eprintln!(
        "baseline203 matches checked (written pos): {} (skipped dummy-pos matches: {})",
        total_matches_checked, skipped_dummy_matches
    );
    eprintln!("---");
    eprintln!("{:20} {:>8} | {:>14} | {:>7} verdict", "hash_mask", "max203D", "min_alpha_rank", "gap");
    for (ci, &(hf, mask)) in combos.iter().enumerate() {
        let m203 = max_needed_rank[ci];
        let alpha_mins: Vec<Option<usize>> = alpha_results.iter().map(|a| a.min_rank[ci]).collect();
        // hypothesis 生存には alpha7 全件で genuine occurrence が存在し、かつその best rank が m203 より大きい必要がある
        let all_present = alpha_mins.iter().all(|x| x.is_some());
        let min_alpha = alpha_mins.iter().filter_map(|x| *x).min();
        let verdict = match (all_present, min_alpha) {
            (true, Some(ma)) if ma > m203 => "SURVIVES",
            (true, Some(_)) => "DEAD (alpha reachable within D)",
            _ => "N/A (alpha has no genuine occurrence for this combo)",
        };
        eprintln!(
            "{:20} {:>8} | {:>14} | {}",
            format!("{}_{:#x}", hf.name(), mask),
            m203,
            min_alpha.map(|x| x as i64).unwrap_or(-1),
            verdict
        );
        if let Some((f, ip, rk)) = &worst_examples[ci] {
            eprintln!("    worst203 example: {} input_pos={} rank={}", f, ip, rk);
        }
    }
    eprintln!("---");
    eprintln!("alpha7 detail (per file, per combo min_rank; -1 = no genuine occurrence found):");
    for a in &alpha_results {
        eprintln!(
            "  {} div_ti={} input_pos={} sim_len={} sim_pos={} n_genuine_occ={}",
            a.file, a.div_ti, a.input_pos, a.sim_len, a.sim_pos, a.n_genuine_occ
        );
        for (ci, &(hf, mask)) in combos.iter().enumerate() {
            eprintln!(
                "    {}_{:#x}: min_rank={:?}",
                hf.name(),
                mask,
                a.min_rank[ci]
            );
        }
    }
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
