// QVP web wrapper: thin JS over the C ABI exported by qvp_ffi.wasm.
// Host renderer = Canvas2D (Path2D). No framework, no bundler.
(function (global) {
  'use strict';

  const NONE = 0xffffffff;
  const SEL = { PATH: 0, WORD: 1, AYAH: 2, LINE: 3, MARK: 4, FAMILY: 5, KIND: 6, DECO: 7 };
  const KIND = { BODY: 0, MARK: 1, AYAH_NUMBER: 2, AYAH_ORNAMENT: 3, HEADER_INK: 4, OTHER: 255 };
  const FAMILY = { NONE: 0, DIACRITIC: 1, TANWEEN: 2, DOTS: 3, WAQF: 4, SIFR: 5, SAJDAH: 6, READING_SIGN: 7 };
  const DECO = { AYAH_MARKER: 0, SURAH_NAME: 1, BASMALAH: 2, HIZB_MARK: 3, SAJDAH_MARK: 4 };

  class QvpEngine {
    static async init(wasmBytes) {
      const { instance } = await WebAssembly.instantiate(wasmBytes, {});
      return new QvpEngine(instance);
    }
    constructor(instance) {
      this.ex = instance.exports;
      this.mem = this.ex.memory;
      this.scratch = this.ex.qvp_alloc(4096);
      this.td = new TextDecoder();
    }
    dv() { return new DataView(this.mem.buffer); }
    str(ptr, len) { return this.td.decode(new Uint8Array(this.mem.buffer, ptr, len)); }
    name(fn, v) { return this.str(this.ex[fn](v), this.ex[fn + '_len'](v)); }
    markName(m) { return this.name('qvp_mark_name', m); }
    familyName(f) { return this.name('qvp_family_name', f); }
    kindName(k) { return this.name('qvp_kind_name', k); }
    loadPage(bytes) {
      const p = this.ex.qvp_alloc(bytes.length);
      new Uint8Array(this.mem.buffer, p, bytes.length).set(bytes);
      const h = this.ex.qvp_page_load(p, bytes.length);
      this.ex.qvp_dealloc(p, bytes.length);
      if (!h) throw new Error('qvp_page_load failed');
      return new QvpPage(this, h);
    }
  }

  class QvpPage {
    constructor(engine, handle) {
      this.e = engine;
      this.h = handle;
      const ex = engine.ex, s = engine.scratch;
      ex.qvp_page_info(handle, s);
      let d = engine.dv();
      this.width = d.getFloat32(s, true);
      this.height = d.getFloat32(s + 4, true);
      this.page = d.getUint32(s + 8, true);
      this.nLines = d.getUint32(s + 12, true);
      this.nAyahs = d.getUint32(s + 16, true);
      this.nWords = d.getUint32(s + 20, true);
      this.nPaths = d.getUint32(s + 24, true);
      this.nDecos = d.getUint32(s + 28, true);
      // geometry is static: copy out of wasm memory once
      ex.qvp_geometry(handle, s);
      d = engine.dv();
      const opsPtr = d.getUint32(s, true), opsLen = d.getUint32(s + 4, true);
      const ptsPtr = d.getUint32(s + 8, true), ptsLen = d.getUint32(s + 12, true);
      const tabPtr = d.getUint32(s + 16, true), nPaths = d.getUint32(s + 20, true);
      this.ops = new Uint8Array(engine.mem.buffer, opsPtr, opsLen).slice();
      this.pts = new Float32Array(engine.mem.buffer.slice(ptsPtr, ptsPtr + ptsLen * 4));
      this.table = new Uint32Array(engine.mem.buffer.slice(tabPtr, tabPtr + nPaths * 32));
      this.paths = null; // Path2D cache, built lazily
      this.words = new Array(this.nWords);
      this.ayahs = new Array(this.nAyahs);
      this.lines = new Array(this.nLines);
      this.decos = new Array(this.nDecos);
      for (let i = 0; i < this.nWords; i++) this.words[i] = this._word(i);
      for (let i = 0; i < this.nAyahs; i++) this.ayahs[i] = this._ayah(i);
      for (let i = 0; i < this.nLines; i++) this.lines[i] = this._line(i);
      for (let i = 0; i < this.nDecos; i++) this.decos[i] = this._deco(i);
    }
    free() { this.e.ex.qvp_page_free(this.h); this.h = 0; }

    pathFlags(i) { return this.table[i * 8 + 4]; }
    pathKind(i) { return this.pathFlags(i) & 0xff; }
    pathMark(i) { return (this.pathFlags(i) >> 8) & 0xff; }
    pathFamily(i) { return (this.pathFlags(i) >> 16) & 0xff; }
    pathEvenOdd(i) { return ((this.pathFlags(i) >> 24) & 1) === 1; }
    pathWord(i) { const w = this.table[i * 8 + 5]; return w === NONE ? -1 : w; }
    pathLine(i) { return this.table[i * 8 + 6]; }

    /** Build (once) a Path2D per path in page units. */
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
      this.paths = out;
      return out;
    }

    _word(i) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_word_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return {
        idx: i, sura: d.getUint16(s, true), ayah: d.getUint16(s + 2, true), word: d.getUint16(s + 4, true),
        line: d.getUint16(s + 6, true), ayahIdx: d.getUint32(s + 8, true),
        x0: d.getFloat32(s + 12, true), y0: d.getFloat32(s + 16, true), x1: d.getFloat32(s + 20, true), y1: d.getFloat32(s + 24, true),
        text: this.e.str(d.getUint32(s + 28, true), d.getUint32(s + 32, true)),
        firstPath: d.getUint32(s + 36, true), nPaths: d.getUint32(s + 40, true),
      };
    }
    _ayah(i) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_ayah_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return {
        idx: i, sura: d.getUint16(s, true), ayah: d.getUint16(s + 2, true), part: d.getUint8(s + 4), parts: d.getUint8(s + 5), flags: d.getUint8(s + 6),
        firstWord: d.getUint32(s + 8, true), nWords: d.getUint32(s + 12, true), markerDeco: d.getUint32(s + 16, true),
        x0: d.getFloat32(s + 20, true), y0: d.getFloat32(s + 24, true), x1: d.getFloat32(s + 28, true), y1: d.getFloat32(s + 32, true),
      };
    }
    _line(i) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_line_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { idx: i, lineNo: d.getUint8(s), firstWord: d.getUint32(s + 4, true), nWords: d.getUint32(s + 8, true),
        x0: d.getFloat32(s + 12, true), y0: d.getFloat32(s + 16, true), x1: d.getFloat32(s + 20, true), y1: d.getFloat32(s + 24, true) };
    }
    _deco(i) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_deco_info(this.h, i, s)) return null;
      const d = this.e.dv();
      return { idx: i, kind: d.getUint8(s), sura: d.getUint16(s + 2, true), ayah: d.getUint16(s + 4, true),
        x0: d.getFloat32(s + 8, true), y0: d.getFloat32(s + 12, true), x1: d.getFloat32(s + 16, true), y1: d.getFloat32(s + 20, true),
        text: this.e.str(d.getUint32(s + 24, true), d.getUint32(s + 28, true)),
        firstPath: d.getUint32(s + 32, true), nPaths: d.getUint32(s + 36, true) };
    }

    /** Hit-test in page units → {word, path, deco} indices (-1 = none) or null. */
    hitTest(x, y) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_hit_test(this.h, x, y, s)) return null;
      const d = this.e.dv();
      const f = v => (v === NONE ? -1 : v);
      return { word: f(d.getUint32(s, true)), path: f(d.getUint32(s + 4, true)), deco: f(d.getUint32(s + 8, true)) };
    }
    findWord(sura, ayah, word) { return this.e.ex.qvp_find_word(this.h, sura, ayah, word); }

    /**
     * Engine layout: {viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, fillHeight, nominalLines}
     * → {scale, ox, oy, contentH, pitch, lineDy: Float32Array, slots: [[top,bottom],...]}.
     * Page → viewport: vx = ox + x*scale ; vy = oy + (y + lineDy[line])*scale.
     */
    layout(spec) {
      const ex = this.e.ex, s = this.e.scratch;
      let d = this.e.dv();
      d.setFloat32(s, spec.viewportW, true); d.setFloat32(s + 4, spec.viewportH, true);
      d.setFloat32(s + 8, spec.padTop || 0, true); d.setFloat32(s + 12, spec.padBottom || 0, true);
      d.setFloat32(s + 16, spec.padLeft || 0, true); d.setFloat32(s + 20, spec.padRight || 0, true);
      d.setFloat32(s + 24, spec.lineSpacing ?? 1, true); d.setUint32(s + 28, spec.fillHeight ? 1 : 0, true);
      d.setUint32(s + 32, spec.nominalLines || 15, true);
      ex.qvp_layout(this.h, s, s + 64);
      d = this.e.dv();
      const o = s + 64;
      const n = d.getUint32(o + 20, true), lp = d.getUint32(o + 24, true);
      const f = new Float32Array(this.e.mem.buffer.slice(lp, lp + n * 12));
      const lineDy = new Float32Array(n), slots = new Array(n);
      for (let i = 0; i < n; i++) { lineDy[i] = f[i * 3]; slots[i] = [f[i * 3 + 1], f[i * 3 + 2]]; }
      this.currentLayout = { scale: d.getFloat32(o, true), ox: d.getFloat32(o + 4, true), oy: d.getFloat32(o + 8, true),
        contentH: d.getFloat32(o + 12, true), pitch: d.getFloat32(o + 16, true), lineDy, slots };
      return this.currentLayout;
    }
    /** Hit-test in viewport px through the current layout. */
    hitTestView(vx, vy) {
      const ex = this.e.ex, s = this.e.scratch;
      if (!ex.qvp_hit_test_view(this.h, vx, vy, s)) return null;
      const d = this.e.dv();
      const f = v => (v === NONE ? -1 : v);
      return { word: f(d.getUint32(s, true)), path: f(d.getUint32(s + 4, true)), deco: f(d.getUint32(s + 8, true)) };
    }

    /** rgba: 0xRRGGBBAA (alpha 0 hides). on=false removes the rule. */
    style(sel, a, b, c, rgba, on = true) { this.e.ex.qvp_style(this.h, sel, a >>> 0, b >>> 0, c >>> 0, rgba >>> 0, on ? 1 : 0); }
    styleWord(s, a, w, rgba, on = true) { this.style(SEL.WORD, s, a, w, rgba, on); }
    styleAyah(s, a, rgba, on = true) { this.style(SEL.AYAH, s, a, 0, rgba, on); }
    styleLine(n, rgba, on = true) { this.style(SEL.LINE, n, 0, 0, rgba, on); }
    styleMark(m, rgba, on = true) { this.style(SEL.MARK, m, 0, 0, rgba, on); }
    styleFamily(f, rgba, on = true) { this.style(SEL.FAMILY, f, 0, 0, rgba, on); }
    styleKind(k, rgba, on = true) { this.style(SEL.KIND, k, 0, 0, rgba, on); }
    styleDeco(k, rgba, on = true) { this.style(SEL.DECO, k, 0, 0, rgba, on); }
    stylePath(i, rgba, on = true) { this.style(SEL.PATH, i, 0, 0, rgba, on); }
    styleClear() { this.e.ex.qvp_style_clear(this.h); }
    styleDefault(rgba) { this.e.ex.qvp_style_default(this.h, rgba >>> 0); }

    /** Full display list: Uint32Array colour per path (copy). */
    paint() {
      const p = this.e.ex.qvp_paint(this.h);
      return new Uint32Array(this.e.mem.buffer.slice(p, p + this.nPaths * 4));
    }
    /** Overlay list: [[pathIdx, rgba], ...] for paths whose colour differs from the default. */
    styled() {
      const ex = this.e.ex;
      const n = ex.qvp_styled(this.h, 0, 0);
      if (!n) return [];
      const buf = ex.qvp_alloc(n * 8);
      ex.qvp_styled(this.h, buf, n);
      const v = new Uint32Array(this.e.mem.buffer.slice(buf, buf + n * 8));
      ex.qvp_dealloc(buf, n * 8);
      const out = new Array(n);
      for (let i = 0; i < n; i++) out[i] = [v[i * 2], v[i * 2 + 1]];
      return out;
    }
  }

  const css = rgba => `rgba(${(rgba >>> 24) & 255},${(rgba >>> 16) & 255},${(rgba >>> 8) & 255},${(rgba & 255) / 255})`;
  const rgba = (hex, a = 255) => ((parseInt(hex.slice(1), 16) << 8) | a) >>> 0;

  /**
   * Canvas2D host renderer with a cached base layer (unstyled ink) and an
   * overlay pass for styled paths. Rebuilds the base when the styled *set*
   * or the view scale changes; recolouring within the same set is overlay-only.
   */
  class CanvasRenderer {
    constructor(canvas) {
      this.canvas = canvas;
      this.ctx = canvas.getContext('2d');
      this.base = document.createElement('canvas');
      this.baseKey = '';
      this.stats = { baseMs: 0, overlayMs: 0, basePaths: 0, overlayPaths: 0 };
    }
    /**
     * view: {scale, ox, oy} — pan/zoom in CSS px applied on top of the engine
     * layout (page.currentLayout, or identity when none). Paths are drawn per
     * line with the line's dy from the layout.
     */
    draw(page, view, defaultInk, dpr) {
      const paths = page.buildPaths();
      const L = page.currentLayout || { scale: 1, ox: 0, oy: 0, lineDy: null };
      const styled = page.styled();
      const styledSet = new Set(styled.map(s => s[0]));
      const key = `${view.scale.toFixed(4)}|${view.ox.toFixed(1)}|${view.oy.toFixed(1)}|${dpr}|${defaultInk}|${L.scale}|${L.lineDy ? Array.from(L.lineDy).map(v => v.toFixed(2)).join(';') : ''}|${[...styledSet].sort((a, b) => a - b).join(',')}`;
      const W = this.canvas.width, H = this.canvas.height;
      const setTf = (c, line) => {
        const s = dpr * view.scale * L.scale;
        const dy = L.lineDy ? L.lineDy[line] : 0;
        c.setTransform(s, 0, 0, s, dpr * (view.ox + view.scale * L.ox), dpr * (view.oy + view.scale * (L.oy + dy * L.scale)));
      };
      if (key !== this.baseKey || this.base.width !== W || this.base.height !== H) {
        const t0 = performance.now();
        this.base.width = W; this.base.height = H;
        const b = this.base.getContext('2d');
        b.fillStyle = css(defaultInk);
        let n = 0, curLine = -1;
        for (let i = 0; i < page.nPaths; i++) {
          if (styledSet.has(i)) continue;
          const ln = page.pathLine(i);
          if (ln !== curLine) { setTf(b, ln); curLine = ln; }
          b.fill(paths[i], page.pathEvenOdd(i) ? 'evenodd' : 'nonzero');
          n++;
        }
        this.baseKey = key;
        this.stats.baseMs = performance.now() - t0;
        this.stats.basePaths = n;
      }
      const t1 = performance.now();
      const c = this.ctx;
      c.setTransform(1, 0, 0, 1, 0, 0);
      c.clearRect(0, 0, W, H);
      c.drawImage(this.base, 0, 0);
      let curLine = -1;
      for (const [i, col] of styled) {
        if ((col & 255) === 0) continue;
        const ln = page.pathLine(i);
        if (ln !== curLine) { setTf(c, ln); curLine = ln; }
        c.fillStyle = css(col);
        c.fill(paths[i], page.pathEvenOdd(i) ? 'evenodd' : 'nonzero');
      }
      this.stats.overlayMs = performance.now() - t1;
      this.stats.overlayPaths = styled.length;
    }
    /** Transform helper for UI overlays: returns [scale, tx, ty] for a line in device px. */
    lineTransform(page, view, line, dpr) {
      const L = page.currentLayout || { scale: 1, ox: 0, oy: 0, lineDy: null };
      const s = dpr * view.scale * L.scale, dy = L.lineDy ? L.lineDy[line] : 0;
      return [s, dpr * (view.ox + view.scale * L.ox), dpr * (view.oy + view.scale * (L.oy + dy * L.scale))];
    }
  }

  global.QVP = { QvpEngine, QvpPage, CanvasRenderer, SEL, KIND, FAMILY, DECO, NONE, css, rgba };
})(typeof window !== 'undefined' ? window : globalThis);
