<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import type { BannerPity } from '../types';

  let { p }: { p: BannerPity } = $props();
  const R = 44;
  const C = 2 * Math.PI * R;
  const progress = new Tween(0, { duration: 800, easing: cubicOut });
  $effect(() => {
    progress.target = Math.min(p.pity / p.hard_pity, 1);
  });
  const inSoft = $derived(p.soft_pity !== null && p.pity >= p.soft_pity);
</script>

<div class="card pity" class:soft={inSoft}>
  <svg class="ring" viewBox="0 0 100 100" role="img" aria-label="{p.pity} pulls since last 5 star">
    <circle class="track" cx="50" cy="50" r={R} />
    <circle class="bar" cx="50" cy="50" r={R} stroke-dasharray={C} stroke-dashoffset={C * (1 - progress.current)} transform="rotate(-90 50 50)" />
    <text class="num" x="50" y="59" text-anchor="middle">{p.pity}</text>
  </svg>
  <div>
    <h3>{p.label}</h3>
    <p class="muted">{p.total} pulls · {p.five_stars}× 5★ · {p.four_stars}× 4★</p>
    <div class="row">
      {#if p.guaranteed !== null}
        <span class="chip" class:gold={p.guaranteed} data-tip={p.guaranteed ? 'Your next 5★ is guaranteed to be the featured one' : 'Next 5★ is a 50/50'}>
          {p.guaranteed ? 'Guaranteed' : '50/50'}
        </span>
      {/if}
      {#if inSoft}<span class="chip gold" data-tip="Soft pity: the 5★ rate is boosted">Soft pity</span>{/if}
      {#if p.avg_five_pity !== null}<span class="chip">avg {p.avg_five_pity.toFixed(1)}</span>{/if}
    </div>
  </div>
</div>
