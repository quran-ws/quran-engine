package ws.quran.qvp

/** Constants mirroring qvp.h. Colours everywhere are 0xRRGGBBAA (see [QvpColor]). */
object QvpKind { const val BODY = 0; const val MARK = 1; const val AYAH_NUMBER = 2; const val AYAH_MARK_ORNAMENT = 3; const val HEADER_INK = 4; const val ORNAMENT = 5; const val PAGE_NUMBER = 6; const val RUNNING_HEAD = 7; const val OTHER = 255 }
object QvpFamily { const val NONE = 0; const val DIACRITIC = 1; const val TANWIN = 2; const val DOTS = 3; const val WAQF = 4; const val SIFR = 5; const val SAJDAH = 6; const val READING_SIGN = 7 }
object QvpCategory { const val NONE = 0; const val HARAKAH = 1; const val TANWIN = 2; const val LETTER_DOT = 3; const val ORTHOGRAPHIC = 4; const val DABT = 5; const val WAQF = 6; const val READING_SIGN = 7; const val STANDALONE = 8 }
object QvpDecorationKind { const val AYAH_MARK = 0; const val SURAH_NAME = 1; const val BASMALAH = 2; const val DIVISION_MARK = 3; const val SAJDAH_MARK = 4; const val PAGE_NUMBER = 5; const val RUNNING_HEAD = 6; const val OTHER = 255 }
object QvpLayer { const val BASE = 0; const val THEME = 10; const val HIGHLIGHT = 50; const val SELECTION = 60; const val TOP = 100 }
enum class Form(val id: Int) { RASM_UTHMANI(0), RASM_IMLAI(1), QPC(2), RASM(3), SEARCH(4) }
enum class SearchMode(val id: Int) { INCLUDES(0), EXACT(1), PREFIX(2) }
enum class HighlightMode(val id: Int) { INK(0), BAND(1), BOTH(2) }
enum class BandHeight(val id: Int) { LINE_SPACING(0), INK(1) }
enum class MaskMode(val id: Int) { HIDE(0), BLOCK(1), BLUR(2) }
enum class Division(val id: Int) { JUZ(0), HIZB(1), NISF(2), RUBU_AL_HIZB(3) }

/** The defaults every wrapper shares (QVP_DEFAULT_* in qvp.h; the parity check compares them). Colours 0xRRGGBBAA. */
object QvpDefaults {
    const val INK = 0x231f20ff.toInt(); const val HIGHLIGHT_INK = 0x1a73e8ff.toInt(); const val HIGHLIGHT_BAND = 0xd6a3264d.toInt()
    const val HIGHLIGHT_PAD_X = 1.2f; const val HIGHLIGHT_PAD_Y = 0f; const val HIGHLIGHT_SEAM = 0.25f
    const val SELECTION_BAND = 0x2d6fd640; const val GAP_BIAS = 0.6f; const val TAP_DISTANCE = 6f
    const val GRID_LINES = 15; const val ASPECT_SLACK = 1.15f
    const val MASK_BLOCK = 0xd9d4c8ff.toInt(); const val MASK_PAD = 0.6f; const val MASK_RADIUS = 0.8f
    const val REVEAL_LIT = 1; const val REVEAL_GREY = 0xc9c4b8ff.toInt(); const val CROP_PAD = 2f
    // the reader's pinch and swipe: the engine's own limits, which every platform holds to
    const val MIN_ZOOM = 0.5f; const val MAX_ZOOM = 12f; const val ZOOMED_THRESHOLD = 1.02f
    const val SWIPE_AXIS_RATIO = 1.5f; const val SWIPE_DISTANCE = 40f; const val SWIPE_VELOCITY = 500f
    const val ZOOM_SNAP_HYSTERESIS = 0.03f; const val ZOOM_QUANTUM = 0.01f
    const val REFLOW_RELAX = 0.5f; const val REFLOW_MAX_STRETCH = 2f
}

/** A mark id by its name, from the engine (255 = unknown). */
fun markId(name: String): Int = QvpEngine.markFromName(name)

/** 0xRRGGBBAA ↔ Android ARGB. */
object QvpColor {
    fun argb(rgba: Int): Int = (rgba shl 24) or (rgba ushr 8)
    fun rgba(argb: Int): Int = (argb shl 8) or (argb ushr 24)
    /** '#rrggbb' or '#rrggbbaa' → 0xRRGGBBAA */
    fun parse(hex: String, alpha: Float = 1f): Int {
        var h = hex.trim().removePrefix("#")
        if (h.length == 3) h = h.map { "$it$it" }.joinToString("")
        if (h.length == 6) h += "%02x".format((alpha * 255).toInt().coerceIn(0, 255))
        return h.toLong(16).toInt()
    }
    fun withAlpha(rgba: Int, alpha: Float): Int = (rgba and 0xffffff00.toInt()) or (alpha * 255).toInt().coerceIn(0, 255)
}

