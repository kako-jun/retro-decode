# RetroDecode Browser Extension

**レトロゲーム画像形式（LF2/PDT）をブラウザで自動デコード**

普通のWebページの `<img>` タグで `.lf2` や `.pdt` ファイルを指定すると、自動的にPNG形式に変換して表示します。

## 🎯 できること

```html
<!-- 普通のWebページで -->
<img src="chara.lf2" alt="キャラクター">
<img src="bg.pdt" alt="背景">
```

拡張をインストールすると、上記のコードがそのまま動作します！

## 📦 対応形式

- **LF2** (ToHeart): LZSS圧縮、完全実装 ✅
- **PDT** (Kanon): RLE圧縮、基礎実装 ⚠️

## 🚀 インストール方法

### Chrome / Edge

1. `chrome://extensions/` を開く
2. 右上の「デベロッパーモード」を有効化
3. 「パッケージ化されていない拡張機能を読み込む」をクリック
4. この `browser-extension` フォルダを選択

### Firefox

1. `about:debugging#/runtime/this-firefox` を開く
2. 「一時的なアドオンを読み込む」をクリック
3. この `browser-extension` フォルダ内の `manifest.json` を選択

## 📖 使い方

### 1. 基本的な使い方

拡張をインストールした状態で、任意のWebページに `.lf2` または `.pdt` 画像を埋め込むだけ：

```html
<img src="path/to/image.lf2" alt="画像">
```

### 2. デモページで確認

`demo.html` を開いて動作確認：

```bash
# このフォルダで
open demo.html  # macOS
start demo.html # Windows
xdg-open demo.html # Linux
```

### 3. 動的に追加される画像にも対応

JavaScript で `<img>` を追加しても自動的にデコードされます：

```javascript
const img = document.createElement('img');
img.src = 'dynamic.lf2';
document.body.appendChild(img); // 自動的にデコード
```

## 🔬 技術詳細

### アーキテクチャ

```
content-script.js
  ├─ MutationObserver（DOM監視）
  ├─ fetch（画像取得）
  └─ デコーダー
      ├─ lf2Decoder.js（LZSS展開）
      ├─ pdtDecoder.js（RLE展開）
      └─ imageConverter.js（Canvas API → PNG data URL）
```

### 処理フロー

1. ページ内の `<img src="*.lf2">` を検出
2. fetch で元ファイルを取得
3. LZSS/RLE デコード → RGBA配列
4. Canvas API で PNG data URL 生成
5. `img.src` を data URL に置換

### MutationObserver

動的に追加される画像やsrc属性の変更も監視：

- `childList: true` - 要素の追加/削除
- `subtree: true` - 子孫要素も監視
- `attributeFilter: ['src']` - src属性の変更

## 🛠️ 開発

### ファイル構成

```
browser-extension/
├── manifest.json          # Manifest V3形式
├── content-script.js      # メイン処理（DOM監視 + 変換）
├── decoder/
│   ├── lf2Decoder.js     # LF2デコーダー（LZSS）
│   ├── pdtDecoder.js     # PDTデコーダー（RLE基礎実装）
│   └── imageConverter.js # RGBA → PNG data URL
├── popup/
│   └── popup.html        # 拡張アイコンクリック時のUI
├── icons/                # アイコン画像（未実装）
├── demo.html             # デモページ
└── README.md             # このファイル
```

### デバッグ

**Chrome DevTools Console:**

```javascript
// 拡張のログを確認
[RetroDecode] Extension loaded
[RetroDecode] Processing LF2: http://example.com/image.lf2
[RetroDecode] Successfully converted: http://example.com/image.lf2
```

**エラーハンドリング:**

- デコード失敗時は画像に赤枠を表示
- `img.title` にエラーメッセージを設定
- Console にエラーログ出力

## ⚖️ 法的遵守

- **教育目的**: レトロゲーム画像フォーマットの技術研究
- **ユーザー所有ファイルのみ**: 著作権侵害を意図していません
- **歴史保存**: 失われた技術の復元と保存

## 🔗 リンク

- **Web版デコーダ**: https://kako-jun.github.io/retro-decode/
- **GitHubリポジトリ**: https://github.com/kako-jun/retro-decode
- **研究成果**: [ultimate-achievement-2025-07-07.md](../docs/ultimate-achievement-2025-07-07.md)

## 📝 ライセンス

MIT License

---

**P⁴ — Pixel by pixel, past preserved**
