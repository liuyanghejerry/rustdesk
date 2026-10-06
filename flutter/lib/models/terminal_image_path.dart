import 'package:xterm/xterm.dart';

final _imagePath = RegExp(
  r'''"([^"\r\n]+\.(?:png|jpe?g|gif|webp|bmp))"|'([^'\r\n]+\.(?:png|jpe?g|gif|webp|bmp))'|([^\s"'<>()[\]{}]+\.(?:png|jpe?g|gif|webp|bmp))''',
  caseSensitive: false,
);

String? imagePathAt(String text, int position) {
  for (final match in _imagePath.allMatches(text)) {
    if (position < match.start || position >= match.end) continue;
    final path = match.group(1) ?? match.group(2) ?? match.group(3)!;
    if (path.startsWith('file://')) {
      try {
        return Uri.tryParse(path)?.toFilePath();
      } on UnsupportedError {
        return null;
      } on FormatException {
        return null;
      }
    }
    if (path.contains('://')) return null;
    return path;
  }
  return null;
}

String? terminalImagePathAt(Terminal terminal, CellOffset offset) {
  final lines = terminal.buffer.lines;
  if (offset.y < 0 || offset.y >= lines.length) return null;
  var first = offset.y;
  while (first > 0 && lines[first].isWrapped) {
    first--;
  }
  final text = StringBuffer();
  var position = 0;
  for (var y = first; y < lines.length; y++) {
    if (y > first && !lines[y].isWrapped) break;
    if (y == offset.y) {
      position = text.length + lines[y].getText(0, offset.x).length;
    }
    text.write(lines[y].getText());
    if (text.length > 16384) return null;
  }
  return imagePathAt(text.toString(), position);
}