/** What a style rule applies to (mirrors Sel in web/qvp.js). */
class Selector private constructor(internal val arr: IntArray) {
    companion object {
        fun page() = Selector(intArrayOf(0, 0, 0, 0))
        fun path(i: Int) = Selector(intArrayOf(1, i, 0, 0))
        fun wordPath(w: Int, nth: Int) = Selector(intArrayOf(2, w, nth, 0))
        /** nth mark of the word (0-based, marks only) */
        fun wordMark(w: Int, nth: Int) = Selector(intArrayOf(3, w, nth, 0))
        fun wordMarkNamed(w: Int, mark: String, nth: Int = 0) = Selector(intArrayOf(4, w, markId(mark), nth))
        fun wordBody(w: Int) = Selector(intArrayOf(5, w, 0, 0))
        fun wordMarks(w: Int) = Selector(intArrayOf(6, w, 0, 0))
        fun word(w: Int) = Selector(intArrayOf(7, w, 0, 0))
        fun ayah(s: Int, a: Int) = Selector(intArrayOf(8, s, a, 0))
        fun line(n: Int) = Selector(intArrayOf(9, n, 0, 0))
        fun mark(mark: String) = Selector(intArrayOf(10, markId(mark), 0, 0))
        fun mark(id: Int) = Selector(intArrayOf(10, id, 0, 0))
        fun category(c: Int) = Selector(intArrayOf(11, c, 0, 0))
        fun family(f: Int) = Selector(intArrayOf(12, f, 0, 0))
        fun kind(k: Int) = Selector(intArrayOf(13, k, 0, 0))
        fun decoration(k: Int) = Selector(intArrayOf(14, k, 0, 0))
        fun decorationIndex(i: Int) = Selector(intArrayOf(15, i, 0, 0))
    }
}

/** What resolves to a word list (mirrors T in web/qvp.js). Strings: "page", "2:255", "2:255:3", "2:255-257", "line:7", "surah:2". */
class Target private constructor(internal val arr: IntArray) {
    companion object {
        fun page() = Target(intArrayOf(0, 0, 0, 0))
        fun word(i: Int) = Target(intArrayOf(1, i, 0, 0))
        fun words(ws: IntArray) = Target(intArrayOf(2, 0, 0, 0) + ws)
        fun words(ws: List<Int>) = words(ws.toIntArray())
        fun ayah(s: Int, a: Int) = Target(intArrayOf(3, s, a, 0))
        fun ayahRange(s: Int, a: Int, b: Int) = Target(intArrayOf(4, s, a, b))
        fun line(n: Int) = Target(intArrayOf(5, n, 0, 0))
        fun surah(s: Int) = Target(intArrayOf(6, s, 0, 0))
        fun range(a: Int, b: Int) = Target(intArrayOf(7, a, b, 0))
        /** parse a string key; word keys need the page to resolve, see [QvpPage.target] */
        fun parse(s: String, page: QvpPage? = null): Target {
            if (s == "page") return page()
            Regex("^line:(\\d+)$").find(s)?.let { return line(it.groupValues[1].toInt()) }
            Regex("^surah:(\\d+)$").find(s)?.let { return surah(it.groupValues[1].toInt()) }
            Regex("^(\\d+):(\\d+)-(\\d+)$").find(s)?.let { val g = it.groupValues; return ayahRange(g[1].toInt(), g[2].toInt(), g[3].toInt()) }
            Regex("^(\\d+):(\\d+):(\\d+)$").find(s)?.let { val g = it.groupValues; val i = page?.findWord(g[1].toInt(), g[2].toInt(), g[3].toInt()) ?: -1; return if (i >= 0) word(i) else words(IntArray(0)) }
            Regex("^(\\d+):(\\d+)$").find(s)?.let { val g = it.groupValues; return ayah(g[1].toInt(), g[2].toInt()) }
            throw IllegalArgumentException("bad target $s")
        }
    }
}

data class QvpWord(val index: Int, val surah: Int, val ayah: Int, val word: Int, val line: Int, val ayahIndex: Int, val lineIndex: Int,
                   val x0: Float, val y0: Float, val x1: Float, val y1: Float, val text: String, val firstPath: Int, val nPaths: Int) {
    val wordKey get() = "$surah:$ayah:$word"
    val ayahKey get() = "$surah:$ayah"
}
data class QvpAyah(val index: Int, val surah: Int, val ayah: Int, val fragment: Int, val fragments: Int, val flags: Int, val rubuAlHizb: Int, val firstWord: Int, val nWords: Int,
                   val ayahMarkDecoration: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float)
