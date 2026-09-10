package net.quranpedia.qvp

/** Constants mirroring qvp.h. Colours everywhere are 0xRRGGBBAA (see [QvpColor]). */
object QvpKind { const val BODY = 0; const val MARK = 1; const val AYAH_NUMBER = 2; const val AYAH_MARK_ORNAMENT = 3; const val HEADER_INK = 4; const val ORNAMENT = 5; const val PAGE_NUMBER = 6; const val RUNNING_HEAD = 7; const val OTHER = 255 }
object QvpFamily { const val NONE = 0; const val DIACRITIC = 1; const val TANWIN = 2; const val DOTS = 3; const val WAQF = 4; const val SIFR = 5; const val SAJDAH = 6; const val READING_SIGN = 7 }
object QvpCategory { const val NONE = 0; const val HARAKAH = 1; const val TANWIN = 2; const val LETTER_DOT = 3; const val ORTHOGRAPHIC = 4; const val DABT = 5; const val WAQF = 6; const val READING_SIGN = 7; const val STANDALONE = 8 }
object QvpDeco { const val AYAH_MARK = 0; const val SURAH_NAME = 1; const val BASMALAH = 2; const val DIVISION_MARK = 3; const val SAJDAH_MARK = 4; const val PAGE_NUMBER = 5; const val RUNNING_HEAD = 6; const val OTHER = 255 }
object QvpLayer { const val BASE = 0; const val THEME = 10; const val HIGHLIGHT = 50; const val SELECTION = 60; const val TOP = 100 }
enum class Form(val id: Int) { RASM_UTHMANI(0), RASM_IMLAI(1), QPC(2), RASM(3), SEARCH(4) }
enum class SearchMode(val id: Int) { INCLUDES(0), EXACT(1), PREFIX(2) }
enum class HighlightMode(val id: Int) { INK(0), BAND(1), BOTH(2) }
enum class BandHeight(val id: Int) { PITCH(0), INK(1) }
enum class MaskMode(val id: Int) { HIDE(0), BLOCK(1), BLUR(2) }
enum class Division(val id: Int) { JUZ(0), HIZB(1), NISF(2), RUBU_AL_HIZB(3) }
/** what an ornament replaces */
object QvpOrnamentKind { const val AYAH_MARK = 0; const val SURAH_HEADER = 1; const val PAGE_FRAME = 2 }

val QVP_MARKS = listOf("", "fathah", "kasrah", "dammah", "tanwin_al_fath", "tanwin_al_kasr", "tanwin_al_damm", "shaddah", "sukun", "maddah", "hamzah", "hamzat_al_wasl", "omitted_alif", "small_waw", "small_yaa", "small_noon", "dot", "two_dots", "three_dots", "rounded_zero", "rectangular_zero", "waqf_jaiz_mustawi_al_tarafayn", "waqf_jaiz_waqf_awla", "waqf_jaiz_wasl_awla", "waqf_lazim", "waqf_al_muanaqah", "saktah", "small_meem", "hizb", "sajdah", "sajdah_mark", "sajdah_line", "seen_al_qiraah", "tashil", "ishmam", "imalah")
fun markId(name: String): Int = QVP_MARKS.indexOf(name).let { if (it < 0) 255 else it }

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
        fun deco(k: Int) = Selector(intArrayOf(14, k, 0, 0))
        fun decoIdx(i: Int) = Selector(intArrayOf(15, i, 0, 0))
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

data class QvpWord(val idx: Int, val surah: Int, val ayah: Int, val word: Int, val line: Int, val ayahIdx: Int, val lineIdx: Int,
                   val x0: Float, val y0: Float, val x1: Float, val y1: Float, val text: String, val firstPath: Int, val nPaths: Int) {
    val wordKey get() = "$surah:$ayah:$word"
    val ayahKey get() = "$surah:$ayah"
}
data class QvpAyah(val idx: Int, val surah: Int, val ayah: Int, val fragment: Int, val fragments: Int, val flags: Int, val rubuAlHizb: Int, val firstWord: Int, val nWords: Int,
                   val ayahMarkDeco: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float)
data class QvpLine(val idx: Int, val lineNo: Int, val isHeader: Boolean, val firstWord: Int, val nWords: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                   val bandY0: Float, val bandY1: Float, val centre: Float)
data class QvpDecoration(val idx: Int, val kind: Int, val surah: Int, val ayah: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float,
                         val text: String, val firstPath: Int, val nPaths: Int)
