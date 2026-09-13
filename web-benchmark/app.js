import { CanvasRenderer, WebglRenderer, page_view } from './renderers.js'

const $ = id => document.getElementById(id)
const modes = ['cached', 'direct', 'gpu']
const names = { cached: 'Canvas cached', direct: 'Canvas direct', gpu: 'WebGL triangles' }
const scenarios = ['preview_zoom', 'sharp_zoom', 'highlight']
const worker = new Worker('./worker.js')
const buffers = new Map()
let manifest, current, running = false

function profile() {
  if ($('profile').value === 'desktop') return { width: 720, height: 1100, dpr: 2 }
  return { width: 390, height: 620, dpr: $('profile').value === 'device' ? devicePixelRatio : 3 }
}

function stats(values) {
  const sorted = [...values].sort((a, b) => a - b)
  return { p50: sorted[Math.floor((sorted.length - 1) * .5)], p95: sorted[Math.floor((sorted.length - 1) * .95)], max: sorted.at(-1), mean: values.reduce((a, b) => a + b, 0) / values.length }
}

function busy(value) {
  running = value
  for (const control of document.querySelectorAll('.controls button, .controls select')) control.disabled = value
  $('download').disabled = value || !window.benchmark_results
}

function frame() {
  return new Promise(resolve => requestAnimationFrame(resolve))
}

async function source(number) {
  if (!buffers.has(number)) {
    const response = await fetch(`./data/${String(number).padStart(3, '0')}.bin`)
    if (!response.ok) throw new Error(`Page ${number}: HTTP ${response.status}`)
    buffers.set(number, await response.arrayBuffer())
  }
  return buffers.get(number)
}

function prepare(buffer, mesh, tolerance) {
  return new Promise((resolve, reject) => {
    worker.onmessage = ({ data }) => data.error ? reject(new Error(data.error)) : resolve(data)
    worker.onerror = event => reject(new Error(event.message))
    worker.postMessage({ buffer, mesh, tolerance }, [buffer])
  })
}

async function create_renderer(number, mode, config) {
  const raw = await source(number)
  current?.dispose()
  current = null
  const start = performance.now()
  const canvas = document.createElement('canvas')
  canvas.width = Math.round(config.width * config.dpr)
  canvas.height = Math.round(config.height * config.dpr)
  canvas.style.width = `${config.width}px`
  canvas.dpr = config.dpr
  $('stage').replaceChildren(canvas)
  const header = qvp_geometry.decode(raw)
  const tolerance = .2 / (page_view(header, canvas).scale * 4)
  const prepared = await prepare(raw.slice(0), mode === 'gpu', tolerance)
  const preparation_elapsed_ms = performance.now() - start
  const page = qvp_geometry.decode(prepared.buffer)
  const construct_start = performance.now()
  const renderer = mode === 'gpu'
    ? new WebglRenderer(canvas, page, new Float32Array(prepared.triangles))
    : new CanvasRenderer(canvas, page, mode === 'cached')
  const construct_ms = performance.now() - construct_start
  const draw_start = performance.now()
  renderer.draw()
  renderer.sync()
  const first_draw_sync_ms = performance.now() - draw_start
  const setup_ms = performance.now() - start
  current = renderer
  renderer.setup = {
    setup_ms, preparation_elapsed_ms, decode_ms: prepared.decode_ms, library_ms: prepared.library_ms,
    mesh_ms: prepared.mesh_ms, construct_ms, first_draw_sync_ms,
    paths: page.n_paths, coordinate_floats: page.n_pts, source_bytes: raw.byteLength,
    tolerance_page_units: tolerance, max_flattening_error_backing_px_at_4x: .2,
    display_css_width: canvas.getBoundingClientRect().width, display_css_height: canvas.getBoundingClientRect().height,
    source_cache_bytes: [...buffers.values()].reduce((sum, value) => sum + value.byteLength, 0),
    live_geometry_copy_bytes: prepared.buffer.byteLength, ...renderer.info
  }
  return renderer
}

