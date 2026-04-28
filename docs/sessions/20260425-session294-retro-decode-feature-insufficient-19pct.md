---
session: 294
date: 2026-04-25
topic: retro-decode 決定木学習を回し切り、特徴量不足で訓練精度 19% 判明
---

# セッション294 — retro-decode 決定木学習の壁・特徴量設計の見直しが必要

## 結論先出し

**今の特徴量設計のままでは決定木で 100% バイナリ一致は無理。** マシン性能・木の深さの問題ではなく、**CSV に乗っている情報が不足している**のが本質。次セッションは特徴量設計のやり直しから。

## やったこと

1. **CSV 完成確認** — 前回セッション292で開始した `lf2_first_diff --full-dataset` (PID 26223) がバックグラウンドで生き続けており、522/522 ファイル・19,398,958 行・1.0GB で完走していた
2. **学習を 3 回試した**:
   - フルデータ深さ無制限 → 44 分稼働で root 分割すら未完、メモリ peak 6.3GB で kill
   - フルデータ深さ 8 → 同じく root 分割で停滞
   - **1M サブセット深さ 8 → 13 分で完了、訓練精度 19.04%**
3. macOS `sample` コマンドで走行中プロセスのコールスタックを取得し、原因を特定
4. `train_decision_tree.rs` のソース読解で根本原因を 2 層に分けて把握（後述）

## 重要な数値

| 項目 | 値 |
|---|---|
| 全行数 | 19,398,958 |
| match トークン (学習対象) | 10,063,802 |
| literal トークン (-1, 学習除外) | 9,335,156 |
| 一意クラス数 | **65,536** (≒ 4096 × 16 = ring × len バリエーション) |
| 最頻クラス candidate_0 の比率 | 18.8% |
| 1M サブセット × 深さ 8 訓練精度 | **19.04%**（candidate_0 ベースラインから +0.24pt のみ） |
| サブセット学習木サイズ | 16KB / 100 ルール |

## 真因 (2 層)

### 層 1: アルゴリズム実装が素朴（解決可能）

`src/bin/train_decision_tree.rs` の `find_best_split` がナイーブ:

```rust
for feature in 5_features {                // 5
    for threshold in N_thresholds {        // ~9000 (image_x 640 + image_y 480 + distance 4096 + length 16 + ring_r 4096)
        for point in 10M_data {            // 10M
            left.push(point.clone());      // クローン!
        }
    }
}
```

→ root 1 ノードで **約 5×10¹¹ 操作**、約 2-3 時間。深さ無制限なら絶望的。
産業実装 (sklearn/LightGBM) は「特徴量ごとに 1 回ソートしてインデックス掃引」「ヒストグラム binning」で数千〜数万倍速い。

### 層 2: 特徴量が情報不足（本質、こっちが重い）

学習に渡している特徴量はこの 5 つだけ:

```rust
let features = vec!["distance", "length", "image_x", "image_y", "ring_r"];
```

ここで:
- `distance` = `min_distance`（最近候補の距離）
- `length` = `min_distance_length`（最近候補の長さ）

CSV にはあるが**未使用**の 3 特徴量:
- `num_candidates`
- `max_candidate_len`
- `prev_token_kind`

さらに**そもそも CSV に出ていない**情報:
- 候補リスト全体の (距離, 長さ) — 上位 N 個分の比較情報がない
- LZSS エンコーダの選択判断（しばしば「長さが等しいなら距離が短い方」「lazy match で 1 バイト先読み」等の比較ベース）に必要なのは候補同士の比較情報だが、CSV は最近距離 1 個分しか記録していない

→ 深さを増やしても、未使用 3 特徴量を追加しても、おそらく頭打ち。
→ **CSV 自体を作り直さないと本質的な進歩が無い**。

## 環境メモ（次セッションで使う）

