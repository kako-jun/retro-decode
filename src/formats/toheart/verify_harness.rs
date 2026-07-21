//! Stage 12-15 (Issue #14): stage9/9_2/9_2d/10_2/10_3/10_4/10_5 の
//! verify/debug バイナリ7本に全文コピペされていた共通部分の最小抽出。
//!
//! 抽出したのは `parse_lf2` / `tokens_to_lf2_payload` / ファイル読込・
//! デコードの定型処理 / byte-exact 判定ヘルパーのみで、各バイナリの
//! per-file モード判定・レポート出力ロジックには一切手を入れない
//! (挙動同一が絶対条件、機能追加・磨き込みはしない)。
//!
//! 以後の新しい verify/debug バイナリはこのモジュールを使う。

use std::fs;
use std::path::{Path, PathBuf};

use super::lf2_tokens::decompress_to_tokens;
use super::okumura_lzss::Token;

pub const LF2_MAGIC: &[u8] = b"LEAF256\0";

/// LF2 ヘッダを読み、(width, height, payload_start) を返す。
pub fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
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

/// トークン列を LF2 圧縮ペイロードに直列化する
/// (`Lf2Image::to_lf2_bytes_okumura` の framing と同一)。
pub fn tokens_to_lf2_payload(tokens: &[Token]) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let flag_pos = compressed.len();
        compressed.push(0); // placeholder

        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    compressed.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let encoded_pos = (pos as usize) & 0x0fff;
                    let encoded_len = ((len as usize) - 3) & 0x0f;
                    let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                    let lower = ((encoded_pos >> 4) & 0xff) as u8;
                    compressed.push(upper ^ 0xff);
                    compressed.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }

        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    compressed
}

/// 圧縮関数 `f` の再エンコード結果が元ペイロードと byte-exact 一致するか。
pub fn matches(ring_input: &[u8], orig: &[u8], f: impl Fn(&[u8]) -> Vec<Token>) -> bool {
    let toks = f(ring_input);
    let reenc = tokens_to_lf2_payload(&toks);
    orig == reenc.as_slice()
}

/// ディレクトリ内の `.LF2` ファイルをソート済みで列挙する (大文字小文字を無視)。
pub fn list_lf2_files(dir: &Path, limit: Option<usize>) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("LF2"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    if let Some(n) = limit {
        files.truncate(n);
    }
    Ok(files)
}

/// 1 ファイル分の読込 + LF2 ヘッダ解析 + トークンデコードの定型処理結果。
pub struct DecodedLf2 {
    pub name: String,
    /// 元の圧縮ペイロード (`data[payload_start..]`)。byte-exact 比較対象。
    pub payload: Vec<u8>,
    /// デコード時の ring 書込み順バイト列 (再圧縮の入力)。
    pub ring_input: Vec<u8>,
}

/// `path` を読み込み、LF2 ヘッダ解析・トークンデコードまでを行う。
/// 失敗時は (name 込みの) エラーメッセージを返す
/// (呼び出し側の既存 `eprintln!("read/parse/decode fail ...")` と同一文言)。
pub fn load_and_decode(path: &Path) -> Result<DecodedLf2, String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?")
        .to_string();
    let data = fs::read(path).map_err(|e| format!("read fail {}: {}", name, e))?;
    let (width, height, ps) =
        parse_lf2(&data).ok_or_else(|| format!("parse fail {}", name))?;
    let decoded = decompress_to_tokens(&data[ps..], width, height)
        .map_err(|e| format!("decode fail {}: {}", name, e))?;
    let payload = data[ps..].to_vec();
    Ok(DecodedLf2 {
        name,
        payload,
        ring_input: decoded.ring_input,
    })
}
