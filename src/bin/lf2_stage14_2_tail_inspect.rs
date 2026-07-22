//! Stage 14-2 (Issue #14 脈) 補助デバッグ: near-miss 上位ファイルの「最後の
//! トークンだけが違う」性質を実際に覗き見る (提出物には含めない使い捨てツール)。
use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::okumura_lzss::compress_okumura;
use retro_decode::formats::toheart::verify_harness;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = PathBuf::from(&args[1]);
    for name in &args[2..] {
        let path = dir.join(name);
        let data = fs::read(&path).unwrap();
        let (w, h, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h).unwrap();
        let gen = compress_okumura(&leaf.ring_input);
        let n = leaf.tokens.len().min(gen.len());
        println!("{}: leaf_tokens={} gen_tokens={}", name, leaf.tokens.len(), gen.len());
        for i in n.saturating_sub(3)..n {
            println!("  idx {} leaf={:?} gen={:?}", i, leaf.tokens[i], gen[i]);
        }
        if gen.len() > n {
            println!("  gen has extra: {:?}", &gen[n..]);
        }
        if leaf.tokens.len() > n {
            println!("  leaf has extra: {:?}", &leaf.tokens[n..]);
        }
    }
}
