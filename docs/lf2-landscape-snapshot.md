# LF2 retro-decode 戦況スナップショット (2026-04-26 時点)

旧 memory `project_lf2_landscape` から移行。dummy 戦線天井 224 確定後の状態を凍結。
セッション 304 以降は AI 路線 (LightGBM) に主戦場が移り、別軸での進捗を `ai-route-2026-04-27-v3-to-v8.md` に記録している。

## 数値

- byte-exact 一致: **215/522 (41.2%)**、`compress_okumura_no_dummy` が現状最良
- dummy 軸ハイブリッド天井: **224/522 (42.9%)**（16 変種の和集合で確定）
- 522 ファイル全体パス: `/tmp/lvns3_extract/out/` (tmpfs、再起動で消える)、SSD 元 `/Volumes/Extreme SSD/先人のお手本/Windows95版のTo Heart/LVNS3DAT/`
- リポジトリ: `/Users/kako-jun/repos/2025/retro-decode`

## 確定した固定パラメータ（変えると壊れる）

- THRESHOLD = 2 固定（Leaf は 3-長 Match を使う、min_len=4 で 0/522 全壊）
- tie 規則 = StrictGt 固定（短マッチでも先勝ち、AllowEq 動的化で 0/522 全壊）
- F = 18, N = 4096（奥村 LZSS 標準）

## 確定した観察事実

- Leaf token 0 = Literal が 518/522 (99.2%)、Match{F=18} が 3、Match{<F} が 1
- 「奥村だけ当たる 9 ファイル」: C0601, CLNO_07, H42, H44, S26N, V71, V80, V89, V91
  - うち先頭 F=18 同値の単色: C0601, CLNO_07, H42, V80, V91 (5/9)
  - 混在: H44, S26N, V71, V89 (4/9)
- 「no_dummy だけ当たる 53 ファイル」: H 系 (H34/51/53/55/60/61/62/63/72/80/83/90 等), S 系 (S01D/02D/03D/08D/25D/26X/42D/43D/48D 等), V 系 (V10/1F 等) 多数
- min_bytes_strict (サイズ判定方式) は「奥村だけ当たる 9 のうち 8」を取れるが「no_dummy が当たる 53 のうち 25」を失う = サイズ判定方式は半分正しい

## bench / inspect ツール

すべて `cargo run --release --bin <name> -- <args>`:

- `lf2_token_bench /tmp/lvns3_extract/out/` — 16 変種の identical 数 + 集合演算 + Leaf token0 ヒストグラム + CSV (`/tmp/lf2_bench.csv`、522 行 16 variant 列)
- `lf2_oku_vs_nodummy_inspect <file.LF2> [max_tokens]` — 奥村/no_dummy/Leaf side-by-side dump
- `lf2_first_div_inspect` — 早期発散ファイル詳細
- `lf2_token0_inspect` — token 0 限定分析
- `lf2_oku_feasibility` — 奥村枠 feasibility 検証 (セッション 316 で大幅改修、`docs/ai-route-2026-04-27-v3-to-v8.md` 参照)

## 関連 commit (2026-04-26)

- retro-decode `783c94a`: 0→215 達成 (no_dummy)
- retro-decode `d719818`: dummy 戦線天井 224 確定 + 4 変種追加
- retro-decode `8619855`: サイズ判定方式 198/522 + uniform_head/min_bytes_strict + inspect ツール

## 次の主戦場

セッション 304 以降は AI 路線 (LightGBM) で別軸の進捗。詳細:
→ `docs/ai-route-2026-04-27-v3-to-v8.md`

セッション 320 末で完全一致 1/522 (C1001) を達成し、本ドキュメントの「215」とは別の数値系を辿っている。
