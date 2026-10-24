<script lang="ts">
  import { app, selectGame } from '../state.svelte';
  import { GAMES, type GameId } from '../types';
  const mine = $derived(app.accounts.filter((a) => a.game === app.game));
</script>

<div class="gamebar">
  <div class="tabs" role="group" aria-label="Game">
    {#each Object.entries(GAMES) as [id, g]}
      <button aria-pressed={app.game === id} class:active={app.game === id} onclick={() => selectGame(id as GameId)}>{g.short}</button>
    {/each}
  </div>
  {#if mine.length}
    <select bind:value={app.uid} aria-label="Account">
      {#each mine as a}
        <option value={a.uid}>UID {a.uid}{a.nickname ? ` · ${a.nickname}` : ''} ({a.pull_count} pulls)</option>
      {/each}
    </select>
  {/if}
</div>
