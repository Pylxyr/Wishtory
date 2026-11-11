import '../../bridge/wishtory_api.dart';
import '../../domain/game_info.dart';

/// The order the change log is shown in.
const severityOrder = [ChangeSeverity.major, ChangeSeverity.minor, ChangeSeverity.info];

String severityLabel(ChangeSeverity s) => switch (s) {
      ChangeSeverity.major => 'Major',
      ChangeSeverity.minor => 'Minor',
      ChangeSeverity.info => 'Info',
    };

/// What a severity means for the person, in plain words.
String severityMeaning(ChangeSeverity s) => switch (s) {
      ChangeSeverity.major => 'Can change how saved pulls are read.',
      ChangeSeverity.minor => 'A small correction.',
      ChangeSeverity.info => 'Something new. Nothing that exists changes.',
    };

/// The change log grouped by severity, in [severityOrder]. Every severity is present (possibly empty) and entries
/// keep the order they came in.
Map<ChangeSeverity, List<ChangeEntry>> groupChanges(List<ChangeEntry> changes) => {
      for (final s in severityOrder) s: [for (final c in changes) if (c.severity == s) c],
    };

/// "2 major, 1 minor, 5 info", skipping zeros.
String countsLabel({required int major, required int minor, required int info}) {
  final parts = [
    if (major > 0) '$major major',
    if (minor > 0) '$minor minor',
    if (info > 0) '$info info',
  ];
  return parts.isEmpty ? 'No changes listed' : parts.join(', ');
}

String pullsWord(int n) => n == 1 ? '1 saved pull' : '$n saved pulls';

/// What a waiting release would do to the saved pulls, as one sentence.
String affectedSentence({required int pulls, required int accounts}) {
  if (pulls == 0) return 'None of your saved pulls would change.';
  final where = accounts == 1 ? '1 account' : '$accounts accounts';
  return '${pullsWord(pulls)} in $where would be read differently.';
}

enum CatalogNotice { none, needsReview, reviewLater }

/// Whether a release needs the person: shown at start-up and as a Settings badge. After "Not now" the badge stays
/// but the start-up notice does not come back for that release.
CatalogNotice noticeFor(CatalogStatus? status) {
  final waiting = status?.waiting;
  if (waiting == null) return CatalogNotice.none;
  return waiting.dismissed ? CatalogNotice.reviewLater : CatalogNotice.needsReview;
}

/// "Last checked 3 hours ago" / "Never checked".
String lastCheckLabel(int? unixSeconds, [DateTime? now]) {
  if (unixSeconds == null || unixSeconds <= 0) return 'Never checked';
  return 'Last checked ${ago(DateTime.fromMillisecondsSinceEpoch(unixSeconds * 1000), now)}';
}

/// The result of a 5★ in words (never colour alone).
String outcomeWords(PullOutcome? outcome) => switch (outcome) {
      PullOutcome.win => 'Won the 50/50',
      PullOutcome.loss => 'Lost the 50/50',
      PullOutcome.guaranteed => 'Guaranteed',
      null => 'Result not tracked',
    };

/// "Character Event" or "No banner" for one side of a before/after pair.
String bannerWords(String? banner) => banner ?? 'No banner';

/// One row of a before/after comparison, only when the two sides differ.
class Difference {
  const Difference(this.label, this.before, this.after);
  final String label;
  final String before;
  final String after;
}

/// What changes for one affected pull, as label / before / after rows.
List<Difference> differencesOf(AffectedPull p) => [
      if (p.bannerBefore != p.bannerAfter) Difference('Banner', bannerWords(p.bannerBefore), bannerWords(p.bannerAfter)),
      if (p.resultBefore != p.resultAfter) Difference('Result', outcomeWords(p.resultBefore), outcomeWords(p.resultAfter)),
      if (p.pityBefore != p.pityAfter) Difference('Pity', '${p.pityBefore ?? 'none'}', '${p.pityAfter ?? 'none'}'),
      if (p.shownBefore.name != p.shownAfter.name) Difference('Name', p.shownBefore.name, p.shownAfter.name),
      if (p.shownBefore.rarity != p.shownAfter.rarity) Difference('Rarity', '${p.shownBefore.rarity}-star', '${p.shownAfter.rarity}-star'),
    ];

/// The change log entries behind a pull's reasons, in the order they first appear, without repeats.
List<ChangeEntry> entriesBehind(AffectedPull p, List<ChangeEntry> changes) {
  final byId = {for (final c in changes) c.id: c};
  final seen = <String>{};
  return [
    for (final r in p.reasons)
      for (final id in r.changeIds)
        if (seen.add(id) && byId[id] != null) byId[id]!,
  ];
}

/// The accounts with something to review, most affected first.
List<AccountImpact> accountsToReview(CatalogReview review) =>
    [for (final a in review.accounts) if (a.affected > 0) a]..sort((a, b) => b.affected.compareTo(a.affected));

String accountTitle(AccountImpact a) => '${GameInfo.of(a.game).name} · ${a.nickname.isEmpty ? a.uid : '${a.nickname} (${a.uid})'}';
