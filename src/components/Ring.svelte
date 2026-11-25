<script lang="ts">
  import { countUp } from '../lib/motion';
  let { value, max, soft = false, label = 'pity' }: { value: number; max: number; soft?: boolean; label?: string } = $props();
  const len = 2 * Math.PI * 44;
  const p = $derived(Math.min(1, value / Math.max(1, max)));
</script>

<div class="ring" style="--len:{len};--p:{p}">
  <svg viewBox="0 0 100 100"><circle class="track" cx="50" cy="50" r="44" /><circle class="fill" class:soft cx="50" cy="50" r="44" /></svg>
  <div class="mid"><b use:countUp={{ value, ms: 1100 }}>0</b><span>{label}</span></div>
</div>
