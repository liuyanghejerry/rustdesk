class TerminalResourceUsage {
  TerminalResourceUsage.fromEvent(Map<String, dynamic> event)
      : memoryTotal = int.tryParse('${event['memory_total']}'),
        memoryUsed = int.tryParse('${event['memory_used']}'),
        diskTotal = int.tryParse('${event['disk_total']}'),
        diskUsed = int.tryParse('${event['disk_used']}'),
        receivedAt = DateTime.now();

  final int? memoryTotal;
  final int? memoryUsed;
  final int? diskTotal;
  final int? diskUsed;
  final DateTime receivedAt;
}

String terminalResourceLabel(int? used, int? total) {
  if (used == null || total == null || total <= 0 || used < 0 || used > total) {
    return '—';
  }
  const gib = 1024 * 1024 * 1024;
  return '${(used / total * 100).round()}% · '
      '${(used / gib).toStringAsFixed(1)}/${(total / gib).toStringAsFixed(1)} GiB';
}
