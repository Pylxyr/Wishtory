import 'package:flutter/material.dart';

/// The one Density setting, made visible everywhere: **compact** fits more on screen, **comfortable** uses larger text
/// and more room around everything. It is provided once at the top of the app and read where spacing is decided.
class Density extends InheritedWidget {
  const Density({super.key, required this.comfortable, required super.child});

  final bool comfortable;

  /// Applied to all text, on top of the system text size.
  double get textFactor => comfortable ? 1.08 : 0.94;

  /// Inside a card.
  double get pad => comfortable ? 22 : 13;

  /// Between cards.
  double get gap => comfortable ? 16 : 10;

  /// Vertical padding inside table rows.
  double get rowPad => comfortable ? 7 : 3;

  /// A row of the history table.
  double get tableRow => comfortable ? 48 : 30;

  static Density of(BuildContext context) => context.dependOnInheritedWidgetOfExactType<Density>() ?? const Density(comfortable: false, child: SizedBox.shrink());

  @override
  bool updateShouldNotify(Density old) => comfortable != old.comfortable;
}

/// Wraps the app: provides [Density] and scales text by it.
class DensityScope extends StatelessWidget {
  const DensityScope({super.key, required this.comfortable, required this.child});
  final bool comfortable;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final media = MediaQuery.of(context);
    final system = media.textScaler.scale(14) / 14;
    final factor = Density(comfortable: comfortable, child: child).textFactor;
    return Density(
      comfortable: comfortable,
      child: MediaQuery(data: media.copyWith(textScaler: TextScaler.linear(system * factor)), child: child),
    );
  }
}
