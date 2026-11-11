import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/recovery.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/refresh.dart';
import '../../state/settings.dart';
import '../../state/toasts.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';
import '../about/about_page.dart';
import '../accounts/accounts_page.dart';
import '../help/help_page.dart';
import '../help/tutorial.dart';
import '../history/history_page.dart';
import '../import_export/import_export_page.dart';
import '../overview/overview_page.dart';
import '../settings/catalog_review.dart';
import '../settings/logs_page.dart';
import '../settings/settings_page.dart';

class AppShell extends ConsumerStatefulWidget {
  const AppShell({super.key});

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  bool _tutorialShown = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) => runStartupTasks(ref));
  }

  void _maybeTutorial() {
    if (_tutorialShown || ref.read(settingsProvider).tutorialDone) return;
    _tutorialShown = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) showTutorial(context, ref);
    });
  }

  @override
  Widget build(BuildContext context) {
    final nav = ref.watch(navProvider);
    final wide = MediaQuery.sizeOf(context).width >= 1100;
    ref.listen(accountsProvider, (_, next) {
      if (next.data != null) _maybeTutorial();
    });
    final controller = ref.read(navProvider.notifier);
    final bindings = <ShortcutActivator, VoidCallback>{
      const SingleActivator(LogicalKeyboardKey.keyR, control: true): () => ref.read(refreshProvider.notifier).run(),
      const SingleActivator(LogicalKeyboardKey.digit1, alt: true): () => controller.openGame(GameId.genshin),
      const SingleActivator(LogicalKeyboardKey.digit2, alt: true): () => controller.openGame(GameId.starRail),
      const SingleActivator(LogicalKeyboardKey.digit3, alt: true): () => controller.openGame(GameId.zenless),
      const SingleActivator(LogicalKeyboardKey.comma, control: true): () => controller.goTo(AppPage.settings),
      const SingleActivator(LogicalKeyboardKey.f1): () => controller.goTo(AppPage.help),
    };
    return Scaffold(
      body: CallbackShortcuts(
        bindings: bindings,
        child: Focus(
          autofocus: true,
          child: Stack(children: [
            Row(children: [
              AppRail(wide: wide),
              Expanded(
                child: Column(children: [
                  const CatalogUpdateNotice(),
                  if (nav.page.isGamePage) const PageBar(),
                  Expanded(child: _page(nav.page)),
                  const StatusBar(),
                ]),
              ),
            ]),
            const ToastHost(),
          ]),
        ),
      ),
    );
  }

  Widget _page(AppPage page) => switch (page) {
        AppPage.overview => const OverviewPage(),
        AppPage.history => const HistoryPage(),
        AppPage.importExport => const ImportExportPage(),
        AppPage.accounts => const AccountsPage(),
        AppPage.settings => const SettingsPage(),
        AppPage.help => const HelpPage(),
        AppPage.about => const AboutPage(),
        AppPage.logs => const LogsPage(),
        AppPage.catalogReview => const CatalogReviewPage(),
      };
}

// ------------------------------------------------------------------ rail

class AppRail extends ConsumerWidget {
  const AppRail({super.key, required this.wide});
  final bool wide;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final nav = ref.watch(navProvider);
    final counts = ref.watch(accountsProvider).data ?? const <Account>[];
    final update = ref.watch(appUpdateProvider).data;
    final catalogWaiting = ref.watch(catalogStatusProvider.select((s) => s.data?.waiting != null));
    final logging = ref.watch(settingsProvider.select((s) => s.loggingEnabled));
    final width = wide ? 224.0 : 68.0;

    Widget item(AppPage page, IconData icon, {bool dot = false, String dotLabel = 'Update'}) => _RailItem(
          icon: icon,
          label: page.title,
          wide: wide,
          selected: nav.page == page,
          dot: dot,
          dotLabel: dotLabel,
          onTap: () => ref.read(navProvider.notifier).goTo(page),
        );

