<script lang="ts">
  import { app, refreshFromGame, fetchFromUrl, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type BannerPity } from '../lib/types';
  import GameBar from '../lib/components/GameBar.svelte';
  import PityRing from '../lib/components/PityRing.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import AnimatedNumber from '../lib/components/AnimatedNumber.svelte';

  let pity = $state<BannerPity[]>([]);
  let loading = $state(false);
  let pasted = $state('');

  const info = $derived(GAMES[app.game]);
  const install = $derived(app.installs.find((i) => i.game === app.game));
  const totalPulls = $derived(pity.reduce((s, b) => s + b.total, 0));
  const totalFives = $derived(pity.reduce((s, b) => s + b.five_stars, 0));

  $effect(() => {
    const game = app.game, uid = app.uid;
    void app.refreshKey;
    if (!uid) { pity = []; return; }
    loading = true;
    let stale = false; // ignore an answer that arrives after the selection changed
    api.getPity(game, uid)
      .then((p) => { if (!stale) pity = p; })
      .catch((e) => { if (!stale) toast('err', errMsg(e)); })
      .finally(() => { if (!stale) loading = false; });
    return () => { stale = true; };
  });
</script>

<h1>Dashboard</h1>
<p class="muted">Pity counters and statistics for your selected account.</p>
<GameBar />

<div class="card rise busy-host" class:is-busy={app.busy}>
  <div class="busybar" aria-hidden="true"></div>
  <div class="row">
    <button class="btn primary" onclick={refreshFromGame} disabled={app.busy} data-tip="Reads the game's cache (read-only), then downloads your history">
      <span class:spin={app.busy}><Icon name="refresh" /></span>{app.busy ? 'Working...' : `Refresh ${info.short}`}
    </button>
    {#if install?.found}
      <span class="chip ok">{info.name} detected</span>
    {:else}
      <span class="chip" data-tip="Launch the game once so it creates its log file">{info.name} not detected yet</span>
    {/if}
    <span class="muted">First open the history page in-game, then click Refresh.</span>
  </div>
  <details>
    <summary>Refresh not working? Paste a history link instead</summary>
    <div class="row" style="margin-top:10px">
      <input bind:value={pasted} placeholder="https://…authkey=…" aria-label="History URL" />
      <button class="btn" disabled={!pasted.trim() || app.busy} onclick={() => fetchFromUrl(pasted.trim())}><Icon name="link" />Fetch</button>
    </div>
  </details>
</div>

{#if !app.uid}
  <EmptyState title="No history yet" text="Let's pull in your first wishes. It takes two steps:">
    <div class="steps">
      <div class="step"><i>1</i><div><b>Open the history page in-game.</b><br /><span class="muted">{info.howTo}</span></div></div>
      <div class="step"><i>2</i><div><b>Click Refresh above.</b><br /><span class="muted">Wishtory reads the cache and does the rest.</span></div></div>
    </div>
  </EmptyState>
{:else}
  <div class="stats">
    <div class="card stat rise"><span class="muted">Total pulls</span><b><AnimatedNumber value={totalPulls} /></b></div>
    <div class="card stat rise" style="animation-delay:60ms"><span class="muted">Five-star items</span><b><AnimatedNumber value={totalFives} /></b></div>
    <div class="card stat rise" style="animation-delay:120ms"><span class="muted">Account</span><b style="font-size:18px">UID {app.uid}</b></div>
  </div>
  {#if loading && !pity.length}
    <div class="grid" aria-busy="true">{#each [0, 1, 2, 3] as n}<div class="card skeleton" style="animation-delay:{n * 80}ms"></div>{/each}</div>
  {/if}
  <div class="grid">
    {#each pity as p, i (p.banner)}
      <div class="rise" style="animation-delay:{i * 60}ms"><PityRing {p} /></div>
    {/each}
  </div>
{/if}
