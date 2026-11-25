<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { PRESETS, palette, FOLLOW_GAME } from '../lib/themes';
  import { ago } from '../lib/gameinfo';
  import type { CatalogReview, LogEntry } from '../lib/types';

  let review = $state<CatalogReview | null>(null);
  let logs = $state<LogEntry[] | null>(null);
  let busy = $state(false);
  const st = $derived(app.catalogStatus);

  async function check() {
    busy = true;
    try {
      const r = await api.checkCatalog(true);
      app.catalogStatus = await api.getCatalogStatus();
      app.toast(r.status === 'up_to_date' ? 'The game catalog is up to date.' : r.status === 'applied' ? 'A catalog update was applied.' : r.status === 'needs_review' ? 'An update is waiting for your review.' : (r.message ?? 'Try again later.'), 'info');
    } catch (e) { app.toast(asError(e).message, 'err'); }
    busy = false;
  }
  async function openReview() { review = await api.getCatalogReview(); }
  async function apply() {
    if (!review) return;
    try {
      const r = await api.applyCatalog(review.version, review.fingerprint);
      app.toast(r.stale ? 'Your pulls changed since this review. Review it again.' : 'Catalog updated.', r.applied ? 'ok' : 'info');
      review = r.stale ? await api.getCatalogReview() : null;
      app.catalogStatus = await api.getCatalogStatus(); app.catalog = await api.getCatalog(); app.banners = await api.getBanners();
      await app.loadPity();
    } catch (e) { app.toast(asError(e).message, 'err'); }
  }
  async function later() { await api.dismissCatalog(); review = null; app.catalogStatus = await api.getCatalogStatus(); }
  async function auto(v: boolean) { try { await api.setCatalogAutoApply(v); app.catalogStatus = await api.getCatalogStatus(); } catch (e) { app.toast(asError(e).message, 'err'); } }
  async function toggleLogs(on: boolean) { await api.setLoggingEnabled(on); logs = on ? await api.getLogs() : null; }
</script>

<div class="page">
  <section class="panel pad pop">
    <h2>Appearance</h2>
    <div class="switch"><div><b>Brightness</b><div class="muted">Follow the system, or always dark or light.</div></div>
      <div class="tabs">{#each ['system', 'dark', 'light'] as m (m)}<button class:on={app.prefs.mode === m} onclick={() => app.setPrefs({ mode: m as 'system' })}>{m[0].toUpperCase() + m.slice(1)}</button>{/each}</div></div>
    <div style="margin:10px 0 6px"><b>Colour theme</b></div>
    <div class="swatches">
      <button class="swatch" class:on={app.prefs.themeId === FOLLOW_GAME} onclick={() => app.setPrefs({ themeId: FOLLOW_GAME })}><i style="background:conic-gradient(#D8C7A0,#8AD7FF,#D6F633,#D8C7A0)"></i>Follow game</button>
      {#each PRESETS as p (p.id)}
        <button class="swatch" class:on={app.prefs.themeId === p.id} onclick={() => app.setPrefs({ themeId: p.id })}><i style="background:{palette(p.id, app.dark, app.game).accent}"></i>{p.name}</button>
      {/each}
    </div>
    <div class="switch"><div><b>Animations</b><div class="muted">Sparkles, orbits, scanlines and the rest of each game's motion. Also off when your system asks for reduced motion.</div></div>
      <input type="checkbox" checked={app.prefs.motion} onchange={(e) => app.setPrefs({ motion: e.currentTarget.checked })} /></div>
    <div class="switch"><div><b>Comfortable rows</b><div class="muted">Taller rows in lists.</div></div>
      <input type="checkbox" checked={app.prefs.comfortable} onchange={(e) => app.setPrefs({ comfortable: e.currentTarget.checked })} /></div>
  </section>

  <section class="panel pad pop" style="--i:1">
    <h2>Pull lists</h2>
    <div class="switch"><div><b>How a pull is shown</b></div>
      <div class="tabs">{#each [['icons_names', 'Icon and name'], ['names', 'Name'], ['icons', 'Icon']] as [id, t] (id)}<button class:on={app.prefs.pullView === id} onclick={() => app.setPrefs({ pullView: id as 'names' })}>{t}</button>{/each}</div></div>
    <div class="switch"><div><b>Newest first</b></div><input type="checkbox" checked={app.prefs.newestFirst} onchange={(e) => app.setPrefs({ newestFirst: e.currentTarget.checked })} /></div>
  </section>

  <section class="panel pad pop" style="--i:2">
    <h2>Game catalog</h2>
    {#if st}
      <p class="muted" style="margin-top:0">Version {st.version} · {st.last_check ? 'checked ' + ago(st.last_check) : 'never checked'}. Names, banners and pity rules update as signed data, separately from app releases.</p>
      {#if st.waiting}
        <div class="panel lit pad" style="margin-bottom:12px">
          <b>Version {st.waiting.version} is waiting for review</b>
          <div class="muted">{st.waiting.major} major · {st.waiting.minor} minor · {st.waiting.info} new · {st.waiting.affected_pulls} saved pull{st.waiting.affected_pulls === 1 ? '' : 's'} would read differently</div>
          <div class="row" style="margin-top:10px"><button class="btn sm primary" onclick={openReview}>Review</button></div>
        </div>
      {/if}
      {#if review}
        <div class="panel pad" style="margin-bottom:12px">
          <div class="list">{#each review.changes as c (c.id)}<div class="item"><span class="tag" class:gold={c.severity === 'major'}>{c.severity}</span><div class="grow">{c.summary}</div></div>{/each}</div>
          <div class="row" style="margin-top:10px"><button class="btn sm primary" onclick={apply}>Apply</button><button class="btn sm" onclick={later}>Not now</button></div>
        </div>
      {/if}
      <div class="row"><button class="btn sm" disabled={busy} onclick={check}>Check now</button></div>
      <div class="switch"><div><b>Apply updates that change no saved pull</b></div><input type="checkbox" checked={st.auto_apply} onchange={(e) => auto(e.currentTarget.checked)} /></div>
    {/if}
  </section>

  <section class="panel pad pop" style="--i:3">
    <h2>Diagnostics</h2>
    <div class="switch"><div><b>Keep a log</b><div class="muted">Kept in memory only; never contains links or keys.</div></div><input type="checkbox" checked={logs !== null} onchange={(e) => toggleLogs(e.currentTarget.checked)} /></div>
    {#if logs}<pre class="num muted" style="max-height:180px;overflow:auto;margin:0;font-size:12px">{#each logs as l (l.seq)}{l.time} {l.level} {l.message}
{/each}</pre>{/if}
  </section>
</div>
