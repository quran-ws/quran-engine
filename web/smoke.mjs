// Smoke test: every public entry point loads and the engine answers.
// Drawing is not exercised — buildPaths() and drawBoxes() need a DOM Path2D.
import { readFileSync, existsSync } from 'node:fs';
import { QvpEngine, wasmUrl } from './index.mjs';

const engine = await QvpEngine.init(readFileSync(wasmUrl));
const strip = engine.strip('الْحَمْدُ');
if (!strip) throw new Error('engine.strip() returned nothing');

console.log('ok  wasm instantiated, engine.strip() ->', strip);
if (engine.engineName() !== 'qvp' || !/^\d+\.\d+\.\d+$/.test(engine.version()) || !(engine.formatVersion() > 0)) throw new Error('engine name or version wrong');
if (engine.markFromName(engine.markName(1)) !== 1) throw new Error('markFromName does not invert markName');
console.log('ok  engine name, version and mark names round-trip');

// The sections that need page data skip with a hint, or fail when QVP_REQUIRE_DATA is set
// (scripts/check.sh gates sets it after scripts/sync-test-data.sh).
function skipData(what) {
  if (process.env.QVP_REQUIRE_DATA) throw new Error(`${what}; run scripts/sync-test-data.sh`);
  console.log(`skip ${what}`);
}

const cjs = (await import('./index.cjs')).default;
if (!cjs.QvpEngine) throw new Error('CJS entry did not expose QvpEngine');
console.log('ok  both entry points expose QvpEngine');

for (const n of ['001', '002']) {
  const file = new URL(`../dist/pages/${n}.qvp`, import.meta.url);
  if (!existsSync(file)) { skipData(`printed breaks: ${n}.qvp is missing`); continue; }
  const page = engine.loadPage(readFileSync(file));
  const spec = { viewportW: 390, viewportH: 844, reflow: { breaks: 'printed', zoom: 2 } };
  const cap = page.reflowMaxZoom(spec);
  if (!(cap > 1.2)) throw new Error(`printed breaks did not enlarge page ${n}`);
  spec.reflow.zoom = cap;
  const fitted = page.layout(spec);
  const rows = Array.from({ length: fitted.rows }, (_, r) => page.rowWords(r));
  spec.reflow.zoom = cap * 2;
  const clamped = page.layout(spec);
  if (clamped.scale !== fitted.scale || rows.some((ws, r) => JSON.stringify(page.rowWords(r)) !== JSON.stringify(ws))) throw new Error('printed breaks did not clamp');
  page.free();
}
console.log('ok  printed breaks reach the core and clamp');

