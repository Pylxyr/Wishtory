<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { onMount } from 'svelte';
  import { app } from '../state.svelte';
  import { dur } from '../motion';

  const steps = [
    { title: 'Welcome to Wishtory', body: 'Wishtory keeps all your wish history locally. It only reads files the game already created. It never touches the game itself. No tool can promise zero risk, so please read the notes in Help (the ? icon at the top right).' },
    { title: 'Step 1: Open the history page in-game', body: 'Launch the game and open its history page (Wish, then History; Warp, then Records; or Signal Search, then History). Wait until the list loads. This is what makes the game cache your temporary history link.' },
    { title: 'Step 2: Click Refresh', body: 'Come back to Wishtory and press Refresh at the top of the window. Wishtory finds the cached link (read-only) and downloads your history. New pulls are merged; nothing is duplicated.' },
    { title: 'Import, export, and share', body: 'Use Import / Export to move data in or out via UIGF, the standard used by most trackers. You can replay this tour any time from Help (the ? icon at the top right).' }
  ];
  let i = $state(0);
  let dialog: HTMLDivElement;
  let previousFocus: HTMLElement | null = null;

  onMount(() => {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.querySelector<HTMLButtonElement>('button')?.focus();
    return () => previousFocus?.focus();
  });

  function keepFocus(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== 'Tab') return;
    const controls = Array.from(dialog.querySelectorAll<HTMLButtonElement>('button:not([disabled])'));
    if (!controls.length) return;
    const first = controls[0];
    const last = controls[controls.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function close() {
    localStorage.setItem('wishtory.tutorialDone', '1');
    app.tutorial = false;
  }
</script>

<div class="overlay" transition:fade={{ duration: dur(150) }} role="dialog" aria-modal="true" aria-labelledby="tutorial-title" tabindex="-1" bind:this={dialog} onkeydown={keepFocus}>
  <div class="card modal" in:fly={{ y: 20, duration: dur(250) }}>
    {#key i}
      <div in:fly={{ x: 24, duration: dur(200) }}>
        <h2 id="tutorial-title">{steps[i].title}</h2>
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
