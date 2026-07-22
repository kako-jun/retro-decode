//! Stage 14-1 (Issue #14 ②): hash-chain 制約探索。
//!
//! Stage 13-5 が特定した逸脱構造（長さ/型は貪欲最長一致で99.97%説明済み、
//! 逸脱は tie_pos_diff = 16.17%、"同長・別位置" に一貫して局在）に対して、
//! 奥村 BST（禁猟区）に代わる **hash-chain 族**（LHarc/LZHUF/LHA 系譜の
//! 実装常道）でタイブレークをモデル化し、byte-exact union への寄与と
//! per-tie 的中率を計測する。
//!
//! 探索軸:
//! - ハッシュ関数: 2バイト直値 / 3バイト（LZHUF風シフト、deflate風ローリング、
//!   3バイト直値=衝突なし）
//! - チェーン挿入順: Front（先頭挿入、新しい方から辿る）/ Back（末尾挿入、
//!   古い方から辿る）
//! - 採択: FirstLongest（最初に見つけた最長を保持）/ LastLongest（同長候補は
//!   最後に見つけたものへ更新）
//! - 打ち切り深さ: 16/32/64/128/無制限（リング全域=N=4096 で天井）
//! - 更新粒度: EveryByte（書込みバイト毎に挿入）/ MatchHeadOnly（マッチの先頭
//!   位置だけ挿入する高速化常道、マッチ内部は挿入省略）
//! - tail 座数系: Clip（残り入力長でクリップ）/ Plus1（残り+1まで許容、Stage
//!   9-2 の既存観測）
//! - bootstrap/dummy 帯の扱い: 初期先読み域の手前 F バイト（0x20 埋め、原典の
//!   dummy F 個挿入に相当）を事前挿入するかどうか
//!
//! 評価:
//! - (a) per-tie 的中率: 522本全件のリアル出力バイト列上で、"同一長タイ
//!   （tie_count>=2）かつ Leaf が greedy 最長を選んだ" イベント集合を、
//!   衝突なし3バイトハッシュ+無制限深さのオラクルチェーンで機械的に列挙し、
//!   各 hash-chain variant 自身の探索（自身のハッシュ/深さ/採択規則）が
//!   その場で Leaf の実際の pos を再現するかを判定する。対照行として
//!   既存の距離タイブレーク規則（最近傍/最遠傍）も同じイベント集合上で
//!   再計測し、Δ の基準を揃える
//! - (b) byte-exact 完全一致ファイル集合と union257 (`.local_data/stage12_18/
//!   union_all.txt`) との差分（純増）
//!
//! usage:
//!   cargo run --release --bin lf2_stage14_1_hashchain -- <LF2_DIR> \
//!       [--out-dir DIR] [--union-file PATH] [--limit N]

use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{Token, F, N};
use retro_decode::formats::toheart::verify_harness::{self, tokens_to_lf2_payload};

const MASK: usize = N - 1;
const NIL: i32 = -1;

// ===================== ハッシュ関数 =====================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HashKind {
    Direct2,
    Lzhuf3,
    Deflate3,
    Direct3,
}

