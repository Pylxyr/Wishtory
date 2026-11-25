<script lang="ts">
  // Tries each address in turn; when none loads (offline, or a name the host does not know yet) the
  // caller's fallback shows instead, so a missing picture never leaves a hole.
  let { urls, alt = '', fallback = '' }: { urls: string[]; alt?: string; fallback?: string } = $props();
  let tried = $state(0);
  let loaded = $state(false);
  $effect(() => { urls; tried = 0; loaded = false; });
</script>

{#if tried < urls.length}
  <img src={urls[tried]} {alt} loading="lazy" referrerpolicy="no-referrer" onload={() => (loaded = true)} onerror={() => (tried += 1)} style:opacity={loaded ? 1 : 0} />
{/if}
{#if tried >= urls.length}<span class="fallback">{fallback}</span>{/if}
