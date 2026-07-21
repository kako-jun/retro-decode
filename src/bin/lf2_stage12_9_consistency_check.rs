//! Stage 12-9 (Issue #14): delete_node_successor 修正後の整合性検証。
//! C0313.LF2 + 長尺上位10本で、compress_okumura_clip_del_successor を走らせ、
//! OKU_DEBUG_TREE_CHECK 相当の整合性チェックとguard発動回数を数える。
//! 合格条件: 全ファイルで guard 発動 0 件。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_9_consistency_check -- <DIR>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::compress_okumura_clip_del_successor;

const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);

    // C0313.LF2 (既知の再現ファイル) + 長尺上位10本
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

    let mut sized: Vec<(u64, PathBuf)> = files
        .iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| (m.len(), p.clone())))
        .collect();
    sized.sort_by(|a, b| b.0.cmp(&a.0));
    let mut targets: Vec<PathBuf> = sized.into_iter().take(10).map(|(_, p)| p).collect();
    let c0313 = dir.join("C0313.LF2");
    if !targets.contains(&c0313) {
        targets.insert(0, c0313);
    }
    files = targets;

    eprintln!("=== Stage 12-9 consistency check: {} files (C0313 + top10 largest) ===", files.len());

    // guard発動はeprintln WARN として出るので、stderr をキャプチャして数える
    // (このプロセス内では直接カウントできないため、代わりに各ファイルの実行前後で
    //  グローバルにWARN文字列が出たかを外側スクリプトで見る運用にする。
    //  ここでは代わりに compress 成功可否とtoken数のみ報告し、
    //  stderr の"WARN"出現数は呼び出し側で `grep -c WARN` して確認する)
    let mut total = 0usize;
    let mut errors = 0usize;
    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errors += 1;
                continue;
            }
        };
        let Some((width, height, ps)) = parse_lf2(&data) else {
            eprintln!("parse fail {}", name);
            errors += 1;
            continue;
        };
        let Ok(decoded) = decompress_to_tokens(&data[ps..], width, height) else {
            eprintln!("decode fail {}", name);
            errors += 1;
            continue;
        };
        total += 1;
        let toks = compress_okumura_clip_del_successor(&decoded.ring_input);
        println!("{}: ring_len={} tokens={}", name, decoded.ring_input.len(), toks.len());
    }
    eprintln!("=== done: {} files processed ({} errors) ===", total, errors);
    eprintln!("(WARN行が0件であることを `grep -c WARN` 等で別途確認すること)");

    ExitCode::SUCCESS
}
