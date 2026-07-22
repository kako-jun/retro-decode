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
///   近いほうを採用（Leaf 系エンコーダの観測されたバイアス）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TieMode {
    StrictGt,
    AllowEq,
    DistanceTie,
    /// 短マッチ (len ≤ 3) は AllowEq、それ以外は StrictGt。
    /// セッション 295 の U 字分布発見 (max_len=3 → rank=末尾 60.5%, max_len=18 → rank=先頭 87.3%) に対応する仮説。
    DynamicShortEq,
    /// 同一長のとき r から **遠い** 候補 (max-dist) を選択する版。
    MaxDistTie,
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
    let _ = writeln!(s, "    pos    dad    lson   rson   dist   bytes[0..3]");
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
            pos as u16, snap.dad[pos], snap.lson[pos], snap.rson[pos], dist as u16, b0, b1, b2
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
    /// BST root key 計算モード (Standard = text_buf[r], XorByte2 = text_buf[r] ^ text_buf[r+1])
    key_mode: KeyMode,
    /// Stage 12-6 (Issue #14): ノード内比較 (`cmp = key[i] - text_buf[p+i]`、
    /// index 1..F) のバイト解釈モード。root byte0 によるバケツ選択 (256分木の
    /// インデックス) には影響しない — あくまで木内部の大小比較だけを変える。
    cmp_mode: CmpMode,
    /// Stage 12-7 (Issue #14): `delete_node` の両子ケースで昇格させる
    /// in-order 隣接ノードの側 (前任者=左部分木最右 / 後継者=右部分木最左)。
    del_mode: DelMode,
    /// Stage 12-11 (Issue #14 脈: 「腐った木」仮説)。true のとき、呼び出し側
    /// (自走エンコーダ・`OkumuraSim::advance`) は消費バイトの `delete_node(s)`
    /// 呼び出しを一切スキップする。ノードは自分の位置が次に `insert_node` で
    /// 再挿入されるまで、リング上書き後も**古い鍵のまま**木に残留し続ける
    /// (=「腐った」ノード)。`insert_node` 側は、既に木に居る位置 r を再挿入
    /// する際にダングリング防止のため構造的 unlink を行う (詳細は
    /// `insert_node` 冒頭のコメント参照)。
    rot_no_delete: bool,
    /// Stage 12-14 (Issue #14 脈1 Prong A): `delete_node_predecessor` の両子
    /// ケースで昇格したノード位置 `q` を記録する (読み取り専用ログ、木構造には
    /// 影響しない)。`OkumuraSim::take_promotion_log` で drain する。
    promotion_log: Vec<i32>,
    /// Stage 12-14: `insert_node` の EQ (F バイト完全一致) 置換で追い出された
    /// 旧ノード位置 `p` を記録する (読み取り専用ログ)。
    replace_log: Vec<i32>,
    /// Stage 12-15 (Issue #14 脈1 Prong B): 「書込み時挿入」変種 (自走エンコーダの
    /// `write_time_descending`) を使っているかどうか。挙動には一切影響せず、
    /// `OKU_DEBUG_TREE_CHECK` 環境変数指定時の毎操作不変条件チェックを
    /// この変種でも有効にするためだけに使う。
    write_time_variant: bool,
    /// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討)。`insert_node` の
    /// ノード内比較ループの上限バイト数。既定は `F` (原典と同一、全比較)。
    /// `F` 未満に設定すると、比較を `f_bound` バイトで打ち切る (それ以上の
    /// 内容は BST の探索・タイブレイクに一切影響しなくなる)。EOF 近傍で
    /// 「残り入力バイト数 (+微小オフセット) までしか比較しない」仮説の検証用。
    f_bound: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyMode {
    Byte0,
    XorByte01,
    AddByte01Mod256,
}

/// Stage 12-7 (Issue #14 脈: 削除昇格側 / 鏡像等価性検証)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelMode {
    /// 原典: 両子ケースは in-order **前任者** (左部分木の最右子孫) を昇格。
    Predecessor,
    /// `Predecessor` の全域鏡像 (単一子ケースの昇格方向も含め `lson`⇔`rson`
    /// を総入れ替え)。両子ケースは in-order **後継者** (右部分木の最左子孫) を昇格。
    Successor,
}

/// Stage 12-16 (Issue #14 脈1 Prong B 続き): 自走エンコーダ
/// (`compress_okumura_impl_hooked_traced_full`) 側の「書込み時挿入」順序。
/// Stage 12-15 の `write_time_descending: bool` を、Ascending も自走側で
/// 実装するために3値化した (`SimMode::WriteTimeDescending`/`WriteTimeAscending`
/// と対応)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTimeOrder {
    /// 原典どおり dummy F 個挿入 + per-byte insert (書込み時挿入なし)。
    None,
    /// 初期先読み充填 [r, r+F-1] を降順 (r+F-1 → r) で一括挿入。
    Descending,
    /// 初期先読み充填 [r, r+F-1] を昇順 (r → r+F-1) で一括挿入。
    Ascending,
    /// Stage 12-17 (Issue #14 脈1 Prong B 続き): Step 1 の外部シャドートラッカーで
    /// 特定した「dummy 帯 [r-F,r-1] が一度も挿入されない空白」問題のハイブリッド修正。
    /// `Descending` に加え、原典の dummy F 個 (`r-F..r-1`) も先に挿入したまま残す
    /// (dummy → 実データ18個・降順の順。`SimMode::WriteTimeDescendingKeepDummy` 相当)。
    DescendingKeepDummy,
    /// `Ascending` + dummy 保持版 (`SimMode::WriteTimeAscendingKeepDummy` 相当)。
    AscendingKeepDummy,
}

