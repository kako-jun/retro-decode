//! Stage 14-2 (Issue #14 脈: per-file 小状態フィッティング「⑧ ring 初期内容
//! 汚染統合」): near-miss 上位ファイルに対する低エントロピー初期状態探索。
//!
//! `lf2_stage14_2_near_miss` の出力 (`near_miss_ledger.csv`、remaining_tokens
//! 昇順) の上位ファイルに対し、writetime 系4系統 (clip/plus1 x
//! ascending/descending。union257 の中核) の ring 初期内容・先読み開始位置を
//! 差し替えて byte-exact 反転が出るか総当りする。
//!
//! 候補:
//!   - ring fill: 0x00 全埋め / 0xff 全埋め (対照: 0x20 は baseline で既知)
//!   - 自ファイルヘッダ残骸: 先頭配置 / 末尾配置 (history 領域 [0,r_init-1] に
//!     タイル)
//!   - バッチ内直前ファイル (PAK格納順=辞書順で直前) の同モードでのエンコード
//!     終了時 ring 内容 (⑨ バッチ残留)
//!   - r_init オフセット ±小範囲 (fill=0x20 固定、他候補と直交で試す)
//!
//! usage:
//!   cargo run --release --bin lf2_stage14_2_ring_probe -- <LF2_DIR> \
//!       --ledger <near_miss_ledger.csv> [--top N] [--out-csv PATH]
//!       [--sweep-type TYPE --sweep-mode MODE]   (反転が出た候補の全265本掃討用)

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_writetime_custom_ring_traced, F, N};
use retro_decode::formats::toheart::verify_harness::{self, tokens_to_lf2_payload};

const BUF: usize = N + F - 1;
const HIST: usize = N - F; // r_init = 4078 (fill 埋め対象領域 [0, HIST-1])

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    ClipDesc,
    ClipAsc,
    Plus1Desc,
    Plus1Asc,
}
const MODES: [Mode; 4] = [Mode::ClipDesc, Mode::ClipAsc, Mode::Plus1Desc, Mode::Plus1Asc];
fn mode_label(m: Mode) -> &'static str {
    match m {
        Mode::ClipDesc => "clip_wtd",
        Mode::ClipAsc => "clip_wta",
        Mode::Plus1Desc => "plus1_wtd",
        Mode::Plus1Asc => "plus1_wta",
    }
}
fn mode_params(m: Mode) -> (bool, bool) {
    // (plus1, ascending)
    match m {
        Mode::ClipDesc => (false, false),
        Mode::ClipAsc => (false, true),
        Mode::Plus1Desc => (true, false),
        Mode::Plus1Asc => (true, true),
    }
}

fn tile_into(dst: &mut [u8], src: &[u8], align_end: bool) {
    if src.is_empty() {
        return;
    }
    let n = dst.len();
    if align_end {
        for i in 0..n {
            // dst[n-1-i] <- src cyclique from the end
            let src_idx = src.len() - 1 - (i % src.len());
            dst[n - 1 - i] = src[src_idx];
        }
    } else {
        for i in 0..n {
            dst[i] = src[i % src.len()];
        }
    }
}

/// history 領域 [0,HIST-1] だけを候補内容で埋め、overlap 領域
/// (`text_buf[N..N+F-2]`) は `text_buf[0..F-2]` のミラーにする
/// (先読み [HIST..N-1] は呼び出し側 `compress_okumura_writetime_custom_ring_traced`
/// が入力の最初の F バイトで即座に上書きするため、ここでの値は使われない)。
fn build_buf_from_history(history: [u8; HIST]) -> [u8; BUF] {
    let mut buf = [0x20u8; BUF];
    buf[0..HIST].copy_from_slice(&history);
    for i in 0..(F - 1) {
        buf[N + i] = buf[i];
    }
    buf
}

fn fill_history(fill: u8) -> [u8; HIST] {
    [fill; HIST]
}

fn header_history(header: &[u8], align_end: bool) -> [u8; HIST] {
    let mut h = [0x20u8; HIST];
    tile_into(&mut h, header, align_end);
    h
}

