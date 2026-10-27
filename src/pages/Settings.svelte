<script lang="ts">
  import { app, checkAppUpdate, openLink, setLogging, setTheme, toast } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';

  const GAME_NAMES = { hk4e: 'Genshin Impact', hkrpg: 'Honkai: Star Rail', nap: 'Zenless Zone Zero' };
  const ROW_LIMIT = 300;
  type Change = 'New' | 'Changed' | 'Removed';
  interface Row { id: string; change: Change; game: string; name: string; featured: string; ran: string; }

  let checking = $state(false);
  let catalogChecking = $state(false);

  const itemNames = $derived(new Map((app.catalog?.items ?? []).map((i) => [`${i.game}:${i.id}`, i.name])));
  // What the last installed update did, newest events first. A removed banner is gone from the list,
  // so only its id is known.
  const rows = $derived.by((): Row[] => {
    const update = app.bannerUpdate, set = app.banners;
    if (!update || !set) return [];
    const byId = new Map(set.banners.map((b) => [b.id, b]));
    const row = (id: string, change: Change): Row => {
      const b = byId.get(id);
      if (!b) return { id, change, game: '', name: id, featured: '', ran: '' };
      return {
        id, change, game: GAME_NAMES[b.game], name: b.name,
        featured: b.featured_5.map((f) => itemNames.get(`${b.game}:${f}`) ?? f).join(' / '),
        ran: `${b.start.slice(0, 10)} → ${b.end.slice(0, 10)}`
      };
    };
    return [
      ...update.added.map((id) => row(id, 'New')),
      ...update.changed.map((id) => row(id, 'Changed')),
      ...update.removed.map((id) => row(id, 'Removed'))
    ].sort((a, b) => b.ran.localeCompare(a.ran));
  });
  const installedAt = $derived(app.bannerUpdate && app.bannerUpdate.installed_at ? new Date(app.bannerUpdate.installed_at * 1000).toLocaleString() : '');

  function openRelease() {
    const url = app.update?.url;
    if (url) void openLink(url);
  }

  // Item catalog and banner list in one go. Both normally refresh by themselves once a day.
  async function updateCatalog() {
    catalogChecking = true;
    try {
      const itemStatus = await api.refreshCatalog(true);
      if (itemStatus === 'updated') app.catalog = await api.getCatalog();
      const banners = await api.refreshBanners(true);
      if (banners.status === 'updated') {
        app.banners = await api.getBanners();
        app.bannerUpdate = banners;
        app.refreshKey++;
      }
      if (itemStatus === 'already-checking' || banners.status === 'already-checking') toast('info', 'A catalog check is already running.');
      else if (itemStatus === 'updated' || banners.status === 'updated') toast('ok', 'The game catalog was updated.');
      else toast('ok', 'The game catalog is up to date.');
    } catch (e) {
      toast('warn', `Could not check for catalog updates: ${errMsg(e)}`);
    } finally {
      catalogChecking = false;
    }
  }

  async function updateBanners() {
    checking = true;
    try {
      const update = await api.refreshBanners(true);
      if (update.status === 'updated') {
        app.banners = await api.getBanners();
        app.bannerUpdate = update;
        app.refreshKey++;
        toast('ok', `Banner list updated to version ${update.banners_version}: ${update.added.length} new, ${update.changed.length} changed.`);
      } else if (update.status === 'already-checking') {
        toast('info', 'A banner check is already running.');
      } else {
        toast('ok', 'The banner list is up to date.');
      }
    } catch (e) {
      toast('warn', `Could not check for banner updates: ${errMsg(e)}`);
    } finally {
      checking = false;
    }
  }
</script>

<div class="page-head">
  <h1>Settings</h1>
  <p>Preferences for this PC.</p>
</div>

