import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../bridge/wishtory_api.dart';
import 'tokens.dart';

/// A quiet decoration behind the page, drawn in the accent colour at a few percent, so each game feels like its own
/// place without getting in the way of the numbers:
///   Genshin Impact     a warm glow in the corner and a scatter of four-pointed sparkles, like the stars of a wish.
///   Honkai: Star Rail  faint orbit rings around the corner and a field of tiny stars, like the view from the train.
///   Zenless Zone Zero  slanted stripes, a fading halftone and CRT scanlines, like a tape in the video store.
/// It fills the space it is given (put it in a Stack, under the page), takes no clicks and never moves.
class GameBackdrop extends StatelessWidget {
  const GameBackdrop({super.key, required this.game});

  final GameId game;

  @override
  Widget build(BuildContext context) {
    final c = context.wc;
    return IgnorePointer(
      child: RepaintBoundary(
        child: CustomPaint(painter: _BackdropPainter(game: game, accent: c.accent, gold: c.gold, ink: c.text), size: Size.infinite),
      ),
    );
  }
}

/// A small deterministic stream of numbers in [0, 1), so the scatter is the same on every frame and every start.
class _Seq {
  _Seq(this._state);

  int _state;

  double next() {
    _state = (_state * 1103515245 + 12345) & 0x7fffffff;
    return _state / 0x7fffffff;
  }
}

class _BackdropPainter extends CustomPainter {
  const _BackdropPainter({required this.game, required this.accent, required this.gold, required this.ink});

  final GameId game;

  /// The theme's accent and gold, and its text colour (light on a dark theme, dark on a light one).
  final Color accent, gold, ink;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty) return;
    switch (game) {
      case GameId.genshin:
        _teyvat(canvas, size);
      case GameId.starRail:
        _astral(canvas, size);
      case GameId.zenless:
        _eridu(canvas, size);
    }
  }

  // ---------------------------------------------------------------- Genshin Impact

  void _teyvat(Canvas canvas, Size size) {
    // A warm glow from the top right corner.
    final centre = Offset(size.width * 0.92, -size.height * 0.05);
    final radius = math.max(size.width, size.height) * 0.7;
    final glow = Paint()
      ..shader = RadialGradient(colors: [gold.withValues(alpha: 0.10), accent.withValues(alpha: 0.04), accent.withValues(alpha: 0)], stops: const [0, 0.45, 1])
          .createShader(Rect.fromCircle(center: centre, radius: radius));
    canvas.drawRect(Offset.zero & size, glow);

    // Sparkles: more of them toward the glow, a few bigger ones among many small.
    final rnd = _Seq(7);
    final paint = Paint()..style = PaintingStyle.fill;
    for (var i = 0; i < 26; i++) {
      final x = size.width * (0.25 + 0.75 * rnd.next());
      final y = size.height * rnd.next() * (0.35 + 0.65 * rnd.next());
      final big = rnd.next() > 0.8;
      final r = big ? 9.0 + 5 * rnd.next() : 3.0 + 3 * rnd.next();
      paint.color = (rnd.next() > 0.5 ? gold : accent).withValues(alpha: big ? 0.16 : 0.10);
      _sparkle(canvas, Offset(x, y), r, paint);
    }
  }

  /// A four-pointed star: four curves pinched toward the middle.
  void _sparkle(Canvas canvas, Offset c, double r, Paint paint) {
    final k = r * 0.18;
    final path = Path()
      ..moveTo(c.dx, c.dy - r)
      ..quadraticBezierTo(c.dx + k, c.dy - k, c.dx + r, c.dy)
      ..quadraticBezierTo(c.dx + k, c.dy + k, c.dx, c.dy + r)
      ..quadraticBezierTo(c.dx - k, c.dy + k, c.dx - r, c.dy)
      ..quadraticBezierTo(c.dx - k, c.dy - k, c.dx, c.dy - r)
      ..close();
    canvas.drawPath(path, paint);
  }

  // ---------------------------------------------------------------- Honkai: Star Rail

  void _astral(Canvas canvas, Size size) {
    // Orbit rings around a point off the right edge, as if a planet were just out of view.
    final centre = Offset(size.width * 1.02, size.height * 0.18);
    final ring = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2;
    for (var i = 0; i < 4; i++) {
      final r = size.height * (0.38 + 0.2 * i);
      ring.color = accent.withValues(alpha: 0.075 - 0.012 * i);
      canvas.drawCircle(centre, r, ring);
    }
    // One small body on the second ring.
    canvas.drawCircle(centre + Offset(-size.height * 0.58 * math.cos(0.5), size.height * 0.58 * math.sin(0.5)), 3.5, Paint()..color = accent.withValues(alpha: 0.28));

    // A field of tiny stars.
    final rnd = _Seq(41);
    final dot = Paint();
    for (var i = 0; i < 90; i++) {
      final x = size.width * rnd.next();
      final y = size.height * rnd.next();
      final bright = rnd.next() > 0.85;
      dot.color = (bright ? accent : ink).withValues(alpha: bright ? 0.22 : 0.07 + 0.05 * rnd.next());
      canvas.drawCircle(Offset(x, y), bright ? 1.6 : 0.9, dot);
    }
  }

  // ---------------------------------------------------------------- Zenless Zone Zero

  void _eridu(Canvas canvas, Size size) {
    // CRT scanlines over the whole page.
    final line = Paint()
      ..color = ink.withValues(alpha: 0.022)
      ..strokeWidth = 1;
    for (var y = 0.0; y < size.height; y += 4) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), line);
    }

    // A band of slanted stripes in the top right corner, clipped to a triangle.
    canvas.save();
    final corner = math.min(size.width, size.height) * 0.55;
    canvas.clipPath(Path()
      ..moveTo(size.width, 0)
      ..lineTo(size.width - corner, 0)
      ..lineTo(size.width, corner)
      ..close());
    final stripe = Paint()
      ..color = accent.withValues(alpha: 0.07)
      ..strokeWidth = 7;
    for (var d = -corner; d < corner * 2; d += 22) {
      canvas.drawLine(Offset(size.width - corner + d, 0), Offset(size.width - corner + d + corner, corner), stripe);
    }
    canvas.restore();

    // A halftone that fades out from the bottom left corner.
    final dot = Paint();
    const step = 14.0;
    final reach = math.min(size.width, size.height) * 0.5;
    for (var x = 0.0; x < reach; x += step) {
      for (var y = size.height; y > size.height - reach; y -= step) {
        final d = math.sqrt(x * x + (size.height - y) * (size.height - y)) / reach;
        if (d >= 1) continue;
        dot.color = accent.withValues(alpha: 0.10 * (1 - d));
        canvas.drawCircle(Offset(x + 8, y - 8), 3.2 * (1 - d) + 0.5, dot);
      }
    }
  }

  @override
  bool shouldRepaint(_BackdropPainter old) => old.game != game || old.accent != accent || old.gold != gold || old.ink != ink;
}
