//! LF2 ペイロードをトークン列として取り出すためのユーティリティ。
//!
//! Issue kako-jun/retro-decode#4 用。既存 `Lf2Image::decompress_lzss` は
//! ピクセル配列だけ返すため、圧縮トークン単位での比較ができなかった。
//! 本モジュールはそれと同じ展開ロジックをなぞりつつ、トークン列（リテラル
//! / マッチ (pos, len)）と、展開時に ring buffer に書き込まれた順の
//! ピクセル列（= 奥村エンコーダへの入力と同じ並び）を残す。
//!
//! 既存 `decompress_lzss` はバイナリ安定のため一切触らない。
//!
//! LF2 の圧縮フォーマット（`decompress_lzss` の逆）:
//! - 8 ステップごとに flag byte（XOR 0xff）
//! - flag ビットが 1: リテラル (pixel XOR 0xff)
//! - flag ビットが 0: 2 バイトのマッチ参照
//!   upper = (len-3) | ((pos & 0x0f) << 4)   (XOR 0xff で格納)
//!   lower = (pos >> 4) & 0xff               (XOR 0xff で格納)
//! - pos は 0..4096 の**絶対リングバッファ位置**（奥村 `match_position` と同じ表現）
//! - len は 3..=18

use anyhow::{anyhow, Result};

/// Leaf 側の圧縮トークン 1 個。`pos` は 0..N=4096 の絶対リングバッファ位置、
/// `len` は実長（3..=18）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafToken {
    Literal(u8),
    Match { pos: u16, len: u8 },
}

/// `decompress_to_tokens` の戻り値。
///
/// - `tokens`: 圧縮ペイロードを展開して得たトークン列
/// - `ring_input`: トークンを順に展開したときに生成されたバイト列
///   （リテラルならそのまま、マッチなら ring buffer からコピーした結果）。
///   これは ring buffer への書き込み順＝奥村エンコーダへの入力と同一の
///   バイト並びになる。既存 `decompress_lzss` の出力を Y 反転で元に戻した
///   ものと一致するはずだが、トークン比較をするときは**この配列**を
///   使うのが最も安全（Y 反転を経由しないため）
#[derive(Debug)]
pub struct LeafDecode {
    pub tokens: Vec<LeafToken>,
    pub ring_input: Vec<u8>,
}

/// LF2 の圧縮ペイロードをトークン列に展開する。
///
/// 既存 `Lf2Image::decompress_lzss` と同じリングバッファ初期化・同じ
/// ビット順・同じ XOR マスクを使う。戻り値 `ring_input` は展開されたバイト列で、
/// 奥村エンコーダに食わせる入力としてそのまま使える。
pub fn decompress_to_tokens(
    compressed: &[u8],
    width: u16,
    height: u16,
) -> Result<LeafDecode> {
    let total_pixels = (width as usize) * (height as usize);

    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos: usize = 0x0fee;

    let mut data_pos = 0usize;
    let mut produced = 0usize;
    let mut flag: u8 = 0;
    let mut flag_count: u8 = 0;

    let mut tokens: Vec<LeafToken> = Vec::new();
    let mut ring_input: Vec<u8> = Vec::with_capacity(total_pixels);

    while produced < total_pixels {
        if flag_count == 0 {
            if data_pos >= compressed.len() {
                return Err(anyhow!(
                    "unexpected end of payload at flag byte (produced {}/{}, data_pos {})",
                    produced,
                    total_pixels,
                    data_pos
                ));
            }
            flag = compressed[data_pos] ^ 0xff;
            data_pos += 1;
            flag_count = 8;
        }

        if (flag & 0x80) != 0 {
            // リテラル
            if data_pos >= compressed.len() {
                return Err(anyhow!(
                    "unexpected end of payload at literal byte (produced {}/{})",
                    produced,
                    total_pixels
                ));
            }
            let pixel = compressed[data_pos] ^ 0xff;
            data_pos += 1;

            tokens.push(LeafToken::Literal(pixel));
            ring[ring_pos] = pixel;
            ring_pos = (ring_pos + 1) & 0x0fff;
            ring_input.push(pixel);
            produced += 1;
        } else {
            // マッチ
            if data_pos + 1 >= compressed.len() {
                return Err(anyhow!(
                    "unexpected end of payload at match pair (produced {}/{})",
                    produced,
                    total_pixels
                ));
            }
            let upper = compressed[data_pos] ^ 0xff;
            let lower = compressed[data_pos + 1] ^ 0xff;
            data_pos += 2;

            let length = ((upper & 0x0f) as usize) + 3;
            let position =
                (((upper >> 4) as usize) | ((lower as usize) << 4)) & 0x0fff;

            tokens.push(LeafToken::Match {
                pos: position as u16,
                len: length as u8,
            });

            let mut copy_pos = position;
            for _ in 0..length {
                if produced >= total_pixels {
                    break;
                }
                let pixel = ring[copy_pos];
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                copy_pos = (copy_pos + 1) & 0x0fff;
                ring_input.push(pixel);
                produced += 1;
            }
        }

        flag <<= 1;
        flag_count -= 1;
    }

    Ok(LeafDecode { tokens, ring_input })
}

