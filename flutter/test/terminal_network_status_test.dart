import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_test/flutter_test.dart';

class _FakeFFI implements FFI {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  test('traffic updates do not make an old latency sample look fresh', () {
    final ffi = _FakeFFI();
    final model = QualityMonitorModel(WeakReference(ffi));
    model.updateQualityStatus({'delay': '25'});
    final received = model.data.delayUpdatedAt;
    expect(received, isNotNull);
    model.updateQualityStatus({'speed': '4.00kB/s', 'delay': ''});
    expect(model.data.delay, '25');
    expect(model.data.delayUpdatedAt, received);
  });

  test('a repeated latency measurement refreshes its age', () {
    final ffi = _FakeFFI();
    final model = QualityMonitorModel(WeakReference(ffi));
    model.updateQualityStatus({'delay': '25'});
    model.data.delayUpdatedAt = DateTime(2000);
    model.updateQualityStatus({'delay': '25'});
    expect(model.data.delayUpdatedAt!.isAfter(DateTime(2000)), isTrue);
  });
}
