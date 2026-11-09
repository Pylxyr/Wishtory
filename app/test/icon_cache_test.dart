import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:wishtory/domain/icon_cache.dart';

final _png = Uint8List.fromList([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4]);

void main() {
  late HttpServer server;
  late Directory dir;
  late IconCache cache;
  late Map<String, int> hits;

  String url(String path) => 'http://127.0.0.1:${server.port}$path';

  setUp(() async {
    hits = {};
    dir = await Directory.systemTemp.createTemp('wishtory_icons_');
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) async {
      final path = request.uri.path;
      hits[path] = (hits[path] ?? 0) + 1;
      switch (path) {
        case '/ok.png':
          request.response.add(_png);
        case '/page.png':
          request.response.write('<html>this is not a picture</html>');
        default:
          request.response.statusCode = HttpStatus.notFound;
      }
      await request.response.close();
    });
    cache = IconCache(dir);
  });

  tearDown(() async {
    await server.close(force: true);
    if (await dir.exists()) await dir.delete(recursive: true);
  });

  test('downloads a picture once and then serves it from disk', () async {
    final first = await cache.get(url('/ok.png'));
    expect(first, isNotNull);
    expect(await first!.readAsBytes(), _png);

    final second = await cache.get(url('/ok.png'));
    expect(second!.path, first.path);
    expect(hits['/ok.png'], 1);
  });

  test('a missing picture gives null and is not asked for again straight away', () async {
    expect(await cache.get(url('/missing.png')), isNull);
    expect(await cache.get(url('/missing.png')), isNull);
    expect(hits['/missing.png'], 1);
  });

  test('something that is not a PNG is never saved', () async {
    expect(await cache.get(url('/page.png')), isNull);
    expect(await dir.list().toList(), isEmpty);
  });

  test('getFirst falls back to the next address', () async {
    final file = await cache.getFirst(IconRequest([url('/missing.png'), url('/ok.png')]));
    expect(file, isNotNull);
    expect(await file!.readAsBytes(), _png);
  });

  test('concurrent requests for one picture share a single download', () async {
    final results = await Future.wait([for (var i = 0; i < 5; i++) cache.get(url('/ok.png'))]);
    expect(results.every((f) => f != null), isTrue);
    expect(hits['/ok.png'], 1);
  });

  test('size and clear', () async {
    await cache.get(url('/ok.png'));
    expect(await cache.sizeBytes(), _png.length);
    expect(await cache.clear(), 1);
    expect(await cache.sizeBytes(), 0);
    // After clearing, the picture is downloaded again when needed.
    expect(await cache.get(url('/ok.png')), isNotNull);
    expect(hits['/ok.png'], 2);
  });

  test('an unreachable host gives null instead of throwing', () async {
    final offline = IconCache(dir, timeout: const Duration(seconds: 2));
    // Build the address first: HttpServer.port throws once the server is closed.
    final gone = url('/ok.png');
    await server.close(force: true);
    expect(await offline.get(gone), isNull);
  });

  test('requests with the same addresses are equal', () {
    expect(const IconRequest(['a', 'b']), const IconRequest(['a', 'b']));
    expect(const IconRequest(['a', 'b']).hashCode, const IconRequest(['a', 'b']).hashCode);
    expect(const IconRequest(['a']), isNot(const IconRequest(['a', 'b'])));
  });
}
