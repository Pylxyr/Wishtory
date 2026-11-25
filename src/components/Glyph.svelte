<script lang="ts">
  let { name, size = 20 }: { name: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if name === 'sparkle'}
    <path d="M12 2c.6 5.2 2.8 8.4 10 10-7.2 1.6-9.4 4.8-10 10-.6-5.2-2.8-8.4-10-10 7.2-1.6 9.4-4.8 10-10Z" fill="currentColor" stroke="none" />
  {:else if name === 'train'}
    <rect x="5" y="3" width="14" height="14" rx="3" /><path d="M5 11h14M9 21l2-4M15 21l-2-4" /><circle cx="9" cy="14" r="0.6" fill="currentColor" /><circle cx="15" cy="14" r="0.6" fill="currentColor" />
  {:else if name === 'tv'}
    <rect x="3" y="6" width="18" height="13" rx="2" /><path d="m8 2 4 4 4-4" />
  {:else if name === 'overview'}
    <rect x="3" y="3" width="7.5" height="9" rx="1.5" /><rect x="13.5" y="3" width="7.5" height="5" rx="1.5" /><rect x="13.5" y="11" width="7.5" height="10" rx="1.5" /><rect x="3" y="15" width="7.5" height="6" rx="1.5" />
  {:else if name === 'history'}
    <path d="M3 12a9 9 0 1 0 3-6.7L3 8" /><path d="M3 3v5h5M12 7v5l3 2" />
  {:else if name === 'import'}
    <path d="M12 3v12m0 0-4-4m4 4 4-4M4 17v3h16v-3" />
  {:else if name === 'accounts'}
    <circle cx="9" cy="8" r="3.5" /><path d="M2.5 20c.7-3.6 3.2-5.5 6.5-5.5s5.8 1.9 6.5 5.5M16 4.8a3.5 3.5 0 0 1 0 6.4M18 14.8c1.9.7 3.2 2.4 3.5 5.2" />
  {:else if name === 'settings'}
    <circle cx="12" cy="12" r="3" /><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1" />
  {:else if name === 'about'}
    <circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7.5v.01" />
  {:else if name === 'refresh'}
    <path d="M20 11a8 8 0 0 0-14.5-4M4 4v4h4M4 13a8 8 0 0 0 14.5 4M20 20v-4h-4" />
  {:else if name === 'plus'}
    <path d="M12 5v14M5 12h14" />
  {/if}
</svg>
