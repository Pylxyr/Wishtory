<script lang="ts">
  import { app } from '../lib/state.svelte';
  import Glyph from './Glyph.svelte';
  import { INFO } from '../lib/gameinfo';
  const r = $derived(app.refresh);
  const other = $derived(r && r.game !== app.game ? INFO[r.game].short : null);
  const hint = $derived(
    r?.error?.code === 'history_page_not_opened' || r?.error?.code === 'auth_expired'
      ? `Open ${app.info.historyPage} in the game, let the list load, then try again.`
      : null
  );
</script>

{#if r}
  <div class="panel refresh pop" role="status">
    {#if other}<b class="chip">{other}</b>{/if}
    {#if r.phase === 'running'}
      <Glyph name="refresh" /><span>{r.message}</span><div class="track"><i></i></div>
    {:else if r.phase === 'done' && r.summary}
      <span class="meteor-done"></span>
      <Glyph name="sparkle" /><span><b class="num">{r.summary.added}</b> new pulls saved <span class="muted">({r.summary.fetched} read)</span>{r.summary.warning ? ' · ' + r.summary.warning : ''}</span>
      <span class="grow" style="flex:1"></span><button class="btn sm" onclick={() => (app.refresh = null)}>Dismiss</button>
    {:else}
      <span class="err">{r.message}</span>{#if hint}<span class="muted">{hint}</span>{/if}
      <span style="flex:1"></span><button class="btn sm" onclick={() => (app.refresh = null)}>Dismiss</button>
    {/if}
  </div>
{/if}
