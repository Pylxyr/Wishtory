<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { rankLabel } from '../lib/gameinfo';
  import type { HistoryPull } from '../lib/types';
  import PullIcon from '../components/PullIcon.svelte';
  import RefreshPanel from '../components/RefreshPanel.svelte';

  let banner = $state<string | null>(null);
  let query = $state('');
  let rows = $state<HistoryPull[]>([]);
  let loading = $state(false);
  let limit = $state(150);
  let token = 0;

  const filters = $derived(app.meta.find((m) => m.game === app.game)?.banners ?? []);

  $effect(() => {
    const acc = app.account;
    const b = banner;
    app.game; // reload when the game changes too
    const my = ++token;
    limit = 150;
    if (!acc) { rows = []; return; }
    loading = true;
    api.getHistory(acc.game, acc.uid, b).then((r) => { if (my === token) rows = r; }).catch((e) => app.toast(asError(e).message, 'err')).finally(() => { if (my === token) loading = false; });
  });
  $effect(() => { app.game; banner = null; });

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let r = rows.filter((p) => (app.prefs.showFours ? p.rank >= 4 : p.rank === 5) || (q.length > 0 && p.name.toLowerCase().includes(q)));
    if (q) r = r.filter((p) => p.name.toLowerCase().includes(q));
    return app.prefs.newestFirst ? r : [...r].reverse();
  });
</script>

<div class="page">
  <RefreshPanel />
  <div class="row wrap">
    <button class="chip" class:on={banner === null} onclick={() => (banner = null)}>All banners</button>
    {#each filters as f (f.key)}<button class="chip" class:on={banner === f.key} onclick={() => (banner = f.key)}>{f.label}</button>{/each}
    <span style="flex:1"></span>
    <input class="field" placeholder="Search name…" bind:value={query} aria-label="Search pulls" />
    <label class="row" style="gap:6px"><input type="checkbox" checked={app.prefs.showFours} onchange={(e) => app.setPrefs({ showFours: e.currentTarget.checked })} />Show {app.info.r4}</label>
  </div>
  <div class="panel">
    {#if !app.account}
      <div class="empty pad"><h2>Nothing saved yet</h2><p class="muted">Refresh from the game or import a file first.</p></div>
    {:else}
      <div class="scroll">
        <table class="table">
          <thead><tr><th>Pull</th><th>Rank</th><th class="num">Pity</th><th>Banner</th><th>Time (server)</th></tr></thead>
          <tbody>
            {#each shown.slice(0, limit) as p (p.id)}
              <tr class:r5={p.rank === 5} class:r4={p.rank === 4}>
                <td><span class="nm">{#if app.prefs.pullView !== 'names'}<PullIcon name={p.name} rank={p.rank} />{/if}{#if app.prefs.pullView !== 'icons'}<span class:gold={p.rank === 5} class:purple={p.rank === 4}>{p.name}</span>{/if}</span></td>
                <td><span class:gold={p.rank === 5} class:purple={p.rank === 4}>{rankLabel(app.game, p.rank)}</span></td>
                <td class="num">{p.pity ?? ''}</td>
                <td class="muted">{p.banner_name ?? p.pool}</td>
                <td class="muted num" style="text-align:left">{p.time}</td>
              </tr>
            {:else}
              <tr><td colspan="5" class="muted" style="height:80px;text-align:center">{loading ? 'Loading…' : 'No pulls match.'}</td></tr>
            {/each}
          </tbody>
        </table>
        {#if shown.length > limit}<div class="row" style="justify-content:center;padding:12px"><button class="btn sm" onclick={() => (limit += 300)}>Show {Math.min(300, shown.length - limit)} more</button></div>{/if}
      </div>
    {/if}
  </div>
  <div class="muted" style="font-size:12px">{shown.length} of {rows.length} pulls</div>
</div>
