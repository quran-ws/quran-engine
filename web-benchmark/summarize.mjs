import { readFile, writeFile } from 'node:fs/promises'

const filename = process.argv[2]
if (!filename) throw new Error('Usage: node summarize.mjs results/<run>.json')
const results = JSON.parse(await readFile(filename, 'utf8'))
if (!results.complete) throw new Error('Cannot summarize an incomplete run')
const modes = ['cached', 'direct', 'gpu']
const names = { cached: 'Canvas cached', direct: 'Canvas direct', gpu: 'WebGL triangles' }
const quantile = (values, q) => [...values].sort((a, b) => a - b)[Math.floor((values.length - 1) * q)]
const f = value => value.toFixed(3)
const output = [
  '# QVP rendering benchmark results', '',
  `Run: ${results.timestamp}. Raw data: [${filename}](${filename}).`, '',
  `Browser: ${results.environment.user_agent}. Cross-origin isolated: ${results.environment.cross_origin_isolated}.`, '',
  `Surface: ${results.config.width} × ${results.config.height} CSS pixels at DPR ${results.config.dpr}. ${results.config.repeats} repetitions, ${results.config.frames} measured frames per workload per trial. Idle rAF median: ${f(results.idle_intervals.p50)} ms.`, '',
  'CPU values below are draw-submission times, not GPU execution times. rAF values describe callback pacing, not physical presentation. Quantiles pool all sampled pages and repetitions.', '',
  '| Renderer | Workload | CPU p50 ms | CPU p95 ms | rAF p95 ms | Late intervals |',
  '| --- | --- | ---: | ---: | ---: | ---: |'
]
const summary = []
for (const mode of modes) {
  for (const scenario of ['preview_zoom', 'sharp_zoom', 'highlight']) {
    const workloads = results.trials.filter(t => t.mode === mode).flatMap(t => t.workloads).filter(w => w.scenario === scenario)
    const cpu = workloads.flatMap(w => w.cpu_ms)
    const intervals = workloads.flatMap(w => w.interval_ms)
    const late = workloads.reduce((sum, w) => sum + w.late_intervals, 0)
    const item = { mode, scenario, cpu_p50: quantile(cpu, .5), cpu_p95: quantile(cpu, .95), raf_p95: quantile(intervals, .95), late, count: intervals.length }
    summary.push(item)
    output.push(`| ${names[mode]} | ${scenario} | ${f(item.cpu_p50)} | ${f(item.cpu_p95)} | ${f(item.raf_p95)} | ${late}/${intervals.length} |`)
  }
}
output.push('', 'Setup includes preparation, construction and a one-pixel readback of the first draw. Downloads and original QVP decoding are excluded.', '', '| Page | Renderer | Setup median ms | Worker mesh median ms | Nominal extra raster MiB | Uploaded mesh MiB |', '| --- | --- | ---: | ---: | ---: | ---: |')
for (const page of results.config.pages) {
  for (const mode of modes) {
    const trials = results.trials.filter(t => t.mode === mode && t.page === page)
    const median = key => quantile(trials.map(t => t.setup[key]), .5)
    output.push(`| ${page} | ${names[mode]} | ${f(median('setup_ms'))} | ${f(median('mesh_ms'))} | ${(median('cache_bytes') / 1048576).toFixed(2)} | ${(median('mesh_bytes') / 1048576).toFixed(2)} |`)
  }
}
output.push('', 'Buffer figures exclude driver/MSAA/swapchain allocations, native Path2D storage, and JS/transient allocations; these are not total memory measurements.', '', 'GPU backend: ' + results.trials.find(t => t.mode === 'gpu').setup.backend + '.', '', 'See [README.md](README.md) for reproduction, visual checks, and workload limitations.', '')
await writeFile('RESULTS.md', output.join('\n'))
await writeFile('results/summary.json', JSON.stringify({ source: filename, summary }, null, 2))
console.log(output.join('\n'))