/// Stage 12-6 (Issue #14 脈: signed char 比較仮説)。奥村原典の
/// `cmp = key[i] - text_buf[p+i]` は Leaf 時代のコンパイラ (Turbo C/VC++)
/// では `char` が既定で符号付きだった可能性がある。現行 Rust 移植は
/// `u8 as i32` (無符号拡張) で比較しており、0x80 以上のバイトで大小関係が
/// 反転しうる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpMode {
    /// 現行 (無変更): `byte as i32`。
    Unsigned,
    /// 仮説本体: `(byte as i8) as i32` (= `(byte ^ 0x80)` の unsigned 比較と等価)。
    Signed,
    /// 対照用: 大小を完全反転 (`-(byte as i32)`)。
    Reversed,
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
            key_mode: KeyMode::Byte0,
            cmp_mode: CmpMode::Unsigned,
            del_mode: DelMode::Predecessor,
            rot_no_delete: false,
            promotion_log: Vec::new(),
            replace_log: Vec::new(),
            write_time_variant: false,
            f_bound: F,
        }
    }

    /// Stage 14-2 (Issue #14 脈: per-file 小状態フィッティング)。`new(fill)` の
    /// 一般化版で、text_buf の初期値を単一 fill バイトではなく任意のバイト列
    /// (`N + F - 1` 要素) から構築する。ring 初期内容汚染仮説 (⑧/⑩) の検証専用。
    fn new_from_buf(buf: [u8; N + F - 1]) -> Self {
        Self {
            text_buf: buf,
            lson: [0; N + 257],
            rson: [0; N + 257],
            dad: [0; N + 1],
            match_position: 0,
            match_length: 0,
            tie_mode: TieMode::StrictGt,
            cur_r: 0,
            bst_mode: BstMode::Standard,
            key_mode: KeyMode::Byte0,
            cmp_mode: CmpMode::Unsigned,
            del_mode: DelMode::Predecessor,
            rot_no_delete: false,
            promotion_log: Vec::new(),
            replace_log: Vec::new(),
            write_time_variant: false,
            f_bound: F,
        }
    }

    /// ノード内比較 (index 1..F) 用にバイトを比較可能な値へ変換する
    /// (`cmp_mode` 参照)。root byte0 のバケツ選択には使わない。
    #[inline]
    fn cmp_val(&self, byte: u8) -> i32 {
        match self.cmp_mode {
            CmpMode::Unsigned => byte as i32,
            CmpMode::Signed => (byte as i8) as i32,
            CmpMode::Reversed => -(byte as i32),
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

    /// Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則)。
    ///
    /// `insert_node` と全く同じ木構造・同じノード内比較ロジックで最長一致を
    /// 探索するが、木の変更 (ノード挿入・delete_node の呼び出し) は一切
    /// 行わない読み取り専用版。呼び出し時点で `r` 自身が既に木に挿入済み
    /// (`insert_node(r)` 呼び出し後) であっても、`next == r` に到達したら
    /// それを「まだ挿入されていない (NIL)」として扱うため、挿入前と同じ
    /// 探索結果が得られる。`tie_mode` を渡すことでタイ勝者選定則だけを
    /// 差し替えられる (例: `AllowEq` は「最初に見つかった同着」ではなく
    /// 「木を下りながら最後に見つかった同着」を勝者にする)。
    fn search_match_readonly(&self, r: i32, tie_mode: TieMode) -> (i32, i32) {
        let mut cmp: i32 = match self.bst_mode {
            BstMode::LeftFirst => -1,
            _ => 1,
        };
        let key_start = r as usize;
        let p_root_byte = match self.key_mode {
            KeyMode::Byte0 => self.text_buf[key_start],
            KeyMode::XorByte01 => self.text_buf[key_start] ^ self.text_buf[key_start + 1],
            KeyMode::AddByte01Mod256 => {
                self.text_buf[key_start].wrapping_add(self.text_buf[key_start + 1])
            }
        };
        let p_root_idx = N as i32 + 1 + p_root_byte as i32;
        let mut p: i32 = p_root_idx;
        let mut match_length: i32 = 0;
        let mut match_position: i32 = 0;

        let mut guard: u32 = 0;
        loop {
            guard += 1;
            if guard > 4 * N as u32 {
                break;
            }
            let go_right = match self.bst_mode {
                BstMode::LeftFirst => cmp > 0,
                _ => cmp >= 0,
            };
            let next = if go_right {
                self.rson[p as usize]
            } else {
                self.lson[p as usize]
            };
            // `r` 自身は (呼び出し時点で既に挿入済みなら) 見かけ上の
            // リーフとして木に存在するが、挿入前の状態を再現するため
            // NIL 到達と同一視して打ち切る。
            if next == NIL || next == r {
                break;
            }
            p = next;

            let mut i: usize = 1;
            cmp = 0;
            while i < self.f_bound {
                let a = self.cmp_val(self.text_buf[key_start + i]);
                let b = self.cmp_val(self.text_buf[p as usize + i]);
                let d = a - b;
                if d != 0 {
                    cmp = d;
                    break;
                }
                i += 1;
            }

            let take = match tie_mode {
                TieMode::StrictGt => (i as i32) > match_length,
                TieMode::AllowEq => (i as i32) >= match_length,
                _ => (i as i32) > match_length,
            };
            if take {
                match_position = p;
                match_length = i as i32;
                if i >= self.f_bound {
                    break;
                }
            }
        }
        (match_position, match_length)
    }

    /// Stage 14-5 (Issue #14 脈: ⑲続) 診断専用。現在の木構造上での in-order
    /// 後続ノード (`p` の次に大きいキーを持つノード) を返す。標準アルゴリズム:
    /// 右部分木があればその最左子孫、なければ「左の子として来た」最初の
    /// 祖先。見つからなければ `NIL`。
    fn inorder_successor(&self, p: i32) -> i32 {
        if self.rson[p as usize] != NIL {
            let mut q = self.rson[p as usize];
            while self.lson[q as usize] != NIL {
                q = self.lson[q as usize];
            }
            return q;
        }
        let mut child = p;
        let mut parent = self.dad[p as usize];
        let mut guard = 0u32;
        while parent < N as i32 && self.rson[parent as usize] == child {
            child = parent;
            parent = self.dad[parent as usize];
            guard += 1;
            if guard > N as u32 {
                return NIL;
            }
        }
        if parent >= N as i32 {
            NIL
        } else {
            parent
        }
    }

    /// `inorder_successor` の鏡像 (in-order 前任ノード)。
    fn inorder_predecessor(&self, p: i32) -> i32 {
        if self.lson[p as usize] != NIL {
            let mut q = self.lson[p as usize];
            while self.rson[q as usize] != NIL {
                q = self.rson[q as usize];
            }
            return q;
        }
        let mut child = p;
        let mut parent = self.dad[p as usize];
        let mut guard = 0u32;
        while parent < N as i32 && self.lson[parent as usize] == child {
            child = parent;
            parent = self.dad[parent as usize];
            guard += 1;
            if guard > N as u32 {
                return NIL;
            }
        }
        if parent >= N as i32 {
            NIL
        } else {
            parent
        }
    }

    /// 原典 `InsertNode(int r)` 逐語移植。
    ///
    /// text_buf[r..r+F-1] を木に挿入し、同時に最長一致を探索する。
    /// 結果は `self.match_position` / `self.match_length` に格納される。
    fn insert_node(&mut self, r: i32) {
        // Stage 12-11 (Issue #14 脈: 「腐った木」仮説): `rot_no_delete` のとき
        // `delete_node(s)` が一切呼ばれないため、位置 r が既に木に残留して
        // いる (前サイクルの「腐った」ノードとして) 場合がある。この関数は
        // 直後に `rson[r]=NIL; lson[r]=NIL;` で r の子リンクを無条件に潰して
        // 新規ノードとして挿入するため、既存の dad[r] リンク (r の古い親から
        // 見た子リンク) を直さないまま進めると、その古い親が r を指したまま
        // ダングリングになる。ここで**構造的unlink** (内容比較なし、
        // del_mode に従った splice) を先に行い、ダングリング/循環を防ぐ。
        // これは「腐敗」そのもの (古い鍵のまま残留すること) には無関係で、
        // あくまで「同じ物理位置に2つのツリーエントリが同時に存在する」
        // という構造的に不正な状態を避けるための処理。
        if self.rot_no_delete && self.dad[r as usize] != NIL {
            self.delete_node(r);
        }
        // BstMode::LeftFirst: cmp = -1 初期 + cmp > 0 のみ右へ (奥村の左右反転)
        let mut cmp: i32 = match self.bst_mode {
            BstMode::LeftFirst => -1,
            _ => 1,
        };
        let key_start = r as usize;
        // key = &text_buf[r..]
        let p_root_byte = match self.key_mode {
            KeyMode::Byte0 => self.text_buf[key_start],
            KeyMode::XorByte01 => self.text_buf[key_start] ^ self.text_buf[key_start + 1],
            KeyMode::AddByte01Mod256 => {
                self.text_buf[key_start].wrapping_add(self.text_buf[key_start + 1])
            }
        };
        let p_root_idx = N as i32 + 1 + p_root_byte as i32;
        let mut p: i32 = p_root_idx;

        self.rson[r as usize] = NIL;
        self.lson[r as usize] = NIL;
        self.match_length = 0;
        self.cur_r = r;

        let mut guard: u32 = 0;
        loop {
            guard += 1;
            if guard > 4 * N as u32 {
                eprintln!("WARN insert_node: traversal guard tripped (r={}, guard={}) — likely a tree cycle, aborting insert", r, guard);
                self.dad[r as usize] = NIL;
                return;
            }
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
                    self.debug_check_after_insert(r, p, "rson");
                    return;
                }
            } else {
                if self.lson[p as usize] != NIL {
                    p = self.lson[p as usize];
                } else {
                    self.lson[p as usize] = r;
                    self.dad[r as usize] = p;
                    self.debug_check_after_insert(r, p, "lson");
                    return;
                }
            }

            // for (i = 1; i < F; i++) if ((cmp = key[i] - text_buf[p + i]) != 0) break;
            // Stage 14-3: 原典は常に `F`。`f_bound < F` のとき、それより先の
            // バイトは一切参照しない (探索・タイブレイク双方に影響しなくなる)。
            let mut i: usize = 1;
            cmp = 0;
            while i < self.f_bound {
                let a = self.cmp_val(self.text_buf[key_start + i]);
                let b = self.cmp_val(self.text_buf[p as usize + i]);
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
                TieMode::MaxDistTie => {
                    if (i as i32) > self.match_length {
                        true
                    } else if (i as i32) == self.match_length {
                        let mask = N as i32 - 1;
                        let cur_dist = (self.cur_r - p) & mask;
                        let best_dist = (self.cur_r - self.match_position) & mask;
                        cur_dist > best_dist
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
                if i >= self.f_bound {
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
        // Stage 12-14 (Issue #14 脈1 Prong A): EQ (F バイト完全一致) で追い出された
        // 旧ノード p を記録する (読み取り専用ログ、挙動には影響しない)。
        self.replace_log.push(p);
    }

    /// Stage 12-7 (Issue #14): `del_mode` に応じて削除昇格側を切り替える。
    fn delete_node(&mut self, p: i32) {
        match self.del_mode {
            DelMode::Predecessor => {
                // Stage 12-15 (Issue #14 脈1 Prong B): 「書込み時挿入」変種でも
                // 毎操作不変条件チェックを有効にする (Successor 側の既存診断と同型)。
                let debug = self.write_time_variant && std::env::var("OKU_DEBUG_TREE_CHECK").is_ok();
                self.delete_node_predecessor(p);
                if debug {
                    if let Err(msg) = self.tree_is_consistent_raw() {
                        eprintln!("BUG: tree inconsistent after delete_node_predecessor(p={}) [write_time_variant]: {}", p, msg);
                        std::process::exit(1);
                    }
                }
            }
            DelMode::Successor => {
                // Stage 12-8 一時診断: 呼び出しごとに木の整合性 (循環なし) を検証する。
                let debug = std::env::var("OKU_DEBUG_TREE_CHECK").is_ok();
                thread_local! {
                    static CALL_COUNT: std::cell::Cell<u64> = std::cell::Cell::new(0);
                    static RECENT: std::cell::RefCell<std::collections::VecDeque<String>> =
                        std::cell::RefCell::new(std::collections::VecDeque::new());
                }
                let (dad_p, lson_p, rson_p) = (self.dad[p as usize], self.lson[p as usize], self.rson[p as usize]);
                self.delete_node_successor(p);
                if debug {
                    let n = CALL_COUNT.with(|c| {
                        let v = c.get() + 1;
                        c.set(v);
                        v
                    });
                    let dad_p_now = self.dad[p as usize];
                    RECENT.with(|r| {
                        let mut r = r.borrow_mut();
                        if r.len() >= 15 {
                            r.pop_front();
                        }
                        r.push_back(format!(
                            "call#{} p={} before(dad={},lson={},rson={}) after(dad[p]={})",
                            n, p, dad_p, lson_p, rson_p, dad_p_now
                        ));
                    });
                    if let Err(msg) = self.tree_is_consistent_raw() {
                        eprintln!("BUG: tree inconsistent detected at call#{}: {}", n, msg);
                        eprintln!("recent delete_node_successor calls (oldest first):");
                        RECENT.with(|r| {
                            for line in r.borrow().iter() {
                                eprintln!("  {}", line);
                            }
                        });
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    /// Stage 12-8 一時診断用: `OkumuraSim::tree_is_consistent` と同じロジックを
    /// `Okumura` 自身に対して直接行う (循環検出 + 親子リンク相互整合)。
    /// Stage 12-8/12-11 一時診断用: insert_node がノードを attach した直後に
    /// 木の**構造的**整合性 (循環なし・dad/子の相互整合。順序は見ない) を確認する
    /// (del_mode==Successor または rot_no_delete のいずれか、かつ環境変数指定時のみ)。
    fn debug_check_after_insert(&self, r: i32, parent: i32, side: &str) {
        let relevant = matches!(self.del_mode, DelMode::Successor) || self.rot_no_delete || self.write_time_variant;
        if !relevant || std::env::var("OKU_DEBUG_TREE_CHECK").is_err() {
            return;
        }
        thread_local! {
            static ICALL: std::cell::Cell<u64> = std::cell::Cell::new(0);
        }
        let n = ICALL.with(|c| {
            let v = c.get() + 1;
            c.set(v);
            v
        });
        if let Err(msg) = self.tree_is_consistent_raw() {
            eprintln!(
                "BUG: tree inconsistent right after insert_node call#{} (attached r={} under parent={} via {}): {}",
                n, r, parent, side, msg
            );
            std::process::exit(1);
        }
    }

    fn tree_is_consistent_raw(&self) -> Result<(), String> {
        for pos in 0..N {
            let d = self.dad[pos];
            if d == NIL {
                continue;
            }
            let du = d as usize;
            if !(du < N || ((N + 1)..=(N + 256)).contains(&du)) {
                return Err(format!("pos={} dad={} out of range", pos, d));
            }
            if self.lson[du] != pos as i32 && self.rson[du] != pos as i32 {
                return Err(format!(
                    "pos={} dad={} but dad.lson={} dad.rson={} (neither == pos)",
                    pos, d, self.lson[du], self.rson[du]
                ));
            }
        }
        let mut reached = 0usize;
        let mut stack: Vec<i32> = Vec::new();
        for root in (N + 1)..=(N + 256) {
            if self.rson[root] != NIL {
                stack.push(self.rson[root]);
            }
            if matches!(self.bst_mode, BstMode::LeftFirst) && self.lson[root] != NIL {
                stack.push(self.lson[root]);
            }
        }
        while let Some(p) = stack.pop() {
            reached += 1;
            if reached > N {
                return Err(format!("cycle: reached > N at node {}", p));
            }
            let pu = p as usize;
            if self.lson[pu] != NIL {
                stack.push(self.lson[pu]);
            }
            if self.rson[pu] != NIL {
                stack.push(self.rson[pu]);
            }
        }
        Ok(())
    }

    /// 原典 `DeleteNode(int p)` 逐語移植。両子ケースは in-order **前任者**
    /// (左部分木の最右子孫) を昇格させる (`DelMode::Predecessor`、既定)。
    fn delete_node_predecessor(&mut self, p: i32) {
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
            // Stage 12-14 (Issue #14 脈1 Prong A): 両子ケースで昇格したノード qv を
            // 記録する (読み取り専用ログ、挙動には影響しない)。
            self.promotion_log.push(qv);
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

    /// Stage 12-7 (Issue #14 脈: 鏡像等価性検証): `delete_node_predecessor` の
    /// 全域鏡像 (`lson` ⇔ `rson` を機械的に総入れ替えしたもの)。両子ケースは
    /// in-order **後継者** (右部分木の最左子孫) を昇格させる。単一子ケースの
    /// 昇格方向も対称に反転する (`DelMode::Successor`)。
    fn delete_node_successor(&mut self, p: i32) {
        if self.dad[p as usize] == NIL {
            return; // not in tree
        }
        let q: i32;
        if self.lson[p as usize] == NIL {
            q = self.rson[p as usize];
        } else if self.rson[p as usize] == NIL {
            q = self.lson[p as usize];
        } else {
            // 両子。rson[p] の最左子孫 q を見つけて p と挿げ替える
            let mut qv = self.rson[p as usize];
            if self.lson[qv as usize] != NIL {
                let mut guard = 0u32;
                loop {
                    qv = self.lson[qv as usize];
                    if self.lson[qv as usize] == NIL {
                        break;
                    }
                    guard += 1;
                    if guard > 2 * N as u32 {
                        eprintln!(
                            "WARN delete_node_successor: descent guard tripped (p={}, guard={}) — treating as cycle, breaking out",
                            p, guard
                        );
                        break;
                    }
                }
                let dad_q = self.dad[qv as usize];
                self.lson[dad_q as usize] = self.rson[qv as usize];
                let rq = self.rson[qv as usize];
                self.dad[rq as usize] = dad_q;
                self.rson[qv as usize] = self.rson[p as usize];
                let rp = self.rson[p as usize];
                self.dad[rp as usize] = qv;
            }
            self.lson[qv as usize] = self.lson[p as usize];
            let lp = self.lson[p as usize];
            self.dad[lp as usize] = qv;
            q = qv;
        }

        // Stage 12-9 バグ修正: この最終リンク付け替えは「p が自分の親から見て
        // lson 側の子か rson 側の子か」という p 自身の位置の話であり、
        // Predecessor/Successor どちらの昇格方式を使うかとは無関係 (この行は
        // 鏡像対象ではない)。dad_p がルート疑似ノード (N+1..=N+256) のとき、
        // Standard モードでは lson[root] が init_tree で初期化されず (常に
        // 配列既定値 0 のまま)、`p==0` だと `lson[dad_p]==p` が偽陽性で成立し
        // 誤った枝に書き込んでしまう (rson[root] が p を指したまま残り、
        // 後で木が循環する原因になっていた)。原典 delete_node_predecessor と
        // 同じ「rson を先にチェック」の順序に統一する。
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

/// `OkumuraSim` の初期化バリアント (Issue #14 v12)。
///
/// - `Basic`: 奥村原典どおり init_tree 後、先頭で F 個の dummy を InsertNode(r-F..r-1) 挿入
/// - `NoDummy`: dummy 挿入なし（`compress_okumura_no_dummy` の初期化）
/// - `DummyThenDrop`: dummy 挿入 → token 0 出力直後に残存 dummy を全 DeleteNode
///   （`compress_okumura_dummy_then_drop` の処理）
/// - `LeftFirst`: Basic と同じ初期化 + `BstMode::LeftFirst`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimMode {
    Basic,
    NoDummy,
    DummyThenDrop,
    LeftFirst,
    /// Stage 12-4 (Issue #14 脈3 再解釈): 「挿入タイミング=書込み時」仮説。
    /// 原典の dummy F 個挿入は行わず、代わりに初期先読み充填 18 バイト分
    /// (`r_init..r_init+F-1` = [4078,4095]、実際に「書き込まれた」データ)
    /// を開始時に**昇順** (4078→4095) で全て挿入する。session844 の物証
    /// (token 3-4 で 4092/4093 が候補に見える = 奥村の消費時挿入では原理的に
    /// 不可能) を説明しうる構造。
    WriteTimeAscending,
    /// 同上、挿入順を**降順** (4095→4078) にした亜種 (挿入順で木の形が
    /// 変わりうるため、順序を軸として分離検証する)。
    WriteTimeDescending,
    /// `WriteTimeAscending` に加え、原典の dummy F 個 (`r-F..r-1` = [4060,4077])
    /// も先に挿入したまま残す亜種 (dummy → 実データ18個・昇順の順)。
    WriteTimeAscendingKeepDummy,
    /// `WriteTimeDescending` + dummy 保持版 (dummy → 実データ18個・降順の順)。
    WriteTimeDescendingKeepDummy,
    /// Stage 12-6 (Issue #14 脈: signed char 比較仮説)。木構造は原典 (Basic)
    /// と同一 (dummy F 個挿入 + 通常の per-byte insert)、ノード内比較だけ
    /// `CmpMode::Signed` に差し替える。
    SignedCmp,
    /// 対照用: ノード内比較を全反転 (`CmpMode::Reversed`)。木構造は Basic と同一。
    ReversedCmp,
    /// `SignedCmp` + `WriteTimeDescending` の併用形 (直交する2軸なので両立を確認)。
    SignedCmpWriteTimeDescending,
    /// Stage 12-7 (Issue #14 脈: 削除昇格側)。木構造・比較は Basic と同一、
    /// `delete_node` の両子ケースのみ `DelMode::Successor` (右部分木の最左
    /// 子孫=in-order後継者を昇格) に差し替える。
    DelSuccessor,
    /// 鏡像等価性サニティ用: `ReversedCmp` (`CmpMode::Reversed`) +
    /// `DelSuccessor` の併用形。完全鏡像閉包の仮説が正しければ
    /// `Basic`(=Unsigned cmp + Predecessor del) と token 列が一致するはず。
    ReversedCmpDelSuccessor,
    /// `DelSuccessor` + `WriteTimeDescending` の併用形 (直交確認、余力があれば)。
    DelSuccessorWriteTimeDescending,
    /// Stage 12-11 (Issue #14 脈: 「腐った木」仮説) RotA。木構造・比較・
    /// 削除昇格側は Basic (Predecessor) と同一初期化だが、`rot_no_delete=true`
    /// にして消費時の `delete_node(s)` を一切呼ばない。ノードは自分の位置が
    /// 次に `insert_node` で再挿入されるまで古い鍵のまま木に残留する。
    RotANoDelete,
    /// RotA に加え、full-F 一致時のノード置換 (swap-with-r) も省略する
    /// (`BstMode::NoSwap` を併用)。
    RotBNoDeleteNoReplace,
}

/// Leaf の実トークン列で BST 状態を teacher-forcing 進行させるシミュレータ
/// (Issue #14 v12: BST 完全状態シミュレーション特徴量)。
///
/// 自エンコーダの選択では進めず、`advance` に渡された Leaf 実出力バイト数だけ
/// 原典 `Encode()` 後半ループと同一の回転 (DeleteNode(s) → text_buf 書込 →
/// InsertNode(r)) を行う。tie token の直前に `search_trace` を呼ぶと、
/// その時点の BST を read-only で辿り、max_len に到達する各ノードの
/// 訪問順位 (rank) と深さ (depth) を返す。
///
/// 注: 先読み (text_buf への入力供給) はシミュレータが内部で行う必要があるため、
/// Issue 仕様の `new(mode)` に加えて入力スライスを受け取る。
pub struct OkumuraSim<'a> {
    inner: Okumura,
    /// ring write head。v8/v11 の ring ループの r と全 token で一致する。
    pub r: i32,
    s: i32,
    /// 残フレーム長 (原典 Encode() の len)
    len: usize,
    input: &'a [u8],
    input_idx: usize,
    mode: SimMode,
    dummy_positions: Vec<i32>,
    first_token_done: bool,
    /// Stage 12-4 `WriteTime*` 系専用: 初期バッチ挿入で既にカバー済みの
    /// 位置に対応する、`advance` 内の per-byte `insert_node(r)` 呼び出しを
    /// 何回スキップするか (F-1 = 17 から開始し、消費するたびに減る)。
    skip_inserts: usize,
}

impl<'a> OkumuraSim<'a> {
    pub fn new(mode: SimMode, input: &'a [u8]) -> Self {
        let mut st = Okumura::new(0x20);
        st.tie_mode = TieMode::StrictGt;
        if matches!(mode, SimMode::LeftFirst) {
            // init_tree が LeftFirst のとき lson root も初期化するため、先に設定する
            st.bst_mode = BstMode::LeftFirst;
        }
        // Stage 12-6: cmp_mode は最初の insert_node より前に設定する必要がある。
        st.cmp_mode = match mode {
            SimMode::SignedCmp | SimMode::SignedCmpWriteTimeDescending => CmpMode::Signed,
            SimMode::ReversedCmp | SimMode::ReversedCmpDelSuccessor => CmpMode::Reversed,
            _ => CmpMode::Unsigned,
        };
        // Stage 12-7: del_mode は delete_node (advance 側) でのみ参照するが、
        // 一貫性のためここで設定する。
        st.del_mode = match mode {
            SimMode::DelSuccessor | SimMode::ReversedCmpDelSuccessor | SimMode::DelSuccessorWriteTimeDescending => {
                DelMode::Successor
            }
            _ => DelMode::Predecessor,
        };
        // Stage 12-11: 「腐った木」仮説。RotB はさらに full-F 一致時の
        // ノード置換 (swap-with-r) も省略する (`BstMode::NoSwap` を併用)。
        st.rot_no_delete = matches!(mode, SimMode::RotANoDelete | SimMode::RotBNoDeleteNoReplace);
        if matches!(mode, SimMode::RotBNoDeleteNoReplace) {
            st.bst_mode = BstMode::NoSwap;
        }
        st.init_tree();

        let r: i32 = (N - F) as i32;
        let s: i32 = 0;

        // 入力を F バイトまで text_buf[r..] に先読み（原典 Encode() と同一）
        let mut input_idx: usize = 0;
        let mut len: usize = 0;
        while len < F && input_idx < input.len() {
            st.text_buf[r as usize + len] = input[input_idx];
            input_idx += 1;
            len += 1;
        }

        let mut dummy_positions: Vec<i32> = Vec::new();
        let mut skip_inserts: usize = 0;
        if len > 0 {
            match mode {
                SimMode::Basic
                | SimMode::LeftFirst
                | SimMode::SignedCmp
                | SimMode::ReversedCmp
                | SimMode::DelSuccessor
                | SimMode::ReversedCmpDelSuccessor
                | SimMode::RotANoDelete
                | SimMode::RotBNoDeleteNoReplace => {
                    // 原典 for (i = 1; i <= F; i++) InsertNode(r - i)
                    // (比較/削除昇格側だけが変わる variant は木構造・挿入タイミング
                    // は Basic と同一)
                    for i in 1..=F {
                        st.insert_node(r - i as i32);
                    }
                    st.insert_node(r);
                }
                SimMode::NoDummy => {
                    st.insert_node(r);
                }
                SimMode::DummyThenDrop => {
                    for i in 1..=F {
                        let p = ((r - i as i32) + N as i32) & (N as i32 - 1);
                        st.insert_node(p);
                        dummy_positions.push(p);
                    }
                    st.insert_node(r);
                }
                SimMode::WriteTimeAscending | SimMode::WriteTimeAscendingKeepDummy => {
                    if matches!(mode, SimMode::WriteTimeAscendingKeepDummy) {
                        for i in 1..=F {
                            st.insert_node(r - i as i32);
                        }
                    }
                    // 初期先読み充填 [r, r+F-1] = [4078,4095] を「書込み時挿入」
                    // 原則で昇順に全て挿入する (r 自身も含めて F 個)。
                    for k in 0..F as i32 {
                        st.insert_node(r + k);
                    }
                    // 通常ループの per-byte insert_node は F-1 回分だけ重複するので
                    // (r 自身の1回は本挿入と同じ、残り F-1 個は per-byte ループが
                    // 本来 r++ のたびに呼ぶはずだった分)、advance 側で F-1 回スキップする。
                    skip_inserts = F - 1;
                }
                SimMode::WriteTimeDescending
                | SimMode::WriteTimeDescendingKeepDummy
                | SimMode::SignedCmpWriteTimeDescending
                | SimMode::DelSuccessorWriteTimeDescending => {
                    if matches!(mode, SimMode::WriteTimeDescendingKeepDummy) {
                        for i in 1..=F {
                            st.insert_node(r - i as i32);
                        }
                    }
                    for k in (0..F as i32).rev() {
                        st.insert_node(r + k);
                    }
                    skip_inserts = F - 1;
                }
            }
        }

        Self {
            inner: st,
            r,
            s,
            len,
            input,
            input_idx,
            mode,
            dummy_positions,
            first_token_done: false,
            skip_inserts,
        }
    }

    /// 現在の ring 削除ヘッド `s` を返す (Stage 12-12: none-of-6 profiling で
    /// insert/delete イベントのタイミングを外部から追跡するための読み取り専用アクセサ)。
    pub fn s(&self) -> i32 {
        self.s
    }

    /// 指定ノードの親 (dad[pos]) を返す (Stage 12-13: diverge ノード精査用の
    /// 読み取り専用アクセサ)。root 疑似ノード (N+1..=N+256) の親は NIL。
    pub fn dad_of(&self, pos: i32) -> i32 {
        self.inner.dad[pos as usize]
    }

    /// 指定ノードの左子を返す (読み取り専用)。
    pub fn lson_of(&self, pos: i32) -> i32 {
        self.inner.lson[pos as usize]
    }

    /// 指定ノードの右子を返す (読み取り専用)。
    pub fn rson_of(&self, pos: i32) -> i32 {
        self.inner.rson[pos as usize]
    }

    /// `text_buf[pos..pos+len]` を読み取り専用でコピーして返す (Stage 12-13:
    /// diverge ノードでの実バイト比較・挿入時内容スナップショット取得用)。
    /// `pos` は 0..N の実位置、`len` は通常 F (18)。
    pub fn text_window(&self, pos: i32, len: usize) -> Vec<u8> {
        let start = pos as usize;
        self.inner.text_buf[start..start + len].to_vec()
    }

    /// Stage 12-14 (Issue #14 脈1 Prong A): `delete_node_predecessor` の両子
    /// ケースで昇格したノード位置のログを drain して返す (呼び出し側で
    /// advance() 1回ごとに回収し、外部の履歴特徴 (promotion_count /
    /// last_promotion_event) を組み立てる用途)。木構造・挙動には影響しない。
    pub fn take_promotion_log(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.inner.promotion_log)
    }

    /// Stage 12-14: `insert_node` の EQ (F バイト完全一致) 置換で追い出された
    /// 旧ノード位置のログを drain して返す。
    pub fn take_replace_log(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.inner.replace_log)
    }

    /// tie token 直前に呼ぶ read-only トレース。木を一切 mutate しない。
    ///
    /// `insert_node` の探索経路 (KeyMode::Byte0 の root key、index 1 からの
    /// cmp 計算、BstMode ごとの左右規則) を逐語一致で辿り、一致長がちょうど
    /// `max_len` になるノードを訪問順に `(pos, rank, depth)` で返す。
    /// rank は 1 始まりの訪問順位、depth は root からの段数。
    ///
    /// 原典 insert_node は len == F で探索を打ち切るが、本トレースは
    /// 全 max_len 候補の rank を得るために NIL まで続行する
    /// (cmp == 0 のまま右へ降りる。最初の max_len ノード = rank 1 は原典の
    /// 採用ノードと一致する)。
    pub fn search_trace(&self, r: i32, max_len: u8) -> Vec<(u16, u32, u8)> {
        let mut results: Vec<(u16, u32, u8)> = Vec::new();
        if self.len == 0 {
            return results;
        }
        let key_start = r as usize;
        // KeyMode::Byte0 固定
        let root_byte = self.inner.text_buf[key_start];
        let mut i: i32 = N as i32 + 1 + root_byte as i32;
        let mut cmp: i32 = match self.inner.bst_mode {
            BstMode::LeftFirst => -1,
            _ => 1,
        };
        let mut visit: u32 = 0;
        let mut depth: u8 = 0;
        let mut guard: u32 = 0;

        loop {
            guard += 1;
            if guard > 4 * N as u32 {
                eprintln!("WARN search_trace: traversal guard tripped (r={}, guard={}) — likely a tree cycle (Stage 12-8 known DelSuccessor bug), aborting trace early", r, guard);
                break;
            }
            let go_right = match self.inner.bst_mode {
                BstMode::LeftFirst => cmp > 0,
                _ => cmp >= 0,
            };
            i = if go_right {
                self.inner.rson[i as usize]
            } else {
                self.inner.lson[i as usize]
            };
            if i == NIL {
                break;
            }
            depth = depth.saturating_add(1);

            // insert_node と同一: index 1 から最初の不一致 byte までが一致長
            let mut j: usize = 1;
            cmp = 0;
            while j < F {
                let a = self.inner.cmp_val(self.inner.text_buf[key_start + j]);
                let b = self.inner.cmp_val(self.inner.text_buf[i as usize + j]);
                let d = a - b;
                if d != 0 {
                    cmp = d;
                    break;
                }
                j += 1;
            }

            if j as u8 == max_len {
                visit += 1;
                results.push((i as u16, visit, depth));
            }
        }
        results
    }

    /// token 確定後に呼ぶ。原典 Encode() 後半ループと同一の回転を
    /// `emitted_bytes.len()` (= last_match_length) 回行う。
    /// Literal は 1 byte、Match は len bytes を渡す (teacher forcing)。
    pub fn advance(&mut self, emitted_bytes: &[u8]) {
        if self.len == 0 {
            return;
        }
        // teacher forcing 検証: 出力バイトは現在の coding position の
        // 先読み内容 text_buf[r..] と一致しているはず
        #[cfg(debug_assertions)]
        {
            let check = emitted_bytes.len().min(self.len);
            for (k, &b) in emitted_bytes.iter().take(check).enumerate() {
                debug_assert_eq!(
                    b,
                    self.inner.text_buf[self.r as usize + k],
                    "OkumuraSim::advance: emitted byte {} != lookahead (mode {:?})",
                    k,
                    self.mode
                );
            }
        }

        let last_match_length = emitted_bytes.len();
        let mut i = 0usize;
        while i < last_match_length && self.input_idx < self.input.len() {
            // Stage 12-11: 「腐った木」仮説の rot_no_delete モードでは
            // delete_node(s) を一切呼ばない。
            if !self.inner.rot_no_delete {
                self.inner.delete_node(self.s);
            }
            let c = self.input[self.input_idx];
            self.input_idx += 1;

            self.inner.text_buf[self.s as usize] = c;
            if (self.s as usize) < F - 1 {
                self.inner.text_buf[self.s as usize + N] = c;
            }

            self.s = (self.s + 1) & (N as i32 - 1);
            self.r = (self.r + 1) & (N as i32 - 1);
            // Stage 12-4 WriteTime*: 初期バッチ挿入が既にカバー済みの位置は
            // ここで重複挿入しない (skip_inserts は他モードでは常に 0 で no-op)。
            if self.skip_inserts > 0 {
                self.skip_inserts -= 1;
            } else {
                self.inner.insert_node(self.r);
            }
            i += 1;
        }

        while i < last_match_length {
            if !self.inner.rot_no_delete {
                self.inner.delete_node(self.s);
            }
            self.s = (self.s + 1) & (N as i32 - 1);
            self.r = (self.r + 1) & (N as i32 - 1);
            self.len -= 1;
            if self.len > 0 {
                if self.skip_inserts > 0 {
                    self.skip_inserts -= 1;
                } else {
                    self.inner.insert_node(self.r);
                }
            }
            i += 1;
        }

        // DummyThenDrop: token 0 の回転が終わった直後に残存 dummy を全削除
        // (compress_okumura_dummy_then_drop と同一。r 自身は削除しない)
        if matches!(self.mode, SimMode::DummyThenDrop) && !self.first_token_done {
            self.first_token_done = true;
            for k in 0..self.dummy_positions.len() {
                let p = self.dummy_positions[k];
                if p == self.r {
                    continue;
                }
                self.inner.delete_node(p);
            }
        }
    }

    /// 木全体の read-only 全走査 (Issue #14 Stage 1)。
    ///
    /// `search_trace` と独立に、256 root の lson/rson を辿って到達可能な
    /// 全ノードを列挙し、各ノードについて現在の coding position `r` の
    /// 先読み key `text_buf[r..r+F]` との一致長 (byte 0 から最初の不一致まで、
    /// 上限 F) と root からの深さを返す。木は一切 mutate しない。
    ///
    /// 返り値: `(pos, match_len, depth)` の Vec。列挙順は root 昇順 ×
    /// 各 root 内は in-order (左→自分→右)。depth は root 直下の子 = 1。
    /// Standard 系は rson[root] のみ、LeftFirst は lson[root] も走査する
    /// (`tree_is_consistent` と同じ規則)。
    pub fn tree_scan(&self, r: i32) -> Vec<(u16, u8, u8)> {
        let mut out: Vec<(u16, u8, u8)> = Vec::new();
        if self.len == 0 {
            return out;
        }
        let key = r as usize;
        for root in (N + 1)..=(N + 256) {
            let mut starts: Vec<i32> = Vec::new();
            if matches!(self.inner.bst_mode, BstMode::LeftFirst) && self.inner.lson[root] != NIL {
                starts.push(self.inner.lson[root]);
            }
            if self.inner.rson[root] != NIL {
                starts.push(self.inner.rson[root]);
            }
            for start in starts {
                // 反復 in-order。stack には (node, depth) を積む
                let mut stack: Vec<(i32, u8)> = Vec::new();
                let mut cur = start;
                let mut d: u8 = 1;
                while cur != NIL || !stack.is_empty() {
                    while cur != NIL {
                        stack.push((cur, d));
                        cur = self.inner.lson[cur as usize];
                        d = d.saturating_add(1);
                    }
                    let (node, nd) = stack.pop().unwrap();
                    let mut ml: u8 = 0;
                    for j in 0..F {
                        if self.inner.text_buf[key + j] != self.inner.text_buf[node as usize + j] {
                            break;
                        }
                        ml += 1;
                    }
                    out.push((node as u16, ml, nd));
                    cur = self.inner.rson[node as usize];
                    d = nd.saturating_add(1);
                }
            }
        }
        out
    }

    /// `search_trace` と同一規則で root から NIL まで降りた探索経路を返す
    /// (Issue #14 Stage 1)。各要素は `(node, went_right)`。先頭要素は
    /// root インデックス (N+1+byte0) 自身で、`went_right` はそのノードで
    /// 次にどちらの子へ降りたか。read-only。
    /// (`classify_off_path` の内部用。外部公開の要件が出るまで private)
    fn search_path(&self, r: i32) -> Vec<(i32, bool)> {
        let mut path: Vec<(i32, bool)> = Vec::new();
        if self.len == 0 {
            return path;
        }
        let key_start = r as usize;
        let root_byte = self.inner.text_buf[key_start];
        let mut i: i32 = N as i32 + 1 + root_byte as i32;
        let mut cmp: i32 = match self.inner.bst_mode {
            BstMode::LeftFirst => -1,
            _ => 1,
        };
        loop {
            let go_right = match self.inner.bst_mode {
                BstMode::LeftFirst => cmp > 0,
                _ => cmp >= 0,
            };
            path.push((i, go_right));
            i = if go_right {
                self.inner.rson[i as usize]
            } else {
                self.inner.lson[i as usize]
            };
            if i == NIL {
                break;
            }
            let mut j: usize = 1;
            cmp = 0;
            while j < F {
                let a = self.inner.cmp_val(self.inner.text_buf[key_start + j]);
                let b = self.inner.cmp_val(self.inner.text_buf[i as usize + j]);
                let d = a - b;
                if d != 0 {
                    cmp = d;
                    break;
                }
                j += 1;
            }
        }
        path
    }

    /// 指定 pos が `search_path(r)` の経路外になった理由を分類する
    /// (Issue #14 Stage 1)。返り値は `(code, diverge_depth)`:
    ///
    /// - 0: 探索経路上にある (rank が付くはずのノード)
    /// - 1: 木に不在 (dad 連鎖が root に到達しない)
    /// - 2: 探索 key と root byte が異なる (byte0 不一致)
    /// - 3: 分岐ノードで探索は左へ、pos は右部分木
    /// - 4: 分岐ノードで探索は右へ、pos は左部分木
    ///
    /// `diverge_depth` は分岐ノードの root からの深さ (root 自身 = 0)。
    /// code 0/1/2 では 255。
    pub fn classify_off_path(&self, r: i32, pos: u16) -> (u8, u8) {
        let p = pos as i32;
        // dad 連鎖で root まで遡る (root インデックスは N+1..=N+256)
        let mut chain: Vec<i32> = vec![p];
        let mut cur = p;
        let mut steps = 0usize;
        loop {
            let d = self.inner.dad[cur as usize];
            if d == NIL {
                return (1, 255); // 木に不在
            }
            chain.push(d);
            if d as usize > N {
                break; // root に到達
            }
            cur = d;
            steps += 1;
            if steps > N {
                return (1, 255); // 循環ガード (壊れた木)
            }
        }
        chain.reverse(); // root → ... → pos

        let path = self.search_path(r);
        if path.is_empty() {
            return (1, 255);
        }
        if path[0].0 != chain[0] {
            return (2, 255); // root byte 不一致
        }

        // root から下り、探索経路と祖先連鎖の最深共通ノードを探す
        let mut k = 0usize;
        while k + 1 < chain.len() && k < path.len() && path[k].0 == chain[k] {
            let went_right = path[k].1;
            let node_right = self.inner.rson[chain[k] as usize] == chain[k + 1];
            if went_right != node_right {
                return (if went_right { 4 } else { 3 }, k as u8);
            }
            k += 1;
        }
        if k + 1 >= chain.len() {
            return (0, 255); // pos 自身が探索経路上
        }
        // ここには来ないはず (共通ノードで必ず分岐が検出される) が、安全側
        (1, 255)
    }

    /// BST の親子リンク整合を検証する（テスト用）。
    /// dad != NIL の全ノードについて「親の lson か rson が自分を指す」ことと、
    /// 各 root からの到達ノードに循環が無いことを確認する。
    pub fn tree_is_consistent(&self) -> bool {
        // 親子リンクの相互整合
        for pos in 0..N {
            let d = self.inner.dad[pos];
            if d == NIL {
                continue;
            }
            let du = d as usize;
            if !(du < N || ((N + 1)..=(N + 256)).contains(&du)) {
                return false;
            }
            if self.inner.lson[du] != pos as i32 && self.inner.rson[du] != pos as i32 {
                return false;
            }
        }
        // root から辿ってノード数が N を超えたら循環
        let mut reached = 0usize;
        let mut stack: Vec<i32> = Vec::new();
        for root in (N + 1)..=(N + 256) {
            if self.inner.rson[root] != NIL {
                stack.push(self.inner.rson[root]);
            }
            if matches!(self.inner.bst_mode, BstMode::LeftFirst) && self.inner.lson[root] != NIL {
                stack.push(self.inner.lson[root]);
            }
        }
        while let Some(p) = stack.pop() {
            reached += 1;
            if reached > N {
                return false;
            }
            let pu = p as usize;
            if self.inner.lson[pu] != NIL {
                stack.push(self.inner.lson[pu]);
            }
            if self.inner.rson[pu] != NIL {
                stack.push(self.inner.rson[pu]);
            }
        }
        true
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
        if allow_equal {
            TieMode::AllowEq
        } else {
            TieMode::StrictGt
        },
    )
}

/// 距離タイブレイク版。同一長のとき `r` に近い (back distance が小さい) 候補を採用。
pub fn compress_okumura_distance_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl(input, TieMode::DistanceTie)
}

/// 反転距離タイブレイク版。同一長のとき r からより遠い候補を採用 (= max dist)。
/// 既存奥村は BST 内部順に頼った tie だが、明示 max-dist 選択を行う。
pub fn compress_okumura_max_dist_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl(input, TieMode::MaxDistTie)
}

/// **Brute-force longest match search**, NOT BST. At each step scans every ring
/// position 0..N to find the maximum match length L. Tie-break:
/// - `BruteTie::MaxDist`: among all positions at max_len, pick the one with largest
///   (r - pos) & mask (= farthest back in ring).
/// - `BruteTie::MinDist`: pick smallest distance.
///
/// session 389+ bulk_stats finding: 48% of "hopeless" tokens have only ONE
/// brute-force max-len candidate (= our BST simply misses the longest match).
/// Among 2-tied, leaf picks max-dist 75% of the time. H/V/S file groups show
/// 75–82% max-dist preference. Hypothesis: leaf encoder uses brute-force search
/// + max-dist tie-break for these file groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BruteTie {
    MaxDist,
    MinDist,
}

fn compress_okumura_brute_impl(input: &[u8], tie: BruteTie) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mask: usize = N - 1;
    let imask: i32 = (N as i32) - 1;

    while s < input.len() {
        let remaining = input.len() - s;
        let max_len_by_input = remaining.min(F);

        // Brute-force scan: for each pos, walk match length.
        // Optimized: avoid 4KB ring copy per pos. Read directly from ring; for
        // self-referential matches (pos within F-1 bytes back from r), the
        // write-back means we read what's CURRENTLY in ring (uninitialised at r+),
        // but since the algorithm extends one byte at a time AND each "would-be
        // write" of byte k overlaps ring[(r+k) & mask] which is referenced as
        // ring[(pos+k) & mask] for k < dist, the simple ring read is correct
        // for non-overlapping cases (dist >= len). For dist < len (self-ref),
        // the source bytes after dist wraps must equal input[s..s+dist] copies.
        // To handle this we use the "writeback" model only when needed.
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;
        let mut best_dist: i32 = 0;

        for pos in 0..N {
            let dist = ((r as i32 - pos as i32) & imask) as usize;
            if dist == 0 {
                continue;
            }

            let mut l = 0usize;
            if dist >= max_len_by_input {
                // Non-overlapping case: simple compare from ring[pos..pos+L] vs input[s..s+L]
                while l < max_len_by_input {
                    let rb = ring[(pos + l) & mask];
                    if rb != input[s + l] {
                        break;
                    }
                    l += 1;
                }
            } else {
                // Self-referential (RLE-like): bytes after dist would be writeback'd.
                // The pattern: extension byte k (k < dist) reads ring[(pos+k) & mask];
                // for k >= dist, it reads what was just written at ring[(r + k - dist) & mask],
                // which equals input[s + k - dist] (since that's what gets written).
                // We compare incrementally.
                while l < max_len_by_input {
                    let rb = if l < dist {
                        ring[(pos + l) & mask]
                    } else {
                        input[s + l - dist]
                    };
                    if rb != input[s + l] {
                        break;
                    }
                    l += 1;
                }
            }

            if l >= 3 {
                if l > best_len {
                    best_len = l;
                    best_pos = pos;
                    best_dist = dist as i32;
                } else if l == best_len {
                    let take = match tie {
                        BruteTie::MaxDist => (dist as i32) > best_dist,
                        BruteTie::MinDist => (dist as i32) < best_dist,
                    };
                    if take {
                        best_pos = pos;
                        best_dist = dist as i32;
                    }
                }
            }
        }

        if best_len < 3 {
            let b = input[s];
            out.push(Token::Literal(b));
            ring[r] = b;
            r = (r + 1) & mask;
            s += 1;
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            for k in 0..best_len {
                let b = input[s + k];
                ring[r] = b;
                r = (r + 1) & mask;
            }
            s += best_len;
        }
    }

    out
}

/// Brute-force longest match + max-distance tie.
pub fn compress_okumura_brute_max_dist(input: &[u8]) -> Vec<Token> {
    compress_okumura_brute_impl(input, BruteTie::MaxDist)
}

/// Brute-force longest match + min-distance tie.
pub fn compress_okumura_brute_min_dist(input: &[u8]) -> Vec<Token> {
    compress_okumura_brute_impl(input, BruteTie::MinDist)
}

/// **Hash-chain limited search**. Hypothesis: leaf encoder uses a recency-ordered
/// chain (head + prev) keyed on a short prefix and walks the chain up to K times,
/// picking the longest match found (with a specific tie-break). This explains
/// the "lazy / non-greedy" observation in session 390 (C1301 t767:
/// no_dummy found Match(0x3ec, 9) but leaf picked Match(0xf5a, 3)).
///
/// Implementation:
/// - hash function: `hash(input[s], input[s+1])` → 16 bits (256×256 buckets, falls back to byte0 only)
/// - head[hash] = most recent pos with this hash; prev[pos] = earlier pos with same hash
/// - From head[hash(input[s..s+2])], walk prev up to K times; compute match
///   length at each visited pos; track longest with chosen tie-break.
/// - On emit, insert each written position into the chain (head/prev update).
///
/// Tie-break:
/// - `FirstLongest`: among chain-walked visits, pick the first (most recent) position
///   that achieves the longest length found so far. (Aggressive: as soon as you
///   see a position with len_so_far_max, keep it; don't overwrite with later one
///   even if it ties.)
/// - `LastLongest`: keep updating to most-recent equal-length (max-dist among visited
///   since chain walks recent → old, "last" = oldest among visited)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainTie {
    FirstLongest, // first hit at max_len kept
    LastLongest,  // last hit at max_len kept (likely oldest in chain)
}

fn hash16(a: u8, b: u8) -> usize {
    ((a as usize) << 8) | (b as usize)
}

fn compress_okumura_chain_impl(input: &[u8], max_chain: usize, tie: ChainTie) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mask: usize = N - 1;
    let imask: i32 = (N as i32) - 1;

    // Hash table: head + prev chain
    let n_buckets = 256 * 256;
    let nil_pos: u16 = 0xffff;
    let mut head: Vec<u16> = vec![nil_pos; n_buckets];
    let mut prev_pos: Vec<u16> = vec![nil_pos; N];

    // Helper: insert position `p` into hash chain based on ring[p..p+2]
    let insert_pos = |head: &mut Vec<u16>, prev_pos: &mut Vec<u16>, ring: &[u8; N], p: usize| {
        let h = hash16(ring[p], ring[(p + 1) & 0x0fff]);
        prev_pos[p] = head[h];
        head[h] = p as u16;
    };

    // Pre-fill: write initial 0x20 fill positions to chain (positions 0..N-F).
    // Actually skip pre-filling; the initial 0x20 ring is filled by Okumura too.
    // We'll insert positions as we write.

    while s < input.len() {
        let remaining = input.len() - s;
        let max_len_by_input = remaining.min(F);

        // Search chain
        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;
        let mut best_dist: i32 = 0;

        if max_len_by_input >= 3 && s + 1 < input.len() {
            let h = hash16(input[s], input[s + 1]);
            let mut p = head[h];
            let mut walked = 0usize;
            while p != nil_pos && walked < max_chain {
                let pos = p as usize;
                let dist = ((r as i32 - pos as i32) & imask) as usize;
                if dist > 0 {
                    // Compute match length with writeback simulation
                    let mut l = 0usize;
                    while l < max_len_by_input {
                        let rb = if l < dist {
                            ring[(pos + l) & mask]
                        } else {
                            input[s + l - dist]
                        };
                        if rb != input[s + l] {
                            break;
                        }
                        l += 1;
                    }
                    if l >= 3 {
                        if l > best_len {
                            best_len = l;
                            best_pos = pos;
                            best_dist = dist as i32;
                        } else if l == best_len {
                            let take = match tie {
                                ChainTie::FirstLongest => false, // keep first (already set)
                                ChainTie::LastLongest => true,   // overwrite with later
                            };
                            if take {
                                best_pos = pos;
                                best_dist = dist as i32;
                            }
                        }
                    }
                }
                p = prev_pos[pos];
                walked += 1;
            }
        }
        let _ = best_dist;

        if best_len < 3 {
            // Literal
            let b = input[s];
            // Insert current r into chain (we're about to write here)
            // Actually, insert after writing so ring is updated.
            ring[r] = b;
            // Insert r-1's hash now that ring[r-1] is set (well, requires ring[r-1] and ring[r] both)
            // For simplicity, insert position when we have 2 bytes available there.
            // We insert ring position r when ring[r] AND ring[r+1] are both written.
            // Since we write 1 byte here, insert position r-1 (which now has its 2nd byte = ring[r]).
            let p_to_insert = (r + N - 1) & mask;
            insert_pos(&mut head, &mut prev_pos, &ring, p_to_insert);
            out.push(Token::Literal(b));
            r = (r + 1) & mask;
            s += 1;
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            for k in 0..best_len {
                let b = input[s + k];
                ring[r] = b;
                // After writing ring[r], position r-1 now has its second byte ring[r], so insert r-1.
                let p_to_insert = (r + N - 1) & mask;
                insert_pos(&mut head, &mut prev_pos, &ring, p_to_insert);
                r = (r + 1) & mask;
            }
            s += best_len;
        }
    }

    out
}

pub fn compress_okumura_chain4_max(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 4, ChainTie::LastLongest)
}
pub fn compress_okumura_chain8_max(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 8, ChainTie::LastLongest)
}
pub fn compress_okumura_chain16_max(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 16, ChainTie::LastLongest)
}
pub fn compress_okumura_chain32_max(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 32, ChainTie::LastLongest)
}
pub fn compress_okumura_chain64_max(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 64, ChainTie::LastLongest)
}
pub fn compress_okumura_chain4_first(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 4, ChainTie::FirstLongest)
}
pub fn compress_okumura_chain8_first(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 8, ChainTie::FirstLongest)
}
pub fn compress_okumura_chain16_first(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 16, ChainTie::FirstLongest)
}
pub fn compress_okumura_chain32_first(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_impl(input, 32, ChainTie::FirstLongest)
}

