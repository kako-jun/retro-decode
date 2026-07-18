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

- `search_trace(r, max_len)`: tie token 直前に呼ぶ **read-only** トレース。insert_node と逐語一致の比較経路 (KeyMode::Byte0 root key、index 1 からの cmp 計算、LeftFirst 反転規則) で木を辿り、一致長がちょうど max_len のノードを訪問順に `(pos, rank, depth)` で返す。rank 1 = 原典 insert_node が採用するノード。**ただしこれは max_len < F の場合**。max_len == F の tie では、insert_node が full-F 一致ノードを r で置換済みのため trace 時点で元 pos が木に不在となり、rank が構造的に 0 に縮退する。Stage 0/1 の集計では **max_len == F 群を分離して評価する**こと (rank 0 多発を仮説棄却と誤読しない)
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

- 単体テスト緑 (計 15 本、commit `9f75713` + `77ea752`): rank 1 == insert_node 採用ノード (全 4 モード)、Basic の advance が原典 Encode() と全過程で BST 一致、trace の read-only 性・冪等性、境界 (max_len 境界・入力長境界・ring wrap・literal only)・事故パターン (teacher forcing 違反・過長 emitted の debug 検出、DummyThenDrop の dummy 全消滅) など。加えて合成 600 byte の teacher-forcing 全過程で「trace pos 集合 ⊆ enumerate_match_candidates_with_writeback の max_len 候補集合」「全 token で sim.r == ring r」「BST 親子リンク整合・無循環」を assert。okumura モジュール 32/32（lib 全体 46/46）緑
- debug ビルドで `test_assets/generated` の LF2 3 本を実走: panic なし・全行 61 列・rank/depth に実分布 (rank 17 種、depth 34 種、不在 0/255 も出現)
- **源 LF2 522 本での本生成と Stage 0 (C1001 での trace 集合 == enumerate max_len 候補のうち木に在るもの assert) は未実施**。物理 SSD 接続待ち

## 判定フロー

1. 522 本で v12 dataset 生成 → cross-tie 衝突グループを再集計
2. **衝突 0**: rank 特徴量が Leaf のタイブレイクを完全決定 → 決定木抽出 → Rust encoder 化
3. **微減にとどまる**: BST 状態仮説を棄却 → 姉妹ファイル並走ダンプ (シリーズ文脈の直接観測) へ転進

## Stage 1: rank=0 の正体解明 (2026-07-17, Issue #14)

Stage 0 で leaf 採用候補の `bst_rank_basic==0` が 13.23% (527,909 行) 残った。仮説「rank=0 は木に不在ではなく、木に在るが探索経路外」を ground truth で検証するフェーズ。

### 追加 API (`OkumuraSim`, いずれも read-only)

- `tree_scan(r) -> Vec<(pos, match_len, depth)>`: `search_trace` と独立に、256 root の lson/rson を全走査して到達可能な**全ノード**を列挙する ground truth。各ノードについて coding position `r` の先読み key との一致長 (byte 0 から、上限 F) と root からの深さを返す。列挙順は root 昇順 × 各 root 内 in-order (左→自分→右)
- `search_path(r)` (private): insert_node と同一規則で root→NIL の探索経路を `(node, went_right)` で返す。`classify_off_path` の内部用
- `classify_off_path(r, pos) -> (code, diverge_depth)`: pos が探索経路外になった理由の分類。0=経路上 / 1=木に不在 / 2=root byte 不一致 / 3=分岐で探索は左・pos は右部分木 / 4=探索は右・pos は左部分木。`diverge_depth` は分岐ノードの深さ (root=0、code 0/1/2 は 255)

### lf2_stage1_rank0 (`src/bin/lf2_stage1_rank0.rs`)

v12 と同じ teacher-forcing (Basic sim のみ) で 522 本を回し、leaf 採用候補が rank0 になる tie token ごとに tree_scan で in-tree 率・一致長・深さ・経路外理由・選択基準 (min/max pos、in-order 端、write tick 最古/最新、min/max dist) の的中率を集計して stdout に出す。**max_len==F 群 (swap-with-r で旧ノードが構造的に木から消える既知縮退) と max_len<F 群を分離して報告する**。

