# Session 297 — retro-decode dummy 戦線天井 224 確定 + 4 変種追加

日付: 2026-04-26
対象: kako-jun/retro-decode#5（LF2 100% 一致再エンコーダ、世界初狙い）

## 入りの状態

- 前セッション (296) 終了時: byte-exact 215/522 (41.2%)、no_dummy 変種が決め手
- 引継ぎ指示: 「no_dummy + 1-dummy at r-F 変種を投入。+1〜150 ファイル獲得期待」

## kako-jun の発令

「ウルトラシンクでやれ。それにまかせる。全セッションでの記憶を引き継いでること前提だが。」

→ 自分で判断・複数仮説を並列に検証する戦線に切り替え。

## やったこと

### 1. 仮説 1: `compress_okumura_one_dummy_at_rf` 投入

`no_dummy` のままで `insert_node(r - F)` を 1 個だけ事前挿入する変種。狙いは「token 0 で `Match{pos=0xFDC, len=18}` を BST から取得し、token 0 残差 4 件を救済」。

結果: **166/522 (-49 後退)**。token 0 残差は救済されたが、r-F dummy が BST に居続けて後続 53 ファイルでカスケード破壊（新出 `MATCH_SAME_LEN_DIFF_POS:len<=5:leaf_nearer 45 files`）。**「dummy は害」を再確認**。

### 2. 仮説 2: `compress_okumura_dummy_then_drop` 投入

奥村原典どおり F dummy 挿入 → token 0 出力 → 全 dummy `delete_node` で削除 → no_dummy 等価で進行。狙いは「奥村が当てる 171 + no_dummy が当てる 215 のいいとこ取り」。

結果: **215/522 ±0 (no_dummy と完全一致集合)**。token 0 で `MATCH_vs_LIT:len=18` クラスタの sample_offsets が `[0,0,0]` → `[5,12,13]` に動いた = token 0 救済はできているが、その先の 5/12/13 トークン目で別の発散が起きて identical 達成にならない。質的改善は match_rate +0.092pt / mean_first_div 1113→1223 のみ。

### 3. 集合演算機能を bench に組み込み

`lf2_token_bench` を以下に拡張:
- `VariantResult` に `identical_set: BTreeSet<String>` を追加
- main 末尾で集合演算サマリ出力（共通・差集合・和集合の天井）
- Leaf token0 ヒストグラム（variant 非依存、Match{F}/Match{<F}/Literal）
- CSV 出力（`LF2_BENCH_CSV` 環境変数、`/tmp/lf2_bench.csv` デフォルト）

これで戦線の天井を即時に把握できる道具を整備。

### 4. 集合演算で天井 224 を確定

```
|tie_strict_gt|              = 171
|no_dummy|                   = 215
|dummy_then_drop|            = 215
|tie_strict_gt ∩ no_dummy|   = 162
|tie_strict_gt \ no_dummy|   = 9   (奥村だけ当たる)
|no_dummy \ tie_strict_gt|   = 53  (no_dummy だけ当たる)
|dummy_then_drop \ no_dummy| = 0
|tie_strict_gt ∪ no_dummy ∪ dummy_then_drop| = 224
```

**dummy 配置軸の真の天井 = 224 ファイル**。現状 215 + 9 の余地。

Leaf token0 分類 (522 ファイル):
- Match{len=F=18}: 3
- Match{len<F}: 1
- Literal: 518 (99.2%)

→ **token 0 救済は方向違い**（最大でも +1% にしかならない）。

### 5. 仮説 3: `compress_okumura_no_dummy_min4` 投入 (THRESHOLD=3)

仮説「Leaf は 3 バイトマッチをリテラル 3 個より得と判断せず Match を抑制」。

結果: **0/522, match_rate 0.502%**。完全敗北。**Leaf は 3 バイトマッチを使っている**。THRESHOLD=2 が正解、min len 引き上げは方向違い。

### 6. 仮説 4: `compress_okumura_no_dummy_dyntie` 投入

セッション 295 の U 字分布発見（max_len=18 → rank=1 87.3%, max_len=3 → rank=末尾 60.5%）に対応する仮説:
- 短マッチ (len ≤ 3) は AllowEq で上書き
- 長マッチは StrictGt で保持

