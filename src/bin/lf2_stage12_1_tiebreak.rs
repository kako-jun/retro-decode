//! Stage 12-1 (Issue #14 脈1): 「r に近い方を選ぶ」tie-break 仮説の全数計測。
//!
//! Stage 10 と同じ per-file フォールバック (Clip が byte-exact ならそれ、
//! でなければ Plus1) で Sim を決め、Leaf との最初の divergence を分類する
//! (Stage 10 の class 定義をそのまま踏襲)。TIE_SUBF (max_len<F の tie で
//! Leaf/Sim の位置が食い違う) の各イベントで、その時点のリング内の同じ
//! 長さ (max_len) を持つ全候補を brute-force 列挙し、各候補にリング距離
//! `dist = (r - pos) mod N` を付けて、Leaf 採用位置・Sim 採用位置がその
//! 距離順で何位かを記録する。実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_1_tiebreak -- <DIR> [--limit N] \
//!       [--out-tsv PATH] [--only-tie-subf]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken, MatchCandidate,
};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, compress_okumura_tail_plus1, Token, F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const BOOTSTRAP_DUMMY_LO: usize = N - F - F; // 4060
const BOOTSTRAP_DUMMY_HI: usize = N - F - 1; // 4077

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

/// Stage 9-2d / Stage 10 と同じ選択規則: Clip が byte-exact ならそれを Sim
/// に、でなければ Plus1 を Sim にする。
fn pick_sim(ring_input: &[u8], orig_payload: &[u8]) -> (Vec<Token>, &'static str) {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    if orig_payload == clip_reenc.as_slice() {
        (clip_tokens, "clip")
    } else {
        (compress_okumura_tail_plus1(ring_input), "plus1")
    }
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

/// ring 距離 (r - pos) mod N。0 = 直近書込み (もっとも新しい)。
fn ring_dist(r: usize, pos: usize) -> usize {
    (r + N - (pos & (N - 1))) & (N - 1)
}

/// 与えられた candidate 集合中で `dist` 昇順の順位 (1 始まり、同点は同順位)
/// を返す。
fn rank_by_dist(candidates: &[(MatchCandidate, usize)], target_pos: u16) -> Option<(usize, usize)> {
    let target_dist = candidates
        .iter()
        .find(|(c, _)| c.pos == target_pos)
        .map(|(_, d)| *d)?;
    let rank = 1 + candidates.iter().filter(|(_, d)| *d < target_dist).count();
    Some((target_dist, rank))
}

