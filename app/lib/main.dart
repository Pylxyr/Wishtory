import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart' show ExternalLibrary;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:window_manager/window_manager.dart';

import 'app.dart';
import 'bridge/generated/frb_generated.dart';
import 'bridge/wishtory_api.dart';
import 'domain/icons.dart';
import 'state/providers.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await windowManager.ensureInitialized();
  await windowManager.waitUntilReadyToShow(
    const WindowOptions(size: Size(1280, 820), minimumSize: Size(960, 640), center: true, title: 'Wishtory'),
    () async {
      await windowManager.show();
      await windowManager.focus();
    },
  );

  final prefs = await SharedPreferences.getInstance();
  final icons = await IconTable.load();
  try {
    await RustLib.init(externalLibrary: ExternalLibrary.open(_nativeLibraryPath()));
    final api = await RustWishtoryApi.open();
    runApp(ProviderScope(
      overrides: [
        sharedPrefsProvider.overrideWithValue(prefs),
        apiProvider.overrideWithValue(api),
        iconTableProvider.overrideWithValue(icons),
      ],
      child: const WishtoryApp(),
    ));
  } catch (e) {
    runApp(StartupError(e));
  }
}

/// Where the Rust library lives: next to the executable in a built bundle, or in the Cargo target
/// folder when running from source. `WISHTORY_NATIVE_LIB` overrides both.
String _nativeLibraryPath() {
  final override = Platform.environment['WISHTORY_NATIVE_LIB'];
  if (override != null && override.isNotEmpty) return override;

  final file = Platform.isWindows
      ? 'wishtory_bridge.dll'
      : Platform.isMacOS
          ? 'libwishtory_bridge.dylib'
          : 'libwishtory_bridge.so';
  final exeDir = File(Platform.resolvedExecutable).parent.path;
  final candidates = <String>[
    '$exeDir${Platform.pathSeparator}$file',
    '$exeDir${Platform.pathSeparator}lib${Platform.pathSeparator}$file',
  ];
  // Running from source: walk up from the working directory looking for target/{release,debug}.
  var dir = Directory.current;
  for (var i = 0; i < 5; i++) {
    for (final profile in ['release', 'debug']) {
      candidates.add('${dir.path}${Platform.pathSeparator}target${Platform.pathSeparator}$profile${Platform.pathSeparator}$file');
    }
    dir = dir.parent;
  }
  return candidates.firstWhere((p) => File(p).existsSync(), orElse: () => candidates.first);
}