`TieMode::DynamicShortEq` を新規追加し insert_node 内で動的分岐。

結果: **0/522, match_rate 69.144%**。完全敗北。**短マッチでも Leaf は最初の候補を保持している**。U 字分布の B (rank=末尾) は AllowEq では再現できない。

### 7. 5 変種ハイブリッド天井確認

```
|tie_strict_gt ∪ no_dummy ∪ dummy_then_drop ∪ no_dummy_min4 ∪ no_dummy_dyntie| = 224
```

**変種を増やしても天井は 224 のまま**。THRESHOLD/tie 規則の動的化は **すべて改悪**。dummy 配置軸も完全飽和。

## 確定した事実

1. **dummy 戦線の真の天井 = 224/522 (42.9%)**
2. **奥村 171 ⊂ no_dummy 215 ではない**（共通 162、奥村だけ 9 が存在）
3. **THRESHOLD は 2 で固定**（3-長 Match を Leaf も使う）
4. **tie 規則は StrictGt で固定**（短マッチでも先勝ち）
5. **Leaf token0 = Literal が 99.2%**（token 0 救済は方向違い）
6. **dummy_then_drop と no_dummy は完全一致集合**（drop しても新規獲得 0）

## 次セッションでやること（最優先順）

### 1. 条件付き drop ハイブリッド変種 (確実な +9 達成見込み)

`compress_okumura_dummy_then_drop_conditional`:
- F-dummy 挿入
- token 0 を出力
- **token 0 が Match なら dummy は維持（奥村と同じ動作になる）**
- **token 0 が Literal なら dummy を全削除（no_dummy と同じ動作になる）**

これで「奥村が当てる 9 + no_dummy が当てる 53 = 62 ファイル」を統合し **224/522 を達成**できる見込み。
ただし token 0 の Match/Literal 分岐は本物のエンコーダにとって自然なロジックではなく、「F-dummy が token 0 で活きるかどうか」をエンコーダが先読みできるかが鍵。

### 2. inspect ツールで「奥村だけ 9」「no_dummy だけ 53」のカスケード差分観察

- `lf2_first_div_inspect` を奥村と no_dummy の両方で並走させ、token 1〜10 の選択肢ダンプ
- 奥村が当てる 9 ファイル: `["C0601.LF2", "CLNO_07.LF2", "H42.LF2", "H44.LF2", "S26N.LF2", "V71.LF2", "V80.LF2", "V89.LF2", "V91.LF2"]`
- no_dummy が当てる 53 ファイルから 5-10 サンプル選び対比
- パターンが見えれば「dummy 動的維持/削除」の判定式が立つ

### 3. insert_node の左右反転変種

- `cmp` 初期値を 0 にする（左に行く）
- `if cmp >= 0` を `if cmp > 0` にする（等しい時左に行く）
- 経路が変わるので新しい識別パターンが出る可能性

### 4. 224 の壁を超えるための再考

天井 224 は「奥村 LZSS の小変更で取れる上限」。残 298 ファイル (57.1%) を取るには:
- 奥村と全く異なるアルゴリズム（hash chain LZ77 など）
- 奥村の枠組み内で大きな変更（例: 探索範囲制限、特殊な早期終了条件）

最終手段: SSD 内で Leaf 同時代の他製品（Filsnown / To Heart / Comic Party）にエンコーダバイナリが残っていないか再走査。

## 環境メモ・成果物

### コード（commit `d719818`、push 済み）

**変更**:
- `src/formats/toheart/okumura_lzss.rs` — `TieMode::DynamicShortEq` 追加、4 変種関数追加 (`compress_okumura_one_dummy_at_rf` / `compress_okumura_dummy_then_drop` / `compress_okumura_no_dummy_min4` / `compress_okumura_no_dummy_dyntie`) + 各 round-trip テスト
- `src/bin/lf2_token_bench.rs` — 集合演算機能 + Leaf token0 ヒストグラム + CSV 出力

### 中間ログ

