<script lang="ts">
  import { app, dismissResult, openHelp, retryLast } from '../state.svelte';
  import { classify } from '../errors';
  import Icon from './Icon.svelte';

  const result = $derived(app.result);
  const r = $derived(result?.kind === 'err' ? classify(result.text, result.game) : null);

  // Rate limiting: the retry button counts down instead of inviting another refusal.
  let tick = $state(Date.now());
  $effect(() => {
    if (!r?.waitSeconds) return;
    tick = Date.now();
    const t = setInterval(() => (tick = Date.now()), 500);
    return () => clearInterval(t);
  });
  const wait = $derived(r && result ? Math.max(0, Math.ceil(r.waitSeconds - (tick - result.at) / 1000)) : 0);
</script>

{#if r && result && !app.busy}
  <section class="recovery" role="alert" aria-labelledby="recovery-title">
    <span class="recovery-icon" aria-hidden="true"><Icon name="alert" size={20} /></span>
    <div class="recovery-body">
      <h2 id="recovery-title">{r.title}</h2>
      <p class="muted">{r.why}</p>
      {#if r.steps.length}
        <ol class="recovery-steps">{#each r.steps as step}<li>{step}</li>{/each}</ol>
      {/if}
      <div class="row">
        {#if r.canRetry}
          <button class="btn primary" disabled={wait > 0} onclick={retryLast}>
            <Icon name="refresh" size={16} />{wait > 0 ? `Try again in ${wait} s` : 'Try again'}
          </button>
        {/if}
        {#if r.help}<button class="btn" onclick={() => openHelp(r.help)}>Why does this happen?</button>{/if}
        <button class="btn" onclick={dismissResult}>Dismiss</button>
      </div>
      <details class="tech">
        <summary>What happened</summary>
        {#if app.steps.length > 1}
          <ul class="plain muted">{#each app.steps.slice(0, -1) as step}<li>{step}</li>{/each}</ul>
        {/if}
        <p class="muted"><code>{result.text}</code></p>
      </details>
    </div>
  </section>
{/if}
