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

/// タイブレイク挙動を指定する。
///
/// - `StrictGt`: 奥村原典 `>`。同一長候補は最初に見つかった (BST 訪問順) を採用
/// - `AllowEq`:  `>=`。同一長候補は最後に訪れたノードで上書き
/// - `DistanceTie`: `>` だが、同一長のときに ring write head `r` への距離が
///                  近いほうを採用（Leaf 系エンコーダの観測されたバイアス）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieMode {
    StrictGt,
    AllowEq,
    DistanceTie,
    /// 短マッチ (len ≤ 3) は AllowEq、それ以外は StrictGt。
    /// セッション 295 の U 字分布発見 (max_len=3 → rank=末尾 60.5%, max_len=18 → rank=先頭 87.3%) に対応する仮説。
    DynamicShortEq,
}

/// BST 探索・挿入の構造的バリアント。
///
/// セッション 296-297 で「奥村 LZSS の dummy/THRESHOLD/tie/サイズ判定」軸を 16 変種
/// 試して 224/522 で天井に達した。これらは **BST の構造そのものを触らない**変種。
/// セッション 298 でこの軸（insert_node の探索順 + swap-with-r の有無）に踏み込む。
///
/// - `Standard`: 奥村原典。`cmp = 1` 初期 → 最初は右、tie 評価で `cmp >= 0` 右
/// - `LeftFirst`: `cmp = -1` 初期 → 最初は左、`cmp > 0` のみ右、tie で左 (奥村の左右反転)
/// - `NoSwap`: F バイト完全一致時に swap-with-r ブロックをスキップ。新ノード r は BST に
///   入らず（孤立、dad[r] = NIL）、既存ノード p がそのまま残る。古いマッチを優先する挙動。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BstMode {
    Standard,
    LeftFirst,
    NoSwap,
}

/// 奥村エンコード途中のステートを丸ごと持ち出すためのスナップショット。
/// `lf2_first_div_inspect` バイナリ専用のデバッグ用構造体。
pub struct OkumuraSnapshot {
    pub tokens: Vec<Token>,
    pub text_buf: Box<[u8; N + F - 1]>,
    pub lson: Box<[i32; N + 257]>,
    pub rson: Box<[i32; N + 257]>,
    pub dad: Box<[i32; N + 1]>,
    pub r: i32,
    pub s: i32,
    pub len: usize,
    pub input_idx: usize,
    /// この時点で次に出されるはずだった token のため insert_node を 1 回回した直後の
    /// match_position / match_length（つまり stop_at_token 番目の token を出す直前の状態）
    pub next_match_position: i32,
    pub next_match_length: i32,
}

/// BST のうち dad != NIL のノードを並べた一覧を整形する。
/// pos, dad, lson, rson, ring 上の (cur_r - pos) & (N-1) 距離, 先頭3バイトを出す。
pub fn format_bst_dump(snap: &OkumuraSnapshot, max_nodes: usize) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    let mask = (N as i32) - 1;
    let cur_r = snap.r;
    let mut count = 0usize;
    let _ = writeln!(
        s,
        "BST nodes (dad != NIL), cur_r = 0x{:03x}, max {} shown:",
        cur_r as u16, max_nodes
    );
    let _ = writeln!(
        s,
        "    pos    dad    lson   rson   dist   bytes[0..3]"
    );
    for pos in 0..N {
        if snap.dad[pos] == NIL {
            continue;
        }
        if count >= max_nodes {
            let _ = writeln!(s, "    ... (truncated)");
            break;
        }
        let dist = (cur_r - pos as i32) & mask;
        let b0 = snap.text_buf[pos];
        let b1 = snap.text_buf[pos + 1];
        let b2 = snap.text_buf[pos + 2];
        let _ = writeln!(
            s,
            "    0x{:03x}  0x{:04x} 0x{:04x} 0x{:04x} 0x{:03x}  {:02x} {:02x} {:02x}",
            pos as u16,
            snap.dad[pos],
            snap.lson[pos],
            snap.rson[pos],
            dist as u16,
            b0,
            b1,
            b2
        );
        count += 1;
    }
    let _ = writeln!(s, "    ({} nodes total in tree)", count);
    s
}