- `/tmp/token_bench_v6_one_dummy.txt` — one_dummy_at_rf bench
- `/tmp/token_bench_v7_drop.txt` — dummy_then_drop bench
- `/tmp/token_bench_v8_set.txt` — 集合演算初出
- `/tmp/token_bench_v9_min4.txt` — min4 失敗確認
- `/tmp/token_bench_v10_dyntie.txt` — dyntie 失敗確認 + 5 変種ハイブリッド天井 224 確定
- `/tmp/lf2_bench.csv` — 522 ファイル × 全 variant の identical 1/0 マトリクス（Python 分析用）

### 戦略的位置

- 215/522 (41.2%) のまま今セッションでは数値前進なし
- ただし **真の天井 224 を確定**し、dummy 戦線の終わりを画定 = 大きな構造的進展
- 「世界初の byte-exact LF2 エンコーダ」までの距離: 残 298 ファイル分の劣化選択パターン特定（=奥村 LZSS 枠外の機構が必要)

## 後半セッション: サイズ判定方式の検証 (commit `8619855`)

kako-jun の「条件付き drop ハイブリッド実装」発令を受けて続行。観察 → 仮説 → 実装 → 集合演算検証の 1 周を 6 回回した。

### inspect ツール `lf2_oku_vs_nodummy_inspect` 新規作成

奥村/no_dummy/Leaf を side-by-side でダンプ。決定的観察:

- **CLNO_07** (奥村だけ当たる): token 20 で Leaf=Match(p=0FEC, l=18)、no_dummy=Literal(20)。dummy 位置 0FEC が直接 long match として活きる単色画像 (入力先頭 18 バイト全 0x01)
- **H34** (no_dummy だけ当たる): token 1423 で Leaf=Match(p=053C, l=03)、奥村=Match(p=0CBC, l=03)。len=3 短マッチで pos 差。dummy が居ると古い pos が選ばれ Leaf と乖離

### 「奥村だけ当たる 9 ファイル」の先頭 18 バイト分析

5 ファイルが完全単色: C0601, CLNO_07, H42, V80, V91
4 ファイルが混在: H44, S26N, V71, V89

→ 仮説 D「先頭 F 同値で奥村」と仮説 E「両者 encode し小さい方採用」を実装

### 6 変種を実装・bench 投入

| 変種 | identical | 新規 vs no_dummy | 失った |
|---|---|---|---|
| `uniform_head` (先頭 F 同値判定) | 192 | +5 | -28 |
| `min_tokens` (token 数 min) | 171 | +9 | -53 |
| `min_bytes` (byte 長 min, タイ=奥村) | 171 | +9 | -53 |
| **`min_bytes_strict`** (奥村<no_dummy のみ奥村, タイ=no_dummy) | **198** | **+8** | **-25** |
| `min_bytes_oku_pref` (no_dummy<奥村 のみ no_dummy) | 171 | +9 | -53 |
| `combo` (uniform_head + min_bytes_strict 合成) | 179 | - | - |

### 戦況確定

- **現状最良: no_dummy = 215/522 (41.2%)** (純粋な数値前進なし)
- min_bytes_strict は「奥村だけ当たる 9 のうち 8」を取れたが「no_dummy だけ当たる 53 のうち 25」を失う
- combo は両方統合のはずが純減 (179) → 「先頭 F 同値だけど Leaf=no_dummy」なファイル群が見える
- **サイズ判定方式は半分しか正しくない**：奥村サイズ < no_dummy サイズ なのに Leaf=no_dummy なファイルが 25 件残る
- 16 変種ハイブリッド天井依然 224 不変

### 次セッション最優先

- [ ] **min_bytes_strict が失った 25 ファイルの inspect** (H53, H55, H60, H62, H63, H72, H80, H83, H90, S01D, S02D, S03D, S08D, S25D, S26X, S42D, S43D, S48D, V10, V1F, ...)
- [ ] サイズ差閾値・サイズ比の検証 (奥村サイズが極端に小さい時のみ奥村)
- [ ] 別の特徴量探索 (色数 / 画像種別 / 入力統計)

## 学習・恒久化

このセッションで判明した運用知:

