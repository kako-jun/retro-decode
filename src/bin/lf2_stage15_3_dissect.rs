//! Stage 15-3 (Issue #14 脈: TF-miss=2 27本の miss間相関)。
//!
//! Stage 14-7 の台帳で特定された TF-miss=2 の27本 (計54決定点) について、
//! Stage 15-1 の解剖ハーネス (`compress_okumura_tf_dissect`,
//! `src/formats/toheart/okumura_lzss.rs`) を「複数 miss 点をまとめてダンプ
//! する」形に拡張した `compress_okumura_tf_dissect_multi` を使い、各点の
//! 完全な状態と、同一ファイル内2点の関係を記述的に集計する。
//!
//! 新しい比較規則の実装・規則スイープ・発火条件の探索的スイープは一切
//! 行っていない。既存3規則 (ClosestDist/FarthestDist/ClosestToRawPos) の
//! 適用可否テストと、既に確定した不変条件 (Leaf選択=実BST生ノード) の
//! 検証のみ。
//!
//! usage:
//!   cargo run --release --bin lf2_stage15_3_dissect -- <LF2_DIR> [--out-dir <dir>]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    self, BstMode, CmpMode, DelMode, KeyMode, TfConfig, TieMode, Token,
};
use retro_decode::formats::toheart::verify_harness;

/// Stage 14-7 台帳 (`tf_miss_ledger.csv`) の TF-miss=2 27本。各エントリは
/// (ファイル名, 勝ち変種, [1点目 token_idx, 2点目 token_idx]) で、
/// `tf_miss_tie_dive.csv` から引用した既知の値 (新しい探索ではない)。
const TARGETS_27: &[(&str, &str, [usize; 2])] = &[
    ("C0101.LF2", "basic_tail1", [509, 1061]),
    ("C0109.LF2", "basic_tail1", [469, 592]),
    ("C0181.LF2", "basic_tail1", [509, 945]),
    ("C0189.LF2", "basic_tail1", [469, 592]),
    ("C0313.LF2", "basic_tail1", [489, 733]),
    ("C0515.LF2", "basic_tail1", [1349, 1374]),
    ("C0516.LF2", "basic_tail1", [1349, 1374]),
    ("C0608.LF2", "basic", [2015, 8749]),
    ("C0609.LF2", "basic", [2015, 8751]),
    ("C060A.LF2", "basic", [2015, 8710]),
    ("C060B.LF2", "basic", [2015, 8785]),
    ("C060C.LF2", "basic", [2015, 8783]),
    ("C060D.LF2", "basic", [2015, 8745]),
    ("C0614.LF2", "basic", [2015, 8475]),
    ("C0804.LF2", "fill00_tail1", [47, 10975]),
    ("C080C.LF2", "fill00_tail1", [47, 10856]),
    ("C0A01.LF2", "basic", [763, 1147]),
    ("C1202.LF2", "no_dummy", [42, 10237]),
    ("C120C.LF2", "no_dummy", [42, 10206]),
    ("C1E12.LF2", "basic", [1864, 11670]),
    ("C1E15.LF2", "basic", [1864, 10756]),
    ("C1E19.LF2", "basic", [1864, 11429]),
    ("C1E20.LF2", "basic", [1864, 11636]),
    ("H22.LF2", "basic", [3837, 5382]),
    ("H43.LF2", "basic", [1736, 1737]),
    ("H52.LF2", "basic", [1686, 10264]),
    ("V70.LF2", "basic", [7115, 8354]),
];

/// Stage 15-1 で解剖済みの「真に新規の TF-miss=1」8本 (比較対象、Δ0の再確認)。
const TARGETS_8: &[(&str, &str, usize)] = &[
    ("C050F.LF2", "basic_tail1", 2637),
    ("C0E01.LF2", "basic", 798),
    ("C0E02.LF2", "basic", 798),
    ("C1301.LF2", "basic_tail1", 767),
    ("C1E17.LF2", "basic_tail1", 1864),
    ("CLNO_09.LF2", "basic", 1207),
    ("V31.LF2", "basic", 2201),
    ("V32.LF2", "basic", 2201),
];

