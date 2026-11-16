import 'package:flutter/material.dart';

import 'tokens.dart';

/// A colour theme: a hue that tints every surface, and one accent for each brightness. The first preset is
/// the original graphite look, so people who never open this keep exactly what they had.
@immutable
class ThemePreset {
  const ThemePreset({required this.id, required this.name, required this.hue, required this.accentDark, required this.accentLight, this.tint = 1});

  final String id, name;

  /// 0-360, the hue the surfaces lean towards.
  final double hue;

  /// How much of the hue the surfaces show: 1 is a clear tint, near 0 is grey.
  final double tint;
  final Color accentDark, accentLight;
}

const defaultThemeId = 'default';

const themePresets = <ThemePreset>[
  // The original palette: WishColors.dark and WishColors.light are used as they are.
  ThemePreset(id: defaultThemeId, name: 'Graphite', hue: 215, accentDark: Color(0xFF5AA2FF), accentLight: Color(0xFF1F5FD0)),
  ThemePreset(id: 'ocean', name: 'Ocean', hue: 200, accentDark: Color(0xFF3DC6E8), accentLight: Color(0xFF0A6C8C)),
  ThemePreset(id: 'forest', name: 'Forest', hue: 150, accentDark: Color(0xFF4FD08C), accentLight: Color(0xFF12703F)),
  ThemePreset(id: 'sunset', name: 'Sunset', hue: 18, accentDark: Color(0xFFFF8A57), accentLight: Color(0xFFB3430E)),
  ThemePreset(id: 'rose', name: 'Rose', hue: 335, accentDark: Color(0xFFFF7FAB), accentLight: Color(0xFFB0225C)),
  ThemePreset(id: 'violet', name: 'Violet', hue: 265, accentDark: Color(0xFFA990FF), accentLight: Color(0xFF5A35C8)),
  ThemePreset(id: 'mono', name: 'Mono', hue: 220, tint: 0.18, accentDark: Color(0xFFE4E8EE), accentLight: Color(0xFF1B2330)),
];

/// The preset with this id, or the default one for an id from a newer version of the app.
ThemePreset themePreset(String id) {
  for (final p in themePresets) {
    if (p.id == id) return p;
  }
  return themePresets.first;
}

Color _hsl(double hue, double saturation, double lightness) => HSLColor.fromAHSL(1, hue, saturation.clamp(0.0, 1.0), lightness).toColor();

/// The colours of preset [id] in [brightness]. Gold (5★), purple (4★) and the status colours never change:
/// they carry meaning in every theme.
WishColors paletteFor(String id, Brightness brightness) {
  final dark = brightness == Brightness.dark;
  final base = dark ? WishColors.dark : WishColors.light;
  if (id == defaultThemeId) return base;
  final p = themePreset(id);
  final h = p.hue;
  double s(double v) => v * p.tint;
  // Text roles keep at least 5:1 against every surface (checked by a test).
  if (dark) {
    return WishColors(
      bg: _hsl(h, s(.24), .065),
      rail: _hsl(h, s(.24), .085),
      panel: _hsl(h, s(.22), .105),
      panel2: _hsl(h, s(.20), .135),
      panel3: _hsl(h, s(.18), .18),
      line: _hsl(h, s(.18), .19),
      line2: _hsl(h, s(.16), .26),
      control: _hsl(h, s(.14), .40),
      text: _hsl(h, s(.20), .92),
      muted: _hsl(h, s(.12), .68),
      faint: _hsl(h, s(.10), .62),
      accent: p.accentDark,
      accentInk: _hsl(h, s(.5), .08),
      gold: base.gold,
      purple: base.purple,
      blue: base.blue,
      ok: base.ok,
      err: base.err,
      onErr: base.onErr,
    );
  }
  return WishColors(
    bg: _hsl(h, s(.20), .96),
    rail: const Color(0xFFFFFFFF),
    panel: const Color(0xFFFFFFFF),
    panel2: _hsl(h, s(.18), .94),
    panel3: _hsl(h, s(.16), .90),
    line: _hsl(h, s(.15), .88),
    line2: _hsl(h, s(.12), .79),
    control: _hsl(h, s(.08), .52),
    text: _hsl(h, s(.25), .10),
    muted: _hsl(h, s(.12), .33),
    faint: _hsl(h, s(.10), .36),
    accent: p.accentLight,
    accentInk: const Color(0xFFFFFFFF),
    gold: base.gold,
    purple: base.purple,
    blue: base.blue,
    ok: base.ok,
    err: base.err,
    onErr: base.onErr,
  );
}
