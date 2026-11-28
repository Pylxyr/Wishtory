<script lang="ts">
  import { app, type Page } from './lib/state.svelte';
  import { GAMES } from './lib/types';
  import { INFO } from './lib/gameinfo';
  import Backdrop from './components/Backdrop.svelte';
  import Glyph from './components/Glyph.svelte';
  import Toasts from './components/Toasts.svelte';
  import Overview from './pages/Overview.svelte';
  import History from './pages/History.svelte';
  import Accounts from './pages/Accounts.svelte';
  import ImportExport from './pages/ImportExport.svelte';
  import Settings from './pages/Settings.svelte';
  import About from './pages/About.svelte';

  $effect(() => { app.paint(); });
  $effect(() => { void app.start(); });
  // Nothing animates while the window is hidden or minimised.
  $effect(() => {
    const sync = () => document.documentElement.toggleAttribute('data-paused', document.hidden);
    document.addEventListener('visibilitychange', sync);
    sync();
    return () => document.removeEventListener('visibilitychange', sync);
  });

  const isGamePage = $derived(app.page === 'overview' || app.page === 'history');
  const TITLES: Record<Page, string> = { overview: 'Overview', history: 'History', import: 'Import / Export', accounts: 'Accounts', settings: 'Settings', about: 'About' };
  const title = $derived(isGamePage ? (app.game === 'genshin' ? app.info.name : app.info.name.toUpperCase()) : TITLES[app.page]);
  const brandIcon = $derived(app.game === 'genshin' ? 'sparkle' : app.game === 'star_rail' ? 'train' : 'tv');
  const wordmark = $derived(app.game === 'genshin' ? 'Wishtory' : 'WISHTORY');
  const tools: [Page, string][] = [['import', 'Import / Export'], ['accounts', 'Accounts'], ['settings', 'Settings'], ['about', 'About']];
  const refreshing = $derived(app.refresh?.phase === 'running');

  // The arrow next to Refresh: paste a history link by hand, or download everything again.
  let linkOpen = $state(false);
  let link = $state('');
  let redownload = $state(false);
  let linkInput = $state<HTMLInputElement>();
  $effect(() => {
    if (linkOpen) linkInput?.focus();
  });
  async function useLink() {
    const url = link.trim();
    if (!url) return;
    linkOpen = false;
    link = '';
    await app.refreshGame(redownload, url);
  }
  async function downloadAgain() {
    linkOpen = false;
    await app.refreshGame(true);
  }

  // The shortcuts listed in docs/using.md.
  function onKey(e: KeyboardEvent) {
    if (app.startupError) return;
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === 'Escape' && linkOpen) linkOpen = false;
    else if (mod && e.key.toLowerCase() === 'r') {
      e.preventDefault(); // otherwise the webview reloads and the window starts over
      void app.refreshGame();
    } else if (mod && e.key === ',') {
      e.preventDefault();
      app.go('settings');
    } else if (e.altKey && !mod && (e.key === '1' || e.key === '2' || e.key === '3')) {
      e.preventDefault();
      app.openGame(GAMES[Number(e.key) - 1]);
    } else if (e.key === 'F1') {
      e.preventDefault();
      app.go('about');
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if app.startupError}
  <div class="empty" style="height:100%;align-content:center"><h2>Wishtory could not start</h2><p class="muted" style="max-width:520px">{app.startupError.message}</p></div>
{:else}
  <div class="shell">
    <nav class="rail" aria-label="Main">
      <div class="brand"><Glyph name={brandIcon} size={26} />{wordmark}</div>
      <div class="label group">Games</div>
      {#each GAMES as g (g)}
        <button class="nav" class:active={isGamePage && app.game === g} aria-current={isGamePage && app.game === g ? 'page' : undefined} onclick={() => app.openGame(g)}><span class="abbr">{INFO[g].abbr}</span><span>{INFO[g].short}</span></button>
      {/each}
      <div class="label group">Manage</div>
      {#each tools as [p, t] (p)}
        <button class="nav" class:active={app.page === p} aria-current={app.page === p ? 'page' : undefined} onclick={() => app.go(p)}>
          <Glyph name={p === 'import' ? 'import' : p} /><span>{t}</span>
          {#if p === 'settings' && app.catalogStatus?.waiting && !app.catalogStatus.waiting.dismissed}<span class="badge" title="A catalog update is waiting"></span>{/if}
        </button>
      {/each}
      <div class="spacer"></div>
      <div class="foot"><span>Read-only · local</span>{#if app.game === 'zenless'}<span class="rec">REC</span>{/if}</div>
    </nav>

    <div class="main">
      <Backdrop />
      <header class="pagebar">
        {#key app.navTick + app.game}<h1 class="glitch">{title}</h1>{/key}
        {#if isGamePage}
          <div class="tabs" role="tablist" aria-label="View">
            <button role="tab" aria-selected={app.page === 'overview'} class:on={app.page === 'overview'} onclick={() => app.go('overview')}>Overview</button>
            <button role="tab" aria-selected={app.page === 'history'} class:on={app.page === 'history'} onclick={() => app.go('history')}>History</button>
          </div>
        {/if}
        <span class="grow"></span>
        {#if isGamePage}
          {#if app.gameAccounts.length > 1}
            <select class="field" aria-label="Account" value={app.account?.uid} onchange={(e) => app.pickAccount(e.currentTarget.value)}>
              {#each app.gameAccounts as a (a.uid)}<option value={a.uid}>{app.nameOf(a)} · {a.uid}</option>{/each}
            </select>
          {:else if app.account}<span class="muted num">{app.nameOf(app.account)} · {app.account.uid}</span>{/if}
          <div class="linkwrap">
            <div class="splitbtn">
              <button class="btn primary" disabled={refreshing} title="Refresh (Ctrl+R)" onclick={() => app.refreshGame()}><Glyph name="refresh" size={16} />{refreshing ? 'Refreshing…' : 'Refresh'}</button>
              <button class="btn primary more" disabled={refreshing} aria-label="More ways to refresh" aria-haspopup="dialog" aria-expanded={linkOpen} onclick={() => (linkOpen = !linkOpen)}>▾</button>
            </div>
            {#if linkOpen}
              <div class="panel linkbox pop" role="dialog" aria-label="Refresh options">
                <b>Paste a history link</b>
                <p class="muted" style="margin:0">Use this when Wishtory cannot find the history page by itself. The link stays on this PC and goes only to the game's own servers.</p>
                <input class="field" bind:this={linkInput} bind:value={link} placeholder="https://…" aria-label="History link" onkeydown={(e) => e.key === 'Enter' && useLink()} />
                <label class="row" style="gap:8px"><input type="checkbox" bind:checked={redownload} />Download the whole history again</label>
                <div class="row" style="justify-content:space-between">
                  <button class="btn sm" onclick={downloadAgain}>Download everything again</button>
                  <button class="btn primary sm" disabled={!link.trim()} onclick={useLink}>Use link</button>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </header>
      <main class="content">
        {#key `${app.page}-${app.game}-${app.navTick}`}
          <div class="page-enter">
            {#if !app.ready}<div class="page"><div class="skeleton"></div></div>
            {:else if app.page === 'overview'}<Overview />
            {:else if app.page === 'history'}<History />
            {:else if app.page === 'import'}<ImportExport />
            {:else if app.page === 'accounts'}<Accounts />
            {:else if app.page === 'settings'}<Settings />
            {:else}<About />{/if}
          </div>
        {/key}
      </main>
      <footer class="statusbar"><span>{app.account ? `${app.info.short} · ${app.account.pull_count} pulls saved` : 'No account yet'}</span><span style="flex:1"></span><span>{app.catalog ? `Catalog v${app.catalog.catalog_version}` : ''}</span></footer>
    </div>
  </div>
{/if}
<Toasts />
