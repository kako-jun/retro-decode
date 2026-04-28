---
session: 304
date: 2026-04-27
topic: retro-decode AI 路線 v3 大ジャンプ (≥99% 0→5)、単一ルール仮説を全 522 検証で否定、次手 C-1 確定
---

# Session 304 — retro-decode AI 路線 v3 binary BIG が大ジャンプ。kako-jun の単一ルール仮説を全 522 評価で実証否定、C-1 (ring 周辺拡大) に進路確定

## 入りの状態

- セッション 301 完了: AI 路線 v1+v2 完走、ランダム説否定、続行確定 (lambdarank 即試行を予約)
- 522 LF2 → pairwise CSV v2 (1.5GB)・v1 モデル鍵保存、kako-rog WSL2 環境継続稼働
- 課題: **lambdarank 試行 + v3 履歴特徴量 + アンサンブル + ≥95% 残数を本気で削る**

## やったこと

### 1. A. lambdarank 単独試行 (`10_lgbm_lambdarank.py`)

- v2 CSV (1.5GB) で `objective="lambdarank"`, `metric=ndcg`, `label_gain=[0,1]`、num_leaves 127, group= 設定
- best_iter=431/84 秒で完走
- **Test Group-wise Top-1: 70.49%** (v2 binary 68.29% から **+2.2pt**)
- **n_cands=2 Top-1: 84.66%** (v2 81.92% から +2.74pt)
- 522 評価: 完全一致 0、≥95% 1、≥90% **18 (v2 binary 22 から減)**、最大 95.37% (v2 95.53% からほぼ同等)
- **群内最適化は効くが上澄みは平準化**で頭打ち。判定: 単独では完全一致を生まない

### 2. B'. v2 binary + lambdarank アンサンブル (`13_eval_only.py`)

T1 で 11_ensemble.py を投入したが **OOM kill** (学習+評価3通りを同プロセスで回しメモリリーク)。  
分割対応で `13_eval_only.py` を作成、保存済みモデルで eval のみ走らせ完走:

| 指標 | A binary | B lambdarank | Ensemble |
|---|---|---|---|
| 完全一致 | 0 | 0 | 0 |
| ≥95% | 0 | 1 | 0 |
| ≥90% | 23 | 23 | **16 (低い)** |
| ≥80% | 40 | 62 | 46 |
| 最大 | 94.74% | 95.37% | 94.74% |

→ **rank-norm 平均アンサンブルは負け**: 上澄みを平準化、最大値も binary の天井に引っ張られた。**B' 不採用確定**

### 3. v3 Rust bin 実装 (`src/bin/lf2_pairwise_dataset_v3.rs`)

v2 + 履歴トークン特徴量 (prev_2_kind, prev_2_len, prev_3_kind, prev_3_len) を追加、CSV 23→27 列。Mac/rog 両方で release build、522 並列生成 6 分で 1.5GB。retro-decode commit 96270d8 として push。

### 4. v3 学習・評価 (`14a_v3_train.py`, `14b_v3_eval.py`)

- v3 binary (1500r): 完全一致 0、≥95% **3 (v2 0→3)**、≥80% **89 (v2 40→89, 2.2倍)**、最大 **96.01% (+1.27pt)**
- v3 lambdarank (406r): 伸びず、≥90% で 10 と低下
- v3 ensemble: また負け

履歴特徴量は binary 目的関数で活用される、lambdarank では効かず。判定: **v3 binary を更に強化する方向**

### 5. B-2. v3 binary BIG (`15a_v3_binary_big.py`) — **大ジャンプ**

`num_leaves 255 / 3000 round / min_data_in_leaf 100 / early_stop 100` で再学習:

- 学習時間 562 秒、AUC 0.853 で **3000 round 完走** (early stopping せず = まだ伸びる余地)
- **完全一致 0**
- **≥99% = 5 (初到達!)**
- **≥95% = 43 (v3 1500r 3→43、14倍)**
- **≥90% = 89 (v3 22→89、4倍)**
- ≥80% = 170 (v3 89→170、2倍)
- 平均 73.92%、**最大 99.4170% (OC_07: 3240/3259 hits、あと 19 token!)**

上位 10 ファイル:

| file | 一致率 | miss |
|---|---|---|
| OC_07 | 99.42% | 19 |
| CMON_05 | 99.20% | 5 |
| CWEEK_02 | **99.10%** | **1** |
| OC_01 | 99.09% | 29 |
| OC_16 | 99.02% | 32 |
| OC_17 | 98.95% | 34 |
| CLNO_00 | 98.77% | 5 |
| OC_14 | 98.77% | 40 |
| OC_15 | 98.17% | 58 |
| LEAF | 98.03% | 13 |

CWEEK_02 はあと **1 token** で完全一致圏。

### 6. E. 上位 miss 解剖 (`16_top_miss_diag.py`) — kako-jun の「単一ルール仮説」を誘発する重要発見

