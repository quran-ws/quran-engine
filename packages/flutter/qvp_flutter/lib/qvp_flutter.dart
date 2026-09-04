/// Flutter wrapper for the QVP vector mushaf engine (dart:ffi over `libqvp_ffi`).
///
/// `QvpEngine` loads the library, `QvpPage` is one decoded page (geometry
/// copied out once; styles, highlights, masks, layout and hit-testing live in
/// the engine), `QvpAtlas` is the cross-page index and `QvpPageView` is the
/// host renderer. Method names mirror `web/qvp.js` one to one.
library;

export 'src/engine.dart';
export 'src/page_view.dart';
