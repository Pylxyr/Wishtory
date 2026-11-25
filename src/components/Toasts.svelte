<script lang="ts">
  import { app } from '../lib/state.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}<div class="toast {t.kind}">{t.text}</div>{/each}
</div>
