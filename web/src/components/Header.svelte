<script>
  import { link } from 'svelte-spa-router';

  let mobileMenuOpen = false;

  function toggleMobileMenu() {
    mobileMenuOpen = !mobileMenuOpen;
  }

  function closeMobileMenu() {
    mobileMenuOpen = false;
  }
</script>

<header class="site-header">
  <div class="header-container">
    <a href="#/" use:link class="logo">
      <span class="logo-icon">🎮</span>
      <span class="logo-text">RetroDecode</span>
    </a>

    <button class="mobile-menu-toggle" on:click={toggleMobileMenu} aria-label="メニュー">
      {#if mobileMenuOpen}
        ✕
      {:else}
        ☰
      {/if}
    </button>

    <nav class="nav" class:open={mobileMenuOpen}>
      <a href="#/" use:link on:click={closeMobileMenu}>ホーム</a>
      <a href="#/decode" use:link on:click={closeMobileMenu}>デコード</a>
      <a href="#/encode" use:link on:click={closeMobileMenu}>エンコード</a>
      <a href="#/about" use:link on:click={closeMobileMenu}>About</a>
      <a href="https://github.com/kako-jun/retro-decode" target="_blank" rel="noopener noreferrer" class="github-link">
        <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
        </svg>
        GitHub
      </a>
    </nav>
  </div>
</header>

<style>
  .site-header {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    position: sticky;
    top: 0;
    z-index: 1000;
  }

  .header-container {
    max-width: 1400px;
    margin: 0 auto;
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    text-decoration: none;
    color: white;
    font-size: 1.5rem;
    font-weight: bold;
    transition: transform 0.2s;
  }

  .logo:hover {
    transform: scale(1.05);
  }

  .logo-icon {
    font-size: 2rem;
  }

  .nav {
    display: flex;
    gap: 2rem;
    align-items: center;
  }

  .nav a {
    color: white;
    text-decoration: none;
    font-weight: 500;
    transition: all 0.2s;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .nav a:hover {
    background: rgba(255, 255, 255, 0.2);
    transform: translateY(-2px);
  }

  .github-link svg {
    width: 18px;
    height: 18px;
  }

  .mobile-menu-toggle {
    display: none;
    background: none;
    border: none;
    color: white;
    font-size: 1.8rem;
    cursor: pointer;
    padding: 0.5rem;
  }

  /* タブレット・モバイル対応 */
  @media (max-width: 768px) {
    .header-container {
      padding: 1rem;
    }

    .mobile-menu-toggle {
      display: block;
    }

    .nav {
      position: absolute;
      top: 100%;
      left: 0;
      right: 0;
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      flex-direction: column;
      padding: 1rem;
      gap: 0;
      display: none;
      box-shadow: 0 4px 10px rgba(0, 0, 0, 0.2);
    }

    .nav.open {
      display: flex;
    }

    .nav a {
      width: 100%;
      padding: 1rem;
      border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    }

    .nav a:last-child {
      border-bottom: none;
    }

    .logo-text {
      font-size: 1.2rem;
    }
  }

  @media (max-width: 480px) {
    .logo-text {
      display: none;
    }

    .logo-icon {
      font-size: 1.8rem;
    }
  }
</style>