```bash
# 全 522 本 (release 推奨、v12 と同程度の実行時間想定)
cargo run --release --bin lf2_stage1_rank0 -- .local_data/lvns3

# 動作確認用: ファイル数制限・per-event CSV 詳細出力
cargo run --bin lf2_stage1_rank0 -- .local_data/lvns3 --limit 8 --csv /tmp/stage1.csv
```

CSV 詳細の注意: `is_*` フラグ列は `leaf_in_s==1` の行でのみ意味を持ち、サマリの基準別的中率は **`s_size>=2` (弁別力のあるイベント) だけ**で集計している。

### 判定フローにおける位置づけ

上記「判定フロー」の 2 (衝突 0 → encoder 化) に進む前段。rank=1 (86.66%) はそのまま使えるが、rank=0 の 13.23% は「rank 特徴量が定義できない」領域なので、その正体を

- **max_len==F 縮退** (swap-with-r。tree_scan でも不在) → rank とは別の規則 (置換直前のノードの復元など) が必要
- **木に在るが経路外** (code 3/4) → 探索経路の拡張 (全走査 rank) で特徴量化できる

の 2 群に切り分け、encoder 化に必要な追加規則を決める。8 本スモークでは rank0 の 98.73% が max_len==F 縮退、残る max_len<F 群は 100% が「木に在り一致長も max_len」で仮説成立だった (本判定は 522 本フルランで行う)。

## Stage 3: 複合規則エンコーダの実装と負結果 (2026-07-17, Issue #14)

Stage 2 で確定した複合タイブレイク規則を自走エンコーダ `compress_okumura_rank1_minage`
(`src/formats/toheart/okumura_lzss.rs`) として実装し、522 本で byte-exact を実測した。

### 実装

- ベースは Basic (dummy 挿入・StrictGt)。基底 `compress_okumura_impl_hooked` に
  override フック (`MinAgeFullFHook`) を挿す構造で、hook 無しは従来 impl と同一挙動
- 出力 Match が `match_length == F` のときのみ、Leaf 実 ring の shadow 状態
  (ring + write_tick、v12 の `cand_age_start` と同一定義) から full-F 候補を列挙し、
  min-age (最も新しく書かれた) 候補へ `match_position` を差し替える
- **適用条件は Stage 2 の検証範囲に限定**: full-F 候補 2..=32 (v12 N_MAX_CAP)・
  min-age 一意・age != u32::MAX。範囲外 (未書込み 0x20 領域の縮退 tie、
  full-F 候補が最大 1,277 個・全候補 age=MAX) では insert_node の選択を維持する。
  無条件適用の初版はこの縮退 tie で Basic の正解を壊すバグがあった (修正済み)

### 結果 (負結果)

| 計測 | 本数 |
|---|---|
| rank1_minage byte-exact | **165/522 (31.61%)** |
| 素の Basic (payload 一致) | 165/522 |
| 既存 variant union (`lf2_variant_best_fit` 再計測) | 257/522 |
| union への上積み | **0 本** |

- 一致集合は Basic と**完全同一** (gained 0 / lost 0)。さらに全 522 本の
  トークン照合 (`--vsbasic`) で **Basic との相違 0 ファイル** — 適用条件下の
  min-age 選択は常に insert_node の選択と一致した
- 結論: **規則 1 (full-F min-age) は「Basic の insert_node 選択の記述」であり、
  Basic を超える修正力を持たない**。Stage 2 の的中率 100.00% は teacher forcing
  下で Basic 由来の選択を言い当てていただけだった

### first-divergence census (先頭 120 本、修正後)

| 分類 | 本数 | 意味 |
|---|---|---|
| TIE_SUBF | 94 | max_len<F tie で rank-1 ≠ Leaf (規則 2 の per-tie 0.27% ミスがファイル単位で複利) |
| LEAF_NOT_CAND | 17 | Leaf トークンが候補列挙に不在 (tail overrun / hopeless 系。例: C0102 は残 12 バイトで len13) |
| KIND_DIFF | 7 | Literal vs Match の種別相違 |
| LEN_DIFF | 1 | 同種 Match の長さ相違 |
| MATCH | 1 | 完全一致 |

次の攻略先は full-F 規則ではなく、(a) max_len<F tie の残り 0.27% の判別、
(b) tail overrun / hopeless 系、(c) Literal/Match 種別差。
「規則違反 0 = 228 本」は teacher forcing 前提の数字で、自走では tie 以外の
1 発 divergence で届かない構造だった。

