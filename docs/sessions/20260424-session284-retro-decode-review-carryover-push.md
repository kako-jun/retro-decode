---
session: 284
date: 2026-04-24
topic: retro-decode PR #11 レビュー継続差分を push して再開点を固定
---

# セッション284 — retro-decode 中断点の固定

## やったこと

- `retro-decode` の `4-first-diff-debugger` ブランチで、PR #11 のレビュー指摘に対する途中修正をコミット
- コミット `053f9d6` (`fix: kako-jun/retro-decode#4 first-diff レビュー差分を継続反映`) を `origin/4-first-diff-debugger` へ push
- `Issue #4` に引き継ぎコメントを追加して、今回入れた差分と未了項目を保存
  - コメント: https://github.com/kako-jun/retro-decode/issues/4#issuecomment-4312039545

## ブランチに入った差分

- `lf2_first_diff` で `token_match` と `byte_match` を分離
- `171 token-perfect / 165 byte-perfect` を観測できるように変更
- candidate 列挙を write-back 対応にして、自己参照 run-length 系を候補集合へ含めるよう修正
- `is_tail_overrun` を追加し、単体表示と CSV/summary で見えるように変更
- `decompress_to_tokens().ring_input` と既存 `decompress_lzss` の整合テストを追加

## 今の数値

- Total files: 522
- Token-perfect: 171
- Byte-perfect: 165
- Divergent: 351
- Leaf choice in candidate set (all divergent): 250 (71.2%)
- Leaf choice in candidate set (match tokens only): 250 / 302 (82.8%)
- Leaf match not in candidate set: 52 (14.8%)
- Leaf literal while Okumura matched: 49 (14.0%)
- Tail overrun cases: 52 (14.8%)

## 未了

- `docs/first-diff-analysis.md` の文面更新
- PR #11 の残レビュー反映と最終整形

## 次回やること

1. `4-first-diff-debugger` を pull
2. `docs/first-diff-analysis.md` を今回の実測値に合わせて更新
3. PR #11 をレビュー観点で再確認して push
4. 収束したらマージして #5 へ進む
