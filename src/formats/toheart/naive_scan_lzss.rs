//! Naive linear-scan LZSS encoder variants (hypothesis tests for Leaf LF2).

use super::okumura_lzss::{Token, F, N, THRESHOLD};

pub fn compress_naive_backward(input: &[u8], allow_equal: bool) -> Vec<Token> {
    let mut text_buf = [0x20u8; N + F - 1];
    let mut out: Vec<Token> = Vec::new();

    let mut r: usize = N - F;
    let mut s: usize = 0;

    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        text_buf[r + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }

    if len == 0 {
        return out;
    }

    loop {
        let max_match = len.min(F);
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        for d in 1..N {
            let pos = (r + N - d) & (N - 1);
            let mut ml = 0usize;
            while ml < max_match && text_buf[pos + ml] == text_buf[r + ml] {
                ml += 1;
            }
            let better = if allow_equal {
                ml >= best_len && ml > 0
            } else {
                ml > best_len
            };
            if better {
                best_len = ml;
                best_pos = pos;
                if best_len >= F {
                    break;
                }
            }
        }

        let last_match_length = if best_len <= THRESHOLD {
            out.push(Token::Literal(text_buf[r]));
            1
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            best_len
        };

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            let c = input[input_idx];
            input_idx += 1;
            text_buf[s] = c;
            if s < F - 1 {
                text_buf[s + N] = c;
            }
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            i += 1;
        }

        while i < last_match_length {
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            i += 1;
            if len == 0 {
                break;
            }
        }

        if len == 0 {
            break;
        }
    }

    out
}

