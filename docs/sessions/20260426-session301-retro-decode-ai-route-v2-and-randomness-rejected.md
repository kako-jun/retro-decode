---
session: 301
date: 2026-04-26
topic: retro-decode AI 路線 v1 + v2 学習・診断、ランダム説を実証で否定、伸びしろ確定
---

# Session 301 — kako-rog で AI 路線本格稼働。LightGBM v1/v2 を回し切り、Leaf エンコーダのランダム説を統計実証で否定

## 入りの状態

- セッション 299 完了: kako-rog 環境 (WSL2 Arch + Polars/LightGBM + RTX 3050) 整備完了、522 LF2 → pairwise CSV 19,260,530 行 / 827MB / `/tmp/lf2_pairwise_csvs/` 生成済み
- 課題: 真の天井 224/522 を超えるか、AI 路線 3 周目で実測する。19% 止まりだった過去 NN/決定木との差を出せるか
- kako-jun 方針: 「100% に近づく方法ならなんでもやる」「徐々に上げて近づくなら継続、99% でも遠い世界ならやめる」

## やったこと

### 1. Polars 行統計 (`01_row_stats.py`)

- 19M 行を Polars lazy で 1.2 秒スキャン、is_leaf=1 比率 20.72% (3.99M 行) を確認
- **n_max 別 leaf_pct は完全に 1/n_max と一致** (n_max=2→50%, n_max=3→33.33%, ..., n_max=32→3.12%) = 同点候補内で正解が均等分布、ベースライン (ランダム選択) 理論通り
- 単一特徴量の弁別力検証: `cand_len` leaf=0/1 平均 7.61/7.07・差わずか、`cand_pos` 中央値 2030/2025 ほぼ無、`prev_kind` L/M で leaf 比率 23.93/19.91 で 4pt 差、`cand_dist` mean 1771/2094 で leaf=1 が 18% 遠 (これだけ少しシグナル)。単軸では弁別困難、組み合わせに賭ける

### 2. LightGBM v1 学習 (`02_lgbm_subsample.py`)

- 522 ファイルを random.shuffle(seed=42) で train 100 / val 30 / test 50 に file 単位 split (ファイル間 leak 防止)
- 9 特徴量 (V1): ring_r, n_cands, n_max, max_len, cand_pos, cand_len, cand_dist, prev_len, prev_kind
- num_leaves 63, lr 0.05, num_boost_round 500, early_stop 30。245 秒で best_iter=500
- **Test AUC = 0.8463**、AP = 0.6133 (baseline 0.2083)、**Group-wise Top-1 = 67.92%** (264,232 / 386,930)
- n_cands 別: n=2 で 81.92% (vs random 50%)、n=3 で 72.6% (vs 33.33%)、n=10-21 で 42-55% (vs 5-10%) — 全帯域でランダムを大幅上回る
- Feature importance (gain): n_max 4M / cand_dist 2.1M / max_len 1.5M / cand_pos 31k (使われてない)。**cand_dist が群内識別の主役**

### 3. ファイル一致率実測 (`03_file_exact_rate.py`)

「Top-1 hit が全 token グループで 100% なら byte-exact 一致候補」のロジックで全 522 で測定:

- **完全一致 0 / 522**、≥99% 0、≥95% 0、≥90% **1 ファイル**、≥80% 28
- Top-1 hit 平均 69.07%、最大 91.23%
- 90% 以上が 1 ファイルだけ = AUC 0.95 に上げて Top-1 を 70%→85% にしても、~36000 token 全部正解する確率は 0.85^36000 ≈ 0
- **これだけ見れば「徐々に上げる世界ではない、断崖がある世界」に見えた** (この時点で撤退 or 戦略転換を検討)

### 4. kako-jun の問い → 決定論性検証 (`04_determinism_test.py` + `04b_determinism_streaming.py`)

kako-jun 提起: 「画像ごとにパラメータが違う / Leaf 自身もエンコードのたびにバイナリが違う、という可能性は?」

検証ロジック: 環境特徴量タプル (n_max, max_len, prev_kind, prev_len, ring_r) + 候補集合フィンガープリント (sorted (cand_pos, cand_len, cand_dist)) を「同一局面キー」とし、複数回出現するキーで選択が一意か測定。
- 当初 04 (一括 collect) は WSL2 7.5GB を食い潰し OOM、04b でファイル単位ストリーミング集計 + Python defaultdict に書き換え 36 秒で完走
- 結果: 全 unique key 3,148,933、2 回以上出現 414,352。**決定論キー (常に同じ選択) 95.31% / 矛盾キー 4.69%**
- トークンベース矛盾比率 4.49% (56,414 / 1,255,521)
- n_max=2 限定で 3.54%、n_max ↑ につれ徐々に増 (~6%)

### 5. ファイル内 vs ファイル間 矛盾分解 (`05_conflict_breakdown.py`)

矛盾キー 19,438 を「ファイル内矛盾」と「ファイル間矛盾」に分けた:

