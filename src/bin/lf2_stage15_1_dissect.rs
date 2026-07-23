//! Stage 15-1 (Issue #14 脈: 新顔8本の記述的解剖)。
//!
//! Stage 14-7 で特定された「真に新規の TF-miss=1」8本 (C050F/C0E01/C0E02/
//! C1301/C1E17/CLNO_09/V31/V32) について、唯一の miss 決定点の完全な状態を
//! ダンプする。新しい tie-break 比較規則の実装・掃引は一切行わない
//! (`compress_okumura_tf_dissect`、`src/formats/toheart/okumura_lzss.rs`、
//! 既存の `insert_node`/`delete_node`/`EofTieRule` スコア式を逐語再利用する
//! 記述的診断のみ)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage15_1_dissect -- <LF2_DIR> [--out-dir <dir>]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    self, BstMode, CmpMode, DelMode, KeyMode, TfConfig, TieMode, Token,
};
use retro_decode::formats::toheart::verify_harness;

/// Stage 14-7 台帳 (`tf_miss_ledger.csv` / `tf_miss_tie_dive.csv`) から読んだ
/// 既知の値をそのまま使う (新しい探索・新しい選定規則ではなく、既に確定した
/// 「勝ち変種・唯一の miss token_idx」を引用するだけ)。
const TARGETS: &[(&str, &str, usize)] = &[
    ("C050F.LF2", "basic_tail1", 2637),
    ("C0E01.LF2", "basic", 798),
    ("C0E02.LF2", "basic", 798),
    ("C1301.LF2", "basic_tail1", 767),
    ("C1E17.LF2", "basic_tail1", 1864),
    ("CLNO_09.LF2", "basic", 1207),
    ("V31.LF2", "basic", 2201),
    ("V32.LF2", "basic", 2201),
];

