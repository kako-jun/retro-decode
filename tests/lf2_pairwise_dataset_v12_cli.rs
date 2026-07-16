//! Issue #14 v12: lf2_pairwise_dataset_v12 バイナリの異常系スモーク。
//!
//! process_file はバイナリ内の非公開関数なので、CARGO_BIN_EXE 経由で
//! 実バイナリを叩き、0 バイトファイルと非 LF2 ファイルが errors として
//! カウントされ、プロセス自体は落ちないことを検証する。

use std::fs;
use std::process::Command;

#[test]
fn v12_cli_counts_errors_for_zero_byte_and_non_lf2_files() {
    let dir =
        std::env::temp_dir().join(format!("retro_decode_v12_cli_test_{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    // 0 バイトファイル (ヘッダ長未満 → parse_lf2 が None)
    fs::write(dir.join("empty.LF2"), b"").expect("write empty");
    // ヘッダ長はあるが magic 不一致
    fs::write(
        dir.join("garbage.LF2"),
        b"NOTLEAF0................................",
    )
    .expect("write garbage");

    let out_csv = dir.join("out.csv");
    let output = Command::new(env!("CARGO_BIN_EXE_lf2_pairwise_dataset_v12"))
        .arg(&dir)
        .arg(&out_csv)
        .output()
        .expect("run lf2_pairwise_dataset_v12");

    assert!(
        output.status.success(),
        "binary must not crash on broken inputs: status={:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("errors=2"),
        "both broken files must be counted as errors: stderr={}",
        stderr
    );
    assert!(stderr.contains("processed=0"), "stderr={}", stderr);

    // CSV はヘッダ行のみ
    let csv = fs::read_to_string(&out_csv).expect("read csv");
    assert_eq!(csv.lines().count(), 1, "csv must contain only the header");

    fs::remove_dir_all(&dir).ok();
}
