<script lang="ts">
  import { app, fetchFromUrl, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type BannerPity } from '../lib/types';
  import PityRing from '../lib/components/PityRing.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let pity = $state<BannerPity[]>([]);
  let loading = $state(false);
  let pasted = $state('');
  let selected = $state('');

  const info = $derived(GAMES[app.game]);
  const wishCurrency = $derived(({ hk4e: 'Primogems', hkrpg: 'Stellar Jade', nap: 'Polychromes' })[app.game]);
  const totalPulls = $derived(pity.reduce((s, b) => s + b.total, 0));
  const totalFives = $derived(pity.reduce((s, b) => s + b.five_stars, 0));
  const avgFive = $derived(
    totalFives ? pity.reduce((s, b) => s + (b.avg_five_pity ?? 0) * b.five_stars, 0) / totalFives : null
  );
  const latestFive = $derived(
    pity.flatMap((b) => b.recent_fives).sort((a, b) => b.time.localeCompare(a.time))[0] ?? null
  );
  const current = $derived(pity.find((b) => b.banner === selected) ?? pity[0] ?? null);

  function rowNote(b: BannerPity): string {
    if (!b.total) return 'No pulls yet';
    if (b.guarantee_odds !== null) {
      if (b.guaranteed === true) return 'Next 5★ is guaranteed';
      if (b.guaranteed === false) return `Not guaranteed: next 5★ is a ${b.guarantee_odds}/${100 - b.guarantee_odds}`;
    }
    return `${b.total} pulls, ${b.five_stars} five-star${b.five_stars === 1 ? '' : 's'}`;
  }

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

<div class="page-head">
  <h1>Overview</h1>
  <p>Pity and guarantee status for every banner.</p>
</div>

{#if !app.uid}
  <EmptyState title="No history yet" text="Let's pull in your first wishes. It takes two steps:">
    <div class="steps">
      <div class="step"><i>1</i><div><b>Open the history page in-game.</b><br /><span class="muted">{info.howTo}</span></div></div>
      <div class="step"><i>2</i><div><b>Click Refresh at the top of the window.</b><br /><span class="muted">Wishtory reads the cache and does the rest.</span></div></div>
    </div>
  </EmptyState>
{:else}
  <dl class="facts">
    <div class="fact"><dt>pulls</dt><dd>{totalPulls.toLocaleString()}</dd></div>
    <div class="fact gold"><dt>five-stars</dt><dd>{totalFives.toLocaleString()}</dd></div>
    <div class="fact"><dt>average five-star pity</dt><dd>{avgFive === null ? '—' : avgFive.toFixed(1)}</dd></div>
    {#if latestFive}
      <div class="fact"><dt>latest five-star, {latestFive.time.slice(0, 10)}</dt><dd class="small">{latestFive.name || 'Five-star item'}</dd></div>
    {/if}
    <div class="fact" title="Every pull counted as 160 {wishCurrency.toLowerCase()}. A comparison value, not a record of currency spent.">
      <dt>{wishCurrency.toLowerCase()} worth of pulls</dt><dd>{(totalPulls * 160).toLocaleString()}</dd>
    </div>
  </dl>

  {#if loading && !pity.length}
    <div class="overview" aria-busy="true"><div class="skeleton"></div><div class="skeleton"></div></div>
  {:else if current}
    <div class="overview">
      <nav class="banner-list" aria-label="Banners">
        {#each pity as b (b.banner)}
          {@const soft = b.soft_pity !== null && b.pity >= b.soft_pity}
          <button class="banner-row" class:soft aria-current={current.banner === b.banner ? 'true' : undefined} onclick={() => (selected = b.banner)}>
            <span class="name">{b.label}</span>
            <span class="count">{b.pity}<small> / {b.hard_pity}</small></span>
            <span class="meter" aria-hidden="true"><i style="width:{Math.min((b.pity / b.hard_pity) * 100, 100)}%"></i></span>
            <span class="sub">{rowNote(b)}</span>
          </button>
        {/each}
      </nav>
      <PityRing p={current} game={app.game} catalog={app.catalog} />
    </div>
  {/if}

  <details class="manual">
    <summary>Refresh not working? Paste a history link instead</summary>
    <div class="row" style="margin-top:10px">
      <input bind:value={pasted} placeholder="https://…authkey=…" aria-label="History URL" />
      <button class="btn" disabled={!pasted.trim() || app.busy} onclick={() => fetchFromUrl(pasted.trim())}><Icon name="link" />Fetch</button>
    </div>
  </details>
{/if}