fn cfg_for(variant: &'static str) -> TfConfig {
    TfConfig {
        name: variant,
        fill: 0x20,
        dummy_init: true,
        tie_mode: TieMode::StrictGt,
        bst_mode: BstMode::Standard,
        key_mode: KeyMode::Byte0,
        cmp_mode: CmpMode::Unsigned,
        del_mode: DelMode::Predecessor,
        tail1_rle_plus1: variant == "basic_tail1",
        eof_retie: None,
        count_ties: false,
    }
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

/// 連続 ring 位置 (gap==1) でのブロック分割。Stage 14-4 と同一アルゴリズム。
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

/// `pos` を含むブロック (start, end) と、末尾から何個目か (0 = block_end 自身)。
fn find_block(blocks: &[(i32, i32)], pos: i32) -> Option<(i32, i32, i32)> {
    blocks
        .iter()
        .find(|(s, e)| pos >= *s && pos <= *e)
        .map(|(s, e)| (*s, *e, *e - pos))
}

/// 距離境界の (直前境界までの距離, 直後境界までの距離) を周期 `period` で計算する。
fn boundary_dist(offset: usize, period: usize) -> (usize, usize) {
    let r = offset % period;
    (r, period - r)
}

struct FileReport {
    name: String,
    variant: &'static str,
    token_idx: usize,
    total_tokens: usize,
    input_len: usize,
    payload_len: usize,
    decompressed_offset: usize,
    compressed_offset: usize,
    residual: usize,
    predicted: Token,
    actual: Token,
    raw_pos: i32,
    r: i32,
    n_candidates: usize,
    n_in_tree: usize,
    leaf_in_tree: bool,
    leaf_rank_by_ring_dist: Option<usize>,  // 0 = 最も r に近い
    leaf_rank_by_write_tick: Option<usize>, // 0 = 最も新しい
    leaf_rank_by_inorder: Option<usize>,    // in_tree 集合内での順位 (0 = 最小キー)
    // 全候補 (written) でのブロック
    block_all_start: i32,
    block_all_end: i32,
    block_all_size: i32,
    block_all_is_end: bool,
    hyp_a_pass: bool,
    hyp_b_pass: bool,
    eof_closest_dist_match: bool,
    eof_farthest_dist_match: bool,
    eof_closest_to_raw_match: bool,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-dir <dir>]", args[0]);
        std::process::exit(1);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_dir = PathBuf::from(".local_data/stage15_1");
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

    let candidates_path = out_dir.join("candidates.csv");
    let mut fcand = fs::File::create(&candidates_path).expect("create candidates.csv");
    writeln!(
        fcand,
        "name,pos,raw_dist,ring_dist,in_tree,write_tick,dad,lson,rson,inorder_rank,is_leaf_pos,is_raw_pos"
    )
    .unwrap();

    let boundary_path = out_dir.join("boundary.csv");
    let mut fbound = fs::File::create(&boundary_path).expect("create boundary.csv");
    writeln!(
        fbound,
        "name,offset_kind,offset,dist_to_prev_64k,dist_to_next_64k,dist_to_prev_ring4096,dist_to_next_ring4096,dist_to_file_start,dist_to_file_end"
    )
    .unwrap();

    let mut reports: Vec<FileReport> = Vec::new();

    for &(name, variant, token_idx) in TARGETS {
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
            .unwrap_or_else(|| panic!("dissect returned None for {} at token {}", name, token_idx));

        assert_eq!(dump.token_idx, token_idx, "{}: token_idx mismatch", name);
        assert_ne!(dump.predicted, dump.actual, "{}: expected a mismatch at this token", name);

        let leaf_pos = match dump.actual {
            Token::Match { pos, .. } => pos as i32,
            Token::Literal(_) => -1,
        };

        // --- candidates.csv ---
        for c in &dump.candidates {
            writeln!(
                fcand,
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                name,
                c.pos,
                c.raw_dist,
                c.ring_dist,
                c.in_tree,
                c.write_tick,
                c.dad,
                c.lson,
                c.rson,
                c.inorder_rank.map(|v| v.to_string()).unwrap_or_default(),
                c.pos == leaf_pos,
                c.pos == dump.raw_pos,
            )
            .unwrap();
        }

        // --- rank 計算 ---
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

        // --- ブロック分割: 全候補 (written) ---
        let all_positions: Vec<i32> = dump.candidates.iter().map(|c| c.pos).collect();
        let blocks_all = blocks_of(all_positions);
        let (block_all_start, block_all_end, block_all_size, block_all_is_end) =
            match find_block(&blocks_all, leaf_pos) {
                Some((s, e, off_from_end)) => (s, e, e - s + 1, off_from_end == 0),
                None => (-1, -1, 0, false),
            };

        // --- ブロック分割: in_tree 限定 (Stage 14-5 と同じ母集団、block_end-1 仮説の対象) ---
        let in_tree_positions: Vec<i32> = in_tree_cands.iter().map(|c| c.pos).collect();
        let blocks_in_tree = blocks_of(in_tree_positions);
        // off_from_end (= block_end - leaf_pos) が 1 のとき「末尾から2番目 (block_end-1)」。
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

        // --- boundary.csv: 非圧縮側・圧縮側オフセット両方 ---
        let compressed_offset = verify_harness::tokens_to_lf2_payload(&actual[..token_idx]).len();
        for (kind, offset, end_len) in [
            ("decompressed", dump.decompressed_offset, input.len()),
            ("compressed", compressed_offset, payload.len()),
        ] {
            let (d_prev_64k, d_next_64k) = boundary_dist(offset, 65536);
            let (d_prev_ring, d_next_ring) = boundary_dist(offset, 4096);
            writeln!(
                fbound,
                "{},{},{},{},{},{},{},{},{}",
                name,
                kind,
                offset,
                d_prev_64k,
                d_next_64k,
                d_prev_ring,
                d_next_ring,
                offset,
                end_len.saturating_sub(offset),
            )
            .unwrap();
        }

        reports.push(FileReport {
            name: name.to_string(),
            variant,
            token_idx,
            total_tokens: actual.len(),
            input_len: input.len(),
            payload_len: payload.len(),
            decompressed_offset: dump.decompressed_offset,
            compressed_offset,
            residual: dump.residual,
            predicted: dump.predicted,
            actual: dump.actual,
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
        });
    }

    // --- summary.csv ---
    let summary_path = out_dir.join("summary.csv");
    let mut fs_out = fs::File::create(&summary_path).expect("create summary.csv");
    writeln!(
        fs_out,
        "name,variant,token_idx,total_tokens,input_len,payload_len,decompressed_offset,compressed_offset,residual,predicted,actual,raw_pos,r,n_candidates,n_in_tree,leaf_in_tree,leaf_rank_by_ring_dist,leaf_rank_by_write_tick,leaf_rank_by_inorder,block_all_start,block_all_end,block_all_size,block_all_is_end,hyp_a_pass,hyp_b_pass,eof_closest_dist_match,eof_farthest_dist_match,eof_closest_to_raw_match"
    )
    .unwrap();
    for r in &reports {
        writeln!(
            fs_out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.name,
            r.variant,
            r.token_idx,
            r.total_tokens,
            r.input_len,
            r.payload_len,
            r.decompressed_offset,
            r.compressed_offset,
            r.residual,
            token_cell(&r.predicted),
            token_cell(&r.actual),
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
        )
        .unwrap();
    }

    eprintln!("wrote {} candidate rows to {:?}", 8, candidates_path);
    eprintln!("wrote boundary rows to {:?}", boundary_path);
    eprintln!("wrote {} summary rows to {:?}", reports.len(), summary_path);

    eprintln!("\n--- 8-file summary ---");
    eprintln!(
        "{:10} {:>6} {:>4} {:>4} {:>6} {:>6} {:>6} {:>6}  a     b     closest farthest closest_raw",
        "name", "tok", "n", "ntr", "rk_d", "rk_t", "rk_io", "blk_sz"
    );
    for r in &reports {
        eprintln!(
            "{:10} {:>6} {:>4} {:>4} {:>6} {:>6} {:>6} {:>6}  {:<5} {:<5} {:<7} {:<8} {:<11}",
            r.name,
            r.token_idx,
            r.n_candidates,
            r.n_in_tree,
            r.leaf_rank_by_ring_dist.map(|v| v as i64).unwrap_or(-1),
            r.leaf_rank_by_write_tick.map(|v| v as i64).unwrap_or(-1),
            r.leaf_rank_by_inorder.map(|v| v as i64).unwrap_or(-1),
            r.block_all_size,
            r.hyp_a_pass,
            r.hyp_b_pass,
            r.eof_closest_dist_match,
            r.eof_farthest_dist_match,
            r.eof_closest_to_raw_match,
        );
    }

    let n_a = reports.iter().filter(|r| r.hyp_a_pass).count();
    let n_b = reports.iter().filter(|r| r.hyp_b_pass).count();
    let n_closest = reports.iter().filter(|r| r.eof_closest_dist_match).count();
    let n_farthest = reports.iter().filter(|r| r.eof_farthest_dist_match).count();
    let n_closest_raw = reports.iter().filter(|r| r.eof_closest_to_raw_match).count();
    eprintln!(
        "\ntotals: hyp_a(in_tree)={}/8 hyp_b(block_end-1)={}/8 eof_closest_dist={}/8 eof_farthest_dist={}/8 eof_closest_to_raw={}/8",
        n_a, n_b, n_closest, n_farthest, n_closest_raw
    );
}
