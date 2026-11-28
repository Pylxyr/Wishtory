<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { ago, plural } from '../lib/gameinfo';
  import type { CloudBackup, CloudStatus } from '../lib/types';

  let { index = 0 }: { index?: number } = $props();

  let st = $state<CloudStatus | null>(null);
  let passphrase = $state('');
  let again = $state('');
  let signingIn = $state(false);
  let busy = $state(false);
  let backups = $state<CloudBackup[] | null>(null);
  let restoring = $state<string | null>(null);
  let restorePass = $state('');
  let disconnecting = $state(false);
  let deleteRemote = $state(false);

  const problem = $derived(
    !passphrase ? null : [...passphrase].length < 10 ? 'Use at least 10 characters.' : again && again !== passphrase ? 'The two passphrases differ.' : null
  );
  const ready = $derived([...passphrase].length >= 10 && again === passphrase);

  $effect(() => {
    api.cloudStatus().then((s) => (st = s)).catch((e) => app.toast(asError(e).message, 'err'));
  });

  async function signIn() {
    signingIn = true;
    try {
      st = await api.cloudSignIn(passphrase);
      passphrase = again = '';
      app.toast('Connected to Google Drive.', 'ok');
    } catch (e) {
      app.toast(asError(e).message, 'err');
    }
    signingIn = false;
  }
  const cancelSignIn = () => api.cloudCancelSignIn().catch(() => {});

  async function backupNow() {
    busy = true;
    try {
      await api.cloudBackupNow();
      st = await api.cloudStatus();
      if (backups) backups = await api.cloudListBackups();
      app.toast('Backed up.', 'ok');
    } catch (e) {
      app.toast(asError(e).message, 'err');
    }
    busy = false;
  }
  async function toggleAuto(on: boolean) {
    try { st = await api.cloudSetAutoBackup(on); } catch (e) { app.toast(asError(e).message, 'err'); st = await api.cloudStatus(); }
  }
  async function showBackups() {
    try { backups = await api.cloudListBackups(); } catch (e) { app.toast(asError(e).message, 'err'); }
  }
  async function restore(id: string) {
    busy = true;
    try {
      const r = await api.cloudRestore(id, restorePass || null);
      await app.reloadAccounts();
      await app.loadPity();
      app.toast(`Restored ${plural(r.added, 'new pull')} across ${plural(r.accounts, 'account')}.`, 'ok');
      restoring = null;
      restorePass = '';
    } catch (e) {
      app.toast(asError(e).message, 'err');
    }
    busy = false;
  }
  async function disconnect() {
    try {
      st = await api.cloudSignOut(deleteRemote);
      backups = null;
      disconnecting = false;
      deleteRemote = false;
      app.toast('Disconnected.', 'ok');
    } catch (e) {
      app.toast(asError(e).message, 'err');
    }
  }
  const size = (n: number) => (n < 1024 * 1024 ? `${Math.max(1, Math.round(n / 1024))} KB` : `${(n / 1024 / 1024).toFixed(1)} MB`);
</script>

{#if st}
  <section class="panel pad pop" style="--i:{index}">
    <h2>Cloud backup</h2>
    {#if !st.available}
      <p class="muted" style="margin:0">This build of Wishtory has no Google sign-in set up, so cloud backup is not offered. Your history still lives only on this PC; use Import / Export to keep a copy yourself.</p>
    {:else if !st.signed_in}
      <p class="muted" style="margin-top:0">Keep an encrypted copy of your history in your own Google Drive. Wishtory can only see the files it creates there, and encrypts the backup on this PC with a passphrase that never leaves it.</p>
      <div class="panel lit pad" style="margin-bottom:12px"><b>Keep the passphrase safe.</b> Nobody, including us, can read a backup without it, and it cannot be recovered.</div>
      <div class="list">
        <input class="field" type="password" autocomplete="new-password" placeholder="Backup passphrase" aria-label="Backup passphrase" bind:value={passphrase} disabled={signingIn} />
        <input class="field" type="password" autocomplete="new-password" placeholder="Type it again" aria-label="Backup passphrase again" bind:value={again} disabled={signingIn} />
      </div>
      {#if problem}<p class="muted" role="alert" style="margin:8px 0 0">{problem}</p>{/if}
      <div class="row" style="margin-top:12px">
        <button class="btn primary sm" disabled={!ready || signingIn} onclick={signIn}>{signingIn ? 'Waiting for your browser…' : 'Sign in with Google'}</button>
        {#if signingIn}<button class="btn sm" onclick={cancelSignIn}>Cancel</button>{/if}
      </div>
    {:else}
      <p class="muted" style="margin-top:0">Connected as <b>{st.account ?? 'your Google account'}</b>. {st.last_backup ? `Last backup ${ago(Date.parse(st.last_backup))}.` : 'No backup yet.'}</p>
      <div class="row" style="margin-bottom:6px">
        <button class="btn primary sm" disabled={busy} onclick={backupNow}>{busy ? 'Working…' : 'Back up now'}</button>
        <button class="btn sm" onclick={showBackups}>{backups ? 'Reload list' : 'Show backups'}</button>
      </div>
      <label class="switch"><div><b>Back up automatically</b><div class="muted">About once a day while Wishtory is open. The latest few backups are kept.</div></div><input type="checkbox" checked={st.auto_backup} onchange={(e) => toggleAuto(e.currentTarget.checked)} /></label>
      {#if backups}
        {#if !backups.length}<p class="muted">No backups in your Drive yet.</p>{/if}
        <div class="list">
          {#each backups as b (b.id)}
            <div class="item">
              <div class="grow num">{b.created.replace('T', ' ').slice(0, 19)} <span class="muted">· {size(b.size)}</span></div>
              {#if restoring === b.id}
                <input class="field" type="password" placeholder="Passphrase (blank = saved one)" aria-label="Passphrase for this backup" bind:value={restorePass} />
                <button class="btn sm primary" disabled={busy} onclick={() => restore(b.id)}>Restore</button>
                <button class="btn sm" onclick={() => { restoring = null; restorePass = ''; }}>Cancel</button>
              {:else}
                <button class="btn sm" onclick={() => (restoring = b.id)}>Restore…</button>
              {/if}
            </div>
          {/each}
        </div>
        <p class="muted" style="font-size:12px">Restoring adds the pulls from a backup to what you already have. It never removes anything.</p>
      {/if}
      <hr class="rule" />
      {#if disconnecting}
        <label class="row" style="gap:8px"><input type="checkbox" bind:checked={deleteRemote} />Also delete my backups from Google Drive</label>
        <div class="row" style="margin-top:10px"><button class="btn sm danger" onclick={disconnect}>Disconnect</button><button class="btn sm" onclick={() => (disconnecting = false)}>Keep connected</button></div>
      {:else}
        <button class="btn sm" onclick={() => (disconnecting = true)}>Disconnect…</button>
      {/if}
    {/if}
  </section>
{/if}
