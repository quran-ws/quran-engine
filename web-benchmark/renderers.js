import './geometry.js'

const ink = [35 / 255, 31 / 255, 32 / 255, 1]
const paper = [1, 253 / 255, 247 / 255, 1]
const highlight = [190 / 255, 219 / 255, 250 / 255, 1]

export function page_view(page, canvas, zoom = 1, pan = 0) {
  const scale = Math.min((canvas.width - 24 * canvas.dpr) / page.width, (canvas.height - 24 * canvas.dpr) / page.height) * zoom
  return { scale, ox: (canvas.width - page.width * scale) / 2 + pan, oy: (canvas.height - page.height * scale) / 2 }
}

export class CanvasRenderer {
  constructor(canvas, page, cached) {
    this.canvas = canvas
    this.page = page
    this.cached = cached
    this.ctx = canvas.getContext('2d', { alpha: false })
    const start = performance.now()
    this.paths = qvp_geometry.paths(page)
    this.path_ms = performance.now() - start
    if (cached) {
      this.cache = new OffscreenCanvas(canvas.width, canvas.height)
      this.cache_ctx = this.cache.getContext('2d')
      this.refresh()
    }
    this.info = { backend: 'Browser Canvas 2D', cache_bytes: cached ? canvas.width * canvas.height * 4 : 0, surface_bytes: canvas.width * canvas.height * 4, mesh_bytes: 0, path_ms: this.path_ms }
  }

  paint_ink(ctx, view) {
    ctx.setTransform(view.scale, 0, 0, view.scale, view.ox, view.oy)
    ctx.fillStyle = '#231f20'
    for (const { path, rule } of this.paths) ctx.fill(path, rule)
  }

  refresh(zoom = 1, pan = 0) {
    this.cache_ctx.resetTransform()
    this.cache_ctx.clearRect(0, 0, this.canvas.width, this.canvas.height)
    this.paint_ink(this.cache_ctx, page_view(this.page, this.canvas, zoom, pan))
  }

  reset() { if (this.cached) this.refresh() }

  draw(zoom = 1, word = -1, sharp = false, pan = 0) {
    const { ctx, canvas, page } = this
    ctx.resetTransform()
    ctx.fillStyle = '#fffdf7'
    ctx.fillRect(0, 0, canvas.width, canvas.height)
    const view = page_view(page, canvas, zoom, pan)
    if (word >= 0) {
      const box = page.words.subarray(word * 4, word * 4 + 4)
      ctx.setTransform(view.scale, 0, 0, view.scale, view.ox, view.oy)
      ctx.fillStyle = '#bedbfa'
      ctx.fillRect(box[0] - 1, box[1] - 1, box[2] - box[0] + 2, box[3] - box[1] + 2)
    }
    if (this.cached) {
      if (sharp) this.refresh(zoom, pan)
      if (sharp) ctx.resetTransform()
      else ctx.setTransform(zoom, 0, 0, zoom, canvas.width * (1 - zoom) / 2 + pan, canvas.height * (1 - zoom) / 2)
      ctx.drawImage(this.cache, 0, 0)
    } else this.paint_ink(ctx, view)
  }

  sync() { this.ctx.getImageData(0, 0, 1, 1) }
  pixels() { return this.ctx.getImageData(0, 0, this.canvas.width, this.canvas.height).data }
  dispose() {
    if (this.cache) { this.cache.width = 1; this.cache.height = 1 }
    this.paths = []
    this.canvas.width = 1
    this.canvas.height = 1
    this.canvas.remove()
  }
}

