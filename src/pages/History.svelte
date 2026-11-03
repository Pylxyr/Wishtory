<script lang="ts">
  import { app, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, rankLabel, type Pull } from '../lib/types';
  import { plural } from '../lib/format';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';

  type SortKey = 'time' | 'name' | 'rank' | 'pity';
  type ColId = 'type' | 'rank' | 'pity' | 'pool' | 'banner';
  const COLS: { id: ColId; label: string; width?: string; sort?: SortKey }[] = [
    { id: 'type', label: 'Type', width: '96px' },
    { id: 'rank', label: 'Rarity', width: '92px', sort: 'rank' },
    { id: 'pity', label: 'Pity', width: '64px', sort: 'pity' },
    { id: 'pool', label: 'Pool', width: '120px' },
    { id: 'banner', label: 'Banner' }
  ];
  const PREF_KEY = 'wishtory.history';
  const saved = (() => { try { return JSON.parse(localStorage.getItem(PREF_KEY) ?? 'null') ?? {}; } catch { return {}; } })();

  let pulls = $state<Pull[]>([]);
  let loading = $state(false);
  let loadError = $state('');
  let reloads = $state(0);
  let loadedFor = '';

  let banner = $state('');
  let query = $state('');
  let rarity = $state(0); // 0 = all; 5 and 4 are the game's top two ranks
  let from = $state('');
  let to = $state('');
  let sort = $state<{ key: SortKey; dir: 'asc' | 'desc' }>({ key: 'time', dir: 'desc' });
  let density = $state<'compact' | 'comfortable'>(saved.density === 'comfortable' ? 'comfortable' : 'compact');
  let cols = $state<ColId[]>(Array.isArray(saved.cols) ? saved.cols.filter((c: string) => COLS.some((x) => x.id === c)) : COLS.map((c) => c.id));
  $effect(() => {
    try { localStorage.setItem(PREF_KEY, JSON.stringify({ density, cols })); } catch { /* nothing to remember */ }
  });

  const info = $derived(GAMES[app.game]);
  const account = $derived(app.accounts.find((a) => a.game === app.game && a.uid === app.uid));
  const tz = $derived(account ? `UTC${account.timezone >= 0 ? '+' : ''}${account.timezone}` : '');
  const filters = $derived(app.meta.find((m) => m.game === app.game)?.banners ?? []);
  const shownCols = $derived(COLS.filter((c) => cols.includes(c.id)));
  const filtered = $derived(!!(banner || query || rarity || from || to));

  const matched = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return pulls.filter(
      (p) => (!rarity || p.rank === rarity) && (!q || p.name.toLowerCase().includes(q)) && (!from || p.time.slice(0, 10) >= from) && (!to || p.time.slice(0, 10) <= to)
    );
  });
  const rows = $derived.by(() => {
    const { key, dir } = sort;
    if (key === 'time' && dir === 'desc') return matched; // the order the backend already uses
    const m = dir === 'asc' ? 1 : -1;
    const primary = (a: Pull, b: Pull) =>
      key === 'time' ? (a.time < b.time ? -1 : a.time > b.time ? 1 : 0)
      : key === 'name' ? a.name.localeCompare(b.name)
      : key === 'rank' ? a.rank - b.rank
      : (a.pity ?? -1) - (b.pity ?? -1);
    return [...matched].sort((a, b) => primary(a, b) * m || (a.time < b.time ? 1 : a.time > b.time ? -1 : 0));
  });

  // Only the rows near the viewport are drawn, so 50,000 pulls scroll as smoothly as 50.
  const rowH = $derived(density === 'compact' ? 32 : 42);
  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(600);
  const first = $derived(Math.max(0, Math.floor(scrollTop / rowH) - 8));
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / rowH) + 8));
  const visible = $derived(rows.slice(first, last));
  const padTop = $derived(first * rowH);
  const padBottom = $derived(Math.max(0, rows.length - last) * rowH);

  $effect(() => {
    void [banner, query, rarity, from, to, sort.key, sort.dir, density, app.game, app.uid];
    if (scroller) scroller.scrollTop = 0;
    scrollTop = 0;
  });

  $effect(() => {
    const game = app.game, uid = app.uid, b = banner;
    void app.refreshKey;
    void reloads;
    if (b && filters.length && !filters.some((x) => x.key === b)) { banner = ''; return; }
    if (!uid) { pulls = []; loadedFor = ''; loadError = ''; return; }
    const forList = `${game}:${uid}:${b}`;
    if (forList !== loadedFor) pulls = []; // a different list: never show the previous one
    // The same list after a refresh stays on screen until the new rows arrive.
    loading = true;
    loadError = '';
    let stale = false;
    api.getHistory(game, uid, b || null)
      .then((p) => { if (!stale) { pulls = p; loadedFor = forList; } })
      .catch((e) => { if (!stale) loadError = errMsg(e); })
      .finally(() => { if (!stale) loading = false; });
    return () => { stale = true; };
  });

  function sortBy(key: SortKey) {
    sort = sort.key === key ? { key, dir: sort.dir === 'asc' ? 'desc' : 'asc' } : { key, dir: key === 'name' ? 'asc' : 'desc' };
  }
  const ariaSort = (key?: SortKey) => (!key || sort.key !== key ? undefined : sort.dir === 'asc' ? 'ascending' : 'descending');
  const arrow = (key: SortKey) => (sort.key === key ? (sort.dir === 'asc' ? '▲' : '▼') : '▲▼');
  function clearFilters() { banner = ''; query = ''; rarity = 0; from = ''; to = ''; }
  function toggleCol(id: ColId) { cols = cols.includes(id) ? cols.filter((c) => c !== id) : COLS.map((c) => c.id).filter((c) => c === id || cols.includes(c)); }

  // "/" jumps to the search box, like in most data tools.
  let search: HTMLInputElement | undefined = $state();
  let menuOpen = $state(false);
  let menuEl: HTMLDivElement | undefined = $state();
  let menuBtn: HTMLButtonElement | undefined = $state();
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && menuOpen) {
      menuOpen = false;
      menuBtn?.focus();
      return;
    }
    if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !(e.target as HTMLElement).closest('input, textarea, select, [contenteditable]')) {
      e.preventDefault();
      search?.focus();
      search?.select();
    }
  }
  function onWindowClick(e: MouseEvent) {
    if (menuOpen && menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
  }

  // A spreadsheet-safe CSV of what the table currently shows (filters and sort applied).
  const csvCell = (v: string | number | null) => {
    let s = v === null ? '' : String(v);
    if (/^[=+\-@\t\r]/.test(s)) s = `'${s}`; // stops a spreadsheet reading an item name as a formula
    return /[",\n\r]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s;
  };
  async function copyCsv() {
    const lines = [['Time', 'Item', 'Type', 'Rarity', 'Pity', 'Pool', 'Banner'].join(',')];
    for (const p of rows) lines.push([p.time, p.name, p.item_type, rankLabel(app.game, p.rank), p.pity, p.pool, p.banner_name].map(csvCell).join(','));
    try {
      await navigator.clipboard.writeText(lines.join('\n'));
      toast('ok', `Copied ${plural(rows.length, 'row')} as CSV.`);
    } catch {
      toast('warn', 'Could not copy to the clipboard.');
    }
  }
</script>

<svelte:window onkeydown={onKey} onclick={onWindowClick} />

<div class="page-head">
  <h1>History</h1>
  <p>Every saved pull. Press / to search.</p>
</div>

{#if !app.uid}
  <EmptyState title="Nothing to show yet" text="Open the history page in-game, then click Refresh at the top of the page." />
{:else}
  <div class="toolbar">
    <select bind:value={banner} aria-label="Banner">
      <option value="">All banners</option>
      {#each filters as b}<option value={b.key}>{b.label}</option>{/each}
    </select>
    <div class="seg" role="group" aria-label="Rarity">
      <button aria-pressed={rarity === 0} onclick={() => (rarity = 0)}>All</button>
      <button aria-pressed={rarity === 5} onclick={() => (rarity = 5)}>{info.r5}</button>
      <button aria-pressed={rarity === 4} onclick={() => (rarity = 4)}>{info.r4}</button>
    </div>
    <input type="search" bind:this={search} bind:value={query} placeholder="Search by name…" aria-label="Search by name" />
    <label class="date"><span class="sr">From date</span><input type="date" bind:value={from} aria-label="From date" title="From date" /></label>
    <label class="date"><span class="sr">To date</span><input type="date" bind:value={to} aria-label="To date" title="To date" /></label>
    {#if filtered}<button class="btn" onclick={clearFilters}>Clear filters</button>{/if}
    <div class="toolbar-end">
      <span class="count" aria-live="polite">{plural(rows.length, 'pull')}{rows.length !== pulls.length ? ` of ${pulls.length.toLocaleString()}` : ''}</span>
      <div class="menu" bind:this={menuEl}>
        <button class="btn" bind:this={menuBtn} aria-expanded={menuOpen} aria-controls="history-view" onclick={() => (menuOpen = !menuOpen)}><Icon name="sliders" size={16} />View</button>
        {#if menuOpen}
          <div class="menu-pop" id="history-view" role="group" aria-label="View options">
            <fieldset>
              <legend>Columns</legend>
              {#each COLS as c}<label class="check"><input type="checkbox" checked={cols.includes(c.id)} onchange={() => toggleCol(c.id)} />{c.label}</label>{/each}
            </fieldset>
            <fieldset>
              <legend>Row height</legend>
              <label class="check"><input type="radio" name="density" value="compact" bind:group={density} />Compact</label>
              <label class="check"><input type="radio" name="density" value="comfortable" bind:group={density} />Comfortable</label>
            </fieldset>
          </div>
        {/if}
      </div>
      <button class="btn" onclick={copyCsv} disabled={!rows.length}><Icon name="copy" size={16} />Copy CSV</button>
    </div>
  </div>

  {#if loadError}
    <div class="notice err" role="alert">
      <span>Could not load the history: {loadError}</span>
      <button class="btn" onclick={() => reloads++}>Try again</button>
    </div>
  {:else}
    <!-- A scrollable region has to be reachable by keyboard so arrow keys and Page Up/Down can scroll it. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="history-scroll" bind:this={scroller} bind:clientHeight={viewH} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)} role="region" aria-label="Pull history" tabindex="0">
      <table class="history" style="--row-h:{rowH}px" aria-rowcount={rows.length + 1}>
        <colgroup>
          <col style="width:180px" /><col />
          {#each shownCols as c}<col style={c.width ? `width:${c.width}` : ''} />{/each}
        </colgroup>
        <thead>
          <tr aria-rowindex="1">
            <th scope="col" aria-sort={ariaSort('time')}><button class="th-sort" onclick={() => sortBy('time')}>Time{tz ? ` (${tz})` : ''} <span class="arrow" aria-hidden="true">{arrow('time')}</span></button></th>
            <th scope="col" aria-sort={ariaSort('name')}><button class="th-sort" onclick={() => sortBy('name')}>Item <span class="arrow" aria-hidden="true">{arrow('name')}</span></button></th>
            {#each shownCols as c}
              <th scope="col" aria-sort={ariaSort(c.sort)}>
                {#if c.sort}<button class="th-sort" onclick={() => sortBy(c.sort!)}>{c.label} <span class="arrow" aria-hidden="true">{arrow(c.sort)}</span></button>{:else}{c.label}{/if}
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#if padTop}<tr class="pad" aria-hidden="true" style="height:{padTop}px"><td colspan={2 + shownCols.length}></td></tr>{/if}
          {#each visible as p, i (p.id)}
            <tr class="row-r{p.rank}" aria-rowindex={first + i + 2}>
              <td>{p.time}</td>
              <td class="item rk{p.rank}" title={p.name}>{p.name}</td>
              {#each shownCols as c}
                {#if c.id === 'type'}<td class="muted">{p.item_type}</td>
                {:else if c.id === 'rank'}<td><span class="rank rk{p.rank}"><span aria-hidden="true">{'★'.repeat(p.rank)}</span><span class="sr">{rankLabel(app.game, p.rank)}</span></span></td>
                {:else if c.id === 'pity'}<td class="pity rk{p.rank}">{p.pity ?? '—'}</td>
                {:else if c.id === 'pool'}<td class="muted">{p.pool}</td>
                {:else}<td class="muted" title={p.banner_name ?? ''}>{p.banner_name ?? '—'}</td>{/if}
              {/each}
            </tr>
          {/each}
          {#if padBottom}<tr class="pad" aria-hidden="true" style="height:{padBottom}px"><td colspan={2 + shownCols.length}></td></tr>{/if}
        </tbody>
      </table>
      {#if !rows.length}
        <div class="table-empty">
          {#if loading}Loading…
          {:else if filtered}No pulls match these filters. <button class="link" onclick={clearFilters}>Clear filters</button>
          {:else}No pulls saved for this account yet.{/if}
        </div>
      {/if}
    </div>
  {/if}
{/if}
