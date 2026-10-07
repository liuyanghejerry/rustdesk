import 'package:flutter/material.dart';
import 'package:flutter_hbb/mobile/pages/terminal_keyboard_insets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:xterm/xterm.dart';

void main() {
  testWidgets(
      'IME animation resizes the terminal once after opening and closing',
      (tester) async {
    final sizes = <Size>[];
    final terminal = Terminal(onResize: (w, h, pw, ph) {
      sizes.add(Size(w.toDouble(), h.toDouble()));
    });
    final child = TerminalKeyboardInsets(
      child: Scaffold(body: SafeArea(child: TerminalView(terminal))),
    );
    Future<void> inset(double bottom) => tester.pumpWidget(MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(
              size: const Size(800, 600),
              viewInsets: EdgeInsets.only(bottom: bottom),
              viewPadding: const EdgeInsets.only(bottom: 24),
              padding: EdgeInsets.only(bottom: bottom == 0 ? 24 : 0),
            ),
            child: child,
          ),
        ));
    await inset(0);
    await tester.pump();
    final initial = sizes.last;
    sizes.clear();
    for (final height in [40.0, 100.0, 180.0, 240.0]) {
      await inset(height);
      await tester.pump(const Duration(milliseconds: 16));
      expect(sizes, isEmpty);
    }
    await tester.pump(const Duration(milliseconds: 120));
    expect(sizes, hasLength(1));
    expect(sizes.single.height, lessThan(initial.height));
    sizes.clear();
    for (final height in [180.0, 100.0, 40.0, 0.0]) {
      await inset(height);
      await tester.pump(const Duration(milliseconds: 16));
      expect(sizes, isEmpty);
    }
    await tester.pump(const Duration(milliseconds: 120));
    expect(sizes, [initial]);
    expect(tester.takeException(), isNull);
  });

  testWidgets('leaving during keyboard animation cancels the pending resize',
      (tester) async {
    const child = TerminalKeyboardInsets(child: SizedBox());
    Future<void> inset(double bottom) => tester.pumpWidget(MediaQuery(
          data: MediaQueryData(viewInsets: EdgeInsets.only(bottom: bottom)),
          child: child,
        ));
    await inset(0);
    await inset(200);
    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(milliseconds: 150));
    expect(tester.takeException(), isNull);
  });
}
