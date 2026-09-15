(async function () {
  'use strict';
  const { QvpEngine, CanvasRenderer, Sel, T, KIND, FAMILY, CATEGORY, DECORATION, LAYER, css, rgba } = window.QVP;
  const $ = id => document.getElementById(id);

  // ── data source: embedded (single-file build) or fetch (dev server) — the SDK ships no data ──
  const EMB = window.QVP_EMBED || null;
  const b64 = s => Uint8Array.from(atob(s), c => c.charCodeAt(0));
  const pad = n => String(n).padStart(3, '0');
  const src = {
    wasm: async () => EMB ? b64(EMB.wasm) : new Uint8Array(await (await fetch('qvp_ffi.wasm')).arrayBuffer()),
    atlas: async () => { if (EMB) return EMB.atlas ? b64(EMB.atlas) : null; try { const r = await fetch('pages/atlas.qva'); return r.ok ? new Uint8Array(await r.arrayBuffer()) : null; } catch { return null; } },
    pages: EMB ? Object.keys(EMB.pages).map(Number).sort((a, b) => a - b) : Array.from({ length: 604 }, (_, i) => i + 1),
    page: async n => EMB ? b64(EMB.pages[pad(n)]) : new Uint8Array(await (await fetch(`pages/${pad(n)}.qvp`)).arrayBuffer()),
  };

  const t0 = performance.now();
  const wasmBytes = await src.wasm();
  const engine = await QvpEngine.init(wasmBytes);
  const wasmMs = performance.now() - t0;
  let atlas = null;
  try { const ab = await src.atlas(); if (ab) atlas = engine.loadAtlas(ab); } catch (e) { console.warn('no atlas', e); }

  const canvas = $('cv'), stage = $('stage'), paper = $('paper');
  const renderer = new CanvasRenderer(canvas);
  const dpr = Math.min(window.devicePixelRatio || 1, 3);

  const S = {
    page: null, bytes: 0, loadMs: 0, n: src.pages[0],
    view: { scale: 1, offsetX: 0, offsetY: 0 },
    selWord: -1, selAyah: null, hlSel: 0, hlAyah: 0, hlSearch: 0, hlPlay: 0, pathHandles: new Map(),
    hover: -1, theme: 'light', themeHandle: 0, tajwidHandle: 0, hideHandle: 0, ayahMarksHandle: 0,
    playing: false, playIdx: 0, lastHitUs: 0, animating: false,
    layout: { lineSpacing: 1, fillHeight: false, padTop: 24, padBottom: 24, padSide: 16 },
    reflow: { on: false, zoom: 1.6, fill: 'centred', breaks: 'fitted', gaps: 'uniform', wordGap: 1, relax: 0.5, maxStretch: 2 },
    hlMode: 'both', hlMs: 250, revealOn: false,
  };
  const INK = { light: '#231f20', sepia: '#3b2a14', dark: '#e8e4dc' };
  const PALETTE = { [CATEGORY.HARAKAH]: '#1a73e8', [CATEGORY.TANWIN]: '#8e24aa', [CATEGORY.LETTER_DOT]: '#c62828', [CATEGORY.WAQF]: '#0a7d32', [CATEGORY.DABT]: '#ef6c00', [CATEGORY.ORTHOGRAPHIC]: '#00838f', [CATEGORY.STANDALONE]: '#6d4c41' };

  // ── rendering loop: draw on demand, keep drawing while the engine animates ──
  let raf = 0;
  function draw() { if (!raf) raf = requestAnimationFrame(frame); }
  function frame(now) {
    raf = 0;
    const p = S.page; if (!p) return;
    S.animating = p.tick(now);
    const v = S.view, L = p.currentLayout;
    paper.style.left = v.offsetX + 'px'; paper.style.top = v.offsetY + 'px';
    paper.style.width = (L ? L.contentW : p.width) * v.scale + 'px'; paper.style.height = (L ? L.contentH : p.height) * v.scale + 'px';
    renderer.draw(p, v, dpr);
    // hover: a cheap UI overlay, not engine state
    if (S.hover >= 0 && S.hover !== S.selWord) {
      // the engine places the band; the view adds only the reader's pan and zoom
      const c = renderer.ctx;
      c.setTransform(dpr * v.scale, 0, 0, dpr * v.scale, dpr * v.offsetX, dpr * v.offsetY);
      c.globalCompositeOperation = 'destination-over';
      c.fillStyle = getComputedStyle(document.body).getPropertyValue('--hover'); c.beginPath();
      for (const b of p.wordBandsView([S.hover], { height: 'ink', padX: 1.2, padY: 1.2 })) c.roundRect(b.x0, b.y0, b.x1 - b.x0, b.y1 - b.y0, 1.5);
      c.fill();
      c.globalCompositeOperation = 'source-over';
    }
    hud();
    if (S.animating) draw();
  }

  // ── view: engine layout + pan/zoom on top ──
  function layoutSpec() {
    const r = stage.getBoundingClientRect(), ls = S.layout;
    return { viewportW: r.width, viewportH: r.height, padTop: ls.padTop, padBottom: ls.padBottom, padLeft: ls.padSide, padRight: ls.padSide, lineSpacing: ls.lineSpacing, fillHeight: ls.fillHeight, maxAspectSlack: QVP.DEFAULTS.ASPECT_SLACK,
      reflow: S.reflow.on
        ? { zoom: S.reflow.zoom, fill: S.reflow.fill, breaks: S.reflow.breaks, gaps: S.reflow.gaps, wordGap: S.reflow.wordGap, relax: S.reflow.relax, maxStretch: S.reflow.maxStretch }
        : null };
  }
  function relayout() {
    const p = S.page; if (!p) return null;
    return p.layout(layoutSpec());
  }
  function fit(redraw = true) {
    const L = relayout(); if (!L) return;
    // a reflowed page is taller than the screen on purpose: show its top and let the reader scroll
    S.view = L.reflowed ? { scale: 1, offsetX: L.fitX, offsetY: 0 } : { scale: L.fitScale, offsetX: L.fitX, offsetY: L.fitY };
    renderer.baseKey = '';
    if (redraw) draw();
  }
  function resize() {
    const r = stage.getBoundingClientRect();
    canvas.width = Math.round(r.width * dpr); canvas.height = Math.round(r.height * dpr);
    canvas.style.width = r.width + 'px'; canvas.style.height = r.height + 'px';
    fit();
  }
  const toView = (cx, cy) => [(cx - S.view.offsetX) / S.view.scale, (cy - S.view.offsetY) / S.view.scale];

  // ── page loading ──
  async function loadPage(n) {
    if (!src.pages.includes(n)) n = src.pages.reduce((a, b) => Math.abs(b - n) < Math.abs(a - n) ? b : a);
    const bytes = await src.page(n);
    const t = performance.now();
    const page = engine.loadPage(bytes); page.buildPaths();
    S.loadMs = performance.now() - t;
    if (S.page) S.page.free();
    S.page = page; S.n = n; S.bytes = bytes.length;
    S.selWord = -1; S.selAyah = null; S.hover = -1; S.playIdx = 0; S.hlSel = S.hlAyah = S.hlSearch = S.hlPlay = 0; S.pathHandles.clear(); S.revealOn = false;
    S.themeHandle = S.tajwidHandle = S.hideHandle = S.ayahMarksHandle = 0;
    $('pageNo').value = n;
    applyTheme(); applyToggles();
    renderer.baseKey = '';
    fit(false);
    showSelection(); showMeta(); runSearch();
    draw();
  }
  function showMeta() {
    const p = S.page, su = p.surahs(), dv = p.divisions();
    const parts = [];
    for (const s of su) parts.push(`${s.number}${s.latin ? ' ' + s.latin : ''}${s.hasBanner ? ' (banner)' : ''}`);
    let t = `surahs: ${parts.join(', ')}`;
    if (dv.length) t += `\nstarts here: ${dv.map(d => `${d.division} ${d.number} at ${d.surah}:${d.ayah}`).join(', ')}`;
    if (atlas) { const j = atlas.juzOf(p.words[0].surah, p.words[0].ayah); if (j) t += `\njuz ${j} · pages ${atlas.pagesOfJuz(j).join('–')}`; }
    t += `\nayahs: ${p.ayahKeys().map(([s, a]) => `${s}:${a}`).join(' ')}`;
    $('meta').textContent = t;
  }

  // ── selection panel ──
  function showSelection() {
    const p = S.page, w = S.selWord >= 0 ? p.words[S.selWord] : null;
    const info = $('selInfo'), chips = $('selPaths'); info.innerHTML = ''; chips.innerHTML = ''; $('cropPreview').hidden = true;
    const sel = p.selection();
    if (sel.length > 1) {
      $('selWord').textContent = p.text(T.words(sel));
      info.innerHTML = `<b>selection</b><span>${sel.length} words · ${p.citation(sel)}</span><b>search form</b><span class="v">${p.text(T.words(sel), { form: 'search' })}</span>`;
      return;
    }
    if (!w) {
      if (S.selAyah) {
        const [s, a] = S.selAyah, ws = p.targetWords(T.ayah(s, a)), { count, isComplete } = p.ayahWordCount(s, a);
        $('selWord').textContent = p.text(T.ayah(s, a));
        info.innerHTML = `<b>ayah</b><span>${s}:${a} · ${count} words${isComplete ? '' : ' (continues on another page)'}</span><b>label</b><span>${p.ayahLabel(p.words[ws[0]].ayahIndex)}</span>`;
      } else { $('selWord').textContent = '—'; }
      return;
    }
    $('selWord').textContent = w.text;
    const rows = [['word_key', p.wordKey(w.index)], ['line', w.line], ['rasm_imlai', p.wordForm(w.index, 'rasm_imlai')], ['qpc', p.wordForm(w.index, 'qpc')], ['rasm', p.wordForm(w.index, 'rasm')], ['search', p.wordForm(w.index, 'search')], ['label', p.wordLabel(w.index)], ['paths', w.nPaths]];
    for (const [k, v] of rows) if (v !== undefined && v !== '') info.insertAdjacentHTML('beforeend', `<b>${k}</b><span class="${/rasm_imlai|qpc|rasm|search/.test(k) ? 'v' : ''}">${v}</span>`);
    for (let i = w.firstPath; i < w.firstPath + w.nPaths; i++) {
      const kind = p.pathKind(i), mark = p.pathMark(i), nth = p.pathNthMark(i);
      const label = kind === KIND.MARK ? `${engine.markName(mark)} #${nth}` : engine.kindName(kind);
      const el = document.createElement('span');
      el.className = 'chip' + (S.pathHandles.has(i) ? ' on' : '');
      el.textContent = label; el.title = `engine path ${i} · ${engine.categoryName(p.pathCategory(i)) || 'body'}`;
      el.onclick = () => {
        if (S.pathHandles.has(i)) { p.removeStyle(S.pathHandles.get(i)); S.pathHandles.delete(i); }
        else S.pathHandles.set(i, kind === KIND.MARK && nth >= 0 ? p.style(Sel.wordMark(w.index, nth), '#ef6c00', { ms: 200, layer: LAYER.TOP }) : p.style(Sel.path(i), '#ef6c00', { ms: 200, layer: LAYER.TOP }));
        el.classList.toggle('on'); draw();
      };
      chips.appendChild(el);
    }
  }
  function selectWord(i) {
    const p = S.page;
    S.selAyah = null; p.clearSelection();
    if (S.hlAyah) { p.removeHighlight(S.hlAyah); S.hlAyah = 0; }
    for (const h of S.pathHandles.values()) p.removeStyle(h); S.pathHandles.clear();
    if (i < 0 || i === S.selWord) { S.selWord = -1; if (S.hlSel) { p.removeHighlight(S.hlSel); S.hlSel = 0; } }
    else {
      S.selWord = i;
      const st = { mode: S.hlMode, ink: '#1a73e8', band: rgba('#1a73e8', 0.18), radius: 1.5, ms: S.hlMs, layer: LAYER.SELECTION };
      if (S.hlSel) p.moveHighlight(S.hlSel, T.word(i)); else S.hlSel = p.highlight(T.word(i), st);
    }
    showSelection(); draw();
  }
  function selectAyah(s, a) {
    const p = S.page;
    if (S.hlSel) { p.removeHighlight(S.hlSel); S.hlSel = 0; } S.selWord = -1; p.clearSelection();
    S.selAyah = [s, a];
    const st = { mode: S.hlMode, ink: '#0a7d32', band: rgba('#0a7d32', 0.14), radius: 1.5, ms: S.hlMs, layer: LAYER.SELECTION };
    if (S.hlAyah) p.moveHighlight(S.hlAyah, T.ayah(s, a)); else S.hlAyah = p.highlight(T.ayah(s, a), st);
    showSelection(); draw();
  }

  // ── search ──
  function runSearch() {
    const p = S.page, q = $('q').value.trim(), box = $('results'); box.innerHTML = '';
    if (S.hlSearch) { p.removeHighlight(S.hlSearch); S.hlSearch = 0; }
    if (!q) { draw(); return; }
    const m = p.search(q, { mode: $('qmode').value });
    if (m.length) S.hlSearch = p.highlight(T.words(m.map(x => x.word)), { mode: 'both', ink: '#c62828', band: rgba('#c62828', 0.12), height: 'ink', padY: 1, radius: 1, ms: S.hlMs });
    box.innerHTML = m.length ? m.map(x => `<div data-w="${x.word}">${x.text} <span class="hint">${x.wordKey}${x.isLooseMatch ? ' ~' : ''}</span></div>`).join('') : `<div class="hint">no match on this page${atlas ? ' — try the goto box for surah names' : ''}</div>`;
    box.querySelectorAll('[data-w]').forEach(el => el.onclick = () => selectWord(+el.dataset.w));
    draw();
  }
  $('q').oninput = runSearch; $('qmode').onchange = runSearch;

  // ── goto: ayah key / surah name / juz ──
  $('goto').onchange = async e => {
    const v = e.target.value.trim(); if (!v || !atlas) return;
    let m;
    if ((m = /^(\d+):(\d+)/.exec(v))) { const pg = atlas.pageOf(+m[1], +m[2]); if (pg) { await loadPage(pg); const ws = S.page.targetWords(T.ayah(+m[1], +m[2])); if (ws.length) selectAyah(+m[1], +m[2]); } return; }
    if ((m = /^juz\s*(\d+)/i.exec(v))) { const j = atlas.juz(+m[1]); if (j) await loadPage(j.page); return; }
    const su = atlas.searchSurahs(v); if (su.length) await loadPage(su[0].page);
  };

  // ── pointer: tap, drag-select, pan, pinch ──
  let drag = null, pinch = null, moved = false, selecting = null;
  const pts = new Map();
  stage.addEventListener('pointerdown', e => {
    try { stage.setPointerCapture(e.pointerId); } catch (_) {}
    pts.set(e.pointerId, [e.clientX, e.clientY]); moved = false;
    const r = stage.getBoundingClientRect(), [x, y] = toView(e.clientX - r.left, e.clientY - r.top);
    const h = S.page.hitTestView(x, y, { maxDistance: 6 });
    if (pts.size === 1) {
      drag = { x: e.clientX, y: e.clientY, offsetX: S.view.offsetX, offsetY: S.view.offsetY };
      selecting = h && h.word >= 0 && (e.shiftKey || e.pointerType !== 'touch') ? { anchor: h.word, active: false } : null;
    }
    if (pts.size === 2) { const [a, b] = [...pts.values()]; pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), scale: S.view.scale, cx: (a[0] + b[0]) / 2, cy: (a[1] + b[1]) / 2, offsetX: S.view.offsetX, offsetY: S.view.offsetY }; drag = null; selecting = null; }
  });
  stage.addEventListener('pointermove', e => {
    const r = stage.getBoundingClientRect();
    if (pts.has(e.pointerId)) pts.set(e.pointerId, [e.clientX, e.clientY]);
    if (pinch && pts.size === 2) {
      // the engine owns the pinch arithmetic; the browser only reports the fingers
      const [a, b] = [...pts.values()]; const d = Math.hypot(a[0] - b[0], a[1] - b[1]);
      const from = { scale: pinch.scale, offsetX: pinch.offsetX, offsetY: pinch.offsetY };
      S.view = engine.viewZoomAbout(from, pinch.cx - r.left, pinch.cy - r.top, d / pinch.d);
      moved = true; draw(); return;
    }
    const [x, y] = toView(e.clientX - r.left, e.clientY - r.top);
    if (drag && selecting) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (!selecting.active && Math.hypot(dx, dy) > 4) { selecting.active = true; stage.classList.add('selecting'); if (S.hlSel) { S.page.removeHighlight(S.hlSel); S.hlSel = 0; } S.selWord = -1; S.selAyah = null; if (S.hlAyah) { S.page.removeHighlight(S.hlAyah); S.hlAyah = 0; } }
      if (selecting.active) {
        const h = S.page.hitTestView(x, y, {});
        if (h && h.word >= 0) {
          S.page.select(selecting.anchor, h.word);
          const ws = S.page.selection();
          if (S.hlSel) S.page.moveHighlight(S.hlSel, T.words(ws)); else S.hlSel = S.page.highlight(T.words(ws), { mode: 'band', band: rgba('#2d6fd6', 0.25), padX: 0.6, ms: 0, layer: LAYER.SELECTION });
          moved = true; draw();
        }
        return;
      }
    }
    if (drag) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      if (Math.hypot(dx, dy) > 3) { moved = true; stage.classList.add('dragging'); S.view = engine.viewPan({ ...S.view, offsetX: drag.offsetX, offsetY: drag.offsetY }, dx, dy); draw(); }
      return;
    }
    const t = performance.now(); const h = S.page.hitTestView(x, y, { maxDistance: 4 }); S.lastHitUs = (performance.now() - t) * 1000;
    const hw = h ? h.word : -1;
    if (hw !== S.hover) { S.hover = hw; stage.style.cursor = hw >= 0 || (h && h.decoration >= 0) ? 'pointer' : 'grab'; draw(); }
  });
  const up = e => {
    pts.delete(e.pointerId);
    if (pts.size < 2) pinch = null;
    if (pts.size === 0) {
      stage.classList.remove('dragging'); stage.classList.remove('selecting');
      if (selecting && selecting.active) { showSelection(); selecting = null; drag = null; return; }
      if (drag && !moved) {
        const r = stage.getBoundingClientRect(); const [x, y] = toView(e.clientX - r.left, e.clientY - r.top);
        const t = performance.now(); const h = S.page.hitTestView(x, y, { maxDistance: 6 }); S.lastHitUs = (performance.now() - t) * 1000;
        if (h && h.word >= 0) selectWord(h.word);
        else if (h && h.decoration >= 0) { const d = S.page.decorations[h.decoration]; if (d.ayah) selectAyah(d.surah, d.ayah); }
        else selectWord(-1);
      }
      drag = null; selecting = null;
    }
  };
  stage.addEventListener('pointerup', up); stage.addEventListener('pointercancel', up);
  stage.addEventListener('lostpointercapture', e => pts.delete(e.pointerId));
  stage.addEventListener('wheel', e => {
    e.preventDefault(); const r = stage.getBoundingClientRect();
    const k = Math.exp(-e.deltaY * 0.0015);
    S.view = engine.viewZoomAbout(S.view, e.clientX - r.left, e.clientY - r.top, k); draw();
  }, { passive: false });
  stage.addEventListener('dblclick', () => fit());

  // ── copy / crop ──
  $('copy').onclick = async () => {
    const p = S.page; let text;
    if (p.selection().length) text = p.selectionText('rasm_uthmani', true);
    else if (S.selAyah) text = `${p.text(T.ayah(...S.selAyah))} (${S.selAyah[0]}:${S.selAyah[1]})`;
    else if (S.selWord >= 0) text = `${p.words[S.selWord].text} (${p.citation([S.selWord])})`;
    else return;
    try { await navigator.clipboard.writeText(text); $('copied').textContent = 'copied'; } catch { $('copied').textContent = text; }
    setTimeout(() => $('copied').textContent = '', 1500);
  };
  $('cropBtn').onclick = () => {
    const p = S.page, sel = p.selection();
    const target = sel.length ? T.words(sel) : S.selAyah ? T.ayah(...S.selAyah) : S.selWord >= 0 ? T.word(S.selWord) : null;
    if (!target) return;
    const svg = p.cropSvg(target, { pad: 3, keepAyahMarks: true, background: getComputedStyle(document.body).getPropertyValue('--paper').trim() });
    const img = $('cropPreview'); img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg); img.hidden = false;
  };

  // ── highlights options / follow words ──
  $('hlMode').onchange = e => { S.hlMode = e.target.value; if (S.selWord >= 0) { const i = S.selWord; S.selWord = -1; selectWord(i); } };
  $('hlMs').oninput = e => { S.hlMs = +e.target.value; $('hlMsVal').textContent = S.hlMs + ' ms'; };
  let timer = 0;
  function stopPlay() { S.playing = false; clearInterval(timer); timer = 0; $('play').classList.remove('on'); $('play').textContent = '▶ Follow words'; if (S.hlPlay) { S.page.removeHighlight(S.hlPlay); S.hlPlay = 0; } draw(); }
  $('play').onclick = () => {
    if (S.playing) { stopPlay(); return; }
    S.playing = true; $('play').classList.add('on'); $('play').textContent = '■ Stop';
    S.playIdx = S.selWord >= 0 ? S.selWord : 0;
    const st = { mode: S.hlMode, ink: '#d81b60', band: rgba('#d81b60', 0.14), radius: 1.5, ms: S.hlMs };
    S.hlPlay = S.page.highlight(T.word(S.playIdx), st);
    timer = setInterval(async () => {
      S.playIdx++;
      if (S.playIdx >= S.page.nWords) { const next = src.pages[src.pages.indexOf(S.n) + 1]; if (next) { await loadPage(next); S.playing = true; $('play').classList.add('on'); $('play').textContent = '■ Stop'; S.playIdx = 0; S.hlPlay = S.page.highlight(T.word(0), st); } else stopPlay(); return; }
      S.page.moveHighlight(S.hlPlay, T.word(S.playIdx)); draw();
    }, 320);
    draw();
  };

  // ── styling toggles (each is one engine handle) ──
  function applyToggles() {
    const p = S.page;
    if (S.tajwidHandle) { p.removeStyle(S.tajwidHandle); S.tajwidHandle = 0; }
    if ($('tajwid').classList.contains('on')) S.tajwidHandle = p.theme({ marks: {}, ms: 200, ...Object.fromEntries([]) , diacritics: PALETTE[CATEGORY.HARAKAH], dots: PALETTE[CATEGORY.LETTER_DOT], waqf: PALETTE[CATEGORY.WAQF], sifr: PALETTE[CATEGORY.DABT] });
    if (S.hideHandle) { p.removeStyle(S.hideHandle); S.hideHandle = 0; }
    if ($('hideMarks').classList.contains('on')) S.hideHandle = p.hide(Sel.kind(KIND.MARK));
    if (S.ayahMarksHandle) { p.removeStyle(S.ayahMarksHandle); S.ayahMarksHandle = 0; }
    if ($('ayahMarks').classList.contains('on')) S.ayahMarksHandle = p.style(Sel.decoration(DECORATION.AYAH_MARK), '#b8860b', { ms: 300, layer: LAYER.THEME + 1 });
    draw();
  }
  for (const id of ['tajwid', 'hideMarks', 'ayahMarks']) $(id).onclick = () => { $(id).classList.toggle('on'); applyToggles(); };
  $('clear').onclick = () => { for (const id of ['tajwid', 'hideMarks', 'ayahMarks']) $(id).classList.remove('on'); S.page.clearStyles(); S.page.clearHighlights(); S.page.unmask(); S.page.revealStop(); S.page.clearSelection(); S.hlSel = S.hlAyah = S.hlSearch = S.hlPlay = 0; S.themeHandle = S.tajwidHandle = S.hideHandle = S.ayahMarksHandle = 0; S.pathHandles.clear(); S.selWord = -1; S.selAyah = null; S.revealOn = false; $('revealMode').classList.remove('on'); $('revealPos').disabled = true; $('q').value = ''; $('results').innerHTML = ''; stopPlay(); applyTheme(); showSelection(); draw(); };
  function applyTheme() {
    const p = S.page; if (!p) return;
    document.body.setAttribute('data-qvp-theme', S.theme);
    p.setDefaultColor($('ink').value);
    renderer.baseKey = ''; draw();
  }
  $('ink').oninput = applyTheme;
  $('theme').onchange = e => { S.theme = e.target.value; $('ink').value = INK[S.theme]; applyTheme(); };
  $('legend').innerHTML = Object.entries(PALETTE).slice(0, 5).map(([c, col]) => `<span class="hint"><i class="sw" style="background:${col}"></i>${engine.categoryName(+c)}</span>`).join('');

  // ── memorisation ──
  $('maskAyah').onclick = () => { const p = S.page; const t = S.selAyah ? T.ayah(...S.selAyah) : S.selWord >= 0 ? T.ayah(p.words[S.selWord].surah, p.words[S.selWord].ayah) : T.page(); p.maskOptions({ blockColor: getComputedStyle(document.body).getPropertyValue('--line').trim() }); p.mask(t, $('maskMode').value); draw(); };
  $('unmaskNext').onclick = () => { S.page.unmaskNext(1); draw(); };
  $('maskBack').onclick = () => { S.page.maskBack(1); draw(); };
  $('unmask').onclick = () => { S.page.unmask(); draw(); };
  $('revealMode').onclick = () => {
    const p = S.page; S.revealOn = !S.revealOn; $('revealMode').classList.toggle('on', S.revealOn);
    if (S.revealOn) { const steps = p.revealStart({ lit: 2, grey: S.theme === 'dark' ? '#4a4f57' : '#c9c4b8', ink: $('ink').value, ms: 150 }); const r = $('revealPos'); r.max = steps - 1; r.value = -1; r.disabled = false; $('revealVal').textContent = `−/${steps}`; }
    else { p.revealStop(); $('revealPos').disabled = true; $('revealVal').textContent = ''; }
    draw();
  };
  $('revealPos').oninput = e => { S.page.revealGoto(+e.target.value); $('revealVal').textContent = `${+e.target.value + 1}/${S.page.revealStepCount()}`; draw(); };

  // ── layout ──
  const relayoutUI = () => { $('spacingVal').textContent = '×' + S.layout.lineSpacing.toFixed(2); fit(); };
  $('spacing').oninput = e => { S.layout.lineSpacing = +e.target.value; S.layout.fillHeight = false; $('fillH').classList.remove('on'); relayoutUI(); };
  $('fillH').onclick = () => { S.layout.fillHeight = !S.layout.fillHeight; $('fillH').classList.toggle('on', S.layout.fillHeight); relayoutUI(); };
  $('fitGap').onclick = () => { const p = S.page, r = stage.getBoundingClientRect(); S.layout.fillHeight = false; $('fillH').classList.remove('on'); S.layout.lineSpacing = p.layoutLineSpacingToFill(layoutSpec()); $('spacing').value = S.layout.lineSpacing; relayoutUI(); };
  $('padTop').oninput = e => { S.layout.padTop = +e.target.value; $('padTopVal').textContent = e.target.value; relayoutUI(); };
  $('padBottom').oninput = e => { S.layout.padBottom = +e.target.value; $('padBottomVal').textContent = e.target.value; relayoutUI(); };

  // ── reflow ──
  const reflowUI = () => {
    clampReflowZoom();
    $('reflow').classList.toggle('on', S.reflow.on);
    $('rZoomVal').textContent = '×' + S.reflow.zoom.toFixed(2);
    $('rGapVal').textContent = '×' + S.reflow.wordGap.toFixed(2);
    $('rRelaxVal').textContent = (100 * S.reflow.relax).toFixed(0) + '%';
    $('rStretchVal').textContent = '×' + S.reflow.maxStretch.toFixed(1);
    fit();
    const L = S.page && S.page.currentLayout;
    $('reflowRows').textContent = L && L.reflowed ? `${L.rows} rows · ${Math.round(L.contentH)} px tall · max zoom ×${(+$('rZoom').max).toFixed(2)}` : 'off · as printed';
  };
  $('reflow').onclick = () => { S.reflow.on = !S.reflow.on; reflowUI(); };
  // the engine says how far this page can zoom before a word outgrows its row
  const clampReflowZoom = () => {
    const max = S.page ? S.page.reflowMaxZoom(layoutSpec()) : 4;
    $('rZoom').max = Math.min(4, Math.max(1, max)).toFixed(2);
    if (S.reflow.zoom > +$('rZoom').max) { S.reflow.zoom = +$('rZoom').max; $('rZoom').value = S.reflow.zoom; }
  };
  $('rZoom').oninput = e => { S.reflow.zoom = +e.target.value; S.reflow.on = true; reflowUI(); };
  $('rGap').oninput = e => { S.reflow.wordGap = +e.target.value; S.reflow.on = true; reflowUI(); };
  $('rFill').onchange = e => { S.reflow.fill = e.target.value; S.reflow.on = true; reflowUI(); };
  $('rGaps').onchange = e => { S.reflow.gaps = e.target.value; S.reflow.on = true; reflowUI(); };
  $('rBreaks').onchange = e => { S.reflow.breaks = e.target.value; S.reflow.on = true; reflowUI(); };
  $('rRelax').oninput = e => { S.reflow.relax = +e.target.value; S.reflow.on = true; reflowUI(); };
  $('rStretch').oninput = e => { S.reflow.maxStretch = +e.target.value; S.reflow.on = true; reflowUI(); };

  // ── navigation ──
  $('prev').onclick = () => loadPage(src.pages[Math.max(0, src.pages.indexOf(S.n) - 1)]);
  $('next').onclick = () => loadPage(src.pages[Math.min(src.pages.length - 1, src.pages.indexOf(S.n) + 1)]);
  $('pageNo').onchange = e => loadPage(+e.target.value);
  $('fit').onclick = () => fit();
  $('pageMax').textContent = src.pages[src.pages.length - 1];
  window.addEventListener('keydown', e => { if (e.target.tagName === 'INPUT') return; if (e.key === 'ArrowLeft') $('next').click(); if (e.key === 'ArrowRight') $('prev').click(); });

  function hud() {
    const p = S.page, s = renderer.stats, L = p.currentLayout;
    $('hud').textContent =
      `wasm engine   ${(wasmBytes.length / 1024).toFixed(0)} KB, init ${wasmMs.toFixed(1)} ms${atlas ? ' · atlas loaded' : ''}\n` +
      `page ${pad(S.n)}      ${(S.bytes / 1024).toFixed(0)} KB, load+decode ${S.loadMs.toFixed(2)} ms\n` +
      `content       ${p.nWords} words · ${p.nPaths} paths · ${p.nAyahs} ayah parts · ${p.nLines} lines\n` +
      `base layer    ${s.basePaths} paths in ${s.baseMs.toFixed(2)} ms (cached)\n` +
      `overlay       ${s.overlayPaths} styled paths + ${s.bands} band boxes in ${s.overlayMs.toFixed(2)} ms\n` +
      `hit-test      ${S.lastHitUs.toFixed(1)} µs (gap-aware, wasm)\n` +
      `styles        ${p.styleHandles().length} handles · ${p.highlightHandles().length} highlights${S.animating ? ' · animating' : ''}\n` +
      `reflow        ${S.reflow.on ? `on · zoom ×${S.reflow.zoom.toFixed(2)} · ${S.reflow.fill} · ${S.reflow.breaks} breaks · ${S.reflow.gaps} gaps ×${S.reflow.wordGap.toFixed(2)} · relax ${(100 * S.reflow.relax).toFixed(0)}% · gap cap ×${S.reflow.maxStretch.toFixed(1)} · ${L ? L.rows : 0} rows` : 'off (as printed)'}\n` +
      `layout        ${S.layout.fillHeight ? 'fill height' : 'spacing ×' + S.layout.lineSpacing.toFixed(2)} · lineSpacing ${(L ? L.lineSpacing : 0).toFixed(1)} u · pad ${S.layout.padTop}/${S.layout.padBottom}\n` +
      `zoom          ${(S.view.scale * (L ? L.scale : 1) * dpr).toFixed(2)}× device px per unit`;
  }

  window.__qvp = { S, engine, atlas, renderer, loadPage, fit, selectWord, selectAyah, draw };
  const prefersDark = document.documentElement.dataset.theme === 'dark' || (document.documentElement.dataset.theme !== 'light' && matchMedia('(prefers-color-scheme: dark)').matches);
  if (prefersDark) { S.theme = 'dark'; $('theme').value = 'dark'; $('ink').value = INK.dark; }
  window.addEventListener('resize', resize);
  await loadPage(src.pages[0]);
  resize();
  const q = new URLSearchParams(location.search).get('p'); if (q) loadPage(+q);
})();