/// 奥村エンコードを stop_at_token 番目の token を**出す直前**で停止させ、
/// その時点のステートを返す。
///
/// stop_at_token=0 は「先読み + 初期 InsertNode を済ませただけで、
/// まだ何も output していない」状態。
pub fn compress_okumura_inspect(input: &[u8], stop_at_token: usize) -> OkumuraSnapshot {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        st.text_buf[r as usize + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }

    if len == 0 {
        return OkumuraSnapshot {
            tokens: out,
            text_buf: Box::new(st.text_buf),
            lson: Box::new(st.lson),
            rson: Box::new(st.rson),
            dad: Box::new(st.dad),
            r,
            s,
            len,
            input_idx,
            next_match_position: 0,
            next_match_length: 0,
        };
    }

    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        // ここで out.len() == stop_at_token なら、まさにこの token を出す直前。
        if out.len() >= stop_at_token {
            return OkumuraSnapshot {
                tokens: out,
                text_buf: Box::new(st.text_buf),
                lson: Box::new(st.lson),
                rson: Box::new(st.rson),
                dad: Box::new(st.dad),
                r,
                s,
                len,
                input_idx,
                next_match_position: st.match_position,
                next_match_length: st.match_length,
            };
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

    OkumuraSnapshot {
        tokens: out,
        text_buf: Box::new(st.text_buf),
        lson: Box::new(st.lson),
        rson: Box::new(st.rson),
        dad: Box::new(st.dad),
        r,
        s,
        len,
        input_idx,
        next_match_position: st.match_position,
        next_match_length: st.match_length,
    }
}

/// 奥村 lzss.c の `Encode` に相当するステート。
struct Okumura {
    /// ring buffer (+F-1 でマッチ検索用に末尾に overlap 領域)
    text_buf: [u8; N + F - 1],
    /// 左子 (奥村原典は N+1 で十分だが、`BstMode::LeftFirst` で root pseudo-node の
    /// 左探索を許すために N+257 に拡張。Standard モードでは余分な末尾領域は未使用)
    lson: [i32; N + 257],
    /// 右子 (N+257 要素、先頭 256 は256分木のルート)
    rson: [i32; N + 257],
    /// 親 (N+1 要素)
    dad: [i32; N + 1],
    /// 直近の `InsertNode` で確定したマッチ位置
    match_position: i32,
    /// 直近の `InsertNode` で確定したマッチ長
    match_length: i32,
    /// タイブレイク挙動
    tie_mode: TieMode,
    /// `InsertNode` 内で参照する現在の ring write head `r`。
    /// `DistanceTie` モードのときに距離計算に使う。
    cur_r: i32,
    /// BST 構造バリアント
    bst_mode: BstMode,
}

impl Okumura {
    fn new(fill: u8) -> Self {
        Self {
            text_buf: [fill; N + F - 1],
            lson: [0; N + 257],
            rson: [0; N + 257],
            dad: [0; N + 1],
            match_position: 0,
            match_length: 0,
            tie_mode: TieMode::StrictGt,
            cur_r: 0,
            bst_mode: BstMode::Standard,
        }
    }

    /// 原典 `InitTree()` 逐語移植。
    fn init_tree(&mut self) {
        // For i = N + 1 to N + 256, rson[i] = NIL   (右子のルート 256 個)
        for i in (N + 1)..=(N + 256) {
            self.rson[i] = NIL;
        }
        // BstMode::LeftFirst: root pseudo-node の lson も探索対象になるので NIL 初期化
        if matches!(self.bst_mode, BstMode::LeftFirst) {
            for i in (N + 1)..=(N + 256) {
                self.lson[i] = NIL;
            }
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
        // BstMode::LeftFirst: cmp = -1 初期 + cmp > 0 のみ右へ (奥村の左右反転)
        let mut cmp: i32 = match self.bst_mode {
            BstMode::LeftFirst => -1,
            _ => 1,
        };
        let key_start = r as usize;
        // key = &text_buf[r..]
        let p_root_idx = N as i32 + 1 + self.text_buf[key_start] as i32;
        let mut p: i32 = p_root_idx;

        self.rson[r as usize] = NIL;
        self.lson[r as usize] = NIL;
        self.match_length = 0;
        self.cur_r = r;

        loop {
            let go_right = match self.bst_mode {
                BstMode::LeftFirst => cmp > 0,
                _ => cmp >= 0,
            };
            if go_right {
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

            let take = match self.tie_mode {
                TieMode::StrictGt => (i as i32) > self.match_length,
                TieMode::AllowEq => (i as i32) >= self.match_length,
                TieMode::DistanceTie => {
                    if (i as i32) > self.match_length {
                        true
                    } else if (i as i32) == self.match_length {
                        let mask = N as i32 - 1;
                        let cur_dist = (self.cur_r - p) & mask;
                        let best_dist = (self.cur_r - self.match_position) & mask;
                        cur_dist > 0 && (best_dist == 0 || cur_dist < best_dist)
                    } else {
                        false
                    }
                }
                TieMode::DynamicShortEq => {
                    if (i as i32) > self.match_length {
                        true
                    } else if (i as i32) == self.match_length && (i as i32) <= 3 {
                        true
                    } else {
                        false
                    }
                }
            };
            if take {
                self.match_position = p;
                self.match_length = i as i32;
                if i >= F {
                    break;
                }
            }
        }

        // BstMode::NoSwap: F バイト完全一致した既存ノード p をそのまま残し、
        // 新ノード r は BST に入れない（孤立、dad[r] = NIL）。
        // delete_node(r) は dad[r] == NIL なら early return するので safe。
        if matches!(self.bst_mode, BstMode::NoSwap) {
            self.dad[r as usize] = NIL;
            // lson[r] / rson[r] は loop 開始時に NIL 済み
            return;
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
    compress_okumura_impl(input, TieMode::StrictGt)
}

/// タイブレイク挙動をパラメータ化した版。
///
/// `allow_equal=false` は奥村原典 (`>`)。`true` のとき、同一長候補が見つかったら
/// BST パス上で**最後に**訪れたノードを `match_position` にする (`>=`)。
pub fn compress_okumura_with_tie(input: &[u8], allow_equal: bool) -> Vec<Token> {
    compress_okumura_impl(
        input,
        if allow_equal { TieMode::AllowEq } else { TieMode::StrictGt },
    )
}

/// 距離タイブレイク版。同一長のとき `r` に近い (back distance が小さい) 候補を採用。
pub fn compress_okumura_distance_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl(input, TieMode::DistanceTie)
}

/// dummy_rev variant
pub fn compress_okumura_dummy_rev(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    for i in (1..=F).rev() {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

/// 奥村原典の "lazy matching" 拡張版。
///
/// 各ステップで `insert_node(r)` 直後の最長一致 `(pos1, len1)` を保存し、
/// もし `len1 > THRESHOLD`（=Match を出すつもり）なら 1 バイトだけ先に
/// ring を進めて `insert_node(r+1)` を実行し、新しい最長一致 `len2` を見る。
/// `len2 > len1` なら、`r` のマッチを捨てて Literal(text_buf[r]) を出し、
/// `r+1` のマッチをそのまま次の反復に持ち越す（既に 1 バイト進んでいるので
/// 自然に正しい位置にいる）。
///
/// `len2 <= len1` なら元のマッチを採用する。既に 1 バイト進めているので、
/// あと `len1 - 1` バイト進めて元のマッチを消費する。
///
/// `compress_okumura` の greedy 版とは独立した関数として動作する。
pub fn compress_okumura_lazy(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    // 直前の insert_node(r) の結果を (pos1, len1) として保持する。
    // 反復の頭で「現在 r の match 情報は (st.match_position, st.match_length)」と
    // いう不変条件が成り立っていることに注意。

    loop {
        // フレーム残量 len で丸める
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        let pos1 = st.match_position;
        let len1 = st.match_length as usize;

        // Match を出す予定なら 1-step lazy lookahead
        // ただし：
        //  - len1 == len (フレーム末尾まで届いている) なら lazy しても得しない
        //  - 入力が尽きていて peek できない可能性も考慮
        let mut take_lazy = false;
        if len1 > THRESHOLD && len1 < len {
            // 1 バイト進めて peek。これは元の loop の advance 1-step と同じ操作。
            let saved_byte_at_r = st.text_buf[r as usize];

            // advance step
            let advanced;
            if input_idx < input.len() {
                st.delete_node(s);
                let c = input[input_idx];
                input_idx += 1;
                st.text_buf[s as usize] = c;
                if (s as usize) < F - 1 {
                    st.text_buf[s as usize + N] = c;
                }
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                st.insert_node(r);
                advanced = true;
            } else {
                // 入力枯渇。元の loop の「len を減らす」分岐と同じ。
                st.delete_node(s);
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                len -= 1;
                if len > 0 {
                    st.insert_node(r);
                } else {
                    // len2 を計算する材料がないので fall back
                    st.match_length = 0;
                }
                advanced = true;
            }

            let _ = advanced;

            // 残量で丸めて len2 を確定
            if st.match_length as usize > len {
                st.match_length = len as i32;
            }
            let len2 = st.match_length as usize;

            if len2 > len1 {
                // lazy 採用：元のマッチを捨てて Literal(saved_byte_at_r) を出す。
                // ring は既に 1 バイト進んだ状態で、そこの match 情報 (st.match_position,
                // st.match_length) = (pos2, len2) も計算済み。次の反復にそのまま渡る。
                out.push(Token::Literal(saved_byte_at_r));
                take_lazy = true;
            } else {
                // lazy 不採用：元のマッチ (pos1, len1) を出力し、残り len1-1 バイトを進める。
                out.push(Token::Match {
                    pos: (pos1 as u16) & ((N as u16) - 1),
                    len: len1 as u8,
                });

                // すでに 1 バイト advance 済み。あと last_match_length-1 進める。
                let last_match_length = len1;
                let mut i = 1usize;
                while i < last_match_length && input_idx < input.len() {
                    st.delete_node(s);
                    let c = input[input_idx];
                    input_idx += 1;
                    st.text_buf[s as usize] = c;
                    if (s as usize) < F - 1 {
                        st.text_buf[s as usize + N] = c;
                    }
                    s = (s + 1) & (N as i32 - 1);
                    r = (r + 1) & (N as i32 - 1);
                    st.insert_node(r);
                    i += 1;
                }
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
                // 不変条件を再確立: 反復先頭の (st.match_position, st.match_length) が r の情報。
                // 上の advance loop の最後の insert_node(r) で既にそうなっている。
                continue;
            }
        }

        if take_lazy {
            // lazy 経路で 1 バイト進めた状態。len チェックして次の反復へ。
            if len == 0 {
                break;
            }
            continue;
        }

        // 通常の greedy 出力経路 (len1 <= THRESHOLD あるいは len1 == len)
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
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

/// 奥村原典と同じだが、`for i in 1..=F { insert_node(r - i) }` のダミー挿入を
/// **行わない**版。`insert_node(r)` のみ最初に行う。
///
/// 仮説: Leaf の LF2 エンコーダはこの F 個のダミーノードを挿入していないため、
/// 序盤の出力が（奥村原典より）リテラル寄りになる。
pub fn compress_okumura_no_dummy(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    // ダミー挿入なし。最初の本挿入のみ。
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

/// no_dummy 変種に「pos = r - F に dummy を 1 個だけ挿入」を加えた版。
///
/// 仮説: Leaf は奥村のような F 個ダミー挿入はしないが、token 0 で
/// `Match{pos=0xFDC=N-2F, len=18}` を出しているファイルが存在する。
/// `insert_node(r - F)` だけ先に行えば、text_buf 全 0x20 初期状態で
/// `pos=r-F, len=18` のマッチが BST から取れる。
pub fn compress_okumura_one_dummy_at_rf(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    // r - F の位置に dummy を 1 個だけ挿入（mod N で正規化）。
    let dummy_pos = ((r - F as i32) + N as i32) & (N as i32 - 1);
    st.insert_node(dummy_pos);
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

/// 奥村原典どおり F-dummy を最初に挿入するが、token 0 を出力した直後に
/// dummy として挿入したノード群（r-1, r-2, ..., r-F の旧位置）を全削除し、
/// それ以降は no_dummy 等価で進行する変種。
///
/// 仮説: 奥村が当てる 171 + no_dummy が当てる 215 のいいとこ取り。
/// dummy が token 0 の `Match{len=18}` を生み、その後カスケードしないので
/// 中盤以降は no_dummy と同じ挙動になる。
pub fn compress_okumura_dummy_then_drop(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    // 奥村原典どおり F dummy 挿入（r-1 .. r-F）。挿入位置を記録しておく。
    let mut dummy_positions: Vec<i32> = Vec::with_capacity(F);
    for i in 1..=F {
        let p = ((r - i as i32) + N as i32) & (N as i32 - 1);
        st.insert_node(p);
        dummy_positions.push(p);
    }
    st.insert_node(r);

    let mut first_token_done = false;

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

        // token 0 の処理が終わったら、dummy として挿入した位置を全削除。
        // ただし通常進行で既に s が回って delete されている範囲は除く。
        if !first_token_done {
            first_token_done = true;
            // s が回ってない範囲の dummy を削除。
            // 通常進行で delete された slot は old_s..old_s + last_match_length。
            // dummy は r-1, r-2, ..., r-F の F 個（r=N-F の場合 N-F-1..N-2F）。
            // 簡便のため、まだ生きてる dummy を「再挿入なしの delete」で除去する。
            // delete_node は不在ノードに対して no-op に近い設計のため
            // 多少の重複削除は安全（奥村の delete_node 実装を確認してから運用）。
            for p in dummy_positions.iter().copied() {
                // r 自身（本挿入）は削除しない。
                if p == r {
                    continue;
                }
                st.delete_node(p);
            }
        }

        if len == 0 {
            break;
        }
    }

    out
}

/// 入力先頭 F バイトが全て同値ならば奥村原典 (dummy あり)、そうでなければ no_dummy。
///
/// セッション 297 観察: 「奥村だけ当たる 9 ファイル」のうち 5 ファイル
/// (C0601, CLNO_07, H42, V80, V91) は先頭 F=18 バイトが全て同値で、これらは
/// dummy が直接 long match (len=F) として活きる単色画像。
pub fn compress_okumura_uniform_head(input: &[u8]) -> Vec<Token> {
    if input.len() >= F && input[..F].iter().all(|&b| b == input[0]) {
        compress_okumura(input)
    } else {
        compress_okumura_no_dummy(input)
    }
}

/// 奥村と no_dummy を両方 encode し、出力 token 数が少ない方を採用。
///
/// 仮説: Leaf エンコーダは「dummy あり/なし両方試して圧縮率の良い方を選ぶ」
/// 二段階エンコーダ。token 数で判定するのは LZSS フラグビットとペイロードの
/// 比例関係から圧縮 byte サイズと相関するため。
pub fn compress_okumura_min_tokens(input: &[u8]) -> Vec<Token> {
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    if oku.len() <= nod.len() {
        oku
    } else {
        nod
    }
}

/// LZSS の正確なバイトサイズを計算（8 token ごとに 1 flag byte、Literal=1B、Match=2B）。
fn lzss_byte_size(toks: &[Token]) -> usize {
    let mut size = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if i % 8 == 0 {
            size += 1;
        }
        match t {
            Token::Literal(_) => size += 1,
            Token::Match { .. } => size += 2,
        }
    }
    size
}

/// 奥村と no_dummy を両方 encode し、LZSS バイト長が短い方を採用。
/// `min_tokens` の精度向上版（フラグバイトと literal/match 比率を考慮）。
pub fn compress_okumura_min_bytes(input: &[u8]) -> Vec<Token> {
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    let ob = lzss_byte_size(&oku);
    let nb = lzss_byte_size(&nod);
    if ob <= nb {
        oku
    } else {
        nod
    }
}

/// 奥村と no_dummy で、奥村が厳密に小さい時のみ奥村採用（タイは no_dummy 優先）。
pub fn compress_okumura_min_bytes_strict(input: &[u8]) -> Vec<Token> {
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    let ob = lzss_byte_size(&oku);
    let nb = lzss_byte_size(&nod);
    if ob < nb {
        oku
    } else {
        nod
    }
}

/// 奥村と no_dummy で、no_dummy が厳密に小さい時のみ no_dummy 採用（タイは奥村優先）。
pub fn compress_okumura_min_bytes_oku_pref(input: &[u8]) -> Vec<Token> {
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    let ob = lzss_byte_size(&oku);
    let nb = lzss_byte_size(&nod);
    if nb < ob {
        nod
    } else {
        oku
    }
}

/// 単色先頭判定 + サイズ判定の合わせ技。
/// 先頭 F バイトが同値なら奥村、それ以外は奥村サイズ < no_dummy サイズの時のみ奥村。
pub fn compress_okumura_combo(input: &[u8]) -> Vec<Token> {
    if input.len() >= F && input[..F].iter().all(|&b| b == input[0]) {
        return compress_okumura(input);
    }
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    if lzss_byte_size(&oku) < lzss_byte_size(&nod) {
        oku
    } else {
        nod
    }
}

/// 奥村と no_dummy のうち「Leaf 出力サイズ」と一致する方を採用（オラクル）。
/// 真のエンコーダロジック解析用。bench で「true Leaf size をどれだけ再現できるか」の上限測定に使う。
pub fn compress_okumura_oracle_size(input: &[u8], leaf_size: usize) -> Vec<Token> {
    let oku = compress_okumura(input);
    let nod = compress_okumura_no_dummy(input);
    let ob = lzss_byte_size(&oku);
    let nb = lzss_byte_size(&nod);
    let od = (ob as i64 - leaf_size as i64).abs();
    let nd = (nb as i64 - leaf_size as i64).abs();
    if od <= nd {
        oku
    } else {
        nod
    }
}

/// no_dummy ベース + 動的 tie 規則（短マッチは AllowEq、長マッチは StrictGt）。
///
/// 仮説 (セッション 295 の U 字分布発見に基づく):
/// max_len=3 → rank=末尾 60.5%, max_len=18 → rank=先頭 87.3%。
/// この非対称性を「短マッチは末尾上書き、長マッチは先頭保持」で再現する。
pub fn compress_okumura_no_dummy_dyntie(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::DynamicShortEq;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

/// no_dummy ベースで最小マッチ長を 4 にした変種（THRESHOLD を 2 → 3 に切替相当）。
///
/// 仮説: Leaf は 3 バイトマッチをリテラル 3 個より得と判断せず、
/// `len <= 3` を Literal で出している。これにより no_dummy 残差の
/// `MATCH_vs_LIT:len<=5` クラスタ（58 ファイル）の解消を狙う。
pub fn compress_okumura_no_dummy_min4(input: &[u8]) -> Vec<Token> {
    const LOCAL_THRESHOLD: usize = 3;
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        if (st.match_length as usize) <= LOCAL_THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }

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

/// no_dummy ベース + insert_node の左右反転 (cmp 初期値 -1 + tie で左)。
///
/// セッション 297 末尾「次にやる」未着手案。同じ subtree の異なる候補を返す可能性を狙う。
pub fn compress_okumura_no_dummy_left_first(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_with_bst(input, BstMode::LeftFirst)
}

/// Lazy match with `>=` condition (equal length も lazy 採用)。
/// 既存 compress_okumura_lazy は strict `>` のみ。
/// 観察 (M14 後の lf2_no_init_diff): C1203/C0101/C0205 で leaf が Literal を
/// 選び翌 token で **同じ長さ** Match を選ぶ pattern。
pub fn compress_okumura_lazy_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl(input, true, false)
}

/// no_dummy + Default BST + Lazy `>=`
pub fn compress_okumura_no_dummy_lazy_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl(input, true, true)
}

/// no_dummy + LeftFirst BST + Lazy `>=` (= 主要候補と組合せる)
pub fn compress_okumura_no_dummy_left_first_lazy_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl_with_bst(input, true, true, BstMode::LeftFirst)
}

/// no_dummy + NoSwap BST + Lazy `>=`
pub fn compress_okumura_no_dummy_no_swap_lazy_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl_with_bst(input, true, true, BstMode::NoSwap)
}

/// (with) dummy + LeftFirst BST + Lazy `>=`
pub fn compress_okumura_left_first_lazy_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl_with_bst(input, true, false, BstMode::LeftFirst)
}

/// AllowEq tie + Lazy `>=` (no_dummy + LeftFirst)
pub fn compress_okumura_no_dummy_left_first_lazy_eq_tie_eq(input: &[u8]) -> Vec<Token> {
    lazy_impl_with_bst_tie(input, true, true, BstMode::LeftFirst, TieMode::AllowEq)
}

fn lazy_impl_with_bst_tie(
    input: &[u8],
    allow_eq: bool,
    no_dummy: bool,
    bst_mode: BstMode,
    tie_mode: TieMode,
) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = tie_mode;
    st.bst_mode = bst_mode;
    st.init_tree();
    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        st.text_buf[r as usize + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }
    if len == 0 { return out; }
    if !no_dummy {
        for i in 1..=F { st.insert_node(r - i as i32); }
    }
    st.insert_node(r);
    loop {
        if st.match_length as usize > len { st.match_length = len as i32; }
        let pos1 = st.match_position;
        let len1 = st.match_length as usize;
        let mut take_lazy = false;
        if len1 > THRESHOLD && len1 < len {
            let saved_byte_at_r = st.text_buf[r as usize];
            if input_idx < input.len() {
                st.delete_node(s);
                let c = input[input_idx];
                input_idx += 1;
                st.text_buf[s as usize] = c;
                if (s as usize) < F - 1 { st.text_buf[s as usize + N] = c; }
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                st.insert_node(r);
            } else {
                st.delete_node(s);
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                len -= 1;
                if len > 0 { st.insert_node(r); } else { st.match_length = 0; }
            }
            if st.match_length as usize > len { st.match_length = len as i32; }
            let len2 = st.match_length as usize;
            let lazy_cond = if allow_eq { len2 >= len1 } else { len2 > len1 };
            if lazy_cond {
                out.push(Token::Literal(saved_byte_at_r));
                take_lazy = true;
            } else {
                out.push(Token::Match { pos: (pos1 as u16) & ((N as u16) - 1), len: len1 as u8 });
                let last_match_length = len1;
                let mut i = 1usize;
                while i < last_match_length && input_idx < input.len() {
                    st.delete_node(s);
                    let c = input[input_idx];
                    input_idx += 1;
                    st.text_buf[s as usize] = c;
                    if (s as usize) < F - 1 { st.text_buf[s as usize + N] = c; }
                    s = (s + 1) & (N as i32 - 1);
                    r = (r + 1) & (N as i32 - 1);
                    st.insert_node(r);
                    i += 1;
                }
                while i < last_match_length {
                    st.delete_node(s);
                    s = (s + 1) & (N as i32 - 1);
                    r = (r + 1) & (N as i32 - 1);
                    len -= 1;
                    if len > 0 { st.insert_node(r); }
                    i += 1;
                }
                if len == 0 { break; }
                continue;
            }
        }
        if take_lazy { if len == 0 { break; } continue; }
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match { pos: (st.match_position as u16) & ((N as u16) - 1), len: st.match_length as u8 });
        }
        let last_match_length = st.match_length as usize;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 { st.text_buf[s as usize + N] = c; }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len -= 1;
            if len > 0 { st.insert_node(r); }
            i += 1;
        }
        if len == 0 { break; }
    }
    out
}

