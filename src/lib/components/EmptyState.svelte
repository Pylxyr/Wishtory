<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  let { title, text, children }: { title: string; text: string; children?: Snippet } = $props();
</script>

<div class="card empty rise">
  <div class="empty-summary">
    <div class="art"><Icon name="sparkle" size={44} /></div>
    <h2>{title}</h2>
    <p class="muted">{text}</p>
  </div>
  {@render children?.()}
</div>
