/** Motion helpers. JS-driven transitions do not see the CSS reduced-motion rule, so they ask here. */
export const reducedMotion = (): boolean =>
  typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;

/** A duration in ms, or 0 when the system asks for reduced motion. */
export const dur = (ms: number): number => (reducedMotion() ? 0 : ms);
