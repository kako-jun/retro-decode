# セッション 320 — retro-decode AI 路線 v3→v8

## 日付
2026-04-27 21:30 ~ 2026-04-28 03:00 (約 5.5h)

## 成果サマリ

retro-decode の LF2 byte-exact エンコーダ復元プロジェクト (Issue #2) で AI 路線 (LightGBM) を v3 から v8 まで 6 バージョン進化、**byte-exact 完全一致 0 → 1 達成 (C1001)**。AUC 0.853→0.8574 (+0.0044)、≥99% 帯 5→7 ファイル。

最大の発見: **Leaf エンコーダは画像縦方向アラインメント (dist が img_w の倍数) を強く優先する**。v8 で `cand_dist mod img_w` を特徴量化し AUC +0.0019 (過去最大伸び)。

## 詳細記録 (本セッションの正本)

→ **`repos/2025/retro-decode/docs/ai-route-2026-04-27-v3-to-v8.md`**

進化テーブル、各バージョン詳細、奥村純粋 greedy 枠の天井確定、CWEEK_02 唯一 miss の解剖、限界考察、次手戦略、ML 環境メモはすべてここに集約。

## メタな決定 (本セッション中の運用ルール変更)

「**プロジェクト固有の記録は当該リポの docs/ に集約、freeza/notes/memory はポインタのみ**」を採用。

memory: `feedback_records_in_project_docs` に永続化済み。

## 次回やること (フリーザ視点)

retro-decode 100% 完全解明への戦略選択:
- **D 路線 (per-file finetune)** で OC 系列 99% 壁突破を試す
- **ツリー蒸留** で LightGBM の決定パスを if-then ルール化
- **DP backtrack + ML 誘導** で 100% 一致パスを探索

詳細・現状判断は retro-decode/docs/ai-route-2026-04-27-v3-to-v8.md 参照。
