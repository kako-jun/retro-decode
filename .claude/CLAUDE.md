# RetroDecode — 1ピクセルずつ、過去を保存 (P⁴)

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

## 🚧 現在の実装状況（2025年7月2日更新）

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

### ✅ 実装済み機能（続き）
- **LF2エンコード**: 実装済み（100%ピクセル精度達成）
  - 往復テスト完全成功：64,525ピクセル全て一致
  - 現在は全ダイレクトモード（マッチング無効化）
  - 圧縮率：237.7%（オリジナル33.4%に対し低効率）
- **技術知見記録**: TECHNICAL_INSIGHTS.md開始

### ❌ 未実装機能（重要）
- **PDTエンコード**: RGB画像 → PDT形式書き込み
- **写真からの変換**: 大きな写真 → LF2/PDT変換

### 🏗️ ディレクトリ整理計画

現在のディレクトリが乱立している問題を解決：

```
retro-decode/
├── src/                    # 🔄 現状維持
├── .claude/               # 🔄 現状維持  
├── docs/                  # 📝 新規：ドキュメント統合
│   ├── README.md         # メイン
│   ├── README.ja.md      # 日本語
│   ├── ARCHITECTURE.md   # アーキテクチャ
│   └── FORMATS.md        # サポート形式
├── tools/                 # 🔄 整理：開発ツール統合
│   ├── examples/         # 🔄 移動：examples/ から
│   ├── benches/          # 🔄 移動：benches/ から  
│   ├── scripts/          # 🔄 移動：scripts/ から
│   └── generators/       # 📝 新規：テストファイル生成
├── assets/               # 🔄 整理：テスト・リソース統合
│   ├── test/            # 🔄 移動：test_assets/ から
│   │   ├── generated/   # 合成ファイル（コミット可能）
│   │   ├── lf2/         # 著作物LF2（ローカルのみ）
│   │   ├── pdt/         # 著作物PDT（ローカルのみ）
│   │   └── photos/      # 📝 新規：変換用写真素材
│   └── reference/       # 📝 新規：参照実装・仕様
├── tests/                # 🔄 現状維持
├── web/                  # 🔄 現状維持
└── target/               # 🔄 現状維持
```

## 🎯 優先実装計画

### Phase 1: LF2エンコード最適化 (完了→改善)
**✅ 完了した作業**:
- 往復テスト結果: 100%ピクセル精度達成（64,525ピクセル全一致）
- LZSS圧縮基本機能正常動作確認
- フラグビット処理とY-flip座標変換修正完了

**📊 発見された最適化機会**:
- 現在：全ダイレクトモード（圧縮率237.7%）
- オリジナル：高度マッチング（圧縮率33.4%、3.4〜5.8倍高効率）
- マッチング機能実装により大幅な圧縮改善可能

**優先度**: 🟡 高（機能完成→性能改善）
**期間**: 2-3日
**理由**: 基本機能完成、圧縮効率向上が次の課題

### Phase 2: 往復テスト実装
```bash
# 既存著作物での往復テスト
retro-decode --roundtrip original.lf2 --output-decoded temp.png --output-encoded test.lf2
# → 元ファイルとtest.lf2をバイナリ比較

# 写真からの変換テスト
retro-decode --encode photo.jpg --output photo.lf2 --format lf2 --colors 256
retro-decode --decode photo.lf2 --output reconstructed.png
# → photo.jpg と reconstructed.png を視覚比較
```

**優先度**: 🟡 高
**期間**: 1週間  
**依存**: Phase 1完了後

### Phase 3: 写真変換機能
```bash
# フリー写真からテスト用LF2/PDT生成
retro-decode --convert-photo assets/photos/landscape.jpg --output assets/test/generated/landscape.lf2 --colors 256 --dithering floyd-steinberg
retro-decode --convert-photo assets/photos/portrait.jpg --output assets/test/generated/portrait.pdt --size 640x480
```

**優先度**: 🟡 高
**期間**: 1週間
**理由**: 著作権フリーなテスト用ファイル大量生成

### Phase 4: ディレクトリ整理とツール改善
- ディレクトリ構造移行スクリプト
- CI/CD更新
- ドキュメント統合

**優先度**: 🔵 中
**期間**: 2-3日

## 🧪 テスト戦略の進化

### 現在の問題
- 著作物ファイルはローカルのみ（CI/CDで使用不可）
- 合成テストファイルは小さすぎる（4x4, 16x16）
- 実際のゲーム画像との比較ができない

### 解決策
1. **往復テスト**: 既存著作物でバイナリ一致確認
2. **写真変換**: 大きなフリー写真からLF2/PDT生成
3. **段階的検証**: 
   - パレット生成精度
   - LZSS圧縮効率
   - 透過処理正確性
   - 色減少品質

## 📊 現在のテスト資産状況

### ✅ 利用可能（コミット済み）
```
assets/test/generated/
├── transparency_4x4.png      # 透過テスト
├── pattern_16x16.png         # パターンテスト  
├── palette_boundary.png      # 境界条件テスト
└── max_palette_8x8.png       # 最大パレットテスト
```

