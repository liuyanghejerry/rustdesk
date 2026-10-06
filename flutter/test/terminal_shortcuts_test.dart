import 'package:flutter/material.dart';
import 'package:flutter_hbb/mobile/pages/terminal_shortcuts.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('cursor group exposes all arrows at phone width and stays open',
      (tester) async {
    tester.view.physicalSize = const Size(348, 640);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final sent = <String>[];
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: Column(children: [
          const Spacer(),
          TerminalShortcuts(
            groupLabels: const ['Control', 'Cursor', 'Input/edit'],
            onKey: sent.add,
          ),
        ]),
      ),
    ));
    expect(find.text('←'), findsNothing);
    await tester.tap(find.text('Cursor'));
    await tester.pump();
    for (final label in ['←', '↑', '↓', '→', 'Home', 'End', 'PgUp', 'PgDn']) {
      expect(find.text(label).hitTestable(), findsOneWidget);
    }
    await tester.tap(find.text('←'));
    await tester.tap(find.text('↑'));
    await tester.tap(find.text('↓'));
    await tester.tap(find.text('→'));
    expect(sent, ['\x1b[D', '\x1b[A', '\x1b[B', '\x1b[C']);
    expect(find.text('Home'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('groups switch and collapse, with working control and input keys',
      (tester) async {
    final sent = <String>[];
    await tester.pumpWidget(MaterialApp(
      home: Scaffold(
        body: TerminalShortcuts(
          groupLabels: const ['Control', 'Cursor', 'Input/edit'],
          onKey: sent.add,
        ),
      ),
    ));
    await tester.tap(find.text('Control'));
    await tester.pump();
    for (final label in ['Ctrl+C', 'Ctrl+D', 'Ctrl+Z']) {
      await tester.tap(find.text(label));
    }
    expect(sent, ['\x03', '\x04', '\x1a']);
    await tester.tap(find.text('Input/edit'));
    await tester.pump();
    expect(find.text('Ctrl+C'), findsNothing);
    await tester.tap(find.text('Enter'));
    expect(sent.last, '\r');
    await tester.tap(find.text('Input/edit'));
    await tester.pump();
    expect(find.text('Enter'), findsNothing);
  });
}
