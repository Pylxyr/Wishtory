<script lang="ts">
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { app } from '../state.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" in:fly={{ x: 40, duration: 220 }} out:fly={{ x: 40, duration: 180 }} animate:flip={{ duration: 200 }}>
      {t.text}
    </div>
  {/each}
</div>
