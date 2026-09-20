// Verify passage identity, complete ranges, layout bounds and decoration ownership.
import assert from 'node:assert/strict'
import { readFile, access } from 'node:fs/promises'
import { performance } from 'node:perf_hooks'
import { decodeGeometry } from './lite.mjs'
import { QvpPassage } from './lite-passage.mjs'
import { clearance, preparePage } from './lite-passage-geometry.mjs'

const rect = (box, kind = 0, mark = 0) => ({ box, kind, mark, family: 0, rule: 'nonzero',
  ops: [0, 1, 1, 1, 4], pts: [box[0], box[1], box[2], box[1], box[2], box[3], box[0], box[3]] })
function synthetic(number = 1, ayah = 1) {
  const paths = [rect([80, 5, 100, 20]), rect([40, 5, 65, 20]), rect([10, 6, 30, 20]),
    rect([0, 6, 8, 20], 3), rect([42, 2, 63, 3], 1, 31)]
  return { number, width: 110, height: 40, paths,
    words: paths.slice(0, 3).map((path, index) => ({ index, surah: 1, ayah, word: index + 1,
      lineIndex: 0, ayahIndex: 0, firstPath: index, nPaths: 1, box: path.box, text: String(index + 1) })),
    lines: [{ index: 0, firstWord: 0, nWords: 3, box: [10, 5, 100, 20] }],
    ayahs: [{ surah: 1, ayah, firstWord: 0, nWords: 3, fragment: 1, fragments: 1, ayahMarkDecoration: 0 }],
    decorations: [
      { index: 0, decoration: 0, surah: 1, ayah, lineIndex: -1, firstPath: 3, nPaths: 1, box: paths[3].box },
      { index: 1, decoration: 4, surah: 1, ayah, lineIndex: 0, firstPath: 4, nPaths: 1, box: paths[4].box }
    ] }
}
function bounded(layout) {
  for (const { box } of [...layout.words, ...layout.rows, ...layout.ayahs]) {
    assert.ok(box.every(Number.isFinite))
    assert.ok(box[0] >= -0.001 && box[1] >= -0.001 && box[2] <= layout.width + 0.001 && box[3] <= layout.height + 0.001,
      `Outside layout ${layout.width}x${layout.height}: ${box}`)
  }
}
const pages = [synthetic(), synthetic(2, 2)]
const passage = new QvpPassage(pages, { surah: 1, from: 1, to: 2 })
const original = JSON.stringify(pages)
const wide = passage.layout({ width: 240 })
const narrow = passage.layout({ width: 70 })
const tiny = passage.layout({ width: 15, scale: 2 })
assert.equal(wide.words.length, 6)
assert.equal(wide.ayahs.length, 2)
assert.ok(narrow.rows.length > wide.rows.length)
assert.ok(tiny.scale < 2)
assert.deepEqual(narrow.words.map(word => [word.ayah, word.word]), [[1, 1], [1, 2], [1, 3], [2, 1], [2, 2], [2, 3]])
for (const layout of [wide, narrow, tiny, passage.layout({ width: 70, align: 'center' })]) bounded(layout)
assert.deepEqual(passage.layout({ width: 70 }), narrow)
assert.equal(JSON.stringify(pages), original)
assert.throws(() => new QvpPassage([pages[0]], { surah: 1, from: 1, to: 2 }), /Incomplete/)
assert.throws(() => new QvpPassage([pages[0], pages[0]], { surah: 1, from: 1 }), /Duplicate/)
for (const change of [{ fragment: 2 }, { nWords: 4 }, { fragments: 2 }]) {
  const incomplete = synthetic()
  Object.assign(incomplete.ayahs[0], change)
  assert.throws(() => new QvpPassage([incomplete], { surah: 1, from: 1 }), /Incomplete/)
}
for (const options of [{ width: 0 }, { width: NaN }, { width: 20, padding: 10 }, { width: 70, scale: -1 },
  { width: 70, lineSpacing: 0.5 }, { width: 70, align: 'guess' }]) assert.throws(() => passage.layout(options), RangeError)

// Canvas receives the original outline operations, including every selected mark.
const fills = []
globalThis.Path2D = class {
  operations = []
  moveTo(...pts) { this.operations.push([0, ...pts]) }
  lineTo(...pts) { this.operations.push([1, ...pts]) }
  quadraticCurveTo(...pts) { this.operations.push([2, ...pts]) }
  bezierCurveTo(...pts) { this.operations.push([3, ...pts]) }
  closePath() { this.operations.push([4]) }
}
let transform
const ctx = { save() {}, restore() {}, setTransform(...values) { transform = values },
  fill(path, rule) { fills.push({ path, rule, transform }) } }
