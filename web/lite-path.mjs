// Build a Canvas path without changing the decoded outline or its fill rule.
export function buildPath({ ops, pts }) {
  const path = new Path2D()
  let point = 0
  for (const operation of ops) {
    switch (operation) {
      case 0: path.moveTo(pts[point++], pts[point++]); break
      case 1: path.lineTo(pts[point++], pts[point++]); break
      case 2: path.quadraticCurveTo(pts[point++], pts[point++], pts[point++], pts[point++]); break
      case 3: path.bezierCurveTo(pts[point++], pts[point++], pts[point++], pts[point++], pts[point++], pts[point++]); break
      case 4: path.closePath(); break
      default: throw new Error(`Unknown QVP operation: ${operation}`)
    }
  }
  return path
}
