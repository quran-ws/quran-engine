// Replay conformance/scenarios/letters.json through the reference wrapper: the letter layer of a
// page (attach, read, select, colour, hit-test) and the draw list and draw revision it changes.
// The fixtures are the synthetic pages and layers in conformance/, so this test needs no data
// and never skips. Needs web/qvp_ffi.wasm.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { QvpEngine, CanvasRenderer, Sel, T, LETTER_PART, NONE, wasmUrl } from './index.mjs'

const root = new URL('..', import.meta.url)
const S = JSON.parse(readFileSync(new URL('conformance/scenarios/letters.json', root), 'utf8'))
const engine = await QvpEngine.init(readFileSync(wasmUrl))

const hex = name => { const h = readFileSync(new URL(`conformance/${name}`, root), 'utf8').replace(/\s+/g, ''); return Uint8Array.from(h.match(/../g) || [], b => parseInt(b, 16)) }
const fromHex = h => Uint8Array.from(h.match(/../g) || [], b => parseInt(b, 16))
const layerOf = name => hex(S.fixtures[name].letters)
/** a fixture's page with no layer */
const bare = name => engine.loadPage(hex(S.fixtures[name].page))
/** a fixture's page with its layer, and the words sidecar when asked */
const loaded = (name, words) => {
  const p = bare(name)
  assert.equal(p.attachLetters(layerOf(name)), true, `${name}: the layer attaches`)
  if (words) p.attachWords(S.words)
  return p
}
const close = (a, b, what) => assert.ok(Math.abs(a - b) <= S.tolerance * Math.max(1, Math.abs(b)), `${what}: got ${a}, engine says ${b}`)
// In the scenario -1 is QVP_NONE in inputs; the wrapper takes -1 or NONE for an absent index.
const input = v => (v === -1 ? NONE : v)
const pairs = flat => { const out = []; for (let i = 0; i < flat.length; i += 2) out.push([flat[i], flat[i + 1]]); return out }
const reds = p => [...p.colors()].flatMap((c, i) => (c === S.red ? [i] : []))
const drawList = p => pairs(p.layoutDrawList())
const found = list => list.map(f => [f.word, f.letter])
/** the selector of a case, through the Sel helper that makes it */
const selector = s => {
  switch (s.selector) {
    case 16: return Sel.letter(s.a, s.b, s.c)
    case 17: return Sel.letterMark(s.a, s.b, s.c)
    case 18: return Sel.letterMarkNamed(s.a, s.b, s.c)
    case 19: return Sel.wordChars(s.a, s.b, s.c)
    default: return s
  }
}
/** the options of a hit case: null passes NULL, otherwise the raw QvpHitOptions fields */
const options = o => (o === null ? null : { maxDistance: o.maxDistance, gapBias: o.gapBias, preferExact: o.preferExact })
const sameInfo = (got, want, what) => {
  assert.ok(got, `${what}: no letter`)
  for (const k of ['word', 'index', 'hasInk', 'nMoreAt']) assert.equal(got[k], want[k], `${what} ${k}`)
  assert.equal(got.at, want.at, `${what} at`)
  assert.equal(got.moreAt, want.moreAt, `${what} moreAt`)
  assert.equal(got.codePoint, want.text, `${what} code point`)
  assert.equal(got.text, want.char, `${what} text`)
  assert.equal(got.nMembers, want.nMembers, `${what} nMembers`)
  assert.equal(got.nMarks, want.nMarks, `${what} nMarks`)
  assert.equal(got.members.length, want.nMembers, `${what} members`)
  assert.equal(got.marks.length, want.nMarks, `${what} marks`)
  for (const k of ['x0', 'y0', 'x1', 'y1']) close(got.box[k], want[k], `${what} ${k}`)
}
// The scenario names a mark by its engine name; the wrapper gives the id and the name, and the
// name is the one the engine's table gives that id.
const sameMark = (got, what) => {
  assert.equal(typeof got.mark, 'number', `${what} mark is an id`)
  assert.equal(got.markName, engine.markName(got.mark), `${what} markName is the engine's name of the id`)
}
const sameMembers = (got, want, what) => {
  got.forEach((m, k) => sameMark(m, `${what} ${k}`))
  assert.deepEqual(got.map(m => ({ path: m.path, at: m.at, mark: m.markName, isBody: m.isBody, isCut: m.isCut })), want, what)
}
const sameHit = (p, got, want, what) => {
  if (want === null) { assert.equal(got, null, what); return }
  assert.ok(got, `${what}: no hit`)
  for (const k of ['word', 'letter', 'path', 'at', 'line', 'isExact', 'isBody']) assert.equal(got[k], want[k], `${what} ${k}`)
  sameMark(got, what)
  assert.equal(got.markName, want.mark, `${what} mark`)
  close(got.distance, want.distance, `${what} distance`)
  assert.equal(got.wordKey, p.wordKey(got.word), `${what} wordKey`)
}

