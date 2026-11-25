import type { GameId } from './types';
import { INFO } from './gameinfo';

export interface Preset { id: string; name: string; hue: number; hueLight?: number; tint?: number; accentDark: string; accentLight: string }

export const DEFAULT_THEME = 'default';
export const FOLLOW_GAME = 'game';

/** The original graphite look first, then the three games' own palettes, then the colour themes. */
export const PRESETS: Preset[] = [
  { id: DEFAULT_THEME, name: 'Graphite', hue: 215, accentDark: '#5AA2FF', accentLight: '#1F5FD0' },
  { id: 'teyvat', name: 'Teyvat', hue: 232, hueLight: 40, tint: 0.85, accentDark: '#D8C7A0', accentLight: '#3B4A6B' },
  { id: 'astral', name: 'Astral', hue: 255, accentDark: '#8AD7FF', accentLight: '#4A4FD1' },
  { id: 'eridu', name: 'New Eridu', hue: 80, tint: 0.15, accentDark: '#D6F633', accentLight: '#4F6B00' },
  { id: 'ocean', name: 'Ocean', hue: 200, accentDark: '#3DC6E8', accentLight: '#0A6C8C' },
  { id: 'forest', name: 'Forest', hue: 150, accentDark: '#4FD08C', accentLight: '#12703F' },
  { id: 'sunset', name: 'Sunset', hue: 18, accentDark: '#FF8A57', accentLight: '#B3430E' },
  { id: 'rose', name: 'Rose', hue: 335, accentDark: '#FF7FAB', accentLight: '#B0225C' },
  { id: 'violet', name: 'Violet', hue: 265, accentDark: '#A990FF', accentLight: '#5A35C8' },
  { id: 'mono', name: 'Mono', hue: 220, tint: 0.18, accentDark: '#E4E8EE', accentLight: '#1B2330' }
];

const BASE_DARK = { bg: '#0D1015', rail: '#11151B', panel: '#151A22', panel2: '#1B212B', panel3: '#252D39', line: '#262E3A', line2: '#36404F', control: '#5A6678', text: '#E9EDF2', muted: '#9CA6B5', faint: '#8793A5', accent: '#5AA2FF', accentInk: '#06101F' };
const BASE_LIGHT = { bg: '#F2F4F7', rail: '#FFFFFF', panel: '#FFFFFF', panel2: '#EEF1F5', panel3: '#E3E8EE', line: '#DFE4EA', line2: '#C5CCD6', control: '#838D9C', text: '#161B22', muted: '#4F5A69', faint: '#5F6B7B', accent: '#1F5FD0', accentInk: '#FFFFFF' };
// Gold (5★), purple (4★) and the status colours never change: they carry meaning in every theme.
const FIXED_DARK = { gold: '#F2BD4B', purple: '#B48CF2', blue: '#7FB0F0', ok: '#55D39B', err: '#FF7D7D', onErr: '#1C0808' };
const FIXED_LIGHT = { gold: '#8A5A00', purple: '#6A3FC4', blue: '#2457B0', ok: '#14724E', err: '#B42727', onErr: '#FFFFFF' };

export const presetFor = (id: string) => PRESETS.find((p) => p.id === id) ?? PRESETS[0];
export const resolveTheme = (id: string, game: GameId) => (id === FOLLOW_GAME ? INFO[game].themeId : id);

const hsl = (h: number, s: number, l: number) => `hsl(${h} ${Math.min(100, Math.max(0, s * 100)).toFixed(1)}% ${(l * 100).toFixed(1)}%)`;

/** CSS variables for a preset in a brightness. */
export function palette(themeId: string, dark: boolean, game: GameId): Record<string, string> {
  const resolved = resolveTheme(themeId, game);
  const fixed = dark ? FIXED_DARK : FIXED_LIGHT;
  if (resolved === DEFAULT_THEME) return { ...(dark ? BASE_DARK : BASE_LIGHT), ...fixed };
  const p = presetFor(resolved);
  const h = dark ? p.hue : (p.hueLight ?? p.hue);
  const s = (v: number) => v * (p.tint ?? 1);
  if (dark) {
    return {
      bg: hsl(h, s(0.24), 0.065), rail: hsl(h, s(0.24), 0.085), panel: hsl(h, s(0.22), 0.105), panel2: hsl(h, s(0.2), 0.135),
      panel3: hsl(h, s(0.18), 0.18), line: hsl(h, s(0.18), 0.19), line2: hsl(h, s(0.16), 0.26), control: hsl(h, s(0.14), 0.4),
      text: hsl(h, s(0.2), 0.92), muted: hsl(h, s(0.12), 0.68), faint: hsl(h, s(0.1), 0.62), accent: p.accentDark,
      accentInk: hsl(h, s(0.5), 0.08), ...fixed
    };
  }
  return {
    bg: hsl(h, s(0.2), 0.96), rail: '#FFFFFF', panel: '#FFFFFF', panel2: hsl(h, s(0.18), 0.94), panel3: hsl(h, s(0.16), 0.9),
    line: hsl(h, s(0.15), 0.88), line2: hsl(h, s(0.12), 0.79), control: hsl(h, s(0.08), 0.52), text: hsl(h, s(0.25), 0.1),
    muted: hsl(h, s(0.12), 0.33), faint: hsl(h, s(0.1), 0.36), accent: p.accentLight, accentInk: '#FFFFFF', ...fixed
  };
}

export function applyPalette(themeId: string, dark: boolean, game: GameId) {
  const root = document.documentElement;
  for (const [k, v] of Object.entries(palette(themeId, dark, game))) root.style.setProperty(`--${k.replace(/[A-Z]/g, (m) => '-' + m.toLowerCase())}`, v);
  root.dataset.mode = dark ? 'dark' : 'light';
}
