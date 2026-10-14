package ws.quran.qvp

import android.graphics.RectF

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
    const val AYAH_MARK_SCALE_SCAN = 1.5f
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
                         /** Break the words onto rows of the page's own width; null lays it out as printed. */ val reflow: QvpReflowSpec? = null,
                         /** How big a banner (surah name, basmalah) may get as the reader zooms in, as a multiple of
                          * its printed size. 0 = uncapped (it grows until it fills the row); 1 = held at the print.
                          * It lives here, not on [reflow], because a host that pinches never builds a reflow spec —
                          * the zoom control does, from this one. */
                         val bannerZoom: Float = 0f,
                         /** Draw source-native frames around surah names on the printed page. Reflow stays frameless. */
                         val surahFrames: Boolean = true) {
    internal fun floats(): FloatArray {
        val base = floatArrayOf(viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, if (fillHeight) 1f else 0f, gridLines.toFloat(), cropLeft, cropRight, maxAspectSlack)
        val r = reflow
        // The reflow slots are always written, with the engine's own defaults where there is no
        // reflow spec (zoom 0 = the printed page, 255 = "ask the engine"), so a knob that
        // outlives the reflow — bannerZoom — reaches the engine on a printed page too.
        return base + floatArrayOf(r?.zoom ?: 0f, (r?.fill?.id ?: 255).toFloat(), (r?.breaks?.id ?: 255).toFloat(),
                                   (r?.gaps?.id ?: 1).toFloat(), r?.wordGap ?: 1f, r?.maxStretch ?: 0f, r?.relax ?: -1f, bannerZoom,
                                   if (surahFrames) 1f else 0f)
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
/** A surah heading: the box a frame fills, and the title ink inside it. */
data class QvpSurahHeader(
    val decoration: Int, val surah: Int, val line: Int,
    val x0: Float, val y0: Float, val x1: Float, val y1: Float,
    val titleX0: Float, val titleY0: Float, val titleX1: Float, val titleY1: Float,
)

data class QvpAyahMark(val decoration: Int, val surah: Int, val ayah: Int, val line: Int, val cx: Float, val cy: Float, val r: Float, val ornamentPath: Int, val numeralPath: Int)
data class QvpRosette(val decoration: Int, val surah: Int, val ayah: Int, val juz: Int, val hizb: Int, val nisf: Int, val rubuAlHizb: Int, val rubuAlHizbInHizb: Int)
data class QvpSajdah(val decoration: Int, val surah: Int, val ayah: Int, val signPath: Int)
data class QvpMatch(val word: Int, val index: Int, val isLooseMatch: Boolean, val wordKey: String, val text: String)
data class QvpCropBounds(val x0: Float, val y0: Float, val x1: Float, val y1: Float, val nWords: Int, val ayahMarkDecoration: Int)
data class QvpAtlasSurah(val number: Int, val page: Int, val ayahCount: Int, val place: String, val arabic: String, val latin: String, val english: String)
data class QvpAtlasRubuAlHizb(val rubuAlHizb: Int, val surah: Int, val ayah: Int, val page: Int) { val ayahKey get() = "$surah:$ayah" }

// ── passage ──
/** Where the rows of a passage start. */
enum class QvpPassageAlign(val id: Int) {
    /** At the right edge, as Arabic text starts. */
    RIGHT(0),
    /** In the middle of the width. */
    CENTER(1),
    /** Across the width: every row but the last opens its word gaps until it reaches both edges,
     * never past [QvpPassageSpec.maxStretch]. What is left over is split between the two sides,
     * so the last row, a one-word row and a capped row are centred. */
    JUSTIFIED(2),
}
/** What a layout of a passage asks for. Lengths are layout px. The defaults put the passage on
 * one row at the printed size. */
data class QvpPassageSpec(
    /** The width of the rows. Zero or less puts the whole passage on one row, as wide as it is. */
    val width: Float = 0f,
    /** Layout px per page unit. It shrinks only when one word and its signs are wider than the
     * rows. To size a passage by its printed line, divide by [QvpPassage.lineSpacing]. */
    val scale: Float = 1f,
    /** The distance between two rows' baselines, as a multiple of the printed one. At least 1. */
    val lineSpacing: Float = 1f,
    /** The empty margin on every side. */
    val padding: Float = 2f,
    /** Where each row starts. */
    val align: QvpPassageAlign = QvpPassageAlign.RIGHT,
    /** The most rows to show. Zero shows every row. */
    val maxRows: Int = 0,
    /** After a cut, place the medallion of the passage's last ayah after the ellipsis. */
    val keepAyahMark: Boolean = false,
    /** The width that the host needs for its ellipsis after a cut. */
    val ellipsisWidth: Float = 0f,
    /** How far a justified row's gaps may open, as a multiple of the passage's usual air between
     * two words. A negative value lifts the cap. */
    val maxStretch: Float = QvpDefaults.REFLOW_MAX_STRETCH,
) {
    internal fun floats() = floatArrayOf(width, scale, lineSpacing, padding, align.id.toFloat(), if (keepAyahMark) 1f else 0f,
                                         maxRows.toFloat(), ellipsisWidth, maxStretch)
}
/** One row of a laid-out passage, in layout px: the box of its ink, and the height its words sit on. */
data class QvpPassageRow(val x0: Float, val y0: Float, val x1: Float, val y1: Float, val baseline: Float)
/** One word of a laid-out passage with the signs that go with it, in layout px: its key, the page it
 * is printed on, the row it is on (from 0), the box of the word and its signs, and the box of the word
 * alone ([wordX0] to [wordY1]), which leaves out the signs: the word that closes an ayah leaves its
 * medallion out. */
data class QvpPassageWord(val surah: Int, val ayah: Int, val word: Int, val page: Int, val row: Int,
                          val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                          val wordX0: Float, val wordY0: Float, val wordX1: Float, val wordY1: Float) {
    val wordKey get() = "$surah:$ayah:$word"
}
/** The bounds of one ayah that a layout shows, in layout px: the ayah, and the box of its shown words. */
data class QvpPassageAyah(val surah: Int, val ayah: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float)
/** One ayah of a passage, with its text from the page's own word records. */
data class QvpPassageText(val surah: Int, val ayah: Int, val text: String)
/** One path that a passage layout draws: its page, its index on that page (draw it with that page's
 * [QvpPage.path]), and which of [QvpPassageLayout.placements] it goes under. */
data class QvpPassageDraw(val page: Int, val path: Int, val placement: Int)
/** A laid-out passage. A point (x, y) of a path draws at `scale · (kx · x + dx), scale · (ky · y + dy)`
 * under the path's placement. [width] and [height] include the padding. [lineSpacing] is the distance
 * between two rows' baselines, in page units, before a row moves down for its ink. */
class QvpPassageLayout(
    val width: Float, val height: Float, val scale: Float, val lineSpacing: Float,
    /** The rows, top to bottom. */
    val rows: List<QvpPassageRow>,
    /** The words shown, in reading order. */
    val words: List<QvpPassageWord>,
    /** The ayahs with at least one word shown. */
    val ayahs: List<QvpPassageAyah>,
    /** Every path to draw, in drawing order. */
    val draws: List<QvpPassageDraw>,
    /** What [QvpPassageDraw.placement] indexes. */
    val placements: List<QvpPlacement>,
    /** The layout shows fewer words than the passage has. */
    val isTruncated: Boolean,
    /** Where the host draws its ellipsis after a cut, from the last row's top to its baseline. Null
     * when nothing was cut. */
    val ellipsis: RectF?,
)

// ── dress: another mushaf's ornaments ──
/** What an ornament replaces (QVP_ORNAMENT_*). */
enum class QvpOrnamentKind(val id: Int) { AYAH_MARK(0), SURAH_HEADER(1), PAGE_FRAME(2) }
/** Where an ornament's artwork came from (QVP_LINEAGE_*): a scanned mushaf or a font. */
enum class QvpLineage(val id: Int) { SCAN(0), FONT(1) }
/** What an ornament may be published under. Read it before you publish a page that wears it. */
data class QvpLicence(val id: String, val status: String, val redistributable: Boolean, val attribution: String)
/** One printed colour of an ornament's design. [color] is 0xRRGGBBAA; alpha 0 is a slot the design
 * leaves open. A [stroke] part is the design's line art. */
data class QvpOrnamentPart(val index: Int, val color: Int, val stroke: Boolean, val name: String)

/** Which ornament a dress takes for one kind: an ornament of the set, its index, a style name (the
 * ornament of that kind in that mushaf), or [None]. */
sealed interface QvpOrnamentRef {
    data class Index(val index: Int) : QvpOrnamentRef
    data class Style(val style: String) : QvpOrnamentRef
    /** No ornament of this kind, even when the spec names a style. */
    data object None : QvpOrnamentRef
}

/** One ornament of a set: a medallion, a band or a border, from a mushaf or a font. */
data class QvpOrnament(val index: Int, val kind: QvpOrnamentKind, val lineage: QvpLineage, val style: String, val riwayah: String,
                       val hasSlices: Boolean, val licence: QvpLicence, val parts: List<QvpOrnamentPart>) : QvpOrnamentRef

/** One mushaf or font of a set, with the ornament it has of each kind, or null. */
data class QvpOrnamentStyle(val style: String, val riwayah: String, val lineage: QvpLineage,
                            val ayahMark: QvpOrnament?, val surahHeader: QvpOrnament?, val pageFrame: QvpOrnament?)

/** Repaint one part of one ornament: [color] is 0xRRGGBBAA. */
data class QvpPartColor(val ornament: Int, val part: Int, val color: Int)

/**
 * What a dress asks for. [style] names a mushaf and takes its medallion, band and border together.
 * [ayahMark], [surahHeader] and [pageFrame] override the style for their kind; null keeps the style's.
 * [gap] is the air between the text and the border, and [bandGap] the air between a band and the
 * border, in page units. [ayahMarkScale] sizes the medallion against the printed ring: 0 takes the
 * design's own ([QvpDefaults.AYAH_MARK_SCALE_SCAN] for a scan, 1 for a font). [pageFrameScale]
 * sizes the border against the design's own; the pattern scales as a whole. [lineArt] draws those
 * kinds as their strokes alone. [colors] repaints parts; every part left out keeps the design's own.
 */
data class QvpDressSpec(
    val style: String? = null,
    val ayahMark: QvpOrnamentRef? = null,
    val surahHeader: QvpOrnamentRef? = null,
    val pageFrame: QvpOrnamentRef? = null,
    val gap: Float = 5f,
    val bandGap: Float = 5f,
    val ayahMarkScale: Float = 0f,
    val pageFrameScale: Float = 1f,
    val lineArt: Set<QvpOrnamentKind> = emptySet(),
    val colors: List<QvpPartColor> = emptyList(),
) {
    /** The QVP_LINE_ART_* bits of [lineArt]. */
    internal fun lineArtBits(): Int = lineArt.fold(0) { m, k -> m or (1 shl k.id) }
    internal fun colorTriples(): IntArray? =
        if (colors.isEmpty()) null else IntArray(colors.size * 3) { k -> colors[k / 3].let { c -> when (k % 3) { 0 -> c.ornament; 1 -> c.part; else -> c.color } } }
    companion object {
        /** Every kind as line art. */
        val ALL_LINE_ART: Set<QvpOrnamentKind> = QvpOrnamentKind.entries.toSet()
    }
}

/** A box in page units: x, y, width, height. */
data class QvpViewBox(val x: Float, val y: Float, val width: Float, val height: Float)
/** How far something reaches past each side, in viewport px. */
data class QvpInsets(val left: Float, val top: Float, val right: Float, val bottom: Float) {
    companion object { val ZERO = QvpInsets(0f, 0f, 0f, 0f) }
}
/** [QvpPage.dressView]: the view for a host that scrolls the page, and the dressed box's size. */
data class QvpDressView(val view: QvpView, val width: Float, val height: Float)

/**
 * What a dress put on the page. The three ornament indices are null for a kind left as printed.
 * [viewBox] is the page box the border grew, in page units. [overflow] is how far the border reaches
 * past the laid-out content on each side, in viewport px: fit content plus overflow, or the border is
 * cropped. [revision] counts up on every rebuild (a new dress, and every layout).
 */
data class QvpDressInfo(val ayahMark: Int?, val surahHeader: Int?, val pageFrame: Int?, val ayahMarks: Int, val surahHeaders: Int,
                        val frameRepeats: Int, val frameStretched: Boolean, val draws: Int, val revision: Int,
                        val viewBox: QvpViewBox, val overflow: QvpInsets) {
    internal companion object {
        fun of(v: IntArray): QvpDressInfo {
            val id = { x: Int -> if (x == -1) null else x }
            val f = { k: Int -> Float.fromBits(v[k]) }
            return QvpDressInfo(id(v[0]), id(v[1]), id(v[2]), v[3], v[4], v[5], v[6] != 0, v[7], v[8],
                                QvpViewBox(f(9), f(10), f(11), f(12)), QvpInsets(f(13), f(14), f(15), f(16)))
        }
    }
}