/// pos = 0..N 絶対位置昇順で全候補スキャン (= leftmost first 全数)。
/// best 採用、tie は allow_equal で制御。
pub fn compress_naive_forward_pos(input: &[u8], allow_equal: bool) -> Vec<Token> {
    let mut text_buf = [0x20u8; N + F - 1];
    let mut out: Vec<Token> = Vec::new();

    let mut r: usize = N - F;
    let mut s: usize = 0;

    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        text_buf[r + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }
    if len == 0 {
        return out;
    }

    loop {
        let max_match = len.min(F);
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        for pos in 0..N {
            if pos == r {
                continue;
            }
            let mut ml = 0usize;
            while ml < max_match && text_buf[pos + ml] == text_buf[r + ml] {
                ml += 1;
            }
            let better = if allow_equal {
                ml >= best_len && ml > 0
            } else {
                ml > best_len
            };
            if better {
                best_len = ml;
                best_pos = pos;
                if best_len >= F {
                    break;
                }
            }
        }

        let last_match_length = if best_len <= THRESHOLD {
            out.push(Token::Literal(text_buf[r]));
            1
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            best_len
        };

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            let c = input[input_idx];
            input_idx += 1;
            text_buf[s] = c;
            if s < F - 1 {
                text_buf[s + N] = c;
            }
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            i += 1;
        }
        while i < last_match_length {
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            i += 1;
            if len == 0 {
                break;
            }
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// 厳格 no-init-region matching エンコーダ。
///
/// 仮説: cc=48 (キャラスプライト) ファイルの encoder は **初期 0x20 fill 領域**
/// (= 書込み未済 ring 位置) を一切マッチ候補にしない厳格規則を持つ。
/// 既存 no_dummy は dummy 挿入を避けるが、ring の中身が偶然 0x20 と一致する
/// 場合のマッチは抑制しきれない。本実装は per-position written bitmap で
/// 「真に書込み済みの位置」のみマッチを許可する。
///
/// `mode` で best/leftmost/min_dist tie-break を選ぶ。
#[derive(Clone, Copy)]
pub enum NoInitMode {
    /// best length, leftmost pos tie-break
    BestLeftmost,
    /// best length, smallest dist tie-break
    BestMinDist,
    /// best length, largest dist tie-break (= 標準奥村寄り)
    BestMaxDist,
}

pub fn compress_no_init_match(input: &[u8], mode: NoInitMode) -> Vec<Token> {
    let mut text_buf = [0x20u8; N + F - 1];
    let mut written = [false; N];
    let mut out: Vec<Token> = Vec::new();

    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        text_buf[r + len] = input[input_idx];
        written[r + len] = true;
        input_idx += 1;
        len += 1;
    }
    if len == 0 {
        return out;
    }

    loop {
        let max_match = len.min(F);
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;
        let mut best_dist: usize = 0;

        for pos in 0..N {
            if pos == r || !written[pos] {
                continue;
            }
            // すべての pos+0..pos+max_match-1 が書込み済みでなければ拒否
            // (ring の writeback で自己参照は許される: dist < ml の場合は r..r+ml の
            // 中で既に書かれた所から読む = 書込み済み)
            // 簡単のため: 開始位置 pos のみ書込み済みチェック。chain extension は
            // ring 内容で評価。
            let mut ml = 0usize;
            while ml < max_match && text_buf[pos + ml] == text_buf[r + ml] {
                ml += 1;
            }
            if ml < 3 {
                continue;
            }
            let dist = (r + N - pos) & (N - 1);
            let take = match mode {
                NoInitMode::BestLeftmost => {
                    ml > best_len || (ml == best_len && pos < best_pos)
                }
                NoInitMode::BestMinDist => {
                    ml > best_len || (ml == best_len && dist > 0 && (best_dist == 0 || dist < best_dist))
                }
                NoInitMode::BestMaxDist => {
                    ml > best_len || (ml == best_len && dist > best_dist)
                }
            };
            if take {
                best_len = ml;
                best_pos = pos;
                best_dist = dist;
            }
        }

        let last_match_length = if best_len <= THRESHOLD {
            out.push(Token::Literal(text_buf[r]));
            1
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            best_len
        };

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            let c = input[input_idx];
            input_idx += 1;
            text_buf[s] = c;
            if s < F - 1 {
                text_buf[s + N] = c;
            }
            written[s] = true;
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            i += 1;
        }
        while i < last_match_length {
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            i += 1;
            if len == 0 {
                break;
            }
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// hash-chain LZSS エンコーダ (zlib/gzip スタイル)。
///
/// 3 バイトプレフィックスで hash、chain は LIFO (新しい順)、各 step で
/// chain を走査して `mode` に応じて選ぶ:
///   - `HashMode::FirstMatch`: 最初に len>=3 が見つかった pos を採用
///   - `HashMode::BestMatch`: chain 全体を走査して max-len を採用
///
/// この変種は奥村 BST とも naive scan とも異なる挿入順序を持つ。
/// 仮説: To Heart encoder が hash-chain ベースなら、特定 mode で binary 一致する。
#[derive(Clone, Copy)]
pub enum HashMode {
    FirstMatch,
    BestMatch,
}

pub fn compress_hash_chain(input: &[u8], mode: HashMode) -> Vec<Token> {
    let mut text_buf = [0x20u8; N + F - 1];
    let mut out: Vec<Token> = Vec::new();

    // 12-bit hash (4096 buckets), prev[] tracks chain
    const HASH_SIZE: usize = 1 << 12;
    let hash_mask = (HASH_SIZE - 1) as u32;
    let hash3 = |a: u8, b: u8, c: u8| -> usize {
        let h = ((a as u32) << 8) ^ ((b as u32) << 4) ^ (c as u32);
        ((h.wrapping_mul(2654435761) >> 12) & hash_mask) as usize
    };

    let mut head = vec![usize::MAX; HASH_SIZE];
    let mut prev = vec![usize::MAX; N];

    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        text_buf[r + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }
    if len == 0 {
        return out;
    }

    // initial insert at r (need at least 3 bytes to hash)
    if len >= 3 {
        let h = hash3(text_buf[r], text_buf[r + 1], text_buf[r + 2]);
        prev[r] = head[h];
        head[h] = r;
    }

    loop {
        let max_match = len.min(F);
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        if max_match >= 3 {
            let h = hash3(text_buf[r], text_buf[r + 1], text_buf[r + 2]);
            let mut p = head[h];
            // chain walk from newest to oldest
            let mut walked = 0;
            while p != usize::MAX && walked < N {
                if p != r {
                    let mut ml = 0usize;
                    while ml < max_match && text_buf[p + ml] == text_buf[r + ml] {
                        ml += 1;
                    }
                    if ml >= 3 {
                        match mode {
                            HashMode::FirstMatch => {
                                best_len = ml;
                                best_pos = p;
                                break;
                            }
                            HashMode::BestMatch => {
                                if ml > best_len {
                                    best_len = ml;
                                    best_pos = p;
                                    if best_len >= F {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                let nx = prev[p];
                // distance check: avoid infinite loop on bad chains
                p = nx;
                walked += 1;
            }
        }

        let last_match_length = if best_len <= THRESHOLD {
            out.push(Token::Literal(text_buf[r]));
            1
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            best_len
        };

        // advance & insert chain entries
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            let c = input[input_idx];
            input_idx += 1;
            text_buf[s] = c;
            if s < F - 1 {
                text_buf[s + N] = c;
            }
            // next position to process is the new r after advance.
            // but we need to insert hash entry for the "previous" r
            // (so it's findable when we move past it).
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            // insert hash for new r if we have 3 bytes
            if len >= 3 {
                let h = hash3(text_buf[r], text_buf[r + 1], text_buf[r + 2]);
                prev[r] = head[h];
                head[r] = r;  // overwrite
                head[h] = r;
            }
            i += 1;
        }
        while i < last_match_length {
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            i += 1;
            if len == 0 {
                break;
            }
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// pos = 0..N 絶対位置昇順で len >= 3 が初めて見つかったら、その pos で
/// **可能な限り extension** して採用 (greedy first-match-then-extend)。
/// 既存の "best" 探索とは異なり、"first" 採用。
pub fn compress_naive_first_match(input: &[u8]) -> Vec<Token> {
    let mut text_buf = [0x20u8; N + F - 1];
    let mut out: Vec<Token> = Vec::new();

    let mut r: usize = N - F;
    let mut s: usize = 0;

    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        text_buf[r + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }
    if len == 0 {
        return out;
    }

    loop {
        let max_match = len.min(F);
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        for pos in 0..N {
            if pos == r {
                continue;
            }
            let mut ml = 0usize;
            while ml < max_match && text_buf[pos + ml] == text_buf[r + ml] {
                ml += 1;
            }
            if ml > THRESHOLD {
                // first-match: ここで採用、最長 extension 完了
                best_len = ml;
                best_pos = pos;
                break;
            }
        }

        let last_match_length = if best_len <= THRESHOLD {
            out.push(Token::Literal(text_buf[r]));
            1
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            best_len
        };

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            let c = input[input_idx];
            input_idx += 1;
            text_buf[s] = c;
            if s < F - 1 {
                text_buf[s + N] = c;
            }
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            i += 1;
        }
        while i < last_match_length {
            s = (s + 1) & (N - 1);
            r = (r + 1) & (N - 1);
            len -= 1;
            i += 1;
            if len == 0 {
                break;
            }
        }
        if len == 0 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_tokens(tokens: &[Token]) -> Vec<u8> {
        let mut ring = [0x20u8; N];
        let mut r: usize = N - F;
        let mut out = Vec::new();
        for tok in tokens {
            match *tok {
                Token::Literal(b) => {
                    out.push(b);
                    ring[r] = b;
                    r = (r + 1) & (N - 1);
                }
                Token::Match { pos, len } => {
                    let pos = pos as usize;
                    for k in 0..len as usize {
                        let b = ring[(pos + k) & (N - 1)];
                        out.push(b);
                        ring[r] = b;
                        r = (r + 1) & (N - 1);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn empty_input() {
        assert!(compress_naive_backward(&[], false).is_empty());
        assert!(compress_naive_backward(&[], true).is_empty());
    }

    #[test]
    fn short_literal() {
        let toks = compress_naive_backward(b"ab", false);
        assert_eq!(toks.len(), 2);
        assert!(matches!(toks[0], Token::Literal(b'a')));
        assert!(matches!(toks[1], Token::Literal(b'b')));
    }

    #[test]
    fn roundtrip_repeat() {
        let input: Vec<u8> = (0..200u32).map(|i| b'A' + (i % 26) as u8).collect();
        for &eq in &[false, true] {
            let toks = compress_naive_backward(&input, eq);
            let decoded = decode_tokens(&toks);
            assert_eq!(decoded, input, "roundtrip failed for allow_equal={}", eq);
        }
    }

    #[test]
    fn roundtrip_spaces() {
        let input = vec![b' '; 50];
        let toks = compress_naive_backward(&input, false);
        let decoded = decode_tokens(&toks);
        assert_eq!(decoded, input);
    }
}
