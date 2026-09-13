// Mushaf Vector Reader — Flutter demo of the QVP engine (mirrors web/app.js).
//
// The engine decides everything (layout, hit-testing, styles, highlights,
// masks, search, text); this app only wires UI controls to QvpPage calls and
// lets QvpPageView draw. Page data is bundled as example assets only — the
// plugin ships no data.
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qvp_flutter/qvp_flutter.dart';

void main() => runApp(const MushafApp());

/// Pages bundled with the example (dist/pages/NNN.qvp + NNN.words.json).
const List<int> kPages = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 440, 441, 442, 443, 444, 445, 582, 604];

String _pad(int n) => n.toString().padLeft(3, '0');

/// UI palette per theme (the CSS variables of web/app.html).
class Palette {
  const Palette({required this.bg, required this.paper, required this.fg, required this.muted, required this.panel, required this.line, required this.ink, required this.grey, required this.brightness, required this.seed});
  final Color bg, paper, fg, muted, panel, line, seed;
  final int ink, grey;
  final Brightness brightness;

  static const light = Palette(bg: Color(0xfff6f1e7), paper: Color(0xfffffdf7), fg: Color(0xff1f1b16), muted: Color(0xff6b6257), panel: Color(0xffffffff), line: Color(0xffe6ded0), ink: 0x231f20ff, grey: 0xc9c4b8ff, brightness: Brightness.light, seed: Color(0xff0a7d32));
  static const sepia = Palette(bg: Color(0xffe9dcc3), paper: Color(0xfff3e7cf), fg: Color(0xff3b2a14), muted: Color(0xff7a6a52), panel: Color(0xfff8efdd), line: Color(0xffd9c9a8), ink: 0x3b2a14ff, grey: 0xc9c4b8ff, brightness: Brightness.light, seed: Color(0xff8a5a1a));
  static const dark = Palette(bg: Color(0xff15171b), paper: Color(0xff1e2126), fg: Color(0xffe8e4dc), muted: Color(0xff9a948b), panel: Color(0xff1b1e23), line: Color(0xff2c3138), ink: 0xe8e4dcff, grey: 0x4a4f57ff, brightness: Brightness.dark, seed: Color(0xff4caf50));
  static const byName = {'light': light, 'sepia': sepia, 'dark': dark};
}

/// Mark-category colours (the "Mark colours" theme + legend).
const Map<int, String> kPalette = {
  QvpCategory.harakah: '#1a73e8',
  QvpCategory.tanwin: '#8e24aa',
  QvpCategory.letterDot: '#c62828',
  QvpCategory.waqf: '#0a7d32',
  QvpCategory.dabt: '#ef6c00',
  QvpCategory.orthographic: '#00838f',
  QvpCategory.standalone: '#6d4c41',
};

class MushafApp extends StatefulWidget {
  const MushafApp({super.key});
  @override
  State<MushafApp> createState() => _MushafAppState();
}

class _MushafAppState extends State<MushafApp> {
  String themeName = 'light';

  @override
  Widget build(BuildContext context) {
    final p = Palette.byName[themeName]!;
    final scheme = ColorScheme.fromSeed(seedColor: p.seed, brightness: p.brightness, surface: p.panel);
    return MaterialApp(
      title: 'Mushaf Vector Reader',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: scheme,
        scaffoldBackgroundColor: p.bg,
        appBarTheme: AppBarTheme(backgroundColor: p.panel, foregroundColor: p.fg),
        visualDensity: VisualDensity.compact,
        inputDecorationTheme: const InputDecorationTheme(isDense: true, border: OutlineInputBorder(), contentPadding: EdgeInsets.symmetric(horizontal: 10, vertical: 8)),
      ),
      home: ReaderPage(palette: p, themeName: themeName, onTheme: (t) => setState(() => themeName = t)),
    );
  }
}

class ReaderPage extends StatefulWidget {
  const ReaderPage({super.key, required this.palette, required this.themeName, required this.onTheme});
  final Palette palette;
  final String themeName;
  final ValueChanged<String> onTheme;
  @override
  State<ReaderPage> createState() => _ReaderPageState();
}

class _ReaderPageState extends State<ReaderPage> with SingleTickerProviderStateMixin {
  // ── engine & data ──
  QvpEngine? engine;
  QvpAtlas? atlas;
  QvpPage? page;
  String? loadError;
  int n = kPages.first, bytes = 0;
  double loadMs = 0, initMs = 0;
  final viewKey = GlobalKey<QvpPageViewState>();
  final view = QvpViewController();

  // ── state (S in app.js) ──
  int selWord = -1;
  (int, int)? selAyah;
  int hlSel = 0, hlAyah = 0, hlSearch = 0, hlPlay = 0, tajwidHandle = 0, hideHandle = 0, ayahMarksHandle = 0;
  final Map<int, int> pathHandles = {};
  bool tajwid = false, hideMarks = false, goldAyahMarks = false;
  bool playing = false;
  int playIdx = 0;
  Timer? playTimer;
  String hlMode = 'both';
  int hlMs = 250;
  int ink = Palette.light.ink;
  bool revealOn = false;
  int revealPos = -1, revealStepCount = 0;
  String maskMode = 'hide';
  QvpViewLayout layout = const QvpViewLayout(padTop: 24, padBottom: 24, padSide: 16);
  final searchCtl = TextEditingController();
  final gotoCtl = TextEditingController();
  final pageCtl = TextEditingController(text: '1');
  String searchMode = 'includes';
  List<QvpMatch> results = const [];
  String? cropInfo;
  String copied = '';
  bool panelOpen = true;
  double lastHitUs = 0;
  late final TabController tabs = TabController(length: 8, vsync: this);

