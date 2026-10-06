import 'package:flutter/material.dart';

class TerminalShortcuts extends StatefulWidget {
  const TerminalShortcuts({
    super.key,
    required this.groupLabels,
    required this.onKey,
  });

  final List<String> groupLabels;
  final ValueChanged<String> onKey;

  @override
  State<TerminalShortcuts> createState() => _TerminalShortcutsState();
}

class _TerminalShortcutsState extends State<TerminalShortcuts> {
  int? _expanded;

  static const _groups = [
    [
      ('Ctrl+C', '\x03'),
      ('Ctrl+D', '\x04'),
      ('Ctrl+Z', '\x1a'),
      ('Ctrl+L', '\x0c'),
      ('Ctrl+R', '\x12'),
      ('Ctrl+O', '\x0f'),
      ('Ctrl+T', '\x14'),
    ],
    [
      ('←', '\x1b[D'),
      ('↑', '\x1b[A'),
      ('↓', '\x1b[B'),
      ('→', '\x1b[C'),
      ('Home', '\x1b[H'),
      ('End', '\x1b[F'),
      ('PgUp', '\x1b[5~'),
      ('PgDn', '\x1b[6~'),
      ('Ctrl+A', '\x01'),
      ('Ctrl+E', '\x05'),
    ],
    [
      ('Enter', '\r'),
      ('Esc', '\x1b'),
      ('Tab', '\t'),
      ('Shift+Tab', '\x1b[Z'),
      ('Alt+Enter', '\x1b\r'),
      ('Ctrl+J', '\x0a'),
      ('Ctrl+U', '\x15'),
      ('Ctrl+K', '\x0b'),
      ('Ctrl+W', '\x17'),
    ],
  ];

  @override
  Widget build(BuildContext context) => ExcludeFocus(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (_expanded != null)
              Wrap(
                alignment: WrapAlignment.center,
                children: [
                  for (final key in _groups[_expanded!])
                    TextButton(
                      onPressed: () => widget.onKey(key.$2),
                      child: Text(key.$1),
                    ),
                ],
              ),
            Row(
              children: [
                for (var i = 0; i < _groups.length; i++)
                  Expanded(
                    child: TextButton(
                      onPressed: () =>
                          setState(() => _expanded = _expanded == i ? null : i),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(widget.groupLabels[i],
                              textAlign: TextAlign.center),
                          Icon(_expanded == i
                              ? Icons.expand_more
                              : Icons.expand_less),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
          ],
        ),
      );
}
