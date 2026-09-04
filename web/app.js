(async function () {
  'use strict';
  const { QvpEngine, CanvasRenderer, KIND, FAMILY, DECO, css, rgba } = window.QVP;
  const $ = id => document.getElementById(id);

  // ── data source: embedded (single-file build) or fetch (dev server) ──
  const EMB = window.QVP_EMBED || null;
  const b64 = s => Uint8Array.from(atob(s), c => c.charCodeAt(0));
  const pad = n => String(n).padStart(3, '0');
  const src = {
    wasm: async () => EMB ? b64(EMB.wasm) : new Uint8Array(await (await fetch('qvp_ffi.wasm')).arrayBuffer()),
    pages: EMB ? Object.keys(EMB.pages).map(Number).sort((a, b) => a - b) : Array.from({ length: 604 }, (_, i) => i + 1),
    page: async n => EMB ? b64(EMB.pages[pad(n)]) : new Uint8Array(await (await fetch(`pages/${pad(n)}.qvp`)).arrayBuffer()),
    words: async n => EMB ? (EMB.words[pad(n)] || {}) : (await fetch(`pages/${pad(n)}.words.json`)).json(),
  };

  const t0 = performance.now();
  const wasmBytes = await src.wasm();
  const engine = await QvpEngine.init(wasmBytes);
  const wasmMs = performance.now() - t0;

  const canvas = $('cv'), stage = $('stage'), paper = $('paper');
  const renderer = new CanvasRenderer(canvas);
  const dpr = Math.min(window.devicePixelRatio || 1, 3);

  const state = {
    page: null, words: {}, bytes: 0, loadMs: 0, n: src.pages[0],
    view: { scale: 1, ox: 0, oy: 0 },
    sel: { word: -1, ayah: null, path: -1, pathColors: new Map() },
    hover: -1, tajweed: false, hideMarks: false, markers: false, playing: false, playIdx: 0,
    ink: 0x231f20ff, theme: 'light', lastHitUs: 0,
    layout: { lineSpacing: 1, fillHeight: false, padTop: 24, padBottom: 24, padSide: 16 },
  };
  const PALETTE = {
    [FAMILY.DIACRITIC]: '#1a73e8', [FAMILY.TANWEEN]: '#8e24aa', [FAMILY.DOTS]: '#c62828',
    [FAMILY.WAQF]: '#0a7d32', [FAMILY.SIFR]: '#ef6c00', [FAMILY.SAJDAH]: '#6d4c41',
  };
  const SEL_COLOR = '#1a73e8', AYAH_COLOR = '#0a7d32', PLAY_COLOR = '#d81b60', GOLD = '#b8860b';

  // ── styling: everything goes through the engine's style state ──
  function applyStyles() {
    const p = state.page; if (!p) return;
    p.styleClear();
    p.styleDefault(state.ink);
    if (state.tajweed) for (const f in PALETTE) p.styleFamily(+f, rgba(PALETTE[f]));
    if (state.hideMarks) p.styleKind(KIND.MARK, 0);
    if (state.markers) { p.styleDeco(DECO.AYAH_MARKER, rgba(GOLD)); }
    if (state.sel.ayah) p.styleAyah(state.sel.ayah[0], state.sel.ayah[1], rgba(AYAH_COLOR));
    if (state.sel.word >= 0) { const w = p.words[state.sel.word]; p.styleWord(w.sura, w.ayah, w.word, rgba(SEL_COLOR)); }
    for (const [pi, col] of state.sel.pathColors) p.stylePath(pi, rgba(col));
    if (state.playing && state.playIdx < p.nWords) { const w = p.words[state.playIdx]; p.styleWord(w.sura, w.ayah, w.word, rgba(PLAY_COLOR)); }
    draw();
  }

  // ── view / drawing ──
  function resize() {
    const r = stage.getBoundingClientRect();
    canvas.width = Math.round(r.width * dpr); canvas.height = Math.round(r.height * dpr);
    canvas.style.width = r.width + 'px'; canvas.style.height = r.height + 'px';
    fit(false); draw();
  }
  function relayout() {
    const p = state.page; if (!p) return null;
    const r = stage.getBoundingClientRect(), ls = state.layout;
    // page width fills the stage width (minus side padding); height follows the layout mode
    const maxW = Math.min(r.width, r.height * p.width / p.height * 1.15);
    const L = p.layout({ viewportW: maxW, viewportH: r.height, padTop: ls.padTop, padBottom: ls.padBottom, padLeft: ls.padSide, padRight: ls.padSide,
      lineSpacing: ls.lineSpacing, fillHeight: ls.fillHeight, nominalLines: 15 });
    L.contentW = maxW;
    return L;
  }
  function fit(redraw = true) {
    const p = state.page; if (!p) return;
    const L = relayout(); if (!L) return;
    const r = stage.getBoundingClientRect();
    const s = Math.min(1, r.height / L.contentH);
    state.view = { scale: s, ox: (r.width - L.contentW * s) / 2, oy: (r.height - L.contentH * s) / 2 };
    renderer.baseKey = '';
    if (redraw) draw();
  }
  /** stage CSS px → engine viewport px (the layout's coordinate space) */
  function toView(cx, cy) { const v = state.view; return [(cx - v.ox) / v.scale, (cy - v.oy) / v.scale]; }
  let raf = 0;
  function draw() { if (!raf) raf = requestAnimationFrame(drawNow); }
  function drawNow() {
    raf = 0;
    const p = state.page; if (!p) return;
    const v = state.view, L = p.currentLayout;
    paper.style.left = v.ox + 'px'; paper.style.top = v.oy + 'px';
    paper.style.width = (L ? L.contentW : p.width) * v.scale + 'px'; paper.style.height = (L ? L.contentH : p.height) * v.scale + 'px';
    renderer.draw(p, v, state.ink, dpr);
    // cheap UI overlays that do not touch the engine: hover + selection backgrounds
    const c = renderer.ctx;
    c.globalCompositeOperation = 'destination-over';
    const lineOf = w => (w.line !== undefined ? p.lines.findIndex(l => l.lineNo === w.line) : -1);
    const box = (w, fill, line) => { const [s, tx, ty] = renderer.lineTransform(p, v, line, dpr); c.setTransform(s, 0, 0, s, tx, ty); c.fillStyle = fill; const pd = 1.2; c.beginPath(); c.roundRect(w.x0 - pd, w.y0 - pd, w.x1 - w.x0 + 2 * pd, w.y1 - w.y0 + 2 * pd, 1.5); c.fill(); };
    if (state.hover >= 0 && state.hover !== state.sel.word) { const w = p.words[state.hover]; box(w, getComputedStyle(document.body).getPropertyValue('--hover'), lineOf(w)); }
    if (state.sel.word >= 0) { const w = p.words[state.sel.word]; box(w, getComputedStyle(document.body).getPropertyValue('--sel'), lineOf(w)); }
    if (state.sel.ayah) for (const a of p.ayahs) if (a.sura === state.sel.ayah[0] && a.ayah === state.sel.ayah[1]) { const w0 = p.words[a.firstWord]; box(a, 'rgba(10,125,50,.08)', w0 ? lineOf(w0) : 0); }
    c.globalCompositeOperation = 'source-over';
    hud();
  }
  function hud() {
    const p = state.page, s = renderer.stats;
    $('hud').textContent =
      `wasm engine   ${(wasmBytes.length / 1024).toFixed(0)} KB, init ${wasmMs.toFixed(1)} ms\n` +
      `page ${pad(state.n)}      ${(state.bytes / 1024).toFixed(0)} KB, load+decode ${state.loadMs.toFixed(2)} ms\n` +
      `content       ${p.nWords} words · ${p.nPaths} paths · ${p.nAyahs} ayah parts · ${p.nLines} lines\n` +
      `base layer    ${s.basePaths} paths in ${s.baseMs.toFixed(2)} ms (cached)\n` +
      `overlay       ${s.overlayPaths} styled paths in ${s.overlayMs.toFixed(2)} ms\n` +
      `hit-test      ${state.lastHitUs.toFixed(1)} µs (wasm)\n` +
      `layout        ${state.layout.fillHeight ? 'fill height' : 'spacing ×' + state.layout.lineSpacing} · pad ${state.layout.padTop}/${state.layout.padBottom} · pitch ${(p.currentLayout ? p.currentLayout.pitch : 0).toFixed(1)} u\n` +
      `zoom          ${(state.view.scale * (p.currentLayout ? p.currentLayout.scale : 1) * dpr).toFixed(2)}× device px per unit`;
  }

  // ── page loading ──
  async function loadPage(n) {
    n = Math.max(src.pages[0], Math.min(src.pages[src.pages.length - 1], n));
    if (!src.pages.includes(n)) n = src.pages.reduce((a, b) => Math.abs(b - n) < Math.abs(a - n) ? b : a);
    const bytes = await src.page(n);
    const t = performance.now();
    const page = engine.loadPage(bytes);
    page.buildPaths();
    state.loadMs = performance.now() - t;
    if (state.page) state.page.free();
    state.page = page; state.n = n; state.bytes = bytes.length;
    state.words = await src.words(n).catch(() => ({}));
    state.sel = { word: -1, ayah: null, path: -1, pathColors: new Map() }; state.hover = -1; state.playIdx = 0;
    $('pageNo').value = n;
    renderer.baseKey = '';
    fit(false);
    applyStyles();
    showSelection();
  }

  // ── selection panel ──
  function showSelection() {
    const p = state.page, w = state.sel.word >= 0 ? p.words[state.sel.word] : null;
    const info = $('selInfo'), chips = $('selPaths');
    info.innerHTML = ''; chips.innerHTML = '';
    if (!w) {
      $('selWord').textContent = state.sel.ayah ? `آية ${state.sel.ayah[1]} · سورة ${state.sel.ayah[0]}` : '—';
      if (state.sel.ayah) {
        const words = p.words.filter(x => x.sura === state.sel.ayah[0] && x.ayah === state.sel.ayah[1]).map(x => x.text).join(' ');
        $('selWord').textContent = words;
        info.innerHTML = `<b>ayah</b><span>${state.sel.ayah[0]}:${state.sel.ayah[1]}</span><b>words on page</b><span>${p.words.filter(x => x.sura === state.sel.ayah[0] && x.ayah === state.sel.ayah[1]).length}</span>`;
      }
      return;
    }
    $('selWord').textContent = w.text;
    const wid = `${w.sura}:${w.ayah}:${w.word}`, t = state.words[wid] || {};
    const rows = [['wid', wid], ['line', w.line], ['imlaei', t.imlaei], ['qpc', t.qpc], ['rasm', t.rasm], ['search', t.search],
      ['bbox', `${w.x0.toFixed(1)}, ${w.y0.toFixed(1)} → ${w.x1.toFixed(1)}, ${w.y1.toFixed(1)}`], ['paths', w.nPaths]];
    for (const [k, v] of rows) if (v !== undefined && v !== '') info.insertAdjacentHTML('beforeend', `<b>${k}</b><span class="${/imlaei|qpc|rasm|search/.test(k) ? 'v' : ''}">${v}</span>`);
    for (let i = w.firstPath; i < w.firstPath + w.nPaths; i++) {
      const kind = p.pathKind(i), mark = p.pathMark(i);
      const label = kind === KIND.MARK ? engine.markName(mark) : engine.kindName(kind);
      const el = document.createElement('span');
      el.className = 'chip' + (state.sel.pathColors.has(i) ? ' on' : '') + (i === state.sel.path ? ' hit' : '');
      el.textContent = `#${i - w.firstPath} ${label}`;
      el.title = `path ${i}`;
      el.onclick = () => { const m = state.sel.pathColors; m.has(i) ? m.delete(i) : m.set(i, '#ef6c00'); applyStyles(); showSelection(); };
      chips.appendChild(el);
    }
  }

  // ── interaction ──
  let drag = null, pinch = null, moved = false;
  const pts = new Map();
  stage.addEventListener('pointerdown', e => {
    try { stage.setPointerCapture(e.pointerId); } catch (_) {}
    pts.set(e.pointerId, [e.clientX, e.clientY]); moved = false;
    if (pts.size === 1) drag = { x: e.clientX, y: e.clientY, ox: state.view.ox, oy: state.view.oy };
    if (pts.size === 2) { const [a, b] = [...pts.values()]; pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), scale: state.view.scale, cx: (a[0] + b[0]) / 2, cy: (a[1] + b[1]) / 2, ox: state.view.ox, oy: state.view.oy }; drag = null; }
  });
  stage.addEventListener('pointermove', e => {
    const r = stage.getBoundingClientRect();
    if (pts.has(e.pointerId)) pts.set(e.pointerId, [e.clientX, e.clientY]);
    if (pinch && pts.size === 2) {
      const [a, b] = [...pts.values()]; const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
      const k = Math.max(0.2, Math.min(40, pinch.scale * d / pinch.d)) / pinch.scale;
      const cx = pinch.cx - r.left, cy = pinch.cy - r.top;
      state.view = { scale: pinch.scale * k, ox: cx - (cx - pinch.ox) * k, oy: cy - (cy - pinch.oy) * k }; moved = true; draw(); return;
    }
    if (drag) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (Math.hypot(dx, dy) > 3) { moved = true; stage.classList.add('dragging'); state.view.ox = drag.ox + dx; state.view.oy = drag.oy + dy; draw(); }
      return;
    }
    const [x, y] = toView(e.clientX - r.left, e.clientY - r.top);
    const t = performance.now(); const h = state.page.hitTestView(x, y); state.lastHitUs = (performance.now() - t) * 1000;
    const hw = h ? h.word : -1;
    if (hw !== state.hover) { state.hover = hw; stage.style.cursor = hw >= 0 || (h && h.deco >= 0) ? 'pointer' : 'grab'; draw(); }
  });
  const up = e => {
    pts.delete(e.pointerId);
    if (pts.size < 2) pinch = null;
    if (pts.size === 0) {
      stage.classList.remove('dragging');
      if (drag && !moved) {
        const r = stage.getBoundingClientRect(); const [x, y] = toView(e.clientX - r.left, e.clientY - r.top);
        const t = performance.now(); const h = state.page.hitTestView(x, y); state.lastHitUs = (performance.now() - t) * 1000;
        if (h && h.word >= 0) { state.sel.word = state.sel.word === h.word ? -1 : h.word; state.sel.ayah = null; state.sel.path = h.path; state.sel.pathColors.clear(); }
        else if (h && h.deco >= 0) { const d = state.page.decos[h.deco]; if (d.ayah) { state.sel.ayah = [d.sura, d.ayah]; state.sel.word = -1; } }
        else { state.sel.word = -1; state.sel.ayah = null; state.sel.pathColors.clear(); }
        applyStyles(); showSelection();
      }
      drag = null;
    }
  };
  stage.addEventListener('pointerup', up); stage.addEventListener('pointercancel', up);
  stage.addEventListener('lostpointercapture', e => pts.delete(e.pointerId));
  stage.addEventListener('wheel', e => {
    e.preventDefault(); const r = stage.getBoundingClientRect();
    const k = Math.exp(-e.deltaY * 0.0015); const v = state.view; const cx = e.clientX - r.left, cy = e.clientY - r.top;
    const ns = Math.max(0.2, Math.min(40, v.scale * k)); const kk = ns / v.scale;
    state.view = { scale: ns, ox: cx - (cx - v.ox) * kk, oy: cy - (cy - v.oy) * kk }; draw();
  }, { passive: false });
  stage.addEventListener('dblclick', () => fit());

  // ── controls ──
  const toggle = (id, key) => $(id).onclick = () => { state[key] = !state[key]; $(id).classList.toggle('on', state[key]); applyStyles(); };
  toggle('tajweed', 'tajweed'); toggle('hideMarks', 'hideMarks'); toggle('markers', 'markers');
  $('clear').onclick = () => { state.tajweed = state.hideMarks = state.markers = false; for (const id of ['tajweed', 'hideMarks', 'markers']) $(id).classList.remove('on'); state.sel = { word: -1, ayah: null, path: -1, pathColors: new Map() }; stopPlay(); applyStyles(); showSelection(); };
  $('ink').oninput = e => { state.ink = rgba(e.target.value); applyStyles(); };
  $('theme').onchange = e => setTheme(e.target.value);
  function setTheme(t) {
    state.theme = t; document.body.setAttribute('data-qvp-theme', t);
    const ink = { light: '#231f20', sepia: '#3b2a14', dark: '#e8e4dc' }[t];
    $('ink').value = ink; state.ink = rgba(ink); applyStyles();
  }
  $('prev').onclick = () => loadPage(state.n - 1);
  $('next').onclick = () => loadPage(state.n + 1);
  $('pageNo').onchange = e => loadPage(+e.target.value);
  $('fit').onclick = () => fit();
  $('pageMax').textContent = src.pages[src.pages.length - 1];
  if (EMB) $('pageNo').title = 'This build embeds pages: ' + src.pages.join(', ');
  window.addEventListener('keydown', e => { if (e.key === 'ArrowLeft') loadPage(state.n + 1); if (e.key === 'ArrowRight') loadPage(state.n - 1); });
  let timer = 0;
  function stopPlay() { state.playing = false; clearInterval(timer); timer = 0; $('play').classList.remove('on'); $('play').textContent = '▶ Play words'; }
  $('play').onclick = () => {
    if (state.playing) { stopPlay(); applyStyles(); return; }
    state.playing = true; $('play').classList.add('on'); $('play').textContent = '■ Stop';
    state.playIdx = state.sel.word >= 0 ? state.sel.word : 0;
    timer = setInterval(async () => {
      state.playIdx++;
      if (state.playIdx >= state.page.nWords) { const next = src.pages[src.pages.indexOf(state.n) + 1]; if (next) { await loadPage(next); state.playing = true; $('play').classList.add('on'); $('play').textContent = '■ Stop'; } else stopPlay(); }
      applyStyles();
    }, 320);
    applyStyles();
  };
  const relayoutUI = () => { $('spacingVal').textContent = '×' + state.layout.lineSpacing.toFixed(2); fit(); };
  $('spacing').oninput = e => { state.layout.lineSpacing = +e.target.value; state.layout.fillHeight = false; $('fillH').classList.remove('on'); relayoutUI(); };
  $('fillH').onclick = () => { state.layout.fillHeight = !state.layout.fillHeight; $('fillH').classList.toggle('on', state.layout.fillHeight); relayoutUI(); };
  $('padTop').oninput = e => { state.layout.padTop = +e.target.value; $('padTopVal').textContent = e.target.value; relayoutUI(); };
  $('padBottom').oninput = e => { state.layout.padBottom = +e.target.value; $('padBottomVal').textContent = e.target.value; relayoutUI(); };
  $('legend').innerHTML = Object.entries(PALETTE).map(([f, c]) => `<span class="hint"><i class="sw" style="background:${c}"></i>${engine.familyName(+f)}</span>`).join('');

  window.__qvp = { state, engine, renderer, loadPage, fit, applyStyles };
  window.addEventListener('resize', resize);
  const prefersDark = document.documentElement.dataset.theme === 'dark' || (document.documentElement.dataset.theme !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches);
  if (prefersDark) { $('theme').value = 'dark'; state.theme = 'dark'; document.body.setAttribute('data-qvp-theme', 'dark'); $('ink').value = '#e8e4dc'; state.ink = rgba('#e8e4dc'); }
  await loadPage(src.pages[0]);
  resize();
  const q = new URLSearchParams(location.search).get('p'); if (q) loadPage(+q);
})();
