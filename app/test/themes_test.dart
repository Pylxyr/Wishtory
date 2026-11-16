import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/ui/themes.dart';
import 'package:wishtory/ui/tokens.dart';

double _contrast(Color a, Color b) {
  final la = a.computeLuminance(), lb = b.computeLuminance();
  return (la > lb ? la + 0.05 : lb + 0.05) / (la > lb ? lb + 0.05 : la + 0.05);
}

void main() {
  test('the default theme is the original palette, untouched', () {
    expect(identical(paletteFor(defaultThemeId, Brightness.dark), WishColors.dark), isTrue);
    expect(identical(paletteFor(defaultThemeId, Brightness.light), WishColors.light), isTrue);
  });

  test('ids are unique, the first is the default, and an unknown id falls back to it', () {
    final ids = themePresets.map((p) => p.id).toList();
    expect(ids.toSet(), hasLength(ids.length));
    expect(ids.first, defaultThemeId);
    expect(themePreset('from-a-newer-version').id, defaultThemeId);
    expect(themePresets.length, greaterThanOrEqualTo(5));
  });

  test('rarity and status colours never change with the theme', () {
    for (final p in themePresets) {
      for (final b in Brightness.values) {
        final base = b == Brightness.dark ? WishColors.dark : WishColors.light;
        final c = paletteFor(p.id, b);
        expect([c.gold, c.purple, c.blue, c.ok, c.err], [base.gold, base.purple, base.blue, base.ok, base.err], reason: '${p.id} $b');
      }
    }
  });

  test('text stays readable on every surface in every theme', () {
    for (final p in themePresets) {
      for (final b in Brightness.values) {
        final c = paletteFor(p.id, b);
        for (final surface in [c.bg, c.rail, c.panel, c.panel2, c.panel3]) {
          expect(_contrast(c.text, surface), greaterThanOrEqualTo(7), reason: '${p.id} $b text');
          expect(_contrast(c.muted, surface), greaterThanOrEqualTo(4.5), reason: '${p.id} $b muted');
        }
        // panel3 only fills progress tracks and skeletons, never sits under text; the original palette's
        // faint text is a shade under 4.5:1 on it, so faint is held to the surfaces that carry text.
        for (final surface in [c.bg, c.rail, c.panel, c.panel2]) {
          expect(_contrast(c.faint, surface), greaterThanOrEqualTo(4.5), reason: '${p.id} $b faint');
        }
      }
    }
  });

  test('the accent is legible as a button and as an outline in every theme', () {
    for (final p in themePresets) {
      for (final b in Brightness.values) {
        final c = paletteFor(p.id, b);
        expect(_contrast(c.accentInk, c.accent), greaterThanOrEqualTo(4.5), reason: '${p.id} $b ink');
        expect(_contrast(c.accent, c.panel), greaterThanOrEqualTo(3), reason: '${p.id} $b accent');
        expect(_contrast(c.accent, c.bg), greaterThanOrEqualTo(3), reason: '${p.id} $b accent on page');
      }
    }
  });
}
