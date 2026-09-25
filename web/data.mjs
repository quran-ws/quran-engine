// Optional verified fetching for Hafs passages; the core and lite decoder stay fetch-free.

import { decodeGeometry } from './lite.mjs'
import surahs from './surahs.json' with { type: 'json' }
import release from './data-hafs.json' with { type: 'json' }

/** Gets the Hafs release URLs, integrity hashes and page-start index. */
export const hafs = Object.freeze({ ...release, pages: Object.freeze(release.pages.map(Object.freeze)) })

/** Loads verified Quran text and pages, with bounded page caches and no import-time I/O. */
export class QvpData {
  #pages = new Map()
  #text
  #options

  /** Sets cache limits and optional fetch/CacheStorage adapters; zero disables that page cache. */
  constructor({ pageCacheSize = 6, responseCacheSize = 24, cachePrefix = 'qvp', fetch = globalThis.fetch, cacheStorage = globalThis.caches } = {}) {
    for (const size of [pageCacheSize, responseCacheSize]) {
      if (!Number.isSafeInteger(size) || size < 0) throw new RangeError('Invalid cache size')
    }
    this.#options = { pageCacheSize, responseCacheSize, cachePrefix, fetch, cacheStorage }
  }

  /** Gets the printed page containing a valid surah and ayah. */
  pageOf(surah, ayah) {
    if (!Number.isInteger(surah) || !Number.isInteger(ayah) || !surahs[surah - 1] || ayah < 1 || ayah > surahs[surah - 1].count)
      throw new RangeError('Invalid Quran reference')
    const key = surah * 1000 + ayah
    for (let index = hafs.pages.length - 1; index >= 0; index--) {
      const [s, a] = hafs.pages[index]
      if (s * 1000 + a <= key) return index + 1
    }
  }

  /** Loads a decoded page by its one-based printed number; failures may be retried. */
  loadPage(number) {
    if (!Number.isInteger(number) || number < 1 || number > hafs.pages.length)
      return Promise.reject(new RangeError('Invalid Quran page'))
    if (this.#pages.has(number)) {
      const request = this.#pages.get(number)
      this.#pages.delete(number)
      this.#pages.set(number, request)
      return request
    }
    const url = `${hafs.pageUrl}${String(number).padStart(3, '0')}.qvp`
    const request = this.#load(url, hafs.pages[number - 1][2], `qvp-${hafs.pageVersion}`, this.#options.responseCacheSize, bytes => {
      const page = decodeGeometry(bytes)
      if (page.number !== number) throw new Error('Unexpected Quran page')
      return page
    }).catch(error => {
      if (this.#pages.get(number) === request) this.#pages.delete(number)
      throw error
    })
    this.#pages.set(number, request)
    while (this.#pages.size > this.#options.pageCacheSize) this.#pages.delete(this.#pages.keys().next().value)
    return request
  }

  /** Loads a complete ayah range as a QvpPassage, including pages crossed by that range. */
  async loadPassage({ surah, from, to = from }) {
    const first = this.pageOf(surah, from)
    const last = this.pageOf(surah, to)
    if (to < from) throw new RangeError('Invalid Quran range')
    const { QvpPassage } = await import('./lite-passage.mjs')
    const pages = []
    // Sequential loads keep a long range from starting hundreds of network requests at once.
    for (let number = first; number <= last; number++) pages.push(await this.loadPage(number))
    return new QvpPassage(pages, { surah, from, to })
  }

  /** Loads Unicode ayahs as arrays by surah; both indices are zero-based. */
  loadText() {
    this.#text ||= this.#load(hafs.textUrl, hafs.textDigest, `quran-text-${hafs.textVersion}`, 1, bytes => {
      const rows = new TextDecoder('utf-8', { fatal: true }).decode(bytes).trimEnd().split(/\r?\n/).map(row => row.split(','))
      if (rows.length !== 6236 || rows.some(row => row.length !== 2 || !row[1].trim()))
        throw new Error('Invalid Quran text asset')
      let offset = 0
      return Object.freeze(surahs.map(({ count }) => {
        const ayahs = Object.freeze(rows.slice(offset, offset + count).map(row => row[1]))
        offset += count
        return ayahs
      }))
    }).catch(error => {
      this.#text = null
      throw error
    })
    return this.#text
  }

  async #load(url, digest, bucket, limit, decode) {
    const { fetch, cacheStorage, cachePrefix } = this.#options
    const cache = limit ? await cacheStorage?.open(`${cachePrefix}-${bucket}`).catch(() => null) : null
    const cached = await cache?.match(url).catch(() => null)
    const read = async response => {
      if (!response.ok) throw new Error(`Quran asset: HTTP ${response.status}`)
      const bytes = await response.arrayBuffer()
      const hash = Array.from(new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', bytes)), byte => byte.toString(16).padStart(2, '0')).join('')
      if (hash !== digest) throw new Error('Invalid Quran asset digest')
      return decode(bytes)
    }
    if (cached) {
      try { return await read(cached) }
      catch { await cache.delete(url).catch(() => {}) }
    }
    const response = await fetch(url)
    const result = await read(response.clone())
    if (cache) {
      await cache.put(url, response).catch(() => {})
      const keys = await cache.keys().catch(() => [])
      await Promise.all(keys.slice(0, -limit).map(key => cache.delete(key).catch(() => {})))
    }
    return result
  }
}
