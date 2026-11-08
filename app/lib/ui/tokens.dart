import 'dart:ui' show FontFeature;

import 'package:flutter/material.dart';

/// Wishtory's design tokens: graphite surfaces, one blue accent for every game, gold reserved for
/// 5★ and purple for 4★. Values carry over from the audited CSS of the previous UI.
@immutable
class WishColors extends ThemeExtension<WishColors> {
  const WishColors({
    required this.bg,
    required this.rail,
    required this.panel,
    required this.panel2,
    required this.panel3,
    required this.line,
    required this.line2,
    required this.control,
    required this.text,
    required this.muted,
    required this.faint,
    required this.accent,
    required this.accentInk,
    required this.gold,
    required this.purple,
    required this.blue,
    required this.ok,
    required this.err,
    required this.onErr,
  });

  final Color bg, rail, panel, panel2, panel3, line, line2, control;
  final Color text, muted, faint;
  final Color accent, accentInk, gold, purple, blue, ok, err, onErr;

  static const dark = WishColors(
    bg: Color(0xFF0D1015),
    rail: Color(0xFF11151B),
    panel: Color(0xFF151A22),
    panel2: Color(0xFF1B212B),
    panel3: Color(0xFF252D39),
    line: Color(0xFF262E3A),
    line2: Color(0xFF36404F),
    control: Color(0xFF5A6678),
    text: Color(0xFFE9EDF2),
    muted: Color(0xFF9CA6B5),
    faint: Color(0xFF8793A5),
    accent: Color(0xFF5AA2FF),
    accentInk: Color(0xFF06101F),
    gold: Color(0xFFF2BD4B),
    purple: Color(0xFFB48CF2),
    blue: Color(0xFF7FB0F0),
    ok: Color(0xFF55D39B),
    err: Color(0xFFFF7D7D),
    onErr: Color(0xFF1C0808),
  );

  static const light = WishColors(
    bg: Color(0xFFF2F4F7),
    rail: Color(0xFFFFFFFF),
    panel: Color(0xFFFFFFFF),
    panel2: Color(0xFFEEF1F5),
    panel3: Color(0xFFE3E8EE),
    line: Color(0xFFDFE4EA),
    line2: Color(0xFFC5CCD6),
    control: Color(0xFF838D9C),
    text: Color(0xFF161B22),
    muted: Color(0xFF4F5A69),
    faint: Color(0xFF5F6B7B),
    accent: Color(0xFF1F5FD0),
    accentInk: Color(0xFFFFFFFF),
    gold: Color(0xFF8A5A00),
    purple: Color(0xFF6A3FC4),
    blue: Color(0xFF2457B0),
    ok: Color(0xFF14724E),
    err: Color(0xFFB42727),
    onErr: Color(0xFFFFFFFF),
  );

  /// Gold for 5★, purple for 4★, muted for everything else.
  Color rarity(int rank) => rank >= 5 ? gold : (rank == 4 ? purple : muted);

  @override
  WishColors copyWith() => this;

  @override
  WishColors lerp(ThemeExtension<WishColors>? other, double t) => t < 0.5 || other is! WishColors ? this : other;
}

extension WishTheme on BuildContext {
  WishColors get wc => Theme.of(this).extension<WishColors>()!;
}

/// Text styles that carry no colour, so they inherit it from the surrounding widget.
class Ty {
  static const String ui = 'GoogleSans';
  static const String mono = 'JetBrainsMono';

  static const h1 = TextStyle(fontSize: 22, fontWeight: FontWeight.w600, height: 1.2, letterSpacing: -0.4);
  static const h2 = TextStyle(fontSize: 16, fontWeight: FontWeight.w600, height: 1.3);
  static const h3 = TextStyle(fontSize: 14, fontWeight: FontWeight.w600, height: 1.3);
  static const body = TextStyle(fontSize: 14, height: 1.5);
  static const small = TextStyle(fontSize: 12, height: 1.4);
  static const label = TextStyle(fontSize: 11, fontWeight: FontWeight.w600, letterSpacing: 0.7, height: 1.2);
  static const num = TextStyle(fontFamily: mono, fontSize: 14, fontFeatures: [FontFeature.tabularFigures()]);
  static const bigNum = TextStyle(
    fontFamily: mono,
    fontSize: 32,
    fontWeight: FontWeight.w600,
    height: 1.1,
    fontFeatures: [FontFeature.tabularFigures()],
  );
}

ThemeData buildTheme(Brightness brightness) {
  final c = brightness == Brightness.dark ? WishColors.dark : WishColors.light;
  final scheme = ColorScheme(
    brightness: brightness,
    primary: c.accent,
    onPrimary: c.accentInk,
    secondary: c.accent,
    onSecondary: c.accentInk,
    error: c.err,
    onError: c.onErr,
    surface: c.panel,
    onSurface: c.text,
    outline: c.control,
    outlineVariant: c.line,
  );
  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    fontFamily: Ty.ui,
    scaffoldBackgroundColor: c.bg,
    canvasColor: c.panel,
    dividerColor: c.line,
    splashFactory: NoSplash.splashFactory,
    visualDensity: VisualDensity.compact,
    extensions: [c],
  );
}
