import { GAMES, type GameId } from './types';

/**
 * Turns the backend's friendly message into a recovery plan. The backend writes one fixed sentence per
 * failure (src-tauri/src/error.rs), so matching on those sentences is reliable today; a typed error
 * code from the backend would be the sturdier long-term contract.
 */
export type ErrorKind = 'expired' | 'no-history-page' | 'game-not-found' | 'rate-limited' | 'network' | 'server' | 'invalid-url' | 'other';

export interface Recovery {
  kind: ErrorKind;
  title: string;
  why: string;
  steps: string[];
  help: string | null; // id of the Help fold that explains it, or null
  locations: boolean; // offer a shortcut to Settings → Game locations (Linux, game not found)
  canRetry: boolean;
  waitSeconds: number; // how long a retry makes no sense (rate limiting)
}

export function classify(message: string, game: GameId, linux = false): Recovery {
  const m = message.toLowerCase();
  const g = GAMES[game];
  const openHistory = `In ${g.name}, open ${g.historyPage} and wait until the list has loaded.`;
  const base = { help: null, canRetry: true, waitSeconds: 0, locations: false };
  if (m.includes("doesn't look like a valid")) {
    return { ...base, kind: 'invalid-url', title: 'That is not a history link', why: 'The pasted text is not a full HoYoverse history link for this game.', steps: ['Use Refresh instead, or paste the whole link: it starts with https:// and contains authkey=.'], canRetry: false };
  }
  if (m.includes('has expired') || /auth ?key/.test(m)) {
    return { ...base, kind: 'expired', title: 'The history link has expired', why: 'The link the game saved in its cache only works for a short time.', steps: [openHistory, 'Come back to Wishtory and click Try again.'], help: 'help-expired' };
  }
  if (m.includes('no history page')) {
    return { ...base, kind: 'no-history-page', title: 'The game has not saved a history page yet', why: 'Wishtory looks for the history page in the game’s cache and found none.', steps: [openHistory, 'Come back to Wishtory and click Try again.'], help: 'help-no-page' };
  }
  if (m.includes("wasn't found") && m.includes('launch the game')) {
    if (linux) {
      return {
        ...base, kind: 'game-not-found', title: `${g.name} was not detected`,
        why: 'Wishtory finds the game from the log file it writes inside its Wine or Proton prefix.',
        steps: [
          'Start the game once from your launcher, so it writes its log file.',
          'If it is still not found, add the folder of its Wine prefix (the one that contains drive_c) under Settings → Game locations.',
          'Then click Try again.'
        ],
        help: 'help-not-detected', locations: true
      };
    }
    return { ...base, kind: 'game-not-found', title: `${g.name} was not detected`, why: 'Wishtory finds the game from the log file it writes on launch.', steps: ['Start the game once so it writes its log file.', 'Then click Try again.'], help: 'help-not-detected' };
  }
  if (m.includes('slow down')) {
    return { ...base, kind: 'rate-limited', title: 'The server asked for a short break', why: 'Too many requests in a row. Waiting a minute is enough.', steps: ['Wait about a minute, then click Try again.'], waitSeconds: 60 };
  }
  if (m.startsWith('network problem')) {
    return { ...base, kind: 'network', title: 'Could not reach the server', why: 'The connection to the history service failed.', steps: ['Check your internet connection.', 'Turn off any VPN or proxy that blocks HoYoverse.', 'Then click Try again.'] };
  }
  if (m.startsWith('the server returned an error')) {
    return { ...base, kind: 'server', title: 'The server refused the request', why: 'This usually means the saved link timed out.', steps: [openHistory, 'Click Try again straight after the list loads.'], help: 'help-server' };
  }
  return { ...base, kind: 'other', title: 'The refresh stopped', why: message, steps: [] };
}
