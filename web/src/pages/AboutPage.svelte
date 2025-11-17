<script>
  import { link } from 'svelte-spa-router';
</script>

<div class="about-page">
  <div class="page-header">
    <h1>📚 About RetroDecode</h1>
    <p class="tagline">P⁴ — Pixel by pixel, past preserved</p>
  </div>

  <!-- プロジェクト概要 -->
  <section class="section">
    <h2>🎯 プロジェクト概要</h2>
    <div class="content-box">
      <p>
        <strong>RetroDecode</strong> は、日本のレトロビジュアルノベルの画像デコード処理を解析・可視化する教育ツールです。
        歴史的な暗号化技術をインタラクティブな段階的可視化で実演します。
      </p>
      <p>
        <strong>キャッチフレーズ</strong>: "Pixel by pixel, past preserved" （1ピクセルずつ、過去を保存）<br>
        <strong>略称</strong>: P⁴ (4つのP: Pixel, by, Pixel, Past, Preserved)
      </p>
    </div>
  </section>

  <!-- 研究成果 -->
  <section class="section">
    <h2>🏆 歴史的突破達成（2025年7月7日）</h2>
    <div class="achievements-grid">
      <div class="achievement-item">
        <h3>✨ 0 diffs 達成</h3>
        <p class="metric">14,366 diffs → 0 diffs</p>
        <p>完全精度での往復変換を実現。自己参照LZSS問題の初の完全解決。</p>
      </div>

      <div class="achievement-item">
        <h3>📉 45.1% 圧縮改善</h3>
        <p class="metric">50,366バイト → 27,646バイト</p>
        <p>LZSS圧縮アルゴリズムの最適化により劇的な圧縮率改善を達成。</p>
      </div>

      <div class="achievement-item">
        <h3>🔬 自己参照問題解決</h3>
        <p class="metric">安全性チェック実装</p>
        <p>distance == length による自己参照の検出と排除を実現。</p>
      </div>

      <div class="achievement-item">
        <h3>🎓 失われた技術復元</h3>
        <p class="metric">1990年代技術の現代再現</p>
        <p>開発企業がソースコードを失った技術を暗号解読的アプローチで復元。</p>
      </div>
    </div>
  </section>

  <!-- 技術詳細 -->
  <section class="section">
    <h2>⚙️ 技術詳細</h2>

    <div class="tech-section">
      <h3>LF2形式（ToHeart）</h3>
      <div class="content-box">
        <ul>
          <li><strong>マジックナンバー</strong>: LEAF256（8バイト）</li>
          <li><strong>パレット</strong>: 256色、BGR形式</li>
          <li><strong>圧縮方式</strong>: LZSS（Lempel-Ziv-Storer-Szymanski）</li>
          <li><strong>リングバッファ</strong>: 4096バイト</li>
          <li><strong>特徴</strong>: フラグバイトによる制御、直接ピクセルとLZSSマッチの切り替え</li>
        </ul>
      </div>
    </div>

    <div class="tech-section">
      <h3>PDT形式（Kanon）</h3>
      <div class="content-box">
        <ul>
          <li><strong>ヘッダー</strong>: 幅・高さ情報（4バイト）</li>
          <li><strong>圧縮方式</strong>: RLEベース</li>
          <li><strong>状態</strong>: 基礎対応済み（構造解析完了）</li>
          <li><strong>今後の展開</strong>: 完全デコード実装予定</li>
        </ul>
      </div>
    </div>

    <div class="tech-section">
      <h3>LZSSアルゴリズム</h3>
      <div class="content-box">
        <p><strong>LZSS</strong>（Lempel-Ziv-Storer-Szymanski）は、辞書ベースの可逆圧縮アルゴリズムです。</p>
        <ul>
          <li><strong>基本原理</strong>: 過去のデータ（リングバッファ）を参照して繰り返しパターンを圧縮</li>
          <li><strong>フラグバイト</strong>: 8ビットで次の8操作を制御（1=直接ピクセル、0=マッチ参照）</li>
          <li><strong>マッチエンコード</strong>: 距離（12ビット）+ 長さ（4ビット）で過去データを参照</li>
          <li><strong>リングバッファ</strong>: 4096バイトの循環バッファで過去データを保持</li>
        </ul>
        <div class="code-example">
          <strong>例</strong>: ピクセル列 [5,5,5,5] の圧縮<br>
          → 直接: [5] + マッチ: (距離=1, 長さ=3) = "1バイト前から3バイトコピー"
        </div>
      </div>
    </div>
  </section>

  <!-- 実装技術 -->
  <section class="section">
    <h2>🛠️ 実装技術</h2>
    <div class="tech-stack">
      <div class="stack-item">
        <h4>コア処理</h4>
        <p>Rust（CLI・LZSS実装）</p>
      </div>
      <div class="stack-item">
        <h4>フロントエンド</h4>
        <p>Svelte 5 + Vite 5.4</p>
      </div>
      <div class="stack-item">
        <h4>機械学習研究</h4>
        <p>PyTorch + CUDA（GTX 1050 Ti）</p>
      </div>
      <div class="stack-item">
        <h4>ルーティング</h4>
        <p>svelte-spa-router</p>
      </div>
      <div class="stack-item">
        <h4>デプロイ</h4>
        <p>GitHub Pages（静的サイト）</p>
      </div>
      <div class="stack-item">
        <h4>WASM対応</h4>
        <p>準備済み（将来統合予定）</p>
      </div>
    </div>
  </section>

  <!-- 機械学習研究 -->
  <section class="section">
    <h2>🧠 機械学習研究の成果と教訓</h2>
    <div class="content-box">
      <h3>研究概要</h3>
      <ul>
        <li><strong>データセット</strong>: 522ファイル → 2,465,637決定ポイント</li>
        <li><strong>モデル</strong>: 447,685パラメータのニューラルネットワーク</li>
        <li><strong>最終精度</strong>: 75.3%決定精度（Epoch 20）</li>
        <li><strong>GPU</strong>: GTX 1050 Ti（3GB VRAM）</li>
      </ul>

      <h3>重要発見</h3>
      <p>
        <code>compression_progress</code>（27.4%）と<code>estimated_y</code>（16.6%）が最重要特徴量。
        しかし75.3%精度では60万誤決定が発生し、完璧な再現には不十分と判明。
      </p>

      <h3>教訓</h3>
      <blockquote>
        「MLは壮大で価値ある遠回り」— 教育価値★★★★★、実用価値★★☆☆☆<br>
        既存手法の精密調整が最も確実で、MLは知見蓄積とアプローチ多様性に貢献。
      </blockquote>
    </div>
  </section>

  <!-- 法的遵守 -->
  <section class="section legal">
    <h2>⚖️ 法的遵守と目的</h2>
    <div class="content-box">
      <ul>
        <li><strong>教育目的</strong>: レトロゲームで使用された暗号化技術の研究</li>
        <li><strong>歴史保存</strong>: 古いメモリ最適化手法の理解と保存</li>
        <li><strong>インタラクティブ学習</strong>: デコード過程の段階的可視化</li>
        <li><strong>法的遵守</strong>: ユーザー所有のファイル処理のみ、著作権侵害なし</li>
        <li><strong>往復検証</strong>: デコード→エンコード→比較による実装完全性確認</li>
        <li><strong>文化的意義</strong>: 開発企業がソースコードを失っても当時と同じLF2生成を可能にする</li>
      </ul>
      <p class="disclaimer">
        このツールはユーザー所有のファイル処理のみを行います。著作権侵害を意図したものではありません。
      </p>
    </div>
  </section>

  <!-- CTA -->
  <section class="cta">
    <h2>さっそく試してみよう</h2>
    <div class="cta-buttons">
      <a href="#/decode" use:link class="cta-button">🔓 デコードを試す</a>
      <a href="#/encode" use:link class="cta-button">🔒 エンコードを試す</a>
      <a href="https://github.com/kako-jun/retro-decode" target="_blank" rel="noopener noreferrer" class="cta-button secondary">
        GitHub リポジトリ
      </a>
    </div>
  </section>