async function preview() {
  if (running) return
  busy(true)
  try {
    $('status').textContent = 'Preparing renderer…'
    const renderer = await create_renderer(Number($('page').value), $('mode').value, profile())
    renderer.draw(Number($('zoom').value), -1, true)
    $('details').textContent = JSON.stringify(renderer.setup, null, 2)
    $('status').textContent = `${names[$('mode').value]} · page ${$('page').value} · ${$('zoom').value}× · ready`
  } finally { busy(false) }
}

async function workload(renderer, scenario, count, warmup, baseline_ms) {
  renderer.reset()
  const cpu_ms = [], interval_ms = []
  let previous = null
  for (let i = -warmup; i < count; i++) {
    const time = await frame()
    if (document.hidden) throw new Error('Keep the benchmark tab visible; background throttling invalidates measurements')
    if (i >= 0 && previous !== null) interval_ms.push(time - previous)
    previous = time
    const phase = i < 0 ? (i + warmup) / Math.max(1, warmup - 1) : i / Math.max(1, count - 1)
    const zoom = scenario === 'highlight' ? 1 : 1 + 3 * Math.sin(phase * Math.PI) ** 2
    const pan = scenario === 'highlight' ? 0 : Math.sin(phase * Math.PI * 2) * renderer.canvas.width * .08
    const word = scenario === 'highlight' ? Math.floor(phase * (renderer.page.n_words - 1)) : -1
    const start = performance.now()
    renderer.draw(zoom, word, scenario === 'sharp_zoom', pan)
    if (i >= 0) cpu_ms.push(performance.now() - start)
  }
  // The next rAF includes the final measured draw's pacing, rather than warmup's last draw.
  const end = await frame()
  interval_ms.push(end - previous)
  interval_ms.shift()
  const intervals = stats(interval_ms)
  return {
    scenario, frames: count, cpu_ms, interval_ms, cpu: stats(cpu_ms), intervals,
    late_intervals: interval_ms.filter(v => v > baseline_ms * 1.5).length,
    late_percent: interval_ms.filter(v => v > baseline_ms * 1.5).length / count * 100
  }
}

function add_row(number, mode, result) {
  const row = document.createElement('tr')
  for (const value of [number, names[mode], result.scenario, `${result.cpu.p50.toFixed(2)} ms`, `${result.cpu.p95.toFixed(2)} ms`, `${result.intervals.p95.toFixed(2)} ms`, `${result.late_percent.toFixed(1)}%`]) {
    const cell = document.createElement('td')
    cell.textContent = value
    row.append(cell)
  }
  $('rows').append(row)
}

