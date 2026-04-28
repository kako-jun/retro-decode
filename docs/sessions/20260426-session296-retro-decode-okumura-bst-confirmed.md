# Session 296 — retro-decode Leaf 完全再現エンコーダ：byte-exact 0 → 215/522 ファイル達成

日付: 2026-04-26
対象: kako-jun/retro-decode#5（LF2 100% 一致再エンコーダ、世界初狙い）

## セッション開始時の認識ミスと修正

セッション 295 の引継ぎを「2 メカニズム競合シミュレータを書く」前提で受け取ったが、kako-jun から複数回叱責を受けて根本的に整理し直した:

1. **「95% でリリースもアリでは」と妥協戦略を提案 → 即否定**: retro-decode は byte-exact 100% が達成条件であり、95% で止めるプロジェクトではない (`feedback_retro_decode_100pct.md` 永続化)
2. **既存 artifact を読まずに戦略を語った**: PAK 解凍済 = decoder 完動、`compress_okumura`/`decision_tree`/`naive_scan` 並列実装済 = encoder 完動、bench が圧縮率と一致率を別出力 = 公式より圧縮率高い、を全部 repo から読めたのに見逃した (`feedback_read_artifacts_first.md`)
3. **計測 refactor を「過剰では」と躊躇した → 即実行で大量シグナル**: token match rate だけでなく per-file 第一発散位置・フィンガープリント分類を出した瞬間に「Leaf は nearer-bias」「`>=` は offset 2 で崩壊」など構造的洞察が出た (`feedback_just_do_better_tooling.md`)
4. **真の問題定義**: retro-decode のゴールは「より良いエンコーダを書く」ではなく「**Leaf が 1997 年に行った劣化選択を完全に再現する**」(`project_retro_decode_context.md`)

## やったこと（実装＋bench）

### 1. TSV 統計分析でアルゴリズム候補絞り込み

セッション 295 の `/tmp/tiebreaks50.tsv` (306,562 行) を Python (`/tmp/analyze_*.py`) で総当たり分析:
- max_len mod 8 = 2 で A 率 97.3% という signal は実値 max_len=18（LZSS 最大長で 47% を占める）
- max_len 別: max_len=18 で A=87.3% / max_len=3 で B=60.5%（U 字分布の正体）
- max(dists)/spread/n_max いずれも B 率と強い負相関 → スキャンが全候補を visit していない = **BST 経路探索のシグネチャ**
- 素朴モデル「max_len=18 なら rank=1、それ以外なら rank=n_max」が TSV 全体で 71.5% 当てる

### 2. Naive 線形バックスキャン → 完全否定

`src/formats/toheart/naive_scan_lzss.rs` 新規 + `src/bin/lf2_naive_bench.rs` 新規。strict / equal 両方で 0/522 ファイル一致。**Leaf は素朴スキャンではなく BST 探索**を確定。

### 3. Okumura BST トークンレベル bench 構築

`src/bin/lf2_token_bench.rs` 新規。Leaf 実デコード結果のトークン列との一致率を測定。`compress_okumura_with_tie(input, allow_equal)` を `okumura_lzss.rs` に追加（`>=` 変種テスト用）。

| 変種 | identical / 522 | match_rate |
|---|---|---|
| `tie_strict_gt` (奥村原典 `>`) | **171** | 82.524% |
| `tie_allow_eq` (`>=`) | 0 | 71.799% |
| `dummy_rev` (F-dummy 逆順) | 165 | 82.523% |
| `distance_tie` (距離で勝ち) | 0 | 73.303% |
| `lazy` (1-byte lazy) | 0 | 0.811% |
| **`no_dummy` (F-dummy 全削除)** | **215** | TBD |

### 4. メトリクス refactor で per-file フィンガープリント取得

`lf2_token_bench` を「第一発散位置 + フィンガープリント分類」に書き換え:
- `MATCH_SAME_LEN_DIFF_POS:len<=5:leaf_nearer/oku_nearer` 等のクラスタを上位 15 個出す
- 結果: `tie_strict_gt` の支配的残差は同長異 pos で leaf_nearer 偏り (157 vs 84)

→ Leaf は奥村より強い nearness バイアスを持つと仮定 → `distance_tie` を投入 → **0/522 で大崩壊**。「常に近い」も違う。**変種ガチャ打ち止め**。

### 5. `lf2_first_div_inspect.rs` で現場検証

第一発散が極めて早いファイル (top 10 = C1203/4/5/6/7/8/9/A/B/D 全部 token 2-3) を side-by-side dump。決定的観察:

> **3 ファイルすべて token 2-3 で「Leaf=Literal(0x20)、Okumura=Match(len=17)」**
> 候補に `pos=r, dist=0, len=18` の self-reference が居る
> Leaf は 17 バイトマッチを蹴って Literal を出している

→ 当初 lazy match 仮説 → 実装 → 0/522 で否定。lazy ですらない。

→ 真の仮説: **Leaf は F-dummy 挿入をしていない**（Okumura の `for i in 1..=F { insert_node(r-i) }` を Leaf は省略）

### 6. `compress_okumura_no_dummy` 投入 → +44 ファイル獲得

dummy 挿入ループを単に削除した変種 = **215/522 識別**（Okumura 原典 171 から +44）。C1203/4/5 cluster 完全消滅。仮説確定。

### 7. `lf2_token0_inspect.rs` で残ギャップ調査

