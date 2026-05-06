# セッション 363 — retro-decode v9 (リング周回 + 画像サイズ + 交互作用) は横這い + Windows 移管準備

## 日付
2026-05-06

## 成果サマリ

v9 binary `lf2_pairwise_dataset_v9.rs` (CSV 53→60 列) を実装、kako-rog WSL で v8 CSV → v9 CSV に**後処理変換**して 522 ファイル分を再生成 (8 分弱)、LightGBM BIG (3000 round) で学習・評価。

**結果: v8 と同等 (Top-1 +0.01%)、完全一致は 1→0 で逆に劣化**。

## v8 vs v9 比較

| 指標 | v8 | v9 | Δ |
|---|---|---|---|
| Top-1 平均 | 75.23% | 75.24% | +0.01% |
| 完全一致 | **1 (C1001)** | **0** | -1 |
| ≥99% | 7 | 6 | -1 |
| ≥95% | (記録なし) | 70 | - |
| ≥90% | (記録なし) | 104 | - |
| ≥80% | 186 | 185 | -1 |
| 最大 hit rate | 100% (C1001) | 99.32% (OC_07) | **100% 喪失** |

## v9 追加特徴量と feature importance ranking

token-level (5):
- `wrap_count` — top 17 圏外 (= 仮説 A 不発)
- `wrap_phase` — top 17 圏外
- `total_pixels` — top 17 圏外 (ファイル内ほぼ定数で当然)
- `colors_bucket` (categorical, 0-4) — top 17 圏外
- `aspect_ratio` — top 17 圏外

candidate-level (2):
- `wrap_x_dist` (= wrap_count × cand_dist) — **rank 11 (importance 223,500)**
- `wrap_x_mod_w` (= wrap_count × cand_dist_mod_w) — **rank 13 (importance 201,762)**

候補レベルの交互作用項 2 つは top 15 入りしたが、cand_dist (rank 3, 908k) と cand_dist_mod_w (rank 8, 348k) を希釈する形になり、**全体の決定境界は安定しなくなった**。C1001 が完全一致を失ったのはこの希釈の典型例。

## 教訓

1. **トークン内で定数の特徴量はノイズにしかならない**: total_pixels / aspect_ratio / colors_bucket はファイル単位で定数。LightGBM は per-token decision を学ぶので、これらはツリー分岐の早い段階で「無関係な分岐」を作るだけ
2. **交互作用項は既存特徴量を希釈する**: wrap_x_dist は cand_dist の情報を持つが、別ノードに分散させて全体精度に影響
3. **C1001 が 100% を失ったのは「ML 近似の境界がシャープすぎる」を裏付ける**: わずか 50 token で全数当てが特徴量分布のごく狭い均衡で成り立っていた

## 環境メモ

- v8 → v9 CSV 変換は **源 LF2 ファイル不要**で実行可能 (col_pos + row_pos × img_w で input_pos 完全復元)
- 源 LF2 ファイル `/tmp/lvns3_extract/out/*.LF2` は kako-rog の WSL2 再起動で既に消失
- 変換スクリプト: `~/work/lf2_ml/v8_to_v9_transform.py` (kako-rog)
- v9 CSV: `~/work/lf2_pairwise_csvs_v9/` (3.0GB)
- v9 model: `~/work/lf2_ml/models/v9_binary_big.lgb`
- 学習スクリプト: `~/work/lf2_ml/26a_v9_binary_big.py`、評価: `26b_v9_binary_big_eval.py`

## 次手戦略

session 320 の `next-session-strategy.md` で予告した「v9 で届かない場合」の方針転換ターン。3 候補から選択：

### α: ツリー蒸留 (rule extraction)
v8 model (現状最強) の決定パスを解析して if-then ルール化。確率的揺らぎ排除で per-token 100% を狙う。**最も "真のルール抽出" に近い**

### β: per-file finetune (D 路線)
OC 系列だけで追加学習し、過学習で OC 系列の if を学習。**「単一エンコーダ仕様」と矛盾する性質、慎重判断**

### γ: DP backtrack
各ファイルで Leaf 出力を生成可能な全 LZSS パース列挙、共通選択ロジックを集合演算で抽出。**計算量大、kako-rog 並列必須**

session 320 で kako-jun が示唆した 3 路線のうち、**α が次の本命**。v8 model からツリーパスを抽出し、score > 0.7 を生成する条件式を Rust エンコーダに直接埋め込む。

## 関連 commit

- `lf2_pairwise_dataset_v9.rs` (untracked) — 仮説 A/B/C 用 7 特徴量実装、CSV 60 列
- 学習・評価結果は git 外 (kako-rog 上のモデルとログのみ)

## 関連メモリ (freeza)

- `feedback_records_in_project_docs` (本記録の置き場ルール)
- v9 路線が打ち止めなため、`project_lf2_ai_route_status` を「v8 が最終」状態に更新する候補

## 環境系の追加メモ (本セッションで判明)

- **WSL2 (kako-rog Arch) は 522 CSV まとめ読み + Python dict 11M エントリで OOM 凍結**: sshd 含む全プロセス応答不能、`wsl --terminate Arch` で復帰必要。**Stage 0 / Phase A は WSL では走らせない方針**。
- **9p 経由の WSL→/mnt/c コピーも 3GB クラスで詰まる**: tar+gzip 単一ファイル経由でも sshd hung。
- **次セッション以降は Windows ホスト直 (16GB RAM) で実行**: uv venv + polars + lightgbm 既導入済 (C:\lf2_work\.venv)。CSV は USB / 別経路で移送するか kako-rog 物理再起動後に再試行する判断保留。
- 段階的: Phase A (selector grid 522 ファイル評価) → Phase B (深さ 1 ツリー) → ... を Windows 側で 1 ヶ月単位の自走として組む。1 日 1 回チェックの token 最小モード。
