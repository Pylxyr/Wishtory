<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  let { title, text, children }: { title: string; text: string; children?: Snippet } = $props();
</script>

<div class="empty">
  <div class="empty-summary">
    <Icon name="sparkle" size={28} />
    <h2>{title}</h2>
    <p class="muted">{text}</p>
  </div>
  {@render children?.()}
</div>