// The constants the wrapper names
assert.deepEqual(LETTER_PART, { ALL: 0, BODY_AND_DOTS: 1, MARKS: 2 })
assert.deepEqual(Sel.letter(0, 1), { selector: 16, a: 0, b: 1, c: 0 })
assert.deepEqual(Sel.letter(0, 1, 'bodyAndDots'), { selector: 16, a: 0, b: 1, c: 1 })
assert.deepEqual(Sel.letter(0, 1, LETTER_PART.MARKS), { selector: 16, a: 0, b: 1, c: 2 })
assert.deepEqual(Sel.letterMarkNamed(0, 0, 'fathah'), { selector: 18, a: 0, b: 0, c: 1 })
assert.deepEqual(Sel.wordChars(1, 2), { selector: 19, a: 1, b: 2, c: 2 })
assert.throws(() => Sel.letter(0, 0, 'body'), TypeError)

// ── letters: what a layer says about each word ──
for (const c of S.letters) {
  const p = loaded(c.fixture, c.words)
  const tag = c.name
  assert.equal(p.hasLetters(), c.hasLetters, `${tag} hasLetters`)
  assert.equal(p.nPaths, c.nPaths, `${tag} nPaths`)
  assert.equal(p.colors().length, c.nPaths, `${tag} colours`)
  assert.equal(p.table.length / 8, c.nPaths, `${tag} geometry rows`)
  assert.deepEqual(c.counts.map((_, w) => p.letterCount(w)), c.counts, `${tag} counts`)
  for (const want of c.letters) {
    const what = `${tag} letter ${want.word}.${want.index}`
    const got = p.letter(want.word, want.index)
    sameInfo(got, want, what)
    sameMembers(got.members, want.members, `${what} members`)
    sameMembers(p.letterMembers(want.word, want.index), want.members, `${what} letterMembers`)
    // a letter's text is its character, one Unicode scalar, and codePoint the same as a number
    assert.equal(typeof got.text, 'string', `${what} text is a string`)
    assert.equal([...got.text].length, 1, `${what} text is one character`)
    assert.equal(got.text.codePointAt(0), got.codePoint, `${what} codePoint`)
    // a letter's marks are its members that are not body, in the engine's order
    assert.deepEqual(got.marks, got.members.filter(m => !m.isBody), `${what} marks`)
  }
  // letters(w) is every letter of the word, in order
  for (let w = 0; w < c.counts.length; w++) {
    assert.deepEqual(p.letters(w), c.letters.filter(l => l.word === w).map(l => p.letter(w, l.index)), `${tag} letters(${w})`)
  }
  for (const m of c.missing) {
    assert.equal(p.letter(m.word, m.letter), null, `${tag} missing ${m.word}.${m.letter}`)
    assert.deepEqual(p.letterMembers(m.word, m.letter), [], `${tag} missing members ${m.word}.${m.letter}`)
  }
  for (const { path, value } of c.pathLetters) assert.deepEqual(p.pathLetter(path), value, `${tag} pathLetter(${path})`)
  for (const [w, at, letter] of c.offsets) assert.equal(p.letterAtOffset(w, input(at)), letter === null ? -1 : letter, `${tag} letterAtOffset(${w}, ${at})`)
  p.free()
}
console.log(`ok  ${S.letters.length} letter layers read as the engine says`)

// ── selections: the four letter selectors, colours, draw revision and draw list ──
for (const c of S.selections) {
  const p = loaded(c.fixture, c.words)
  p.layout(S.layouts.printed)
  p.tick(0)
  const h = p.style(selector(c.selector), S.red, { ms: 0, layer: 0 })
  assert.equal(h !== 0, c.handle, `${c.name}: handle`)
  p.tick(0)
  assert.deepEqual(reds(p), c.red, `${c.name}: red`)
  assert.equal(p.layoutDrawRevision(), c.drawRevision, `${c.name}: draw revision`)
  assert.deepEqual(drawList(p), c.drawList, `${c.name}: draw list`)
  p.free()
}
console.log(`ok  ${S.selections.length} letter selections colour and draw as the engine says`)

