// Replay conformance/scenarios/passage.json through the reference wrapper and compare every
// number with the engine's recorded answer. Needs web/qvp_ffi.wasm and the pages in dist/pages.
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { QvpEngine, wasmUrl } from './index.mjs'

const root = new URL('..', import.meta.url)
const scenarios = JSON.parse(readFileSync(new URL('conformance/scenarios/passage.json', root), 'utf8'))
const pagePath = n => new URL(`dist/pages/${String(n).padStart(3, '0')}.qvp`, root)
const missing = [...new Set(scenarios.cases.flatMap(c => c.pages))].filter(n => !existsSync(pagePath(n)))
if (missing.length) {
  console.log(`skip  dist/pages is missing ${missing.map(n => String(n).padStart(3, '0') + '.qvp').join(', ')}; run scripts/sync-test-data.sh`)
  process.exit(0)
}
const engine = await QvpEngine.init(readFileSync(wasmUrl))

const close = (a, b, what) => assert.ok(Math.abs(a - b) <= scenarios.tolerance, `${what}: got ${a}, engine says ${b}`)
let n = 0
for (const c of scenarios.cases) {
  const pages = c.pages.map(p => engine.loadPage(readFileSync(pagePath(p))))
  const passage = c.basmalah ? engine.loadBasmalah(pages, c.surah) : engine.loadPassage(pages, c.surah, c.from, c.to)
  // The passage keeps what it needs, so the pages go before the layout.
  for (const p of pages) p.free()
  const tag = `${c.basmalah ? 'basmalah of ' : ''}${c.surah}:${c.from}-${c.to} at ${c.spec.width} ${c.spec.align} rows ${c.spec.maxRows}`
  const L = passage.layout(c.spec)
  assert.ok(L, `${tag}: the spec was refused`)
  assert.equal(passage.currentLayout, L, tag)
  const want = c.layout
  for (const k of ['width', 'height', 'scale', 'lineSpacing']) close(L[k], want[k], `${tag} ${k}`)
  assert.equal(L.isTruncated, want.isTruncated, `${tag} isTruncated`)
  assert.equal(L.draws.length, want.draws, `${tag} draws`)
  assert.equal(L.rows.length, want.rows.length, `${tag} rows`)
  assert.equal(L.words.length, want.words.length, `${tag} words`)
  L.rows.forEach((r, i) => [r.x0, r.y0, r.x1, r.y1, r.baseline].forEach((v, j) => close(v, want.rows[i][j], `${tag} row ${i}[${j}]`)))
  L.words.forEach((w, i) => {
    assert.deepEqual([w.page, w.ayah, w.word, w.row], want.words[i].slice(0, 4), `${tag} word ${i}`)
    ;[w.x0, w.y0, w.x1, w.y1].forEach((v, j) => close(v, want.words[i][4 + j], `${tag} word ${i} box[${j}]`))
    ;[w.wordX0, w.wordY0, w.wordX1, w.wordY1].forEach((v, j) => close(v, want.words[i][8 + j], `${tag} word ${i} alone[${j}]`))
  })
  if (want.ellipsis) {
    assert.ok(L.ellipsis, `${tag} ellipsis`)
    ;[L.ellipsis.x0, L.ellipsis.y0, L.ellipsis.x1, L.ellipsis.y1].forEach((v, j) => close(v, want.ellipsis[j], `${tag} ellipsis[${j}]`))
  } else {
    assert.equal(L.ellipsis, null, `${tag} ellipsis`)
  }
  // Every draw names a page the passage was loaded from and a placement the layout returned.
  for (const d of L.draws) {
    assert.ok(c.pages.includes(d.page), `${tag} draw names page ${d.page}`)
    assert.ok(d.placement < L.placements.length, `${tag} draw placement ${d.placement}`)
  }
  if (c.basmalah) assert.equal(passage.ayahs.length, 0, tag)
  else assert.deepEqual(passage.ayahs.map(a => [a.surah, a.ayah]), Array.from({ length: c.to - c.from + 1 }, (_, i) => [c.surah, c.from + i]), tag)
  passage.free()
  passage.free()
  n++
}
console.log(`ok  ${n} passage scenarios match the engine`)
