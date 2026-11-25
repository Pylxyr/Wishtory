<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { liveCards, pityOf, type LiveCard } from '../lib/live';
  import { artUrls } from '../lib/icons';
  import Art from './Art.svelte';

  let { onSelect }: { onSelect: (key: string) => void } = $props();
  const cards = $derived(
    app.account ? liveCards(app.banners, app.game, app.account.timezone, (g, id) => app.index.byId.get(`${g}|${id}`), app.info.key) : []
  );
  // Balanced rows, so no row ends with a lone card: 4 cards are 2 + 2, not 3 + 1.
  const cols = $derived(Math.ceil(cards.length / Math.ceil(cards.length / 3)) || 1);
  const cols2 = $derived(Math.ceil(cards.length / Math.ceil(cards.length / 2)) || 1);
  const left = (c: LiveCard) => (c.days > 1 ? `${c.days} days left` : `${c.hours} h left`);
</script>

{#if cards.length}
  <section>
    <div class="live-head" role="heading" aria-level="2"><span class="pulse"></span><span class="label">{app.info.liveTitle}</span></div>
    <div class="live" style="--cols:{cols};--cols2:{cols2}">
      {#each cards as c, i (c.key)}
        {@const p = pityOf(app.pity, c.banner)}
        <button class="panel livecard pop lift" class:shine={p?.soft_pity != null && p.pity >= p.soft_pity} style="--i:{i}" onclick={() => p && onSelect(p.banner)}>
          <div class="art">
            {#each c.items.slice(0, 1) as it (it.id)}<Art urls={artUrls(it)} fallback={it.name.charAt(0)} />{/each}
            {#if !c.items.length}<span class="fallback">★</span>{/if}
          </div>
          <div class="body">
            <span class="label">{c.banner.pool === 'weapon' ? 'Weapon' : 'Character'} · {left(c)}</span>
            <span class="nm">{c.title}</span>
            {#if c.banner.pool === 'weapon'}<span class="muted">{c.items.map((x) => x.name).join(' · ')}</span>{:else if c.items[0]}<span class="muted">{c.items[0].name}</span>{/if}
            {#if p}
              <span class="pn num">{p.pity}<small> / {p.hard_pity}</small></span>
              <span class="row wrap" style="gap:6px">
                {#if p.guaranteed === true}<span class="tag gold">Guaranteed</span>{:else if p.guarantee_odds}<span class="tag">{p.guarantee_odds}% featured</span>{/if}
                {#if p.soft_pity != null && p.pity >= p.soft_pity}<span class="tag gold">Soft pity</span>{/if}
              </span>
            {/if}
          </div>
        </button>
      {/each}
    </div>
  </section>
{/if}
