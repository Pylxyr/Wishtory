<script lang="ts">
  import { app } from '../lib/state.svelte';
  import LiveHero from '../components/LiveHero.svelte';
  import StatStrip from '../components/StatStrip.svelte';
  import BannerBoard from '../components/BannerBoard.svelte';
  import BannerDetail from '../components/BannerDetail.svelte';
  import RefreshPanel from '../components/RefreshPanel.svelte';
  import Glyph from '../components/Glyph.svelte';

  const selected = $derived(app.selectedBanner ?? app.pity[0]?.banner ?? '');
  const detail = $derived(app.pity.find((p) => p.banner === selected));
  const icon = $derived(app.game === 'genshin' ? 'sparkle' : app.game === 'star_rail' ? 'train' : 'tv');
</script>

<div class="page">
  <RefreshPanel />
  {#if !app.account}
    <div class="panel empty pad">
      <span class="big"><Glyph name={icon} size={96} /></span>
      <h2>No {app.info.short} history yet</h2>
      <p class="muted" style="max-width:480px;margin:0">{app.info.howTo} Then switch back here and press Refresh. Wishtory only reads the file the game already wrote.</p>
      <div class="row">
        <button class="btn primary" onclick={() => app.refreshGame()}><Glyph name="refresh" size={16} />Refresh</button>
        <button class="btn" onclick={() => app.go('import')}><Glyph name="import" size={16} />Import a file</button>
      </div>
    </div>
  {:else if app.pityLoading && !app.pity.length}
    <div class="skeleton"></div><div class="skeleton" style="min-height:140px"></div>
  {:else}
    <LiveHero onSelect={(k) => (app.selectedBanner = k)} />
    <StatStrip />
    <section>
      <div class="live-head"><span class="label">{app.info.bannersTitle}</span></div>
      <BannerBoard {selected} onSelect={(k) => (app.selectedBanner = k)} />
    </section>
    {#if detail}<BannerDetail p={detail} />{/if}
  {/if}
</div>
