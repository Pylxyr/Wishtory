import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/ui/pull_tiles.dart';

void main() {
  test('columns are as many as fit, never fewer than one', () {
    // 3 columns of 190 with 6 gaps need 3 * 190 + 2 * 6 = 582.
    expect(tileColumns(582, 190, 6), 3);
    expect(tileColumns(581, 190, 6), 2);
    expect(tileColumns(100, 190, 6), 1);
    expect(tileColumns(0, 190, 6), 1);
    expect(tileColumns(double.infinity, 190, 6), 1);
  });

  test('a wide window gets many dense columns', () {
    expect(tileColumns(1600, 150, 6), 10);
    expect(tileColumns(1600, 52, 6), 27);
  });
}