export class WebglRenderer {
  constructor(canvas, page, triangles) {
    this.canvas = canvas
    this.page = page
    const gl = canvas.getContext('webgl2', { alpha: false, antialias: true, depth: false, stencil: false })
    if (!gl) throw new Error('WebGL2 is unavailable')
    this.gl = gl
    const vertex = `#version 300 es
      in vec2 position;
      uniform vec2 viewport;
      uniform vec2 factor;
      uniform vec2 offset;
      void main() {
        vec2 p = (position * factor + offset) / viewport * 2.0 - 1.0;
        gl_Position = vec4(p.x, -p.y, 0.0, 1.0);
      }`
    const fragment = `#version 300 es
      precision highp float;
      uniform vec4 color;
      out vec4 output_color;
      void main() { output_color = color; }`
    this.program = gl.createProgram()
    for (const [type, source] of [[gl.VERTEX_SHADER, vertex], [gl.FRAGMENT_SHADER, fragment]]) {
      const shader = gl.createShader(type)
      gl.shaderSource(shader, source)
      gl.compileShader(shader)
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader))
      gl.attachShader(this.program, shader)
      gl.deleteShader(shader)
    }
    gl.linkProgram(this.program)
    if (!gl.getProgramParameter(this.program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(this.program))
    gl.useProgram(this.program)
    this.position = gl.getAttribLocation(this.program, 'position')
    this.factor = gl.getUniformLocation(this.program, 'factor')
    this.offset = gl.getUniformLocation(this.program, 'offset')
    this.color = gl.getUniformLocation(this.program, 'color')
    gl.uniform2f(gl.getUniformLocation(this.program, 'viewport'), canvas.width, canvas.height)
    gl.enableVertexAttribArray(this.position)
    this.mesh = gl.createBuffer()
    gl.bindBuffer(gl.ARRAY_BUFFER, this.mesh)
    gl.bufferData(gl.ARRAY_BUFFER, triangles, gl.STATIC_DRAW)
    this.count = triangles.length / 2
    this.rect = gl.createBuffer()
    gl.bindBuffer(gl.ARRAY_BUFFER, this.rect)
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1]), gl.STATIC_DRAW)
    gl.viewport(0, 0, canvas.width, canvas.height)
    const debug = gl.getExtension('WEBGL_debug_renderer_info')
    this.info = {
      backend: debug ? gl.getParameter(debug.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
      antialias: gl.getContextAttributes().antialias, samples: gl.getParameter(gl.SAMPLES),
      mesh_bytes: triangles.byteLength, triangles: triangles.length / 6,
      cache_bytes: 0, surface_bytes: canvas.width * canvas.height * 4,
      gpu_timer_available: Boolean(gl.getExtension('EXT_disjoint_timer_query_webgl2'))
    }
    if (gl.getError() !== gl.NO_ERROR) throw new Error('WebGL setup failed')
  }

  reset() {}

  draw(zoom = 1, word = -1, sharp = false, pan = 0) {
    const { gl, canvas, page } = this
    const view = page_view(page, canvas, zoom, pan)
    gl.clearColor(...paper)
    gl.clear(gl.COLOR_BUFFER_BIT)
    if (word >= 0) {
      const box = page.words.subarray(word * 4, word * 4 + 4)
      gl.uniform2f(this.factor, (box[2] - box[0] + 2) * view.scale, (box[3] - box[1] + 2) * view.scale)
      gl.uniform2f(this.offset, (box[0] - 1) * view.scale + view.ox, (box[1] - 1) * view.scale + view.oy)
      gl.uniform4fv(this.color, highlight)
      gl.bindBuffer(gl.ARRAY_BUFFER, this.rect)
      gl.vertexAttribPointer(this.position, 2, gl.FLOAT, false, 0, 0)
      gl.drawArrays(gl.TRIANGLES, 0, 6)
    }
    gl.uniform2f(this.factor, view.scale, view.scale)
    gl.uniform2f(this.offset, view.ox, view.oy)
    gl.uniform4fv(this.color, ink)
    gl.bindBuffer(gl.ARRAY_BUFFER, this.mesh)
    gl.vertexAttribPointer(this.position, 2, gl.FLOAT, false, 0, 0)
    gl.drawArrays(gl.TRIANGLES, 0, this.count)
  }

  sync() {
    this.gl.readPixels(0, 0, 1, 1, this.gl.RGBA, this.gl.UNSIGNED_BYTE, new Uint8Array(4))
  }

  pixels() {
    const { gl, canvas } = this
    const raw = new Uint8Array(canvas.width * canvas.height * 4)
    gl.readPixels(0, 0, canvas.width, canvas.height, gl.RGBA, gl.UNSIGNED_BYTE, raw)
    const flipped = new Uint8Array(raw.length)
    const stride = canvas.width * 4
    for (let y = 0; y < canvas.height; y++) flipped.set(raw.subarray(y * stride, (y + 1) * stride), (canvas.height - 1 - y) * stride)
    return flipped
  }

  dispose() {
    this.gl.deleteBuffer(this.mesh)
    this.gl.deleteBuffer(this.rect)
    this.gl.deleteProgram(this.program)
    this.gl.getExtension('WEBGL_lose_context')?.loseContext()
    this.canvas.remove()
  }
}
