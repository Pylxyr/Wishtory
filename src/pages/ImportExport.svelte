<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { app, reloadAccounts, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import Icon from '../lib/components/Icon.svelte';

  let format = $state('uigf4');
  let scope = $state<'selected' | 'all'>('selected');
  let busy = $state(false);

  const names: Record<string, string> = { uigf4: 'uigf-v4.2', uigf3: 'uigf-v3.0', srgf1: 'srgf-v1.0' };

  async function doImport() {
    const path = await open({ multiple: false, filters: [{ name: 'UIGF / SRGF JSON', extensions: ['json'] }] });
    if (typeof path !== 'string') return;
    busy = true;
    try {
      const r = await api.importFile(path);
      await reloadAccounts();
      app.refreshKey++;
      toast('ok', `Imported ${r.format}: ${r.added} new pulls across ${r.accounts} account(s)${r.skipped ? `. ${r.skipped} rows were skipped because they lack a rarity or id` : ''}.`);
    } catch (e) { toast('err', errMsg(e)); }
    busy = false;
  }

  async function doExport() {
    const single = scope === 'selected' || format !== 'uigf4';
    if (single && !app.uid) { toast('err', 'Select an account first (or import / refresh some data).'); return; }
    const path = await save({ defaultPath: `wishtory-${names[format]}.json`, filters: [{ name: 'JSON', extensions: ['json'] }] });
    if (!path) return;
    busy = true;
    try {
      await api.exportFile(path, format, single ? app.game : null, single ? app.uid : null);
      toast('ok', 'Export saved.');
    } catch (e) { toast('err', errMsg(e)); }
    busy = false;
  }
</script>

<h1>Import / Export</h1>
<p class="muted">Wishtory speaks <a href="https://uigf.org" target="_blank" rel="noreferrer">UIGF</a>, the shared standard for wish history.</p>

<div class="grid">
  <div class="card rise">
    <h2>Import</h2>
    <p class="muted">UIGF v4.x, older UIGF (v2/v3), or SRGF v1.0. Duplicates are merged by pull ID.</p>
    <button class="btn primary" onclick={doImport} disabled={busy}><Icon name="upload" />Choose file...</button>
  </div>
  <div class="card rise" style="animation-delay:70ms">
    <h2>Export</h2>
    <div class="row" style="margin-bottom:10px">
      <select bind:value={format} aria-label="Format">
        <option value="uigf4">UIGF v4.2 (recommended)</option>
        <option value="uigf3">UIGF v3.0 (Genshin only)</option>
        <option value="srgf1">SRGF v1.0 (Star Rail only)</option>
      </select>
      {#if format === 'uigf4'}
        <select bind:value={scope} aria-label="Scope">
          <option value="selected">Selected account</option>
          <option value="all">All accounts</option>
        </select>
      {/if}
    </div>
    <button class="btn primary" onclick={doExport} disabled={busy}><Icon name="download" />Export...</button>
  </div>
</div>