// ── queries: find and colour letters by what they are ──
for (const c of S.queries) {
  const p = loaded(c.fixture, false)
  p.layout(S.layouts.printed)
  p.tick(0)
  assert.deepEqual(found(p.findLetters(c.query)), c.found, `${c.name}: found`)
  const h = p.styleLetters(c.query, S.red, { ms: 0, layer: 0 })
  assert.equal(h !== 0, c.handle, `${c.name}: handle`)
  p.tick(0)
  assert.deepEqual(reds(p), c.red, `${c.name}: red`)
  assert.deepEqual(drawList(p), c.drawList, `${c.name}: draw list`)
  p.free()
}
// the friendly query: a character, a mark name and a part name
{
  const p = loaded('letters', false)
  assert.deepEqual(found(p.findLetters({ text: 'ل' })), [[0, 1], [1, 0]])
  assert.deepEqual(found(p.findLetters({ scope: 'page', mark: 'dot' })), [[0, 0]])
  assert.deepEqual(found(p.findLetters({ scope: [1], part: 'marks' })), [[1, 0], [1, 1]])
  assert.deepEqual(found(p.findLetters({ scope: T.word(2) })), [[2, 0]])
  assert.throws(() => p.findLetters({ text: 'لا' }), TypeError)
  assert.throws(() => p.findLetters({ text: '\ud800' }), TypeError)
  assert.throws(() => p.styleLetters({ text: 'بَ' }, S.red), TypeError)
  // '' is any letter, as 0 and a text left out are
  assert.deepEqual(found(p.findLetters({ text: '' })), found(p.findLetters({})))
  assert.deepEqual(found(p.findLetters({ text: 0 })), found(p.findLetters({})))
  p.free()
}
console.log(`ok  ${S.queries.length} letter queries find and colour as the engine says`)

// ── hits: the letter under a point, in page units and in view px ──
for (const c of S.hits) {
  const p = loaded(c.fixture, c.words)
  if (c.layout !== null) p.layout(S.layouts[c.layout])
  p.tick(0)
  const got = c.units === 'page' ? p.hitTestLetter(c.x, c.y, options(c.options)) : p.hitTestLetterView(c.x, c.y, options(c.options))
  sameHit(p, got, c.hit, c.name)
  p.free()
}
console.log(`ok  ${S.hits.length} letter hits match the engine`)

// ── sequences: ordered calls on a page that starts with no layer ──
for (const seq of S.sequences) {
  const p = bare(seq.fixture), handles = {}
  seq.steps.forEach((st, k) => {
    const what = `${seq.name}, step ${k} (${st.call})`
    switch (st.call) {
      case 'tick': assert.equal(p.tick(st.ms), st.animating, what); break
      case 'attach': assert.equal(p.attachLetters(layerOf(st.letters)) ? 0 : -1, st.result, what); break
      case 'attachBytes': assert.equal(p.attachLetters(fromHex(st.hex)) ? 0 : -1, st.result, what); break
      case 'attachWords': assert.equal(p.attachWords(S.words), st.result, what); break
      case 'layout': p.layout(S.layouts[st.spec]); break
      case 'style': { const h = p.style(selector(st.selector), st.rgba, { ms: st.ms, layer: 0 }); handles[st.as] = h; assert.equal(h !== 0, st.handle, what); break }
      case 'styleLetters': { const h = p.styleLetters(st.query, st.rgba, { ms: st.ms, layer: 0 }); handles[st.as] = h; assert.equal(h !== 0, st.handle, what); break }
      case 'remove': assert.equal(p.removeStyle(handles[st.handle]), st.result, what); break
      case 'theme': p.theme({ ink: st.ink, dots: st.dots, ms: st.ms }); break
      case 'highlight': handles[st.as] = p.highlight(T.word(st.word), { mode: 'ink', ink: st.ink, ms: st.ms }); break
      case 'removeHighlight': assert.equal(p.removeHighlight(handles[st.handle]), st.result, what); break
      // qvp_mask_transition is a declared gap of this wrapper (docs/API-PARITY.md): call the engine
      case 'maskTransition': p.e.ex.qvp_mask_transition(p.h, st.ms); break
      case 'mask': p.mask(T.word(st.word), 'hide'); break
      case 'unmask': p.unmask(); break
      case 'hasLetters': assert.equal(p.hasLetters(), st.value, what); break
      case 'nPaths':
        assert.equal(p.nPaths, st.value, what)
        assert.equal(p.table.length / 8, st.value, `${what}: geometry rows`)
        assert.equal(p.colors().length, st.value, `${what}: colours`)
        break
      case 'rows': assert.deepEqual(Array.from({ length: st.count }, (_, i) => Array.from(p.table.subarray((st.first + i) * 8, (st.first + i + 1) * 8))), st.value, what); break
      case 'colors': assert.deepEqual([...p.colors()], st.value, what); break
      case 'styledPaths': assert.deepEqual(p.styledPaths(), st.value, what); break
      case 'drawRevision': assert.equal(p.layoutDrawRevision(), st.value, what); break
      case 'drawList': assert.deepEqual(drawList(p), st.value, what); break
      case 'letterCount': assert.equal(p.letterCount(st.word), st.value, what); break
      case 'letterInfo': {
        const got = p.letter(st.word, st.letter)
        if (st.value === null) assert.equal(got, null, what); else sameInfo(got, st.value, what)
        break
      }
      case 'letterMembers': sameMembers(p.letterMembers(st.word, st.letter), st.value, what); break
      case 'pathLetter': assert.deepEqual(p.pathLetter(st.path), st.value, what); break
      case 'letterAtOffset': assert.equal(p.letterAtOffset(st.word, input(st.at)), st.value === null ? -1 : st.value, what); break
      case 'findLetters': assert.deepEqual(found(p.findLetters(st.query)), st.value, what); break
      case 'hitTestLetter': case 'hitTestLetterView': {
        const got = st.call === 'hitTestLetter' ? p.hitTestLetter(st.x, st.y, options(st.options)) : p.hitTestLetterView(st.x, st.y, options(st.options))
        sameHit(p, got, st.value, what)
        break
      }
      default: throw new Error(`${what}: a step this replay does not know`)
    }
  })
  p.free()
}
console.log(`ok  ${S.sequences.length} letter sequences replay as the engine says`)

