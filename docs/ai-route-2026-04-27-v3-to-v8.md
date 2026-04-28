# AI 路線 v3 → v8 進化記録 (2026-04-27 ~ 28)

## 概要

LF2 byte-exact エンコーダ復元プロジェクト (Issue #2) の AI 路線で、LightGBM ベースの選択モデルを v3 から v8 まで 6 バージョン進化させた。**byte-exact 完全一致 0 → 1 達成 (C1001)**、AUC 0.853 → 0.8574 (+0.0044)、≥99% 帯 5→7 ファイル、≥80% 帯 170→186 ファイル。

## v3-v8 進化テーブル

| Version | 追加特徴量 | CSV列 | AUC | Top-1 | 完全一致 | ≥99% | ≥95% | ≥80% | 最大 | commit |
|---|---|---|---|---|---|---|---|---|---|---|
| v3 | (基準) prev_2/3 履歴 | 27 | 0.853 | 73.92% | 0 | 5 | 43 | 170 | 99.42% | (前) |
| v4 | ring 周辺 16 byte | 39 | 0.8536 | 74.53% | 0 | 4 | 59 | 178 | 99.42% | `084598d` |
| v5 | lookahead p3..8 | 45 | 0.8542 | 74.71% | **1** | 5 | 69 | 178 | **100%** | `88e4066` |
| v6 | cand_age 2 col | 47 | 0.8547 | 74.69% | 1 | 6 | 69 | 177 | 100% | `f90f2a3` |
| v7 | cand_ow + col/row_pos | 51 | 0.8555 | 75.14% | 1 | 6 | 67 | 183 | 100% | `d01497d` |
| v8 | cand_dist mod img_w | 53 | **0.8574** | 75.23% | 1 | **7** | - | **186** | 100% | `4ac12af` |

各バージョン詳細は対応コミットメッセージにフル数値+特徴量重要度。

## 重要な発見

### 1. 奥村純粋 greedy 枠の真の天井 (v3 前段、commit `d140740`)

3 つの診断 binary を追加し、threshold × init_match × lazy の 8 組合せ全測定:

| threshold | init_match | lazy | feasible/522 |
|---|---|---|---|
| 2 | true | false | 0 |
| 2 | false | false | 13 (2.49%) |
| 3 | false | false | **14 (2.68%)** |

**純粋 greedy 枠の天井は 14/522**。memory の「224 byte-match」は BST/dummy 軸操作の偶然合致で、純粋 greedy では原理的に到達不能と確定。

### 2. cand_age と cand_dist の冗長性 (v6 後解析)

OC_07 の 23 miss を解析すると、**全件で `cand_age_start == cand_dist`** が成立。リング上書きが頻発しないファイルでは age と dist が同値になり、独立した識別力にならない。BST 仮説 ("新しく挿入されたノード優先") の信号は実在するが、決定打にならない理由はこれ。

### 3. **画像縦方向アラインメント — 最大の発見** (v7→v8)

CWEEK_02 の唯一の miss (token 327) を完全解剖したところ:
- 候補 A (model 選択): pos=1825, **dist=97**
- 候補 B (Leaf 選択): pos=1745, **dist=177**
- **177 - 97 = 80 = img_w!** → **真上の行の同列 (1 row up, same col)**
- in_byte = 1,1,1,1,1 (背景ピクセル run)

つまり Leaf は **`cand_dist` が `img_w` の倍数になるマッチ (= 真上の行同列) を強く優先する**。これは LZSS 一般の挙動ではなく、画像エンコーダ特有の最適化。

v8 で `cand_dist_mod_w` (mod), `cand_dist_div_w` (div) を直接特徴量化 → AUC +0.0019 (過去最大の伸び)、importance rank 6/7 に新規上位入り、cand_dist の重要度は 1.20M→904k と低下 (置換効果)。

### 4. 100% 候補ファイル

セッション末時点で「あと 1 token」の候補:
- C1001 (50 tokens, 50/50 hit) — **byte-exact 100% 達成済み**
- CWEEK_02 (111 tokens, 110/111 hit, miss=token 327)
- CSNO_07 (205 tokens, 204/205 hit, v8 で新規 99.51%)

## 限界考察

### 数学的限界

- 各 token 精度 ~99.4%、AUC 平均伸び +0.001/version
- 3000+ token ファイル (OC 系列) で 100% 到達には **token 精度 99.99%** 必要
- 現方式では数十バージョン要する計算 → 別アプローチ必要

### 残存する隠れ状態

memory `project_lf2_ai_route_status` 記載「同一ファイル内 99.77% 決定論、ファイル間 99.77% 矛盾」が示唆するのは、**特徴量に捕捉されていない隠れ状態の存在**。Leaf エンコーダは決定論的なので、全変数を捕捉できれば原理的に 100% 一致可能。

未捕捉の候補:
- BST 内部状態 (parent/child/sibling 関係、cand_age では部分捕捉)
- 画像の他軸構造 (隣接スキャンライン参照、同 byte 値の 2D アラインメント)
- ファイル先頭からの絶対位置効果

## 次手戦略

memory `project_lf2_ai_route_status` で次手候補として明記:

1. **D 路線 (per-file finetune)**: 上位 10 ファイルに個別追加学習、過学習で 100% 押し込む
2. **ツリー蒸留**: LightGBM の決定パスを if-then ルールに変換、確率的揺らぎ排除
3. **DP backtrack + ML 誘導**: 全パース列挙 + ML スコアで Leaf 寄りパス選定

「単一の正解組み合わせ」が必ず存在する (PRNG なし、決定論的) ため、次手は ML 近似ではなく **真のルール抽出**を狙う方向に転換すべき。

## ML 環境メモ (kako-rog)

- v[5-8] CSV: `~/work/lf2_pairwise_csvs_v[5-8]/` (永続、各 2.6-3.1GB)
- ML スクリプト: `~/work/lf2_ml/19a-25b_*.py` (永続、git 外、計 14 本)
- モデル: `~/work/lf2_ml/models/v[4-8]_binary_big.lgb` (永続)
- 学習時間: 約 10 分/version、評価 約 22 分/version
- WSL2 tmpfs 3.8G は v3+v4 CSV で満杯になり書込み失敗 → /home に退避運用に変更

## 関連 commit (本セッション、新しい順)

- `4ac12af` v8 cand_dist mod img_w
- `d01497d` v7 cand_ow + col/row_pos
- `f90f2a3` v6 cand_age 2 col
- `88e4066` v5 lookahead p3..8 (完全一致初突破)
- `084598d` v4 ring 周辺 16 byte
- `d140740` 奥村 LZSS 枠 14/522 天井確定 (3 診断 binary)

## 関連 Issue

- #2 [Epic] LF2 バイナリ一致プロジェクト
- #5 (closed) 決定木によるルール帰納 (522 ファイル × 2.4M 決定点)

## 関連メモリ (freeza)

- `feedback_okumura_lzss_dead_end` (奥村 LZSS の限界、本記録のサマリ)
- `project_lf2_ai_route_status` (戦況スナップショット、本記録のサマリ)
- `reference_lf2_source` (LF2 522 ファイル原本場所)
- `reference_kako_rog_host` (kako-rog 環境)
