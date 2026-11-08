import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../state/providers.dart';
import '../../state/toasts.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

class LogsPage extends ConsumerStatefulWidget {
  const LogsPage({super.key});

  @override
  ConsumerState<LogsPage> createState() => _LogsPageState();
}

class _LogsPageState extends ConsumerState<LogsPage> {
  List<LogEntry> _logs = const [];

  @override
  void initState() {
    super.initState();
    _reload();
  }

  Future<void> _reload() async {
    final logs = await ref.read(apiProvider).getLogs();
    if (mounted) setState(() => _logs = logs);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    Color levelColor(String l) => switch (l) {
          'ERROR' => c.err,
          'WARN' => c.gold,
          'DEBUG' => c.faint,
          _ => c.muted,
        };
    return PageBody(maxWidth: 1100, children: [
      Row(children: [
        const Expanded(child: Text('Logs', style: Ty.h1)),
        TextButton.icon(onPressed: _reload, icon: const Icon(Icons.refresh, size: 16), label: const Text('Reload')),
        TextButton.icon(
          onPressed: _logs.isEmpty
              ? null
              : () async {
                  await Clipboard.setData(ClipboardData(text: _logs.map((l) => '${l.time} ${l.level} ${l.source}: ${l.message}').join('\n')));
                  ref.read(toastProvider.notifier).ok('Log copied.');
                },
          icon: const Icon(Icons.copy_all_outlined, size: 16),
          label: const Text('Copy'),
        ),
        TextButton.icon(
          onPressed: () async {
            await ref.read(apiProvider).clearLogs();
            _reload();
          },
          icon: const Icon(Icons.delete_sweep_outlined, size: 16),
          label: const Text('Clear'),
        ),
      ]),
      const SizedBox(height: 12),
      Panel(
        child: _logs.isEmpty
            ? Text('Nothing logged yet. Use the app, then press Reload.', style: Ty.body.copyWith(color: c.muted))
            : SelectionArea(
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  for (final l in _logs)
                    Text.rich(
                      TextSpan(children: [
                        TextSpan(text: '${l.time.substring(11)}  ', style: TextStyle(color: c.faint)),
                        TextSpan(text: '${l.level.padRight(5)}  ', style: TextStyle(color: levelColor(l.level), fontWeight: FontWeight.w600)),
                        TextSpan(text: '${l.source}: ${l.message}'),
                      ]),
                      style: Ty.small.copyWith(fontFamily: Ty.mono),
                    ),
                ]),
              ),
      ),
    ]);
  }
}
