globalThis.qvp_geometry = {
  decode(buffer) {
    const header = new DataView(buffer)
    if (header.getUint32(0, true) !== 0x31425651) throw new Error('Expected QVB1 geometry export')
    const page = {
      width: header.getFloat32(4, true), height: header.getFloat32(8, true),
      n_paths: header.getUint32(12, true), n_ops: header.getUint32(16, true),
      n_pts: header.getUint32(20, true), n_words: header.getUint32(24, true),
      number: header.getUint32(28, true), buffer
    }
    let offset = 32
    page.table = new Uint32Array(buffer, offset, page.n_paths * 8)
    offset += page.table.byteLength
    page.ops = new Uint8Array(buffer, offset, page.n_ops)
    offset += (page.n_ops + 3) & ~3
    page.pts = new Float32Array(buffer, offset, page.n_pts)
    offset += page.pts.byteLength
    page.words = new Float32Array(buffer, offset, page.n_words * 4)
    if (offset + page.words.byteLength !== buffer.byteLength) throw new Error('Invalid geometry length')
    return page
  },

  walk(page, index, target) {
    const start = page.table[index * 8]
    const end = start + page.table[index * 8 + 1]
    let p = page.table[index * 8 + 2]
    const pts = page.pts
    for (let o = start; o < end; o++) {
      switch (page.ops[o]) {
        case 0: target.moveTo(pts[p++], pts[p++]); break
        case 1: target.lineTo(pts[p++], pts[p++]); break
        case 2: target.quadraticCurveTo(pts[p++], pts[p++], pts[p++], pts[p++]); break
        case 3: target.bezierCurveTo(pts[p++], pts[p++], pts[p++], pts[p++], pts[p++], pts[p++]); break
        case 4: target.closePath(); break
        default: throw new Error(`Unknown path operation ${page.ops[o]}`)
      }
    }
  },

  paths(page) {
    return Array.from({ length: page.n_paths }, (_, i) => {
      const path = new Path2D()
      this.walk(page, i, path)
      return { path, rule: page.table[i * 8 + 4] & 0x1000000 ? 'evenodd' : 'nonzero' }
    })
  }
}
