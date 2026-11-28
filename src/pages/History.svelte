<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { groupDigits, plural, rankLabel } from '../lib/gameinfo';
  import type { HistoryRank } from '../lib/prefs';
  import type { HistoryPull } from '../lib/types';
  import PullIcon from '../components/PullIcon.svelte';
  import RefreshPanel from '../components/RefreshPanel.svelte';

  const PAGE = 150;
  const MORE = 300;
  const TIMELINE_MAX = 400;

  let banner = $state<string | null>(null);
  let query = $state('');
  let rows = $state<HistoryPull[]>([]);
  let loading = $state(false);
  let limit = $state(PAGE);
  let token = 0;

  const filters = $derived(app.meta.find((m) => m.game === app.game)?.banners ?? []);
  const rankChoices = $derived<[HistoryRank, string][]>([
    ['five', app.info.r5],
    ['four', `${app.info.r4} +`],
    ['all', 'All pulls']
  ]);
  const rankMeaning = $derived(
    app.prefs.historyRank === 'five' ? `${app.info.r5} pulls` : app.prefs.historyRank === 'four' ? `${app.info.r5} and ${app.info.r4} pulls` : 'pulls'
  );
  // The backend leaves the lower ranks out, so a short list is all that crosses over.
  const minRank = $derived(app.prefs.historyRank === 'five' ? 5 : app.prefs.historyRank === 'four' ? 4 : null);

  $effect(() => {
    const acc = app.account;
    const b = banner;
    const min = minRank;
    const my = ++token;
    limit = PAGE;
    if (!acc) {
      rows = [];
      loading = false;
      return;
    }
    loading = true;
    api
      .getHistory(acc.game, acc.uid, b, min)
      .then((r) => {
        if (my === token) rows = r;
      })
      .catch((e) => app.toast(asError(e).message, 'err'))
      .finally(() => {
        if (my === token) loading = false;
      });
  });
  // Another game has other banners, and a name typed for one is no use in the next.
  $effect(() => {
    void app.game;
    banner = null;
    query = '';
  });

  const q = $derived(query.trim().toLowerCase());
  const shown = $derived.by(() => {
    const r = q ? rows.filter((p) => p.name.toLowerCase().includes(q)) : rows;
    return app.prefs.newestFirst ? r : [...r].reverse(); // the backend sends newest first
  });

  // Totals come from the pity summary, which counts every pull whatever the table is showing.
  const totals = $derived.by(() => {
    const list = banner ? app.pity.filter((p) => p.banner === banner) : app.pity;
    return list.reduce((t, p) => ({ pulls: t.pulls + p.total, fives: t.fives + p.five_stars, fours: t.fours + p.four_stars }), { pulls: 0, fives: 0, fours: 0 });
  });

  type Item = { kind: 'day'; day: string; n: number } | { kind: 'pull'; p: HistoryPull };
  const items = $derived.by<Item[]>(() => {
    const visible = shown.slice(0, limit);
    if (!app.prefs.groupByDay) return visible.map((p) => ({ kind: 'pull', p }));
    const perDay = new Map<string, number>();
    for (const p of shown) perDay.set(p.time.slice(0, 10), (perDay.get(p.time.slice(0, 10)) ?? 0) + 1);
    const out: Item[] = [];
    let last = '';
    for (const p of visible) {
      const day = p.time.slice(0, 10);
      if (day !== last) {
        out.push({ kind: 'day', day, n: perDay.get(day) ?? 0 });
        last = day;
      }
      out.push({ kind: 'pull', p });
    }
    return out;
  });

  // Every top-rank pull as a dot along the time it was made, so the gaps between them show at a glance.
  const stamp = (t: string) => Date.parse(`${t.replace(' ', 'T')}Z`);
  const timeline = $derived.by(() => {
    const tops = rows.filter((p) => p.rank === 5).slice(0, TIMELINE_MAX);
    if (tops.length < 2) return null;
    const times = tops.map((p) => stamp(p.time));
    const lo = Math.min(...times);
    const hi = Math.max(...times);
    const span = Math.max(1, hi - lo);
    const day = (t: number) => new Date(t).toISOString().slice(0, 10);
    return { from: day(lo), to: day(hi), dots: tops.map((p, i) => ({ p, x: (times[i] - lo) / span })) };
  });

  // How far into the pity count a pull was, as a bar: out of 90 for the top rank, out of 10 for the next.
  function fill(p: HistoryPull): { pct: number; soft: boolean } | null {
    if (p.pity == null || p.rank < 4) return null;
    const of = p.rank === 5 ? 90 : 10;
    return { pct: Math.min(100, (p.pity / of) * 100), soft: p.rank === 5 && p.pity >= 74 };
  }

  const footer = $derived(
    [
      `Showing ${groupDigits(shown.length)} of ${plural(totals.pulls, 'pull')} saved`,
      app.prefs.historyRank === 'all' ? null : `${rankMeaning} only`,
      `${totals.fives} × ${app.info.r5}, ${totals.fours} × ${app.info.r4}`
    ]
      .filter(Boolean)
      .join(' · ')
  );

  async function copyCsv() {
    const esc = (v: string | number) => {
      const s = String(v);
      return /[",\n]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s;
    };
    const lines = ['time,name,rank,pity,banner', ...shown.map((p) => [p.time, p.name, rankLabel(app.game, p.rank), p.pity ?? '', p.banner_name ?? p.pool].map(esc).join(','))];
    try {
      await navigator.clipboard.writeText(lines.join('\n'));
      app.toast(`Copied ${plural(shown.length, 'pull')} as CSV.`);
    } catch {
      app.toast('The clipboard is not available here.', 'err');
    }
  }
</script>

<div class="page history">
  <RefreshPanel />

  <div class="row wrap">
    <div class="tabs" role="group" aria-label="Which pulls to list">
      {#each rankChoices as [key, label] (key)}
        <button class:on={app.prefs.historyRank === key} aria-pressed={app.prefs.historyRank === key} onclick={() => app.setPrefs({ historyRank: key })}>{label}</button>
      {/each}
    </div>
    <span style="flex:1"></span>
    <input class="field" type="search" placeholder="Search name…" bind:value={query} aria-label="Search pulls" />
    <label class="row" style="gap:6px"><input type="checkbox" checked={app.prefs.groupByDay} onchange={(e) => app.setPrefs({ groupByDay: e.currentTarget.checked })} />Group by day</label>
    <button class="btn sm" disabled={!shown.length} onclick={copyCsv}>Copy CSV</button>
  </div>

  <div class="row wrap">
    <button class="chip" class:on={banner === null} aria-pressed={banner === null} onclick={() => (banner = null)}>All banners</button>
    {#each filters as f (f.key)}<button class="chip" class:on={banner === f.key} aria-pressed={banner === f.key} onclick={() => (banner = f.key)}>{f.label}</button>{/each}
  </div>

  {#if timeline}
    <!-- The inner box matters: some game skins restyle every direct child of a panel. -->
    <div class="panel timeline" role="group" aria-label="{app.info.r5} pulls over time">
      <div class="tl">
        <div class="track" aria-hidden="true"></div>
        {#each timeline.dots as d (d.p.id)}
          <button class="dot" style="--x:{d.x}" title="{d.p.name} · pity {d.p.pity ?? '?'} · {d.p.time.slice(0, 10)}" aria-label="{d.p.name}, {d.p.time.slice(0, 10)}" onclick={() => (query = d.p.name)}></button>
        {/each}
        <div class="ends num"><span>{timeline.from}</span><span>{timeline.dots.length} × {app.info.r5}</span><span>{timeline.to}</span></div>
      </div>
    </div>
  {/if}

  <div class="panel grow">
    {#if !app.account}
      <div class="empty pad"><h2>Nothing saved yet</h2><p class="muted">Refresh from the game or import a file first.</p></div>
    {:else}
      <div class="scroll">
        <table class="table">
          <thead>
            <tr>
              <th>Pull</th><th>Rank</th><th class="num">Pity</th><th>Banner</th>
              <th aria-sort={app.prefs.newestFirst ? 'descending' : 'ascending'}>
                <button class="thbtn" onclick={() => app.setPrefs({ newestFirst: !app.prefs.newestFirst })} title="Flip the order">Time (server) {app.prefs.newestFirst ? '↓' : '↑'}</button>
              </th>
            </tr>
          </thead>
          <tbody>
            {#each items as it (it.kind === 'day' ? `d${it.day}` : it.p.id)}
              {#if it.kind === 'day'}
                <tr class="day"><td colspan="5">{it.day} · {plural(it.n, 'pull')}</td></tr>
              {:else}
                {@const p = it.p}
                {@const bar = fill(p)}
                <tr class:r5={p.rank === 5} class:r4={p.rank === 4}>
                  <td><span class="nm">{#if app.prefs.pullView !== 'names'}<PullIcon name={p.name} rank={p.rank} />{/if}{#if app.prefs.pullView !== 'icons'}<span class:gold={p.rank === 5} class:purple={p.rank === 4}>{p.name}</span>{/if}</span></td>
                  <td><span class:gold={p.rank === 5} class:purple={p.rank === 4}>{rankLabel(app.game, p.rank)}</span></td>
                  <td class="num">{p.pity ?? ''}{#if bar}<span class="pbar" aria-hidden="true"><i class:soft={bar.soft} style="width:{bar.pct}%"></i></span>{/if}</td>
                  <td class="muted clip" title={p.banner_name ?? p.pool}>{p.banner_name ?? p.pool}</td>
                  <td class="muted num" style="text-align:left">{p.time}</td>
                </tr>
              {/if}
            {:else}
              <tr>
                <td colspan="5" class="muted" style="height:96px;text-align:center">
                  {#if loading}Loading…
                  {:else if q}No {rankMeaning} match “{query.trim()}”.{#if app.prefs.historyRank !== 'all'} <button class="linkbtn" onclick={() => app.setPrefs({ historyRank: 'all' })}>Search all pulls</button>{/if}
                  {:else}No {rankMeaning} here yet.{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if shown.length > limit}<div class="row" style="justify-content:center;padding:12px"><button class="btn sm" onclick={() => (limit += MORE)}>Show {Math.min(MORE, shown.length - limit)} more</button></div>{/if}
      </div>
    {/if}
  </div>

  <div class="muted" style="font-size:12px">{footer}</div>
</div>
