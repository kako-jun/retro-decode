<script>
  const currentYear = new Date().getFullYear();
</script>

<footer class="site-footer">
  <div class="footer-container">
    <div class="footer-section">
      <h3>RetroDecode</h3>
      <p class="tagline">P⁴ — Pixel by pixel, past preserved</p>
      <p class="description">
        日本のレトロビジュアルノベルの画像デコード処理を<br>
        解析・可視化する教育ツール
      </p>
    </div>

    <div class="footer-section">
      <h4>機能</h4>
      <ul>
        <li>LF2/PDT デコード可視化</li>
        <li>PNG/JPEG → LF2 エンコード</li>
        <li>LZSS圧縮アルゴリズム学習</li>
        <li>ステップバイステップ解説</li>
      </ul>
    </div>

    <div class="footer-section">
      <h4>研究成果</h4>
      <ul>
        <li>✅ 0 diffs 達成（完全精度）</li>
        <li>📉 45.1% 圧縮改善</li>
        <li>🔬 LZSS自己参照問題解決</li>
        <li>📚 失われた技術の復元</li>
      </ul>
    </div>

    <div class="footer-section">
      <h4>リンク</h4>
      <ul>
        <li><a href="https://github.com/kako-jun/retro-decode" target="_blank" rel="noopener noreferrer">GitHub リポジトリ</a></li>
        <li><a href="https://github.com/kako-jun/retro-decode/issues" target="_blank" rel="noopener noreferrer">Issues / フィードバック</a></li>
        <li><a href="https://github.com/kako-jun/retro-decode/blob/main/docs/ultimate-achievement-2025-07-07.md" target="_blank" rel="noopener noreferrer">研究成果詳細</a></li>
      </ul>
    </div>
  </div>

  <div class="footer-bottom">
    <p>
      © {currentYear} RetroDecode Project |
      <span class="legal">教育目的・歴史保存・法的遵守</span>
    </p>
    <p class="disclaimer">
      このツールはユーザー所有のファイル処理のみを行います。著作権侵害を意図したものではありません。
    </p>
  </div>
</footer>

<style>
  .site-footer {
    background: linear-gradient(135deg, #2c3e50 0%, #34495e 100%);
    color: #ecf0f1;
    padding: 3rem 2rem 1rem;
    margin-top: 4rem;
  }

  .footer-container {
    max-width: 1400px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .footer-section h3 {
    color: white;
    margin: 0 0 1rem 0;
    font-size: 1.5rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .footer-section h4 {
    color: #3498db;
    margin: 0 0 0.8rem 0;
    font-size: 1.1rem;
  }

  .tagline {
    font-style: italic;
    color: #95a5a6;
    margin: 0.5rem 0;
  }

  .description {
    color: #bdc3c7;
    line-height: 1.6;
    font-size: 0.9rem;
  }

  .footer-section ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .footer-section li {
    margin-bottom: 0.5rem;
    color: #bdc3c7;
    font-size: 0.9rem;
  }

  .footer-section a {
    color: #3498db;
    text-decoration: none;
    transition: color 0.2s;
  }

  .footer-section a:hover {
    color: #5dade2;
    text-decoration: underline;
  }

  .footer-bottom {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-top: 1.5rem;
    text-align: center;
    color: #95a5a6;
    font-size: 0.85rem;
  }

  .footer-bottom p {
    margin: 0.5rem 0;
  }

  .legal {
    color: #e74c3c;
    font-weight: 600;
  }

  .disclaimer {
    font-size: 0.75rem;
    color: #7f8c8d;
  }

  /* モバイル対応 */
  @media (max-width: 768px) {
    .site-footer {
      padding: 2rem 1rem 1rem;
    }

    .footer-container {
      grid-template-columns: 1fr;
      gap: 1.5rem;
    }

    .footer-section {
      text-align: center;
    }

    .description br {
      display: none;
    }
  }
</style>