<div class="settings-group">
  <div class="setting-row">
    <div class="setting-copy">
      <h2>Theme</h2>
      <p class="muted">Choose the colour scheme.</p>
    </div>
    <div class="seg" role="group" aria-label="Theme">
      <button aria-pressed={app.theme === 'dark'} onclick={() => setTheme('dark')}>Dark</button>
      <button aria-pressed={app.theme === 'light'} onclick={() => setTheme('light')}>Light</button>
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-copy">
      <h2>Logging</h2>
      <p class="muted">Keeps a short in-memory diagnostic log (DEBUG level) and adds a Logging tab. Nothing is written to disk, links are never recorded, and nothing is logged while this is off.</p>
    </div>
    <button
      class="switch" class:on={app.loggingEnabled} role="switch" aria-checked={app.loggingEnabled} aria-label="Enable logging"
      onclick={() => setLogging(!app.loggingEnabled)}><i></i></button>
  </div>
</div>

<section class="banner-section" aria-label="Updates">
  <div class="settings-group">
    <div class="setting-row">
      <div class="setting-copy">
        <h2>App updates</h2>
        <p class="muted">
          {#if app.update?.available}
            Wishtory {app.update.latest}{app.update.prerelease ? ' (pre-release)' : ''} is available; you have {app.update.current}. It opens the release page in your browser, and nothing is installed automatically.
          {:else if app.update}
            You have the latest version, {app.update.current}.
          {:else}
            You have version {app.about?.version ?? '…'}. Checked at most once a day against this project's GitHub releases. Nothing about you or your wishes is sent.
          {/if}
        </p>
      </div>
      <div class="row-actions">
        {#if app.update?.available}<button class="btn primary" onclick={openRelease}>View release</button>{/if}
        <button class="btn" disabled={app.updateChecking} onclick={() => checkAppUpdate(true)}>{app.updateChecking ? 'Checking…' : 'Check for updates'}</button>
      </div>
    </div>
    <div class="setting-row">
      <div class="setting-copy">
        <h2>Catalog updates</h2>
        <p class="muted">
          {#if app.catalog && app.banners}
            Item catalog version {app.catalog.catalog_version} ({app.catalog.items.length.toLocaleString()} items, {app.catalog.generated_at.slice(0, 10)}) and banner list version {app.banners.banners_version}. Both refresh by themselves once a day; this checks both now.
          {:else}
            The game catalog could not be loaded.
          {/if}
        </p>
      </div>
      <div class="row-actions">
        <button class="btn" disabled={catalogChecking || checking} onclick={updateCatalog}>{catalogChecking ? 'Checking…' : 'Check for updates'}</button>
      </div>
    </div>
  </div>
</section>

<section class="banner-section" aria-labelledby="banner-heading">
  <div class="settings-group">
    <div class="setting-row">
      <div class="setting-copy">
        <h2 id="banner-heading">Banner list</h2>
        <p class="muted">
          {#if app.banners}
            Version {app.banners.banners_version} with {app.banners.banners.length.toLocaleString()} banner events. It is checked once a day, and History uses it to show which banner each pull was made on.
          {:else}
            The banner list could not be loaded.
          {/if}
        </p>
      </div>
      <button class="btn primary" disabled={checking} onclick={updateBanners}>{checking ? 'Checking…' : 'Update banners'}</button>
    </div>
  </div>
  {#if app.bannerUpdate && app.bannerUpdate.status === 'updated'}
    <p class="muted banner-meta">Last update: version {app.bannerUpdate.banners_version}, installed {installedAt}. {app.bannerUpdate.added.length} new, {app.bannerUpdate.changed.length} changed, {app.bannerUpdate.removed.length} removed.</p>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Change</th><th>Game</th><th>Banner</th><th>Featured</th><th>Ran</th></tr></thead>
        <tbody>
          {#each rows.slice(0, ROW_LIMIT) as r (r.change + r.id)}
            <tr>
              <td><span class="chip" class:ok={r.change === 'New'} class:err={r.change === 'Removed'}>{r.change}</span></td>
              <td class="muted">{r.game}</td>
              <td class="item">{r.name}</td>
              <td class="featured">{r.featured}</td>
              <td class="muted">{r.ran}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if rows.length > ROW_LIMIT}<p class="muted banner-meta">Showing the newest {ROW_LIMIT} of {rows.length.toLocaleString()} changes.</p>{/if}
  {:else}
    <p class="muted banner-meta">No banner update has been installed yet. The list that came with this version is in use.</p>
  {/if}
</section>
