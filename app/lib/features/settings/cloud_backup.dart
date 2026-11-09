import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';
import '../../domain/recovery.dart';
import '../../state/providers.dart';
import '../../state/toasts.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

/// Settings → Cloud backup. Sign in to Google once, then back up and restore.
///
/// Everything is encrypted on this PC with the person's passphrase before it is uploaded, and
/// Wishtory can only see its own hidden app-data folder in the person's Drive (never any other file).
/// The Google sign-in and the passphrase are kept in the system's credential store by the core.
class CloudBackupPanel extends ConsumerStatefulWidget {
  const CloudBackupPanel({super.key});

  @override
  ConsumerState<CloudBackupPanel> createState() => _CloudBackupPanelState();
}

class _CloudBackupPanelState extends ConsumerState<CloudBackupPanel> {
  static const _minPassphrase = 10;

  CloudStatus? _status;
  Object? _loadError;
  bool _busy = false;
  String _busyText = '';
  bool _signingIn = false;
  List<CloudBackup>? _backups;

  final _pass = TextEditingController();
  final _again = TextEditingController();
  bool _showPass = false;
  String? _passError;

  @override
  void initState() {
    super.initState();
    _loadStatus();
  }

  @override
  void dispose() {
    _pass.dispose();
    _again.dispose();
    super.dispose();
  }

  WishtoryApi get _api => ref.read(apiProvider);
  ToastController get _toast => ref.read(toastProvider.notifier);

  Future<void> _loadStatus() async {
    try {
      final status = await _api.cloudStatus();
      if (mounted) setState(() => _status = status);
    } catch (e) {
      if (mounted) setState(() => _loadError = e);
    }
  }