  Palette get pal => widget.palette;

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    try {
      final sw = Stopwatch()..start();
      final e = QvpEngine.open();
      initMs = sw.elapsedMicroseconds / 1000;
      QvpAtlas? a;
      try {
        a = e.loadAtlas((await rootBundle.load('assets/pages/atlas.qva')).buffer.asUint8List());
      } catch (_) {
        a = null;
      }
      setState(() {
        engine = e;
        atlas = a;
      });
      await loadPage(kPages.first);
    } catch (err, st) {
      debugPrint('qvp init failed: $err\n$st');
      setState(() => loadError = '$err');
    }
  }

  @override
  void dispose() {
    playTimer?.cancel();
    page?.dispose();
    atlas?.dispose();
    engine?.dispose();
    tabs.dispose();
    view.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(ReaderPage old) {
    super.didUpdateWidget(old);
    if (old.themeName != widget.themeName) {
      ink = pal.ink;
      _applyTheme();
    }
  }

  // ── page loading ──
  Future<void> loadPage(int want) async {
    final e = engine;
    if (e == null) return;
    var target = want;
    if (!kPages.contains(target)) target = kPages.reduce((a, b) => (b - want).abs() < (a - want).abs() ? b : a);
    final data = (await rootBundle.load('assets/pages/${_pad(target)}.qvp')).buffer.asUint8List();
    final sw = Stopwatch()..start();
    final np = e.loadPage(data);
    try {
      final json = await rootBundle.loadString('assets/pages/${_pad(target)}.words.json');
      np.attachWords(json);
    } catch (_) {}
    loadMs = sw.elapsedMicroseconds / 1000;
    final old = page;
    _stopPlay(notify: false);
    setState(() {
      page = np;
      n = target;
      bytes = data.length;
      selWord = -1;
      selAyah = null;
      playIdx = 0;
      hlSel = hlAyah = hlSearch = hlPlay = 0;
      pathHandles.clear();
      revealOn = false;
      revealPos = -1;
      tajwidHandle = hideHandle = ayahMarksHandle = 0;
      cropInfo = null;
      pageCtl.text = '$target';
    });
    if (old != null) WidgetsBinding.instance.addPostFrameCallback((_) => old.dispose());
    _applyTheme();
    _applyToggles();
    runSearch();
  }

  // ── selection ──
  void selectWord(int i) {
    final p = page!;
    selAyah = null;
    viewKey.currentState?.clearSelection();
    if (hlAyah != 0) {
      p.removeHighlight(hlAyah);
      hlAyah = 0;
    }
    for (final h in pathHandles.values) {
      p.removeStyle(h);
    }
    pathHandles.clear();
    if (i < 0 || i == selWord) {
      selWord = -1;
      if (hlSel != 0) {
        p.removeHighlight(hlSel);
        hlSel = 0;
      }
    } else {
      selWord = i;
      final st = QvpHighlightStyle(mode: hlMode, ink: '#1a73e8', band: rgba('#1a73e8', 0.18), radius: 1.5, ms: hlMs, layer: QvpLayer.selection);
      if (hlSel != 0) {
        p.moveHighlight(hlSel, T.word(i));
      } else {
        hlSel = p.highlight(T.word(i), st);
      }
    }
    cropInfo = null;
    setState(() {});
    if (i >= 0) tabs.animateTo(1);
  }

  void selectAyah(int s, int a) {
    final p = page!;
    if (hlSel != 0) {
      p.removeHighlight(hlSel);
      hlSel = 0;
    }
    selWord = -1;
    viewKey.currentState?.clearSelection();
    selAyah = (s, a);
    final st = QvpHighlightStyle(mode: hlMode, ink: '#0a7d32', band: rgba('#0a7d32', 0.14), radius: 1.5, ms: hlMs, layer: QvpLayer.selection);
    if (hlAyah != 0) {
      p.moveHighlight(hlAyah, T.ayah(s, a));
    } else {
      hlAyah = p.highlight(T.ayah(s, a), st);
    }
    cropInfo = null;
    setState(() {});
    tabs.animateTo(1);
  }

  void _onDragSelection(List<int> ws) {
    if (ws.isEmpty) return;
    final p = page!;
    if (hlSel != 0) {
      p.removeHighlight(hlSel);
      hlSel = 0;
    }
    if (hlAyah != 0) {
      p.removeHighlight(hlAyah);
      hlAyah = 0;
    }
    selWord = -1;
    selAyah = null;
    cropInfo = null;
    setState(() {});
    tabs.animateTo(1);
  }

  // ── search ──
  void runSearch() {
    final p = page;
    if (p == null) return;
    final q = searchCtl.text.trim();
    if (hlSearch != 0) {
      p.removeHighlight(hlSearch);
      hlSearch = 0;
    }
    if (q.isEmpty) {
      setState(() => results = const []);
      return;
    }
    final m = p.search(q, mode: searchMode);
    if (m.isNotEmpty) {
      hlSearch = p.highlight(T.words(m.map((x) => x.word).toList()), QvpHighlightStyle(mode: 'both', ink: '#c62828', band: rgba('#c62828', 0.12), height: 'ink', padY: 1, radius: 1, ms: hlMs));
    }
    setState(() => results = m);
  }

  // ── goto ──
  Future<void> goto(String v) async {
    v = v.trim();
    final a = atlas;
    if (v.isEmpty || a == null) return;
    RegExpMatch? m;
    if ((m = RegExp(r'^(\d+):(\d+)').firstMatch(v)) != null) {
      final s = int.parse(m!.group(1)!), ay = int.parse(m.group(2)!);
      final pg = a.pageOf(s, ay);
      if (pg != null) {
        await loadPage(pg);
        if (page!.targetWords(T.ayah(s, ay)).isNotEmpty) selectAyah(s, ay);
      } else {
        _snack('no page for $s:$ay');
      }
      return;
    }
    if ((m = RegExp(r'^juz\s*(\d+)', caseSensitive: false).firstMatch(v)) != null) {
      final j = a.juz(int.parse(m!.group(1)!));
      if (j != null) await loadPage(j.page);
      return;
    }
    var su = a.searchSurahs(v);
    if (su.isEmpty) {
      // UI nicety: loose latin spelling ("yasin" → "Ya-Sin") against the atlas list
      String fold(String t) => t.toLowerCase().replaceAll(RegExp('[^a-z0-9]'), '');
      final q = fold(v);
      if (q.isNotEmpty) su = a.surahs().where((s) => fold(s.latin).contains(q) || fold(s.english).contains(q)).toList();
    }
    if (su.isNotEmpty) {
      await loadPage(su.first.page);
    } else {
      _snack('no surah matches "$v"');
    }
  }

  void _snack(String s) => ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(s), duration: const Duration(seconds: 2)));

  // ── copy / crop ──
  Future<void> copy() async {
    final p = page!;
    String text;
    if (p.selection().isNotEmpty) {
      text = p.selectionText('rasmUthmani', true);
    } else if (selAyah != null) {
      text = '${p.text(T.ayah(selAyah!.$1, selAyah!.$2))} (${selAyah!.$1}:${selAyah!.$2})';
    } else if (selWord >= 0) {
      text = '${p.words[selWord].text} (${p.citation([selWord])})';
    } else {
      return;
    }
    await Clipboard.setData(ClipboardData(text: text));
    setState(() => copied = 'copied');
    Timer(const Duration(milliseconds: 1500), () => mounted ? setState(() => copied = '') : null);
  }

  QvpTarget? _selTarget() {
    final p = page!;
    final sel = p.selection();
    if (sel.isNotEmpty) return T.words(sel);
    if (selAyah != null) return T.ayah(selAyah!.$1, selAyah!.$2);
    if (selWord >= 0) return T.word(selWord);
    return null;
  }

  void crop() {
    final t = _selTarget();
    if (t == null) return;
    final p = page!;
    final svg = p.cropSvg(t, pad: 3, keepAyahMarks: true, background: QvpColor.fromColor(pal.paper));
    final box = p.cropBounds(t, pad: 3);
    setState(() => cropInfo = svg == null || box == null
        ? 'crop failed'
        : 'SVG ${svg.length} chars · ${svg.substring(0, svg.indexOf('>') + 1).replaceAll(RegExp(r'\s+'), ' ')}\n'
            'box ${box.x0.toStringAsFixed(1)},${box.y0.toStringAsFixed(1)} → ${box.x1.toStringAsFixed(1)},${box.y1.toStringAsFixed(1)} · ${box.nWords} words${box.ayahMarkDeco != qvpNone ? ' · ayahMark' : ''}');
  }

  // ── follow words ──
  void _stopPlay({bool notify = true}) {
    playing = false;
    playTimer?.cancel();
    playTimer = null;
    if (hlPlay != 0 && page != null && !page!.isDisposed) page!.removeHighlight(hlPlay);
    hlPlay = 0;
    if (notify) setState(() {});
  }

  void togglePlay() {
    if (playing) {
      _stopPlay();
      return;
    }
    final p = page!;
    playing = true;
    playIdx = selWord >= 0 ? selWord : 0;
    final st = QvpHighlightStyle(mode: hlMode, ink: '#d81b60', band: rgba('#d81b60', 0.14), radius: 1.5, ms: hlMs);
    hlPlay = p.highlight(T.word(playIdx), st);
    playTimer = Timer.periodic(const Duration(milliseconds: 320), (_) async {
      final cur = page!;
      playIdx++;
      if (playIdx >= cur.nWords) {
        final i = kPages.indexOf(n);
        if (i + 1 < kPages.length) {
          await loadPage(kPages[i + 1]);
          playing = true;
          playIdx = 0;
          hlPlay = page!.highlight(T.word(0), st);
          setState(() {});
        } else {
          _stopPlay();
        }
        return;
      }
      cur.moveHighlight(hlPlay, T.word(playIdx));
    });
    setState(() {});
  }

  // ── styling toggles (each is one engine handle) ──
  void _applyToggles() {
    final p = page;
    if (p == null) return;
    if (tajwidHandle != 0) {
      p.removeStyle(tajwidHandle);
      tajwidHandle = 0;
    }
    if (tajwid) {
      tajwidHandle = p.theme(QvpTheme(diacritics: kPalette[QvpCategory.harakah], dots: kPalette[QvpCategory.letterDot], waqf: kPalette[QvpCategory.waqf], sifr: kPalette[QvpCategory.dabt], ms: 200));
    }
    if (hideHandle != 0) {
      p.removeStyle(hideHandle);
      hideHandle = 0;
    }
    if (hideMarks) hideHandle = p.hide(Sel.kind(QvpKind.mark));
    if (ayahMarksHandle != 0) {
      p.removeStyle(ayahMarksHandle);
      ayahMarksHandle = 0;
    }
    if (goldAyahMarks) ayahMarksHandle = p.style(Sel.deco(QvpDeco.ayahMark), '#b8860b', ms: 300, layer: QvpLayer.theme + 1);
    setState(() {});
  }

  void clearAll() {
    final p = page!;
    tajwid = hideMarks = goldAyahMarks = false;
    p.clearStyles();
    p.clearHighlights();
    p.unmask();
    p.revealStop();
    viewKey.currentState?.clearSelection();
    hlSel = hlAyah = hlSearch = hlPlay = 0;
    tajwidHandle = hideHandle = ayahMarksHandle = 0;
    pathHandles.clear();
    selWord = -1;
    selAyah = null;
    revealOn = false;
    revealPos = -1;
    searchCtl.clear();
    results = const [];
    cropInfo = null;
    _stopPlay(notify: false);
    _applyTheme();
    setState(() {});
  }

  void _applyTheme() {
    final p = page;
    if (p == null) return;
    p.setDefaultColor(ink);
    setState(() {});
  }

  // ── memorisation ──
  void maskAyah() {
    final p = page!;
    final t = selAyah != null
        ? T.ayah(selAyah!.$1, selAyah!.$2)
        : selWord >= 0
            ? T.ayah(p.words[selWord].surah, p.words[selWord].ayah)
            : T.page();
    p.maskOptions(blockColor: QvpColor.fromColor(pal.line));
    p.mask(t, maskMode);
    setState(() {});
  }

  void toggleReveal() {
    final p = page!;
    revealOn = !revealOn;
    if (revealOn) {
      revealStepCount = p.revealStart(lit: 2, grey: pal.grey, ink: ink, ms: 150);
      revealPos = -1;
    } else {
      p.revealStop();
    }
    setState(() {});
  }

  // ── layout ──
  void _setLayout(QvpViewLayout l) => setState(() => layout = l);

  void fitGap() {
    final p = page!;
    final vp = view.viewport;
    final gap = p.layoutGapToFill(layout.toSpec(vp.width, vp.height));
    _setLayout(layout.copyWith(fillHeight: false, lineSpacing: 1, lineGap: gap));
  }

  // ═════════ UI ═════════
  @override
  Widget build(BuildContext context) {
    final p = page;
    if (loadError != null) return Scaffold(body: Center(child: Padding(padding: const EdgeInsets.all(24), child: Text('Engine failed to load:\n$loadError'))));
    if (p == null) return const Scaffold(body: Center(child: CircularProgressIndicator()));
    return LayoutBuilder(builder: (context, c) {
      final wide = c.maxWidth >= 800;
      final stage = Container(
        color: pal.bg,
        child: QvpPageView(
          key: viewKey,
          page: p,
          controller: view,
          layout: layout,
          paper: pal.paper,
          defaultInk: ink,
          onWordTap: (w, hit) {
            lastHitUs = hit.distance; // distance in page units of the gap-aware hit
            selectWord(w);
          },
          onDecoTap: (d) {
            if (d.ayah > 0) selectAyah(d.surah, d.ayah);
          },
          onEmptyTap: () => selectWord(-1),
          onSelectionChanged: _onDragSelection,
        ),
      );
      final panel = _panel(wide);
      return Scaffold(
        appBar: AppBar(
          titleSpacing: 12,
          title: const Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
            Text('Mushaf Vector Reader', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            Text('QVP engine via dart:ffi · host CustomPainter · every decision in the engine', style: TextStyle(fontSize: 11)),
          ]),
          actions: [
            IconButton(tooltip: 'Fit page', icon: const Icon(Icons.fit_screen), onPressed: () => viewKey.currentState?.fit()),
            PopupMenuButton<String>(
              tooltip: 'Theme',
              icon: const Icon(Icons.palette_outlined),
              onSelected: widget.onTheme,
              itemBuilder: (_) => [for (final t in Palette.byName.keys) CheckedPopupMenuItem(value: t, checked: t == widget.themeName, child: Text(t))],
            ),
            if (!wide) IconButton(tooltip: panelOpen ? 'Hide panel' : 'Show panel', icon: Icon(panelOpen ? Icons.expand_more : Icons.expand_less), onPressed: () => setState(() => panelOpen = !panelOpen)),
          ],
        ),
        body: Column(children: [
          _navBar(),
          Expanded(
            child: wide
                ? Row(children: [
                    Expanded(child: stage),
                    SizedBox(width: 360, child: panel),
                  ])
                : Column(children: [
                    Expanded(child: stage),
                    if (panelOpen) SizedBox(height: c.maxHeight * 0.42, child: panel),
                  ]),
          ),
        ]),
      );
    });
  }

  Widget _navBar() {
    return Material(
      color: pal.panel,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
        child: Row(children: [
          Expanded(
            child: TextField(
              controller: gotoCtl,
              decoration: const InputDecoration(hintText: '2:255 · Ya-Sin · juz 30', prefixIcon: Icon(Icons.explore_outlined, size: 18)),
              onSubmitted: goto,
              textInputAction: TextInputAction.go,
            ),
          ),
          const SizedBox(width: 6),
          IconButton(tooltip: 'Previous page', icon: const Icon(Icons.navigate_before), onPressed: () => loadPage(kPages[(kPages.indexOf(n) - 1).clamp(0, kPages.length - 1)])),
          SizedBox(
            width: 58,
            child: TextField(
              controller: pageCtl,
              textAlign: TextAlign.center,
              keyboardType: TextInputType.number,
              onSubmitted: (v) => loadPage(int.tryParse(v) ?? n),
            ),
          ),
          Padding(padding: const EdgeInsets.symmetric(horizontal: 4), child: Text('/ ${kPages.last}', style: TextStyle(color: pal.muted, fontSize: 12))),
          IconButton(tooltip: 'Next page', icon: const Icon(Icons.navigate_next), onPressed: () => loadPage(kPages[(kPages.indexOf(n) + 1).clamp(0, kPages.length - 1)])),
        ]),
      ),
    );
  }

  Widget _panel(bool wide) {
    final sections = <(String, Widget Function())>[
      ('Search', _searchSection),
      ('Selection', _selectionSection),
      ('Highlights', _highlightSection),
      ('Styling', _stylingSection),
      ('Memorise', _memoSection),
      ('Layout', _layoutSection),
      ('Page', _pageSection),
      ('Engine', _engineSection),
    ];
    if (wide) {
      return Material(
        color: pal.panel,
        child: ListView(padding: const EdgeInsets.all(12), children: [
          for (final s in sections) ...[_h2(s.$1), s.$2(), const SizedBox(height: 12)],
        ]),
      );
    }
    return Material(
      color: pal.panel,
      child: Column(children: [
        TabBar(controller: tabs, isScrollable: true, tabAlignment: TabAlignment.start, labelPadding: const EdgeInsets.symmetric(horizontal: 12), tabs: [for (final s in sections) Tab(text: s.$1, height: 36)]),
        Expanded(
          child: TabBarView(controller: tabs, children: [
            for (final s in sections) SingleChildScrollView(padding: const EdgeInsets.all(12), child: s.$2()),
          ]),
        ),
      ]),
    );
  }

  Widget _h2(String t) => Padding(padding: const EdgeInsets.only(bottom: 6), child: Text(t.toUpperCase(), style: TextStyle(fontSize: 11, fontWeight: FontWeight.w600, letterSpacing: 0.6, color: pal.muted)));
  Widget _hint(String t) => Text(t, style: TextStyle(color: pal.muted, fontSize: 12));
  TextStyle get _ar => const TextStyle(fontSize: 26, height: 1.6);

  // ── Search ──
  Widget _searchSection() {
    final p = page!;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        Expanded(
          child: TextField(
            controller: searchCtl,
            textDirection: TextDirection.rtl,
            decoration: const InputDecoration(hintText: 'الله · الرحمان · كتاب', hintTextDirection: TextDirection.rtl),
            onChanged: (_) => runSearch(),
          ),
        ),
        const SizedBox(width: 6),
        DropdownButton<String>(
          value: searchMode,
          items: [for (final m in ['includes', 'exact', 'prefix']) DropdownMenuItem(value: m, child: Text(m))],
          onChanged: (v) {
            searchMode = v!;
            runSearch();
          },
        ),
      ]),
      const SizedBox(height: 6),
      if (searchCtl.text.trim().isNotEmpty && results.isEmpty) _hint('no match on this page${atlas != null ? ' — try the goto box for surah names' : ''}'),
      if (results.isNotEmpty)
        ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 160),
          child: ListView(shrinkWrap: true, children: [
            for (final m in results)
              InkWell(
                onTap: () => selectWord(m.word),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 3, horizontal: 4),
                  child: Directionality(
                    textDirection: TextDirection.rtl,
                    child: Row(children: [
                      Text(m.text, style: const TextStyle(fontSize: 18)),
                      const SizedBox(width: 8),
                      Text('${m.wordKey}${m.loose ? ' ~' : ''}', style: TextStyle(color: pal.muted, fontSize: 12)),
                    ]),
                  ),
                ),
              ),
          ]),
        ),
      if (results.isNotEmpty) _hint('${results.length} match${results.length == 1 ? '' : 'es'} on page ${p.page}'),
    ]);
  }

  // ── Selection ──
  Widget _selectionSection() {
    final p = page!;
    final sel = p.selection();
    final w = selWord >= 0 ? p.words[selWord] : null;
    final rows = <(String, String, bool)>[];
    String big = '—';
    final chips = <Widget>[];
    if (sel.length > 1) {
      big = p.text(T.words(sel));
      rows.add(('selection', '${sel.length} words · ${p.citation(sel)}', false));
      rows.add(('search form', p.text(T.words(sel), 'search'), true));
    } else if (w != null) {
      big = w.text;
      rows.add(('wordKey', w.wordKey, false));
      rows.add(('line', '${w.line}', false));
      for (final f in ['rasmImlai', 'qpc', 'rasm', 'search']) {
        final v = p.wordForm(w.idx, f);
        if (v.isNotEmpty) rows.add((f, v, true));
      }
      rows.add(('label', p.wordLabel(w.idx), false));
      rows.add(('paths', '${w.nPaths}', false));
      for (var i = w.firstPath; i < w.firstPath + w.nPaths; i++) {
        final kind = p.pathKind(i), mark = p.pathMark(i), nth = p.pathNthMark(i);
        final label = kind == QvpKind.mark ? '${engine!.markName(mark)} #$nth' : engine!.kindName(kind);
        final on = pathHandles.containsKey(i);
        chips.add(FilterChip(
          label: Text(label, style: const TextStyle(fontSize: 11)),
          selected: on,
          visualDensity: VisualDensity.compact,
          tooltip: 'engine path $i · ${engine!.categoryName(p.pathCategory(i))}',
          onSelected: (_) {
            if (on) {
              p.removeStyle(pathHandles.remove(i)!);
            } else {
              pathHandles[i] = kind == QvpKind.mark && nth >= 0 ? p.style(Sel.wordMark(w.idx, nth), '#ef6c00', ms: 200, layer: QvpLayer.top) : p.style(Sel.path(i), '#ef6c00', ms: 200, layer: QvpLayer.top);
            }
            setState(() {});
          },
        ));
      }
    } else if (selAyah != null) {
      final (s, a) = selAyah!;
      final ws = p.targetWords(T.ayah(s, a));
      final c = p.ayahWordCount(s, a);
      big = p.text(T.ayah(s, a));
      rows.add(('ayah', '$s:$a · ${c.count} words${c.complete ? '' : ' (continues on another page)'}', false));
      if (ws.isNotEmpty) rows.add(('label', p.ayahLabel(p.words[ws.first].ayahIdx), false));
    }
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      SizedBox(width: double.infinity, child: Text(big, textDirection: TextDirection.rtl, textAlign: TextAlign.right, style: _ar)),
      for (final r in rows)
        Padding(
          padding: const EdgeInsets.only(bottom: 2),
          child: Row(crossAxisAlignment: CrossAxisAlignment.start, children: [
            SizedBox(width: 84, child: Text(r.$1, style: TextStyle(color: pal.muted, fontSize: 13))),
            Expanded(child: Text(r.$2, textDirection: r.$3 ? TextDirection.rtl : null, textAlign: r.$3 ? TextAlign.right : null, style: TextStyle(fontSize: r.$3 ? 16 : 13))),
          ]),
        ),
      if (chips.isNotEmpty) Wrap(spacing: 4, runSpacing: -6, children: chips),
      const SizedBox(height: 6),
      Wrap(spacing: 6, crossAxisAlignment: WrapCrossAlignment.center, children: [
        OutlinedButton.icon(onPressed: copy, icon: const Icon(Icons.copy, size: 16), label: const Text('Copy with citation')),
        OutlinedButton.icon(onPressed: crop, icon: const Icon(Icons.crop, size: 16), label: const Text('Crop → SVG')),
        if (copied.isNotEmpty) _hint(copied),
      ]),
      if (cropInfo != null) Padding(padding: const EdgeInsets.only(top: 6), child: Container(padding: const EdgeInsets.all(8), decoration: BoxDecoration(border: Border.all(color: pal.line), borderRadius: BorderRadius.circular(6)), child: Text(cropInfo!, style: const TextStyle(fontSize: 11, fontFamily: 'monospace')))),
      const SizedBox(height: 6),
      _hint('Tap a word. Long-press and drag across words to select (whole words, gap-aware). Tap an ayah mark for the ayah. Chips recolour one path — e.g. the 2nd diacritic only.'),
    ]);
  }

  // ── Highlights ──
  Widget _highlightSection() {
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Row(children: [
        const Text('mode '),
        DropdownButton<String>(
          value: hlMode,
          items: const [DropdownMenuItem(value: 'both', child: Text('band + ink')), DropdownMenuItem(value: 'band', child: Text('band only')), DropdownMenuItem(value: 'ink', child: Text('ink only'))],
          onChanged: (v) {
            hlMode = v!;
            if (selWord >= 0) {
              final i = selWord;
              selWord = -1;
              selectWord(i);
            } else {
              setState(() {});
            }
          },
        ),
        const SizedBox(width: 12),
        FilledButton.tonalIcon(onPressed: togglePlay, icon: Icon(playing ? Icons.stop : Icons.play_arrow, size: 18), label: Text(playing ? 'Stop' : 'Follow words')),
      ]),
      Row(children: [
        const Text('fade '),
        Expanded(child: Slider(value: hlMs.toDouble(), min: 0, max: 800, divisions: 16, label: '$hlMs ms', onChanged: (v) => setState(() => hlMs = v.round()))),
        SizedBox(width: 56, child: Text('$hlMs ms', style: const TextStyle(fontSize: 12))),
      ]),
      _hint('Highlights are engine state: the band slides and the ink fades over the transition time when the target moves (moveHighlight).'),
    ]);
  }

  // ── Styling ──
  Widget _stylingSection() {
    Widget toggle(String label, bool on, VoidCallback f) => FilterChip(label: Text(label), selected: on, onSelected: (_) => f());
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Wrap(spacing: 6, runSpacing: 4, children: [
        toggle('Mark colours', tajwid, () {
          tajwid = !tajwid;
          _applyToggles();
        }),
        toggle('Hide marks', hideMarks, () {
          hideMarks = !hideMarks;
          _applyToggles();
        }),
        toggle('Gold ayah marks', goldAyahMarks, () {
          goldAyahMarks = !goldAyahMarks;
          _applyToggles();
        }),
        ActionChip(label: const Text('Clear all'), onPressed: clearAll),
      ]),
      const SizedBox(height: 8),
      Row(children: [
        const Text('Theme '),
        DropdownButton<String>(value: widget.themeName, items: [for (final t in ['light', 'sepia', 'dark']) DropdownMenuItem(value: t, child: Text(t))], onChanged: (v) => widget.onTheme(v!)),
        const SizedBox(width: 12),
        const Text('Ink '),
        for (final c in [pal.ink, 0x1a3a6aff, 0x0a4d2aff, 0x6a1a1aff, 0x000000ff])
          Padding(
            padding: const EdgeInsets.only(right: 4),
            child: InkWell(
              onTap: () {
                ink = c;
                _applyTheme();
              },
              child: Container(width: 22, height: 22, decoration: BoxDecoration(color: QvpColor.toColor(c), borderRadius: BorderRadius.circular(4), border: Border.all(color: ink == c ? Theme.of(context).colorScheme.primary : pal.line, width: ink == c ? 2 : 1))),
            ),
          ),
      ]),
      const SizedBox(height: 6),
      Wrap(spacing: 10, runSpacing: 4, children: [
        for (final e in kPalette.entries.take(5))
          Row(mainAxisSize: MainAxisSize.min, children: [
            Container(width: 12, height: 12, margin: const EdgeInsets.only(right: 4), decoration: BoxDecoration(color: QvpColor.toColor(rgba(e.value)), borderRadius: BorderRadius.circular(3))),
            _hint(engine!.categoryName(e.key)),
          ]),
      ]),
    ]);
  }

  // ── Memorisation ──
  Widget _memoSection() {
    final p = page!;
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      Wrap(spacing: 6, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [
        FilledButton.tonal(onPressed: maskAyah, child: const Text('Mask selected ayah')),
        DropdownButton<String>(value: maskMode, items: const [DropdownMenuItem(value: 'hide', child: Text('hide')), DropdownMenuItem(value: 'block', child: Text('block'))], onChanged: (v) => setState(() => maskMode = v!)),
        OutlinedButton(
            onPressed: () {
              p.unmaskNext(1);
              setState(() {});
            },
            child: const Text('Reveal next')),
        OutlinedButton(
            onPressed: () {
              p.maskBack(1);
              setState(() {});
            },
            child: const Text('Hide back')),
        OutlinedButton(
            onPressed: () {
              p.unmask();
              setState(() {});
            },
            child: const Text('Unmask')),
      ]),
      if (p.maskWords().isNotEmpty) _hint('${p.maskHidden().length} of ${p.maskWords().length} masked words hidden'),
      const SizedBox(height: 6),
      Row(children: [
        FilterChip(label: const Text('Greyed page'), selected: revealOn, onSelected: (_) => toggleReveal()),
        Expanded(
          child: Slider(
            value: revealPos.toDouble(),
            min: -1,
            max: revealOn && revealStepCount > 0 ? (revealStepCount - 1).toDouble() : 0,
            divisions: revealOn && revealStepCount > 0 ? revealStepCount : null,
            onChanged: revealOn
                ? (v) {
                    revealPos = v.round();
                    p.revealGoto(revealPos);
                    setState(() {});
                  }
                : null,
          ),
        ),
        SizedBox(width: 48, child: _hint(revealOn ? '${revealPos + 1}/${p.revealStepCount()}' : '')),
      ]),
      _hint('Mask hides (or blocks) the words of the selected ayah — or the whole page when nothing is selected. The greyed page lights a two-word window that the slider moves.'),
    ]);
  }

  // ── Layout ──
  Widget _layoutSection() {
    Widget row(String label, double v, double min, double max, int div, String text, ValueChanged<double> f) => Row(children: [
          SizedBox(width: 84, child: Text(label, style: const TextStyle(fontSize: 12))),
          Expanded(child: Slider(value: v, min: min, max: max, divisions: div, onChanged: f)),
          SizedBox(width: 44, child: Text(text, style: const TextStyle(fontSize: 12))),
        ]);
    return Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
      row('line spacing', layout.lineSpacing, 1.0, 2.2, 24, '×${layout.lineSpacing.toStringAsFixed(2)}', (v) => _setLayout(layout.copyWith(lineSpacing: v, lineGap: 0, fillHeight: false))),
      row('pad top', layout.padTop, 0, 120, 30, '${layout.padTop.round()}', (v) => _setLayout(layout.copyWith(padTop: v))),
      row('pad bottom', layout.padBottom, 0, 120, 30, '${layout.padBottom.round()}', (v) => _setLayout(layout.copyWith(padBottom: v))),
      Wrap(spacing: 6, children: [
        FilterChip(label: const Text('Fill screen height'), selected: layout.fillHeight, onSelected: (_) => _setLayout(layout.copyWith(fillHeight: !layout.fillHeight))),
        ActionChip(label: const Text('Leading to fill'), onPressed: fitGap),
      ]),
      _hint('Layout is computed by the engine (per-line dy, pitch, scale); the view only applies pan and zoom on top. Double-tap the page to fit. Leading only grows — the printed pitch is the floor — and the text width is always the viewport\'s.'),
    ]);
  }

  // ── Page ──
  Widget _pageSection() {
    final p = page!;
    final su = p.surahs(), dv = p.divisions();
    final names = [for (final s in su) '${s.number}${s.latin.isNotEmpty ? ' ${s.latin}' : ''}${s.arabic.isNotEmpty ? ' ${s.arabic}' : ''}${s.hasBanner ? ' (banner)' : ''}'];
    var t = 'surahs: ${names.join(', ')}';
    if (dv.isNotEmpty) t += '\nstarts here: ${dv.map((d) => '${d.kind} ${d.n} at ${d.surah}:${d.ayah}').join(', ')}';
    final a = atlas;
    if (a != null && p.words.isNotEmpty) {
      final j = a.juzOf(p.words.first.surah, p.words.first.ayah);
      if (j != null) t += '\njuz $j · pages ${a.pagesOfJuz(j)?.join('–')}';
      final r = a.pageRange(p.page);
      if (r != null) t += '\nrange ${r.first.$1}:${r.first.$2} → ${r.last.$1}:${r.last.$2}';
    }
    final ros = p.rosettes(), saj = p.sajdahs();
    if (ros.isNotEmpty) t += '\nrosettes: ${ros.map((r) => 'hizb ${r.hizb} rubuAlHizb ${r.rubuAlHizbInHizb}').join(', ')}';
    if (saj.isNotEmpty) t += '\nsajdah: ${saj.map((s) => '${s.surah}:${s.ayah}').join(', ')}';
    t += '\nayahs: ${p.ayahKeys().map((k) => '${k.$1}:${k.$2}').join(' ')}';
    return _hint(t);
  }

  // ── Engine HUD ──
  Widget _engineSection() {
    final p = page!;
    return ListenableBuilder(
      listenable: Listenable.merge([p, view]),
      builder: (context, _) {
        final st = viewKey.currentState?.stats;
        final l = p.currentLayout;
        final anim = viewKey.currentState?.animating ?? false;
        final dpr = MediaQuery.devicePixelRatioOf(context);
        final text = 'engine        ${engine!.engineName} v${engine!.version} (dart:ffi), open ${initMs.toStringAsFixed(1)} ms${atlas != null ? ' · atlas loaded (${atlas!.pages} pages)' : ''}\n'
            'page ${_pad(n)}      ${(bytes / 1024).toStringAsFixed(0)} KB, load+decode ${loadMs.toStringAsFixed(2)} ms\n'
            'content       ${p.nWords} words · ${p.nPaths} paths · ${p.nAyahs} ayah fragments · ${p.nLines} lines\n'
            'base layer    ${st?.basePaths ?? 0} paths in ${(st?.baseMs ?? 0).toStringAsFixed(2)} ms (cached image)\n'
            'overlay       ${st?.overlayPaths ?? 0} styled paths + ${st?.bands ?? 0} band boxes in ${(st?.overlayMs ?? 0).toStringAsFixed(2)} ms\n'
            'hit-test      gap-aware, engine (last distance ${lastHitUs.toStringAsFixed(2)} u)\n'
            'styles        ${p.styleHandles().length} handles · ${p.highlightHandles().length} highlights${anim ? ' · animating' : ''}\n'
            'layout        ${layout.fillHeight ? 'fill height' : layout.lineGap != 0 ? 'gap +${layout.lineGap.toStringAsFixed(1)} u' : 'spacing ×${layout.lineSpacing.toStringAsFixed(2)}'} · pitch ${(l?.pitch ?? 0).toStringAsFixed(1)} u · pad ${layout.padTop.round()}/${layout.padBottom.round()}\n'
            'zoom          ${(view.scale * (l?.scale ?? 1) * dpr).toStringAsFixed(2)}× device px per unit';
        return Text(text, style: TextStyle(fontFamily: 'monospace', fontSize: 11, height: 1.6, color: pal.muted));
      },
    );
  }
}
