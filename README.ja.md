# RetroDecode — 1ピクセルずつ、過去を保存

<div align="center">

## P⁴ (Pixel by pixel, past preserved)

*クラシック日本ゲームの画像デコード過程を解析・可視化する教育ツール*

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/tauri-%2324C8DB.svg?style=for-the-badge&logo=tauri&logoColor=%23FFFFFF)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

[English](README.md) | [日本語](README.ja.md)

</div>

## 概要

RetroDecodeは、クラシック日本のビジュアルノベルで使用された歴史的な画像圧縮・暗号化技術を実演するインタラクティブな教育ツールです。デコード過程の段階的可視化を提供し、限られたハードウェアで使用された巧妙なメモリ最適化手法を理解できます。

**主要機能:**
- 🎮 **マルチフォーマット対応**: ToHeart (PAK/LF2/SCN)、Kanon (PDT/G00)、痕
- 🔍 **段階的可視化**: 画像が1ピクセルずつ再構築される様子を観察
- 🖥️ **クロスプラットフォーム**: Windows、macOS、Linux対応
- ⚡ **多言語エンジン**: Rust、Python、TypeScript実装
- 🎯 **教育重視**: リングバッファとレトロ圧縮技術を学習
- 🌙 **モダンダークUI**: 詳細解析のためのクリーンで技術的なインターフェース

## クイックスタート

### インストール

```bash
# リポジトリをクローン
git clone https://github.com/your-username/retro-decode.git
cd retro-decode

# プロジェクトをビルド
cargo build --release
```

### 基本的な使用方法

```bash
# ヘルプを表示（引数なしのデフォルト動作）
retro-decode

# 単一ファイルをデコード（拡張子から形式を自動判定）
retro-decode --input image.lf2

# PAKアーカイブを展開
retro-decode --input archive.pak --output ./extracted/

# GPU加速でPythonエンジンを使用
retro-decode --input file.pdt --lang python --gpu

# パフォーマンス比較のため並列処理を有効化
retro-decode --input data.g00 --parallel

# GUIインターフェースを起動
retro-decode --gui

# 教育的可視化のための段階的モード
retro-decode --input file.lf2 --step-by-step --verbose
```

## サポート形式

| ゲームシリーズ | アーカイブ | 画像形式 | 説明 |
|-------------|---------|-------------|-------------|
| **ToHeart** | `.pak/.PAK` | `.lf2/.LF2`, `.scn/.SCN` | アーカイブ展開 + 画像デコード |
| **Kanon** | - | `.pdt/.PDT`, `.g00/.G00` | 圧縮画像形式（2バージョン） |
| **痕（Kizuato）** | `.pak/.PAK` | `.lf2/.LF2` | ToHeartと同形式 |

*大文字小文字を区別しない拡張子判定*

## 教育機能

### インタラクティブ可視化
- **タイムラインスクラビング**: 動画編集のようなデコード段階ナビゲーション
- **バイナリエディタビュー**: リアルタイムhexダンプ表示
- **ピクセル単位プレビュー**: リアルタイム画像再構築の観察
- **メモリ状態可視化**: リングバッファと最適化技術
- **歴史的文脈**: 開発者の制約と解決策を学習

### パフォーマンス解析
- **シングル vs マルチスレッド**: `--parallel`で処理モード比較
- **エンジン比較**: Rust vs Python vs TypeScript実装のベンチマーク
- **GPU加速**: 利用可能な場合のCUDA/OpenCLサポート

## CLIリファレンス

### 必須引数
- `--input <file>`: 入力ファイルパス（`--gui`使用時以外は必須）

### オプション引数
- `--output <path>`: 出力ディレクトリ（デフォルト: `./output/`）
- `--lang <engine>`: 処理エンジン（`rust`|`python`|`typescript`、デフォルト: `rust`）
- `--gui`: Tauri GUIインターフェースを起動
- `--step-by-step`: 教育的段階実行モードを有効化
- `--parallel`: 並列処理を有効化
- `--gpu`: GPU加速を使用（利用可能な場合）
- `--verbose`: 詳細ログ出力
- `--help`: ヘルプ情報を表示

### 使用例

```bash
# 基本ファイル変換
retro-decode --input game.PDT --output ./images/

# 詳細出力付き教育モード
retro-decode --input archive.PAK --step-by-step --verbose

# パフォーマンス比較
retro-decode --input large.G00 --parallel --lang rust
retro-decode --input large.G00 --lang python --gpu

# フォーマット間実験
retro-decode --input toheart_image.lf2 --output ./converted/
retro-decode --input kanon_image.pdt --output ./converted/
```

## アーキテクチャ

```
retro-decode/
├── src/
│   ├── main.rs          # CLIエントリーポイント
│   ├── formats/         # フォーマット別デコーダー
│   │   ├── toheart/     # PAK、LF2、SCNサポート
│   │   └── kanon/       # PDT、G00サポート
│   ├── bridge/          # 多言語ブリッジ
│   └── gui/             # Tauri GUIコンポーネント
├── web/                 # フロントエンド（HTML/CSS/JS）
├── scripts/             # Python/TypeScript実装
└── examples/            # サンプルファイルと使用例
```

## 開発

### 前提条件
- Rust 1.70+
- Node.js 18+（Tauriフロントエンド用）
- Python 3.9+（オプション、Pythonエンジン用）
- TypeScript/Deno（オプション、TSエンジン用）

### ソースからビルド

```bash
# 開発ビルド
cargo build

# 最適化リリースビルド
cargo build --release

# テスト実行
cargo test

# Tauri GUIビルド
cargo tauri build
```

### クロスプラットフォーム注意事項
- **Windows**: Visual Studio Build Toolsが必要
- **macOS**: Xcode Command Line Toolsが必要
- **Linux**: build-essentialとwebkit2gtkが必要

## 法的・倫理的配慮

このプロジェクトは**教育目的専用**として設計されています：
- ✅ **ユーザー所有ファイル**: 法的に所有するファイルのみ処理
- ✅ **歴史保存**: レトロゲーム技術の理解
- ✅ **教育研究**: 圧縮・最適化技術の学習
- ❌ **海賊行為禁止**: 著作権コンテンツを配布しない
- ❌ **商業的害悪なし**: 歴史的フォーマットの研究ツール

## 貢献

貢献を歓迎します！貢献ガイドラインと行動規範をお読みください。

### 貢献分野
- 追加ゲーム形式サポート
- パフォーマンス最適化
- 教育コンテンツとドキュメント
- クロスプラットフォームテスト
- アクセシビリティ改善

## ライセンス

このプロジェクトはMITライセンスの下でライセンスされています - 詳細は[LICENSE](LICENSE)ファイルをご覧ください。

## 謝辞

- レトロゲームコミュニティによる元フォーマットドキュメントとリバースエンジニアリング
- ゲームアーカイブプロジェクトによる歴史保存努力
- コンピュータグラフィックスと圧縮アルゴリズム研究からの教育的インスピレーション

---

<div align="center">

**P⁴へようこそ — 1ピクセルずつ、過去を保存**

*限られたハードウェアでビジュアルストーリーテリングを実現した巧妙な圧縮技術を探求*

</div>