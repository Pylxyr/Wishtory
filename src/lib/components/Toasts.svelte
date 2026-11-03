<script lang="ts">
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import { app } from '../state.svelte';
  import { dur } from '../motion';
  import Toast from './Toast.svelte';
</script>

<!-- Each toast carries its own status/alert role, so the container is a plain landmark, not a second live region. -->
<div class="toasts" role="region" aria-label="Notifications">
  {#each app.toasts as t (t.id)}
    <div in:fly={{ x: 40, duration: dur(220) }} out:fly={{ x: 40, duration: dur(180) }} animate:flip={{ duration: dur(200) }}>
      <Toast {t} />
    </div>
  {/each}
</div>
