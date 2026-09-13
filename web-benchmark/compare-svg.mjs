import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { readFile, readdir, stat, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { brotliCompressSync, constants, gzipSync } from 'node:zlib'

const root = fileURLToPath(new URL('../', import.meta.url))
const muhaffidh_dir = resolve(process.argv[2] ?? '../../muhaffidh')
const url = process.argv[3] ?? 'http://127.0.0.1:4187/'
const output = resolve(process.argv[4] ?? `${root}/web-benchmark/results/comparisons/svg-browser-comparison.json`)
const hardware = process.argv[5] ?? null
const require = createRequire(resolve(muhaffidh_dir, 'package.json'))
const { chromium } = require('@playwright/test')

const browser = await chromium.launch({ headless: true, channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 390, height: 620 } })
await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 120000 })

const measured = await page.evaluate(async () => {
  document.body.replaceChildren()
  const target = document.createElement('div')
  target.style.cssText = 'width:390px;height:620px;position:absolute;inset:0'
  document.body.append(target)

  const module = await import('/packages/digitalkhatt-web/src/index.js')
  const pages = [1, 42, 585]
  const options = page_number => module.create_digitalkhatt_renderer_options({
    target,
    mushaf: 'dk-medina1441-hafs',
    page_number,
    tajweed_enabled: false,
    wasm_url: '/hb.wasm'
  })
  const renderer = new module.DigitalKhattPageRenderer(options(1))
  const wait_until_finished = async () => {
    while (renderer.view?.renderingState !== 3) await new Promise(requestAnimationFrame)
  }
  await renderer.mount()
  await wait_until_finished()

  const generation = Object.fromEntries(pages.map(number => [number, []]))
  for (let round = 0; round < 25; round++) {
    for (let offset = 0; offset < pages.length; offset++) {
      const number = pages[(round + offset) % pages.length]
      const start = performance.now()
      await renderer.update(options(number))
      await wait_until_finished()
      renderer.page_element.getBoundingClientRect()
      const end = performance.now()
      if (round >= 5) generation[number].push(end - start)
    }
  }

  const markups = {}
  const descriptions = {}
  for (const number of pages) {
    await renderer.update(options(number))
    await wait_until_finished()
    const markup = renderer.page_element.innerHTML
    markups[number] = markup
    descriptions[number] = {
      elements: renderer.page_element.querySelectorAll('*').length,
      svg_elements: renderer.page_element.querySelectorAll('svg').length,
      path_elements: renderer.page_element.querySelectorAll('path').length,
      word_groups: renderer.page_element.querySelectorAll('g[data-wid]').length
    }
  }

  const detached = Object.fromEntries(pages.map(number => [number, []]))
  const attached = Object.fromEntries(pages.map(number => [number, []]))
  const sandbox = document.createElement('div')
  sandbox.style.cssText = 'position:fixed;inset:0;width:390px;height:620px;opacity:0;pointer-events:none'
  document.body.append(sandbox)
  for (let round = 0; round < 35; round++) {
    for (let offset = 0; offset < pages.length; offset++) {
      const number = pages[(round + offset) % pages.length]
      const markup = markups[number]

      let start = performance.now()
      const template = document.createElement('template')
      template.innerHTML = markup
      template.content.querySelectorAll('*').length
      let end = performance.now()
      if (round >= 5) detached[number].push(end - start)

      start = performance.now()
      const host = document.createElement('div')
      host.style.cssText = 'width:390px;height:620px'
      host.innerHTML = markup
      sandbox.replaceChildren(host)
      for (const group of host.querySelectorAll('svg > g')) group.getBBox()
      host.getBoundingClientRect()
      end = performance.now()
      if (round >= 5) attached[number].push(end - start)
      sandbox.replaceChildren()
    }
  }
  sandbox.remove()

  const stats = values => {
    const sorted = [...values].sort((a, b) => a - b)
    return {
      median_ms: sorted[Math.floor((sorted.length - 1) * 0.5)],
      p95_ms: sorted[Math.floor((sorted.length - 1) * 0.95)]
    }
  }
  return {
    user_agent: navigator.userAgent,
    rows: pages.map(number => ({
      page: number,
      ...descriptions[number],
      detached_parse: stats(detached[number]),
      parse_attach_geometry: stats(attached[number]),
      digitalkhatt_completion: stats(generation[number])
    })),
    markups
  }
})
await browser.close()

