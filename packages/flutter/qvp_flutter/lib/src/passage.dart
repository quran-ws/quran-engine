// QvpPassage — the layout of a contiguous ayah range across pages.
import 'engine.dart';

/// A contiguous range of complete ayahs, or a surah's basmalah, ready to lay out at any width.
class QvpPassage {
  QvpPassage._(
    this.engine,
    this._backend,
    this._handle,
    this.ayahs,
    this.lineSpacing,
  );

  /// Loads ayahs [from] to [to] of [surah] from the pages that print them.
  factory QvpPassage.load(List<QvpPage> pages, int surah, int from, {int? to}) {
    final last = to ?? from;
    _checkU16(surah, 'surah');
    _checkU16(from, 'from');
    _checkU16(last, 'to');
    final engine = _engineOf(pages);
    final handles = [for (final p in pages) p.handle];
    final (handle, ayahs, lineSpacing) = engine.backend.passageLoad(
      handles,
      surah,
      from,
      last,
    );
    return QvpPassage._(engine, engine.backend, handle, ayahs, lineSpacing);
  }

  /// Loads the basmalah printed above [surah].
  factory QvpPassage.basmalah(List<QvpPage> pages, int surah) {
    _checkU16(surah, 'surah');
    final engine = _engineOf(pages);
    final handles = [for (final p in pages) p.handle];
    final (handle, ayahs, lineSpacing) = engine.backend.passageLoadBasmalah(
      handles,
      surah,
    );
    return QvpPassage._(engine, engine.backend, handle, ayahs, lineSpacing);
  }

  /// Loads the medallion that closes ayah [ayah] of [surah], from whichever of the pages
  /// prints it. It lays out as one piece on one row and has no ayahs and no words. It sits on
  /// the baseline of the line its ayah ends on, so at the scale of the ayah's own layout it has
  /// the size it has after the ayah's last word. Lay the ayah out with
  /// [QvpPassageSpec.omitAyahMarks] to show it apart. It is the page's printed medallion, or
  /// the design [dress] puts on it. Throws when no page given prints it.
  factory QvpPassage.ayahMark(List<QvpPage> pages, int surah, int ayah) {
    _checkU16(surah, 'surah');
    _checkU16(ayah, 'ayah');
    final engine = _engineOf(pages);
    final handles = [for (final p in pages) p.handle];
    final (handle, ayahs, lineSpacing) = engine.backend.passageLoadAyahMark(
      handles,
      surah,
      ayah,
    );
    return QvpPassage._(engine, engine.backend, handle, ayahs, lineSpacing);
  }

  static void _checkU16(int v, String name) {
    if (v < 1 || v > 0xffff) throw RangeError.range(v, 1, 0xffff, name);
  }

  static QvpEngine _engineOf(List<QvpPage> pages) {
    if (pages.isEmpty) {
      throw ArgumentError.value(
        pages,
        'pages',
        'a passage needs at least one page',
      );
    }
    for (final p in pages) {
      if (p.isDisposed) throw StateError('QvpPassage: a page was disposed');
    }
    return pages.first.engine;
  }

  /// The engine of the pages the passage was loaded from.
  final QvpEngine engine;
  final QvpBackend _backend;
  PassageHandle? _handle;

  /// The ayahs in reading order. Empty for a basmalah.
  final List<QvpPassageText> ayahs;

  /// The printed line spacing of the passage's first word, in page units.
  final double lineSpacing;

  /// The last layout, if any.
  QvpPassageLayout? currentLayout;

  /// Throws a [StateError] once the passage is closed or disposed, in every build mode.
  PassageHandle get handle =>
      _handle ?? (throw StateError('QvpPassage is disposed'));

  /// True after [close] or [dispose].
  bool get isDisposed => _handle == null;

  /// Frees backend passage memory. A second call does nothing.
  void close() {
    final h = _handle;
    if (h != null) {
      _handle = null;
      _backend.passageFree(h);
    }
  }

  /// The same as [close].
  void dispose() => close();

  /// Lays the passage out. Null when spec is out of range.
  QvpPassageLayout? layout(QvpPassageSpec spec) {
    final l = _backend.passageLayout(handle, spec);
    if (l != null) currentLayout = l;
    return l;
  }

  /// Puts the ayah medallion of another mushaf on the passage, as [QvpPage.dress] puts it on a
  /// page. It takes the same set and spec and uses only the medallion: [QvpDressSpec.ayahMark]
  /// (or the style's medallion), [QvpDressSpec.ayahMarkScale], line art for
  /// [QvpOrnamentKind.ayahMark] and the colours. Each design goes on its printed ring as on the
  /// page. The ring leaves the draw list and the numeral stays. The layout makes room for the
  /// design, the same in colour and as line art, so dark mode moves no row break. Returns false,
  /// and leaves the passage undressed, when the spec names no medallion. A change of dress drops
  /// the layout: call [layout] again, whose [QvpPassageLayout.ornaments] are the medallions to
  /// draw.
  bool dress(QvpOrnaments set, [QvpDressSpec spec = const QvpDressSpec()]) {
    final ok = _backend.passageDress(handle, set.handle, spec);
    currentLayout = null;
    return ok;
  }

  /// Takes the dress off: the printed medallions come back. Lay the passage out again.
  void undress() {
    _backend.passageUndress(handle);
    currentLayout = null;
  }

  /// The draw list of the last layout as its pages style it now. With a letter layer, a cut
  /// path whose pieces differ in colour on its page is listed as its pieces, in its place and
  /// under its placement. Draw each path in its page's colour ([QvpPage.colors]) to show a
  /// letter coloured on its own: [paintPassage] with `draws:` and `pageColors: true` does.
  /// [pages] are the open pages the passage was loaded from; a draw whose page is not given is
  /// listed as the layout has it. Empty with no layout.
  List<QvpPassageDraw> styledDrawList(List<QvpPage> pages) {
    for (final p in pages) {
      if (p.isDisposed) throw StateError('QvpPassage: a page was disposed');
    }
    return _backend.passageStyledDrawList(handle, [
      for (final p in pages) p.handle,
    ]);
  }
}
