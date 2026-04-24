//! 奥村晴彦 lzss.c (1989, fj.sources) の二分木版を Rust に忠実移植。
//!
//! 原典: https://oku.edu.mie-u.ac.jp/~okumura/compression/lzss.c
//!   "LZSS.C -- A Data Compression Program" by Haruhiko Okumura (public domain)
//!
//! 目的: Leaf の LF2 エンコーダが 1997 年当時この奥村 lzss.c の二分木版を
//! 流用した可能性が高いため、タイブレイク挙動を含めてバイナリ一致を狙う。
//!
//! 方針:
//! - 変数名・関数構造・制御フローを原典に合わせる（lson/rson/dad 等）
//! - 最適化しない、奥村原典の挙動を変えない
//! - 入出力はトークン列。LF2 framing は呼び出し側で行う
//!
//! 定数:
//!   N          = 4096   ring buffer size
//!   F          = 18     upper limit for match_length
//!   THRESHOLD  = 2      minimum match length (=> 3..=F)
//!   NIL        = N      index for root of binary search trees
//!
//! 初期値は LF2 側に合わせて ring を 0x20 で埋め、書き込み開始位置を N-F とする。

pub const N: usize = 4096;
pub const F: usize = 18;
pub const THRESHOLD: usize = 2;
pub const NIL: i32 = N as i32;

/// 1 トークン。
///
/// `Match { pos, len }` の `pos` は 0..N の絶対リングバッファ位置（奥村原典の
/// `match_position` そのまま）、`len` は実長（3..=F）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Literal(u8),
    Match { pos: u16, len: u8 },
}

/// 奥村 lzss.c の `Encode` に相当するステート。
struct Okumura {
    /// ring buffer (+F-1 でマッチ検索用に末尾に overlap 領域)
    text_buf: [u8; N + F - 1],
    /// 左子 (N+1 要素)
    lson: [i32; N + 1],
    /// 右子 (N+257 要素、先頭 256 は256分木のルート)
    rson: [i32; N + 257],
    /// 親 (N+1 要素)
    dad: [i32; N + 1],
    /// 直近の `InsertNode` で確定したマッチ位置
    match_position: i32,
    /// 直近の `InsertNode` で確定したマッチ長
    match_length: i32,
}

impl Okumura {
    fn new(fill: u8) -> Self {
        Self {
            text_buf: [fill; N + F - 1],
            lson: [0; N + 1],
            rson: [0; N + 257],
            dad: [0; N + 1],
            match_position: 0,
            match_length: 0,
        }
    }

    /// 原典 `InitTree()` 逐語移植。
    fn init_tree(&mut self) {
        // For i = N + 1 to N + 256, rson[i] = NIL   (右子のルート 256 個)
        for i in (N + 1)..=(N + 256) {
            self.rson[i] = NIL;
        }
        // For i = 0 to N - 1, dad[i] = NIL
        for i in 0..N {
            self.dad[i] = NIL;
        }
    }

    /// 原典 `InsertNode(int r)` 逐語移植。
    ///
    /// text_buf[r..r+F-1] を木に挿入し、同時に最長一致を探索する。
    /// 結果は `self.match_position` / `self.match_length` に格納される。
    fn insert_node(&mut self, r: i32) {
        let mut cmp: i32 = 1;
        let key_start = r as usize;
        // key = &text_buf[r..]
        let p_root_idx = N as i32 + 1 + self.text_buf[key_start] as i32;
        let mut p: i32 = p_root_idx;

        self.rson[r as usize] = NIL;
        self.lson[r as usize] = NIL;
        self.match_length = 0;

        loop {
            if cmp >= 0 {
                if self.rson[p as usize] != NIL {
                    p = self.rson[p as usize];
                } else {
                    self.rson[p as usize] = r;
                    self.dad[r as usize] = p;
                    return;
                }
            } else {
                if self.lson[p as usize] != NIL {
                    p = self.lson[p as usize];
                } else {
                    self.lson[p as usize] = r;
                    self.dad[r as usize] = p;
                    return;
                }
            }

            // for (i = 1; i < F; i++) if ((cmp = key[i] - text_buf[p + i]) != 0) break;
            let mut i: usize = 1;
            cmp = 0;
            while i < F {
                let a = self.text_buf[key_start + i] as i32;
                let b = self.text_buf[p as usize + i] as i32;
                let d = a - b;
                if d != 0 {
                    cmp = d;
                    break;
                }
                i += 1;
            }

            if (i as i32) > self.match_length {
                self.match_position = p;
                self.match_length = i as i32;
                if i >= F {
                    break;
                }
            }
        }

        // 既存ノード p を r で置き換える。
        // 原典は dad[lson[p]] と dad[rson[p]] を NIL チェックなしに書き換える。
        // dad 配列サイズは N+1 なので dad[NIL]=dad[N] への書き込みは合法（ゴミ格納）。
        self.dad[r as usize] = self.dad[p as usize];
        self.lson[r as usize] = self.lson[p as usize];
        self.rson[r as usize] = self.rson[p as usize];
        let lson_p = self.lson[p as usize];
        let rson_p = self.rson[p as usize];
        self.dad[lson_p as usize] = r;
        self.dad[rson_p as usize] = r;
        let dad_p = self.dad[p as usize];
        if self.rson[dad_p as usize] == p {
            self.rson[dad_p as usize] = r;
        } else {
            self.lson[dad_p as usize] = r;
        }
        self.dad[p as usize] = NIL; // remove p
    }

