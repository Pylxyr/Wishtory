<script lang="ts">
  import type { BannerPity } from '../lib/types';
  import { app } from '../lib/state.svelte';
  import { elementOf } from '../lib/icons';
  import PullIcon from './PullIcon.svelte';

  let { p }: { p: BannerPity } = $props();
  const outcome = { win: 'WON 50/50', loss: 'LOST 50/50', guaranteed: 'GUARANTEED' } as const;

  // The core returns every 5★ and 4★ of the banner, oldest first. Show the latest few by default.
  const FIVES_SHOWN = 12;
  const FOURS_SHOWN = 24;
  let allFives = $state(false);
  let allFours = $state(false);
  $effect(() => {
    void p.banner;
    allFives = false;
    allFours = false;
  });
  const fives = $derived([...p.recent_fives].reverse());
  const fours = $derived([...p.recent_fours].reverse());
  const shownFives = $derived(allFives ? fives : fives.slice(0, FIVES_SHOWN));
  const shownFours = $derived(allFours ? fours : fours.slice(0, FOURS_SHOWN));
</script>

<div class="panel detail">
  {#key p.banner}
    <div class="pop">
      <div class="row wrap" style="justify-content:space-between">
        <h2 style="margin:0">{p.label}</h2>
        <div class="stats">
          <div><span class="label">Total</span><b>{p.total}</b></div>
          <div><span class="label">{app.info.r5}</span><b class="gold">{p.five_stars}</b></div>
          <div><span class="label">{app.info.r4}</span><b class="purple">{p.four_stars}</b></div>
          <div><span class="label">Avg {app.info.r5} pity</span><b>{p.avg_five_pity?.toFixed(1) ?? '–'}</b></div>
          <div><span class="label">Avg {app.info.r4} pity</span><b>{p.avg_four_pity?.toFixed(1) ?? '–'}</b></div>
        </div>
      </div>
      <div style="margin:22px 4px 26px">
        <div class="softbar" style="--p:{Math.min(1, p.pity / p.hard_pity)}">
          <i class:soft={p.soft_pity != null && p.pity >= p.soft_pity}></i>
          {#if p.soft_pity}<em style="--at:{p.soft_pity / p.hard_pity}" data-t="soft {p.soft_pity}"></em>{/if}
        </div>
        <div class="row" style="justify-content:space-between;margin-top:6px"><span class="muted num">{p.pity} pulls since the last {app.info.r5}</span><span class="muted num">hard pity {p.hard_pity}</span></div>
      </div>
      <div class="row more-head">
        <h2>Recent {app.info.fives}</h2>
        {#if fives.length > FIVES_SHOWN}
          <button class="linkbtn" onclick={() => (allFives = !allFives)}>{allFives ? 'Show fewer' : `Show all ${fives.length}`}</button>
        {/if}
      </div>
      <div class="fives">
        {#each shownFives as f}
          <div class="five shine"><PullIcon name={f.name} rank={5} /><div><div style="font-weight:600">{f.name}</div><div class="row" style="gap:8px"><span class="p gold">{f.pity}</span>{#if f.outcome}<span class="res {f.outcome}">{outcome[f.outcome]}</span>{/if}</div></div></div>
        {:else}<span class="muted">None on this banner yet.</span>{/each}
      </div>
      {#if p.recent_fours.length}
        <div class="row more-head" style="margin-top:20px">
          <h2>Recent {app.info.r4}</h2>
          {#if fours.length > FOURS_SHOWN}
            <button class="linkbtn" onclick={() => (allFours = !allFours)}>{allFours ? 'Show fewer' : `Show all ${fours.length}`}</button>
          {/if}
        </div>
        <div class="fives">
          {#each shownFours as f}
            <span class="chip"><span class="purple num">{f.pity}</span>{f.name}{#if f.is_character}<small class="faint">{elementOf(f.name) !== 'neutral' ? elementOf(f.name) : ''}</small>{/if}</span>
          {/each}
        </div>
      {/if}
    </div>
  {/key}
</div>
