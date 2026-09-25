// Optional data-loader contracts without network access or bundled Quran text.
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { webcrypto } from 'node:crypto'
import { QvpData, hafs } from './data.mjs'
import { surahs } from './metadata.mjs'

const crypto_descriptor = Object.getOwnPropertyDescriptor(globalThis, 'crypto')
Object.defineProperty(globalThis, 'crypto', { configurable: true, value: webcrypto })
const digest = webcrypto.subtle.digest.bind(webcrypto.subtle)
const fixture = Buffer.from((await readFile(new URL('../conformance/qvp1-lite.hex', import.meta.url), 'utf8')).trim(), 'hex')
const text = Array.from({ length: 6236 }, (_, i) => `glyph,test-${i}`).join('\n')
const hashes = new Map([[text, hafs.textDigest], [fixture.toString('hex'), hafs.pages[6][2]]])
// Synthetic content stands in for release bytes; unrecognised content uses real SHA-256.
webcrypto.subtle.digest = async (algorithm, bytes) => {
  const hash = hashes.get(new TextDecoder().decode(bytes)) || hashes.get(Buffer.from(bytes).toString('hex'))
  return hash ? Buffer.from(hash, 'hex') : digest(algorithm, bytes)
}
function storage() {
  const buckets = new Map()
  return {
    buckets,
    async open(name) {
      if (!buckets.has(name)) buckets.set(name, new Map())
      const entries = buckets.get(name)
      return {
        match: async url => entries.get(url)?.clone(),
        put: async (url, response) => { entries.set(url, response.clone()) },
        delete: async url => entries.delete(url),
        keys: async () => [...entries.keys()],
      }
    },
  }
}
try {
  assert.equal(surahs.length, 114)
  assert.equal(surahs.reduce((total, surah) => total + surah.count, 0), 6236)
  assert.equal(hafs.pages.length, 604)
  let previous = 0
  for (const [surah, ayah, hash] of hafs.pages) {
    assert.ok(ayah >= 1 && ayah <= surahs[surah - 1].count)
    assert.ok(surah * 1000 + ayah > previous)
    assert.match(hash, /^[a-f0-9]{64}$/)
    previous = surah * 1000 + ayah
  }
  let calls = 0
  const cache = storage()
  const options = { cacheStorage: cache, fetch: async () => { calls++; return new Response(text) } }
  const data = new QvpData(options)
  assert.equal(calls, 0)
  for (const [surah, ayah, page] of [[1, 1, 1], [2, 1, 2], [2, 6, 3], [2, 255, 42], [2, 282, 48], [16, 50, 272], [114, 6, 604]])
    assert.equal(data.pageOf(surah, ayah), page)
  for (const [s, a] of [[0, 1], [115, 1], [1, 8], [1, 0], [1, 1.5], ['1', 1]])
    assert.throws(() => data.pageOf(s, a), /Invalid Quran reference/)
  assert.throws(() => new QvpData({ pageCacheSize: -1 }), /cache size/)
  const pending = data.loadText()
  assert.equal(data.loadText(), pending)
  const quran = await pending
  assert.equal(quran[1][0], 'test-7')
  assert.equal(quran.at(-1).at(-1), 'test-6235')
  assert.equal(calls, 1)
  assert.deepEqual(await new QvpData({ ...options, fetch: async () => { throw new Error('offline') } }).loadText(), quran)
  const text_cache = await cache.open('qvp-quran-text-v1')
  await text_cache.put(hafs.textUrl, new Response('corrupt'))
  assert.deepEqual(await new QvpData(options).loadText(), quran)
  assert.equal(calls, 2)
  let attempts = 0
  const retry = new QvpData({ cacheStorage: null, fetch: async () => ++attempts === 1 ? new Response('', { status: 503 }) : new Response(text) })
  await assert.rejects(retry.loadText(), /503/)
  assert.equal((await retry.loadText())[0][0], 'test-0')
  await assert.rejects(new QvpData({ cacheStorage: null, fetch: async () => new Response(text + 'tampered') }).loadText(), /digest/)
  assert.equal((await new QvpData({ cacheStorage: { open: async () => { throw new Error('denied') } }, fetch: options.fetch }).loadText())[0][0], 'test-0')

  let page_calls = 0
  const page_cache = storage()
  const pages = new QvpData({ cacheStorage: page_cache, responseCacheSize: 1, fetch: async () => { page_calls++; return new Response(fixture) } })
  const response_cache = await page_cache.open('qvp-qvp-v0.3.0')
  await response_cache.put('old', new Response('old'))
  const page_request = pages.loadPage(7)
  assert.equal(pages.loadPage(7), page_request)
  assert.equal((await page_request).number, 7)
  assert.equal(page_calls, 1)
  assert.equal((await response_cache.keys()).length, 1)
  assert.equal((await new QvpData({ cacheStorage: page_cache, fetch: async () => { throw new Error('offline') } }).loadPage(7)).number, 7)
  await assert.rejects(pages.loadPage(0), /Invalid Quran page/)
  await assert.rejects(pages.loadPassage({ surah: 1, from: 7, to: 1 }), /range/)
  const uncached = new QvpData({ pageCacheSize: 0, responseCacheSize: 0, cacheStorage: page_cache, fetch: async () => { page_calls++; return new Response(fixture) } })
  await uncached.loadPage(7)
  await uncached.loadPage(7)
  assert.equal(page_calls, 3)
  // A correct digest does not excuse a decoded page with the wrong page number.
  hashes.set(fixture.toString('hex'), hafs.pages[7][2])
  await assert.rejects(pages.loadPage(8), /Unexpected Quran page/)
  assert.equal((await response_cache.keys()).length, 1)
  console.log('ok  optional data: metadata, validation, shared loads, retry, integrity, offline and cache limits')
} finally {
  webcrypto.subtle.digest = digest
  if (crypto_descriptor) Object.defineProperty(globalThis, 'crypto', crypto_descriptor)
  else delete globalThis.crypto
}