// ── the wrapper's own bounds: inputs that outgrow the scratch, a freed handle, a bad spec ──
{
  const file = new URL('../dist/pages/002.qvp', import.meta.url);
  if (!existsSync(file)) console.log('skip wrapper bounds: 002.qvp is missing');
  else {
    const page = engine.loadPage(readFileSync(file));
    // A word list longer than its scratch region used to overwrite the target struct behind it,
    // and the engine then resolved a target of another kind.
    const repeated = Array.from({ length: 1000 }, () => 5);
    if (JSON.stringify(page.targetWords(repeated)) !== '[5]') throw new Error('a 1,000-word target resolved to ' + JSON.stringify(page.targetWords(repeated)));
    const handle = page.highlight(repeated);
    if (JSON.stringify(page.highlightWords(handle)) !== '[5]') throw new Error('a 1,000-word highlight covers ' + JSON.stringify(page.highlightWords(handle)));
    page.removeHighlight(handle);
    // A word separator longer than its region used to run into the line separator's.
    const sep = '•'.repeat(100);   // 300 bytes of UTF-8
    const plain = page.text('page'), joined = page.text('page', { wordSep: sep });
    if (joined.split(sep).length !== plain.split(' ').length || joined.split('\n').length !== plain.split('\n').length) throw new Error('a 300-byte separator corrupted the text');
    if (page.text('page', { wordSep: ' ', lineSep: sep }).split(sep).length !== plain.split('\n').length) throw new Error('a 300-byte line separator corrupted the text');
    // An Arabic string and a search query longer than the region take a block of their own.
    const long = plain.repeat(8);
    if (engine.strip(long).length === 0 || page.search(long).length !== 0) throw new Error('a long string broke strip or search');
    // the blocks those inputs took go when their call returns; none pins memory for the session
    if (engine._spill.length) throw new Error(`${engine._spill.length} input blocks outlived their call`);
    console.log('ok  inputs that outgrow the scratch reach the engine whole, and their blocks go with the call');
    // A spec with a field that is not a finite number is refused by name before the engine sees
    // it, and the layout in hand stays.
    const before = page.layout({ viewportW: 390, viewportH: 844 });
    for (const [bad, field] of [[{}, 'viewportW'], [{ viewportW: 390, viewportH: '844' }, 'viewportH'], [{ viewportW: 390, viewportH: 844, padTop: NaN }, 'padTop'], [{ viewportW: 390, viewportH: 844, reflow: { zoom: Infinity } }, 'reflow.zoom']]) {
      let refused = null;
      try { page.layout(bad); } catch (e) { refused = e; }
      if (!(refused instanceof TypeError) || !refused.message.includes(field)) throw new Error(`a spec with a bad ${field} was not refused by name: ${refused}`);
    }
    if (page.currentLayout !== before) throw new Error('a refused spec replaced the layout');
    const after = page.layout({ viewportW: 390, viewportH: 844 });
    if (after.scale !== before.scale || after.rows !== before.rows) throw new Error('the layout after a refused spec differs');
    console.log('ok  a bad layout spec is refused and the page lays out after it');
    // free is idempotent, and a call on a freed handle throws one clear error instead of
    // handing the engine a null
    page.free(); page.free();
    for (const call of [() => page.text(), () => page.layout({ viewportW: 390, viewportH: 844 }), () => page.h, () => engine.loadPassage([page], 2, 1)]) {
      let threw = null;
      try { call(); } catch (e) { threw = e; }
      if (!threw || threw.message !== 'QvpPage was freed') throw new Error('a call on a freed page did not throw QvpPage was freed: ' + threw);
    }
    console.log('ok  free twice is safe and a freed page refuses every call');
  }
}

