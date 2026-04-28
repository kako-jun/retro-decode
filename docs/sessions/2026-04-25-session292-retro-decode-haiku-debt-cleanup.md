# セッション292 — retro-decode Phase 3 真因特定 + Haiku 残骸総整理

**日時**: 2026-04-25
**主要プロジェクト**: retro-decode（Issue #5 Phase 3 のバイナリ一致 0% 問題）
**結果**: 真因 4 件特定・全修正・Haiku 負債総整理完了。CSV 生成完了待ち

---

## やったこと

### 1. 安いテストでの前提検証（前セッション残置タスク）

セッション291 で「ペイロードがそもそも近いのか・遠いのか」が未検証のまま保留していた。`lf2_decision_tree_debug.rs` を拡張して並列ダンプ機能を追加し、5 ファイルで検証。

結論：
- ヘッダ 16 バイト一致、パディング 0x10-0x11 のみ差（decode 側は読まない飾り）
- **0x12 から 150 バイト連続一致**（全ファイルで同一 = 構造的）
- 圧縮ストリーム開始位置 `0x18 + color_count*3` までが一致
- **圧縮ストリームの token #0 から既に発散**（リエンコード版のフラグバイトが `0x1f` vs オリジナル `0x72`）
- ピクセル差は 0.19% (323/167040)

つまり「Phase 3 撤退の理由は薄い、決定木が token #0 から外している」。

### 2. 真因 4 件の特定

`lf2_decision_tree_debug` にトークン列比較機能（`decompress_to_tokens` 流用）を追加して、token-by-token で Leaf vs DecisionTree を並べたところ:

```
idx 0  Lit(0x30)              Lit(0x30)              一致
idx 1  Match{pos:0xfee,len:18} Lit(0x30)              発散
```

Leaf は ring 初期値の `0x20` 列を活用して **長さ 18 の自己オーバーラップマッチ**を取っているのに、DecisionTreeGuided は同じ場所でリテラルを選択。

これを起点に逆方向に追跡して **4 つの致命傷** を発見：

| # | バグ | 場所 | 影響 |
|---|---|---|---|
| 1 | `find_optimal_matches` が自己オーバーラップ非対応 | `lf2.rs:707` | len=18 の長いマッチを候補に入れられない |
| 2 | 学習・推論で候補集合が完全別物 | `lf2_first_diff.rs` ↔ `lf2.rs` | 決定木の出力インデックスが意味を失う |
| 3 | `train_decision_tree` の列インデックス全部 +1 ズレ | `train_decision_tree.rs:362-373` | 4 特徴量すべて間違った列を学習 |
| 4 | 学習データセットが test fixture 3 ファイルのみ | `/tmp/full_dataset.csv` | LVNS3 本物の 522 ファイルから学習できていなかった |

#3 が特に重大。`image_x`(=fields[4]) と書いていたが実際は `max_candidate_len` の列。`length` 特徴量と書いていたが実際は `candidate_1_distance`（1〜4096 の値）の列。決定木の `length <= 296.50` などの謎しきい値の正体だった。

### 3. 修正実施

- 推論側を `enumerate_match_candidates_with_writeback` に統一（学習側と同じ候補集合）
- `train_decision_tree` の列インデックスを修正 + 候補リストカラム読み捨てを廃止
- `lf2_first_diff --full-dataset` を全行メモリ保持から `BufWriter` ストリーミングへ（OOM 解消）
- CSV 列を「特徴量＋ラベル＋Leaf 選択メタデータ」固定 13 列に簡略化（旧形式は 1 行 100KB → 新形式 ~50B）
- 522 ファイルから正規 CSV 再生成（バックグラウンド実行中、現時点 350/522, 367MB, 完了見込み 30 分前後）

### 4. 100 rules ハードコード方式の廃止

監査の「100 rules ハードコードは Haiku 的『量で押し切り』、設計の妥当性を疑うべき」という指摘に対応。

- `select_best_candidate_with_rules` の 193 行 if-else 連鎖を削除
- 新モジュール `src/formats/toheart/decision_tree.rs` を作成
- `train_decision_tree` が学習結果を bincode で `models/lf2_decision_tree.bin` に保存
- 推論側は `OnceLock` で 1 度だけロードして TreeNode を走査
- 環境変数 `RETRO_DECODE_TREE_PATH` で上書き可能

### 5. Haiku 残骸の総整理