**OC 系列 5 ファイル横断で同じ token_idx で同じ miss**:

```
tk=2279: n_max=16 ml=10 prev=M4  p2=M18 → 全 5 file miss、rank=2、diff=0.000、gt_dist=627 vs pr_dist=628 (差 1)
tk=2451: n_max=20 ml=11 prev=M6  p2=M17 → 全 5 file miss、rank=1 tie、diff=0.000、gt_dist=2395 vs pr_dist=2396 (差 1)
tk=2525: n_max=22 ml=8  prev=M18 p2=M16 → 全 5 file miss、rank=2、diff=0.000、gt_dist=10 vs pr_dist=12 (差 2)
tk=2972: n_max=2  ml=10 prev=M3  p2=L1  → 全 5 file miss、rank=2、diff=0.020、gt_dist=314 vs pr_dist=628
```

特徴量で見えない情報が確実に存在することを実測で確認。kako-jun が「あと 1 token miss なら、それを 100% にできる解を全ファイルに適用したら全部 100% にならないか?」と仮説提起。検証へ。

### 7. F. 全 522 miss プロファイル集計 (`17_global_miss_profile.py`)

522 ファイルで model_v3_binary_big を実行、全 miss を CSV 化 (`/tmp/all_misses.csv`、119 万行):

- **total tied groups: 3,990,322**
- **total miss groups: 1,191,206 (29.85%)** = group-wise Top-1 70.15% と整合
- **diff 分布**:
  - |diff|<0.001 (完全同点): **13,898 (1.17%)** ← モデルから区別不能、特徴量に欠けがある証拠
  - |diff|<0.01: 8.68%、|diff|<0.05: 33%、|diff|<0.1: 55%
- **rank_gt 分布**:
  - rank=1 (tie 違い): 603
  - **rank=2: 647,861 (54.4%)** ← 過半数が「あと一段」
  - rank=3: 22.4 万、rank=4: 11.1 万、rank=5+: 約 17 万
- **GT vs PR の dist 関係 (全 miss)**:
  - **gt_dist < pr_dist (GT が近い): 73.2%** ← モデルは「遠い候補」に偏る
  - gt_dist > pr_dist (GT が遠い): 26.8%
  - gt_dist == pr_dist: 0
- **n_max=2 同点 miss (2,788 件)**: GT が遠い 67.6% / GT が近い 32.4% ← サブパターンで方向逆転

### 8. G. 単一ルール仮説の検証 (`18_rule_search.py`)

6 ルール候補で 522 全評価:

| ルール | full | ≥99% | ≥95% | ≥90% | avg |
|---|---|---|---|---|---|
| R0 baseline | 0 | 5 | 43 | 89 | 73.91% |
| R1 n_max=2 同点→遠い | 0 | 5 | 43 | 89 | 73.94% |
| R2 全同点→近い | 0 | 5 | 43 | 89 | 73.90% |
| R3 全同点→遠い | 0 | 5 | 43 | 89 | 73.93% |
| R4 \|diff\|<0.01→近い | 0 | 5 | **46** | 89 | 73.31% |
| R5 n_max=2 遠い+他近い | 0 | 5 | 43 | 89 | 73.94% |
| R6 \|diff\|<0.05→近い | 0 | 2 | 38 | 81 | 70.10% |

**結果: 全ルールで完全一致 0、改善は誤差レベル。kako-jun の単一ルール仮説は実証ベースで否定**

理由 (実測):
- 同点 miss は 1.17% (13,898 件) しかない → 全部解いても全 miss の 1.2% にしかならない
- rank=2 miss が 54.4% (64.8 万) で押し負け = 同点 tie ルールでは触れない
- 1 ファイル完全一致には全 miss 解明が必要、1 種類のルールでは多重ヒューリスティックを覆えない

## わかったこと (永続化価値あり)

- **v3 binary BIG (3000r/leaves255) が現状チャンピオン**: ≥99% 0→5、最大 99.42%、≥95% 43、≥80% 170。AUC まだ伸びてる
- **完全一致は依然 0**: 「徐々に上がる世界」だが残り 1% 弱が壁
- **kako-jun の単一ルール仮説は否定 (構造的)**: 13,898 同点 miss を全部解いても完全一致 0、rank=2 押し負け 54% が本丸
- **rank=2 比率 54% + GT が近い 73%** = モデルは方向感ある (rank2 は見えてる) が判別力不足、特徴量拡張で押し勝てる可能性
- **OC 系列ファイル横断共通 miss** (tk=2279, 2451, 2525, 2972) は ring 周辺の微差 (dist 1-2) で決まる → ring 視野拡大が直接効く可能性
- **E と F の解析の数字は次セッションでも再現可能**: model 保存済 (~/work/lf2_ml/models/v3_binary_big.lgb)、CSV `/tmp/all_misses.csv` (再起動で消える、必要なら再生成 22 分)

## 環境メモ (運用知見)

