<script lang="ts">
  import type { BannerPity, FiveStar, GameCatalog } from '../types';
  import { elementTone, iconUrl } from '../elements';

  let { p, game, catalog }: { p: BannerPity; game: string; catalog: GameCatalog | null } = $props();

  const inSoft = $derived(p.soft_pity !== null && p.pity >= p.soft_pity);
  const fiveRate = $derived(p.total ? ((p.five_stars * 100) / p.total).toFixed(2) : '0.00');
  const fourRate = $derived(p.total ? ((p.four_stars * 100) / p.total).toFixed(2) : '0.00');
  const fivesNewestFirst = $derived([...p.recent_fives].reverse());
  const foursNewestFirst = $derived([...p.recent_fours].reverse());
  const pct = (n: number) => Math.min((n / p.hard_pity) * 100, 100);
  // 50/50 on character banners, 75/25 on weapon banners; null when the banner tracks no result.
  const odds = $derived(p.guarantee_odds !== null ? `${p.guarantee_odds}/${100 - p.guarantee_odds}` : '');
  const outcomeLabel = (o: FiveStar['outcome']) =>
    o === 'win' ? `Won ${odds}` : o === 'loss' ? `Lost ${odds}` : o === 'guaranteed' ? 'Guaranteed' : '';
  const legend = $derived(
    p.guarantee_odds !== null
      ? [['win', `Won ${odds}`], ['loss', `Lost ${odds}`], ['guaranteed', 'Guaranteed'], ['plain', 'Result unknown'], ['now', 'Current pity']]
      : [['plain', 'Five-star pull'], ['now', 'Current pity']]
  );
</script>

<section class="card detail" class:in-soft={inSoft} aria-labelledby="detail-title">
  <header class="detail-head">
    <div>
      <h2 id="detail-title">{p.label}</h2>
      <p class="guarantee">
        {#if p.guarantee_odds !== null}
          {#if p.guaranteed === true}
            <span class="state state-guaranteed">Next 5★ is guaranteed to be featured</span>
            <span class="muted">Your last 5★ was not the featured one.</span>
          {:else if p.guaranteed === false}
            <span class="state">Not guaranteed: next 5★ is a {odds}</span>
            <span class="muted">Your last 5★ was the featured one.</span>
          {:else}
            <span class="state">Guarantee status unknown</span>
            <span class="muted">Not enough history to tell.</span>
          {/if}
        {:else}
          <span class="muted">The featured item cannot be told from the history on this banner, so no guarantee is tracked.</span>
        {/if}
        {#if inSoft}<span class="chip">Past soft pity</span>{/if}
      </p>
    </div>
    <div class="pity-now" aria-label="{p.pity} pulls toward a five-star, out of {p.hard_pity}">
      <strong>{p.pity}</strong><span>of {p.hard_pity} pulls</span>
    </div>
  </header>

  <div class="detail-body">
    <div class="detail-main">
      <div class="detail-section">
        <div class="meter-big" aria-hidden="true">
          <span class="track">
            <i style="width:{pct(p.pity)}%"></i>
            {#if p.soft_pity}<b style="left:{pct(p.soft_pity)}%"></b>{/if}
          </span>
          <span class="scale">
            <span>0</span>
            {#if p.soft_pity}<span class="soft-at" style="left:{pct(p.soft_pity)}%">soft pity {p.soft_pity}</span>{/if}
            <span class="end">hard pity {p.hard_pity}</span>
          </span>
        </div>
      </div>

      {#if p.total === 0}
        <div class="detail-section"><p class="muted" style="margin:0">No pulls on this banner yet.</p></div>
      {:else}
        {#if p.recent_fives.length}
          <div class="detail-section">
            <h3>Pity at each five-star <span class="muted">oldest to newest</span></h3>
            <div class="luck" aria-hidden="true">
              <div class="luck-plot">
                <span class="luck-line" style="bottom:100%"><em>hard {p.hard_pity}</em></span>
                {#if p.soft_pity}<span class="luck-line soft" style="bottom:{pct(p.soft_pity)}%"><em>soft {p.soft_pity}</em></span>{/if}
                {#each p.recent_fives as f, i (`${f.time}:${f.name}:${i}`)}
                  <div class="luck-bar {f.outcome ?? 'plain'}" title="{f.name}: {f.pity} pulls">
                    <b style="bottom:calc({pct(f.pity)}% + 3px)">{f.pity}</b><i style="height:{Math.max(pct(f.pity), 2)}%"></i>
                  </div>
                {/each}
                <div class="luck-bar now" title="Current pity: {p.pity}">
                  <b style="bottom:calc({pct(p.pity)}% + 3px)">{p.pity}</b><i style="height:{Math.max(pct(p.pity), 2)}%"></i>
                </div>
              </div>
            </div>
            <ul class="luck-legend" aria-hidden="true">
              {#each legend as [kind, text]}<li><span class="sw {kind}"></span>{text}</li>{/each}
            </ul>
          </div>
        {/if}

        <div class="detail-section">
          <h3>Rates on this banner</h3>
          <table class="stats">
            <thead><tr><th>Rarity</th><th>Pulls</th><th>Share</th><th>Average pity</th></tr></thead>
            <tbody>
              <tr><td class="r5">5★</td><td>{p.five_stars}</td><td>{fiveRate}%</td><td>{p.avg_five_pity?.toFixed(2) ?? '—'}</td></tr>
              <tr><td class="r4">4★</td><td>{p.four_stars}</td><td>{fourRate}%</td><td>{p.avg_four_pity?.toFixed(2) ?? '—'}</td></tr>
            </tbody>
          </table>
          <p class="stats-note">{p.total.toLocaleString()} pulls in total on this banner.</p>
        </div>

        {#if foursNewestFirst.length}
          <div class="detail-section">
            <details class="four-details">
              <summary>4★ pulls <span class="muted">{foursNewestFirst.length}, newest first</span></summary>
              <div class="pull-chip-list">
                {#each foursNewestFirst as f, i (`${f.time}:${f.name}:${i}`)}
                  <span class="pull-chip"><strong>{f.name || 'Four-star item'}</strong><i>{f.pity}</i></span>
                {/each}
              </div>
            </details>
          </div>
        {/if}
      {/if}
    </div>

    {#if p.total > 0 && p.recent_fives.length}
      <div class="detail-side detail-section">
        <h3>Five-star pulls <span class="muted">newest first</span></h3>
        <ol class="five-list">
          {#each fivesNewestFirst as f, i (`${f.time}:${f.name}:${i}`)}
            {@const itemMeta = catalog?.items.find((item) => item.game === game && (item.name === f.name || item.aliases.includes(f.name)))}
            {@const icon = iconUrl(itemMeta)}
            <li class="five-item element-{itemMeta?.traits.element ?? elementTone(f.name)}">
              <span class="item-icon" class:cover={itemMeta?.kind === 'light_cone'}>
                <span class="glyph" aria-hidden="true">✦</span>
                {#if icon}<img src={icon} alt="" loading="lazy" decoding="async" referrerpolicy="no-referrer" onload={(e) => e.currentTarget.parentElement?.classList.add('loaded')} onerror={(e) => e.currentTarget.remove()} />{/if}
              </span>
              <span class="five-name">{f.name || 'Five-star item'}</span>
              {#if f.outcome}<span class="five-outcome {f.outcome}">{outcomeLabel(f.outcome)}</span>{:else}<span></span>{/if}
              <span class="five-pity">{f.pity}<small> pulls</small></span>
              <time class="five-date">{f.time.slice(0, 10)}</time>
            </li>
          {/each}
        </ol>
      </div>
    {/if}
  </div>
</section>
