<script>
  import { link } from 'svelte-spa-router';
</script>

<div class="home-page">
  <!-- ヒーローセクション -->
  <section class="hero">
    <div class="hero-content">
      <h1 class="hero-title">
        <span class="gradient-text">RetroDecode</span>
      </h1>
      <p class="hero-subtitle">P⁴ — Pixel by pixel, past preserved</p>
      <p class="hero-description">
        日本のレトロビジュアルノベルの画像デコード処理を解析・可視化する教育ツール<br>
        歴史的な暗号化技術をインタラクティブな段階的可視化で実演
      </p>
    </div>
  </section>

  <!-- 研究成果ハイライト -->
  <section class="achievements">
    <h2>🏆 歴史的突破達成（2025年7月7日）</h2>
    <div class="achievement-grid">
      <div class="achievement-card">
        <div class="achievement-icon">✨</div>
        <h3>0 diffs</h3>
        <p>完全精度達成</p>
        <span class="detail">14,366 diffs → 0 diffs の完全解決</span>
      </div>

      <div class="achievement-card">
        <div class="achievement-icon">📉</div>
        <h3>45.1%</h3>
        <p>圧縮率改善</p>
        <span class="detail">50,366バイト → 27,646バイト</span>
      </div>

      <div class="achievement-card">
        <div class="achievement-icon">🔬</div>
        <h3>自己参照問題</h3>
        <p>完全解決</p>
        <span class="detail">LZSS安全性チェック実装</span>
      </div>

      <div class="achievement-card">
        <div class="achievement-icon">🎓</div>
        <h3>失われた技術</h3>
        <p>復元完了</p>
        <span class="detail">1990年代圧縮技術の現代再現</span>
      </div>
    </div>
  </section>

  <!-- 機能カード -->
  <section class="features">
    <h2>🚀 機能</h2>
    <div class="feature-cards">
      <a href="#/decode" use:link class="feature-card">
        <div class="card-icon">🔓</div>
        <h3>デコード可視化</h3>
        <p>LF2/PDT形式の画像を段階的にデコード</p>
        <ul>
          <li>LZSS展開過程の可視化</li>
          <li>リングバッファ状態表示</li>
          <li>バイナリ構造の色分け</li>
          <li>ステップバイステップ解説</li>
        </ul>
        <div class="card-action">デコードを試す →</div>
      </a>

      <a href="#/encode" use:link class="feature-card">
        <div class="card-icon">🔒</div>
        <h3>エンコード可視化</h3>
        <p>PNG/JPEG画像をLF2形式に圧縮</p>
        <ul>
          <li>パレット生成過程</li>
          <li>LZSS圧縮アルゴリズム</li>
          <li>圧縮率のリアルタイム表示</li>
          <li>LF2ファイル自動生成</li>
        </ul>
        <div class="card-action">エンコードを試す →</div>
      </a>

      <a href="#/about" use:link class="feature-card">
        <div class="card-icon">📚</div>
        <h3>技術詳細 & 研究成果</h3>
        <p>プロジェクトの背景と技術解説</p>
        <ul>
          <li>LF2/PDT形式の仕様</li>
          <li>LZSS圧縮の仕組み</li>
          <li>研究成果の詳細</li>
          <li>実装技術の解説</li>
        </ul>
        <div class="card-action">詳しく見る →</div>
      </a>
    </div>
  </section>

  <!-- 対応形式 -->
  <section class="formats">
    <h2>📦 対応形式</h2>
    <div class="format-grid">
      <div class="format-card">
        <h3>ToHeart シリーズ</h3>
        <div class="format-badge supported">LF2 ✅</div>
        <div class="format-badge">PAK ✅</div>
        <p class="format-desc">LEAF256形式、LZSS圧縮対応</p>
      </div>

      <div class="format-card">
        <h3>Kanon シリーズ</h3>
        <div class="format-badge supported">PDT ✅</div>
        <div class="format-badge">G00 ⚠️</div>
        <p class="format-desc">RLEベース圧縮、基礎対応済み</p>
      </div>

      <div class="format-card">
        <h3>標準画像形式</h3>
        <div class="format-badge supported">PNG ✅</div>
        <div class="format-badge supported">JPEG ✅</div>
        <p class="format-desc">エンコード入力として対応</p>
      </div>
    </div>
  </section>

  <!-- プロジェクト目的 -->
  <section class="purpose">
    <h2>🎯 プロジェクト目的</h2>
    <div class="purpose-grid">
      <div class="purpose-item">
        <div class="purpose-icon">📖</div>
        <h4>教育目的</h4>
        <p>レトロゲームで使用された暗号化技術の研究と学習</p>
      </div>
      <div class="purpose-item">
        <div class="purpose-icon">🏛️</div>
        <h4>歴史保存</h4>
        <p>古いメモリ最適化手法（リングバッファ等）の理解と保存</p>
      </div>
      <div class="purpose-item">
        <div class="purpose-icon">🎨</div>
        <h4>インタラクティブ学習</h4>
        <p>デコード過程の段階的可視化による直感的理解</p>
      </div>
      <div class="purpose-item">
        <div class="purpose-icon">⚖️</div>
        <h4>法的遵守</h4>
        <p>ユーザー所有のファイル処理のみ、著作権侵害なし</p>
      </div>
      <div class="purpose-item">
        <div class="purpose-icon">🔄</div>
        <h4>往復検証</h4>
        <p>デコード→エンコード→比較による実装完全性確認</p>
      </div>
      <div class="purpose-item">
        <div class="purpose-icon">🔓</div>
        <h4>失われた技術の復元</h4>
        <p>暗号解読的アプローチによる完全アルゴリズム再現</p>
      </div>
    </div>
  </section>

  <!-- CTA -->
  <section class="cta">
    <h2>今すぐ試してみよう</h2>
    <p>LF2/PDT画像のデコード、またはPNG/JPEG画像のエンコードを体験</p>
    <div class="cta-buttons">
      <a href="#/decode" use:link class="cta-button primary">
        🔓 デコードを試す
      </a>
      <a href="#/encode" use:link class="cta-button secondary">
        🔒 エンコードを試す
      </a>
    </div>
  </section>
