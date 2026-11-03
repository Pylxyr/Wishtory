<script lang="ts">
  import { accountLabel, app, forgetAccounts, lastRefreshOf, reloadAccounts, selectGame, setAlias, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type Account } from '../lib/types';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import { ago, plural } from '../lib/format';
  import Icon from '../lib/components/Icon.svelte';

  let confirming = $state<string | null>(null);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;
  let editing = $state<string | null>(null); // the account whose label is being edited
  let draft = $state('');
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
      forgetAccounts();
      toast('ok', `Cleared ${plural(deleted, 'pull')} across ${plural(accountCount, 'account')}.`);
    } catch (e) { toast('err', errMsg(e)); }
    finally { clearing = false; }
  }

  async function remove(a: Account) {
    const key = `${a.game}:${a.uid}`;
    if (confirming !== key) {
      confirming = key;
      clearTimeout(confirmTimer); // one timer, so a second account never inherits the first one's countdown
      confirmTimer = setTimeout(() => (confirming = null), 5000);
      return;
    }
    clearTimeout(confirmTimer);
    try {
      await api.deleteAccount(a.game, a.uid);
      await reloadAccounts();
      app.refreshKey++;
      forgetAccounts({ game: a.game, uid: a.uid });
      toast('ok', `Removed UID ${a.uid} from Wishtory (the game was not touched).`);
    } catch (e) { toast('err', errMsg(e)); }
    confirming = null;
  }
  function open(a: Account) { selectGame(a.game); app.uid = a.uid; app.lastUid[a.game] = a.uid; app.page = 'dashboard'; }
  function startEdit(a: Account) { editing = `${a.game}:${a.uid}`; draft = app.aliases[editing] ?? ''; }
  function saveEdit(a: Account) { setAlias(a.game, a.uid, draft); editing = null; }
</script>

<div class="page-head">
  <h1>Accounts</h1>
  <p>Everything is stored locally on this PC.</p>
</div>

{#if !app.accounts.length}
  <EmptyState title="No accounts yet" text="Accounts appear automatically after your first Refresh or import." />
{:else}
  <div class="account-list">
    {#each app.accounts as a (a.game + a.uid)}
      {@const key = `${a.game}:${a.uid}`}
      {@const label = accountLabel(a)}
      {@const refreshed = lastRefreshOf(a.game, a.uid)}
      <div class="account" data-game={a.game}>
        <div class="account-main">
          <strong>{GAMES[a.game].name}</strong>
          {#if editing === key}
            <form class="label-edit" onsubmit={(e) => { e.preventDefault(); saveEdit(a); }}>
              <input bind:value={draft} maxlength="40" placeholder="Label, for example Main" aria-label="Label for UID {a.uid}" onkeydown={(e) => e.key === 'Escape' && (editing = null)} />
              <button class="btn primary" type="submit">Save</button>
              <button class="btn" type="button" onclick={() => (editing = null)}>Cancel</button>
            </form>
          {:else}
            <span class="muted">UID {a.uid}{label ? `, ${label}` : ''}</span>
          {/if}
        </div>
        <span class="account-meta">{plural(a.pull_count, 'pull')}, UTC{a.timezone >= 0 ? '+' : ''}{a.timezone}{refreshed ? `, updated ${ago(refreshed)}` : ''}</span>
        <div class="account-actions">
          <button class="btn" onclick={() => open(a)}>Open</button>
          <button class="btn" onclick={() => startEdit(a)} aria-label="Rename UID {a.uid}"><Icon name="edit" size={16} />Label</button>
          <button class="btn" class:danger={confirming === key} onclick={() => remove(a)}>
            <Icon name="trash" size={16} />{confirming === key ? 'Click again to confirm' : 'Remove'}
          </button>
        </div>
      </div>
    {/each}
  </div>
  <span class="sr" role="status">{confirming ? 'Select Remove again within five seconds to confirm.' : ''}</span>
{/if}

<section class="danger-zone" aria-labelledby="clear-history-heading">
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
    <p id="clear-dialog-description">This will permanently delete {plural(pullCount, 'saved pull')} across {plural(app.accounts.length, 'account')} and all games from Wishtory on this PC. This can’t be undone. Export a backup first if you may want this data later.</p>
    <div class="row clear-history-actions">
      <button class="btn" disabled={clearing} onclick={() => clearDialog.close()}>Keep my history</button>
      <button class="btn danger" disabled={clearing} onclick={clearAllHistory}>{clearing ? 'Clearing…' : 'Delete all history'}</button>
    </div>
  </div>
</dialog>
