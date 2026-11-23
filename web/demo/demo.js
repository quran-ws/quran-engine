// The Arabic showcase: six scenes, each a live mushaf page that the engine draws.
// The engine decides everything on the page: layout, zoom, hit tests, colours, masks. This
// file chooses which call to make, loads the data, and writes the words beside the page.
(async function () {
  'use strict';
  const { QvpEngine, CanvasRenderer, Sel, T, LAYER, DEFAULTS, rgba } = window.QVP;
  const $ = id => document.getElementById(id);
  const CFG = window.QVP_DEMO || {};
  const pad = n => String(n).padStart(3, '0');
  const bytes = async url => { const r = await fetch(url); if (!r.ok) throw new Error(url + ' ' + r.status); return new Uint8Array(await r.arrayBuffer()); };
  const maybe = async url => { try { return await bytes(url); } catch { return null; } };
  const json = async url => { const r = await fetch(url); if (!r.ok) throw new Error(url + ' ' + r.status); return r.json(); };
  // a word that is only the number of its ayah has no letters to colour, time or recite
  const isNumber = s => !s || /^[٠-٩۝\s]+$/.test(s);
  const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; };
  // a card's content: a large line, then small lines. Text only, never markup from data.
  function card(id, big, ...subs) {
    const c = $(id); c.textContent = '';
    if (big) c.append(el('div', 'big', big));
    // a small line is text, or [label, words of the Quran]: the words take the mushaf's typeface
    for (const s of subs) {
      if (!s) continue;
      const d = c.appendChild(el('div', 'sub'));
      if (Array.isArray(s)) d.append(s[0], el('span', 'q', s[1])); else d.textContent = s;
    }
  }

  const engine = await QvpEngine.init(await bytes('qvp_ffi.wasm'));
  const atlasBytes = await maybe('pages/atlas.qva');
  const atlas = atlasBytes ? engine.loadAtlas(atlasBytes) : null;
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
      for (const [label, title, fn] of [['+', 'تكبير', () => this.nudge(1)], ['−', 'تصغير', () => this.nudge(-1)], ['⤢', 'ملاءمة الصفحة', () => this.reset()]]) {
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
      start().catch(e => { console.error(id, e); const host = $(id).querySelector('.stage'); host.appendChild(el('div', 'note', 'تعذّر تحميل هذا القسم: ' + e.message)); });
    }, { rootMargin: '400px' }).observe($(id));
  }

  // ── 1. the opening: the page writes itself, and every word answers a tap ──
  scene('hero', async () => {
    let hl = 0, timer = 0;
    const st = new Stage($('heroStage'), {
      onTap: (x, y) => {
        const p = st.page, h = p.hitTestView(x, y, { maxDistance: 6 });
        if (!h || h.word < 0) { if (hl) { p.removeHighlight(hl); hl = 0; } card('heroCard', '', 'اضغط على كلمة لعرض بياناتها.'); st.draw(); return; }
        const style = { mode: 'both', ink: '#9a6b00', band: rgba('#c9a44c', 0.28), radius: 1.5, ms: 220, layer: LAYER.SELECTION };
        if (hl) p.moveHighlight(hl, T.word(h.word)); else hl = p.highlight(T.word(h.word), style);
        const w = p.words[h.word], letters = st.letters ? p.letters(h.word).map(l => l.text).join(' · ') : '';
        card('heroCard', p.wordForm(h.word, 'qpc') || w.text, `السورة ${w.surah}، الآية ${w.ayah}، الكلمة ${w.word}`, letters && ['الحروف: ', letters]);
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
    ['ghn', 'غنّة', '#FF7E1E', ['noon-mushaddadah', 'meem-mushaddadah']],
    ['qlq', 'قلقلة', '#DD0008', ['qalqalah-']],
    ['idgh_ghn', 'إدغام بغنّة', '#169200', ['idgham-bi-ghunnah']],
    ['idgh_w_ghn', 'إدغام بغير غنّة', '#0E6B00', ['idgham-bila-ghunnah']],
    ['iqlb', 'إقلاب', '#26BFFD', ['iqlab-']],
    ['ikhf', 'إخفاء', '#9400A8', ['ikhfa-haqiqi', 'ikhfa-ghunnah']],
    ['ikhf_shfw', 'إخفاء شفوي', '#D500B7', ['ikhfa-shafawi', 'mutajanisain-ikhfa-shafawi']],
    ['idghm_shfw', 'إدغام شفوي', '#58B800', ['idgham-shafawi']],
    ['madda_necessary', 'مدّ لازم', '#000EBC', ['madd-lazim']],
    ['madda_obligatory', 'مدّ واجب متّصل', '#2144C1', ['madd-muttasil']],
    ['madda_permissible', 'مدّ جائز', '#4050FF', ['madd-munfasil', 'madd-arid', 'madd-silah-kubra']],
    ['madda_normal', 'مدّ طبيعي', '#537FFF', ['madd-tabee']],
    ['idgh_mus', 'إدغام متماثلين ومتجانسين', '#A1A1A1', ['mutamathilain-idgham', 'mutajanisain-idgham']],
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
        if (!st.letters) { $('tjStage').appendChild(el('div', 'hint', 'لا تتوفر طبقة الحروف لهذه الصفحة، ولا يمكن تلوين الحروف منفردة.')); return; }
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
        if (!here.length) { card('tjCard', '', h ? 'لا يوجد حكم ملوّن على هذا الحرف.' : 'اضغط على حرف ملوّن لعرض حكمه.'); st.draw(); return; }
        // the ruling that has a colour first, then the others on the same letter
        here.sort((a, b) => (b.fam >= 0) - (a.fam >= 0));
        const m = here[0], id = ann.ruleIds[m.rule];
        picked = p.highlight(T.word(h.word), { mode: 'band', band: rgba('#c9a44c', 0.2), radius: 1.5, ms: 180 });
        const own = rulingName.get(id.split('.')[0]) || (m.fam >= 0 ? TAJWID[m.fam][1] : 'حكم');
        // the other rulings on the same letter, by name
        const more = [...new Set(here.slice(1).map(x => rulingName.get(ann.ruleIds[x.rule].split('.')[0])).filter(Boolean))].filter(x => x !== own);
        card('tjCard', '', label.get(id) || '', ['الكلمة: ', p.wordForm(h.word, 'qpc')], more.length ? 'أحكام أخرى على الحرف: ' + more.join('، ') : '');
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
      for (const ch of p.wordForm(word, 'qpc')) { if (/\p{L}/u.test(ch) && ch !== 'ـ' && ++n === nth) return p.letterAtOffset(word, at); at++; }
      return -1;
    }
    async function show(ms) {
      const p = st.page; if (!p || turning) return;
      const i = last(words, 2, ms);
      if (i < 0) return;
      const [ayah, pos] = words[i];
      // the ayah left the page: the atlas says which page holds it
      const pg = atlas ? atlas.pageOf(SURAH, ayah) : st.n;
      if (pg && pg !== st.n) { turning = true; try { await st.load(pg); } finally { turning = false; } return; }
      if (repeat && cur >= 0 && words[cur][0] !== ayah && !audio.seeking) { const a = words[cur][0], first = words.find(v => v[0] === a); cur = -1; audio.currentTime = first[2] / 1000; return; }
      const w = index.by.get(`${SURAH}:${ayah}:${pos}`); if (w === undefined) return;
      if (i !== cur) {
        cur = i;
        // by word, the ink alone takes the colour. By letter, a faint band holds the word and the letter takes the colour.
        const style = byLetter ? { mode: 'band', band: rgba('#c9a44c', 0.22), radius: 1.5, ms: 120 } : { mode: 'ink', ink: '#0a7d32', ms: 120 };
        if (hl) p.moveHighlight(hl, T.word(w)); else hl = p.highlight(T.word(w), style);
        card('plCard', p.wordForm(w, 'qpc'), `سورة يس، الآية ${ayah}، الكلمة ${pos}`, recitation && recitation.reciter.name);
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
      card('plCard', '', `${r.reciter.name} · سورة يس · المدة ${clock(info.data.duration_ms)}`);
      if (byLetter) await needLetters();
    }
    async function needLetters() { if (!letters && recitation) letters = (await json(`${API}/v4/recitations/${recitation.id}/surahs/${SURAH}/letter-timings`)).data.letters; }
    audio.onplay = () => { $('plPlay').textContent = '⏸ إيقاف مؤقت'; loop(); };
    audio.onpause = () => { $('plPlay').textContent = '▶ تشغيل'; };
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
    const list = (await json(`${API}/v4/recitations?riwayah=hafs_an_asim&surah=${SURAH}&limit=40&locale=ar`)).data.filter(r => r.has_ayah_timings);
    const sel = $('plReciter');
    for (const r of list) { const o = el('option', '', r.reciter.name); o.value = r.id; sel.append(o); }
    sel.onchange = () => pick(list.find(r => String(r.id) === sel.value)).catch(e => card('plCard', '', 'تعذّر تحميل هذه التلاوة: ' + e.message));
    if (list.length) await pick(list[0]); else card('plCard', '', 'لا توجد تلاوات متاحة حاليًا.');
  });

  // ── 4. memorisation: hide, then reveal by a tap or by the reader's own voice ──
  scene('hifz', async () => {
    let mode = 'hide', expect = [], said = 0;
    const st = new Stage($('hzStage'), { onLoad: p => { expect = wordIndex(p).list; }, onTap: () => next() });
    const left = () => st.page.maskHidden().length;
    const status = text => card('hzCard', '', text || (left() ? `الكلمات المخفية: ${left()}. اضغط على الصفحة لكشف كلمة.` : 'جميع الكلمات ظاهرة.'));
    function mask() { const p = st.page; p.unmask(); p.maskOptions({ blockColor: '#d9cfb8' }); p.mask(T.page(), mode); said = 0; st.draw(); status(); }
    function next() { st.page.unmaskNext(1); st.draw(); status(); }
    for (const [id, m] of [['hzHide', 'hide'], ['hzBlock', 'block']]) $(id).onclick = () => { mode = m; for (const b of ['hzHide', 'hzBlock']) $(b).classList.toggle('on', b === id); mask(); };
    $('hzNext').onclick = next;
    $('hzBack').onclick = () => { st.page.maskBack(1); st.draw(); status(); };
    $('hzReset').onclick = mask;
    $('hzShow').onclick = () => { st.page.unmask(); st.draw(); status(); };
    await st.load(1);
    mask();

    // The reader's voice. TajweedFormer V4 runs in a frame of Quran Lab's on this device and
    // gives the sounds it heard. It does not judge. The match below is this demo's own: a
    // word shows when most of its strong consonants came, in order, in what was heard.
    const strong = s => [...s.normalize('NFC')].filter(c => /\p{L}/u.test(c) && !/[ـاأإآٱوؤيىئءة]/.test(c));
    function hear(decoded) {
      const tokens = decoded.tokens || [], n = decoded.isFinal ? tokens.length : (decoded.settled ?? tokens.length);
      const heard = tokens.slice(0, n).map(t => t.name).filter(x => /^\p{L}$/u.test(x) && /[؀-ۿ]/.test(x) && !/[ءوي]/.test(x));
      let at = 0, k = 0;
      for (; k < expect.length; k++) {
        const need = strong(expect[k].qpc); if (!need.length) continue;
        let got = 0, j = at, end = at;
        for (const c of need) { const f = heard.indexOf(c, j); if (f >= 0 && f - j <= 2) { got++; j = f + 1; end = j; } }
        if (got < (need.length <= 2 ? need.length : Math.ceil(need.length * 0.7))) break;
        at = end;
      }
      for (let i = said; i < k; i++) st.page.unmaskWord(expect[i].index);
      if (k > said) { said = k; st.draw(); }
      status(`النص المسموع: ${decoded.text || '…'}`);
    }
    let model = null;
    async function voice() {
      if (model) return model;
      status('جارٍ تجهيز النموذج. قد يستغرق التحميل الأول بعض الوقت.');
      await new Promise((ok, bad) => { const s = document.createElement('script'); s.src = 'https://web.quranlab.ai/4/loader.js'; s.crossOrigin = 'anonymous'; s.onload = ok; s.onerror = () => bad(new Error('loader')); document.head.append(s); });
      const client = await window.QuranLab.TajweedClient.create({ key: CFG.quranlabKey });
      return (model = await client.model());
    }
    async function take(clip) {
      try {
        const m = await voice();
        mask(); status(clip ? 'جارٍ تحليل المقطع الصوتي.' : 'ابدأ القراءة.');
        const session = await m.listen(clip ? { from: m.sampleClips[0] } : { endOnSilenceMs: 2500 });
        for await (const chunk of session.chunks) if (chunk.decoded) hear(chunk.decoded);
      } catch (e) { console.error(e); status('تعذّر تشغيل القراءة بالصوت: ' + (e.code || e.message)); }
    }
    if (CFG.quranlabKey) {
      $('hzListen').disabled = false; $('hzClip').disabled = false;
      $('hzListen').onclick = () => take(false); $('hzClip').onclick = () => take(true);
    } else {
      $('hzFine').prepend('تحتاج القراءة بالصوت إلى مفتاح Quran Lab في الملف config.local.js. ');
    }
  });

  // ── 5. reading comfort: the engine's zoom steps, reflow and paper ──
  scene('read', async () => {
    const sec = $('read'), st = new Stage($('rdStage'), { zoomMode: 'stepped', hint: 'اسحب الصفحة للتنقل بعد التكبير' });
    st.onZoom = z => {
      for (const b of sec.querySelectorAll('[data-step]')) b.classList.toggle('on', z.mode === 'stepped' && +b.dataset.step === z.step);
      for (const b of sec.querySelectorAll('[data-zmode]')) b.classList.toggle('on', b.dataset.zmode === z.mode);
      const L = st.page && st.page.currentLayout;
      card('rdCard', '', L && L.reflowed ? `التكبير: ×${z.zoom.toFixed(2)}. عدد السطور: ${L.rows} (في المطبوع: ${st.page.nLines}).` : 'الحجم المطبوع: السطور كما في المصحف.');
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
      if (!set || !style) { p.undress(); card('drCard', '', set ? 'الزخارف المطبوعة.' : 'لا توجد مجموعة زخارف مع الصفحات (ornaments.qvo).'); }
      else {
        // a whole mushaf, or its ayah marker alone when the set has no border and band for it
        const info = p.dress(set, whole ? { style, lineArt } : { ayahMark: style, lineArt });
        card('drCard', '', info ? `علامات الآيات: ${info.ayahMarks}. رؤوس السور: ${info.surahHeaders}. الإطار: ${info.pageFrame === null ? 'لا يوجد' : 'موجود'}.` : 'تعذّر تطبيق هذه الزخارف.');
      }
      for (const c of $('drStyles').children) c.classList.toggle('on', c.dataset.style === style);
      $('drLine').classList.toggle('on', lineArt);
      st.fit();
    }
    await st.load(293);
    const all = set ? set.styles() : [];
    let full = all.filter(s => s.ayahMark && s.surahHeader && s.pageFrame);
    const whole = full.length > 0;
    // with markers only, one design of each family is enough to show the idea
    if (!whole) { const seen = new Set(); full = all.filter(s => s.ayahMark && !seen.has(s.style.slice(0, 8)) && seen.add(s.style.slice(0, 8))).slice(0, 10); $('drSay').textContent = 'يستخدم المحرّك علامات آيات بتصميم آخر مع نص مصحف المدينة، ويضع كل علامة في موضع العلامة المطبوعة وبحجمها.'; }
    // the Arabic name of a style of the set, or its own name when the list has none
    const NAMES = { 'mushaf-douri': 'مصحف الدوري', 'mushaf-hafs-adi': 'مصحف حفص العادي', 'mushaf-hafs-madinah-kabir': 'مصحف المدينة الكبير', 'mushaf-hafs-madinah-mumtaza': 'مصحف المدينة الممتاز', 'mushaf-qalon': 'مصحف قالون', 'mushaf-shubah': 'مصحف شعبة', 'mushaf-sousi': 'مصحف السوسي', 'mushaf-warsh': 'مصحف ورش' };
    const name = (st0, i) => NAMES[st0] || (whole ? st0 : `التصميم ${i + 1}`);
    for (const [i, s] of full.entries()) { const c = el('button', '', name(s.style, i)); c.dataset.style = s.style; c.onclick = () => { style = s.style; apply(); }; $('drStyles').append(c); }
    $('drLine').onclick = () => { lineArt = !lineArt; apply(); };
    $('drPlain').onclick = () => { style = null; apply(); };
    if (full.length) { style = full[0].style; apply(); } else apply();
  });

  // ── 7. the comparison: the same page three ways, measured on this device ──
  scenes.push('compare');
  $('cmpRun').onclick = async () => {
    const first = 3, count = +$('cmpPages').value, W = 1080, cells = $('cmpCells'), btn = $('cmpRun');
    btn.disabled = true; cells.textContent = '';
    const mb = b => (b / 1048576).toFixed(b < 10485760 ? 2 : 1) + ' MB', ms = t => t.toFixed(t < 100 ? 1 : 0) + ' ms';
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
      c.append(el('p', '', `عدد الصفحات: ${count} · متوسط الصفحة: ${ms(r.time / count)} · حجم الملفات: ${mb(r.bytes)}`));
      c.append(el('p', '', 'الذاكرة: ' + r.memory));
      if (note) c.append(el('p', '', note));
    };
    const pagesOf = n => Array.from({ length: n }, (_, i) => first + i);
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
      show('cmpQvp', 'محرّك المصحف', { time: engineTime, bytes: files.reduce((a, f) => a + f.length, 0), memory: `ذاكرة المحرّك للصفحات المفتوحة ${mb(grown)}، وبكسلات الرسم ${mb(pixels)}` }, `يشمل فك الملفات وبناء المسارات ورسمها. عدد المسارات: ${paths}.`);

      // ── images: the same pixels as PNG files, then decode and draw each one ──
      const blobs = []; for (const c of canvases) blobs.push(await new Promise(r => c.toBlob(r, 'image/png')));
      const bitmaps = []; let decoded = 0;
      t = performance.now();
      for (const blob of blobs) { const bmp = await createImageBitmap(blob); document.createElement('canvas').getContext('2d').drawImage(bmp, 0, 0); bitmaps.push(bmp); decoded += bmp.width * bmp.height * 4; }
      const pngBytes = blobs.reduce((a, x) => a + x.size, 0);
      show('cmpImg', 'صور PNG', { time: performance.now() - t, bytes: pngBytes, memory: `الصور بعد فك ضغطها ${mb(decoded)}` }, `بعرض ${W} بكسل، دون بيانات للكلمات أو الحروف.`);
      $('cmpPng').textContent = '≈ ' + (pngBytes / count * 604 / 1048576).toFixed(0) + ' MB';
      // the words of each page as text, for the Hafs font. The sidecar gives them, and no measure counts it.
      const texts = [];
      for (const [i, p] of open.entries()) { const wb = await fresh(`pages/${pad(first + i)}.words.json`); if (wb) p.attachWords(wb); texts.push(p.words.map(w => p.wordForm(w.index, 'qpc') || w.text).join(' ')); }
      for (const p of open) p.free();
      for (const x of bitmaps) x.close();

      // the browser holds SVG nodes and fonts where a page cannot measure them, so the result says so
      const domMemory = nodes => `عدد عناصر الصفحة ${nodes}. لا يتيح المتصفح قياس ذاكرتها من داخل الصفحة.`;

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
        show('cmpSvg', 'SVG', { time, bytes: svgs.reduce((a, f) => a + f.length, 0), memory: domMemory(nodes) }, 'يشمل قراءة الملفات وإضافتها إلى الصفحة.');
        host.remove();
      } else show('cmpSvg', 'SVG', null, 'ملفات SVG غير متوفرة في هذا البناء.');

      // ── fonts: decode a font, set a page's text in it, and let the browser lay it out and paint it ──
      const withFonts = async (id, title, fontOf, textOf, note) => {
        const host = box(), faces = []; let time = 0, size = 0; const seen = new Map();
        for (let i = 0; i < count; i++) {
          const url = fontOf(i), text = textOf(i); if (!url || text === null) { host.remove(); for (const f of faces) document.fonts.delete(f); show(id, title, null, 'هذا الخط غير متوفر في هذا البناء.'); return; }
          const line = host.appendChild(el('div'));
          let fam = seen.get(url), fb = null;
          if (!fam) { fb = await fresh(url); if (!fb) { host.remove(); show(id, title, null, 'هذا الخط غير متوفر في هذا البناء.'); return; } size += fb.length; }
          size += new TextEncoder().encode(text).length;
          t = performance.now();
          if (!fam) { fam = `bench-${id}-${run}-${i}`; const face = new FontFace(fam, fb); await face.load(); document.fonts.add(face); faces.push(face); seen.set(url, fam); }
          line.style.fontFamily = fam; line.textContent = text;
          time += await painted(host, t);
        }
        show(id, title, { time, bytes: size, memory: `عدد الخطوط المفتوحة ${faces.length}. لا يتيح المتصفح قياس ذاكرتها من داخل الصفحة.` }, note);
        host.remove(); for (const f of faces) document.fonts.delete(f);
      };
      // a page font holds one glyph for each word of its page, at the code points the build lists
      const codes = []; for (const n of pagesOf(count)) { const cb = await fresh(`fonts/qpc-${pad(n)}.json`); codes.push(cb ? JSON.parse(new TextDecoder().decode(cb)) : null); }
      await withFonts('cmpQpc', 'خطوط الصفحات (QPC)', i => (codes[i] ? `fonts/qpc-${pad(first + i)}.woff2` : ''), i => (codes[i] ? codes[i].map(c => String.fromCodePoint(c)).join(' ') : null), 'خط لكل صفحة، وكل كلمة محرف فيه.');
      await withFonts('cmpHafs', 'خط حفص مع النص', () => 'fonts/quran.woff2', i => texts[i], 'خط واحد للمصحف كاملًا مع نص كل صفحة.');
      $('cmpNote').textContent = 'هذه نتائج جهازك، وتختلف من جهاز إلى آخر ومن قياس إلى آخر.';
    } catch (e) { console.error(e); $('cmpNote').textContent = 'تعذّر القياس: ' + e.message; }
    btn.disabled = false;
  };

  // the rail at the top follows the scene on the screen
  const links = [...$('rail').children];
  for (const id of scenes) {
    new IntersectionObserver(es => {
      if (es[0].isIntersecting) for (const a of links) a.classList.toggle('on', a.getAttribute('href') === '#' + id);
    }, { threshold: 0.5 }).observe($(id));
  }
  window.__demo = { engine, atlas };
})();
