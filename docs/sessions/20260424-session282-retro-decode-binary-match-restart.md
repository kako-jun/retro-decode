# セッション282: retro-decode バイナリ一致プロジェクト再起動

## やったこと

**retro-decode を塩漬けから復活**。kako-jun から「Leaf/Key のアルゴリズムは解けているがバイナリ一致できていない、そもそも可能なのか」相談。

- 診断: これは「SEED」問題ではなく LZSS の**決定規則同定**問題。LZSS は決定的プログラムで PRNG は絡まない → 理論上 100% 再現可能。NN 75.3% で詰まっていたのは道具違い（決定規則は「少数の if で 100%」なので決定木/ルール帰納が正解）
- 攻め筋を4本整理（奥村 lzss.c 忠実移植 / first-diff 候補列挙 / 決定木ルール帰納 / lfview 精読 / 実行バイナリ逆アセンブル）、Key/PDT は LF2 決着後に同じ手法を流用する順序で設計
- Issue 8本起票: **[#2 Epic]** LF2 バイナリ一致、[#3]奥村移植、[#4]first-diff デバッガ、[#5]決定木帰納、[#6]lfview 精読、[#7]Ghidra、[#8]PDT Ver1、[#9]PDT Ver2

**[#3] 奥村 lzss.c 二分木版の忠実移植 → マージ済み（PR #10）**
- `src/formats/toheart/okumura_lzss.rs` 新規（329→369行、原典忠実）
- `Lf2Image::to_lf2_bytes_okumura` ラッパ追加（既存 compress_lzss_* は不変）
- `src/bin/lf2_okumura_bench.rs` で 522 LF2 ファイル走査
- LVNS3DAT.PAK を `/mnt/ext4/20260219/プロトタイプ/他サークルの同人ゲーム/LEAF/LVNS3/` から発掘→ `/tmp/lvns3_extract/out/` に展開、CLAUDE.md 記載の 522 件と一致。公式ゲーム内データで信頼できる
- 結果: **ペイロード完全一致 165/522 (31.61%)**。現行 `compress_lzss_*` は 0 件 → 奥村原典仮説を強く支持
- セルフレビューで must-1 発覚（ダミー InsertNode ループが `1..=(F-1)` で原典 `1..=F` より 1 個少ない）→ 修正 → 再計測 165/522（件数不変、diff バイト数わずかに改善）。一致数は変わらなかったが原典忠実性は v2 が正しい
- 外れ値 98 件は `C*`（CG系）に集中、平均サイズ 67KB（通常 32KB の2倍）→ 長いファイルほど末端まで同期ずれが伝播

**[#4] first-diff 候補列挙デバッガ → PR #11 で中断**
- `src/formats/toheart/lf2_tokens.rs`（`decompress_to_tokens`, `LeafToken`, `enumerate_match_candidates`）
- `src/bin/lf2_first_diff.rs`（単一ファイル詳細 + `--histogram` 集計）
- 主要数値: 522 中 171 件トークン列一致、351 件発散、うち **Leaf 選択が候補集合に含まれる 250/351 (71.2%)** → docs レビュー指摘で **match tokens only では 250/302 (82.8%)** と再定義必要
- 独立レビューで must 2件 + should 5件 + nit 5件 + question 2件。**修正は着手前にトークン切れで中断**

## わかったこと

- LVNS3DAT.PAK は「信用してよい公式ゲーム内データ」（kako-jun 明言）。LVNS3 はエンジン、DAT.PAK はそのエンジンで動く公式タイトル用データで Leaf 社内ツール出力
- retro-decode のリポで PAK 解凍が既に動く → 522 コーパスを手元で量産できる
- 奥村原典移植だけで 31.6% 完全一致。残り 357 件は**タイブレイク（82%）+ 末尾処理**で説明される割合が大半、純粋な逸脱は少数
- サブエージェント委譲は #3/#4 両方でうまく機能（実装 + 計測 + docs をワンショットで回せる）

## 次回やること

**最優先: PR #11 のレビュー指摘消化 → マージ**
- セッション中断メモは PR #11 コメント（`gh pr view 11 --repo kako-jun/retro-decode --comments`）に全指摘と修正方針・コミット分割案が詳細に残してある。再開時は `/impl retro-decode 4` で同ブランチに戻り、そのコメントをそのまま実装エージェントに渡せば消化可能
- 要点: must-1（171 vs 165 の定義統一、差分6件 C0601 他）、must-2（母数を match tokens only に統一して 82.8% 併記）、should-1（run-length 近接マッチの偽陰性解消）
- 再計測コマンド: `cargo run --release --bin lf2_first_diff -- --histogram /tmp/lvns3_extract/out/ > /tmp/first_diff_v2.csv 2>/tmp/first_diff_v2_summary.txt`
- `/tmp/lvns3_extract/out/` は tmpfs。PC 再起動後は PAK 再展開必要: `retro-decode --input /mnt/ext4/20260219/プロトタイプ/他サークルの同人ゲーム/LEAF/LVNS3/LVNS3DAT.PAK --output /tmp/lvns3_extract/out/`

**その次: #5 決定木ルール帰納**
- #4 の CSV を食わせて「候補集合内の Leaf 選択」を 100% 説明する最小ルールを帰納
- NN ではなく **sklearn CART を depth 無制限で train 100%** まで学習 → ルール抽出
- 成功すれば残 351 件中 82% を解決 → 完全一致率が 31.6% → 60%台へ跳ねる見込み

## メモ: 残作業の順序

1. PR #11 レビュー対応 → マージ
2. #5 決定木帰納（#4 CSV → 最小ルール）
3. #6 lfview 精読（並行可）
4. 残 101 件の literal vs match 逸脱 + 候補集合外ケース → #7 実行バイナリ逆アセ検討
5. LF2 バイナリ一致達成 → #8 PDT Ver1（RLE、ほぼ確実に一致）
6. #9 PDT Ver2（LF2 手法流用）
