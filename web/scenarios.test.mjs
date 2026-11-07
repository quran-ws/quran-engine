// Replay conformance/scenarios/layout.json, dress.json, zoom.json, hit.json and ink-band.json through the
// reference wrapper and compare every number with the engine's recorded answer. Needs
// web/qvp_ffi.wasm and dist/pages/042.qvp.
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { QvpEngine, CanvasRenderer, wasmUrl } from './index.mjs'

const root = new URL('..', import.meta.url)
const pagePath = new URL('dist/pages/042.qvp', root)
if (!existsSync(pagePath)) {
  if (process.env.QVP_REQUIRE_DATA) throw new Error('dist/pages/042.qvp is missing; run scripts/sync-test-data.sh')
  console.log('skip  dist/pages/042.qvp is missing; run scripts/sync-test-data.sh')
  process.exit(0)
}
const scenarios = JSON.parse(readFileSync(new URL('conformance/scenarios/layout.json', root), 'utf8'))
const engine = await QvpEngine.init(readFileSync(wasmUrl))
const page = engine.loadPage(readFileSync(pagePath))

const close = (a, b, what) => assert.ok(Math.abs(a - b) <= scenarios.tolerance * Math.max(1, Math.abs(b)), `${what}: got ${a}, engine says ${b}`)
let n = 0
for (const c of scenarios.cases) {
  const L = page.layout(c.spec)
  const tag = `${c.spec.viewportW}x${c.spec.viewportH} fill=${c.spec.fillHeight} slack=${c.spec.maxAspectSlack} crop=${c.spec.cropLeft}`
  for (const k of ['scale', 'offsetX', 'offsetY', 'contentW', 'contentH', 'lineSpacing', 'fitScale', 'fitX', 'fitY']) close(L[k], c.layout[k], `${tag} ${k}`)
  close(L.lineDy[0], c.layout.lineDy0, `${tag} lineDy[0]`)
  close(L.lineDy[L.lineDy.length - 1], c.layout.lineDyLast, `${tag} lineDy[last]`)
  close(page.layoutLineSpacingToFill(c.spec), c.lineSpacingToFill, `${tag} lineSpacingToFill`)
  close(page.layoutWastedFraction(c.spec), c.wastedFraction, `${tag} wastedFraction`)
  n++
}
console.log(`ok  ${n} layout scenarios match the engine`)

