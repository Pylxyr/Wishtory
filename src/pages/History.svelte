<script lang="ts">
  import { app, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { BANNERS, GACHA_LABELS, type Pull } from '../lib/types';
  import GameBar from '../lib/components/GameBar.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';

  let pulls = $state<Pull[]>([]);
  let banner = $state('');
  let query = $state('');
  let limit = $state(200);
  let loading = $state(false);

  const shown = $derived(
    pulls.filter((p) => !query || p.name.toLowerCase().includes(query.toLowerCase())).slice(0, limit)
  );

  $effect(() => {
    const game = app.game, uid = app.uid, b = banner;
    void app.refreshKey;
    if (b && !BANNERS[game].some((x) => x.key === b)) { banner = ''; return; }
    limit = 200;
    if (!uid) { pulls = []; return; }
    loading = true;
    let stale = false;
    api.getHistory(game, uid, b || null)
      .then((p) => { if (!stale) pulls = p; })
      .catch((e) => { if (!stale) toast('err', errMsg(e)); })
      .finally(() => { if (!stale) loading = false; });
    return () => { stale = true; };
  });
</script>

<h1>History</h1>
<p class="muted">Newest first.</p>
<GameBar />

{#if !app.uid}
  <EmptyState title="Nothing to show yet" text="Open the history page in-game, then click Refresh on the Dashboard." />
{:else}
  <div class="row" style="margin-bottom:12px">
    <select bind:value={banner} aria-label="Banner">
      <option value="">All banners</option>
      {#each BANNERS[app.game] as b}<option value={b.key}>{b.label}</option>{/each}
    </select>
    <input bind:value={query} placeholder="Search by name..." aria-label="Search" />
    <span class="muted">{pulls.length} pulls</span>
  </div>
  <div class="card" style="padding:4px 8px">
    <table>
      <thead><tr><th>Time</th><th>Item</th><th>Type</th><th>Rarity</th><th>Banner</th></tr></thead>
      <tbody>
        {#each shown as p, i (p.id)}
          <tr class="rise" style="animation-delay:{Math.min(i, 20) * 18}ms">
            <td class="muted">{p.time}</td>
            <td class={`r${p.rank_type}`}>{p.name}</td>
            <td class="muted">{p.item_type}</td>
            <td class={`r${p.rank_type}`}>{'★'.repeat(p.rank_type)}</td>
            <td class="muted">{GACHA_LABELS[app.game][p.gacha_type] ?? p.gacha_type}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="muted">{loading ? 'Loading...' : 'No matching pulls.'}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if pulls.length > limit}
    <p style="text-align:center;margin-top:12px"><button class="btn" onclick={() => (limit += 300)}>Show more</button></p>
  {/if}
{/if}