1. **「天井検証」の重要性**: 1 軸ずつ試行する前に「集合演算で複数変種の和集合が動かないか」を測ると、その軸が飽和か未飽和かが即時に判明する。bench に集合演算機能を追加するだけで、無駄な試行を 5 回分節約できた可能性
2. **failed bench は捨てない**: min4=0/522, dyntie=0/522 という「全敗北」結果も「Leaf はその軸で固定」という強い陽性情報。負例も permanent な仮説棄却として残す
3. **「合成変種は単純な OR にならない」**: combo (179) が uniform_head (192) と min_bytes_strict (198) のいずれよりも下回った。複数判定の組み合わせは "両方が間違える" ケースで純減を生む。観察 → 単一判定での天井 → そこから合成、の順序が必要

## 終盤: バイナリ解析路線の検証と戦略全面転換

kako-jun の問い「数十年前の暗号も破られている、不可能ではないのではないか」を受けて、技術的に到達可能な手段を再検証。

### MKFONT.EXE / LVNS3.EXE バイナリ確認 (SSD 接続後)

- MKFONT.EXE (36KB): 出力は `LVNS3_%02d.KNJ` (フォントファイル)、`CreateFontIndirectA + EnumFontFamiliesA + GetGlyphOutlineA` で Windows GDI から glyph 取得
- KNJ_ALL.KNJ (199KB): 先頭 0xFF だらけ、magic なし = **無圧縮の生ビットマップ**
- MKFONT 内 LZSS マスク 0x00000FFF: **0 hits** (LVNS3 は 13 hits) → MKFONT は LZSS 使用していない確定
- LVNS3.EXE の `cmp imm8=17` 5 hits を全数 disasm 確認 → 全部「ゲームロジック内のキャラ ID/シナリオ番号判定」(連続 cmp 4/7/10/12/14/17 ; je) で encoder ループではない

→ **MKFONT は LF2 encoder ではない確定、LVNS3 内 encoder static link の証拠なし**

### kako-jun の方針指示

- ソース入手・人間コンタクト系: **禁止** (技術勝負を貫く)
- Ghidra フル解析: **禁止** (可能性低、時間対効果悪い)
- **kako-rog (192.168.0.115, ROG WSL2 Arch Linux) の GPU/CPU 計算リソースを前提に重い処理を躊躇なく回せ**

### 最終戦略 (次セッション以降)

奥村近傍探索を完全に捨て、以下の 3 路線に転換:

1. **特徴量豊富化 + 機械学習 (本命)**:
   - セッション 294 で 5 特徴量 (distance/length/image_x/image_y/ring_r) のみ → 訓練精度 19%
   - 追加すべき特徴量: 同長候補集合 (pos 配列) / 前 N token 履歴 / 先読み入力 36 byte / ring 内バイト分布 / BST 深さ / 奥村 vs no_dummy の判断差
   - GBDT (LightGBM/XGBoost) または NN で 522 ファイル全 token 学習
   - kako-rog で計算
2. **DP backtrack**: 各ファイルで Leaf 出力を生成可能な全 LZSS パース列挙、共通選択ロジック抽出。522 並列で kako-rog
3. **完全別構造実装**: hash chain LZ77 / suffix array / lazy DP encoder

## 最終追記の学習・恒久化 (memory に永続化済)

1. `feedback_okumura_lzss_dead_end` - 奥村いじりは 224 で頭打ち、機械学習 + kako-rog 転換
2. `project_lf2_landscape` - 戦況スナップショット (数値・確定事実・ファイルリスト・関連 commit)
3. `feedback_no_external_help` - コンタクト/ソース入手/Ghidra 全部禁止、技術手段のみ
4. `feedback_avoid_short_term_promises` - 「30分でできる」発言禁止、判定軸の弁別力確認必須

## 今夜の戦況サマリ

- 数値前進: なし (215/522 維持)
- 構造的進展: 大 (天井 224 確定 + 16 変種 + サイズ判定方式の限界判明 + バイナリ路線 NG 確定 + 戦略全面転換)
- 次セッション初手: 機械学習特徴量豊富化の設計 → kako-rog 環境構築 → CSV 再生成 → GBDT 学習
