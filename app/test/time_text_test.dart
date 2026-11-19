import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/domain/time_text.dart';

void main() {
  test('dayOf and monthOf cut a pull time and never throw on a short one', () {
    expect(dayOf('2026-11-05 18:04:09'), '2026-11-05');
    expect(monthOf('2026-11-05 18:04:09'), '2026-11');
    expect(dayOf('2026-11-05'), '2026-11-05');
    expect(dayOf('x'), 'x');
    expect(dayOf(''), '');
    expect(monthOf('2026-1'), isNull);
    expect(monthOf(''), isNull);
  });
}
