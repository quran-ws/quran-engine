// Run from a browser serving the repository root. See docs/qvp-format-investigation.md.
export const compareQvp = async ({ qvpBase = 'https://qvp.quran.ws/v0.1.0' } = {}) => {
  await import('/web-benchmark/geometry.js')
  const { decodePage } = await import('/web/lite.mjs')

  const decodeExport = buffer => {
    const geometry = qvp_geometry.decode(buffer)
    const paths = qvp_geometry.paths(geometry)
    const words = Array.from({ length: geometry.n_words }, (_, index) =>
      Array.from(geometry.words.subarray(index * 4, index * 4 + 4)))
    return {
      width: geometry.width,
      height: geometry.height,
      number: geometry.number,
      pathCount: paths.length,
      words,
      fit(canvas, padding = 0) {
        const scale = Math.min((canvas.width - 2 * padding) / this.width, (canvas.height - 2 * padding) / this.height)
        return {
          scale,
          x: (canvas.width - this.width * scale) / 2,
          y: (canvas.height - this.height * scale) / 2
        }
      },
      draw(context, { scale = 1, x = 0, y = 0, ink = '#231f20', highlight = [], band = '#bedbfa' } = {}) {
        context.save()
        context.setTransform(scale, 0, 0, scale, x, y)
        context.fillStyle = band
        for (const index of highlight) {
          const box = words[index]
          if (box) context.fillRect(box[0], box[1], box[2] - box[0], box[3] - box[1])
        }
        context.fillStyle = ink
        for (const path of paths) context.fill(path.path, path.rule)
        context.restore()
      }
    }
  }

  const pages = [1, 42, 585]
  const sources = []
  for (const number of pages) {
    const name = String(number).padStart(3, '0')
    const [qvpResponse, qvbResponse] = await Promise.all([
      fetch(`${qvpBase}/${name}.qvp`),
      fetch(`/web-benchmark/data/${name}.bin`)
    ])
    if (!qvpResponse.ok) throw new Error(`Could not load ${name}.qvp: HTTP ${qvpResponse.status}`)
    if (!qvbResponse.ok) throw new Error(`Could not load ${name}.bin: HTTP ${qvbResponse.status}`)
    sources.push({ qvp: await qvpResponse.arrayBuffer(), qvb: await qvbResponse.arrayBuffer() })
  }

  const assert = (value, message) => { if (!value) throw new Error(message) }
  let pixelChecks = 0
  for (const source of sources) {
    const direct = decodePage(source.qvp)
    const exported = decodeExport(source.qvb)
    assert(
      JSON.stringify(direct.words.map(word => word.box)) === JSON.stringify(exported.words),
      'word boxes differ'
    )
    for (const zoom of [1, 2, 4]) {
      const directCanvas = new OffscreenCanvas(1170, 1860)
      const exportedCanvas = new OffscreenCanvas(1170, 1860)
      const directContext = directCanvas.getContext('2d')
      const exportedContext = exportedCanvas.getContext('2d')
      const view = direct.fit(directCanvas, 36)
      view.scale *= zoom
      view.x = (directCanvas.width - direct.width * view.scale) / 2
      view.y = (directCanvas.height - direct.height * view.scale) / 2
      const options = { ...view, highlight: [Math.floor(direct.words.length / 2)] }
      direct.draw(directContext, options)
      exported.draw(exportedContext, options)
      const directPixels = directContext.getImageData(0, 0, directCanvas.width, directCanvas.height).data
      const exportedPixels = exportedContext.getImageData(0, 0, exportedCanvas.width, exportedCanvas.height).data
      assert(directPixels.every((value, index) => value === exportedPixels[index]), 'pixels differ')
      pixelChecks++
    }
  }

  const samples = pages.map(() => ({ qvp: [], qvb: [] }))
  let pageAccumulator = 0
  for (let round = 0; round < 120; round++) {
    for (let offset = 0; offset < pages.length; offset++) {
      const index = (round + offset) % pages.length
      for (const mode of round % 2 ? ['qvp', 'qvb'] : ['qvb', 'qvp']) {
        const start = performance.now()
        const page = mode === 'qvp' ? decodePage(sources[index].qvp) : decodeExport(sources[index].qvb)
        const elapsed = performance.now() - start
        pageAccumulator += page.number
        if (round >= 20) samples[index][mode].push(elapsed)
      }
    }
    await new Promise(requestAnimationFrame)
  }

  const stats = values => {
    const sorted = [...values].sort((a, b) => a - b)
    return {
      median_ms: sorted[Math.floor((sorted.length - 1) * 0.5)],
      p95_ms: sorted[Math.floor((sorted.length - 1) * 0.95)],
      samples: values
    }
  }
  return {
    user_agent: navigator.userAgent,
    isolated: crossOriginIsolated,
    qvp_base: qvpBase,
    warmup: 20,
    repetitions: 100,
    pixel_checks: pixelChecks,
    page_accumulator: pageAccumulator,
    rows: pages.map((page, index) => ({ page, qvp: stats(samples[index].qvp), qvb: stats(samples[index].qvb) }))
  }
}
