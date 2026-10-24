<script lang="ts">
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { app } from '../state.svelte';
  import { dur } from '../motion';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === 'err' ? 'alert' : 'status'} in:fly={{ x: 40, duration: dur(220) }} out:fly={{ x: 40, duration: dur(180) }} animate:flip={{ duration: dur(200) }}>
      {t.text}
    </div>
  {/each}
</div>
