import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../state/nav.dart';
import '../state/settings.dart';
import 'themes.dart';
import 'tokens.dart';

/// The colour themes as swatches, each previewed in the current brightness. Picking one applies it at once.
class ThemePicker extends ConsumerWidget {
  const ThemePicker({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final current = ref.watch(settingsProvider.select((s) => s.themeId));
    final brightness = Theme.of(context).brightness;
    final prefs = ref.read(settingsProvider.notifier);
    final game = ref.watch(navProvider.select((n) => n.game));
    return Wrap(spacing: 10, runSpacing: 10, children: [
      for (final preset in themePresets)
        _Swatch(
          name: preset.name,
          colors: paletteFor(preset.id, brightness),
          selected: current != followGameId && themePreset(current).id == preset.id,
          onTap: () => prefs.setThemeId(preset.id),
        ),
      // Not a palette: Teyvat, Astral or New Eridu, whichever game is open.
      _Swatch(
        name: 'Follow game',
        colors: paletteFor(followGameId, brightness, game: game),
        selected: current == followGameId,
        onTap: () => prefs.setThemeId(followGameId),
      ),
    ]);
  }
}

class _Swatch extends StatelessWidget {
  const _Swatch({required this.name, required this.colors, required this.selected, required this.onTap});

  final String name;
  final WishColors colors;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return Semantics(
      button: true,
      selected: selected,
      label: '$name theme',
      child: ExcludeSemantics(
        child: InkWell(
          borderRadius: BorderRadius.circular(12),
          onTap: onTap,
          child: Container(
            width: 112,
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: c.panel2,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: selected ? c.accent : c.line, width: selected ? 2 : 1),
            ),
            child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
              // A miniature of the window: the page, a panel on it, and the accent.
              Container(
                height: 44,
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(color: colors.bg, borderRadius: BorderRadius.circular(8), border: Border.all(color: colors.line)),
                child: Row(children: [
                  Expanded(
                    child: Container(
                      decoration: BoxDecoration(color: colors.panel, borderRadius: BorderRadius.circular(5), border: Border.all(color: colors.line2)),
                    ),
                  ),
                  const SizedBox(width: 5),
                  Container(width: 14, decoration: BoxDecoration(color: colors.accent, borderRadius: BorderRadius.circular(5))),
                ]),
              ),
              const SizedBox(height: 6),
              Row(children: [
                Expanded(child: Text(name, style: Ty.small.copyWith(color: c.text, fontWeight: FontWeight.w600), maxLines: 1, overflow: TextOverflow.ellipsis)),
                if (selected) Icon(Icons.check_circle, size: 15, color: c.accent),
              ]),
            ]),
          ),
        ),
      ),
    );
  }
}
