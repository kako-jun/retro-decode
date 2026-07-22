//! Stage 14-2 (Issue #14 脈: per-file 小状態フィッティング「⑧ ring 初期内容
//! 汚染統合」): near-miss 台帳。
//!
//! union257 (`.local_data/stage12_18/union_all.txt`) に含まれない265本
//! それぞれについて、既存の全変種ファミリ (`lf2_variant_best_fit` の91変種 +
//! stage12-16/17 の4象限系統15変種) を流し、実 Leaf トークン列との最長一致
//! 接頭辞 (token 単位) が最も長い = 最初の相違位置が最も遅い変種を特定する。
//!
//! 出力: `<out>` (既定 `.local_data/stage14_2/near_miss_ledger.csv`)。
//! 列: name,total_tokens,best_variant,match_prefix_len,remaining_tokens,input_len
//! remaining_tokens = total_tokens - match_prefix_len (小さいほど惜しい)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage14_2_near_miss -- <LF2_DIR> \
//!       --union-file <union_all.txt> [--out PATH]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::naive_scan_lzss;
use retro_decode::formats::toheart::okumura_lzss::{self, Token};
use retro_decode::formats::toheart::verify_harness;

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        ("okumura_basic", okumura_lzss::compress_okumura as fn(&[u8]) -> Vec<Token>),
        ("okumura_distance_tie", okumura_lzss::compress_okumura_distance_tie),
        ("okumura_dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("okumura_lazy", okumura_lzss::compress_okumura_lazy),
        ("okumura_no_dummy", okumura_lzss::compress_okumura_no_dummy),
        ("okumura_no_dummy_tail1", okumura_lzss::compress_okumura_no_dummy_tail1),
        ("okumura_no_dummy_left_first_tail1", okumura_lzss::compress_okumura_no_dummy_left_first_tail1),
        ("okumura_no_dummy_no_swap_tail1", okumura_lzss::compress_okumura_no_dummy_no_swap_tail1),
        ("okumura_no_dummy_lit_for_0x20", okumura_lzss::compress_okumura_no_dummy_lit_for_0x20),
        ("okumura_basic_tail1", okumura_lzss::compress_okumura_basic_tail1),
        ("okumura_dummy_then_drop_tail1", okumura_lzss::compress_okumura_dummy_then_drop_tail1),
        ("okumura_basic_tail1_full", okumura_lzss::compress_okumura_basic_tail1_full),
        ("okumura_no_dummy_tail1_full", okumura_lzss::compress_okumura_no_dummy_tail1_full),
        ("okumura_one_dummy_at_rf", okumura_lzss::compress_okumura_one_dummy_at_rf),
        ("okumura_dummy_then_drop", okumura_lzss::compress_okumura_dummy_then_drop),
        ("okumura_uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("okumura_min_tokens", okumura_lzss::compress_okumura_min_tokens),
        ("okumura_min_bytes", okumura_lzss::compress_okumura_min_bytes),
        ("okumura_min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("okumura_min_bytes_oku_pref", okumura_lzss::compress_okumura_min_bytes_oku_pref),
        ("okumura_combo", okumura_lzss::compress_okumura_combo),
        ("okumura_no_dummy_dyntie", okumura_lzss::compress_okumura_no_dummy_dyntie),
        ("okumura_no_dummy_left_first", okumura_lzss::compress_okumura_no_dummy_left_first),
        ("okumura_no_dummy_left_first_lazy", okumura_lzss::compress_okumura_no_dummy_left_first_lazy),
        ("okumura_no_dummy_left_first_eq", okumura_lzss::compress_okumura_no_dummy_left_first_eq),
        ("okumura_no_dummy_eq", okumura_lzss::compress_okumura_no_dummy_eq),
        ("okumura_no_dummy_distance_tie", okumura_lzss::compress_okumura_no_dummy_distance_tie),
        ("okumura_no_dummy_no_swap", okumura_lzss::compress_okumura_no_dummy_no_swap),
        ("okumura_dummy_no_swap", okumura_lzss::compress_okumura_dummy_no_swap),
        ("okumura_no_dummy_min4", okumura_lzss::compress_okumura_no_dummy_min4),
        ("naive_backward_strict", |i: &[u8]| naive_scan_lzss::compress_naive_backward(i, false)),
        ("naive_backward_equal", |i: &[u8]| naive_scan_lzss::compress_naive_backward(i, true)),
        ("naive_forward_strict", |i: &[u8]| naive_scan_lzss::compress_naive_forward_pos(i, false)),
        ("naive_forward_equal", |i: &[u8]| naive_scan_lzss::compress_naive_forward_pos(i, true)),
        ("naive_first_match", |i: &[u8]| naive_scan_lzss::compress_naive_first_match(i)),
        ("hash_chain_first", |i: &[u8]| naive_scan_lzss::compress_hash_chain(i, naive_scan_lzss::HashMode::FirstMatch)),
        ("hash_chain_best", |i: &[u8]| naive_scan_lzss::compress_hash_chain(i, naive_scan_lzss::HashMode::BestMatch)),
        ("no_init_leftmost", |i: &[u8]| naive_scan_lzss::compress_no_init_match(i, naive_scan_lzss::NoInitMode::BestLeftmost)),
        ("no_init_min_dist", |i: &[u8]| naive_scan_lzss::compress_no_init_match(i, naive_scan_lzss::NoInitMode::BestMinDist)),
        ("no_init_max_dist", |i: &[u8]| naive_scan_lzss::compress_no_init_match(i, naive_scan_lzss::NoInitMode::BestMaxDist)),
        ("circular_first", |i: &[u8]| naive_scan_lzss::compress_circular(i, naive_scan_lzss::CircularMode::FirstMatch)),
        ("circular_best", |i: &[u8]| naive_scan_lzss::compress_circular(i, naive_scan_lzss::CircularMode::BestMatch)),
        ("okumura_lazy_eq", okumura_lzss::compress_okumura_lazy_eq),
        ("okumura_no_dummy_lazy_eq", okumura_lzss::compress_okumura_no_dummy_lazy_eq),
        ("okumura_no_dummy_left_first_lazy_eq", okumura_lzss::compress_okumura_no_dummy_left_first_lazy_eq),
        ("okumura_no_dummy_no_swap_lazy_eq", okumura_lzss::compress_okumura_no_dummy_no_swap_lazy_eq),
        ("okumura_left_first_lazy_eq", okumura_lzss::compress_okumura_left_first_lazy_eq),
        ("okumura_no_dummy_left_first_lazy_eq_tie_eq", okumura_lzss::compress_okumura_no_dummy_left_first_lazy_eq_tie_eq),
        ("okumura_max_dist_tie", okumura_lzss::compress_okumura_max_dist_tie),
        ("okumura_basic_no_init", okumura_lzss::compress_okumura_basic_no_init),
        ("okumura_with_tie_strict", |i: &[u8]| okumura_lzss::compress_okumura_with_tie(i, false)),
        ("okumura_with_tie_equal", |i: &[u8]| okumura_lzss::compress_okumura_with_tie(i, true)),
        ("okumura_no_dummy_min_dist_exh_tail1", okumura_lzss::compress_okumura_no_dummy_min_dist_exh_tail1),
        ("okumura_no_dummy_max_dist_exh_tail1", okumura_lzss::compress_okumura_no_dummy_max_dist_exh_tail1),
        ("okumura_no_dummy_len_split13_exh_tail1", okumura_lzss::compress_okumura_no_dummy_len_split13_exh_tail1),
        ("okumura_no_dummy_len_split14_exh_tail1", okumura_lzss::compress_okumura_no_dummy_len_split14_exh_tail1),
        ("okumura_no_dummy_len_split15_exh_tail1", okumura_lzss::compress_okumura_no_dummy_len_split15_exh_tail1),
        ("okumura_no_dummy_len_split16_exh_tail1", okumura_lzss::compress_okumura_no_dummy_len_split16_exh_tail1),
        ("okumura_no_dummy_min_dist_only18_exh_tail1", okumura_lzss::compress_okumura_no_dummy_min_dist_only18_exh_tail1),
        ("okumura_basic_min_dist_only18_exh_tail1", okumura_lzss::compress_okumura_basic_min_dist_only18_exh_tail1),
        ("okumura_basic_min_dist_exh_tail1", okumura_lzss::compress_okumura_basic_min_dist_exh_tail1),
        ("okumura_basic_max_dist_exh_tail1", okumura_lzss::compress_okumura_basic_max_dist_exh_tail1),
        ("okumura_basic_len_split8_exh_tail1", okumura_lzss::compress_okumura_basic_len_split8_exh_tail1),
        ("okumura_basic_len_split13_exh_tail1", okumura_lzss::compress_okumura_basic_len_split13_exh_tail1),
        ("okumura_no_dummy_tail1_xor", okumura_lzss::compress_okumura_no_dummy_tail1_xor),
        ("okumura_no_dummy_tail1_add", okumura_lzss::compress_okumura_no_dummy_tail1_add),
        ("okumura_basic_tail1_xor", okumura_lzss::compress_okumura_basic_tail1_xor),
        ("okumura_basic_tail1_add", okumura_lzss::compress_okumura_basic_tail1_add),
        ("okumura_basic_no_init_tail1", okumura_lzss::compress_okumura_basic_no_init_tail1),
        ("okumura_no_dummy_no_init_tail1", okumura_lzss::compress_okumura_no_dummy_no_init_tail1),
        ("okumura_basic_no_init_strict_tail1", okumura_lzss::compress_okumura_basic_no_init_strict_tail1),
        ("okumura_basic_tail1_stop_on_input", okumura_lzss::compress_okumura_basic_tail1_stop_on_input),
        ("okumura_no_dummy_tail1_stop_on_input", okumura_lzss::compress_okumura_no_dummy_tail1_stop_on_input),
        ("okumura_basic_tail1_phantom_lit", okumura_lzss::compress_okumura_basic_tail1_phantom_lit),
        ("okumura_no_dummy_tail1_phantom_lit", okumura_lzss::compress_okumura_no_dummy_tail1_phantom_lit),
        ("okumura_basic_tail1_phantom_lit2", okumura_lzss::compress_okumura_basic_tail1_phantom_lit2),
        ("okumura_basic_tail1_phantom_lit_pad8", okumura_lzss::compress_okumura_basic_tail1_phantom_lit_pad8),
        ("okumura_basic_tail1_no_cap", okumura_lzss::compress_okumura_basic_tail1_no_cap),
        ("okumura_no_dummy_tail1_no_cap", okumura_lzss::compress_okumura_no_dummy_tail1_no_cap),
        ("okumura_brute_max_dist", okumura_lzss::compress_okumura_brute_max_dist),
        ("okumura_brute_min_dist", okumura_lzss::compress_okumura_brute_min_dist),
        ("okumura_chain4_max", okumura_lzss::compress_okumura_chain4_max),
        ("okumura_chain8_max", okumura_lzss::compress_okumura_chain8_max),
        ("okumura_chain16_max", okumura_lzss::compress_okumura_chain16_max),
        ("okumura_chain32_max", okumura_lzss::compress_okumura_chain32_max),
        ("okumura_chain64_max", okumura_lzss::compress_okumura_chain64_max),
        ("okumura_chain4_first", okumura_lzss::compress_okumura_chain4_first),
        ("okumura_chain8_first", okumura_lzss::compress_okumura_chain8_first),
        ("okumura_chain16_first", okumura_lzss::compress_okumura_chain16_first),
        ("okumura_chain32_first", okumura_lzss::compress_okumura_chain32_first),
        ("okumura_chain3_8", okumura_lzss::compress_okumura_chain3_8),
        ("okumura_chain3_32", okumura_lzss::compress_okumura_chain3_32),
        ("okumura_chain3_first8", okumura_lzss::compress_okumura_chain3_first8),
        ("okumura_chain_rev8", okumura_lzss::compress_okumura_chain_rev8),
        ("okumura_chain_rev32", okumura_lzss::compress_okumura_chain_rev32),
        ("okumura_basic_tail1_fill00", okumura_lzss::compress_okumura_basic_tail1_fill00),
        ("okumura_basic_tail1_fillff", okumura_lzss::compress_okumura_basic_tail1_fillff),
        // stage12-16/17 の4象限系統 (clip/plus1 x {Allow,v1,v2,v3,wtd,wta,wtd_kd,wta_kd})。
        // "okumura_basic" (=clip/Allow) は上で既出のため重複させない。
        ("clip_tail_plus1", okumura_lzss::compress_okumura_tail_plus1),
        ("clip_no_bootstrap_v1", okumura_lzss::compress_okumura_clip_no_bootstrap_v1),
        ("plus1_no_bootstrap_v1", okumura_lzss::compress_okumura_plus1_no_bootstrap_v1),
        ("clip_no_bootstrap_v2", okumura_lzss::compress_okumura_clip_no_bootstrap_v2),
        ("plus1_no_bootstrap_v2", okumura_lzss::compress_okumura_plus1_no_bootstrap_v2),
        ("clip_no_bootstrap_v3", okumura_lzss::compress_okumura_clip_no_bootstrap_v3),
        ("plus1_no_bootstrap_v3", okumura_lzss::compress_okumura_plus1_no_bootstrap_v3),
        ("clip_writetime_descending", okumura_lzss::compress_okumura_clip_writetime_descending),
        ("plus1_writetime_descending", okumura_lzss::compress_okumura_plus1_writetime_descending),
        ("clip_writetime_ascending", okumura_lzss::compress_okumura_clip_writetime_ascending),
        ("plus1_writetime_ascending", okumura_lzss::compress_okumura_plus1_writetime_ascending),
        (
            "clip_writetime_descending_keepdummy",
            okumura_lzss::compress_okumura_clip_writetime_descending_keepdummy,
        ),
        (
            "plus1_writetime_descending_keepdummy",
            okumura_lzss::compress_okumura_plus1_writetime_descending_keepdummy,
        ),
        (
            "clip_writetime_ascending_keepdummy",
            okumura_lzss::compress_okumura_clip_writetime_ascending_keepdummy,
        ),
        (
            "plus1_writetime_ascending_keepdummy",
            okumura_lzss::compress_okumura_plus1_writetime_ascending_keepdummy,
        ),
    ]
}

/// `okumura_lzss::Token` と `lf2_tokens::LeafToken` は同じ形の別型なので比較用に揃える。
fn tok_eq(a: &Token, b: &LeafToken) -> bool {
    match (a, b) {
        (Token::Literal(x), LeafToken::Literal(y)) => x == y,
        (Token::Match { pos: p1, len: l1 }, LeafToken::Match { pos: p2, len: l2 }) => p1 == p2 && l1 == l2,
        _ => false,
    }
}

/// 生成トークン列と実 Leaf トークン列の最長一致接頭辞長 (token 単位)。
fn match_prefix_len(generated: &[Token], actual: &[LeafToken]) -> usize {
    let n = generated.len().min(actual.len());
    let mut i = 0;
    while i < n && tok_eq(&generated[i], &actual[i]) {
        i += 1;
    }
    i
}

struct NearMissRow {
    name: String,
    total_tokens: usize,
    input_len: usize,
    best_variant: String,
    match_prefix_len: usize,
    remaining_tokens: usize,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> --union-file <path> [--out <path>]", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut union_file = PathBuf::from(".local_data/stage12_18/union_all.txt");
    let mut out_path = PathBuf::from(".local_data/stage14_2/near_miss_ledger.csv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--union-file" => {
                if let Some(v) = args.get(i + 1) {
                    union_file = PathBuf::from(v);
                }
                i += 2;
            }
            "--out" => {
                if let Some(v) = args.get(i + 1) {
                    out_path = PathBuf::from(v);
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let union_names: std::collections::HashSet<String> = fs::read_to_string(&union_file)
        .expect("read union file")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let files = verify_harness::list_lf2_files(&dir, None).expect("list dir");
    let non_union: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            !union_names.contains(name)
        })
        .collect();
    eprintln!("total non-union files: {}", non_union.len());

    let var_list = variants();
    eprintln!("variant count: {}", var_list.len());

    let n_threads: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(non_union.len().max(1));
    let chunk_size = (non_union.len() + n_threads - 1) / n_threads;

    let rows: Vec<NearMissRow> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in non_union.chunks(chunk_size) {
            let var_list = &var_list;
            let h = scope.spawn(move || {
                let mut out = Vec::new();
                for path in chunk {
                    let decoded = match verify_harness::load_and_decode(path) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("{}", e);
                            continue;
                        }
                    };
                    // actual leaf token stream: re-derive via lf2_tokens directly
                    // (verify_harness only exposes ring_input/payload, so decode again here).
                    let data = fs::read(path).unwrap();
                    let (w, h_, ps) = verify_harness::parse_lf2(&data).unwrap();
                    let leaf = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h_)
                        .expect("decode leaf tokens");
                    let actual = &leaf.tokens;
                    let input = &decoded.ring_input;

                    let mut best_prefix = 0usize;
                    let mut best_name = "none";
                    for (name, f) in var_list.iter() {
                        let toks = f(input);
                        let pl = match_prefix_len(&toks, actual);
                        if pl > best_prefix {
                            best_prefix = pl;
                            best_name = name;
                        }
                    }
                    out.push(NearMissRow {
                        name: decoded.name.clone(),
                        total_tokens: actual.len(),
                        input_len: input.len(),
                        best_variant: best_name.to_string(),
                        match_prefix_len: best_prefix,
                        remaining_tokens: actual.len().saturating_sub(best_prefix),
                    });
                }
                out
            });
            handles.push(h);
        }
        let mut all = Vec::new();
        for h in handles {
            all.extend(h.join().unwrap());
        }
        all
    });

    let mut rows = rows;
    rows.sort_by_key(|r| r.remaining_tokens);

    let mut f = fs::File::create(&out_path).expect("create out csv");
    writeln!(f, "name,total_tokens,best_variant,match_prefix_len,remaining_tokens,input_len").unwrap();
    for r in &rows {
        writeln!(
            f,
            "{},{},{},{},{},{}",
            r.name, r.total_tokens, r.best_variant, r.match_prefix_len, r.remaining_tokens, r.input_len
        )
        .unwrap();
    }

    eprintln!("wrote {} rows to {:?}", rows.len(), out_path);
    let le10 = rows.iter().filter(|r| r.remaining_tokens <= 10).count();
    let le3 = rows.iter().filter(|r| r.remaining_tokens <= 3).count();
    eprintln!("remaining_tokens <= 10: {}", le10);
    eprintln!("remaining_tokens <= 3 : {}", le3);
    eprintln!("--- top 20 near-miss ---");
    for r in rows.iter().take(20) {
        eprintln!(
            "{:12} remaining={:5} prefix={:6}/{:6} best={}",
            r.name, r.remaining_tokens, r.match_prefix_len, r.total_tokens, r.best_variant
        );
    }

    ExitCode::SUCCESS
}
