<script lang="ts">
  import { api, asError } from '../lib/api';
  import { app } from '../lib/state.svelte';
  import type { AboutInfo, AppUpdate } from '../lib/types';
  let about = $state<AboutInfo | null>(null);
  let update = $state<AppUpdate | null>(null);
  $effect(() => { api.getAbout().then((a) => (about = a)); });
  async function check() { try { update = await api.checkAppUpdate(); } catch (e) { app.toast(asError(e).message, 'err'); } }
</script>

<div class="page">
  {#if about}
    <section class="panel pad pop">
      <h2 style="font-size:22px">{about.name} <span class="muted num" style="font-size:14px">{about.version}</span></h2>
      <p class="muted" style="margin-top:0">{about.description}</p>
      <div class="row wrap">
        <button class="btn sm" onclick={check}>Check for updates</button>
        <button class="btn sm" onclick={() => api.openExternal(about!.repository)}>Source code</button>
        {#if update}<span class={update.available ? 'gold' : 'muted'}>{update.available ? `Version ${update.latest} is available.` : 'You are up to date.'}</span>{/if}
      </div>
      {#each about.disclaimers as d (d)}<p class="muted" style="font-size:12.5px">{d}</p>{/each}
    </section>
    {#each about.groups as g, i (g.title)}
      <section class="panel pad pop" style="--i:{i + 1}">
        <h2>{g.title}</h2><p class="muted" style="margin-top:0">{g.note}</p>
        <div class="list">{#each g.credits as c (c.name)}<div class="item"><div class="grow"><b>{c.name}</b> <span class="muted">· {c.role}</span></div><span class="faint">{c.license}</span></div>{/each}</div>
      </section>
    {/each}
  {/if}
</div>