struct TieEvent {
    file: String,
    mode: &'static str,
    di: usize,
    input_pos: usize,
    len: u8,
    n_candidates: usize,
    leaf_pos: u16,
    leaf_dist: usize,
    leaf_rank: usize,
    sim_pos: u16,
    sim_dist: usize,
    sim_rank: usize,
    leaf_is_dist_min: bool,
    leaf_write_tick_set: bool,
    leaf_in_bootstrap_band: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--limit N] [--out-tsv PATH]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut limit: Option<usize> = None;
    let mut out_tsv = String::from(".local_data/stage12_1_tie_events.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                limit = args.get(i + 1).and_then(|v| v.parse().ok());
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

    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("LF2"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    if let Some(n) = limit {
        files.truncate(n);
    }

    let mut class_counts: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut tie_events: Vec<TieEvent> = Vec::new();
    let mut errors = 0usize;
    let mut total = 0usize;

    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, &name) else {
            errors += 1;
            continue;
        };
        total += 1;
        let (sim_tokens, mode) = pick_sim(&ring_input, &orig_payload);

        let mut di = None;
        for (i2, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
            let same = match (a, b) {
                (LeafToken::Literal(x), Token::Literal(y)) => x == y,
                (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                    p1 == p2 && l1 == l2
                }
                _ => false,
            };
            if !same {
                di = Some(i2);
                break;
            }
        }
        let Some(di) = di else {
            *class_counts.entry("MATCH").or_insert(0) += 1;
            continue;
        };

        // teacher forcing で di 直前まで shadow ring + write_tick を再現
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

        let candidates = enumerate_match_candidates_with_writeback(&ring, &ring_input, input_pos, r);
        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);

        let class = match (&leaf_tokens[di], &sim_tokens[di]) {
            (LeafToken::Match { pos: lp, len: ll }, Token::Match { len: ml, .. }) => {
                let leaf_in = candidates.iter().any(|c| c.pos == *lp && c.len == *ll);
                if !leaf_in {
                    "LEAF_NOT_CAND"
                } else if ll != ml {
                    "LEN_DIFF"
                } else if *ll as usize == F {
                    "TIE_F"
                } else if *ll == max_len {
                    "TIE_SUBF"
                } else {
                    "LEN_DIFF"
                }
            }
            (LeafToken::Literal(_), Token::Literal(_)) => "KIND_DIFF",
            _ => "KIND_DIFF",
        };
        *class_counts.entry(class).or_insert(0) += 1;

        if class == "TIE_SUBF" {
            let (LeafToken::Match { pos: leaf_pos, len }, Token::Match { pos: sim_pos, .. }) =
                (&leaf_tokens[di], &sim_tokens[di])
            else {
                unreachable!()
            };
            let subset: Vec<(MatchCandidate, usize)> = candidates
                .iter()
                .filter(|c| c.len == max_len)
                .map(|c| (*c, ring_dist(r, c.pos as usize)))
                .collect();
            let Some((leaf_dist, leaf_rank)) = rank_by_dist(&subset, *leaf_pos) else {
                // 理論上起こらない (leaf_in==true で確認済み) が保険
                continue;
            };
            let (sim_dist, sim_rank) = rank_by_dist(&subset, *sim_pos).unwrap_or((usize::MAX, 0));

            let leaf_pos_u = (*leaf_pos as usize) & (N - 1);
            let leaf_write_tick_set = write_tick[leaf_pos_u] != u32::MAX;
            let leaf_in_bootstrap_band =
                leaf_pos_u >= BOOTSTRAP_DUMMY_LO && leaf_pos_u <= BOOTSTRAP_DUMMY_HI;

            tie_events.push(TieEvent {
                file: name.clone(),
                mode,
                di,
                input_pos,
                len: *len,
                n_candidates: subset.len(),
                leaf_pos: *leaf_pos,
                leaf_dist,
                leaf_rank,
                sim_pos: *sim_pos,
                sim_dist,
                sim_rank,
                leaf_is_dist_min: leaf_rank == 1,
                leaf_write_tick_set,
                leaf_in_bootstrap_band,
            });
        }
    }

    // TSV 出力
    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(
            f,
            "file\tmode\tdi\tinput_pos\tlen\tn_candidates\tleaf_pos\tleaf_dist\tleaf_rank\tsim_pos\tsim_dist\tsim_rank\tleaf_is_dist_min\tleaf_write_tick_set\tleaf_in_bootstrap_band"
        )
        .ok();
        for e in &tie_events {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                e.file,
                e.mode,
                e.di,
                e.input_pos,
                e.len,
                e.n_candidates,
                e.leaf_pos,
                e.leaf_dist,
                e.leaf_rank,
                e.sim_pos,
                e.sim_dist,
                e.sim_rank,
                e.leaf_is_dist_min,
                e.leaf_write_tick_set,
                e.leaf_in_bootstrap_band
            )
            .ok();
        }
    }

    // サマリ
    eprintln!("files: {} (errors {})", total, errors);
    eprintln!("class counts:");
    for (k, v) in &class_counts {
        eprintln!("  {:15}: {}", k, v);
    }
    eprintln!("---");
    eprintln!("TIE_SUBF events analyzed: {}", tie_events.len());
    let n_leaf_min = tie_events.iter().filter(|e| e.leaf_is_dist_min).count();
    eprintln!(
        "leaf dist-min rate: {}/{} ({:.2}%)",
        n_leaf_min,
        tie_events.len(),
        100.0 * n_leaf_min as f64 / tie_events.len().max(1) as f64
    );
    // sim rank distribution
    let mut sim_rank_hist: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for e in &tie_events {
        *sim_rank_hist.entry(e.sim_rank).or_insert(0) += 1;
    }
    eprintln!("sim rank distribution (rank -> count):");
    for (k, v) in &sim_rank_hist {
        eprintln!("  rank {:3}: {}", k, v);
    }
    // leaf-not-min の特徴
    let not_min: Vec<&TieEvent> = tie_events.iter().filter(|e| !e.leaf_is_dist_min).collect();
    eprintln!("leaf NOT dist-min: {} events", not_min.len());
    let mut len_hist: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
    let mut band_count = 0usize;
    let mut wt_set_count = 0usize;
    let mut leaf_rank_hist: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for e in &not_min {
        *len_hist.entry(e.len).or_insert(0) += 1;
        if e.leaf_in_bootstrap_band {
            band_count += 1;
        }
        if e.leaf_write_tick_set {
            wt_set_count += 1;
        }
        *leaf_rank_hist.entry(e.leaf_rank).or_insert(0) += 1;
    }
    eprintln!("  len histogram: {:?}", len_hist);
    eprintln!("  in_bootstrap_band: {}/{}", band_count, not_min.len());
    eprintln!("  write_tick_set (already written): {}/{}", wt_set_count, not_min.len());
    eprintln!("  leaf_rank histogram: {:?}", leaf_rank_hist);
    eprintln!("---");
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
