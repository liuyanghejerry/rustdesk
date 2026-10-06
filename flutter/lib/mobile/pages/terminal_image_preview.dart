import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_hbb/common.dart';

class TerminalImagePreview extends StatefulWidget {
  const TerminalImagePreview(
      {super.key, required this.path, required this.image});
  final String path;
  final Future<Uint8List> image;

  @override
  State<TerminalImagePreview> createState() => _TerminalImagePreviewState();
}

class _TerminalImagePreviewState extends State<TerminalImagePreview> {
  final _transform = TransformationController();
  Offset _doubleTapPosition = Offset.zero;

  @override
  void dispose() {
    _transform.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        backgroundColor: Colors.black,
        appBar: AppBar(
          title:
              Text(widget.path, maxLines: 1, overflow: TextOverflow.ellipsis),
          actions: [
            IconButton(
              icon: const Icon(Icons.fit_screen),
              tooltip: translate('Reset canvas'),
              onPressed: () => _transform.value = Matrix4.identity(),
            ),
          ],
        ),
        body: FutureBuilder<Uint8List>(
          future: widget.image,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              return Center(
                  child: Padding(
                padding: const EdgeInsets.all(24),
                child: Text('${translate('Failed')}: ${snapshot.error}',
                    style: const TextStyle(color: Colors.white)),
              ));
            }
            if (!snapshot.hasData) {
              return const Center(child: CircularProgressIndicator());
            }
            return GestureDetector(
              onDoubleTapDown: (details) =>
                  _doubleTapPosition = details.localPosition,
              onDoubleTap: () =>
                  _transform.value = _transform.value.getMaxScaleOnAxis() > 1
                      ? Matrix4.identity()
                      : (Matrix4.identity()
                        ..translate(
                            -_doubleTapPosition.dx, -_doubleTapPosition.dy)
                        ..scale(2.0)),
              child: InteractiveViewer(
                transformationController: _transform,
                minScale: 1,
                maxScale: 8,
                child: SizedBox.expand(
                    child: Image(
                  image: ResizeImage(MemoryImage(snapshot.data!),
                      width: 4096,
                      height: 4096,
                      policy: ResizeImagePolicy.fit,
                      allowUpscaling: false),
                  fit: BoxFit.contain,
                  errorBuilder: (_, error, __) => Center(
                      child: Text(
                    '${translate('Failed')}: $error',
                    style: const TextStyle(color: Colors.white),
                  )),
                )),
              ),
            );
          },
        ),
      );
}