/// 3-byte hash chain (uses input[s], input[s+1], input[s+2] for 24-bit key).
fn hash24(a: u8, b: u8, c: u8) -> usize {
    ((a as usize) ^ ((b as usize) << 5) ^ ((c as usize) << 10)) & 0xffff
}

fn compress_okumura_chain3_impl(input: &[u8], max_chain: usize, tie: ChainTie) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mask: usize = N - 1;
    let imask: i32 = (N as i32) - 1;
    let nil_pos: u16 = 0xffff;
    let n_buckets = 65536;
    let mut head: Vec<u16> = vec![nil_pos; n_buckets];
    let mut prev_pos: Vec<u16> = vec![nil_pos; N];

    while s < input.len() {
        let remaining = input.len() - s;
        let max_len_by_input = remaining.min(F);

        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        if max_len_by_input >= 3 && s + 2 < input.len() {
            let h = hash24(input[s], input[s + 1], input[s + 2]);
            let mut p = head[h];
            let mut walked = 0usize;
            while p != nil_pos && walked < max_chain {
                let pos = p as usize;
                let dist = ((r as i32 - pos as i32) & imask) as usize;
                if dist > 0 {
                    let mut l = 0usize;
                    while l < max_len_by_input {
                        let rb = if l < dist {
                            ring[(pos + l) & mask]
                        } else {
                            input[s + l - dist]
                        };
                        if rb != input[s + l] {
                            break;
                        }
                        l += 1;
                    }
                    if l >= 3 {
                        if l > best_len {
                            best_len = l;
                            best_pos = pos;
                        } else if l == best_len && tie == ChainTie::LastLongest {
                            best_pos = pos;
                        }
                    }
                }
                p = prev_pos[pos];
                walked += 1;
            }
        }

        if best_len < 3 {
            let b = input[s];
            ring[r] = b;
            if (r + N - 2) < N + N {
                let p_to_insert = (r + N - 2) & mask;
                let h = hash24(
                    ring[p_to_insert],
                    ring[(p_to_insert + 1) & mask],
                    ring[(p_to_insert + 2) & mask],
                );
                prev_pos[p_to_insert] = head[h];
                head[h] = p_to_insert as u16;
            }
            out.push(Token::Literal(b));
            r = (r + 1) & mask;
            s += 1;
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            for k in 0..best_len {
                let b = input[s + k];
                ring[r] = b;
                let p_to_insert = (r + N - 2) & mask;
                let h = hash24(
                    ring[p_to_insert],
                    ring[(p_to_insert + 1) & mask],
                    ring[(p_to_insert + 2) & mask],
                );
                prev_pos[p_to_insert] = head[h];
                head[h] = p_to_insert as u16;
                r = (r + 1) & mask;
            }
            s += best_len;
        }
    }
    out
}

pub fn compress_okumura_chain3_8(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain3_impl(input, 8, ChainTie::LastLongest)
}
pub fn compress_okumura_chain3_32(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain3_impl(input, 32, ChainTie::LastLongest)
}
pub fn compress_okumura_chain3_first8(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain3_impl(input, 8, ChainTie::FirstLongest)
}

/// Reverse-order chain: head points to OLDEST entry, next points to NEWER. Walk
/// from head goes oldest → newer. With FirstLongest tie, picks oldest first.
fn compress_okumura_chain_rev_impl(input: &[u8], max_chain: usize) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let mask: usize = N - 1;
    let imask: i32 = (N as i32) - 1;
    let nil_pos: u16 = 0xffff;
    let n_buckets = 65536;
    let mut head: Vec<u16> = vec![nil_pos; n_buckets];
    let mut tail_v: Vec<u16> = vec![nil_pos; n_buckets]; // most-recent pos per bucket
    let mut next_pos: Vec<u16> = vec![nil_pos; N];

    while s < input.len() {
        let remaining = input.len() - s;
        let max_len_by_input = remaining.min(F);

        let mut best_len: usize = 0;
        let mut best_pos: usize = 0;

        if max_len_by_input >= 3 && s + 1 < input.len() {
            let h = hash16(input[s], input[s + 1]);
            let mut p = head[h];
            let mut walked = 0usize;
            while p != nil_pos && walked < max_chain {
                let pos = p as usize;
                let dist = ((r as i32 - pos as i32) & imask) as usize;
                if dist > 0 {
                    let mut l = 0usize;
                    while l < max_len_by_input {
                        let rb = if l < dist {
                            ring[(pos + l) & mask]
                        } else {
                            input[s + l - dist]
                        };
                        if rb != input[s + l] {
                            break;
                        }
                        l += 1;
                    }
                    if l >= 3 && l > best_len {
                        best_len = l;
                        best_pos = pos;
                    }
                }
                p = next_pos[pos];
                walked += 1;
            }
        }

        let insert = |head: &mut Vec<u16>,
                      tail: &mut Vec<u16>,
                      next: &mut Vec<u16>,
                      ring: &[u8; N],
                      p: usize| {
            let h = hash16(ring[p], ring[(p + 1) & 0x0fff]);
            // Append to end of chain
            let t = tail[h];
            if t == nil_pos {
                head[h] = p as u16;
            } else {
                next[t as usize] = p as u16;
            }
            next[p] = nil_pos;
            tail[h] = p as u16;
        };

        if best_len < 3 {
            let b = input[s];
            ring[r] = b;
            let p_to_insert = (r + N - 1) & mask;
            insert(&mut head, &mut tail_v, &mut next_pos, &ring, p_to_insert);
            out.push(Token::Literal(b));
            r = (r + 1) & mask;
            s += 1;
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            for k in 0..best_len {
                let b = input[s + k];
                ring[r] = b;
                let p_to_insert = (r + N - 1) & mask;
                insert(&mut head, &mut tail_v, &mut next_pos, &ring, p_to_insert);
                r = (r + 1) & mask;
            }
            s += best_len;
        }
    }

    out
}

pub fn compress_okumura_chain_rev8(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_rev_impl(input, 8)
}
pub fn compress_okumura_chain_rev32(input: &[u8]) -> Vec<Token> {
    compress_okumura_chain_rev_impl(input, 32)
}

