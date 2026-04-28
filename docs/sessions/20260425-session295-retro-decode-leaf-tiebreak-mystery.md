# Session 295 — retro-decode 目視観察フェーズ：Leaf は greedy だがタイブレーク規則が未知

日付: 2026-04-25
対象: kako-jun/retro-decode#4（LF2 100% 一致再エンコーダ）

## 着手の背景

Session 294 で決定木学習を 1M サブセット深さ 8 で完走したが訓練精度 19.04%（多数派ベースライン +0.24pt）。**特徴量 5 個では情報不足**と結論。次セッションは「ML より先に目視観察でルール仮説を立てる」とした。本セッションはその第一手。

## やったこと

### 1. `lf2_first_diff` 単一モードに距離列＋全件出力を追加

`src/bin/lf2_first_diff.rs:411-` の `print_single` を改造:

- 各候補に `distance = (r - pos) mod 4096` を表示
- `MAX_PRINT = 40` 制限を撤廃し全候補を出す
- ソート順を `(len desc, distance asc)` に変更（同じ長さの中で近い順に並ぶ）
- `Leaf distance` / `Okumura distance` の数値サマリを追加

`enumerate_match_candidates_with_writeback` の出力（pos 昇順）と並べ替え後の差を見比べることで、Leaf 選択の偏りが視覚化できる。

### 2. 6 ファイル（C0A01, C0A02, C1F06, C018A, C0405, C0502）で第一発散を観察

#### 観察結果サマリ

| ファイル | 第一発散 token | max len | max len 候補数 | Leaf 選択 distance | distance 順位 | pos 順位（昇順） |
|---|---|---|---|---|---|---|
| C0A01 | 763 | 5 | 6 | 1043 | **1（最近）** | 6/6（最大）|
| C0A02 | 490 | 3 | 5 | 3476 | **5（最遠）** | 2/5 |
| C1F06 | 656 | 4 | 11 | 3044 | 2 | 10/11 |
| C018A | 369 | 3 | 多 | 164 | 4 | 中位 |
| C0405 | 465 | 7 | 多（連続範囲）| 2113 | 5 | 中位 |
| C0502 | 763 | 5 | 5 | 1737 | 2 | 2/5 |

#### 見えたこと（堅い結論）

1. **長さは常に最大長**: 6 件全てで `Longer candidate than Leaf's choice: 0`。Leaf は lazy match や cost-based ではなく**長さに関しては greedy**。これは大きな絞り込み。
2. **揺れているのは「同最大長タイ内での pos 選択」だけ**: 距離順位も pos 順位も一貫しない。「常に最近」「常に最遠」「pos 昇順 1 番目」のような単純な規則では説明できない。

#### 否定された仮説

- **「最近距離」**: C0A02 で破綻（最遠を選んでいる）
- **「最遠距離」**: C0A01 で破綻（最近を選んでいる）
- **「pos 昇順 1 番目」**: C0A01 が pos 最大、C0A02 が 2 番目、C0502 が 2 番目で揃わない
- **「pos 降順 1 番目」**: C018A・C0502 で破綻

### 3. 残る仮説（次セッションの検証候補）

- **(a) ハッシュチェーン式 LZSS**: `hash(c1, c2, c3) → head[h] → prev[pos]` のチェーンを辿り、辿る順は「最近 hash 衝突したものから」。同じ最大長が複数ある場合「チェーン上で最初に max に達したもの」を選ぶ。Leaf 選択 pos の "ハッシュバケット" を推定すれば検証可能
- **(b) ring を領域分割しての走査順**: 4096 ÷ N 分割しクォドラント順スキャン。各クォドラントで最後に見つけた最大長
- **(c) `prev_pos[pos]` 双方向リンク**: 古典 Storer-Szymanski の二分木では、同じプレフィックスを持つノードがツリー中で繋がる。挿入順がツリー位置を決めるので、特定の挿入パターンが特定の選択順を生む