/// `(pos, len)` マッチ候補 1 件。`pos` は 0..4096 の絶対リングバッファ位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchCandidate {
    pub pos: u16,
    pub len: u8,
}

/// 与えられた ring buffer 状態と input 位置に対して、長さ 3..=18 の全マッチ
/// 候補を列挙する。
///
/// 奥村の `text_buf` は `[u8; N + F - 1]` で末尾 F-1 が overlap 領域になっている
/// が、本関数はあくまで「ring buffer の各位置から始めて input[s..] と最長
/// どれだけ一致するか」を総当たりする。wrap-around は `& 0x0fff` で処理。
///
/// ring buffer 上の同じ系列に対して、長さ L が成立すれば 3..=L も自動的に
/// 候補となる（奥村エンコーダは 3..=L の中から L を選ぶが、別エンコーダが
/// 短い L' を返すこともあるので両方候補集合に入れる）。
///
/// 戻り値は `(pos, len)` の並び。重複 `(pos, len)` ペアは 1 度だけ。
/// 出力順序は pos 昇順 → len 昇順。
pub fn enumerate_match_candidates(
    ring: &[u8; 0x1000],
    input: &[u8],
    s: usize,
) -> Vec<MatchCandidate> {
    let mut out: Vec<MatchCandidate> = Vec::new();
    if s >= input.len() {
        return out;
    }
    let remaining = input.len() - s;
    let max_len_by_input = remaining.min(18);
    if max_len_by_input < 3 {
        return out;
    }

    for pos in 0..0x1000usize {
        let mut l = 0usize;
        while l < max_len_by_input {
            let rb = ring[(pos + l) & 0x0fff];
            let ib = input[s + l];
            if rb != ib {
                break;
            }
            l += 1;
        }
        if l >= 3 {
            for len in 3..=l {
                out.push(MatchCandidate {
                    pos: pos as u16,
                    len: len as u8,
                });
            }
        }
    }

    out.sort_by(|a, b| a.pos.cmp(&b.pos).then(a.len.cmp(&b.len)));
    out
}

