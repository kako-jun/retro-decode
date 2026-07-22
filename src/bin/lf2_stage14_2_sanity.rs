//! Stage 14-2 (Issue #14): sanity check that `compress_okumura_writetime_custom_ring`
//! (fill=0x20, r_init_delta=0) reproduces the 4 existing winning writetime
//! variants byte-for-byte, before using it for ring-init experiments.
//!
//! usage: cargo run --release --bin lf2_stage14_2_sanity -- <LF2_DIR>

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_clip_writetime_ascending, compress_okumura_clip_writetime_descending,
    compress_okumura_plus1_writetime_ascending, compress_okumura_plus1_writetime_descending,
    compress_okumura_writetime_custom_ring, F, N,
};
use retro_decode::formats::toheart::verify_harness;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir>", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let files = verify_harness::list_lf2_files(&dir, None).expect("list dir");
    let fill_buf = [0x20u8; N + F - 1];

    let mut mismatches = 0usize;
    let mut checked = 0usize;
    for path in &files {
        let decoded = match verify_harness::load_and_decode(path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let input = &decoded.ring_input;
        checked += 1;

        let pairs: [(bool, bool, &str); 4] = [
            (false, false, "clip_wtd"),
            (false, true, "clip_wta"),
            (true, false, "plus1_wtd"),
            (true, true, "plus1_wta"),
        ];
        for (plus1, ascending, label) in pairs {
            let baseline = match (plus1, ascending) {
                (false, false) => compress_okumura_clip_writetime_descending(input),
                (false, true) => compress_okumura_clip_writetime_ascending(input),
                (true, false) => compress_okumura_plus1_writetime_descending(input),
                (true, true) => compress_okumura_plus1_writetime_ascending(input),
            };
            let custom = compress_okumura_writetime_custom_ring(input, plus1, ascending, fill_buf, 0);
            if baseline != custom {
                mismatches += 1;
                eprintln!(
                    "MISMATCH {} {}: baseline {} tokens, custom {} tokens",
                    decoded.name,
                    label,
                    baseline.len(),
                    custom.len()
                );
                let n = baseline.len().min(custom.len());
                for i in 0..n {
                    if baseline[i] != custom[i] {
                        eprintln!("  first diff at token {}: baseline={:?} custom={:?}", i, baseline[i], custom[i]);
                        break;
                    }
                }
            }
        }
    }
    eprintln!("checked {} files x 4 modes, mismatches={}", checked, mismatches);
    if mismatches == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
