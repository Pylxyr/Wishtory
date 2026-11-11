import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../bridge/wishtory_api.dart';
import '../../domain/recovery.dart';
import '../../state/nav.dart';
import '../../state/providers.dart';
import '../../state/refresh.dart';
import '../../ui/tokens.dart';
import '../../ui/widgets.dart';
import 'catalog_logic.dart';

/// Shown at the top of every page while a release is waiting and "Not now" was not chosen.
class CatalogUpdateNotice extends ConsumerWidget {
  const CatalogUpdateNotice({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final status = ref.watch(catalogStatusProvider).data;
    final onReview = ref.watch(navProvider.select((n) => n.page == AppPage.catalogReview));
    final waiting = status?.waiting;
    if (waiting == null || noticeFor(status) != CatalogNotice.needsReview || onReview) return const SizedBox.shrink();
    final nav = ref.read(navProvider.notifier);
    return Semantics(
      container: true,
      liveRegion: true,
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
        decoration: BoxDecoration(color: c.panel2, border: Border(bottom: BorderSide(color: c.gold.withValues(alpha: 0.6)))),
        child: Row(children: [
          Icon(Icons.rule_folder_outlined, color: c.gold, size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              'A game catalog update (version ${waiting.version}) is waiting for your review. '
              '${affectedSentence(pulls: waiting.affectedPulls.toInt(), accounts: waiting.affectedAccounts.toInt())} It has not been applied.',
              style: Ty.body,
            ),
          ),
          const SizedBox(width: 12),
          TextButton(onPressed: () => ref.read(catalogControllerProvider.notifier).dismiss(), child: const Text('Not now')),
          const SizedBox(width: 4),
          FilledButton(onPressed: () => nav.goTo(AppPage.catalogReview), child: const Text('Review')),
        ]),
      ),
    );
  }
}

class CatalogReviewPage extends ConsumerWidget {
  const CatalogReviewPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final review = ref.watch(catalogReviewProvider);
    return review.when(
      loading: () => const PageBody(maxWidth: 900, children: [Skeleton(height: 160, radius: 12)]),
      error: (e, _) => PageBody(maxWidth: 900, children: [RecoveryCard(recovery: recoveryFor(e))]),
      data: (r) => r == null ? const _NothingWaiting() : _ReviewBody(review: r),
    );
  }
}

class _NothingWaiting extends ConsumerWidget {
  const _NothingWaiting();

  @override
  Widget build(BuildContext context, WidgetRef ref) => PageBody(maxWidth: 900, children: [
        const Text('Game catalog update', style: Ty.h1),
        const SizedBox(height: 16),
        Panel(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text('There is no update waiting.', style: Ty.body.copyWith(color: context.wc.muted)),
            const SizedBox(height: 10),
            OutlinedButton(onPressed: () => ref.read(navProvider.notifier).goTo(AppPage.settings), child: const Text('Back to Settings')),
          ]),
        ),
      ]);
}

class _ReviewBody extends ConsumerWidget {
  const _ReviewBody({required this.review});
  final CatalogReview review;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final c = context.wc;
    final ui = ref.watch(catalogControllerProvider);
    final controller = ref.read(catalogControllerProvider.notifier);
    final nav = ref.read(navProvider.notifier);
    final groups = groupChanges(review.changes);
    final accounts = accountsToReview(review);
    final affected = review.totalAffected.toInt();
    return PageBody(maxWidth: 900, children: [
      Align(
        alignment: Alignment.centerLeft,
        child: TextButton.icon(onPressed: () => nav.goTo(AppPage.settings), icon: const Icon(Icons.arrow_back, size: 18), label: const Text('Back to Settings')),
      ),
      const Text('Game catalog update', style: Ty.h1),
      const SizedBox(height: 4),
      Text('Version ${review.currentVersion} to version ${review.version}', style: Ty.body.copyWith(color: c.muted)),
      const SizedBox(height: 16),
      Panel(
        child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
          Semantics(header: true, child: const Text('What this update does', style: Ty.h3)),
          const SizedBox(height: 6),
          Text(affectedSentence(pulls: affected, accounts: accounts.length), style: Ty.body),
          const SizedBox(height: 4),
          Text(
            'Nothing has been applied yet, and the list below was worked out on this PC from your saved pulls. '
            'Apply installs the whole update at once and then reads your pulls with it. If anything goes wrong, the current catalog stays.',
            style: Ty.body.copyWith(color: c.muted),
          ),
          const SizedBox(height: 12),
          Wrap(spacing: 8, runSpacing: 8, children: [
            FilledButton(
              onPressed: ui.busy
                  ? null
                  : () async {
                      final applied = await controller.apply(review);
                      if (applied) nav.goTo(AppPage.settings);
                    },
              child: Text(ui.applying ? 'Applying…' : 'Apply update'),
            ),
            OutlinedButton(
              onPressed: ui.busy
                  ? null
                  : () async {
                      await controller.dismiss();
                      nav.goTo(AppPage.settings);
                    },
              child: const Text('Not now'),
            ),
          ]),
        ]),
      ),
      const SizedBox(height: 24),
      const SectionLabel('Change log'),
      if (review.changes.isEmpty)
        Panel(child: Text('This update lists no changes.', style: Ty.body.copyWith(color: c.muted)))
      else
        for (final s in severityOrder)
          if (groups[s]!.isNotEmpty) _SeverityGroup(severity: s, entries: groups[s]!),
      const SizedBox(height: 24),
      const SectionLabel('Saved pulls that would change'),
      if (accounts.isEmpty)
        Panel(child: Text('None of your saved pulls would change.', style: Ty.body.copyWith(color: c.muted)))
      else ...[
        for (final a in accounts) ...[
          _AccountImpact(account: a, changes: review.changes, version: review.version.toInt(), fingerprint: review.fingerprint),
          const SizedBox(height: 12),
        ],
        if (review.notListed > 0)
          Text(
            '${pullsWord(review.notListed.toInt())} more would change but are not listed here to keep this page fast. They are included in the counts above.',
            style: Ty.small.copyWith(color: c.muted),
          ),
      ],
    ]);
  }
}

