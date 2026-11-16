import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/wishtory_api.dart';
import '../domain/icon_cache.dart';
import '../domain/icons.dart';
import '../state/providers.dart';
import 'tokens.dart';

/// The wish-screen artwork of the items a banner features, side by side, to sit behind the banner's text.
/// Each picture is shown whole (never cropped through a face) and aligned to the right, where the text
/// is not; until a picture has downloaded, or when there is none, a soft gradient shows instead.
class BannerArt extends StatelessWidget {
  const BannerArt({super.key, required this.game, required this.names});

  final GameId game;
  final List<String> names;

  @override
  Widget build(BuildContext context) {
    if (names.isEmpty) return const SizedBox.shrink();
    return Row(children: [
      for (final name in names)
        Expanded(child: _ArtSlice(game: game, name: name, alignment: names.length == 1 ? Alignment.centerRight : Alignment.center)),
    ]);
  }
}

class _ArtSlice extends ConsumerWidget {
  const _ArtSlice({required this.game, required this.name, required this.alignment});

  final GameId game;
  final String name;
  final Alignment alignment;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final item = ref.watch(catalogIndexProvider).find(game, name);
    final urls = ref.watch(iconTableProvider).bannerArtUrls(item);
    if (urls.isEmpty) return const SizedBox.shrink();
    final file = switch (ref.watch(iconFileProvider(IconRequest(urls, art: true)))) {
      AsyncData(:final value) => value,
      _ => null,
    };
    if (file == null) return const SizedBox.shrink();
    return Image.file(
      file,
      fit: BoxFit.contain,
      alignment: alignment,
      // Decoded at the size it is shown, not at the size of the file.
      cacheHeight: (360 * MediaQuery.devicePixelRatioOf(context)).round(),
      gaplessPlayback: true,
      excludeFromSemantics: true,
      errorBuilder: (_, _, _) => ColoredBox(color: c.panel2.withValues(alpha: 0)),
    );
  }
}
