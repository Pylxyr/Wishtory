<script lang="ts">
  import { app, setLogging, setTheme } from '../lib/state.svelte';
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
