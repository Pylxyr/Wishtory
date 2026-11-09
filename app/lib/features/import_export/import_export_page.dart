import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/recovery.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/toasts.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

class _Format {
  const _Format(this.id, this.label, this.note, this.game, this.fileName);
  final String id, label, note, fileName;
  final GameId? game;
}

const _formats = [
  _Format('uigf4', 'UIGF v4.2', 'Recommended. Covers every game and account.', null, 'uigf-v4.2'),
  _Format('uigf3', 'UIGF v3.0', 'Genshin only. For Paimon.moe and older tools.', GameId.genshin, 'uigf-v3.0'),
  _Format('srgf1', 'SRGF v1.0', 'Star Rail only.', GameId.starRail, 'srgf-v1.0'),
  _Format('paimon', 'Paimon.moe local wish data', 'Genshin only.', GameId.genshin, 'paimon-moe-wishes'),
];

class ImportExportPage extends ConsumerStatefulWidget {
  const ImportExportPage({super.key});

  @override
  ConsumerState<ImportExportPage> createState() => _ImportExportPageState();
}

class _ImportExportPageState extends ConsumerState<ImportExportPage> {
  String _format = 'uigf4';
  bool _allAccounts = false;
  bool _busy = false;
  ImportSummary? _imported;
  Object? _error;

  Future<void> _import() async {
    if (_busy) return;
    final file = await openFile(acceptedTypeGroups: [const XTypeGroup(label: 'Wish history JSON', extensions: ['json'])]);
    if (file == null) return;
    setState(() {
      _busy = true;
      _error = null;
      _imported = null;
    });
    try {
      final r = await ref.read(apiProvider).importFile(file.path);
      await ref.read(accountsProvider.notifier).reload();
      ref.read(dataTickProvider.notifier).bump();
      if (mounted) setState(() => _imported = r);
    } catch (e) {
      if (mounted) setState(() => _error = e);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _export(Account? account) async {
    if (_busy) return;
    final f = _formats.firstWhere((f) => f.id == _format);
    final single = !(_allAccounts && _format == 'uigf4');
    if (single && account == null) {
      ref.read(toastProvider.notifier).error('Select an account first (or import / refresh some data).');
      return;
    }
    final location = await getSaveLocation(
      suggestedName: 'wishtory-${f.fileName}.json',
      acceptedTypeGroups: [const XTypeGroup(label: 'JSON', extensions: ['json'])],
    );
    if (location == null) return;
    setState(() => _busy = true);
    try {
      await ref.read(apiProvider).exportFile(location.path, _format, game: single ? account!.game : null, uid: single ? account!.uid : null);
      ref.read(toastProvider.notifier).ok('Export saved.');
    } catch (e) {
      ref.read(toastProvider.notifier).error(recoveryFor(e).detail);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final game = ref.watch(navProvider.select((n) => n.game));
    final account = ref.watch(currentAccountProvider);
    bool usable(_Format f) => f.game == null || f.game == game;
    if (!_formats.any((f) => f.id == _format && usable(f))) _format = 'uigf4';

    return PageBody(maxWidth: 900, children: [
      const Text('Import / Export', style: Ty.h1),
      const SizedBox(height: 4),
      Row(children: [
        Text('Wishtory speaks UIGF, the shared standard for wish history. ', style: Ty.body.copyWith(color: c.muted)),
        TextButton(onPressed: () => ref.read(apiProvider).openExternal('https://uigf.org/'), child: const Text('About UIGF')),
      ]),
      const SizedBox(height: 16),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Text('Import', style: Ty.h2),
          const SizedBox(height: 4),
          Text('Paimon.moe local wish backups, UIGF v4.x, older UIGF (v2/v3), or SRGF v1.0. Importing the same file again does not create duplicates.', style: Ty.body.copyWith(color: c.muted)),
          const SizedBox(height: 12),
          FilledButton.icon(onPressed: _busy ? null : _import, icon: const Icon(Icons.upload_file, size: 18), label: const Text('Choose file…')),
          if (_imported != null) ...[
            const SizedBox(height: 12),
            Semantics(
              liveRegion: true,
              child: StatusChip(
                'Imported ${_imported!.format}: ${plural(_imported!.added, 'new pull')} across ${plural(_imported!.accounts, 'account')}'
                '${_imported!.skipped > 0 ? '. ${_imported!.skipped} rows were skipped (malformed, or item ids not recognized)' : ''}',
                color: c.ok,
                icon: Icons.check_circle_outline,
              ),
            ),
          ],
          if (_error != null) ...[
            const SizedBox(height: 12),
            RecoveryCard(recovery: recoveryFor(_error!), onDismiss: () => setState(() => _error = null)),
          ],
        ]),
      ),
      const SizedBox(height: 16),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Text('Export', style: Ty.h2),
          const SizedBox(height: 10),
          Text('Game', style: Ty.small.copyWith(color: c.muted)),
          const SizedBox(height: 6),
          SegmentedButton<GameId>(
            showSelectedIcon: false,
            segments: [for (final g in GameId.values) ButtonSegment(value: g, label: Text(GameInfo.of(g).short))],
            selected: {game},
            onSelectionChanged: (v) => ref.read(navProvider.notifier).selectGame(v.first),
          ),
          const SizedBox(height: 12),
          RadioGroup<String>(
            groupValue: _format,
            onChanged: (v) => setState(() => _format = v ?? _format),
            child: Column(children: [
              for (final f in _formats)
                RadioListTile<String>(
                  value: f.id,
                  enabled: usable(f),
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  title: Text(f.label, style: Ty.h3),
                  subtitle: Text(
                    f.game != null && !usable(f) ? '${f.note} Choose ${GameInfo.of(f.game!).short} above to use it.' : f.note,
                    style: Ty.small.copyWith(color: c.muted),
                  ),
                ),
            ]),
          ),
          if (_format == 'uigf4') ...[
            const SizedBox(height: 8),
            SegmentedButton<bool>(
              showSelectedIcon: false,
              segments: const [
                ButtonSegment(value: false, label: Text('Selected account')),
                ButtonSegment(value: true, label: Text('All accounts')),
              ],
              selected: {_allAccounts},
              onSelectionChanged: (s) => setState(() => _allAccounts = s.first),
            ),
          ],
          const SizedBox(height: 14),
          FilledButton.icon(onPressed: _busy ? null : () => _export(account), icon: const Icon(Icons.download, size: 18), label: const Text('Export…')),
        ]),
      ),
    ]);
  }
}
