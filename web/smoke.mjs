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

const cjs = (await import('./index.cjs')).default;
if (!cjs.QvpEngine) throw new Error('CJS entry did not expose QvpEngine');
console.log('ok  both entry points expose QvpEngine');

// ── dress: another mushaf's ornaments ──
// The engine refuses bytes that are not a set, and the binding reports it.
let refused = false;
try { engine.loadOrnaments(new Uint8Array([0x51, 0x56, 0x50, 0x31, 0, 0])); } catch { refused = true; }
if (!refused) throw new Error('loadOrnaments accepted a page file');
console.log('ok  loadOrnaments refuses foreign bytes');
// With a set and a page on disk, the whole path: dress, dressInfo, dressGeometry, undress.
const dataDir = new URL('../dist/pages/', import.meta.url);
const setPath = new URL('ornaments.qvo', dataDir), pagePath = new URL('604.qvp', dataDir);
if (existsSync(setPath) && existsSync(pagePath)) {
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
  set.free(); page.free();
  console.log(`ok  dress: ${info.ayahMarks} medallions, ${info.surahHeaders} bands, ${info.draws} draws, revision ${info.revision}`);
} else {
  console.log('skip dress with real data: dist/pages/ornaments.qvo or 604.qvp is missing');
}
