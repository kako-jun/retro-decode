//! Issue #14 Stage 6: 単一イベント編集の総当たり逆算
//!
//! LF2 (Leaf) と奥村 BST basic の木形状差の因果イベントを特定するため、
//! 「1イベントだけ挙動を変えた」差分再生を機械列挙する。
//!
//! 手順:
//! 1. LF2 をトークン分解し、teacher-forcing で basic BST を全再生して
//!    タイ違反 (rank1 != Leaf採用) を全列挙する
//! 2. 対象違反 ti (既定: 初違反) のリング 1 周窓 (既定 4,114 tick) の
//!    開始トークン境界で状態スナップショットを取る
//! 3. 窓内の全イベント (insert / full-F swap / delete leaf/one/two) を列挙し、
//!    各イベント × 適用可能な編集タイプについて「そのイベントだけ編集した」
//!    再生を行い、(i) 対象 ti が rank1 一致になるか
//!    (ii) 窓内の他の正常 tie を壊さないか を判定する
//!
//! 編集タイプ:
//! - skip_insert:        その挿入を 1 tick 遅延 (次 tick 冒頭で挿入)
//! - skip_delete:        その削除を 1 tick 遅延 (次 tick 冒頭で削除)
//! - del_promote_other:  del-two で前任者 (左部分木最大) でなく後継者 (右部分木最小) を昇格
//! - swap_leaf_attach:   full-F 一致時に位置継承置換せず右へ降り続けて葉として挿入
//!
//! 使い方:
//!   cargo run --release --bin lf2_stage6_event_edit -- \
//!     [--file .local_data/lvns3/C0602.LF2] [--ti 861] [--window 4114] \
//!     [--limit N] [--self-test] [--dump-ties PATH]
//!
//! 注意: 判定は窓内 [snap_ti, target.ti] の tie に限定した割り切り。
//! 「broken=0」は窓内副作用ゼロの意味であり、窓外への影響は測っていない。
//! --ti には違反でない tie も指定できる (編集で正常 tie を壊さないかの検証用途)。

use std::env;
use std::fs;

const N: usize = 4096;
const F: usize = 18;
const NIL: i32 = N as i32;
const LF2_MAGIC: &[u8] = b"LEAF256\0";

// ---------------------------------------------------------------------------
// LF2 パースとトークン分解 (stage4 harness replay.py と同一意味論)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Token {
    Literal(#[allow(dead_code)] u8),
    Match { pos: u16, len: u8 },
}

impl Token {
    fn emit_len(&self) -> usize {
        match self {
            Token::Literal(_) => 1,
            Token::Match { len, .. } => *len as usize,
        }
    }
}

fn parse_lf2(data: &[u8]) -> (Vec<u8>, u16, u16) {
    assert_eq!(&data[..8], LF2_MAGIC, "not an LF2 file");
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16] as usize;
    let start = 0x18 + colors * 3;
    (data[start..].to_vec(), w, h)
}

fn decode_tokens(comp: &[u8], w: u16, h: u16) -> (Vec<Token>, Vec<u8>) {
    let total = w as usize * h as usize;
    let mut ring = vec![0x20u8; 0x1000];
    let mut rp: usize = 0x0fee;
    let mut dp: usize = 0;
    let mut produced: usize = 0;
    let mut flag: u8 = 0;
    let mut fc: u8 = 0;
    let mut tokens = Vec::new();
    let mut ring_input = Vec::with_capacity(total);
    while produced < total {
        if fc == 0 {
            flag = comp[dp] ^ 0xff;
            dp += 1;
            fc = 8;
        }
        if flag & 0x80 != 0 {
            let px = comp[dp] ^ 0xff;
            dp += 1;
            tokens.push(Token::Literal(px));
            ring[rp] = px;
            rp = (rp + 1) & 0xfff;
            ring_input.push(px);
            produced += 1;
        } else {
            let up = comp[dp] ^ 0xff;
            let lo = comp[dp + 1] ^ 0xff;
            dp += 2;
            let ln = (up & 0x0f) as usize + 3;
            let pos = (((up >> 4) as usize) | ((lo as usize) << 4)) & 0xfff;
            tokens.push(Token::Match {
                pos: pos as u16,
                len: ln as u8,
            });
            for k in 0..ln {
                let b = ring[(pos + k) & 0xfff];
                ring[rp] = b;
                rp = (rp + 1) & 0xfff;
                ring_input.push(b);
                produced += 1;
            }
        }
        flag <<= 1;
        fc -= 1;
    }
    (tokens, ring_input)
}

