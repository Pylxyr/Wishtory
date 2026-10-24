<script lang="ts">
  import { Tween } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  import { dur } from '../motion';

  let { value, ms = 700 }: { value: number; ms?: number } = $props();
  const t = new Tween(0, { easing: cubicOut });
  $effect(() => {
    t.set(value, { duration: dur(ms) });
  });
</script>

<span class="tabnum">{Math.round(t.current).toLocaleString()}</span>
