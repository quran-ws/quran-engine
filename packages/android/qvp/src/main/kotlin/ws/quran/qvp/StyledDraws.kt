package ws.quran.qvp

/**
 * The styled paths to draw, in draw-list order. [styled] holds pairs (path, colour), as
 * `QvpPage.styledPaths()` returns them. The result holds triples (path, placement, colour), one
 * for each draw of a styled path. It skips a hidden colour. It leaves out a path that the draw
 * list does not hold. It holds a path twice when the draw list holds it twice.
 */
internal fun styledDraws(draws: List<QvpDraw>, styled: IntArray, nPaths: Int): IntArray =
    StyledDraws(draws, nPaths).also { it.update(styled) }.let { it.triples.copyOf(it.size) }

/**
 * [styledDraws] for one draw list, with its buffers kept: a frame of a fade gives new styled
 * paths, and the view asks again with no allocation. After [update], the first [size] ints of
 * [triples] hold the result.
 */
internal class StyledDraws(private val draws: List<QvpDraw>, private val nPaths: Int) {
    // A hidden colour has an alpha of 0, so 0 also means "not styled" here.
    private val colorOf = IntArray(nPaths)
    val triples = IntArray(draws.size * 3)
    var size = 0; private set

    /** Fill [triples] for the styled paths [styled], pairs (path, colour). */
    fun update(styled: IntArray) {
        colorOf.fill(0)
        var i = 0
        while (i + 1 < styled.size) {
            val path = styled[i]
            if (path in 0 until nPaths) colorOf[path] = styled[i + 1]
            i += 2
        }
        var n = 0
        for (k in draws.indices) {
            val d = draws[k]
            if (d.path !in 0 until nPaths) continue
            val color = colorOf[d.path]
            if (color and 0xff == 0) continue
            triples[n] = d.path; triples[n + 1] = d.placement; triples[n + 2] = color
            n += 3
        }
        size = n
    }
}
