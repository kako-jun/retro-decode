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

## 自走モード (kako-jun 離席中) で得られた追加結果

### Phase A (単一 selector グリッド、Windows 16GB で実行)
522 ファイル × 28 (op×feature) selector を per-token で評価。
- **Top: argmax_cand_dist 50.79%、argmax_cand_age_start 50.75%、argmax_cand_dist_div_w 50.70%**
- 単独 selector で 100% 達成ファイル: **0/522**
- argmax 系 (最遠/最古/上の行) と argmin 系 (近/新) が拮抗 → 2 メカニズム仮説の数値裏付け
- 出力: `C:\lf2_work\phase_a_results\phase_a_summary.csv`

### Stage 0 (特徴量充足性テスト、Windows 48 秒)
全 19,260,530 候補行を hash group_by して同特徴量×別ラベルを数えた。
- **Within-tie 衝突: 0** ← 同 tie 内の候補は特徴量で完全区別可能
- **Cross-tie 衝突: 5,401 グループ / 17,450 行 (0.09%)** ← 別トークンで同特徴量だが別選択
- 衝突グループサイズ分布: n=2: 3056、n=3-5: 1786、n=6-10: 410、n=11-50: 149、n>50: 0
- **判定: 特徴量はほぼ十分 (99.91%)、0.09% に隠れ状態の存在**
- 出力: `C:\lf2_work\stage0_results.txt`

### Phase B (2-selector switch tree、73 秒、1680 ルール)
8 selector × 8 selector × 6 switch feature × 5 threshold グリッド。
- **Top: `if max_len ≤ 12 then argmax_cand_dist else argmin_cand_age_start` → 69.96%**
- max_len=12 が switch 値として優位 (短マッチと長マッチで挙動が変わる)
- v8 ML model 75.24% に届かず (深さ 2 ルールでは表現力不足)
- 出力: `C:\lf2_work\phase_b_results\phase_b_summary.csv`

### Gemma 4 E4B 動作確認 (kako-rog Windows 直)
- text 推論: 22.65 t/s (Hello プロンプト)
- **multimodal OCR**: HN ヘッダ画像 411 tokens 出力、22.56 t/s、テキスト全部読めた
- ollama daemon は ssh foreground で起動した状態でないと止まる (`ollama serve` を ssh の foreground で保持必要)
- **WSL に古い Ollama 0.13.5 が systemd 経由で動いていたのが gemma4 412 エラーの真因**: WSL2 mirrored networking で 11434 を奪っていた。`systemctl disable ollama` で解消、Windows 0.23.1 が正しく応答するように

