<script lang="ts">
  import { app, selectGame, refreshFromGame, resyncFromGame } from '../state.svelte';
  import { GAMES, type GameId } from '../types';
  import Icon from './Icon.svelte';

  const mine = $derived(app.accounts.filter((a) => a.game === app.game));
  const install = $derived(app.installs.find((i) => i.game === app.game));
  const info = $derived(GAMES[app.game]);
  // The refresh progress belongs to the game and page it was started on: switching to another game or
  // page hides it, and coming back shows it again.
  const here = $derived(app.runView?.game === app.game && app.runView?.page === app.page);
  const working = $derived(app.busy && here);
  const shownSteps = $derived(app.steps.slice(-6));
  // Screen readers hear the banner or phase name, not every page update.
  const phase = $derived.by(() => {
    const last = app.steps.at(-1) ?? '';
    const colon = last.indexOf(':');
    return colon === -1 ? last : last.slice(0, colon);
  });
</script>

<div class="context" class:is-busy={working}>
  <div class="seg" role="group" aria-label="Game">
    {#each Object.entries(GAMES) as [id, g]}
      <button data-game={id} aria-pressed={app.game === id} onclick={() => selectGame(id as GameId)}>{g.short}</button>
    {/each}
  </div>
  {#if mine.length}
    <select bind:value={app.uid} aria-label="Account">
      {#each mine as a}
        <option value={a.uid}>UID {a.uid}{a.nickname ? ` · ${a.nickname}` : ''} ({a.pull_count} pulls)</option>
      {/each}
    </select>
  {/if}
  <div class="context-end">
    {#if working}
      <span class="sr" role="status">{phase}</span>
      <span class="status-line" aria-hidden="true"><i class="pulse"></i><span class="status-text">{app.status}</span></span>
    {:else if install?.found}
      <span class="chip ok">{info.name} detected</span>
    {:else}
      <span class="chip" data-tip="Launch the game once so it creates its log file">{info.name} not detected yet</span>
    {/if}
    <button class="btn" onclick={resyncFromGame} disabled={app.busy} data-tip="Downloads everything the server still keeps, not only what is new. Use it if pulls look missing">
      Re-sync all
    </button>
    <button class="btn primary" onclick={refreshFromGame} disabled={app.busy} data-tip="Reads the game's cache (read-only), then downloads what is new in your history">
      <span class:spin={working}><Icon name="refresh" size={16} /></span>{working ? 'Working…' : `Refresh ${info.short}`}
    </button>
  </div>
  {#if app.steps.length && here}
    <div class="activity">
      <ol aria-label="Refresh progress">
        {#each shownSteps as step, i}
          {@const last = i === shownSteps.length - 1}
          {@const failed = last && !app.busy && app.result?.kind === 'err'}
          <li class:current={last && app.busy} class:failed>
            <span class="mark" aria-hidden="true">{failed ? '✕' : last && app.busy ? '' : '✓'}</span>{step}
          </li>
        {/each}
      </ol>
      {#if app.result}
        <div class="activity-foot">
          <p class="result {app.result.kind}">{app.result.text}</p>
          <div class="row">
            {#if app.result.kind === 'err'}<button class="btn" onclick={() => (app.page = 'help')}>Open Help</button>{/if}
            <button class="btn" onclick={() => { app.steps = []; app.result = null; app.runView = null; }}>Dismiss</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
  <div class="busytrack" aria-hidden="true"><div class="busybar"></div></div>
</div>