data class QvpLine(val index: Int, val lineNumber: Int, val isHeader: Boolean, val firstWord: Int, val nWords: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                   val bandY0: Float, val bandY1: Float, val centre: Float)
data class QvpDecoration(val index: Int, val decoration: Int, val surah: Int, val ayah: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                         val text: String, val firstPath: Int, val nPaths: Int)
/** Indices are -1 when absent. */
/** The one hit shape for every hit test; the exact variants report distance 0 and `isExact`. Indices are -1 when absent. */
data class QvpHit(val word: Int, val path: Int, val decoration: Int, val line: Int, val distance: Float, val isExact: Boolean)
data class QvpHitOptions(val maxDistance: Float = 0f, val gapBias: Float = QvpDefaults.GAP_BIAS, val preferExact: Boolean = true)
data class QvpBox(val id: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float, val color: Int, val radius: Float)
data class QvpHitArea(val word: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float, val inkX0: Float, val inkY0: Float, val inkX1: Float, val inkY1: Float)
data class QvpLineBand(val line: Int, val lineNumber: Int, val y0: Float, val y1: Float, val mid: Float, val inkY0: Float, val inkY1: Float)
/** Spacing only opens up: `lineSpacing` < 1 is clamped by the engine. `gridLines` 0 = the page's own grid. */
/** How a reflowed row fills the width, how the words are broken onto rows, and where the air
 * between two words comes from. */
enum class QvpFill(val id: Int) { RAGGED(0), JUSTIFIED(1), CENTRED(2) }
enum class QvpBreaks(val id: Int) { GREEDY(0), EVEN(1), FITTED(2) }
enum class QvpGapMode(val id: Int) { PRINTED(0), UNIFORM(1) }

/** Ask for a reflowed page: the same page width with bigger ink, so fewer words fit a row and the
 * rest move down. [zoom] is the ink size as a multiple of fit-to-width; 1 is the printed page.
 * Every other field has a default the engine picked and a reader never needs to change. */
data class QvpReflowSpec(val zoom: Float, val fill: QvpFill = QvpFill.CENTRED, val breaks: QvpBreaks = QvpBreaks.FITTED,
                         val gaps: QvpGapMode = QvpGapMode.UNIFORM, val wordGap: Float = 1f,
                         val relax: Float = QvpDefaults.REFLOW_RELAX, val maxStretch: Float = QvpDefaults.REFLOW_MAX_STRETCH)

data class QvpLayoutSpec(val viewportW: Float, val viewportH: Float, val padTop: Float = 0f, val padBottom: Float = 0f, val padLeft: Float = 0f, val padRight: Float = 0f,
                         val lineSpacing: Float = 1f, val fillHeight: Boolean = false, val gridLines: Int = 0,
                         /** Printed side margins to cut, page units (0 = keep). */ val cropLeft: Float = 0f, val cropRight: Float = 0f,
                         /** The content is never wider than viewportH·pageW/pageH·slack (0 = no bound). */ val maxAspectSlack: Float = 0f,
                         /** Break the words onto rows of the page's own width; null lays it out as printed. */ val reflow: QvpReflowSpec? = null) {
    internal fun floats(): FloatArray {
        val base = floatArrayOf(viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, if (fillHeight) 1f else 0f, gridLines.toFloat(), cropLeft, cropRight, maxAspectSlack)
        val r = reflow ?: return base
        return base + floatArrayOf(r.zoom, r.fill.id.toFloat(), r.breaks.id.toFloat(), r.gaps.id.toFloat(), r.wordGap, r.maxStretch, r.relax)
    }
}

/** What a pinch does to the page. */
enum class QvpZoomMode(val id: Int) { STEPPED(0), CONTINUOUS(1), MAGNIFY(2) }
/** Where the reader's zoom control stands. The default is what a page opens on: stepped, printed. */
data class QvpZoom(val mode: QvpZoomMode = QvpZoomMode.STEPPED, val step: Int = 0, val zoom: Float = 1f) {
    internal fun floats() = floatArrayOf(mode.id.toFloat(), step.toFloat(), zoom)
    internal companion object {
        fun of(v: FloatArray) = QvpZoom(QvpZoomMode.entries.firstOrNull { it.id == v[0].toInt() } ?: QvpZoomMode.STEPPED, v[1].toInt(), v[2])
    }
}
/** Pan and zoom over a laid-out page: a point p draws at offset + scale·p. */
data class QvpView(val scale: Float = 1f, val offsetX: Float = 0f, val offsetY: Float = 0f) {
    internal fun floats() = floatArrayOf(scale, offsetX, offsetY)
    internal companion object { fun of(v: FloatArray) = QvpView(v[0], v[1], v[2]) }
}
/** What a gesture produced: the control to keep, the view to draw with, and whether the page was
 * laid out again under it. */
