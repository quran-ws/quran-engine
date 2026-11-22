package ws.quran.qvp

/**
 * The styled paths to draw, in draw-list order. [styled] holds pairs (path, colour), as
 * `QvpPage.styledPaths()` returns them. The result holds triples (path, placement, colour), one
 * for each draw of a styled path. It skips a hidden colour. It leaves out a path that the draw
 * list does not hold. It holds a path twice when the draw list holds it twice.
 */
internal fun styledDraws(draws: List<QvpDraw>, styled: IntArray, nPaths: Int): IntArray {
    // A hidden colour has an alpha of 0, so 0 also means "not styled" here.
    val colorOf = IntArray(nPaths)
    var i = 0
    while (i + 1 < styled.size) {
        val path = styled[i]
        if (path in 0 until nPaths) colorOf[path] = styled[i + 1]
        i += 2
    }
    val out = IntArray(draws.size * 3)
    var n = 0
    for (d in draws) {
        if (d.path !in 0 until nPaths) continue
        val color = colorOf[d.path]
        if (color and 0xff == 0) continue
        out[n] = d.path; out[n + 1] = d.placement; out[n + 2] = color
        n += 3
    }
    return out.copyOf(n)
}
