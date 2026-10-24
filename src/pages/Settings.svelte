<script lang="ts">
  import { app, setLogging } from '../lib/state.svelte';
</script>

<h1>Settings</h1>
<p class="muted">Preferences for this PC.</p>

<div class="card setting">
  <div class="setting-copy">
    <h2>Logging</h2>
    <p class="muted">Keeps a short in-memory diagnostic log (DEBUG level) and adds a Logging tab. Nothing is written to disk, links are never recorded, and nothing is logged while this is off.</p>
  </div>
  <button
    class="switch" class:on={app.loggingEnabled} role="switch" aria-checked={app.loggingEnabled} aria-label="Enable logging"
    onclick={() => setLogging(!app.loggingEnabled)}><i></i></button>
</div>