(a) が最も可能性高い。古い LZSS 実装（`lzss.c` Haruhiko Okumura 版以前のもの、特に `lzss15.c` や Loi MOU の版、Yoshi の Lha の流派など）はハッシュチェーン式が主流。

### 4. 決定打となる別の手段

リバースエンジニアリング：**Leaf/AQUAPLUS の TOHEART/LVNS 実行ファイル本体に圧縮ルーチンが埋まっている可能性**がある。LF2 圧縮ロジックは元々ゲーム内ツール／パッカーの実装そのもの。逆アセンブルできれば 100% 確定。

ただし:
- 1990 年代の DOS/Win16 実行ファイルは IDA Pro や Ghidra で開けるが、難読化はないだろう
- ROM パッチが目的（Issue #5）なので、最終ゴールが「アルゴリズムを再現する Rust 関数」なら逆アセンブルでルーチンを直接読むのが最短
- ML は近似でしか追えない。100% 一致を求めるなら**ルール抽出が必須**であり、ルール抽出はソース読みが最確

## セッション後半で追加判明したこと

### Web 調査（2 サブエージェント並走）の結論

- **公開された LF2 エンコーダは世界に存在しない**（fan-made / 翻訳パッチ系 / Susie プラグイン全滅）
- arc_unpacker / GARbro / xlvns / akkera102/gbadev-ja-test の `th_bmp.c` 等、確認できる LF2 関連実装は**全部デコーダのみ**
- arc_unpacker README は "Packing / encoding support? Not going to happen." と明記
- GARbro `ImageLFG.cs` は LFG（LF2 と同じ LZSS パラメータの 16 色版）の Write を `NotImplementedException` で放棄
- 中国漢化グループ・2ch 系日本語ソースもヒットゼロ。LF2 略号は Little Fighter 2 で SEO が完全に汚染されており Leaf の LF2 は実質 invisible
- 戦略的示唆: **retro-decode が完成したら世界初の LF2 エンコーダ**になる。arc_unpacker / GARbro へ PR できる立ち位置

### SSD 調査の結論

- `/Volumes/Extreme SSD/プロトタイプ/他サークルの同人ゲーム/LEAF/LVNS3/` に **LVNS3.EXE / lvns3.org / MKFONT.EXE**（PE32 i386 Win32）あり
- ただし **LVNS3.EXE はデコーダ側のみ**（strings で LEAF256 / LEAFPACK 等のシンボル確認）。エンコーダは Leaf 社内ツール
- MKFONT.EXE は「Leaf Visual Novel Series Vol.3 Font Maker」、LF2 圧縮無関係
- `/tmp/lvns3_re/` にコピー済み。Ghidra で LVNS3.EXE を逆アセンブルしても得られるのはデコーダ実装の裏取りのみで、タイブレーク規則は得られない

### 追加実装: `--tiebreaks` モード

`src/bin/lf2_first_diff.rs:874-` に新モード `--tiebreaks <file.LF2>` を追加（commit `bc6a446`、push 済み）。
1 ファイル内の全マッチトークンで「同最大長候補が 2 件以上ある」状況を TSV で出力:

```
# file token_idx ring_r leaf_pos leaf_dist max_len n_max dists_csv leaf_dist_rank
```

### 50 ファイル実測 (306,562 行) のランク分布 — **U 字形を確認**

`/tmp/tiebreaks50.tsv` (306,562 行)。`leaf_dist_rank` の分布:

- 全体: rank=1 (最近) **49.6%** / rank=N (最遠) **30.1%**
- n=2: rank=1 **37.4%** / rank=2 **62.6%**（最遠優勢）
- n=5: 37.4 / 6.3 / 7.5 / 9.6 / 39.2 → **両端で双峰**
- n=10: 31.4 / 4.2 / 3.6 / 4.2 / 4.3 / 4.6 / 4.5 / 7.5 / 7.8 / 27.7 → **U 字形が明白**
- 中間ランクは n が増えるほど一様に薄くなる（一様分布なら 1/n 期待値、Leaf は明確に両端バイアス）