/// バッチ残留候補: 直前ファイルを同モード・baseline(0x20) fill でエンコードした
/// 終了時 ring (`text_buf[0..N]`) の下位 HIST バイトをそのまま history として使う。
fn prev_batch_history(prev_ring_input: &[u8], mode: Mode) -> [u8; HIST] {
    let (plus1, ascending) = mode_params(mode);
    let base = [0x20u8; BUF];
    let (_, snapshot) = compress_okumura_writetime_custom_ring_traced(prev_ring_input, plus1, ascending, base, 0);
    let mut h = [0x20u8; HIST];
    h.copy_from_slice(&snapshot[0..HIST]);
    h
}

struct Candidate {
    label: String,
    buf: [u8; BUF],
    r_init_delta: i32,
}

fn candidates_for_file(header: &[u8], prev_ring_input: Option<&[u8]>, mode: Mode) -> Vec<Candidate> {
    let mut out = Vec::new();
    out.push(Candidate {
        label: "fill00".to_string(),
        buf: build_buf_from_history(fill_history(0x00)),
        r_init_delta: 0,
    });
    out.push(Candidate {
        label: "fillff".to_string(),
        buf: build_buf_from_history(fill_history(0xff)),
        r_init_delta: 0,
    });
    out.push(Candidate {
        label: "header_head".to_string(),
        buf: build_buf_from_history(header_history(header, false)),
        r_init_delta: 0,
    });
    out.push(Candidate {
        label: "header_tail".to_string(),
        buf: build_buf_from_history(header_history(header, true)),
        r_init_delta: 0,
    });
    if let Some(prev) = prev_ring_input {
        out.push(Candidate {
            label: "prev_batch_residue".to_string(),
            buf: build_buf_from_history(prev_batch_history(prev, mode)),
            r_init_delta: 0,
        });
    }
    // 小整数パラメータ: r_init オフセット (fill=0x20 固定、他候補と直交)。
    // r_init(=N-F=4078) は `insert_node` 内の比較ループが text_buf[r..r+F-1] を
    // マスクなしで直接読むため、`r_init+delta+F-1 <= N-1` (= delta<=0) を
    // 満たす必要がある (r_init は元々この上限ちょうどに選ばれている定数)。
    // 正方向は配列外アクセスになるため候補から除外し、負方向のみ試す。
    for delta in [-5, -4, -3, -2, -1] {
        out.push(Candidate {
            label: format!("r_init_delta{:+}", delta),
            buf: [0x20u8; BUF],
            r_init_delta: delta,
        });
    }
    out
}

struct FileCtx {
    name: String,
    ring_input: Vec<u8>,
    orig_payload: Vec<u8>,
    header: Vec<u8>,
}

fn load_ctx(path: &PathBuf) -> Option<FileCtx> {
    let data = fs::read(path).ok()?;
    let (w, h, ps) = verify_harness::parse_lf2(&data)?;
    let decoded = retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h).ok()?;
    Some(FileCtx {
        name: path.file_name()?.to_str()?.to_string(),
        ring_input: decoded.ring_input,
        orig_payload: data[ps..].to_vec(),
        header: data[0..ps].to_vec(),
    })
}