// ── the wrapper's own state after an attach ──
{
  const p = bare('letters')
  const before = p.table.slice()
  assert.equal(p.nPaths, 8)
  assert.equal(p.hasLetters(), false)
  assert.equal(p.layoutDrawRevision(), 0)
  // a refused attach changes nothing
  assert.equal(p.attachLetters(fromHex('51564c31deadbeef')), false)
  assert.equal(p.nPaths, 8)
  assert.equal(p.attachLetters(layerOf('letters')), true)
  assert.equal(p.nPaths, 10)
  assert.equal(p.colors().length, 10)
  assert.deepEqual(p.table.subarray(0, 64), before, 'the page rows stay')
  p.layout(S.layouts.printed)
  p.tick(0)
  const rev = p.layoutDrawRevision()
  p.style(Sel.letter(0, 1), S.red)
  p.tick(0)
  p.colors()
  assert.notEqual(p.layoutDrawRevision(), rev, 'a letter rule changes the draw revision')
  p.free()
}

// ── CanvasRenderer: the paths cover the pieces, and the cached draw list follows the revision ──
{
  const ctx = canvas => {
    const calls = { fills: [] }
    return new Proxy({ canvas, calls }, {
      get(t, k) {
        if (k in t) return t[k]
        if (k === 'fill') return path => calls.fills.push(path.index)
        return () => {}
      },
      set() { return true },
    })
  }
  const canvas = (width = 1, height = 1) => { const c = { width, height }; c.ctx = ctx(c); c.getContext = () => c.ctx; return c }
  const had = { document: globalThis.document, Path2D: globalThis.Path2D }
  let made = 0
  globalThis.document = { createElement: () => canvas() }
  globalThis.Path2D = class { constructor() { this.index = made++ } moveTo() {} lineTo() {} quadraticCurveTo() {} bezierCurveTo() {} closePath() {} rect() {} roundRect() {} }
  const p = bare('letters')
  assert.equal(p.buildPaths().length, 8)
  const L = p.layout(S.layouts.printed)
  const view = { scale: L.fitScale, offsetX: L.fitX, offsetY: L.fitY }
  const c = canvas(200, 200), r = new CanvasRenderer(c)
  // which page paths a frame filled: the paths are numbered as they were built
  const frame = () => {
    const paths = p.buildPaths(), at = new Map(paths.map((q, i) => [q.index, i]))
    c.ctx.calls.fills.length = 0
    p.tick(0)
    r.draw(p, view, 1)
    return { base: r.stats.basePaths, overlay: c.ctx.calls.fills.map(i => at.get(i)), list: pairs(r.list).map(([i]) => i) }
  }
  assert.deepEqual(frame().list, [0, 1, 2, 3, 4, 5, 6, 7], 'no layer: the page paths')
  assert.equal(p.attachLetters(layerOf('letters')), true)
  assert.equal(p.buildPaths().length, 10, 'the paths cover the pieces')
  // a letter rule swaps the cut path for its pieces with no layout change
  p.style(Sel.letter(0, 1), S.red)
  const swapped = frame()
  assert.deepEqual(swapped.list, [8, 9, 1, 2, 3, 4, 5, 6, 7], 'the cached draw list follows the revision')
  assert.deepEqual(swapped.overlay, [9], 'the styled piece draws over the base ink')
  assert.equal(swapped.base, 8, 'the base ink holds the other piece and the page paths')
  // the same colour on the whole word draws the cut path whole again
  p.clearStyles()
  p.style(Sel.word(0), S.red)
  const whole = frame()
  assert.deepEqual(whole.list, [0, 1, 2, 3, 4, 5, 6, 7], 'the cut path draws whole')
  assert.deepEqual(whole.overlay, [0, 1, 2])
  p.free()
  globalThis.document = had.document
  globalThis.Path2D = had.Path2D
  console.log('ok  CanvasRenderer draws the pieces after an attach and follows the draw revision')
}
engine.free()
