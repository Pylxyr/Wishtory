<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { INFO } from '../lib/gameinfo';
  import type { GameLocations } from '../lib/types';

  let { index = 0 }: { index?: number } = $props();

  let loc = $state<GameLocations | null>(null);
  let path = $state('');
  let busy = $state(false);

  $effect(() => {
    api.getGameLocations().then((l) => (loc = l)).catch((e) => app.toast(asError(e).message, 'err'));
  });

  async function save(list: string[]) {
    busy = true;
    try { loc = await api.setGameLocations(list); } catch (e) { app.toast(asError(e).message, 'err'); }
    busy = false;
  }
  async function add() {
    const p = path.trim();
    if (!p || !loc || loc.locations.includes(p)) return;
    await save([...loc.locations, p]);
    path = '';
  }
</script>

{#if loc?.supported}
  <section class="panel pad pop" style="--i:{index}">
    <h2>Game locations</h2>
    <p class="muted" style="margin-top:0">The games run in Wine or Proton here, and Wishtory reads the same files inside the Wine prefix. It checks the usual places by itself (Wine, Lutris, Bottles, Steam and the anime-game launchers). If a game is not found, launch it once, then add its prefix folder below.</p>
    <h3 style="margin:0 0 6px">Found automatically</h3>
    {#if loc.detected.length}
      <div class="list">
        {#each loc.detected as d (d.path)}
          <div class="item"><div class="grow num" style="overflow-wrap:anywhere">{d.path}</div><span class="muted">{d.games.map((g) => INFO[g].short).join(', ')}</span></div>
        {/each}
      </div>
    {:else}
      <p class="muted" style="margin:0">Nothing yet.</p>
    {/if}
    <h3 style="margin:16px 0 6px">Added by you</h3>
    {#if loc.locations.length}
      <div class="list">
        {#each loc.locations as p (p)}
          <div class="item"><div class="grow num" style="overflow-wrap:anywhere">{p}</div><button class="btn sm" disabled={busy} onclick={() => save(loc!.locations.filter((x) => x !== p))}>Remove</button></div>
        {/each}
      </div>
    {/if}
    <div class="row" style="margin-top:10px">
      <input class="field" style="flex:1" placeholder="/home/you/Games/genshin" aria-label="Prefix folder to add" bind:value={path} onkeydown={(e) => e.key === 'Enter' && add()} />
      <button class="btn sm primary" disabled={!path.trim() || busy} onclick={add}>Add</button>
    </div>
  </section>
{/if}