fn run_one(ctx: &FileCtx, prev_ring_input: Option<&[u8]>, out_hits: &mut Vec<(String, String, String)>, out_rows: &mut Vec<String>) {
    for &mode in MODES.iter() {
        let (plus1, ascending) = mode_params(mode);
        for cand in candidates_for_file(&ctx.header, prev_ring_input, mode) {
            let (toks, _) =
                compress_okumura_writetime_custom_ring_traced(&ctx.ring_input, plus1, ascending, cand.buf, cand.r_init_delta);
            let payload = tokens_to_lf2_payload(&toks);
            let hit = payload == ctx.orig_payload;
            out_rows.push(format!("{},{},{},{}", ctx.name, mode_label(mode), cand.label, hit as u8));
            if hit {
                out_hits.push((ctx.name.clone(), mode_label(mode).to_string(), cand.label.clone()));
            }
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> --ledger <csv> [--top N] [--out-csv PATH] [--sweep-type T --sweep-mode M]", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut ledger_path = PathBuf::from(".local_data/stage14_2/near_miss_ledger.csv");
    let mut top_n: usize = 20;
    let mut out_csv = PathBuf::from(".local_data/stage14_2/ring_probe_results.csv");
    let mut sweep_type: Option<String> = None;
    let mut sweep_mode: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--ledger" => {
                ledger_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--top" => {
                top_n = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--out-csv" => {
                out_csv = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--sweep-type" => {
                sweep_type = Some(args[i + 1].clone());
                i += 2;
            }
            "--sweep-mode" => {
                sweep_mode = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(parent) = out_csv.parent() {
        fs::create_dir_all(parent).ok();
    }

    // 全ファイル (union 込み) をソート順で読み、直前ファイル (PAK格納順=辞書順)
    // を引けるようにする。
    let all_files = verify_harness::list_lf2_files(&dir, None).expect("list dir");
    let name_to_idx: HashMap<String, usize> = all_files
        .iter()
        .enumerate()
        .map(|(idx, p)| (p.file_name().unwrap().to_str().unwrap().to_string(), idx))
        .collect();

    let target_names: Vec<String> = if let Some(st) = &sweep_type {
        // 全265本掃討モード: ledger の全行 (union 非所属 = ledger に載っている全ファイル) を対象にする。
        eprintln!("SWEEP MODE: type={} mode={:?}", st, sweep_mode);
        fs::read_to_string(&ledger_path)
            .unwrap()
            .lines()
            .skip(1)
            .filter_map(|l| l.split(',').next().map(|s| s.to_string()))
            .collect()
    } else {
        fs::read_to_string(&ledger_path)
            .unwrap()
            .lines()
            .skip(1)
            .take(top_n)
            .filter_map(|l| l.split(',').next().map(|s| s.to_string()))
            .collect()
    };
    eprintln!("target files: {}", target_names.len());

    let mut out_rows: Vec<String> = Vec::new();
    let mut hits: Vec<(String, String, String)> = Vec::new();

    for name in &target_names {
        let idx = match name_to_idx.get(name) {
            Some(i) => *i,
            None => {
                eprintln!("WARN: {} not found in dir listing", name);
                continue;
            }
        };
        let path = &all_files[idx];
        let ctx = match load_ctx(path) {
            Some(c) => c,
            None => {
                eprintln!("WARN: load fail {}", name);
                continue;
            }
        };
        let prev_ring_input: Option<Vec<u8>> = if idx > 0 {
            load_ctx(&all_files[idx - 1]).map(|c| c.ring_input)
        } else {
            None
        };

        if let (Some(st), Some(sm)) = (&sweep_type, &sweep_mode) {
            // 単一 (type, mode) だけを流す掃討専用パス
            let mode = MODES.iter().copied().find(|m| mode_label(*m) == sm).expect("unknown mode");
            let (plus1, ascending) = mode_params(mode);
            let cands = candidates_for_file(&ctx.header, prev_ring_input.as_deref(), mode);
            if let Some(cand) = cands.into_iter().find(|c| &c.label == st) {
                let (toks, _) = compress_okumura_writetime_custom_ring_traced(
                    &ctx.ring_input,
                    plus1,
                    ascending,
                    cand.buf,
                    cand.r_init_delta,
                );
                let payload = tokens_to_lf2_payload(&toks);
                let hit = payload == ctx.orig_payload;
                out_rows.push(format!("{},{},{},{}", ctx.name, mode_label(mode), cand.label, hit as u8));
                if hit {
                    hits.push((ctx.name.clone(), mode_label(mode).to_string(), cand.label.clone()));
                }
            }
        } else {
            run_one(&ctx, prev_ring_input.as_deref(), &mut hits, &mut out_rows);
        }
    }

    let mut f = fs::File::create(&out_csv).expect("create out csv");
    writeln!(f, "name,mode,candidate,hit").unwrap();
    for row in &out_rows {
        writeln!(f, "{}", row).unwrap();
    }

    eprintln!("--- rows: {} ---", out_rows.len());
    eprintln!("--- HITS: {} ---", hits.len());
    for (name, mode, cand) in &hits {
        eprintln!("HIT {} mode={} candidate={}", name, mode, cand);
    }

    ExitCode::SUCCESS
}