class _SeverityGroup extends StatelessWidget {
  const _SeverityGroup({required this.severity, required this.entries});
  final ChangeSeverity severity;
  final List<ChangeEntry> entries;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final color = switch (severity) {
      ChangeSeverity.major => c.err,
      ChangeSeverity.minor => c.gold,
      ChangeSeverity.info => c.blue,
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Panel(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        child: ExpansionTile(
          // Only the important group starts open; the others are one tap away.
          initiallyExpanded: severity == ChangeSeverity.major,
          tilePadding: EdgeInsets.zero,
          childrenPadding: const EdgeInsets.only(bottom: 8),
          shape: const Border(),
          collapsedShape: const Border(),
          title: Row(children: [
            StatusChip('${severityLabel(severity)} · ${entries.length}', color: color),
            const SizedBox(width: 10),
            Expanded(child: Text(severityMeaning(severity), style: Ty.small.copyWith(color: c.muted))),
          ]),
          children: [
            for (final e in entries)
              Align(
                alignment: Alignment.centerLeft,
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                    Text(e.summary, style: Ty.body),
                    if (e.note != null) Text(e.note!, style: Ty.small.copyWith(color: c.muted, fontStyle: FontStyle.italic)),
                  ]),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

const _pageSize = 25;

/// One account's affected pulls, loaded a page at a time.
class _AccountImpact extends ConsumerStatefulWidget {
  const _AccountImpact({required this.account, required this.changes, required this.version, required this.fingerprint});
  final AccountImpact account;
  final List<ChangeEntry> changes;
  final int version;
  final String fingerprint;

  @override
  ConsumerState<_AccountImpact> createState() => _AccountImpactState();
}

class _AccountImpactState extends ConsumerState<_AccountImpact> {
  final _pulls = <AffectedPull>[];
  bool _loading = false;
  bool _done = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _more();
  }

  Future<void> _more() async {
    if (_loading || _done) return;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final a = widget.account;
      final page = await ref.read(apiProvider).getImpactPage(a.game, a.uid, offset: _pulls.length, limit: _pageSize);
      if (!mounted) return;
      setState(() {
        _pulls.addAll(page);
        _done = page.length < _pageSize || _pulls.length >= a.listed.toInt();
      });
    } catch (e) {
      if (mounted) setState(() => _error = e);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final a = widget.account;
    return Panel(
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        Semantics(header: true, child: Text(accountTitle(a), style: Ty.h3)),
        const SizedBox(height: 2),
        Text('${pullsWord(a.affected.toInt())} of ${a.pulls} would change.', style: Ty.small.copyWith(color: c.muted)),
        const SizedBox(height: 8),
        for (final p in _pulls) _PullTile(pull: p, changes: widget.changes),
        if (_error != null) RecoveryCard(recovery: recoveryFor(_error!)),
        if (_loading) const Padding(padding: EdgeInsets.symmetric(vertical: 8), child: Skeleton(height: 48, radius: 8)),
        if (!_done && !_loading)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: OutlinedButton(onPressed: _more, child: Text('Show more (${a.listed.toInt() - _pulls.length} left)')),
          ),
      ]),
    );
  }
}

class _PullTile extends StatelessWidget {
  const _PullTile({required this.pull, required this.changes});
  final AffectedPull pull;
  final List<ChangeEntry> changes;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    final rows = differencesOf(pull);
    final because = entriesBehind(pull, changes);
    final summary = '${pull.name}, ${pull.time}. ${[for (final d in rows) '${d.label} changes from ${d.before} to ${d.after}'].join('. ')}.';
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(color: c.panel2, borderRadius: BorderRadius.circular(8), border: Border.all(color: c.line)),
      child: Semantics(
        container: true,
        label: summary,
        child: ExcludeSemantics(
          child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Row(children: [
              Expanded(child: Text(pull.name, style: Ty.h3, overflow: TextOverflow.ellipsis)),
              const SizedBox(width: 8),
              RarityChip(game: pull.game, rank: pull.rank.toInt()),
            ]),
            Text(pull.time, style: Ty.small.copyWith(color: c.muted, fontFamily: Ty.mono)),
            const SizedBox(height: 6),
            for (final d in rows)
              Padding(
                padding: const EdgeInsets.only(bottom: 2),
                child: Text.rich(TextSpan(style: Ty.body, children: [
                  TextSpan(text: '${d.label}: ', style: const TextStyle(fontWeight: FontWeight.w600)),
                  TextSpan(text: d.before, style: TextStyle(color: c.muted)),
                  const TextSpan(text: '  →  '),
                  TextSpan(text: d.after),
                ])),
              ),
            const SizedBox(height: 4),
            for (final r in pull.reasons) Text('Why: ${r.text}', style: Ty.small.copyWith(color: c.muted)),
            for (final e in because.take(3)) Text('Change log: ${e.summary}', style: Ty.small.copyWith(color: c.faint)),
          ]),
        ),
      ),
    );
  }
}
