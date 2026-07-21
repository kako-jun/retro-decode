//! Stage 12-2b (Issue #14 脈1 follow-up): binary tie (n_candidates==2) 50件の
//! 判別特徴マイニング。距離 (ring dist) は無相関 (50%) と判明済みなので、
//! 別の決定因を機械特徴で探す。実装変更なし・観測専用。
//!
//! 入力: `lf2_stage12_1_tiebreak` が出力した stage12_1_tie_events.tsv の
//! `n_candidates==2` 行 (winner=leaf_pos, loser=sim_pos。2択なので他候補は
//! 存在しない)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_2_binarytie -- <DIR> \
//!       [--events-tsv PATH] [--out-tsv PATH]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode, F, N};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const INITIAL_LOOKAHEAD_LO: usize = N - F; // 4078
const INITIAL_LOOKAHEAD_HI: usize = N - 1; // 4095

#[derive(Clone, Copy)]
enum HashFn {
    H1,
    H2,
    H3,
}
impl HashFn {
    fn name(&self) -> &'static str {
        match self {
            HashFn::H1 => "h1",
            HashFn::H2 => "h2",
            HashFn::H3 => "h3",
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

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input))
}

fn chain_rank(ring: &[u8; N], write_tick: &[u32; N], pos: usize, hf: HashFn, mask: u32) -> Option<usize> {
    if write_tick[pos] == u32::MAX {
        return None;
    }
    let target_tick = write_tick[pos];
    let b0 = ring[pos] as u32;
    let b1 = ring[(pos + 1) & (N - 1)] as u32;
    let b2 = ring[(pos + 2) & (N - 1)] as u32;
    let target_hash = hf.raw(b0, b1, b2) & mask;
    let mut newer = 0usize;
    for p in 0..N {
        let t = write_tick[p];
        if t == u32::MAX || t <= target_tick {
            continue;
        }
        let c0 = ring[p] as u32;
        let c1 = ring[(p + 1) & (N - 1)] as u32;
        let c2 = ring[(p + 2) & (N - 1)] as u32;
        if (hf.raw(c0, c1, c2) & mask) == target_hash {
            newer += 1;
        }
    }
    Some(1 + newer)
}

struct Event {
    file: String,
    di: usize,
    len: u8,
    winner: u16, // leaf_pos
    loser: u16,  // sim_pos
}

