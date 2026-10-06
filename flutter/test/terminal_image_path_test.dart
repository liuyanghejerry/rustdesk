import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_hbb/models/terminal_image_path.dart';
import 'package:xterm/xterm.dart';

void main() {
  test('image paths are clickable, including quoted spaces and Chinese', () {
    const text = 'saved "/tmp/中文 image.PNG" and ./out.jpg';
    expect(imagePathAt(text, text.indexOf('中文')), '/tmp/中文 image.PNG');
    expect(imagePathAt(text, text.indexOf('out')), './out.jpg');
    expect(imagePathAt(text, 0), isNull);
    expect(imagePathAt('https://example.com/image.png', 20), isNull);
    expect(imagePathAt('file:///tmp/image.png', 14), '/tmp/image.png');
    expect(imagePathAt('file://other-host/tmp/image.png', 25), isNull);
  });
  test('wrapped paths remain clickable at terminal cell coordinates', () {
    final terminal = Terminal();
    terminal.resize(12, 10);
    terminal.write('/tmp/long-image.png\r\n');
    expect(terminalImagePathAt(terminal, const CellOffset(3, 1)),
        '/tmp/long-image.png');
    expect(terminalImagePathAt(terminal, const CellOffset(0, 2)), isNull);
  });
}
