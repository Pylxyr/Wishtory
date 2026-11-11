import 'dart:io' show Platform;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
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
import '../help/tutorial.dart';
import 'catalog_card.dart';
import 'cloud_backup.dart';

class SettingsPage extends ConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final s = ref.watch(settingsProvider);
    final prefs = ref.read(settingsProvider.notifier);
    return PageBody(maxWidth: 900, children: [
      const Text('Settings', style: Ty.h1),
      const SizedBox(height: 20),
      const SectionLabel('Appearance'),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          const Text('Theme', style: Ty.h3),
          const SizedBox(height: 8),
          SegmentedButton<ThemeMode>(
            showSelectedIcon: false,
            segments: const [
              ButtonSegment(value: ThemeMode.system, label: Text('System'), icon: Icon(Icons.brightness_auto, size: 16)),
              ButtonSegment(value: ThemeMode.dark, label: Text('Dark'), icon: Icon(Icons.dark_mode_outlined, size: 16)),
              ButtonSegment(value: ThemeMode.light, label: Text('Light'), icon: Icon(Icons.light_mode_outlined, size: 16)),
            ],
            selected: {s.themeMode},
            onSelectionChanged: (v) => prefs.setTheme(v.first),
          ),
          const SizedBox(height: 16),
          const Text('Density', style: Ty.h3),
          const SizedBox(height: 2),
          Text('Compact fits more on screen. Comfortable uses larger text, more space around everything and taller table rows.', style: Ty.small.copyWith(color: context.wc.muted)),
          const SizedBox(height: 8),
          SegmentedButton<bool>(
            showSelectedIcon: false,
            segments: const [
              ButtonSegment(value: false, label: Text('Compact')),
              ButtonSegment(value: true, label: Text('Comfortable')),
            ],
            selected: {s.comfortable},
            onSelectionChanged: (v) => prefs.setComfortable(v.first),
          ),
        ]),
      ),
      if (Platform.isLinux) ...[
        const SizedBox(height: 24),
        const SectionLabel('Game locations'),
        const _GameLocations(),
      ],
      const SizedBox(height: 24),
      const SectionLabel('Updates'),
      const _Updates(),
      const SizedBox(height: 24),
      const SectionLabel('Game catalog'),
      const CatalogCard(),
      const SizedBox(height: 24),
      const SectionLabel('Cloud backup'),
      const CloudBackupPanel(),
      const SizedBox(height: 24),
      const SectionLabel('Item pictures'),
      const _ItemPictures(),
      const SizedBox(height: 24),
      const SectionLabel('Diagnostics'),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          SwitchListTile(
            contentPadding: EdgeInsets.zero,
            title: const Text('Keep a diagnostics log', style: Ty.h3),
            subtitle: Text('Off by default. Kept only in memory (last 1,000 lines), never written to disk, and never contains links or keys. Resets when you quit.', style: Ty.small.copyWith(color: c.muted)),
            value: s.loggingEnabled,
            onChanged: (v) async {
              await prefs.setLogging(v);
              if (v) ref.read(navProvider.notifier).goTo(AppPage.logs);
            },
          ),
          const Divider(),
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(onPressed: () => showTutorial(context, ref), icon: const Icon(Icons.school_outlined, size: 18), label: const Text('Show the first-run tour again')),
          ),
        ]),
      ),
    ]);
  }
}

class _Updates extends ConsumerWidget {
  const _Updates();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final update = ref.watch(appUpdateProvider);
    final u = update.data;
    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        const Text('Wishtory', style: Ty.h3),
        const SizedBox(height: 4),
        Text(
          u == null
              ? 'Wishtory only compares version numbers; nothing is downloaded or installed automatically.'
              : u.available
                  ? 'Version ${u.latest} is available (you have ${u.current}).'
                  : 'You are up to date (${u.current}).',
          style: Ty.body.copyWith(color: c.muted),
        ),
        const SizedBox(height: 10),
        Wrap(spacing: 8, children: [
          OutlinedButton(
            onPressed: update.loading ? null : () => ref.read(appUpdateProvider.notifier).check(),
            child: Text(update.loading ? 'Checking…' : 'Check for updates'),
          ),
          if (u != null && u.available) FilledButton(onPressed: () => ref.read(apiProvider).openExternal(u.url), child: const Text('View release')),
        ]),
      ]),
    );
  }
}

