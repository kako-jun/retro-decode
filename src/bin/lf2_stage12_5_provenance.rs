//! Stage 12-5 (Issue #14 脈: 「長 match 消費中の挿入スキップ」仮説の観測ゲート):
//! 各リング位置に「由来タグ」(literal書込み / matchの先頭バイト / match内部
//! バイト / 初期先読み充填 / 未書込み) を、Leaf 実トークン列の teacher forcing
//! だけで復元する。実装変更なし・観測専用 (insert_node のスキップは実装しない)。
//!
//! 由来タグは「最後にその位置に書いたトークン」基準:
//!   - Literal: リテラルで書かれた
//!   - MatchFirst: match の先頭バイト (コピー元 offset 0) として書かれた
//!   - MatchInternal: match の2バイト目以降として書かれた
//!   - InitialLookaheadFill: 初期 F バイト先読み充填のまま一度も上書きされていない
//!   - NeverWritten: 上記のいずれでもない (bootstrap dummy 帯など)
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_5_provenance -- <DIR> \
//!       [--events-tsv PATH] [--baseline-list PATH] [--control-n N] [--out-prefix PREFIX]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Prov {
    NeverWritten,
    InitialLookaheadFill,
    Literal,
    MatchFirst,
    MatchInternal,
}

impl Prov {
    fn name(&self) -> &'static str {
        match self {
            Prov::NeverWritten => "NeverWritten",
            Prov::InitialLookaheadFill => "InitialLookaheadFill",
            Prov::Literal => "Literal",
            Prov::MatchFirst => "MatchFirst",
            Prov::MatchInternal => "MatchInternal",
        }
    }
}

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

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input))
}

/// 由来タグ状態。`prov[pos]` / `offset[pos]` (Match* のときの token 内 offset)。
struct ProvState {
    prov: Vec<Prov>,
    offset: Vec<u8>,
}

impl ProvState {
    fn new() -> Self {
        let mut prov = vec![Prov::NeverWritten; N];
        let offset = vec![0u8; N];
        for k in 0..F {
            let slot = (N - F + k) & (N - 1);
            prov[slot] = Prov::InitialLookaheadFill;
        }
        Self { prov, offset }
    }

    /// 1 token を適用し ring/由来 を進める。r と input_pos を更新して返す。
    fn apply(&mut self, tok: &LeafToken, ring_input: &[u8], r: &mut usize, input_pos: &mut usize) {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        for k in 0..l {
            if *input_pos >= ring_input.len() {
                break;
            }
            self.prov[*r] = match tok {
                LeafToken::Literal(_) => Prov::Literal,
                LeafToken::Match { .. } => {
                    if k == 0 {
                        Prov::MatchFirst
                    } else {
                        Prov::MatchInternal
                    }
                }
            };
            self.offset[*r] = k as u8;
            *r = (*r + 1) & (N - 1);
            *input_pos += 1;
        }
    }
}

/// leaf_tokens[0..di] を teacher forcing 再生し、その時点の由来状態と (r, input_pos) を返す。
fn replay_provenance(leaf_tokens: &[LeafToken], ring_input: &[u8], di: usize) -> (ProvState, usize, usize) {
    let mut st = ProvState::new();
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for tok in leaf_tokens.iter().take(di) {
        st.apply(tok, ring_input, &mut r, &mut input_pos);
    }
    (st, r, input_pos)
}

/// α群7本: (file, div_ti, sim_len, sim_pos) — Stage 10-2 (`.local_data/stage10_2_report.csv`)。
const ALPHA7: [(&str, usize, u8, u16); 7] = [
    ("C1801.LF2", 3064, 18, 2371),
    ("H21.LF2", 1192, 4, 1177),
    ("H43.LF2", 1736, 4, 439),
    ("H91.LF2", 612, 10, 205),
    ("S06E.LF2", 1701, 3, 3050),
    ("V24.LF2", 4517, 5, 446),
    ("V9E.LF2", 1851, 3, 3375),
];

struct TieRow {
    file: String,
    di: usize,
    n_candidates: usize,
    winner: u16,
    loser: u16,
}