data class QvpZoomChange(val zoom: QvpZoom, val view: QvpView, val relaid: Boolean)
/** What a sideways drag on a page means. */
enum class QvpSideways { PAN, TURN_PAGE }
/** One drawing of one path: the path, and which of the layout's placements it goes under. */
data class QvpDraw(val path: Int, val placement: Int)
/** Where one group of paths is placed: a point p is drawn at (kx·p.x + dx, ky·p.y + dy), page units. */
data class QvpPlacement(val dx: Float, val dy: Float, val kx: Float, val ky: Float) {
    companion object { val IDENTITY = QvpPlacement(0f, 0f, 1f, 1f) }
}
/** The grid a page is laid out inside: the mushaf's line count and the printed line spacing (page units). */
data class QvpGrid(val lines: Int, val lineSpacing: Float)
/** Page → viewport: viewX = offsetX + x*scale ; viewY = offsetY + (y + lineDy[line])*scale. [fitScale], [fitX], [fitY] show the
 *  whole content in the viewport (shrink to height, never enlarge, centred): the host's pan and zoom go on top. */
class QvpLayout(val scale: Float, val offsetX: Float, val offsetY: Float, val contentW: Float, val contentH: Float, val lineSpacing: Float, val lineDy: FloatArray, val slotTop: FloatArray, val slotBottom: FloatArray,
                val fitScale: Float = 1f, val fitX: Float = 0f, val fitY: Float = 0f,
                /** True when this layout broke the words onto rows of its own, and how many it made. */
                val reflowed: Boolean = false, val rows: Int = 0)
data class QvpHighlightStyle(val mode: HighlightMode = HighlightMode.BAND, val ink: Int = QvpDefaults.HIGHLIGHT_INK, val band: Int = QvpDefaults.HIGHLIGHT_BAND, val height: BandHeight = BandHeight.LINE_SPACING,
                             val padX: Float = QvpDefaults.HIGHLIGHT_PAD_X, val padY: Float = QvpDefaults.HIGHLIGHT_PAD_Y, val radius: Float = 0f, val seam: Float = QvpDefaults.HIGHLIGHT_SEAM, val transitionMs: Int = 0, val layer: Int = QvpLayer.HIGHLIGHT) {
    internal fun ints() = intArrayOf(mode.id, height.id, ink, band, transitionMs, layer)
    internal fun floats() = floatArrayOf(padX, padY, radius, seam)
}
/** Colours with alpha 0 (or null) leave that colour alone. */
data class QvpTheme(val ink: Int? = null, val diacritics: Int? = null, val dots: Int? = null, val waqf: Int? = null, val sifr: Int? = null, val ayahMark: Int? = null, val numeral: Int? = null,
                    val headers: Int? = null, val marks: Map<String, Int> = emptyMap(), val transitionMs: Int = 0)
data class QvpSurah(val number: Int, val ayahCount: Int, val hasBanner: Boolean, val hasBasmalah: Boolean, val place: String, val bannerDecoration: Int, val arabic: String, val latin: String, val english: String)
data class QvpDivision(val division: Division, val number: Int, val surah: Int, val ayah: Int, val line: Int, val ayahIndex: Int)
data class QvpAyahMark(val decoration: Int, val surah: Int, val ayah: Int, val line: Int, val cx: Float, val cy: Float, val r: Float, val ornamentPath: Int, val numeralPath: Int)
data class QvpRosette(val decoration: Int, val surah: Int, val ayah: Int, val juz: Int, val hizb: Int, val nisf: Int, val rubuAlHizb: Int, val rubuAlHizbInHizb: Int)
data class QvpSajdah(val decoration: Int, val surah: Int, val ayah: Int, val signPath: Int)
data class QvpMatch(val word: Int, val index: Int, val isLooseMatch: Boolean, val wordKey: String, val text: String)
data class QvpCropBounds(val x0: Float, val y0: Float, val x1: Float, val y1: Float, val nWords: Int, val ayahMarkDecoration: Int)
data class QvpAtlasSurah(val number: Int, val page: Int, val ayahCount: Int, val place: String, val arabic: String, val latin: String, val english: String)
data class QvpAtlasRubuAlHizb(val rubuAlHizb: Int, val surah: Int, val ayah: Int, val page: Int) { val ayahKey get() = "$surah:$ayah" }
