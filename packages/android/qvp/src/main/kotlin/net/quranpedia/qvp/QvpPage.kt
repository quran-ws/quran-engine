package net.quranpedia.qvp

import android.graphics.Path

/** Selector kinds for [QvpPage.style]. */
object QvpSel { const val PATH = 0; const val WORD = 1; const val AYAH = 2; const val LINE = 3; const val MARK = 4; const val FAMILY = 5; const val KIND = 6; const val DECO = 7 }
object QvpKind { const val BODY = 0; const val MARK = 1; const val AYAH_NUMBER = 2; const val AYAH_ORNAMENT = 3; const val HEADER_INK = 4; const val OTHER = 255 }
object QvpFamily { const val NONE = 0; const val DIACRITIC = 1; const val TANWEEN = 2; const val DOTS = 3; const val WAQF = 4; const val SIFR = 5; const val SAJDAH = 6; const val READING_SIGN = 7 }
object QvpDeco { const val AYAH_MARKER = 0; const val SURAH_NAME = 1; const val BASMALAH = 2; const val HIZB_MARK = 3; const val SAJDAH_MARK = 4 }

data class QvpWord(val idx: Int, val sura: Int, val ayah: Int, val word: Int, val line: Int, val ayahIdx: Int,
                   val x0: Float, val y0: Float, val x1: Float, val y1: Float, val text: String, val firstPath: Int, val nPaths: Int)
data class QvpAyah(val idx: Int, val sura: Int, val ayah: Int, val part: Int, val parts: Int, val flags: Int, val firstWord: Int, val nWords: Int,
                   val markerDeco: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float)
data class QvpLine(val idx: Int, val lineNo: Int, val firstWord: Int, val nWords: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float)
data class QvpDecoration(val idx: Int, val kind: Int, val sura: Int, val ayah: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                         val text: String, val firstPath: Int, val nPaths: Int)
/** Indices are -1 when absent. */
data class QvpHit(val word: Int, val path: Int, val deco: Int)

/** All lengths in viewport px. lineSpacing multiplies the printed pitch; fillHeight spreads the nominal grid. */
data class QvpLayoutSpec(val viewportW: Float, val viewportH: Float, val padTop: Float = 0f, val padBottom: Float = 0f,
                         val padLeft: Float = 0f, val padRight: Float = 0f, val lineSpacing: Float = 1f,
                         val fillHeight: Boolean = false, val nominalLines: Int = 15)
/** Page → viewport: vx = ox + x*scale ; vy = oy + (y + lineDy[line])*scale. */
class QvpLayout(val scale: Float, val ox: Float, val oy: Float, val contentH: Float, val pitch: Float,
                val lineDy: FloatArray, val slotTop: FloatArray, val slotBottom: FloatArray) {
    var contentW: Float = 0f
}

/**
 * One loaded page. Geometry is copied out of the engine once; hit-testing, layout and
 * styling stay inside the engine. Call [close] when done.
 */
class QvpPage(bytes: ByteArray) : AutoCloseable {
    private var h: Long = QvpNative.pageLoad(bytes)
    init { require(h != 0L) { "qvp_page_load failed (not a QVP1 file?)" } }

    val width: Float; val height: Float; val pageNo: Int
    val nLines: Int; val nAyahs: Int; val nWords: Int; val nPaths: Int; val nDecos: Int
    val ops: ByteArray; val pts: FloatArray
    /** stride 8 per path: opStart, opCount, ptStart, ptCount, flags, word, line, reserved */
    val table: IntArray
    val words: List<QvpWord>; val ayahs: List<QvpAyah>; val lines: List<QvpLine>; val decos: List<QvpDecoration>
    var currentLayout: QvpLayout? = null; private set
    private var paths: Array<Path>? = null

