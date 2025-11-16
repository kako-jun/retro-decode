# RetroDecode Web Visualizer

LZSS圧縮・展開処理をステップバイステップで可視化するWebアプリケーション

## 🚀 クイックスタート

```bash
# 依存関係インストール
npm install

# 開発サーバー起動 (http://localhost:5173)
npm run dev

# プロダクションビルド
npm run build
```

## 🎨 機能

- **複数パネル同期表示**
  - 圧縮データビュー（ハイライト付き）
  - リングバッファ（ヒートマップ）
  - 出力画像（段階的描画）

- **インタラクティブ制御**
  - スライダーで任意のステップに移動
  - 再生/一時停止機能
  - 速度調整（0.25x〜4x）
  - 10ステップスキップ

- **教育的解説**
  - 各ステップの詳細説明（日本語）
  - 操作タイプ別のアイコン表示
  - 生バイト表示

## 📁 プロジェクト構造

```
web/
├── src/
│   ├── App.svelte              # メインアプリケーション
│   ├── main.js                 # エントリーポイント
│   ├── mockData.js             # モックデータ
│   └── components/
│       ├── CompressedDataPanel.svelte  # 圧縮データパネル
│       ├── RingBufferPanel.svelte      # リングバッファパネル
│       ├── ImagePanel.svelte           # 画像出力パネル
│       ├── ExplanationPanel.svelte     # 解説パネル
│       └── ControlPanel.svelte         # 再生制御パネル
├── index.html
├── package.json
├── vite.config.js
└── svelte.config.js
```

## 🔧 技術スタック

- **Svelte 5** - リアクティブUIフレームワーク
- **Vite 5** - 高速ビルドツール
- **JavaScript (ES6+)** - メインロジック

## 🎯 次のステップ

- [ ] WASM統合（Rust backend）
- [ ] 実際のLF2ファイル読み込み
- [ ] エンコード過程の可視化
- [ ] ステップ比較モード
