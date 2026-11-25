<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { seq } from '../lib/motion';

  // A quiet decoration behind the page, drawn at a few percent, so each game feels like its own place
  // without getting in the way of the numbers. It takes no clicks. Genshin Impact: a warm glow and
  // sparkles that rise and twinkle. Star Rail: orbit rings that turn, stars, a meteor now and then.
  // Zenless Zone Zero: CRT scanlines, a rolling band, marching stripes and a halftone.
  const sparkles = (() => {
    const r = seq(7);
    return Array.from({ length: 26 }, (_, i) => {
      const big = r() > 0.8;
      const x = 25 + 75 * r(), y = 100 * r() * (0.35 + 0.65 * r());
      return { x, y, s: big ? 18 + 10 * r() : 6 + 6 * r(), o: big ? 0.5 : 0.32, d: 5 + 5 * r(), delay: -10 * r(), alt: r() > 0.5, i };
    });
  })();
  const stars = (() => {
    const r = seq(41);
    return Array.from({ length: 90 }, () => ({ x: 100 * r(), y: 100 * r(), b: r() > 0.85, d: 3 + 5 * r(), delay: -8 * r() }));
  })();
</script>

<div class="backdrop" aria-hidden="true">
  {#if app.game === 'genshin'}
    <div class="glow"></div>
    {#each sparkles as s (s.i)}
      <span class="sparkle" class:alt={s.alt} style="left:{s.x}%;top:{s.y}%;width:{s.s}px;height:{s.s}px;--d:{s.d}s;--delay:{s.delay}s;--o:{s.o}">
        <svg viewBox="0 0 24 24"><path d="M12 0c.6 7 3 10.4 12 12-9 1.6-11.4 5-12 12-.6-7-3-10.4-12-12C9 10.4 11.4 7 12 0Z" fill="currentColor" /></svg>
      </span>
    {/each}
  {:else if app.game === 'star_rail'}
    {#each [0, 1, 2, 3] as i (i)}
      <div class="orbit" class:rev={i % 2 === 1} style="width:{76 + 40 * i}vh;margin-right:{(76 + 40 * i) / -2 + 20}vh;margin-top:{-(40 * i) / 2}vh;--t:{150 + 60 * i}s"></div>
    {/each}
    {#each stars as s, i (i)}
      <span class="star" class:b={s.b} style="left:{s.x}%;top:{s.y}%;width:{s.b ? 2.6 : 1.6}px;height:{s.b ? 2.6 : 1.6}px;--d:{s.d}s;--delay:{s.delay}s"></span>
    {/each}
    <span class="meteor" style="left:88%;top:6%;--delay:2s"></span>
    <span class="meteor" style="left:70%;top:30%;--delay:7s"></span>
    <span class="meteor" style="left:96%;top:52%;--delay:11s"></span>
  {:else}
    <div class="scan"></div>
    <div class="roll"></div>
    <div class="stripes"></div>
    <div class="halftone"></div>
  {/if}
</div>
