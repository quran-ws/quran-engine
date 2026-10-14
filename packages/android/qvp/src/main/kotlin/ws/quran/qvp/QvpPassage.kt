// A range of complete ayahs, or a surah's basmalah, laid out on rows away from its page, over the
// C ABI in qvp.h. The engine measures and places every word; this file marshals the calls.
// Mirrors packages/ios/QvpKit/Sources/QvpKit/QvpPassage.swift. See docs/API.md.
package ws.quran.qvp

import android.graphics.Canvas
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.RectF

/**
 * A contiguous range of complete ayahs, ready to lay out at any width. It copies what it needs
 * from its pages, so the pages can be closed after it is made. Call [close] when done.
 */
class QvpPassage private constructor(handle: Long, surah: Int, from: Int) : AutoCloseable {
    private var nativeHandle: Long = handle
    private val h: Long get() = nativeHandle.takeIf { it != 0L } ?: error("QvpPassage is closed")

    /** The ayahs in reading order. A basmalah has none. */
    val ayahs: List<QvpPassageText>
    /** The printed line spacing of the passage's first word, in page units. Every word of the
     * passage has that size. */
    val lineSpacing: Float
    /** The last layout. */
    var currentLayout: QvpPassageLayout? = null; private set

    init {
        ayahs = List(QvpNative.passageAyahCount(h)) { i -> QvpPassageText(surah, from + i, QvpNative.passageAyahText(h, i)) }
        lineSpacing = QvpNative.passageLineSpacing(h)
    }

    companion object {
        private const val MAX_U16 = 0xffff

        /** Load the ayahs [from] to [to] of [surah] from the pages that print them, in any order.
         * Throws when an ayah is missing or incomplete, a page is given twice, or the range has more
         * than 4,096 words. */
        fun load(pages: List<QvpPage>, surah: Int, from: Int, to: Int = from): QvpPassage {
            // Every engine call on a closed page traps; the engine checks the range itself.
            require(pages.all { it.isOpen }) { "every page of a passage must be open" }
            require(surah in 1..MAX_U16 && from in 1..MAX_U16 && to in 1..MAX_U16) { "bad passage range $surah:$from-$to" }
            val handle = QvpNative.passageLoad(LongArray(pages.size) { pages[it].handle }, surah, from, to)
            require(handle != 0L) { "qvp_passage_load failed (an ayah missing or incomplete, a page repeated, or over 4096 words)" }
            return QvpPassage(handle, surah, from)
        }

        /** Load the basmalah printed above [surah] from whichever of the pages prints it. It lays out
         * as one piece on one row and has no ayahs and no words. Its line spacing is its page's, so
         * it is sized by the printed line as an ayah is. Throws for surah 1, whose basmalah is its
         * first ayah, for surah 9, which has none, and when no page given prints it. */
        fun basmalah(pages: List<QvpPage>, surah: Int): QvpPassage {
            require(pages.all { it.isOpen }) { "every page of a passage must be open" }
            require(surah in 1..MAX_U16) { "bad surah $surah" }
            val handle = QvpNative.passageLoadBasmalah(LongArray(pages.size) { pages[it].handle }, surah)
            require(handle != 0L) { "qvp_passage_load_basmalah failed (surah 1 or 9, or no page given prints it)" }
            return QvpPassage(handle, surah, 1)
        }
    }

    /** False once [close] has been called. */
    val isOpen: Boolean get() = nativeHandle != 0L
    val isClosed: Boolean get() = nativeHandle == 0L

    /** Lay the passage out. Null when the spec is out of range. The last layout then stays. */
    fun layout(spec: QvpPassageSpec): QvpPassageLayout? {
        if (spec.maxRows < 0) return null
        val v = QvpNative.passageLayout(h, spec.floats()) ?: return null
        val rows = QvpNative.passageRows(h)
        val words = QvpNative.passageWords(h)
        val ayahs = QvpNative.passageAyahs(h)
        val draws = QvpNative.passageDrawList(h)
        val places = QvpNative.passagePlacements(h)
        val isTruncated = v[8] != 0f
        return QvpPassageLayout(
            v[0], v[1], v[2], v[3],
            List(rows.size / 5) { k -> val o = k * 5; QvpPassageRow(rows[o], rows[o + 1], rows[o + 2], rows[o + 3], rows[o + 4]) },
            List(words.size / 13) { k ->
                val o = k * 13
                QvpPassageWord(words[o].toInt(), words[o + 1].toInt(), words[o + 2].toInt(), words[o + 3].toInt(), words[o + 4].toInt(),
                               words[o + 5], words[o + 6], words[o + 7], words[o + 8],
                               words[o + 9], words[o + 10], words[o + 11], words[o + 12])
            },
            List(ayahs.size / 6) { k -> val o = k * 6; QvpPassageAyah(ayahs[o].toInt(), ayahs[o + 1].toInt(), ayahs[o + 2], ayahs[o + 3], ayahs[o + 4], ayahs[o + 5]) },
            List(draws.size / 3) { k -> QvpPassageDraw(draws[k * 3], draws[k * 3 + 1], draws[k * 3 + 2]) },
            List(places.size / 4) { k -> QvpPlacement(places[k * 4], places[k * 4 + 1], places[k * 4 + 2], places[k * 4 + 3]) },
            isTruncated,
            if (isTruncated) RectF(v[9], v[10], v[11], v[12]) else null,
        ).also { currentLayout = it }
    }

    @Synchronized
    override fun close() {
        val handle = nativeHandle
        if (handle != 0L) {
            nativeHandle = 0L
            QvpNative.passageFree(handle)
        }
    }
}

/**
 * Draw the layout on [canvas] with its top-left corner at ([x], [y]), in one colour. [pages] maps a
 * page number to the open page it was loaded from, which holds the outlines. The helper skips a
 * draw whose page is not in the map. The ellipsis after a cut is the host's to draw, at
 * [QvpPassageLayout.ellipsis].
 */
fun QvpPassageLayout.draw(canvas: Canvas, pages: Map<Int, QvpPage>, paint: Paint, x: Float = 0f, y: Float = 0f) {
    val m = Matrix()
    var cur = -1
    canvas.save()
    for (d in draws) {
        val page = pages[d.page] ?: continue
        if (d.placement != cur) {
            // A point p of a path lands at scale · (k · p + d): the placement first, then the scale.
            val q = placements.getOrElse(d.placement) { QvpPlacement.IDENTITY }
            m.setScale(scale * q.kx, scale * q.ky)
            m.postTranslate(x + scale * q.dx, y + scale * q.dy)
            canvas.restore(); canvas.save(); canvas.concat(m); cur = d.placement
        }
        canvas.drawPath(page.path(d.path), paint)
    }
    canvas.restore()
}