  /// Runs one action with the busy line shown; failures become a toast with the core's message.
  Future<T?> _run<T>(String busyText, Future<T> Function() action) async {
    setState(() {
      _busy = true;
      _busyText = busyText;
    });
    try {
      return await action();
    } catch (e) {
      _toast.error(recoveryFor(e).detail);
      return null;
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _signIn() async {
    final phrase = _pass.text;
    if (phrase.runes.length < _minPassphrase) {
      setState(() => _passError = 'Use at least $_minPassphrase characters.');
      return;
    }
    if (phrase != _again.text) {
      setState(() => _passError = 'The two passphrases do not match.');
      return;
    }
    setState(() {
      _passError = null;
      _signingIn = true;
    });
    final status = await _run('Waiting for you to finish signing in to Google in your browser…', () => _api.cloudSignIn(phrase));
    if (!mounted) return;
    setState(() => _signingIn = false);
    if (status != null) {
      _pass.clear();
      _again.clear();
      setState(() => _status = status);
      _toast.ok('Connected. You can back up now.');
    } else {
      await _loadStatus();
    }
  }

  Future<void> _backupNow() async {
    final file = await _run('Backing up…', () => _api.cloudBackupNow());
    if (file == null) return;
    await _loadStatus();
    if (_backups != null) await _loadBackups();
    _toast.ok('Backed up to your Google Drive.');
  }

  Future<void> _setAuto(bool on) async {
    final status = await _run('Saving…', () => _api.cloudSetAutoBackup(on));
    if (status != null && mounted) setState(() => _status = status);
  }

  Future<void> _loadBackups() async {
    final list = await _run('Loading your backups…', () => _api.cloudListBackups());
    if (list != null && mounted) setState(() => _backups = list);
  }

  Future<void> _restore(CloudBackup backup) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: context.wc.panel,
        title: const Text('Restore this backup?', style: Ty.h2),
        content: SizedBox(
          width: 420,
          child: Text(
            'The backup from ${_when(backup.created)} is merged into your saved history. Pulls you already have are skipped and nothing is removed.',
            style: Ty.body.copyWith(color: context.wc.muted),
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          FilledButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('Restore')),
        ],
      ),
    );
    if (ok != true) return;
    final summary = await _run('Restoring…', () => _api.cloudRestore(backup.id));
    if (summary == null) return;
    await ref.read(accountsProvider.notifier).reload();
    ref.read(dataTickProvider.notifier).bump();
    _toast.ok(
      summary.added == 0
          ? 'Restored. Everything in this backup was already saved.'
          : 'Restored: ${plural(summary.added, 'new pull')} across ${plural(summary.accounts, 'account')}.',
    );
  }

  Future<void> _signOut() async {
    final choice = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        backgroundColor: context.wc.panel,
        title: const Text('Disconnect from Google?', style: Ty.h2),
        content: SizedBox(
          width: 440,
          child: Text(
            'Wishtory forgets your Google sign-in and the backup passphrase on this PC, and tells Google to stop honouring it. '
            'Your saved history on this PC is not touched.\n\n'
            'Your existing backups can stay in your Drive (you can reconnect later and use them with the same passphrase), '
            'or be deleted now.',
            style: Ty.body.copyWith(color: context.wc.muted),
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          OutlinedButton(onPressed: () => Navigator.pop(ctx, 'delete'), child: const Text('Disconnect and delete backups')),
          FilledButton(onPressed: () => Navigator.pop(ctx, 'keep'), child: const Text('Disconnect')),
        ],
      ),
    );
    if (choice == null) return;
    final status = await _run('Disconnecting…', () => _api.cloudSignOut(deleteBackups: choice == 'delete'));
    if (status == null || !mounted) return;
    setState(() {
      _status = status;
      _backups = null;
    });
    _toast.ok(choice == 'delete' ? 'Disconnected, and your backups were deleted from Drive.' : 'Disconnected from Google.');
  }

  // Two-digit helper for dates; no extra package needed.
  static String _two(int n) => n.toString().padLeft(2, '0');

  static String _when(String rfc3339) {
    final t = DateTime.tryParse(rfc3339)?.toLocal();
    if (t == null) return 'an unknown time';
    return '${t.year}-${_two(t.month)}-${_two(t.day)} ${_two(t.hour)}:${_two(t.minute)}';
  }

  static String _size(int bytes) => bytes < 1024 * 1024 ? '${(bytes / 1024).ceil()} KB' : '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';

  @override
  Widget build(BuildContext context) {
    if (_loadError != null) return RecoveryCard(recovery: recoveryFor(_loadError!));
    final status = _status;
    if (status == null) return const Skeleton(height: 120, radius: 12);
    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        if (!status.available)
          _unavailable(context)
        else if (!status.signedIn)
          _signedOut(context)
        else
          _signedIn(context, status),
        if (_busy) ...[
          const SizedBox(height: 14),
          Row(children: [
            const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)),
            const SizedBox(width: 10),
            Expanded(child: Text(_busyText, style: Ty.small.copyWith(color: context.wc.muted))),
            if (_signingIn) TextButton(onPressed: () => _api.cloudCancelSignIn(), child: const Text('Cancel')),
          ]),
        ],
      ]),
    );
  }

  Widget _unavailable(BuildContext context) {
    final c = context.wc;
    return Text(
      'This copy of Wishtory was built without cloud backup, so nothing is ever sent to Google. '
      'You can still back up with Export on the Import / Export page.',
      style: Ty.body.copyWith(color: c.muted),
    );
  }

  Widget _signedOut(BuildContext context) {
    final c = context.wc;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      const Text('Back up to your Google Drive', style: Ty.h3),
      const SizedBox(height: 4),
      Text(
        'Off until you connect it. Your wish history is encrypted on this PC with a passphrase that only you know, and only then uploaded to a hidden folder in your own Google Drive. '
        'Google can store the file but not read it, and Wishtory can see nothing else in your Drive. '
        'Forget the passphrase and the backups cannot be opened, so keep it somewhere safe.',
        style: Ty.body.copyWith(color: c.muted),
      ),
      const SizedBox(height: 14),
      SizedBox(
        width: 360,
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          TextField(
            controller: _pass,
            enabled: !_busy,
            obscureText: !_showPass,
            autocorrect: false,
            enableSuggestions: false,
            decoration: fieldDecoration(
              context,
              hint: 'Backup passphrase (at least $_minPassphrase characters)',
              suffix: IconButton(
                tooltip: _showPass ? 'Hide' : 'Show',
                icon: Icon(_showPass ? Icons.visibility_off_outlined : Icons.visibility_outlined, size: 18),
                onPressed: () => setState(() => _showPass = !_showPass),
              ),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _again,
            enabled: !_busy,
            obscureText: !_showPass,
            autocorrect: false,
            enableSuggestions: false,
            decoration: fieldDecoration(context, hint: 'Type it again'),
            onSubmitted: (_) {
              if (!_busy) _signIn();
            },
          ),
          if (_passError != null) ...[
            const SizedBox(height: 6),
            Text(_passError!, style: Ty.small.copyWith(color: c.err)),
          ],
        ]),
      ),
      const SizedBox(height: 12),
      FilledButton.icon(
        onPressed: _busy ? null : _signIn,
        icon: const Icon(Icons.cloud_upload_outlined, size: 18),
        label: const Text('Sign in with Google'),
      ),
      const SizedBox(height: 8),
      Text(
        'Opens Google\'s own sign-in page in your browser. Wishtory never sees your Google password.',
        style: Ty.small.copyWith(color: c.faint),
      ),
    ]);
  }

  Widget _signedIn(BuildContext context, CloudStatus status) {
    final c = context.wc;
    final last = status.lastBackup;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        const Icon(Icons.cloud_done_outlined, size: 20),
        const SizedBox(width: 8),
        Expanded(child: Text(status.account ?? 'Google account', style: Ty.h3, overflow: TextOverflow.ellipsis)),
      ]),
      const SizedBox(height: 4),
      Text(
        last == null ? 'No backup yet.' : 'Last backup: ${_when(last)}.',
        style: Ty.body.copyWith(color: c.muted),
      ),
      const SizedBox(height: 12),
      Wrap(spacing: 8, runSpacing: 8, children: [
        FilledButton.icon(
          onPressed: _busy ? null : _backupNow,
          icon: const Icon(Icons.cloud_upload_outlined, size: 18),
          label: const Text('Back up now'),
        ),
        OutlinedButton.icon(
          onPressed: _busy ? null : _loadBackups,
          icon: const Icon(Icons.cloud_download_outlined, size: 18),
          label: const Text('Restore…'),
        ),
        TextButton(onPressed: _busy ? null : _signOut, child: const Text('Disconnect')),
      ]),
      const Divider(height: 28),
      SwitchListTile(
        contentPadding: EdgeInsets.zero,
        title: const Text('Back up automatically', style: Ty.h3),
        subtitle: Text(
          'At most once a day, while Wishtory is open. The last 5 backups are kept in your Drive.',
          style: Ty.small.copyWith(color: c.muted),
        ),
        value: status.autoBackup,
        onChanged: _busy ? null : _setAuto,
      ),
      if (_backups != null) ...[
        const Divider(height: 28),
        const Text('Backups in your Drive', style: Ty.h3),
        const SizedBox(height: 6),
        if (_backups!.isEmpty) Text('No backups yet.', style: Ty.small.copyWith(color: c.faint)),
        for (final b in _backups!)
          ListTile(
            dense: true,
            contentPadding: EdgeInsets.zero,
            leading: const Icon(Icons.insert_drive_file_outlined, size: 18),
            title: Text(_when(b.created)),
            subtitle: Text(_size(b.size)),
            trailing: OutlinedButton(onPressed: _busy ? null : () => _restore(b), child: const Text('Restore')),
          ),
      ],
    ]);
  }
}
