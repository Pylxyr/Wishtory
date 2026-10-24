<script lang="ts">
  import { app, setTheme, type Page } from '../state.svelte';
  import Icon from './Icon.svelte';

  const main: { id: Page; label: string }[] = [
    { id: 'dashboard', label: 'Dashboard' },
    { id: 'history', label: 'History' },
    { id: 'accounts', label: 'Accounts' },
    { id: 'importexport', label: 'Import / Export' }
  ];
  const system = $derived<{ id: Page; label: string }[]>([
    { id: 'help', label: 'Help' },
    { id: 'settings', label: 'Settings' },
    ...(app.loggingEnabled ? [{ id: 'logging' as Page, label: 'Logging' }] : [])
  ]);
  const pad = (n: number) => String(n).padStart(2, '0');
</script>

<nav class="sidebar" aria-label="Main">
  <div class="brand">
    <svg class="brand-mark" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 1.5l2.4 8.1L22.5 12l-8.1 2.4L12 22.5l-2.4-8.1L1.5 12l8.1-2.4z" /></svg>
    <span>Wish<span>tory</span></span>
  </div>
  <div class="brand-sub">LOCAL WISH LEDGER</div>

  {#each main as it, i}
    <button class="nav" class:active={app.page === it.id} aria-current={app.page === it.id ? 'page' : undefined} onclick={() => (app.page = it.id)}>
      <span class="idx">{pad(i + 1)}</span>{it.label}
    </button>
  {/each}

  <div class="nav-label">SYSTEM</div>
  {#each system as it, i}
    <button class="nav" class:active={app.page === it.id} aria-current={app.page === it.id ? 'page' : undefined} onclick={() => (app.page = it.id)}>
      <span class="idx">{pad(main.length + i + 1)}</span>{it.label}
    </button>
  {/each}

  <div class="spacer"></div>
  <span class="chip ok" data-tip="Wishtory only reads files the game already wrote. It never injects, edits game files or uses a proxy."><Icon name="shield" size={12} /> Read-only</span>
  <button class="nav theme" onclick={() => setTheme(app.theme === 'dark' ? 'light' : 'dark')}>
    <span class="idx"><Icon name={app.theme === 'dark' ? 'sun' : 'moon'} size={15} /></span>{app.theme === 'dark' ? 'Light theme' : 'Dark theme'}
  </button>
</nav>
