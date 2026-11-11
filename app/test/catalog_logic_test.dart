import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/bridge/wishtory_api.dart';
import 'package:wishtory/features/settings/catalog_logic.dart';

ChangeEntry change(String id, ChangeSeverity severity, {String? note}) => ChangeEntry(
      id: id,
      scope: ChangeScope.banners,
      game: GameId.genshin,
      ids: [id],
      summary: 'Summary of $id',
      severity: severity,
      note: note,
    );

WaitingRelease waiting({bool dismissed = false, int pulls = 3, int accounts = 1}) => WaitingRelease(
      version: 8,
      generatedAt: '2026-11-01T00:00:00Z',
      major: 1,
      minor: 2,
      info: 3,
      dismissed: dismissed,
      affectedPulls: pulls,
      affectedAccounts: accounts,
    );

CatalogStatus status({WaitingRelease? waiting, int? lastCheck}) => CatalogStatus(
      version: 7,
      components: const CatalogComponents(catalog: 4, banners: 5, rules: 1),
      generatedAt: '2026-10-01T00:00:00Z',
      lastCheck: lastCheck,
      autoApply: true,
      waiting: waiting,
    );

AffectedPull pull({
  String? bannerBefore = 'Event A',
  String? bannerAfter = 'Event A',
  PullOutcome? resultBefore,
  PullOutcome? resultAfter,
  int? pityBefore = 10,
  int? pityAfter = 10,
  ShownItem shownBefore = const ShownItem(name: 'Diluc', rarity: 5),
  ShownItem shownAfter = const ShownItem(name: 'Diluc', rarity: 5),
  List<ImpactReason> reasons = const [],
}) =>
    AffectedPull(
      game: GameId.genshin,
      uid: '1',
      pullId: 'p1',
      time: '2026-01-05 12:00:00',
      itemId: '10000016',
      name: 'Diluc',
      rank: 5,
      bannerBefore: bannerBefore,
      bannerAfter: bannerAfter,
      resultBefore: resultBefore,
      resultAfter: resultAfter,
      pityBefore: pityBefore,
      pityAfter: pityAfter,
      shownBefore: shownBefore,
      shownAfter: shownAfter,
      reasons: reasons,
    );

void main() {
  group('change log grouping', () {
    test('groups by severity in the order major, minor, info and keeps every group', () {
      final groups = groupChanges([
        change('a', ChangeSeverity.info),
        change('b', ChangeSeverity.major),
        change('c', ChangeSeverity.minor),
        change('d', ChangeSeverity.major),
      ]);
      expect(groups.keys.toList(), severityOrder);
      expect(groups[ChangeSeverity.major]!.map((c) => c.id), ['b', 'd']);
      expect(groups[ChangeSeverity.minor]!.map((c) => c.id), ['c']);
      expect(groups[ChangeSeverity.info]!.map((c) => c.id), ['a']);
      expect(groupChanges(const [])[ChangeSeverity.major], isEmpty);
    });

    test('counts are worded without zeros', () {
      expect(countsLabel(major: 2, minor: 0, info: 5), '2 major, 5 info');
      expect(countsLabel(major: 0, minor: 0, info: 0), 'No changes listed');
    });

    test('every severity has words, not only a colour', () {
      for (final s in severityOrder) {
        expect(severityLabel(s), isNotEmpty);
        expect(severityMeaning(s), isNotEmpty);
      }
    });
  });

  group('notice states', () {
    test('nothing waiting means no notice', () {
      expect(noticeFor(null), CatalogNotice.none);
      expect(noticeFor(status()), CatalogNotice.none);
    });

    test('a waiting release needs review until Not now is chosen, then only the badge stays', () {
      expect(noticeFor(status(waiting: waiting())), CatalogNotice.needsReview);
      expect(noticeFor(status(waiting: waiting(dismissed: true))), CatalogNotice.reviewLater);
    });

    test('the sentences say what would happen to the saved pulls', () {
      expect(affectedSentence(pulls: 0, accounts: 0), 'None of your saved pulls would change.');
      expect(affectedSentence(pulls: 1, accounts: 1), '1 saved pull in 1 account would be read differently.');
      expect(affectedSentence(pulls: 12, accounts: 2), '12 saved pulls in 2 accounts would be read differently.');
    });

    test('the last check is worded', () {
      expect(lastCheckLabel(null), 'Never checked');
      final now = DateTime.fromMillisecondsSinceEpoch(1800000000 * 1000);
      expect(lastCheckLabel(1800000000 - 3 * 3600, now), startsWith('Last checked'));
    });
  });

  group('affected pulls', () {
    test('only what differs is listed, in words', () {
      final rows = differencesOf(pull(
        bannerBefore: null,
        bannerAfter: 'Event A',
        resultBefore: PullOutcome.loss,
        resultAfter: null,
        shownAfter: const ShownItem(name: 'Diluc Ragnvindr', rarity: 4),
      ));
      expect(rows.map((d) => d.label), ['Banner', 'Result', 'Name', 'Rarity']);
      expect(rows[0].before, 'No banner');
      expect(rows[1].before, 'Lost the 50/50');
      expect(rows[1].after, 'Result not tracked');
      expect(rows[3].after, '4-star');
    });

    test('an unchanged pull has no differences', () {
      expect(differencesOf(pull()), isEmpty);
    });

    test('a changed pity number is shown', () {
      expect(differencesOf(pull(pityAfter: 11)).single.label, 'Pity');
    });

    test('the change log entries behind a pull come once each, in order', () {
      final entries = [change('x', ChangeSeverity.major), change('y', ChangeSeverity.minor)];
      final p = pull(reasons: const [
        ImpactReason(text: 'one', changeIds: ['y', 'x']),
        ImpactReason(text: 'two', changeIds: ['x', 'missing']),
      ]);
      expect(entriesBehind(p, entries).map((e) => e.id), ['y', 'x']);
    });

    test('accounts with nothing to review are left out and the most affected comes first', () {
      final review = CatalogReview(
        version: 8,
        currentVersion: 7,
        generatedAt: 'x',
        changes: const [],
        fingerprint: 'f',
        accounts: const [
          AccountImpact(game: GameId.genshin, uid: '1', nickname: '', pulls: 100, affected: 0, listed: 0),
          AccountImpact(game: GameId.genshin, uid: '2', nickname: 'Aether', pulls: 100, affected: 2, listed: 2),
          AccountImpact(game: GameId.starRail, uid: '3', nickname: '', pulls: 100, affected: 9, listed: 9),
        ],
        totalAffected: 11,
        notListed: 0,
        dismissed: false,
      );
      expect(accountsToReview(review).map((a) => a.uid), ['3', '2']);
      expect(accountTitle(review.accounts[1]), contains('Aether (2)'));
    });
  });
}
