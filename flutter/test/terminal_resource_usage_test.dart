import 'package:flutter_hbb/models/terminal_resource_usage.dart';
import 'package:flutter_hbb/models/terminal_model.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_test/flutter_test.dart';

class _FakeFFI implements FFI {
  @override
  String id = 'test-peer';
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  test('channel metrics update listeners and clear on transport loss', () {
    final model = TerminalModel(_FakeFFI(), 0, true);
    addTearDown(model.dispose);
    var updates = 0;
    model.addListener(() => updates++);
    model.handleTerminalResponse({
      'type': 'resources',
      'terminal_id': 0,
      'memory_total': '4096',
      'memory_used': '1024',
      'disk_total': '8192',
      'disk_used': '4096',
    });
    expect(model.resourceUsage?.memoryUsed, 1024);
    expect(updates, 1);
    model.handleTerminalResponse({
      'type': 'error',
      'terminal_id': 0,
      'message': 'Terminal transport disconnected',
    });
    expect(model.resourceUsage, isNull);
    expect(updates, 2);
  });
  test('shows percentage and used/total GiB for remote resources', () {
    const gib = 1024 * 1024 * 1024;
    final usage = TerminalResourceUsage.fromEvent({
      'memory_total': '${4 * gib}',
      'memory_used': 2 * gib,
      'disk_total': 100 * gib,
      'disk_used': '${25 * gib}',
    });
    expect(terminalResourceLabel(usage.memoryUsed, usage.memoryTotal),
        '50% · 2.0/4.0 GiB');
    expect(terminalResourceLabel(usage.diskUsed, usage.diskTotal),
        '25% · 25.0/100.0 GiB');
    expect(terminalResourceLabel(0, gib), '0% · 0.0/1.0 GiB');
  });

  test('missing and invalid metrics show unavailable rather than zero usage',
      () {
    final usage = TerminalResourceUsage.fromEvent({'disk_total': 'invalid'});
    expect(terminalResourceLabel(usage.memoryUsed, usage.memoryTotal), '—');
    expect(terminalResourceLabel(usage.diskUsed, usage.diskTotal), '—');
    expect(terminalResourceLabel(1, 0), '—');
    expect(terminalResourceLabel(-1, 10), '—');
    expect(terminalResourceLabel(11, 10), '—');
  });
}