fn load_tie_events(path: &str) -> Vec<TieRow> {
    let content = fs::read_to_string(path).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_ncand, i_leafpos, i_simpos) =
        (idx("file"), idx("di"), idx("n_candidates"), idx("leaf_pos"), idx("sim_pos"));
    let mut out = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        out.push(TieRow {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            n_candidates: f[i_ncand].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
            loser: f[i_simpos].parse().unwrap(),
        });
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--events-tsv PATH] [--baseline-list PATH] [--control-n N] [--out-prefix PREFIX]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_tsv = String::from(".local_data/stage12_1_tie_events.tsv");
    let mut baseline_list = String::from(".local_data/stage11_4_baseline203_sorted.txt");
    let mut control_n = 50usize;
    let mut out_prefix = String::from(".local_data/stage12_5");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" => {
                events_tsv = args[i + 1].clone();
                i += 2;
            }
            "--baseline-list" => {
                baseline_list = args[i + 1].clone();
                i += 2;
            }
            "--control-n" => {
                control_n = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--out-prefix" => {
                out_prefix = args[i + 1].clone();
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    // === 1. alpha7: 見逃し候補の窓の由来分布 ===
    eprintln!("=== 1. alpha7: 見逃し候補窓の由来分布 ===");
    let mut alpha_rows: Vec<String> = Vec::new();
    for &(file, div_ti, sim_len, sim_pos) in &ALPHA7 {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
            eprintln!("  WARN load fail {}", file);
            continue;
        };
        let (st, _r, _input_pos) = replay_provenance(&leaf_tokens, &ring_input, div_ti);
        let mut tags: Vec<Prov> = Vec::new();
        for k in 0..sim_len as usize {
            let slot = (sim_pos as usize + k) & (N - 1);
            tags.push(st.prov[slot]);
        }
        let first_tag = tags[0];
        let mut hist: BTreeMap<&'static str, usize> = BTreeMap::new();
        for t in &tags {
            *hist.entry(t.name()).or_insert(0) += 1;
        }
        eprintln!(
            "  {:12} div_ti={:6} sim_pos={:5} sim_len={:2} first_byte_tag={:20} window_hist={:?}",
            file, div_ti, sim_pos, sim_len, first_tag.name(), hist
        );
        alpha_rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{:?}",
            file, div_ti, sim_pos, sim_len, first_tag.name(), hist
        ));
    }

    // === 2 & 4. binary tie 50件 + TIE_SUBF 241件全体: winner/loser 由来クロス表 ===
    let all_tie_events = load_tie_events(&events_tsv);
    let binary_events: Vec<&TieRow> = all_tie_events.iter().filter(|e| e.n_candidates == 2).collect();

    let build_cross_table = |events: &[&TieRow], label: &str, out_path: &str| {
        eprintln!("=== {} (n={}) ===", label, events.len());
        let mut cross: BTreeMap<(&'static str, &'static str), usize> = BTreeMap::new();
        let mut winner_marg: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut loser_marg: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut rows: Vec<String> = Vec::new();
        for ev in events {
            let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
                continue;
            };
            let (st, _r, _ip) = replay_provenance(&leaf_tokens, &ring_input, ev.di);
            let wp = st.prov[(ev.winner as usize) & (N - 1)];
            let lp = st.prov[(ev.loser as usize) & (N - 1)];
            *cross.entry((wp.name(), lp.name())).or_insert(0) += 1;
            *winner_marg.entry(wp.name()).or_insert(0) += 1;
            *loser_marg.entry(lp.name()).or_insert(0) += 1;
            rows.push(format!("{}\t{}\t{}\t{}\t{}\t{}", ev.file, ev.di, ev.winner, ev.loser, wp.name(), lp.name()));
        }
        eprintln!("  winner marginal: {:?}", winner_marg);
        eprintln!("  loser  marginal: {:?}", loser_marg);
        eprintln!("  cross (winner_tag, loser_tag) -> count:");
        for (k, v) in &cross {
            eprintln!("    {:?}: {}", k, v);
        }
        if let Ok(mut f) = fs::File::create(out_path) {
            writeln!(f, "file\tdi\twinner\tloser\twinner_tag\tloser_tag").ok();
            for r in &rows {
                writeln!(f, "{}", r).ok();
            }
        }
        (winner_marg, loser_marg)
    };

    let (_bw, binary_loser_marg) = build_cross_table(
        &binary_events,
        "2. binary tie 50件 由来クロス表",
        &format!("{}_binarytie_provenance.tsv", out_prefix),
    );
    let all_refs: Vec<&TieRow> = all_tie_events.iter().collect();
    let (_tw, tiesubf_loser_marg) = build_cross_table(
        &all_refs,
        "4. TIE_SUBF 241件全体 由来クロス表",
        &format!("{}_tiesubf241_provenance.tsv", out_prefix),
    );

    // === 3. 対照群: byte-exact 203本からサンプルNで採用候補の由来分布 ===
    eprintln!("=== 3. 対照群 (byte-exact サンプル{}本): 採用match候補の由来分布 ===", control_n);
    let baseline_names: Vec<String> = fs::read_to_string(&baseline_list)
        .expect("read baseline list")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let step = (baseline_names.len() as f64 / control_n as f64).ceil().max(1.0) as usize;
    let sample: Vec<&String> = baseline_names.iter().step_by(step).take(control_n).collect();
    eprintln!("  sample files: {} (step={})", sample.len(), step);

    let mut accepted_hist: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut accepted_rows: Vec<String> = Vec::new();
    let mut internal_offsets: Vec<u8> = Vec::new();
    let mut internal_lens: Vec<u8> = Vec::new();

    for name in &sample {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, name) else {
            eprintln!("  WARN load fail {}", name);
            continue;
        };
        let mut st = ProvState::new();
        let mut r: usize = N - F;
        let mut input_pos: usize = 0;
        for tok in &leaf_tokens {
            if let LeafToken::Match { pos, len } = tok {
                let pos_u = (*pos as usize) & (N - 1);
                let tag = st.prov[pos_u];
                let off = st.offset[pos_u];
                *accepted_hist.entry(tag.name()).or_insert(0) += 1;
                accepted_rows.push(format!("{}\t{}\t{}\t{}\t{}", name, input_pos, pos_u, len, tag.name()));
                if tag == Prov::MatchInternal {
                    internal_offsets.push(off);
                    internal_lens.push(*len);
                }
            }
            st.apply(tok, &ring_input, &mut r, &mut input_pos);
        }
    }
    eprintln!("  accepted match candidates: {}", accepted_rows.len());
    eprintln!("  accepted provenance hist: {:?}", accepted_hist);
    if !internal_offsets.is_empty() {
        let sum_off: u64 = internal_offsets.iter().map(|&x| x as u64).sum();
        let sum_len: u64 = internal_lens.iter().map(|&x| x as u64).sum();
        eprintln!(
            "  MatchInternal 採用側: n={} mean_offset_in_source_token={:.2} mean_source_len={:.2}",
            internal_offsets.len(),
            sum_off as f64 / internal_offsets.len() as f64,
            sum_len as f64 / internal_lens.len() as f64
        );
    }
    if let Ok(mut f) = fs::File::create(format!("{}_control_provenance.tsv", out_prefix)) {
        writeln!(f, "file\tinput_pos\tpos\tlen\ttag").ok();
        for r in &accepted_rows {
            writeln!(f, "{}", r).ok();
        }
    }

    // === 判定 ===
    eprintln!("=== 判定 (loser/見逃し側 vs 採用側の MatchInternal 比率) ===");
    let accepted_total: usize = accepted_hist.values().sum();
    let accepted_internal = *accepted_hist.get("MatchInternal").unwrap_or(&0);
    let accepted_rate = accepted_internal as f64 / accepted_total.max(1) as f64;
    let loser_total: usize = binary_loser_marg.values().sum();
    let loser_internal = *binary_loser_marg.get("MatchInternal").unwrap_or(&0);
    let loser_rate = loser_internal as f64 / loser_total.max(1) as f64;
    let tie_loser_total: usize = tiesubf_loser_marg.values().sum();
    let tie_loser_internal = *tiesubf_loser_marg.get("MatchInternal").unwrap_or(&0);
    let tie_loser_rate = tie_loser_internal as f64 / tie_loser_total.max(1) as f64;

    eprintln!(
        "  accepted(対照203サンプル) MatchInternal率: {}/{} = {:.1}%",
        accepted_internal, accepted_total, 100.0 * accepted_rate
    );
    eprintln!(
        "  loser(binary tie 50件)   MatchInternal率: {}/{} = {:.1}%  (比 {:.2}x)",
        loser_internal, loser_total, 100.0 * loser_rate,
        if accepted_rate > 0.0 { loser_rate / accepted_rate } else { f64::INFINITY }
    );
    eprintln!(
        "  loser(TIE_SUBF 241件)    MatchInternal率: {}/{} = {:.1}%  (比 {:.2}x)",
        tie_loser_internal, tie_loser_total, 100.0 * tie_loser_rate,
        if accepted_rate > 0.0 { tie_loser_rate / accepted_rate } else { f64::INFINITY }
    );

    ExitCode::SUCCESS
}