- **WSL2 Linger=no で nohup プロセスが ssh 切断時に systemd-logind に殺される**。長時間ジョブは `tmux` 内で走らせる必要、かつ `sudo loginctl enable-linger kako-jun` で linger 有効化が前提
- **WSL2 IP 変更時の再接続**: ~/.ssh/config の HostName と直接 IP 接続を切り替える運用、IP は再起動で 73 に戻る (今回 115 を経由)
- **WSL2 OOM 多発**: train+val+test 一括 collect + 学習 2 本 + 522 評価×3 通りを同プロセスで回すと 7.5GB 超過 → **学習と評価をスクリプト分割**、評価ループに `gc.collect()` 必須
- **/tmp は tmpfs、再起動で消える**: 1.5GB の CSV、522 LF2、models 全消失。LF2 原本は **`/Volumes/Extreme SSD/先人のお手本/Windows95版のTo Heart/LVNS3DAT/`** (522 個 LF2 展開済み、tar pipe over ssh で 32MB 転送 30 秒)
- **モデル/スクリプトは `~/work/lf2_ml/` (ext4 永続)** に置く、CSV は `/tmp/` で OK (再生成 6 分)
- **Python stdout は block buffer、flush しないと長時間 0 byte log**: `python3 -u` または `print(..., flush=True)` 必須

## kako-rog 永続資産 (~/work/lf2_ml/)

- スクリプト: 01〜18_*.py (18 本)
- モデル: `models/v2_binary.lgb` (13.8MB), `models/v2_lambdarank.lgb` (7.5MB), `models/v3_binary.lgb`, `models/v3_lambdarank.lgb`, **`models/v3_binary_big.lgb` (現チャンピオン)**
- pipeline shell: `run_v3_pipeline.sh`, `run_v3_big_pipeline.sh`

## 次セッションでやること

### 推奨第一手: C-1 (ring 周辺バイト範囲拡大、v4 bin)

**目的**: rank=2 miss 54% を押し勝つための情報補強。ファイル横断共通 miss が ring の微差 (dist 1-2) で決まる証拠あり。

**実装**:
- `src/bin/lf2_pairwise_dataset_v4.rs` 作成: v3 + ring の `r_bef[1..8]`, `r_aft[0..8]` を追加 (16 バイト)。CSV 27→39 列くらい
- 522 並列生成 (~6 分)
- 14a/14b と同じパターンで学習・評価
- BIG パラメータ (3000r, leaves 255) で再学習

**期待**:
- AUC 0.853 → 0.86+
- ≥99% 5 → 10+ 
- 完全一致 0 → 数件出るかどうか (ここが分岐点)

### 第二手 (C-1 後の選択肢)

- **C-2: ring 累積 hash** — 過去 N トークンの ring 状態を hash 化、変わり目を捉える
- **B-3: 5000 round + train 200 ファイル拡張** — シンプル底上げ、+0.5pt 期待
- **D: ファイル別ファインチューニング** — 上位 10 ファイル個別に追加学習

### 撤退条件 (kako-jun 方針「99% でも遠い世界ならやめる」)

- C-1 で完全一致 0 のままなら、C-2 へ
- C-2 でも 0 なら、AI 路線で 100% は **構造的に届かない世界**と確定 → 「公式エンコーダ最大圧縮 + AI guidance で部分的な完全一致だけ狙う」運用案に切り替えを kako-jun と協議

## やらないこと（提案禁止）

- 奥村 LZSS 枠内の小変更 (memory `feedback_okumura_lzss_dead_end`)
- ソース入手・人間コンタクト・Ghidra (memory `feedback_no_external_help`)
- 単一ルール後処理 (このセッションで実証否定済)
- アンサンブル平均 (このセッションで負け確定)
- ML 結果から「99% でリリース」の妥協提案 (memory `feedback_retro_decode_100pct`)

## 戦況の総括

| 項目 | 値 | 含意 |
|---|---|---|
| 既存決定論ルール (奥村 LZSS) | 224/522 | 過去の天井 |
| AI 路線 v1 | 0/522 完全一致 | 既存ルール未到達 |
| AI 路線 v2 | 0/522、≥90% 22 | 漸進前進 |
| AI 路線 v2 + lambdarank | 0/522、≥90% 23 | +2.2pt 平均、頭打ち |
| AI 路線 v3 (1500r) | 0/522、≥95% 3、≥90% 22 | 履歴特徴量で +1pt |
| **AI 路線 v3 binary BIG (3000r/leaves255)** | **0/522、≥99% 5、≥95% 43、最大 99.42%** | **大ジャンプ、しかし 100% 未達** |
| 単一ルール仮説 | **否定** (実証) | 多重ヒューリスティック確定 |
| 次手の手応え | **C-1 高、C-2 中** | rank=2 押し負け 54% を ring 拡大で潰せるか |

撤退ではなく続行確定。次セッションで C-1 (v4 bin) を実装。
