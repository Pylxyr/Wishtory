import { groupDigits } from './gameinfo';

/** A deterministic stream of numbers in [0, 1), so a scatter is the same on every start. */
export function seq(seed: number) {
  let s = seed;
  return () => ((s = (s * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
}

const reduced = () => document.documentElement.dataset.motion === 'off' || matchMedia('(prefers-reduced-motion: reduce)').matches;

/** Counts a number up from zero when it appears, and re-counts from the old value when it changes. */
export function countUp(node: HTMLElement, opts: { value: number; digits?: number; ms?: number }) {
  let shown = 0;
  let raf = 0;
  const fmt = (v: number, d: number) => (d > 0 ? v.toFixed(d) : groupDigits(Math.round(v)));
  const run = (o: { value: number; digits?: number; ms?: number }) => {
    cancelAnimationFrame(raf);
    const d = o.digits ?? 0;
    if (reduced()) { node.textContent = fmt(o.value, d); shown = o.value; return; }
    const from = shown, t0 = performance.now(), ms = o.ms ?? 900;
    const step = (t: number) => {
      const k = Math.min(1, (t - t0) / ms);
      const eased = 1 - Math.pow(1 - k, 3);
      shown = from + (o.value - from) * eased;
      node.textContent = fmt(shown, d);
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
  };
  run(opts);
  return { update: run, destroy: () => cancelAnimationFrame(raf) };
}