- retro-decode のパス: `/Users/kako-jun/repos/2025/retro-decode`（`/Users/kako-jun/repos/private/retro-decode` ではない）
- CSV: `/tmp/lvns3_full_dataset.csv` (1.0GB) — まだ存在
- サブセット: `/tmp/lvns3_subset.csv` (56MB, 1M 行) — まだ存在
- 学習結果（深さ 8 サブセット）: `/Users/kako-jun/repos/2025/retro-decode/models/lf2_decision_tree_d8_sub.bin` (16KB) — git untracked
- LF2 元データ: `/tmp/lvns3_extract/out/` (522 ファイル + LFG/P16/KNJ/DAT、SSD マウント由来)
- macOS は `shuf` 未導入、サブサンプリングは `awk 'BEGIN{srand(42)} rand() < 0.1'` で代替
- `cargo build --release` でビルド済み: `train_decision_tree`, `lf2_decision_tree_debug`, `lf2_decision_tree_bench`, `lf2_first_diff` 等
- kako-rog (192.168.0.115, ROG WSL2 Arch) へ転送する選択肢もあるが、層 1 を直さないと数倍速くなるだけで本質的解決にならない

## 検討した代替案と判断

| 案 | 判断 |
|---|---|
| kako-rog に CSV 送って学習 | × 層 1 のアルゴリズム素朴さは数倍速になっても根本的解決ではない |
| `find_best_split` インデックス掃引リファクタ | △ 層 1 だけ直しても層 2 の特徴量不足で精度は伸びない |
| 65,536 クラスを (pos ビン化, len) などに圧縮 | △ 表現の問題で本質ではない |
| **特徴量 CSV を作り直す（候補上位 N 個の (距離, 長さ) を列に追加）** | ◎ 層 2 を直接攻める、これが本筋 |
| ML より先にオリジナル選択ルールを目視観察 | ◎ そもそも何を学習させたいかが固まる |

## 次セッションでやること（優先順）

1. **`/tmp/lvns3_extract/out/` の 1-2 ファイルで「オリジナルがどんな選択ルールを使っているか」を目視観察する** — `lf2_first_diff` 単一ファイルモードで token-by-token に「正解」と「最近距離候補」の差を見る。仮説 (longest match / smallest distance / lazy match / cost-based) のどれに近いか感触を取る
2. **CSV 設計を書き直す案**:
   - 候補リスト全体を出すと行数爆発するので、**上位 N 候補の (距離, 長さ) を固定列**にする (例: top3_pos1/len1, top3_pos2/len2, top3_pos3/len3)
   - または候補を「距離順 top-3」「長さ順 top-3」の 2 軸で取って 6 個の (pos, len) を列に並べる
   - `run_full_dataset` 関数 (`src/bin/lf2_first_diff.rs:703-`) を改造する
3. 新 CSV で `train_decision_tree` の `features = vec![...]` を更新して 1M サブセット × 深さ 12 で再走 → 精度がどこまで伸びるか確認
4. それで 90%+ 出るなら層 1 (アルゴリズム改善) に進む価値あり。出なければさらに特徴量を考え直す
5. もし CART 単体で頭打ちが見えたら、Random Forest や勾配ブースティングではなく「**ルールベースのハードコードに戻す**」可能性も考慮（Issue #5 の目的は ROM パッチに使えるルール、複雑モデルは目的に合わない）

## 触ったファイル

- なし（コード変更なし）。`/tmp/` に CSV と学習出力、`models/` に bin が出ただけ
- `/Users/kako-jun/repos/2025/retro-decode/models/` は git untracked のまま（`.gitignore` か削除かは保留）

## 振り返り

- **時間効率は悪かった**。CSV 完成確認に 25 分、学習 3 回で計 60 分以上、結局判明したのは「設計やり直し」。ただし**最終的に「特徴量が不足」という最重要情報が固まった**ので、次セッションの方針は明確
- フリーザ的反省: 「学習を回せばわかる」と短絡せず、最初に「CSV に何が乗っているか」「学習コードが何を見ているか」を読むべきだった。10 分で見抜けた話を 1 時間半かけた
- 次は「目視観察 → 仮説 → CSV 再設計」の順で、ML を回す前に骨格を固める
