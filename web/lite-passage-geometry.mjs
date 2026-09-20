// Measure passage ink; Canvas still draws the untouched curves, not these samples.
const prepared_pages = new WeakMap()
const band_height = 0.375
const curve_steps = 8

export function bounds(boxes) {
  const result = [Infinity, Infinity, -Infinity, -Infinity]
  for (const box of boxes) {
    if (!box) continue
    result[0] = Math.min(result[0], box[0])
    result[1] = Math.min(result[1], box[1])
    result[2] = Math.max(result[2], box[2])
    result[3] = Math.max(result[3], box[3])
  }
  return Number.isFinite(result[0]) ? result : null
}

export function median(values, fallback = 0) {
  if (!values.length) return fallback
  values.sort((a, b) => a - b)
  return values[Math.floor(values.length / 2)]
}

// A shared band height lets words from different printed pages share one row.
function trace(paths, baseline) {
  const bands = new Map()
  const mark = (x, y) => {
    const row = Math.floor((y - baseline) / band_height)
    const span = bands.get(row)
    if (span) {
      span[0] = Math.min(span[0], x)
      span[1] = Math.max(span[1], x)
    } else bands.set(row, [x, x])
  }
  const segment = (a, b) => {
    const steps = Math.max(1, Math.min(512, Math.ceil(Math.abs(b[1] - a[1]) / band_height)))
    for (let i = 0; i <= steps; i++) {
      const t = i / steps
      mark(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    }
  }
  for (const { ops, pts } of paths) {
    let point = 0
    let here = [0, 0]
    let start = here
    const take = () => [pts[point++], pts[point++]]
    for (const op of ops) {
      if (op === 0) {
        here = take()
        start = here
        mark(...here)
      } else if (op === 1 || op === 4) {
        const to = op === 4 ? start : take()
        segment(here, to)
        here = to
      } else {
        const first = take()
        const second = take()
        const to = op === 2 ? second : take()
        let previous = here
        for (let i = 1; i <= curve_steps; i++) {
          const t = i / curve_steps
          const u = 1 - t
          const next = [0, 1].map(axis => op === 2
            ? u * u * here[axis] + 2 * u * t * first[axis] + t * t * to[axis]
            : u ** 3 * here[axis] + 3 * u * u * t * first[axis] + 3 * u * t * t * second[axis] + t ** 3 * to[axis])
          segment(previous, next)
          previous = next
        }
        here = to
      }
    }
  }
  return [...bands].sort((a, b) => a[0] - b[0]).map(([row, [left, right]]) => [row, left, right])
}

export function clearance(a, b) {
  let air = Infinity
  let i = 0
  let j = 0
  while (i < a.length && j < b.length) {
    if (a[i][0] < b[j][0]) i++
    else if (b[j][0] < a[i][0]) j++
    else {
      air = Math.min(air, a[i][1] - b[j][2])
      i++
      j++
    }
  }
  return Number.isFinite(air) ? air : null
}

function merged_slices(groups) {
  const rows = new Map()
  for (const slices of groups) for (const [row, left, right] of slices) {
    const previous = rows.get(row)
    rows.set(row, previous ? [Math.min(left, previous[0]), Math.max(right, previous[1])] : [left, right])
  }
  return [...rows].sort((a, b) => a[0] - b[0]).map(([row, [left, right]]) => [row, left, right])
}

export function preparePage(page) {
  if (prepared_pages.has(page)) return prepared_pages.get(page)
  const paths_of = record => page.paths.slice(record.firstPath, record.firstPath + record.nPaths)
  const line_words = page.lines.map(line => page.words.slice(line.firstWord, line.firstWord + line.nWords))
  const centres = page.lines.map((line, i) => {
    let box = bounds(line_words[i].flatMap(word => paths_of(word).filter(path => path.kind === 0).map(path => path.box)))
    if (!box) box = bounds(page.decorations.filter(d => d.lineIndex === i && [1, 2].includes(d.decoration)).map(d => d.box))
    return box ? (box[1] + box[3]) / 2 : 0
  })
  const pitch = median(centres.slice(1).map((y, i) => Math.abs(y - centres[i])).filter(gap => gap > 1), page.height / 15)
  const baselines = line_words.map((words, i) => median(words.map(word => word.box[3]), centres[i]))
  const word_slices = page.words.map(word => trace(paths_of(word), baselines[word.lineIndex]))
  const gaps = []
  for (const words of line_words) for (let i = 1; i < words.length; i++) {
    const a = words[i - 1]
    const b = words[i]
    if (a.surah !== b.surah || a.ayah !== b.ayah) continue
    const air = clearance(word_slices[a.index], word_slices[b.index])
    if (air > 0) gaps.push(air)
  }
  const gap = median(gaps, pitch / 4)
  const attached = page.words.map(() => [])
  const strokes = []
  for (const decoration of page.decorations) {
    if (![0, 3, 4].includes(decoration.decoration)) continue
    const path_indices = Array.from({ length: decoration.nPaths }, (_, i) => decoration.firstPath + i)
    for (const index of path_indices.filter(index => page.paths[index].mark === 31)) {
      const box = page.paths[index].box
      const under = page.words.filter(word => word.box[0] < box[2] && word.box[2] > box[0] && word.box[1] >= box[1] - 2)
        .sort((a, b) => a.box[1] - b.box[1])[0]
      if (!under) throw new Error('Sajdah line has no words')
      const words = line_words[under.lineIndex].filter(word => word.box[0] < box[2] && word.box[2] > box[0])
      strokes.push({ path: index, words: words.map(word => word.index), box, baseline: baselines[under.lineIndex] })
    }
    const indices = path_indices.filter(index => page.paths[index].mark !== 31)
    if (!indices.length) continue
    const box = bounds(indices.map(index => page.paths[index].box))
    let word
    if (decoration.decoration === 0 || decoration.decoration === 4) {
      word = page.words.filter(word => word.surah === decoration.surah && word.ayah === decoration.ayah).pop()
    } else {
      const line = decoration.lineIndex >= 0 ? decoration.lineIndex : centres.reduce((best, y, i) =>
        Math.abs(y - (box[1] + box[3]) / 2) < Math.abs(centres[best] - (box[1] + box[3]) / 2) ? i : best, 0)
      const candidates = line_words[line]
      word = candidates.filter(word => word.box[2] <= box[0] + gap).sort((a, b) => b.box[2] - a.box[2])[0]
      word ||= [...candidates].sort((a, b) => Math.min(Math.abs(a.box[0] - box[0]), Math.abs(a.box[2] - box[2])) -
        Math.min(Math.abs(b.box[0] - box[0]), Math.abs(b.box[2] - box[2])))[0]
    }
    if (!word) throw new Error('QVP passage decoration has no word')
    attached[word.index].push({ indices, box, decoration: decoration.index,
      slices: trace(indices.map(index => page.paths[index]), baselines[word.lineIndex]) })
  }
  const atoms = page.words.map(word => {
    const decorations = attached[word.index]
    const indices = Array.from({ length: word.nPaths }, (_, i) => word.firstPath + i)
    for (const decoration of decorations) indices.push(...decoration.indices)
    return { page, word, indices, decorations, pitch, gap,
      baseline: baselines[word.lineIndex],
      box: bounds([word.box, ...decorations.map(d => d.box)]),
      slices: merged_slices([word_slices[word.index], ...decorations.map(d => d.slices)]),
      wordSlices: word_slices[word.index] }
  })
  const result = { atoms, strokes, pitch, gap, baselines }
  prepared_pages.set(page, result)
  return result
}
