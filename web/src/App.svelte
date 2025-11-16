<script>
  import Router from 'svelte-spa-router';
  import Header from './components/Header.svelte';
  import Footer from './components/Footer.svelte';

  // ページコンポーネント
  import Home from './pages/Home.svelte';
  import DecodePage from './pages/DecodePage.svelte';
  import EncodePage from './pages/EncodePage.svelte';
  import AboutPage from './pages/AboutPage.svelte';

  // ルート定義
  const routes = {
    '/': Home,
    '/decode': DecodePage,
    '/encode': EncodePage,
    '/about': AboutPage,
  };
</script>

<div class="app">
  <Header />

  <main class="main-content">
    <Router {routes} />
  </main>

  <Footer />
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: #f5f7fa;
    color: #2c3e50;
  }

  :global(*) {
    box-sizing: border-box;
  }

  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .main-content {
    flex: 1;
    padding-top: 2rem;
  }

  /* スムーススクロール */
  :global(html) {
    scroll-behavior: smooth;
  }

  /* リンクホバー効果 */
  :global(a) {
    transition: all 0.2s ease;
  }

  /* コードブロック */
  :global(code) {
    background: #f8f9fa;
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.9em;
    color: #e83e8c;
  }

  /* 選択テキスト */
  :global(::selection) {
    background: #667eea;
    color: white;
  }
</style>
