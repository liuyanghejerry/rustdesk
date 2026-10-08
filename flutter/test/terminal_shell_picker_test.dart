import 'package:flutter/material.dart';
import 'package:flutter_hbb/mobile/pages/terminal_shell_picker.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('chooses retained shell by identity independently of tabs',
      (tester) async {
    final choices = <String?>[];
    await tester.pumpWidget(MaterialApp(
        home: Builder(
            builder: (context) => Scaffold(
                  body: TextButton(
                      onPressed: () async {
                        choices.add(await showDialog<String>(
                            context: context,
                            builder: (_) => const TerminalShellPicker(
                                    sessions: [
                                      {
                                        'resume_token': 'shell-a',
                                        'pid': 123,
                                        'working_directory': '/work/a'
                                      },
                                      {
                                        'resume_token': 'shell-b',
                                        'pid': 456,
                                        'working_directory': '/work/b'
                                      }
                                    ],
                                    title: 'Retained shells',
                                    switchNotice:
                                        'Keep current shell when switching',
                                    emptyLabel: 'None',
                                    cancelLabel: 'Cancel',
                                    newLabel: 'New shell')));
                      },
                      child: const Text('Show shells')),
                ))));
    for (final choice in ['PID 456', 'New shell', 'Cancel']) {
      await tester.tap(find.text('Show shells'));
      await tester.pumpAndSettle();
      expect(find.text('/work/a'), findsOneWidget);
      expect(find.text('Keep current shell when switching'), findsOneWidget);
      expect(find.text('shell-a'), findsNothing);
      await tester.tap(find.text(choice));
      await tester.pumpAndSettle();
    }
    expect(choices, ['shell-b', '', null]);
  });

  testWidgets('empty list explains state and allows a new shell',
      (tester) async {
    await tester.pumpWidget(const MaterialApp(
        home: TerminalShellPicker(
            sessions: [],
            title: 'Retained shells',
            emptyLabel: 'No retained shells',
            cancelLabel: 'Cancel',
            newLabel: 'New shell')));
    expect(find.text('No retained shells'), findsOneWidget);
    expect(find.text('New shell'), findsOneWidget);
    expect(find.byType(ListTile), findsNothing);
  });
}
