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

  const isGamePage = $derived(app.page === 'overview' || app.page === 'history');
  const TITLES: Record<Page, string> = { overview: 'Overview', history: 'History', import: 'Import / Export', accounts: 'Accounts', settings: 'Settings', about: 'About' };
  const title = $derived(isGamePage ? (app.game === 'genshin' ? app.info.name : app.info.name.toUpperCase()) : TITLES[app.page]);
  const brandIcon = $derived(app.game === 'genshin' ? 'sparkle' : app.game === 'star_rail' ? 'train' : 'tv');
  const wordmark = $derived(app.game === 'genshin' ? 'Wishtory' : 'WISHTORY');
  const tools: [Page, string][] = [['import', 'Import / Export'], ['accounts', 'Accounts'], ['settings', 'Settings'], ['about', 'About']];
  const refreshing = $derived(app.refresh?.phase === 'running');
</script>

{#if app.startupError}
  <div class="empty" style="height:100%;align-content:center"><h2>Wishtory could not start</h2><p class="muted" style="max-width:520px">{app.startupError.message}</p></div>
{:else}
  <div class="shell">
    <nav class="rail" aria-label="Main">
      <div class="brand"><Glyph name={brandIcon} size={26} />{wordmark}</div>
      <div class="label group">Games</div>
      {#each GAMES as g (g)}
        <button class="nav" class:active={isGamePage && app.game === g} onclick={() => app.openGame(g)}><span class="abbr">{INFO[g].abbr}</span><span>{INFO[g].short}</span></button>
      {/each}
      <div class="label group">Manage</div>
      {#each tools as [p, t] (p)}
        <button class="nav" class:active={app.page === p} onclick={() => app.go(p)}>
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
          <div class="tabs" role="tablist">
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
          <button class="btn primary" disabled={refreshing} onclick={() => app.refreshGame()}><Glyph name="refresh" size={16} />{refreshing ? 'Refreshing…' : 'Refresh'}</button>
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
      <div class="statusbar"><span>{app.account ? `${app.info.short} · ${app.account.pull_count} pulls saved` : 'No account yet'}</span><span style="flex:1"></span><span>{app.catalog ? `Catalog v${app.catalog.catalog_version}` : ''}</span></div>
    </div>
  </div>
{/if}
<Toasts />
