<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api, errMsg } from '../lib/api';
  import { toast } from '../lib/state.svelte';
  import type { LogEntry } from '../lib/types';

  const LEVELS = ['DEBUG', 'INFO', 'WARN', 'ERROR'] as const;
  let logs = $state<LogEntry[]>([]);
  let level = $state<(typeof LEVELS)[number]>('DEBUG'); // minimum level shown
  let query = $state('');
  let box: HTMLDivElement | undefined = $state();

  const shown = $derived(
    logs.filter(
      (l) => LEVELS.indexOf(l.level) >= LEVELS.indexOf(level) && (!query || `${l.source} ${l.message}`.toLowerCase().includes(query.toLowerCase()))
    )
  );

  async function load() {
    try {
      const next = await api.getLogs();
      if (next.length === logs.length && next.at(-1)?.seq === logs.at(-1)?.seq) return; // nothing new
      const stick = !box || box.scrollHeight - box.scrollTop - box.clientHeight < 40;
      logs = next;
      if (stick) { await tick(); box?.scrollTo({ top: box.scrollHeight }); }
    } catch { /* backend unavailable */ }
  }
  async function clear() {
    try { await api.clearLogs(); logs = []; } catch (e) { toast('err', errMsg(e)); }
  }

  onMount(() => {
    void load();
    const t = setInterval(load, 1500); // polls only while this page is open
    return () => clearInterval(t);
  });
</script>

<h1>Logging</h1>
<p class="muted">Recent activity, newest at the bottom. Kept in memory only (last 1000 lines).</p>

<div class="row log-tools">
  <select bind:value={level} aria-label="Minimum level">
    {#each LEVELS as l}<option value={l}>{l === 'DEBUG' ? 'All levels' : `${l} and above`}</option>{/each}
  </select>
  <input bind:value={query} placeholder="Filter text..." aria-label="Filter log text" />
  <span class="muted tabnum">{shown.length} / {logs.length}</span>
  <button class="btn" onclick={load}>Refresh</button>
  <button class="btn" onclick={clear} disabled={!logs.length}>Clear</button>
</div>

<div class="card log-view" bind:this={box} role="log" aria-live="off">
  {#each shown as l (l.seq)}
    <div class="log-line">
      <span class="log-time">{l.time}</span>
      <b class="log-level lv-{l.level.toLowerCase()}">{l.level}</b>
      <span class="log-src">{l.source}</span>
      <span class="log-msg">{l.message}</span>
    </div>
  {:else}
    <p class="muted log-empty">{logs.length ? 'No lines match the filter.' : 'No log lines yet. Use the app and they will appear here.'}</p>
  {/each}
</div>
