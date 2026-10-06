import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/terminal_model.dart';

const terminalResumeOption = 'session-terminal-resume-token';

Future<bool?> chooseTerminalExit(BuildContext context) => showDialog<bool>(
      context: context,
      builder: (_) => TerminalExitDialog(
        title: translate('Keep the shell running?'),
        description: translate(
            'Keep the shell to resume later, or destroy it and stop its jobs.'),
        cancelLabel: translate('Cancel'),
        keepLabel: translate('Keep and exit'),
        destroyLabel: translate('Destroy and exit'),
      ),
    );

class TerminalExitDialog extends StatelessWidget {
  const TerminalExitDialog(
      {super.key,
      required this.title,
      required this.description,
      required this.cancelLabel,
      required this.keepLabel,
      required this.destroyLabel});
  final String title, description, cancelLabel, keepLabel, destroyLabel;

  @override
  Widget build(BuildContext context) => AlertDialog(
        title: Text(title),
        content: Text(description),
        actions: [
          TextButton(
              onPressed: () => Navigator.pop(context),
              child: Text(cancelLabel)),
          TextButton(
              onPressed: () => Navigator.pop(context, true),
              child: Text(keepLabel)),
          TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: Text(destroyLabel)),
        ],
      );
}

bool hasRetainedTerminal(FFI ffi) => bind
    .mainGetPeerOptionSync(id: ffi.id, key: terminalResumeOption)
    .isNotEmpty;

Future<bool> confirmRetainedTerminalExit(FFI ffi) async {
  final token =
      bind.mainGetPeerOptionSync(id: ffi.id, key: terminalResumeOption);
  if (token.isEmpty) return true;
  final context = globalKey.currentContext;
  if (context == null) return false;
  final keep = await chooseTerminalExit(context);
  if (keep == null) return false;
  if (keep) return true;
  final existing = ffi.terminalModels[0];
  final model = existing ?? TerminalModel(ffi, 0, true);
  if (existing == null) ffi.registerTerminalModel(0, model);
  try {
    await model.closeTerminal(keepShell: keep);
  } catch (error) {
    showToast('${translate('Failed')}: $error');
    return false;
  } finally {
    if (existing == null) {
      ffi.unregisterTerminalModel(0);
      model.dispose();
    }
  }
  return true;
}
