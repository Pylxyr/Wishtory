<script lang="ts">
  import { onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { app, initTheme, reloadAccounts } from './lib/state.svelte';
  import { api } from './lib/api';
  import { dur } from './lib/motion';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import Tutorial from './lib/components/Tutorial.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import History from './pages/History.svelte';
  import Accounts from './pages/Accounts.svelte';
  import ImportExport from './pages/ImportExport.svelte';
  import Help from './pages/Help.svelte';

  onMount(async () => {
    initTheme();
    app.installs = await api.discoverGames().catch(() => []);
    await reloadAccounts().catch(() => {});
    if (!localStorage.getItem('wishtory.tutorialDone')) app.tutorial = true;
  });
</script>

<div class="shell">
  <Sidebar />
  <main>
    {#key app.page}
      <div class="page" in:fly={{ y: 12, duration: dur(220) }}>
        {#if app.page === 'dashboard'}<Dashboard />
        {:else if app.page === 'history'}<History />
        {:else if app.page === 'accounts'}<Accounts />
        {:else if app.page === 'importexport'}<ImportExport />
        {:else}<Help />{/if}
      </div>
    {/key}
  </main>
</div>
<Toasts />
{#if app.tutorial}<Tutorial />{/if}