    return Container(
      width: width,
      decoration: BoxDecoration(color: c.rail, border: Border(right: BorderSide(color: c.line))),
      padding: const EdgeInsets.fromLTRB(10, 14, 10, 12),
      child: FocusTraversalGroup(
        child: Column(children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 2, 8, 10),
            child: Row(mainAxisAlignment: wide ? MainAxisAlignment.start : MainAxisAlignment.center, children: [
              Icon(Icons.auto_awesome, color: c.accent, size: 20),
              if (wide) ...[const SizedBox(width: 9), const Text('Wishtory', style: TextStyle(fontSize: 17, fontWeight: FontWeight.w600, letterSpacing: -0.3))],
            ]),
          ),
          // The games are where you spend your time, so they come first. Choosing one always opens that
          // game's Overview or History, whichever page you were on.
          Expanded(
            child: ListView(padding: EdgeInsets.zero, children: [
              if (wide) const _RailLabel('Games'),
              for (final g in GameId.values)
                _RailItem(
                  abbr: GameInfo.of(g).abbr,
                  label: GameInfo.of(g).short,
                  wide: wide,
                  selected: nav.page.isGamePage && nav.game == g,
                  count: wide ? counts.where((a) => a.game == g).length : null,
                  onTap: () => ref.read(navProvider.notifier).openGame(g),
                ),
              if (wide) const _RailLabel('Your data') else const _RailDivider(),
              item(AppPage.accounts, Icons.people_outline),
              item(AppPage.importExport, Icons.swap_vert),
            ]),
          ),
          // The rest of the app stays at the bottom, out of the way.
          const _RailDivider(),
          item(AppPage.settings, Icons.settings_outlined, dot: update?.available == true || catalogWaiting, dotLabel: catalogWaiting ? 'Review' : 'Update'),
          if (logging) item(AppPage.logs, Icons.receipt_long_outlined),
          item(AppPage.help, Icons.help_outline),
          item(AppPage.about, Icons.info_outline),
        ]),
      ),
    );
  }
}

class _RailDivider extends StatelessWidget {
  const _RailDivider();

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 4),
        child: Divider(height: 1, color: context.wc.line),
      );
}

class _RailLabel extends StatelessWidget {
  const _RailLabel(this.text);
  final String text;

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(10, 14, 10, 4),
        child: Text(text.toUpperCase(), style: Ty.label.copyWith(color: context.wc.faint)),
      );
}

class _RailItem extends StatelessWidget {
  const _RailItem({
    this.icon,
    this.abbr,
    required this.label,
    required this.wide,
    required this.selected,
    required this.onTap,
    this.count,
    this.dot = false,
    this.dotLabel = 'Update',
  });

  final IconData? icon;
  final String? abbr;
  final String label;
  final bool wide;
  final bool selected;
  final VoidCallback onTap;
  final int? count;
  final bool dot;
  final String dotLabel;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final fg = selected ? c.text : c.muted;
    final lead = abbr != null
        ? Container(
            width: 30,
            height: 22,
            alignment: Alignment.center,
            decoration: BoxDecoration(border: Border.all(color: selected ? c.accent : c.control), borderRadius: BorderRadius.circular(6)),
            child: Text(abbr!, style: TextStyle(fontSize: 10.5, fontWeight: FontWeight.w700, color: selected ? c.accent : c.muted)),
          )
        : Stack(clipBehavior: Clip.none, children: [
            Icon(icon, size: 20, color: selected ? c.accent : c.muted),
            if (dot) Positioned(right: -3, top: -3, child: Container(width: 8, height: 8, decoration: BoxDecoration(color: c.accent, shape: BoxShape.circle))),
          ]);
    final row = Row(mainAxisAlignment: wide ? MainAxisAlignment.start : MainAxisAlignment.center, children: [
      lead,
      if (wide) ...[
        const SizedBox(width: 10),
        Expanded(child: Text(label, overflow: TextOverflow.ellipsis, style: TextStyle(color: fg, fontWeight: selected ? FontWeight.w600 : FontWeight.w400))),
        if (count != null && count! > 0) Text('$count', style: Ty.small.copyWith(fontFamily: Ty.mono, color: c.faint)),
        if (dot) Text(dotLabel, style: Ty.small.copyWith(color: c.accent)),
      ],
    ]);
    return Semantics(
      button: true,
      selected: selected,
      label: dot ? '$label, $dotLabel' : label,
      child: Tooltip(
        message: wide ? '' : label,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 1),
          child: Material(
            color: selected ? c.panel2 : Colors.transparent,
            borderRadius: BorderRadius.circular(8),
            child: InkWell(
              borderRadius: BorderRadius.circular(8),
              onTap: onTap,
              child: Padding(padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7), child: row),
            ),
          ),
        ),
      ),
    );
  }
}

// ------------------------------------------------------------------ page bar

class PageBar extends ConsumerWidget {
  const PageBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final nav = ref.watch(navProvider);
    final account = ref.watch(currentAccountProvider);
    final settings = ref.watch(settingsProvider);
    final busy = ref.watch(refreshProvider.select((s) => s.busy));
    final last = account == null ? null : settings.lastRefresh[accountKey(account.game, account.uid)];