async function run_benchmark(options = {}) {
  if (running) throw new Error('Already running')
  busy(true)
  $('rows').replaceChildren()
  $('qa').replaceChildren()
  const config = { ...profile(), frames: 90, warmup: 90, repeats: 3, pages: manifest.pages, modes, ...options }
  const results = {
    timestamp: new Date().toISOString(), config,
    environment: { user_agent: navigator.userAgent, platform: navigator.platform, hardware_concurrency: navigator.hardwareConcurrency, device_memory_gib: navigator.deviceMemory ?? null, actual_dpr: devicePixelRatio, cross_origin_isolated: crossOriginIsolated, surface_is_cpu_emulation: false },
    methodology: {
      data: 'Exact decoded QVP outlines exported offline; QVB parsing, not original QVP decompression. Network excluded; assets fetched before measurement.',
      setup: 'Warm setup: every mode/page is exercised during untimed visual validation, then every mode is set up/disposed again before idle calibration. New renderer per trial, persistent worker; includes buffer copy/transfer, preparation, context/path/mesh creation, first draw and one-pixel synchronous readback.',
      cpu: 'JavaScript draw submission duration, not GPU execution time.',
      intervals: 'requestAnimationFrame callback intervals, not hardware presentation timestamps. Late means >1.5 times measured idle median.',
      preview_zoom: 'Canvas cached scales a 1x raster and is temporarily softer; direct Canvas and WebGL rerender geometry.',
      sharp_zoom: 'All rerender at each zoom. Cached Canvas refreshes a viewport-sized raster each frame.',
      highlight: 'Moving word bounding-box band behind black ink; no ink recolouring or layout engine.',
      memory: 'Explicitly sized source-cache, live geometry-copy and mesh buffers plus nominal RGBA surfaces only. Canvas Path2D native storage, driver/MSAA/swapchain allocations, JS overhead, transients and total process memory are not measured.',
      order: 'Page, renderer and scenario order rotate across repeats. Explicit trial order is recorded. Two idle frames follow each context disposal.',
      warmup: 'One complete 90-frame trajectory before each measured 90-frame trajectory by default.',
      quantiles: 'Sorted sample at floor((n-1)*q); pooled frame quantiles are descriptive, not independent observations or confidence estimates.',
      gpu: 'Adaptive curve flattening (0.2 backing-pixel tolerance through 4x), libtess winding-rule tessellation in worker, single triangle batch for ink, native WebGL MSAA. No WASM.'
    }, trials: []
  }
  window.benchmark_results = results
  try {
    results.provenance = await (await fetch('./provenance')).json()
    $('status').textContent = 'Loading assets and measuring idle frame pacing…'
    for (const number of config.pages) await source(number)
    results.validation = []
    for (const number of config.pages) {
      const page = qvp_geometry.decode(await source(number))
      for (const zoom of [1, 2, 4]) {
        for (const word of [-1, Math.floor(page.n_words / 2)]) {
          for (const mode of ['cached', 'gpu']) {
            $('status').textContent = `Visual check · page ${number} · ${names[mode]} · ${zoom}× · ${word < 0 ? 'ink' : 'highlight'}`
            const validation = await validate_case(number, zoom, mode, word, config)
            results.validation.push(validation)
            if (!validation.passed) throw new Error(`Visual validation failed for ${mode}, page ${number}, zoom ${zoom}`)
          }
        }
      }
    }
    for (const mode of config.modes) await create_renderer(config.pages[0], mode, config)
    current?.dispose()
    current = null
    await frame()
    await frame()
    const idle = []
    let last = await frame()
    for (let i = 0; i < 45; i++) { const now = await frame(); idle.push(now - last); last = now }
    results.idle_intervals = stats(idle)
    for (let repeat = 0; repeat < config.repeats; repeat++) {
      for (let page_index = 0; page_index < config.pages.length; page_index++) {
        const number = config.pages[(page_index + repeat) % config.pages.length]
        for (let mode_index = 0; mode_index < config.modes.length; mode_index++) {
          const mode = config.modes[(mode_index + repeat + config.pages.indexOf(number)) % config.modes.length]
          $('status').textContent = `Round ${repeat + 1}/${config.repeats} · page ${number} · ${names[mode]} · preparing`
          const renderer = await create_renderer(number, mode, config)
          $('details').textContent = JSON.stringify(renderer.setup, null, 2)
          const trial = { repeat, page: number, mode, setup: renderer.setup, workloads: [] }
          results.trials.push(trial)
          for (let scenario_index = 0; scenario_index < scenarios.length; scenario_index++) {
            const scenario = scenarios[(scenario_index + repeat) % scenarios.length]
            $('status').textContent = `Round ${repeat + 1}/${config.repeats} · page ${number} · ${names[mode]} · ${scenario}`
            const result = await workload(renderer, scenario, config.frames, config.warmup, results.idle_intervals.p50)
            trial.workloads.push(result)
            add_row(number, mode, result)
          }
          if (mode === 'gpu' && renderer.gl.getError() !== renderer.gl.NO_ERROR) throw new Error('WebGL error during measured workload')
          renderer.dispose()
          current = null
          await frame()
          await frame()
        }
      }
    }
    results.complete = true
    const response = await fetch('./results', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(results) })
    if (!response.ok) throw new Error('Could not save results')
    const saved = await response.json()
    $('status').textContent = `Complete · ${results.trials.length} trials · saved to ${saved.file}`
    results.saved_file = saved.file
    return results
  } catch (error) {
    results.error = error.stack
    $('status').textContent = `Stopped: ${error.message}`
    throw error
  } finally { busy(false) }
}

function image_for(pixels, width, height, label) {
  const canvas = document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(pixels), width, height), 0, 0)
  const image = new Image()
  image.src = canvas.toDataURL()
  image.alt = label
  const figure = document.createElement('figure')
  const caption = document.createElement('figcaption')
  caption.textContent = label
  figure.append(image, caption)
  $('qa').append(figure)
  return image.src
}

