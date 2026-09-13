// Replay conformance/scenarios/layout.json through the reference wrapper and compare every
// number with the engine's recorded answer. Needs web/qvp_ffi.wasm and dist/pages/042.qvp.
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
  for (const k of ['scale', 'ox', 'oy', 'contentW', 'contentH', 'pitch', 'fitScale', 'fitX', 'fitY']) close(L[k], c.layout[k], `${tag} ${k}`)
  close(L.lineDy[0], c.layout.lineDy0, `${tag} lineDy[0]`)
  close(L.lineDy[L.lineDy.length - 1], c.layout.lineDyLast, `${tag} lineDy[last]`)
  close(page.layoutGapToFill(c.spec), c.gapToFill, `${tag} gapToFill`)
  n++
}
page.free()
console.log(`ok  ${n} layout scenarios match the engine`)