// ── dress: another mushaf's ornaments ──
// The engine refuses bytes that are not a set, and the binding reports it.
let refused = false;
try { engine.loadOrnaments(new Uint8Array([0x51, 0x56, 0x50, 0x31, 0, 0])); } catch { refused = true; }
if (!refused) throw new Error('loadOrnaments accepted a page file');
console.log('ok  loadOrnaments refuses foreign bytes');
// With a set and a page on disk, the whole path: dress, dressInfo, dressGeometry, undress.
const dataDir = new URL('../dist/pages/', import.meta.url);
const setPath = new URL('ornaments.qvo', dataDir), pagePath = new URL('604.qvp', dataDir);
if (!existsSync(pagePath)) {
  skipData('dress with real data: dist/pages/604.qvp is missing');
} else if (!existsSync(setPath)) {
  // scripts/sync-test-data.sh accepts a data release that ships no set, so this is a skip
  // under QVP_REQUIRE_DATA too.
  console.log('skip dress with real data: this data release ships no dist/pages/ornaments.qvo');
} else {
  const set = engine.loadOrnaments(readFileSync(setPath));
  if (!set.count || set.list.length !== set.count) throw new Error('the set lists nothing');
  const marker = set.list.find(o => o.kind === 'ayah_mark');
  const page = engine.loadPage(readFileSync(pagePath));
  if (page.dressInfo() !== null) throw new Error('an undressed page reports a dress');
  const info = page.dress(set, { ayahMark: marker, surahHeader: set.find('surah_header', marker.style), pageFrame: set.find('page_frame', marker.style) });
  if (!info || info.ayahMarks < 1 || info.revision < 1) throw new Error('dress failed: ' + JSON.stringify(info));
  const draws = page.dressGeometry();
  if (draws.length !== info.draws) throw new Error(`dressGeometry gave ${draws.length} draws, info says ${info.draws}`);
  if (!draws.every(d => typeof d.placement === 'number' && typeof d.color === 'number')) throw new Error('a draw lacks its placement or colour');
  if (page.dressRevision() !== info.revision) throw new Error('dressRevision disagrees with dressInfo');
  // a host that scrolls the page: the size is content plus overflow, the border's top at 0
  page.layout({ viewportW: 390, viewportH: 844 });
  const lay = page.currentLayout, over = page.dressInfo().overflow;
  const shown = page.dressView(1, 390);
  if (Math.abs(shown.width - (lay.contentW + over[0] + over[2])) > 1e-3 || Math.abs(shown.height - (lay.contentH + over[1] + over[3])) > 1e-3) throw new Error('dressView size is not content plus overflow: ' + JSON.stringify(shown));
  if (Math.abs(shown.view.offsetY - shown.view.scale * over[1]) > 1e-3) throw new Error('dressView does not put the border top at 0: ' + JSON.stringify(shown));
  const wrongKind = set.list.find(o => o.kind !== 'ayah_mark');
  if (!wrongKind) console.log('skip dress by wrong kind: the set holds ayah marks only');
  else if (page.dress(set, { ayahMark: wrongKind }) !== null) throw new Error('an ornament of the wrong kind was accepted');
  // an entry of styles() is not an ornament: it has no index, and the binding says so
  for (const bad of [set.styles()[0], NaN, -1, 1.5, {}]) {
    let threw = null;
    try { page.dress(set, { ayahMark: bad }); } catch (e) { threw = e; }
    if (!(threw instanceof TypeError) || !threw.message.includes('ayahMark')) throw new Error(`ayahMark ${JSON.stringify(bad)} did not throw a TypeError naming the key: ${threw}`);
  }
  let threwColour = null;
  try { page.dress(set, { ayahMark: marker, colors: [[set.styles()[0], 0, '#ff0000']] }); } catch (e) { threwColour = e; }
  if (!(threwColour instanceof TypeError) || !threwColour.message.includes('colors[0]')) throw new Error('a colour for a non-ornament did not throw');
  if (page.dress(set, { ayahMark: 'no-such-style', surahHeader: 'no-such-style', pageFrame: 'no-such-style' }) !== null) throw new Error('unknown style names were accepted');
  page.undress();
  if (page.dressInfo() !== null) throw new Error('undress left a dress behind');
  // The style-name form: three string lookups in one call must not corrupt the spec.
  const style = set.list.find(o => o.kind === 'ayah_mark' && set.find('surah_header', o.style) && set.find('page_frame', o.style))?.style;
  // The released set holds the font markers only. A set with the scanned mushafs also has a
  // style with all three kinds, and the checks below need one.
  if (!style) console.log('skip dress by style: no style in the set has all three kinds');
  else {
    const named = page.dress(set, { ayahMark: style, surahHeader: style, pageFrame: style });
    // the preset: one mushaf dresses all three kinds, and a kind key overrides it
    const preset = page.dress(set, { style });
    if (!preset || preset.ayahMark !== named.ayahMark || preset.surahHeader !== named.surahHeader || preset.pageFrame !== named.pageFrame) throw new Error('the style preset differs from naming the three kinds');
    const mixed = page.dress(set, { style, ayahMark: marker });
    if (!mixed || mixed.ayahMark !== marker.index || mixed.surahHeader !== named.surahHeader) throw new Error('a kind key did not override the preset');
    const full = set.styles().find(s => s.style === style);
    if (!full || !full.ayahMark || !full.surahHeader || !full.pageFrame) throw new Error('styles() does not list all three kinds of ' + style);
    page.dress(set, { ayahMark: style, surahHeader: style, pageFrame: style });
    if (!named) throw new Error('dress by style name failed');
    for (const [field, kind] of [['ayahMark', 'ayah_mark'], ['surahHeader', 'surah_header'], ['pageFrame', 'page_frame']]) {
      const want = set.find(kind, style);
      if (!want || named[field] !== want.index) throw new Error(`dress by name: ${field} is ${named[field]}, expected ${want && want.index}`);
    }
    if (!(named.draws > 0)) throw new Error('dress by name drew nothing');
    // line art for one kind: the bands keep only their strokes, the medallions keep their fills
    const lined = page.dress(set, { ayahMark: style, surahHeader: style, pageFrame: style, lineArt: ['surah_header'] });
    if (!lined || lined.draws >= named.draws || lined.ayahMarks !== named.ayahMarks) throw new Error('line art for the bands changed the wrong draws');
    const linedDraws = page.dressGeometry();
    if (!linedDraws.filter(d => d.kind === 'surah_header').every(d => d.stroke)) throw new Error('a line-art band kept a fill');
    if (!linedDraws.some(d => d.kind === 'ayah_mark' && !d.stroke)) throw new Error('the medallions lost their fills');
  }
  page.undress();
  set.free(); set.free(); page.free();
  let freedSet = null;
  try { set.find('ayah_mark', marker.style); } catch (e) { freedSet = e; }
  if (!freedSet || freedSet.message !== 'QvpOrnaments was freed') throw new Error('a call on a freed set did not throw: ' + freedSet);
  console.log(`ok  dress: ${info.ayahMarks} medallions, ${info.surahHeaders} bands, ${info.draws} draws, revision ${info.revision}`);
}

