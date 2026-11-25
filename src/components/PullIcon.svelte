<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { iconUrls } from '../lib/icons';
  import Art from './Art.svelte';
  let { name, rank = 3 }: { name: string; rank?: number } = $props();
  const item = $derived(app.itemByName(name));
</script>

<span class="icon" class:r5={rank === 5} class:r4={rank === 4}><Art urls={iconUrls(item)} fallback={name.trim().charAt(0).toUpperCase()} /></span>