no_dummy の新支配残差 `MATCH_vs_LIT:len=18` 151 ファイルのうち厳密に「token 0 で Leaf=Match、no_dummy=Literal」は 4 ファイルのみ:
- `pos = 0xFDC = N - 2F` (3/4)
- `pos = 0xFED = r - 1` (1/4)
- `len = 18` (3) / `len = 16` (1)
- 全て first byte = 0x20

→ Leaf は dummy を **完全削除でもなく F 個全部でもなく**、何か中間の初期 BST 状態を持っている。「pos = r-F に 1 個だけ dummy 挿入」または「F dummy 入れるが swap-with-r をしない」あたり。

## 残ギャップと次の手

### 数値の現状

- **byte-exact 一致**: 215/522 = **41.2%**
- セッション開始時: 0/522 = 0%

### 次セッション最優先（変種候補、優先度順）

1. **no_dummy + 1-dummy at r-F**: pos r-F だけに dummy 1 個。token 0 で Match{pos=0xFDC, len=18} を作れる。+150 file 期待
2. **no_dummy + F-dummy with no swap-with-r**: insert_node の `if i >= F { break; }` 抜け道を消し、swap ブロックを実行しない変種
3. **THRESHOLD=3 でなく 4 にする**: Match の最小長を 4 にすれば短マッチ寄りの誤差が消えるかも
4. **F dummy をすべて pos r-F に重ねる**（同じ位置に F 回挿入＝1個と等価だが、実装が違う）
5. **insert_node 内の `cmp` 初期値を 0 に**（奥村は 1、初期 cmp が 0 なら左右の選び方が変わる）

各変種で 4 ファイル程度の token 0 ケースを救済できるか + 全体カスケード影響を bench で見る。

### 詰みかけた時の予備路線

- `lf2_first_div_inspect` で no_dummy の残差ファイル top 10 を再検証 → 別の発散パターン発見
- TSV (`/tmp/tiebreaks50.tsv`) の素朴予測 71.5% が実エンコーダ走行 215 file (≒ 41.2%) より上回るのは何故か再考（token 一致率 vs file 一致率の違い、cascade ノイズ）
- 最終手段: SSD 全体再走査で Leaf 同時代の他製品（Filsnown / To Heart / Comic Party 等）に encoder バイナリが残っていないか grep

## 環境メモ・成果物

### コード（commit 済み、retro-decode `/Users/kako-jun/repos/2025/retro-decode`）

**新規ファイル**:
- `src/formats/toheart/naive_scan_lzss.rs` — naive 線形スキャン（負例として温存）
- `src/bin/lf2_naive_bench.rs` — 負例 bench
- `src/bin/lf2_token_bench.rs` — **主役**。トークンレベル一致率 + per-file 発散位置 + フィンガープリント分類
- `src/bin/lf2_first_div_inspect.rs` — top-10 早期発散の side-by-side ダンプ
- `src/bin/lf2_token0_inspect.rs` — token 0 限定の Leaf 選択分析

**変更ファイル**:
- `src/formats/toheart/okumura_lzss.rs` — `TieMode` enum 追加、変種関数 5 種追加 (`compress_okumura_with_tie` / `compress_okumura_dummy_rev` / `compress_okumura_distance_tie` / `compress_okumura_lazy` / `compress_okumura_no_dummy`) + `dump_state` debug accessor
- `src/formats/toheart/lf2.rs` — `to_lf2_bytes_naive_strict` / `to_lf2_bytes_naive_equal` 追加
- `src/formats/toheart/mod.rs` — `pub mod naive_scan_lzss;` 追加

### 中間ログ・データ

- `/tmp/token_bench_v2.txt` / `_v3.txt` / `_v4.txt` / `_v5.txt` — 各回の bench 出力
- `/tmp/first_div_inspect.txt` — top-10 早期発散ファイルの完全ダンプ (1005 行)
- `/tmp/token0_inspect.txt` — token 0 限定の Leaf 選択分析
- `/tmp/tiebreaks50.tsv` — セッション 295 の 306,562 行タイブレーク統計（次セッションも使う）
- `/tmp/lvns3_extract/out/` — 522 LF2 ファイル（tmpfs、再起動なければ残存）

### スクリプト

- `/tmp/analyze_ab.py` / `/tmp/analyze_short.py` / `/tmp/analyze_maxlen.py` / `/tmp/score_naive.py` — TSV 統計分析

## 学習・恒久化

このセッションで判明した学習（memory に永続化済み）:

1. `feedback_retro_decode_100pct.md` — 100% byte-exact が条件。妥協リリース提案禁止
2. `project_retro_decode_context.md` — 既に動作・公式より圧縮率高い。劣化選択の再現が目的
3. `feedback_read_artifacts_first.md` — 戦略前に artifact から状況読み取り。妥協案は目的取り違えのサイン
4. `feedback_just_do_better_tooling.md` — 道具改善は「やった方がいい」なら過剰でも実行

## 戦略的位置

- 215/522 (41.2%) はセッション開始時 0% からの大躍進
- 残 307 ファイル中、token 0 ケース 4 件は次セッション初手で取れる見込み（+1〜+150 file 期待）
- 「世界初の byte-exact LF2 エンコーダ」までの距離: 残 307 ファイル分の劣化選択パターン特定
- 次セッションは「1-dummy at r-F」を最優先で投入、結果次第で no-swap 変種または別仮説へ
