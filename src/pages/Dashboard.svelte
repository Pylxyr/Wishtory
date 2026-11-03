<script lang="ts">
  import { app, fetchFromUrl } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type BannerPity } from '../lib/types';
  import { liveBanners, timeLeft } from '../lib/planning';
  import { tip } from '../lib/tip';
  import PityRing from '../lib/components/PityRing.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let pity = $state<BannerPity[]>([]);
  let loading = $state(false);
  let loadError = $state('');
  let reloads = $state(0); // bumped by "Try again"
  let loadedFor = ''; // which account `pity` belongs to
  let pasted = $state('');
  let selected = $state('');
  let now = $state(Date.now()); // ticks once a minute so the banner countdowns stay true
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(t);
  });

  const info = $derived(GAMES[app.game]);
  const account = $derived(app.accounts.find((a) => a.game === app.game && a.uid === app.uid));
  const totalPulls = $derived(pity.reduce((s, b) => s + b.total, 0));
  const totalFives = $derived(pity.reduce((s, b) => s + b.five_stars, 0));
  const avgFive = $derived(
    totalFives ? pity.reduce((s, b) => s + (b.avg_five_pity ?? 0) * b.five_stars, 0) / totalFives : null
  );
  const latestFive = $derived(
    pity.flatMap((b) => b.recent_fives).sort((a, b) => b.time.localeCompare(a.time))[0] ?? null
  );
  const current = $derived(pity.find((b) => b.banner === selected) ?? pity[0] ?? null);
  // Banners running right now, counted on the account's server clock (banner times are server-local).
  const live = $derived(liveBanners(app.banners, app.game, account?.timezone ?? 8, app.catalog, now));
  const poolLabel = (pool: string) => app.meta.find((m) => m.game === app.game)?.banners.find((b) => b.key === pool)?.label ?? pool;
  const tz = $derived(account ? `UTC${account.timezone >= 0 ? '+' : ''}${account.timezone}` : 'server');

  function rowNote(b: BannerPity): string {
    if (!b.total) return 'No pulls yet';
    if (b.guarantee_odds !== null) {
      if (b.guaranteed === true) return `Next ${info.r5} is guaranteed`;
      if (b.guaranteed === false) return `Not guaranteed: next ${info.r5} is a ${b.guarantee_odds}/${100 - b.guarantee_odds}`;
    }
    const summary = `${b.total} pulls, ${b.five_stars} ${b.five_stars === 1 ? info.five : info.fives}`;
    // Zenless rerun channels keep one count across every Rescreening / Reverberation period.
    return b.banner === 'rescreening' || b.banner === 'reverberation' ? `${summary}, all periods` : summary;
  }

  $effect(() => {
    const game = app.game, uid = app.uid;
    void app.refreshKey;
    void reloads;
    if (!uid) { pity = []; loadedFor = ''; loadError = ''; return; }
    const forAccount = `${game}:${uid}`;
    if (forAccount !== loadedFor) pity = []; // another account: never show the previous one's numbers
    // The same account after a refresh keeps its numbers on screen until the new ones arrive.
    loading = true;
    loadError = '';
    let stale = false; // ignore an answer that arrives after the selection changed
    api.getPity(game, uid)
      .then((p) => { if (!stale) { pity = p; loadedFor = forAccount; } })
      .catch((e) => { if (!stale) loadError = errMsg(e); })
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
      <div class="step"><i>2</i><div><b>Click Refresh at the top of the page.</b><br /><span class="muted">Wishtory reads the cache and does the rest.</span></div></div>
    </div>
  </EmptyState>
{:else}
  {#if loadError}
    <div class="notice err" role="alert">
      <span>Could not load the pity numbers: {loadError}</span>
      <button class="btn" onclick={() => reloads++}>Try again</button>
    </div>
  {:else if loading && !pity.length}
    <div class="overview" aria-busy="true"><div class="skeleton"></div><div class="skeleton"></div></div>
  {:else if current}
    <dl class="facts">
      <div class="fact"><dt>pulls</dt><dd>{totalPulls.toLocaleString()}</dd></div>
      <div class="fact gold"><dt>{info.fives}</dt><dd>{totalFives.toLocaleString()}</dd></div>
      <div class="fact"><dt>average {info.five} pity</dt><dd>{avgFive === null ? '—' : avgFive.toFixed(1)}</dd></div>
      {#if latestFive}
        <div class="fact"><dt>latest {info.five}, {latestFive.time.slice(0, 10)}</dt><dd class="small">{latestFive.name || `${info.five} item`}</dd></div>
      {/if}
      <div class="fact" use:tip={`Every pull counted as ${info.pullCost} ${info.currency.toLowerCase()}. A comparison value, not a record of currency spent.`}>
        <dt>{info.currency.toLowerCase()} worth of pulls</dt><dd>{(totalPulls * info.pullCost).toLocaleString()}</dd>
      </div>
    </dl>

    {#if live.length}
      <section class="live" aria-labelledby="live-title">
        <h2 id="live-title"><Icon name="calendar" size={15} /> Live now</h2>
        <ul class="live-list">
          {#each live as b (b.id)}
            <li class="live-item">
              <span class="live-name">{b.name}</span>
              <span class="muted live-featured">{poolLabel(b.pool)}{b.featured.length ? ` · ${b.featured.join(', ')}` : ''}</span>
              <span class="live-left tabnum" use:tip={`Counted on the ${tz} server clock of this account.`}><Icon name="clock" size={13} /> {timeLeft(b.msLeft)}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

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
    <span class="sr" role="status">{current.label}: {current.pity} of {current.hard_pity} pulls</span>
  {/if}

  <details class="manual">
    <summary>Refresh not working? Paste a history link instead</summary>
    <div class="row" style="margin-top:10px">
      <input bind:value={pasted} placeholder="https://…authkey=…" aria-label="History URL" />
      <button class="btn" disabled={!pasted.trim() || app.busy} onclick={() => fetchFromUrl(pasted.trim())}><Icon name="link" />Fetch</button>
    </div>
  </details>
{/if}
