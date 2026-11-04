<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { app, checkAppUpdate, initLogging, initProgress, reloadAccounts, restoreView, saveView, type Page } from './lib/state.svelte';
  import { api } from './lib/api';
  import { dur } from './lib/motion';
  import ErrorCard from './lib/components/ErrorCard.svelte';
  import PageBar from './lib/components/PageBar.svelte';
  import Rail from './lib/components/Rail.svelte';
  import StatusBar from './lib/components/StatusBar.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import Tutorial from './lib/components/Tutorial.svelte';
  import Dashboard from './pages/Dashboard.svelte';
  import History from './pages/History.svelte';
  import Accounts from './pages/Accounts.svelte';
  import ImportExport from './pages/ImportExport.svelte';
  import Help from './pages/Help.svelte';
  import Settings from './pages/Settings.svelte';
  import About from './pages/About.svelte';
  import Logging from './pages/Logging.svelte';

  // Pages that work on one game and account show the game and account bar with the Refresh buttons.
  const gamePages: Page[] = ['dashboard', 'history', 'importexport'];

  // Comes back to the page, game and account the person last used. Runs before the effects below.
  restoreView();
  $effect(() => saveView());

  let main: HTMLElement;
  $effect(() => {
    void app.page;
    main?.scrollTo(0, 0);
  });
  function skipToContent(event: MouseEvent) {
    event.preventDefault();
    main.focus();
  }

  onMount(async () => {
    initProgress();
    await initLogging();
    app.about = await api.getAbout().catch(() => null);
    app.meta = await api.getGameMeta().catch(() => []);
    app.catalog = await api.getCatalog().catch(() => null);
    // Best-effort data-only refresh; the bundled catalog remains available offline.
    void api.refreshCatalog(false).then(async (status) => {
      if (status === 'updated') app.catalog = await api.getCatalog();
    }).catch(() => {});
    app.banners = await api.getBanners().catch(() => null);
    app.bannerUpdate = await api.getBannerUpdate().catch(() => null);
    // Same daily, best-effort refresh for the signed banner list.
    void api.refreshBanners(false).then(async (update) => {
      if (update.status !== 'updated') return;
      app.banners = await api.getBanners();
      app.bannerUpdate = update;
      app.refreshKey++; // saved pulls were matched against the new list: reload what pages show
    }).catch(() => {});
    void checkAppUpdate(false); // at most once a day; only compares version numbers
    app.installs = await api.discoverGames().catch(() => []);
    app.locations = await api.getGameLocations().catch(() => null);
    await reloadAccounts().catch(() => {});
    if (!localStorage.getItem('wishtory.tutorialDone')) app.tutorial = true;
  });
</script>

<a class="skip" href="#main" onclick={skipToContent}>Skip to content</a>
<div class="shell">
  <Rail />
  <div class="work">
    {#if gamePages.includes(app.page)}<PageBar />{/if}
    <main id="main" tabindex="-1" bind:this={main}>
      {#if gamePages.includes(app.page) && app.result?.kind === 'err' && app.result.game === app.game}
        <div class="page"><ErrorCard /></div>
      {/if}
      {#key app.page}
        <div class="page" in:fade={{ duration: dur(120) }}>
          {#if app.page === 'dashboard'}<Dashboard />
          {:else if app.page === 'history'}<History />
          {:else if app.page === 'accounts'}<Accounts />
          {:else if app.page === 'importexport'}<ImportExport />
          {:else if app.page === 'settings'}<Settings />
          {:else if app.page === 'about'}<About />
          {:else if app.page === 'logging' && app.loggingEnabled}<Logging />
          {:else}<Help />{/if}
        </div>
      {/key}
    </main>
    <StatusBar />
  </div>
</div>
<Toasts />
{#if app.tutorial}<Tutorial />{/if}
