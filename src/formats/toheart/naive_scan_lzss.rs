//! Naive backward linear-scan LZSS encoder variant (hypothesis test for Leaf LF2).

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
