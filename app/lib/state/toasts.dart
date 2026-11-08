import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

enum ToastKind { info, ok, error }

class Toast {
  const Toast(this.id, this.kind, this.message);
  final int id;
  final ToastKind kind;
  final String message;
}

/// Short messages. Errors stay until dismissed; others fade after a few seconds.
class ToastController extends Notifier<List<Toast>> {
  int _next = 0;

  @override
  List<Toast> build() => const [];

  void show(String message, {ToastKind kind = ToastKind.info}) {
    final toast = Toast(_next++, kind, message);
    state = [...state.where((t) => t.message != message), toast];
    if (kind != ToastKind.error) {
      Timer(const Duration(seconds: 5), () => dismiss(toast.id));
    }
  }

  void error(String message) => show(message, kind: ToastKind.error);
  void ok(String message) => show(message, kind: ToastKind.ok);

  void dismiss(int id) {
    if (!ref.mounted) return;
    state = [
      for (final t in state)
        if (t.id != id) t,
    ];
  }
}

final toastProvider = NotifierProvider<ToastController, List<Toast>>(ToastController.new);
