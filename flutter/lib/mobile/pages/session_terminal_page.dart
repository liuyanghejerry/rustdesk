import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/models/platform_model.dart';
import 'package:flutter_hbb/models/model.dart';
import 'package:flutter_hbb/models/terminal_model.dart';
import 'package:flutter_hbb/models/terminal_image_path.dart';
import 'package:flutter_hbb/mobile/pages/terminal_image_preview.dart';
import 'package:xterm/xterm.dart';

/// Uses the remote page's authenticated session, without opening another connection.
class SessionTerminalPage extends StatefulWidget {
  const SessionTerminalPage({super.key, required this.ffi});
  final FFI ffi;

  @override
  State<SessionTerminalPage> createState() => _SessionTerminalPageState();
}

class _SessionTerminalPageState extends State<SessionTerminalPage> {
  late final TerminalModel _model;
  final _viewKey = GlobalKey<TerminalViewState>();
  final _focusNode = FocusNode();
  bool _closing = false;
  int _imageId = 0;
  Completer<Uint8List>? _imageResult;
  BytesBuilder? _imageBytes;
  PointerDownEvent? _imageTap;

  @override
  void initState() {
    super.initState();
    _model = TerminalModel(widget.ffi, 0, true);
    widget.ffi.registerTerminalModel(0, _model);
    _model.onClosed = _close;
    _model.onImageResponse = _imageResponse;
    widget.ffi.ffiModel.addListener(_permissionChanged);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) unawaited(_model.openTerminal());
    });
  }

  void _permissionChanged() {
    if (widget.ffi.ffiModel.permissions['terminal'] == false) _close();
  }

  void _close() {
    if (mounted && !_closing) {
      _closing = true;
      final route = ModalRoute.of(context);
      Navigator.of(context).popUntil((r) => r == route);
      Navigator.of(context).pop();
    }
  }

  Future<void> _openKeyboard() async {
    await widget.ffi.invokeMethod('enable_soft_keyboard', true);
    if (!mounted) return;
    _focusNode.unfocus();
    // Android must apply the window flag before a fresh input connection is attached.
    await Future<void>.delayed(const Duration(milliseconds: 50));
    if (mounted) _viewKey.currentState?.requestKeyboard();
  }

  Future<void> _paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (mounted && data?.text != null) await _model.pasteText(data!.text!);
  }

  void _imagePointerUp(PointerUpEvent event) {
    final down = _imageTap;
    if (down == null || down.pointer != event.pointer) return;
    _imageTap = null;
    if ((event.position - down.position).distance > 12 ||
        event.timeStamp - down.timeStamp > const Duration(milliseconds: 400)) {
      return;
    }
    final render = _viewKey.currentState?.renderTerminal;
    if (render == null) return;
    final offset = render.getCellOffset(render.globalToLocal(event.position));
    final path = terminalImagePathAt(_model.terminal, offset);
    if (path != null) {
      scheduleMicrotask(() {
        if (mounted) unawaited(_previewImage(path));
      });
    }
  }

  void _imageResponse(Map<String, dynamic> evt) {
    if (_imageResult == null ||
        (evt['type'] != 'error' &&
            int.tryParse('${evt['request_id']}') != _imageId)) return;
    final result = _imageResult!;
    if (result.isCompleted) return;
    final error = evt['error']?.toString() ?? '';
    if (error.isNotEmpty) {
      result.completeError(Exception(error));
      return;
    }
    try {
      _imageBytes!.add(base64Decode(evt['data'] ?? ''));
      if (_imageBytes!.length > 8 * 1024 * 1024) {
        throw Exception('Image exceeds 8 MB');
      }
      if (evt['done'] == 'true' || evt['done'] == true) {
        result.complete(_imageBytes!.takeBytes());
      }
    } catch (error) {
      result.completeError(error);
    }
  }

  Future<Uint8List> _loadImage(String path) async {
    final result = Completer<Uint8List>();
    _imageResult = result;
    _imageBytes = BytesBuilder(copy: false);
    try {
      unawaited(bind
          .sessionTerminalImage(
              sessionId: widget.ffi.sessionId,
              requestId: _imageId =
                  DateTime.now().microsecondsSinceEpoch & 0xffffffff,
              path: path)
          .catchError((Object error) {
        if (!result.isCompleted) result.completeError(error);
      }));
      return await result.future.timeout(const Duration(seconds: 45));
    } finally {
      _imageResult = null;
      _imageBytes = null;
    }
  }

  Future<void> _previewImage(String path) async {
    if (_imageResult != null) return;
    _focusNode.unfocus();
    final image = _loadImage(path);
    await Navigator.of(context).push(MaterialPageRoute(
      builder: (_) => TerminalImagePreview(path: path, image: image),
    ));
    if (mounted && !_closing) _focusNode.requestFocus();
  }

  Future<void> _enterImagePath() async {
    await widget.ffi.invokeMethod('enable_soft_keyboard', true);
    if (!mounted) return;
    var enteredPath = '';
    final path = await showDialog<String>(
        context: context,
        builder: (context) => AlertDialog(
              title: Text(translate('Preview image')),
              content: TextField(
                onChanged: (value) => enteredPath = value,
                autofocus: true,
                decoration: InputDecoration(
                    labelText: translate('Image path'),
                    hintText: './image.png'),
                onSubmitted: (value) => Navigator.pop(context, value),
              ),
              actions: [
                TextButton(
                    onPressed: () => Navigator.pop(context),
                    child: Text(translate('Cancel'))),
                TextButton(
                    onPressed: () => Navigator.pop(context, enteredPath),
                    child: Text(translate('OK'))),
              ],
            ));
    if (mounted && path != null && path.trim().isNotEmpty) {
      await _previewImage(path.trim());
    }
  }

  @override
  void dispose() {
    if (_imageResult?.isCompleted == false) {
      _imageResult!.completeError(Exception('Terminal closed'));
    }
    widget.ffi.ffiModel.removeListener(_permissionChanged);
    widget.ffi.unregisterTerminalModel(0);
    unawaited(_model.closeTerminal());
    _model.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  Widget _key(String label, String sequence) => TextButton(
        onPressed: () => _model.sendVirtualKey(sequence),
        child: Text(label),
      );

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
          title: Text(translate('Terminal')),
          actions: [
            IconButton(
                icon: const Icon(Icons.image_outlined),
                tooltip: translate('Preview image'),
                onPressed: _enterImagePath),
            IconButton(
              icon: const Icon(Icons.content_paste),
              tooltip: translate('Paste'),
              onPressed: _paste,
            ),
            IconButton(
              icon: const Icon(Icons.keyboard),
              tooltip: translate('wayland-soft-keyboard-input-label'),
              onPressed: _openKeyboard,
            ),
          ],
        ),
        backgroundColor: Colors.black,
        body: SafeArea(
          child: Column(children: [
            Expanded(
              child: Listener(
                onPointerDown: (event) => _imageTap = event,
                onPointerUp: _imagePointerUp,
                child: TerminalView(
                  _model.terminal,
                  key: _viewKey,
                  focusNode: _focusNode,
                  controller: _model.terminalController,
                  autofocus: true,
                  deleteDetection: true,
                  textStyle: const TerminalStyle(fontSize: 14),
                  padding: const EdgeInsets.all(8),
                ),
              ),
            ),
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(children: [
                _key('Ctrl+C', '\x03'),
                _key('Tab', '\t'),
                _key('Esc', '\x1b'),
                _key('←', '\x1b[D'),
                _key('↑', '\x1b[A'),
                _key('↓', '\x1b[B'),
                _key('→', '\x1b[C'),
                _key('Enter', '\r'),
                _key('Shift+Tab', '\x1b[Z'),
              ]),
            ),
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(children: [
                _key('Ctrl+D', '\x04'),
                _key('Ctrl+Z', '\x1a'),
                _key('Ctrl+L', '\x0c'),
                _key('Ctrl+R', '\x12'),
                _key('Ctrl+J', '\x0a'),
                _key('Ctrl+O', '\x0f'),
                _key('Ctrl+T', '\x14'),
                _key('Ctrl+A', '\x01'),
                _key('Ctrl+E', '\x05'),
                _key('Ctrl+U', '\x15'),
                _key('Ctrl+K', '\x0b'),
                _key('Ctrl+W', '\x17'),
                _key('Alt+Enter', '\x1b\r'),
                _key('Home', '\x1b[H'),
                _key('End', '\x1b[F'),
                _key('PgUp', '\x1b[5~'),
                _key('PgDn', '\x1b[6~'),
              ]),
            ),
          ]),
        ),
      );
}
