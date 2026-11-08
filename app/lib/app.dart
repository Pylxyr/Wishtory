import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'features/shell/shell.dart';
import 'state/settings.dart';
import 'ui/tokens.dart';

class WishtoryApp extends ConsumerWidget {
  const WishtoryApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(settingsProvider.select((s) => s.themeMode));
    return MaterialApp(
      title: 'Wishtory',
      debugShowCheckedModeBanner: false,
      themeMode: mode,
      theme: buildTheme(Brightness.light),
      darkTheme: buildTheme(Brightness.dark),
      home: const AppShell(),
    );
  }
}

/// Shown instead of the app when the native library could not be loaded or the data folder opened.
class StartupError extends StatelessWidget {
  const StartupError(this.error, {super.key});
  final Object error;

  @override
  Widget build(BuildContext context) => MaterialApp(
        debugShowCheckedModeBanner: false,
        theme: buildTheme(Brightness.dark),
        home: Scaffold(
          body: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 640),
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(mainAxisSize: MainAxisSize.min, crossAxisAlignment: CrossAxisAlignment.start, children: [
                  const Text("Wishtory couldn't start", style: Ty.h1),
                  const SizedBox(height: 8),
                  const Text(
                    'The Rust core could not be loaded or its data folder could not be opened. '
                    'If you are running from source, build it first: cargo build -p wishtory-bridge --release (see docs/redesign/BUILDING.md).',
                    style: Ty.body,
                  ),
                  const SizedBox(height: 12),
                  SelectableText('$error', style: Ty.small.copyWith(fontFamily: Ty.mono)),
                ]),
              ),
            ),
          ),
        ),
      );
}
