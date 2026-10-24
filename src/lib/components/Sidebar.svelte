<script lang="ts">
  import { app, setTheme, type Page } from '../state.svelte';
  import Icon from './Icon.svelte';

  const items: { id: Page; label: string; icon: string }[] = [
    { id: 'dashboard', label: 'Dashboard', icon: 'home' },
    { id: 'history', label: 'History', icon: 'list' },
    { id: 'accounts', label: 'Accounts', icon: 'users' },
    { id: 'importexport', label: 'Import / Export', icon: 'swap' },
    { id: 'help', label: 'Help', icon: 'help' }
  ];
</script>

<nav class="sidebar" aria-label="Main">
  <div class="brand">Wish<span>tory</span></div>
  {#each items as it}
    <button class="nav" class:active={app.page === it.id} aria-current={app.page === it.id ? 'page' : undefined} onclick={() => (app.page = it.id)}>
      <Icon name={it.icon} />{it.label}
    </button>
  {/each}
  <div class="spacer"></div>
  <span class="chip ok" data-tip="Wishtory only reads files the game already wrote. It never injects, edits game files or uses a proxy."><Icon name="shield" size={12} /> Read-only</span>
  <button class="nav" onclick={() => setTheme(app.theme === 'dark' ? 'light' : 'dark')}>
    <Icon name={app.theme === 'dark' ? 'sun' : 'moon'} />{app.theme === 'dark' ? 'Light theme' : 'Dark theme'}
  </button>
</nav>
