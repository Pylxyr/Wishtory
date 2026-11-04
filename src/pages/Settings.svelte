<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app, checkAppUpdate, openLink, saveGameLocations, setLogging, setThemePref, toast, type ThemePref } from '../lib/state.svelte';
  import { api, errMsg } from '../lib/api';
  import { GAMES } from '../lib/types';
  import Icon from '../lib/components/Icon.svelte';

  const GAME_NAMES = { hk4e: 'Genshin Impact', hkrpg: 'Honkai: Star Rail', nap: 'Zenless Zone Zero' };
  const ROW_LIMIT = 300;
  type Change = 'New' | 'Changed' | 'Removed';
  interface Row { id: string; change: Change; game: string; name: string; featured: string; ran: string; }

  let catalogChecking = $state(false);
  let locationsBusy = $state(false);
  const themes: { id: ThemePref; label: string }[] = [{ id: 'system', label: 'System' }, { id: 'dark', label: 'Dark' }, { id: 'light', label: 'Light' }];

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

  // Game locations (Linux): extra folders that hold Wine prefixes. Wishtory only reads them.
  async function addFolder() {
    const picked = await open({ directory: true, multiple: false, title: 'Choose the folder that holds your Wine prefix' }).catch(() => null);
    if (typeof picked !== 'string') return;
    await changeLocations([...(app.locations?.locations ?? []), picked], picked);
  }
  async function changeLocations(next: string[], added?: string) {
    locationsBusy = true;
    try {
      await saveGameLocations(next);
      if (!added) return;
      const base = added.replace(/\/+$/, '');
      const hit = app.locations?.detected.some((d) => d.path === base || d.path.startsWith(`${base}/`));
      if (hit) toast('ok', `Added ${added}.`);
      else toast('warn', `Added ${added}, but no Wine prefix was found in it. Choose the folder that contains drive_c, or a folder above it.`);
    } catch (e) {
      toast('err', errMsg(e));
    } finally {
      locationsBusy = false;
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
      <p class="muted">System follows your operating system's light or dark setting and changes with it.</p>
    </div>
    <div class="seg" role="group" aria-label="Theme">
      {#each themes as t}<button aria-pressed={app.themePref === t.id} onclick={() => setThemePref(t.id)}>{t.label}</button>{/each}
    </div>
  </div>
  <div class="setting-row">
    <div class="setting-copy">
      <h2>Logging</h2>
      <p class="muted">Keeps a short in-memory diagnostic log (DEBUG level) and adds a Logging page to the sidebar. Nothing is written to disk, links are never recorded, and nothing is logged while this is off.</p>
    </div>
    <button
      class="switch" class:on={app.loggingEnabled} role="switch" aria-checked={app.loggingEnabled} aria-label="Enable logging"
      onclick={() => setLogging(!app.loggingEnabled)}><i></i></button>
  </div>
</div>

{#if app.locations?.supported}
  <section class="settings-group locations" aria-labelledby="locations-title">
    <div class="setting-row">
      <div class="setting-copy">
        <h2 id="locations-title">Game locations</h2>
        <p class="muted">On Linux the games run in Wine or Proton. Wishtory looks in the usual places by itself. If a game is not found, add the folder that holds its Wine prefix (the one that contains <code>drive_c</code>), or a folder above it such as <code>~/Games</code>. Wishtory only reads these folders.</p>
      </div>
      <button class="btn" onclick={addFolder} disabled={locationsBusy}><Icon name="folder" size={16} />Add folder…</button>
    </div>
    {#if app.locations.locations.length}
      <div class="setting-row loc-block">
        <div class="loc-body">
          <h3>Folders you added</h3>
          <ul class="loc-list">
            {#each app.locations.locations as path (path)}
              <li><code>{path}</code><button class="btn" aria-label="Remove {path}" disabled={locationsBusy} onclick={() => changeLocations(app.locations!.locations.filter((p) => p !== path))}>Remove</button></li>
            {/each}
          </ul>
        </div>
      </div>
    {/if}
    <div class="setting-row loc-block">
      <div class="loc-body">
        <h3>Found on this computer</h3>
        {#if app.locations.detected.length}
          <ul class="loc-list">
            {#each app.locations.detected as d (d.path)}
              <li>
                <code>{d.path}</code>
                <span class="loc-games">
                  {#each d.games as g}<span class="chip ok">{GAMES[g].short}</span>{:else}<span class="muted">no game log yet</span>{/each}
                </span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">No Wine prefixes were found yet. Add the folder of yours above.</p>
        {/if}
      </div>
    </div>
  </section>
{/if}

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
        <h2>Game data</h2>
        <p class="muted">
          {#if app.catalog && app.banners}
            Item catalog version {app.catalog.catalog_version} ({app.catalog.items.length.toLocaleString()} items, {app.catalog.generated_at.slice(0, 10)}) and banner list version {app.banners.banners_version} ({app.banners.banners.length.toLocaleString()} events). Both refresh by themselves once a day; this checks both now.
          {:else}
            The game catalog could not be loaded.
          {/if}
        </p>
      </div>
      <div class="row-actions">
        <button class="btn" disabled={catalogChecking} onclick={updateCatalog}>{catalogChecking ? 'Checking…' : 'Check for updates'}</button>
      </div>
    </div>
  </div>
</section>

<section class="banner-section" aria-label="Last banner update">
  <details class="settings-details">
    <summary>
      {#if app.bannerUpdate && app.bannerUpdate.status === 'updated'}
        Last banner update: version {app.bannerUpdate.banners_version}, {app.bannerUpdate.added.length} new, {app.bannerUpdate.changed.length} changed, {app.bannerUpdate.removed.length} removed
      {:else}
        Last banner update
      {/if}
    </summary>
    {#if app.bannerUpdate && app.bannerUpdate.status === 'updated'}
      <p class="muted banner-meta">Installed {installedAt}. History uses the banner list to show which banner each pull was made on.</p>
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
  </details>
</section>
