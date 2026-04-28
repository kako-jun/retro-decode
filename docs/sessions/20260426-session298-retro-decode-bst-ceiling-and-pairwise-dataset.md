---
session: 298
date: 2026-04-26
topic: retro-decode 奥村 LZSS 枠内 224 完全確定 + AI 路線 3 周目の前段 pairwise CSV 実装
---

# Session 298 — 奥村 LZSS の真の天井 224/522 を BST 構造軸まで含めて確定

## 入りの状態

- 前セッション (297) 終了時: byte-exact 215/522 (41.2%)、奥村 LZSS 軸の天井 224 確定
- memory `feedback_okumura_lzss_dead_end`: 「16 変種ハイブリッドで 224 打ち止め、機械学習 + kako-rog に転換」と書かれていた
- kako-jun の発令: 「前回のセッションの反省点や引き継ぎを読んでウルトラシンクしてから方針を決めて」「もともと最初も AI を使って特定するアプローチだった。それで失敗した記録も把握して」

## ウルトラシンク

私は最初「DP backtrack の前段 → ML」を提案したが、kako-jun の指摘で過去 AI 路線の失敗履歴を全て読み返した:

| 周 | 試したこと | 結果 |
|---|---|---|
| 0 | NN 75.3% (セッション 282 開始時点) | 詰み |
| 1 | 決定木 CART 訓練精度 100% (セッション 287) | 実は test fixture 3 ファイル過学習、本番 0/584 |
| 2 | 真因 4 件修正 + 522 ファイル正規 CSV で深さ 8 学習 (セッション 292-294) | **訓練精度 19.04% (ベースライン +0.24pt)** |

→ AI 路線は既に 2 周失敗。それで 295-297 で奥村変種戦線に切り替え、215/522 まで来た。

`feedback_okumura_lzss_dead_end` の根拠 = **16 変種 (dummy 配置 / THRESHOLD / tie 規則 / サイズ判定の 4 軸)** だが、**BST 構造そのものを触る変種は 1 つも試していなかった**。297 末尾に「次にやる」と書かれた未着手項目:

1. insert_node の左右反転 (cmp 初期値 0 + `<=` 更新)
2. swap-with-r ブロックを実行しない
3. F dummy をすべて pos r-F に重ねる

→ AI 路線 3 周目に入る前に、**未着手 BST 構造変種を尽くす**判断。これで天井 224 が動かなければ初めて「奥村枠を完全に出る必要がある」という正当な根拠ができ、AI 路線 3 周目の設計に進める。

## やったこと

### 1. BstMode enum + Okumura 構造体への組み込み

`src/formats/toheart/okumura_lzss.rs` に `BstMode { Standard, LeftFirst, NoSwap }` を追加。`Okumura` 構造体に `bst_mode: BstMode` フィールド、`insert_node` 内で:

- LeftFirst: cmp 初期値 -1 + `cmp > 0` のみ右へ
- NoSwap: F バイト完全一致時の swap-with-r ブロックを skip、新ノード r は BST に入れない (孤立、dad[r] = NIL)

### 2. lson のサイズ拡張 (構造的 fix)

LeftFirst で root pseudo-node (N+1..N+256) の左探索を許すために `lson` を `[i32; N + 1]` → `[i32; N + 257]` に拡張。`init_tree` でも LeftFirst モードでは `lson[N+1..=N+256] = NIL` 初期化を追加。Standard モードでは余分な末尾領域は未使用。

`OkumuraSnapshot.lson` の型も追従。

### 3. 3 変種関数 + round-trip テスト追加

- `compress_okumura_no_dummy_left_first` (cmp init -1)
- `compress_okumura_no_dummy_no_swap` (swap-with-r skip)
- `compress_okumura_dummy_no_swap` (F dummy + swap skip)

各 round-trip テストと run-of-spaces テスト追加。`encode_loop` ヘルパー関数で重複コードを集約。

cargo test --release: 17 tests pass。

### 4. bench に 3 variants 追加 + 集合演算サマリ更新

`src/bin/lf2_token_bench.rs` の `variants` 配列に追加、union_all・差集合表示も追加。

### 5. 522 ファイル bench 実行 → 天井 224 不変

```
Variant: no_dummy_left_first identical=215/522   (no_dummy と完全一致集合)
Variant: no_dummy_no_swap    identical=1/522     (全壊)
Variant: dummy_no_swap       identical=1/522     (全壊)

|no_dummy_left_first \ no_dummy| = 0  (lf が新規に当てる)
|no_dummy \ no_dummy_left_first| = 0  (lf で失った no_dummy)
|no_dummy_no_swap \ no_dummy|   = 0
|no_dummy \ no_dummy_no_swap|   = 214
|dummy_no_swap \ no_dummy|      = 0
|no_dummy \ dummy_no_swap|      = 214
|全 11 変種 ∪| = 224  ← 天井不変
```

