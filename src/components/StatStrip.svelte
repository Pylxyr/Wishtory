<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { countUp } from '../lib/motion';

  const total = $derived(app.pity.reduce((a, p) => a + p.total, 0));
  const fives = $derived(app.pity.reduce((a, p) => a + p.five_stars, 0));
  const avg = $derived.by(() => {
    const n = app.pity.reduce((a, p) => a + (p.avg_five_pity ? p.five_stars : 0), 0);
    return n ? app.pity.reduce((a, p) => a + (p.avg_five_pity ?? 0) * (p.avg_five_pity ? p.five_stars : 0), 0) / n : null;
  });
  const main = $derived(app.pity.find((p) => p.banner === 'character') ?? app.pity[0]);
  const figs = $derived([
    { label: 'Pulls', value: total, digits: 0 },
    { label: `${app.info.r5} pulls`, value: fives, digits: 0 },
    { label: `Average ${app.info.r5} pity`, value: avg, digits: 1 },
    { label: main ? main.label : 'Pity', value: main?.pity ?? null, digits: 0, of: main?.hard_pity }
  ]);
</script>

<div class="panel strip {app.game}">
  {#each figs as f, i (f.label)}
    <div class="fig">
      {#if app.game === 'star_rail'}<span class="n">0{i + 1}</span>{/if}
      <span class="label">{f.label}</span>
      <span class="v" class:gold={i === 1}>
        {#if f.value === null}–{:else}<span use:countUp={{ value: f.value, digits: f.digits }}>0</span>{/if}{#if f.of}<small class="muted" style="font-size:14px"> / {f.of}</small>{/if}
      </span>
    </div>
  {/each}
</div>
