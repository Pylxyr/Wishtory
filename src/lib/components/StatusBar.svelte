<script lang="ts">
  import { app, dismissResult } from '../state.svelte';
  import { classify } from '../errors';
  import { GAMES } from '../types';
  import Icon from './Icon.svelte';

  let open = $state(false);

  const runGame = $derived(app.runGame ? GAMES[app.runGame].short : '');
  const failure = $derived(app.result?.kind === 'err' ? classify(app.result.text, app.result.game, !!app.locations?.supported) : null);
  // Screen readers hear the banner or phase name while working, then the outcome, not every progress line.
  const announce = $derived.by(() => {
    if (app.busy) {
      const last = app.steps.at(-1) ?? '';
      const colon = last.indexOf(':');
      return `${runGame}: ${colon === -1 ? last : last.slice(0, colon)}`;
    }
    if (failure) return `Refresh stopped. ${failure.title}.`;
    return app.result?.text ?? '';
  });
  const done = $derived(app.busy ? app.steps.slice(0, -1) : app.steps);

  $effect(() => { if (!app.steps.length) open = false; });
</script>

<footer class="statusbar" class:busy={app.busy}>
  <div class="status-main {app.busy ? '' : app.result?.kind ?? ''}">
    {#if app.busy}
      <i class="pulse" aria-hidden="true"></i>
      <span class="status-text" aria-hidden="true">{runGame}: {app.status}</span>
    {:else if app.result}
      <span class="mark" aria-hidden="true"><Icon name={app.result.kind === 'ok' ? 'check' : app.result.kind === 'warn' ? 'alert' : 'x'} size={14} /></span>
      <span class="status-text" aria-hidden="true">{failure ? `Refresh stopped: ${failure.title}` : app.result.text}</span>
    {:else}
      <span class="status-text muted" aria-hidden="true">Ready</span>
    {/if}
  </div>
  <div class="status-end">
    {#if app.steps.length > 1 || (app.steps.length === 1 && !app.busy)}
      <button class="link" aria-expanded={open} aria-controls="status-steps" onclick={() => (open = !open)}>Details</button>
    {/if}
    {#if app.result && !app.busy}
      <button class="icon-btn" aria-label="Dismiss" onclick={dismissResult}><Icon name="x" size={15} /></button>
    {/if}
    {#if !app.busy && !app.result}
      {#if app.catalog}<span class="muted tabnum">Catalog {app.catalog.catalog_version}</span>{/if}
      {#if app.banners}<span class="muted tabnum">Banners {app.banners.banners_version}</span>{/if}
    {/if}
  </div>
  <div class="sr" role="status">{announce}</div>
  {#if open && app.steps.length}
    <div class="status-pop" id="status-steps">
      <ol aria-label="Refresh steps">
        {#each done as step}<li><span class="mark" aria-hidden="true">✓</span>{step}</li>{/each}
        {#if app.busy}<li class="current"><span class="mark" aria-hidden="true"></span>{app.steps.at(-1)}</li>{/if}
        {#if failure}<li class="failed"><span class="mark" aria-hidden="true">✕</span>Stopped: {failure.title}</li>{/if}
      </ol>
    </div>
  {/if}
</footer>