/// Stage 14-4/14-5/14-6 で凍結した EOF巨大tie 13本 (比較対象、Δ0の再確認)。
/// `tf_miss_ledger.csv` の `first_miss_token_idx` = `total_tokens - 1` を引用。
const TARGETS_13: &[(&str, &str, usize)] = &[
    ("C0508.LF2", "basic", 10206),
    ("C0509.LF2", "basic", 10179),
    ("C050A.LF2", "basic", 10199),
    ("C0511.LF2", "basic", 9859),
    ("C0518.LF2", "basic", 10223),
    ("C1205.LF2", "no_dummy", 9401),
    ("C1E03.LF2", "basic", 11012),
    ("C1E05.LF2", "basic", 10347),
    ("C1E06.LF2", "basic", 10877),
    ("C1E0A.LF2", "basic", 11083),
    ("C1E13.LF2", "basic", 11393),
    ("C1E16.LF2", "basic", 11246),
    ("C1E1A.LF2", "basic", 11464),
];

fn cfg_for(variant: &'static str) -> TfConfig {
    let mut c = TfConfig {
        name: variant,
        fill: 0x20,
        dummy_init: true,
        tie_mode: TieMode::StrictGt,
        bst_mode: BstMode::Standard,
        key_mode: KeyMode::Byte0,
        cmp_mode: CmpMode::Unsigned,
        del_mode: DelMode::Predecessor,
        tail1_rle_plus1: false,
        eof_retie: None,
        count_ties: false,
    };
    match variant {
        "basic" => {}
        "basic_tail1" => c.tail1_rle_plus1 = true,
        "fill00_tail1" => {
            c.fill = 0x00;
            c.tail1_rle_plus1 = true;
        }
        "no_dummy" => c.dummy_init = false,
        other => panic!("unknown variant: {}", other),
    }
    c
}

fn leaf_to_tokens(leaf: &[LeafToken]) -> Vec<Token> {
    leaf.iter()
        .map(|t| match t {
            LeafToken::Literal(b) => Token::Literal(*b),
            LeafToken::Match { pos, len } => Token::Match { pos: *pos, len: *len },
        })
        .collect()
}

fn token_cell(t: &Token) -> String {
    match t {
        Token::Literal(b) => format!("Literal(0x{:02x})", b),
        Token::Match { pos, len } => format!("Match(pos={} len={})", pos, len),
    }
}

/// Stage 14-7 の `TfMissKind` 分類ロジックの逐語再利用 (新分類の考案ではない)。
fn classify_kind(predicted: &Token, actual: &Token) -> &'static str {
    match (predicted, actual) {
        (Token::Literal(a), Token::Literal(b)) => {
            if a == b {
                "LiteralSame(bug)"
            } else {
                "LiteralValueDiff"
            }
        }
        (Token::Match { len: l1, .. }, Token::Match { len: l2, .. }) => {
            if l1 == l2 {
                "PosDiffOnly"
            } else {
                "LenDiff"
            }
        }
        _ => "LiteralVsMatch",
    }
}

/// ファイル名から族キーを引く。Cxxyy 命名規則 (xx=場面/キャラID, yy=ポーズ
/// 変種) では先頭3文字が族、CLNO/H/V は接頭アルファベット連続部分が族。
fn family_of(name: &str) -> String {
    let stem = name.trim_end_matches(".LF2");
    if stem.starts_with("CLNO") {
        return "CLNO".to_string();
    }
    if stem.starts_with('C') && stem.len() >= 3 {
        return stem[0..3].to_string();
    }
    let alpha_len = stem.chars().take_while(|c| c.is_ascii_alphabetic()).count().max(1);
    stem[0..alpha_len].to_string()
}

/// 連続 ring 位置 (gap==1) でのブロック分割。Stage 14-4/15-1 と同一アルゴリズム。
fn blocks_of(mut positions: Vec<i32>) -> Vec<(i32, i32)> {
    positions.sort_unstable();
    positions.dedup();
    let mut blocks: Vec<(i32, i32)> = Vec::new();
    if positions.is_empty() {
        return blocks;
    }
    let mut start = positions[0];
    let mut prev = positions[0];
    for &p in &positions[1..] {
        if p == prev + 1 {
            prev = p;
        } else {
            blocks.push((start, prev));
            start = p;
            prev = p;
        }
    }
    blocks.push((start, prev));
    blocks
}

fn find_block(blocks: &[(i32, i32)], pos: i32) -> Option<(i32, i32, i32)> {
    blocks
        .iter()
        .find(|(s, e)| pos >= *s && pos <= *e)
        .map(|(s, e)| (*s, *e, *e - pos))
}

