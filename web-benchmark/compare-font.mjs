import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { resolve } from 'node:path'

const app_dir = resolve(process.argv[2] ?? '../../muhaffidh')
const app_url = process.argv[3] ?? 'http://127.0.0.1:4187/'
const font_dir = resolve(process.argv[4] ?? '/tmp/qcf4-fonts')
const output = resolve(process.argv[5] ?? new URL('./results/comparisons/font-browser-comparison.json', import.meta.url).pathname)
const hardware = process.argv[6] ?? null
const qvp_url = process.argv[7] ?? 'http://127.0.0.1:4186/'
const qvp_data_base = process.argv[8] ?? 'https://qvp.quran.ws/v0.1.0'
const pages = [42, 300, 585]
const data_rows = (await readFile(resolve(app_dir, 'src/data.txt'), 'utf8'))
  .trim().split('\n').map(line => line.split(',').map(Number))
const page_fonts = Object.fromEntries(pages.map(page => [page,
  [...new Set(data_rows.filter(row => row[2] === page).map(row => row[4]))]]))
const font_ids = [...new Set(Object.values(page_fonts).flat())]
const font_filename = id => id === 0
  ? 'QCF4_QBSML.woff2'
  : `QCF4_Hafs_${String(id).padStart(2, '0')}_W.woff2`
const font_buffers = Object.fromEntries(await Promise.all(font_ids.map(async id => [
  id,
  (await readFile(resolve(font_dir, font_filename(id)))).toString('base64')
])))

let require = createRequire(resolve(app_dir, 'package.json'))
let playwright
try {
  playwright = require('@playwright/test')
} catch {
  require = createRequire(resolve(app_dir, '../muhaffidh/package.json'))
  playwright = require('@playwright/test')
}
const { chromium } = playwright
const browser = await chromium.launch({ headless: true, channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 390, height: 620 } })
await page.goto(app_url, { waitUntil: 'domcontentloaded', timeout: 120000 })
const results = await page.evaluate(async ({ pages, page_fonts, font_buffers }) => {
  document.body.replaceChildren()
  const { build_mushaf_page } = await import('/src/mushaf/index.js')
  const markups = Object.fromEntries(pages.map(number => {
    const built = build_mushaf_page(number)
    return [number, typeof built === 'string' ? built : built.html]
  }))
  const buffers = Object.fromEntries(Object.entries(font_buffers).map(([id, base64]) => {
    const binary = atob(base64)
    const bytes = Uint8Array.from(binary, character => character.charCodeAt(0))
    return [id, bytes.buffer]
  }))
  const samples = Object.fromEntries(pages.map(number => [number, {
    font_construct: [], font_load: [], font_total: [], html_create: [], layout: [],
    first_canvas_raster: [], cached_canvas_raster: []
  }]))

  function draw_glyphs(canvas, root, glyphs, family_by_id) {
    const context = canvas.getContext('2d')
    const root_rect = root.getBoundingClientRect()
    context.clearRect(0, 0, canvas.width, canvas.height)
    context.save()
    context.scale(3, 3)
    context.fillStyle = '#000'
    context.textBaseline = 'alphabetic'
    for (const glyph of glyphs) {
      const rect = glyph.getBoundingClientRect()
      const style = getComputedStyle(glyph)
      context.font = `${style.fontSize} ${family_by_id[glyph.dataset.font]}`
      context.fillText(glyph.textContent, rect.left - root_rect.left, rect.bottom - root_rect.top)
    }
    context.restore()
    context.getImageData(0, 0, 1, 1)
  }

  for (let round = 0; round < 25; round++) {
    for (let offset = 0; offset < pages.length; offset++) {
      const number = pages[(round + offset) % pages.length]
      const faces = []
      const family_by_id = {}
      let construct_time = 0
      let load_time = 0
      const font_start = performance.now()
      for (const id of page_fonts[number]) {
        const family = `QcfBenchmark${number}_${round}_${id}`
        family_by_id[id] = family
        let start = performance.now()
        const face = new FontFace(family, buffers[id], { display: 'block' })
        construct_time += performance.now() - start
        start = performance.now()
        await face.load()
        load_time += performance.now() - start
        document.fonts.add(face)
        faces.push(face)
      }
      const font_end = performance.now()

      let start = performance.now()
      const root = document.createElement('mushaf-page-inner')
      root.style.cssText = 'display:block;width:355px;height:574px'
      root.innerHTML = `<div class="w-full pr-2" data-fonts-loaded="true">${markups[number]}</div>`
      const glyphs = [...root.querySelectorAll('[data-char]')]
      for (const glyph of glyphs) glyph.style.fontFamily = family_by_id[glyph.dataset.font]
      document.body.replaceChildren(root)
      const html_end = performance.now()

      for (const glyph of glyphs) glyph.getBoundingClientRect()
      root.getBoundingClientRect()
      const layout_end = performance.now()

      const canvas = document.createElement('canvas')
      canvas.width = 1170
      canvas.height = 1860
      start = performance.now()
      draw_glyphs(canvas, root, glyphs, family_by_id)
      const first_raster_end = performance.now()
      draw_glyphs(canvas, root, glyphs, family_by_id)
      const cached_raster_end = performance.now()

      if (round >= 5) {
        const values = samples[number]
        values.font_construct.push(construct_time)
        values.font_load.push(load_time)
        values.font_total.push(font_end - font_start)
        values.html_create.push(html_end - font_end)
        values.layout.push(layout_end - html_end)
        values.first_canvas_raster.push(first_raster_end - start)
        values.cached_canvas_raster.push(cached_raster_end - first_raster_end)
      }
      for (const face of faces) document.fonts.delete(face)
    }
  }

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
      fonts: page_fonts[number],
      glyphs: data_rows_for_page(markups[number]),
      ...Object.fromEntries(Object.entries(samples[number]).map(([key, value]) => [key, stats(value)]))
    }))
  }

  function data_rows_for_page(markup) {
    const template = document.createElement('template')
    template.innerHTML = markup
    return template.content.querySelectorAll('[data-char]').length
  }
}, { pages, page_fonts, font_buffers })
await browser.close()

