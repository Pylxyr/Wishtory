<script lang="ts">
  import { app, reloadAccounts, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES, type Account } from '../lib/types';
  import EmptyState from '../lib/components/EmptyState.svelte';
  import Icon from '../lib/components/Icon.svelte';

  let confirming = $state<string | null>(null);

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