// conformance/scenarios/dress.json: the layout's fit holds the border, dressClamp holds a
// view to the dressed box, and dressView places that box for a host that scrolls the page.
// The ornament set is the synthetic one the generator wrote as hex.
const dressed = JSON.parse(readFileSync(new URL('conformance/scenarios/dress.json', root), 'utf8'))
const hex = readFileSync(new URL(`conformance/${dressed.ornaments}`, root), 'utf8').replace(/\s+/g, '')
const set = engine.loadOrnaments(Uint8Array.from(hex.match(/../g), b => parseInt(b, 16)))
const near = (a, b, what) => assert.ok(Math.abs(a - b) <= dressed.tolerance * Math.max(1, Math.abs(b)), `${what}: got ${a}, engine says ${b}`)
let m = 0
for (const c of dressed.cases) {
  const tag = `dress ${c.spec.viewportW}x${c.spec.viewportH}${c.spec.reflow ? ' reflow ' + c.spec.reflow.zoom : ''}`
  page.undress()
  page.layout(c.spec)
  assert.ok(page.dress(set, dressed.dress), `${tag}: the engine refused the dress`)
  const L = page.currentLayout
  for (const k of ['fitScale', 'fitX', 'fitY']) near(L[k], c.fit[k], `${tag} ${k}`)
  page.dressInfo().overflow.forEach((v, i) => near(v, c.overflow[i], `${tag} overflow[${i}]`))
  // the border on screen at the fit: a reflowed page is scrolled up and down only, so the
  // border must fit across the viewport
  const vb = page.dressInfo().viewBox
  const at = x => L.fitX + L.fitScale * (L.offsetX + x * L.scale)
  near(at(vb[0]), c.border[0], `${tag} border left`)
  near(at(vb[0] + vb[2]), c.border[1], `${tag} border right`)
  if (c.spec.reflow) assert.ok(c.border[0] >= -0.5 && c.border[1] <= c.spec.viewportW + 0.5, `${tag}: border ${c.border} is off the screen`)
  for (const { view, held } of c.clamps) {
    const v = page.dressClamp(view, c.spec.viewportW, c.spec.viewportH)
    for (const k of ['scale', 'offsetX', 'offsetY']) near(v[k], held[k], `${tag} dressClamp(${view.offsetX}, ${view.offsetY}) ${k}`)
  }
  for (const want of c.dressViews) {
    const got = page.dressView(want.scale, want.viewportW)
    const call = `dressView(${want.scale}, ${want.viewportW})`
    for (const k of ['scale', 'offsetX', 'offsetY']) near(got.view[k], want.view[k], `${tag} ${call} ${k}`)
    near(got.width, want.width, `${tag} ${call} width`)
    near(got.height, want.height, `${tag} ${call} height`)
    // scale 0 fits the dressed box to the width
    if (want.scale === 0) assert.ok(got.width <= want.viewportW + 0.5 && got.view.scale <= 1, `${tag} ${call}: ${got.width} wide`)
  }
  m++
}
page.undress()
set.free()
console.log(`ok  ${m} dress scenarios match the engine`)

// conformance/scenarios/zoom.json: the reader's pan and zoom, from the fit through a pinch
// under each reflowing control, and the spec each pinch asks for.
const zoomed = JSON.parse(readFileSync(new URL('conformance/scenarios/zoom.json', root), 'utf8'))
const about = (a, b, what) => assert.ok(Math.abs(a - b) <= zoomed.tolerance * Math.max(1, Math.abs(b)), `${what}: got ${a}, engine says ${b}`)
const sameView = (v, want, what) => { for (const k of ['scale', 'offsetX', 'offsetY']) about(v[k], want[k], `${what} ${k}`) }
const zspec = zoomed.spec
const ZL = page.layout(zspec)
for (const k of ['fitScale', 'fitX', 'fitY', 'contentW', 'contentH']) about(ZL[k], zoomed.fit[k], `fit ${k}`)
for (const c of zoomed.zoomAbout) sameView(engine.viewZoomAbout(c.view, c.focalX, c.focalY, c.factor, { min: c.min, max: c.max }), c.result, `zoomAbout ×${c.factor} max ${c.max}`)
sameView(engine.viewPan(zoomed.pan.view, zoomed.pan.dx, zoomed.pan.dy), zoomed.pan.result, 'pan')
for (const c of zoomed.clamp) sameView(engine.viewClamp(c.view, ZL.contentW, ZL.contentH, zspec.viewportW, zspec.viewportH), c.result, `clamp of ${c.view.offsetX}, ${c.view.offsetY}`)
for (const [vx, vy, x, y] of zoomed.viewToLayout.points) {
  const q = page.viewToLayout(zoomed.viewToLayout.view, vx, vy)
  about(q.x, x, `viewToLayout(${vx}, ${vy}) x`); about(q.y, y, `viewToLayout(${vx}, ${vy}) y`)
}
for (const c of zoomed.isZoomed) assert.equal(page.zoomIsZoomed(c.zoom, c.view, c.fitScale), c.result, `isZoomed at scale ${c.view.scale}`)
const fitted = { scale: ZL.fitScale, offsetX: ZL.fitX, offsetY: ZL.fitY }
for (const c of zoomed.pinches) {
  const tag = `${c.mode} pinch`
  // each pinch starts from the printed layout, as the generator did
  page.layout(zspec)
  const got = page.zoomPinch(zspec, c.start, fitted, c.factor, c.focalX, c.focalY)
  assert.equal(got.zoom.mode, c.zoom.mode, tag); assert.equal(got.zoom.step, c.zoom.step, tag); about(got.zoom.zoom, c.zoom.zoom, `${tag} zoom`)
  sameView(got.view, c.view, tag)
  assert.equal(got.relaid, c.relaid, tag)
  assert.equal(page.zoomIsZoomed(got.zoom, got.view, ZL.fitScale), c.zoomedAfter, `${tag} zoomed after`)
  // the spec the control asks for comes back whole: the zoom in force and the host's knobs
  const asks = page.zoomSpec(zspec, got.zoom)
  about(asks.viewportW, c.spec.viewportW, `${tag} spec viewportW`); about(asks.padTop, c.spec.padTop, `${tag} spec padTop`)
  assert.equal(asks.surahFrames, c.spec.surahFrames, `${tag} spec surahFrames`)
  about(asks.reflow.zoom, c.spec.reflowZoom, `${tag} spec reflow zoom`)
  assert.equal(asks.reflow.fill, c.spec.fill, `${tag} spec fill`)
  about(asks.reflow.wordGap, c.spec.wordGap, `${tag} spec wordGap`); about(asks.reflow.maxStretch, c.spec.maxStretch, `${tag} spec maxStretch`)
}