const qvp_browser = await chromium.launch({ headless: true, channel: 'chrome' })
const qvp_page = await qvp_browser.newPage()
await qvp_page.goto(qvp_url, { waitUntil: 'domcontentloaded', timeout: 120000 })
const qvp_results = await qvp_page.evaluate(async ({ pages, qvp_data_base }) => {
  const { decodePage } = await import('/web/lite.mjs')
  const buffers = {}
  for (const number of pages) {
    buffers[number] = await (await fetch(`${qvp_data_base}/${String(number).padStart(3, '0')}.qvp`)).arrayBuffer()
  }
  const samples = Object.fromEntries(pages.map(number => [number, {
    prepare: [], first_draw: [], cached_draw: []
  }]))
  for (let round = 0; round < 120; round++) {
    for (let offset = 0; offset < pages.length; offset++) {
      const number = pages[(round + offset) % pages.length]
      let start = performance.now()
      const qvp = decodePage(buffers[number])
      const prepared = performance.now()
      const canvas = new OffscreenCanvas(1170, 1860)
      const context = canvas.getContext('2d')
      const view = qvp.fit(canvas, 36)
      qvp.draw(context, view)
      context.getImageData(0, 0, 1, 1)
      const first_drawn = performance.now()
      context.clearRect(0, 0, canvas.width, canvas.height)
      qvp.draw(context, view)
      context.getImageData(0, 0, 1, 1)
      const cached_drawn = performance.now()
      if (round >= 20) {
        samples[number].prepare.push(prepared - start)
        samples[number].first_draw.push(first_drawn - prepared)
        samples[number].cached_draw.push(cached_drawn - first_drawn)
      }
    }
  }
  const stats = values => {
    const sorted = [...values].sort((a, b) => a - b)
    return {
      median_ms: sorted[Math.floor((sorted.length - 1) * 0.5)],
      p95_ms: sorted[Math.floor((sorted.length - 1) * 0.95)]
    }
  }
  return pages.map(number => ({
    page: number,
    ...Object.fromEntries(Object.entries(samples[number]).map(([key, value]) => [key, stats(value)]))
  }))
}, { pages, qvp_data_base })
await qvp_browser.close()

for (const row of results.rows) {
  row.font_files = await Promise.all(row.fonts.map(async id => {
    const bytes = await readFile(resolve(font_dir, font_filename(id)))
    return {
      id,
      filename: font_filename(id),
      bytes: bytes.byteLength,
      sha256: createHash('sha256').update(bytes).digest('hex')
    }
  }))
}
const git = (...args) => execFileSync('git', ['-C', app_dir, ...args], { encoding: 'utf8' }).trim()
const result = {
  date: new Date().toISOString(),
  hardware,
  user_agent: results.user_agent,
  source: {
    application: app_dir,
    repository: git('remote', 'get-url', 'origin'),
    commit: git('rev-parse', 'HEAD')
  },
  method: {
    network_included: false,
    font_source: 'FontFace constructed from an in-memory WOFF2 ArrayBuffer under a unique family name',
    layout: 'Actual application page markup inserted at 355x574 CSS pixels, then every glyph bounding box read',
    raster: 'Every page glyph drawn at its laid-out position into a 1170x1860 Canvas 2D surface, followed by a one-pixel synchronous readback',
    warmup: 5,
    repetitions: 20
  },
  rows: results.rows,
  qvp: {
    url: qvp_url,
    data_base: qvp_data_base,
    surface_backing_pixels: [1170, 1860],
    warmup: 20,
    repetitions: 100,
    rows: qvp_results
  },
  limitations: [
    'Repeated FontFace construction may benefit from browser-level caching of identical font bytes.',
    'Canvas raster measurement isolates glyph rasterization but is not a measurement of DOM paint or physical presentation.',
    'These are warm desktop measurements and do not establish mobile performance.'
  ]
}
await writeFile(output, JSON.stringify(result, null, 2) + '\n')
console.log(JSON.stringify(result, null, 2))