</div>

<style>
  .about-page {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  .page-header {
    text-align: center;
    margin-bottom: 3rem;
  }

  .page-header h1 {
    font-size: 3rem;
    color: #2c3e50;
    margin: 0 0 0.5rem 0;
  }

  .tagline {
    font-size: 1.3rem;
    color: #7f8c8d;
    font-style: italic;
  }

  .section {
    margin-bottom: 3rem;
  }

  .section h2 {
    font-size: 2rem;
    color: #2c3e50;
    margin-bottom: 1.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 3px solid #667eea;
  }

  .content-box {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    border-left: 4px solid #667eea;
  }

  .content-box p {
    line-height: 1.8;
    color: #2c3e50;
    margin-bottom: 1rem;
  }

  .content-box ul {
    line-height: 1.8;
    color: #34495e;
  }

  .content-box li {
    margin-bottom: 0.5rem;
  }

  .achievements-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.5rem;
  }

  .achievement-item {
    background: white;
    padding: 1.5rem;
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    border: 2px solid #e1e8ed;
    transition: all 0.3s;
  }

  .achievement-item:hover {
    transform: translateY(-5px);
    box-shadow: 0 8px 20px rgba(102, 126, 234, 0.3);
    border-color: #667eea;
  }

  .achievement-item h3 {
    color: #667eea;
    margin: 0 0 0.5rem 0;
    font-size: 1.3rem;
  }

  .metric {
    font-size: 1.1rem;
    font-weight: 600;
    color: #2c3e50;
    margin: 0.5rem 0;
  }

  .achievement-item p:last-child {
    color: #7f8c8d;
    font-size: 0.9rem;
    margin-top: 0.5rem;
  }

  .tech-section {
    margin-bottom: 1.5rem;
  }

  .tech-section h3 {
    color: #34495e;
    margin-bottom: 0.8rem;
    font-size: 1.3rem;
  }

  .code-example {
    background: #f8f9fa;
    padding: 1rem;
    border-radius: 6px;
    margin-top: 1rem;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
    border-left: 3px solid #3498db;
  }

  .tech-stack {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }

  .stack-item {
    background: white;
    padding: 1rem;
    border-radius: 8px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
    text-align: center;
  }

  .stack-item h4 {
    color: #667eea;
    margin: 0 0 0.5rem 0;
  }

  .stack-item p {
    color: #7f8c8d;
    font-size: 0.9rem;
    margin: 0;
  }

  blockquote {
    background: #fff3cd;
    border-left: 4px solid #ffc107;
    padding: 1rem;
    margin: 1rem 0;
    font-style: italic;
    color: #856404;
  }

  .legal {
    background: linear-gradient(135deg, #f8f9fa 0%, #e9ecef 100%);
    padding: 2rem;
    border-radius: 12px;
  }

  .disclaimer {
    background: #fff3cd;
    border: 2px solid #ffc107;
    padding: 1rem;
    border-radius: 8px;
    font-weight: 600;
    color: #856404;
    margin-top: 1rem;
  }

  .cta {
    text-align: center;
    padding: 3rem 2rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border-radius: 20px;
    color: white;
  }

  .cta h2 {
    font-size: 2rem;
    margin: 0 0 2rem 0;
  }

  .cta-buttons {
    display: flex;
    gap: 1rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .cta-button {
    padding: 1rem 2rem;
    background: white;
    color: #667eea;
    border-radius: 8px;
    text-decoration: none;
    font-weight: 600;
    transition: all 0.3s;
    display: inline-block;
  }

  .cta-button:hover {
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
  }

  /* レスポンシブ */
  @media (max-width: 768px) {
    .about-page {
      padding: 1rem;
    }

    .page-header h1 {
      font-size: 2rem;
    }

    .tagline {
      font-size: 1.1rem;
    }

    .section h2 {
      font-size: 1.5rem;
    }

    .achievements-grid,
    .tech-stack {
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