fn lazy_impl(input: &[u8], allow_eq: bool, no_dummy: bool) -> Vec<Token> {
    lazy_impl_with_bst(input, allow_eq, no_dummy, BstMode::Standard)
}

fn lazy_impl_with_bst(
    input: &[u8],
    allow_eq: bool,
    no_dummy: bool,
    bst_mode: BstMode,
) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.bst_mode = bst_mode;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    if !no_dummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }
        let pos1 = st.match_position;
        let len1 = st.match_length as usize;

        let mut take_lazy = false;
        if len1 > THRESHOLD && len1 < len {
            let saved_byte_at_r = st.text_buf[r as usize];
            if input_idx < input.len() {
                st.delete_node(s);
                let c = input[input_idx];
                input_idx += 1;
                st.text_buf[s as usize] = c;
                if (s as usize) < F - 1 {
                    st.text_buf[s as usize + N] = c;
                }
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                st.insert_node(r);
            } else {
                st.delete_node(s);
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                len -= 1;
                if len > 0 {
                    st.insert_node(r);
                } else {
                    st.match_length = 0;
                }
            }
            if st.match_length as usize > len {
                st.match_length = len as i32;
            }
            let len2 = st.match_length as usize;

            let lazy_cond = if allow_eq { len2 >= len1 } else { len2 > len1 };
            if lazy_cond {
                out.push(Token::Literal(saved_byte_at_r));
                take_lazy = true;
            } else {
                out.push(Token::Match {
                    pos: (pos1 as u16) & ((N as u16) - 1),
                    len: len1 as u8,
                });
                let last_match_length = len1;
                let mut i = 1usize;
                while i < last_match_length && input_idx < input.len() {
                    st.delete_node(s);
                    let c = input[input_idx];
                    input_idx += 1;
                    st.text_buf[s as usize] = c;
                    if (s as usize) < F - 1 {
                        st.text_buf[s as usize + N] = c;
                    }
                    s = (s + 1) & (N as i32 - 1);
                    r = (r + 1) & (N as i32 - 1);
                    st.insert_node(r);
                    i += 1;
                }
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
                if len == 0 { break; }
                continue;
            }
        }
        if take_lazy {
            if len == 0 { break; }
            continue;
        }
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }
        let last_match_length = st.match_length as usize;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
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
        if len == 0 { break; }
    }
    out
}

