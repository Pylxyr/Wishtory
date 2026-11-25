<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { plural } from '../lib/gameinfo';
  import type { GameId, ImportSummary } from '../lib/types';
  import Glyph from '../components/Glyph.svelte';

  const FORMATS: { id: string; name: string; note: string; game: GameId | null; file: string }[] = [
    { id: 'uigf4', name: 'UIGF v4.2', note: 'Recommended. Covers every game and account.', game: null, file: 'uigf-v4.2' },
    { id: 'uigf3', name: 'UIGF v3.0', note: 'Genshin only. For Paimon.moe and older tools.', game: 'genshin', file: 'uigf-v3.0' },
    { id: 'srgf1', name: 'SRGF v1.0', note: 'Star Rail only.', game: 'star_rail', file: 'srgf-v1.0' },
    { id: 'paimon', name: 'Paimon.moe local wish data', note: 'Genshin only.', game: 'genshin', file: 'paimon-moe-wishes' }
  ];
  let format = $state('uigf4');
  let all = $state(true);
  let result = $state<ImportSummary | null>(null);
  let busy = $state(false);
  const usable = (f: (typeof FORMATS)[number]) => f.game === null || (app.account?.game === f.game);

  async function doImport() {
    const path = await api.pickOpenPath();
    if (!path) return;
    busy = true;
    try { result = await api.importFile(path); await app.reloadAccounts(); } catch (e) { app.toast(asError(e).message, 'err'); }
    busy = false;
  }
  async function doExport() {
    const f = FORMATS.find((x) => x.id === format)!;
    const single = !(all && format === 'uigf4');
    const path = await api.pickSavePath(`${f.file}.json`, 'json');
    if (!path) return;
    try {
      await api.exportFile(path, format, single ? (app.account?.game ?? null) : null, single ? (app.account?.uid ?? null) : null);
      app.toast('Export saved.', 'ok');
    } catch (e) { app.toast(asError(e).message, 'err'); }
  }
</script>

<div class="page">
  <section class="panel pad pop">
    <h2>Import</h2>
    <p class="muted" style="margin-top:0">UIGF, SRGF and Paimon.moe files. Pulls you already have are skipped, so importing twice is safe.</p>
    <div class="row"><button class="btn primary" disabled={busy} onclick={doImport}><Glyph name="import" size={16} />Choose a file…</button>
      {#if result}<span class="ok">Imported {result.format}: {plural(result.added, 'new pull')} across {plural(result.accounts, 'account')}{result.skipped ? `, ${result.skipped} skipped` : ''}.</span>{/if}</div>
  </section>
  <section class="panel pad pop" style="--i:1">
    <h2>Export</h2>
    <div class="list">
      {#each FORMATS as f (f.id)}
        <label class="item" style:opacity={usable(f) ? 1 : 0.45}><input type="radio" name="fmt" value={f.id} bind:group={format} disabled={!usable(f)} /><div class="grow"><b>{f.name}</b><div class="muted">{f.note}</div></div></label>
      {/each}
    </div>
    {#if format === 'uigf4'}<label class="row" style="margin:8px 0"><input type="checkbox" bind:checked={all} />All accounts of every game</label>{/if}
    <button class="btn primary" onclick={doExport} disabled={!app.account}>Export…</button>
  </section>
</div>
