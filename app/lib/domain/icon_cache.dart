import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart' show immutable;

/// The pictures to try for one item, in order (for example a front-facing portrait, then the
/// side-facing one). Value equality, so it can key a provider.
@immutable
class IconRequest {
  const IconRequest(this.urls);

  final List<String> urls;

  @override
  bool operator ==(Object other) =>
      other is IconRequest && other.urls.length == urls.length && Iterable<int>.generate(urls.length).every((i) => other.urls[i] == urls[i]);

  @override
  int get hashCode => Object.hashAll(urls);
}

/// Keeps item pictures on disk, so they appear instantly, and work offline, after the first time.
///
/// Only PNG files up to [maxBytes] are kept, at most [maxConcurrent] downloads run at once, and a
/// picture that could not be fetched is not asked for again for [retryAfter]. Files are written
/// to a temporary name first, so an interrupted download never leaves a broken picture behind.
class IconCache {
  IconCache(
    this.directory, {
    HttpClient? client,
    this.maxConcurrent = 6,
    this.timeout = const Duration(seconds: 10),
    this.retryAfter = const Duration(minutes: 5),
  }) : _client = client ?? (HttpClient()..connectionTimeout = const Duration(seconds: 8));

  static const maxBytes = 2 * 1024 * 1024;

  /// The same folder name the Rust core uses for its data, under the platform's cache location.
  static Directory defaultDirectory() {
    final env = Platform.environment;
    final String base;
    if (Platform.isWindows) {
      base = env['LOCALAPPDATA'] ?? env['APPDATA'] ?? Directory.systemTemp.path;
    } else if (Platform.isMacOS) {
      base = '${env['HOME'] ?? Directory.systemTemp.path}/Library/Caches';
    } else {
      base = env['XDG_CACHE_HOME'] ?? '${env['HOME'] ?? Directory.systemTemp.path}/.cache';
    }
    final sep = Platform.pathSeparator;
    return Directory('$base${sep}app.wishtory.desktop${sep}icons');
  }

  final Directory directory;
  final int maxConcurrent;
  final Duration timeout;
  final Duration retryAfter;
  final HttpClient _client;

  final Map<String, Future<File?>> _inflight = {};
  final Map<String, DateTime> _failedAt = {};
  final List<Completer<void>> _waiting = [];
  int _running = 0;

  /// The first picture of [request] that is cached or can be downloaded, or null.
  Future<File?> getFirst(IconRequest request) async {
    for (final url in request.urls) {
      final file = await get(url);
      if (file != null) return file;
    }
    return null;
  }

  /// The cached file for [url], downloading it first when needed; null when it is not available.
  Future<File?> get(String url) {
    final failed = _failedAt[url];
    if (failed != null && DateTime.now().difference(failed) < retryAfter) return Future.value(null);
    return _inflight[url] ??= _load(url).whenComplete(() => _inflight.remove(url));
  }

  Future<File?> _load(String url) async {
    // Never throws: callers (and Riverpod) must see "not available" as null, not as an error.
    try {
      final target = _fileFor(url);
      if (await target.exists() && await target.length() > 0) return target;
      await _acquire();
      try {
        final file = await _download(url, target);
        if (file == null) _failedAt[url] = DateTime.now();
        return file;
      } finally {
        _release();
      }
    } catch (_) {
      _failedAt[url] = DateTime.now();
      return null;
    }
  }

  Future<File?> _download(String url, File target) async {
    final request = await _client.getUrl(Uri.parse(url)).timeout(timeout);
    final response = await request.close().timeout(timeout);
    if (response.statusCode != 200) {
      await response.drain<void>();
      return null;
    }
    final bytes = BytesBuilder(copy: false);
    await for (final chunk in response.timeout(timeout)) {
      bytes.add(chunk);
      if (bytes.length > maxBytes) return null;
    }
    final data = bytes.takeBytes();
    if (!_isPng(data)) return null;
    await directory.create(recursive: true);
    final temp = File('${target.path}.part');
    await temp.writeAsBytes(data, flush: true);
    await temp.rename(target.path);
    return target;
  }

  static bool _isPng(Uint8List d) => d.length > 8 && d[0] == 0x89 && d[1] == 0x50 && d[2] == 0x4E && d[3] == 0x47;

  File _fileFor(String url) {
    final uri = Uri.parse(url);
    final name = '${uri.host}${uri.path}'.replaceAll(RegExp(r'[^A-Za-z0-9._-]'), '_');
    return File('${directory.path}${Platform.pathSeparator}$name');
  }

  Future<void> _acquire() async {
    if (_running < maxConcurrent) {
      _running++;
      return;
    }
    final turn = Completer<void>();
    _waiting.add(turn);
    await turn.future; // the slot is handed over, so the running count stays as it is
  }

  void _release() {
    if (_waiting.isNotEmpty) {
      _waiting.removeAt(0).complete();
    } else {
      _running--;
    }
  }

  /// Total size of the saved pictures in bytes.
  Future<int> sizeBytes() async {
    if (!await directory.exists()) return 0;
    var total = 0;
    await for (final entity in directory.list()) {
      if (entity is File) {
        try {
          total += await entity.length();
        } on FileSystemException {
          // A file removed while counting.
        }
      }
    }
    return total;
  }

  /// Deletes every saved picture. They are downloaded again when needed.
  Future<int> clear() async {
    _failedAt.clear();
    if (!await directory.exists()) return 0;
    var removed = 0;
    await for (final entity in directory.list()) {
      if (entity is File) {
        try {
          await entity.delete();
          removed++;
        } on FileSystemException {
          // In use or already gone: leave it.
        }
      }
    }
    return removed;
  }
}