    // Who and when: the game, the account, and how fresh the data is.
    final title = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(mainAxisSize: MainAxisSize.min, children: [
          Text(GameInfo.of(nav.game).name, style: Ty.h1),
          const SizedBox(width: 12),
          const AccountMenu(),
        ]),
        if (account != null) ...[
          const SizedBox(height: 2),
          Text(
            last == null ? 'Not refreshed on this PC yet' : 'Refreshed ${ago(DateTime.fromMillisecondsSinceEpoch(last))}',
            style: Ty.small.copyWith(color: c.muted),
          ),
        ],
      ],
    );

    // What to look at: the two views of this game's data.
    final views = SegmentedButton<AppPage>(
      showSelectedIcon: false,
      segments: const [
        ButtonSegment(value: AppPage.overview, label: Text('Overview'), icon: Icon(Icons.dashboard_outlined, size: 16)),
        ButtonSegment(value: AppPage.history, label: Text('History'), icon: Icon(Icons.history, size: 16)),
      ],
      selected: {nav.page},
      onSelectionChanged: (v) => ref.read(navProvider.notifier).goTo(v.first),
    );

    // What to do: refresh from the game.
    final refresh = Row(mainAxisSize: MainAxisSize.min, children: [
      FilledButton.icon(
        onPressed: busy ? null : () => ref.read(refreshProvider.notifier).run(),
        icon: const Icon(Icons.sync, size: 18),
        label: Text(busy ? 'Refreshing…' : 'Refresh'),
      ),
      PopupMenuButton<String>(
        tooltip: 'More ways to refresh',
        enabled: !busy,
        icon: const Icon(Icons.arrow_drop_down),
        onSelected: (v) async {
          if (v == 'full') {
            ref.read(refreshProvider.notifier).run(full: true);
          } else {
            final url = await showPasteLinkDialog(context);
            if (url != null) ref.read(refreshProvider.notifier).run(url: url);
          }
        },
        itemBuilder: (_) => const [
          PopupMenuItem(value: 'full', child: Text('Full re-sync')),
          PopupMenuItem(value: 'link', child: Text('Paste a link…')),
        ],
      ),
    ]);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 12),
      decoration: BoxDecoration(color: c.bg, border: Border(bottom: BorderSide(color: c.line))),
      child: LayoutBuilder(builder: (context, box) {
        if (box.maxWidth >= 900) {
          return Row(children: [title, const SizedBox(width: 24), views, const Spacer(), refresh]);
        }
        return Wrap(
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 10,
          children: [title, views, refresh],
        );
      }),
    );
  }
}

class AccountMenu extends ConsumerWidget {
  const AccountMenu({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final accounts = ref.watch(gameAccountsProvider);
    final current = ref.watch(currentAccountProvider);
    final settings = ref.watch(settingsProvider);
    if (current == null) return const SizedBox.shrink();
    return PopupMenuButton<Account>(
      tooltip: 'Switch account',
      enabled: accounts.length > 1,
      onSelected: (a) => ref.read(navProvider.notifier).selectAccount(a.game, a.uid),
      itemBuilder: (_) => [
        for (final a in accounts)
          PopupMenuItem(
            value: a,
            child: Row(children: [
              Expanded(child: Text(accountName(settings, a))),
              const SizedBox(width: 12),
              Text(a.uid, style: Ty.small.copyWith(fontFamily: Ty.mono, color: c.faint)),
            ]),
          ),
      ],
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
        decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(8), border: Border.all(color: c.line2)),
        child: Row(mainAxisSize: MainAxisSize.min, children: [
          Text(accountName(settings, current)),
          const SizedBox(width: 8),
          Text(current.uid, style: Ty.small.copyWith(fontFamily: Ty.mono, color: c.faint)),
          if (accounts.length > 1) Icon(Icons.expand_more, size: 18, color: c.muted),
        ]),
      ),
    );
  }
}

Future<String?> showPasteLinkDialog(BuildContext context) {
  final controller = TextEditingController();
  final c = context.wc;
  return showDialog<String>(
    context: context,
    builder: (ctx) => AlertDialog(
      backgroundColor: c.panel,
      title: const Text('Paste a history link', style: Ty.h2),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
          Text(
            'Only needed when Refresh cannot find the link on its own. Paste the full link from the game: it starts with https:// and contains authkey=. It is used once and never saved.',
            style: Ty.body.copyWith(color: c.muted),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: controller,
            autofocus: true,
            obscureText: true,
            decoration: fieldDecoration(ctx, hint: 'https://…'),
            onSubmitted: (v) => Navigator.pop(ctx, v.trim()),
          ),
        ]),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(ctx, controller.text.trim()), child: const Text('Download')),
      ],
    ),
  );
}

// ------------------------------------------------------------------ refresh panel