### ❌ 不足しているもの
- 大きな画像（512x512以上）のテストケース
- 実際のゲーム画像相当の複雑度
- 往復テスト用の基準ファイル
- パフォーマンステスト用の大容量ファイル

## 🚀 アーキテクチャ

### 多言語コア設計

```
retro-decode/
├── src/
│   ├── main.rs           # CLI エントリーポイント
│   ├── lib.rs            # ライブラリルート
│   ├── formats/          # フォーマット別エンコーダー・デコーダー
│   │   ├── mod.rs
│   │   ├── toheart/      # ToHeart形式（PAK、LF2、SCN）
│   │   │   ├── lf2.rs    # ✅ デコード ❌ エンコード
│   │   │   ├── pak.rs    # ✅ 展開のみ
│   │   │   └── scn.rs    # ⚠️ 未完成
│   │   └── kanon/        # Kanon形式（PDT、G00）
│   │       ├── pdt.rs    # ✅ デコード ❌ エンコード
│   │       └── g00.rs    # ⚠️ 未完成
│   ├── bridge/           # 言語間連携
│   └── gui/              # Tauri GUI コード
├── tools/                # 📝 整理後
│   ├── examples/         # 使用例
│   ├── benches/          # ベンチマーク
│   ├── scripts/          # Python/TypeScript実装
│   └── generators/       # テストファイル生成
└── assets/               # 📝 整理後
    ├── test/             # テスト用ファイル
    └── reference/        # 参照資料
```

### 技術スタック

- **コア処理**: Rust（主要）、Python、TypeScript
- **GUI**: Tauri + Web技術
- **CLI**: Rust（単体バイナリ配布）
- **可視化**: HTML5 Canvas、WebGL
- **クロスプラットフォーム**: デスクトップアプリ + Web版

## サポート対象ゲーム形式

### ToHeartシリーズ
- **PAK**: アーカイブ展開 ✅
- **LF2**: 画像デコード ✅ エンコード ❌
- **SCN**: シーンデータデコード ⚠️

### Kanonシリーズ（2バージョン）
- **PDT**: 画像形式 デコード ✅ エンコード ❌
- **G00**: 圧縮画像形式 ⚠️

### 痕（Kizuato）
- ToHeartと同形式（確認できれば統合）

## 主要機能

### 1. 段階的可視化
- **タイムラインスライダー**: 動画編集のようなデコード段階ナビゲーション
- **再生/一時停止**: 段階の自動進行
- **バイナリエディタビュー**: リアルタイムhex表示
- **ピクセルプレビュー**: ライブピクセル単位再構築
- **メモリ状態**: リングバッファと最適化の可視化

### 2. マルチフォーマットサポート
```bash
# CLI使用例（拡張子で自動判定、大文字小文字区別なし）
retro-decode --input game.PDT --output results --format png
retro-decode --input image.LF2 --output results --format bmp --parallel
retro-decode --input archive.PAK --output ./extracted/
retro-decode --gui  # Tauriインターフェース起動
retro-decode        # ヘルプ表示（引数なし）

# 往復テスト（実装予定）
retro-decode --roundtrip original.lf2
retro-decode --encode photo.jpg --output test.lf2 --colors 256

# 完全なオプション例
retro-decode --input file.g00 --output ./result/ --step-by-step --verbose
```

### 3. フォーマット間実験
- **再圧縮**: フォーマット間でのサイズ最適化テスト
- **フォーマット変換**: ToHeart ↔ Kanon形式実験
- **カスタム画像**: ユーザーの写真/画像をレトロ圧縮

### 4. 教育インターフェース
- **ダークテーマ**: 技術コンテンツ向けモダン美学
- **色分け可視化**: 異なる圧縮段階
- **歴史的文脈**: 開発者の意図とメモリ制約の説明
- **インタラクティブ凡例**: ホバー/クリック説明

## 参考資料

`~/repos/42/2025/_ref/`内に配置:

### 既存デコーダー
- `leafpak`, `lfview-1.1a`: ToHeartツール
- `p2b_000s`: Kanonツール
- `egg_cli/src/apps/to_heart.ts`: 以前のTypeScript実装
- `egg_cli/src/apps/kanon.ts`: 以前のTypeScript実装

### ゲームデータ
- `Windows95版の痕/`
- `Windows95版のTo Heart/`
- `Windows95？版のKanon/`
- `Windows2000？版のKanon/`

### 解析リソース
- `解析.xlsx`: フォーマット解析ドキュメント
- `compress/`: 再圧縮テストファイル
- `dump/`: 以前の解析からのランタイムダンプ

## 実装計画

### フェーズ1: コアインフラ
- [x] TauriでRustワークスペース設定
- [x] CLI引数処理（引数なし=ヘルプ、大文字小文字区別なし拡張子）
- [x] 基本ファイル形式パーサー実装
- [x] 言語振り分けシステム設計
- [x] クロスプラットフォーム対応確認

