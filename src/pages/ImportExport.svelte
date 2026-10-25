<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { app, reloadAccounts, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES } from '../lib/types';
  import Icon from '../lib/components/Icon.svelte';

  let format = $state('uigf4');
  let scope = $state<'selected' | 'all'>('selected');
  let busy = $state(false);

  const formats = [
    { id: 'uigf4', label: 'UIGF v4.2', note: 'Recommended. Covers every game and account.', game: null },
    { id: 'uigf3', label: 'UIGF v3.0', note: 'Genshin only. For Paimon.moe and older tools.', game: 'hk4e' },
    { id: 'srgf1', label: 'SRGF v1.0', note: 'Star Rail only.', game: 'hkrpg' },
    { id: 'paimon', label: 'Paimon.moe local wish data', note: 'Genshin only.', game: 'hk4e' }
  ] as const;
  const usable = (f: (typeof formats)[number]) => f.game === null || f.game === app.game;
  // Switching games can leave a format selected that the new game cannot use.
  $effect(() => {
    if (!formats.some((f) => f.id === format && usable(f))) format = 'uigf4';
  });

  const names: Record<string, string> = { uigf4: 'uigf-v4.2', uigf3: 'uigf-v3.0', srgf1: 'srgf-v1.0', paimon: 'paimon-moe-wishes' };

  async function doImport() {
    if (busy) return;
    busy = true;
    try {
      const path = await open({ multiple: false, filters: [{ name: 'Wish history JSON', extensions: ['json'] }] });
      if (typeof path !== 'string') return;
      const r = await api.importFile(path);
      await reloadAccounts();
      app.refreshKey++;
      toast('ok', `Imported ${r.format}: ${r.added} new pulls across ${r.accounts} account(s)${r.skipped ? `. ${r.skipped} rows were skipped because they were malformed or their item IDs were not recognized` : ''}.`);
    } catch (e) { toast('err', errMsg(e)); }
    finally { busy = false; }
  }

  async function doExport() {
    if (busy) return;
    const single = scope === 'selected' || format !== 'uigf4';
    if (single && !app.uid) { toast('err', 'Select an account first (or import / refresh some data).'); return; }
    busy = true;
    try {
      const path = await save({ defaultPath: `wishtory-${names[format]}.json`, filters: [{ name: 'JSON', extensions: ['json'] }] });
      if (!path) return;
      await api.exportFile(path, format, single ? app.game : null, single ? app.uid : null);
      toast('ok', 'Export saved.');
    } catch (e) { toast('err', errMsg(e)); }
    finally { busy = false; }
  }
</script>

<div class="page-head">
  <h1>Import / Export</h1>
  <p>Wishtory speaks <a href="https://uigf.org" target="_blank" rel="noreferrer">UIGF</a>, the shared standard for wish history.</p>
</div>

<div class="split">
  <section class="card" aria-labelledby="import-title">
    <h2 id="import-title">Import</h2>
    <p class="muted">Paimon.moe local wish backups, UIGF v4.x, older UIGF (v2/v3), or SRGF v1.0. Importing the same file again does not create duplicates.</p>
    <button class="btn primary" onclick={doImport} disabled={busy}><Icon name="upload" />Choose file…</button>
  </section>

  <section class="card" aria-labelledby="export-title">
    <h2 id="export-title">Export</h2>
    <fieldset class="options">
      <legend>Format</legend>
      {#each formats as f}
        <label class="option" class:disabled={!usable(f)}>
          <input type="radio" name="format" value={f.id} bind:group={format} disabled={!usable(f)} />
          <span><b>{f.label}</b><small>{f.game !== null && !usable(f) ? `${f.note} Switch to ${GAMES[f.game].short} to use it.` : f.note}</small></span>
        </label>
      {/each}
    </fieldset>
    {#if format === 'uigf4'}
      <fieldset class="options">
        <legend>Accounts to include</legend>
        <label class="option">
          <input type="radio" name="scope" value="selected" bind:group={scope} />
          <span><b>Selected account</b><small>{app.uid ? `UID ${app.uid}` : 'No account selected'}</small></span>
        </label>
        <label class="option">
          <input type="radio" name="scope" value="all" bind:group={scope} />
          <span><b>All accounts</b><small>Every game and UID saved in Wishtory.</small></span>
        </label>
      </fieldset>
    {/if}
    <button class="btn primary" onclick={doExport} disabled={busy}><Icon name="download" />Export…</button>
  </section>
</div>