impl HashKind {
    fn all_three_byte() -> [HashKind; 3] {
        [HashKind::Lzhuf3, HashKind::Deflate3, HashKind::Direct3]
    }
    fn table_bits(&self) -> u32 {
        match self {
            HashKind::Direct2 => 16,
            HashKind::Lzhuf3 => 12,
            HashKind::Deflate3 => 15,
            HashKind::Direct3 => 24,
        }
    }
    fn key_bytes(&self) -> usize {
        match self {
            HashKind::Direct2 => 2,
            _ => 3,
        }
    }
    fn hash(&self, b0: u8, b1: u8, b2: u8) -> u32 {
        let (a, b, c) = (b0 as u32, b1 as u32, b2 as u32);
        match self {
            HashKind::Direct2 => (a << 8) | b,
            // LZHUF 系実装常道: シフト4/2/0 の XOR 折り込み
            HashKind::Lzhuf3 => ((a << 4) ^ (b << 2) ^ c) & 0x0fff,
            // deflate 風ローリングハッシュ
            HashKind::Deflate3 => (((a << 5) ^ b) << 5 ^ c) & 0x7fff,
            // 3バイト直値（衝突なし、24bit）
            HashKind::Direct3 => (a << 16) | (b << 8) | c,
        }
    }
    fn name(&self) -> &'static str {
        match self {
            HashKind::Direct2 => "direct2",
            HashKind::Lzhuf3 => "lzhuf3",
            HashKind::Deflate3 => "deflate3",
            HashKind::Direct3 => "direct3exact",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainOrder {
    Front, // 先頭挿入 (LIFO)。新しい方から辿る
    Back,  // 末尾追加 (FIFO)。古い方から辿る
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Adopt {
    First, // 同長は最初に見つけたものを保持
    Last,  // 同長は最後に見つけたものへ上書き
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Granularity {
    EveryByte,
    MatchHeadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TailMode2 {
    Clip,
    Plus1,
}

#[derive(Debug, Clone, Copy)]
struct ChainConfig {
    hash: HashKind,
    order: ChainOrder,
    adopt: Adopt,
    max_chain: usize, // N (=4096) を「無制限」の意味で使う
    gran: Granularity,
    tail: TailMode2,
    dummy_boot: bool,
}

impl ChainConfig {
    fn id(&self) -> String {
        format!(
            "{}_{}_{}_d{}_{}_{}_{}",
            self.hash.name(),
            match self.order {
                ChainOrder::Front => "front",
                ChainOrder::Back => "back",
            },
            match self.adopt {
                Adopt::First => "first",
                Adopt::Last => "last",
            },
            if self.max_chain >= N {
                "inf".to_string()
            } else {
                self.max_chain.to_string()
            },
            match self.gran {
                Granularity::EveryByte => "every",
                Granularity::MatchHeadOnly => "headonly",
            },
            match self.tail {
                TailMode2::Clip => "clip",
                TailMode2::Plus1 => "plus1",
            },
            if self.dummy_boot { "dummy" } else { "nodummy" },
        )
    }
}

// ===================== チェーン構造体 =====================

struct Chain {
    hash: HashKind,
    order: ChainOrder,
    table_size: usize,
    head: Vec<i32>,
    touched: Vec<u32>,
    prevlink: Box<[i32; N]>, // Front モード用
    nextlink: Box<[i32; N]>, // Back モード用
    tail: Vec<i32>,          // Back モード用 (bucket -> 最新挿入位置)
}

impl Chain {
    fn new(hash: HashKind, order: ChainOrder) -> Self {
        let table_size = 1usize << hash.table_bits();
        let tail = if matches!(order, ChainOrder::Back) {
            vec![NIL; table_size]
        } else {
            Vec::new()
        };
        Chain {
            hash,
            order,
            table_size,
            head: vec![NIL; table_size],
            touched: Vec::new(),
            prevlink: Box::new([NIL; N]),
            nextlink: Box::new([NIL; N]),
            tail,
        }
    }

    /// 新しいファイルの処理前に呼ぶ。前ファイルのチェーン内容を消す。
    /// head/tail のうち実際に触れたバケツだけ NIL に戻す（フル配列クリアを避ける）。
    /// prevlink/nextlink は head 経由でしか到達しないため、head を NIL に戻せば
    /// 古い値は事実上到達不能になり、明示リセットは不要。
    fn reset(&mut self) {
        for &b in &self.touched {
            self.head[b as usize] = NIL;
            if matches!(self.order, ChainOrder::Back) {
                self.tail[b as usize] = NIL;
            }
        }
        self.touched.clear();
    }

    fn insert(&mut self, pos: usize, key: u32) {
        let h = (key as usize) % self.table_size;
        match self.order {
            ChainOrder::Front => {
                if self.head[h] == NIL {
                    self.touched.push(h as u32);
                }
                self.prevlink[pos] = self.head[h];
                self.head[h] = pos as i32;
            }
            ChainOrder::Back => {
                let t = self.tail[h];
                if t == NIL {
                    self.head[h] = pos as i32;
                    self.touched.push(h as u32);
                } else {
                    self.nextlink[t as usize] = pos as i32;
                }
                self.nextlink[pos] = NIL;
                self.tail[h] = pos as i32;
            }
        }
    }

    fn walk_start(&self, key: u32) -> i32 {
        let h = (key as usize) % self.table_size;
        self.head[h]
    }

    fn walk_next(&self, pos: usize) -> i32 {
        match self.order {
            ChainOrder::Front => self.prevlink[pos],
            ChainOrder::Back => self.nextlink[pos],
        }
    }
}

/// bootstrap 帯 (常に 0x20 埋め、実データ到達前) の事前挿入専用。内容は
/// 未来永劫変わらないため `ring` から直接読んでよい。
fn insert_dummy(chain: &mut Chain, ring: &[u8; N], p: usize, kb: usize) {
    let b0 = ring[p];
    let b1 = ring[(p + 1) & MASK];
    let b2 = if kb == 3 { ring[(p + 2) & MASK] } else { 0 };
    let key = chain.hash.hash(b0, b1, b2);
    chain.insert(p, key);
}

/// 実データ位置 `p`（常に `p == r`、たった今書いたばかりの位置）の挿入。
/// 原典 Okumura の `text_buf` が常に r より先の F バイトを先読みで保持して
/// いる（＝offline圧縮なので将来バイトは既知）ことを利用し、`ring`（まだ
/// コミットされていないかもしれない）ではなく `input`（絶対オフセット
/// `off`, `off+1`, `off+2`）から直接ハッシュ鍵を作る。これにより distance=1,2
/// のような直近自己参照マッチ（フラットカラー領域の連続ラン等）も、
/// 「まだ ring に書かれていないから」という理由で見逃されなくなる。
/// EOF 以降は 0x20 の仮想パディングとして読む（`get_byte`）。
fn insert_real(chain: &mut Chain, input: &[u8], off: usize, p: usize, kb: usize) {
    let b0 = get_byte(input, off);
    let b1 = get_byte(input, off + 1);
    let b2 = if kb == 3 { get_byte(input, off + 2) } else { 0 };
    let key = chain.hash.hash(b0, b1, b2);
    chain.insert(p, key);
}

/// EOF を越えた読み出しは仮想 0x20 パディングとして扱う（原典の text_buf が
/// 入力終端以降も 0x20 埋めのまま保持される挙動を再現する。Plus1 の +1 バイト
/// 延長が指す先はこの仮想パディング）。
#[inline]
fn get_byte(input: &[u8], idx: usize) -> u8 {
    if idx < input.len() {
        input[idx]
    } else {
        0x20
    }
}

/// 単一候補探索: variant 自身の (ハッシュ/深さ/採択) 規則での最良 (len,pos)。
fn find_best(
    chain: &Chain,
    ring: &[u8; N],
    r: usize,
    input: &[u8],
    s: usize,
    cap_len: usize,
    max_depth: usize,
    adopt: Adopt,
) -> (usize, usize) {
    let kb = chain.hash.key_bytes();
    let max_l = cap_len.min(F);
    if max_l < 3 {
        return (0, 0);
    }
    let b0 = get_byte(input, s);
    let b1 = get_byte(input, s + 1);
    let b2 = if kb == 3 { get_byte(input, s + 2) } else { 0 };
    let key = chain.hash.hash(b0, b1, b2);
    let mut p = chain.walk_start(key);
    let mut walked = 0usize;
    let mut best_len = 0usize;
    let mut best_pos = 0usize;
    let imask = (N as i32) - 1;
    while p != NIL && walked < max_depth {
        let pos = p as usize;
        let dist = ((r as i32 - pos as i32) & imask) as usize;
        if dist > 0 {
            let mut l = 0usize;
            while l < max_l {
                let rb = if l < dist { ring[(pos + l) & MASK] } else { get_byte(input, s + l - dist) };
                if rb != get_byte(input, s + l) {
                    break;
                }
                l += 1;
            }
            if l >= 3 {
                if l > best_len {
                    best_len = l;
                    best_pos = pos;
                } else if l == best_len && matches!(adopt, Adopt::Last) {
                    best_pos = pos;
                }
            }
        }
        p = chain.walk_next(pos);
        walked += 1;
    }
    (best_len, best_pos)
}

/// オラクル探索: 衝突なし3バイトハッシュ+無制限深さで真の (best_len, tie_count,
/// nearest_pos, farthest_pos) を求める。
fn oracle_scan(chain: &Chain, ring: &[u8; N], r: usize, input: &[u8], s: usize, cap_len: usize) -> (usize, usize, usize, usize) {
    let max_l = cap_len.min(F);
    if s >= input.len() {
        // 終端ケース（滅多に起きない、Leaf 側もこの近辺では Match トークンを
        // 出さない想定）は候補ゼロとして扱う
        return (0, 0, 0, 0);
    }
    let b0 = get_byte(input, s);
    let b1 = get_byte(input, s + 1);
    let b2 = get_byte(input, s + 2);
    let key = chain.hash.hash(b0, b1, b2);
    let mut p = chain.walk_start(key);
    let mut walked = 0usize;
    let mut best_len = 0usize;
    let mut tie_count = 0usize;
    let mut nearest_dist = usize::MAX;
    let mut nearest_pos = 0usize;
    let mut farthest_dist = 0usize;
    let mut farthest_pos = 0usize;
    let imask = (N as i32) - 1;
    while p != NIL && walked < N {
        let pos = p as usize;
        let dist = ((r as i32 - pos as i32) & imask) as usize;
        if dist > 0 {
            let mut l = 0usize;
            while l < max_l {
                let rb = if l < dist { ring[(pos + l) & MASK] } else { get_byte(input, s + l - dist) };
                if rb != get_byte(input, s + l) {
                    break;
                }
                l += 1;
            }
            if l >= 3 {
                if l > best_len {
                    best_len = l;
                    tie_count = 1;
                    nearest_dist = dist;
                    nearest_pos = pos;
                    farthest_dist = dist;
                    farthest_pos = pos;
                } else if l == best_len {
                    tie_count += 1;
                    if dist < nearest_dist {
                        nearest_dist = dist;
                        nearest_pos = pos;
                    }
                    if dist > farthest_dist {
                        farthest_dist = dist;
                        farthest_pos = pos;
                    }
                }
            }
        }
        p = chain.walk_next(pos);
        walked += 1;
    }
    (best_len, tie_count, nearest_pos, farthest_pos)
}

// ===================== 自走エンコーダ (union 判定用) =====================

fn compress_variant_with_chain(chain: &mut Chain, input: &[u8], cfg: &ChainConfig) -> Vec<Token> {
    chain.reset();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let kb = cfg.hash.key_bytes();
    let max_depth = cfg.max_chain.min(N);

    if cfg.dummy_boot {
        // 原典 Okumura の text_buf 初期化を再現: dummy 挿入前に [r, r+F-1] を
        // 実入力の先頭 F バイトで先読み充填する（さもないと bootstrap 帯の
        // 末尾側位置が「まだ 0x20 のまま」の誤った鍵で挿入されてしまう）。
        for i in 0..F.min(input.len()) {
            ring[(r + i) % N] = input[i];
        }
        for i in 1..=F {
            let p = (r + N - i) % N;
            insert_dummy(chain, &ring, p, kb);
        }
    }

    let mut out: Vec<Token> = Vec::new();
    while s < input.len() {
        let remaining = input.len() - s;
        let cap_len = match cfg.tail {
            TailMode2::Clip => remaining,
            TailMode2::Plus1 => remaining + 1,
        };
        let (best_len, best_pos) = find_best(chain, &ring, r, input, s, cap_len, max_depth, cfg.adopt);
        if best_len < 3 {
            let b = input[s];
            ring[r] = b;
            insert_real(chain, input, s, r, kb);
            out.push(Token::Literal(b));
            r = (r + 1) % N;
            s += 1;
        } else {
            out.push(Token::Match {
                pos: (best_pos as u16) & ((N as u16) - 1),
                len: best_len as u8,
            });
            for k in 0..best_len {
                ring[r] = get_byte(input, s + k);
                let do_insert = match cfg.gran {
                    Granularity::EveryByte => true,
                    Granularity::MatchHeadOnly => k == 0,
                };
                if do_insert {
                    insert_real(chain, input, s + k, r, kb);
                }
                r = (r + 1) % N;
            }
            s += best_len;
        }
    }
    out
}

// ===================== per-tie 評価 =====================

#[derive(Debug, Clone, Copy)]
struct TieEvent {
    s: usize,
    leaf_len: u8,
    leaf_pos: u16,
    nearest_pos: u16,
    farthest_pos: u16,
}

/// オラクル (衝突なし3バイトハッシュ+無制限深さ+EveryByte+dummy_boot=true) で
/// 「同長タイ (tie_count>=2) かつ Leaf が greedy 最長を選んだ」イベント集合を
/// 522本のリアル出力バイト列上で列挙する。あわせて距離タイブレーク対照
/// (nearest/farthest) の正解位置も記録する。
fn oracle_tie_events(oracle_chain: &mut Chain, ring_input: &[u8], leaf_tokens: &[LeafToken]) -> Vec<TieEvent> {
    oracle_chain.reset();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let kb = 3usize;
    for i in 0..F.min(ring_input.len()) {
        ring[(r + i) % N] = ring_input[i];
    }
    for i in 1..=F {
        let p = (r + N - i) % N;
        insert_dummy(oracle_chain, &ring, p, kb);
    }
    let mut events = Vec::new();
    for tok in leaf_tokens {
        match *tok {
            LeafToken::Literal(b) => {
                ring[r] = b;
                insert_real(oracle_chain, ring_input, s, r, kb);
                r = (r + 1) % N;
                s += 1;
            }
            LeafToken::Match { pos, len } => {
                let remaining = ring_input.len() - s;
                let cap = remaining.min(F);
                let (best_len, tie_count, nearest_pos, farthest_pos) = oracle_scan(oracle_chain, &ring, r, ring_input, s, cap);
                if best_len == len as usize && tie_count >= 2 {
                    events.push(TieEvent {
                        s,
                        leaf_len: len,
                        leaf_pos: pos,
                        nearest_pos: nearest_pos as u16,
                        farthest_pos: farthest_pos as u16,
                    });
                }
                for k in 0..len as usize {
                    ring[r] = get_byte(ring_input, s + k);
                    insert_real(oracle_chain, ring_input, s + k, r, kb);
                    r = (r + 1) % N;
                }
                s += len as usize;
            }
        }
    }
    events
}

/// 与えられた variant 自身の (ハッシュ/深さ/採択/粒度/dummy_boot) 規則で、
/// Leaf の実トークン境界を再生しながら状態を進め、`tie_events` の各イベントで
/// 「その場の自分の探索で Leaf の pos を再現できたか」を判定する。
fn score_combo(chain: &mut Chain, ring_input: &[u8], leaf_tokens: &[LeafToken], cfg: &ChainConfig, tie_events: &[TieEvent]) -> usize {
    chain.reset();
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut s: usize = 0;
    let kb = cfg.hash.key_bytes();
    let max_depth = cfg.max_chain.min(N);
    if cfg.dummy_boot {
        for i in 0..F.min(ring_input.len()) {
            ring[(r + i) % N] = ring_input[i];
        }
        for i in 1..=F {
            let p = (r + N - i) % N;
            insert_dummy(chain, &ring, p, kb);
        }
    }
    let mut ev_idx = 0usize;
    let mut hits = 0usize;
    for tok in leaf_tokens {
        while ev_idx < tie_events.len() && tie_events[ev_idx].s == s {
            let ev = &tie_events[ev_idx];
            let remaining = ring_input.len() - s;
            let cap_len = match cfg.tail {
                TailMode2::Clip => remaining,
                TailMode2::Plus1 => remaining + 1,
            };
            let (ml, mp) = find_best(chain, &ring, r, ring_input, s, cap_len, max_depth, cfg.adopt);
            if ml == ev.leaf_len as usize && mp == ev.leaf_pos as usize {
                hits += 1;
            }
            ev_idx += 1;
        }
        match *tok {
            LeafToken::Literal(b) => {
                ring[r] = b;
                insert_real(chain, ring_input, s, r, kb);
                r = (r + 1) % N;
                s += 1;
            }
            LeafToken::Match { len, .. } => {
                for k in 0..len as usize {
                    ring[r] = get_byte(ring_input, s + k);
                    let do_insert = match cfg.gran {
                        Granularity::EveryByte => true,
                        Granularity::MatchHeadOnly => k == 0,
                    };
                    if do_insert {
                        insert_real(chain, ring_input, s + k, r, kb);
                    }
                    r = (r + 1) % N;
                }
                s += len as usize;
            }
        }
    }
    hits
}

// ===================== ファイル入出力 =====================

struct FileData {
    name: String,
    orig: Vec<u8>,
    ring_input: Vec<u8>,
    leaf_tokens: Vec<LeafToken>,
    tie_events: Vec<TieEvent>,
}

fn load_file(path: &Path) -> Result<(String, Vec<u8>, Vec<u8>, Vec<LeafToken>), String> {
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string();
    let data = fs::read(path).map_err(|e| format!("read fail {}: {}", name, e))?;
    let (width, height, ps) = verify_harness::parse_lf2(&data).ok_or_else(|| format!("parse fail {}", name))?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).map_err(|e| format!("decode fail {}: {}", name, e))?;
    let orig = data[ps..].to_vec();
    Ok((name, orig, decoded.ring_input, decoded.tokens))
}

fn load_union257(path: &Path) -> HashSet<String> {
    fs::read_to_string(path)
        .map(|s| s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

// ===================== combo 生成 =====================

fn coarse_combos() -> Vec<ChainConfig> {
    let mut hashes = vec![HashKind::Direct2];
    hashes.extend(HashKind::all_three_byte());
    let mut out = Vec::new();
    for &hash in &hashes {
        for &order in &[ChainOrder::Front, ChainOrder::Back] {
            for &adopt in &[Adopt::First, Adopt::Last] {
                for &tail in &[TailMode2::Clip, TailMode2::Plus1] {
                    for &dummy_boot in &[true, false] {
                        out.push(ChainConfig {
                            hash,
                            order,
                            adopt,
                            max_chain: 32,
                            gran: Granularity::EveryByte,
                            tail,
                            dummy_boot,
                        });
                    }
                }
            }
        }
    }
    out
}

fn refine_combos(base: &[ChainConfig]) -> Vec<ChainConfig> {
    let mut out = Vec::new();
    for b in base {
        for &depth in &[16usize, 64, 128, N] {
            for &gran in &[Granularity::EveryByte, Granularity::MatchHeadOnly] {
                if depth == 32 && matches!(gran, Granularity::EveryByte) {
                    continue; // 既にコース段で実施済み
                }
                out.push(ChainConfig {
                    hash: b.hash,
                    order: b.order,
                    adopt: b.adopt,
                    max_chain: depth,
                    gran,
                    tail: b.tail,
                    dummy_boot: b.dummy_boot,
                });
            }
        }
    }
    out
}

// ===================== 計測本体 =====================

struct ComboResult {
    cfg: ChainConfig,
    phase: &'static str,
    byte_exact: usize,
    matched: Vec<String>,
    tie_hits: usize,
    tie_total: usize,
    elapsed_ms: u128,
}

fn run_combo(cfg: ChainConfig, phase: &'static str, files: &[FileData]) -> ComboResult {
    let started = Instant::now();
    let mut chain_enc = Chain::new(cfg.hash, cfg.order);
    let mut chain_tie = Chain::new(cfg.hash, cfg.order);
    let mut matched = Vec::new();
    let mut tie_hits = 0usize;
    let mut tie_total = 0usize;
    for f in files {
        let toks = compress_variant_with_chain(&mut chain_enc, &f.ring_input, &cfg);
        let reenc = tokens_to_lf2_payload(&toks);
        if reenc == f.orig {
            matched.push(f.name.clone());
        }
        let hits = score_combo(&mut chain_tie, &f.ring_input, &f.leaf_tokens, &cfg, &f.tie_events);
        tie_hits += hits;
        tie_total += f.tie_events.len();
    }
    ComboResult {
        cfg,
        phase,
        byte_exact: matched.len(),
        matched,
        tie_hits,
        tie_total,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

fn write_csv_row(w: &mut impl Write, r: &ComboResult, union257: &HashSet<String>) -> std::io::Result<()> {
    let new_vs_union: usize = r.matched.iter().filter(|n| !union257.contains(*n)).count();
    let tie_rate = if r.tie_total > 0 { (r.tie_hits as f64) / (r.tie_total as f64) * 100.0 } else { 0.0 };
    writeln!(
        w,
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
        r.phase,
        r.cfg.id(),
        r.cfg.hash.name(),
        match r.cfg.order { ChainOrder::Front => "front", ChainOrder::Back => "back" },
        match r.cfg.adopt { Adopt::First => "first", Adopt::Last => "last" },
        if r.cfg.max_chain >= N { "inf".to_string() } else { r.cfg.max_chain.to_string() },
        match r.cfg.gran { Granularity::EveryByte => "every", Granularity::MatchHeadOnly => "headonly" },
        match r.cfg.tail { TailMode2::Clip => "clip", TailMode2::Plus1 => "plus1" },
        r.cfg.dummy_boot,
        r.byte_exact,
        new_vs_union,
        r.tie_hits,
        r.tie_total,
        format!("{:.4}", tie_rate),
        r.elapsed_ms,
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> [--out-dir DIR] [--union-file PATH] [--limit N]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_dir = PathBuf::from(".local_data/stage14_1");
    let mut union_file = PathBuf::from(".local_data/stage12_18/union_all.txt");
    let mut limit: Option<usize> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out-dir" => {
                if let Some(v) = args.get(i + 1) {
                    out_dir = PathBuf::from(v);
                }
                i += 2;
            }
            "--union-file" => {
                if let Some(v) = args.get(i + 1) {
                    union_file = PathBuf::from(v);
                }
                i += 2;
            }
            "--limit" => {
                if let Some(v) = args.get(i + 1) {
                    limit = v.parse().ok();
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }
    fs::create_dir_all(&out_dir).ok();

    let union257 = load_union257(&union_file);
    if union257.is_empty() {
        eprintln!("warning: union257 file empty/missing at {:?} (Δ計算は全部0として進行)", union_file);
    } else {
        eprintln!("union257 loaded: {} files", union257.len());
    }

    let mut paths: Vec<PathBuf> = match verify_harness::list_lf2_files(&dir, None) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };
    if let Some(n) = limit {
        paths.truncate(n);
    }

    eprintln!("=== ファイル読込 + Leaf トークンデコード ({} files) ===", paths.len());
    let mut files: Vec<FileData> = Vec::with_capacity(paths.len());
    let mut load_errors = 0usize;
    for p in &paths {
        match load_file(p) {
            Ok((name, orig, ring_input, leaf_tokens)) => {
                files.push(FileData {
                    name,
                    orig,
                    ring_input,
                    leaf_tokens,
                    tie_events: Vec::new(),
                });
            }
            Err(e) => {
                eprintln!("{}", e);
                load_errors += 1;
            }
        }
    }
    eprintln!("読込完了: {} 成功, {} 失敗", files.len(), load_errors);

    eprintln!("=== オラクル tie イベント列挙 (衝突なし3バイトハッシュ+無制限深さ) ===");
    let oracle_started = Instant::now();
    let mut oracle_chain = Chain::new(HashKind::Direct3, ChainOrder::Front);
    let mut total_events = 0usize;
    for f in files.iter_mut() {
        let events = oracle_tie_events(&mut oracle_chain, &f.ring_input, &f.leaf_tokens);
        total_events += events.len();
        f.tie_events = events;
    }
    eprintln!(
        "オラクル完了: {} 件のジェニュインタイ (tie_count>=2, agree-length) を {} ms で列挙",
        total_events,
        oracle_started.elapsed().as_millis()
    );

    // 対照行: 既存の距離タイブレーク規則 (最近傍 / 最遠傍) を同一イベント集合で再計測
    let mut nearest_hits = 0usize;
    let mut farthest_hits = 0usize;
    for f in &files {
        for ev in &f.tie_events {
            if ev.nearest_pos == ev.leaf_pos {
                nearest_hits += 1;
            }
            if ev.farthest_pos == ev.leaf_pos {
                farthest_hits += 1;
            }
        }
    }
    let control_rate = |h: usize| if total_events > 0 { (h as f64) / (total_events as f64) * 100.0 } else { 0.0 };
    eprintln!(
        "対照 (同一イベント集合上): nearest-dist的中 {}/{} ({:.2}%), farthest-dist的中 {}/{} ({:.2}%)",
        nearest_hits, total_events, control_rate(nearest_hits),
        farthest_hits, total_events, control_rate(farthest_hits)
    );

    let csv_path = out_dir.join("sweep_results.csv");
    let mut csv = fs::File::create(&csv_path).expect("create sweep_results.csv");
    writeln!(
        csv,
        "phase,variant_id,hash,order,adopt,max_chain,granularity,tail,dummy_boot,byte_exact_count,new_vs_union257,tie_hits,tie_total,tie_hit_rate_pct,elapsed_ms"
    )
    .unwrap();
    // 対照行 (距離タイブレーク) をヘッダ直後に control 行として出力
    writeln!(
        csv,
        "control,nearest_dist,-,-,-,-,-,-,-,-,-,{},{},{:.4},0",
        nearest_hits, total_events, control_rate(nearest_hits)
    )
    .unwrap();
    writeln!(
        csv,
        "control,farthest_dist,-,-,-,-,-,-,-,-,-,{},{},{:.4},0",
        farthest_hits, total_events, control_rate(farthest_hits)
    )
    .unwrap();

    let mut all_matched: HashSet<String> = HashSet::new();
    let mut best_byte_exact: Option<ComboResult> = None;
    let mut best_tie_rate: Option<ComboResult> = None;

    eprintln!("=== Phase 1: 粗格子 (depth=32, EveryByte 固定) ===");
    let coarse = coarse_combos();
    eprintln!("coarse combos: {}", coarse.len());
    let mut coarse_results: Vec<ComboResult> = Vec::new();
    for (idx, cfg) in coarse.into_iter().enumerate() {
        let r = run_combo(cfg, "coarse", &files);
        eprintln!(
            "  [{:>2}] {} byte_exact={} tie={}/{} ({:.2}%) {}ms",
            idx,
            r.cfg.id(),
            r.byte_exact,
            r.tie_hits,
            r.tie_total,
            if r.tie_total > 0 { r.tie_hits as f64 / r.tie_total as f64 * 100.0 } else { 0.0 },
            r.elapsed_ms
        );
        write_csv_row(&mut csv, &r, &union257).unwrap();
        for n in &r.matched {
            all_matched.insert(n.clone());
        }
        if best_byte_exact.as_ref().map(|b| r.byte_exact > b.byte_exact).unwrap_or(true) {
            best_byte_exact = Some(ComboResult {
                cfg: r.cfg,
                phase: r.phase,
                byte_exact: r.byte_exact,
                matched: r.matched.clone(),
                tie_hits: r.tie_hits,
                tie_total: r.tie_total,
                elapsed_ms: r.elapsed_ms,
            });
        }
        coarse_results.push(r);
    }

    // top-5 選定: byte_exact 優先、同値は tie_hit_rate で決着
    coarse_results.sort_by(|a, b| {
        b.byte_exact.cmp(&a.byte_exact).then_with(|| {
            let ra = if a.tie_total > 0 { a.tie_hits as f64 / a.tie_total as f64 } else { 0.0 };
            let rb = if b.tie_total > 0 { b.tie_hits as f64 / b.tie_total as f64 } else { 0.0 };
            rb.partial_cmp(&ra).unwrap()
        })
    });
    let top_bases: Vec<ChainConfig> = coarse_results.iter().take(5).map(|r| r.cfg).collect();
    eprintln!("=== Phase 1 完了。上位5設定を Phase 2 の基底に採用 ===");
    for b in &top_bases {
        eprintln!("  base: {}", b.id());
    }

    eprintln!("=== Phase 2: 細格子 (上位5設定 × depth{{16,64,128,inf}} × granularity{{every,headonly}}) ===");
    let refine = refine_combos(&top_bases);
    eprintln!("refine combos: {}", refine.len());
    for (idx, cfg) in refine.into_iter().enumerate() {
        let r = run_combo(cfg, "refine", &files);
        eprintln!(
            "  [{:>2}] {} byte_exact={} tie={}/{} ({:.2}%) {}ms",
            idx,
            r.cfg.id(),
            r.byte_exact,
            r.tie_hits,
            r.tie_total,
            if r.tie_total > 0 { r.tie_hits as f64 / r.tie_total as f64 * 100.0 } else { 0.0 },
            r.elapsed_ms
        );
        write_csv_row(&mut csv, &r, &union257).unwrap();
        for n in &r.matched {
            all_matched.insert(n.clone());
        }
        if best_byte_exact.as_ref().map(|b| r.byte_exact > b.byte_exact).unwrap_or(true) {
            best_byte_exact = Some(ComboResult {
                cfg: r.cfg,
                phase: r.phase,
                byte_exact: r.byte_exact,
                matched: r.matched.clone(),
                tie_hits: r.tie_hits,
                tie_total: r.tie_total,
                elapsed_ms: r.elapsed_ms,
            });
        }
        if best_tie_rate.as_ref().map(|b| {
            let ra = if r.tie_total > 0 { r.tie_hits as f64 / r.tie_total as f64 } else { 0.0 };
            let rb = if b.tie_total > 0 { b.tie_hits as f64 / b.tie_total as f64 } else { 0.0 };
            ra > rb
        }).unwrap_or(true) {
            best_tie_rate = Some(ComboResult {
                cfg: r.cfg,
                phase: r.phase,
                byte_exact: r.byte_exact,
                matched: r.matched.clone(),
                tie_hits: r.tie_hits,
                tie_total: r.tie_total,
                elapsed_ms: r.elapsed_ms,
            });
        }
    }

    let new_vs_union: Vec<String> = {
        let mut v: Vec<String> = all_matched.iter().filter(|n| !union257.contains(*n)).cloned().collect();
        v.sort();
        v
    };

    eprintln!("=== 総括 ===");
    eprintln!("hash-chain family 全variant 合計 byte-exact union: {} files", all_matched.len());
    eprintln!("union257 に対する純増: {} files", new_vs_union.len());
    for n in &new_vs_union {
        eprintln!("  純増: {}", n);
    }
    if let Some(b) = &best_byte_exact {
        eprintln!("最良 byte_exact 単体: {} ({} files)", b.cfg.id(), b.byte_exact);
    }
    if let Some(b) = &best_tie_rate {
        let rate = if b.tie_total > 0 { b.tie_hits as f64 / b.tie_total as f64 * 100.0 } else { 0.0 };
        eprintln!("最良 tie_hit_rate 単体: {} ({:.2}%, n={})", b.cfg.id(), rate, b.tie_total);
    }

    let summary_path = out_dir.join("summary.txt");
    let mut summary = fs::File::create(&summary_path).expect("create summary.txt");
    writeln!(summary, "files_processed={}", files.len()).unwrap();
    writeln!(summary, "load_errors={}", load_errors).unwrap();
    writeln!(summary, "union257_size={}", union257.len()).unwrap();
    writeln!(summary, "tie_events_total={}", total_events).unwrap();
    writeln!(summary, "control_nearest_hit_rate_pct={:.4}", control_rate(nearest_hits)).unwrap();
    writeln!(summary, "control_farthest_hit_rate_pct={:.4}", control_rate(farthest_hits)).unwrap();
    writeln!(summary, "hashchain_family_union_size={}", all_matched.len()).unwrap();
    writeln!(summary, "new_vs_union257_count={}", new_vs_union.len()).unwrap();
    for n in &new_vs_union {
        writeln!(summary, "new_file={}", n).unwrap();
    }
    if let Some(b) = &best_byte_exact {
        writeln!(summary, "best_byte_exact_variant={}", b.cfg.id()).unwrap();
        writeln!(summary, "best_byte_exact_count={}", b.byte_exact).unwrap();
    }
    if let Some(b) = &best_tie_rate {
        let rate = if b.tie_total > 0 { b.tie_hits as f64 / b.tie_total as f64 * 100.0 } else { 0.0 };
        writeln!(summary, "best_tie_rate_variant={}", b.cfg.id()).unwrap();
        writeln!(summary, "best_tie_rate_pct={:.4}", rate).unwrap();
    }

    eprintln!("出力: {:?}, {:?}", csv_path, summary_path);
    ExitCode::SUCCESS
}