async function validate_case(number, zoom, mode, word, config, show = false) {
    const canvas = await create_renderer(number, 'direct', config)
    canvas.draw(zoom, word, true)
    const reference = canvas.pixels()
    const width = canvas.canvas.width, height = canvas.canvas.height
    const renderer = await create_renderer(number, mode, config)
    renderer.draw(zoom, word, true)
    const actual = renderer.pixels()
    let absolute_error = 0, different_pixels = 0, different_ink_pixels = 0, missing_interior = 0, extra_interior = 0, union_ink = 0
    for (let i = 0; i < reference.length; i += 4) {
      const difference = Math.max(Math.abs(reference[i] - actual[i]), Math.abs(reference[i + 1] - actual[i + 1]), Math.abs(reference[i + 2] - actual[i + 2]))
      absolute_error += Math.abs(reference[i] - actual[i]) + Math.abs(reference[i + 1] - actual[i + 1]) + Math.abs(reference[i + 2] - actual[i + 2])
      if (difference > 32) different_pixels++
      if (reference[i] < 200 || actual[i] < 200) {
        union_ink++
        if (difference > 32) different_ink_pixels++
      }
      const p = i / 4, x = p % width, y = Math.floor(p / width)
      if (x > 0 && x < width - 1 && y > 0 && y < height - 1) {
        const neighbors = [i - 4, i, i + 4, i - width * 4, i + width * 4]
        if (actual[i] > 150 && neighbors.every(j => reference[j] < 90)) missing_interior++
        if (actual[i] < 90 && neighbors.every(j => reference[j] > 220)) extra_interior++
      }
    }
    const thresholds = { max_mae_rgb_255: mode === 'cached' ? .25 : 2, max_missing_interior: 8, max_extra_interior: 8 }
    const result = { page: number, zoom, mode, word, width, height, mae_rgb_255: absolute_error / (width * height * 3), different_pixels_over_32: different_pixels, fraction_different_of_ink_union: different_ink_pixels / union_ink, missing_interior, extra_interior, backend: renderer.info, thresholds }
    result.passed = result.mae_rgb_255 <= thresholds.max_mae_rgb_255 && missing_interior <= thresholds.max_missing_interior && extra_interior <= thresholds.max_extra_interior
    if (show) {
      window.last_comparison_images = {
        reference: image_for(reference, width, height, `Canvas curves · page ${number} · ${zoom}×`),
        gpu: image_for(actual, width, height, `${names[mode]} · page ${number} · ${zoom}×`)
      }
    }
    return result
}

async function compare_pixels(number = Number($('page').value), zoom = Number($('zoom').value)) {
  if (running) throw new Error('Already running')
  busy(true)
  try {
    $('qa').replaceChildren()
    $('status').textContent = 'Comparing Canvas curves and WebGL triangles…'
    const result = await validate_case(number, zoom, 'gpu', -1, profile(), true)
    $('details').textContent = JSON.stringify(result, null, 2)
    $('status').textContent = `Compared · average RGB error ${result.mae_rgb_255.toFixed(3)}/255 · missing interior pixels ${result.missing_interior} · extra interior pixels ${result.extra_interior}`
    window.last_comparison = result
    return result
  } finally { busy(false) }
}

function handle_error(error) { $('status').textContent = error.message; console.error(error) }
$('preview').onclick = () => preview().catch(handle_error)
$('validate').onclick = () => compare_pixels().catch(handle_error)
$('run').onclick = () => run_benchmark().catch(handle_error)
$('download').onclick = () => {
  const url = URL.createObjectURL(new Blob([JSON.stringify(window.benchmark_results, null, 2)], { type: 'application/json' }))
  const link = document.createElement('a')
  link.href = url
  link.download = 'qvp-benchmark.json'
  link.click()
  URL.revokeObjectURL(url)
}
window.qvp_benchmark = { run: run_benchmark, compare: compare_pixels, preview }
manifest = await (await fetch('./data/manifest.json')).json()
for (const number of manifest.pages) {
  const option = document.createElement('option')
  option.value = number
  option.textContent = `${number}${number === manifest.densest_page ? ' · most coordinates' : number === 1 ? ' · opening' : ' · example text page'}`
  $('page').append(option)
}
$('page').value = '42'
await preview().catch(handle_error)
