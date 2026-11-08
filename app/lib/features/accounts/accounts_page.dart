import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/recovery.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/settings.dart';
import '../../state/toasts.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

class AccountsPage extends ConsumerWidget {
  const AccountsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final accounts = ref.watch(accountsProvider);
    final settings = ref.watch(settingsProvider);
    final list = accounts.data ?? const <Account>[];
    return PageBody(maxWidth: 900, children: [
      const Text('Accounts', style: Ty.h1),
      const SizedBox(height: 4),
      Text('Everything here is stored on this PC. Names are local labels; they are never sent anywhere.', style: Ty.body.copyWith(color: c.muted)),
      const SizedBox(height: 20),
      if (accounts.failed && list.isEmpty)
        RecoveryCard(recovery: recoveryFor(accounts.error!), onRetry: () => ref.read(accountsProvider.notifier).reload())
      else if (accounts.data == null)
        const Skeleton(height: 90, radius: 12)
      else if (list.isEmpty)
        const SizedBox(height: 300, child: EmptyState(icon: Icons.people_outline, title: 'No accounts yet', body: 'Accounts appear after your first refresh or import.'))
      else
        for (final g in GameId.values)
          for (final a in list.where((a) => a.game == g))
            Padding(padding: const EdgeInsets.only(bottom: 12), child: _AccountCard(account: a, settings: settings)),
      const SizedBox(height: 24),
      const SectionLabel('Danger zone'),
      Panel(
        child: Row(children: [
          Expanded(
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              const Text('Clear all saved history', style: Ty.h3),
              Text('Deletes every saved pull of every account from this PC. The game and your in-game history are not touched.', style: Ty.small.copyWith(color: c.muted)),
            ]),
          ),
          const SizedBox(width: 12),
          OutlinedButton(
            style: OutlinedButton.styleFrom(foregroundColor: c.err, side: BorderSide(color: c.err)),
            onPressed: list.isEmpty ? null : () => _clearAll(context, ref),
            child: const Text('Clear all…'),
          ),
        ]),
      ),
    ]);
  }

  Future<void> _clearAll(BuildContext context, WidgetRef ref) async {
    final ok = await confirm(
      context,
      title: 'Clear all saved history?',
      body: 'This deletes every saved pull of every account from this PC. Export first if you may want them back.',
      action: 'Delete everything',
      danger: true,
    );
    if (!ok) return;
    try {
      final n = await ref.read(apiProvider).clearAllHistory();
      await ref.read(accountsProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
      ref.read(toastProvider.notifier).ok('Removed ${plural(n, 'pull')}.');
    } catch (e) {
      ref.read(toastProvider.notifier).error(recoveryFor(e).detail);
    }
  }
}

class _AccountCard extends ConsumerWidget {
  const _AccountCard({required this.account, required this.settings});
  final Account account;
  final AppSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final info = GameInfo.of(account.game);
    final last = settings.lastRefresh[accountKey(account.game, account.uid)];
    return Panel(
      child: Row(children: [
        Container(
          width: 40,
          height: 28,
          alignment: Alignment.center,
          decoration: BoxDecoration(border: Border.all(color: c.control), borderRadius: BorderRadius.circular(6)),
          child: Text(info.abbr, style: const TextStyle(fontSize: 11, fontWeight: FontWeight.w700)),
        ),
        const SizedBox(width: 14),
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text(accountName(settings, account), style: Ty.h2),
            const SizedBox(height: 2),
            Wrap(spacing: 14, children: [
              SelectableText('UID ${account.uid}', style: Ty.small.copyWith(fontFamily: Ty.mono, color: c.muted)),
              Text(plural(account.pullCount, 'pull'), style: Ty.small.copyWith(color: c.muted)),
              Text(utcLabel(account.timezone), style: Ty.small.copyWith(color: c.muted)),
              Text(last == null ? 'Not refreshed on this PC' : 'Refreshed ${ago(DateTime.fromMillisecondsSinceEpoch(last))}', style: Ty.small.copyWith(color: c.faint)),
            ]),
          ]),
        ),
        TextButton(
          onPressed: () {
            ref.read(navProvider.notifier).selectAccount(account.game, account.uid);
            ref.read(navProvider.notifier).goTo(AppPage.overview);
          },
          child: const Text('View'),
        ),
        IconButton(tooltip: 'Rename', icon: const Icon(Icons.edit_outlined, size: 18), onPressed: () => _rename(context, ref)),
        IconButton(tooltip: 'Remove from Wishtory', icon: Icon(Icons.delete_outline, size: 18, color: c.err), onPressed: () => _remove(context, ref)),
      ]),
    );
  }

  Future<void> _rename(BuildContext context, WidgetRef ref) async {
    final controller = TextEditingController(text: settings.aliases[accountKey(account.game, account.uid)] ?? '');
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: context.wc.panel,
        title: const Text('Rename account', style: Ty.h2),
        content: SizedBox(
          width: 360,
          child: TextField(
            controller: controller,
            autofocus: true,
            maxLength: 40,
            decoration: fieldDecoration(ctx, hint: account.nickname.isNotEmpty ? account.nickname : account.uid),
            onSubmitted: (v) => Navigator.pop(ctx, v),
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, controller.text), child: const Text('Save')),
        ],
      ),
    );
    if (name != null) ref.read(settingsProvider.notifier).setAlias(account.game, account.uid, name);
  }

  Future<void> _remove(BuildContext context, WidgetRef ref) async {
    final ok = await confirm(
      context,
      title: 'Remove UID ${account.uid}?',
      body: 'This deletes ${plural(account.pullCount, 'saved pull')} of this account from this PC. Your in-game history and the game files are not touched. You can bring them back with a refresh or an import.',
      action: 'Remove',
      danger: true,
    );
    if (!ok) return;
    try {
      await ref.read(apiProvider).deleteAccount(account.game, account.uid);
      ref.read(settingsProvider.notifier).forgetAccount(account.game, account.uid);
      await ref.read(accountsProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
      ref.read(toastProvider.notifier).ok('Removed UID ${account.uid}.');
    } catch (e) {
      ref.read(toastProvider.notifier).error(recoveryFor(e).detail);
    }
  }
}
