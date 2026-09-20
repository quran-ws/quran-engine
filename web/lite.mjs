/**
 * Dependency-free QVP1 decoder and Canvas2D renderer.
 *
 * This entry point is for displaying original QVP pages with optional word
 * bands. Use the main package when an application also needs the engine's
 * layout, exact hit-testing, search, styling, selection, or masking APIs.
 *
 * @example
 * import { loadPage } from '@quran.ws/engine/lite'
 * const page = await loadPage('/pages/042.qvp')
 * page.draw(canvas.getContext('2d'), page.fit(canvas, 24))
 */

import { buildPath } from './lite-path.mjs'

const limits = {
  fileBytes: 2 * 1024 * 1024,
  paths: 20000,
  operations: 500000,
  coordinates: 2000000,
  decodedBytes: 16 * 1024 * 1024,
  records: 4096
}

export async function loadPage(url, options) {
  const response = await fetch(url, options)
  if (!response.ok) throw new Error(`Could not load QVP page: HTTP ${response.status}`)
  return decodePage(await response.arrayBuffer())
}

export function decodePage(buffer) {
  return new QvpLitePage(decodeGeometry(buffer))
}

export function decodeGeometry(buffer) {
  const bytes = new Uint8Array(buffer.buffer ?? buffer, buffer.byteOffset ?? 0, buffer.byteLength)
  if (bytes.byteLength < 64) throw new Error('Truncated QVP header')
  if (bytes.byteLength > limits.fileBytes) throw new Error('QVP page is too large')
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const u16 = offset => data.getUint16(offset, true)
  const u32 = offset => data.getUint32(offset, true)
  const f32 = offset => data.getFloat32(offset, true)
  if (u32(0) !== 0x31505651 || u16(4) !== 1) throw new Error('Expected a QVP1 page')

  const quant = u16(6)
  const number = u16(8)
  const width = f32(12)
  const height = f32(16)
  const nLines = u16(20)
  const nAyahs = u16(22)
  const nWords = u16(24)
  const nDecorations = u16(26)
  const nPaths = u32(28)
  const nStrings = u16(32)
  const nGlyphs = u16(34)
  const nInstances = u16(36)
  if (nPaths > limits.paths || nLines > limits.records || nAyahs > limits.records ||
      nWords > limits.records || nDecorations > limits.records || nStrings > limits.records ||
      nGlyphs > limits.records || nInstances > limits.records) {
    throw new Error('QVP page exceeds decoder limits')
  }
  if (!quant || !Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    throw new Error('Invalid QVP dimensions')
  }

  let sectionOffset = 64
  const sections = Array.from({ length: 6 }, (_, i) => {
    const start = sectionOffset
    sectionOffset += u32(40 + i * 4)
    if (sectionOffset > bytes.length) throw new Error('Invalid QVP section length')
    return { start, end: sectionOffset }
  })
  if (sectionOffset !== bytes.length) throw new Error('Invalid QVP length')

  const reader = section => ({
    position: section.start,
    byte() {
      if (this.position >= section.end) throw new Error('Truncated QVP section')
      return bytes[this.position++]
    },
    varint() {
      let value = 0
      for (let shift = 0; shift < 35; shift += 7) {
        const byte = this.byte()
        if (shift === 28 && byte > 15) throw new Error('Invalid QVP varint')
        value += (byte & 127) * 2 ** shift
        if (byte < 128) return value
      }
      throw new Error('Invalid QVP varint')
    },
    zigzag() {
      const value = this.varint()
      return (value >>> 1) ^ -(value & 1)
    },
    float() {
      if (this.position + 4 > section.end) throw new Error('Truncated QVP float')
      const value = f32(this.position)
      this.position += 4
      return value
    }
  })

  const metadataSize = sections[0].end - sections[0].start
  const minimumMetadataSize = nLines * 2 + nAyahs * 8 + nWords * 12 +
    nPaths * 5 + nDecorations * 7 + nGlyphs * 5 + nInstances * 26
  if (minimumMetadataSize > metadataSize) throw new Error('Invalid QVP metadata counts')

  const metadata = reader(sections[0])
  let firstWord = 0
  const lines = Array.from({ length: nLines }, (_, index) => {
    const lineNumber = metadata.byte()
    const nWords = metadata.varint()
    const line = { index, lineNumber, firstWord, nWords }
    firstWord += nWords
    return line
  })
  if (firstWord !== nWords) throw new Error('Invalid QVP line word count')
  firstWord = 0
  let ayahSurah = 0
  let ayahNumber = 0
  const ayahs = Array.from({ length: nAyahs }, (_, index) => {
    ayahSurah += metadata.zigzag()
    ayahNumber += metadata.zigzag()
    const fragment = metadata.byte()
    const fragments = metadata.byte()
    const flags = metadata.byte()
    const nWords = metadata.varint()
    const ayahMarkDecoration = metadata.varint() - 1
    const rubuAlHizb = metadata.varint()
    const ayah = { index, surah: ayahSurah, ayah: ayahNumber, fragment, fragments,
      flags, firstWord, nWords, ayahMarkDecoration, rubuAlHizb }
    firstWord += nWords
    return ayah
  })
  if (firstWord !== nWords) throw new Error('Invalid QVP ayah word count')
  let surah = 0
  let ayah = 0
  let word = 0
  let lineIndex = 0
  let ayahIndex = 0
  const wordRecords = Array.from({ length: nWords }, (_, index) => {
    const firstPathDelta = metadata.zigzag()
    surah += metadata.zigzag()
    ayah += metadata.zigzag()
    word += metadata.zigzag()
    lineIndex += metadata.zigzag()
    ayahIndex += metadata.zigzag()
    const textIndex = metadata.varint() - 1
    for (let i = 0; i < 4; i++) metadata.varint()
    return {
      textIndex,
      index,
      surah,
      ayah,
      word,
      lineIndex,
      ayahIndex,
      firstPathDelta,
      nPaths: metadata.varint()
    }
  })
  const columns = Array.from({ length: 4 }, () =>
    Uint8Array.from({ length: nPaths }, () => metadata.byte()))
  const counts = Uint32Array.from({ length: nPaths }, () => metadata.varint())
  const nInstancePaths = columns[3].reduce((sum, flags) => sum + Boolean(flags & 16), 0)
  const instanceBoxes = Array.from({ length: nInstancePaths }, () => {
    const x0 = metadata.zigzag()
    const y0 = metadata.zigzag()
    const x1 = x0 + metadata.zigzag()
    const y1 = y0 + metadata.zigzag()
    return [Math.fround(x0 / quant), Math.fround(y0 / quant), Math.fround(x1 / quant), Math.fround(y1 / quant)]
  })
  let decorationEnd = 0
  let decorationSurah = 0
  const decorationRecords = Array.from({ length: nDecorations }, (_, index) => {
    const firstPath = decorationEnd + metadata.zigzag()
    const decoration = metadata.byte()
    decorationSurah += metadata.zigzag()
    const ayah = metadata.varint()
    const textIndex = metadata.varint() - 1
    const nPaths = metadata.varint()
    const lineIndex = metadata.varint() - 1
    decorationEnd = firstPath + nPaths
    return { index, firstPath, nPaths, decoration, surah: decorationSurah, ayah, textIndex, lineIndex }
  })
  const glyphCounts = Array.from({ length: nGlyphs }, () => {
    const count = metadata.varint()
    for (let i = 0; i < 4; i++) metadata.zigzag()
    return count
  })
  const instances = Array.from({ length: nInstances }, () => {
    const instance = [metadata.byte() | metadata.byte() << 8,
      ...Array.from({ length: 6 }, () => metadata.float())]
    if (instance.some(value => !Number.isFinite(value))) throw new Error('Invalid QVP transform')
    return instance
  })
  if (metadata.position !== sections[0].end) throw new Error('Invalid QVP metadata')

  const wireCounts = [...counts.filter((_, i) => !(columns[3][i] & 16)), ...glyphCounts]
  const wireOperations = wireCounts.reduce((sum, count) => sum + count, 0)
  if (!Number.isSafeInteger(wireOperations) || wireOperations > limits.operations ||
      Math.ceil(wireOperations / 4) !== sections[1].end - sections[1].start) {
    throw new Error('Invalid QVP operation count')
  }

  const closeReader = reader(sections[2])
  let closePosition = 0
  const nCloses = closeReader.varint()
  if (nCloses > sections[2].end - closeReader.position ||
      wireOperations + nCloses > limits.operations) throw new Error('Invalid QVP close count')
  const closePositions = Array.from({ length: nCloses }, () =>
    closePosition += closeReader.varint())
  if (closeReader.position !== sections[2].end) throw new Error('Invalid QVP close paths')

  const xReader = reader(sections[3])
  const yReader = reader(sections[4])
  let previousX = 0
  let previousY = 0
  let operationIndex = 0
  let closeIndex = 0
  const shapes = wireCounts.map(count => {
    const ops = []
    const pts = []
    for (let i = 0; i < count; i++) {
      const operation = bytes[sections[1].start + (operationIndex >> 2)] >> ((operationIndex % 4) * 2) & 3
      ops.push(operation)
      for (let p = 0; p < [1, 1, 2, 3][operation]; p++) {
        previousX += xReader.zigzag()
        previousY += yReader.zigzag()
        pts.push(previousX, previousY)
      }
      operationIndex++
      while (closePositions[closeIndex] === operationIndex) {
        ops.push(4)
        closeIndex++
      }
    }
    if (ops[0] === 0) {
      previousX = pts[0]
      previousY = pts[1]
    }
    return { ops, pts }
  })
  if (Math.ceil(operationIndex / 4) !== sections[1].end - sections[1].start ||
      closeIndex !== closePositions.length ||
      xReader.position !== sections[3].end || yReader.position !== sections[4].end) {
    throw new Error('Invalid QVP geometry')
  }

  const strings = reader(sections[5])
  const decoder = new TextDecoder('utf-8', { fatal: true })
  const texts = Array.from({ length: nStrings }, () => {
    const length = strings.varint()
    const start = strings.position
    strings.position += length
    if (strings.position > sections[5].end) throw new Error('Truncated QVP string')
    return decoder.decode(bytes.subarray(start, strings.position))
  })
  if (strings.position !== sections[5].end) throw new Error('Invalid QVP strings')

  let directShape = 0
  let instanceBox = 0
  let decodedSize = nPaths * 32
  let decodedOperations = 0
  let decodedCoordinates = 0
  const paths = Array.from(counts, (reference, i) => {
    const isInstance = Boolean(columns[3][i] & 16)
    const transform = isInstance ? instances[reference] : null
    if (isInstance && (!transform || transform[0] >= nGlyphs)) throw new Error('Invalid QVP glyph instance')
    const shape = isInstance ? shapes[shapes.length - nGlyphs + transform[0]] : shapes[directShape++]
    if (!shape) throw new Error('Invalid QVP path shape')
    decodedOperations += shape.ops.length
    decodedCoordinates += shape.pts.length
    decodedSize += shape.ops.length + shape.pts.length * 4
    if (!Number.isSafeInteger(decodedSize) || decodedOperations > limits.operations ||
        decodedCoordinates > limits.coordinates || decodedSize > limits.decodedBytes ||
        decodedSize > bytes.length * 64) {
      throw new Error('QVP decoded geometry is too large')
    }

    const pts = new Float32Array(shape.pts.length)
    const box = isInstance ? instanceBoxes[instanceBox++] : [Infinity, Infinity, -Infinity, -Infinity]
    for (let p = 0; p < pts.length; p += 2) {
      const x = shape.pts[p] / quant
      const y = shape.pts[p + 1] / quant
      let outputX = x
      let outputY = y
      if (isInstance) {
        const [, a, b, c, d, e, f] = transform
        outputX = a * x + c * y + e
        outputY = b * x + d * y + f
      }
      if (!Number.isFinite(outputX) || !Number.isFinite(outputY)) throw new Error('Invalid QVP coordinate')
      pts[p] = outputX
      pts[p + 1] = outputY
      if (!isInstance) {
        box[0] = Math.min(box[0], pts[p])
        box[1] = Math.min(box[1], pts[p + 1])
        box[2] = Math.max(box[2], pts[p])
        box[3] = Math.max(box[3], pts[p + 1])
      }
    }
    if (!box || box.some(value => !Number.isFinite(value))) throw new Error('Invalid QVP path box')
    return {
      ops: Uint8Array.from(shape.ops),
      pts,
      box,
      kind: columns[0][i],
      mark: columns[1][i],
      family: columns[2][i],
      rule: columns[3][i] & 1 ? 'evenodd' : 'nonzero'
    }
  })

  let previousWordEnd = 0
  const words = wordRecords.map(record => {
    const { firstPathDelta, nPaths: wordPathCount } = record
    const firstPath = previousWordEnd + firstPathDelta
    const endPath = firstPath + wordPathCount
    if (firstPath < 0 || endPath > nPaths) throw new Error('Invalid QVP word path range')
    const box = [Infinity, Infinity, -Infinity, -Infinity]
    for (let path = firstPath; path < endPath; path++) {
      const pathBox = paths[path].box
      box[0] = Math.min(box[0], pathBox[0])
      box[1] = Math.min(box[1], pathBox[1])
      box[2] = Math.max(box[2], pathBox[2])
      box[3] = Math.max(box[3], pathBox[3])
    }
    previousWordEnd = endPath
    if (box.some(value => !Number.isFinite(value))) throw new Error('Invalid QVP word box')
    if (record.lineIndex < 0 || record.lineIndex >= lines.length ||
        record.ayahIndex < 0 || record.ayahIndex >= ayahs.length ||
        record.textIndex < -1 || record.textIndex >= texts.length) {
      throw new Error('Invalid QVP word metadata')
    }
    const line = lines[record.lineIndex]
    const owner = ayahs[record.ayahIndex]
    if (record.index < line.firstWord || record.index >= line.firstWord + line.nWords ||
        record.index < owner.firstWord || record.index >= owner.firstWord + owner.nWords ||
        record.surah !== owner.surah || record.ayah !== owner.ayah) {
      throw new Error('Invalid QVP word ownership')
    }
    return {
      text: texts[record.textIndex] ?? '',
      index: record.index,
      surah: record.surah,
      ayah: record.ayah,
      word: record.word,
      lineIndex: record.lineIndex,
      ayahIndex: record.ayahIndex,
      firstPath,
      nPaths: wordPathCount,
      box
    }
  })
  const decorations = decorationRecords.map(({ textIndex, ...record }) => {
    const { firstPath, nPaths: count, lineIndex } = record
    if (firstPath < 0 || firstPath + count > paths.length || lineIndex < -1 ||
        lineIndex >= lines.length || textIndex < -1 || textIndex >= texts.length) {
      throw new Error('Invalid QVP decoration metadata')
    }
    const box = bounds(paths.slice(firstPath, firstPath + count))
    return { ...record, text: texts[textIndex] ?? '', box }
  })
  for (const line of lines) line.box = bounds(words.slice(line.firstWord, line.firstWord + line.nWords))
  for (const ayah of ayahs) {
    if (ayah.fragment < 1 || ayah.fragment > ayah.fragments ||
        ayah.ayahMarkDecoration < -1 || ayah.ayahMarkDecoration >= decorations.length) {
      throw new Error('Invalid QVP ayah decoration')
    }
    ayah.box = bounds(words.slice(ayah.firstWord, ayah.firstWord + ayah.nWords))
  }
  return { width, height, number, paths, words, lines, ayahs, decorations }
}

