# アイコンファイル

ブラウザ拡張用のアイコンファイルを配置してください。

## 必要なファイル

- `icon16.png` - 16x16px（ツールバー表示用）
- `icon48.png` - 48x48px（拡張管理ページ用）
- `icon128.png` - 128x128px（Chrome Web Store用）

## 推奨デザイン

- **モチーフ**: 🎮 ゲームコントローラー + 🔓 鍵（デコードの象徴）
- **カラー**: グラデーション（`#667eea` → `#764ba2`）
- **スタイル**: フラットデザイン、モダン

## 一時的な代替案

アイコンファイルがない場合、拡張は動作しますが警告が表示されます。
開発中はこのまま進めて、後でデザイナーに依頼するか、Figma/Canvaで作成してください。

## SVGテンプレート

```svg
<svg width="128" height="128" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#667eea;stop-opacity:1" />
      <stop offset="100%" style="stop-color:#764ba2;stop-opacity:1" />
    </linearGradient>
  </defs>
  <rect width="128" height="128" rx="24" fill="url(#grad)"/>
  <text x="64" y="85" font-size="60" text-anchor="middle" fill="white">🎮</text>
</svg>
```

上記SVGを https://cloudconvert.com/svg-to-png 等で16px, 48px, 128pxに変換してください。
