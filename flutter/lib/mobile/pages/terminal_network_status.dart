import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/model.dart';

class TerminalNetworkStatus extends StatefulWidget {
  const TerminalNetworkStatus({super.key, required this.ffi});

  final FFI ffi;

  @override
  State<TerminalNetworkStatus> createState() => _TerminalNetworkStatusState();
}

class _TerminalNetworkStatusState extends State<TerminalNetworkStatus> {
  final _openedAt = DateTime.now();
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) => setState(() {}));
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
        animation: Listenable.merge(
            [widget.ffi.qualityMonitorModel, widget.ffi.ffiModel]),
        builder: (context, _) {
          final data = widget.ffi.qualityMonitorModel.data;
          final stale =
              DateTime.now().difference(data.delayUpdatedAt ?? _openedAt) >=
                  const Duration(seconds: 6);
          final delay = int.tryParse(data.delay ?? '');
          final color = stale
              ? Colors.redAccent
              : delay == null
                  ? Colors.grey
                  : delay >= 500
                      ? Colors.redAccent
                      : delay >= 150
                          ? Colors.amber
                          : Colors.greenAccent;
          final latency = stale
              ? translate('No latency response')
              : delay == null
                  ? translate('Waiting')
                  : '${delay}ms';
          final connection = widget.ffi.ffiModel;
          return Container(
            width: double.infinity,
            color: const Color(0xff202020),
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            child: Wrap(
              spacing: 12,
              runSpacing: 4,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                Tooltip(
                  message:
                      connection.secure == null || connection.direct == null
                          ? translate('Waiting')
                          : getConnectionText(
                              connection.secure!,
                              connection.direct!,
                              connection.cachedPeerData.streamType),
                  child: Icon(
                    connection.direct == true
                        ? Icons.link
                        : Icons.cloud_outlined,
                    size: 16,
                    color: color,
                  ),
                ),
                Text('${translate('Round-trip latency')}: $latency',
                    style: TextStyle(fontSize: 12, color: color)),
                Text(
                    '${translate('Receive rate')}: ${stale ? '-' : data.speed ?? '-'}',
                    style:
                        const TextStyle(fontSize: 12, color: Colors.white70)),
                Text(
                    '${translate('Desktop video paused')} · ${translate('Desktop FPS')}: ${stale ? '-' : data.fps ?? '-'}',
                    style:
                        const TextStyle(fontSize: 12, color: Colors.white70)),
              ],
            ),
          );
        },
      );
}
