# RetroDecode — 1ピクセルずつ、過去を保存 (P⁴)

**やりとりは日本語が基本です**

## プロジェクト概要

**RetroDecode** は、日本のレトロビジュアルノベルの画像デコード処理を解析・可視化する教育ツールです。歴史的な暗号化技術をインタラクティブな段階的可視化で実演します。

**キャッチフレーズ**: "Pixel by pixel, past preserved" （1ピクセルずつ、過去を保存）  
**略称**: P⁴ (4つのP: Pixel, by, Pixel, Past, Preserved)

## 目的と法的遵守

- **教育目的**: レトロゲームで使用された暗号化技術の研究
- **歴史保存**: 古いメモリ最適化手法（リングバッファなど）の理解
- **インタラクティブ学習**: デコード過程の段階的可視化
- **法的遵守**: ユーザー所有のファイル処理のみ、著作権侵害なし
- **往復検証**: デコード→エンコード→比較による実装完全性確認

## 🚧 現在の実装状況（2025年7月3日更新）

### ✅ 実装済み機能
- **LF2デコード**: ToHeart LF2形式の読み込み・LZSS展開
- **PDTデコード**: Kanon PDT形式の読み込み
- **透過PNG出力**: アルファチャンネル対応PNG生成
- **マルチフォーマット出力**: BMP, PNG, RAW RGB/RGBA
- **CLI設計**: --input, --output, --format分離アーキテクチャ
- **バッチ処理**: --input-dir での一括処理
- **ベンチマーク**: --benchmark での構造化出力
- **テスト生成**: 合成テストファイル自動生成
- **往復テスト基盤**: roundtrip_test.rs実装済み
- **LF2エンコード**: 実装済み（100%ピクセル精度達成）
  - 往復テスト完全成功：64,525ピクセル全て一致
  - 現在は全ダイレクトモード（マッチング無効化）
  - 圧縮率：237.7%（オリジナル33.4%に対し低効率）
- **技術知見記録**: 体系的技術文書作成完了（4ファイルに分離）

### ❌ 未実装機能（重要）
- **PDTエンコード**: RGB画像 → PDT形式書き込み
- **写真からの変換**: 大きな写真 → LF2/PDT変換
- **LF2圧縮最適化**: マッチング機能で3.4〜5.8倍効率化

## 🎯 直近のTODO

### 最優先課題
1. **LF2圧縮最適化**: マッチング機能実装で3.4〜5.8倍効率化
2. **PDTエンコード実装**: PNG→PDT変換機能
3. **写真変換機能**: フリー写真からテスト用LF2/PDT生成

### 今週の目標
- LF2エンコード圧縮率改善（237.7% → 100%以下）
- 往復テスト完全成功維持
- PDTエンコード基盤実装開始

## 🚀 アーキテクチャ概要

### 技術スタック
- **コア処理**: Rust（主要）、Python、TypeScript
- **GUI**: Tauri + Web技術（計画中）
- **CLI**: Rust（単体バイナリ配布）
- **可視化**: HTML5 Canvas、WebGL（計画中）

### サポート対象ゲーム形式
- **ToHeartシリーズ**: PAK ✅, LF2 ✅, SCN ⚠️
- **Kanonシリーズ**: PDT ✅, G00 ⚠️
- **痕（Kizuato）**: ToHeartと同形式（確認中）

## 📋 ツール利用情報

### 主要コマンド
```bash
# デコード（拡張子で自動判定）
retro-decode --input file.lf2 --output result.png
retro-decode --input file.pdt --output result.png

# 往復テスト
cargo test roundtrip

# ベンチマーク
retro-decode --benchmark --input file.lf2
```

### 開発ツール
- **Rust**: `cargo build`, `cargo test`
- **Git**: 進捗追跡とコミット
- **エディタ**: VSCode/Claude Codeで開発

## 📚 技術文書体系

### 文書構成
1. **`.claude/CLAUDE.md`** (本ファイル) - プロジェクト管理・進捗追跡・ツール情報
2. **[TECHNICAL_INSIGHTS.md](.claude/TECHNICAL_INSIGHTS.md)** - 設計思想・実装パターン・アーキテクチャ
3. **[LF2_COMPRESSION_ANALYSIS.md](.claude/LF2_COMPRESSION_ANALYSIS.md)** - LF2圧縮解析・性能比較データ
4. **[PDT_KNOWLEDGE.md](.claude/PDT_KNOWLEDGE.md)** - PDT形式解析・実装知見
5. **[PROJECT_ROADMAP.md](.claude/PROJECT_ROADMAP.md)** - 長期実装計画・ディレクトリ整理計画
6. **[REFERENCES.md](.claude/REFERENCES.md)** - 参考資料・既存ツール・ゲームデータ情報
7. **[DEVELOPMENT_GUIDE.md](.claude/DEVELOPMENT_GUIDE.md)** - 葉鍵ベンチ・テスト戦略・開発ノート

### 文書活用方針
- **開発作業時**: 本ファイルで現在の状況とTODO確認
- **技術実装時**: TECHNICAL_INSIGHTSで設計パターン参照
- **LF2最適化時**: LF2_COMPRESSION_ANALYSISで目標値確認
- **PDT実装時**: PDT_KNOWLEDGEで仕様詳細確認

## 📈 進捗追跡

### 最新マイルストーン
- **✅ 2025年7月2日完了**: LF2エンコード機能実装完了
- **✅ 2025年7月3日完了**: 技術文書体系整理完了
- **🎯 2025年7月9日目標**: LF2圧縮最適化（マッチング機能実装）
- **2025年7月16日目標**: PDTエンコード機能実装完了

---

**P⁴ — 1ピクセルずつ、過去を保存**

*限られたハードウェアでビジュアルストーリーテリングを実現した巧妙な圧縮技術を探求*