/// basic_tail1 with parameterized initial fill byte.
/// 仮説: leaf encoder が 0x20 以外の fill 値で ring を初期化していた可能性。
/// 結果として BST が選ぶ位置が変わる (= 0x20 fill 領域への match を picture data の
/// match と区別できる)。decoder は 0x20 fill 想定で動くので、encoder が異なる fill
/// で内部処理しても出力 binary は decoder で正しく decode できる (= 未書込み位置への
/// match を回避できれば fill 値は何でもよい)。
fn compress_okumura_basic_tail1_fill_impl(input: &[u8], fill: u8) -> Vec<Token> {
    let mut st = Okumura::new(fill);
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
    loop {
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

pub fn compress_okumura_basic_tail1_fill00(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_tail1_fill_impl(input, 0x00)
}
pub fn compress_okumura_basic_tail1_fillff(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_tail1_fill_impl(input, 0xff)
}

/// 基本奥村 + 書き込み済み bitmap フィルタ。
/// match の pos が初期 0x20 fill 領域 (= 未書込み) なら Literal に格下げ。
///
/// 仮説: cc=48 ファイルの encoder は基本奥村 (StrictGt tie + Default BST + greedy) を
/// 使うが、未書込み ring 位置への match のみ拒否する。session 364 の C0205 token 161
/// 観察が動機。
pub fn compress_okumura_basic_no_init(input: &[u8]) -> Vec<Token> {
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

    // 書込み済み bitmap (true = 実データが書かれた位置)
    let mut written = [false; N];
    // 先読み済 F バイト = 書込み済
    // (M17 厳格版テスト後、permissive に戻す)
    for k in 0..F {
        written[((N - F + k) & (N - 1))] = true;
    }

    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    loop {
        if st.match_length as usize > len {
            st.match_length = len as i32;
        }
        let pos1 = (st.match_position & (N as i32 - 1)) as usize;
        let len1 = st.match_length as usize;

        // フィルタ: pos が未書込み AND 入力バイトが 0x20 (= initial fill 値) なら Literal 強制
        // 観察: leaf は「実質 0x20 列を 0x20 fill で match」する場合のみ拒否し、
        // 実データの match は context によらず accept する。
        let force_literal = if len1 > THRESHOLD && !written[pos1] {
            st.text_buf[r as usize] == 0x20
        } else {
            false
        };

        if force_literal || (st.match_length as usize) <= THRESHOLD {
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
            // この s 位置は実データで書き込まれた
            written[s as usize] = true;
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

/// basic_no_init + tail1 RLE +1 phantom 拡張。
/// no_init の未書込み bitmap フィルタ (input=0x20 のときのみ Literal 強制) と、
/// session 386 で +33 した tail1 phantom padding ルールを組合せる。
/// 仮説: leaf encoder は両方のルールを同時に持つ可能性がある。
pub fn compress_okumura_basic_no_init_tail1(input: &[u8]) -> Vec<Token> {
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
    let mut written = [false; N];
    for k in 0..F {
        written[(N - F + k) & (N - 1)] = true;
    }
    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }
        let pos1 = (st.match_position & mask) as usize;
        let len1 = st.match_length as usize;
        let force_literal = if len1 > THRESHOLD && !written[pos1] {
            st.text_buf[r as usize] == 0x20
        } else {
            false
        };
        if force_literal || (st.match_length as usize) <= THRESHOLD {
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
            written[s as usize] = true;
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// basic_tail1 の variant で、input_idx が input.len() に達した直後に main loop を抜ける。
/// 既存 basic_tail1 は len=0 まで window drain を続けて末尾 phantom token を出すが、
/// leaf encoder の中には input 消費だけで止めるものがある可能性 (C0601 oracle 解析根拠)。
pub fn compress_okumura_basic_tail1_stop_on_input(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
        // Stop as soon as input is exhausted. Don't drain window.
        if input_idx >= input.len() {
            break;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
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

/// basic_tail1 + 末尾に phantom Literal を追加 (text_buf[r] 値)。
/// C0601 解析: leaf は 10395 token 後に 1 phantom Literal (= 同値) を emit して
/// 8-token group の bit alignment を行うことが判明 (session 389 続編)。
pub fn compress_okumura_basic_tail1_phantom_lit(input: &[u8]) -> Vec<Token> {
    let mut out = compress_okumura_basic_tail1(input);
    // phantom = 最後の literal byte (last Literal token があれば)、なければ input 最終 byte
    if let Some(b) = input.last() {
        out.push(Token::Literal(*b));
    }
    out
}

/// no_dummy_tail1 + 末尾 phantom Literal。
pub fn compress_okumura_no_dummy_tail1_phantom_lit(input: &[u8]) -> Vec<Token> {
    let mut out = compress_okumura_no_dummy_tail1(input);
    if let Some(b) = input.last() {
        out.push(Token::Literal(*b));
    }
    out
}

/// basic_tail1 + 末尾 2 個の phantom Literal (input 最終 byte 重複)。
pub fn compress_okumura_basic_tail1_phantom_lit2(input: &[u8]) -> Vec<Token> {
    let mut out = compress_okumura_basic_tail1(input);
    if let Some(b) = input.last() {
        out.push(Token::Literal(*b));
        out.push(Token::Literal(*b));
    }
    out
}

/// basic_tail1 だが、最終 iter の cap 制約を外し BST が返す match_length を
/// そのまま使う (input 残量より長い Match も emit、decoder 側で truncate)。
/// C0182 解析: leaf=M(724,18) vs ours=M(724,17) で pos 一致 len 1 短い → cap=len
/// 制約が原因と判明。
pub fn compress_okumura_basic_tail1_no_cap(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        // RLE のみ +1 cap、それ以外は cap 無効 = BST が返す長さをそのまま使う
        // (ただし F=18 が上限)
        let cap = if is_rle { (len + 1).min(F) } else { F };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
        // Effective consumed length is min(last_match_length, len)
        let effective = last_match_length.min(len);
        let mut i = 0usize;
        while i < effective && input_idx < input.len() {
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
        while i < effective {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && effective > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// 同上 no_dummy ベース版。
pub fn compress_okumura_no_dummy_tail1_no_cap(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { F };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
        let effective = last_match_length.min(len);
        let mut i = 0usize;
        while i < effective && input_idx < input.len() {
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
        while i < effective {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && effective > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// basic_tail1 + 8-token group 完成までの phantom Literal padding。
/// 残り bit を全て Literal で埋める。
pub fn compress_okumura_basic_tail1_phantom_lit_pad8(input: &[u8]) -> Vec<Token> {
    let mut out = compress_okumura_basic_tail1(input);
    if let Some(b) = input.last() {
        let pad = (8 - (out.len() % 8)) % 8;
        for _ in 0..pad {
            out.push(Token::Literal(*b));
        }
    }
    out
}

/// 同上の no_dummy 版。
pub fn compress_okumura_no_dummy_tail1_stop_on_input(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
        if input_idx >= input.len() {
            break;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
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

/// no_init 厳格版 + tail1: unwritten pos への match は input byte に関係なく Lit 強制。
pub fn compress_okumura_basic_no_init_strict_tail1(input: &[u8]) -> Vec<Token> {
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
    let mut written = [false; N];
    for k in 0..F {
        written[(N - F + k) & (N - 1)] = true;
    }
    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }
        let pos1 = (st.match_position & mask) as usize;
        let len1 = st.match_length as usize;
        // 厳格版: input byte に関係なく unwritten pos なら Lit 強制
        let force_literal = len1 > THRESHOLD && !written[pos1];
        if force_literal || (st.match_length as usize) <= THRESHOLD {
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
            written[s as usize] = true;
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy + no_init bitmap + tail1 phantom padding。
pub fn compress_okumura_no_dummy_no_init_tail1(input: &[u8]) -> Vec<Token> {
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
    let mut written = [false; N];
    for k in 0..F {
        written[(N - F + k) & (N - 1)] = true;
    }
    st.insert_node(r);
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }
        let pos1 = (st.match_position & mask) as usize;
        let len1 = st.match_length as usize;
        let force_literal = if len1 > THRESHOLD && !written[pos1] {
            st.text_buf[r as usize] == 0x20
        } else {
            false
        };
        if force_literal || (st.match_length as usize) <= THRESHOLD {
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
            written[s as usize] = true;
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
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

/// no_dummy + RLE 限定 tail+1 phantom: match_position == r-1 のときだけ
/// 残窓 len を +1 まで超過させる。non-RLE は std no_dummy と同じ。
///
/// session 375 phantom padding finding を狭く適用 (Mode A の wins は全て pos=r-1)。
/// 広く+1 する full 版は 18 win / 29 regress = -11 で失敗、副作用を消す試行。
pub fn compress_okumura_no_dummy_tail1(input: &[u8]) -> Vec<Token> {
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
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }

        // tail+1 phantom extension: 残り input を全消費した上で +1 phantom を踏んだ場合、
        // window は完全に空 (len=0) で終了させる。標準の len 減算では len_before-1 で残ってしまう
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// 奥村原典 (basic, F dummy あり) + RLE tail+1 phantom。
/// no_dummy BST と異なる pos を返すケース (Mode A 14 ファイル) を狙う。
pub fn compress_okumura_basic_tail1(input: &[u8]) -> Vec<Token> {
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

    // 奥村原典: F-1 個の dummy (text_buf 全部 0x20 で位置 r-1..r-F に dummy node) + 本挿入
    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);

    loop {
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// 奥村原典 + 任意 pos の tail+1 phantom (RLE 限定なし)。
/// Mode A 16 非 RLE 拡張ファイル (TITLE2/C0508/C1701/C1E03/C1002 系) を狙う。
/// 副作用: 約 -29 regress 想定だが union には貢献の可能性。
pub fn compress_okumura_basic_tail1_full(input: &[u8]) -> Vec<Token> {
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

    loop {
        let len_before = len;
        let cap = (len + 1).min(F);
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy + 任意 pos の tail+1 phantom (RLE 限定なし)。Mode A 非 RLE 用。
pub fn compress_okumura_no_dummy_tail1_full(input: &[u8]) -> Vec<Token> {
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
        let len_before = len;
        let cap = (len + 1).min(F);
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// dummy_then_drop (奥村 F dummy → token 0 後に dummy node 削除) + tail+1 RLE.
pub fn compress_okumura_dummy_then_drop_tail1(input: &[u8]) -> Vec<Token> {
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

    let mut dummy_positions: Vec<i32> = Vec::with_capacity(F);
    for i in 1..=F {
        let p = ((r - i as i32) + N as i32) & (N as i32 - 1);
        st.insert_node(p);
        dummy_positions.push(p);
    }
    st.insert_node(r);

    let mut first_token_done = false;
    loop {
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if !first_token_done {
            first_token_done = true;
            for p in dummy_positions.iter().copied() {
                if p == r {
                    continue;
                }
                st.delete_node(p);
            }
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy + LeftFirst BST + RLE tail+1 phantom. tail1_rle と LeftFirst の合成。
pub fn compress_okumura_no_dummy_left_first_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_with_bst(input, BstMode::LeftFirst)
}

/// no_dummy + NoSwap BST + RLE tail+1 phantom.
pub fn compress_okumura_no_dummy_no_swap_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_with_bst(input, BstMode::NoSwap)
}

/// no_dummy + tail1_rle の BstMode 切替版。
fn compress_okumura_no_dummy_tail1_with_bst(input: &[u8], bst_mode: BstMode) -> Vec<Token> {
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

    loop {
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }

        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// no_dummy + tie-break = 最大 pos 値 (= 数値が大きい方優先) + tail1 RLE phantom.
/// Mode B の「leaf_pos > oku_pos が 65%」観察に基づく。
pub fn compress_okumura_no_dummy_max_pos_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_with_tie(input, TieMode::AllowEq)
}

/// no_dummy + tie-break = 最大距離 (MaxDistTie) + tail1 RLE phantom.
pub fn compress_okumura_no_dummy_max_dist_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_with_tie(input, TieMode::MaxDistTie)
}

/// no_dummy + tie-break = 最小距離 (DistanceTie) + tail1 RLE phantom.
pub fn compress_okumura_no_dummy_min_dist_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_with_tie(input, TieMode::DistanceTie)
}

fn compress_okumura_no_dummy_tail1_with_tie(input: &[u8], tie_mode: TieMode) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = tie_mode;
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
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy_tail1 + post-BST exhaustive min-dist override.
/// セッション 389 (2026-05-10) ML quick-eval で `dist_rank=1` が dominant feature
/// (38%) と判明: tie 場面で encoder が最小距離を選ぶ仮説。BST 内 tie (DistanceTie)
/// では届かない外候補を捕えるため、4096 ring 位置を全探索して同 len の最小 dist
/// 候補で match_position を上書きする。
pub fn compress_okumura_no_dummy_min_dist_exh_tail1(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        if (st.match_length as usize) > THRESHOLD {
            let target_len = st.match_length as usize;
            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if cur_dist > 0 { cur_dist } else { N as i32 };
            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d == 0 || d >= best_dist {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// 同上の max-dist (= 最遠) 版。dist=0 (= r 自身) を除外し、最大距離の同 len 候補を選ぶ。
pub fn compress_okumura_no_dummy_max_dist_exh_tail1(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        if (st.match_length as usize) > THRESHOLD {
            let target_len = st.match_length as usize;
            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if cur_dist > 0 { cur_dist } else { 0 };
            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d <= best_dist {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy_tail1 + len-split exhaustive: 短マッチ (len ≤ split) は max-dist、
/// 長マッチ (len > split) は min-dist で同 len 候補を上書き。
/// セッション 389 (2026-05-10) ML depth-3 tree が示した rule:
/// - cand_len ≤ 13 → max_minus_dist == 0 (= 最遠) のみ chosen
/// - cand_len ≥ 14 → dist_minus_min == 0 (= 最近) のみ chosen
fn compress_okumura_no_dummy_len_split_exh_tail1(input: &[u8], split: u32) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        if (st.match_length as usize) > THRESHOLD {
            let target_len = st.match_length as usize;
            let want_min = (target_len as u32) > split;

            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if want_min {
                if cur_dist > 0 {
                    cur_dist
                } else {
                    N as i32
                }
            } else {
                if cur_dist > 0 {
                    cur_dist
                } else {
                    0
                }
            };

            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d == 0 {
                    continue;
                }
                let take_dist = if want_min {
                    d < best_dist
                } else {
                    d > best_dist
                };
                if !take_dist {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

pub fn compress_okumura_no_dummy_len_split13_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_len_split_exh_tail1(input, 13)
}

pub fn compress_okumura_no_dummy_len_split14_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_len_split_exh_tail1(input, 14)
}

pub fn compress_okumura_no_dummy_len_split15_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_len_split_exh_tail1(input, 15)
}

pub fn compress_okumura_no_dummy_len_split16_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_len_split_exh_tail1(input, 16)
}

/// no_dummy_tail1 with custom KeyMode (BST root key calculation).
/// セッション 389 (2026-05-11) 仮説: 実 encoder は BST root key を 1byte ではなく
/// 2byte hash (XOR or add) で計算しているかもしれない。違う visit 順 → 違う tie 結果。
fn compress_okumura_no_dummy_tail1_keymode(input: &[u8], key_mode: KeyMode) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.key_mode = key_mode;
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

pub fn compress_okumura_no_dummy_tail1_xor(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_keymode(input, KeyMode::XorByte01)
}

pub fn compress_okumura_no_dummy_tail1_add(input: &[u8]) -> Vec<Token> {
    compress_okumura_no_dummy_tail1_keymode(input, KeyMode::AddByte01Mod256)
}

/// basic_tail1 with custom KeyMode.
fn compress_okumura_basic_tail1_keymode(input: &[u8], key_mode: KeyMode) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = TieMode::StrictGt;
    st.key_mode = key_mode;
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

pub fn compress_okumura_basic_tail1_xor(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_tail1_keymode(input, KeyMode::XorByte01)
}

pub fn compress_okumura_basic_tail1_add(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_tail1_keymode(input, KeyMode::AddByte01Mod256)
}

/// no_dummy_tail1 + 「len = F (= 18) のみ」exhaustive min-dist override。
/// セッション 389 (2026-05-11) データ分析: cand_len=18 の tie 場面で
/// 99.94% が min_dist 候補 (rank_1) → BST + tail1 の選択を len=18 だけ補正する。
pub fn compress_okumura_no_dummy_min_dist_only18_exh_tail1(input: &[u8]) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        // len=F=18 のときだけ exhaustive min-dist override
        if (st.match_length as usize) == F {
            let target_len = F;
            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if cur_dist > 0 { cur_dist } else { N as i32 };
            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d == 0 || d >= best_dist {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// basic_tail1 + len-split exhaustive (短 → max_dist, 長 → min_dist)
fn compress_okumura_basic_len_split_exh_tail1(input: &[u8], split: u32) -> Vec<Token> {
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
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        if (st.match_length as usize) > THRESHOLD {
            let target_len = st.match_length as usize;
            let want_min = (target_len as u32) > split;
            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if want_min {
                if cur_dist > 0 {
                    cur_dist
                } else {
                    N as i32
                }
            } else {
                if cur_dist > 0 {
                    cur_dist
                } else {
                    0
                }
            };
            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d == 0 {
                    continue;
                }
                let take = if want_min {
                    d < best_dist
                } else {
                    d > best_dist
                };
                if !take {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

pub fn compress_okumura_basic_len_split8_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_len_split_exh_tail1(input, 8)
}

pub fn compress_okumura_basic_len_split13_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_len_split_exh_tail1(input, 13)
}

pub fn compress_okumura_basic_max_dist_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_len_split_exh_tail1(input, 18)
}

pub fn compress_okumura_basic_min_dist_exh_tail1(input: &[u8]) -> Vec<Token> {
    compress_okumura_basic_len_split_exh_tail1(input, 0)
}

/// 同上の basic ベース版 (basic_tail1 + len=18 min_dist override)
pub fn compress_okumura_basic_min_dist_only18_exh_tail1(input: &[u8]) -> Vec<Token> {
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
    // basic mode: F dummy inserts before the real one
    for i in 1..=F {
        st.insert_node(r - i as i32);
    }
    st.insert_node(r);
    let mask: i32 = (N as i32) - 1;
    loop {
        let mp = (st.match_position & mask) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & mask) as usize;
        let is_rle = mp == r_minus_1;
        let len_before = len;
        let cap = if is_rle { (len + 1).min(F) } else { len };
        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }

        if (st.match_length as usize) == F {
            let target_len = F;
            let cur_dist = (r - st.match_position) & mask;
            let mut best_pos = st.match_position;
            let mut best_dist = if cur_dist > 0 { cur_dist } else { N as i32 };
            for cand_pos in 0..(N as i32) {
                if cand_pos == r {
                    continue;
                }
                let d = (r - cand_pos) & mask;
                if d == 0 || d >= best_dist {
                    continue;
                }
                let mut l = 0usize;
                while l < target_len {
                    let a = st.text_buf[cand_pos as usize + l];
                    let b = st.text_buf[r as usize + l];
                    if a != b {
                        break;
                    }
                    l += 1;
                }
                if l >= target_len {
                    best_dist = d;
                    best_pos = cand_pos;
                }
            }
            st.match_position = best_pos;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }
    out
}

/// no_dummy + 入力 byte=0x20 のとき literal 強制 (Mode C 対策)。
/// match_position が r-1 RLE の場合は除外 (RLE は別ルール)。
pub fn compress_okumura_no_dummy_lit_for_0x20(input: &[u8]) -> Vec<Token> {
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

        let cur_byte = st.text_buf[r as usize];
        let mp = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp == r_minus_1;
        // 0x20 byte で RLE でない場合は match を捨てて literal にする
        let force_lit = cur_byte == 0x20 && !is_rle && (st.match_length as usize) > THRESHOLD;

        if force_lit || (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(cur_byte));
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
    encode_loop(
        &mut st,
        &mut out,
        &mut r,
        &mut s,
        &mut input_idx,
        &mut len,
        input,
    );
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
    encode_loop(
        &mut st,
        &mut out,
        &mut r,
        &mut s,
        &mut input_idx,
        &mut len,
        input,
    );
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
    encode_loop(
        &mut st,
        &mut out,
        &mut r,
        &mut s,
        &mut input_idx,
        &mut len,
        input,
    );
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

    encode_loop(
        &mut st,
        &mut out,
        &mut r,
        &mut s,
        &mut input_idx,
        &mut len,
        input,
    );
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

    encode_loop(
        &mut st,
        &mut out,
        &mut r,
        &mut s,
        &mut input_idx,
        &mut len,
        input,
    );
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

/// Stage 3 (Issue #14) の full-F min-age override 用 shadow 状態。
///
/// Leaf decoder と同一の ring 状態 (`ring`) と、各 slot の最終書込み tick
/// (`write_tick`、v12 dataset の `cand_age_start` と同一定義) を、
/// エンコード進行に合わせて teacher-forcing なしで維持する。
struct MinAgeFullFHook {
    ring: [u8; N],
    write_tick: [u32; N],
    shadow_r: usize,
    input_pos: usize,
}

impl MinAgeFullFHook {
    fn new() -> Self {
        Self {
            ring: [0x20u8; N],
            write_tick: [u32::MAX; N],
            shadow_r: N - F,
            input_pos: 0,
        }
    }

    /// full-F tie 規則: full-F 候補から cand_age_start 最小を採用。
    /// ただし Stage 2 の検証データは n_max <= 32 (v12 N_MAX_CAP) の
    /// グループに限られるため、それを超える巨大 tie (未書込み 0x20
    /// 領域の縮退 tie 等) は規則の適用範囲外 → insert_node の選択を維持。
    /// 最小 age が一意でない場合 (全候補未書込み u32::MAX 等) も
    /// 規則では決められないので insert_node の選択を維持する。
    fn override_full_f_pos(&self, input: &[u8]) -> Option<u16> {
        use super::lf2_tokens::enumerate_match_candidates_with_writeback;
        const N_MAX_CAP: usize = 32;
        let candidates = enumerate_match_candidates_with_writeback(
            &self.ring,
            input,
            self.input_pos,
            self.shadow_r,
        );
        let full: Vec<(u32, u16)> = candidates
            .iter()
            .filter(|c| c.len as usize == F)
            .map(|c| {
                let ps = (c.pos as usize) & 0x0fff;
                let age = if self.write_tick[ps] == u32::MAX {
                    u32::MAX
                } else {
                    (self.input_pos as u32).saturating_sub(self.write_tick[ps])
                };
                (age, c.pos)
            })
            .collect();
        if full.len() >= 2 && full.len() <= N_MAX_CAP {
            let min_age = full.iter().map(|&(a, _)| a).min().unwrap();
            let mins: Vec<u16> = full
                .iter()
                .filter(|&&(a, _)| a == min_age)
                .map(|&(_, p)| p)
                .collect();
            if mins.len() == 1 && min_age != u32::MAX {
                return Some(mins[0]);
            }
        }
        None
    }

    /// shadow ring を出力バイト分進める (v12 の ring/write_tick 更新と同一)
    fn advance(&mut self, input: &[u8], emitted: usize) {
        for _ in 0..emitted {
            if self.input_pos >= input.len() {
                break;
            }
            self.ring[self.shadow_r] = input[self.input_pos];
            self.write_tick[self.shadow_r] = self.input_pos as u32;
            self.shadow_r = (self.shadow_r + 1) & (N - 1);
            self.input_pos += 1;
        }
    }
}

/// 末尾 (入力残り `len` バイトの局面) での `match_length` クリップ規則 (Issue #14 Stage 9)。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TailMode {
    /// 素の奥村実装: `match_length` を残り入力バイト数 (`len`) にクリップする。
    Clip,
    /// Stage 9 当初案: クリップを一切行わない (insert_node の生一致長をそのまま採用)。
    Unbounded,
    /// Stage 9-2 (実測から導出): `match_length` を `len + 1` にクリップする。
    /// LEAF_NOT_CAND 44 本全数で `tail_len - remaining == 1` だった観測に基づく。
    Plus1,
}

/// 未書込みリング領域 (0x20 初期埋め) へのマッチ許可規則 (Issue #14 Stage 10-3)。
///
/// Stage 10-2 の観測で、KIND_DIFF 49 本中 42 本 (86%) で Sim の match position が
/// 「一度も実際に書き込まれていない」ダミー初期化領域 (`write_tick` 未設定) を
/// 指していると判明。「Leaf は一度も書き込まれていない領域へのマッチを許可
/// しない」という仮説を検証する variant。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DummyMode {
    /// 既存挙動 (無変更): 未書込み領域へのマッチも許可する。
    Allow,
    /// マッチ窓 (`match_position..match_position+match_length`) に未書込み
    /// スロットが1つでも含まれれば、そのマッチ候補を不採用にし Literal に
    /// フォールバックする。
    RejectAny,
    /// マッチ窓が**全域**未書込みの場合のみ不採用にする
    /// (一部だけ未書込みの窓は許可する、より緩い規則)。
    RejectAllUnwritten,
    /// Stage 10-4 の発見 (拒否候補は例外なく `[r-F, r-1]` = ブートストラップ
    /// ダミーノード帯 (r=N-F=4078 なので 4060..=4077) に collapse する) を
    /// 受けた絞り込み variant 群 (Stage 10-5)。
    ///
    /// `in_bootstrap_band` / `at_bootstrap_edge` の帯判定はいずれも
    /// **match の開始位置 (`match_position`) のみ**で行う。窓全体
    /// (`match_position..match_position+match_length`) が帯とどれだけ
    /// overlap するかは見ていない (窓の「未書込み」判定自体は別途
    /// `any_unwritten`/`all_unwritten` で窓全体を見る)。
    ///
    /// v1: 候補位置がダミーノード帯かつ窓が全域未書込みなら不採用。
    RejectBootstrapUnwritten,
    /// v2: v1 に加えて `match_length > 10` のときのみ不採用にする
    /// (Stage 10-4 実測: Leaf 採用済み fully-unwritten 窓の最大長は 10)。
    RejectBootstrapUnwrittenLenGt10,
    /// v3: 窓条件を問わず、候補位置が帯の端 (4076 or 4077、r に最も近い =
    /// 最も新しいダミーノード) ならそれだけで不採用にする。
    RejectBootstrapEdge,
    /// Stage 11-4 (Issue #14) v4: v1 の write_tick 偽陽性を補正した精緻化版。
    /// 候補窓の**全バイトが「bootstrap 合成 0x20」のみ**で構成される場合に
    /// 限り不採用にする。初期先読み充填領域 (`r..r+F-1` = 4078..4095、実
    /// データ) は `is_real_slot` で常に「実在」扱いにするため、帯 [4060,4077]
    /// 始まりの候補でも窓が 4078 以降の実データにまたがれば許可される
    /// (帯端 4077 の救済)。位置帯 (`in_bootstrap_band`) は判定に使わない
    /// (v1/v2/v3 と異なり、窓の内容だけで判定する)。
    RejectPureBootstrap,
}

/// Stage 10-4 で確認したブートストラップダミーノード帯: `insert_node(r-i)`
/// (i=1..=F, r=N-F=4078) が挿入する合成ノードの位置範囲 `[r-F, r-1]`。
/// N/F は定数なのでこの範囲も定数 (4060..=4077)。
const BOOTSTRAP_DUMMY_LO: usize = N - F - F; // r - F = 4078 - 18 = 4060
const BOOTSTRAP_DUMMY_HI: usize = N - F - 1; // r - 1 = 4078 - 1  = 4077
/// 帯の端2スロット (`r-1`, `r-2` = 4077, 4076): 最も新しく挿入されたダミー
/// ノード。`DummyMode::RejectBootstrapEdge` (v3) が狙う範囲。
const BOOTSTRAP_DUMMY_EDGE_LO: usize = N - F - 2; // r - 2 = 4078 - 2 = 4076
/// 初期先読み充填領域: `r_init..r_init+F-1` = `4078..4095`。実データだが
/// write_tick は更新されない (Stage 10-4 の偽陽性ギャップ)。
/// `DummyMode::RejectPureBootstrap` (v4) はこの範囲を「実在」として扱う
/// ことでこの偽陽性を補正する。
const INITIAL_LOOKAHEAD_LO: usize = N - F; // r_init = 4078
const INITIAL_LOOKAHEAD_HI: usize = N - 1; // 4095

fn compress_okumura_impl(input: &[u8], tie_mode: TieMode) -> Vec<Token> {
    compress_okumura_impl_hooked(input, tie_mode, None, TailMode::Clip)
}

/// 奥村 lzss.c `Encode()` 逐語移植の共通実装。
///
/// `hook` が `None` のとき従来の `compress_okumura_impl` と完全に同一の挙動。
/// `Some` のときのみ、出力 Match の `match_length == F` の箇所で
/// `MinAgeFullFHook::override_full_f_pos` による `match_position` 差し替えを試みる。
///
/// `tail_mode` で入力末尾での `match_length` クリップ規則を切り替える
/// (Issue #14 Stage 9 / Stage 9-2)。`TailMode` のドキュメント参照。
///
/// `dummy_mode` は既定で `DummyMode::Allow` (無変更) を使う内部ヘルパー。
fn compress_okumura_impl_hooked(
    input: &[u8],
    tie_mode: TieMode,
    hook: Option<&mut MinAgeFullFHook>,
    tail_mode: TailMode,
) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(input, tie_mode, hook, tail_mode, DummyMode::Allow, None)
}

/// Stage 9-2c 用トレース1ステップ: cap 適用前の生の match_length/match_position、
/// その時点の残り入力バイト数 (`len`)、cap 適用後の match_length。
#[derive(Debug, Clone, Copy)]
pub struct TailTraceStep {
    pub raw_match_length: i32,
    pub raw_match_position: i32,
    pub remaining: usize,
    pub capped_match_length: i32,
}

fn compress_okumura_impl_hooked_traced(
    input: &[u8],
    tie_mode: TieMode,
    hook: Option<&mut MinAgeFullFHook>,
    tail_mode: TailMode,
    dummy_mode: DummyMode,
    trace: Option<&mut Vec<TailTraceStep>>,
) -> Vec<Token> {
    // 既定 (cmp_mode=Unsigned, del_mode=Predecessor, write_time_order=None,
    // rot_no_delete=false, no_swap=false) = 従来と完全同一の挙動。
    compress_okumura_impl_hooked_traced_full(
        input,
        tie_mode,
        hook,
        tail_mode,
        dummy_mode,
        trace,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::None,
        false,
        false,
    )
}

/// Stage 12-7/12-8/12-11 (Issue #14): `compress_okumura_impl_hooked_traced` に
/// `cmp_mode` / `del_mode` / `write_time_descending` / `rot_no_delete` /
/// `no_swap` を追加したフル版。既存呼び出しは全て上の薄いラッパー経由で
/// 無変更値を渡す。
///
/// `write_time_descending=true` のとき、Stage 12-4 の `SimMode::WriteTimeDescending`
/// と同じ初期化 (dummy F 個挿入なし、初期先読み充填 [r,r+F-1] を降順で
/// 開始時に一括挿入、以降 F-1 回分の per-byte insert_node をスキップ) を
/// 自走エンコーダ側でも再現する。
///
/// `rot_no_delete=true` のとき「腐った木」仮説 (Stage 12-11):
/// 消費時の `delete_node(s)` を一切呼ばない。`no_swap=true` を併用すると
/// full-F 一致時のノード置換 (swap-with-r) も省略する (RotB)。
#[allow(clippy::too_many_arguments)]
fn compress_okumura_impl_hooked_traced_full(
    input: &[u8],
    tie_mode: TieMode,
    mut hook: Option<&mut MinAgeFullFHook>,
    tail_mode: TailMode,
    dummy_mode: DummyMode,
    mut trace: Option<&mut Vec<TailTraceStep>>,
    cmp_mode: CmpMode,
    del_mode: DelMode,
    write_time_order: WriteTimeOrder,
    rot_no_delete: bool,
    no_swap: bool,
) -> Vec<Token> {
    let mut st = Okumura::new(0x20);
    st.tie_mode = tie_mode;
    st.cmp_mode = cmp_mode;
    st.del_mode = del_mode;
    st.rot_no_delete = rot_no_delete;
    // Stage 12-15/12-16 (Issue #14 脈1 Prong B): write_time_order 使用時は
    // OKU_DEBUG_TREE_CHECK での毎操作不変条件チェック対象に含める。
    st.write_time_variant = !matches!(write_time_order, WriteTimeOrder::None);
    if no_swap {
        st.bst_mode = BstMode::NoSwap;
    }
    st.init_tree();

    // Stage 10-3: 各リングスロットへの最終書込み input_pos。u32::MAX = 未書込み。
    // `DummyMode::Allow` (既存呼び出し全て) では確保も更新も一切行わない
    // (レビュー指摘: 無条件確保はコストが無駄。`None` のときは判定ブロック
    // 自体に到達しない設計)。
    let mut write_tick: Option<Vec<u32>> =
        (!matches!(dummy_mode, DummyMode::Allow)).then(|| vec![u32::MAX; N]);

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

    // Stage 12-8/12-16: write_time_order != None のときは Stage 12-4 の
    // WriteTimeDescending/WriteTimeAscending と同じ初期化 (dummy なし、初期
    // 先読み充填を降順/昇順で一括挿入) にし、以降 F-1 回分の per-byte
    // insert_node をスキップする。
    let mut skip_inserts: usize = 0;
    match write_time_order {
        WriteTimeOrder::Descending | WriteTimeOrder::DescendingKeepDummy => {
            // Stage 12-17: KeepDummy 版は原典と同じ dummy F 個 (r-F..r-1) を
            // 先に挿入して残す (Step 1 で特定した「dummy 帯が一度も挿入されない
            // 空白」を埋める)。
            if matches!(write_time_order, WriteTimeOrder::DescendingKeepDummy) {
                for i in 1..=F {
                    st.insert_node(r - i as i32);
                }
            }
            for k in (0..F as i32).rev() {
                st.insert_node(r + k);
            }
            skip_inserts = F - 1;
        }
        WriteTimeOrder::Ascending | WriteTimeOrder::AscendingKeepDummy => {
            if matches!(write_time_order, WriteTimeOrder::AscendingKeepDummy) {
                for i in 1..=F {
                    st.insert_node(r - i as i32);
                }
            }
            for k in 0..F as i32 {
                st.insert_node(r + k);
            }
            skip_inserts = F - 1;
        }
        WriteTimeOrder::None => {
            // 最初に F 個のダミー挿入（奥村原典 for (i = 1; i <= F; i++) InsertNode(r - i)）
            for i in 1..=F {
                st.insert_node(r - i as i32);
            }
            // 最初の本挿入
            st.insert_node(r);
        }
    }

    loop {
        // match_length をフレーム残量に丸める (Clip のみ)。Unbounded は丸めなし、
        // Plus1 は len+1 まで許容する (Stage 9-2)。
        let cap = match tail_mode {
            TailMode::Clip => Some(len),
            TailMode::Unbounded => None,
            TailMode::Plus1 => Some(len + 1),
        };
        let raw_match_length = st.match_length;
        let raw_match_position = st.match_position;
        if let Some(cap) = cap {
            if st.match_length as usize > cap {
                st.match_length = cap as i32;
            }
        }
        if let Some(t) = trace.as_deref_mut() {
            t.push(TailTraceStep {
                raw_match_length,
                raw_match_position,
                remaining: len,
                capped_match_length: st.match_length,
            });
        }

        // Stage 10-3: 未書込みリング領域へのマッチを不採用にする (dummy_mode)。
        // マッチとして採用されるサイズ (> THRESHOLD) のときだけ判定する。
        // 不採用ならその場で Literal にフォールバックする (match_length=1 に
        // 落として下の THRESHOLD 分岐に流す。これは既存の「マッチが短すぎて
        // Literal になる」経路と完全に同じ扱い)。
        // `write_tick` が `None` (= `DummyMode::Allow`) のときはこのブロック
        // 自体に入らない (既存経路は判定コスト・確保コストとも一切発生しない)。
        if let Some(write_tick) = write_tick.as_ref() {
            if st.match_length as usize > THRESHOLD {
                let pos = (st.match_position as usize) & (N - 1);
                let mlen = st.match_length as usize;
                let mut any_unwritten = false;
                let mut all_unwritten = true;
                for k in 0..mlen {
                    if write_tick[(pos + k) & (N - 1)] == u32::MAX {
                        any_unwritten = true;
                    } else {
                        all_unwritten = false;
                    }
                }
                let in_bootstrap_band = pos >= BOOTSTRAP_DUMMY_LO && pos <= BOOTSTRAP_DUMMY_HI;
                let at_bootstrap_edge = pos == BOOTSTRAP_DUMMY_HI || pos == BOOTSTRAP_DUMMY_EDGE_LO;
                // v4: 窓の全バイトが「実在しない (未書込み かつ 初期先読み充填域
                // でもない)」場合のみ true。初期先読み充填域 (4078..4095) は
                // write_tick 未更新でも「実在」扱いする (Stage 10-4 の偽陽性補正)。
                let all_pure_bootstrap = (0..mlen).all(|k| {
                    let slot = (pos + k) & (N - 1);
                    let is_lookahead = slot >= INITIAL_LOOKAHEAD_LO && slot <= INITIAL_LOOKAHEAD_HI;
                    write_tick[slot] == u32::MAX && !is_lookahead
                });
                let reject = match dummy_mode {
                    DummyMode::Allow => false,
                    DummyMode::RejectAny => any_unwritten,
                    DummyMode::RejectAllUnwritten => all_unwritten,
                    DummyMode::RejectBootstrapUnwritten => in_bootstrap_band && all_unwritten,
                    DummyMode::RejectBootstrapUnwrittenLenGt10 => {
                        in_bootstrap_band && all_unwritten && mlen > 10
                    }
                    DummyMode::RejectBootstrapEdge => at_bootstrap_edge,
                    DummyMode::RejectPureBootstrap => all_pure_bootstrap,
                };
                if reject {
                    st.match_length = 1;
                }
            }
        }

        // 出力
        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            // match_position は InsertNode 内で必ず 0..N の範囲に収まる（ring index）
            let mut pos = (st.match_position as u16) & ((N as u16) - 1);
            if st.match_length as usize == F {
                if let Some(h) = hook.as_deref() {
                    if let Some(p) = h.override_full_f_pos(input) {
                        pos = p;
                    }
                }
            }
            out.push(Token::Match {
                pos,
                len: st.match_length as u8,
            });
        }

        let last_match_length = st.match_length as usize;

        if let Some(h) = hook.as_deref_mut() {
            h.advance(input, last_match_length);
        }

        // last_match_length 回 ring を進める
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            // Stage 12-11: rot_no_delete のときは delete_node(s) を一切呼ばない
            // (「腐った木」仮説: Leaf は消費時の DeleteNode を省略している)。
            if !st.rot_no_delete {
                st.delete_node(s);
            }
            let c = input[input_idx];
            input_idx += 1;

            st.text_buf[s as usize] = c;
            // s < F-1 のときは末尾 overlap 領域にもコピー
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            // Stage 10-3: このスロットに実データが書かれた input_pos を記録
            // (`DummyMode::Allow` では `write_tick` が `None` のため更新自体
            // 発生しない。既存の write_tick 定義 [write_tick[slot] = 最後に
            // 書いた byte の input_pos] と同一の規約)。
            if let Some(write_tick) = write_tick.as_mut() {
                write_tick[s as usize] = (input_idx - 1) as u32;
            }

            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            if skip_inserts > 0 {
                skip_inserts -= 1;
            } else {
                st.insert_node(r);
            }
            i += 1;
        }

        // 入力が尽きた後の残り処理: len を減らしつつ DeleteNode
        // `&& len > 0` は underflow ガード。TailMode::Unbounded / Plus1 では
        // `last_match_length > len` (出力 match が実残り入力より長い) が起こり得て、
        // このガードが無いと `len -= 1` が len==0 の状態で呼ばれ usize underflow
        // で panic する。TailMode::Clip では match_length を必ず len 以下に
        // クリップするため (`if st.match_length as usize > cap { ... }` 参照)、
        // この分岐は絶対に発火しない (このガードは Unbounded/Plus1 専用の保険)。
        while i < last_match_length && len > 0 {
            if !st.rot_no_delete {
                st.delete_node(s);
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len -= 1;
            if len > 0 {
                if skip_inserts > 0 {
                    skip_inserts -= 1;
                } else {
                    st.insert_node(r);
                }
            }
            i += 1;
        }

        if len == 0 {
            break;
        }
    }

    out
}

/// Stage 3 (Issue #14): Stage 2 で確定した Leaf タイブレイク複合規則のエンコーダ。
///
/// ベースは `compress_okumura` (Basic: dummy 挿入あり・StrictGt) と完全に同じ。
/// ただし出力 Match の `match_length == F` のとき**のみ**、Leaf 実 ring の
/// 全候補列挙 (`enumerate_match_candidates_with_writeback`) から full-F
/// (len == F) 候補を集め、`cand_age_start` (= 候補開始 slot の最終書込みから
/// の経過 input_pos。v12 データセットと同一定義。未書込み slot は u32::MAX)
/// が最小 = ring に最も新しく書かれた候補へ `match_position` を差し替える。
///
/// - Stage 2 検証: max_len==F の tie 522,913 グループで min-age 規則の的中率 100.00%
/// - max_len < F は奥村 insert_node (rank=1) の返す match をそのまま使う (99.73%)
///
/// **Stage 3 実測の負結果 (2026-07-17)**: 本 variant の 522 本 byte-exact は
/// 165/522 で素の Basic と完全同一集合 (トークン相違 0 ファイル)。規則 1 は
/// 「Basic の insert_node 選択の記述」であり、Basic を超える修正力は無い。
///
/// age の定義は `lf2_pairwise_dataset_v12.rs` の `cand_age_start` を踏襲:
///   pos_start = cand_pos & 0x0fff
///   age = write_tick[pos_start] == u32::MAX ? u32::MAX
///                                          : input_pos - write_tick[pos_start]
/// (write_tick[slot] = その slot に最後に書いた byte の input_pos)
pub fn compress_okumura_rank1_minage(input: &[u8]) -> Vec<Token> {
    let mut hook = MinAgeFullFHook::new();
    compress_okumura_impl_hooked(input, TieMode::StrictGt, Some(&mut hook), TailMode::Clip)
}

/// Stage 9 (Issue #14): 末尾緩和 (tail-relaxed) variant。クリップを一切行わない
/// (`TailMode::Unbounded`)。
///
/// ベースは `compress_okumura` (Basic: dummy 挿入あり・StrictGt・hook なし) と
/// 完全に同じだが、入力末尾で `match_length` を残り入力バイト数にクリップする
/// 処理を行わない。素の奥村実装は末尾で「残り入力バイト数より長い match」を
/// 出せないが、Leaf 実エンコーダはこれを出す (例: C0102.LF2 は残り12バイトの
/// 位置で長さ13の match)。
///
/// **Stage 9 実測の負結果**: この単純なクリップ解除は 131/522 に回帰する
/// (Basic の 165/522 を下回る)。Stage 9-2 の実測 (LEAF_NOT_CAND 44 本全数で
/// `tail_len - remaining == 1`) を受けて `compress_okumura_tail_plus1` を追加した。
pub fn compress_okumura_tail_relaxed(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked(input, TieMode::StrictGt, None, TailMode::Unbounded)
}

/// Stage 9-2 (Issue #14): 末尾クリップを `remaining + 1` に緩和する variant。
///
/// LEAF_NOT_CAND 44 本全数の実測で `leaf の tail token 長 - divergence 時点の
/// 入力残りバイト数 == 1` だったことに基づく仮説の実装。
pub fn compress_okumura_tail_plus1(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked(input, TieMode::StrictGt, None, TailMode::Plus1)
}

/// Stage 9-2c (Issue #14) デバッグ用: `compress_okumura_tail_plus1` と同じ挙動で
/// トークン列を出力しつつ、各ステップの cap 適用前の生 match_length/match_position
/// もトレースとして返す。
pub fn compress_okumura_tail_plus1_traced(input: &[u8]) -> (Vec<Token>, Vec<TailTraceStep>) {
    let mut trace = Vec::new();
    let tokens = compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        Some(&mut trace),
    );
    (tokens, trace)
}

/// Stage 10-3 (Issue #14): Clip + 未書込み領域マッチ不採用 (`DummyMode::RejectAny`)。
///
/// マッチ窓に未書込みスロットが1つでも含まれれば Literal にフォールバックする。
/// Stage 10-2 で KIND_DIFF 49 本中 42 本が Sim 側の未書込み領域マッチだったこと
/// を受けた仮説「Leaf は一度も書き込まれていない領域へのマッチを許可しない」の検証。
pub fn compress_okumura_clip_no_dummy_any(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectAny,
        None,
    )
}

/// Stage 10-3 (Issue #14): Plus1 + 未書込み領域マッチ不採用 (`DummyMode::RejectAny`)。
pub fn compress_okumura_plus1_no_dummy_any(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectAny,
        None,
    )
}

/// Stage 10-3 (Issue #14): Clip + 未書込み領域マッチ不採用 (`DummyMode::RejectAllUnwritten`、
/// 窓全体が未書込みのときのみ不採用にする緩い規則)。
pub fn compress_okumura_clip_no_dummy_all(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectAllUnwritten,
        None,
    )
}

/// Stage 10-3 (Issue #14): Plus1 + 未書込み領域マッチ不採用 (`DummyMode::RejectAllUnwritten`)。
pub fn compress_okumura_plus1_no_dummy_all(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectAllUnwritten,
        None,
    )
}

/// Stage 10-5 (Issue #14) v1: Clip + `DummyMode::RejectBootstrapUnwritten`。
pub fn compress_okumura_clip_no_bootstrap_v1(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapUnwritten,
        None,
    )
}

/// Stage 10-5 (Issue #14) v1: Plus1 + `DummyMode::RejectBootstrapUnwritten`。
pub fn compress_okumura_plus1_no_bootstrap_v1(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapUnwritten,
        None,
    )
}

/// Stage 10-5 (Issue #14) v2: Clip + `DummyMode::RejectBootstrapUnwrittenLenGt10`。
pub fn compress_okumura_clip_no_bootstrap_v2(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapUnwrittenLenGt10,
        None,
    )
}

/// Stage 10-5 (Issue #14) v2: Plus1 + `DummyMode::RejectBootstrapUnwrittenLenGt10`。
pub fn compress_okumura_plus1_no_bootstrap_v2(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapUnwrittenLenGt10,
        None,
    )
}

/// Stage 10-5 (Issue #14) v3: Clip + `DummyMode::RejectBootstrapEdge`。
pub fn compress_okumura_clip_no_bootstrap_v3(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapEdge,
        None,
    )
}

/// Stage 10-5 (Issue #14) v3: Plus1 + `DummyMode::RejectBootstrapEdge`。
pub fn compress_okumura_plus1_no_bootstrap_v3(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapEdge,
        None,
    )
}

/// Stage 12-3 (Issue #14 脈1 EQ_UPDATE 仮説): `TieMode::AllowEq` (`>=`。同一長
/// 候補は BST 探索経路上で最後に訪れたノードを採用) 版一式。既存 8 variant
/// (Clip/Plus1 × Allow/v1/v2/v3) の tie_mode だけを `StrictGt` → `AllowEq` に
/// 差し替えた対。
///
/// 動機: Stage 12-2b (binary tie 50件) で「両候補が BST 探索経路上にある
/// 32件全てで Leaf 採用位置が sim (StrictGt, 経路上最初のノード) より**後**に
/// 訪問される」というシグナルが確認された。これは奥村原典の
/// `if (i > match_length)` を `>=` に変えるだけで「同一長候補は経路上
/// 最後に訪れたノードが勝つ」動作になり、上記シグナルを直接説明しうる。
pub fn compress_okumura_clip_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(input, TieMode::AllowEq, None, TailMode::Clip, DummyMode::Allow, None)
}

/// Stage 12-3: Plus1 + `TieMode::AllowEq`。
pub fn compress_okumura_plus1_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(input, TieMode::AllowEq, None, TailMode::Plus1, DummyMode::Allow, None)
}

/// Stage 12-3: Clip + v1 (`DummyMode::RejectBootstrapUnwritten`) + `TieMode::AllowEq`。
pub fn compress_okumura_clip_no_bootstrap_v1_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapUnwritten,
        None,
    )
}

/// Stage 12-3: Plus1 + v1 + `TieMode::AllowEq`。
pub fn compress_okumura_plus1_no_bootstrap_v1_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapUnwritten,
        None,
    )
}

/// Stage 12-3: Clip + v2 (`DummyMode::RejectBootstrapUnwrittenLenGt10`) + `TieMode::AllowEq`。
pub fn compress_okumura_clip_no_bootstrap_v2_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapUnwrittenLenGt10,
        None,
    )
}

/// Stage 12-3: Plus1 + v2 + `TieMode::AllowEq`。
pub fn compress_okumura_plus1_no_bootstrap_v2_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapUnwrittenLenGt10,
        None,
    )
}

/// Stage 12-3: Clip + v3 (`DummyMode::RejectBootstrapEdge`) + `TieMode::AllowEq`。
pub fn compress_okumura_clip_no_bootstrap_v3_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Clip,
        DummyMode::RejectBootstrapEdge,
        None,
    )
}

