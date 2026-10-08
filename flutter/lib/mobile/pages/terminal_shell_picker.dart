import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';

Future<String?> chooseRetainedShell(
        BuildContext context, List<Map<String, dynamic>> sessions,
        {required bool keepsCurrent}) =>
    showDialog<String>(
        context: context,
        builder: (_) => TerminalShellPicker(
              sessions: sessions,
              title: translate('Retained shells'),
              switchNotice: keepsCurrent
                  ? translate('The current shell will be kept when switching.')
                  : null,
              emptyLabel: translate('No retained shells'),
              cancelLabel: translate('Cancel'),
              newLabel: translate('New shell'),
            ));

class TerminalShellPicker extends StatelessWidget {
  const TerminalShellPicker(
      {super.key,
      required this.sessions,
      required this.title,
      this.switchNotice,
      required this.emptyLabel,
      required this.cancelLabel,
      required this.newLabel});
  final List<Map<String, dynamic>> sessions;
  final String title;
  final String? switchNotice;
  final String emptyLabel;
  final String cancelLabel;
  final String newLabel;

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: Text(title),
        content: SizedBox(
            width: 420,
            child: Column(mainAxisSize: MainAxisSize.min, children: [
              if (switchNotice != null)
                Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Text(switchNotice!)),
              if (sessions.isEmpty) Text(emptyLabel),
              Flexible(
                  child: ListView(shrinkWrap: true, children: [
                for (final session in sessions)
                  ListTile(
                    leading: const Icon(Icons.terminal),
                    title: Text('PID ${session['pid']}'),
                    subtitle: Text('${session['working_directory']}'.isEmpty ? '—' : '${session['working_directory']}',
                        maxLines: 2, overflow: TextOverflow.ellipsis),
                    trailing: const Icon(Icons.play_arrow),
                    onTap: () => Navigator.pop(
                        context, session['resume_token'] as String),
                  ),
              ])),
            ])),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(context),
              child: Text(cancelLabel)),
          TextButton(
              onPressed: () => Navigator.pop(context, ''),
              child: Text(newLabel)),
        ],
      );
}