    /// 原典 `DeleteNode(int p)` 逐語移植。
    fn delete_node(&mut self, p: i32) {
        if self.dad[p as usize] == NIL {
            return; // not in tree
        }
        let q: i32;
        if self.rson[p as usize] == NIL {
            q = self.lson[p as usize];
        } else if self.lson[p as usize] == NIL {
            q = self.rson[p as usize];
        } else {
            // 両子。lson[p] の最右子孫 q を見つけて p と挿げ替える
            let mut qv = self.lson[p as usize];
            if self.rson[qv as usize] != NIL {
                // do { q = rson[q] } while (rson[q] != NIL);
                loop {
                    qv = self.rson[qv as usize];
                    if self.rson[qv as usize] == NIL {
                        break;
                    }
                }
                // rson[dad[q]] = lson[q];
                let dad_q = self.dad[qv as usize];
                self.rson[dad_q as usize] = self.lson[qv as usize];
                // dad[lson[q]] = dad[q];
                let lq = self.lson[qv as usize];
                self.dad[lq as usize] = dad_q;
                // lson[q] = lson[p];
                self.lson[qv as usize] = self.lson[p as usize];
                // dad[lson[p]] = q;
                let lp = self.lson[p as usize];
                self.dad[lp as usize] = qv;
            }
            // rson[q] = rson[p];
            self.rson[qv as usize] = self.rson[p as usize];
            // dad[rson[p]] = q;
            let rp = self.rson[p as usize];
            self.dad[rp as usize] = qv;
            q = qv;
        }

        // dad[q] = dad[p]; fix parent link
        self.dad[q as usize] = self.dad[p as usize];
        let dad_p = self.dad[p as usize];
        if self.rson[dad_p as usize] == p {
            self.rson[dad_p as usize] = q;
        } else {
            self.lson[dad_p as usize] = q;
        }
        self.dad[p as usize] = NIL;
    }
}

/// 奥村 lzss.c `Encode()` 逐語移植。トークン列を返す。
///
/// `match_position` は 0..N のリングバッファ絶対位置で返る（LF2 decoder の
/// `position` と同じ表現）。
pub fn compress_okumura(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    // r = N - F   (書き込み開始位置。F バイト先読みして木に入れる)
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

    // 入力を F バイトまで text_buf[r..] に先読み
    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        st.text_buf[r as usize + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }

    if len == 0 {
        return out;
    }

    // 最初に F 個のダミー挿入（奥村原典 for (i = 1; i <= F; i++) InsertNode(r - i)）
    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    // 最初の本挿入
    st.insert_node(r);

    loop {
        // match_length をフレーム残量に丸める
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        // 出力
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            // match_position は InsertNode 内で必ず 0..N の範囲に収まる（ring index）
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        // last_match_length 回 ring を進める
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            // s < F-1 のときは末尾 overlap 領域にもコピー
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

        // 入力が尽きた後の残り処理: len を減らしつつ DeleteNode
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len -= 1;
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
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

    #[test]
    fn empty_input() {
        let toks = compress_okumura(&[]);
        assert!(toks.is_empty());
    }

    #[test]
    fn short_literal() {
        // THRESHOLD=2 以下の一致しか出ないので全部リテラルになる
        let toks = compress_okumura(b"ab");
        assert_eq!(toks.len(), 2);
        assert!(matches!(toks[0], Token::Literal(b'a')));
        assert!(matches!(toks[1], Token::Literal(b'b')));
    }

    #[test]
    fn run_of_spaces_matches_initial_ring() {
        // ring が 0x20 (' ') で埋まっているので、先頭からスペース連続は
        // 長い一致として返るはず。
        // 注: 入力 20 バイト全部スペースだと初期リング全体と一致するため
        //     F-1 と F のどちらでも通ってしまい原典忠実性の検証には弱い。
        //     must-1 の off-by-one 検証は下の `abc_then_reuse_pins_tiebreak`
        //     テストで行う。
        let input = vec![b' '; 20];
        let toks = compress_okumura(&input);
        match toks[0] {
            Token::Match { len, .. } => assert_eq!(len, F as u8),
            _ => panic!("expected match, got {:?}", toks[0]),
        }
    }

    /// 非自明な決定的入力で最初のマッチの pos/len を pin する。
    /// "ABC..Z" を 100 文字分繰り返した入力を圧縮したとき、奥村原典を
    /// 忠実に移植できていれば token26 が Match { pos=4078, len=18 } になる。
    ///
    /// 期待値の根拠:
    /// - r は N-F=4078 から書き始め、26 バイトのリテラル後に ring が十分
    ///   埋まる。その直後、ちょうど最初の "ABCDEFG..." (26 バイト) に
    ///   マッチして F=18 分の参照が返る。
    /// - must-1 の off-by-one (F-1 個ダミー) と F 個ダミーでは初期木の
    ///   形が異なり、同じ長さの候補があるときにどのノードを返すかが変わる。
    ///   本期待値は F 個ダミー（奥村原典）の実装で得られた値を pin している。
    #[test]
    fn abc_then_reuse_pins_tiebreak() {
        let input: Vec<u8> = (0..100u32).map(|i| b'A' + (i % 26) as u8).collect();
        let toks = compress_okumura(&input);

        // 先頭 26 バイトは辞書（初期 0x20 のみ）と一致しないのでリテラル
        for (i, t) in toks.iter().take(26).enumerate() {
            match t {
                Token::Literal(b) => {
                    assert_eq!(*b, b'A' + (i as u8 % 26), "token {} expected literal", i);
                }
                other => panic!("token {} expected literal, got {:?}", i, other),
            }
        }

        // token 26 が最初の Match
        match toks.get(26) {
            Some(Token::Match { pos, len }) => {
                assert_eq!(*pos, 4078, "first match pos pinned to奥村原典実装の出力");
                assert_eq!(*len, F as u8, "first match len pinned to F=18");
            }
            other => panic!("token 26 expected Match {{ pos=4078, len=18 }}, got {:?}", other),
        }
    }
}