/// Stage 12-3: Plus1 + v3 + `TieMode::AllowEq`。
pub fn compress_okumura_plus1_no_bootstrap_v3_eq(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::AllowEq,
        None,
        TailMode::Plus1,
        DummyMode::RejectBootstrapEdge,
        None,
    )
}

/// Stage 12-7 (Issue #14 脈: 鏡像等価性検証)。自走エンコーダの `cmp_mode`/
/// `del_mode` パラメータ化版。tie_mode=StrictGt (First) / tail_mode=Clip /
/// dummy_mode=Allow 固定。`OkumuraSim` の teacher-forcing 経路とは独立に、
/// 自走 (self-driven) の完全な token 列比較で鏡像等価性をサニティ検証する用途。
pub fn compress_okumura_cmp_del_variant(input: &[u8], cmp_mode: CmpMode, del_mode: DelMode) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        cmp_mode,
        del_mode,
        WriteTimeOrder::None,
        false,
        false,
    )
}

/// Stage 12-8 (Issue #14): Clip + `DelMode::Successor` (削除昇格側を鏡像化、
/// 比較は無変更)。522本フル計測用。
pub fn compress_okumura_clip_del_successor(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::None,
        false,
        false,
    )
}

/// Stage 12-8 (Issue #14): Plus1 + `DelMode::Successor`。
pub fn compress_okumura_plus1_del_successor(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::None,
        false,
        false,
    )
}

/// Stage 12-15 (Issue #14 脈1 Prong B): 「書込み時挿入」フル変種本命。
/// Clip + 奥村原典どおりの比較・削除 (`DelMode::Predecessor`、比較は Unsigned) +
/// `WriteTimeDescending`。C120x token3-4 を 14/14 満点で説明した挿入モデル
/// (Stage 12-4 `SimMode::WriteTimeDescending`) を、削除昇格側は変更せず
/// ベース P-F に単独適用したもの (Prong A close-out で本命に格上げ)。
pub fn compress_okumura_clip_writetime_descending(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::Descending,
        false,
        false,
    )
}

/// Stage 12-15: Plus1 + `DelMode::Predecessor` + `WriteTimeDescending`。
pub fn compress_okumura_plus1_writetime_descending(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::Descending,
        false,
        false,
    )
}

/// Stage 12-16 (Issue #14 脈1 Prong B 続き): Clip + `DelMode::Predecessor` +
/// `WriteTimeAscending`。Stage 12-16 Step 1 の対比プロファイリングで、Ascending
/// (`SimMode::WriteTimeAscending`) の per-tie 的中率が Descending (98.86%) より
/// 高く (98.91%)、P-F退行も711→362に半減した一方 none-of-6救済90件中84件
/// (93.3%) を維持することを確認済み。Descending より優先度の高い主軸候補。
pub fn compress_okumura_clip_writetime_ascending(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::Ascending,
        false,
        false,
    )
}

/// Stage 12-16: Plus1 + `DelMode::Predecessor` + `WriteTimeAscending`。
pub fn compress_okumura_plus1_writetime_ascending(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::Ascending,
        false,
        false,
    )
}

/// Stage 12-17 (Issue #14 脈1 Prong B 続き): Clip + `DelMode::Predecessor` +
/// `WriteTimeDescendingKeepDummy`。Step 1 の外部シャドートラッカーで、退行711件中
/// 173件 (24.3%) が「dummy帯 [4060,4077] が一度も挿入されない」ことに起因すると
/// 特定 (already_deleted 0件、origin=never が100%その帯に集中)。dummy挿入を
/// 復活させるハイブリッド修正。
pub fn compress_okumura_clip_writetime_descending_keepdummy(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::DescendingKeepDummy,
        false,
        false,
    )
}

/// Stage 12-17: Plus1 + `WriteTimeDescendingKeepDummy`。
pub fn compress_okumura_plus1_writetime_descending_keepdummy(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::DescendingKeepDummy,
        false,
        false,
    )
}

/// Stage 12-17: Clip + `WriteTimeAscendingKeepDummy`。
pub fn compress_okumura_clip_writetime_ascending_keepdummy(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::AscendingKeepDummy,
        false,
        false,
    )
}

/// Stage 12-17: Plus1 + `WriteTimeAscendingKeepDummy`。
pub fn compress_okumura_plus1_writetime_ascending_keepdummy(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::AscendingKeepDummy,
        false,
        false,
    )
}

/// Stage 12-8 (Issue #14): Clip + `DelMode::Successor` + `WriteTimeDescending`
/// (自走エンコーダ側での write_time_descending 再現込み)。
pub fn compress_okumura_clip_del_successor_wtd(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::Descending,
        false,
        false,
    )
}

/// Stage 12-8 (Issue #14): Plus1 + `DelMode::Successor` + `WriteTimeDescending`。
pub fn compress_okumura_plus1_del_successor_wtd(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::Descending,
        false,
        false,
    )
}

/// Stage 12-16 (Issue #14 脈1 Prong B 続き): Clip + `DelMode::Successor` +
/// `WriteTimeAscending` (削除昇格側と挿入順序の2軸併用、直交確認)。
pub fn compress_okumura_clip_del_successor_wta(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::Ascending,
        false,
        false,
    )
}

/// Stage 12-16: Plus1 + `DelMode::Successor` + `WriteTimeAscending`。
pub fn compress_okumura_plus1_del_successor_wta(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Successor,
        WriteTimeOrder::Ascending,
        false,
        false,
    )
}

/// Stage 12-11 (Issue #14 脈: 「腐った木」仮説) RotA: Clip + `rot_no_delete=true`
/// (消費時の delete_node(s) を一切呼ばない)。522本フル計測用。
pub fn compress_okumura_clip_rot_a(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::None,
        true,
        false,
    )
}

/// Stage 12-11 (Issue #14): Plus1 + RotA。
pub fn compress_okumura_plus1_rot_a(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::None,
        true,
        false,
    )
}

/// Stage 12-11 (Issue #14): RotA + full-F 一致時のノード置換も省略 (RotB)。
pub fn compress_okumura_clip_rot_b(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::None,
        true,
        true,
    )
}

/// Stage 12-11 (Issue #14): Plus1 + RotB。
pub fn compress_okumura_plus1_rot_b(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced_full(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::Allow,
        None,
        CmpMode::Unsigned,
        DelMode::Predecessor,
        WriteTimeOrder::None,
        true,
        true,
    )
}

/// Stage 11-4 (Issue #14) v4: Clip + `DummyMode::RejectPureBootstrap`。
pub fn compress_okumura_clip_no_bootstrap_v4(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Clip,
        DummyMode::RejectPureBootstrap,
        None,
    )
}

/// Stage 11-4 (Issue #14) v4: Plus1 + `DummyMode::RejectPureBootstrap`。
pub fn compress_okumura_plus1_no_bootstrap_v4(input: &[u8]) -> Vec<Token> {
    compress_okumura_impl_hooked_traced(
        input,
        TieMode::StrictGt,
        None,
        TailMode::Plus1,
        DummyMode::RejectPureBootstrap,
        None,
    )
}

