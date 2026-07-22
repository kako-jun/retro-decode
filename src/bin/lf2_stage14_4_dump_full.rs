//! Stage 14-4 (Issue #14 脈: ⑲) — 1ファイルの最終トークンタイ候補を全件ダンプする
//! (手動観察用の使い捨てツール)。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_eof_retie_last_candidates, TaxBase,
};
use retro_decode::formats::toheart::verify_harness;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let name = &args[2];
    let base = match args[3].as_str() {
        "basic" => TaxBase::Basic,
        "no_dummy" => TaxBase::NoDummy,
        "fill00" => TaxBase::Fill00,
        _ => panic!("bad base"),
    };

    let path = dir.join(name);
    let data = fs::read(&path).unwrap();
    let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
    let leaf =
        retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h)
            .unwrap();
    let n = leaf.tokens.len();
    let leaf_last = leaf.tokens[n - 1];
    let (leaf_pos, leaf_len) = match leaf_last {
        LeafToken::Match { pos, len } => (pos as i32, len as i32),
        LeafToken::Literal(_) => (-1, -1),
    };
    eprintln!("width={} height={} leaf_pos={} leaf_len={}", w, h, leaf_pos, leaf_len);

    let dump = compress_okumura_eof_retie_last_candidates(&leaf.ring_input, base);
    let (r, len, raw_pos, mut cands) = dump.unwrap();
    eprintln!("r={} len={} raw_pos={} n_cands={}", r, len, raw_pos, cands.len());
    cands.sort_by_key(|(p, _, _)| *p);

    println!("pos,dist,write_tick,is_leaf,is_raw,gap_to_prev_pos");
    let mut prev: Option<i32> = None;
    for (p, d, t) in &cands {
        let gap = prev.map(|pp| p - pp).unwrap_or(-1);
        println!(
            "{},{},{},{},{},{}",
            p,
            d,
            t,
            *p == leaf_pos,
            *p == raw_pos,
            gap
        );
        prev = Some(*p);
    }
}
