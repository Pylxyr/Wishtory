<script lang="ts">
  import { app, refreshFromGame, fetchFromUrl, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type BannerPity } from '../lib/types';
  import GameBar from '../lib/components/GameBar.svelte';
  import PityRing from '../lib/components/PityRing.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';
  import AnimatedNumber from '../lib/components/AnimatedNumber.svelte';
  import { elementTone } from '../lib/elements';

  let pity = $state<BannerPity[]>([]);
  let loading = $state(false);
  let pasted = $state('');

  const info = $derived(GAMES[app.game]);
  const install = $derived(app.installs.find((i) => i.game === app.game));
  const wishCurrency = $derived(({ hk4e: 'Primogems', hkrpg: 'Stellar Jade', nap: 'Polychromes' })[app.game]);
  const totalPulls = $derived(pity.reduce((s, b) => s + b.total, 0));
  const totalFives = $derived(pity.reduce((s, b) => s + b.five_stars, 0));
  const primogemEquivalent = $derived(totalPulls * 160);
  const recentFives = $derived(pity.flatMap((b) => b.recent_fives.map((pull) => ({ ...pull, banner: b.label })))
    .sort((a, b) => b.time.localeCompare(a.time)).slice(0, 8));

  $effect(() => {
    const game = app.game, uid = app.uid;
    void app.refreshKey;
    if (!uid) { pity = []; return; }
    pity = [];
    loading = true;
    let stale = false; // ignore an answer that arrives after the selection changed
    api.getPity(game, uid)
      .then((p) => { if (!stale) pity = p; })
      .catch((e) => { if (!stale) toast('err', errMsg(e)); })
      .finally(() => { if (!stale) loading = false; });
    return () => { stale = true; };
  });
</script>

<h1>Wish history</h1>
<p class="muted">A clear view of every pull, your five-star milestones, and current banner progress.</p>
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
  <section class="wish-overview rise" aria-label="Wish history overview">
    <div class="wish-overview-head">
      <div><span class="eyebrow">PULL SUMMARY</span><h2>Wish totals</h2></div>
      <span class="account-tag">UID {app.uid}</span>
    </div>
    <div class="wish-metrics">
      <div class="wish-metric"><span>Total pulls</span><strong><AnimatedNumber value={totalPulls} /></strong><small>Across all channels</small></div>
      <div class="wish-metric"><span>Five-star finds</span><strong class="gold-number"><AnimatedNumber value={totalFives} /></strong><small>Recorded in your history</small></div>
      <div class="wish-metric"><span>Pull value</span><strong><AnimatedNumber value={primogemEquivalent} /></strong><small>{wishCurrency} equivalent · 160 per pull</small></div>
    </div>
    {#if recentFives.length}
      <div class="five-star-strip">
        <div class="strip-heading"><span class="eyebrow">RECENT FIVE-STAR PULLS</span><span class="muted">Newest first</span></div>
        <div class="five-star-list">
          {#each recentFives as pull, i (`${pull.time}:${pull.name}:${i}`)}
            {@const itemMeta = app.catalog?.items.find((item) => item.game === app.game && (item.name === pull.name || item.aliases.includes(pull.name)))}
            <article class="five-star-item element-{itemMeta?.traits.element ?? elementTone(pull.name)}" style="animation-delay:{i * 45}ms">
              <span class="star-mark" aria-hidden="true">✦</span>
              <div class="five-star-copy"><strong>{pull.name || 'Five-star item'}</strong><span>{pull.banner}</span></div>
              {#if pull.outcome === 'loss'}<span class="outcome-badge outcome-loss">Loss</span>
              {:else if pull.outcome === 'win'}<span class="outcome-badge outcome-win">Win</span>
              {:else if pull.outcome === 'guaranteed'}<span class="outcome-badge outcome-next">Guaranteed</span>{/if}
              <div class="five-star-meta"><b>{pull.pity}</b><span>pulls</span></div>
            </article>
          {/each}
        </div>
      </div>
    {/if}
  </section>
  {#if loading && !pity.length}
    <div class="grid" aria-busy="true">{#each [0, 1, 2, 3] as n}<div class="card skeleton" style="animation-delay:{n * 80}ms"></div>{/each}</div>
  {/if}
  <div class="grid">
    {#each pity as p, i (p.banner)}
      <div class="rise" style="animation-delay:{i * 60}ms"><PityRing {p} game={app.game} catalog={app.catalog} /></div>
    {/each}
  </div>
  <p class="equivalent-note">The value treats every pull as 160 {wishCurrency.toLowerCase()}; it’s a comparison value, not a record of currency spent.</p>
{/if}