// ---------------------------------------------------------------------------
// イベント編集付き奥村 BST basic シミュレータ (teacher forcing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditType {
    SkipInsert,
    SkipDelete,
    DelPromoteOther,
    SwapLeafAttach,
}

impl EditType {
    const ALL: [EditType; 4] = [
        EditType::SkipInsert,
        EditType::SkipDelete,
        EditType::DelPromoteOther,
        EditType::SwapLeafAttach,
    ];
    fn name(&self) -> &'static str {
        match self {
            EditType::SkipInsert => "skip_insert",
            EditType::SkipDelete => "skip_delete",
            EditType::DelPromoteOther => "del_promote_other",
            EditType::SwapLeafAttach => "swap_leaf_attach",
        }
    }
    /// このイベント種に適用可能か
    fn applicable(&self, kind: EventKind) -> bool {
        match self {
            EditType::SkipInsert => matches!(kind, EventKind::Ins | EventKind::InsSwap),
            EditType::SkipDelete => matches!(
                kind,
                EventKind::DelLeaf | EventKind::DelOne | EventKind::DelTwo
            ),
            EditType::DelPromoteOther => matches!(kind, EventKind::DelTwo),
            EditType::SwapLeafAttach => matches!(kind, EventKind::InsSwap),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    /// 葉として挿入された insert
    Ins,
    /// full-F 一致で置換 (swap) が起きた insert
    InsSwap,
    /// 対象が木に不在 (dad==NIL) だった delete (no-op)
    DelAbsent,
    /// 子 0 個の削除
    DelLeaf,
    /// 子 1 個の削除
    DelOne,
    /// 子 2 個の削除 (前任者昇格)
    DelTwo,
}

#[derive(Debug, Clone, Copy)]
struct Event {
    op_id: u64,
    tick: u64,
    kind: EventKind,
    #[allow(dead_code)]
    node: i32,
}

#[derive(Clone)]
struct Sim {
    text: Vec<u8>,
    lson: Vec<i32>,
    rson: Vec<i32>,
    dad: Vec<i32>,
    input: Vec<u8>,
    idx: usize,
    r: i32,
    s: i32,
    len: usize,
    tick: u64,
    /// insert/delete 呼び出しごとに 1 増える大域 op カウンタ
    op_counter: u64,
    /// 編集対象 op と編集タイプ (None なら無編集再生)
    edit_op: Option<u64>,
    edit_type: EditType,
    /// skip_insert で遅延された挿入ノード (次 tick 冒頭で挿入)
    pending_insert: Option<i32>,
    /// skip_delete で遅延された削除ノード (次 tick 冒頭で削除)
    pending_delete: Option<i32>,
    /// イベント記録先 (enumeration パスのみ Some)
    record: Option<Vec<Event>>,
}

impl Sim {
    fn new(input: Vec<u8>) -> Self {
        let mut sim = Sim {
            text: vec![0x20u8; N + F - 1],
            lson: vec![0i32; N + 257],
            rson: vec![0i32; N + 257],
            dad: vec![0i32; N + 1],
            input,
            idx: 0,
            r: (N - F) as i32,
            s: 0,
            len: 0,
            tick: 0,
            op_counter: 0,
            edit_op: None,
            edit_type: EditType::SkipInsert,
            pending_insert: None,
            pending_delete: None,
            record: None,
        };
        for i in (N + 1)..(N + 257) {
            sim.rson[i] = NIL;
        }
        for i in 0..N {
            sim.dad[i] = NIL;
        }
        while sim.len < F && sim.idx < sim.input.len() {
            sim.text[sim.r as usize + sim.len] = sim.input[sim.idx];
            sim.idx += 1;
            sim.len += 1;
        }
        if sim.len > 0 {
            // 初期化 dummy 挿入は編集対象外なので raw で行う
            for i in 1..=F as i32 {
                sim.insert(sim.r - i, false);
            }
            sim.insert(sim.r, false);
        }
        sim
    }

    /// 奥村原典 InsertNode と同一 (match 更新は teacher forcing のため不要)。
    /// `leaf_attach` が真なら full-F 一致時も置換せず右へ降り続けて葉として挿入する。
    /// 戻り値: full-F 置換 (swap) が起きたか。
    fn insert(&mut self, r: i32, leaf_attach: bool) -> bool {
        let ks = r as usize;
        let mut cmp: i32 = 1;
        let mut p: i32 = N as i32 + 1 + self.text[ks] as i32;
        self.rson[r as usize] = NIL;
        self.lson[r as usize] = NIL;
        loop {
            if cmp >= 0 {
                if self.rson[p as usize] != NIL {
                    p = self.rson[p as usize];
                } else {
                    self.rson[p as usize] = r;
                    self.dad[r as usize] = p;
                    return false;
                }
            } else if self.lson[p as usize] != NIL {
                p = self.lson[p as usize];
            } else {
                self.lson[p as usize] = r;
                self.dad[r as usize] = p;
                return false;
            }
            let mut i: usize = 1;
            cmp = 0;
            while i < F {
                let d = self.text[ks + i] as i32 - self.text[p as usize + i] as i32;
                if d != 0 {
                    cmp = d;
                    break;
                }
                i += 1;
            }
            if i >= F && !leaf_attach {
                break;
            }
            // leaf_attach: cmp==0 のまま右へ降り続ける (葉挿入で return)
        }
        // full-F 一致: p を r で置換 (位置継承)
        self.dad[r as usize] = self.dad[p as usize];
        self.lson[r as usize] = self.lson[p as usize];
        self.rson[r as usize] = self.rson[p as usize];
        self.dad[self.lson[p as usize] as usize] = r;
        self.dad[self.rson[p as usize] as usize] = r;
        let dp = self.dad[p as usize];
        if self.rson[dp as usize] == p {
            self.rson[dp as usize] = r;
        } else {
            self.lson[dp as usize] = r;
        }
        self.dad[p as usize] = NIL;
        true
    }

    /// 奥村原典 DeleteNode と同一。`promote_other` が真なら del-two で
    /// 前任者でなく後継者 (右部分木最小) を昇格する。
    /// 戻り値: イベント種。
    fn delete(&mut self, p: i32, promote_other: bool) -> EventKind {
        if self.dad[p as usize] == NIL {
            return EventKind::DelAbsent;
        }
        let (q, kind): (i32, EventKind) = if self.rson[p as usize] == NIL {
            let k = if self.lson[p as usize] == NIL {
                EventKind::DelLeaf
            } else {
                EventKind::DelOne
            };
            (self.lson[p as usize], k)
        } else if self.lson[p as usize] == NIL {
            (self.rson[p as usize], EventKind::DelOne)
        } else if !promote_other {
            // 原典: 前任者 (左部分木の最大) を昇格
            let mut q = self.lson[p as usize];
            if self.rson[q as usize] != NIL {
                while self.rson[q as usize] != NIL {
                    q = self.rson[q as usize];
                }
                self.rson[self.dad[q as usize] as usize] = self.lson[q as usize];
                self.dad[self.lson[q as usize] as usize] = self.dad[q as usize];
                self.lson[q as usize] = self.lson[p as usize];
                self.dad[self.lson[p as usize] as usize] = q;
            }
            self.rson[q as usize] = self.rson[p as usize];
            self.dad[self.rson[p as usize] as usize] = q;
            (q, EventKind::DelTwo)
        } else {
            // 編集: 後継者 (右部分木の最小) を昇格
            let mut q = self.rson[p as usize];
            if self.lson[q as usize] != NIL {
                while self.lson[q as usize] != NIL {
                    q = self.lson[q as usize];
                }
                self.lson[self.dad[q as usize] as usize] = self.rson[q as usize];
                self.dad[self.rson[q as usize] as usize] = self.dad[q as usize];
                self.rson[q as usize] = self.rson[p as usize];
                self.dad[self.rson[p as usize] as usize] = q;
            }
            self.lson[q as usize] = self.lson[p as usize];
            self.dad[self.lson[p as usize] as usize] = q;
            (q, EventKind::DelTwo)
        };
        self.dad[q as usize] = self.dad[p as usize];
        let dp = self.dad[p as usize];
        if self.rson[dp as usize] == p {
            self.rson[dp as usize] = q;
        } else {
            self.lson[dp as usize] = q;
        }
        self.dad[p as usize] = NIL;
        kind
    }

    /// tick 冒頭の遅延イベント消化 → delete(s) → text 書込 → s/r 前進 → insert(r)。
    /// insert/delete それぞれで op_counter を進め、edit_op に一致したら編集を適用する。
    fn step_delete(&mut self, target: i32) {
        self.op_counter += 1;
        let op = self.op_counter;
        if self.edit_op == Some(op) && self.edit_type == EditType::SkipDelete {
            self.pending_delete = Some(target);
            return;
        }
        let promote_other =
            self.edit_op == Some(op) && self.edit_type == EditType::DelPromoteOther;
        let kind = self.delete(target, promote_other);
        if let Some(rec) = self.record.as_mut() {
            rec.push(Event {
                op_id: op,
                tick: self.tick,
                kind,
                node: target,
            });
        }
    }

    fn step_insert(&mut self, target: i32) {
        self.op_counter += 1;
        let op = self.op_counter;
        if self.edit_op == Some(op) && self.edit_type == EditType::SkipInsert {
            self.pending_insert = Some(target);
            return;
        }
        let leaf_attach =
            self.edit_op == Some(op) && self.edit_type == EditType::SwapLeafAttach;
        let swapped = self.insert(target, leaf_attach);
        if let Some(rec) = self.record.as_mut() {
            rec.push(Event {
                op_id: op,
                tick: self.tick,
                kind: if swapped {
                    EventKind::InsSwap
                } else {
                    EventKind::Ins
                },
                node: target,
            });
        }
    }

    fn drain_pending(&mut self) {
        if let Some(p) = self.pending_delete.take() {
            // 遅延削除 (op としては数えない)
            self.delete(p, false);
        }
        if let Some(nd) = self.pending_insert.take() {
            // 遅延挿入 (op としては数えない)
            self.insert(nd, false);
        }
    }

    fn advance(&mut self, nbytes: usize) {
        let mut i = 0usize;
        while i < nbytes && self.idx < self.input.len() {
            self.drain_pending();
            self.step_delete(self.s);
            let c = self.input[self.idx];
            self.idx += 1;
            self.text[self.s as usize] = c;
            if (self.s as usize) < F - 1 {
                self.text[self.s as usize + N] = c;
            }
            self.s = (self.s + 1) & (N as i32 - 1);
            self.r = (self.r + 1) & (N as i32 - 1);
            self.step_insert(self.r);
            self.tick += 1;
            i += 1;
        }
        while i < nbytes {
            self.drain_pending();
            self.step_delete(self.s);
            self.s = (self.s + 1) & (N as i32 - 1);
            self.r = (self.r + 1) & (N as i32 - 1);
            self.len -= 1;
            if self.len > 0 {
                self.step_insert(self.r);
            }
            self.tick += 1;
            i += 1;
        }
    }

    /// read-only トレース: 一致長がちょうど max_len のノードを訪問順に列挙。
    fn trace(&self, r: i32, max_len: u8) -> Vec<i32> {
        let mut res = Vec::new();
        if self.len == 0 {
            return res;
        }
        let ks = r as usize;
        let mut i: i32 = N as i32 + 1 + self.text[ks] as i32;
        let mut cmp: i32 = 1;
        loop {
            i = if cmp >= 0 {
                self.rson[i as usize]
            } else {
                self.lson[i as usize]
            };
            if i == NIL {
                break;
            }
            let mut j: usize = 1;
            cmp = 0;
            while j < F {
                let d = self.text[ks + j] as i32 - self.text[i as usize + j] as i32;
                if d != 0 {
                    cmp = d;
                    break;
                }
                j += 1;
            }
            if j as u8 == max_len {
                res.push(i);
            }
        }
        res
    }
}

// ---------------------------------------------------------------------------
// tie 情報とベースライン再生
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct TieInfo {
    ti: usize,
    tick_start: u64,
    max_len: u8,
    chosen: i32,
    baseline_ok: bool,
    /// chosen が trace 候補に含まれるか (false = tie 祖先問題ではなく候補欠落)
    chosen_in_trace: bool,
}

/// 全トークンを teacher-forcing 再生し、tie (max_len<18 の match で
/// trace 候補が存在するもの) の rank1 一致/違反を全列挙する。
fn baseline_run(tokens: &[Token], input: &[u8]) -> Vec<TieInfo> {
    let mut sim = Sim::new(input.to_vec());
    let mut ties = Vec::new();
    for (ti, tok) in tokens.iter().enumerate() {
        if let Token::Match { pos, len } = tok {
            if *len < F as u8 {
                let tr = sim.trace(sim.r, *len);
                if !tr.is_empty() {
                    ties.push(TieInfo {
                        ti,
                        tick_start: sim.tick,
                        max_len: *len,
                        chosen: *pos as i32,
                        baseline_ok: tr[0] == *pos as i32,
                        chosen_in_trace: tr.contains(&(*pos as i32)),
                    });
                }
            }
        }
        sim.advance(tok.emit_len());
    }
    ties
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

struct Args {
    file: String,
    ti: Option<usize>,
    window: u64,
    limit: Option<usize>,
    self_test: bool,
    dump_ties: Option<String>,
}

fn usage_exit(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    eprintln!(
        "usage: lf2_stage6_event_edit [--file PATH] [--ti N] [--window N] \
         [--limit N] [--self-test] [--dump-ties PATH]"
    );
    eprintln!(
        "  --ti は違反でない tie も指定可 (編集で正常 tie を壊さないかの検証用途)"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = Args {
        file: ".local_data/lvns3/C0602.LF2".to_string(),
        ti: None,
        window: 4114,
        limit: None,
        self_test: false,
        dump_ties: None,
    };
    let argv: Vec<String> = env::args().skip(1).collect();
    let next_val = |argv: &[String], i: usize| -> String {
        match argv.get(i + 1) {
            Some(v) => v.clone(),
            None => usage_exit(&format!("{} には値が必要", argv[i])),
        }
    };
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "--file" => {
                args.file = next_val(&argv, i);
                i += 2;
            }
            "--ti" => {
                args.ti = Some(
                    next_val(&argv, i)
                        .parse()
                        .unwrap_or_else(|_| usage_exit("--ti は整数")),
                );
                i += 2;
            }
            "--window" => {
                args.window = next_val(&argv, i)
                    .parse()
                    .unwrap_or_else(|_| usage_exit("--window は整数"));
                i += 2;
            }
            "--limit" => {
                args.limit = Some(
                    next_val(&argv, i)
                        .parse()
                        .unwrap_or_else(|_| usage_exit("--limit は整数")),
                );
                i += 2;
            }
            "--self-test" => {
                args.self_test = true;
                i += 1;
            }
            "--dump-ties" => {
                args.dump_ties = Some(next_val(&argv, i));
                i += 2;
            }
            other => usage_exit(&format!("unknown arg: {}", other)),
        }
    }
    args
}

fn main() {
    let args = parse_args();
    let data = fs::read(&args.file).expect("read LF2");
    let (comp, w, h) = parse_lf2(&data);
    let (tokens, input) = decode_tokens(&comp, w, h);
    println!(
        "file={} {}x{} tokens={} input_bytes={}",
        args.file,
        w,
        h,
        tokens.len(),
        input.len()
    );

    // 1. ベースライン: 全 tie と違反を列挙
    let t0 = std::time::Instant::now();
    let ties = baseline_run(&tokens, &input);
    let violations: Vec<&TieInfo> = ties.iter().filter(|t| !t.baseline_ok).collect();
    let tie_violations = violations.iter().filter(|t| t.chosen_in_trace).count();
    println!(
        "baseline: ties={} violations={} (tie祖先違反={} 候補欠落={}) ({:.2}s)",
        ties.len(),
        violations.len(),
        tie_violations,
        violations.len() - tie_violations,
        t0.elapsed().as_secs_f64()
    );
    if let Some(path) = &args.dump_ties {
        let mut out = String::from("token_idx,max_len,chosen,baseline_ok,chosen_in_trace\n");
        for t in &ties {
            out.push_str(&format!(
                "{},{},{},{},{}\n",
                t.ti, t.max_len, t.chosen, t.baseline_ok as u8, t.chosen_in_trace as u8
            ));
        }
        fs::write(path, out).expect("write dump");
        println!("ties dumped to {}", path);
    }
    if violations.is_empty() {
        println!("違反なし。終了。");
        return;
    }

    // 2. 対象 ti と窓
    // --ti は違反でない tie の指定も意図的に許容する
    // (編集が正常 tie を壊さないことを検証する用途)
    let target = match args.ti {
        Some(ti) => *ties
            .iter()
            .find(|t| t.ti == ti)
            .unwrap_or_else(|| panic!("ti={} は tie ではない", ti)),
        None => *violations[0],
    };
    println!(
        "target: ti={} tick={} max_len={} chosen={} baseline_ok={}",
        target.ti, target.tick_start, target.max_len, target.chosen, target.baseline_ok
    );

    let window_start_tick = target.tick_start.saturating_sub(args.window);
    // 窓開始以前で最後のトークン境界を探す
    let mut snap_ti: usize = 0;
    let mut snap_tick: u64 = 0;
    {
        let mut tick: u64 = 0;
        for (ti, tok) in tokens.iter().enumerate() {
            if tick > window_start_tick || ti >= target.ti {
                break;
            }
            snap_ti = ti;
            snap_tick = tick;
            tick += tok.emit_len() as u64;
        }
    }
    println!(
        "window: [{}, {}) snapshot at token {} (tick {})",
        window_start_tick, target.tick_start, snap_ti, snap_tick
    );

    // 3. スナップショットまで進める
    let mut base = Sim::new(input.clone());
    for tok in &tokens[..snap_ti] {
        base.advance(tok.emit_len());
    }
    assert_eq!(base.tick, snap_tick);

    // 窓内で判定対象になる tie (snap_ti <= ti <= target.ti)
    let window_ties: Vec<TieInfo> = ties
        .iter()
        .filter(|t| t.ti >= snap_ti && t.ti <= target.ti)
        .cloned()
        .collect();
    println!(
        "window ties: {} (ok={} viol={})",
        window_ties.len(),
        window_ties.iter().filter(|t| t.baseline_ok).count(),
        window_ties.iter().filter(|t| !t.baseline_ok).count()
    );

    // スナップショット復元同一性の検証: clone 再生がベースラインと完全一致するか
    if args.self_test {
        let mut clone = base.clone();
        let mut ok = true;
        // clone を窓内再生して各 tie の判定がベースラインと一致するか確認
        let mut cursor = snap_ti;
        for t in &window_ties {
            for tok in &tokens[cursor..t.ti] {
                clone.advance(tok.emit_len());
            }
            cursor = t.ti;
            let tr = clone.trace(clone.r, t.max_len);
            let ok_now = !tr.is_empty() && tr[0] == t.chosen;
            if ok_now != t.baseline_ok {
                println!("SELF-TEST MISMATCH at ti={}", t.ti);
                ok = false;
            }
        }
        println!(
            "self-test (snapshot restore identity): {}",
            if ok { "PASS" } else { "FAIL" }
        );
        if !ok {
            return;
        }
    }

    // 4. 窓内イベントの列挙 (record パス)
    let events: Vec<Event> = {
        let mut enumr = base.clone();
        enumr.record = Some(Vec::new());
        for tok in &tokens[snap_ti..target.ti] {
            enumr.advance(tok.emit_len());
        }
        enumr.record.take().unwrap()
    };
    let n_ins = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Ins | EventKind::InsSwap))
        .count();
    let n_swap = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::InsSwap))
        .count();
    let n_del = events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::DelLeaf | EventKind::DelOne | EventKind::DelTwo
            )
        })
        .count();
    let n_del2 = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::DelTwo))
        .count();
    println!(
        "events in window: total={} ins={} (swap={}) del={} (del-two={})",
        events.len(),
        n_ins,
        n_swap,
        n_del,
        n_del2
    );

    // 5. 総当たり: 各イベント × 適用可能編集で差分再生
    let t1 = std::time::Instant::now();
    // (op_id, tick, edit, broken, fixed_others)
    let mut solutions: Vec<(u64, u64, EditType, usize, usize)> = Vec::new();
    let mut tried: usize = 0;
    let event_iter: Vec<&Event> = match args.limit {
        Some(l) => events.iter().take(l).collect(),
        None => events.iter().collect(),
    };
    for ev in &event_iter {
        for edit in EditType::ALL {
            if !edit.applicable(ev.kind) {
                continue;
            }
            tried += 1;
            let mut sim = base.clone();
            sim.edit_op = Some(ev.op_id);
            sim.edit_type = edit;
            // op_counter はスナップショット時点から続きで数えたいが、
            // record パスと同一初期値である必要がある。base.op_counter を保持済み。
            let mut target_ok = false;
            let mut broken = 0usize;
            let mut fixed_others = 0usize;
            let mut cursor = snap_ti;
            for t in &window_ties {
                for tok in &tokens[cursor..t.ti] {
                    sim.advance(tok.emit_len());
                }
                cursor = t.ti;
                let tr = sim.trace(sim.r, t.max_len);
                let ok_now = !tr.is_empty() && tr[0] == t.chosen;
                if t.ti == target.ti {
                    target_ok = ok_now;
                } else if t.baseline_ok && !ok_now {
                    broken += 1;
                } else if !t.baseline_ok && ok_now {
                    fixed_others += 1;
                }
            }
            if target_ok {
                solutions.push((ev.op_id, ev.tick, edit, broken, fixed_others));
            }
        }
    }
    println!(
        "brute force: tried={} edits in {:.1}s",
        tried,
        t1.elapsed().as_secs_f64()
    );

    // 6. 結果
    if solutions.is_empty() {
        println!(
            "==> 単一編集解なし: ti={} は窓内 {} イベント × 編集タイプ全組合せで rank1 一致にならない",
            target.ti,
            event_iter.len()
        );
    } else {
        solutions.sort_by_key(|&(_, _, _, broken, _)| broken);
        println!(
            "==> 解 {} 件 (op_id, event_tick, edit_type, 壊れる正常tie数, 直る他違反数):",
            solutions.len()
        );
        for (op_id, tick, edit, broken, fixed) in &solutions {
            println!(
                "  op={} tick={} edit={} broken={} fixed_others={}",
                op_id,
                tick,
                edit.name(),
                broken,
                fixed
            );
        }
        let clean = solutions.iter().filter(|s| s.3 == 0).count();
        println!("  うち副作用ゼロ (broken=0): {}", clean);
    }
}

