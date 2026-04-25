//! Issue kako-jun/retro-decode#3 回帰テスト。
//!
//! 奥村 lzss.c 二分木版ポート (`Lf2Image::to_lf2_bytes_okumura`) が、
//! LVNS3DAT.PAK から展開された LF2 ファイル群のうち現在ペイロード完全
//! 一致しているサンプルについて、今後も一致を維持することを保証する。
//!
//! コーパスは著作権物のためリポジトリにコミットしない。環境変数
//! `LF2_CORPUS_DIR` でディレクトリを指定できる。未設定時は既定パス
//! (`/tmp/lvns3_extract/out/`) を参照する。ファイルが見つからなければ
//! `eprintln!` で skip した旨を表示して OK を返す（CI 配慮）。

use std::env;
use std::fs;
use std::path::PathBuf;

use retro_decode::formats::toheart::Lf2Image;

fn corpus_dir() -> PathBuf {
    PathBuf::from(
        env::var("LF2_CORPUS_DIR").unwrap_or_else(|_| "/tmp/lvns3_extract/out".to_string()),
    )
}

/// ファイルをデコード → 奥村ポートで再エンコードし、ペイロード部
/// (0x18 + palette) 以降がオリジナルと一致することを検証する。
fn assert_payload_matches(filename: &str) {
    let path = corpus_dir().join(filename);
    if !path.exists() {
        eprintln!(
            "[skip] {} not found (set LF2_CORPUS_DIR to enable)",
            path.display()
        );
        return;
    }

    let original = fs::read(&path).expect("read corpus file");
    let lf2 = Lf2Image::from_data(&original).expect("decode LF2");
    let reenc = lf2.to_lf2_bytes_okumura().expect("reencode okumura");

    let color_count = original[0x16] as usize;
    let payload_start = 0x18 + color_count * 3;
    assert!(
        payload_start < original.len(),
        "{}: payload_start out of range",
        filename
    );
    assert!(
        payload_start < reenc.len(),
        "{}: reencoded payload_start out of range",
        filename
    );

    let orig_payload = &original[payload_start..];
    let reenc_payload = &reenc[payload_start..];

    assert_eq!(
        orig_payload,
        reenc_payload,
        "{}: payload bytes differ (orig {} bytes vs reenc {} bytes)",
        filename,
        orig_payload.len(),
        reenc_payload.len(),
    );
}

#[test]
fn regression_c0f01() {
    assert_payload_matches("C0F01.LF2");
}

#[test]
fn regression_cbak_00() {
    assert_payload_matches("CBAK_00.LF2");
}

#[test]
fn regression_clno_00() {
    assert_payload_matches("CLNO_00.LF2");
}

#[test]
fn regression_clno_02() {
    assert_payload_matches("CLNO_02.LF2");
}

#[test]
fn regression_cbak_05() {
    assert_payload_matches("CBAK_05.LF2");
}