struct Features {
    // 単純な数値関係 (winner - loser の符号や剰余)
    winner_lt_loser: bool,
    winner_even: bool,
    loser_even: bool,
    winner_mod4: u16,
    loser_mod4: u16,
    winner_mod8: u16,
    loser_mod8: u16,
    winner_mod16: u16,
    loser_mod16: u16,
    // hash chain rank (小さいほど新しい)
    winner_chain_rank: BTreeMap<&'static str, Option<usize>>,
    loser_chain_rank: BTreeMap<&'static str, Option<usize>>,
    // window overlap
    winner_wraps_r: bool,
    loser_wraps_r: bool,
    winner_touches_lookahead: bool,
    loser_touches_lookahead: bool,
    winner_self_overlap: bool, // dist < len (copy-forward with write-back)
    loser_self_overlap: bool,
    // 直前トークンとの関係
    prev_is_match: bool,
    winner_is_seq_continuation: bool, // winner == prev end pos
    loser_is_seq_continuation: bool,
    // BST 構造 (OkumuraSim Basic)
    winner_on_search_path: bool,
    loser_on_search_path: bool,
    winner_search_rank: Option<u32>,
    loser_search_rank: Option<u32>,
    winner_off_code: u8,
    loser_off_code: u8,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--events-tsv PATH] [--out-tsv PATH]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_tsv = String::from(".local_data/stage12_1_tie_events.tsv");
    let mut out_tsv = String::from(".local_data/stage12_2_binarytie_features.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" => {
                if let Some(v) = args.get(i + 1) {
                    events_tsv = v.clone();
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

    let content = fs::read_to_string(&events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_len, i_ncand, i_leafpos, i_simpos) = (
        idx("file"),
        idx("di"),
        idx("len"),
        idx("n_candidates"),
        idx("leaf_pos"),
        idx("sim_pos"),
    );

    let mut events: Vec<Event> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let ncand: usize = f[i_ncand].parse().unwrap();
        if ncand != 2 {
            continue;
        }
        events.push(Event {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            len: f[i_len].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
            loser: f[i_simpos].parse().unwrap(),
        });
    }
    eprintln!("binary-tie (n_candidates==2) events loaded: {}", events.len());

    let combos: Vec<(HashFn, u32)> = ALL_HASHES
        .iter()
        .flat_map(|&hf| MASKS.iter().map(move |&m| (hf, m)))
        .collect();

    let mut all_features: Vec<Features> = Vec::new();
    let mut out_rows: Vec<String> = Vec::new();

    for ev in &events {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
            eprintln!("WARN load fail {}", ev.file);
            continue;
        };

        // shadow ring/write_tick teacher forcing (0..di) + OkumuraSim 並走
        let mut ring = [0x20u8; N];
        let mut write_tick = [u32::MAX; N];
        let mut r: usize = N - F;
        let mut input_pos: usize = 0;
        let mut sim = OkumuraSim::new(SimMode::Basic, &ring_input);
        let mut prev_match_end: Option<usize> = None;
        let mut prev_is_match = false;

        for tok in leaf_tokens.iter().take(ev.di) {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            let start = input_pos;
            for _ in 0..l {
                if input_pos >= ring_input.len() {
                    break;
                }
                ring[r] = ring_input[input_pos];
                write_tick[r] = input_pos as u32;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
            let end = input_pos;
            if end > start {
                sim.advance(&ring_input[start..end]);
            }
            match tok {
                LeafToken::Match { pos, len } => {
                    prev_match_end = Some(((*pos as usize) + (*len as usize)) & (N - 1));
                    prev_is_match = true;
                }
                LeafToken::Literal(_) => {
                    prev_match_end = None;
                    prev_is_match = false;
                }
            }
        }

        let winner_u = ev.winner as usize;
        let loser_u = ev.loser as usize;
        let len_u = ev.len as usize;

        let wraps_r = |pos: usize| -> bool {
            // window [pos, pos+len) mod N が現在の write pointer r を跨ぐか
            let dist = (r + N - (pos & (N - 1))) & (N - 1); // r からみた pos の距離 (何バイト前に書かれたか)
            dist < len_u && dist != 0
        };
        let touches_lookahead = |pos: usize| -> bool {
            (0..len_u).any(|k| {
                let slot = (pos + k) & (N - 1);
                slot >= INITIAL_LOOKAHEAD_LO && slot <= INITIAL_LOOKAHEAD_HI
            })
        };
        let self_overlap = |pos: usize| -> bool {
            let dist = (r + N - (pos & (N - 1))) & (N - 1);
            dist < len_u
        };

        let mut winner_chain_rank = BTreeMap::new();
        let mut loser_chain_rank = BTreeMap::new();
        for &(hf, mask) in &combos {
            let key: &'static str = match (hf.name(), mask) {
                ("h1", 0xFFF) => "h1_fff",
                ("h1", 0x7FF) => "h1_7ff",
                ("h1", 0x3FF) => "h1_3ff",
                ("h2", 0xFFF) => "h2_fff",
                ("h2", 0x7FF) => "h2_7ff",
                ("h2", 0x3FF) => "h2_3ff",
                ("h3", 0xFFF) => "h3_fff",
                ("h3", 0x7FF) => "h3_7ff",
                ("h3", 0x3FF) => "h3_3ff",
                _ => "unknown",
            };
            winner_chain_rank.insert(key, chain_rank(&ring, &write_tick, winner_u, hf, mask));
            loser_chain_rank.insert(key, chain_rank(&ring, &write_tick, loser_u, hf, mask));
        }

        // BST search_trace / classify_off_path (Basic)
        let trace = sim.search_trace(sim.r, ev.len);
        let winner_hit = trace.iter().find(|(p, _, _)| *p == ev.winner);
        let loser_hit = trace.iter().find(|(p, _, _)| *p == ev.loser);
        let (winner_off_code, _wd) = sim.classify_off_path(sim.r, ev.winner);
        let (loser_off_code, _ld) = sim.classify_off_path(sim.r, ev.loser);

        let feat = Features {
            winner_lt_loser: winner_u < loser_u,
            winner_even: winner_u % 2 == 0,
            loser_even: loser_u % 2 == 0,
            winner_mod4: (winner_u % 4) as u16,
            loser_mod4: (loser_u % 4) as u16,
            winner_mod8: (winner_u % 8) as u16,
            loser_mod8: (loser_u % 8) as u16,
            winner_mod16: (winner_u % 16) as u16,
            loser_mod16: (loser_u % 16) as u16,
            winner_chain_rank: winner_chain_rank.clone(),
            loser_chain_rank: loser_chain_rank.clone(),
            winner_wraps_r: wraps_r(winner_u),
            loser_wraps_r: wraps_r(loser_u),
            winner_touches_lookahead: touches_lookahead(winner_u),
            loser_touches_lookahead: touches_lookahead(loser_u),
            winner_self_overlap: self_overlap(winner_u),
            loser_self_overlap: self_overlap(loser_u),
            prev_is_match,
            winner_is_seq_continuation: prev_match_end == Some(winner_u),
            loser_is_seq_continuation: prev_match_end == Some(loser_u),
            winner_on_search_path: winner_hit.is_some(),
            loser_on_search_path: loser_hit.is_some(),
            winner_search_rank: winner_hit.map(|(_, rk, _)| *rk),
            loser_search_rank: loser_hit.map(|(_, rk, _)| *rk),
            winner_off_code,
            loser_off_code,
        };

        out_rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            ev.file,
            ev.di,
            ev.winner,
            ev.loser,
            feat.winner_lt_loser,
            feat.winner_even,
            feat.loser_even,
            feat.winner_wraps_r,
            feat.loser_wraps_r,
            feat.winner_touches_lookahead,
            feat.loser_touches_lookahead,
            feat.winner_self_overlap,
            feat.loser_self_overlap,
            feat.winner_is_seq_continuation,
            feat.loser_is_seq_continuation,
            feat.winner_on_search_path,
            feat.loser_on_search_path,
            feat.winner_off_code,
        ));
        all_features.push(feat);
    }

    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(f, "file\tdi\twinner\tloser\twinner_lt_loser\twinner_even\tloser_even\twinner_wraps_r\tloser_wraps_r\twinner_touches_lookahead\tloser_touches_lookahead\twinner_self_overlap\tloser_self_overlap\twinner_is_seq_continuation\tloser_is_seq_continuation\twinner_on_search_path\tloser_on_search_path\twinner_off_code").ok();
        for row in &out_rows {
            writeln!(f, "{}", row).ok();
        }
    }

    let n = all_features.len();
    eprintln!("features computed for {} events", n);
    eprintln!("---");

    // 単一特徴の的中率 (二値ルール: 「winner は X である」/「winner は loser より X」)
    let mut report = |label: &str, hit: usize| {
        eprintln!("{:45}: {}/{} ({:.1}%)", label, hit, n, 100.0 * hit as f64 / n as f64);
    };

    report(
        "winner_pos < loser_pos",
        all_features.iter().filter(|f| f.winner_lt_loser).count(),
    );
    report(
        "winner_pos > loser_pos",
        all_features.iter().filter(|f| !f.winner_lt_loser).count(),
    );
    report(
        "winner even (loser odd と対で見る)",
        all_features
            .iter()
            .filter(|f| f.winner_even && !f.loser_even)
            .count(),
    );
    report(
        "winner odd (loser even)",
        all_features
            .iter()
            .filter(|f| !f.winner_even && f.loser_even)
            .count(),
    );
    report(
        "winner_wraps_r (loser does not)",
        all_features
            .iter()
            .filter(|f| f.winner_wraps_r && !f.loser_wraps_r)
            .count(),
    );
    report(
        "loser_wraps_r (winner does not)",
        all_features
            .iter()
            .filter(|f| f.loser_wraps_r && !f.winner_wraps_r)
            .count(),
    );
    report(
        "winner_touches_lookahead (loser does not)",
        all_features
            .iter()
            .filter(|f| f.winner_touches_lookahead && !f.loser_touches_lookahead)
            .count(),
    );
    report(
        "loser_touches_lookahead (winner does not)",
        all_features
            .iter()
            .filter(|f| f.loser_touches_lookahead && !f.winner_touches_lookahead)
            .count(),
    );
    report(
        "winner_self_overlap (loser does not)",
        all_features
            .iter()
            .filter(|f| f.winner_self_overlap && !f.loser_self_overlap)
            .count(),
    );
    report(
        "loser_self_overlap (winner does not)",
        all_features
            .iter()
            .filter(|f| f.loser_self_overlap && !f.winner_self_overlap)
            .count(),
    );
    report(
        "winner_is_seq_continuation (prev token end)",
        all_features.iter().filter(|f| f.winner_is_seq_continuation).count(),
    );
    report(
        "loser_is_seq_continuation (prev token end)",
        all_features.iter().filter(|f| f.loser_is_seq_continuation).count(),
    );
    report(
        "winner_on_search_path (BST Basic)",
        all_features.iter().filter(|f| f.winner_on_search_path).count(),
    );
    report(
        "loser_on_search_path (BST Basic)",
        all_features.iter().filter(|f| f.loser_on_search_path).count(),
    );
    report(
        "winner_off_code==0 (on path)",
        all_features.iter().filter(|f| f.winner_off_code == 0).count(),
    );
    for code in 1..=4u8 {
        report(
            &format!("winner_off_code=={}", code),
            all_features.iter().filter(|f| f.winner_off_code == code).count(),
        );
    }
    // winner_search_rank と loser_search_rank の関係 (両方 on-path の場合)
    let both_on_path: Vec<&Features> = all_features
        .iter()
        .filter(|f| f.winner_on_search_path && f.loser_on_search_path)
        .collect();
    eprintln!(
        "both on search path: {}/{}",
        both_on_path.len(),
        n
    );
    let winner_rank_smaller = both_on_path
        .iter()
        .filter(|f| f.winner_search_rank.unwrap() < f.loser_search_rank.unwrap())
        .count();
    eprintln!(
        "  of those, winner has smaller search_rank (visited earlier): {}/{}",
        winner_rank_smaller,
        both_on_path.len()
    );

    // mod4/8/16 分布 (winner側の値のヒストグラム)
    let mut mod4_hist: BTreeMap<u16, usize> = BTreeMap::new();
    let mut mod8_hist: BTreeMap<u16, usize> = BTreeMap::new();
    let mut mod16_hist: BTreeMap<u16, usize> = BTreeMap::new();
    for f in &all_features {
        *mod4_hist.entry(f.winner_mod4).or_insert(0) += 1;
        *mod8_hist.entry(f.winner_mod8).or_insert(0) += 1;
        *mod16_hist.entry(f.winner_mod16).or_insert(0) += 1;
    }
    eprintln!("winner pos mod4 histogram: {:?}", mod4_hist);
    eprintln!("winner pos mod8 histogram: {:?}", mod8_hist);
    eprintln!("winner pos mod16 histogram: {:?}", mod16_hist);

    // hash chain rank: winner の rank が loser より小さい (新しい) 割合 per combo
    eprintln!("---");
    eprintln!("hash chain rank comparison (winner vs loser, both-present only):");
    for &(hf, mask) in &combos {
        let key: &'static str = match (hf.name(), mask) {
            ("h1", 0xFFF) => "h1_fff",
            ("h1", 0x7FF) => "h1_7ff",
            ("h1", 0x3FF) => "h1_3ff",
            ("h2", 0xFFF) => "h2_fff",
            ("h2", 0x7FF) => "h2_7ff",
            ("h2", 0x3FF) => "h2_3ff",
            ("h3", 0xFFF) => "h3_fff",
            ("h3", 0x7FF) => "h3_7ff",
            ("h3", 0x3FF) => "h3_3ff",
            _ => "unknown",
        };
        let mut both = 0usize;
        let mut winner_smaller = 0usize;
        for f in &all_features {
            if let (Some(Some(wr)), Some(Some(lr))) =
                (f.winner_chain_rank.get(key), f.loser_chain_rank.get(key))
            {
                both += 1;
                if wr < lr {
                    winner_smaller += 1;
                }
            }
        }
        eprintln!(
            "  {:10}: both_present={} winner_rank<loser_rank={} ({:.1}%)",
            key,
            both,
            winner_smaller,
            if both > 0 { 100.0 * winner_smaller as f64 / both as f64 } else { 0.0 }
        );
    }

    eprintln!("---");
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
