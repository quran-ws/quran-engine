// QVP web wrapper — the reference thin wrapper over the C ABI (qvp_ffi.wasm).
// Host renderer: Canvas2D. Everything that decides (hit-testing, layout, styles,
// highlights, masks, search, text) lives in the engine; this file only marshals
// and draws. API shape is mirrored by the Kotlin, Dart and React Native wrappers.
(function (global) {
  'use strict';

  const NONE = 0xffffffff;
  /** The defaults every wrapper shares (QVP_DEFAULT_* in qvp.h; the parity check compares them). */
  const DEFAULTS = { INK: 0x231f20ff, HIGHLIGHT_INK: 0x1a73e8ff, HIGHLIGHT_BAND: 0xd6a3264d, HIGHLIGHT_PAD_X: 1.2, HIGHLIGHT_PAD_Y: 0, HIGHLIGHT_SEAM: 0.25,
    SELECTION_BAND: 0x2d6fd640, GAP_BIAS: 0.6, TAP_DISTANCE: 6, GRID_LINES: 15, ASPECT_SLACK: 1.15, MASK_BLOCK: 0xd9d4c8ff, MASK_PAD: 0.6, MASK_RADIUS: 0.8,
    REVEAL_LIT: 1, REVEAL_GREY: 0xc9c4b8ff, CROP_PAD: 2 };
  const KIND = { BODY: 0, MARK: 1, AYAH_NUMBER: 2, AYAH_MARK_ORNAMENT: 3, HEADER_INK: 4, ORNAMENT: 5, PAGE_NUMBER: 6, RUNNING_HEAD: 7, OTHER: 255 };
  const FAMILY = { NONE: 0, DIACRITIC: 1, TANWIN: 2, DOTS: 3, WAQF: 4, SIFR: 5, SAJDAH: 6, READING_SIGN: 7 };
  const CATEGORY = { NONE: 0, HARAKAH: 1, TANWIN: 2, LETTER_DOT: 3, ORTHOGRAPHIC: 4, DABT: 5, WAQF: 6, READING_SIGN: 7, STANDALONE: 8 };
  const DECORATION = { AYAH_MARK: 0, SURAH_NAME: 1, BASMALAH: 2, DIVISION_MARK: 3, SAJDAH_MARK: 4, PAGE_NUMBER: 5, RUNNING_HEAD: 6, OTHER: 255 };
  const FORM = { rasm_uthmani: 0, rasm_imlai: 1, qpc: 2, rasm: 3, search: 4 };
  const LAYER = { BASE: 0, THEME: 10, HIGHLIGHT: 50, SELECTION: 60, TOP: 100 };
  // The engine's name tables (QVP_NAMES_*), read from the engine when it is initialised.
  const NAMES_TABLE = { mark: 0, kind: 1, family: 2, category: 3, decoration: 4, division: 5, place: 6 };
  let NAMES = null;
  const names = table => { if (!NAMES) throw new Error('QvpEngine.init() first: names come from the engine'); return NAMES[table]; };
  const idOf = (table, v) => { if (typeof v !== 'string') return v; const i = names(table).indexOf(v); return i < 0 ? 255 : i; };

  // colours: numbers are 0xRRGGBBAA; strings '#rgb', '#rrggbb', '#rrggbbaa'
  function rgba(c, alpha) {
    if (typeof c === 'number') return c >>> 0;
    if (typeof c !== 'string') return 0;
    let h = c.trim().replace('#', '');
    if (h.length === 3 || h.length === 4) h = [...h].map(x => x + x).join('');
    if (h.length === 6) h += alpha !== undefined ? Math.round(alpha * 255).toString(16).padStart(2, '0') : 'ff';
    return parseInt(h, 16) >>> 0;
  }
  const css = c => `rgba(${(c >>> 24) & 255},${(c >>> 16) & 255},${(c >>> 8) & 255},${(c & 255) / 255})`;

  // Selectors (what a style rule applies to)
  const Sel = {
    page: () => ({ selector: 0 }),
    path: i => ({ selector: 1, a: i }),
    wordPath: (w, n) => ({ selector: 2, a: w, b: n }),
    wordMark: (w, n) => ({ selector: 3, a: w, b: n }),                    // nth mark of the word (0-based)
    wordMarkNamed: (w, mark, n = 0) => ({ selector: 4, a: w, b: markId(mark), c: n }),
    wordBody: w => ({ selector: 5, a: w }),
    wordMarks: w => ({ selector: 6, a: w }),
    word: w => ({ selector: 7, a: w }),
    ayah: (s, a) => ({ selector: 8, a: s, b: a }),
    line: n => ({ selector: 9, a: n }),
    mark: m => ({ selector: 10, a: markId(m) }),
    category: c => ({ selector: 11, a: idOf('category', c) }),
    family: f => ({ selector: 12, a: idOf('family', f) }),
    kind: k => ({ selector: 13, a: idOf('kind', k) }),
    decoration: k => ({ selector: 14, a: idOf('decoration', k) }),
    decorationIndex: i => ({ selector: 15, a: i }),
  };
  const markId = m => idOf('mark', m);

  // Targets (what resolves to a word list). Strings: 'page', '2:255', '2:255:3', '2:255-257', 'line:7', 'surah:2'
  const T = {
    page: () => ({ target: 0 }),
    word: i => ({ target: 1, a: i }),
    words: ws => ({ target: 2, words: ws }),
    ayah: (s, a) => ({ target: 3, a: s, b: a }),
    ayahRange: (s, a, b) => ({ target: 4, a: s, b: a, c: b }),
    line: n => ({ target: 5, a: n }),
    surah: s => ({ target: 6, a: s }),
    range: (a, b) => ({ target: 7, a, b }),
  };

  class QvpEngine {
    static async init(wasmBytes) {
      const { instance } = await WebAssembly.instantiate(wasmBytes, {});
      return new QvpEngine(instance);
    }
    constructor(instance) {
      this.ex = instance.exports;
      this.mem = this.ex.memory;
      this.scratch = this.ex.qvp_alloc(1 << 16);   // 64 KB for outputs
      this.scratch2 = this.ex.qvp_alloc(1 << 16);  // 64 KB for inputs
      this.td = new TextDecoder();
      this.te = new TextEncoder();
      // Load every name table from the engine once; Sel and markId read them.
      NAMES = Object.fromEntries(Object.entries(NAMES_TABLE).map(([k, t]) => [k, this.names(t)]));
    }
    /** All names of a table (QVP_NAMES_* id or key), index = id. */
    names(table) { const t = typeof table === 'string' ? NAMES_TABLE[table] : table; const n = this.ex.qvp_name_count(t); return Array.from({ length: n }, (_, i) => this.name(t, i)); }
    nameCount(table) { return this.ex.qvp_name_count(typeof table === 'string' ? NAMES_TABLE[table] : table); }
    name(table, id) { this.ex.qvp_name(typeof table === 'string' ? NAMES_TABLE[table] : table, id, this.scratch); return this.qstr(this.scratch); }
    nameId(table, s) { const [p, n] = this.putStr(s); return this.ex.qvp_name_id(typeof table === 'string' ? NAMES_TABLE[table] : table, p, n); }
    dv() { return new DataView(this.mem.buffer); }
    str(ptr, len) { return this.td.decode(new Uint8Array(this.mem.buffer, ptr, len)); }
    qstr(at) { const d = this.dv(); return this.str(d.getUint32(at, true), d.getUint32(at + 4, true)); }
    /** write a JS string into scratch2 at offset; returns [ptr, len] */
    putStr(s, off = 0) { const b = this.te.encode(s); new Uint8Array(this.mem.buffer, this.scratch2 + off, b.length).set(b); return [this.scratch2 + off, b.length]; }
    putU32(arr, off = 0) { const p = this.scratch2 + off; new Uint32Array(this.mem.buffer, p, arr.length).set(arr); return p; }
    nameOf(fn, v) { this.ex[fn](v, this.scratch); return this.qstr(this.scratch); }
    markName(m) { return this.nameOf('qvp_mark_name', m); }
    familyName(f) { return this.nameOf('qvp_family_name', f); }
    kindName(k) { return this.nameOf('qvp_kind_name', k); }
    categoryName(c) { return this.nameOf('qvp_category_name', c); }
    markCategory(m) { return this.ex.qvp_mark_category(markId(m)); }
    markFromName(s) { const [p, n] = this.putStr(s); return this.ex.qvp_mark_from_name(p, n); }
    /** The page format version the engine reads. */
    version() { return this.ex.qvp_version(); }
    /** The engine's name, `qvp` (a NUL-terminated C string in wasm memory). */
    engineName() { const p = this.ex.qvp_engine_name(), u = new Uint8Array(this.mem.buffer); let n = 0; while (u[p + n]) n++; return this.str(p, n); }
    /** Arabic text tools */
    strip(s) { return this._arabic(0, s); }
    fold(s) { return this._arabic(1, s); }
    normalize(s) { return this._arabic(2, s); }
    looseKey(s) { return this._arabic(3, s); }
    _arabic(op, s) { const [p, n] = this.putStr(s); this.ex.qvp_arabic(op, p, n, this.scratch); return this.qstr(this.scratch); }
    loadPage(bytes) {
      const p = this.ex.qvp_alloc(bytes.length);
      new Uint8Array(this.mem.buffer, p, bytes.length).set(bytes);
      const h = this.ex.qvp_page_load(p, bytes.length);
      this.ex.qvp_dealloc(p, bytes.length);
      if (!h) throw new Error('qvp_page_load failed');
      return new QvpPage(this, h);
    }
    loadAtlas(bytes) {
      const p = this.ex.qvp_alloc(bytes.length);
      new Uint8Array(this.mem.buffer, p, bytes.length).set(bytes);
      const h = this.ex.qvp_atlas_load(p, bytes.length);
      this.ex.qvp_dealloc(p, bytes.length);
      if (!h) throw new Error('qvp_atlas_load failed');
      return new QvpAtlas(this, h);
    }
  }

  // struct writers (wasm32 layouts, see qvp.h)
  function writeTarget(e, at, t) {
    if (typeof t === 'string') t = parseTarget(t);
    const d = e.dv();
    d.setUint8(at, t.target); d.setUint32(at + 4, t.a >>> 0 || 0, true); d.setUint32(at + 8, t.b >>> 0 || 0, true); d.setUint32(at + 12, t.c >>> 0 || 0, true);
    let wp = 0, wn = 0;
    if (t.words) { wp = e.putU32(Uint32Array.from(t.words), 1024); wn = t.words.length; }
    d.setUint32(at + 16, wp, true); d.setUint32(at + 20, wn, true);
    return at;
  }
  function parseTarget(s) {
    if (s === 'page') return T.page();
    let m;
    if ((m = /^line:(\d+)$/.exec(s))) return T.line(+m[1]);
    if ((m = /^surah:(\d+)$/.exec(s))) return T.surah(+m[1]);
    if ((m = /^(\d+):(\d+)-(\d+)$/.exec(s))) return T.ayahRange(+m[1], +m[2], +m[3]);
    if ((m = /^(\d+):(\d+):(\d+)$/.exec(s))) return { target: 1, wordKey: [+m[1], +m[2], +m[3]] };
    if ((m = /^(\d+):(\d+)$/.exec(s))) return T.ayah(+m[1], +m[2]);
    throw new Error('bad target ' + s);
  }
  function writeSel(e, at, s) {
    const d = e.dv();
    d.setUint8(at, s.selector); d.setUint32(at + 4, s.a >>> 0 || 0, true); d.setUint32(at + 8, s.b >>> 0 || 0, true); d.setUint32(at + 12, s.c >>> 0 || 0, true);
    return at;
  }
  const HL_DEFAULT = { mode: 'band', height: 'lineSpacing', ink: DEFAULTS.HIGHLIGHT_INK, band: DEFAULTS.HIGHLIGHT_BAND, padX: DEFAULTS.HIGHLIGHT_PAD_X, padY: DEFAULTS.HIGHLIGHT_PAD_Y, radius: 0, seam: DEFAULTS.HIGHLIGHT_SEAM, ms: 0, layer: LAYER.HIGHLIGHT };
  function writeHl(e, at, st) {
    st = { ...HL_DEFAULT, ...st };
    const d = e.dv();
    d.setUint8(at, { ink: 0, band: 1, both: 2 }[st.mode] ?? 1); d.setUint8(at + 1, st.height === 'ink' ? 1 : 0);
    d.setUint32(at + 4, rgba(st.ink), true); d.setUint32(at + 8, rgba(st.band), true);
    d.setFloat32(at + 12, st.padX, true); d.setFloat32(at + 16, st.padY, true); d.setFloat32(at + 20, st.radius, true); d.setFloat32(at + 24, st.seam, true);
    d.setUint32(at + 28, st.ms, true); d.setInt32(at + 32, st.layer, true);
    return at;
  }
  function readBoxes(e, at, n) {
    const d = e.dv(), out = new Array(n);
    for (let i = 0; i < n; i++) { const o = at + i * 32; out[i] = { id: d.getUint32(o, true), line: d.getUint32(o + 4, true), x0: d.getFloat32(o + 8, true), y0: d.getFloat32(o + 12, true), x1: d.getFloat32(o + 16, true), y1: d.getFloat32(o + 20, true), color: d.getUint32(o + 24, true), radius: d.getFloat32(o + 28, true) }; }
    return out;
  }

  class QvpPage {
    constructor(engine, handle) {
      this.e = engine; this.h = handle;
      const ex = engine.ex, s = engine.scratch;
      ex.qvp_page_info(handle, s);
      let d = engine.dv();
      this.width = d.getFloat32(s, true); this.height = d.getFloat32(s + 4, true); this.page = d.getUint32(s + 8, true);
      this.nLines = d.getUint32(s + 12, true); this.nAyahs = d.getUint32(s + 16, true); this.nWords = d.getUint32(s + 20, true); this.nPaths = d.getUint32(s + 24, true); this.nDecorations = d.getUint32(s + 28, true);
      ex.qvp_geometry(handle, s);
      d = engine.dv();
      const opsPtr = d.getUint32(s, true), opsLen = d.getUint32(s + 4, true), ptsPtr = d.getUint32(s + 8, true), ptsLen = d.getUint32(s + 12, true), tabPtr = d.getUint32(s + 16, true), nPaths = d.getUint32(s + 20, true);
      this.ops = new Uint8Array(engine.mem.buffer, opsPtr, opsLen).slice();
      this.pts = new Float32Array(engine.mem.buffer.slice(ptsPtr, ptsPtr + ptsLen * 4));
      this.table = new Uint32Array(engine.mem.buffer.slice(tabPtr, tabPtr + nPaths * 32));
      this.paths = null;
      this.words = Array.from({ length: this.nWords }, (_, i) => this._word(i));
      this.ayahs = Array.from({ length: this.nAyahs }, (_, i) => this._ayah(i));
      this.lines = Array.from({ length: this.nLines }, (_, i) => this._line(i));
      this.decorations = Array.from({ length: this.nDecorations }, (_, i) => this._deco(i));
      this.lineSpacing = ex.qvp_page_line_spacing(handle);
      this.currentLayout = null;
      this._defaultInk = DEFAULTS.INK;
    }
    free() { this.e.ex.qvp_page_free(this.h); this.h = 0; }

    // ── geometry ──
    pathFlags(i) { return this.table[i * 8 + 4]; }
    pathKind(i) { return this.pathFlags(i) & 0xff; }
    pathMark(i) { return (this.pathFlags(i) >> 8) & 0xff; }
    pathFamily(i) { return (this.pathFlags(i) >> 16) & 0xff; }
    pathEvenOdd(i) { return ((this.pathFlags(i) >> 24) & 1) === 1; }
    pathWord(i) { const w = this.table[i * 8 + 5]; return w === NONE ? -1 : w; }
    pathLine(i) { return this.table[i * 8 + 6]; }
    pathCategory(i) { return this.table[i * 8 + 7] & 0xff; }
    pathNthInWord(i) { return (this.table[i * 8 + 7] >> 8) & 0xff; }
    pathNthMark(i) { const n = (this.table[i * 8 + 7] >> 16) & 0xff; return n === 0xff ? -1 : n; }
    buildPaths() {
      if (this.paths) return this.paths;
      const t = this.table, ops = this.ops, pts = this.pts, out = new Array(this.nPaths);
      for (let i = 0; i < this.nPaths; i++) {
        const p = new Path2D();
        let o = t[i * 8], oe = o + t[i * 8 + 1], k = t[i * 8 + 2];
        for (; o < oe; o++) {
          switch (ops[o]) {
            case 0: p.moveTo(pts[k], pts[k + 1]); k += 2; break;
            case 1: p.lineTo(pts[k], pts[k + 1]); k += 2; break;
            case 2: p.quadraticCurveTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3]); k += 4; break;
            case 3: p.bezierCurveTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3], pts[k + 4], pts[k + 5]); k += 6; break;
            case 4: p.closePath(); break;
          }
        }
        out[i] = p;
      }
      return (this.paths = out);
    }

    // ── info records ──
    _word(i) {
      const ex = this.e.ex, s = this.e.scratch; if (!ex.qvp_word_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { index: i, surah: d.getUint16(s, true), ayah: d.getUint16(s + 2, true), word: d.getUint16(s + 4, true), line: d.getUint16(s + 6, true), ayahIndex: d.getUint32(s + 8, true), lineIndex: d.getUint32(s + 12, true),
        x0: d.getFloat32(s + 16, true), y0: d.getFloat32(s + 20, true), x1: d.getFloat32(s + 24, true), y1: d.getFloat32(s + 28, true), text: this.e.qstr(s + 32), firstPath: d.getUint32(s + 40, true), nPaths: d.getUint32(s + 44, true) };
    }
    _ayah(i) {
      const ex = this.e.ex, s = this.e.scratch; if (!ex.qvp_ayah_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { index: i, surah: d.getUint16(s, true), ayah: d.getUint16(s + 2, true), fragment: d.getUint8(s + 4), fragments: d.getUint8(s + 5), flags: d.getUint8(s + 6), rubuAlHizb: d.getUint16(s + 8, true),
        firstWord: d.getUint32(s + 12, true), nWords: d.getUint32(s + 16, true), ayahMarkDecoration: d.getUint32(s + 20, true), x0: d.getFloat32(s + 24, true), y0: d.getFloat32(s + 28, true), x1: d.getFloat32(s + 32, true), y1: d.getFloat32(s + 36, true) };
    }
    _line(i) {
      const ex = this.e.ex, s = this.e.scratch; if (!ex.qvp_line_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { index: i, lineNumber: d.getUint8(s), isHeader: !!d.getUint8(s + 1), firstWord: d.getUint32(s + 4, true), nWords: d.getUint32(s + 8, true), x0: d.getFloat32(s + 12, true), y0: d.getFloat32(s + 16, true), x1: d.getFloat32(s + 20, true), y1: d.getFloat32(s + 24, true),
        bandY0: d.getFloat32(s + 28, true), bandY1: d.getFloat32(s + 32, true), centre: d.getFloat32(s + 36, true) };
    }
    _deco(i) {
      const ex = this.e.ex, s = this.e.scratch; if (!ex.qvp_decoration_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { index: i, decoration: d.getUint8(s), surah: d.getUint16(s + 2, true), ayah: d.getUint16(s + 4, true), line: d.getUint32(s + 8, true), x0: d.getFloat32(s + 12, true), y0: d.getFloat32(s + 16, true), x1: d.getFloat32(s + 20, true), y1: d.getFloat32(s + 24, true),
        text: this.e.qstr(s + 28), firstPath: d.getUint32(s + 36, true), nPaths: d.getUint32(s + 40, true) };
    }
    wordForm(i, form = 'rasm_uthmani') { const s = this.e.scratch; if (!this.e.ex.qvp_word_form(this.h, i, FORM[form] ?? 0, s)) return ''; return this.e.qstr(s); }
    findWord(surah, ayah, word) { const i = this.e.ex.qvp_find_word(this.h, surah, ayah, word); return i < 0 ? -1 : i; }
    _target(t) {
      if (typeof t === 'string') t = parseTarget(t);
      if (t.wordKey) { const i = this.findWord(...t.wordKey); t = i >= 0 ? T.word(i) : T.words([]); }
      if (t instanceof Array) t = T.words(t);
      return writeTarget(this.e, this.e.scratch2 + 4096, t);
    }
    targetWords(t) { const n = this.e.ex.qvp_target_words(this.h, this._target(t), this.e.scratch, 16384); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 16384))); }

    // ── metadata ──
    surahs() {
      const ex = this.e.ex, s = this.e.scratch, n = ex.qvp_surah_count(this.h), out = [];
      for (let i = 0; i < n; i++) { ex.qvp_surah_at(this.h, i, s); const d = this.e.dv();
        out.push({ number: d.getUint16(s, true), ayahCount: d.getUint16(s + 2, true), hasBanner: !!d.getUint8(s + 4), hasBasmalah: !!d.getUint8(s + 5), place: names('place')[d.getUint8(s + 6)] || '', bannerDecoration: d.getUint32(s + 8, true), arabic: this.e.qstr(s + 12), latin: this.e.qstr(s + 20), english: this.e.qstr(s + 28) }); }
      return out;
    }
    divisions() {
      const ex = this.e.ex, s = this.e.scratch, n = ex.qvp_divisions(this.h, s, 64), d = this.e.dv(), out = [];
      for (let i = 0; i < Math.min(n, 64); i++) { const o = s + i * 12; out.push({ division: names('division')[d.getUint8(o)], line: d.getUint8(o + 1), number: d.getUint16(o + 2, true), surah: d.getUint16(o + 4, true), ayah: d.getUint16(o + 6, true), ayahIndex: d.getUint32(o + 8, true) }); }
      return out;
    }
    ayahMarks() {
      const ex = this.e.ex, s = this.e.scratch, n = ex.qvp_ayah_marks(this.h, s, 128), d = this.e.dv(), out = [];
      for (let i = 0; i < Math.min(n, 128); i++) { const o = s + i * 32; out.push({ decoration: d.getUint32(o, true), surah: d.getUint16(o + 4, true), ayah: d.getUint16(o + 6, true), line: d.getUint32(o + 8, true), cx: d.getFloat32(o + 12, true), cy: d.getFloat32(o + 16, true), r: d.getFloat32(o + 20, true), ornamentPath: d.getUint32(o + 24, true), numeralPath: d.getUint32(o + 28, true) }); }
      return out;
    }
    rosettes() {
      const ex = this.e.ex, s = this.e.scratch, n = ex.qvp_rosettes(this.h, s, 32), d = this.e.dv(), out = [];
      for (let i = 0; i < Math.min(n, 32); i++) { const o = s + i * 20; out.push({ decoration: d.getUint32(o, true), surah: d.getUint16(o + 4, true), ayah: d.getUint16(o + 6, true), juz: d.getUint16(o + 8, true), hizb: d.getUint16(o + 10, true), nisf: d.getUint16(o + 12, true), rubuAlHizb: d.getUint16(o + 14, true), rubuAlHizbInHizb: d.getUint16(o + 16, true) }); }
      return out;
    }
    sajdahs() {
      const ex = this.e.ex, s = this.e.scratch, n = ex.qvp_sajdahs(this.h, s, 16), d = this.e.dv(), out = [];
      for (let i = 0; i < Math.min(n, 16); i++) { const o = s + i * 12; out.push({ decoration: d.getUint32(o, true), surah: d.getUint16(o + 4, true), ayah: d.getUint16(o + 6, true), signPath: d.getUint32(o + 8, true) }); }
      return out;
    }
    ayahKeys() { const n = this.e.ex.qvp_ayah_keys(this.h, this.e.scratch, 256); const v = new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 256)); return Array.from(v, k => [k >>> 16, k & 0xffff]); }
    ayahWordCount(surah, ayah) { const n = this.e.ex.qvp_ayah_word_count(this.h, surah, ayah, this.e.scratch); return { count: n, complete: !!this.e.dv().getUint32(this.e.scratch, true) }; }
    /** words for n recitation segments, or null when the counts disagree (follow the ayah whole) */
    reciteMap(surah, ayah, nSegments) { const n = this.e.ex.qvp_recite_map(this.h, surah, ayah, nSegments, this.e.scratch, 4096); return n < 0 ? null : Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, n)); }
    wordLabel(i) { this.e.ex.qvp_word_label(this.h, i, this.e.scratch); return this.e.qstr(this.e.scratch); }
    ayahLabel(i) { this.e.ex.qvp_ayah_label(this.h, i, this.e.scratch); return this.e.qstr(this.e.scratch); }

    // ── text & search ──
    text(target = 'page', { form = 'rasm_uthmani', wordSep = ' ', lineSep = '\n' } = {}) {
      const [wp, wn] = this.e.putStr(wordSep, 0), [lp, ln] = this.e.putStr(lineSep, 256);
      this.e.ex.qvp_text(this.h, this._target(target), FORM[form] ?? 0, wp, wn, lp, ln, this.e.scratch);
      return this.e.qstr(this.e.scratch);
    }
    search(query, { form = 'search', mode = 'includes', normalize = true, looseMatch = true, limit = 0 } = {}) {
      const [qp, qn] = this.e.putStr(query);
      const n = this.e.ex.qvp_search(this.h, qp, qn, FORM[form] ?? 4, { includes: 0, exact: 1, prefix: 2 }[mode] ?? 0, normalize ? 1 : 0, looseMatch ? 1 : 0, limit, this.e.scratch, 1024);
      const d = this.e.dv(), out = [];
      for (let i = 0; i < Math.min(n, 1024); i++) { const o = this.e.scratch + i * 12; const w = d.getUint32(o, true); out.push({ word: w, index: d.getUint32(o + 4, true), isLooseMatch: !!d.getUint32(o + 8, true), wordKey: this.wordKey(w), text: this.words[w].text }); }
      return out;
    }
    wordKey(i) { const w = this.words[i]; return `${w.surah}:${w.ayah}:${w.word}`; }
    /** attach a words sidecar (object or JSON string/bytes); returns words updated */
    attachWords(sidecar) { const s = typeof sidecar === 'string' ? sidecar : sidecar instanceof Uint8Array ? this.e.td.decode(sidecar) : JSON.stringify(sidecar); const b = this.e.te.encode(s); const p = this.e.ex.qvp_alloc(b.length); new Uint8Array(this.e.mem.buffer, p, b.length).set(b); const n = this.e.ex.qvp_attach_words(this.h, p, b.length); this.e.ex.qvp_dealloc(p, b.length); return n; }
    hasForm(form) { return !!this.e.ex.qvp_has_form(this.h, FORM[form] ?? 0); }
    citation(words) { const p = this.e.putU32(Uint32Array.from(words)); this.e.ex.qvp_citation(this.h, p, words.length, this.e.scratch); return this.e.qstr(this.e.scratch); }

    // ── hit testing ──
    hitTestExact(x, y) { return this.e.ex.qvp_hit_test_exact(this.h, x, y, this.e.scratch) ? this._hit(this.e.scratch) : null; }
    hitTestExactView(viewX, viewY) { return this.e.ex.qvp_hit_test_exact_view(this.h, viewX, viewY, this.e.scratch) ? this._hit(this.e.scratch) : null; }
    _hitOpt(o) { const d = this.e.dv(), at = this.e.scratch2 + 8192; d.setFloat32(at, o.maxDistance ?? 0, true); d.setFloat32(at + 4, o.gapBias ?? 0.6, true); d.setUint32(at + 8, o.preferExact === false ? 0 : 1, true); return at; }
    /** the one hit shape for every hit test; the exact variants report distance 0 and isExact */
    _hit(s) { const d = this.e.dv(); const f = v => (v === NONE ? -1 : v); const h = { word: f(d.getUint32(s, true)), path: f(d.getUint32(s + 4, true)), decoration: f(d.getUint32(s + 8, true)), line: f(d.getUint32(s + 12, true)), distance: d.getFloat32(s + 16, true), isExact: !!d.getUint32(s + 20, true) }; if (h.word >= 0) { h.wordKey = this.wordKey(h.word); const w = this.words[h.word]; h.ayahKey = `${w.surah}:${w.ayah}`; } return h; }
    /** gap-aware: every point on a printed line resolves to the word the user meant */
    hitTest(x, y, opt = {}) { return this.e.ex.qvp_hit_test(this.h, x, y, this._hitOpt(opt), this.e.scratch) ? this._hit(this.e.scratch) : null; }
    hitTestView(viewX, viewY, opt = {}) { return this.e.ex.qvp_hit_test_view(this.h, viewX, viewY, this._hitOpt(opt), this.e.scratch) ? this._hit(this.e.scratch) : null; }
    lineBands() { const n = this.e.ex.qvp_line_bands(this.h, this.e.scratch, 64), d = this.e.dv(), out = []; for (let i = 0; i < Math.min(n, 64); i++) { const o = this.e.scratch + i * 28; out.push({ line: d.getUint32(o, true), lineNumber: d.getUint32(o + 4, true), y0: d.getFloat32(o + 8, true), y1: d.getFloat32(o + 12, true), mid: d.getFloat32(o + 16, true), inkY0: d.getFloat32(o + 20, true), inkY1: d.getFloat32(o + 24, true) }); } return out; }
    hitAreas(gapBias = DEFAULTS.GAP_BIAS) { const n = this.e.ex.qvp_hit_areas(this.h, gapBias, this.e.scratch, 1024), d = this.e.dv(), out = []; for (let i = 0; i < Math.min(n, 1024); i++) { const o = this.e.scratch + i * 40; out.push({ word: d.getUint32(o, true), line: d.getUint32(o + 4, true), x0: d.getFloat32(o + 8, true), y0: d.getFloat32(o + 12, true), x1: d.getFloat32(o + 16, true), y1: d.getFloat32(o + 20, true), inkX0: d.getFloat32(o + 24, true), inkY0: d.getFloat32(o + 28, true), inkX1: d.getFloat32(o + 32, true), inkY1: d.getFloat32(o + 36, true) }); } return out; }

    // ── layout ──
    /** Write a layout spec (QvpLayoutSpec, 52 bytes) at scratch offset `s`. */
    _writeLayoutSpec(spec, s) {
      const d = this.e.dv();
      d.setFloat32(s, spec.viewportW, true); d.setFloat32(s + 4, spec.viewportH, true); d.setFloat32(s + 8, spec.padTop || 0, true); d.setFloat32(s + 12, spec.padBottom || 0, true);
      d.setFloat32(s + 16, spec.padLeft || 0, true); d.setFloat32(s + 20, spec.padRight || 0, true); d.setFloat32(s + 24, spec.lineSpacing ?? 1, true);
      d.setUint32(s + 28, spec.fillHeight ? 1 : 0, true); d.setUint32(s + 32, spec.gridLines || 0, true);
      d.setFloat32(s + 36, spec.cropLeft || 0, true); d.setFloat32(s + 40, spec.cropRight || 0, true); d.setFloat32(s + 44, spec.maxAspectSlack || 0, true);
    }
    /** Leading (page units) that makes the page fill the padded viewport of `spec`; max 0 = unlimited. */
    layoutLineSpacingToFill(spec, max = 0) { const s = this.e.scratch; this._writeLayoutSpec(spec, s); return this.e.ex.qvp_layout_line_spacing_to_fill(this.h, s, max); }
    /** the share of the padded viewport left empty when the page is fitted to width */
    layoutWastedFraction(spec) { const s = this.e.scratch; this._writeLayoutSpec(spec, s); return this.e.ex.qvp_layout_wasted_fraction(this.h, s); }
    /** the grid this page is laid out inside: the mushaf's line count and the printed line spacing */
    get grid() { this.e.ex.qvp_page_grid(this.h, this.e.scratch); const d = this.e.dv(); return { lines: d.getUint32(this.e.scratch, true), lineSpacing: d.getFloat32(this.e.scratch + 4, true) }; }
    layout(spec) {
      const ex = this.e.ex, s = this.e.scratch; let d = this.e.dv();
      this._writeLayoutSpec(spec, s);
      ex.qvp_layout(this.h, s, s + 64);
      d = this.e.dv(); const o = s + 64;
      const n = d.getUint32(o + 24, true), lp = d.getUint32(o + 28, true);
      const f = new Float32Array(this.e.mem.buffer.slice(lp, lp + n * 12));
      const lineDy = new Float32Array(n), slots = new Array(n);
      for (let i = 0; i < n; i++) { lineDy[i] = f[i * 3]; slots[i] = [f[i * 3 + 1], f[i * 3 + 2]]; }
      return (this.currentLayout = { scale: d.getFloat32(o, true), offsetX: d.getFloat32(o + 4, true), offsetY: d.getFloat32(o + 8, true), contentW: d.getFloat32(o + 12, true), contentH: d.getFloat32(o + 16, true), lineSpacing: d.getFloat32(o + 20, true), lineDy, slots,
        fitScale: d.getFloat32(o + 32, true), fitX: d.getFloat32(o + 36, true), fitY: d.getFloat32(o + 40, true) });
    }
    wordBoundsView(i) { this.e.ex.qvp_word_bounds_view(this.h, i, this.e.scratch); const f = new Float32Array(this.e.mem.buffer, this.e.scratch, 4); return { x0: f[0], y0: f[1], x1: f[2], y1: f[3] }; }

    // ── styles (handles undo exactly) ──
    style(sel, color, { ms = 0, layer = LAYER.BASE } = {}) { return this.e.ex.qvp_style_add(this.h, layer, writeSel(this.e, this.e.scratch2 + 12288, sel), rgba(color), ms); }
    styleTarget(target, color, { ms = 0, layer = LAYER.BASE } = {}) { return this.e.ex.qvp_style_add_target(this.h, layer, this._target(target), rgba(color), ms); }
    removeStyle(handle) { return this.e.ex.qvp_style_remove(this.h, handle); }
    recolorStyle(handle, color, ms = 0) { return this.e.ex.qvp_style_recolor(this.h, handle, rgba(color), ms); }
    hide(sel) { return this.e.ex.qvp_style_hide(this.h, writeSel(this.e, this.e.scratch2 + 12288, sel)); }
    clearStyles() { this.e.ex.qvp_style_clear(this.h); }
    clearLayer(layer) { this.e.ex.qvp_style_clear_layer(this.h, layer); }
    setDefaultColor(color) { this._defaultInk = rgba(color); this.e.ex.qvp_style_default_color(this.h, this._defaultInk); }
    get defaultInk() { return this._defaultInk; }
    /** {ink, diacritics, dots, waqf, sifr, ayahMark, numeral, headers, marks: {name: colour}, ms} → handle */
    theme(t) {
      const d = this.e.dv(), at = this.e.scratch2 + 16384, c = v => (v === undefined || v === null ? 0 : rgba(v));
      ['ink', 'diacritics', 'dots', 'waqf', 'sifr', 'ayahMark', 'numeral', 'headers'].forEach((k, i) => d.setUint32(at + i * 4, c(t[k]), true));
      d.setUint32(at + 32, t.ms || 0, true);
      const pairs = Object.entries(t.marks || {}).flatMap(([m, col]) => [markId(m), rgba(col)]);
      const mp = pairs.length ? this.e.putU32(Uint32Array.from(pairs), 20480) : 0;
      d.setUint32(at + 36, mp, true); d.setUint32(at + 40, pairs.length / 2, true);
      return this.e.ex.qvp_theme(this.h, at);
    }
    styleHandles() { const n = this.e.ex.qvp_style_handles(this.h, this.e.scratch, 4096); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 4096))); }

    // ── clock & display list ──
    /** advance animations; returns true while something is still moving */
    tick(nowMs) { return !!this.e.ex.qvp_tick(this.h, nowMs); }
    colors() { const p = this.e.ex.qvp_colors(this.h); return new Uint32Array(this.e.mem.buffer.slice(p, p + this.nPaths * 4)); }
    styledPaths() {
      const ex = this.e.ex; const n = ex.qvp_styled_paths(this.h, 0, 0); if (!n) return [];
      const buf = ex.qvp_alloc(n * 8); ex.qvp_styled_paths(this.h, buf, n);
      const v = new Uint32Array(this.e.mem.buffer.slice(buf, buf + n * 8)); ex.qvp_dealloc(buf, n * 8);
      const out = new Array(n); for (let i = 0; i < n; i++) out[i] = [v[i * 2], v[i * 2 + 1]]; return out;
    }
    colorOf(i) { return this.e.ex.qvp_color_of(this.h, i) >>> 0; }

    // ── highlights ──
    /** style: {mode:'ink'|'band'|'both', height:'lineSpacing'|'ink', ink, band, padX, padY, radius, seam, ms, layer} */
    highlight(target, style = {}) { return this.e.ex.qvp_highlight_add(this.h, this._target(target), writeHl(this.e, this.e.scratch2 + 24576, style)); }
    moveHighlight(handle, target) { return !!this.e.ex.qvp_highlight_move(this.h, handle, this._target(target)); }
    restyleHighlight(handle, style) { return !!this.e.ex.qvp_highlight_restyle(this.h, handle, writeHl(this.e, this.e.scratch2 + 24576, style)); }
    removeHighlight(handle) { return !!this.e.ex.qvp_highlight_remove(this.h, handle); }
    clearHighlights() { this.e.ex.qvp_highlight_clear(this.h); }
    highlightHandles() { const n = this.e.ex.qvp_highlight_handles(this.h, this.e.scratch, 1024); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 1024))); }
    highlightWords(h) { const n = this.e.ex.qvp_highlight_words(this.h, h, this.e.scratch, 4096); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 4096))); }
    /** animated band boxes in viewport px; draw each id as one nonzero path behind the ink */
    highlightBoxesView() { const n = this.e.ex.qvp_highlight_boxes_view(this.h, this.e.scratch, 1024); return readBoxes(this.e, this.e.scratch, Math.min(n, 1024)); }
    wordBands(words, { height = 'lineSpacing', padX = DEFAULTS.HIGHLIGHT_PAD_X, padY = DEFAULTS.HIGHLIGHT_PAD_Y } = {}) { const p = this.e.putU32(Uint32Array.from(words)); const n = this.e.ex.qvp_word_bands(this.h, p, words.length, height === 'ink' ? 1 : 0, padX, padY, this.e.scratch, 64); return readBoxes(this.e, this.e.scratch, Math.min(n, 64)); }

    // ── selection ──
    select(anchor, focus = anchor) { this.e.ex.qvp_select(this.h, anchor < 0 ? NONE : anchor, focus < 0 ? NONE : focus); }
    clearSelection() { this.e.ex.qvp_select(this.h, NONE, NONE); }
    selection() { const n = this.e.ex.qvp_selection(this.h, this.e.scratch, 4096); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 4096))); }
    selectionText(form = 'rasm_uthmani', citation = false) { this.e.ex.qvp_selection_text(this.h, FORM[form] ?? 0, citation ? 1 : 0, this.e.scratch); return this.e.qstr(this.e.scratch); }

    // ── memorisation ──
    mask(target, mode = 'hide') { this.e.ex.qvp_mask(this.h, this._target(target), { hide: 0, block: 1, blur: 2 }[mode] ?? 0); }
    maskFrom(i, mode = 'hide') { this.e.ex.qvp_mask_from(this.h, i, { hide: 0, block: 1, blur: 2 }[mode] ?? 0); }
    maskOptions({ blockColor = DEFAULTS.MASK_BLOCK, padX = DEFAULTS.MASK_PAD, padY = DEFAULTS.MASK_PAD, radius = DEFAULTS.MASK_RADIUS, reverse = false } = {}) { this.e.ex.qvp_mask_options(this.h, rgba(blockColor), padX, padY, radius, reverse ? 1 : 0); }
    unmaskNext(n = 1) { return this.e.ex.qvp_unmask_next(this.h, n); }
    maskBack(n = 1) { return this.e.ex.qvp_mask_back(this.h, n); }
    unmaskWord(i) { return !!this.e.ex.qvp_unmask_word(this.h, i); }
    maskWord(i) { return !!this.e.ex.qvp_mask_word(this.h, i); }
    unmaskAll() { this.e.ex.qvp_unmask_all(this.h); }
    maskAll() { this.e.ex.qvp_mask_all(this.h); }
    unmask() { this.e.ex.qvp_unmask(this.h); }
    maskHidden() { const n = this.e.ex.qvp_mask_hidden(this.h, this.e.scratch, 4096); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 4096))); }
    maskWords() { const n = this.e.ex.qvp_mask_words(this.h, this.e.scratch, 4096); return Array.from(new Uint32Array(this.e.mem.buffer, this.e.scratch, Math.min(n, 4096))); }
    maskBoxesView() { const n = this.e.ex.qvp_mask_boxes_view(this.h, this.e.scratch, 1024); return readBoxes(this.e, this.e.scratch, Math.min(n, 1024)); }
    /** greyed page with a lit window: {lit, byAyah, grey, ink, ayahMarks, ms} → steps */
    revealStart({ lit = DEFAULTS.REVEAL_LIT, byAyah = false, grey = DEFAULTS.REVEAL_GREY, ink = DEFAULTS.INK, ayahMarks = true, ms = 0 } = {}) { return this.e.ex.qvp_reveal_start(this.h, lit, byAyah ? 1 : 0, rgba(grey), rgba(ink), ayahMarks ? 1 : 0, ms); }
    revealGoto(at) { return !!this.e.ex.qvp_reveal_goto(this.h, BigInt(at)); }
    revealPosition() { const v = Number(this.e.ex.qvp_reveal_position(this.h)); return v === -2 ? null : v; }
    revealStepCount() { return this.e.ex.qvp_reveal_step_count(this.h); }
    revealStepOf(i) { return Number(this.e.ex.qvp_reveal_step_of(this.h, i)); }
    revealStop() { this.e.ex.qvp_reveal_stop(this.h); }

    // ── crop ──
    cropBounds(target, { pad = DEFAULTS.CROP_PAD, keepAyahMarks = true } = {}) { if (!this.e.ex.qvp_crop_bounds(this.h, this._target(target), pad, keepAyahMarks ? 1 : 0, this.e.scratch)) return null; const d = this.e.dv(), s = this.e.scratch; return { x0: d.getFloat32(s, true), y0: d.getFloat32(s + 4, true), x1: d.getFloat32(s + 8, true), y1: d.getFloat32(s + 12, true), nWords: d.getUint32(s + 16, true), ayahMarkDecoration: d.getUint32(s + 20, true) }; }
    cropSvg(target, { pad = DEFAULTS.CROP_PAD, keepAyahMarks = true, background = null } = {}) { if (!this.e.ex.qvp_crop_svg(this.h, this._target(target), pad, keepAyahMarks ? 1 : 0, background ? rgba(background) : 0, this.e.scratch)) return null; return this.e.qstr(this.e.scratch); }
  }

  class QvpAtlas {
    constructor(engine, handle) { this.e = engine; this.h = handle; }
    free() { this.e.ex.qvp_atlas_free(this.h); this.h = 0; }
    pageOf(surah, ayah) { const p = this.e.ex.qvp_atlas_page_of(this.h, surah, ayah); return p < 0 ? null : p; }
    pageRange(page) { if (!this.e.ex.qvp_atlas_page_range(this.h, page, this.e.scratch)) return null; const v = new Uint16Array(this.e.mem.buffer, this.e.scratch, 4); return { first: [v[0], v[1]], last: [v[2], v[3]] }; }
    _surah(s) { const d = this.e.dv(); return { number: d.getUint16(s, true), page: d.getUint16(s + 2, true), ayahCount: d.getUint16(s + 4, true), place: names('place')[d.getUint8(s + 6)] || '', arabic: this.e.qstr(s + 8), latin: this.e.qstr(s + 16), english: this.e.qstr(s + 24) }; }
    surah(n) { return this.e.ex.qvp_atlas_surah(this.h, n, this.e.scratch) ? this._surah(this.e.scratch) : null; }
    surahs() { const n = this.e.ex.qvp_atlas_surah_count(this.h), out = []; for (let i = 0; i < n; i++) { this.e.ex.qvp_atlas_surah_at(this.h, i, this.e.scratch); out.push(this._surah(this.e.scratch)); } return out; }
    pageOfSurah(n) { const s = this.surah(n); return s ? s.page : null; }
    /** How many pages the atlas covers. */
    pageCount() { return this.e.ex.qvp_atlas_page_count(this.h); }
    /** The whole atlas as JSON text. */
    json() { this.e.ex.qvp_atlas_json(this.h, this.e.scratch); return this.e.qstr(this.e.scratch); }
    division(kind, n) { if (!this.e.ex.qvp_atlas_division(this.h, { juz: 0, hizb: 1, nisf: 2, rubu_al_hizb: 3 }[kind], n, this.e.scratch)) return null; const v = new Uint16Array(this.e.mem.buffer, this.e.scratch, 4); return { rubuAlHizb: v[0], surah: v[1], ayah: v[2], page: v[3], ayahKey: `${v[1]}:${v[2]}` }; }
    juz(n) { return this.division('juz', n); }
    hizb(n) { return this.division('hizb', n); }
    rubuAlHizb(n) { return this.division('rubu_al_hizb', n); }
    divisionOf(kind, surah, ayah) { const v = this.e.ex.qvp_atlas_division_of(this.h, { juz: 0, hizb: 1, nisf: 2, rubu_al_hizb: 3 }[kind], surah, ayah); return v < 0 ? null : v; }
    juzOf(surah, ayah) { return this.divisionOf('juz', surah, ayah); }
    pagesOfJuz(n) { if (!this.e.ex.qvp_atlas_pages_of_juz(this.h, n, this.e.scratch)) return null; const v = new Uint16Array(this.e.mem.buffer, this.e.scratch, 2); return [v[0], v[1]]; }
    searchSurahs(text) { const [p, n] = this.e.putStr(text); const c = this.e.ex.qvp_atlas_search_surahs(this.h, p, n, this.e.scratch, 128); return Array.from(new Uint16Array(this.e.mem.buffer, this.e.scratch, Math.min(c, 128))).map(k => this.surah(k)); }
  }

  /**
   * Canvas2D host renderer. Order: highlight bands → base ink (cached) → styled ink → mask boxes.
   * Call `page.tick(performance.now())` before `draw` each frame; keep drawing while it returns true.
   */
  class CanvasRenderer {
    constructor(canvas) {
      this.canvas = canvas; this.ctx = canvas.getContext('2d');
      this.base = document.createElement('canvas'); this.baseKey = '';
      this.stats = { baseMs: 0, overlayMs: 0, basePaths: 0, overlayPaths: 0, bands: 0 };
    }
    lineTransform(page, view, line, dpr) {
      const L = page.currentLayout || { scale: 1, offsetX: 0, offsetY: 0, lineDy: null };
      const s = dpr * view.scale * L.scale, dy = L.lineDy ? L.lineDy[line] : 0;
      return [s, dpr * (view.offsetX + view.scale * L.offsetX), dpr * (view.offsetY + view.scale * (L.offsetY + dy * L.scale))];
    }
    /** boxes are in layout viewport px; view adds pan/zoom on top */
    drawBoxes(c, boxes, view, dpr) {
      c.setTransform(dpr * view.scale, 0, 0, dpr * view.scale, dpr * view.offsetX, dpr * view.offsetY);
      let cur = null, col = 0;
      const flush = () => { if (cur) { c.fillStyle = css(col); c.fill(cur, 'nonzero'); cur = null; } };
      for (const b of boxes) {
        if (!cur || b.id !== cur.id || b.color !== col) { flush(); cur = new Path2D(); cur.id = b.id; col = b.color; }
        if (b.radius > 0) cur.roundRect(b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, b.radius); else cur.rect(b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0);
      }
      flush();
    }
    draw(page, view, dpr) {
      const paths = page.buildPaths();
      const styled = page.styledPaths();
      const styledSet = new Set(styled.map(s => s[0]));
      const L = page.currentLayout || { scale: 1, offsetX: 0, offsetY: 0, lineDy: null, lineSpacing: 0 };
      const ink = page.defaultInk;
      const key = `${view.scale.toFixed(4)}|${view.offsetX.toFixed(1)}|${view.offsetY.toFixed(1)}|${dpr}|${ink}|${L.scale}|${L.lineSpacing}|${L.lineDy ? L.lineDy[0] : ''}|${[...styledSet].sort((a, b) => a - b).join(',')}`;
      const W = this.canvas.width, H = this.canvas.height;
      const setTf = (c, line) => { const [s, tx, ty] = this.lineTransform(page, view, line, dpr); c.setTransform(s, 0, 0, s, tx, ty); };
      if (key !== this.baseKey || this.base.width !== W || this.base.height !== H) {
        const t0 = performance.now();
        this.base.width = W; this.base.height = H;
        const b = this.base.getContext('2d');
        b.fillStyle = css(ink);
        let n = 0, cur = -1;
        for (let i = 0; i < page.nPaths; i++) {
          if (styledSet.has(i)) continue;
          const ln = page.pathLine(i); if (ln !== cur) { setTf(b, ln); cur = ln; }
          b.fill(paths[i], page.pathEvenOdd(i) ? 'evenodd' : 'nonzero'); n++;
        }
        this.baseKey = key; this.stats.baseMs = performance.now() - t0; this.stats.basePaths = n;
      }
      const t1 = performance.now();
      const c = this.ctx;
      c.setTransform(1, 0, 0, 1, 0, 0); c.clearRect(0, 0, W, H);
      const bands = page.highlightBoxesView();
      this.drawBoxes(c, bands, view, dpr);
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.drawImage(this.base, 0, 0);
      let cur = -1;
      for (const [i, col] of styled) {
        if ((col & 255) === 0) continue;
        const ln = page.pathLine(i); if (ln !== cur) { setTf(c, ln); cur = ln; }
        c.fillStyle = css(col); c.fill(paths[i], page.pathEvenOdd(i) ? 'evenodd' : 'nonzero');
      }
      this.drawBoxes(c, page.maskBoxesView(), view, dpr);
      this.stats.overlayMs = performance.now() - t1; this.stats.overlayPaths = styled.length; this.stats.bands = bands.length;
    }
  }

  global.QVP = { QvpEngine, QvpPage, QvpAtlas, CanvasRenderer, Sel, T, KIND, FAMILY, CATEGORY, DECORATION, FORM, LAYER, NAMES_TABLE, DEFAULTS, NONE, css, rgba, parseTarget };
})(typeof window !== 'undefined' ? window : globalThis);
