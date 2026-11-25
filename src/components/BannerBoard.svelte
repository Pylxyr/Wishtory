<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { countUp } from '../lib/motion';
  import Ring from './Ring.svelte';
  import Glyph from './Glyph.svelte';

  let { selected, onSelect }: { selected: string; onSelect: (key: string) => void } = $props();
  const inSoft = (p: { pity: number; soft_pity: number | null }) => p.soft_pity != null && p.pity >= p.soft_pity;
  const idx = $derived(Math.max(0, app.pity.findIndex((p) => p.banner === selected)));
  const prog = $derived(app.pity.length > 1 ? (idx / (app.pity.length - 1)) * 100 : 0);
  // The train sits over the selected station: stations are equal columns inside the padded track.
  const trainX = $derived(app.pity.length ? ((idx + 0.5) / app.pity.length) * 100 : 6);
</script>

{#if app.game === 'genshin'}
  <div class="gcards">
    {#each app.pity as p, i (p.banner)}
      <button class="gcard pop" class:sel={selected === p.banner} style="--i:{i}" onclick={() => onSelect(p.banner)}>
        <Ring value={p.pity} max={p.hard_pity} soft={inSoft(p)} />
        <div>
          <div class="t">{p.label}</div>
          <div class="muted">{p.total} pulls · {p.five_stars}× 5★</div>
          <div class="chips">
            {#if p.guaranteed === true}<span class="tag gold"><Glyph name="sparkle" size={11} />Guaranteed</span>{:else if p.guarantee_odds}<span class="tag">{p.guarantee_odds}% featured</span>{/if}
            {#if inSoft(p)}<span class="tag gold">Soft pity</span>{:else if p.soft_pity}<span class="tag">Soft pity {p.soft_pity}</span>{/if}
          </div>
        </div>
      </button>
    {/each}
  </div>
{:else if app.game === 'star_rail'}
  <div class="panel route pad" style="--prog:{prog}%">
    <div class="line"></div>
    <div class="train" style="--x:{trainX}%"><Glyph name="train" size={38} /></div>
    <div class="stations" style="grid-template-columns:repeat({app.pity.length},1fr)">
      {#each app.pity as p, i (p.banner)}
        <button class="station pop" class:sel={selected === p.banner} style="--i:{i}" onclick={() => onSelect(p.banner)}>
          <span class="dot"></span>
          <div class="stn">STATION {String(i + 1).padStart(2, '0')}</div>
          <div class="t">{p.label}</div>
          <div class="big" class:gold={inSoft(p)} use:countUp={{ value: p.pity }}>0</div>
          <div class="muted" style="font-size:12px;margin-top:4px">of {p.hard_pity}{p.guaranteed === true ? ' · guaranteed' : p.guarantee_odds ? ` · ${p.guarantee_odds}%` : ''}</div>
          <div class="bar"><i class:soft={inSoft(p)} style="--p:{Math.min(1, p.pity / p.hard_pity)}"></i></div>
        </button>
      {/each}
    </div>
  </div>
{:else}
  <div class="channels">
    {#each app.pity as p, i (p.banner)}
      <button class="channel pop" class:sel={selected === p.banner} style="--i:{i}" onclick={() => onSelect(p.banner)}>
        <div><span class="rec">CH {String(i + 1).padStart(2, '0')}</span><div style="font-weight:800;text-transform:uppercase;font-style:italic;margin-top:4px">{p.label}</div></div>
        <div>
          <div class="bar"><i class:soft={inSoft(p)} style="--p:{Math.min(1, p.pity / p.hard_pity)}"></i></div>
          <div class="muted" style="margin-top:8px;font-size:12px">{p.total} pulls · {p.five_stars}× {app.info.r5}{p.guaranteed === true ? ' · GUARANTEED' : p.guarantee_odds ? ` · ${p.guarantee_odds}% featured` : ''}</div>
        </div>
        <div class="huge flicker" class:gold={inSoft(p) && selected !== p.banner} use:countUp={{ value: p.pity, ms: 700 }}>0</div>
      </button>
    {/each}
  </div>
{/if}
