// Replay conformance/scenarios/layout.json and dress.json through the reference wrapper and
// compare every number with the engine's recorded answer. Needs web/qvp_ffi.wasm and dist/pages/042.qvp.
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { QvpEngine, wasmUrl } from './index.mjs'

const root = new URL('..', import.meta.url)
const pagePath = new URL('dist/pages/042.qvp', root)
if (!existsSync(pagePath)) {
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
page.free()
console.log(`ok  ${m} dress scenarios match the engine`)