### フェーズ2: デコードエンジン
- [x] Rustコア: ToHeart PAKアーカイブ展開
- [x] Rustコア: ToHeart LF2デコーダー
- [x] Rustコア: Kanon PDTデコーダー
- [ ] Rustコア: Kanon G00デコーダー
- [ ] Pythonブリッジ実装
- [ ] TypeScript/WASMブリッジ

### 🚨 フェーズ2.5: エンコードエンジン（緊急追加）
- [ ] Rustコア: ToHeart LF2エンコーダー
- [ ] Rustコア: Kanon PDTエンコーダー
- [ ] 往復テスト実装
- [ ] 写真からの変換機能

### フェーズ3: 可視化システム
- [ ] HTML5 Canvasレンダラー
- [ ] 段階的状態管理
- [ ] タイムライン制御コンポーネント
- [ ] バイナリ/hexビューコンポーネント
- [ ] メモリ状態可視化

### フェーズ4: GUI統合
- [ ] Tauriウィンドウ設定
- [ ] ファイルドラッグ&ドロップ
- [ ] フォーマット自動検出
- [ ] エクスポート機能

### フェーズ5: 教育機能
- [ ] インタラクティブツールチップと説明
- [ ] 歴史的文脈パネル
- [ ] 開発者洞察注釈
- [ ] パフォーマンス比較

### フェーズ6: 高度機能
- [ ] GPU加速（CUDA/OpenCL/WebGPU）
- [ ] フォーマット変換実験
- [ ] カスタム画像処理
- [ ] バッチ処理モード

## 葉鍵ベンチ（パフォーマンス比較システム）

### 概要
RetroDecode独自のベンチマークシステム。異なる技術手法・制約条件での処理速度を比較し、アルゴリズムの教育的理解を深める。

### Python関連オプション
```bash
--numpy          # NumPy使用（デフォルト）
--no-numpy       # 純Python（リスト・ループのみ）
--numba          # Numba JIT最適化
--cupy           # GPU加速NumPy互換
```

### JavaScript/TypeScript関連オプション
```bash
--typed-arrays   # Uint8Array使用（デフォルト）
--no-typed-arrays # 通常のArray使用
--wasm           # WebAssembly版を呼び出し
--worker-threads # Node.js Worker threads使用
```

### Rust関連オプション
```bash
--unsafe         # unsafeポインタ最適化
--safe-only      # safe Rustのみ（境界チェック有）
--simd           # SIMD命令使用
--single-thread  # シングルスレッド強制
```

### アルゴリズム制約オプション
```bash
--no-bulk-io     # 1バイト読み込み強制
--no-bitwise     # ビット演算禁止（除算・剰余使用）
--naive-ring     # リングバッファ最適化なし
--recursive      # 再帰実装（スタック負荷テスト）
```

### メモリ制約オプション
```bash
--memory-limit <MB>  # 使用メモリ上限
--no-cache           # キャッシュ無効化
--streaming          # ストリーミング処理強制
```

### ベンチマーク例
```bash
# 基本比較: 最速 vs 最遅
retro-decode --input file.lf2 --lang rust --unsafe --simd
retro-decode --input file.lf2 --lang python --no-numpy --recursive

# 言語間パフォーマンス比較
retro-decode --input file.pdt --lang rust --benchmark
retro-decode --input file.pdt --lang python --numpy --benchmark
retro-decode --input file.pdt --lang typescript --typed-arrays --benchmark

# 制約条件での教育的比較
retro-decode --input archive.pak --no-bulk-io --no-bitwise  # 古典手法
retro-decode --input archive.pak --memory-limit 64         # 制限環境
```

### 葉鍵ベンチ結果出力
- 処理時間詳細（パース・デコード・書き込み別）
- メモリ使用量推移
- CPU使用率
- 最適化手法の効果測定
- 教育的洞察とレコメンデーション

## 開発ノート

### パフォーマンス考慮事項
- メモリ効率のためのリングバッファ実装
- GPU加速オプション（--gpu）
- 並列処理オプション（--parallel）：シングルスレッドとの比較用
- ブラウザパフォーマンスのためのWebAssembly
- クロスプラットフォーム対応（Windows/macOS/Linux）

### ユーザーエクスペリエンス
- 直感的なタイムラインスクラビング
- リアルタイムプレビュー更新
- レスポンシブダークテーマデザイン
- アクセシブルキーボードショートカット

### 遵守と倫理
- ユーザー所有ファイル処理のみ
- 教育目的ドキュメント
- 歴史保存フォーカス
- 著作権コンテンツ配布なし

## 📈 進捗追跡

### 週次更新
- **2025年7月2日**: 透過PNG機能完成、エンコード機能不足判明、ディレクトリ整理計画策定

### 次回マイルストーン
- **2025年7月9日目標**: LF2エンコード機能実装完了
- **2025年7月16日目標**: PDTエンコード機能実装完了
- **2025年7月23日目標**: 往復テスト実装完了

---

**P⁴へようこそ — 1ピクセルずつ、過去を保存**

*限られたハードウェアでビジュアルストーリーテリングを実現した巧妙な圧縮技術を探求します。*

**現在の重要課題**: エンコード機能実装による往復テスト実現