/// no_dummy + LeftFirst BST + AllowEq tie (= 同最大長候補で「より新しい」を採用)。
/// 既存組み合わせに含まれていなかったセル。
pub fn compress_okumura_no_dummy_left_first_eq(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::AllowEq;
    st.bst_mode = BstMode::LeftFirst;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    st.insert_node(r);
    encode_loop(&mut st, &mut out, &mut r, &mut s, &mut input_idx, &mut len, input);
    out
}

/// no_dummy + Default BST + AllowEq tie。同様に未試行セル。
pub fn compress_okumura_no_dummy_eq(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::AllowEq;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    st.insert_node(r);
    encode_loop(&mut st, &mut out, &mut r, &mut s, &mut input_idx, &mut len, input);
    out
}

/// no_dummy + Default BST + DistanceTie。同様に未試行セル。
pub fn compress_okumura_no_dummy_distance_tie(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::DistanceTie;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    st.insert_node(r);
    encode_loop(&mut st, &mut out, &mut r, &mut s, &mut input_idx, &mut len, input);
    out
}

/// no_dummy_left_first + lazy lookahead (1 step).
///
/// 仮説: Leaf encoder は no_dummy ベースで left_first BST を使い、かつ
/// 1 step ルックアヘッドの lazy を併用する。compress_okumura_lazy は
/// dummy + Default BST、no_dummy_left_first は greedy。両方混ぜたものは未試行。
pub fn compress_okumura_no_dummy_left_first_lazy(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.bst_mode = BstMode::LeftFirst;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    // no_dummy: dummy 挿入なし
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        let pos1 = st.match_position;
        let len1 = st.match_length as usize;

        let mut take_lazy = false;
        if len1 > THRESHOLD && len1 < len {
            let saved_byte_at_r = st.text_buf[r as usize];
            if input_idx < input.len() {
                st.delete_node(s);
                let c = input[input_idx];
                input_idx += 1;
                st.text_buf[s as usize] = c;
                if (s as usize) < F - 1 {
                    st.text_buf[s as usize + N] = c;
                }
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                st.insert_node(r);
            } else {
                st.delete_node(s);
                s = (s + 1) & (N as i32 - 1);
                r = (r + 1) & (N as i32 - 1);
                len -= 1;
                if len > 0 {
                    st.insert_node(r);
                } else {
                    st.match_length = 0;
                }
            }
            if st.match_length as usize > len {
                st.match_length = len as i32;
            }
            let len2 = st.match_length as usize;

            if len2 > len1 {
                out.push(Token::Literal(saved_byte_at_r));
                take_lazy = true;
            } else {
                out.push(Token::Match {
                    pos: (pos1 as u16) & ((N as u16) - 1),
                    len: len1 as u8,
                });
                let last_match_length = len1;
                let mut i = 1usize;
                while i < last_match_length && input_idx < input.len() {
                    st.delete_node(s);
                    let c = input[input_idx];
                    input_idx += 1;
                    st.text_buf[s as usize] = c;
                    if (s as usize) < F - 1 {
                        st.text_buf[s as usize + N] = c;
                    }
                    s = (s + 1) & (N as i32 - 1);
                    r = (r + 1) & (N as i32 - 1);
                    st.insert_node(r);
                    i += 1;
                }
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
                continue;
            }
        }
        if take_lazy {
            if len == 0 {
                break;
            }
            continue;
        }
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }
        let last_match_length = st.match_length as usize;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
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

