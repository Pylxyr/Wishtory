<script lang="ts">
  import { app, openHelp, selectGame, type Page } from '../state.svelte';
  import { GAMES, type GameId } from '../types';
  import { tip } from '../tip';
  import Icon from './Icon.svelte';

  const gamePages: { id: Page; label: string; icon: string }[] = [
    { id: 'dashboard', label: 'Overview', icon: 'home' },
    { id: 'history', label: 'History', icon: 'list' },
    { id: 'importexport', label: 'Import / Export', icon: 'swap' }
  ];
  const appPages = $derived<{ id: Page; label: string; icon: string }[]>([
    { id: 'accounts', label: 'Accounts', icon: 'users' },
    { id: 'settings', label: 'Settings', icon: 'sliders' },
    { id: 'help', label: 'Help', icon: 'help' },
    { id: 'about', label: 'About', icon: 'info' },
    ...(app.loggingEnabled ? [{ id: 'logging' as Page, label: 'Logging', icon: 'terminal' }] : [])
  ]);
  const pullsOf = (g: GameId) => app.accounts.filter((a) => a.game === g).reduce((sum, a) => sum + a.pull_count, 0);
  const info = $derived(GAMES[app.game]);
</script>

<aside class="rail">
  <div class="brand">
    <svg class="brand-mark" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 1.5l2.4 8.1L22.5 12l-8.1 2.4L12 22.5l-2.4-8.1L1.5 12l8.1-2.4z" /></svg>
    <span>Wishtory</span>
  </div>

  <div class="rail-label" id="rail-games">Games</div>
  <div class="rail-group" role="group" aria-labelledby="rail-games">
    {#each Object.entries(GAMES) as [id, g]}
      {@const n = pullsOf(id as GameId)}
      <button class="rail-item" aria-pressed={app.game === id} aria-label="{g.name}, {n.toLocaleString()} pulls" title={g.name} onclick={() => selectGame(id as GameId)}>
        <span class="abbr" aria-hidden="true">{g.abbr}</span>
        <span class="rail-text" aria-hidden="true">{g.short}</span>
        <span class="rail-count" aria-hidden="true">{n.toLocaleString()}</span>
      </button>
    {/each}
  </div>

  <div class="rail-label" id="rail-game-pages">{info.short}</div>
  <nav class="rail-group" aria-labelledby="rail-game-pages">
    {#each gamePages as t}
      <button class="rail-item" aria-label={t.label} title={t.label} aria-current={app.page === t.id ? 'page' : undefined} onclick={() => (app.page = t.id)}>
        <Icon name={t.icon} size={17} /><span class="rail-text" aria-hidden="true">{t.label}</span>
      </button>
    {/each}
  </nav>

  <div class="rail-spacer"></div>

  <nav class="rail-group" aria-label="App">
    {#each appPages as t}
      {@const update = t.id === 'settings' && app.update?.available}
      <button class="rail-item" aria-label={update ? 'Settings (update available)' : t.label} title={update ? 'Settings (update available)' : t.label} aria-current={app.page === t.id ? 'page' : undefined} onclick={() => (app.page = t.id)}>
        <Icon name={t.icon} size={17} /><span class="rail-text" aria-hidden="true">{t.label}</span>
        {#if update}<span class="dot" aria-hidden="true"></span>{/if}
      </button>
    {/each}
  </nav>

  <div class="rail-foot">
    <button
      class="chip ok"
      aria-label="Read-only. Open the safety notes in Help."
      use:tip={{ text: 'Wishtory only reads files the game already wrote. It never injects, edits game files or uses a proxy. Select to read the safety notes.', placement: 'right' }}
      onclick={() => openHelp('help-safety')}
    >
      <Icon name="shield" size={13} /><span class="chip-text">Read-only</span>
    </button>
  </div>
</aside>
