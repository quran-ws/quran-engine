import assert from 'node:assert/strict'
import { readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { decodeGeometry } from '../web/lite.mjs'

const referenceDir = process.argv[2]
if (!referenceDir) {
  throw new Error('Usage: node web-benchmark/check-qvp.mjs <directory of all 604 native QVB1 exports> [result.json]')
}

const root = fileURLToPath(new URL('../', import.meta.url))
const pagesDir = resolve(process.env.QVP_PAGES_DIR ?? resolve(root, 'dist/pages'))
const result = { pages: 0, paths: 0, commands: 0, coordinate_floats: 0, word_boxes: 0 }

for (let number = 1; number <= 604; number++) {
  const name = String(number).padStart(3, '0')
  const page = decodeGeometry(await readFile(resolve(pagesDir, `${name}.qvp`)))
  const reference = await readFile(resolve(referenceDir, `${name}.bin`))
  const nPaths = reference.readUInt32LE(12)
  const nOperations = reference.readUInt32LE(16)
  const nCoordinates = reference.readUInt32LE(20)
  const nWords = reference.readUInt32LE(24)

  assert.equal(page.number, number)
  assert.equal(page.width, reference.readFloatLE(4))
  assert.equal(page.height, reference.readFloatLE(8))
  assert.equal(page.paths.length, nPaths)
  assert.equal(page.words.length, nWords)

  const operationOffset = 32 + nPaths * 32
  const coordinateOffset = operationOffset + Math.ceil(nOperations / 4) * 4
  let decodedOperations = 0
  let decodedCoordinates = 0
  for (let pathIndex = 0; pathIndex < nPaths; pathIndex++) {
    const path = page.paths[pathIndex]
    const tableOffset = 32 + pathIndex * 32
    const operationStart = reference.readUInt32LE(tableOffset)
    const operationCount = reference.readUInt32LE(tableOffset + 4)
    const coordinateStart = reference.readUInt32LE(tableOffset + 8)
    const coordinateCount = reference.readUInt32LE(tableOffset + 12)
    const flags = reference.readUInt32LE(tableOffset + 16)

    assert.equal(decodedOperations, operationStart, `page ${number} path ${pathIndex} operation start`)
    assert.equal(path.ops.length, operationCount, `page ${number} path ${pathIndex} operation count`)
    assert.deepEqual(
      Buffer.from(path.ops),
      reference.subarray(operationOffset + operationStart, operationOffset + operationStart + operationCount),
      `page ${number} path ${pathIndex} operations`
    )
    assert.equal(decodedCoordinates, coordinateStart, `page ${number} path ${pathIndex} coordinate start`)
    assert.equal(path.pts.length, coordinateCount, `page ${number} path ${pathIndex} coordinate count`)
    assert.deepEqual(
      Buffer.from(path.pts.buffer, path.pts.byteOffset, path.pts.byteLength),
      reference.subarray(coordinateOffset + coordinateStart * 4, coordinateOffset + (coordinateStart + coordinateCount) * 4),
      `page ${number} path ${pathIndex} coordinates`
    )
    assert.equal(path.rule, flags & 0x1000000 ? 'evenodd' : 'nonzero', `page ${number} path ${pathIndex} fill rule`)
    decodedOperations += operationCount
    decodedCoordinates += coordinateCount
  }
  assert.equal(decodedOperations, nOperations)
  assert.equal(decodedCoordinates, nCoordinates)

  const wordOffset = coordinateOffset + nCoordinates * 4
  for (let wordIndex = 0; wordIndex < nWords; wordIndex++) {
    for (let coordinate = 0; coordinate < 4; coordinate++) {
      assert.equal(
        page.words[wordIndex].box[coordinate],
        reference.readFloatLE(wordOffset + wordIndex * 16 + coordinate * 4),
        `page ${number} word ${wordIndex} box coordinate ${coordinate}`
      )
    }
  }

  result.pages++
  result.paths += nPaths
  result.commands += nOperations
  result.coordinate_floats += nCoordinates
  result.word_boxes += nWords
}

console.log(JSON.stringify(result, null, 2))
if (process.argv[3]) await writeFile(process.argv[3], `${JSON.stringify(result, null, 2)}\n`)