/// Stage 14-2 (Issue #14 脈: per-file 小状態フィッティング「⑧ ring 初期内容
/// 汚染統合」)。`compress_okumura_clip/plus1_writetime_descending/ascending`
/// (union257 の中核4系統) の一般化版。`Okumura::new(0x20)` 固定 fill を任意の
/// 初期 ring バイト列 (`init_buf`、`N + F - 1` 要素) に差し替えられる点と、
/// 先読み開始位置 `r_init = N - F` を `r_init_delta` で小さくずらせる点だけが
/// 違う (tie_mode/dummy_mode/cmp_mode/del_mode は writetime 系4関数と同一)。
///
/// 戻り値はトークン列と、エンコード終了時点の ring 内容 (`text_buf[0..N]`)
/// のスナップショット。後者はバッチ内直前ファイルの残留 ring を次ファイルへ
/// 持ち込む仮説 (⑨) の材料に使う。
pub fn compress_okumura_writetime_custom_ring_traced(
    input: &[u8],
    plus1: bool,
    ascending: bool,
    init_buf: [u8; N + F - 1],
    r_init_delta: i32,
) -> (Vec<Token>, Vec<u8>) {
    let tail_mode = if plus1 { TailMode::Plus1 } else { TailMode::Clip };

    let mut st = Okumura::new_from_buf(init_buf);
    st.tie_mode = TieMode::StrictGt;
    st.cmp_mode = CmpMode::Unsigned;
    st.del_mode = DelMode::Predecessor;
    st.write_time_variant = true;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();

    // r_init_delta は「小整数パラメータ」探索用の小さなずれのみを想定する。
    // `insert_node` 内の比較ループが text_buf[r..r+F-1] をマスクなしで直接
    // 読むため `r_init+delta+F-1 <= N-1` (= delta<=0) が必須 (元の r_init=N-F は
    // ちょうどこの上限に選ばれている定数)。呼び出し側は 0 以下の小さな値のみ
    // 渡すこと (正方向はここで panic する)。
    let r_init = (N - F) as i32 + r_init_delta;
    assert!(
        r_init >= 0 && r_init + F as i32 - 1 <= N as i32 - 1,
        "r_init_delta out of safe range: r_init={} (N-F={}, F={})",
        r_init,
        N - F,
        F
    );
    let mut r: i32 = r_init;
    let mut s: i32 = 0;

    let mut input_idx: usize = 0;
    let mut len: usize = 0;
    while len < F && input_idx < input.len() {
        st.text_buf[r as usize + len] = input[input_idx];
        input_idx += 1;
        len += 1;
    }

    if len == 0 {
        return (out, st.text_buf[0..N].to_vec());
    }

    if ascending {
        for k in 0..F as i32 {
            st.insert_node(r + k);
        }
    } else {
        for k in (0..F as i32).rev() {
            st.insert_node(r + k);
        }
    }
    let mut skip_inserts: usize = F - 1;

    loop {
        let cap = match tail_mode {
            TailMode::Clip => Some(len),
            TailMode::Unbounded => None,
            TailMode::Plus1 => Some(len + 1),
        };
        if let Some(cap) = cap {
            if st.match_length as usize > cap {
                st.match_length = cap as i32;
            }
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
            out.push(Token::Literal(st.text_buf[r as usize]));
        } else {
            let pos = (st.match_position as u16) & ((N as u16) - 1);
            out.push(Token::Match {
                pos,
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
            if skip_inserts > 0 {
                skip_inserts -= 1;
            } else {
                st.insert_node(r);
            }
            i += 1;
        }

        while i < last_match_length && len > 0 {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len -= 1;
            if len > 0 {
                if skip_inserts > 0 {
                    skip_inserts -= 1;
                } else {
                    st.insert_node(r);
                }
            }
            i += 1;
        }

        if len == 0 {
            break;
        }
    }

    (out, st.text_buf[0..N].to_vec())
}

/// `compress_okumura_writetime_custom_ring_traced` のトークン列のみ返す
/// 薄いラッパー (ring スナップショットが要らない呼び出し用)。
pub fn compress_okumura_writetime_custom_ring(
    input: &[u8],
    plus1: bool,
    ascending: bool,
    init_buf: [u8; N + F - 1],
    r_init_delta: i32,
) -> Vec<Token> {
    compress_okumura_writetime_custom_ring_traced(input, plus1, ascending, init_buf, r_init_delta).0
}

/// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討) 診断専用。
///
/// near-miss 上位24本 (remaining_tokens=1) の best_variant は
/// `compress_okumura` (dummy挿入+fill=0x20) / `compress_okumura_no_dummy`
/// (dummy挿入なし+fill=0x20) / `compress_okumura_basic_tail1_fill00`
/// (dummy挿入+fill=0x00、RLE時のみ+1 cap) の3系統に限られる (Stage 14-2 台帳)。
/// この3系統を単一実装に統合し、各ステップで cap 適用前の生 match 情報と
/// text_buf の生ウィンドウ (現在の書込み位置 `r` 側 / 候補位置側) を追加で
/// 記録する。既存関数は無変更 (追加のみ)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxBase {
    /// `compress_okumura` 系: F個dummy挿入 + fill=0x20 + cap=len (Clip)。
    Basic,
    /// `compress_okumura_no_dummy` 系: dummy挿入なし + fill=0x20 + cap=len (Clip)。
    NoDummy,
    /// `compress_okumura_basic_tail1_fill00` 系: F個dummy挿入 + fill=0x00、
    /// RLE (match_position == r-1) のときだけ cap=len+1、それ以外は cap=len。
    Fill00,
}

/// `compress_okumura_tax_trace` の1ステップ分の診断情報 (EOF 近傍、
/// `len_residual <= F` のときのみ記録)。
#[derive(Debug, Clone)]
pub struct TaxStep {
    /// このステップ開始時点の書込み位置 `r`。
    pub r: i32,
    /// このステップ開始時点の実残り入力バイト数。
    pub len_residual: usize,
    /// cap 適用前の生 match_length。
    pub raw_len: i32,
    /// cap 適用前の生 match_position (`insert_node` が返した実 index、
    /// N-1 以下に収まっている)。
    pub raw_pos: i32,
    /// cap 適用後に実際に出力された match_length (Literal のときは 1)。
    pub capped_len: i32,
    /// `text_buf[r..r+F-1]` (書込み位置側の生バイト列。実残り分を超えた
    /// 部分は「まだ現ラップで書かれていない ring 残骸」または初期 fill 値)。
    pub window_r: Vec<u8>,
    /// `text_buf[raw_pos..raw_pos+F-1]` (候補位置側の生バイト列。全域が
    /// 過去に実際に書かれた実データ)。
    pub window_pos: Vec<u8>,
}

/// `TaxBase` 3系統統合トレーサ。トークン列と EOF 近傍ステップの診断ログを返す。
pub fn compress_okumura_tax_trace(input: &[u8], base: TaxBase) -> (Vec<Token>, Vec<TaxStep>) {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut steps: Vec<TaxStep> = Vec::new();

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
        return (out, steps);
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        let mp0 = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp0 == r_minus_1;
        let cap: usize = match base {
            TaxBase::Fill00 => {
                if is_rle {
                    (len + 1).min(F)
                } else {
                    len
                }
            }
            TaxBase::Basic | TaxBase::NoDummy => len,
        };

        let raw_len = st.match_length;
        let raw_pos = st.match_position;

        if len <= F {
            let mut window_r = Vec::with_capacity(F);
            let mut window_pos = Vec::with_capacity(F);
            for k in 0..F {
                window_r.push(st.text_buf[r as usize + k]);
                window_pos.push(st.text_buf[raw_pos as usize + k]);
            }
            steps.push(TaxStep {
                r,
                len_residual: len,
                raw_len,
                raw_pos,
                capped_len: 0, // 下で cap 適用後に埋める
                window_r,
                window_pos,
            });
        }

        if st.match_length as usize > cap {
            st.match_length = cap as i32;
        }
        if let Some(last) = steps.last_mut() {
            if last.r == r {
                last.capped_len = st.match_length;
            }
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
        let len_before = len;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        // Fill00 系 (RLE+1) と同じ「cap で len を超えて出力した」ときの終了条件。
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    (out, steps)
}

/// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討)。
///
/// `compress_okumura_tax_trace` (3系統統合トレーサ) による診断で、
/// near-miss上位24本は全て「Leaf の最終トークン長 = 生の (post-hoc clip
/// 前の) match_length + 1」かつ、うち22/24本は Leaf の選ぶ match_position
/// 自体が (F=18固定窓での) 生探索結果と異なることが判明した (
/// `.local_data/stage14_3/final_token_taxonomy.csv`)。
///
/// 本 variant 群の仮説: `insert_node` のノード内比較窓は原典どおり常に
/// `F` 固定ではなく、EOF近傍 (残り入力バイト数 `len < F`) では
/// `min(F, len + search_extra)` に動的に縮む。窓が縮むと BST 探索が
/// 到達する分岐点自体が変わり、tie-break の勝者 (match_position) が
/// 生探索 (F固定) と異なるものになりうる — post-hoc な出力側 clip
/// (`TailMode`) では原理的に再現できない構造 (位置そのものが変わる)。
///
/// 出力側の追加 clip は行わない: `match_length` は `insert_node` の時点で
/// 既に `f_bound` (= 探索時に使った窓幅) 以下に制約されているため、
/// そのまま出力する。
pub fn compress_okumura_eof_search_bound(
    input: &[u8],
    base: TaxBase,
    search_extra: i32,
) -> Vec<Token> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
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

    // f_bound(len) = min(F, max(1, len as i32 + search_extra)) — len>=F の
    // 通常時は常に F (原典と同一)。
    let bound_for = |len: usize| -> usize {
        if len >= F {
            F
        } else {
            (len as i32 + search_extra).clamp(1, F as i32) as usize
        }
    };

    st.f_bound = bound_for(len);
    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        // Fill00 系の RLE+1 相当を、探索窓ベースでも維持する: RLE 候補
        // (match_position == r-1) のときだけ探索窓を1バイト広げる。
        let mp0 = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp0 == r_minus_1;
        if base == TaxBase::Fill00 && is_rle && len < F {
            // RLE 候補は search_extra とは独立に len+1 まで許可 (既存
            // basic_tail1_fill00 の cap 規則を踏襲)。再探索はしない
            // (match_length はすでに f_bound=bound_for(len) で求まって
            // いるため、RLE のときだけ +1 の余地を出力側で足す)。
            if (st.match_length as usize) == bound_for(len) && bound_for(len) < len + 1 {
                st.match_length = (len as i32 + 1).min(F as i32);
            }
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
        let len_before = len;
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
            st.f_bound = bound_for(len);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.f_bound = bound_for(len);
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討) 続き。
///
/// `compress_okumura_eof_search_bound` (探索窓のEOF縮小) だけでは
/// near-miss上位24本の match_position 相違を再現できなかった (0/24)。
/// これは「同じ BST 状態から探索しても同じ結果になるはず」という直感に
/// 反するように見えるが、実際には Stage 12 系で調べた比較・タイブレイク
/// 規則そのもの (`CmpMode`/`DelMode`/`TieMode`/`rot_no_delete`/`no_swap`)
/// の相違が、たまたま EOF 直前までは表面化せず、最終トークンでのみ
/// 顕在化している可能性がある。本関数は `TaxBase` (3系統) と、それらの軸
/// + `compress_okumura_eof_search_bound` の探索窓縮小を組み合わせた
/// 直積探索用の統合実装。
#[allow(clippy::too_many_arguments)]
pub fn compress_okumura_stage14_3_sweep(
    input: &[u8],
    base: TaxBase,
    tie_mode: TieMode,
    cmp_mode: CmpMode,
    del_mode: DelMode,
    rot_no_delete: bool,
    no_swap: bool,
    search_extra: i32,
) -> Vec<Token> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = tie_mode;
    st.cmp_mode = cmp_mode;
    st.del_mode = del_mode;
    st.rot_no_delete = rot_no_delete;
    if no_swap {
        st.bst_mode = BstMode::NoSwap;
    }
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

    let bound_for = |len: usize| -> usize {
        if len >= F {
            F
        } else {
            (len as i32 + search_extra).clamp(1, F as i32) as usize
        }
    };

    st.f_bound = bound_for(len);
    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        let mp0 = (st.match_position & (N as i32 - 1)) as usize;
        let r_minus_1 = ((r - 1 + N as i32) & (N as i32 - 1)) as usize;
        let is_rle = mp0 == r_minus_1;
        if base == TaxBase::Fill00 && is_rle && len < F {
            if (st.match_length as usize) == bound_for(len) && bound_for(len) < len + 1 {
                st.match_length = (len as i32 + 1).min(F as i32);
            }
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
        let len_before = len;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            if !st.rot_no_delete {
                st.delete_node(s);
            }
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.f_bound = bound_for(len);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            if !st.rot_no_delete {
                st.delete_node(s);
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.f_bound = bound_for(len);
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討) 続き2。
///
/// `compress_okumura_stage14_3_sweep` (既存 CmpMode/DelMode/TieMode/
/// rot_no_delete/no_swap の直積、全域に適用) では 0/24 だった。BST 非依存
/// オラクル (`lf2_stage14_3_oracle`) の実測により、near-miss上位24本は
/// 「clip 境界 (raw match_length が残り入力バイト数ちょうどまで縮んでいる)
/// で、実データのみで同じ長さに達する候補が数百〜数千件並ぶ大規模タイ」
/// であり、Leaf が選ぶ候補は StrictGt の「木を辿って最初に見つかる」規則
/// とは異なる基準で選ばれていると判明した。
///
/// 本 variant は、EOF近傍 (残り入力バイト数 `len < F`) かつ raw
/// match_length が `len` ちょうどまで clip される局面**だけ**、実データの
/// みで `len` バイト完全一致する候補を全走査し直し、`tie_rule` で勝者を
/// 選び直した上で長さを `len+1` (Stage 9-2 の "+1" 仮説、無条件) に
/// 差し替える。それ以外の局面 (通常時、あるいは raw が `len` に届かない
/// 局面) は既存 Basic/NoDummy/Fill00 と完全に同じ (StrictGt の通常探索)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EofTieRule {
    /// r に最も近い (back distance が最小) 候補。
    ClosestDist,
    /// r から最も遠い候補。
    FarthestDist,
    /// pos (絶対 ring index) が最小の候補。
    SmallestPos,
    /// pos (絶対 ring index) が最大の候補。
    LargestPos,
    /// 最も新しく書き込まれた (write_tick 最大) 候補。
    MostRecentWrite,
    /// 最も古く書き込まれた (write_tick 最小) 候補。
    LeastRecentWrite,
    /// 実データ一致 (len バイト) タイの中で、text_buf 上でさらに F バイトまで
    /// 延長比較したときの一致長 (phantom 込み) が最大の候補。同点なら距離最小。
    MaxPhantomExtension,
    /// Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則)。
    /// タイ再選定発火**前**の生 BST 探索が返す `match_position` (raw_pos、
    /// 自身は候補から除外) に絶対 ring index で最も近い候補。同点
    /// (raw_pos ± 同じ距離に候補がある) は back distance (r に近い方) で
    /// 決める。17本中4本 (C0805/C080D/C1201/C1709) で raw_pos の隣接位置が
    /// 実際の Leaf 選択と一致することが分かった (台帳 ⑲)。
    ClosestToRawPos,
}

pub fn compress_okumura_eof_retie(input: &[u8], base: TaxBase, tie_rule: EofTieRule) -> Vec<Token> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut out: Vec<Token> = Vec::new();
    let mut written = vec![false; N];
    let mut write_tick = vec![0u32; N];
    let mut tick: u32 = 0;

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
    // 初期先読み充填領域 [r, r+len) は「実データ」として書込み済み扱い。
    for k in 0..len {
        written[(r as usize + k) & (N - 1)] = true;
        write_tick[(r as usize + k) & (N - 1)] = tick;
        tick += 1;
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        // EOF境界タイの再選定: len < F かつ raw match_length が len ちょうど
        // (Literal閾値は超えている) のときだけ発火。
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let raw_pos = st.match_position;
            let mut best: Option<(usize, i32)> = None; // (score, pos) — score の意味は tie_rule 依存
            for p in 0..N as i32 {
                if p == r {
                    // 自己参照 (distance=0) は ring が周回して物理スロットを再利用
                    // しているだけの degenerate ケース (text_buf[r] は N 周期前の
                    // 古いデータであり `written` は true だが、自分自身との比較は
                    // 常に自明に一致してしまうため除外する)。
                    continue;
                }
                if tie_rule == EofTieRule::ClosestToRawPos && p == raw_pos {
                    // raw_pos 自身は「タイ再選定」の意味がないので除外
                    // (常に dist_to_raw=0 で自明に勝ってしまう)。
                    continue;
                }
                if !written[p as usize] {
                    continue;
                }
                // 実データのみで len バイト完全一致するか (text_buf は常に
                // 最新の実書込み内容を保持しているので、len 範囲内は必ず実データ)。
                let mut ok = true;
                for j in 0..len {
                    if st.text_buf[p as usize + j] != st.text_buf[r as usize + j] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                let dist = ((r - p) & (N as i32 - 1)) as usize;
                let score = match tie_rule {
                    EofTieRule::ClosestDist => usize::MAX - dist,
                    EofTieRule::FarthestDist => dist,
                    EofTieRule::SmallestPos => usize::MAX - p as usize,
                    EofTieRule::LargestPos => p as usize,
                    EofTieRule::MostRecentWrite => write_tick[p as usize] as usize,
                    EofTieRule::LeastRecentWrite => usize::MAX - write_tick[p as usize] as usize,
                    EofTieRule::MaxPhantomExtension => {
                        let mut j = len;
                        while j < F && st.text_buf[p as usize + j] == st.text_buf[r as usize + j] {
                            j += 1;
                        }
                        // 主キー: phantom 延長長 (大きいほど優先)。副キー: 距離最小。
                        (j << 16) | (usize::MAX - dist).min(0xffff)
                    }
                    EofTieRule::ClosestToRawPos => {
                        let dist_to_raw = {
                            let d = (p - raw_pos) & (N as i32 - 1);
                            (d.min(N as i32 - d)) as usize
                        };
                        // 主キー: raw_pos に近いほど優先。副キー: r に近いほど優先
                        // (raw_pos の両隣が同着したときの決め手、C1709 で確認)。
                        // dist/dist_to_raw は共に < N = 4096 (12bit) なので
                        // 32bit ずつシフトしても usize (64bit) で安全に収まる。
                        ((N - dist_to_raw) << 32) | (N - dist)
                    }
                };
                if best.map(|(bs, _)| score > bs).unwrap_or(true) {
                    best = Some((score, p));
                }
            }
            if let Some((_, p)) = best {
                st.match_position = p;
                st.match_length = (len as i32 + 1).min(F as i32);
            }
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
        let len_before = len;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            written[s as usize] = true;
            write_tick[s as usize] = tick;
            tick += 1;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則) 診断専用。
///
/// `compress_okumura_eof_retie` と同じタイ条件・同じ (overlap を正しく
/// 扱う) 静的窓比較でタイ候補集合を再列挙するが、勝者を選ばず**最後に
/// 発火した局面の候補一覧**をそのまま返す (pos, back distance, write_tick)。
/// 独立の (BST を経由しない) `lf2_stage14_3_oracle` は overlap 候補
/// (dist < len の自己参照RLE的パターン) を正しく判定できない既知の限界が
/// あるため、この関数は実際の候補選定ロジックと同じ静的窓比較
/// (`text_buf[p+j] == text_buf[r+j]`) を再利用し、overlap を含めて正しい
/// タイ集合を返す。
pub fn compress_okumura_eof_retie_last_candidates(
    input: &[u8],
    base: TaxBase,
) -> Option<(i32, usize, i32, Vec<(i32, i32, u32)>)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut written = vec![false; N];
    let mut write_tick = vec![0u32; N];
    let mut tick: u32 = 0;

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
        return None;
    }
    for k in 0..len {
        written[(r as usize + k) & (N - 1)] = true;
        write_tick[(r as usize + k) & (N - 1)] = tick;
        tick += 1;
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last_dump: Option<(i32, usize, i32, Vec<(i32, i32, u32)>)> = None;

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let raw_pos = st.match_position;
            let mut cands: Vec<(i32, i32, u32)> = Vec::new();
            for p in 0..N as i32 {
                if p == r {
                    continue;
                }
                if !written[p as usize] {
                    continue;
                }
                let mut ok = true;
                for j in 0..len {
                    if st.text_buf[p as usize + j] != st.text_buf[r as usize + j] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                let dist = ((r - p) & (N as i32 - 1)) as i32;
                cands.push((p, dist, write_tick[p as usize]));
            }
            last_dump = Some((r, len, raw_pos, cands));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            written[s as usize] = true;
            write_tick[s as usize] = tick;
            tick += 1;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last_dump
}

/// Stage 14-5 (Issue #14 脈: ⑲続 EOF巨大tie ブロック選択規則の特定) 1 候補分の診断情報。
#[derive(Debug, Clone, Copy)]
pub struct Stage145Candidate {
    pub pos: i32,
    pub dist: i32,
    pub write_tick: u32,
    /// 仮説1 (置換セマンティクス): この候補が発火時点で実際に BST に
    /// 生存しているか (`dad[pos] != NIL`)。`written[]` ベースの brute-force
    /// 走査 (実データ一致さえすれば無条件で候補に数える) とは別軸。
    pub in_tree: bool,
    /// 仮説2 (拡張比較長): `len` バイトだけでなく `len+1` バイト目
    /// (宣言される token 長ぶんの phantom byte) まで text_buf の実内容で
    /// 一致するか。
    pub survives_extended: bool,
}

/// Stage 14-5 (Issue #14 脈: ⑲続) 診断専用。
///
/// `compress_okumura_eof_retie_last_candidates` と同じ発火条件・同じ
/// (overlap を正しく扱う) 候補列挙を行うが、各候補に司令塔の2大仮説を
/// 直接検証するためのフラグを付与する:
///
/// - 仮説1「同一文字列は置換」: 候補が発火時点で実際に BST に生存して
///   いるか (`dad[pos] != NIL`)。brute-force 走査は `written[]` (実データ
///   として書かれたか) しか見ないため、既に `insert_node` の EQ 置換で
///   木から追い出されたノードも「候補」に数えてしまっている可能性がある。
/// - 仮説2「比較長を宣言長 (len+1) まで伸ばす」: `len` バイト目の次
///   (index `len`、phantom byte) まで実際の text_buf 内容で一致するか。
///   一致すれば「len バイトだけのタイ」ではなく「len+1 バイトでも本当は
///   タイのまま」であり、逆に不一致ならこの拡張比較で自然に脱落する。
pub fn compress_okumura_eof_retie_probe_stage14_5(
    input: &[u8],
    base: TaxBase,
) -> Option<(i32, usize, i32, Vec<Stage145Candidate>)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut written = vec![false; N];
    let mut write_tick = vec![0u32; N];
    let mut tick: u32 = 0;

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
        return None;
    }
    for k in 0..len {
        written[(r as usize + k) & (N - 1)] = true;
        write_tick[(r as usize + k) & (N - 1)] = tick;
        tick += 1;
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last_dump: Option<(i32, usize, i32, Vec<Stage145Candidate>)> = None;

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let raw_pos = st.match_position;
            let mut cands: Vec<Stage145Candidate> = Vec::new();
            for p in 0..N as i32 {
                if p == r {
                    continue;
                }
                if !written[p as usize] {
                    continue;
                }
                let mut ok = true;
                for j in 0..len {
                    if st.text_buf[p as usize + j] != st.text_buf[r as usize + j] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                let dist = ((r - p) & (N as i32 - 1)) as i32;
                let in_tree = st.dad[p as usize] != NIL;
                // len 番目 (0-index) = 宣言長 (len+1) の追加バイト。境界は
                // p+len, r+len とも text_buf 配列内 (N+F-1 要素) に収まる
                // (p < N, len < F <= 17 => p+len < N+F-1)。
                let survives_extended = st.text_buf[p as usize + len] == st.text_buf[r as usize + len];
                cands.push(Stage145Candidate {
                    pos: p,
                    dist,
                    write_tick: write_tick[p as usize],
                    in_tree,
                    survives_extended,
                });
            }
            last_dump = Some((r, len, raw_pos, cands));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            written[s as usize] = true;
            write_tick[s as usize] = tick;
            tick += 1;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last_dump
}

/// Stage 14-6 (Issue #14 脈: ⑲続々 奥村EOFドレインループの忠実再現) 診断専用。
///
/// 司令塔仮説「EOF近傍で `InsertNode` が縮んでいく実効長で挿入される」を、
/// Stage 14-3 の `compress_okumura_eof_search_bound` と同じ `f_bound`
/// 縮小機構 (`bound_for(len) = min(F, max(1, len + search_extra))`、
/// `len < F` の間だけ発動) を、`len < F` に入った時点以降の**全て**の
/// `insert_node` 呼び出しに適用したうえで、Stage 14-5 と同じ「実データ一致
/// (overlap 込み静的窓比較)・実BST生存フラグ付き」候補列挙を行う。
///
/// Stage 14-3 は縮小窓のまま出力される生の `match_position` (最初に見つかる
/// もの) をそのまま見て 24 本全数で勝者不変 (0/24) だったが、その検証は
/// 「窓を縮めた結果ツリーに残る/追い出されるノード集合そのもの」までは見て
/// いない。本関数は縮小窓ドレインが生む木構造の変化 (EQ 置換によるノード
/// 追い出しのタイミングが早まる/遅まる) が、Stage 14-4/14-5 で確立した
/// block_end-1 選択則の母集団 (`in_tree` 候補のブロック分割) を変えるか
/// どうかを直接見るためのもの。`search_extra=0` のとき「今の実効長 = 残り
/// 入力バイト数ちょうど」という司令塔仮説の最も素直な実装になる。
pub fn compress_okumura_eof_fbound_retie_probe(
    input: &[u8],
    base: TaxBase,
    search_extra: i32,
) -> Option<(i32, usize, i32, Vec<Stage145Candidate>)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

    let mut written = vec![false; N];
    let mut write_tick = vec![0u32; N];
    let mut tick: u32 = 0;

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
        return None;
    }
    for k in 0..len {
        written[(r as usize + k) & (N - 1)] = true;
        write_tick[(r as usize + k) & (N - 1)] = tick;
        tick += 1;
    }

    // f_bound(len) = min(F, max(1, len + search_extra)) — len>=F の通常時は
    // 常に F (原典と同一、ファイル冒頭〜中盤は完全に無変更)。
    let bound_for = |len: usize| -> usize {
        if len >= F {
            F
        } else {
            (len as i32 + search_extra).clamp(1, F as i32) as usize
        }
    };

    st.f_bound = bound_for(len);
    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last_dump: Option<(i32, usize, i32, Vec<Stage145Candidate>)> = None;

    loop {
        // 発火判定・候補列挙そのものは Stage 14-5 と揃える (f_bound 縮小は
        // 「木の形」だけに影響させ、比較可能性を保つ)。
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let raw_pos = st.match_position;
            let mut cands: Vec<Stage145Candidate> = Vec::new();
            for p in 0..N as i32 {
                if p == r {
                    continue;
                }
                if !written[p as usize] {
                    continue;
                }
                let mut ok = true;
                for j in 0..len {
                    if st.text_buf[p as usize + j] != st.text_buf[r as usize + j] {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }
                let dist = ((r - p) & (N as i32 - 1)) as i32;
                let in_tree = st.dad[p as usize] != NIL;
                let survives_extended =
                    st.text_buf[p as usize + len] == st.text_buf[r as usize + len];
                cands.push(Stage145Candidate {
                    pos: p,
                    dist,
                    write_tick: write_tick[p as usize],
                    in_tree,
                    survives_extended,
                });
            }
            last_dump = Some((r, len, raw_pos, cands));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
        let mut i = 0usize;
        while i < last_match_length && input_idx < input.len() {
            st.delete_node(s);
            let c = input[input_idx];
            input_idx += 1;
            st.text_buf[s as usize] = c;
            written[s as usize] = true;
            write_tick[s as usize] = tick;
            tick += 1;
            if (s as usize) < F - 1 {
                st.text_buf[s as usize + N] = c;
            }
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            st.f_bound = bound_for(len);
            st.insert_node(r);
            i += 1;
        }
        while i < last_match_length {
            st.delete_node(s);
            s = (s + 1) & (N as i32 - 1);
            r = (r + 1) & (N as i32 - 1);
            len = len.saturating_sub(1);
            if len > 0 {
                st.f_bound = bound_for(len);
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last_dump
}

/// Stage 14-5 (Issue #14 脈: ⑲続) 診断専用。
///
/// `raw_pos` (タイ再選定発火前の生 BST 探索勝者) を起点に、**実際の木構造**
/// (`raw_pos` と同じ 256分木バケツ = 同じ root byte0 の部分木) 内で in-order
/// 前任/後続方向にそれぞれ `max_hops` 回まで辿り、途中で通過したノード位置
/// (ring 絶対位置) の列を返す。「巨大タイ集合の中の正解ノードは、木構造上
/// raw_pos の近傍 (in-order neighbor) にいる」という仮説の検証用。
pub fn compress_okumura_eof_retie_probe_inorder_neighbors(
    input: &[u8],
    base: TaxBase,
    max_hops: usize,
) -> Option<(i32, i32, i32, Vec<i32>, Vec<i32>)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

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
        return None;
    }
    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last_result: Option<(i32, i32, i32, Vec<i32>, Vec<i32>)> = None;

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let raw_pos = st.match_position;
            let mut succ_chain: Vec<i32> = Vec::new();
            let mut q = raw_pos;
            for _ in 0..max_hops {
                q = st.inorder_successor(q);
                if q == NIL {
                    break;
                }
                succ_chain.push(q);
            }
            let mut pred_chain: Vec<i32> = Vec::new();
            let mut q = raw_pos;
            for _ in 0..max_hops {
                q = st.inorder_predecessor(q);
                if q == NIL {
                    break;
                }
                pred_chain.push(q);
            }
            last_result = Some((r, len as i32, raw_pos, succ_chain, pred_chain));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last_result
}

/// Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則)。
///
/// 「巨大タイ集合から距離/pos/write_tick で選び直す」post-hoc 方式ではなく、
/// 実際のBST構造 (`insert_node` が既に構築した木) を `search_match_readonly`
/// で `AllowEq` タイモードにより**再探索**する方式。`StrictGt` (原典・現行既定)
/// は「木を下りながら最初に見つかった同着」を勝者にするが、`AllowEq` は
/// 「最後に見つかった同着」を勝者にする — 木構造上のタイブレイクという
/// 全く別の原理。EOF境界 (`len < F` かつ raw match_length が `len` ちょうど)
/// でだけ発火し、それ以外は既存 Basic/NoDummy/Fill00 と完全に同じ。
pub fn compress_okumura_eof_retree_allow_eq(input: &[u8], base: TaxBase) -> Vec<Token> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
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

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let (ae_pos, ae_len) = st.search_match_readonly(r, TieMode::AllowEq);
            if ae_len as usize >= len {
                st.match_position = ae_pos;
                st.match_length = (len as i32 + 1).min(F as i32);
            }
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
        let len_before = len;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    out
}

/// Stage 14-4 (Issue #14 脈: ⑲) 診断専用。`compress_okumura_eof_retree_allow_eq`
/// と同じ発火条件で、最後に発火した局面の `(r, len, strict_gt_pos, allow_eq_pos,
/// allow_eq_len)` を返す。
/// Stage 14-4 (Issue #14 脈: ⑲) 診断専用。
///
/// `match_position` (木を下りながら「最初に見つかった同着」) とは別に、
/// `insert_node(r)` が実際に `r` を挿入した**構造上の親ノード位置**
/// (`dad[r]`) を返す。巨大タイ集合の内部では、`match_position` を更新する
/// 条件 (`i > match_length`、厳密不等号) を満たすのは経路上で最初の1回だけ
/// だが、木の物理的な挿入位置は経路の**末端**（NILの子に到達した時点）で
/// 決まるため、両者は別の位置になりうる。`dad[r]` が `N` 以上 (256分木の
/// pseudo-root) の場合は候補として無効 (呼び出し側で除外)。
pub fn probe_eof_attach_point_last(
    input: &[u8],
    base: TaxBase,
) -> Option<(i32, usize, i32, i32)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

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
        return None;
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last: Option<(i32, usize, i32, i32)> = None;

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let attach = st.dad[r as usize];
            last = Some((r, len, st.match_position, attach));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last
}

pub fn probe_eof_retree_allow_eq_last(
    input: &[u8],
    base: TaxBase,
) -> Option<(i32, usize, i32, i32, i32)> {
    let fill = if base == TaxBase::Fill00 { 0x00 } else { 0x20 };
    let mut st = Okumura::new(fill);
    st.tie_mode = TieMode::StrictGt;
    st.init_tree();

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
        return None;
    }

    if base != TaxBase::NoDummy {
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
    }
    st.insert_node(r);

    let mut last: Option<(i32, usize, i32, i32, i32)> = None;

    loop {
        if len < F && (st.match_length as usize) == len && len > THRESHOLD {
            let (ae_pos, ae_len) = st.search_match_readonly(r, TieMode::AllowEq);
            last = Some((r, len, st.match_position, ae_pos, ae_len));
        }

        if (st.match_length as usize) <= THRESHOLD {
            st.match_length = 1;
        }

        let last_match_length = st.match_length as usize;
        let len_before = len;
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
            len = len.saturating_sub(1);
            if len > 0 {
                st.insert_node(r);
            }
            i += 1;
        }
        if input_idx >= input.len() && last_match_length > len_before {
            len = 0;
        }
        if len == 0 {
            break;
        }
    }

    last
}

