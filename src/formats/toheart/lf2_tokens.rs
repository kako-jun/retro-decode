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
//!     upper = (len-3) | ((pos & 0x0f) << 4)   (XOR 0xff で格納)
//!     lower = (pos >> 4) & 0xff               (XOR 0xff で格納)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_to_tokens_roundtrip_trivial() {
        // 単純なリテラルだけの圧縮ペイロードを手で作って展開する。
        // width=4, height=1 → 4 ピクセル。flag=0xff (全ビット 1 = リテラル)
        // flag XOR 0xff = 0x00 が格納される。
        // pixels [0x10, 0x20, 0x30, 0x40] の XOR 0xff = [0xef, 0xdf, 0xcf, 0xbf]
        let mut compressed = Vec::new();
        compressed.push(0x00); // flag = 0xff after XOR
        compressed.push(0xef);
        compressed.push(0xdf);
        compressed.push(0xcf);
        compressed.push(0xbf);
        // 残り 4 ビットぶんは消費されない

        let decoded = decompress_to_tokens(&compressed, 4, 1).unwrap();
        assert_eq!(decoded.tokens.len(), 4);
        assert_eq!(decoded.tokens[0], LeafToken::Literal(0x10));
        assert_eq!(decoded.tokens[3], LeafToken::Literal(0x40));
        assert_eq!(decoded.ring_input, vec![0x10, 0x20, 0x30, 0x40]);
    }
}