fn boundary_dist(offset: usize, period: usize) -> (usize, usize) {
    let r = offset % period;
    (r, period - r)
}

/// 1決定点の完全レポート (54点・8点・13点の全てで共通の schema)。
struct PointReport {
    source_stage: &'static str,
    name: String,
    family: String,
    variant: &'static str,
    point_idx: usize, // 0 = 1点目(昇順), 1 = 2点目 (単点ファイルは常に0)
    token_idx: usize,
    total_tokens: usize,
    input_len: usize,
    payload_len: usize,
    decompressed_offset: usize,
    compressed_offset: usize,
    residual: usize,
    eof_near: bool,
    kind: &'static str,
    predicted: Token,
    actual: Token,
    is_literal_actual: bool,
    raw_pos: i32,
    r: i32,
    n_candidates: usize,
    n_in_tree: usize,
    leaf_in_tree: bool,
    leaf_rank_by_ring_dist: Option<usize>,
    leaf_rank_by_write_tick: Option<usize>,
    leaf_rank_by_inorder: Option<usize>,
    block_all_start: i32,
    block_all_end: i32,
    block_all_size: i32,
    block_all_is_end: bool,
    hyp_a_pass: bool,
    hyp_b_pass: bool,
    eof_closest_dist_match: bool,
    eof_farthest_dist_match: bool,
    eof_closest_to_raw_match: bool,
    dist_prev_64k_decomp: usize,
    dist_next_64k_decomp: usize,
    dist_prev_ring_decomp: usize,
    dist_next_ring_decomp: usize,
    dist_prev_64k_comp: usize,
    dist_next_64k_comp: usize,
    dist_prev_ring_comp: usize,
    dist_next_ring_comp: usize,
    dist_to_file_start_decomp: usize,
    dist_to_file_end_decomp: usize,
}

fn matched_rules(r: &PointReport) -> Vec<&'static str> {
    let mut v = Vec::new();
    if r.eof_closest_dist_match {
        v.push("ClosestDist");
    }
    if r.eof_farthest_dist_match {
        v.push("FarthestDist");
    }
    if r.eof_closest_to_raw_match {
        v.push("ClosestToRawPos");
    }
    v
}