/// Stage 14-3 (Issue #14 脈: ⑱ EOF終トークン分岐の掃討) 確定名。
///
/// near-miss上位24本の実測 (`compress_okumura_eof_retie` の `ClosestDist`/
/// `FarthestDist` 掃討) で判明した2系統:
/// - RLE隣接 (`match_position == r-1`) の局面では「最も近い距離」で再選定
///   すると 24本中6本 (C0182/C0183/C040E/C040F/C0410/C0411) が反転する。
/// - 非RLEの局面では「最も遠い距離」が 1本 (C1002) を反転させる。
///
/// 以下は `TaxBase` (Basic/NoDummy/Fill00) × 2ルールの直積を union257 全件
/// 候補プールに追加するための命名ラッパー。
pub fn compress_okumura_basic_eof_closest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Basic, EofTieRule::ClosestDist)
}
pub fn compress_okumura_no_dummy_eof_closest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::NoDummy, EofTieRule::ClosestDist)
}
pub fn compress_okumura_fill00_eof_closest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Fill00, EofTieRule::ClosestDist)
}
pub fn compress_okumura_basic_eof_farthest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Basic, EofTieRule::FarthestDist)
}
pub fn compress_okumura_no_dummy_eof_farthest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::NoDummy, EofTieRule::FarthestDist)
}
pub fn compress_okumura_fill00_eof_farthest_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Fill00, EofTieRule::FarthestDist)
}