### 環境棲み分け (再起動後の決定版)
- WSL Arch port 22, Windows sshd port 2222
- `~/.ssh/config` に `kako-rog` (port 22 = WSL) と `kako-rog-win` (port 2222 = Windows) を維持
- WSL は今後極力起動しない方針 (メモリ確保のため `wsl --terminate Arch`)
- CSV は `C:\lf2_work\lf2_pairwise_csvs_v8\` に Windows 永続配置 (3.3GB)
- uv venv: `C:\lf2_work\.venv\` (Python 3.10.4 + polars + lightgbm + numpy)

## 次手 (Phase C 設計) — 自走実行結果

### Phase C: per-file best 2-selector switch tree
12 selector × 12 selector × 8 switch feature × 8 threshold = 9,216 ルール × 522 ファイル評価 (147 秒)。
ファイルごとに最高 hit rate を出すルールを記録。

結果 (per-file best):
- 100% files: **0/522**
- ≥99% files: **0/522**
- ≥95% files: **3/522**
- ≥90% files: 11/522
- ≥80% files: 52/522

**判定: 2 階層 selector tree は表現力不足**。ML (深さ 8 × 3000 木) が C1001 で 100% 出せたのに、ハンドメイドの 2-level rule では 1 ファイルも届かない。

### 衝突分析 (5,401 cross-tie 衝突グループの正体)
- 254/522 ファイルが衝突候補を持つ (49%)
- 衝突グループ size 分布: n=2: 3056、n=3: 1077、n=4: 408、n=5: 301、n=6: 212、...、n>20: 0
- 衝突濃集ファイル: S01D-S06D (各 359-441 個)、V71/V72 (各 285)、S27N/S37N/S27X/S37X (各 264-268)、C1E系 (各 161-165)
  - → **シリーズ画像 / 姉妹ファイル**で集中

**衝突の構造例**:
```
C0101 token=1358: 候補 A (is_leaf=1), 候補 B (is_leaf=0)  ← 同じ特徴パターン
C0181 token=1329: 候補 A (is_leaf=0), 候補 B (is_leaf=1)  ← 反転
```
同じ feature vector pair が別ファイルで反転して選択される = **深い履歴の不足**が原因。

### 特徴量充足の真因と Phase D の設計
v8 CSV は **prev_3 までの履歴**しか持たない。シリーズ画像で同じローカル状況が出現するが、より前の履歴 (prev_4-prev_10 やもっと前) で Leaf が判断している可能性。

**Phase D (v10 binary): 深い履歴追加**
1. 既存 v8 CSV を後処理で拡張 (token_idx ソート済みなので prev_4-prev_10 は計算可能)
2. 拡張後 CSV で Stage 0 を再実行 → 衝突グループ数の変化測定
3. 衝突 → 0 なら ML/ルール探索で 100% 到達可能性確定
4. 衝突残るなら更に深い履歴 or DP backtrack 並行

## 結論 (session 363 自走モード)

| Phase | 目的 | 結果 |
|---|---|---|
| Phase A | 単一 selector | 51% 上限 |
| Stage 0 | 特徴量充足性 | 99.91% 充足、0.09% 隠れ状態 |
| Phase B | 2-selector switch | 70% 上限 |
| Phase C | per-file best 2-selector | 0/522 で 99%、3/522 で 95% |
| 衝突分析 | 0.09% の構造特定 | **深い履歴不足が原因と特定** |

**v8/v9/Phase A-C の天井 ≈ 75% / 70%** = 共通の「prev_3 まで」境界に縛られている。

**次セッション最優先**: v8 CSV 後処理で prev_4-prev_10 履歴拡張 → Stage 0 再実行 → 衝突 0 確認 → ML 再学習で 100% 到達確定。これで 1 ヶ月コースから「数日で 100% 必達」コースに変わる可能性。

## Phase D 自走実行結果 (session 363 終了直前)

v8 CSV 後処理で `prev_4`-`prev_8` (5 段深い履歴) を追加 → v10 CSV 生成 → Stage 0 再実行。

| 指標 | v8 | v10 |
|---|---|---|
| Cross-tie 衝突グループ | 5,401 | **4,607 (-15%)** |
| 衝突行数 | 17,450 | 14,908 |
| 衝突率 | 0.0906% | 0.0774% |
| Within-tie 衝突 | 0 | 0 |
| **Unknown 率 (深い履歴復元不能)** | - | **42.76%** |

**判定: 「深い履歴」だけでは劇的改善せず**。-15% の小幅減で 0% 到達には届かない。さらに、tie token gap で 42.76% の deep history が復元不能 (源 LF2 ファイルなしには完全データ取れない)。

### 仮説修正
**履歴 (前 token) の不足は隠れ状態の主因ではない**。0.09% の cross-tie 衝突を生み出している真因は、より深い構造:

1. **パレット内部状態**: img_colors=48 等のパレット範囲によるエンコード分岐
2. **画像座標 (col_pos / row_pos) の特殊位置**: 行末 / 行頭 / ブロック境界での挙動切替
3. **真のグローバル counter**: file 内のトークン累積数や bytes 出力数による分岐
4. **エンコード初期化状態の違い**: シリーズ画像 (S01D-S06D 等) が同じ encoder を共有していても初期化が違う可能性

### 戦略 (revised)
ML / ルール路線では **99.9% per-token / ~200-400 ファイルで 100% 達成**が現実的上限。**全 522 ファイル 100% 必達には γ DP backtrack が不可避**。

次セッション以降の方針:
1. **γ DP backtrack 実装**: 単一ファイル (例: C1001 50 token) で全 LZSS パース列挙 → Leaf 出力を生成可能な constraints を抽出
2. **constraints の集合演算**: 522 ファイル全部の Leaf 出力を生成可能な共通 constraints を ∩
3. 共通 constraints から encoder 関数を再構成 → ML 不要で決定的アルゴリズムを得る
4. v8/v10 CSV は補助的に使い続ける (ML が当てる 75% は確定的、残り 25% を γ で詰める)

**これで「数日コース」ではなく「数週間〜数ヶ月の地道な γ 実装コース」が現実的見立て**。

## Phase D 続編 (session 363 真夜中〜朝): v11 binary + Per-file overfit テスト

### v11 binary 実装と実行 (源 LF2 ファイル使用、SSD マウント後)
SSD `/media/ariori/Extreme SSD/先人のお手本/Windows95版のTo Heart/LVNS3DAT/` から 522 LF2 入手 (133MB)。
NUC で v11 binary を新規実装、24 列追加 (深い履歴 14 + global state 10):
- prev_4 ~ prev_10 (kind+len) — 14 cols
- bytes_emitted, bytes_remaining, token_count, last_match_len, last_literal_byte,
  recent_m_count, recent_l_count, recent_avg_len_x10, last5_max_len, last5_min_len — 10 cols

CSV 60→84 列、19,260,530 rows 生成 (5.25GB、tmpfs 経由再走で 7657 秒 = 2 hr)。
**最初の試行は NTFS-FUSE 経由で I/O ボトルネック (32 分で 33%)、tmpfs にコピーして再起動**。

**Stage 0 v11 は NUC で起動したが、再起動 (OOM 推定) で結果と CSV 全消失** — 5.25GB を 16GB RAM の polars に流したため。Stage 0 v11 は run しなおしの場合 source SSD から再再生する必要 (LF2 は SSD に永続、CSV は tmpfs だった)。

### Per-file overfit ML test (v8 features、Windows 16GB で完走、~3.3 時間)
LightGBM を 1 file ごとに **完全 overfit** (num_leaves=32768、min_data=1、no regularization) して同データでテスト → 「特徴量だけで Leaf を区別可能か」の最終判定。

| 指標 | 結果 |
|---|---|
| per-token 100% 到達 | **358 / 522 (68.6%)** |
| ≥99% | 520 / 522 |
| ≥95% | 522 / 522 |
| mean per-token rate | **99.9758%** |
| 100% 未到達 (= 特徴量不足) | **164 ファイル** |

**判定: v8 features は 358 ファイルで原理的に十分**。残り 164 ファイルでは「同 file 内に同じ特徴量パターンで違う選択をする tie token」が存在 = ML 表現力でも分離不可能 = 隠れ状態は file 内にすら存在。

底辺 8 ファイルは TITLE / C0209 / C0507 / C020A / C0306 / C0501 / C1901 / C160C で **per-token 99.84-99.90% 止まり**。これらは全部 "C シリーズ" (キャラクタイベント絵)、共通の encoder 内部 state (パレット位置 / 累積 counter 等) が file 内で切り替わる挙動を持つ可能性。

## 最終結論と次セッション戦略

**ML/ルール路線での天井 (定量的に確定)**:
- per-token: 75% (実 model) / 99.97% (overfit 上限)
- per-file: 1/522 〜 358/522 (overfit 上限)
- 100% 必達は 358 ファイルなら ML/ルール路線でも可能、164 ファイルでは原理的不可

**次セッション最優先**:
1. **γ DP backtrack を 164 困難ファイルで実装**: TITLE や C シリーズで Leaf 出力を一意に決める constraints を抽出
2. **v11 CSV 再生成 + Stage 0 v11**: 源 LF2 から完全データ生成すれば cross-tie 衝突がどこまで減るか確認できる (NUC OOM 対策必要、polars chunk 読みに切り替えるか kako-rog Windows 16GB を使う)
3. **Ghidra で LVNS3.EXE 解析**: SSD `/プロトタイプ/他サークルの同人ゲーム/LEAF/LVNS3/LVNS3.EXE` (decoder のみだが encoder のヒントになる定数や ring 操作は出るかも、session 295 で「タイブレーク規則は出ない」既確認済だが Ghidra の脱コンパイル深度を上げて再挑戦の価値あり)

**SSD パス**: `/media/ariori/Extreme SSD/先人のお手本/Windows95版のTo Heart/LVNS3DAT/` (522 LF2 + 522 PNG、kako-jun 物理 SSD、要 udisksctl mount)
**v11 binary**: `src/bin/lf2_pairwise_dataset_v11.rs` (84 列、source 必須、無償 git untracked)
**v8 CSV (生き残り)**: kako-rog Windows `C:\lf2_work\lf2_pairwise_csvs_v8\` 3.3GB