// ── the atlas, and the engine's own memory ──
const atlasPath = new URL('atlas.qva', dataDir);
if (existsSync(atlasPath)) {
  const atlas = engine.loadAtlas(readFileSync(atlasPath));
  if (atlas.pageOf(2, 255) !== 42) throw new Error('atlas.pageOf(2, 255) is ' + atlas.pageOf(2, 255));
  atlas.free(); atlas.free();
  let freedAtlas = null;
  try { atlas.pageOf(2, 255); } catch (e) { freedAtlas = e; }
  if (!freedAtlas || freedAtlas.message !== 'QvpAtlas was freed') throw new Error('a call on a freed atlas did not throw: ' + freedAtlas);
  console.log('ok  atlas free twice is safe and a freed atlas refuses every call');
} else {
  console.log('skip atlas: dist/pages/atlas.qva is missing');
}
// The engine goes last. It frees a page still open, after which the page's own free does
// nothing, its cached records stay readable, and a call that reaches the engine throws. free is
// idempotent here too.
const open = existsSync(new URL('002.qvp', dataDir)) ? engine.loadPage(readFileSync(new URL('002.qvp', dataDir))) : null;
engine.free(); engine.free();
if (open) {
  open.free();
  if (!(open.words.length > 0) || open.currentLayout !== null) throw new Error('a page freed by the engine lost its cached records');
  let freedByEngine = null;
  try { open.text(); } catch (e) { freedByEngine = e; }
  // the call names whichever it meets first: the page's handle or the engine's exports
  if (!freedByEngine || !/^Qvp(Page|Engine) was freed$/.test(freedByEngine.message)) throw new Error('a page freed by the engine did not refuse a call: ' + freedByEngine);
}
let freedEngine = null;
try { engine.strip('x'); } catch (e) { freedEngine = e; }
if (!freedEngine || freedEngine.message !== 'QvpEngine was freed') throw new Error('a call on a freed engine did not throw: ' + freedEngine);
console.log('ok  engine.free() frees what is still open, releases the scratch once and refuses every call after it');
