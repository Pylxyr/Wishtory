<script lang="ts">
  import { app, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { BANNERS, GACHA_LABELS, type Pull } from '../lib/types';
  import GameBar from '../lib/components/GameBar.svelte';
  import EmptyState from '../lib/components/EmptyState.svelte';

  let pulls = $state<Pull[]>([]);
  let banner = $state('');
  let query = $state('');
  let rarity = $state(0); // 0 = all; 5 = 5★ / S rank; 4 = 4★ / A rank
  let limit = $state(200);
  let loading = $state(false);

  // Zenless Zone Zero reports ranks 2-4 (B/A/S); show them on the same 3-5 scale as the other games.
  const rankOf = (p: Pull) => (app.game === 'nap' && p.rank_type >= 2 && p.rank_type <= 4 ? p.rank_type + 1 : p.rank_type);
  const rarityLabel = (r: number) => (app.game === 'nap' ? (r === 5 ? 'S rank' : 'A rank') : `${r}★`);
  const matched = $derived(
    pulls.filter((p) => (!rarity || rankOf(p) === rarity) && (!query || p.name.toLowerCase().includes(query.toLowerCase())))
  );
  const shown = $derived(matched.slice(0, limit));

  $effect(() => {
    const game = app.game, uid = app.uid, b = banner;
    void app.refreshKey;
    if (b && !BANNERS[game].some((x) => x.key === b)) { banner = ''; return; }
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
    <select bind:value={rarity} aria-label="Rarity">
      <option value={0}>All rarities</option>
      <option value={5}>{rarityLabel(5)}</option>
      <option value={4}>{rarityLabel(4)}</option>
    </select>
    <input bind:value={query} placeholder="Search by name..." aria-label="Search" />
    <span class="muted">{matched.length} pulls</span>
  </div>
  <div class="card" style="padding:4px 8px">
    <table>
      <thead><tr><th>Time</th><th>Item</th><th>Type</th><th>Rarity</th><th>Banner</th></tr></thead>
      <tbody>
        {#each shown as p, i (p.id)}
          {@const rank = rankOf(p)}
          <tr class="rise" style="animation-delay:{Math.min(i, 20) * 18}ms">
            <td class="muted">{p.time}</td>
            <td class={`r${rank}`}>{p.name}</td>
            <td class="muted">{p.item_type}</td>
            <td class={`r${rank}`}>{'★'.repeat(rank)}</td>
            <td class="muted">{GACHA_LABELS[app.game][p.gacha_type] ?? p.gacha_type}</td>
          </tr>
        {:else}
          <tr><td colspan="5" class="muted">{loading ? 'Loading...' : 'No matching pulls.'}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if matched.length > limit}
    <p style="text-align:center;margin-top:12px"><button class="btn" onclick={() => (limit += 300)}>Show more</button></p>
  {/if}
{/if}
