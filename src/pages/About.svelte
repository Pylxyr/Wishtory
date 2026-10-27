<script lang="ts">
  import { app, checkAppUpdate, openLink } from '../lib/state.svelte';

  function openRelease() {
    const url = app.update?.url;
    if (url) void openLink(url);
  }
</script>

<div class="page-head">
  <h1>About</h1>
  <p>What Wishtory is, who made it possible, and what it is built with.</p>
</div>

{#if app.about}
  <section class="about-hero" aria-label="Wishtory">
    <div class="about-title">
      <svg class="brand-mark" viewBox="0 0 24 24" width="28" height="28" aria-hidden="true"><path d="M12 1.5l2.4 8.1L22.5 12l-8.1 2.4L12 22.5l-2.4-8.1L1.5 12l8.1-2.4z" /></svg>
      <h2>{app.about.name}</h2>
      <span class="chip">Version {app.about.version}</span>
      {#if app.update?.available}<span class="chip ok">{app.update.latest} available</span>{/if}
    </div>
    <p>{app.about.description}</p>
    <p class="muted">{app.about.copyright}. Released under the {app.about.license}.</p>
    <p class="muted">{app.about.disclaimer}</p>
    <div class="about-links">
      {#if app.update?.available}<button class="btn primary" onclick={openRelease}>View release</button>{/if}
      <button class="btn" disabled={app.updateChecking} onclick={() => checkAppUpdate(true)}>{app.updateChecking ? 'Checking…' : 'Check for updates'}</button>
      <button class="btn" onclick={() => openLink(app.about!.repository)}>Source code</button>
      <button class="btn" onclick={() => openLink(app.about!.releases)}>Releases</button>
      <button class="btn" onclick={() => openLink(app.about!.issues)}>Report an issue</button>
    </div>
  </section>

  <h2 class="credits-title">Credits</h2>
  {#each app.about.groups as group (group.title)}
    <section class="credit-group" aria-label={group.title}>
      <h3>{group.title}</h3>
      <p class="muted">{group.note}</p>
      <div class="settings-group">
        {#each group.credits as c (c.url)}
          <div class="setting-row">
            <div class="setting-copy">
              <h2><button class="link" onclick={() => openLink(c.url)}>{c.name}</button></h2>
              <p class="muted">{c.role}</p>
            </div>
            {#if c.license}<span class="chip">{c.license}</span>{/if}
          </div>
        {/each}
      </div>
    </section>
  {/each}
{:else}
  <p class="muted">About information could not be loaded.</p>
{/if}