class _GameLocations extends ConsumerStatefulWidget {
  const _GameLocations();

  @override
  ConsumerState<_GameLocations> createState() => _GameLocationsState();
}

class _GameLocationsState extends ConsumerState<_GameLocations> {
  GameLocations? _locations;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _load(() => ref.read(apiProvider).getGameLocations());
  }

  Future<void> _load(Future<GameLocations> Function() action) async {
    try {
      final l = await action();
      if (mounted) setState(() => _locations = l);
    } catch (e) {
      if (mounted) setState(() => _error = e);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final l = _locations;
    if (_error != null) return RecoveryCard(recovery: recoveryFor(_error!, isLinux: true));
    if (l == null) return const Skeleton(height: 90, radius: 12);
    final api = ref.read(apiProvider);
    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Text(
          'Wishtory looks for the games in Wine and Proton prefixes it finds on its own. If yours is somewhere else, add the folder that contains the game (or the prefix).',
          style: Ty.body.copyWith(color: c.muted),
        ),
        const SizedBox(height: 12),
        for (final p in l.detected)
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.auto_awesome_outlined, size: 18),
            title: Text(p.path, style: Ty.small.copyWith(fontFamily: Ty.mono)),
            subtitle: Text('Found automatically · ${p.games.map((g) => GameInfo.of(g).name).join(', ')}'),
          ),
        for (final path in l.locations)
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.folder_outlined, size: 18),
            title: Text(path, style: Ty.small.copyWith(fontFamily: Ty.mono)),
            trailing: IconButton(
              tooltip: 'Remove',
              icon: const Icon(Icons.close, size: 16),
              onPressed: () => _load(() => api.setGameLocations([for (final x in l.locations) if (x != path) x])),
            ),
          ),
        if (l.detected.isEmpty && l.locations.isEmpty) Text('No locations yet.', style: Ty.small.copyWith(color: c.faint)),
        const SizedBox(height: 8),
        OutlinedButton.icon(
          onPressed: () async {
            final dir = await getDirectoryPath();
            if (dir != null) _load(() => api.setGameLocations([...l.locations, dir]));
          },
          icon: const Icon(Icons.create_new_folder_outlined, size: 18),
          label: const Text('Add a folder…'),
        ),
      ]),
    );
  }
}

/// Item pictures are saved on this PC after their first download. Shows the size and clears them.
class _ItemPictures extends ConsumerStatefulWidget {
  const _ItemPictures();

  @override
  ConsumerState<_ItemPictures> createState() => _ItemPicturesState();
}

class _ItemPicturesState extends ConsumerState<_ItemPictures> {
  int? _bytes;

  @override
  void initState() {
    super.initState();
    _measure();
  }

  Future<void> _measure() async {
    final bytes = await ref.read(iconCacheProvider).sizeBytes();
    if (mounted) setState(() => _bytes = bytes);
  }

  String _size(int bytes) => bytes < 1024 * 1024 ? '${(bytes / 1024).round()} KB' : '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final bytes = _bytes;
    return Panel(
      child: Row(children: [
        Expanded(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            const Text('Saved item pictures', style: Ty.h3),
            const SizedBox(height: 4),
            Text(
              'Pictures are downloaded from Enka.Network once and kept on this PC, so they load instantly and work offline. '
              '${bytes == null ? '' : 'Using ${_size(bytes)}. '}Clearing them is safe: they download again when needed.',
              style: Ty.body.copyWith(color: c.muted),
            ),
          ]),
        ),
        const SizedBox(width: 16),
        OutlinedButton(
          onPressed: bytes == 0
              ? null
              : () async {
                  await ref.read(iconCacheProvider).clear();
                  ref.invalidate(iconFileProvider);
                  ref.read(toastProvider.notifier).ok('Saved pictures cleared.');
                  await _measure();
                },
          child: const Text('Clear'),
        ),
      ]),
    );
  }
}
