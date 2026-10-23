<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { app } from '../state.svelte';

  const steps = [
    { title: 'Welcome to Wishtory', body: 'Wishtory keeps all your wish history locally. It only reads files the game already created. It never touches the game itself, so there is no ban risk.' },
    { title: '1 · Open the history page in-game', body: 'Launch the game and open its history page (Wish → History, Warp → Records, or Signal Search → History). Wait until the list loads. This is what makes the game cache your temporary history link.' },
    { title: '2 · Click Refresh', body: 'Come back to Wishtory and press Refresh on the Dashboard. Wishtory finds the cached link (read-only) and downloads your history. New pulls are merged; nothing is duplicated.' },
    { title: 'Import, export, and share', body: 'Use Import / Export to move data in or out via UIGF, the standard used by most trackers. You can replay this tour any time from the Help page.' }
  ];
  let i = $state(0);

  function close() {
    localStorage.setItem('wishtory.tutorialDone', '1');
    app.tutorial = false;
  }
</script>

<div class="overlay" transition:fade={{ duration: 150 }} role="dialog" aria-modal="true" aria-label="Welcome tutorial">
  <div class="card modal" in:fly={{ y: 20, duration: 250 }}>
    {#key i}
      <div in:fly={{ x: 24, duration: 200 }}>
        <h2>{steps[i].title}</h2>
        <p class="muted">{steps[i].body}</p>
      </div>
    {/key}
    <div class="dots">{#each steps as _, n}<i class:on={n === i}></i>{/each}</div>
    <div class="row">
      <button class="btn" onclick={close}>Skip</button>
      <span class="spacer" style="flex:1"></span>
      {#if i > 0}<button class="btn" onclick={() => i--}>Back</button>{/if}
      {#if i < steps.length - 1}
        <button class="btn primary" onclick={() => i++}>Next</button>
      {:else}
        <button class="btn primary" onclick={close}>Get started</button>
      {/if}
    </div>
  </div>
</div>
