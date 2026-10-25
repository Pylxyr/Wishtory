<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { app, initLogging, initProgress, reloadAccounts, setTheme, type Page } from './lib/state.svelte';
  import { api } from './lib/api';
  import { dur } from './lib/motion';
  import GameBar from './lib/components/GameBar.svelte';
  import Icon from './lib/components/Icon.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import Tutorial from './lib/components/Tutorial.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import History from './pages/History.svelte';
  import Accounts from './pages/Accounts.svelte';
  import ImportExport from './pages/ImportExport.svelte';
  import Help from './pages/Help.svelte';
  import Settings from './pages/Settings.svelte';
  import Logging from './pages/Logging.svelte';

  const tabs: { id: Page; label: string; icon: string }[] = [
    { id: 'dashboard', label: 'Overview', icon: 'home' },
    { id: 'history', label: 'History', icon: 'list' },
    { id: 'accounts', label: 'Accounts', icon: 'users' },
    { id: 'importexport', label: 'Import / Export', icon: 'swap' }
  ];
  const tools = $derived<{ id: Page; label: string; icon: string }[]>([
    { id: 'help', label: 'Help', icon: 'help' },
    { id: 'settings', label: 'Settings', icon: 'sliders' },
    ...(app.loggingEnabled ? [{ id: 'logging' as Page, label: 'Logging', icon: 'terminal' }] : [])
  ]);
  // Pages that work on one game and account show the game, account and Refresh bar.
  const gamePages: Page[] = ['dashboard', 'history', 'importexport'];

  onMount(async () => {
    initProgress();
    await initLogging();
    app.catalog = await api.getCatalog().catch(() => null);
    // Best-effort data-only refresh; the bundled catalog remains available offline.
    void api.refreshCatalog().then(async (status) => {
      if (status === 'updated') app.catalog = await api.getCatalog();
    }).catch(() => {});
    app.installs = await api.discoverGames().catch(() => []);
    await reloadAccounts().catch(() => {});
    if (!localStorage.getItem('wishtory.tutorialDone')) app.tutorial = true;
  });
</script>

<div class="shell">
  <header class="topbar">
    <div class="brand">
      <svg class="brand-mark" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 1.5l2.4 8.1L22.5 12l-8.1 2.4L12 22.5l-2.4-8.1L1.5 12l8.1-2.4z" /></svg>
      <span>Wishtory</span>
    </div>
    <nav class="topnav" aria-label="Main">
      {#each tabs as t}
        <button aria-label={t.label} aria-current={app.page === t.id ? 'page' : undefined} onclick={() => (app.page = t.id)}>
          <Icon name={t.icon} size={16} /><span class="nav-text">{t.label}</span>
        </button>
      {/each}
    </nav>
    <div class="topbar-tools">
      <span class="chip ok" data-tip="Wishtory only reads files the game already wrote. It never injects, edits game files or uses a proxy."><Icon name="shield" size={12} /> Read-only</span>
      {#each tools as t}
        <button class="icon-btn" aria-label={t.label} title={t.label} aria-current={app.page === t.id ? 'page' : undefined} onclick={() => (app.page = t.id)}><Icon name={t.icon} size={18} /></button>
      {/each}
      <button class="icon-btn" aria-label={app.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'} title={app.theme === 'dark' ? 'Light theme' : 'Dark theme'} onclick={() => setTheme(app.theme === 'dark' ? 'light' : 'dark')}>
        <Icon name={app.theme === 'dark' ? 'sun' : 'moon'} size={18} />
      </button>
    </div>
  </header>
  {#if gamePages.includes(app.page)}<GameBar />{/if}
  <main>
    {#key app.page}
      <div class="page" in:fade={{ duration: dur(120) }}>
        {#if app.page === 'dashboard'}<Dashboard />
        {:else if app.page === 'history'}<History />
        {:else if app.page === 'accounts'}<Accounts />
        {:else if app.page === 'importexport'}<ImportExport />
        {:else if app.page === 'settings'}<Settings />
        {:else if app.page === 'logging' && app.loggingEnabled}<Logging />
        {:else}<Help />{/if}
      </div>
    {/key}
  </main>
</div>
<Toasts />
{#if app.tutorial}<Tutorial />{/if}
