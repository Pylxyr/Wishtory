<script lang="ts">
  import { app, reloadAccounts, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type Account } from '../lib/types';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let confirming = $state<string | null>(null);
  let clearDialog: HTMLDialogElement;
  let clearing = $state(false);
  let pullCount = $derived(app.accounts.reduce((sum, account) => sum + account.pull_count, 0));

  function openClearDialog() {
    if (app.accounts.length) clearDialog.showModal();
  }

  async function clearAllHistory() {
    if (clearing) return;
    clearing = true;
    const accountCount = app.accounts.length;
    try {
      const deleted = await api.clearAllHistory();
      clearDialog.close();
      await reloadAccounts();
      app.refreshKey++;
      toast('ok', `Cleared ${deleted} pulls across ${accountCount} accounts.`);
    } catch (e) { toast('err', errMsg(e)); }
    finally { clearing = false; }
  }

  async function remove(a: Account) {
    const key = `${a.game}:${a.uid}`;
    if (confirming !== key) { confirming = key; setTimeout(() => (confirming = null), 3500); return; }
    try {
      await api.deleteAccount(a.game, a.uid);
      await reloadAccounts();
      app.refreshKey++;
      toast('ok', `Removed UID ${a.uid} from Wishtory (the game was not touched).`);
    } catch (e) { toast('err', errMsg(e)); }
    confirming = null;
  }
  function open(a: Account) { app.game = a.game; app.uid = a.uid; app.page = 'dashboard'; }
</script>

<h1>Accounts</h1>
<p class="muted">Everything is stored locally on this PC.</p>

<section class="card clear-history-card" aria-labelledby="clear-history-heading">
  <div>
    <h2 id="clear-history-heading">Clear all history</h2>
    <p class="muted">Remove every saved pull and account from this PC.</p>
  </div>
  <button class="btn danger" disabled={!app.accounts.length} onclick={openClearDialog}>
    <Icon name="trash" size={16} />Clear all history
  </button>
</section>

<dialog bind:this={clearDialog} class="clear-history-dialog" aria-labelledby="clear-dialog-title" aria-describedby="clear-dialog-description" oncancel={(event) => { if (clearing) event.preventDefault(); }}>
  <div class="clear-history-dialog-content">
    <span class="clear-history-label">Permanent action</span>
    <h2 id="clear-dialog-title">Delete all saved history?</h2>
    <p id="clear-dialog-description">This will permanently delete {pullCount} saved pulls across {app.accounts.length} accounts and all games from Wishtory on this PC. This can’t be undone. Export a backup first if you may want this data later.</p>
    <div class="row clear-history-actions">
      <button class="btn" disabled={clearing} onclick={() => clearDialog.close()}>Keep my history</button>
      <button class="btn danger" disabled={clearing} onclick={clearAllHistory}>{clearing ? 'Clearing…' : 'Delete all history'}</button>
    </div>
  </div>
</dialog>

{#if !app.accounts.length}
  <EmptyState title="No accounts yet" text="Accounts appear automatically after your first Refresh or import." />
{:else}
  <div class="grid">
    {#each app.accounts as a, i (a.game + a.uid)}
      <div class="card hover rise" style="animation-delay:{i * 50}ms">
        <h3>{GAMES[a.game].name}</h3>
        <p class="muted">UID {a.uid} · UTC{a.timezone >= 0 ? '+' : ''}{a.timezone} · {a.pull_count} pulls</p>
        <div class="row">
          <button class="btn" onclick={() => open(a)}>Open</button>
          <button class="btn" class:danger={confirming === `${a.game}:${a.uid}`} onclick={() => remove(a)}>
            <Icon name="trash" size={16} />{confirming === `${a.game}:${a.uid}` ? 'Click again to confirm' : 'Remove'}
          </button>
        </div>
      </div>
    {/each}
  </div>
{/if}
