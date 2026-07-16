# AI 路線 v12: BST 完全状態シミュレーション実験 (2026-07-16)

## 概要

Issue #14。v8 データセットで残った **cross-tie 衝突 5,401 グループ**（同一ローカル特徴量なのに Leaf の選択が異なる）が、「BST 探索順 rank」特徴量の追加で消滅するかの検証。仮説: Leaf のタイブレイクはローカル文脈ではなく、奥村 BST の完全状態（挿入・削除の全履歴が作る木構造）で決まっている。

## 実装物 (commit `9f75713`)

### OkumuraSim (`src/formats/toheart/okumura_lzss.rs`)

Leaf の実トークン列で奥村 BST を teacher-forcing 進行させるシミュレータ。4 モード:

| SimMode | 初期化 |
|---|---|
| Basic | 原典どおり F 個 dummy 挿入 (InsertNode(r-F..r-1)) |
| NoDummy | dummy 挿入なし (`compress_okumura_no_dummy` 相当) |
| DummyThenDrop | dummy 挿入 → token 0 直後に残存 dummy を全 DeleteNode |
| LeftFirst | Basic + `BstMode::LeftFirst` (左右反転探索) |

- `search_trace(r, max_len)`: tie token 直前に呼ぶ **read-only** トレース。insert_node と逐語一致の比較経路 (KeyMode::Byte0 root key、index 1 からの cmp 計算、LeftFirst 反転規則) で木を辿り、一致長がちょうど max_len のノードを訪問順に `(pos, rank, depth)` で返す。rank 1 = 原典 insert_node が採用するノード
- `advance(emitted_bytes)`: token 確定後、原典 Encode() 後半と同一の回転 (DeleteNode(s) → text_buf 書込 overlap 複製込み → InsertNode(r))。debug ビルドでは emitted bytes と text_buf[r..] の一致も assert

### lf2_pairwise_dataset_v12 (`src/bin/lf2_pairwise_dataset_v12.rs`)

v8 (53 列) + 末尾 8 列 = **61 列**:

- `bst_rank_basic / bst_rank_nodummy / bst_rank_dtd / bst_rank_leftfirst` (u32): 探索が何番目に訪問する max_len ノードか (1 始まり)。0 = 候補が木に不在
- `bst_depth_basic / bst_depth_nodummy / bst_depth_dtd / bst_depth_leftfirst` (u8): root からの段数。不在 = 255

全 token で 4 sim を advance し、`debug_assert` で sim.r と ring ループ r の一致を常時検証。

## 使い方

```bash
cargo run --release --bin lf2_pairwise_dataset_v12 -- <LF2ディレクトリ> <出力CSV>
```

注記: 本生成の前に **debug ビルドで一度流して debug_assert (r 同期・teacher forcing 整合) を効かせてから** release で回すこと。

## 検証状況

- 単体テスト緑 (計 15 本、commit `9f75713` + `77ea752`): rank 1 == insert_node 採用ノード (全 4 モード)、Basic の advance が原典 Encode() と全過程で BST 一致、trace の read-only 性・冪等性、境界 (max_len 境界・入力長境界・ring wrap・literal only)・事故パターン (teacher forcing 違反・過長 emitted の debug 検出、DummyThenDrop の dummy 全消滅) など。加えて合成 600 byte の teacher-forcing 全過程で「trace pos 集合 ⊆ enumerate_match_candidates_with_writeback の max_len 候補集合」「全 token で sim.r == ring r」「BST 親子リンク整合・無循環」を assert。lib テスト 32/32 緑
- debug ビルドで `test_assets/generated` の LF2 3 本を実走: panic なし・全行 61 列・rank/depth に実分布 (rank 17 種、depth 34 種、不在 0/255 も出現)
- **源 LF2 522 本での本生成と Stage 0 (C1001 での trace 集合 == enumerate max_len 候補のうち木に在るもの assert) は未実施**。物理 SSD 接続待ち

## 判定フロー

1. 522 本で v12 dataset 生成 → cross-tie 衝突グループを再集計
2. **衝突 0**: rank 特徴量が Leaf のタイブレイクを完全決定 → 決定木抽出 → Rust encoder 化
3. **微減にとどまる**: BST 状態仮説を棄却 → 姉妹ファイル並走ダンプ (シリーズ文脈の直接観測) へ転進
