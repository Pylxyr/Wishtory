import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../state/providers.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';

final aboutProvider = FutureProvider<AboutInfo>((ref) => ref.read(apiProvider).getAbout());

class AboutPage extends ConsumerWidget {
  const AboutPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final api = ref.read(apiProvider);
    return ref.watch(aboutProvider).when(
          loading: () => const PageBody(maxWidth: 800, children: [Skeleton(height: 200, radius: 12)]),
          error: (e, _) => PageBody(maxWidth: 800, children: [Text('Could not load About: $e')]),
          data: (a) => PageBody(maxWidth: 800, children: [
            Row(crossAxisAlignment: CrossAxisAlignment.baseline, textBaseline: TextBaseline.alphabetic, children: [
              Text(a.name, style: Ty.h1),
              const SizedBox(width: 10),
              Text('v${a.version}', style: Ty.num.copyWith(color: c.muted)),
            ]),
            const SizedBox(height: 6),
            Text(a.description, style: Ty.body.copyWith(color: c.muted)),
            const SizedBox(height: 4),
            Text('${a.copyright} · ${a.license}', style: Ty.small.copyWith(color: c.faint)),
            const SizedBox(height: 12),
            Wrap(spacing: 8, children: [
              OutlinedButton(onPressed: () => api.openExternal(a.repository), child: const Text('Source code')),
              OutlinedButton(onPressed: () => api.openExternal(a.releases), child: const Text('Releases')),
              OutlinedButton(onPressed: () => api.openExternal(a.issues), child: const Text('Report an issue')),
            ]),
            const SizedBox(height: 24),
            const SectionLabel('Please note'),
            Panel(
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                for (final d in a.disclaimers) Padding(padding: const EdgeInsets.symmetric(vertical: 3), child: Text(d, style: Ty.body)),
              ]),
            ),
            for (final g in a.groups) ...[
              const SizedBox(height: 24),
              SectionLabel(g.title),
              if (g.note.isNotEmpty) Padding(padding: const EdgeInsets.only(bottom: 8), child: Text(g.note, style: Ty.small.copyWith(color: c.muted))),
              Panel(
                padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 12),
                child: Column(children: [
                  for (final cr in g.credits)
                    ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      title: Text(cr.name, style: Ty.h3),
                      subtitle: Text([cr.role, if (cr.license.isNotEmpty) cr.license].join(' · '), style: Ty.small.copyWith(color: c.muted)),
                      trailing: const Icon(Icons.open_in_new, size: 16),
                      onTap: () => api.openExternal(cr.url),
                    ),
                ]),
              ),
            ],
          ]),
        );
  }
}