/// no_dummy_left_first ベース + 先頭 `early_bytes` までは強制 Literal。
///
/// session 364 first_diff 解析で判明: leaf=lit, oku=match の divergence が
/// 49 ファイル発生。うち 26 が y=0、42 が y<=5 (image width 起因)。
/// encoder は初期 ring (0x20 で初期化された未書込み領域) へのマッチを
/// 既存 no_dummy より厳しく排除している可能性。
pub fn compress_okumura_no_dummy_left_first_early_lit(
    input: &[u8],
    early_bytes: usize,
) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.bst_mode = BstMode::LeftFirst;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;
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
    st.insert_node(r);

    // encode_loop 相当 + early literal 強制
    let mut emitted_bytes: usize = 0;
    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }

        let force_lit = emitted_bytes < early_bytes;
        if force_lit || (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;
        emitted_bytes += last_match_length;

        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
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

/// no_dummy ベース + insert_node の swap-with-r ブロックをスキップ。
///
/// 仮説: Leaf は F バイト完全一致が出ても新ノードを BST に入れず、古いノードを保持する。
/// セッション 296 末尾「F-dummy with no swap-with-r」の no_dummy 版。
pub fn compress_okumura_no_dummy_no_swap(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_with_bst(input, BstMode::NoSwap)
}

/// 奥村原典どおり F dummy 挿入 + insert_node の swap-with-r ブロックをスキップ。
///
/// 仮説: F dummy で BST に F 個のノードが入り、swap が抑制されるので
/// dummy ノードがそのまま残り続ける。token 0 で奥村と同じ Match{len=F} を出しつつ、
/// その後の swap 抑制で no_dummy に近い挙動になる可能性。
pub fn compress_okumura_dummy_no_swap(input: &[u8]) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.bst_mode = BstMode::NoSwap;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    encode_loop(&mut st, &mut out, &mut r, &mut s, &mut input_idx, &mut len, input);
    out
}