// On the printed page the control reflows nothing, and the host's knobs come back as it set them
{
  const want = zoomed.printedSpec.spec
  const asks = page.zoomSpec(zspec, zoomed.printedSpec.zoom)
  assert.ok(asks.reflow, 'printed spec keeps the reflow knobs')
  about(asks.viewportW, want.viewportW, 'printed spec viewportW'); about(asks.padTop, want.padTop, 'printed spec padTop')
  assert.equal(asks.surahFrames, want.surahFrames, 'printed spec surahFrames')
  about(asks.reflow.zoom, want.reflowZoom, 'printed spec reflow zoom')
  assert.equal(asks.reflow.fill, want.fill, 'printed spec fill')
  about(asks.reflow.wordGap, want.wordGap, 'printed spec wordGap'); about(asks.reflow.maxStretch, want.maxStretch, 'printed spec maxStretch')
}

// A pinch whose fingers drift, replayed as a page view runs it: the engine works from the
// control and the focal point the gesture began on, with the factor of the whole gesture; the
// magnifying glass zooms the view the pinch began on, and the reflowing controls take the view
// the last frame returned. The host pans the answer by the drift and holds it.
for (const g of zoomed.drifts) {
  page.layout(zspec)
  let base = g.view
  g.frames.forEach((f, i) => {
    const tag = `${g.mode} drifting pinch, frame ${i}`
    const c = page.zoomPinch(zspec, g.start, g.start.mode === 'magnify' ? g.view : base, f.factor, g.focalX, g.focalY)
    base = c.view
    const drawn = page.dressClamp(engine.viewPan(c.view, f.focalX - g.focalX, f.focalY - g.focalY), zspec.viewportW, zspec.viewportH)
    assert.equal(c.zoom.mode, f.zoom.mode, tag); assert.equal(c.zoom.step, f.zoom.step, tag); about(c.zoom.zoom, f.zoom.zoom, `${tag} zoom`)
    assert.equal(c.relaid, f.relaid, `${tag} relaid`)
    sameView(drawn, f.view, tag)
  })
}
page.free()
const driftFrames = zoomed.drifts.reduce((n, g) => n + g.frames.length, 0)
console.log(`ok  ${zoomed.zoomAbout.length + zoomed.clamp.length + zoomed.isZoomed.length + zoomed.pinches.length + 3 + driftFrames} zoom scenarios match the engine`)