中間ログ: `/tmp/token_bench_v11_bst.txt`

### 6. LeftFirst が無意味な構造的洞察

**奥村 BST の inner loop で cmp は次の outer iteration の direction 判定に使われない**。理由:

- inner loop で `cmp = key[i] - text_buf[p+i]`、`cmp != 0` で break
- outer loop の最初の方向判定 (`cmp >= 0` vs `cmp > 0`) の差は cmp == 0 でのみ生じる
- cmp == 0 は F バイト全一致 → take 評価 → `if i >= F { break }` で outer も break
- つまり cmp == 0 は次 outer iteration に持ち越されない
- → Standard と LeftFirst は最初の root 遷移以外で完全に同じ動作

NoSwap が壊れる理由: 新ノード r を BST に入れないと後続の長マッチ探索で r の内容を参照できず、長さの推移が壊れる。Leaf が後続マッチを正常に取れる事実と乖離。

### 7. memory 補強

`feedback_okumura_lzss_dead_end.md` を更新:
- 「16 変種で 224」→「19 変種で 224、BST 構造軸まで含めて天井確定」
- LeftFirst が無意味な構造的洞察を明文化
- NoSwap が壊れる理由を明文化

これで本当に**「奥村 LZSS 枠内」の主要な構造変種 (dummy 配置 5 + tie 3 + サイズ判定 5 + BST 構造 3 = 計 16+ 軸) を一通り尽くし、真の天井 = 224/522** を確定。

### 8. AI 路線 3 周目の前段: pairwise CSV 実装

過去 2 周失敗の根本原因 = **「Leaf が選んだ正解 + 同 token の全合法候補集合」がデータに乗っていなかった**。CSV にあったのは「最近候補 1 個分」のみ。

新規バイナリ `src/bin/lf2_pairwise_dataset.rs` (262 行):

- 各 LF2 ファイルを Leaf decode 出力で ring 駆動 (シミュレーション側の発散を排除)
- 各 token 位置で `enumerate_match_candidates_with_writeback` で全合法 (pos, len) を取得
- 同最大長候補 2..=32 の token について 1 行 1 候補で is_leaf=1/0 ラベル付き CSV 出力
- スキーマ: `file, token_idx, ring_r, n_cands, n_max, max_len, cand_pos, cand_len, cand_dist, is_leaf, prev_kind, prev_len`
- n_max > 32 はスキップ (4096 全 pos が同点な初期段階等は弁別力ゼロ)

5 ファイル profile:

| ファイル | rows | size | time | n_max>32 skipped |
|---|---|---|---|---|
| C0101.LF2 | 22,653 | ~1MB | 2.5s | 2,112 |
| C0A01.LF2 | 32,592 | 1.5MB | 3.7s | 3,805 |
| H32.LF2 | 50,772 | 2.4MB | 20.4s | 2,148 |
| S01D.LF2 | 73,865 | 3.5MB | 13.4s | 1,441 |
| V10.LF2 | 63,074 | 3.0MB | 18.8s | 1,135 |

平均 ~48,000 行/file, ~2.3MB/file, ~12s/file → 522 ファイル extrapolation: **~25M 行, ~1.2GB, ~100 分@mac single core**。

特徴量はミニマル v1。ML 解析中に追加していく方針 (input_byte_at_s, byte_at_cand_pos, image_width 系, prev_N_token 履歴等)。

## 数値の現状

- byte-exact 一致: **215/522 (41.2%)** (compress_okumura_no_dummy、不変)
- 奥村 LZSS 全軸ハイブリッド天井: **224/522 (42.9%)** (BST 構造軸も含めて完全確定)
- 残ギャップ: 522 - 215 = 307 ファイル (= 奥村枠外の劣化選択を機械学習等で解明)

## 次セッションでやること（優先順位順）

### 1. 522 ファイル pairwise CSV 一括生成

選択肢:
- (a) **Mac で background 実行** (~100 分): 確実、追加環境構築なし
- (b) **kako-rog (192.168.0.115, ROG WSL2 Arch) に転送 + 並列実行**: 522 並列で数分で済むが SSH 越し転送 + 環境構築の overhead

判断基準: kako-jun の方針次第。kako-rog 環境がまだ立っていないなら (a) が早い。立っているなら (b) が遥かに速い。