ユーザーから「軽微だろうと残留はよくない」「examples 残す意味あるの？」のプッシュバックを受け、徹底削除：

| 削除対象 | 規模 |
|---|---|
| 5 つの compress_lzss_* 戦略（Phase 3 試行錯誤の 5 経路） | ~400 行 |
| `apply_*_logic` / `calculate_*_score` / `DeterministicRules` 等 ML 風命名関数群 | ~250 行 |
| `find_optimal_matches` / `calculate_original_quality` / `find_lzss_match` / `OriginalAction` / `MatchCandidate(lf2.rs版)` | ~150 行 |
| `decompress_with_steps`（誰も呼ばない死蔵） | ~190 行 |
| `examples/` から Haiku 製 78 本（`final_213_perfect_assault` 等） | ~30,000 行 |
| `select_best_candidate_with_rules` 100 rules ハードコード | ~190 行 |

合計: 純減 **31,924 行**（lf2.rs 単体は 1700 行 → 425 行）

### 6. 環境問題の解消

- `[lib] crate-type = ["cdylib", "rlib"]` の同名 .dylib 衝突 → bin/test の linker error → `rlib` のみに修正
- `Cargo.toml` に `autoexamples = false` 追加（暗黙登録された 78 本を遮断）
- `cargo test --release` が 18 件全 pass（前セッションまでコンパイル不能）
- `cargo clippy --release --all-targets` 警告 0、エラー 0
- 監査指摘の clippy 警告（identical blocks 17 / always return zero 3 / has no effect 2 / unneeded return 87）はすべて消滅
- 残った `(0 * 4 + 2) * 4` 等の erasing_op も `pixel_idx(row, col)` 関数化で除去

---

## わかったこと

### 1. Haiku の負債は単点バグではなく「層をなしている」

最初に見つけた `length=0.0` ハードコードバグ（セッション291）を直しても 0% のままだった理由が判明：
- 候補集合ミスマッチ
- 列インデックスズレ
- 学習データ不足
- 100 rules 自体の設計不安定性

ひとつずつ取り出して測れば微小な影響、しかし重なると 0% を保証する組み合わせ。「単点修正で改善するはず」期待は禁物。

### 2. 「100% 訓練精度」は信頼してはいけないシグナル

train_decision_tree が「Training accuracy 100%」と言っていたのは、**サンプル 17,371 個（= 3 ファイル分）に対して 100 ノードの木を完全に当てはめた過学習の極み**。テストファイル数も特徴量の意味も両方破綻していたのに、訓練精度は 100%。

学んだ：訓練精度の前に「サンプル数とソース」「特徴量が意図した量を表しているか」「クラス分布」を見るべき。

### 3. Opus 切り替えは戦略見直しトリガーとして有効（再確認）

セッション291 で Haiku → Opus に切り替えた時点で「2.5〜3.5 倍のサイズ膨張は微調整で直る規模ではない」と冷静になれた。今セッション開始時にも同じ感覚で「測定の前に前提検証」「妥協ゼロ宣言」「Haiku 監査の指摘と現実の症状を対応付け」が機能した。

kako-jun の「100% だと言ってるだろ。二度と妥協するな」というプッシュバックも、フリーザ様としての軌道修正の機会だった（私が pixel-perfect で許容案を出した直後）。

### 4. 「examples の負債」は見えづらい

`examples/` が 80 本あって、そのうち 78 本が `final_213_perfect_assault` のような Haiku 試行錯誤残骸だった。`autoexamples = true` がデフォルトなので Cargo.toml を見ても気付かない。`cargo test` 時に静かにコンパイルされてエラーまで出していた。

副作用：grep ノイズ、IDE インデックス汚染、リポジトリ規模の誤認、新規参加者の混乱。

---

## 次セッションでやること

CSV 生成（バックグラウンド `bjbc2xb1g`）の完了を起点に：

1. **CSV 生成完了確認** (`/tmp/lvns3_full_dataset.csv`、見込み数百 MB)
2. **train_decision_tree 実行** → `models/lf2_decision_tree.bin` 生成
3. **C0A01.LF2 単体検証** → token #1 で Leaf と同じ `Match{pos:0xfee, len:18}` を選べているか
4. **`lf2_decision_tree_bench` で全 522 ファイル走査** → バイナリ一致率測定
5. もし 100% に届かなければ:
   - 学習データの distribution チェック
   - max_depth 制限・剪定・特徴量追加（distance, prev_token_kind 等）の検討
   - lf2_first_diff の単一ファイル詳細モードで失敗ケースを掘る