**この U 字分布は単一スキャンアルゴリズムでは絶対に生成できない**。Leaf は **2 つの独立した選択メカニズムを併用**している強い示唆:

- メカニズム A (~30-40%): rank=1 を選ぶ → 古典的 hash chain head（最直近の挿入を返す）
- メカニズム B (~28-39%): rank=N を選ぶ → 線形スキャン末尾、または別の遅延チェーン
- 中間バイアスはほぼフラット → 「ランダム」ではなく「両モードのいずれかが当たり、外したときだけ中間」

### 否定された追加仮説（U 字データから）

- **「常に最近距離」「常に最遠距離」**: 単峰ならどちらかにピーク。U 字なので両方否定
- **「pos 昇順 / 降順スキャンで `>=` 更新」**: これは単峰になる。U 字を生成不可
- **「ハッシュチェーン単体で `>=` 更新」**: 同上、単峰
- **「ランダム選択」**: 一様分布になり U 字にならない

## 次にやること（次セッション最優先）

### 王道路線（推し）: 「2 メカニズム競合」モデルのシミュレータ実装

`src/bin/lf2_first_diff.rs` または別バイナリで以下を実装:

1. **メカニズム A 候補**: hash chain（hash 関数 = `c1 ^ c2 ^ c3` など 3-5 種）で「head から最初の max_len 候補」
2. **メカニズム B 候補**: 線形スキャン（pos 昇順 / r からの cyclic 順 / 行幅倍数優先 など 4-6 種）で「最後の max_len 候補」
3. **競合ルール**: A と B が違う pos を返したとき、どちらを採用するか? 候補:
   - (a) 長さで勝ち（A.len > B.len なら A、逆は B、同じなら ??）
   - (b) 入力位置の何らかのビット (s & 1, s & 7 等)
   - (c) hash 衝突カウントによる切り替え
4. **スコアリング**: 50 ファイル × 全マッチで、シミュレータ予測 vs 実際の Leaf 選択の一致率を測る。一致率 95%+ なら正解アルゴリズム
5. データセット: `/tmp/tiebreaks50.tsv`（306,562 行）が既にある。最初は 1,000 行サブセットで A/B/競合ルールを当てに行く

### 補助路線: LVNS3.EXE 逆アセンブル

Ghidra で `/tmp/lvns3_re/LVNS3.EXE` を開き、デコーダ実装 (LZSS 展開ループ) を特定 → ringbuffer 操作の細部を確認 → エンコーダの実装スタイル仮説を補強。タイブレーク規則は出ないが、**Leaf 社のコード書きクセ**（変数名規約、最適化スタイル）を観察することで仮説の絞り込みに使える。低優先。

## 環境メモ

- retro-decode は `/Users/kako-jun/repos/2025/retro-decode`（private 配下ではない）
- `/tmp/lvns3_extract/out/` に 522 LF2 ファイル（tmpfs だが再起動なければ残存）
- **新規**: `/tmp/tiebreaks50.tsv`（50 ファイル分の同最大長タイ TSV、306,562 行、U 字分布の根拠データ）
- **新規**: `/tmp/lvns3_re/{LVNS3.EXE, lvns3.org, MKFONT.EXE}` (Ghidra 用にコピー済み)
- macOS は `shuf` 未導入、`sort -R` を使う
- `cargo build --release --bin lf2_first_diff` で 5 秒
- `./target/release/lf2_first_diff <file.LF2>` で第一発散とその全候補を表示
- `./target/release/lf2_first_diff --tiebreaks <file.LF2>` で全タイブレーク TSV 出力
- `lf2_first_diff.rs:411-499` が単一モード、`:501-600` あたりが `--tiebreaks` モード（今回追加）