- **同一ファイル内矛盾: 44 / 19,438 (0.23%)** — 全部 `CBAK` 1 ファイル中、全部 prev=M18 の境界条件
- **ファイル間のみ矛盾: 19,394 / 19,438 (99.77%)**
- n_max=2 矛盾の選択比率分布: balanced (45-55%) が **66.94%**、moderate 31.53%、skewed 1.52%
- 自動判定スクリプトの結論: **「同一ファイル内矛盾が極小 = (c) ファイル単位パラメータ変動説 が最有力」**

### 6. ファイルクラスタリング検証 (`06_file_clustering.py`)

「ファイル別パラメータ」が 2-4 種類のスイッチなら明確なクラスタが出るはずを検証。n_max=2 の矛盾キー上位 1000 で各ファイルがどちらの候補を選ぶかを 0/1 行列化:

- 上位 1000 キーでカバーされたファイル 256/522、10 件以上カバー 79 ファイルのみ (サンプル薄い)
- 候補 B 選択率: 中間帯 (0.3-0.7) が **73.4%** = 二極化していない
- K-means k=2/3/4: クラスタサイズ [78,1] [77,1,1] [76,1,1,1] = 大半 1 クラスタに集約、外れ値が分離されただけ
- 完全シグネチャ一致: 79 ファイル中 **70 種類**、最大グループ 3 ファイル
- ペア間不一致率 中央値 37.5% — 連続スペクトル
- → 「単純 2-4 種パラメータスイッチ」は否定。**多次元 / 連続的 or 入力依存** に修正

### 7. v2 CSV 生成 (Rust 側 bin 改修)

`src/bin/lf2_pairwise_dataset_v2.rs` を新規作成 (Mac で実装、scp で rog 配置、両方で build)。v1 から追加した特徴量:

- file-level: `img_w` / `img_h` / `img_colors`
- token-level: `in_byte` (input[s])、`in_byte_p1`、`in_byte_p2`、`in_byte_after` (input[s+max_len])
- candidate-level (差別化のキモ): `r_bef1` (ring[cp-1])、`r_bef2`、`r_aft` (ring[cp+max_len])、`r_aft1`

CSV ヘッダ 23 列。スモークテストで C0A01 の token 2 を確認: 候補 1 の r_aft=32, 候補 2 の r_aft=48 で実際違う値が出ており差別化シグナルとして機能している。

rog で `xargs -P 16` 並列実行、522 ファイル → `/tmp/lf2_pairwise_csvs_v2/` に **1.5GB**生成。生成時間は v1 と同じく ~6 分。

### 8. LightGBM v2 学習 (`07_lgbm_v2.py`)

20 特徴量で num_leaves 127、num_boost_round 1000、early_stop 50。675 秒で best_iter=1000:

- **Test AUC = 0.8552** (v1: 0.8463、+0.009)
- **Group-wise Top-1 = 68.29%** (v1: 67.92%、+0.37pt)
- **完全一致 0/522** (v1 から変化なし)
- ≥95% ファイル: **1** (v1: 0)
- **≥90% ファイル: 22** (v1: 1、+21、22 倍改善)
- 最大ファイル一致率: **95.53%** (v1: 91.23%、+4.3pt)

Feature importance (gain top 15): n_max / cand_dist / max_len は v1 と同じく上位、新特徴量は **img_w 5位 (196k) / r_aft1 7位 (106k) / r_aft 8位 (96k) / in_byte 10位 (79k) / r_bef2 11位 (73k)** と中堅入り。新特徴量は確かに使われているが決定的ではない、AUC の伸びはわずか。

「徐々に上げる世界ではない」可能性が再浮上。撤退判断の手前で 1 つ最終診断を入れることを kako-jun に提案 → 承認。

### 9. CMON_05 詳細解剖 (`08_final_diagnosis.py`)

最大 95.53% を出した CMON_05 (627 tokens、miss 28) の miss を全部開く:

- score 差分布: diff < 0.01 (極僅差) 3, < 0.05 (僅差) 11, < 0.10 18, ≥ 0.30 (大差) 2、平均 **0.0985**
- 正解の順位: **rank 1: 0**、rank 2: **16**、rank 3: 5、rank 4: 3 — 大半が「2 位どまり」、モデルは方向感覚あり
- ファイル内 同一局面 12 件、当たり/外し混在 **0**、同じ局面で違う正解 **0** = **ファイル内では決定論的に常に同じ間違い方をしている**

判定スクリプトの結論: 「大差で外しているケースが目立つ → モデルが本質を捉えていない、別軸の特徴量で伸びる可能性」

### 10. 大ファイル 4 つ追加診断 (`09_large_file_diag.py`)

CMON_05 の性質が小ファイル特有 (627 tokens) でないかを大ファイル (3000+) で検証:

| file | hit% | tokens | miss | rank1 | rank2 | rank3+ | avg_diff | intra | choice_split |
|---|---|---|---|---|---|---|---|---|---|
| CMON_05 | 95.53% | 627 | 28 | 0 | 16 | 12 | 0.0985 | 0 | 0 |
| OC_08 | 92.25% | 3186 | 247 | 1 | 142 | 104 | 0.0832 | 0 | 0 |
| OC_01 | 92.16% | 3189 | 250 | 2 | 152 | 96 | 0.0845 | 0 | 0 |
| OC_14 | 92.16% | 3240 | 254 | 1 | 155 | 98 | 0.0907 | 0 | 0 |
| OC_15 | 92.06% | 3173 | 252 | 1 | 145 | 106 | 0.0865 | 0 | 0 |