---

## 試したことの記録（レポート用）

| 試行 | 動機 | 結果 | 学び |
|------|------|------|------|
| 並列ダンプによる前提検証（先頭 150B 一致確認）| ペイロードが近いか遠いかを安く判定 | 「圧縮ストリーム冒頭から発散」が確定 | 安いテストを先にやることで方向性が固まる |
| token-by-token 比較ツール追加 | 決定木の選択を直視 | token #1 で len=18 vs len=3 で発散 | 候補集合の問題が確定 |
| `find_optimal_matches` を `enumerate_match_candidates_with_writeback` に置換 | 自己オーバーラップ対応の候補集合へ | pixel-perfect roundtrip 達成（0 diff）/ サイズ 3.13x→3.67x（マッチを選ぶようになって短いマッチ連発） | 可逆性は得たが圧縮率は別問題 |
| train_decision_tree 列インデックス +1 修正 | 特徴量がすべてズレていた | （CSV 完了後に効果測定）| 「100% 訓練精度」を信用しない |
| lf2_first_diff ストリーミング化 | 旧 CSV は 3 ファイルでも 2.1GB → 522 ファイルで OOM（92/522 で死亡）| 522 ファイルで CSV 生成中、367MB at 350/522 | データ生成側の OOM は致命傷 |
| 100 rules ハードコード削除 + bincode ロード化 | 「量で押し切り」設計を廃止 | clippy 警告 100+ 件が一掃、テスト復活 | 監査指摘と症状の対応が綺麗に取れた |
| Haiku 残骸 examples 78 本 git rm | 「軽微だろうと残留はよくない」 | 純減 31,924 行 | リポ規模の誤認解消 |

### 駄目だったこと

- セッション291 でデバッグツール骨組みを書いて測定を後回しにしたこと（先に並列ダンプを書けば真因 30 分で見えていた）
- pixel-perfect 達成時点で「許容案」を提案してしまったこと（kako-jun の「妥協するな」で軌道修正）
- `cargo build` だけ見て満足したこと（`--all-targets` で test target に重大エラー残留を見落としていた、kako-jun の「軽微だろうと残留はよくない」で再点検）

### 監査結果の現実検証（kako-jun より）

| 監査指摘 | セッション292 終了時 |
|---|---|
| clippy 違反 29 → 126 件（+100） | **0 件** |
| identical blocks 17 件（重大ロジックバグ兆候）| **0 件** |
| always return zero 3 件 | **0 件** |
| has no effect 2 件 | **0 件** |
| cargo test コンパイル不能（example "final_213_perfect_assault" 18 errors）| **解消、全 18 テスト pass** |
| 100 rules ハードコードは Haiku 的「量で押し切り」 | **削除、動的ロード方式へ** |

---

## 技術スナップショット

| 項目 | 数値 |
|------|------|
| retro-decode 純減行数 | 31,924 行 |
| lf2.rs 行数 | 1700 → 425 |
| examples ファイル数 | 80 → 2 |
| compress_lzss_* 戦略数 | 6 → 1（DecisionTreeGuided のみ）|
| clippy 警告/エラー | 100+/3 → 0/0 |
| cargo test | 18/18 pass |
| 522 ファイル CSV 生成（バックグラウンド進行中）| 350/522, 367MB |
| 旧 CSV（test fixture 3 ファイル）| 2.1GB（廃止）|

---

## サマリ

**セッション292: Phase 3 バイナリ一致 0% の真因 4 件特定・全修正・Haiku 残骸総整理完了。次セッションで初の正規ベンチが回せる**

セッション291 で「Phase 3 撤退すべきか」迷っていたところを、Opus で冷静に並列ダンプ → トークン比較 → 学習データの中身を順に見ていき、致命傷 4 件を 1 セッションで全特定。修正は単純な「候補集合統一・列インデックス修正・データ再生成・100 rules ハードコード廃止」で、概念的には素直な再設計。

副産物として Haiku 残骸 31,924 行の総整理が完了。clippy 警告 0、cargo test 全 pass、リポジトリ規模が正常な大きさになった。

CSV 生成完了次第、次セッションは「学習 → 単体検証 → 全 522 ファイル bench」で Phase 3 の真の能力が初めて測れる。
