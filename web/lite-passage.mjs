// Lay out complete ayahs as a standalone passage without loading Wasm.
import { buildPath } from './lite-path.mjs'
import { bounds, clearance, median, preparePage, resizedSlices } from './lite-passage-geometry.mjs'

const canvas_paths = new WeakMap()

/** Lay out and draw a contiguous ayah range from decoded QVP pages. */
export class QvpPassage {
  #atoms
  #pages
  #gaps
  #open
  #lineSpacing
  #layouts = new WeakMap()

  /** Select complete ayahs; pages come from decodeGeometry(), in any order. */
  constructor(pages, { surah, from, to = from }) {
    if (!Number.isInteger(surah) || surah < 1 || surah > 114 ||
        !Number.isInteger(from) || from < 1 || !Number.isInteger(to) || to < from || to > 286) {
      throw new RangeError('Invalid passage ayah range')
    }
    if (new Set(pages.map(page => page.number)).size !== pages.length) throw new Error('Duplicate QVP pages')
    this.#pages = pages
    this.#atoms = []
    this.ayahs = []
    for (let ayah = from; ayah <= to; ayah++) {
      const words = pages.flatMap(page => page.words.filter(word => word.surah === surah && word.ayah === ayah)
        .map(word => ({ page, word }))).sort((a, b) => a.word.word - b.word.word)
      const fragments = pages.flatMap(page => page.ayahs.filter(record => record.surah === surah && record.ayah === ayah))
      fragments.sort((a, b) => a.fragment - b.fragment)
      if (!words.length || !fragments.length || fragments.length !== fragments[0].fragments ||
          fragments.some((record, i) => record.fragment !== i + 1 || record.fragments !== fragments.length) ||
          words.length !== fragments.reduce((sum, record) => sum + record.nWords, 0) ||
          words.some(({ word }, i) => word.word !== i + 1)) {
        throw new Error(`Incomplete passage ayah ${surah}:${ayah}`)
      }
      if (this.#atoms.length + words.length > 4096) throw new RangeError('A passage may contain at most 4096 words')
      for (const { page, word } of words) this.#atoms.push(preparePage(page).atoms[word.index])
      this.ayahs.push({ surah, ayah, text: words.map(({ word }) => word.text).join(' ') })
    }
    // Pages 1 and 2 print their words smaller than the other pages. A word of a page that prints
    // at another size than the passage's first word is resized to that word's size, so that
    // every word of the passage is drawn at one size.
    this.#lineSpacing = print_sizes(this.#atoms)
    // Two words drawn as one piece of calligraphy keep their printed distance, and a justified
    // row never opens it.
    const spacing = this.#atoms.map((atom, index) => {
      if (!index) return [0, false]
      const previous = this.#atoms[index - 1]
      const natural = previous.box[0] - atom.box[2]
      const word_air = clearance(previous.wordSlices, atom.wordSlices)
      const joined = previous.page === atom.page && previous.word.lineIndex === atom.word.lineIndex &&
        previous.word.index + 1 === atom.word.index && word_air !== null && -word_air >= atom.pitch * 0.15
      if (joined) return [natural, false]
      const air = clearance(previous.slices, atom.slices)
      const wanted = (previous.gap + atom.gap) / 2
      return [air === null ? wanted : natural + wanted - air, true]
    })
    this.#gaps = spacing.map(([gap]) => gap)
    this.#open = spacing.map(([, open]) => open)
  }

  /**
   * Return rows and word/ayah bounds in CSS pixels, at the requested width and scale. A
   * justified row opens its gaps until it reaches both edges, never past maxStretch times the
   * usual air between words (0 or less lifts the cap); what is left over is split between its
   * sides, as for a centred row.
   */
  layout({ width, scale = 1, lineSpacing = 1, padding = 2, align = 'right', maxStretch = 2 }) {
    if (!Number.isFinite(width) || width <= 0 || width > 32768 ||
        !Number.isFinite(scale) || scale <= 0 || scale > 32 ||
        !Number.isFinite(padding) || padding < 0 || 2 * padding >= width ||
        !Number.isFinite(lineSpacing) || lineSpacing < 1 || lineSpacing > 4 ||
        !['right', 'center', 'justified'].includes(align) ||
        !Number.isFinite(maxStretch)) throw new RangeError('Invalid passage layout')
    const atoms = this.#atoms
    const widest = Math.max(...atoms.map(atom => atom.box[2] - atom.box[0]))
    // An indivisible word and its signs must fit even at an unusually narrow width.
    scale = Math.min(scale, (width - 2 * padding) / widest)
    const row_width = (width - 2 * padding) / scale
    const rows = break_rows(atoms, this.#gaps, row_width)
    const drawings = []
    const placements = new Map()
    const result = { width, height: 0, scale, rows: [], words: [], ayahs: [] }
    const pitch = this.#lineSpacing * lineSpacing
    // A justified gap opens by the air between words, never by the distance between boxes.
    const air = median(atoms.map(atom => atom.gap))
    let last_bottom = 0
    let last_baseline = 0
    for (const indices of rows) {
      let used = indices.reduce((sum, index, i) => sum + atoms[index].box[2] - atoms[index].box[0] + (i ? this.#gaps[index] : 0), 0)
      const ascent = Math.max(...indices.map(index => atoms[index].baseline - atoms[index].box[1]))
      const descent = Math.max(...indices.map(index => atoms[index].box[3] - atoms[index].baseline))
      // Signs and tall marks have the same right to vertical space as the letters.
      const baseline = result.rows.length ? Math.max(last_baseline + pitch, last_bottom + ascent + 1) : ascent
      // The last row is short because the text ran out, and keeps its gaps.
      let share = 0
      if (align === 'justified' && indices !== rows[rows.length - 1]) {
        const opening = indices.filter((index, i) => i && this.#open[index]).length
        const slack = row_width - used
        if (opening > 0 && slack > 0) {
          const cap = maxStretch > 0 ? opening * air * (maxStretch - 1) : Infinity
          const widen = Math.min(slack, Math.max(cap, 0))
          share = widen / opening
          used += widen
        }
      }
      let cursor = align === 'right' ? row_width : (row_width + used) / 2
      const row = result.rows.length
      const row_bounds = []
      for (let i = 0; i < indices.length; i++) {
        const index = indices[i]
        const atom = atoms[index]
        if (i) cursor -= this.#gaps[index] + (this.#open[index] ? share : 0)
        const dx = cursor - atom.box[2]
        const dy = baseline - atom.baseline
        const size = atom.size
        placements.set(atom.source, { dx, dy, row, size })
        for (const path of atom.indices) drawings.push({ page: atom.page, path, dx, dy, kx: size, ky: size, row })
        const box = [atom.box[0] + dx, atom.box[1] + dy, atom.box[2] + dx, atom.box[3] + dy]
        // The word alone, without its signs: the word that closes an ayah leaves its medallion out.
        const wordBox = [atom.word.box[0] * size + dx, atom.word.box[1] * size + dy,
          atom.word.box[2] * size + dx, atom.word.box[3] * size + dy]
        row_bounds.push(box)
        result.words.push({ surah: atom.word.surah, ayah: atom.word.ayah, word: atom.word.word, row, box, wordBox })
        cursor -= atom.box[2] - atom.box[0]
      }
      result.rows.push({ box: bounds(row_bounds), baseline })
      last_baseline = baseline
      last_bottom = baseline + descent
    }
    // A sajdah line belongs to the words underneath it, not necessarily to the ayah
    // carrying the sign. Its stroke is repeated only when those words span rows. It has the size
    // of its words.
    for (const page of this.#pages) for (const stroke of preparePage(page).strokes) {
      const spans = new Map()
      for (const index of stroke.words) {
        const atom = preparePage(page).atoms[index]
        const placement = placements.get(atom)
        if (!placement) continue
        const { dx, dy, size } = placement
        const box = [atom.word.box[0] * size + dx, atom.word.box[1] * size + dy,
          atom.word.box[2] * size + dx, atom.word.box[3] * size + dy]
        const old = spans.get(placement.row)
        spans.set(placement.row, { box: bounds([old?.box, box]), dy, size })
      }
      for (const [row, { box, dy, size }] of spans) {
        const kx = (box[2] - box[0]) / (stroke.box[2] - stroke.box[0])
        const dx = box[0] - kx * stroke.box[0]
        drawings.push({ page, path: stroke.path, dx, dy, kx, ky: size, row })
        result.rows[row].box = bounds([result.rows[row].box,
          [box[0], stroke.box[1] * size + dy, box[2], stroke.box[3] * size + dy]])
      }
    }
    // Include the strokes in row spacing too. Moving whole rows cannot reshape ink.
    for (let row = 1; row < result.rows.length; row++) {
      const needed = result.rows[row - 1].box[3] + 1 - result.rows[row].box[1]
      if (needed <= 0) continue
      for (let i = row; i < result.rows.length; i++) {
        result.rows[i].box[1] += needed
        result.rows[i].box[3] += needed
        result.rows[i].baseline += needed
      }
      for (const word of result.words) if (word.row >= row) {
        word.box[1] += needed
        word.box[3] += needed
        word.wordBox[1] += needed
        word.wordBox[3] += needed
      }
      for (const drawing of drawings) if (drawing.row >= row) drawing.dy += needed
    }
    const ink = bounds(result.rows.map(row => row.box))
    const offset = padding / scale - ink[1]
    for (const drawing of drawings) {
      drawing.dx += padding / scale
      drawing.dy += offset
    }
    const css_box = box => [box[0] * scale + padding, (box[1] + offset) * scale,
      box[2] * scale + padding, (box[3] + offset) * scale]
    for (const word of result.words) {
      word.box = css_box(word.box)
      word.wordBox = css_box(word.wordBox)
    }
    for (const row of result.rows) {
      row.box = css_box(row.box)
      row.baseline = (row.baseline + offset) * scale
    }
    result.height = (ink[3] - ink[1]) * scale + 2 * padding
    result.ayahs = this.ayahs.map(ayah => ({ ...ayah,
      box: bounds(result.words.filter(word => word.surah === ayah.surah && word.ayah === ayah.ayah).map(word => word.box)) }))
    this.#layouts.set(result, drawings)
    return result
  }

  /** Draw a layout without clearing or sizing the canvas; pixelRatio is its backing ratio. */
  draw(ctx, layout, { pixelRatio = 1, ink = '#231f20' } = {}) {
    const drawings = this.#layouts.get(layout)
    if (!drawings) throw new Error('Layout belongs to another passage')
    if (!Number.isFinite(pixelRatio) || pixelRatio <= 0) throw new RangeError('Invalid pixel ratio')
    const scale = layout.scale * pixelRatio
    ctx.save()
    ctx.fillStyle = ink
    for (const { page, path, dx, dy, kx, ky } of drawings) {
      let paths = canvas_paths.get(page)
      if (!paths) canvas_paths.set(page, paths = new Map())
      if (!paths.has(path)) paths.set(path, buildPath(page.paths[path]))
      ctx.setTransform(scale * kx, 0, 0, scale * ky, dx * scale, dy * scale)
      ctx.fill(paths.get(path), page.paths[path].rule)
    }
    ctx.restore()
  }
}

// Resize every word printed at another size than the passage's first word to that word's size,
// and return the passage's line spacing: the median over the words printed at the first word's
// size. Two pages print at one size when their line spacings are less than 1.15 times apart.
// The first page that prints at a size stands for that size. Each word records its prepared atom
// as its source, and how much it is resized.
function print_sizes(atoms) {
  const sizes = []
  const size_of = atoms.map(atom => {
    let i = sizes.findIndex(({ first }) => Math.max(first, atom.pitch) < 1.15 * Math.min(first, atom.pitch))
    if (i < 0) i = sizes.push({ first: atom.pitch, pitches: [] }) - 1
    sizes[i].pitches.push(atom.pitch)
    return i
  })
  const spacings = sizes.map(({ pitches }) => median(pitches))
  atoms.forEach((atom, i) => {
    atoms[i] = size_of[i] ? resized(atom, spacings[0] / spacings[size_of[i]]) : { ...atom, source: atom, size: 1 }
  })
  return spacings.length ? spacings[0] : 0
}

// A prepared atom resized by k from the page's origin. Its paths stay as printed; their
// placement resizes them.
function resized(atom, k) {
  const scaled = box => box.map(v => v * k)
  return { ...atom, source: atom, size: k, box: scaled(atom.box), baseline: atom.baseline * k,
    pitch: atom.pitch * k, gap: atom.gap * k,
    slices: resizedSlices(atom.slices, k), wordSlices: resizedSlices(atom.wordSlices, k),
    decorations: atom.decorations.map(d => ({ ...d, box: scaled(d.box), slices: resizedSlices(d.slices, k) })) }
}

// Balance a bounded passage without stretching its gaps. The last row may be short.
function break_rows(atoms, gaps, width) {
  const n = atoms.length
  const best = new Float64Array(n + 1).fill(Infinity)
  const previous = new Uint32Array(n + 1)
  best[0] = 0
  for (let end = 1; end <= n; end++) {
    let used = 0
    for (let start = end - 1; start >= 0; start--) {
      used += atoms[start].box[2] - atoms[start].box[0] + (start < end - 1 ? gaps[start + 1] : 0)
      if (used > width + 0.001) break
      const slack = width - used
      const cost = best[start] + (end === n ? 0 : slack * slack) + 1
      if (cost < best[end]) {
        best[end] = cost
        previous[end] = start
      }
    }
  }
  const rows = []
  for (let end = n; end > 0;) {
    const start = previous[end]
    rows.push(Array.from({ length: end - start }, (_, i) => start + i))
    end = start
  }
  return rows.reverse()
}