passage.draw(ctx, narrow, { pixelRatio: 2 })
assert.equal(fills.length, 10)
assert.ok(fills.every(fill => fill.rule === 'nonzero' && fill.transform.every(Number.isFinite)))
for (const page of pages) for (const path of page.paths) {
  const first = path.pts.slice(0, 2)
  assert.ok(fills.some(fill => fill.path.operations[0][1] === first[0] && fill.path.operations[0][2] === first[1]))
}
assert.throws(() => passage.draw(ctx, {}), /another passage/)
assert.throws(() => passage.draw(ctx, narrow, { pixelRatio: 0 }), RangeError)

// Individually valid outline operations must not amplify into unbounded work/maps.
for (const unbounded of [true, false]) {
  const oversized = synthetic()
  const pts = [0, 0]
  for (let i = 1; i <= 4000; i++) pts.push(0, unbounded ? i * 63 : (i % 2) * 200)
  oversized.paths[0] = { ...oversized.paths[0], ops: [0, ...Array(4000).fill(1)], pts }
  assert.throws(() => new QvpPassage([oversized], { surah: 1, from: 1 }), /measurement limit/)
}

const fixture_hex = (await readFile(new URL('../conformance/qvp1-lite.hex', import.meta.url), 'utf8')).trim()
const fixture = decodeGeometry(Buffer.from(fixture_hex, 'hex'))
const fixture_passage = new QvpPassage([fixture], { surah: 2, from: 3 })
assert.equal(fixture_passage.ayahs[0].text, 'one two')
bounded(fixture_passage.layout({ width: 100 }))
console.log('ok  lite passage synthetic and codec-fixture tests')

const data_root = new URL('../dist/pages/', import.meta.url)
const has_data = await access(new URL('042.qvp', data_root)).then(() => true, () => false)
if (!has_data) {
  if (process.env.QVP_REQUIRE_DATA) throw new Error('Run scripts/sync-test-data.sh for passage data tests')
  console.log('skip  real passage tests: run scripts/sync-test-data.sh')
} else {
  const metrics = JSON.parse(await readFile(new URL('../conformance/lite-passage-metrics.json', import.meta.url), 'utf8'))
  const durations = []
  for (let number = 1; number <= 604; number++) {
    const page = decodeGeometry(await readFile(new URL(`${String(number).padStart(3, '0')}.qvp`, data_root)))
    const started = performance.now()
    const measured = preparePage(page)
    const golden = metrics.find(item => item.page === number)
    if (golden) {
      assert.ok(Math.abs(measured.pitch - golden.lineSpacing) < 0.02, `Pitch on ${number}`)
      for (const { a, b, air } of golden.pairs) {
        const got = clearance(measured.atoms[a].wordSlices, measured.atoms[b].wordSlices)
        assert.ok(Math.abs(got - air) < 0.75, `Ink clearance on ${number}: ${a},${b}: ${got} vs ${air}`)
      }
    }
    const surahs = [...new Set(page.words.map(word => word.surah))]
    for (const surah of surahs) {
      const ayahs = page.ayahs.filter(ayah => ayah.surah === surah)
      const excerpt = new QvpPassage([page], { surah, from: ayahs[0].ayah, to: ayahs[ayahs.length - 1].ayah })
      const selected = page.words.filter(word => word.surah === surah)
      for (const width of [200, 320, 480]) {
        const layout = excerpt.layout({ width, scale: 0.9 })
        bounded(layout)
        assert.equal(layout.words.length, selected.length)
        for (const word of selected) assert.ok(layout.words.some(w => w.ayah === word.ayah && w.word === word.word))
        // The final word's bounds include its medallion. No ayah loses a number.
        for (const ayah of excerpt.ayahs) {
          const marker = page.decorations.find(d => d.decoration === 0 && d.surah === surah && d.ayah === ayah.ayah)
          assert.ok(marker, `Missing marker ${surah}:${ayah.ayah}`)
          const last = measured.atoms.filter(atom => atom.word.surah === surah && atom.word.ayah === ayah.ayah).pop()
          assert.ok(last.decorations.some(d => d.decoration === marker.index))
        }
      }
    }
    durations.push(performance.now() - started)
  }
  durations.sort((a, b) => a - b)
  const cross_pages = await Promise.all([2, 3].map(n => readFile(new URL(`${String(n).padStart(3, '0')}.qvp`, data_root)).then(decodeGeometry)))
  const across = new QvpPassage(cross_pages.reverse(), { surah: 2, from: 5, to: 6 })
  const cross_layout = across.layout({ width: 366, scale: 0.9 })
  assert.equal(cross_layout.words.length, 19)
  bounded(cross_layout)
  console.log(`ok  604 pages at three widths; prepare + layouts median ${durations[302].toFixed(1)}ms, p95 ${durations[573].toFixed(1)}ms`)
}