/// Stage 14-4 (Issue #14 脈: ⑲ 大規模タイ集合内での候補選定規則) 確定名。
///
/// 残17本の実測 (`compress_okumura_eof_retie` の `ClosestToRawPos` 掃討)
/// で判明: 4本 (C0805/C080D/C1201/C1709) は「タイ再選定発火前の生 BST
/// 探索勝者 (raw_pos) の隣接位置 (raw_pos±1)」が実際の Leaf 選択と一致する
/// (raw_pos 自身ではなく、他の候補すべての中で raw_pos に最も近い位置)。
/// `TaxBase` (Basic/NoDummy/Fill00) × 本ルールの直積を union264 候補プールに
/// 追加するための命名ラッパー。
pub fn compress_okumura_basic_eof_closest_to_raw_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Basic, EofTieRule::ClosestToRawPos)
}
pub fn compress_okumura_no_dummy_eof_closest_to_raw_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::NoDummy, EofTieRule::ClosestToRawPos)
}
pub fn compress_okumura_fill00_eof_closest_to_raw_tie(input: &[u8]) -> Vec<Token> {
    compress_okumura_eof_retie(input, TaxBase::Fill00, EofTieRule::ClosestToRawPos)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stage 1 (Issue #14): tree_scan の基本性質。
    /// - 列挙 pos に重複がない
    /// - 列挙集合 == {pos | dad[pos] != NIL} (挿入済みノードが全件列挙される)
    /// - search_trace の訪問集合 ⊆ tree_scan 集合 (pos / depth / 一致長が整合)
    /// - classify_off_path: trace 上のノードは code 0、木内ノードは code != 1
    #[test]
    fn sim_tree_scan_matches_dad_and_covers_search_trace() {
        // 反復のある決定的入力 (マッチと tie が発生する)
        let mut input: Vec<u8> = Vec::new();
        for k in 0..600usize {
            input.push(b'A' + (k % 7) as u8);
            if k % 11 == 0 {
                input.push(b' ');
            }
        }

        for mode in [
            SimMode::Basic,
            SimMode::NoDummy,
            SimMode::DummyThenDrop,
            SimMode::LeftFirst,
        ] {
            let mut sim = OkumuraSim::new(mode, &input);
            // teacher forcing: 全バイトをリテラル相当で 1 byte ずつ進める
            let mut idx = 0usize;
            while idx < input.len() {
                if idx > 0 && idx % 37 == 0 {
                    let scan = sim.tree_scan(sim.r);

                    // 重複なし
                    let mut seen = std::collections::HashSet::new();
                    for &(pos, _, _) in &scan {
                        assert!(seen.insert(pos), "duplicate pos {} in tree_scan", pos);
                    }

                    // dad != NIL の全ノードが列挙される (逆も成り立つ)
                    let dad_set: std::collections::HashSet<u16> = (0..N)
                        .filter(|&p| sim.inner.dad[p] != NIL)
                        .map(|p| p as u16)
                        .collect();
                    assert_eq!(seen, dad_set, "tree_scan != dad set (mode {:?})", mode);

                    // search_trace ⊆ tree_scan (max_len を複数試す)
                    for max_len in [3u8, 4, 18] {
                        let trace = sim.search_trace(sim.r, max_len);
                        for &(pos, _rank, depth) in &trace {
                            let hit = scan.iter().find(|s| s.0 == pos).unwrap_or_else(|| {
                                panic!("trace pos {} not in tree_scan (mode {:?})", pos, mode)
                            });
                            assert_eq!(hit.2, depth, "depth mismatch pos {}", pos);
                            assert_eq!(hit.1, max_len, "match_len mismatch pos {}", pos);
                            let (code, _) = sim.classify_off_path(sim.r, pos);
                            assert_eq!(code, 0, "trace pos {} should be on path", pos);
                        }
                    }

                    // 木内ノードは classify で「不在(1)」にならない
                    for &(pos, _, _) in &scan {
                        let (code, _) = sim.classify_off_path(sim.r, pos);
                        assert_ne!(code, 1, "in-tree pos {} classified as absent", pos);
                    }
                }
                sim.advance(&input[idx..idx + 1]);
                idx += 1;
            }
        }
    }

    /// Stage 3: full-F tie の override で min-age (最も新しく書かれた) 候補が
    /// 選ばれることを確認する。
    ///
    /// 入力 = P(18byte 固有パターン) + P + 固有 filler 30byte + P。
    /// 3 回目の P に対する full-F 候補は書込み済み 2 箇所のみ:
    /// - 1 回目の P: ring 4078..4095 (tick 0..17)  → age 66
    /// - 2 回目の P: ring 0..17    (tick 18..35) → age 48 (min)
    /// n_max=2 (<=32)・min age 一意なので override が発動し pos=0 を選ぶ。
    #[test]
    fn rank1_minage_full_f_override_picks_most_recent_write() {
        let p: Vec<u8> = (0..F as u8).map(|i| 0x41 + i).collect();
        let filler: Vec<u8> = (0..30u8).map(|i| 0xC0 + i).collect();
        let mut input = Vec::new();
        input.extend_from_slice(&p);
        input.extend_from_slice(&p);
        input.extend_from_slice(&filler);
        input.extend_from_slice(&p);
        let toks = compress_okumura_rank1_minage(&input);
        // 最後のトークンが 3 回目の P の full-F match で、min-age の 2 回目
        // コピー (ring pos 0) を指すこと
        let last = *toks.last().unwrap();
        assert_eq!(
            last,
            Token::Match {
                pos: 0,
                len: F as u8
            }
        );
    }

    /// Stage 3: 巨大 tie (n_max > 32, 全候補未書込みの縮退 tie) では override
    /// せず Basic (insert_node) の選択を維持すること。
    #[test]
    fn rank1_minage_degenerate_huge_tie_keeps_basic_choice() {
        let input = vec![b' '; 54];
        let a = compress_okumura(&input);
        let b = compress_okumura_rank1_minage(&input);
        assert_eq!(a, b);
    }

    /// Stage 3: full-F match が出ない入力では Basic (compress_okumura) と
    /// 完全一致すること (override は F 限定)。
    #[test]
    fn rank1_minage_matches_basic_when_no_full_f() {
        // 擬似乱数 (LCG) に短い反復を混ぜる。18 連続一致は出ない
        let mut input: Vec<u8> = Vec::new();
        let mut x: u32 = 12345;
        for k in 0..600usize {
            x = x.wrapping_mul(1103515245).wrapping_add(12345);
            input.push((x >> 16) as u8);
            if k % 40 == 0 {
                // 短いマッチ (len 4) を誘発する反復
                input.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
            }
        }
        let a = compress_okumura(&input);
        let b = compress_okumura_rank1_minage(&input);
        assert!(
            a.iter()
                .all(|t| !matches!(t, Token::Match { len, .. } if *len as usize == F)),
            "test input must not produce full-F matches"
        );
        assert_eq!(a, b);
    }

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
            other => panic!(
                "token 26 expected Match {{ pos=4078, len=18 }}, got {:?}",
                other
            ),
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
        assert_eq!(
            decoded, input,
            "lazy round-trip on ABC..Z*8 must reproduce input"
        );
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

    /// Stage 1 (Issue #14): classify_off_path の code 3/4 の向きを、
    /// 手組みした既知の小さな木で直接 assert する。
    ///
    /// key = "A" + "B"*17。NoDummy の new() 直後は root('A') の右子に
    /// r=4078 だけが居る。探索は root→右→r で、r 上で key と自分自身の
    /// 比較 (cmp=0) により Standard 規則で右へ降りる。そこへ:
    /// - lson[r] = 100      → 探索は右・pos は左部分木 → code 4
    /// - rson[r] = 200      (text_buf[201]='C' で cmp<0 → 探索は左へ)
    ///   - rson[200] = 300  → 探索は左・pos は右部分木 → code 3
    /// - dad==NIL の 500    → code 1 (不在)
    /// - root('Z') 配下の 600 → code 2 (root byte 不一致)
    #[test]
    fn classify_off_path_reports_divergence_direction() {
        let mut input = vec![b'B'; 20];
        input[0] = b'A';
        let mut sim = OkumuraSim::new(SimMode::NoDummy, &input);
        let r = sim.r; // 4078
        assert_eq!(r as usize, N - F);

        let st = &mut sim.inner;
        // 手組み: r の左子 100、右子 200、200 の右子 300
        st.lson[r as usize] = 100;
        st.dad[100] = r;
        st.lson[100] = NIL;
        st.rson[100] = NIL;
        st.rson[r as usize] = 200;
        st.dad[200] = r;
        st.lson[200] = NIL;
        st.rson[200] = 300;
        st.dad[300] = 200;
        st.lson[300] = NIL;
        st.rson[300] = NIL;
        // ノード 200 の内容: byte0='A'、byte1='C' (> key の 'B') → cmp<0 で探索は左へ
        st.text_buf[200] = b'A';
        st.text_buf[201] = b'C';
        // 別 root ('Z') 配下のノード 600
        let root_z = N + 1 + b'Z' as usize;
        st.rson[root_z] = 600;
        st.dad[600] = root_z as i32;
        st.lson[600] = NIL;
        st.rson[600] = NIL;

        // 探索経路上 (r 自身)
        assert_eq!(sim.classify_off_path(r, r as u16), (0, 255));
        // code 4: 分岐ノード r (depth 1) で探索は右、pos=100 は左部分木
        assert_eq!(sim.classify_off_path(r, 100), (4, 1));
        // code 3: 分岐ノード 200 (depth 2) で探索は左、pos=300 は右部分木
        assert_eq!(sim.classify_off_path(r, 300), (3, 2));
        // code 1: dad==NIL → 木に不在
        assert_eq!(sim.classify_off_path(r, 500), (1, 255));
        // code 2: root byte 不一致 ('Z' 配下)
        assert_eq!(sim.classify_off_path(r, 600), (2, 255));
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

    /// Issue #14 v12: OkumuraSim の自己検証。
    ///
    /// 合成データを LF2 トークン化（compress_okumura の出力を teacher に流用。
    /// decode すると元入力に戻るので teacher forcing の入力として妥当）し、
    /// 4 モードの OkumuraSim について:
    /// - search_trace が返す pos 集合 ⊆ enumerate_match_candidates_with_writeback
    ///   の max_len 候補集合
    /// - 全 token で sim.r == ring ループの r
    /// - advance 後も BST の親子リンク整合が保たれる
    #[test]
    fn okumura_sim_trace_subset_and_tree_consistency() {
        use crate::formats::toheart::lf2_tokens::enumerate_match_candidates_with_writeback;
        use std::collections::HashSet;

        // 低エントロピー合成入力: 小さいアルファベット + 周期性で tie を多発させる
        let mut input: Vec<u8> = Vec::new();
        let mut lcg: u32 = 0x1234_5678;
        for i in 0..600usize {
            lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let b = match i % 7 {
                0..=3 => (i % 4) as u8 + 0x10,          // 周期パターン
                _ => ((lcg >> 24) & 0x03) as u8 + 0x10, // 4 値ノイズ
            };
            input.push(b);
        }

        let teacher = compress_okumura(&input);
        // teacher が入力を正しく復元することを前提確認
        assert_eq!(decode_oku_tokens(&teacher), input);

        let mut sims = [
            OkumuraSim::new(SimMode::Basic, &input),
            OkumuraSim::new(SimMode::NoDummy, &input),
            OkumuraSim::new(SimMode::DummyThenDrop, &input),
            OkumuraSim::new(SimMode::LeftFirst, &input),
        ];

        let mut ring = [0x20u8; N];
        let mut r_ring: usize = N - F;
        let mut input_pos: usize = 0;
        let mut trace_hits = 0usize;

        for tok in &teacher {
            let l = match tok {
                Token::Literal(_) => 1usize,
                Token::Match { len, .. } => *len as usize,
            };

            let candidates =
                enumerate_match_candidates_with_writeback(&ring, &input, input_pos, r_ring);
            let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
            if max_len >= 3 {
                let cand_pos: HashSet<u16> = candidates
                    .iter()
                    .filter(|c| c.len == max_len)
                    .map(|c| c.pos)
                    .collect();
                for sim in &sims {
                    let mut seen_rank = 0u32;
                    for (pos, rank, depth) in sim.search_trace(sim.r, max_len) {
                        assert!(
                            cand_pos.contains(&pos),
                            "trace pos 0x{:03x} not in enumerate max_len candidates \
                             (mode {:?}, input_pos {}, max_len {})",
                            pos,
                            sim.mode,
                            input_pos,
                            max_len
                        );
                        assert_eq!(rank, seen_rank + 1, "rank must be 1-based sequential");
                        seen_rank = rank;
                        assert!(depth >= 1);
                        trace_hits += 1;
                    }
                }
            }

            let end = (input_pos + l).min(input.len());
            let emitted = &input[input_pos..end];
            for sim in &mut sims {
                sim.advance(emitted);
            }
            for &b in emitted {
                ring[r_ring] = b;
                r_ring = (r_ring + 1) & (N - 1);
            }
            input_pos = end;

            for sim in &sims {
                assert_eq!(
                    sim.r as usize, r_ring,
                    "sim.r desync (mode {:?}, input_pos {})",
                    sim.mode, input_pos
                );
                assert!(
                    sim.tree_is_consistent(),
                    "BST inconsistent after advance (mode {:?}, input_pos {})",
                    sim.mode,
                    input_pos
                );
            }
        }
        assert_eq!(input_pos, input.len());
        // trace が一度も候補を返さないならテストとして無意味なので下限を張る
        assert!(trace_hits > 0, "no trace hits: synthetic input too random");
    }

    // ------------------------------------------------------------------
    // Issue #14 v12: OkumuraSim 追加テスト群
    //
    // 既存の okumura_sim_trace_subset_and_tree_consistency がカバーする観点
    // (trace ⊆ enumerate、rank 連番、r 同期、BST 整合) は重複させない。
    // ここでは rank1=採用ノード一致・advance 全過程一致・境界・事故パターンを
    // 1 テスト 1 観点で検証する。
    //
    // 観点 9 (mirror 境界 text_buf[s+N] の overlap 複製) は
    // sim_advance_matches_original_encode_full_process の text_buf 全域比較
    // (mirror 領域 N..N+F-1 込み) でカバーされるため独立テストは持たない。
    // ------------------------------------------------------------------

    /// tie を多発させる低エントロピー合成入力（既存整合テストと同じ生成器）。
    fn tie_heavy_input(n: usize) -> Vec<u8> {
        let mut input: Vec<u8> = Vec::new();
        let mut lcg: u32 = 0x1234_5678;
        for i in 0..n {
            lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let b = match i % 7 {
                0..=3 => (i % 4) as u8 + 0x10,
                _ => ((lcg >> 24) & 0x03) as u8 + 0x10,
            };
            input.push(b);
        }
        input
    }

    /// token の出力バイト数。
    fn tok_len(t: &Token) -> usize {
        match t {
            Token::Literal(_) => 1,
            Token::Match { len, .. } => *len as usize,
        }
    }

    /// 観点 1 (最重要): search_trace の rank1 が原典 insert_node の採用ノードと
    /// 一致する。各 tick 直前の `inner.match_position` / `match_length` は
    /// 直前 insert_node(r) の探索結果そのもの (= 原典の採用) なので、
    /// クローン再実行と等価な照合になる。4 モード全部。
    ///
    /// match_length == F の場合だけは insert_node が採用ノード p を r で
    /// 置換してから返るため、trace の rank1 は p の位置を継いだ r 自身になる
    /// (trace は r を full-match として踏む)。
    #[test]
    fn sim_rank1_matches_insert_node_adoption_all_modes() {
        let input = tie_heavy_input(600);
        let teacher = compress_okumura(&input);
        assert_eq!(decode_oku_tokens(&teacher), input);

        for mode in [
            SimMode::Basic,
            SimMode::NoDummy,
            SimMode::DummyThenDrop,
            SimMode::LeftFirst,
        ] {
            let mut sim = OkumuraSim::new(mode, &input);
            let mut input_pos = 0usize;
            let mut checks = 0usize;

            for (ti, tok) in teacher.iter().enumerate() {
                // DummyThenDrop の tick 1 だけは、直前 insert_node の結果が
                // dummy 一斉削除の前に計算されているため trace と食い違い得る。
                let skip = matches!(mode, SimMode::DummyThenDrop) && ti == 1;
                if sim.len > 0 && !skip {
                    let ml = sim.inner.match_length;
                    if ml >= 3 && (ml as usize) <= F {
                        let trace = sim.search_trace(sim.r, ml as u8);
                        let &(pos, rank, _depth) = trace.first().unwrap_or_else(|| {
                            panic!(
                                "adopted node must appear in trace \
                                 (mode {:?}, token {}, ml {})",
                                mode, ti, ml
                            )
                        });
                        assert_eq!(rank, 1);
                        if ml as usize == F {
                            assert_eq!(
                                pos as i32, sim.r,
                                "full-F: rank1 must be r (replacement of adopted p) \
                                 (mode {:?}, token {})",
                                mode, ti
                            );
                        } else {
                            assert_eq!(
                                pos as i32, sim.inner.match_position,
                                "rank1 pos != insert_node adoption \
                                 (mode {:?}, token {}, ml {})",
                                mode, ti, ml
                            );
                        }
                        checks += 1;
                    }
                }

                let l = tok_len(tok);
                let end = (input_pos + l).min(input.len());
                sim.advance(&input[input_pos..end]);
                input_pos = end;
            }
            assert!(checks > 0, "no rank1 checks exercised (mode {:?})", mode);
        }
    }

    /// 観点 12 (advance 順序の全過程一致): Basic モードの advance が原典
    /// compress_okumura の Encode() 実行過程と token ごとに BST 完全状態
    /// (dad/lson/rson/text_buf mirror 込み) と r/s/len/match 結果まで一致する。
    /// 回転順序 (DeleteNode(s) → 書込 → InsertNode(r)) のズレの最強検出器。
    /// 観点 9 (mirror 境界) は text_buf 全域比較に含まれる。
    #[test]
    fn sim_advance_matches_original_encode_full_process() {
        let input = tie_heavy_input(700);

        // 原典 compress_okumura_impl(StrictGt) の逐語再現 (snapshot 付き)
        let mut st = Okumura::new(0x20);
        st.tie_mode = TieMode::StrictGt;
        st.init_tree();
        let mut r: i32 = (N - F) as i32;
        let mut s: i32 = 0;
        let mut input_idx: usize = 0;
        let mut len: usize = 0;
        while len < F && input_idx < input.len() {
            st.text_buf[r as usize + len] = input[input_idx];
            input_idx += 1;
            len += 1;
        }
        assert!(len > 0);
        for i in 1..=F {
            st.insert_node(r - i as i32);
        }
        st.insert_node(r);

        let mut sim = OkumuraSim::new(SimMode::Basic, &input);

        let compare = |st: &Okumura, sim: &OkumuraSim, tick: usize| {
            assert!(
                st.text_buf[..] == sim.inner.text_buf[..],
                "text_buf (mirror 込み) mismatch at token {}",
                tick
            );
            assert!(
                st.dad[..] == sim.inner.dad[..],
                "dad mismatch at token {}",
                tick
            );
            assert!(
                st.lson[..] == sim.inner.lson[..],
                "lson mismatch at token {}",
                tick
            );
            assert!(
                st.rson[..] == sim.inner.rson[..],
                "rson mismatch at token {}",
                tick
            );
        };
        compare(&st, &sim, 0);
        assert_eq!(sim.r, r);
        assert_eq!(sim.s, s);
        assert_eq!(sim.len, len);

        let mut out_pos = 0usize;
        let mut tick = 0usize;
        loop {
            if st.match_length as usize > len {
                st.match_length = len as i32;
            }
            if (st.match_length as usize) <= THRESHOLD {
                st.match_length = 1;
            }
            let last_match_length = st.match_length as usize;
            let emitted = &input[out_pos..out_pos + last_match_length];
            out_pos += last_match_length;

            // 原典側の回転
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

            // sim 側は teacher forcing で同じバイト列を流す
            sim.advance(emitted);
            tick += 1;

            compare(&st, &sim, tick);
            assert_eq!(sim.r, r, "r mismatch at token {}", tick);
            assert_eq!(sim.s, s, "s mismatch at token {}", tick);
            assert_eq!(sim.len, len, "len mismatch at token {}", tick);
            if len > 0 {
                assert_eq!(
                    sim.inner.match_length, st.match_length,
                    "match_length mismatch at token {}",
                    tick
                );
                assert_eq!(
                    sim.inner.match_position, st.match_position,
                    "match_position mismatch at token {}",
                    tick
                );
            }

            if len == 0 {
                break;
            }
        }
        assert_eq!(out_pos, input.len());
    }

    /// 観点 10: teacher forcing 違反 (lookahead と食い違うバイト) は
    /// debug ビルドで debug_assert により panic する。
    /// 注: release ビルドでは debug_assert が消えるため検出されない。
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "emitted byte")]
    fn sim_advance_panics_on_teacher_forcing_violation_in_debug() {
        let input = b"abcdef";
        let mut sim = OkumuraSim::new(SimMode::Basic, input);
        // lookahead 先頭は 'a' なのに 'x' を流す
        sim.advance(&[b'x']);
    }

    /// 観点 13: emitted が残フレーム (len) より長い前提違反は、入力枯渇後の
    /// tail ループで len が underflow し debug ビルドで panic する（仕様として固定）。
    /// 注: release ビルドでは wrap して検出されない。
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "subtract with overflow")]
    fn sim_advance_overlong_emitted_underflows_in_debug() {
        let input = b"ABCDE";
        let mut sim = OkumuraSim::new(SimMode::Basic, input);
        // 残フレーム 5 バイトに対して 9 バイト分の回転を要求する
        let mut emitted = input.to_vec();
        emitted.extend_from_slice(&[0u8; 4]);
        sim.advance(&emitted);
    }

    /// 観点 6: max_len 境界 3 / F-1 / F のそれぞれで rank1 = 採用ノード一致が
    /// 実際に踏まれることを確認する（一致長を狙って作った入力で観測必須にする）。
    #[test]
    fn sim_rank1_at_max_len_boundaries() {
        // len=3: "XYZ" 再出現、直後バイトが異なるので一致はちょうど 3 で止まる
        let mut in3: Vec<u8> = Vec::new();
        in3.extend_from_slice(b"XYZ");
        in3.extend_from_slice(b"abcdefg");
        in3.extend_from_slice(b"XYZ");
        in3.extend_from_slice(b"mnopqrstuv");

        // len=F-1=17: 17 バイト列 S + 区切り + S + 異なる続き
        let s17: Vec<u8> = (0x30..0x30 + 17).collect();
        let mut in17 = s17.clone();
        in17.push(0x21);
        in17.extend_from_slice(&s17);
        in17.extend_from_slice(&[0x60, 0x61, 0x62, 0x63, 0x64]);

        // len=F=18: 18 バイト列を 2 回 + 続き
        let s18: Vec<u8> = (0x41..0x41 + 18).collect();
        let mut in18 = s18.clone();
        in18.extend_from_slice(&s18);
        in18.extend_from_slice(&[0x70, 0x71, 0x72, 0x73]);

        for (input, target) in [(&in3, 3usize), (&in17, F - 1), (&in18, F)] {
            let teacher = compress_okumura(input);
            assert_eq!(decode_oku_tokens(&teacher), *input);

            let mut sim = OkumuraSim::new(SimMode::Basic, input);
            let mut input_pos = 0usize;
            let mut hit = false;
            for tok in &teacher {
                if sim.len > 0 && sim.inner.match_length as usize == target {
                    let trace = sim.search_trace(sim.r, target as u8);
                    let &(pos, rank, _d) = trace
                        .first()
                        .unwrap_or_else(|| panic!("no trace at target len {}", target));
                    assert_eq!(rank, 1);
                    if target == F {
                        // insert_node は len==F で break して p を r に置換済み。
                        // trace は続行して r 自身 (置換後の採用位置) を rank1 で返す
                        assert_eq!(pos as i32, sim.r, "full-F rank1 must be r");
                    } else {
                        assert_eq!(pos as i32, sim.inner.match_position);
                    }
                    hit = true;
                }
                let l = tok_len(tok);
                let end = (input_pos + l).min(input.len());
                sim.advance(&input[input_pos..end]);
                input_pos = end;
            }
            assert!(hit, "target max_len {} never exercised", target);
        }
    }

    /// 観点 5: 深い退化木で depth が 255 に saturate しても、実在ノードは
    /// rank >= 1 で返り「不在 (trace に現れない → v12 側で rank=0/depth=255)」と
    /// 弁別できる。300 ノードの右一直線チェーンを直接構築して検証する。
    #[test]
    fn sim_trace_depth_saturates_but_rank_marks_presence() {
        let input = vec![b'A'; 20];
        let mut sim = OkumuraSim::new(SimMode::NoDummy, &input);

        // 全 text_buf を 'A' にし、bucket 'A' に右一直線 300 ノードを直接構築
        for b in sim.inner.text_buf.iter_mut() {
            *b = b'A';
        }
        sim.inner.init_tree();
        let mut parent = (N + 1 + b'A' as usize) as i32;
        let chain: Vec<i32> = (100..400).collect();
        for &p in &chain {
            sim.inner.rson[parent as usize] = p;
            sim.inner.lson[p as usize] = NIL;
            sim.inner.rson[p as usize] = NIL;
            sim.inner.dad[p as usize] = parent;
            parent = p;
        }
        assert!(sim.tree_is_consistent());

        // 全ノードが key と full-F 一致 → 全員 j == F で列挙される
        let trace = sim.search_trace(0, F as u8);
        assert_eq!(trace.len(), chain.len());
        for (k, &(pos, rank, depth)) in trace.iter().enumerate() {
            assert_eq!(pos as i32, chain[k]);
            assert_eq!(
                rank,
                (k + 1) as u32,
                "rank must stay 1-based past depth 255"
            );
            let expected_depth = (k + 1).min(255) as u8;
            assert_eq!(depth, expected_depth, "depth must saturate at 255");
        }
        // depth 255 でも rank >= 1 で「実在」と分かるノードが複数ある
        assert!(trace.iter().filter(|t| t.2 == 255).count() >= 40);
        // 不在は空 (v12 は trace_lookup で rank=0/depth=255 に落とす)
        assert!(sim.search_trace(0, 5).is_empty());
    }

    /// 観点 2: search_trace は read-only。前後で BST 状態が完全一致し、
    /// 二重呼び出しも同一結果を返す。
    #[test]
    fn sim_search_trace_is_read_only_and_idempotent() {
        let input = tie_heavy_input(300);
        let teacher = compress_okumura(&input);
        let mut sim = OkumuraSim::new(SimMode::Basic, &input);

        // 途中状態まで進める
        let mut input_pos = 0usize;
        for tok in teacher.iter().take(teacher.len() / 2) {
            let l = tok_len(tok);
            let end = (input_pos + l).min(input.len());
            sim.advance(&input[input_pos..end]);
            input_pos = end;
        }

        let text_before = sim.inner.text_buf.to_vec();
        let dad_before = sim.inner.dad.to_vec();
        let lson_before = sim.inner.lson.to_vec();
        let rson_before = sim.inner.rson.to_vec();

        for max_len in 1..=(F as u8) {
            let t1 = sim.search_trace(sim.r, max_len);
            let t2 = sim.search_trace(sim.r, max_len);
            assert_eq!(t1, t2, "double call must return identical results");
        }

        assert_eq!(text_before, sim.inner.text_buf.to_vec());
        assert_eq!(dad_before, sim.inner.dad.to_vec());
        assert_eq!(lson_before, sim.inner.lson.to_vec());
        assert_eq!(rson_before, sim.inner.rson.to_vec());
    }

    /// 観点 3: Basic と NoDummy で序盤 tick の trace が実際に異なる固定ケース。
    /// 入力先頭を 0x20 にすると key が dummy と同じ 0x20 bucket に入り、
    /// 続きが distinct なので dummy 同士は full-match 置換で潰れず 18 個残る。
    /// (全空白入力だと dummy が互いに full-match して 1 ノードに潰れ、
    /// Basic と NoDummy が同一になってしまう)
    #[test]
    fn sim_mode_basic_vs_no_dummy_traces_differ_early() {
        let mut input = vec![0x20u8];
        input.extend_from_slice(b"BCDEFGHIJKLMNOPQRSTUVW");
        let basic = OkumuraSim::new(SimMode::Basic, &input);
        let nodummy = OkumuraSim::new(SimMode::NoDummy, &input);

        // 一致長 1 (key index1 'B' vs dummy の空白で即不一致): dummy は
        // 左一直線チェーンになり、探索パスは root の dummy r-1 で即右に
        // 逸れるので Basic は r-1 を 1 個踏む。NoDummy は該当ノードが無い
        let tb1 = basic.search_trace(basic.r, 1);
        let tn1 = nodummy.search_trace(nodummy.r, 1);
        assert_eq!(tb1.len(), 1, "Basic must visit the root dummy r-1");
        assert_eq!(tb1[0].0 as i32, basic.r - 1);
        assert!(tn1.is_empty());

        // full-F (r 自身の自明一致): 両方 r を返すが depth が異なる
        // (Basic では r が dummy チェーンの下に付く)
        let tb_f = basic.search_trace(basic.r, F as u8);
        let tn_f = nodummy.search_trace(nodummy.r, F as u8);
        assert_eq!(tb_f.len(), 1);
        assert_eq!(tn_f.len(), 1);
        assert_eq!(tb_f[0].0 as i32, basic.r);
        assert_eq!(tn_f[0].0 as i32, nodummy.r);
        assert_eq!(
            tn_f[0].2, 1,
            "NoDummy: r must sit directly under the bucket root"
        );
        assert!(
            tb_f[0].2 > 1,
            "Basic: r must sit below the dummy chain (depth {} should be > 1)",
            tb_f[0].2
        );
    }

    /// 観点 7: 入力長境界。空入力は new が panic せず trace 空・advance no-op、
    /// F-1 / F / F+1 は全 token 処理後まで r 同期と BST 整合が保たれる。
    #[test]
    fn sim_input_length_boundaries() {
        // 空入力
        for mode in [
            SimMode::Basic,
            SimMode::NoDummy,
            SimMode::DummyThenDrop,
            SimMode::LeftFirst,
        ] {
            let mut sim = OkumuraSim::new(mode, &[]);
            assert!(sim.search_trace(sim.r, F as u8).is_empty());
            let r0 = sim.r;
            sim.advance(&[]);
            sim.advance(b"x"); // len==0 なので no-op
            assert_eq!(sim.r, r0, "advance on empty input must be a no-op");
            assert!(sim.tree_is_consistent());
        }

        // F-1 / F / F+1
        for n in [F - 1, F, F + 1] {
            let input: Vec<u8> = (0..n).map(|i| (i % 5) as u8 + 0x10).collect();
            let teacher = compress_okumura(&input);
            assert_eq!(decode_oku_tokens(&teacher), input);
            let mut sim = OkumuraSim::new(SimMode::Basic, &input);
            let mut input_pos = 0usize;
            let mut r_ring = N - F;
            for tok in &teacher {
                let l = tok_len(tok);
                let end = (input_pos + l).min(input.len());
                sim.advance(&input[input_pos..end]);
                r_ring = (r_ring + (end - input_pos)) & (N - 1);
                input_pos = end;
                assert_eq!(sim.r as usize, r_ring, "r desync (n={})", n);
                assert!(sim.tree_is_consistent(), "BST inconsistent (n={})", n);
            }
            assert_eq!(input_pos, n);
        }
    }

    /// 観点 8: ring wrap。入力を N+α まで進めて r (4095→0) と s の両方が
    /// 一周した後も r 同期・BST 整合・trace が保たれる。
    #[test]
    fn sim_ring_wrap_keeps_consistency() {
        let input = tie_heavy_input(N + 204);
        let teacher = compress_okumura(&input);
        assert_eq!(decode_oku_tokens(&teacher), input);

        let mut sim = OkumuraSim::new(SimMode::Basic, &input);
        let mut input_pos = 0usize;
        let mut r_ring = N - F;
        let mut wrapped = false;
        for tok in &teacher {
            let l = tok_len(tok);
            let end = (input_pos + l).min(input.len());
            sim.advance(&input[input_pos..end]);
            let prev = r_ring;
            r_ring = (r_ring + (end - input_pos)) & (N - 1);
            if r_ring < prev {
                wrapped = true;
            }
            input_pos = end;

            assert_eq!(
                sim.r as usize, r_ring,
                "r desync at input_pos {}",
                input_pos
            );
            if wrapped {
                assert!(
                    sim.tree_is_consistent(),
                    "BST inconsistent after wrap (input_pos {})",
                    input_pos
                );
                // wrap 後も trace は呼べて panic しない
                let _ = sim.search_trace(sim.r, 3);
            }
        }
        assert!(wrapped, "r never wrapped: input too short");
        assert_eq!(input_pos, input.len());
        // s も一周している (入力 > N)
        assert_eq!(sim.s as usize, input.len() & (N - 1));
        assert!(sim.tree_is_consistent());
    }

    /// 観点 11: trace が返す pos は木に実在する後方参照であること。
    /// dist = (r - pos) & (N-1) > 0、かつ dad[pos] != NIL。
    /// 例外は max_len == F のときの r 自身 (key と自明に full 一致するため
    /// trace に現れる。dist == 0 なので後方参照候補としては v12 側で
    /// enumerate 由来の pos 引きから外れる)。
    #[test]
    fn sim_trace_positions_are_backrefs_in_tree() {
        let input = tie_heavy_input(600);
        let teacher = compress_okumura(&input);
        let mask = N as i32 - 1;

        for mode in [SimMode::Basic, SimMode::NoDummy] {
            let mut sim = OkumuraSim::new(mode, &input);
            let mut input_pos = 0usize;
            let mut checked = 0usize;
            for tok in &teacher {
                if sim.len > 0 {
                    for max_len in 3..=(F as u8) {
                        for &(pos, _rank, _depth) in &sim.search_trace(sim.r, max_len) {
                            let dist = (sim.r - pos as i32) & mask;
                            if max_len as usize == F && pos as i32 == sim.r {
                                continue; // 自明な自己 full-match
                            }
                            assert!(
                                dist > 0,
                                "forward/self pos 0x{:03x} in trace (mode {:?}, max_len {})",
                                pos,
                                mode,
                                max_len
                            );
                            assert_ne!(
                                sim.inner.dad[pos as usize], NIL,
                                "trace pos 0x{:03x} not in tree (mode {:?})",
                                pos, mode
                            );
                            checked += 1;
                        }
                    }
                }
                let l = tok_len(tok);
                let end = (input_pos + l).min(input.len());
                sim.advance(&input[input_pos..end]);
                input_pos = end;
            }
            assert!(checked > 0, "no trace positions checked (mode {:?})", mode);
        }
    }

    /// 観点 14: DummyThenDrop は 0x20 先頭入力で token 0 の回転直後に
    /// dummy が全消滅し、r 自身は木に残る。
    #[test]
    fn sim_dummy_then_drop_removes_all_dummies_after_token0() {
        let input = vec![0x20u8; 40];
        let teacher = compress_okumura_dummy_then_drop(&input);
        assert_eq!(decode_oku_tokens(&teacher), input);

        let mut sim = OkumuraSim::new(SimMode::DummyThenDrop, &input);
        assert_eq!(sim.dummy_positions.len(), F);

        let l0 = tok_len(&teacher[0]);
        sim.advance(&input[..l0]);

        let dummies = sim.dummy_positions.clone();
        for &p in &dummies {
            if p == sim.r {
                continue;
            }
            assert_eq!(
                sim.inner.dad[p as usize], NIL,
                "dummy 0x{:03x} still in tree after token 0",
                p
            );
        }
        // r 自身は残る
        assert_ne!(
            sim.inner.dad[sim.r as usize], NIL,
            "r itself must survive drop"
        );
        assert!(sim.tree_is_consistent());
    }

    /// 観点 16: NoDummy 序盤では enumerate 側 (ring) に候補があっても
    /// 木に不在で trace が空になる (v12 は rank=0/depth=255 に落とすケース)。
    /// 初期 ring の空白 run は Basic の dummy でしか木に居ない。
    #[test]
    fn sim_no_dummy_early_trace_empty_when_candidate_absent_from_tree() {
        let input = b"AB\x20\x20\x20\x20CD".to_vec();
        let mut basic = OkumuraSim::new(SimMode::Basic, &input);
        let mut nodummy = OkumuraSim::new(SimMode::NoDummy, &input);

        // 'A' 'B' の 2 literal 分進める → 次 tick の key は空白 run
        for i in 0..2 {
            basic.advance(&input[i..i + 1]);
            nodummy.advance(&input[i..i + 1]);
        }
        let max_len = 4u8; // 空白 4 個 + 'C' で一致はちょうど 4
        let tb = basic.search_trace(basic.r, max_len);
        let tn = nodummy.search_trace(nodummy.r, max_len);
        assert!(!tb.is_empty(), "Basic must find dummy space windows");
        assert!(
            tn.is_empty(),
            "NoDummy must report absence (candidate not in tree)"
        );
    }

    /// 観点 17: Literal のみの高エントロピー入力 (全バイト distinct) で
    /// advance が 1 バイトずつ回転し r が毎 token +1 で同期する。
    #[test]
    fn sim_literal_only_input_advances_one_byte_per_token() {
        let input: Vec<u8> = (0u16..=255).map(|b| b as u8).collect();
        let teacher = compress_okumura(&input);
        assert!(
            teacher.iter().all(|t| matches!(t, Token::Literal(_))),
            "distinct-byte input must tokenize to literals only"
        );

        for mode in [
            SimMode::Basic,
            SimMode::NoDummy,
            SimMode::DummyThenDrop,
            SimMode::LeftFirst,
        ] {
            let mut sim = OkumuraSim::new(mode, &input);
            let mut r_ring = N - F;
            for (i, _) in teacher.iter().enumerate() {
                sim.advance(&input[i..i + 1]);
                r_ring = (r_ring + 1) & (N - 1);
                assert_eq!(
                    sim.r as usize, r_ring,
                    "r must advance exactly 1 per literal (mode {:?}, token {})",
                    mode, i
                );
            }
            assert!(sim.tree_is_consistent());
        }
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

    /// Stage 9-2 (Issue #14): `compress_okumura_tail_plus1` は末尾で
    /// `match_length` を `remaining + 1` までしか許容しない (Clip の `remaining`
    /// より 1 大きいだけで、無制限ではない)。
    ///
    /// "AB AB" は末尾 "AB" が先頭 "AB" の再掲で、text_buf の overlap 領域が
    /// (`Okumura::new(0x20)` の) スペース初期値のため、tree 上の一致は実入力の
    /// 3 バイト ("AB " まで) + 1 バイトの overlap 一致で raw_match_length=4 まで
    /// 伸びる。Clip は remaining=3 に切り詰めて `Match{len:3}`、Plus1 は
    /// remaining+1=4 まで許容して `Match{len:4}` になる。
    #[test]
    fn plus1_clips_to_remaining_plus_one_not_unbounded() {
        let input = b"AB AB";
        let clip = compress_okumura(input);
        let (plus1, trace) = compress_okumura_tail_plus1_traced(input);

        assert_eq!(
            decode_oku_tokens(&clip),
            input,
            "Clip must roundtrip exactly"
        );
        // Plus1 は末尾トークンが実入力より 1 バイト長いため、decode は入力そのままの
        // prefix + overrun 1 バイトになる (Stage 9 の狙いどおり: LF2 decoder は画像の
        // 実ピクセル数までしか読まないので、この overrun バイトは無害)。
        let plus1_decoded = decode_oku_tokens(&plus1);
        assert_eq!(
            &plus1_decoded[..input.len()],
            input,
            "Plus1 decode must reproduce input as a prefix"
        );
        assert_eq!(
            plus1_decoded.len(),
            input.len() + 1,
            "Plus1 overruns by exactly 1 byte (the whole point of remaining+1)"
        );

        let clip_last_len = match clip.last() {
            Some(Token::Match { len, .. }) => *len as usize,
            other => panic!("expected trailing Match in Clip, got {:?}", other),
        };
        let plus1_last_len = match plus1.last() {
            Some(Token::Match { len, .. }) => *len as usize,
            other => panic!("expected trailing Match in Plus1, got {:?}", other),
        };
        let last_step = trace.last().expect("trace must be non-empty");

        assert_eq!(
            clip_last_len, last_step.remaining,
            "Clip clips to remaining"
        );
        assert_eq!(
            plus1_last_len,
            last_step.remaining + 1,
            "Plus1 clips to remaining+1, not unbounded (raw_match_length={})",
            last_step.raw_match_length
        );
        assert!(
            (last_step.raw_match_length as usize) > last_step.remaining,
            "test fixture must exercise an actual clip (raw > remaining), got raw={}",
            last_step.raw_match_length
        );
    }

    /// Stage 9-2c (Issue #14) で確認した閾値越え副作用の回帰テスト。
    ///
    /// `remaining == 2` の局面で raw match が 3 (overlap 領域のスペース初期値
    /// との偶然一致) まで伸びると、Clip は `remaining=2` に切り詰めて
    /// `match_length <= THRESHOLD` (=2) となり Literal 2 個になるが、Plus1 は
    /// `remaining+1=3` を許容するため THRESHOLD を超えて Match(len=3) に化ける。
    /// broken30 の `other_kind_diff` 6 本 (H11/H31/CBAK_05/CMON_03/S29E/S30D) は
    /// すべてこの境界で発生した (Stage 9-2c 実測)。
    #[test]
    fn plus1_can_flip_literal_to_match_at_remaining_two_threshold() {
        let input = b"XXXXXAB AB";
        let clip = compress_okumura(input);
        let (plus1, trace) = compress_okumura_tail_plus1_traced(input);

        assert_eq!(
            decode_oku_tokens(&clip),
            input,
            "Clip must roundtrip exactly"
        );
        let plus1_decoded = decode_oku_tokens(&plus1);
        assert_eq!(
            &plus1_decoded[..input.len()],
            input,
            "Plus1 decode must reproduce input as a prefix (overrun byte follows)"
        );

        // 末尾 2 バイト ("AB") の局面: remaining==2
        let last_step = trace.last().expect("trace must be non-empty");
        assert_eq!(last_step.remaining, 2, "fixture must land on remaining==2");
        assert!(
            last_step.raw_match_length as usize >= last_step.remaining + 1,
            "fixture must have a raw match reaching remaining+1 or beyond (got {})",
            last_step.raw_match_length
        );

        // Clip: remaining=2 は THRESHOLD 以下 → 末尾は Literal 2 個
        let clip_tail = &clip[clip.len() - 2..];
        assert!(
            clip_tail.iter().all(|t| matches!(t, Token::Literal(_))),
            "Clip must fall back to literals at remaining==2, got {:?}",
            clip_tail
        );

        // Plus1: remaining+1=3 は THRESHOLD 超 → 末尾が Match(len=3) に化ける
        match plus1.last() {
            Some(Token::Match { len, .. }) => assert_eq!(*len, 3),
            other => panic!(
                "expected Plus1 to flip to a Match(len=3) at this boundary, got {:?}",
                other
            ),
        }
    }

    /// Stage 10-5 (Issue #14): `DummyMode::RejectBootstrapUnwritten` (v1) は
    /// ブートストラップダミーノード帯 `[N-F-F, N-F-1]` (= `[4060, 4077]`) への
    /// 全域未書込みマッチだけを不採用にする。
    ///
    /// 20 バイトの空白列 (0x20) は、リング初期化そのものが 0x20 のため token 0
    /// の時点で「木に挿入済みのノードは最初の F 個のダミー (r-1..r-F =
    /// 4077..4060) と r=4078 だけ」という奥村ブートストラップの構造がそのまま
    /// 露出し、Basic (Clip, dummy 許可) は帯の中の pos=4060 を選ぶ
    /// (Stage 10-4 で観測した「拒否候補は帯に collapse する」の再現)。
    /// v1 はこれを不採用にし、literal 1 個を挟んで帯の外 (pos=4078、実際の
    /// 先読みバッファ) の候補に切り替わる。
    #[test]
    fn no_bootstrap_v1_rejects_dummy_node_band_match() {
        let input = vec![0x20u8; 20];
        let base = compress_okumura(&input);
        let v1 = compress_okumura_clip_no_bootstrap_v1(&input);

        assert_eq!(decode_oku_tokens(&base), input, "Basic must roundtrip");
        assert_eq!(decode_oku_tokens(&v1), input, "v1 must roundtrip");

        match base.first() {
            Some(Token::Match { pos, .. }) => {
                assert!(
                    (*pos as usize) >= BOOTSTRAP_DUMMY_LO && (*pos as usize) <= BOOTSTRAP_DUMMY_HI,
                    "test fixture must exercise a bootstrap-band match in Basic, got pos={}",
                    pos
                );
            }
            other => panic!("expected Basic to open with a Match, got {:?}", other),
        }

        // v1: 帯内マッチが不採用になり、まず Literal になる
        assert!(
            matches!(v1.first(), Some(Token::Literal(_))),
            "v1 must reject the bootstrap-band match and fall back to Literal first, got {:?}",
            v1.first()
        );
        // その後の Match は帯の外を指す
        if let Some(Token::Match { pos, .. }) = v1.get(1) {
            assert!(
                !((*pos as usize) >= BOOTSTRAP_DUMMY_LO && (*pos as usize) <= BOOTSTRAP_DUMMY_HI),
                "v1's fallback Match must point outside the bootstrap band, got pos={}",
                pos
            );
        }
    }
}
