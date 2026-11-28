<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { api, asError } from '../lib/api';
  import { GAMES } from '../lib/types';
  import { INFO, utcLabel, ago, plural } from '../lib/gameinfo';
  import type { Account } from '../lib/types';

  let confirming = $state<string | null>(null);
  let wiping = $state(false);
  const total = $derived(app.accounts.reduce((n, a) => n + a.pull_count, 0));
  async function wipe() {
    try {
      const n = await api.clearAllHistory();
      await app.reloadAccounts();
      await app.loadPity();
      app.toast(`Deleted ${plural(n, 'pull')} from this PC.`, 'ok');
    } catch (e) { app.toast(asError(e).message, 'err'); }
    wiping = false;
  }
  const k = (a: Account) => `${a.game}|${a.uid}`;
  async function remove(a: Account) {
    try { await api.deleteAccount(a.game, a.uid); await app.reloadAccounts(); app.toast('Account removed from this PC.', 'ok'); }
    catch (e) { app.toast(asError(e).message, 'err'); }
    confirming = null;
  }
</script>

<div class="page">
  {#each GAMES as g, i (g)}
    {@const list = app.accounts.filter((a) => a.game === g)}
    <section class="panel pad pop" style="--i:{i}">
      <h2>{INFO[g].name}</h2>
      {#if !list.length}<p class="muted" style="margin:0">No account yet. {INFO[g].howTo}</p>{/if}
      <div class="list">
        {#each list as a (a.uid)}
          <div class="item">
            <div class="grow">
              <input class="field" style="width:240px" value={app.prefs.aliases[k(a)] ?? a.nickname} aria-label="Nickname" placeholder="Nickname"
                onchange={(e) => app.setPrefs({ aliases: { ...app.prefs.aliases, [k(a)]: e.currentTarget.value } })} />
              <div class="muted num" style="margin-top:6px;font-size:12px">UID {a.uid} · {utcLabel(a.timezone)} · {plural(a.pull_count, 'pull')}{app.prefs.lastRefresh[k(a)] ? ' · refreshed ' + ago(app.prefs.lastRefresh[k(a)]) : ''}</div>
            </div>
            {#if confirming === k(a)}
              <span class="muted">Delete its saved pulls?</span>
              <button class="btn sm danger" onclick={() => remove(a)}>Delete</button><button class="btn sm" onclick={() => (confirming = null)}>Keep</button>
            {:else}<button class="btn sm" onclick={() => (confirming = k(a))}>Remove</button>{/if}
          </div>
        {/each}
      </div>
    </section>
  {/each}

  {#if app.accounts.length}
    <section class="panel pad pop" style="--i:3">
      <h2>Your data</h2>
      <p class="muted" style="margin-top:0">Everything Wishtory knows lives on this PC. Nicknames are kept only in this app's settings, not in exports. Export first if you may want your history back.</p>
      {#if wiping}
        <div class="row"><span class="muted">Delete all {plural(total, 'pull')} on every account?</span><button class="btn sm danger" onclick={wipe}>Delete everything</button><button class="btn sm" onclick={() => (wiping = false)}>Keep</button></div>
      {:else}
        <button class="btn sm" onclick={() => (wiping = true)}>Delete all saved history…</button>
      {/if}
    </section>
  {/if}
</div>
