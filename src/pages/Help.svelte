<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { GAMES } from '../lib/types';
  import Icon from '../lib/components/Icon.svelte';
</script>

<div class="page-head">
  <h1>Help</h1>
  <p>The workflow in two steps.</p>
</div>

<div class="help-grid">
  <div class="help-stack">
    <section class="card" aria-labelledby="workflow-title">
      <h2 id="workflow-title">Get your history into Wishtory</h2>
      <div class="steps">
        <div class="step"><i>1</i><div><b>Open the history page in-game.</b><br /><span class="muted">Wait until the list has loaded. This makes the game save a temporary link in its own cache.</span></div></div>
        <div class="step"><i>2</i><div><b>Click Refresh in Wishtory.</b><br /><span class="muted">Wishtory reads that cache (read-only) and downloads your history.</span></div></div>
      </div>
      <p style="margin:14px 0 0"><button class="btn" onclick={() => (app.tutorial = true)}>Replay the tour</button></p>
    </section>

    <section class="card" aria-labelledby="games-title">
      <h2 id="games-title">Where the history page is</h2>
      <ul class="how-to">
        {#each Object.values(GAMES) as g}
          <li><b>{g.name}</b><span class="muted">{g.howTo}</span></li>
        {/each}
      </ul>
    </section>

    <section class="card" aria-labelledby="trouble-title">
      <h2 id="trouble-title">Troubleshooting</h2>
      <details class="fold">
        <summary>“History page not found”</summary>
        <p class="muted">Open the history page in-game again, let it load fully, then Refresh.</p>
      </details>
      <details class="fold">
        <summary>“Link has expired”</summary>
        <p class="muted">The link only lives for a limited time. Reopen the history page in-game. If a download stops part-way, the banners that finished are saved; click Refresh again for the rest.</p>
      </details>
      <details class="fold">
        <summary>“The server returned an error” (for example “auth key time out”)</summary>
        <p class="muted">The history link the game saved has timed out. Open the history page in-game again, wait until the list loads, then click Refresh straight away. Wishtory tries your saved links newest first, so a fresh page view is usually enough.</p>
      </details>
      <details class="fold">
        <summary>“Game not detected”</summary>
        <p class="muted">Launch the game once so it writes its log file. Wishtory finds the install folder from that log.</p>
      </details>
      <details class="fold">
        <summary>Old wishes missing?</summary>
        <p class="muted">The game only exposes about 6 months. Import an older UIGF export to keep the rest.</p>
      </details>
    </section>
  </div>

  <section class="card" aria-labelledby="safety-title">
    <h2 id="safety-title"><Icon name="shield" size={16} /> Safety model and disclaimer</h2>
    <p class="muted">Wishtory is designed to be as safe as the well-known community tools. It reads only files the game already wrote (the log file and the web cache) and then talks to the official history API, the same service the in-game history page uses.</p>
    <ul class="muted plain">
      <li>It never attaches to, injects into or reads the memory of the game process.</li>
      <li>It never modifies, locks or writes to any game file or folder.</li>
      <li>It never sets a system proxy and never uses man-in-the-middle techniques.</li>
      <li>It never needs administrator rights.</li>
    </ul>
    <p class="muted">No tool can offer a 100% guarantee: HoYoverse may change its terms or its detection at any time. Use Wishtory at your own risk. It is an unofficial fan project and is not affiliated with or endorsed by HoYoverse or COGNOSPHERE.</p>
  </section>
</div>