/// dump を PointReport に変換する。stage15_1 と同一のランク・ブロック・
/// 境界距離計算 (新規則ではなく既存指標の再計算)。
fn dump_to_report(
    source_stage: &'static str,
    name: &str,
    family: &str,
    variant: &'static str,
    point_idx: usize,
    total_tokens: usize,
    input_len: usize,
    payload_len: usize,
    compressed_offset: usize,
    dump: &okumura_lzss::Stage151Dump,
) -> PointReport {
    let is_literal_actual = matches!(dump.actual, Token::Literal(_));
    let leaf_pos = match dump.actual {
        Token::Match { pos, .. } => pos as i32,
        Token::Literal(_) => -1,
    };

    let mut by_ring_dist: Vec<&okumura_lzss::Stage151Candidate> = dump.candidates.iter().collect();
    by_ring_dist.sort_by_key(|c| c.ring_dist);
    let leaf_rank_by_ring_dist = by_ring_dist.iter().position(|c| c.pos == leaf_pos);

    let mut by_write_tick: Vec<&okumura_lzss::Stage151Candidate> = dump.candidates.iter().collect();
    by_write_tick.sort_by_key(|c| std::cmp::Reverse(c.write_tick));
    let leaf_rank_by_write_tick = by_write_tick.iter().position(|c| c.pos == leaf_pos);

    let in_tree_cands: Vec<&okumura_lzss::Stage151Candidate> =
        dump.candidates.iter().filter(|c| c.in_tree).collect();
    let mut by_inorder: Vec<&okumura_lzss::Stage151Candidate> = in_tree_cands.clone();
    by_inorder.sort_by_key(|c| c.inorder_rank.unwrap_or(usize::MAX));
    let leaf_rank_by_inorder = by_inorder.iter().position(|c| c.pos == leaf_pos);

    let leaf_cand = dump.candidates.iter().find(|c| c.pos == leaf_pos);
    let leaf_in_tree = leaf_cand.map(|c| c.in_tree).unwrap_or(false);

    let all_positions: Vec<i32> = dump.candidates.iter().map(|c| c.pos).collect();
    let blocks_all = blocks_of(all_positions);
    let (block_all_start, block_all_end, block_all_size, block_all_is_end) =
        match find_block(&blocks_all, leaf_pos) {
            Some((s, e, off_from_end)) => (s, e, e - s + 1, off_from_end == 0),
            None => (-1, -1, 0, false),
        };

    let in_tree_positions: Vec<i32> = in_tree_cands.iter().map(|c| c.pos).collect();
    let blocks_in_tree = blocks_of(in_tree_positions);
    let hyp_b_pass = find_block(&blocks_in_tree, leaf_pos)
        .map(|(_, _, off_from_end)| off_from_end == 1)
        .unwrap_or(false);
    let hyp_a_pass = leaf_in_tree;

    let eof_closest = dump
        .eof_rule_winner
        .iter()
        .find(|(n, _)| *n == "ClosestDist")
        .and_then(|(_, p)| *p);
    let eof_farthest = dump
        .eof_rule_winner
        .iter()
        .find(|(n, _)| *n == "FarthestDist")
        .and_then(|(_, p)| *p);
    let eof_closest_raw = dump
        .eof_rule_winner
        .iter()
        .find(|(n, _)| *n == "ClosestToRawPos")
        .and_then(|(_, p)| *p);

    let (d_prev_64k_decomp, d_next_64k_decomp) = boundary_dist(dump.decompressed_offset, 65536);
    let (d_prev_ring_decomp, d_next_ring_decomp) = boundary_dist(dump.decompressed_offset, 4096);
    let (d_prev_64k_comp, d_next_64k_comp) = boundary_dist(compressed_offset, 65536);
    let (d_prev_ring_comp, d_next_ring_comp) = boundary_dist(compressed_offset, 4096);

    PointReport {
        source_stage,
        name: name.to_string(),
        family: family.to_string(),
        variant,
        point_idx,
        token_idx: dump.token_idx,
        total_tokens,
        input_len,
        payload_len,
        decompressed_offset: dump.decompressed_offset,
        compressed_offset,
        residual: dump.residual,
        eof_near: dump.residual < okumura_lzss::F,
        kind: classify_kind(&dump.predicted, &dump.actual),
        predicted: dump.predicted,
        actual: dump.actual,
        is_literal_actual,
        raw_pos: dump.raw_pos,
        r: dump.r,
        n_candidates: dump.candidates.len(),
        n_in_tree: in_tree_cands.len(),
        leaf_in_tree,
        leaf_rank_by_ring_dist,
        leaf_rank_by_write_tick,
        leaf_rank_by_inorder,
        block_all_start,
        block_all_end,
        block_all_size,
        block_all_is_end,
        hyp_a_pass,
        hyp_b_pass,
        eof_closest_dist_match: eof_closest == Some(leaf_pos),
        eof_farthest_dist_match: eof_farthest == Some(leaf_pos),
        eof_closest_to_raw_match: eof_closest_raw == Some(leaf_pos),
        dist_prev_64k_decomp: d_prev_64k_decomp,
        dist_next_64k_decomp: d_next_64k_decomp,
        dist_prev_ring_decomp: d_prev_ring_decomp,
        dist_next_ring_decomp: d_next_ring_decomp,
        dist_prev_64k_comp: d_prev_64k_comp,
        dist_next_64k_comp: d_next_64k_comp,
        dist_prev_ring_comp: d_prev_ring_comp,
        dist_next_ring_comp: d_next_ring_comp,
        dist_to_file_start_decomp: dump.decompressed_offset,
        dist_to_file_end_decomp: input_len.saturating_sub(dump.decompressed_offset),
    }
}

fn write_point_header(f: &mut fs::File) {
    writeln!(
        f,
        "source_stage,name,family,variant,point_idx,token_idx,total_tokens,input_len,payload_len,decompressed_offset,compressed_offset,residual,eof_near,kind,predicted,actual,is_literal_actual,raw_pos,r,n_candidates,n_in_tree,leaf_in_tree,leaf_rank_by_ring_dist,leaf_rank_by_write_tick,leaf_rank_by_inorder,block_all_start,block_all_end,block_all_size,block_all_is_end,hyp_a_pass,hyp_b_pass,eof_closest_dist_match,eof_farthest_dist_match,eof_closest_to_raw_match,matched_rules,dist_prev_64k_decomp,dist_next_64k_decomp,dist_prev_ring_decomp,dist_next_ring_decomp,dist_prev_64k_comp,dist_next_64k_comp,dist_prev_ring_comp,dist_next_ring_comp,dist_to_file_start_decomp,dist_to_file_end_decomp"
    )
    .unwrap();
}