**5 ファイル全部で intra=0 / choice_split=0** = サイズによらずファイル内完全決定論。ランダム説 (b) は実証ベースで明確に否定された。

## わかったこと

- **仮説 (b) Leaf 自身がエンコードのたびに違うバイナリ説は否定** — 同一ファイル内決定論性 99.77% (全体)、≥90% 達成 5 ファイルでは 100%
- **仮説 (c) ファイル別パラメータ説は方向性は正しい** — ファイル間矛盾 99.77%、しかし「単純 2-4 種スイッチ」ではなく **多次元 or 入力依存**
- **仮説 (a) 隠れた入力依存説 が依然有力** — 同じ環境特徴量タプルでも実バイト列や ring 累積状態が違えば違う選択、ファイル内では ring が同じ流れで進むので一貫、ファイル間では累積状態が違うので「同じ環境」でも別選択
- **AI 路線にはまだ伸びしろがある** — 推奨次手:
  - **A. 学習目的を `lambdarank` に変更** (CSV 不要、即試せる、30 分): binary classification ではなく群内順位最適化で Top-1 直接押し上げ
  - **B. 履歴トークン特徴量追加** (Rust 側 v3 bin、中規模): `prev_2_kind`, `prev_2_len`, `prev_3_*` 等
  - **C. ring 全体の状態特徴量** (実装大): ring の周辺数十バイト・累積パターン・過去 N トークンの位置統計
- **n_max=2 矛盾の 50:50 比率 67% は別解釈可能** — 「ランダム」ではなく「ring 累積状態が二分される」結果として整合する
- **memory `feedback_okumura_lzss_dead_end` は依然正しい** が、AI 路線 (CSV+ML) は別軸で伸びしろあり、両者を混同しないこと

## 環境メモ

- pairwise CSV (rog): v1 = `/tmp/lf2_pairwise_csvs/` (827MB), v2 = `/tmp/lf2_pairwise_csvs_v2/` (1.5GB)
- ML スクリプト (rog): `~/work/lf2_ml/01_row_stats.py` ... `~/work/lf2_ml/09_large_file_diag.py` (9 本)
- Python venv (rog): `~/work/ml-env-test/.venv/` (Python 3.12.13 + Polars 1.40.1 + LightGBM 4.6.0 + sklearn)
- v2 bin source (Mac/rog 両方): `src/bin/lf2_pairwise_dataset_v2.rs` (262 行、release build 済み)
- v2 bin ビルド時間: Mac 4.8 秒, rog 8.3 秒
- LightGBM v2 学習時間: 16 thread フル稼働で 675 秒 (best_iter=1000)
- 522 ファイル評価時間: 181 秒
- WSL2 メモリ食い潰し事故: 04 (Polars 一括 collect) で RSS 6.5GB / 7.5GB 、kill して 04b ストリーミング版に書き換え

## 次セッションでやること

1. **A. lambdarank 学習を即試行** — CSV 不要、30 分で結果。`objective="lambdarank"` + `group=` で群内順位最適化に切り替え。AUC 同等でも Top-1 が押し上がる可能性
2. **A 結果を見て次手判定**: AUC 0.86+ で完全一致が出れば即勝ち筋、出なければ B (履歴トークン特徴量) に進む
3. **B. v3 bin 実装** (必要なら): `prev_2_kind`, `prev_2_len`, `prev_3_*`, `prev_4_*` を追加。CSV 列 ~28 まで増えるが 522 並列再生成は 6 分
4. **C は B でも届かなければ実装検討**: ring 累積状態の hash や周辺バイトベクトル化

## やらないこと（提案禁止）

- 奥村 LZSS 枠内の小変更 (memory `feedback_okumura_lzss_dead_end` で確定済、AI 路線とは別軸)
- ソース入手・人間コンタクト・Ghidra (memory `feedback_no_external_help`)
- WSL2 メモリ拡張 (kako-jun 指示「ゲーミング用途で Windows 側を削らない」)
- ML 結果から「99% でリリース」の妥協提案 (memory `feedback_retro_decode_100pct`)

## 戦況の総括

| 項目 | 値 | 含意 |
|---|---|---|
| 既存決定論ルール (奥村 LZSS 枠内) | 224/522 一致 | これが現在の天井、AI 路線で超えるのが課題 |
| AI 路線 v1 完全一致 | 0/522 | LightGBM 単独では既存ルールの精密さに届かず |
| AI 路線 v2 完全一致 | 0/522 | 特徴量豊富化で AUC +0.009、≥90% ファイルは 1→22 で前進 |
| ランダム説 | **否定** | ファイル内決定論性 99.77%、押し上げ可能 |
| 次手の手応え | **あり** | rank 2 大半 + avg diff 0.09 = 押し上げ可能距離 |

撤退ではなく続行確定。次セッションで A (lambdarank) を即試行。