### 2. CSV を Python/Polars でロード → 行統計

- n_max 別の `is_leaf=1` 分布 (rank 1, 2, ..., n_max の頻度)
- cand_dist 分布 (近距離 / 遠距離 偏り再確認、tiebreaks50.tsv 結果と整合性チェック)
- prev_kind / prev_len 別の偏り (履歴の影響度)
- 「正解 vs not-正解」の特徴量分布差 = ML feature importance の事前見積もり

弁別力が弱い特徴量しかなければ ML 投入前に追加特徴量を CSV に足す。

### 3. LightGBM / XGBoost で binary classification 学習

- target = is_leaf
- features = 既存 9 列 + (追加候補の特徴量)
- group = (file, token_idx) で learning-to-rank として扱える
- pairwise binary classification の AUC が 0.95+ なら筋が良い、0.7 以下なら特徴量設計を再考

### 4. feature importance → 追加特徴量の設計

考えられる追加候補:
- `input_byte_at_s`, `input_byte_at_s+1`, ..., `input_byte_at_s+max_len-1` (入力バイト列、最大 18)
- `byte_at_cand_pos`, ... (候補先のバイト列)
- `cand_pos % image_width`, `cand_pos / image_width` (画像座標系)
- `prev_2_token_kind`, `prev_2_token_len` (履歴 2 step)
- `cand_dist_log2`, `cand_dist_bin` (距離のビン化)

### 5. 224 を超える証拠が出れば、ML 結果をルール化して新変種を実装

ML が 90%+ 当てる軸を見つけたら、それをハードコードルール化して `okumura_lzss.rs` に新変種として追加 → bench で実測。

## 環境メモ・成果物

### コード (commit 済み push 済み)

- `cb07cd9`: BST 構造変種 3 種追加・天井 224 補強
- `60c2abb`: pairwise candidate dataset 生成 bin

変更:
- `src/formats/toheart/okumura_lzss.rs`: BstMode enum / Okumura 拡張 / 3 変種関数 / encode_loop ヘルパー / lson size 拡張 / 4 round-trip テスト追加
- `src/bin/lf2_token_bench.rs`: 3 variants 追加 + 集合演算サマリ拡張
- `src/bin/lf2_pairwise_dataset.rs` (新規, 262 行)

### 中間ログ・データ

- `/tmp/token_bench_v11_bst.txt` — BST 変種追加後の bench 全出力
- (テスト用 CSV は削除済み、全 522 ファイル CSV は次セッション)

### memory 更新

- `feedback_okumura_lzss_dead_end.md`: 16 変種 → 19 変種 (BST 構造軸追加)、LeftFirst/NoSwap が機能しない構造的洞察を追記

## 学習・恒久化

このセッションで判明した運用知:

1. **「枠内 / 枠外」の判定は試した変種の総和で決まる**: memory `feedback_okumura_lzss_dead_end` が「16 変種で 224 打ち止め」と書いていたが、その 16 変種は「dummy 配置 / THRESHOLD / tie / サイズ」の 4 軸だけで BST 構造軸を含まなかった。「未着手の軸が残っていないか」を確認してから次の戦略に移る、という反省。memory を「結論」として固めると未着手軸を見落とす危険があるので、根拠を明示しておくのが大事。

2. **AI 路線 3 周目に入る前の正当な根拠**: 「過去 2 周失敗 + 奥村枠は本当に天井 224 で打ち止め」を構造的に確認した上で進む方が、また「特徴量足りないだけ」で 3 回目の失敗を繰り返すのを避けられる。3 周目の設計は pairwise learning-to-rank で「Leaf 正解 + 同 token 全候補」を学習データにする = 過去 2 周で欠けていた最重要情報を補う。

3. **LeftFirst の構造的失敗から学ぶ「変種の事前検証」の重要性**: LeftFirst は「奥村 BST の左右反転」と直感的には別動作に見えるが、実装してみたら最初の root 遷移以外で Standard と完全同じ動作になることが構造的に判明した (cmp == 0 が次 outer iteration に持ち越されない設計)。この種の「実装前に意味の有無を考える」ステップを習慣化すべき。

## 戦略的位置

- 数値前進: なし (215/522 維持)
- 構造的進展: **奥村 LZSS 枠内の真の天井 224 完全確定** + AI 路線 3 周目の正当な根拠確立 + pairwise CSV 生成 bin 実装 + 5 ファイルで動作確認
- 「世界初の byte-exact LF2 エンコーダ」までの距離: 残 307 ファイル分の劣化選択パターンを ML で解明 (= AI 路線 3 周目の本番)