// conformance/scenarios/hit.json: the exact and the gap-aware hit tests at a point on a word's
// outline, on a word's box alone, on a decoration's outline and on a decoration's box alone,
// in page units and in viewport px.
const hits = JSON.parse(readFileSync(new URL('conformance/scenarios/hit.json', root), 'utf8'))
const hitPage = engine.loadPage(readFileSync(pagePath))
hitPage.layout(hits.spec)
const sameHit = (got, want, what) => {
  if (want === null) return assert.equal(got, null, `${what}: got a hit, engine says none`)
  assert.ok(got, `${what}: got no hit`)
  for (const k of ['word', 'path', 'decoration', 'line', 'isExact']) assert.equal(got[k], want[k], `${what} ${k}`)
  assert.ok(Math.abs(got.distance - want.distance) <= hits.tolerance, `${what} distance: got ${got.distance}, engine says ${want.distance}`)
}
for (const c of hits.cases) {
  const tag = `hit ${c.name} (${c.units} ${c.x}, ${c.y})`
  const view = c.units === 'view'
  sameHit(view ? hitPage.hitTestExactView(c.x, c.y) : hitPage.hitTestExact(c.x, c.y), c.exact, `${tag} exact`)
  sameHit(view ? hitPage.hitTestView(c.x, c.y) : hitPage.hitTest(c.x, c.y), c.hit, `${tag} gap-aware`)
}
hitPage.free()
console.log(`ok  ${hits.cases.length} hit scenarios match the engine`)

// conformance/scenarios/ink-band.json: the band of the page a host keeps drawn, for a page
// that fits, a magnified page drawn a band at a time, pans inside one step and across it, the
// host's own limits, and no screen at all; then the same page dressed.
const inks = JSON.parse(readFileSync(new URL('conformance/scenarios/ink-band.json', root), 'utf8'))
const inkPage = engine.loadPage(readFileSync(pagePath))
const inkLayout = inkPage.layout(inks.spec)
assert.ok(Math.abs(inkLayout.contentH - inks.contentH) <= inks.tolerance * inks.contentH, `ink band contentH: got ${inkLayout.contentH}, engine says ${inks.contentH}`)
const inkCases = (cases, tag) => {
  for (const c of cases) {
    const got = inkPage.layoutInkBand(c.view, c.viewportW, c.viewportH, { maxHeight: c.maxHeight, maxPixels: c.maxPixels })
    for (const k of ['top', 'height']) {
      assert.ok(Math.abs(got[k] - c.band[k]) <= inks.tolerance * Math.max(1, Math.abs(c.band[k])), `${tag} ${c.name} ${k}: got ${got[k]}, engine says ${c.band[k]}`)
    }
  }
}
inkCases(inks.cases, 'ink band')
assert.equal(inks.cases.length, 14)
// The same page dressed in the synthetic set: the band is that of the dressed box, which
// holds the border above and below the content.
const inkHex = readFileSync(new URL(`conformance/${inks.dressed.ornaments}`, root), 'utf8').replace(/\s+/g, '')
const inkSet = engine.loadOrnaments(Uint8Array.from(inkHex.match(/../g), b => parseInt(b, 16)))
assert.ok(inkPage.dress(inkSet, inks.dressed.dress), 'ink band: the engine refused the dress')
inkPage.dressInfo().overflow.forEach((v, i) => assert.ok(Math.abs(v - inks.dressed.overflow[i]) <= inks.tolerance * Math.max(1, Math.abs(inks.dressed.overflow[i])), `ink band overflow[${i}]: got ${v}, engine says ${inks.dressed.overflow[i]}`))
inkCases(inks.dressed.cases, 'dressed ink band')
assert.equal(inks.dressed.cases.length, 17)
console.log(`ok  ${inks.cases.length} ink band scenarios and ${inks.dressed.cases.length} dressed ones match the engine`)