fn write_point_row(f: &mut fs::File, r: &PointReport) {
    let rules = matched_rules(r).join("|");
    writeln!(
        f,
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        r.source_stage,
        r.name,
        r.family,
        r.variant,
        r.point_idx,
        r.token_idx,
        r.total_tokens,
        r.input_len,
        r.payload_len,
        r.decompressed_offset,
        r.compressed_offset,
        r.residual,
        r.eof_near,
        r.kind,
        token_cell(&r.predicted),
        token_cell(&r.actual),
        r.is_literal_actual,
        r.raw_pos,
        r.r,
        r.n_candidates,
        r.n_in_tree,
        r.leaf_in_tree,
        r.leaf_rank_by_ring_dist.map(|v| v.to_string()).unwrap_or_default(),
        r.leaf_rank_by_write_tick.map(|v| v.to_string()).unwrap_or_default(),
        r.leaf_rank_by_inorder.map(|v| v.to_string()).unwrap_or_default(),
        r.block_all_start,
        r.block_all_end,
        r.block_all_size,
        r.block_all_is_end,
        r.hyp_a_pass,
        r.hyp_b_pass,
        r.eof_closest_dist_match,
        r.eof_farthest_dist_match,
        r.eof_closest_to_raw_match,
        rules,
        r.dist_prev_64k_decomp,
        r.dist_next_64k_decomp,
        r.dist_prev_ring_decomp,
        r.dist_next_ring_decomp,
        r.dist_prev_64k_comp,
        r.dist_next_64k_comp,
        r.dist_prev_ring_comp,
        r.dist_next_ring_comp,
        r.dist_to_file_start_decomp,
        r.dist_to_file_end_decomp,
    )
    .unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-dir <dir>]", args[0]);
        std::process::exit(1);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_dir = PathBuf::from(".local_data/stage15_3");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out-dir" => {
                if let Some(v) = args.get(i + 1) {
                    out_dir = PathBuf::from(v);
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                std::process::exit(1);
            }
        }
    }
    fs::create_dir_all(&out_dir).expect("create out dir");

    // --- 27本 x 2点 = 54点 ---
    let mut points54: Vec<PointReport> = Vec::new();
    let mut file_points_idx: Vec<(String, usize, usize)> = Vec::new(); // (name, start_idx_in_points54, count)

    for &(name, variant, targets) in TARGETS_27 {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", name, e));
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap_or_else(|| panic!("parse {}", name));
        let leaf_tokens = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
            .unwrap_or_else(|e| panic!("decode {}: {}", name, e));
        let actual = leaf_to_tokens(&leaf_tokens.tokens);
        let decoded = verify_harness::load_and_decode(&path).unwrap_or_else(|e| panic!("{}", e));
        let input = &decoded.ring_input;
        let payload = &decoded.payload;

        let cfg = cfg_for(variant);
        let dumps = okumura_lzss::compress_okumura_tf_dissect_multi(input, &actual, &cfg, &targets);
        assert_eq!(dumps.len(), 2, "{}: expected 2 dumps, got {}", name, dumps.len());
        for (pi, d) in dumps.iter().enumerate() {
            assert_eq!(d.token_idx, targets[pi], "{}: token_idx mismatch at point {}", name, pi);
            assert_ne!(d.predicted, d.actual, "{}: expected a mismatch at point {}", name, pi);
        }

        let family = family_of(name);
        let start_idx = points54.len();
        for (pi, dump) in dumps.iter().enumerate() {
            let compressed_offset = verify_harness::tokens_to_lf2_payload(&actual[..dump.token_idx]).len();
            let rep = dump_to_report(
                "stage15_3",
                name,
                &family,
                variant,
                pi,
                actual.len(),
                input.len(),
                payload.len(),
                compressed_offset,
                dump,
            );
            points54.push(rep);
        }
        file_points_idx.push((name.to_string(), start_idx, 2));
    }

    let points_path = out_dir.join("points54.csv");
    let mut fpoints = fs::File::create(&points_path).expect("create points54.csv");
    write_point_header(&mut fpoints);
    for r in &points54 {
        write_point_row(&mut fpoints, r);
    }

    // --- 同一ファイル内2点の関係 (iii) ---
    struct FileRelation {
        name: String,
        family: String,
        variant: &'static str,
        tok0: usize,
        tok1: usize,
        token_distance: usize,
        decomp_byte_distance: usize,
        comp_byte_distance: usize,
        kind0: &'static str,
        kind1: &'static str,
        rules0: String,
        rules1: String,
        relation_class: &'static str,
    }

    let mut relations: Vec<FileRelation> = Vec::new();
    for (name, start_idx, count) in &file_points_idx {
        assert_eq!(*count, 2);
        let p0 = &points54[*start_idx];
        let p1 = &points54[*start_idx + 1];
        let rules0 = matched_rules(p0);
        let rules1 = matched_rules(p1);
        let rules0_set: std::collections::BTreeSet<&str> = rules0.iter().copied().collect();
        let rules1_set: std::collections::BTreeSet<&str> = rules1.iter().copied().collect();

        let relation_class = if rules0_set.is_empty() && rules1_set.is_empty() {
            "none_both"
        } else if rules0_set.is_empty() || rules1_set.is_empty() {
            "one_rule_one_none"
        } else if rules0_set == rules1_set {
            "same_rule_both"
        } else {
            "different_rule_each"
        };

        relations.push(FileRelation {
            name: name.clone(),
            family: p0.family.clone(),
            variant: p0.variant,
            tok0: p0.token_idx,
            tok1: p1.token_idx,
            token_distance: p1.token_idx - p0.token_idx,
            decomp_byte_distance: p1.decompressed_offset.saturating_sub(p0.decompressed_offset),
            comp_byte_distance: p1.compressed_offset.saturating_sub(p0.compressed_offset),
            kind0: p0.kind,
            kind1: p1.kind,
            rules0: rules0.join("|"),
            rules1: rules1.join("|"),
            relation_class,
        });
    }

    let rel_path = out_dir.join("file_relations27.csv");
    let mut frel = fs::File::create(&rel_path).expect("create file_relations27.csv");
    writeln!(
        frel,
        "name,family,variant,tok0,tok1,token_distance,decomp_byte_distance,comp_byte_distance,kind0,kind1,rules0,rules1,relation_class"
    )
    .unwrap();
    for r in &relations {
        writeln!(
            frel,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.name,
            r.family,
            r.variant,
            r.tok0,
            r.tok1,
            r.token_distance,
            r.decomp_byte_distance,
            r.comp_byte_distance,
            r.kind0,
            r.kind1,
            r.rules0,
            r.rules1,
            r.relation_class,
        )
        .unwrap();
    }

    // --- Stage 15-1 の8点 (Δ0 の再確認、既存関数を単一ターゲットのまま再利用) ---
    let mut points8: Vec<PointReport> = Vec::new();
    for &(name, variant, token_idx) in TARGETS_8 {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", name, e));
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap_or_else(|| panic!("parse {}", name));
        let leaf_tokens = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
            .unwrap_or_else(|e| panic!("decode {}: {}", name, e));
        let actual = leaf_to_tokens(&leaf_tokens.tokens);
        let decoded = verify_harness::load_and_decode(&path).unwrap_or_else(|e| panic!("{}", e));
        let input = &decoded.ring_input;
        let payload = &decoded.payload;
        let cfg = cfg_for(variant);
        let dump = okumura_lzss::compress_okumura_tf_dissect(input, &actual, &cfg, token_idx)
            .unwrap_or_else(|| panic!("dissect None for {}", name));
        let compressed_offset = verify_harness::tokens_to_lf2_payload(&actual[..token_idx]).len();
        let family = family_of(name);
        points8.push(dump_to_report(
            "stage15_1",
            name,
            &family,
            variant,
            0,
            actual.len(),
            input.len(),
            payload.len(),
            compressed_offset,
            &dump,
        ));
    }

    // --- 凍結13本 (Δ0 の再確認) ---
    let mut points13: Vec<PointReport> = Vec::new();
    for &(name, variant, token_idx) in TARGETS_13 {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", name, e));
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap_or_else(|| panic!("parse {}", name));
        let leaf_tokens = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
            .unwrap_or_else(|e| panic!("decode {}: {}", name, e));
        let actual = leaf_to_tokens(&leaf_tokens.tokens);
        let decoded = verify_harness::load_and_decode(&path).unwrap_or_else(|e| panic!("{}", e));
        let input = &decoded.ring_input;
        let payload = &decoded.payload;
        let cfg = cfg_for(variant);
        let dump = okumura_lzss::compress_okumura_tf_dissect(input, &actual, &cfg, token_idx)
            .unwrap_or_else(|| panic!("dissect None for {}", name));
        let compressed_offset = verify_harness::tokens_to_lf2_payload(&actual[..token_idx]).len();
        let family = family_of(name);
        points13.push(dump_to_report(
            "frozen13",
            name,
            &family,
            variant,
            0,
            actual.len(),
            input.len(),
            payload.len(),
            compressed_offset,
            &dump,
        ));
    }

    // --- 統合俯瞰 (75点) ---
    let overview_path = out_dir.join("overview75.csv");
    let mut fov = fs::File::create(&overview_path).expect("create overview75.csv");
    write_point_header(&mut fov);
    for r in points54.iter().chain(points8.iter()).chain(points13.iter()) {
        write_point_row(&mut fov, r);
    }

    // --- 族クラスタ集計 (iv) ---
    use std::collections::BTreeMap;
    let mut family_members: BTreeMap<String, Vec<(&'static str, String)>> = BTreeMap::new();
    for r in points54.iter().chain(points8.iter()).chain(points13.iter()) {
        family_members
            .entry(r.family.clone())
            .or_default()
            .push((r.source_stage, r.name.clone()));
    }
    let family_path = out_dir.join("family_clusters.csv");
    let mut ffam = fs::File::create(&family_path).expect("create family_clusters.csv");
    writeln!(ffam, "family,source_stage,name").unwrap();
    for (fam, members) in &family_members {
        let mut seen: std::collections::BTreeSet<(&'static str, String)> = std::collections::BTreeSet::new();
        for (stage, name) in members {
            if seen.insert((*stage, name.clone())) {
                writeln!(ffam, "{},{},{}", fam, stage, name).unwrap();
            }
        }
    }

    // --- console summary ---
    eprintln!("wrote {} rows to {:?}", points54.len(), points_path);
    eprintln!("wrote {} rows to {:?}", relations.len(), rel_path);
    eprintln!(
        "wrote {} rows to {:?}",
        points54.len() + points8.len() + points13.len(),
        overview_path
    );
    eprintln!("wrote family clusters to {:?}", family_path);

    let applicable: Vec<&PointReport> = points54.iter().filter(|r| !r.is_literal_actual).collect();
    let n_a = applicable.iter().filter(|r| r.hyp_a_pass).count();
    let n_b = applicable.iter().filter(|r| r.hyp_b_pass).count();
    let n_closest = applicable.iter().filter(|r| r.eof_closest_dist_match).count();
    let n_farthest = applicable.iter().filter(|r| r.eof_farthest_dist_match).count();
    let n_closest_raw = applicable.iter().filter(|r| r.eof_closest_to_raw_match).count();
    eprintln!(
        "\n(i) hyp_a (leaf_in_tree) = {}/{} applicable ({} total, {} literal-actual N/A)",
        n_a,
        applicable.len(),
        points54.len(),
        points54.len() - applicable.len()
    );
    eprintln!("(i) hyp_b (block_end-1) = {}/{} applicable", n_b, applicable.len());
    eprintln!(
        "(ii) eof_closest_dist={}/{} eof_farthest_dist={}/{} eof_closest_to_raw={}/{}",
        n_closest,
        applicable.len(),
        n_farthest,
        applicable.len(),
        n_closest_raw,
        applicable.len()
    );

    let mut class_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for r in &relations {
        *class_counts.entry(r.relation_class).or_insert(0) += 1;
    }
    eprintln!("\n(iii) per-file 2点関係 (27本):");
    for (k, v) in &class_counts {
        eprintln!("  {}: {}", k, v);
    }

    eprintln!("\n--- per-file detail (27) ---");
    for r in &relations {
        eprintln!(
            "{:10} fam={:4} tok=({:>6},{:>6}) tokdist={:>6} kind=({},{}) rules=({} / {}) class={}",
            r.name, r.family, r.tok0, r.tok1, r.token_distance, r.kind0, r.kind1, r.rules0, r.rules1, r.relation_class
        );
    }
}
