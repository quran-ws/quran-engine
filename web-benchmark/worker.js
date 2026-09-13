importScripts('./geometry.js')

function distance_sq(x, y, ax, ay, bx, by) {
  const dx = bx - ax, dy = by - ay
  const t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy || 1)))
  return (x - ax - t * dx) ** 2 + (y - ay - t * dy) ** 2
}

function contours_for(page, index, tolerance) {
  const contours = []
  let contour, x = 0, y = 0
  function line_to(nx, ny) {
    if (nx !== x || ny !== y) contour.push([nx, ny, 0])
    x = nx
    y = ny
  }
  function cubic(ax, ay, bx, by, cx, cy, dx, dy, depth = 0) {
    if (depth >= 18 || Math.max(distance_sq(bx, by, ax, ay, dx, dy), distance_sq(cx, cy, ax, ay, dx, dy)) <= tolerance ** 2) {
      line_to(dx, dy)
      return
    }
    const abx = (ax + bx) / 2, aby = (ay + by) / 2
    const bcx = (bx + cx) / 2, bcy = (by + cy) / 2
    const cdx = (cx + dx) / 2, cdy = (cy + dy) / 2
    const abcx = (abx + bcx) / 2, abcy = (aby + bcy) / 2
    const bcdx = (bcx + cdx) / 2, bcdy = (bcy + cdy) / 2
    const mx = (abcx + bcdx) / 2, my = (abcy + bcdy) / 2
    cubic(ax, ay, abx, aby, abcx, abcy, mx, my, depth + 1)
    cubic(mx, my, bcdx, bcdy, cdx, cdy, dx, dy, depth + 1)
  }
  qvp_geometry.walk(page, index, {
    moveTo(nx, ny) { contour = [[nx, ny, 0]]; contours.push(contour); x = nx; y = ny },
    lineTo: line_to,
    quadraticCurveTo(cx, cy, nx, ny) { cubic(x, y, x + (cx - x) * 2 / 3, y + (cy - y) * 2 / 3, nx + (cx - nx) * 2 / 3, ny + (cy - ny) * 2 / 3, nx, ny) },
    bezierCurveTo(bx, by, cx, cy, nx, ny) { cubic(x, y, bx, by, cx, cy, nx, ny) },
    closePath() { if (contour) { x = contour[0][0]; y = contour[0][1] } }
  })
  return contours.filter(c => c.length >= 3)
}

self.onmessage = ({ data }) => {
  try {
    const start = performance.now()
    const page = qvp_geometry.decode(data.buffer)
    const decode_ms = performance.now() - start
    let triangles = new Float32Array(0)
    let library_ms = 0, mesh_ms = 0
    if (data.mesh) {
      const library_start = performance.now()
      if (!self.libtess) importScripts('./node_modules/libtess/libtess.min.js')
      library_ms = performance.now() - library_start
      const mesh_start = performance.now()
      const output = []
      const tess = new libtess.GluTesselator()
      tess.gluTessNormal(0, 0, 1)
      tess.gluTessCallback(libtess.gluEnum.GLU_TESS_VERTEX_DATA, vertex => output.push(vertex[0], vertex[1]))
      tess.gluTessCallback(libtess.gluEnum.GLU_TESS_COMBINE, coords => coords)
      tess.gluTessCallback(libtess.gluEnum.GLU_TESS_EDGE_FLAG, () => {})
      tess.gluTessCallback(libtess.gluEnum.GLU_TESS_BEGIN, type => {
        if (type !== libtess.primitiveType.GL_TRIANGLES) throw new Error('Expected triangles')
      })
      tess.gluTessCallback(libtess.gluEnum.GLU_TESS_ERROR, code => { throw new Error(`Tessellation error ${code}`) })
      for (let i = 0; i < page.n_paths; i++) {
        tess.gluTessProperty(libtess.gluEnum.GLU_TESS_WINDING_RULE,
          page.table[i * 8 + 4] & 0x1000000 ? libtess.windingRule.GLU_TESS_WINDING_ODD : libtess.windingRule.GLU_TESS_WINDING_NONZERO)
        tess.gluTessBeginPolygon(null)
        for (const contour of contours_for(page, i, data.tolerance)) {
          tess.gluTessBeginContour()
          for (const vertex of contour) tess.gluTessVertex(vertex, vertex)
          tess.gluTessEndContour()
        }
        tess.gluTessEndPolygon()
      }
      tess.gluDeleteTess()
      triangles = new Float32Array(output)
      if (triangles.length % 6 || triangles.some(v => !Number.isFinite(v))) throw new Error('Invalid mesh')
      mesh_ms = performance.now() - mesh_start
    }
    self.postMessage({ buffer: data.buffer, triangles: triangles.buffer, decode_ms, library_ms, mesh_ms }, [data.buffer, triangles.buffer])
  } catch (error) {
    self.postMessage({ error: error.stack })
  }
}
