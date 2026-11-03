<script lang="ts">
  import { app, accountLabel, lastRefreshOf, refreshFromGame, resyncFromGame } from '../state.svelte';
  import { GAMES } from '../types';
  import { ago } from '../format';
  import { tip } from '../tip';
  import Icon from './Icon.svelte';

  // "Updated 5 min ago" ticks along while the page is open.
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(t);
  });

  const mine = $derived(app.accounts.filter((a) => a.game === app.game));
  const install = $derived(app.installs.find((i) => i.game === app.game));
  const info = $derived(GAMES[app.game]);
  const last = $derived(app.uid ? lastRefreshOf(app.game, app.uid) : null);
  const working = $derived(app.busy && app.runGame === app.game);
</script>

<section class="pagebar" class:is-busy={app.busy} aria-label="Game and account">
  <div class="pagebar-id">
    <span class="pagebar-game">{info.name}</span>
    {#if mine.length}
      <label class="sr" for="account-select">Account</label>
      <select id="account-select" bind:value={app.uid} onchange={() => app.uid && (app.lastUid[app.game] = app.uid)}>
        {#each mine as a}
          {@const label = accountLabel(a)}
          <option value={a.uid}>UID {a.uid}{label ? ` · ${label}` : ''} ({a.pull_count.toLocaleString()} pulls)</option>
        {/each}
      </select>
    {/if}
  </div>
  <div class="pagebar-meta">
    {#if last}
      <span class="updated" use:tip={`Wishtory last downloaded this account on ${new Date(last).toLocaleString()}. Tracked on this PC only.`}><Icon name="clock" size={13} /> Updated {ago(last, now)}</span>
    {/if}
    {#if install?.found}
      <span class="chip ok">{info.short} detected</span>
    {:else}
      <span class="chip" use:tip={'Launch the game once so it creates its log file.'}>{info.short} not detected yet</span>
    {/if}
  </div>
  <div class="pagebar-actions">
    <button class="btn" onclick={resyncFromGame} disabled={app.busy} use:tip={'Downloads everything the server still keeps, not only what is new. Use it if pulls look missing.'}>
      Re-sync all
    </button>
    <button class="btn primary" onclick={refreshFromGame} disabled={app.busy} use:tip={"Reads the game's cache (read-only), then downloads what is new in your history."}>
      <span class:spin={working}><Icon name="refresh" size={16} /></span>{working ? 'Working…' : `Refresh ${info.short}`}
    </button>
  </div>
  <div class="busytrack" aria-hidden="true"><div class="busybar"></div></div>
</section>