// ---------------------------------------------------------------------------
// テスト: スナップショット (Clone) 復元の同一性
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 擬似乱数入力を literal 列として teacher-forcing 再生し、
    /// 途中で Clone したシミュレータの続行結果がオリジナルと完全一致することを確認。
    #[test]
    fn snapshot_restore_identity() {
        // xorshift で決定的な擬似乱数入力を作る
        let mut x: u32 = 0x12345678;
        let input: Vec<u8> = (0..6000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x & 0xff) as u8
            })
            .collect();
        let mut a = Sim::new(input.clone());
        // 半分進めて clone
        a.advance(3000);
        let mut b = a.clone();
        a.advance(2000);
        b.advance(2000);
        assert_eq!(a.text, b.text);
        assert_eq!(a.lson, b.lson);
        assert_eq!(a.rson, b.rson);
        assert_eq!(a.dad, b.dad);
        assert_eq!(a.r, b.r);
        assert_eq!(a.s, b.s);
        assert_eq!(a.tick, b.tick);
        assert_eq!(a.op_counter, b.op_counter);
        // trace も一致
        assert_eq!(a.trace(a.r, 5), b.trace(b.r, 5));
    }

    /// BST 不変条件の検証: dad/lson/rson の相互整合・256 root からの
    /// 全ノード到達可能性 (dad!=NIL のノード集合と一致)・循環なし。
    fn assert_bst_invariants(sim: &Sim) {
        // 相互整合: 各ノードの子の dad は自分、dad の子リンクは自分
        for p in 0..N as i32 {
            if sim.dad[p as usize] == NIL {
                continue;
            }
            let dp = sim.dad[p as usize];
            assert!(
                sim.rson[dp as usize] == p || sim.lson[dp as usize] == p,
                "node {} is not a child of its dad {}",
                p,
                dp
            );
            for &c in &[sim.lson[p as usize], sim.rson[p as usize]] {
                if c != NIL {
                    assert_eq!(sim.dad[c as usize], p, "child {} dad != {}", c, p);
                }
            }
        }
        // 到達可能性と循環なし: root の rson 部分木を DFS
        let mut visited = vec![false; N];
        for root in (N + 1)..(N + 257) {
            let mut stack = vec![sim.rson[root]];
            while let Some(n) = stack.pop() {
                if n == NIL {
                    continue;
                }
                assert!(!visited[n as usize], "cycle or duplicate at node {}", n);
                visited[n as usize] = true;
                stack.push(sim.lson[n as usize]);
                stack.push(sim.rson[n as usize]);
            }
        }
        for p in 0..N {
            assert_eq!(
                visited[p],
                sim.dad[p] != NIL,
                "reachability mismatch at node {}",
                p
            );
        }
    }

    /// del_promote_other / swap_leaf_attach の各編集適用後も BST 不変条件
    /// (相互整合・到達可能性・循環なし) が保たれることを確認。
    #[test]
    fn edits_preserve_bst_invariants() {
        // 前半: 周期 500 の反復 (full-F 一致 → ins-swap がすぐ発生)
        // 後半: 64 記号の擬似乱数 (分岐の多い木を作り、実削除が始まる
        // tick 4078 以降に del-two を発生させる)
        let mut x: u32 = 0xcafe1234;
        let block: Vec<u8> = (0..500)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                (x & 0x3f) as u8
            })
            .collect();
        let mut input = Vec::new();
        for _ in 0..4 {
            input.extend_from_slice(&block);
        }
        input.extend((0..5000).map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            (x & 0x3f) as u8
        }));
        // 無編集でイベントを列挙し、del-two / ins-swap の op を集める
        let mut probe = Sim::new(input.clone());
        probe.advance(400);
        let base = probe.clone();
        let mut enumr = base.clone();
        enumr.record = Some(Vec::new());
        enumr.advance(6600);
        let events = enumr.record.take().unwrap();
        let del_twos: Vec<u64> = events
            .iter()
            .filter(|e| e.kind == EventKind::DelTwo)
            .map(|e| e.op_id)
            .take(20)
            .collect();
        let ins_swaps: Vec<u64> = events
            .iter()
            .filter(|e| e.kind == EventKind::InsSwap)
            .map(|e| e.op_id)
            .take(20)
            .collect();
        assert!(!del_twos.is_empty(), "no del-two events in probe input");
        assert!(!ins_swaps.is_empty(), "no ins-swap events in probe input");
        for (ops, edit) in [
            (&del_twos, EditType::DelPromoteOther),
            (&ins_swaps, EditType::SwapLeafAttach),
        ] {
            for &op in ops.iter() {
                let mut sim = base.clone();
                sim.edit_op = Some(op);
                sim.edit_type = edit;
                sim.advance(6600);
                assert_bst_invariants(&sim);
            }
        }
        // 無編集でも成り立つこと (検証関数自体の健全性)
        let mut plain = base.clone();
        plain.advance(6600);
        assert_bst_invariants(&plain);
    }

    /// 無編集 (edit_op=None) の clone 再生が、clone 元をそのまま進めた場合と
    /// 完全一致すること (編集フックの素通り性)。
    #[test]
    fn no_edit_passthrough() {
        let mut x: u32 = 0xdeadbeef;
        let input: Vec<u8> = (0..5000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                // 繰り返しの多い入力にして swap/del-two を発生させる
                ((x & 0x3) * 7) as u8
            })
            .collect();
        let mut a = Sim::new(input.clone());
        a.advance(2500);
        let mut b = a.clone();
        b.record = Some(Vec::new());
        a.advance(2000);
        b.advance(2000);
        assert_eq!(a.lson, b.lson);
        assert_eq!(a.rson, b.rson);
        assert_eq!(a.dad, b.dad);
        let ev = b.record.take().unwrap();
        // record パスでイベントが取れている (ins/del 両方)
        assert!(ev.iter().any(|e| matches!(e.kind, EventKind::Ins | EventKind::InsSwap)));
        assert!(ev
            .iter()
            .any(|e| matches!(e.kind, EventKind::DelLeaf | EventKind::DelOne | EventKind::DelTwo)));
    }
}
