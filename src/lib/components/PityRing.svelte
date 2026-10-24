<script lang="ts">
  import type { BannerPity } from '../types';
  import AnimatedNumber from './AnimatedNumber.svelte';
  import { elementTone } from '../elements';

  let { p, game, catalog }: { p: BannerPity; game: string; catalog: import('../types').GameCatalog | null } = $props();
  const inSoft = $derived(p.soft_pity !== null && p.pity >= p.soft_pity);
  const fivesLost = $derived(p.recent_fives.filter((pull) => pull.outcome === 'loss').length);
  const fivesWon = $derived(p.recent_fives.filter((pull) => pull.outcome === 'win').length);
  const fivesGuaranteed = $derived(p.recent_fives.filter((pull) => pull.outcome === 'guaranteed').length);
  const fiveRate = $derived(p.total ? (p.five_stars * 100 / p.total).toFixed(2) : '0.00');
  const fourRate = $derived(p.total ? (p.four_stars * 100 / p.total).toFixed(2) : '0.00');
</script>

<section class="card analysis-card" class:soft={inSoft}>
  <header class="analysis-head">
    <div>
      <span class="eyebrow">PULL BREAKDOWN</span>
      <h3>{p.label}</h3>
    </div>
    <div class="current-pity" aria-label="{p.pity} pulls toward a five-star, out of {p.hard_pity}">
      <span class="pity-track"><i style="width:{Math.min(p.pity / p.hard_pity * 100, 100)}%"></i></span>
      <strong><AnimatedNumber value={p.pity} /></strong><small> / {p.hard_pity} pity</small>
    </div>
  </header>

  <div class="analysis-table" role="table" aria-label="{p.label} pull breakdown">
    <div class="analysis-row analysis-labels" role="row"><span>Rarity</span><span>Total</span><span>Share</span><span>Avg pity</span></div>
    <div class="analysis-row rarity-five" role="row">
      <strong>5 <span aria-hidden="true">★</span></strong>
      <b><AnimatedNumber value={p.five_stars} /></b><span>{fiveRate}%</span>
      <span>{p.avg_five_pity?.toFixed(2) ?? '—'}</span>
    </div>
    <div class="analysis-row rarity-four" role="row">
      <strong>4 <span aria-hidden="true">★</span></strong>
      <b><AnimatedNumber value={p.four_stars} /></b><span>{fourRate}%</span>
      <span>{p.avg_four_pity?.toFixed(2) ?? '—'}</span>
    </div>
    <div class="analysis-total"><span>Total signals</span><strong><AnimatedNumber value={p.total} /></strong></div>
  </div>

  <div class="analysis-status">
    {#if p.banner === 'character'}
      {#if p.guaranteed === true}<span class="outcome-badge outcome-neutral">Next 5★ guaranteed</span>
      {:else if p.guaranteed === false}<span class="outcome-badge outcome-neutral">Next 5★: featured chance</span>
      {:else}<span class="outcome-badge outcome-neutral">Guarantee status unknown</span>{/if}
      <span class="status-note">{fivesWon} wins · {fivesLost} losses · {fivesGuaranteed} guaranteed</span>
    {:else}
      <span class="chip">Separate featured-rate rules apply</span>
    {/if}
    {#if inSoft}<span class="chip">Soft pity</span>{/if}
  </div>

  {#if p.recent_fives.length}
    <details class="pull-list" open>
      <summary><span>5★ pulls <span class="muted">· oldest → newest</span></span><b>{p.recent_fives.length}</b></summary>
      <div class="pull-chip-list">
        {#each p.recent_fives as pull, i (`${pull.time}:${pull.name}:${i}`)}
          {@const itemMeta = catalog?.items.find((item) => item.game === game && (item.name === pull.name || item.aliases.includes(pull.name)))}
          <span class="pull-chip five-chip element-{itemMeta?.traits.element ?? elementTone(pull.name)}" style="animation-delay:{Math.min(i, 12) * 25}ms">
            <span class="element-star" aria-hidden="true">✦</span>
            <strong>{pull.name || 'Five-star item'}</strong>
            {#if pull.outcome === 'loss'}<em class="outcome-badge outcome-loss" title="Off-banner five-star; the following five-star is guaranteed">Loss</em>
            {:else if pull.outcome === 'win'}<em class="outcome-badge outcome-win" title="Featured five-star won the featured chance">Win</em>
            {:else if pull.outcome === 'guaranteed'}<em class="outcome-badge outcome-next" title="Featured five-star obtained while guarantee was active">Guaranteed</em>{/if}
            <i>{pull.pity}</i>
          </span>
        {/each}
      </div>
    </details>
  {/if}
  {#if p.recent_fours.length}
    <details class="pull-list">
      <summary><span>4★ pulls</span><b>{p.recent_fours.length}</b></summary>
      <div class="pull-chip-list">
        {#each p.recent_fours as pull, i (`${pull.time}:${pull.name}:${i}`)}
          <span class="pull-chip four-chip" style="animation-delay:{Math.min(i, 12) * 20}ms"><strong>{pull.name || 'Four-star item'}</strong><i>{pull.pity}</i></span>
        {/each}
      </div>
    </details>
  {/if}
</section>