const qvp_sizes_path = resolve(root, 'web-benchmark/results/comparisons/qvp-size-comparison.json')
const qvp_times_path = resolve(root, 'web-benchmark/results/comparisons/qvp-browser-comparison.json')
const qvp_sizes = JSON.parse(await readFile(qvp_sizes_path))
const qvp_times = JSON.parse(await readFile(qvp_times_path))
for (const row of measured.rows) {
  const markup = Buffer.from(measured.markups[row.page])
  const qvp_size = qvp_sizes.rows.find(item => item.page === row.page)
  const qvp_time = qvp_times.rows.find(item => item.page === row.page)
  Object.assign(row, {
    markup_bytes: markup.byteLength,
    markup_gzip_bytes: gzipSync(markup, { level: 9 }).byteLength,
    markup_brotli_11_bytes: brotliCompressSync(markup, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 }
    }).byteLength,
    qvp_bytes: qvp_size.qvp,
    qvp_brotli_11_bytes: qvp_size.qvp_br,
    qvp_decode_path2d: qvp_time.qvp
  })
}

async function hash_tree(paths) {
  const hash = createHash('sha256')
  async function add(path) {
    const info = await stat(path)
    if (info.isDirectory()) {
      for (const name of (await readdir(path)).sort()) await add(resolve(path, name))
    } else {
      hash.update(path.slice(muhaffidh_dir.length))
      hash.update(await readFile(path))
    }
  }
  for (const path of paths) await add(path)
  return hash.digest('hex')
}

const source_paths = [
  resolve(muhaffidh_dir, 'package.json'),
  resolve(muhaffidh_dir, 'pnpm-lock.yaml'),
  resolve(muhaffidh_dir, 'index.html'),
  resolve(muhaffidh_dir, 'vite.config.js'),
  resolve(muhaffidh_dir, 'src'),
  resolve(muhaffidh_dir, 'packages/digitalkhatt-web'),
  resolve(muhaffidh_dir, 'public/hb.wasm'),
  resolve(muhaffidh_dir, 'public/dk-fonts/madina.otf')
]
const git = (...args) => execFileSync('git', ['-C', muhaffidh_dir, ...args], { encoding: 'utf8' }).trim()
let relevant_inputs_clean = true
try {
  execFileSync('git', ['-C', muhaffidh_dir, 'diff', '--quiet', 'HEAD', '--', ...source_paths])
} catch {
  relevant_inputs_clean = false
}

const result = {
  date: new Date().toISOString(),
  hardware,
  user_agent: measured.user_agent,
  source: {
    repository: git('remote', 'get-url', 'origin'),
    directory: muhaffidh_dir,
    commit: git('rev-parse', 'HEAD'),
    relevant_inputs_clean,
    mushaf: 'dk-medina1441-hafs',
    surface_css_pixels: [390, 620],
    tajwid: false
  },
  method: {
    generation: 'DigitalKhattPageRenderer update through FINISHED, including its cooperative animation-frame yields, with warmed HarfBuzz WASM and font, followed by a page bounding-box read',
    detached_parse: 'Assign serialized generated page markup to a detached HTML template and query all descendants',
    parse_attach_geometry: 'Assign markup to innerHTML, attach it to a 390x620 transparent on-screen container, call getBBox on every line SVG group, and read the host bounding box; paint is not forced',
    network_included: false,
    generation_warmup: 5,
    generation_repetitions: 20,
    parse_warmup: 5,
    parse_repetitions: 30
  },
  rows: measured.rows,
  shared_digitalkhatt_assets: {
    harfbuzz_wasm_bytes: (await stat(resolve(muhaffidh_dir, 'public/hb.wasm'))).size,
    harfbuzz_wasm_brotli_11_bytes: brotliCompressSync(await readFile(resolve(muhaffidh_dir, 'public/hb.wasm')), {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 }
    }).byteLength,
    font_bytes: (await stat(resolve(muhaffidh_dir, 'public/dk-fonts/madina.otf'))).size,
    font_brotli_11_bytes: brotliCompressSync(await readFile(resolve(muhaffidh_dir, 'public/dk-fonts/madina.otf')), {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 }
    }).byteLength
  },
  limitations: [
    'Muhaffidh generates line SVGs at runtime rather than loading a static SVG page file.',
    'The serialized markup repeats glyph path strings and was not optimized as a purpose-built static SVG corpus.',
    'QVP and DigitalKhatt are different outline sources, so matching page numbers do not imply identical path complexity or pixels.',
    'These are warm desktop measurements and do not establish mobile performance.',
    'The QVP preparation values come from a separate run in the same Chrome version and hardware class.'
  ],
  sha256: {
    benchmark: createHash('sha256').update(await readFile(fileURLToPath(import.meta.url))).digest('hex'),
    muhaffidh_inputs: await hash_tree(source_paths),
    qvp_sdk: createHash('sha256').update(await readFile(resolve(root, 'web/lite.mjs'))).digest('hex'),
    qvp_measurements: createHash('sha256')
      .update(await readFile(qvp_sizes_path))
      .update(await readFile(qvp_times_path))
      .digest('hex')
  }
}
await writeFile(output, JSON.stringify(result, null, 2) + '\n')
console.log(JSON.stringify(result, null, 2))