function bounds(records) {
  if (!records.length) return null
  const box = [Infinity, Infinity, -Infinity, -Infinity]
  for (const record of records) {
    box[0] = Math.min(box[0], record.box[0])
    box[1] = Math.min(box[1], record.box[1])
    box[2] = Math.max(box[2], record.box[2])
    box[3] = Math.max(box[3], record.box[3])
  }
  return box
}

export class QvpLitePage {
  #paths
  #decorationPaths

  constructor({ width, height, number, paths, words }) {
    this.width = width
    this.height = height
    this.number = number
    this.words = words
    this.#paths = paths.map(shape => ({ path: buildPath(shape), rule: shape.rule }))
    const wordPaths = new Uint8Array(this.#paths.length)
    for (const { firstPath, nPaths } of words) {
      for (let path = firstPath; path < firstPath + nPaths; path++) {
        if (wordPaths[path]) throw new Error('Overlapping QVP word paths')
        wordPaths[path] = 1
      }
    }
    this.#decorationPaths = Array.from(wordPaths, (owned, path) => owned ? -1 : path)
      .filter(path => path >= 0)
  }

  fit(canvas, padding = 0) {
    const scale = Math.min((canvas.width - 2 * padding) / this.width, (canvas.height - 2 * padding) / this.height)
    return {
      scale,
      x: (canvas.width - this.width * scale) / 2,
      y: (canvas.height - this.height * scale) / 2
    }
  }

  draw(ctx, { scale = 1, x = 0, y = 0, ink = '#231f20', highlight = [], band = '#bedbfa' } = {}) {
    ctx.save()
    ctx.setTransform(scale, 0, 0, scale, x, y)
    ctx.fillStyle = band
    for (const index of highlight) {
      const box = this.words[index]?.box
      if (box) ctx.fillRect(box[0], box[1], box[2] - box[0], box[3] - box[1])
    }
    ctx.fillStyle = ink
    for (const { path, rule } of this.#paths) ctx.fill(path, rule)
    ctx.restore()
  }

  drawWords(ctx, wordIndices, options = {}) {
    const pathIndices = []
    const seen = new Set()
    for (const index of wordIndices) {
      if (!Number.isInteger(index) || index < 0 || index >= this.words.length) {
        throw new RangeError(`Invalid QVP word index: ${index}`)
      }
      if (seen.has(index)) continue
      seen.add(index)
      const { firstPath, nPaths } = this.words[index]
      for (let path = firstPath; path < firstPath + nPaths; path++) pathIndices.push(path)
    }
    this.#drawPaths(ctx, pathIndices, options)
  }

  drawDecorations(ctx, options = {}) {
    this.#drawPaths(ctx, this.#decorationPaths, options)
  }

  #drawPaths(ctx, pathIndices, { scale = 1, x = 0, y = 0, ink = '#231f20' } = {}) {
    ctx.save()
    ctx.setTransform(scale, 0, 0, scale, x, y)
    ctx.fillStyle = ink
    for (const index of pathIndices) {
      const { path, rule } = this.#paths[index]
      ctx.fill(path, rule)
    }
    ctx.restore()
  }

  hitTest(x, y) {
    return this.words.find(({ box: [x0, y0, x1, y1] }) =>
      x >= x0 && x <= x1 && y >= y0 && y <= y1) ?? null
  }
}