/// no_dummy ベースで BstMode を切り替えて回す共通実装。
fn compress_okumura_no_dummy_with_bst(input: &[u8], bst_mode: BstMode) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.bst_mode = bst_mode;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut r: i32 = (N - F) as i32;
    let mut s: i32 = 0;

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

    st.insert_node(r);

    encode_loop(&mut st, &mut out, &mut r, &mut s, &mut input_idx, &mut len, input);
    out
}

/// 奥村 lzss.c の `Encode()` メインループ部分（先読み・初期 insert は呼び出し側責任）。
fn encode_loop(
    st: &mut Okumura,
    out: &mut Vec<Token>,
    r: &mut i32,
    s: &mut i32,
    input_idx: &mut usize,
    len: &mut usize,
    input: &[u8],
) {
    loop {
        if st.match_length as usize > *len {
            st.match_length = *len as i32;
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[*r as usize]));
        } else {
            out.push(Token::Match {
                pos: (st.match_position as u16) & ((N as u16) - 1),
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        let mut i = 0usize;
        while i < last_match_length && *input_idx < input.len() {
            st.delete_node(*s);
            let c = input[*input_idx];
            *input_idx += 1;

            st.text_buf[*s as usize] = c;
            if (*s as usize) < F - 1 {
                st.text_buf[*s as usize + N] = c;
            }

            *s = (*s + 1) & (N as i32 - 1);
            *r = (*r + 1) & (N as i32 - 1);
            st.insert_node(*r);
            i += 1;
        }

        while i < last_match_length {
            st.delete_node(*s);
            *s = (*s + 1) & (N as i32 - 1);
            *r = (*r + 1) & (N as i32 - 1);
            *len -= 1;
            if *len > 0 {
                st.insert_node(*r);
            }
            i += 1;
        }

        if *len == 0 {
            break;
        }
    }
}

fn compress_okumura_impl(input: &[u8], tie_mode: TieMode) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = tie_mode;
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

    /// 距離タイブレイク版でもスペース連続が F バイトのマッチで返ることを確認。
    #[test]
    fn distance_tie_run_of_spaces_matches_initial_ring() {
        let input = vec![b' '; 20];
        let toks = compress_okumura_distance_tie(&input);
        match toks[0] {
            Token::Match { len, .. } => assert_eq!(len, F as u8),
            _ => panic!("expected match, got {:?}", toks[0]),
        }
    }

    /// lazy 版でもスペース連続が F バイトの match で返り、トークン列が
    /// 元の入力に decode し直せることを確認するスモークテスト。
    #[test]
    fn lazy_run_of_spaces_roundtrip() {
        let input = vec![b' '; 20];
        let toks = compress_okumura_lazy(&input);
        // decode token stream into bytes using the same ring init as encoder.
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input, "lazy round-trip must reproduce input");
    }

    /// より複雑な入力でも lazy 版が round-trip することを確認。
    #[test]
    fn lazy_abc_repeat_roundtrip() {
        let input: Vec<u8> = (0..200u32).map(|i| b'A' + (i % 26) as u8).collect();
        let toks = compress_okumura_lazy(&input);
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input, "lazy round-trip on ABC..Z*8 must reproduce input");
    }

    #[test]
    fn lazy_short_inputs_dont_panic() {
        // end-of-input bookkeeping のスモーク。短い入力で panic しないこと。
        for n in 0..40usize {
            let input: Vec<u8> = (0..n as u32).map(|i| (i & 0xff) as u8).collect();
            let toks = compress_okumura_lazy(&input);
            let decoded = decode_oku_tokens(&toks);
            assert_eq!(decoded, input, "lazy round-trip for n={}", n);
        }
    }

    /// no_dummy 版のスモーク: AAAAA を round-trip できる。
    #[test]
    fn no_dummy_aaaaa_roundtrip() {
        let input = b"AAAAA".to_vec();
        let toks = compress_okumura_no_dummy(&input);
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input, "no_dummy round-trip on AAAAA");
    }

    /// one_dummy_at_rf 版のスモーク: round-trip と、空白18バイトで
    /// token 0 が `Match{pos=N-2F, len=18}` になることを確認。
    #[test]
    fn one_dummy_at_rf_aaaaa_roundtrip() {
        let input = b"AAAAA".to_vec();
        let toks = compress_okumura_one_dummy_at_rf(&input);
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input, "one_dummy_at_rf round-trip on AAAAA");
    }

    #[test]
    fn dummy_then_drop_aaaaa_roundtrip() {
        let input = b"AAAAA".to_vec();
        let toks = compress_okumura_dummy_then_drop(&input);
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input, "dummy_then_drop round-trip on AAAAA");
    }

    #[test]
    fn dummy_then_drop_emits_token0_match_for_spaces() {
        let input = vec![0x20u8; 18];
        let toks = compress_okumura_dummy_then_drop(&input);
        let first = toks.first().expect("at least one token");
        // 奥村原典どおり F dummy が居れば token 0 は Match{len=F}。pos は最初に当たったノード
        // (実装依存: 奥村は r-1 を最後に挿入するので最も新しい r-1 が当たることが多い)
        match *first {
            Token::Match { len, .. } => assert_eq!(len as usize, F),
            Token::Literal(_) => panic!("expected Match"),
        }
    }

    #[test]
    fn one_dummy_at_rf_emits_match_for_18_spaces() {
        let input = vec![0x20u8; 18];
        let toks = compress_okumura_one_dummy_at_rf(&input);
        let first = toks.first().expect("at least one token");
        match *first {
            Token::Match { pos, len } => {
                assert_eq!(pos as usize, N - 2 * F, "pos should be N - 2F");
                assert_eq!(len as usize, F, "len should be F=18");
            }
            Token::Literal(_) => panic!("expected Match, got Literal"),
        }
    }

    #[test]
    fn no_dummy_left_first_roundtrips() {
        for n in 0..40usize {
            let input: Vec<u8> = (0..n as u32).map(|i| (i & 0xff) as u8).collect();
            let toks = compress_okumura_no_dummy_left_first(&input);
            let decoded = decode_oku_tokens(&toks);
            assert_eq!(decoded, input, "no_dummy_left_first round-trip n={}", n);
        }
    }

    #[test]
    fn no_dummy_no_swap_roundtrips() {
        for n in 0..40usize {
            let input: Vec<u8> = (0..n as u32).map(|i| (i & 0xff) as u8).collect();
            let toks = compress_okumura_no_dummy_no_swap(&input);
            let decoded = decode_oku_tokens(&toks);
            assert_eq!(decoded, input, "no_dummy_no_swap round-trip n={}", n);
        }
    }

    #[test]
    fn dummy_no_swap_roundtrips() {
        for n in 0..40usize {
            let input: Vec<u8> = (0..n as u32).map(|i| (i & 0xff) as u8).collect();
            let toks = compress_okumura_dummy_no_swap(&input);
            let decoded = decode_oku_tokens(&toks);
            assert_eq!(decoded, input, "dummy_no_swap round-trip n={}", n);
        }
    }

    #[test]
    fn no_dummy_left_first_run_of_spaces() {
        let input = vec![b' '; 20];
        let toks = compress_okumura_no_dummy_left_first(&input);
        let decoded = decode_oku_tokens(&toks);
        assert_eq!(decoded, input);
    }

    /// 奥村 token 列を decode してバイト列に戻す簡易デコーダ（テスト専用）。
    fn decode_oku_tokens(toks: &[Token]) -> Vec<u8> {
        let mut ring = vec![0x20u8; N];
        let mut r: usize = N - F;
        let mut out: Vec<u8> = Vec::new();
        for t in toks {
            match *t {
                Token::Literal(b) => {
                    ring[r] = b;
                    r = (r + 1) & (N - 1);
                    out.push(b);
                }
                Token::Match { pos, len } => {
                    let p = pos as usize;
                    for i in 0..len as usize {
                        let b = ring[(p + i) & (N - 1)];
                        ring[r] = b;
                        r = (r + 1) & (N - 1);
                        out.push(b);
                    }
                }
            }
        }
        out
    }
}
