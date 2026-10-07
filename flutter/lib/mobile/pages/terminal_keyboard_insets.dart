import 'dart:async';
import 'dart:math';

import 'package:flutter/widgets.dart';

/// Keeps terminal geometry stable until the keyboard stops changing height.
class TerminalKeyboardInsets extends StatefulWidget {
  const TerminalKeyboardInsets({super.key, required this.child});

  final Widget child;

  @override
  State<TerminalKeyboardInsets> createState() => _TerminalKeyboardInsetsState();
}

class _TerminalKeyboardInsetsState extends State<TerminalKeyboardInsets> {
  Timer? _settleTimer;
  double? _bottom;
  double? _pendingBottom;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final bottom = MediaQuery.of(context).viewInsets.bottom;
    _bottom ??= bottom;
    if (bottom == _pendingBottom) return;
    _pendingBottom = bottom;
    _settleTimer?.cancel();
    if (bottom == _bottom) return;
    // IME animation frames update MediaQuery; resize once after a quiet interval.
    _settleTimer = Timer(const Duration(milliseconds: 120), () {
      setState(() => _bottom = bottom);
    });
  }

  @override
  void dispose() {
    _settleTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final media = MediaQuery.of(context);
    final bottom = _bottom!;
    return MediaQuery(
      data: media.copyWith(
        viewInsets: media.viewInsets.copyWith(bottom: bottom),
        padding: media.padding.copyWith(
          bottom: max(0.0, media.viewPadding.bottom - bottom),
        ),
      ),
      child: widget.child,
    );
  }
}