// CanvasRenderer draws a dressed page's ornament layer into the engine's band: the layer has the
// base ink's rows and is blitted where the base ink is, so the border is not cut off at the
// tallest canvas or the ink budget. A stub canvas records each canvas's size and every blit.
{
  const ctx = canvas => {
    const calls = { blits: [] }
    return new Proxy({ canvas, calls }, {
      get(t, k) {
        if (k in t) return t[k]
        if (k === 'drawImage') return (img, x, y) => calls.blits.push({ img, h: img.height, y })
        return () => {}
      },
      set() { return true },
    })
  }
  const canvas = (width = 1, height = 1) => { const c = { width, height }; c.ctx = ctx(c); c.getContext = () => c.ctx; return c }
  const had = { document: globalThis.document, Path2D: globalThis.Path2D }
  globalThis.document = { createElement: () => canvas() }
  globalThis.Path2D = class { moveTo() {} lineTo() {} quadraticCurveTo() {} bezierCurveTo() {} closePath() {} rect() {} roundRect() {} }
  const L = inkPage.currentLayout
  const [W, H] = [inks.spec.viewportW, inks.spec.viewportH]
  const views = [
    ['fitted', { scale: L.fitScale, offsetX: L.fitX, offsetY: L.fitY }, 64 << 20],
    ['magnified', { scale: 8, offsetX: 0, offsetY: -2000 }, 64 << 20],
    ['magnified at the bottom', { scale: 8, offsetX: 0, offsetY: H - 8 * L.contentH }, 64 << 20],
    ['under a budget of one screen', { scale: 8, offsetX: 0, offsetY: -2000 }, 4 * W * 2 * H * 2],
    ['under a budget the bare page fits', { scale: 1, offsetX: 0, offsetY: 0 }, 4 * W * 2 * Math.ceil(L.contentH * 2 + 2)],
  ]
  let drawn = 0
  for (const dpr of [1, 2, 3]) {
    for (const [name, view, budget] of views) {
      const c = canvas(W * dpr, H * dpr)
      const r = new CanvasRenderer(c)
      r.inkBudget = budget
      r.draw(inkPage, view, dpr)
      const band = inkPage.layoutInkBand({ scale: view.scale * dpr, offsetX: view.offsetX * dpr, offsetY: view.offsetY * dpr }, W * dpr, H * dpr,
        { maxHeight: 16384, maxPixels: budget / 4 })
      const rows = Math.max(1, Math.ceil(band.height * view.scale * dpr))
      const y = Math.round(dpr * (view.offsetY + band.top * view.scale))
      const tag = `renderer ${name} at dpr ${dpr}`
      const [orn, base] = c.ctx.calls.blits
      assert.ok(orn && orn.img === r.ornamentLayer && base && base.img === r.base, `${tag}: the ornament layer, then the base ink`)
      assert.equal(orn.h, rows, `${tag}: the ornament layer is the band`)
      assert.equal(base.h, rows, `${tag}: the base ink is the band`)
      assert.equal(orn.y, y, `${tag}: the ornament layer sits at the band's top`)
      assert.equal(base.y, y, `${tag}: the base ink sits at the band's top`)
      assert.ok(rows <= 16384, `${tag}: ${rows} rows`)
      // the band covers the screen, so every row of the border on it
      assert.ok(y <= 0 && y + rows >= H * dpr || band.height * view.scale * dpr <= 16384 && band.top <= 0, `${tag}: the band covers the screen`)
      drawn++
    }
  }
  // an undressed page has no ornament layer
  inkPage.undress()
  const bare = canvas(W, H)
  const r = new CanvasRenderer(bare)
  r.draw(inkPage, { scale: 1, offsetX: 0, offsetY: 0 }, 1)
  assert.equal(r.ornamentLayer, null)
  assert.deepEqual(bare.ctx.calls.blits.map(b => b.img), [r.base])
  globalThis.document = had.document
  globalThis.Path2D = had.Path2D
  console.log(`ok  CanvasRenderer draws a dressed page's border into the engine's band (${drawn} views)`)
}
inkPage.free()
inkSet.free()