### ツール

```bash
# 522 本 byte-exact 実測 (一致リストは .local_data/stage3_matched.txt)
cargo run --release --bin lf2_stage3_verify -- .local_data/lvns3 [--limit N] [--out matched.txt]

# 1 ファイルの最初の相違点と候補 age の詳細
cargo run --release --bin lf2_stage3_debug -- <FILE.LF2>

# first-divergence の分類 census (name,class,diff_idx,input_pos)
cargo run --release --bin lf2_stage3_debug -- --summary <DIR> [N]

# Basic とのトークン相違数 (override の実効測定)
cargo run --release --bin lf2_stage3_debug -- --vsbasic <DIR> [N]
```

## Stage 6: 単一イベント編集の総当たり逆算 (2026-07-17)

### 目的

Stage 4-5 で「Leaf と basic の差はタイ候補クラスタ内の祖先関係のみ・大域ルール
差し替え 13 種は全滅」まで確定した。Stage 6 は消去論法の次段として、リング 1 周窓
(4,114 tick ≈ 8,256 イベント) 内の BST イベント 1 つだけ挙動を変えた差分再生を
機械列挙し、違反 (rank1 ≠ Leaf 採用) を単一イベント編集で説明できるかを問う。

編集タイプ 4 種: skip_insert (挿入 1 tick 遅延) / skip_delete (削除 1 tick 遅延) /
del_promote_other (del-two で前任者でなく後継者を昇格) /
swap_leaf_attach (full-F 一致で置換せず葉挿入)。

### 使い方

```bash
# 既定: C0602 の初違反を対象に窓内全イベント × 全編集を総当たり
cargo run --release --bin lf2_stage6_event_edit -- \
  [--file .local_data/lvns3/C0602.LF2] [--ti 861] [--window 4114] \
  [--limit N] [--self-test] [--dump-ties PATH]
```

- `--ti`: 対象 tie の token index (省略時は自動検出した初違反)。違反でない tie も
  指定可 (編集で正常 tie を壊さないかの検証用途)
- `--window`: 窓幅 tick (既定 4,114 = リング 1 周)
- `--limit`: 総当たり対象イベント数の上限
- `--self-test`: スナップショット (Clone) 復元後の無編集再生がベースラインと
  完全一致することを実ファイルで検証
- `--dump-ties`: 全 tie の判定結果を CSV 出力 (groups.parquet との突合用)

**判定範囲の割り切り**: 各編集の合否判定は窓内 `[snap_ti, target.ti]` の tie に
限定している。出力の「broken=0」は**窓内**副作用ゼロの意味であり、窓外 (対象 ti
より後) の tie への影響は測っていない。因果イベント特定という目的には十分だが、
「その編集を恒久ルール化してよい」ことの証明ではない。

### C0602 実測結果 (全 224 違反への掃引、1 違反 ≈ 1.5 秒)

- 意味論検証: 本実装の違反集合は Stage 4 groups.parquet の 217 違反の厳密な
  上位集合 (224 = 217 + CSV 除外 7 件)、chosen 位置の不一致 0 件。
  ti=861 / tick=8120 は Python ハーネスと完全一致
- **初違反 ti=861: 単一編集解なし** (9,100 編集を総当たり)
- 全 224 違反: **解なし 150 / 解あり 74** (うち窓内副作用ゼロ解を持つ ti は 70)
- **139 解の 100% が del_promote_other**。skip_insert / skip_delete /
  swap_leaf_attach は 0 件 → 差の在り処は削除時の昇格規則 (del-two の
  前任者/後継者選択) 周辺に集中
- 違反 tick とイベント tick の距離: 中央値 1,620、窓内にほぼ一様分布
  (昇格選択で決まった祖先関係が長時間持続してから顕在化)

### 解釈と次手

解なし 150 (67%) が多数派 = 単一イベント編集では説明できない系統的ルール差。
ただし解ありが del_promote_other に 100% 集中したことから、Leaf は「常に前任者」
でも「常に後継者」でもなく**条件依存で昇格先を選ぶ規則** (または削除順序自体の
系統差) が最有力仮説。次手は解あり 74 件の del-two イベントの局所特徴
(p/q の位置関係・部分木形状・距離) からの共通条件抽出。
