import 'package:flutter/material.dart';
import 'package:flutter_hbb/mobile/pages/terminal_exit_dialog.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets(
      'exit explicitly distinguishes cancellation, retention and destruction',
      (tester) async {
    final choices = <bool?>[];
    await tester.pumpWidget(MaterialApp(
        home: Builder(
            builder: (context) => Scaffold(
                  body: TextButton(
                      onPressed: () async {
                        choices.add(await showDialog<bool>(
                            context: context,
                            builder: (_) => const TerminalExitDialog(
                                  title: 'Keep shell?',
                                  description:
                                      'Keep to resume later, destroy to stop jobs.',
                                  cancelLabel: 'Cancel',
                                  keepLabel: 'Keep and exit',
                                  destroyLabel: 'Destroy and exit',
                                )));
                      },
                      child: const Text('Exit')),
                ))));
    for (final label in ['Cancel', 'Keep and exit', 'Destroy and exit']) {
      await tester.tap(find.text('Exit'));
      await tester.pumpAndSettle();
      await tester.tap(find.text(label));
      await tester.pumpAndSettle();
      expect(find.text('Keep shell?'), findsNothing);
    }
    expect(choices, [null, true, false]);
  });
}