    init {
        val i = QvpNative.pageInfo(h)
        width = i[0]; height = i[1]; pageNo = i[2].toInt(); nLines = i[3].toInt(); nAyahs = i[4].toInt()
        nWords = i[5].toInt(); nPaths = i[6].toInt(); nDecos = i[7].toInt()
        ops = QvpNative.geomOps(h); pts = QvpNative.geomPts(h); table = QvpNative.geomTable(h)
        words = List(nWords) { k -> val v = QvpNative.wordInfo(h, k)!!; QvpWord(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5], v[6], v[7], v[8], QvpNative.wordText(h, k) ?: "", v[9].toInt(), v[10].toInt()) }
        ayahs = List(nAyahs) { k -> val v = QvpNative.ayahInfo(h, k)!!; QvpAyah(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5].toInt(), v[6].toInt(), v[7].toInt(), v[8], v[9], v[10], v[11]) }
        lines = List(nLines) { k -> val v = QvpNative.lineInfo(h, k)!!; QvpLine(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3], v[4], v[5], v[6]) }
        decos = List(nDecos) { k -> val v = QvpNative.decoInfo(h, k)!!; QvpDecoration(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3], v[4], v[5], v[6], QvpNative.decoText(h, k) ?: "", v[7].toInt(), v[8].toInt()) }
    }

    fun pathFlags(i: Int) = table[i * 8 + 4]
    fun pathKind(i: Int) = pathFlags(i) and 0xff
    fun pathMark(i: Int) = (pathFlags(i) shr 8) and 0xff
    fun pathFamily(i: Int) = (pathFlags(i) shr 16) and 0xff
    fun pathEvenOdd(i: Int) = ((pathFlags(i) ushr 24) and 1) == 1
    fun pathWord(i: Int) = table[i * 8 + 5]
    fun pathLine(i: Int) = table[i * 8 + 6]

    /** android.graphics.Path per engine path, in page units, built once. */
    fun buildPaths(): Array<Path> {
        paths?.let { return it }
        val out = Array(nPaths) { Path() }
        for (i in 0 until nPaths) {
            val p = out[i]
            var o = table[i * 8]; val oe = o + table[i * 8 + 1]; var k = table[i * 8 + 2]
            while (o < oe) {
                when (ops[o].toInt()) {
                    0 -> { p.moveTo(pts[k], pts[k + 1]); k += 2 }
                    1 -> { p.lineTo(pts[k], pts[k + 1]); k += 2 }
                    2 -> { p.quadTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3]); k += 4 }
                    3 -> { p.cubicTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3], pts[k + 4], pts[k + 5]); k += 6 }
                    4 -> p.close()
                }
                o++
            }
            p.fillType = if (pathEvenOdd(i)) Path.FillType.EVEN_ODD else Path.FillType.WINDING
        }
        paths = out
        return out
    }

    fun hitTest(x: Float, y: Float): QvpHit? = QvpNative.hitTest(h, x, y)?.let { QvpHit(it[0], it[1], it[2]) }
    /** Viewport px through [currentLayout]. */
    fun hitTestView(vx: Float, vy: Float): QvpHit? = QvpNative.hitTestView(h, vx, vy)?.let { QvpHit(it[0], it[1], it[2]) }
    fun findWord(sura: Int, ayah: Int, word: Int): Int = QvpNative.findWord(h, sura, ayah, word)

    fun layout(spec: QvpLayoutSpec): QvpLayout {
        val v = QvpNative.layout(h, spec.viewportW, spec.viewportH, spec.padTop, spec.padBottom, spec.padLeft, spec.padRight, spec.lineSpacing, spec.fillHeight, spec.nominalLines)
        val n = v[5].toInt()
        val l = QvpLayout(v[0], v[1], v[2], v[3], v[4], FloatArray(n) { v[6 + it * 3] }, FloatArray(n) { v[7 + it * 3] }, FloatArray(n) { v[8 + it * 3] })
        l.contentW = spec.viewportW
        currentLayout = l
        return l
    }

    /** rgba is 0xRRGGBBAA; alpha 0 hides. */
    fun style(sel: Int, a: Int, b: Int, c: Int, rgba: Int, on: Boolean = true) { QvpNative.style(h, sel, a, b, c, rgba, on) }
    fun styleWord(s: Int, a: Int, w: Int, rgba: Int, on: Boolean = true) = style(QvpSel.WORD, s, a, w, rgba, on)
    fun styleAyah(s: Int, a: Int, rgba: Int, on: Boolean = true) = style(QvpSel.AYAH, s, a, 0, rgba, on)
    fun styleLine(n: Int, rgba: Int, on: Boolean = true) = style(QvpSel.LINE, n, 0, 0, rgba, on)
    fun styleMark(m: Int, rgba: Int, on: Boolean = true) = style(QvpSel.MARK, m, 0, 0, rgba, on)
    fun styleFamily(f: Int, rgba: Int, on: Boolean = true) = style(QvpSel.FAMILY, f, 0, 0, rgba, on)
    fun styleKind(k: Int, rgba: Int, on: Boolean = true) = style(QvpSel.KIND, k, 0, 0, rgba, on)
    fun styleDeco(k: Int, rgba: Int, on: Boolean = true) = style(QvpSel.DECO, k, 0, 0, rgba, on)
    fun stylePath(i: Int, rgba: Int, on: Boolean = true) = style(QvpSel.PATH, i, 0, 0, rgba, on)
    fun styleClear() = QvpNative.styleClear(h)
    fun styleDefault(rgba: Int) = QvpNative.styleDefault(h, rgba)
    /** Full display list: 0xRRGGBBAA per path. */
    fun paint(): IntArray = QvpNative.paint(h)
    /** Overlay list: flat pairs [pathIdx, rgba, ...]. */
    fun styled(): IntArray = QvpNative.styled(h)

    override fun close() { if (h != 0L) { QvpNative.pageFree(h); h = 0 } }

    companion object {
        fun markName(m: Int) = QvpNative.markName(m)
        fun familyName(f: Int) = QvpNative.familyName(f)
        fun kindName(k: Int) = QvpNative.kindName(k)
        fun engineVersion() = QvpNative.version()
        /** 0xRRGGBBAA → Android ARGB. */
        fun argb(rgba: Int): Int = (rgba shl 24) or (rgba ushr 8)
        /** Android ARGB → 0xRRGGBBAA. */
        fun rgba(argb: Int): Int = (argb shl 8) or (argb ushr 24)
    }
}
