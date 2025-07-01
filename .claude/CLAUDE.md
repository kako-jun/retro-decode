# RetroDecode — 1ピクセルずつ、過去を保存 (P⁴)

## プロジェクト概要

**RetroDecode** は、クラシック日本ゲームの画像デコード処理を解析・可視化する教育ツールです。歴史的な暗号化技術をインタラクティブな段階的可視化で実演します。

**キャッチフレーズ**: "Pixel by pixel, past preserved" （1ピクセルずつ、過去を保存）  
**略称**: P⁴ (4つのP: Pixel, by, Pixel, Past, Preserved)

## 目的と法的遵守

- **教育目的**: レトロゲームで使用された暗号化技術の研究
- **歴史保存**: 古いメモリ最適化手法（リングバッファなど）の理解
- **インタラクティブ学習**: デコード過程の段階的可視化
- **法的遵守**: ユーザー所有のファイル処理のみ、著作権侵害なし

## アーキテクチャ

### 多言語コア設計

```
retro-decode/
├── Cargo.toml
├── src/
│   ├── main.rs        # CLI エントリーポイント
│   ├── lib.rs         # ライブラリルート
│   ├── formats/       # フォーマット別デコーダー
│   │   ├── mod.rs
│   │   ├── toheart/   # ToHeart形式（PAK、LF2、SCN）
│   │   └── kanon/     # Kanon形式（PDT、G00）
│   ├── bridge/        # 言語間連携
│   └── gui/           # Tauri GUI コード
├── src-tauri/         # Tauri設定ファイル
├── web/               # フロントエンド（HTML/CSS/JS）
├── scripts/
│   ├── python/        # Python実装スクリプト
│   └── typescript/    # TypeScript実装スクリプト
└── examples/          # 使用例とテストデータ
```

### 技術スタック

- **コア処理**: Rust（主要）、Python、TypeScript
- **GUI**: Tauri + Web技術
- **CLI**: Rust（単体バイナリ配布）
- **可視化**: HTML5 Canvas、WebGL
- **クロスプラットフォーム**: デスクトップアプリ + Web版

## サポート対象ゲーム形式

### ToHeartシリーズ
- **PAK**: アーカイブ展開
- **LF2**: 画像デコード
- **SCN**: シーンデータデコード

### Kanonシリーズ（2バージョン）
- **PDT**: 画像形式
- **G00**: 圧縮画像形式

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
retro-decode --input game.PDT --lang rust
retro-decode --input image.LF2 --lang python --gpu --parallel
retro-decode --input archive.PAK --output ./extracted/
retro-decode --gui  # Tauriインターフェース起動
retro-decode        # ヘルプ表示（引数なし）

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
- [ ] TauriでRustワークスペース設定
- [ ] CLI引数処理（引数なし=ヘルプ、大文字小文字区別なし拡張子）
- [ ] 基本ファイル形式パーサー実装
- [ ] 言語振り分けシステム設計
- [ ] クロスプラットフォーム対応確認

### フェーズ2: デコードエンジン
- [ ] Rustコア: ToHeart PAKアーカイブ展開
- [ ] Rustコア: ToHeart LF2デコーダー
- [ ] Rustコア: Kanon PDT/G00デコーダー
- [ ] Pythonブリッジ実装
- [ ] TypeScript/WASMブリッジ

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

---

**P⁴へようこそ — 1ピクセルずつ、過去を保存**

*限られたハードウェアでビジュアルストーリーテリングを実現した巧妙な圧縮技術を探求します。*