<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, text, children }: { title: string; text: string; children?: Snippet } = $props();
</script>

<div class="card empty rise">
  <div class="art">✦</div>
  <h2>{title}</h2>
  <p class="muted">{text}</p>
  {@render children?.()}
</div>