/// write-back を考慮した候補列挙版。
///
/// `distance < len` の自己参照マッチでは、コピーしながら ring buffer へ
/// 書き戻したバイトをそのまま続きで読む。snapshot のみを見る単純列挙では
/// この種の候補を過小検出するため、decoder と同じ write-back を模擬する。
pub fn enumerate_match_candidates_with_writeback(
    ring: &[u8; 0x1000],
    input: &[u8],
    s: usize,
    r: usize,
) -> Vec<MatchCandidate> {
    let mut out: Vec<MatchCandidate> = Vec::new();
    if s >= input.len() {
        return out;
    }
    let remaining = input.len() - s;
    let max_len_by_input = remaining.min(18);
    if max_len_by_input < 3 {
        return out;
    }

    for pos in 0..0x1000usize {
        let mut tmp_ring = *ring;
        let mut tmp_r = r & 0x0fff;
        let mut copy_pos = pos;
        let mut l = 0usize;

        while l < max_len_by_input {
            let rb = tmp_ring[copy_pos];
            let ib = input[s + l];
            if rb != ib {
                break;
            }
            tmp_ring[tmp_r] = rb;
            tmp_r = (tmp_r + 1) & 0x0fff;
            copy_pos = (copy_pos + 1) & 0x0fff;
            l += 1;
        }

        if l >= 3 {
            for len in 3..=l {
                out.push(MatchCandidate {
                    pos: pos as u16,
                    len: len as u8,
                });
            }
        }
    }

    out.sort_by(|a, b| a.pos.cmp(&b.pos).then(a.len.cmp(&b.len)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::toheart::lf2::Lf2Image;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn decompress_to_tokens_roundtrip_trivial() {
        // 単純なリテラルだけの圧縮ペイロードを手で作って展開する。
        // width=4, height=1 → 4 ピクセル。flag=0xff (全ビット 1 = リテラル)
        // flag XOR 0xff = 0x00 が格納される。
        // pixels [0x10, 0x20, 0x30, 0x40] の XOR 0xff = [0xef, 0xdf, 0xcf, 0xbf]
        let compressed = vec![
            0x00, // flag = 0xff after XOR
            0xef, 0xdf, 0xcf, 0xbf,
        ];
        // 残り 4 ビットぶんは消費されない

        let decoded = decompress_to_tokens(&compressed, 4, 1).unwrap();
        assert_eq!(decoded.tokens.len(), 4);
        assert_eq!(decoded.tokens[0], LeafToken::Literal(0x10));
        assert_eq!(decoded.tokens[3], LeafToken::Literal(0x40));
        assert_eq!(decoded.ring_input, vec![0x10, 0x20, 0x30, 0x40]);
    }

    #[test]
    fn enumerate_candidates_finds_initial_space_run() {
        // ring が 0x20 で埋まっているとき、input 先頭の 0x20 連続に対して
        // 多数の候補が見つかるはず。
        let ring = [0x20u8; 0x1000];
        let input = vec![0x20u8; 30];
        let cands = enumerate_match_candidates(&ring, &input, 0);
        // 各 pos (0..4096) について長さ 3..=18 が候補になるはず
        assert_eq!(cands.len(), 4096 * (18 - 3 + 1));
        // 最大長は 18 で打ち切られている
        assert!(cands.iter().all(|c| c.len <= 18));
        // 3 未満は出ない
        assert!(cands.iter().all(|c| c.len >= 3));
    }

    #[test]
    fn enumerate_candidates_no_match_returns_empty() {
        let mut ring = [0x00u8; 0x1000];
        ring[100] = 0xAA; // 単発なので 3 バイト連続一致は取れない
        let input = vec![0xAA, 0xAA, 0xAA, 0xAA];
        let cands = enumerate_match_candidates(&ring, &input, 0);
        assert!(cands.is_empty(), "no 3-byte run should give no candidates");
    }

    #[test]
    fn enumerate_candidates_exact_3byte_match() {
        let mut ring = [0x00u8; 0x1000];
        ring[10] = 0xDE;
        ring[11] = 0xAD;
        ring[12] = 0xBE;
        let input = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let cands = enumerate_match_candidates(&ring, &input, 0);
        // pos=10 で len=3 のみ
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0], MatchCandidate { pos: 10, len: 3 });
    }

    #[test]
    fn enumerate_candidates_wrap_around() {
        let mut ring = [0x00u8; 0x1000];
        ring[0x0fff] = 0x11;
        ring[0x0000] = 0x22;
        ring[0x0001] = 0x33;
        let input = vec![0x11, 0x22, 0x33];
        let cands = enumerate_match_candidates(&ring, &input, 0);
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0], MatchCandidate { pos: 0x0fff, len: 3 });
    }

    #[test]
    fn enumerate_candidates_respect_input_end() {
        // input が短くて 3 バイトないときは何も返らない
        let ring = [0x20u8; 0x1000];
        let input = vec![0x20, 0x20];
        let cands = enumerate_match_candidates(&ring, &input, 0);
        assert!(cands.is_empty());
    }

    #[test]
    fn enumerate_candidates_with_writeback_finds_self_reference_run() {
        let mut ring = [0x00u8; 0x1000];
        let r = 0x0feeusize;
        let pos = (r + 0x1000 - 1) & 0x0fff;
        ring[pos] = b'A';
        let input = b"AAAAAA".to_vec();

        let snapshot = enumerate_match_candidates(&ring, &input, 0);
        assert!(
            !snapshot.iter().any(|c| c.pos as usize == pos && c.len == 6),
            "snapshot-only enumeration should miss self-reference len=6"
        );

        let with_writeback =
            enumerate_match_candidates_with_writeback(&ring, &input, 0, r);
        assert!(with_writeback
            .iter()
            .any(|c| c.pos as usize == pos && c.len == 6));
    }

    #[test]
    fn ring_input_matches_decompress_lzss_after_unflipping_rows() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_assets/generated/debug_compression.lf2");
        let data = fs::read(&path).expect("read test LF2");
        let width = u16::from_le_bytes([data[12], data[13]]);
        let height = u16::from_le_bytes([data[14], data[15]]);
        let color_count = data[0x16];
        let payload_start = 0x18 + (color_count as usize) * 3;

        let decoded =
            decompress_to_tokens(&data[payload_start..], width, height).expect("token decode");
        let image = Lf2Image::open(&path).expect("lf2 open");

        let mut unflipped = Vec::with_capacity(image.pixels.len());
        for y in (0..height as usize).rev() {
            let row_start = y * width as usize;
            let row_end = row_start + width as usize;
            unflipped.extend_from_slice(&image.pixels[row_start..row_end]);
        }

        assert_eq!(decoded.ring_input, unflipped);
    }
}
