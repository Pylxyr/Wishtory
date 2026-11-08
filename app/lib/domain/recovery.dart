import '../bridge/generated/api/error.dart';
import '../bridge/generated/api/types.dart';
import 'game_info.dart';

/// What to show when a call fails: a title, why it happened, and concrete steps. The backend's
/// message is always kept as the detail. (Replaces the old text-matching `errors.ts`; the backend
/// now sends an [ErrorCode].)
class Recovery {
  const Recovery({
    required this.title,
    required this.why,
    this.steps = const [],
    this.helpTopic,
    this.canRetry = true,
    this.offerGameLocations = false,
    required this.detail,
  });

  final String title;
  final String why;
  final List<String> steps;
  final String? helpTopic;
  final bool canRetry;
  final bool offerGameLocations;
  final String detail;
}

Recovery recoveryFor(Object error, {GameId? game, bool isLinux = false}) {
  if (error is! ApiError) {
    return Recovery(
      title: 'Something went wrong',
      why: 'An unexpected problem stopped this. Your saved history is untouched.',
      detail: error.toString(),
    );
  }
  final info = game == null ? null : GameInfo.of(game);
  final page = info?.historyPage ?? 'the wish history page';
  return switch (error.code) {
    ErrorCode.historyPageNotOpened || ErrorCode.authExpired => Recovery(
        title: error.code == ErrorCode.authExpired ? 'The history link has expired' : 'No history page found yet',
        why: 'Wishtory reads the link the game saves when you open its history page. It only lasts a short while.',
        steps: [
          'Keep the game open.',
          'Open $page and wait until the list loads.',
          'Come back here and press Refresh.',
        ],
        helpTopic: 'refresh',
        detail: error.message,
      ),
    ErrorCode.gameNotFound => Recovery(
        title: "The game wasn't found",
        why: 'Wishtory looks for the game\'s log file and cache, read-only.',
        steps: [
          'Launch the game at least once, so it creates its log file.',
          if (isLinux) 'On Linux, add the game\'s folder or Wine prefix in Settings → Game locations.',
          'Press Refresh again.',
        ],
        helpTopic: 'game-not-found',
        offerGameLocations: isLinux,
        detail: error.message,
      ),
    ErrorCode.invalidUrl => Recovery(
        title: "That link doesn't look right",
        why: 'It has to be the full history link from the game: it starts with https:// and contains authkey=.',
        steps: ['Use Refresh instead, or copy the whole link again.'],
        helpTopic: 'paste-link',
        detail: error.message,
      ),
    ErrorCode.rateLimited => Recovery(
        title: 'Slow down for a moment',
        why: 'The game server asked Wishtory to wait.',
        steps: ['Wait about a minute, then try again.'],
        detail: error.message,
      ),
    ErrorCode.network || ErrorCode.api => Recovery(
        title: error.code == ErrorCode.network ? "Couldn't reach the server" : 'The server returned an error',
        why: 'The request did not complete. Your saved history is untouched.',
        steps: [
          'Check your internet connection.',
          'Turn off any VPN or proxy that blocks HoYoverse.',
          'Try again in a minute.',
        ],
        detail: error.message,
      ),
    ErrorCode.database || ErrorCode.file || ErrorCode.format || ErrorCode.other => Recovery(
        title: switch (error.code) {
          ErrorCode.database => "Wishtory couldn't use its database",
          ErrorCode.file => "A file couldn't be read or written",
          ErrorCode.format => "That file isn't supported",
          _ => 'Something went wrong',
        },
        why: 'Nothing was changed in the game. Your saved history was not removed by this.',
        canRetry: false,
        detail: error.message,
      ),
  };
}