</div>

<style>
  .home-page {
    max-width: 1400px;
    margin: 0 auto;
    padding: 2rem;
  }

  /* ヒーローセクション */
  .hero {
    text-align: center;
    padding: 4rem 2rem;
    background: linear-gradient(135deg, #f5f7fa 0%, #e8ecf1 100%);
    border-radius: 20px;
    margin-bottom: 3rem;
  }

  .hero-title {
    font-size: 4rem;
    margin: 0;
    line-height: 1.2;
  }

  .gradient-text {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .hero-subtitle {
    font-size: 1.5rem;
    color: #7f8c8d;
    font-style: italic;
    margin: 1rem 0;
  }

  .hero-description {
    font-size: 1.1rem;
    color: #2c3e50;
    line-height: 1.8;
    max-width: 800px;
    margin: 1.5rem auto 0;
  }

  /* 研究成果 */
  .achievements {
    margin-bottom: 4rem;
  }

  .achievements h2 {
    text-align: center;
    font-size: 2rem;
    color: #2c3e50;
    margin-bottom: 2rem;
  }

  .achievement-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.5rem;
  }

  .achievement-card {
    background: white;
    padding: 2rem;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    text-align: center;
    border: 2px solid #e1e8ed;
    transition: all 0.3s;
  }

  .achievement-card:hover {
    transform: translateY(-5px);
    box-shadow: 0 8px 20px rgba(102, 126, 234, 0.3);
    border-color: #667eea;
  }

  .achievement-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .achievement-card h3 {
    color: #667eea;
    font-size: 2rem;
    margin: 0.5rem 0;
  }

  .achievement-card p {
    color: #2c3e50;
    font-weight: 600;
    margin: 0.5rem 0;
  }

  .detail {
    display: block;
    font-size: 0.85rem;
    color: #7f8c8d;
    margin-top: 0.5rem;
  }

  /* 機能カード */
  .features {
    margin-bottom: 4rem;
  }

  .features h2 {
    text-align: center;
    font-size: 2rem;
    color: #2c3e50;
    margin-bottom: 2rem;
  }

  .feature-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
  }

  .feature-card {
    background: white;
    padding: 2rem;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border: 2px solid #e1e8ed;
    text-decoration: none;
    color: inherit;
    transition: all 0.3s;
    display: flex;
    flex-direction: column;
  }

  .feature-card:hover {
    transform: translateY(-8px);
    box-shadow: 0 12px 24px rgba(102, 126, 234, 0.3);
    border-color: #667eea;
  }

  .card-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .feature-card h3 {
    color: #2c3e50;
    margin: 0 0 1rem 0;
  }

  .feature-card p {
    color: #7f8c8d;
    margin-bottom: 1rem;
  }

  .feature-card ul {
    list-style: none;
    padding: 0;
    margin: 1rem 0;
    flex-grow: 1;
  }

  .feature-card li {
    color: #34495e;
    padding: 0.3rem 0;
    padding-left: 1.5rem;
    position: relative;
    font-size: 0.9rem;
  }

  .feature-card li::before {
    content: "✓";
    position: absolute;
    left: 0;
    color: #27ae60;
    font-weight: bold;
  }

  .card-action {
    color: #667eea;
    font-weight: 600;
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #e1e8ed;
  }

  /* 対応形式 */
  .formats {
    margin-bottom: 4rem;
  }

  .formats h2 {
    text-align: center;
    font-size: 2rem;
    color: #2c3e50;
    margin-bottom: 2rem;
  }

  .format-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.5rem;
  }

  .format-card {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    border: 2px solid #e1e8ed;
  }

  .format-card h3 {
    color: #2c3e50;
    margin: 0 0 1rem 0;
  }

  .format-badge {
    display: inline-block;
    padding: 0.3rem 0.8rem;
    border-radius: 20px;
    font-size: 0.85rem;
    margin: 0.2rem;
    background: #ecf0f1;
    color: #7f8c8d;
  }

  .format-badge.supported {
    background: #d5f4e6;
    color: #27ae60;
    font-weight: 600;
  }

  .format-desc {
    color: #7f8c8d;
    font-size: 0.9rem;
    margin-top: 0.8rem;
  }

  /* プロジェクト目的 */
  .purpose {
    margin-bottom: 4rem;
  }

  .purpose h2 {
    text-align: center;
    font-size: 2rem;
    color: #2c3e50;
    margin-bottom: 2rem;
  }

  .purpose-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1.5rem;
  }

  .purpose-item {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    border-left: 4px solid #667eea;
  }

  .purpose-icon {
    font-size: 2rem;
    margin-bottom: 0.5rem;
  }

  .purpose-item h4 {
    color: #2c3e50;
    margin: 0.5rem 0;
  }

  .purpose-item p {
    color: #7f8c8d;
    font-size: 0.9rem;
    margin: 0.5rem 0 0;
  }

  /* CTA */
  .cta {
    text-align: center;
    padding: 3rem 2rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 20px;
    color: white;
  }

  .cta h2 {
    font-size: 2rem;
    margin: 0 0 1rem 0;
  }

  .cta p {
    font-size: 1.1rem;
    margin-bottom: 2rem;
    opacity: 0.9;
  }

  .cta-buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .cta-button {
    padding: 1rem 2rem;
    border-radius: 8px;
    text-decoration: none;
    font-weight: 600;
    font-size: 1.1rem;
    transition: all 0.3s;
    display: inline-block;
  }

  .cta-button.primary {
    background: white;
    color: #667eea;
  }

  .cta-button.primary:hover {
    transform: translateY(-3px);
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
  }

  .cta-button.secondary {
    background: rgba(255, 255, 255, 0.2);
    color: white;
    border: 2px solid white;
  }

  .cta-button.secondary:hover {
    background: rgba(255, 255, 255, 0.3);
    transform: translateY(-3px);
  }

  /* レスポンシブ */
  @media (max-width: 768px) {
    .home-page {
      padding: 1rem;
    }

    .hero {
      padding: 2rem 1rem;
    }

    .hero-title {
      font-size: 2.5rem;
    }

    .hero-subtitle {
      font-size: 1.2rem;
    }

    .hero-description {
      font-size: 1rem;
    }

    .achievements h2,
    .features h2,
    .formats h2,
    .purpose h2 {
      font-size: 1.5rem;
    }

    .achievement-grid,
    .format-grid,
    .purpose-grid {
      grid-template-columns: 1fr;
    }

    .cta h2 {
      font-size: 1.5rem;
    }

    .cta-buttons {
      flex-direction: column;
    }

    .cta-button {
      width: 100%;
    }
  }
</style>
