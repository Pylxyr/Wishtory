<script lang="ts">
  import { app, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import type { Pull } from '../lib/types';
  import EmptyState from '../lib/components/EmptyState.svelte';

  let pulls = $state<Pull[]>([]);
  let banner = $state('');
  let query = $state('');
  let rarity = $state(0); // 0 = all; 5 = 5★ / S rank; 4 = 4★ / A rank
  let limit = $state(200);
  let loading = $state(false);

  const filters = $derived(app.meta.find((m) => m.game === app.game)?.banners ?? []);
  const rarityLabel = (r: number) => (app.game === 'nap' ? (r === 5 ? 'S rank' : 'A rank') : `${r}★`);
  const matched = $derived(
    pulls.filter((p) => (!rarity || p.rank === rarity) && (!query || p.name.toLowerCase().includes(query.toLowerCase())))
  );
  const shown = $derived(matched.slice(0, limit));

  $effect(() => {
    const game = app.game, uid = app.uid, b = banner;
    void app.refreshKey;
    if (b && filters.length && !filters.some((x) => x.key === b)) { banner = ''; return; }
    limit = 200;
    if (!uid) { pulls = []; return; }
    pulls = [];
    loading = true;
    let stale = false;
    api.getHistory(game, uid, b || null)
      .then((p) => { if (!stale) pulls = p; })
      .catch((e) => { if (!stale) toast('err', errMsg(e)); })
      .finally(() => { if (!stale) loading = false; });
    return () => { stale = true; };
  });
</script>

<div class="page-head">
  <h1>History</h1>
  <p>Every saved pull, newest first.</p>
</div>

{#if !app.uid}
  <EmptyState title="Nothing to show yet" text="Open the history page in-game, then click Refresh at the top of the window." />
{:else}
  <div class="toolbar">
    <select bind:value={banner} aria-label="Banner">
      <option value="">All banners</option>
      {#each filters as b}<option value={b.key}>{b.label}</option>{/each}
    </select>
    <div class="seg" role="group" aria-label="Rarity">
      <button aria-pressed={rarity === 0} onclick={() => (rarity = 0)}>All</button>
      <button aria-pressed={rarity === 5} onclick={() => (rarity = 5)}>{rarityLabel(5)}</button>
      <button aria-pressed={rarity === 4} onclick={() => (rarity = 4)}>{rarityLabel(4)}</button>
    </div>
    <input bind:value={query} placeholder="Search by name…" aria-label="Search" />
    <span class="count" aria-live="polite">{matched.length.toLocaleString()} pulls</span>
  </div>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Time</th><th>Item</th><th>Type</th><th>Rarity</th><th>Pity</th><th>Pool</th><th>Banner</th></tr></thead>
      <tbody>
        {#each shown as p (p.id)}
          {@const rank = p.rank}
          <tr class="row-r{rank}">
            <td>{p.time}</td>
            <td class="item r{rank}">{p.name}</td>
            <td class="muted">{p.item_type}</td>
            <td><span class="rank r{rank}"><span aria-hidden="true">{'★'.repeat(rank)}</span><span class="sr">{rank} star</span></span></td>
            <td class="pity r{rank}">{p.pity ?? ''}</td>
            <td class="muted">{p.pool}</td>
            <td class="muted">{p.banner_name ?? '—'}</td>
          </tr>
        {:else}
          <tr><td colspan="7" class="muted">{loading ? 'Loading…' : 'No matching pulls.'}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if matched.length > limit}
    <div class="more"><button class="btn" onclick={() => (limit += 300)}>Show more</button></div>
  {/if}
{/if}