/** Indices are -1 when absent. */
/** what a mushaf's ornaments may be redistributed under */
data class QvpOrnamentLicence(val id: String, val status: String, val redistributable: Boolean, val attribution: String)
/** one printed colour of a design; `slot` is the window it leaves open (alpha 0) */
data class QvpOrnamentPart(val index: Int, val name: String, val color: Int, val stroke: Boolean)
data class QvpOrnamentStyle(
    val index: Int, val name: String, val riwayah: String,
    val hasAyahMark: Boolean, val hasSurahHeader: Boolean, val hasPageFrame: Boolean,
    /** the frame is assembled from a corner and two repeat units rather than stretched */
    val tiles: Boolean,
    val licence: QvpOrnamentLicence, val parts: List<QvpOrnamentPart>,
)
/** what dressing the page did; `viewBox` is x, y, w, h — the border grew it */
data class QvpDress(
    val style: Int, val ayahMarks: Int, val surahHeaders: Int, val frameRepeats: Int,
    val frameStretched: Boolean, val nDraws: Int,
    /** bumped on every rebuild of the display list; cache a raster against it */
    val revision: Int,
    val viewBox: FloatArray,
)
/** one placed ornament outline in page units; [kind] is a [QvpOrnamentKind] */
data class QvpOrnamentDraw(
    val path: android.graphics.Path, val color: Int, val kind: Int, val stroke: Boolean,
    val strokeWidth: Float, val part: Int,
    /** the page line it was measured against, or -1 for the border, which never moves */
    val line: Int,
)

data class QvpHit(val word: Int, val path: Int, val deco: Int)
data class QvpHitEx(val word: Int, val path: Int, val deco: Int, val line: Int, val distance: Float, val exact: Boolean)
data class QvpHitOptions(val maxDistance: Float = 0f, val gapBias: Float = 0.6f, val exactFirst: Boolean = true)
data class QvpBox(val id: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float, val color: Int, val radius: Float)
data class QvpHitBox(val word: Int, val line: Int, val x0: Float, val y0: Float, val x1: Float, val y1: Float, val inkX0: Float, val inkY0: Float, val inkX1: Float, val inkY1: Float)
data class QvpLineBand(val line: Int, val lineNo: Int, val y0: Float, val y1: Float, val mid: Float, val inkY0: Float, val inkY1: Float)
data class QvpLayoutSpec(val viewportW: Float, val viewportH: Float, val padTop: Float = 0f, val padBottom: Float = 0f, val padLeft: Float = 0f, val padRight: Float = 0f,
                         val lineSpacing: Float = 1f, val lineGap: Float = 0f, val fillHeight: Boolean = false, val nominalLines: Int = 15)
/** Page → viewport: vx = ox + x*scale ; vy = oy + (y + lineDy[line])*scale. */
class QvpLayout(val scale: Float, val ox: Float, val oy: Float, val contentW: Float, val contentH: Float, val pitch: Float, val lineDy: FloatArray, val slotTop: FloatArray, val slotBottom: FloatArray)
data class QvpHighlightStyle(val mode: HighlightMode = HighlightMode.BAND, val ink: Int = 0x1a73e8ff.toInt(), val band: Int = 0xd6a3264d.toInt(), val height: BandHeight = BandHeight.PITCH,
                             val padX: Float = 1.2f, val padY: Float = 0f, val radius: Float = 0f, val seam: Float = 0.25f, val transitionMs: Int = 0, val layer: Int = QvpLayer.HIGHLIGHT) {
    internal fun ints() = intArrayOf(mode.id, height.id, ink, band, transitionMs, layer)
    internal fun floats() = floatArrayOf(padX, padY, radius, seam)
}
/** Colours with alpha 0 (or null) leave that colour alone. */
data class QvpTheme(val ink: Int? = null, val diacritics: Int? = null, val dots: Int? = null, val waqf: Int? = null, val sifr: Int? = null, val ayahMark: Int? = null, val numeral: Int? = null,
                    val headers: Int? = null, val marks: Map<String, Int> = emptyMap(), val transitionMs: Int = 0)
data class QvpSurah(val number: Int, val ayahCount: Int, val hasBanner: Boolean, val hasBasmalah: Boolean, val place: String, val bannerDeco: Int, val arabic: String, val latin: String, val english: String)
data class QvpDivision(val kind: Division, val n: Int, val surah: Int, val ayah: Int, val line: Int, val ayahIdx: Int)
data class QvpAyahMark(val deco: Int, val surah: Int, val ayah: Int, val line: Int, val cx: Float, val cy: Float, val r: Float, val ornamentPath: Int, val numeralPath: Int)
data class QvpRosette(val deco: Int, val surah: Int, val ayah: Int, val juz: Int, val hizb: Int, val nisf: Int, val rubuAlHizb: Int, val rubuAlHizbInHizb: Int)
data class QvpSajdah(val deco: Int, val surah: Int, val ayah: Int, val signPath: Int)
data class QvpMatch(val word: Int, val index: Int, val loose: Boolean, val wordKey: String, val text: String)
data class QvpCropBox(val x0: Float, val y0: Float, val x1: Float, val y1: Float, val nWords: Int, val ayahMarkDeco: Int)
data class QvpAtlasSurah(val n: Int, val page: Int, val ayahCount: Int, val place: String, val arabic: String, val latin: String, val english: String)
data class QvpAtlasRubuAlHizb(val rubuAlHizb: Int, val surah: Int, val ayah: Int, val page: Int) { val ayahKey get() = "$surah:$ayah" }
