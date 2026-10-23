<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { GAMES } from '../lib/types';
  import Icon from '../lib/components/Icon.svelte';
</script>

<h1>Help</h1>
<p class="muted">The safe workflow in two steps.</p>

<div class="card rise">
  <div class="steps" style="margin:0">
    <div class="step"><i>1</i><div><b>Open the history page in-game.</b><br /><span class="muted">Wait until the list has loaded. This makes the game save a temporary link in its own cache.</span></div></div>
    <div class="step"><i>2</i><div><b>Click Refresh in Wishtory.</b><br /><span class="muted">Wishtory reads that cache (read-only) and downloads your history.</span></div></div>
  </div>
  <p style="margin-top:14px"><button class="btn" onclick={() => (app.tutorial = true)}>Replay the tour</button></p>
</div>

<div class="grid" style="margin-top:16px">
  {#each Object.values(GAMES) as g, i}
    <div class="card rise" style="animation-delay:{i * 60}ms"><h3>{g.name}</h3><p class="muted">{g.howTo}</p></div>
  {/each}
</div>

<div class="card rise" style="margin-top:16px">
  <h2><Icon name="shield" size={16} /> Why this is safe</h2>
  <p class="muted">Wishtory never connects to the game, never edits its files, never sets a proxy, and never asks for admin rights. It only reads the log file and the cache file the game already wrote, then talks to the official history API, the same thing the in-game page does.</p>
</div>

<div class="card rise" style="margin-top:16px">
  <h2>Troubleshooting</h2>
  <p><b>"History page not found"</b><br /><span class="muted">Open the history page in-game again, let it load fully, then Refresh.</span></p>
  <p><b>"Link has expired"</b><br /><span class="muted">The link only lives for about a day. Reopen the history page in-game.</span></p>
  <p><b>"Game not detected"</b><br /><span class="muted">Launch the game once so it writes its log file. Wishtory finds the install folder from that log.</span></p>
  <p><b>Old wishes missing?</b><br /><span class="muted">The game only exposes about 6 months. Import an older UIGF export to keep the rest.</span></p>
</div>