/// Progress while refreshing, the result afterwards, or a recovery card on failure. Shown at the
/// top of the game pages and stays until dismissed.
class RefreshPanel extends ConsumerWidget {
  const RefreshPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final s = ref.watch(refreshProvider);
    final controller = ref.read(refreshProvider.notifier);
    if (s.busy) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 16),
        child: Panel(
          child: Semantics(
            liveRegion: true,
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              Row(children: [
                const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)),
                const SizedBox(width: 10),
                Text('Refreshing ${s.game == null ? '' : GameInfo.of(s.game!).short}…', style: Ty.h3),
              ]),
              const SizedBox(height: 8),
              for (final line in s.lines) Text(line, style: Ty.small.copyWith(color: c.muted)),
            ]),
          ),
        ),
      );
    }
    if (s.error != null) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 16),
        child: RecoveryCard(
          recovery: recoveryFor(s.error!, game: s.game, isLinux: Platform.isLinux),
          onRetry: () => controller.run(),
          onHelp: (topic) {
            ref.read(helpTopicProvider.notifier).open(topic);
            ref.read(navProvider.notifier).goTo(AppPage.help);
          },
          onLocations: () => ref.read(navProvider.notifier).goTo(AppPage.settings),
          onDismiss: controller.dismiss,
        ),
      );
    }
    final r = s.result;
    if (r != null) {
      final text = r.added == 0
          ? 'Everything was already saved (${plural(r.fetched, 'pull')} checked).'
          : '${plural(r.added, 'new pull')} saved (${plural(r.fetched, 'pull')} downloaded).';
      return Padding(
        padding: const EdgeInsets.only(bottom: 16),
        child: Panel(
          tint: c.ok.withValues(alpha: 0.07),
          child: Semantics(
            liveRegion: true,
            child: Row(children: [
              Icon(Icons.check_circle_outline, color: c.ok),
              const SizedBox(width: 10),
              Expanded(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Text('Refreshed UID ${r.uid}', style: Ty.h3),
                  Text(r.warning ?? text, style: Ty.small.copyWith(color: c.muted)),
                ]),
              ),
              IconButton(tooltip: 'Dismiss', onPressed: controller.dismiss, icon: const Icon(Icons.close, size: 18)),
            ]),
          ),
        ),
      );
    }
    return const SizedBox.shrink();
  }
}

// ------------------------------------------------------------------ status bar and toasts

class StatusBar extends ConsumerWidget {
  const StatusBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final catalog = ref.watch(catalogProvider).data;
    final busy = ref.watch(refreshProvider.select((s) => s.busy));
    return Container(
      height: 28,
      padding: const EdgeInsets.symmetric(horizontal: 24),
      decoration: BoxDecoration(color: c.rail, border: Border(top: BorderSide(color: c.line))),
      child: Row(children: [
        Icon(Icons.lock_outline, size: 13, color: c.faint),
        const SizedBox(width: 6),
        Text('Stored only on this PC. Game files are read, never changed.', style: Ty.small.copyWith(color: c.faint)),
        const Spacer(),
        if (busy) Text('Refreshing…  ', style: Ty.small.copyWith(color: c.muted)),
        if (catalog != null) Text('Catalog v${catalog.catalogVersion}', style: Ty.small.copyWith(color: c.faint, fontFamily: Ty.mono)),
      ]),
    );
  }
}

class ToastHost extends ConsumerWidget {
  const ToastHost({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final toasts = ref.watch(toastProvider);
    if (toasts.isEmpty) return const SizedBox.shrink();
    return Positioned(
      right: 20,
      bottom: 44,
      child: Semantics(
        liveRegion: true,
        child: Column(crossAxisAlignment: CrossAxisAlignment.end, mainAxisSize: MainAxisSize.min, children: [
          for (final t in toasts)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Material(
                color: c.panel3,
                elevation: 4,
                borderRadius: BorderRadius.circular(10),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 420),
                  child: Padding(
                    padding: const EdgeInsets.fromLTRB(14, 10, 6, 10),
                    child: Row(mainAxisSize: MainAxisSize.min, children: [
                      Icon(
                        switch (t.kind) {
                          ToastKind.ok => Icons.check_circle_outline,
                          ToastKind.error => Icons.error_outline,
                          ToastKind.info => Icons.info_outline,
                        },
                        size: 18,
                        color: switch (t.kind) {
                          ToastKind.ok => c.ok,
                          ToastKind.error => c.err,
                          ToastKind.info => c.accent,
                        },
                      ),
                      const SizedBox(width: 10),
                      Flexible(child: Text(t.message)),
                      IconButton(
                        tooltip: 'Dismiss',
                        onPressed: () => ref.read(toastProvider.notifier).dismiss(t.id),
                        icon: const Icon(Icons.close, size: 16),
                      ),
                    ]),
                  ),
                ),
              ),
            ),
        ]),
      ),
    );
  }
}
