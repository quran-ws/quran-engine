// The Arabic showcase: six scenes, each a live mushaf page that the engine draws.
// The engine decides everything on the page: layout, zoom, hit tests, colours, masks. This
// file chooses which call to make, loads the data, and writes the words beside the page.
(async function () {
  'use strict';
  const { QvpEngine, CanvasRenderer, Sel, T, LAYER, DEFAULTS, rgba } = window.QVP;
  const $ = id => document.getElementById(id);
  const CFG = window.QVP_DEMO || {};
  // The language of the page: ?lang=, then the reader's last choice, then Arabic. The page is
  // written in Arabic. en.js maps each Arabic string to its English one, with {} where a
  // value goes. A string that the map does not hold stays Arabic and shows in the console.
  const EN = window.DEMO_EN || {};
  const LANG = (() => {
    let kept = null; try { kept = localStorage.getItem('qvp.demo.lang'); } catch { /* no storage */ }
    const lang = (new URLSearchParams(location.search).get('lang') || kept) === 'en' ? 'en' : 'ar';
    try { localStorage.setItem('qvp.demo.lang', lang); } catch { /* no storage */ }
    return lang;
  })();
  const arabic = s => typeof s === 'string' && /[\u0621-\u064a]/.test(s);
  // a string of this file. `quiet` is for text that can be data, such as a word of the Quran.
  const tr = (s, quiet = false) => {
    if (LANG !== 'en' || !arabic(s)) return s;
    if (EN[s] === undefined) { if (!quiet) console.warn('no English for:', s); return s; }
    return EN[s];
  };
  // a string with values in it: tt`الآية ${n}` looks up "الآية {}" and puts the values back
  const tt = (parts, ...vals) => { let i = 0; return tr(parts.join('{}')).replace(/\{\}/g, () => vals[i++]); };
  // The page's own text: each text node and each title, label and placeholder.
  if (LANG === 'en') {
    document.documentElement.lang = 'en'; document.documentElement.dir = 'ltr';
    document.title = tr(document.title);
    const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const key = n.nodeValue.trim().replace(/\s+/g, ' ');
      // data-en gives an element its own English, where one Arabic word has two meanings in the page
      if (arabic(key)) n.nodeValue = n.nodeValue.replace(n.nodeValue.trim(), n.parentElement.dataset.en || tr(key));
    }
    for (const e of document.querySelectorAll('[title], [aria-label], [placeholder]')) for (const a of ['title', 'aria-label', 'placeholder']) if (e.hasAttribute(a)) e.setAttribute(a, tr(e.getAttribute(a)));
  }
  // the other language, one click away
  { const a = $('langSwitch'); a.href = LANG === 'en' ? '?lang=ar' : '?lang=en'; a.textContent = LANG === 'en' ? 'العربية' : 'English'; a.lang = LANG === 'en' ? 'ar' : 'en'; a.dir = LANG === 'en' ? 'rtl' : 'ltr'; }
  const pad = n => String(n).padStart(3, '0');
  const bytes = async url => { const r = await fetch(url); if (!r.ok) throw new Error(url + ' ' + r.status); return new Uint8Array(await r.arrayBuffer()); };
  const maybe = async url => { try { return await bytes(url); } catch { return null; } };
  const json = async url => { const r = await fetch(url); if (!r.ok) throw new Error(url + ' ' + r.status); return r.json(); };
  // a word that is only the number of its ayah has no letters to colour, time or recite
  const isNumber = s => !s || /^[٠-٩۝\s]+$/.test(s);
  const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = tr(text, true); return e; };
  // a card's content: a large line, then small lines. Text only, never markup from data.
  function card(id, big, ...subs) {
    const c = $(id); c.textContent = '';
    if (big) c.append(el('div', 'big', big));
    // a small line is text, or [label, words of the Quran]: the words take the mushaf's typeface
    for (const s of subs) {
      if (!s) continue;
      const d = c.appendChild(el('div', 'sub'));
      if (Array.isArray(s)) d.append(tr(s[0], true), el('span', 'q', s[1])); else d.textContent = tr(s, true);
    }
  }

  const engine = await QvpEngine.init(await bytes('qvp_ffi.wasm'));
  const atlasBytes = await maybe('pages/atlas.qva');
  const atlas = atlasBytes ? engine.loadAtlas(atlasBytes) : null;
  // A build can hold some pages only. Then pages/index.json lists them, and each scene stays
  // inside the list. With no list, the build has all 604 pages.
  const listed = await json('pages/index.json').then(d => d.pages.map(Number).sort((a, b) => a - b)).catch(() => null);
  const has = n => !listed || listed.includes(n);
  const THEMES = { light: ['#fffdf7', '#231f20'], sepia: ['#f3e7cf', '#3b2a14'], dark: ['#1e2126', '#e8e4dc'] };

  // ── a stage: one page on the screen, with the reader's zoom and pan ──
  class Stage {
    constructor(host, { zoomMode = 'magnify', onTap = null, onLoad = null, hint = '' } = {}) {
      this.frameEl = host; this.onTap = onTap; this.onLoad = onLoad;
      // the bed is the page's own area, inside the frame, so a page at rest has air around it
      host = this.host = host.appendChild(el('div', 'bed'));
      this.paper = host.appendChild(el('div', 'paper'));
      this.canvas = host.appendChild(document.createElement('canvas'));
      this.renderer = new CanvasRenderer(this.canvas);
      this.dpr = Math.min(Math.max(2, window.devicePixelRatio || 1), 3);
      this.page = null; this.n = 0; this.letters = false; this.ink = THEMES.light[1];
      this.view = { scale: 1, offsetX: 0, offsetY: 0 }; this.fitScale = 1;
      this.zoom = { mode: zoomMode, step: 0, zoom: 1 };
      this.raf = 0; this.visible = false; this.loading = null; this.onZoom = null;
      if (hint) this.frameEl.appendChild(el('div', 'hint', hint));
      const z = this.frameEl.appendChild(el('div', 'zoom'));
      for (const [label, title, fn] of [['+', tr('تكبير'), () => this.nudge(1)], ['−', tr('تصغير'), () => this.nudge(-1)], ['⤢', tr('ملاءمة الصفحة'), () => this.reset()]]) {
        const b = z.appendChild(el('button', '', label)); b.title = title; b.setAttribute('aria-label', title); b.onclick = fn;
      }
      new ResizeObserver(() => this.resize()).observe(host);
      new IntersectionObserver(es => { this.visible = es[0].isIntersecting; if (this.visible) this.draw(); }, { rootMargin: '200px' }).observe(host);
      this.pointers();
    }
    spec() {
      const r = this.host.getBoundingClientRect();
      return { viewportW: r.width, viewportH: r.height, padTop: 20, padBottom: 20, padLeft: 16, padRight: 16, lineSpacing: 1, fillHeight: false, surahFrames: true, maxAspectSlack: DEFAULTS.ASPECT_SLACK, reflow: null };
    }
    // the layout spec with the reader's zoom in it: the engine adds the reflow
    zspec() { return this.page.zoomSpec(this.spec(), this.zoom); }
    async load(n) {
      // one load at a time: a second request waits for the first
      while (this.loading) await this.loading;
      if (n === this.n && this.page) return this.page;
      this.loading = (async () => {
        const [pb, wb, lb] = await Promise.all([bytes(`pages/${pad(n)}.qvp`), maybe(`pages/${pad(n)}.words.json`), maybe(`letters/${pad(n)}.qvl`)]);
        const p = engine.loadPage(pb);
        if (wb) p.attachWords(wb);
        // the layer adds its pieces to the path table, so it goes on before the paths are built
        this.letters = !!lb && p.attachLetters(lb);
        p.buildPaths(); p.setDefaultColor(this.ink);
        if (this.page) this.page.free();
        this.page = p; this.n = n;
        this.zoom = p.zoomCarried(this.spec(), this.zoom);
        this.fit();
      })();
      try { await this.loading; } finally { this.loading = null; }
      if (this.onLoad) this.onLoad(this.page);
      this.draw();
      return this.page;
    }
    fit() {
      if (!this.page) return;
      const L = this.page.layout(this.zspec());
      this.view = { scale: L.fitScale, offsetX: L.fitX, offsetY: L.fitY }; this.fitScale = L.fitScale;
      this.renderer.baseKey = ''; this.zoomed(); this.draw();
    }
    resize() {
      const r = this.host.getBoundingClientRect(); if (!r.width || !r.height) return;
      this.canvas.width = Math.round(r.width * this.dpr); this.canvas.height = Math.round(r.height * this.dpr);
      this.canvas.style.width = r.width + 'px'; this.canvas.style.height = r.height + 'px';
      this.fit();
    }
    setTheme(name) {
      const [paper, ink] = THEMES[name]; this.ink = ink; this.paper.style.background = paper;
      if (this.page) { this.page.setDefaultColor(ink); this.renderer.baseKey = ''; this.draw(); }
    }
    // the page is larger than its resting fit: then a drag moves the page, not the screen
    zoomed() {
      const z = this.zoom.step > 0 || this.zoom.zoom > 1.01 || this.view.scale > this.fitScale * 1.01;
      this.host.classList.toggle('zoomed', z); this.frameEl.classList.toggle('zoomed', z); this.host.style.touchAction = z ? 'none' : 'pan-y';
      if (this.onZoom) this.onZoom(this.zoom);
      return z;
    }
    clamp(v) { return this.page.currentLayout ? this.page.dressClamp(v, this.host.clientWidth, this.host.clientHeight) : v; }
    // a zoom about a point of the stage: the engine picks the step, lays out, and holds the place
    pinch(factor, x, y, zoom = this.zoom, view = this.view) {
      const c = this.page.zoomPinch(this.page.zoomSpec(this.spec(), zoom), zoom, view, factor, x, y);
      this.view = c.view; if (c.relaid) this.zoom = c.zoom;
      this.zoomed(); this.draw(); return c;
    }
    toStep(step) {
      const c = this.page.zoomToStep(this.zspec(), this.zoom, step, this.view);
      this.zoom = c.zoom; this.view = c.view; this.renderer.baseKey = ''; this.zoomed(); this.draw();
    }
    nudge(dir) {
      if (!this.page) return;
      if (this.zoom.mode === 'stepped') this.toStep(Math.min(3, Math.max(0, this.zoom.step + dir)));
      else this.pinch(dir > 0 ? 1.3 : 1 / 1.3, this.host.clientWidth / 2, this.host.clientHeight / 2);
    }
    reset() { if (!this.page) return; this.zoom = { mode: this.zoom.mode, step: 0, zoom: 1 }; this.fit(); }
    setZoomMode(mode) { this.zoom = this.page.zoomMode(this.zspec(), this.zoom, mode); this.fit(); }
    // a point of the stage as a point of the laid-out page: the engine inverts the view
    at(e) { const r = this.host.getBoundingClientRect(), q = this.page.viewToLayout(this.view, e.clientX - r.left, e.clientY - r.top); return [q.x, q.y]; }
    draw() { if (!this.raf && this.visible && this.page) this.raf = requestAnimationFrame(t => this.frame(t)); }
    frame(now) {
      this.raf = 0; const p = this.page; if (!p) return;
      const moving = p.tick(now), v = this.view, L = p.currentLayout, s = this.paper.style;
      s.left = v.offsetX + 'px'; s.top = v.offsetY + 'px';
      s.width = (L ? L.contentW : p.width) * v.scale + 'px'; s.height = (L ? L.contentH : p.height) * v.scale + 'px';
      this.renderer.draw(p, v, this.dpr);
      if (moving) this.draw();
    }
    pointers() {
      const h = this.host, pts = new Map(); let drag = null, pinch = null, moved = false;
      h.addEventListener('pointerdown', e => {
        if (!this.page) return;
        pts.set(e.pointerId, [e.clientX, e.clientY]); moved = false;
        const r = h.getBoundingClientRect();
        if (pts.size === 1) drag = { x: e.clientX, y: e.clientY, view: this.view };
        if (pts.size === 2) {
          const [a, b] = [...pts.values()];
          pinch = { zoom: this.zoom, d: Math.hypot(a[0] - b[0], a[1] - b[1]), view: this.view, base: this.view, fx: (a[0] + b[0]) / 2 - r.left, fy: (a[1] + b[1]) / 2 - r.top };
          drag = null; h.style.touchAction = 'none';
        }
      });
      h.addEventListener('pointermove', e => {
        if (!this.page || !pts.has(e.pointerId)) return;
        pts.set(e.pointerId, [e.clientX, e.clientY]);
        if (pinch && pts.size === 2) {
          const [a, b] = [...pts.values()], d = Math.hypot(a[0] - b[0], a[1] - b[1]);
          // the factor is the whole gesture's, from the control and the view the pinch began on
          const c = this.pinch(d / pinch.d, pinch.fx, pinch.fy, pinch.zoom, pinch.zoom.mode === 'magnify' ? pinch.view : pinch.base);
          pinch.base = c.view; moved = true; return;
        }
        if (drag && Math.hypot(e.clientX - drag.x, e.clientY - drag.y) > 4) {
          moved = true;
          if (!h.classList.contains('zoomed')) return;
          try { h.setPointerCapture(e.pointerId); } catch { /* the drag still works without capture */ }
          h.classList.add('dragging');
          this.view = this.clamp(engine.viewPan(drag.view, e.clientX - drag.x, e.clientY - drag.y)); this.draw();
        }
      });
      const up = e => {
        const had = pts.delete(e.pointerId);
        if (pts.size < 2) pinch = null;
        if (pts.size === 0) {
          h.classList.remove('dragging');
          if (had && drag && !moved && e.type === 'pointerup' && this.page && this.onTap) this.onTap(...this.at(e), e);
          drag = null; this.zoomed();
        }
      };
      h.addEventListener('pointerup', up); h.addEventListener('pointercancel', up);
      // The wheel belongs to the screen. With Ctrl (a trackpad pinch sends it) it zooms the
      // page, and on a zoomed page it moves the page until the page reaches its end.
      h.addEventListener('wheel', e => {
        if (!this.page) return;
        const r = h.getBoundingClientRect();
        if (e.ctrlKey || e.metaKey) { e.preventDefault(); this.pinch(Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top); return; }
        if (!h.classList.contains('zoomed')) return;
        const v = this.clamp(engine.viewPan(this.view, -e.deltaX, -e.deltaY));
        if (Math.abs(v.offsetY - this.view.offsetY) < 0.5 && Math.abs(v.offsetX - this.view.offsetX) < 0.5) return;
        e.preventDefault(); this.view = v; this.draw();
      }, { passive: false });
    }
  }

  // The words of the page that carry text, with their place in the ayah: "surah:ayah:word".
  function wordIndex(p) {
    const by = new Map(), list = [];
    for (const w of p.words) {
      const qpc = p.wordForm(w.index, 'qpc');
      if (isNumber(qpc)) continue;
      by.set(`${w.surah}:${w.ayah}:${w.word}`, w.index); list.push({ index: w.index, surah: w.surah, ayah: w.ayah, word: w.word, qpc });
    }
    return { by, list };
  }

  // A scene starts when the reader comes near it, so the first screen opens at once.
  const scenes = [];
  function scene(id, start) {
    let done = false;
    scenes.push(id);
    new IntersectionObserver((es, o) => {
      if (done || !es[0].isIntersecting) return;
      done = true; o.disconnect();
      start().catch(e => { console.error(id, e); const host = $(id).querySelector('.stage'); host.appendChild(el('div', 'note', tr('تعذّر تحميل هذا القسم: ') + e.message)); });
    }, { rootMargin: '400px' }).observe($(id));
  }

  // ── 1. the opening: the page writes itself, and every word answers a tap ──
  scene('hero', async () => {
    let hl = 0, timer = 0;
    const st = new Stage($('heroStage'), {
      onTap: (x, y) => {
        const p = st.page, h = p.hitTestView(x, y, { maxDistance: 6 });
        if (!h || h.word < 0) { if (hl) { p.removeHighlight(hl); hl = 0; } card('heroCard', '', tr('اضغط على كلمة لعرض بياناتها.')); st.draw(); return; }
        const style = { mode: 'both', ink: '#9a6b00', band: rgba('#c9a44c', 0.28), radius: 1.5, ms: 220, layer: LAYER.SELECTION };
        if (hl) p.moveHighlight(hl, T.word(h.word)); else hl = p.highlight(T.word(h.word), style);
        const w = p.words[h.word], letters = st.letters ? p.letters(h.word).map(l => l.text).join(' · ') : '';
        card('heroCard', p.wordForm(h.word, 'qpc') || w.text, tt`السورة ${w.surah}، الآية ${w.ayah}، الكلمة ${w.word}`, letters && [tr('الحروف: '), letters]);
        st.draw();
      },
    });
    // write the page: hide it, then the engine shows one more word at each step
    const write = () => {
      const p = st.page; clearInterval(timer);
      if (hl) { p.removeHighlight(hl); hl = 0; }
      p.unmask(); p.mask(T.page(), 'hide'); st.draw();
      timer = setInterval(() => { if (!p.unmaskNext(2)) { clearInterval(timer); p.unmask(); } st.draw(); }, 40);
    };
    await st.load(42);
    $('heroWrite').onclick = write;
    write();
  });

  // ── 2. tajwid: each ruling on its own letters ──
  // The colour classes of quranpedia.net (resources/css/pages/tajweed.css), and the rulings
  // of quran-tajweed that go under each. A ruling goes under a class only when the class
  // names it. So `madd_al_iwad`, which is read at a stop only, and `madd_al_badal` are not under
  // the natural madd, and they keep the page's ink. A tap still names each of them.
  const TAJWID = [
    ['ghn', tr('غنّة'), '#FF7E1E', ['noon-mushaddadah', 'meem-mushaddadah']],
    ['qlq', tr('قلقلة'), '#DD0008', ['qalqalah-']],
    ['idgh_ghn', tr('إدغام بغنّة'), '#169200', ['idgham-bi-ghunnah']],
    ['idgh_w_ghn', tr('إدغام بغير غنّة'), '#0E6B00', ['idgham-bila-ghunnah']],
    ['iqlb', tr('إقلاب'), '#26BFFD', ['iqlab-']],
    ['ikhf', tr('إخفاء'), '#9400A8', ['ikhfa-haqiqi', 'ikhfa-ghunnah']],
    ['ikhf_shfw', tr('إخفاء شفوي'), '#D500B7', ['ikhfa-shafawi', 'mutajanisain-ikhfa-shafawi']],
    ['idghm_shfw', tr('إدغام شفوي'), '#58B800', ['idgham-shafawi']],
    ['madda_necessary', tr('مدّ لازم'), '#000EBC', ['madd-lazim']],
    ['madda_obligatory', tr('مدّ واجب متّصل'), '#2144C1', ['madd-muttasil']],
    ['madda_permissible', tr('مدّ جائز'), '#4050FF', ['madd-munfasil', 'madd-arid', 'madd-silah-kubra']],
    ['madda_normal', tr('مدّ طبيعي'), '#537FFF', ['madd-tabee']],
    ['idgh_mus', tr('إدغام متماثلين ومتجانسين'), '#A1A1A1', ['mutamathilain-idgham', 'mutajanisain-idgham']],
  ];
  // quran-ws/quran-tajweed at the commit that gave each span its focus: the letters that the
  // ruling is about. The npm package 0.1.0 is older and has no focus.
  const TAJWID_AT = 'https://cdn.jsdelivr.net/gh/quran-ws/quran-tajweed@b8e45f4e0a10fc72ffb31568db0e0309d5195c05/packages/';
  const TAJWID_DATA = TAJWID_AT + 'annotations/uthmani-hafs.json';
  const TAJWID_RULINGS = TAJWID_AT + 'rules/rules.json';
  scene('tajwid', async () => {
    const off = new Set(); let marks = [], handles = new Map(), picked = 0;
    // The build can name another file of rulings (QVP_TAJWID). A span covers the letter and
    // its condition, and its focus is the letters that the ruling is about. The scene colours
    // the focus.
    const [ann, rules] = await Promise.all([json('tajwid.json').catch(() => json(TAJWID_DATA)), json(TAJWID_RULINGS).catch(() => null)]);
    const family = ann.ruleIds.map(id => TAJWID.findIndex(f => f[3].some(start => id.startsWith(start))));
    const label = new Map((rules && rules.rules || []).map(r => [r.id, r.label && r.label.ar]));
    // the corpus's own name of each ruling: the card shows it, so a ruling is never renamed here
    const rulingName = new Map((rules && rules.hukums || []).map(h => [h.id, h.label && h.label.ar]));
    const st = new Stage($('tjStage'), {
      onLoad: p => {
        marks = [];
        if (!st.letters) { $('tjStage').appendChild(el('div', 'hint', tr('لا تتوفر طبقة الحروف لهذه الصفحة، ولا يمكن تلوين الحروف منفردة.'))); return; }
        const { list } = wordIndex(p), ayahs = new Map();
        for (const w of list) { const k = `${w.surah}:${w.ayah}`; if (!ayahs.has(k)) ayahs.set(k, []); ayahs.get(k).push(w); }
        for (const [key, ws] of ayahs) {
          const [s, a] = key.split(':').map(Number);
          // the spans count code points from the start of the ayah, so the ayah must be whole
          if (!p.ayahWordCount(s, a).isComplete || !ann.spans[key]) continue;
          let at = 0; const starts = ws.map(w => { const from = at; at += [...w.qpc].length + 1; return from; });
          for (const [start, end, rule, fs = start, fe = end] of ann.spans[key]) {
            ws.forEach((w, i) => {
              const from = Math.max(fs, starts[i]) - starts[i], to = Math.min(fe, starts[i] + [...w.qpc].length) - 1 - starts[i];
              if (from <= to) marks.push({ word: w.index, from, to, rule, fam: family[rule] });
            });
          }
        }
        // The corpus marks the natural madd on every madd letter, also where another madd
        // applies to the same letter, such as `madd_munfasil` or `madd_muttasil`. The other
        // madd is the ruling of that letter, so the natural madd gives way to it.
        const isMadd = m => ann.ruleIds[m.rule].startsWith('madd-'), natural = m => ann.ruleIds[m.rule].startsWith('madd-tabee');
        const others = marks.filter(m => isMadd(m) && !natural(m));
        marks = marks.filter(m => !(natural(m) && others.some(o => o.word === m.word && o.from <= m.to && o.to >= m.from)));
        paint();
      },
      onTap: (x, y) => {
        const p = st.page, h = p.hitTestLetterView(x, y, { maxDistance: 6 });
        if (picked) { p.removeHighlight(picked); picked = 0; }
        const here = h && h.at >= 0 ? marks.filter(m => m.word === h.word && h.at >= m.from && h.at <= m.to) : [];
        if (!here.length) { card('tjCard', '', h ? tr('لا يوجد حكم ملوّن على هذا الحرف.') : tr('اضغط على حرف ملوّن لعرض حكمه.')); st.draw(); return; }
        // the ruling that has a colour first, then the others on the same letter
        here.sort((a, b) => (b.fam >= 0) - (a.fam >= 0));
        const m = here[0], id = ann.ruleIds[m.rule];
        picked = p.highlight(T.word(h.word), { mode: 'band', band: rgba('#c9a44c', 0.2), radius: 1.5, ms: 180 });
        const inWords = r => ann.ruleIds[r].split('.')[0].replace(/-/g, ' ');
        const nameOf = x => (LANG === 'en' ? (x.fam >= 0 ? TAJWID[x.fam][1] : inWords(x.rule)) : rulingName.get(ann.ruleIds[x.rule].split('.')[0]));
        const own = nameOf(m) || (m.fam >= 0 ? TAJWID[m.fam][1] : tr('حكم'));
        // the other rulings on the same letter, by name
        const more = [...new Set(here.slice(1).map(nameOf).filter(Boolean))].filter(x => x !== own);
        card('tjCard', '', label.get(id) || '', [tr('الكلمة: '), p.wordForm(h.word, 'qpc')], more.length ? tr('أحكام أخرى على الحرف: ') + more.join('، ') : '');
        $('tjCard').prepend(el('div', 'name', own));
        st.draw();
      },
    });
    function paint() {
      const p = st.page;
      for (const hs of handles.values()) for (const h of hs) p.removeStyle(h);
      handles = new Map();
      for (const m of marks) {
        if (m.fam < 0 || off.has(m.fam)) continue;
        const h = p.style(Sel.wordChars(m.word, m.from, m.to), TAJWID[m.fam][2], { ms: 260, layer: LAYER.THEME });
        if (!handles.has(m.fam)) handles.set(m.fam, []);
        handles.get(m.fam).push(h);
      }
      for (const c of $('tjLegend').children) c.classList.toggle('off', off.has(+c.dataset.fam));
      st.draw();
    }
    TAJWID.forEach(([, name, colour], i) => {
      const c = el('button', 'chip'); c.dataset.fam = i; c.append(el('i'), name); c.firstChild.style.background = colour;
      c.onclick = () => { if (off.has(i)) off.delete(i); else off.add(i); paint(); };
      $('tjLegend').append(c);
    });
    $('tjAll').onclick = () => { off.clear(); paint(); };
    $('tjNone').onclick = () => { TAJWID.forEach((_, i) => off.add(i)); paint(); };
    await st.load(440);
  });

  // ── 3. recitation: the page follows the reciter ──
  const API = CFG.mp3quran || 'https://api-staging.mp3quran.net';
  const SURAH = 36;
  scene('listen', async () => {
    const audio = new Audio(); audio.preload = 'none';
    let words = [], letters = null, index = { by: new Map() }, hl = 0, letterRule = 0, cur = -1, curLetter = -1, byLetter = false, repeat = false, recitation = null, turning = false;
    const clock = ms => { const s = Math.floor(ms / 1000); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`; };
    const st = new Stage($('plStage'), {
      onLoad: p => { index = wordIndex(p); hl = 0; letterRule = 0; cur = -1; curLetter = -1; },
      onTap: (x, y) => {
        const h = st.page.hitTestView(x, y, { maxDistance: 8 }); if (!h || h.word < 0) return;
        const w = st.page.words[h.word], t = words.find(v => w.surah === SURAH && v[0] === w.ayah && v[1] === w.word);
        if (t) { audio.currentTime = t[2] / 1000; audio.play(); }
      },
    });
    // the last entry that started at or before `ms`: the lists come in the order heard
    const last = (list, k, ms) => { let lo = 0, hi = list.length - 1, at = -1; while (lo <= hi) { const mid = (lo + hi) >> 1; if (list[mid][k] <= ms) { at = mid; lo = mid + 1; } else hi = mid - 1; } return at; };
    // The API counts the letters of the written word. The engine names a letter by the offset
    // of its character, so the nth written letter gives the offset, and the engine the letter.
    function engineLetter(p, word, nth) {
      let n = 0, at = 0;
      for (const ch of p.wordForm(word, 'qpc')) { if (/\p{L}/u.test(ch) && ch !== '\u0640' && ++n === nth) return p.letterAtOffset(word, at); at++; }
      return -1;
    }
    async function show(ms) {
      const p = st.page; if (!p || turning) return;
      const i = last(words, 2, ms);
      if (i < 0) return;
      const [ayah, pos] = words[i];
      // the ayah left the page: the atlas says which page holds it
      const pg = atlas ? atlas.pageOf(SURAH, ayah) : st.n;
      if (pg && pg !== st.n) {
        if (!has(pg)) { audio.pause(); card('plCard', '', tt`الصفحة ${pg} غير متوفرة في هذا العرض.`); return; }
        turning = true; try { await st.load(pg); } finally { turning = false; } return;
      }
      if (repeat && cur >= 0 && words[cur][0] !== ayah && !audio.seeking) { const a = words[cur][0], first = words.find(v => v[0] === a); cur = -1; audio.currentTime = first[2] / 1000; return; }
      const w = index.by.get(`${SURAH}:${ayah}:${pos}`); if (w === undefined) return;
      if (i !== cur) {
        cur = i;
        // by word, the ink alone takes the colour. By letter, a faint band holds the word and the letter takes the colour.
        const style = byLetter ? { mode: 'band', band: rgba('#c9a44c', 0.22), radius: 1.5, ms: 120 } : { mode: 'ink', ink: '#0a7d32', ms: 120 };
        if (hl) p.moveHighlight(hl, T.word(w)); else hl = p.highlight(T.word(w), style);
        card('plCard', p.wordForm(w, 'qpc'), tt`سورة يس، الآية ${ayah}، الكلمة ${pos}`, recitation && recitation.reciter.name);
      }
      if (byLetter && letters && st.letters) {
        const j = last(letters, 3, ms);
        if (j >= 0 && j !== curLetter && letters[j][0] === ayah && letters[j][1] === pos) {
          curLetter = j;
          const l = engineLetter(p, w, letters[j][2]);
          if (letterRule) { p.removeStyle(letterRule); letterRule = 0; }
          if (l >= 0) letterRule = p.style(Sel.letter(w, l, 'all'), '#c62828', { ms: 60, layer: LAYER.TOP });
        }
      }
      st.draw();
    }
    function loop() {
      $('plTime').textContent = clock(audio.currentTime * 1000);
      if (audio.duration) $('plSeek').value = Math.round(1000 * audio.currentTime / audio.duration);
      show(audio.currentTime * 1000);
      if (!audio.paused) requestAnimationFrame(loop);
    }
    function clearMarks() { const p = st.page; if (!p) return; if (hl) { p.removeHighlight(hl); hl = 0; } if (letterRule) { p.removeStyle(letterRule); letterRule = 0; } cur = -1; curLetter = -1; st.draw(); }
    async function pick(r) {
      audio.pause(); clearMarks(); recitation = r; words = []; letters = null;
      const at = `${API}/v4/recitations/${r.id}/surahs/${SURAH}`;
      const [info, wt] = await Promise.all([json(at), json(`${at}/word-timings`)]);
      const files = info.data.files; audio.src = (files.mp3_64 || files.original || Object.values(files)[0]).url;
      words = wt.data.words;
      $('plByLetter').disabled = !info.data.timings.includes('letter') || !st.letters;
      card('plCard', '', tt`${r.reciter.name} · سورة يس · المدة ${clock(info.data.duration_ms)}`);
      if (byLetter) await needLetters();
    }
    async function needLetters() { if (!letters && recitation) letters = (await json(`${API}/v4/recitations/${recitation.id}/surahs/${SURAH}/letter-timings`)).data.letters; }
    audio.onplay = () => { $('plPlay').textContent = tr('⏸ إيقاف مؤقت'); loop(); };
    audio.onpause = () => { $('plPlay').textContent = tr('▶ تشغيل'); };
    audio.addEventListener('seeked', () => { cur = -1; curLetter = -1; show(audio.currentTime * 1000); });
    $('plPlay').onclick = () => { if (audio.paused) audio.play(); else audio.pause(); };
    $('plSeek').oninput = e => { if (audio.duration) audio.currentTime = audio.duration * e.target.value / 1000; };
    const mode = async letter => {
      byLetter = letter; $('plByWord').classList.toggle('on', !letter); $('plByLetter').classList.toggle('on', letter);
      clearMarks(); if (letter) await needLetters(); show(audio.currentTime * 1000);
    };
    $('plByWord').onclick = () => mode(false); $('plByLetter').onclick = () => mode(true);
    $('plRepeat').onclick = () => { repeat = !repeat; $('plRepeat').classList.toggle('on', repeat); };
    await st.load(440);
    // the reciters of Hafs who have this surah with its timings
    const list = (await json(`${API}/v4/recitations?riwayah=hafs_an_asim&surah=${SURAH}&limit=40&locale=${LANG}`)).data.filter(r => r.has_ayah_timings);
    const sel = $('plReciter');
    for (const r of list) { const o = el('option', '', r.reciter.name); o.value = r.id; sel.append(o); }
    sel.onchange = () => pick(list.find(r => String(r.id) === sel.value)).catch(e => card('plCard', '', tr('تعذّر تحميل هذه التلاوة: ') + e.message));
    if (list.length) await pick(list[0]); else card('plCard', '', tr('لا توجد تلاوات متاحة حاليًا.'));
  });

  // ── 4. memorisation: hide, then reveal by a tap or by the reader's own voice ──
  scene('hifz', async () => {
    let mode = 'hide', expect = [], said = 0;
    const st = new Stage($('hzStage'), { onLoad: p => { expect = wordIndex(p).list; }, onTap: () => next() });
    const left = () => st.page.maskHidden().length;
    const status = text => card('hzCard', '', text || (left() ? tt`الكلمات المخفية: ${left()}. اضغط على الصفحة لكشف كلمة.` : tr('جميع الكلمات ظاهرة.')));
    function mask() { const p = st.page; p.unmask(); p.maskOptions({ blockColor: '#d9cfb8' }); p.mask(T.page(), mode); said = 0; st.draw(); status(); }
    function next() { st.page.unmaskNext(1); st.draw(); status(); }
    for (const [id, m] of [['hzHide', 'hide'], ['hzBlock', 'block']]) $(id).onclick = () => { mode = m; for (const b of ['hzHide', 'hzBlock']) $(b).classList.toggle('on', b === id); mask(); };
    $('hzNext').onclick = next;
    $('hzBack').onclick = () => { st.page.maskBack(1); st.draw(); status(); };
    $('hzReset').onclick = mask;
    $('hzShow').onclick = () => { st.page.unmask(); st.draw(); status(); };
    await st.load(1);
    mask();

    // The reader's voice. TajweedFormer V4 runs in a frame of Quran Lab's on this device. It
    // hears, and it reports what it heard: the sounds, and how probable each one is in each
    // frame. It does not judge. Finding the place, following the words and marking a word
    // that did not come are this demo's own work on that output.
    //
    // The whole Quran as one run of sounds, with the word of each sound. The sounds are the
    // model's own reference (`model.reference`): what a correct recitation of each ayah has.
    // The text file gives the first word of each page, ayah and surah.
    const TEXT = CFG.quranText || 'https://text.quran.ws/files/mushaf/hafs.json';
    let quran = null;
    async function wholeQuran() {
      if (quran) return quran;
      const d = await json(TEXT), BAR = model.tokens.wordBar, jobs = [];
      // the last start at or before a word: its page, its ayah or its surah, counted from 1
      const at = (starts, w) => { let lo = 0, hi = starts.length - 1; while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (starts[mid] <= w) lo = mid; else hi = mid - 1; } return lo + 1; };
      const surah = w => at(d.surah_starts, w), ayah = w => at(d.ayah_starts, w) - at(d.ayah_starts, d.surah_starts[surah(w) - 1]) + 1;
      d.ayah_starts.forEach(w => jobs.push(model.reference.units(surah(w), ayah(w))));
      const refs = await Promise.all(jobs), sounds = [], wordOf = [];
      refs.forEach((ref, i) => {
        const first = d.ayah_starts[i], words = (i + 1 < d.ayah_starts.length ? d.ayah_starts[i + 1] : d.words.length) - first;
        // The reference counts each letter name of an opening such as alif lam mim as a word,
        // and the mushaf writes them as one. The extra words are at the start of the ayah.
        const extra = ref.wordCount - words;
        for (let k = 0; k < ref.units.length; k++) if (ref.units[k] !== BAR) { sounds.push(ref.units[k]); wordOf.push(first + Math.max(0, ref.words[k] - extra)); }
      });
      const basmalah = []; for (let k = 0; k < sounds.length && wordOf[k] < 4; k++) basmalah.push(sounds[k]);
      // the isti'adhah is in no ayah, so its sounds are named here, as the model names them
      const istiadhah = 'ء a ع uu ذ u ب i ل aa ه i م i ن a ش a ي ط aa ن i ر a ج ii م'.split(' ').map(name => model.tokens.id(name));
      return (quran = { sounds: Uint8Array.from(sounds), wordOf, basmalah, istiadhah, page: w => at(d.page_starts, w), firstOf: pg => d.page_starts[pg - 1], surah, ayah,
        // the words of an ayah as the mushaf has them, for the sounds of one ayah of the reference
        shift: (s, a, ref) => ref.wordCount - ((() => { const i = at(d.ayah_starts, d.surah_starts[s - 1]) - 1 + a - 1; return (i + 1 < d.ayah_starts.length ? d.ayah_starts[i + 1] : d.words.length) - d.ayah_starts[i]; })()) });
    }
    // the edits between two runs of sounds
    function edits(a, b) {
      let row = Array.from({ length: b.length + 1 }, (_, j) => j);
      for (let i = 1; i <= a.length; i++) { const next = [i]; for (let j = 1; j <= b.length; j++) next[j] = Math.min(row[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1), row[j] + 1, next[j - 1] + 1); row = next; }
      return row[b.length];
    }
    // Every place where a run of sounds ends in the Quran with at most `k` edits (Sellers'
    // search). The model can hear one sound as another, or miss one, so an exact search
    // would lose the place. Ends that are close are one place.
    function ends(run, k) {
      const text = quran.sounds, m = run.length, col = new Int16Array(m + 1), out = [];
      for (let i = 0; i <= m; i++) col[i] = i;
      for (let j = 0; j < text.length; j++) {
        let diag = col[0];   // col[0] stays 0: a match can start anywhere
        const c = text[j];
        for (let i = 1; i <= m; i++) { const up = col[i]; let v = diag + (run[i - 1] === c ? 0 : 1); if (up + 1 < v) v = up + 1; if (col[i - 1] + 1 < v) v = col[i - 1] + 1; col[i] = v; diag = up; }
        if (col[m] <= k) { const last = out[out.length - 1]; if (last && j - last.at <= m) { if (col[m] < last.cost) { last.at = j; last.cost = col[m]; } } else out.push({ at: j, cost: col[m] }); }
      }
      return out;
    }
    // The place of what was heard: where it ends, as soon as it is in the Quran in one place
    // only. All that was heard is tried first, so a short phrase that is in one place is found
    // at once. When that is in no place, a later part is tried, because the reader can have
    // said something else first. Eight sounds are the least: fewer are in many places.
    function locate(heard) {
      for (const part of [1, 0.75, 0.5]) {
        const m = Math.min(48, Math.floor(heard.length * part));
        if (m < 8) return null;
        const found = ends(heard.slice(-m), Math.floor(m / 6));
        if (found.length === 1) return { end: found[0].at, m };
        if (found.length > 1) return null;
      }
      return null;
    }
    // The word where the recitation began: the start of the found run in the Quran, by the
    // length that fits best.
    function begin(heard, hit) {
      const run = heard.slice(-hit.m); let start = hit.end - hit.m + 1, best = Infinity;
      for (let len = hit.m - 4; len <= hit.m + 4; len++) { const from = hit.end - len + 1; if (from < 0) continue; const c = edits(run, Array.from(quran.sounds.subarray(from, hit.end + 1))) + Math.abs(len - hit.m) * 0.01; if (c < best) { best = c; start = from; } }
      // a word counts from its first sound
      let w = quran.wordOf[start]; if (start > 0 && quran.wordOf[start - 1] === w) w++;
      return w;
    }
    // The isti'adhah and the basmalah come before a recitation and are not its place: the
    // basmalah is before every surah. They are cut from the start of what was heard. The
    // answer is null while what was heard can still be only the start of one of them.
    function afterOpening(heard) {
      let cut = 0, basmalah = false;
      for (const formula of [quran.istiadhah, quran.basmalah]) {
        const rest = heard.slice(cut), slack = Math.ceil(formula.length / 6);
        if (rest.length < formula.length + slack) { if (rest.length && edits(rest.slice(0, formula.length), formula.slice(0, rest.length)) <= slack) return null; continue; }
        let best = Infinity, len = 0;
        for (let n = formula.length - slack; n <= formula.length + slack; n++) { const c = edits(rest.slice(0, n), formula); if (c < best) { best = c; len = n; } }
        if (best <= slack) { cut += len; if (formula === quran.basmalah) basmalah = true; }
      }
      return { sounds: heard.slice(cut), basmalah };
    }
    // A mistake, as the reader hears and sees it: two short low tones, and a red mark on the
    // place of the word that was expected. The word stays hidden.
    let tones = null;
    function wrong(word) {
      try {
        tones = tones || new AudioContext();
        [[220, 0], [165, 0.16]].forEach(([hz, at]) => { const o = tones.createOscillator(), g = tones.createGain(); o.type = 'square'; o.frequency.value = hz; g.gain.setValueAtTime(0.08, tones.currentTime + at); g.gain.exponentialRampToValueAtTime(0.001, tones.currentTime + at + 0.14); o.connect(g).connect(tones.destination); o.start(tones.currentTime + at); o.stop(tones.currentTime + at + 0.15); });
      } catch { /* no sound on this device: the mark still shows */ }
      const p = st.page, h = p.highlight(T.word(word), { mode: 'band', band: rgba('#c62828', 0.45), radius: 1.5, ms: 120 });
      st.draw(); setTimeout(() => { if (st.page === p) { p.removeHighlight(h); st.draw(); } }, 700);
    }
    // ── following the recitation ──
    // The model gives, 25 times a second, how probable each sound is (`log_probs`), and for
    // each ayah the sounds a correct recitation has (`model.reference`). The follower lines
    // the expected sounds up with the frames as they come: the best path through the sounds,
    // where a frame stays on a sound, moves to the next, or rests between two (CTC).
    // So a word shows in the frame where its last sound is heard, and the page does not wait
    // for the model's text, which comes later and leaves out a sound it was not sure of.
    //   units       the expected sounds, in order
    //   wordOfUnit  for each sound, the index of its word in `expect`
    //   from        the word where the reader starts
    function follower(units, wordOfUnit, from, earliest = from) {
      const BLANK = model.tokens.blank, BAR = model.tokens.wordBar, V = model.tokens.count;
      // A frame outside the expected sounds costs this much more than the best sound of the
      // frame. It lets the path wait, before the first sound, through anything else said.
      const WAIT = 0.3;
      // This many sounds heard with no step forward is a mistake. On correct recitations of
      // two reciters the most was 3, and a wrong continuation passed 6 in 1.5 seconds.
      const STALL = 6;
      const N = units.length, S = 2 * N + 1, first = [], last = [];
      wordOfUnit.forEach((w, i) => { last[w] = i; if (first[w] === undefined) first[w] = i; });
      const sound = s => (s % 2 === 0 ? BLANK : units[(s - 1) >> 1]);
      // The path can start at the first sound of any word from `earliest` to `from`: the place
      // that was found can be some words after the word where the reader began.
      let starts = new Set(); for (let w = earliest; w <= from; w++) if (first[w] !== undefined) starts.add(first[w]);
      let score = new Float32Array(S).fill(-Infinity), wait = 0, reached = -1;
      let done = earliest - 1, started = false, begun = false, stall = 0, before = BLANK, warned = false;
      return {
        // One frame of `log_probs`, at `at` in `lp`. Answers what changed: {said} when words
        // were completed, {mistake} with the word that was expected, or null.
        step(lp, at) {
          let best = -Infinity, top = 0;
          for (let k = 0; k < V; k++) if (lp[at + k] > best) { best = lp[at + k]; top = k; }
          const heard = top !== BLANK && top !== BAR && top !== before; before = top;
          const next = new Float32Array(S); let state = -1, high = -Infinity;
          for (let i = 0; i < S; i++) {
            let m = score[i];
            if (i > 0 && score[i - 1] > m) m = score[i - 1];
            // a sound can follow the sound before it with no rest between, when the two differ
            if (i % 2 === 1 && i >= 3 && units[(i - 1) >> 1] !== units[(i - 3) >> 1] && score[i - 2] > m) m = score[i - 2];
            // states 2u and 2u + 1 are the rest before sound u, and sound u itself
            if (wait > m && starts.has(i >> 1)) m = wait;
            next[i] = m + lp[at + sound(i)];
            if (next[i] > high) { high = next[i]; state = i; }
          }
          wait += best - WAIT; score = next;
          if (!started && wait >= high) return null;
          started = true;
          const unit = state % 2 === 1 ? state >> 1 : (state >> 1) - 1;
          let out = null;
          // the first step tells where the reader began: the words before it stay hidden
          if (!begun && unit >= 0) { begun = true; reached = unit; done = wordOfUnit[unit] - 1; out = { from: wordOfUnit[unit] }; }
          else if (unit > reached) { reached = unit; stall = 0; warned = false; } else if (heard && begun) stall++;
          if (unit >= 0) { const w = wordOfUnit[unit], end = unit >= last[w] ? w : w - 1; if (end > done) { done = end; out = { ...(out || {}), said: done }; } }
          if (stall >= STALL && done < wordOfUnit[N - 1]) {
            // start again at the word that did not come, so the reader can say it again
            const w = done + 1; starts = new Set([first[w]]); reached = first[w] - 1; score = new Float32Array(S).fill(-Infinity); wait = 0; started = false; stall = 0;
            if (!warned) { warned = true; out = { ...(out || {}), mistake: w }; } else out = { ...(out || {}), again: w };
          }
          return out;
        },
      };
    }
    // The expected sounds of the page in hand, from the reference of each of its ayahs. A
    // sound belongs to a word of `expect` by its surah, its ayah and its place in the ayah.
    async function pageSounds() {
      const at = new Map(expect.map((w, i) => [`${w.surah}:${w.ayah}:${w.word}`, i])), units = [], wordOfUnit = [];
      const ayahs = [...new Set(expect.map(w => `${w.surah}:${w.ayah}`))];
      for (const key of ayahs) {
        const [surah, ayah] = key.split(':').map(Number), ref = await model.reference.units(surah, ayah), extra = quran.shift(surah, ayah, ref);
        for (let i = 0; i < ref.units.length; i++) {
          const w = at.get(`${surah}:${ayah}:${Math.max(0, ref.words[i] - extra) + 1}`);
          // the word bar is no sound, and a word of this ayah can be on another page
          if (ref.units[i] !== model.tokens.wordBar && w !== undefined) { units.push(ref.units[i]); wordOfUnit.push(w); }
        }
      }
      return { units, wordOfUnit };
    }
    // `said` is the next word of the page that the reader must say. `frames` keeps every
    // frame since the take began, so the follower can read again from the start once the
    // place is known. `lost` counts mistakes in a row: after three the place is sought again.
    let place = null, mistakes = 0, follow = null, frames = [], lost = 0, skip = 0;
    const maskPage = () => { st.page.maskOptions({ blockColor: '#d9cfb8' }); st.page.mask(T.page(), mode); };
    // `from` is the word where the recitation began, and `sinceMs` the time of its first
    // sound: the follower reads the frames again from that time, and not the words said
    // before it, such as the basmalah.
    async function startFollowing(from, sinceMs) {
      const { units, wordOfUnit } = await pageSounds();
      said = from; follow = units.length ? follower(units, wordOfUnit, from) : null;
      if (sinceMs !== undefined && follow) for (const f of frames) for (let i = 0; i < f.n; i++) if ((f.first + i) * f.ms >= sinceMs) advance(follow.step(f.lp, i * f.classes));
    }
    function advance(change) {
      if (!change) return;
      if (change.from !== undefined) said = change.from;
      if (change.said !== undefined && change.said + 1 > said) { for (let i = said; i <= change.said; i++) st.page.unmaskWord(expect[i].index); said = change.said + 1; lost = 0; st.draw(); }
      if (change.mistake !== undefined) { mistakes++; lost++; wrong(expect[change.mistake].index); }
      if (change.again !== undefined) lost++;
    }
    async function hear(chunk) {
      const head = chunk.heads && chunk.heads.get('log_probs');
      if (head) frames.push({ lp: head.values, n: head.frames, classes: head.classes, first: chunk.firstFrame, ms: head.frameMs });
      if (follow) {
        if (head) for (let i = 0; i < head.frames; i++) advance(follow.step(head.values, i * head.classes));
        // every word of this page was said: the recitation goes on to the next page
        if (said >= expect.length && has(st.n + 1) && st.n < 604) { await st.load(st.n + 1); maskPage(); frames = []; await startFollowing(0); }
        // the reader left this place: seek the place again, in what is heard from now on
        if (lost >= 3) { follow = null; place = null; lost = 0; skip = (chunk.decoded.tokens || []).length; frames = []; }
      }
      if (!follow) {
        // the sounds heard so far: every token but the rest, the word bar and a foreign sound
        const tokens = (chunk.decoded.tokens || []).slice(skip).filter(t => t.id !== model.tokens.blank && t.id !== model.tokens.wordBar && t.id < model.tokens.count - 1);
        const all = tokens.map(t => t.id);
        // nothing was heard yet: the invitation to recite stays on the card
        if (!all.length) return;
        const opening = afterOpening(all);
        const hit = opening ? locate(opening.sounds) : null;
        if (!hit) { card('hzCard', '', tr('جارٍ الاستماع وتحديد الموضع.')); return; }
        // The reader can recite from any page, and from any word of it. Nothing shows until
        // what was heard is in one place only.
        let first = begin(opening.sounds, hit);
        // in the first surah the basmalah is the first ayah, so there it is part of the recitation
        if (opening.basmalah && first === 4) first = 0;
        const pg = quran.page(first);
        if (!has(pg)) { card('hzCard', '', tt`سورة ${quran.surah(first)}، الآية ${quran.ayah(first)}`, tt`الصفحة ${pg} غير متوفرة في هذا العرض.`); return; }
        if (pg !== st.n) await st.load(pg);
        maskPage(); place = first;
        // The words before the first word stay hidden. The follower reads again from the first
        // sound of the run that was found.
        const since = tokens[Math.max(0, all.length - hit.m)].firstMs - 240;
        await startFollowing(Math.min(expect.length - 1, first - quran.firstOf(pg)), since);
      }
      const now = expect[Math.min(said, expect.length - 1)];
      card('hzCard', '', tt`سورة ${now.surah}، الآية ${now.ayah} · الصفحة ${st.n}`, tt`الكلمات المقروءة حتى الكلمة ${said} من ${expect.length} · الأخطاء: ${mistakes}`);
    }
    // One Client for the page, and the model asked for early: the download starts when the
    // reader comes to this scene, before the first press. The Quran's sounds and the recorded
    // sample are made ready then too, so a press has nothing to wait for.
    const RATE = 16000;
    let model = null, sample = null, busy = false, live = null;
    async function prepare() {
      await new Promise((ok, bad) => { const s = document.createElement('script'); s.src = 'https://web.quranlab.ai/4/loader.js'; s.crossOrigin = 'anonymous'; s.onload = ok; s.onerror = () => bad(new Error('loader')); document.head.append(s); });
      const client = await window.QuranLab.TajweedClient.create({ key: CFG.quranlabKey });
      model = await client.model();
      await wholeQuran();
      // A recorded recitation, as the model takes its own audio: mono at 16 kHz. Surah 112 is
      // on the last page, so the reader sees the demo find the place. Al-Husary's murattal
      // recitation is slow and clear.
      // `?sample=95` takes another surah, to try another place.
      const surah = Math.min(114, Math.max(1, +new URLSearchParams(location.search).get('sample') || 112));
      const info = await json(`${API}/v4/recitations/mahmoud-husary/r1/surahs/${surah}`), files = info.data.files;
      const url = (files.mp3_64 || files.original || Object.values(files)[0]).url;
      const decoded = await new OfflineAudioContext(1, RATE, RATE).decodeAudioData((await bytes(url)).buffer);
      const off = new OfflineAudioContext(1, Math.ceil(decoded.duration * RATE), RATE), src = off.createBufferSource();
      src.buffer = decoded; src.connect(off.destination); src.start();
      sample = { url, samples: (await off.startRendering()).getChannelData(0) };
    }
    // The recorded sample: the model gets the samples that the reader has already heard, and
    // no more, so a word never shows before its sound. The browser can refuse to play sound:
    // then the feed keeps its own clock, and the words still show at the reciter's pace.
    function recorded() {
      const session = model.session(), audio = new Audio(sample.url), played = audio.play().then(() => true, () => false);
      const feed = (async () => {
        const playing = await played, t0 = performance.now(), samples = sample.samples;
        for (let sent = 0; sent < samples.length && busy;) {
          const heardTo = playing ? (audio.ended ? Infinity : audio.currentTime) : (performance.now() - t0) / 1000;
          const upTo = Math.min(samples.length, Math.floor(heardTo * RATE));
          if (upTo > sent) { await session.push(samples.subarray(sent, upTo)); sent = upTo; }
          await new Promise(r => setTimeout(r, 60));
        }
        audio.pause(); await session.finish();
      })();
      feed.catch(e => { console.error(e); status(tr('تعذّر تشغيل القراءة بالصوت: ') + (e.code || e.message)); });
      return { session, feed };
    }
    const buttons = () => {
      $('hzListen').textContent = busy === 'mic' ? tr('إيقاف') : tr('القراءة بالصوت');
      $('hzClip').textContent = busy === 'sample' ? tr('إيقاف') : tr('تجربة بتلاوة مسجّلة');
      $('hzListen').disabled = busy === 'sample'; $('hzClip').disabled = busy === 'mic';
    };
    // A second press, or a page that the reader left, ends the take.
    function stop() { if (!busy) return; busy = false; if (live) live.finish().catch(() => {}); }
    async function take(fromRecording) {
      if (busy) { stop(); return; }
      busy = fromRecording ? 'sample' : 'mic';
      try {
        // The browser gives the microphone, and lets sound play, only in answer to a press. So
        // the take starts as the first thing in the handler, with nothing to wait for before it.
        try { tones = tones || new AudioContext(); } catch { /* the mistake sound is optional */ }
        let feed = null;
        if (fromRecording) ({ session: live, feed } = recorded());
        // The take goes on through the reader's pauses, until the reader stops it.
        else live = await model.listen();
        place = null; mistakes = 0; follow = null; frames = []; lost = 0; skip = 0;
        buttons(); mask(); status(fromRecording ? tr('جارٍ تحليل التلاوة المسجّلة.') : tr('ابدأ القراءة من أي موضع في المصحف.'));
        for await (const chunk of live.chunks) if (chunk.decoded) await hear(chunk);
        if (feed) await feed;
      } catch (e) { console.error(e); status(tr('تعذّر تشغيل القراءة بالصوت: ') + (e.code || e.message)); }
      busy = false; live = null; buttons();
    }
    if (CFG.quranlabKey) {
      $('hzListen').onclick = () => take(false); $('hzClip').onclick = () => take(true);
      document.addEventListener('visibilitychange', () => { if (document.hidden) stop(); });
      status(tr('جارٍ تجهيز النموذج. قد يستغرق التحميل الأول بعض الوقت.'));
      prepare().then(() => { $('hzListen').disabled = false; $('hzClip').disabled = false; status(); },
        e => { console.error(e); status(tr('تعذّر تشغيل القراءة بالصوت: ') + (e.code || e.message)); });
    } else {
      $('hzFine').prepend(tr('تحتاج القراءة بالصوت إلى مفتاح Quran Lab في الملف config.local.js. '));
    }
  });

  // ── 5. reading comfort: the engine's zoom steps, reflow and paper ──
  scene('read', async () => {
    const sec = $('read'), st = new Stage($('rdStage'), { zoomMode: 'stepped', hint: tr('اسحب الصفحة للتنقل بعد التكبير') });
    st.onZoom = z => {
      for (const b of sec.querySelectorAll('[data-step]')) b.classList.toggle('on', z.mode === 'stepped' && +b.dataset.step === z.step);
      for (const b of sec.querySelectorAll('[data-zmode]')) b.classList.toggle('on', b.dataset.zmode === z.mode);
      const L = st.page && st.page.currentLayout;
      card('rdCard', '', L && L.reflowed ? tt`التكبير: ×${z.zoom.toFixed(2)}. عدد السطور: ${L.rows} (في المطبوع: ${st.page.nLines}).` : tr('الحجم المطبوع: السطور كما في المصحف.'));
    };
    for (const b of sec.querySelectorAll('[data-step]')) b.onclick = () => { if (st.zoom.mode !== 'stepped') st.setZoomMode('stepped'); st.toStep(+b.dataset.step); };
    for (const b of sec.querySelectorAll('[data-zmode]')) b.onclick = () => st.setZoomMode(b.dataset.zmode);
    for (const b of sec.querySelectorAll('[data-theme]')) b.onclick = () => { for (const o of sec.querySelectorAll('[data-theme]')) o.classList.toggle('on', o === b); st.setTheme(b.dataset.theme); };
    await st.load(3);
  });

  // ── 6. dress: another mushaf's ornaments on the same text ──
  scene('dress', async () => {
    const st = new Stage($('drStage'));
    // a set that the build names, or the one that comes with the pages
    const ob = await maybe('ornaments.qvo') || await maybe('pages/ornaments.qvo'), set = ob ? engine.loadOrnaments(ob) : null;
    let style = null, lineArt = false;
    function apply() {
      const p = st.page;
      if (!set || !style) { p.undress(); card('drCard', '', set ? tr('الزخارف المطبوعة.') : tr('لا توجد مجموعة زخارف مع الصفحات (ornaments.qvo).')); }
      else {
        // a whole mushaf, or its ayah marker alone when the set has no border and band for it
        const info = p.dress(set, whole ? { style, lineArt } : { ayahMark: style, lineArt });
        card('drCard', '', info ? tt`علامات الآيات: ${info.ayahMarks}. رؤوس السور: ${info.surahHeaders}. الإطار: ${info.pageFrame === null ? tr('لا يوجد') : tr('موجود')}.` : tr('تعذّر تطبيق هذه الزخارف.'));
      }
      // the source of the ornaments on the page, and their licence, as the set states them
      const worn = set && style ? set.list.find(o => o.style === style) : null;
      $('drFine').textContent = worn ? [tr('مصدر الزخارف: مشروع quran-assets.'), worn.lineage === 'scan' ? tr('الزخارف منقولة عن مصحف مطبوع، وتصميمها ملك لناشره.') : '', worn.licence.id ? tr('الرخصة: ') + worn.licence.id + '.' : ''].filter(Boolean).join(' ') : '';
      for (const c of $('drStyles').children) c.classList.toggle('on', c.dataset.style === style);
      $('drLine').classList.toggle('on', lineArt);
      st.fit();
    }
    await st.load(293);
    const all = set ? set.styles() : [];
    let full = all.filter(s => s.ayahMark && s.surahHeader && s.pageFrame);
    const whole = full.length > 0;
    // with markers only, one design of each family is enough to show the idea
    if (!whole) { const seen = new Set(); full = all.filter(s => s.ayahMark && !seen.has(s.style.slice(0, 8)) && seen.add(s.style.slice(0, 8))).slice(0, 10); $('drSay').textContent = tr('يستخدم المحرّك علامات آيات بتصميم آخر مع نص مصحف المدينة، ويضع كل علامة في موضع العلامة المطبوعة وبحجمها.'); }
    // the Arabic name of a style of the set, or its own name when the list has none
    const NAMES = { 'mushaf-douri': tr('مصحف الدوري'), 'mushaf-hafs-adi': tr('مصحف حفص العادي'), 'mushaf-hafs-madinah-kabir': tr('مصحف المدينة الكبير'), 'mushaf-hafs-madinah-mumtaza': tr('مصحف المدينة الممتاز'), 'mushaf-qalon': tr('مصحف قالون'), 'mushaf-shubah': tr('مصحف شعبة'), 'mushaf-sousi': tr('مصحف السوسي'), 'mushaf-warsh': tr('مصحف ورش') };
    const name = (st0, i) => NAMES[st0] || (whole ? st0 : tt`التصميم ${i + 1}`);
    for (const [i, s] of full.entries()) { const c = el('button', '', name(s.style, i)); c.dataset.style = s.style; c.onclick = () => { style = s.style; apply(); }; $('drStyles').append(c); }
    $('drLine').onclick = () => { lineArt = !lineArt; apply(); };
    $('drPlain').onclick = () => { style = null; apply(); };
    if (full.length) { style = full[0].style; apply(); } else apply();
  });

  // ── 7. the comparison: the same page three ways, measured on this device ──
  scenes.push('compare', 'platforms');
  if (listed) for (const o of [...$('cmpPages').options]) if (+o.value > listed.length) o.remove();
  $('cmpRun').onclick = async () => {
    const count = +$('cmpPages').value, W = 1080, cells = $('cmpCells'), btn = $('cmpRun');
    // the pages of the measure: from page 3, or the first pages that this build holds
    const numbers = (listed || Array.from({ length: 602 }, (_, i) => i + 3)).slice(0, count);
    btn.disabled = true; cells.textContent = '';
    // A number with its unit reads left to right. The isolate keeps it whole inside Arabic text.
    const ltr = s => '\u2066' + s + '\u2069';
    const mb = b => ltr((b / 1048576).toFixed(b < 10485760 ? 2 : 1) + ' MB'), ms = t => ltr(t.toFixed(t < 100 ? 1 : 0) + ' ms');
    const frame = () => new Promise(r => requestAnimationFrame(() => r()));
    // Each run reads every file again from the server, and no measure reuses the work of a
    // run before it: a new engine, new pages, new images, and fonts under new names.
    const run = Date.now();
    const fresh = async url => { try { const r = await fetch(`${url}?run=${run}`, { cache: 'no-store' }); return r.ok ? new Uint8Array(await r.arrayBuffer()) : null; } catch { return null; } };
    // One result: the time for all the pages, the bytes that came from the server, and the
    // memory when the page can know it.
    const show = (id, title, r, note) => {
      $(id).textContent = r ? `${ms(r.time / count)}` : '—';
      const c = cells.appendChild(el('div', 'cell')); c.append(el('h4', '', title));
      if (!r) { c.append(el('div', 'v', '—'), el('p', '', note)); return; }
      c.append(el('div', 'v', ms(r.time)));
      c.append(el('p', '', tt`عدد الصفحات: ${count} · متوسط الصفحة: ${ms(r.time / count)} · حجم الملفات: ${mb(r.bytes)}`));
      c.append(el('p', '', tr('الذاكرة: ') + r.memory));
      if (note) c.append(el('p', '', note));
    };
    const pagesOf = () => numbers;
    try {
      // a box that the browser lays out and paints: almost clear, so the paint still happens
      const box = () => { const h = document.body.appendChild(el('div')); h.style.cssText = `position:fixed;opacity:0.01;pointer-events:none;inset-inline-start:0;top:0;width:${W}px;z-index:-1;direction:rtl;text-align:justify;font-size:46px;line-height:2.1`; return h; };
      const twoFrames = async () => { await frame(); await frame(); };
      // Two frames pass even when the browser has nothing to do. That wait is not work, so the
      // measures of SVG and of the fonts leave it out: the least of three idle waits.
      let idle = Infinity;
      for (let i = 0; i < 3; i++) { const t0 = performance.now(); await twoFrames(); idle = Math.min(idle, performance.now() - t0); }
      // The work the page can time itself comes first: reading, and the layout that the box
      // forces. The paint comes with the frames, and it counts where it took longer than the
      // idle wait. So the measure is never less than the work that was timed.
      const painted = async (h, t0) => { h.getBoundingClientRect(); const timed = performance.now() - t0; await twoFrames(); return Math.max(timed, performance.now() - t0 - idle); };

      // ── the engine: decode each page file, build its paths, lay it out and draw it. Every
      // page stays open, as in a reader that holds its pages. ──
      const files = [];
      for (const n of pagesOf(count)) { const f = await fresh(`pages/${pad(n)}.qvp`); if (!f) throw new Error(`pages/${pad(n)}.qvp`); files.push(f); }
      const own = await QvpEngine.init(await fresh('qvp_ffi.wasm')), before = own.mem.buffer.byteLength;
      const open = [], canvases = []; let paths = 0, pixels = 0;
      let t = performance.now();
      for (const f of files) {
        const p = own.loadPage(f); p.buildPaths();
        const L = p.layout({ viewportW: W, viewportH: W * 2, padTop: 0, padBottom: 0, padLeft: 0, padRight: 0, lineSpacing: 1, fillHeight: false, surahFrames: true, maxAspectSlack: DEFAULTS.ASPECT_SLACK, reflow: null });
        const c = document.createElement('canvas'); c.width = W; c.height = Math.ceil(L.contentH * L.fitScale);
        p.tick(performance.now());
        new CanvasRenderer(c).draw(p, { scale: L.fitScale, offsetX: L.fitX, offsetY: 0 }, 1);
        open.push(p); canvases.push(c); paths += p.nPaths; pixels += c.width * c.height * 4;
      }
      const engineTime = performance.now() - t, grown = own.mem.buffer.byteLength - before;
      show('cmpQvp', tr('Quran Engine'), { time: engineTime, bytes: files.reduce((a, f) => a + f.length, 0), memory: tt`ذاكرة المحرّك للصفحات المفتوحة ${mb(grown)}، وبكسلات الرسم ${mb(pixels)}` }, tt`يشمل فك الملفات وبناء المسارات ورسمها. عدد المسارات: ${paths}.`);

      // ── images: the same pixels as PNG files, then decode and draw each one ──
      const blobs = []; for (const c of canvases) blobs.push(await new Promise(r => c.toBlob(r, 'image/png')));
      const bitmaps = []; let decoded = 0;
      t = performance.now();
      for (const blob of blobs) { const bmp = await createImageBitmap(blob); document.createElement('canvas').getContext('2d').drawImage(bmp, 0, 0); bitmaps.push(bmp); decoded += bmp.width * bmp.height * 4; }
      const pngBytes = blobs.reduce((a, x) => a + x.size, 0);
      show('cmpImg', tr('صور PNG'), { time: performance.now() - t, bytes: pngBytes, memory: tt`الصور بعد فك ضغطها ${mb(decoded)}` }, tt`بعرض ${W} بكسل، دون بيانات للكلمات أو الحروف.`);
      $('cmpPng').textContent = '≈ ' + (pngBytes / count * 604 / 1048576).toFixed(0) + ' MB';   // this cell is set left to right in the page
      // the words of each page as text, for the Hafs font. The sidecar gives them, and no measure counts it.
      const texts = [];
      for (const [i, p] of open.entries()) { const wb = await fresh(`pages/${pad(numbers[i])}.words.json`); if (wb) p.attachWords(wb); texts.push(p.words.map(w => p.wordForm(w.index, 'qpc') || w.text).join(' ')); }
      for (const p of open) p.free();
      for (const x of bitmaps) x.close();

      // the browser holds SVG nodes and fonts where a page cannot measure them, so the result says so
      const domMemory = nodes => tt`عدد عناصر الصفحة ${nodes}. لا يتيح المتصفح قياس ذاكرتها من داخل الصفحة.`;

      // ── SVG: read each file and put it in the page, as an app must to reach its words ──
      const svgs = []; for (const n of pagesOf(count)) { const f = await fresh(`svg/${pad(n)}.svg`); if (!f) break; svgs.push(f); }
      if (svgs.length === count) {
        const host = box(); let time = 0, nodes = 0; 
        for (const f of svgs) {
          t = performance.now();
          const doc = new DOMParser().parseFromString(new TextDecoder().decode(f), 'image/svg+xml');
          const parse = performance.now() - t;
          // The file is data. Nothing in it may run: no script, no foreign content, no handler.
          for (const bad of doc.querySelectorAll('script, foreignObject, iframe, object, embed')) bad.remove();
          for (const node of doc.querySelectorAll('*')) for (const at of [...node.attributes]) if (/^on/i.test(at.name) || /^\s*javascript:/i.test(at.value)) node.removeAttribute(at.name);
          t = performance.now();
          host.append(document.importNode(doc.documentElement, true));
          time += parse + await painted(host, t);
        }
        nodes = host.querySelectorAll('*').length;
        show('cmpSvg', 'SVG', { time, bytes: svgs.reduce((a, f) => a + f.length, 0), memory: domMemory(nodes) }, tr('يشمل قراءة الملفات وإضافتها إلى الصفحة.'));
        host.remove();
      } else show('cmpSvg', 'SVG', null, tr('ملفات SVG غير متوفرة في هذا البناء.'));

      // ── fonts: decode a font, set a page's text in it, and let the browser lay it out and paint it ──
      const withFonts = async (id, title, fontOf, textOf, note) => {
        const host = box(), faces = []; let time = 0, size = 0; const seen = new Map();
        for (let i = 0; i < count; i++) {
          const url = fontOf(i), text = textOf(i); if (!url || text === null) { host.remove(); for (const f of faces) document.fonts.delete(f); show(id, title, null, tr('هذا الخط غير متوفر في هذا البناء.')); return; }
          const line = host.appendChild(el('div'));
          let fam = seen.get(url), fb = null;
          if (!fam) { fb = await fresh(url); if (!fb) { host.remove(); show(id, title, null, tr('هذا الخط غير متوفر في هذا البناء.')); return; } size += fb.length; }
          size += new TextEncoder().encode(text).length;
          t = performance.now();
          if (!fam) { fam = `bench-${id}-${run}-${i}`; const face = new FontFace(fam, fb); await face.load(); document.fonts.add(face); faces.push(face); seen.set(url, fam); }
          line.style.fontFamily = fam; line.textContent = text;
          time += await painted(host, t);
        }
        show(id, title, { time, bytes: size, memory: tt`عدد الخطوط المفتوحة ${faces.length}. لا يتيح المتصفح قياس ذاكرتها من داخل الصفحة.` }, note);
        host.remove(); for (const f of faces) document.fonts.delete(f);
      };
      // a page font holds one glyph for each word of its page, at the code points the build lists
      const codes = []; for (const n of pagesOf(count)) { const cb = await fresh(`fonts/qpc-${pad(n)}.json`); codes.push(cb ? JSON.parse(new TextDecoder().decode(cb)) : null); }
      await withFonts('cmpQpc', tr('خطوط الصفحات (QPC)'), i => (codes[i] ? `fonts/qpc-${pad(numbers[i])}.woff2` : ''), i => (codes[i] ? codes[i].map(c => String.fromCodePoint(c)).join(' ') : null), tr('خط لكل صفحة، وكل كلمة محرف فيه.'));
      await withFonts('cmpHafs', tr('خط حفص مع النص'), () => 'fonts/quran.woff2', i => texts[i], tr('خط واحد للمصحف كاملًا مع نص كل صفحة.'));
      $('cmpNote').textContent = tr('هذه نتائج جهازك، وتختلف من جهاز إلى آخر ومن قياس إلى آخر.');
    } catch (e) { console.error(e); $('cmpNote').textContent = tr('تعذّر القياس: ') + e.message; }
    btn.disabled = false;
  };

  // the rail at the top follows the scene on the screen
  const links = [...$('rail').children];
  for (const id of scenes) {
    // The scene on the screen is the one that crosses the middle line. On a phone a scene is
    // taller than the screen, so a share of the scene is no measure. The rail is a strip
    // there, and it moves to keep the scene's link in view.
    new IntersectionObserver(es => {
      if (!es[0].isIntersecting) return;
      for (const a of links) {
        const on = a.getAttribute('href') === '#' + id; a.classList.toggle('on', on);
        if (on && $('rail').scrollWidth > $('rail').clientWidth) $('rail').scrollTo({ left: a.offsetLeft - ($('rail').clientWidth - a.offsetWidth) / 2, behavior: 'smooth' });
      }
    }, { rootMargin: '-50% 0px -50% 0px' }).observe($(id));
  }
  window.__demo = { engine, atlas };
})();
