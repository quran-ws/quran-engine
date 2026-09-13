package ws.quran.qvp

/**
 * Raw JNI surface over the QVP C ABI (qvp.h). Use [QvpPage] / [QvpAtlas] instead.
 * Record layouts (see qvp_jni.c): targets are IntArray {target, a, b, c, words...};
 * selectors IntArray {selector, a, b, c}; boxes are 8 ints per box {id, line, x0, y0, x1, y1 (float bits), colour, radius (float bits)}.
 */
internal object QvpNative {
    init { System.loadLibrary("qvp_jni") }
    // page
    @JvmStatic external fun pageLoad(bytes: ByteArray): Long
    @JvmStatic external fun pageFree(h: Long)
    @JvmStatic external fun pageInfo(h: Long): FloatArray
    @JvmStatic external fun geomOps(h: Long): ByteArray
    @JvmStatic external fun geomPts(h: Long): FloatArray
    @JvmStatic external fun geomTable(h: Long): IntArray
    @JvmStatic external fun wordInfo(h: Long, i: Int): FloatArray?
    @JvmStatic external fun wordText(h: Long, i: Int): String?
    @JvmStatic external fun wordForm(h: Long, i: Int, form: Int): String?
    @JvmStatic external fun ayahInfo(h: Long, i: Int): FloatArray?
    @JvmStatic external fun lineInfo(h: Long, i: Int): FloatArray?
    @JvmStatic external fun decorationInfo(h: Long, i: Int): FloatArray?
    @JvmStatic external fun decorationText(h: Long, i: Int): String?
    @JvmStatic external fun findWord(h: Long, s: Int, a: Int, w: Int): Int
    @JvmStatic external fun targetWords(h: Long, target: IntArray): IntArray
    @JvmStatic external fun naturalPitch(h: Long): Float
    // metadata
    @JvmStatic external fun surahCount(h: Long): Int
    @JvmStatic external fun surahNums(h: Long, i: Int): FloatArray?
    @JvmStatic external fun surahNames(h: Long, i: Int): Array<String>?
    @JvmStatic external fun divisions(h: Long): IntArray
    @JvmStatic external fun ayahMarks(h: Long): FloatArray
    @JvmStatic external fun rosettes(h: Long): IntArray
    @JvmStatic external fun sajdahs(h: Long): IntArray
    @JvmStatic external fun ayahKeys(h: Long): IntArray
    @JvmStatic external fun ayahWordCount(h: Long, s: Int, a: Int): IntArray
    @JvmStatic external fun reciteMap(h: Long, s: Int, a: Int, nSegments: Int): IntArray?
    @JvmStatic external fun wordLabel(h: Long, i: Int): String
    @JvmStatic external fun ayahLabel(h: Long, i: Int): String
    // text & search
    @JvmStatic external fun text(h: Long, target: IntArray, form: Int, wordSep: String, lineSep: String): String
    @JvmStatic external fun search(h: Long, query: String, form: Int, mode: Int, normalize: Boolean, looseMatch: Boolean, limit: Int): IntArray
    @JvmStatic external fun arabic(op: Int, s: String): String
    @JvmStatic external fun citation(h: Long, words: IntArray): String
    @JvmStatic external fun attachWords(h: Long, json: ByteArray): Int
    @JvmStatic external fun hasForm(h: Long, form: Int): Boolean
    // hit testing
    @JvmStatic external fun hitTestExact(h: Long, x: Float, y: Float): FloatArray?
    @JvmStatic external fun hitTestExactView(h: Long, x: Float, y: Float): FloatArray?
    @JvmStatic external fun hitTest(h: Long, x: Float, y: Float, maxDistance: Float, gapBias: Float, preferExact: Boolean): FloatArray?
    @JvmStatic external fun hitTestView(h: Long, x: Float, y: Float, maxDistance: Float, gapBias: Float, preferExact: Boolean): FloatArray?
    @JvmStatic external fun lineBands(h: Long): FloatArray
    @JvmStatic external fun hitAreas(h: Long, gapBias: Float): FloatArray
    // layout
    @JvmStatic external fun layout(h: Long, spec: FloatArray): FloatArray
    @JvmStatic external fun layoutGapToFill(h: Long, spec: FloatArray, max: Float): Float
    @JvmStatic external fun gapToFill(pw: Float, ph: Float, lines: Int, vw: Float, vh: Float, max: Float): Float
    @JvmStatic external fun wastedFraction(pw: Float, ph: Float, vw: Float, vh: Float): Float
    @JvmStatic external fun wordBoundsView(h: Long, i: Int): FloatArray?
    // styles
    @JvmStatic external fun styleAdd(h: Long, layer: Int, sel: IntArray, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleAddTarget(h: Long, layer: Int, target: IntArray, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleRemove(h: Long, handle: Int): Int
    @JvmStatic external fun styleRecolor(h: Long, handle: Int, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleClear(h: Long)
    @JvmStatic external fun styleClearLayer(h: Long, layer: Int)
    @JvmStatic external fun styleDefaultColor(h: Long, rgba: Int)
    @JvmStatic external fun hide(h: Long, sel: IntArray): Int
    @JvmStatic external fun theme(h: Long, theme: IntArray): Int
    @JvmStatic external fun styleHandles(h: Long): IntArray
    // clock & display list
    @JvmStatic external fun tick(h: Long, nowMs: Double): Boolean
    @JvmStatic external fun colors(h: Long): IntArray
    @JvmStatic external fun styledPaths(h: Long): IntArray
    @JvmStatic external fun colorOf(h: Long, path: Int): Int
    // highlights
    @JvmStatic external fun highlight(h: Long, target: IntArray, styleInts: IntArray, styleFloats: FloatArray): Int
    @JvmStatic external fun moveHighlight(h: Long, handle: Int, target: IntArray): Boolean
    @JvmStatic external fun restyleHighlight(h: Long, handle: Int, styleInts: IntArray, styleFloats: FloatArray): Boolean
    @JvmStatic external fun removeHighlight(h: Long, handle: Int): Boolean
    @JvmStatic external fun clearHighlights(h: Long)
    @JvmStatic external fun highlightHandles(h: Long): IntArray
    @JvmStatic external fun highlightWords(h: Long, handle: Int): IntArray
    @JvmStatic external fun highlightBoxesView(h: Long): IntArray
    @JvmStatic external fun wordBands(h: Long, words: IntArray, height: Int, padX: Float, padY: Float): IntArray
    // selection
    @JvmStatic external fun select(h: Long, anchor: Int, focus: Int)
    @JvmStatic external fun selection(h: Long): IntArray
    @JvmStatic external fun selectionText(h: Long, form: Int, citation: Boolean): String
    // memorisation
    @JvmStatic external fun mask(h: Long, target: IntArray, mode: Int)
    @JvmStatic external fun maskFrom(h: Long, wordIndex: Int, mode: Int)
    @JvmStatic external fun maskOptions(h: Long, blockColor: Int, padX: Float, padY: Float, radius: Float, reverse: Boolean)
    @JvmStatic external fun unmaskNext(h: Long, n: Int): Int
    @JvmStatic external fun maskBack(h: Long, n: Int): Int
    @JvmStatic external fun unmaskWord(h: Long, wordIndex: Int): Boolean
    @JvmStatic external fun maskWord(h: Long, wordIndex: Int): Boolean
    @JvmStatic external fun unmaskAll(h: Long)
    @JvmStatic external fun maskAll(h: Long)
    @JvmStatic external fun unmask(h: Long)
    @JvmStatic external fun maskHidden(h: Long): IntArray
    @JvmStatic external fun maskWords(h: Long): IntArray
    @JvmStatic external fun maskBoxesView(h: Long): IntArray
    @JvmStatic external fun revealStart(h: Long, lit: Int, byAyah: Boolean, grey: Int, ink: Int, ayahMarks: Boolean, ms: Int): Int
    @JvmStatic external fun revealGoto(h: Long, at: Long): Boolean
    @JvmStatic external fun revealPosition(h: Long): Long
    @JvmStatic external fun revealStepCount(h: Long): Int
    @JvmStatic external fun revealStepOf(h: Long, wordIndex: Int): Long
    @JvmStatic external fun revealStop(h: Long)
    // crop
    @JvmStatic external fun cropBounds(h: Long, target: IntArray, pad: Float, keepAyahMarks: Boolean): FloatArray?
    @JvmStatic external fun cropSvg(h: Long, target: IntArray, pad: Float, keepAyahMarks: Boolean, background: Int): String?
    // atlas
    @JvmStatic external fun atlasLoad(bytes: ByteArray): Long
    @JvmStatic external fun atlasFree(h: Long)
    @JvmStatic external fun atlasPageOf(h: Long, s: Int, a: Int): Int
    @JvmStatic external fun atlasPageRange(h: Long, page: Int): IntArray?
    @JvmStatic external fun atlasPageCount(h: Long): Int
    @JvmStatic external fun atlasSurahCount(h: Long): Int
    @JvmStatic external fun atlasSurah(h: Long, n: Int): Array<String>?
    @JvmStatic external fun atlasSurahAt(h: Long, i: Int): Array<String>?
    @JvmStatic external fun atlasDivision(h: Long, division: Int, n: Int): IntArray?
    @JvmStatic external fun atlasDivisionOf(h: Long, division: Int, s: Int, a: Int): Int
    @JvmStatic external fun atlasPagesOfJuz(h: Long, n: Int): IntArray?
    @JvmStatic external fun atlasSearchSurahs(h: Long, text: String): IntArray
    // names
    @JvmStatic external fun markName(m: Int): String
    @JvmStatic external fun familyName(f: Int): String
    @JvmStatic external fun kindName(k: Int): String
    @JvmStatic external fun categoryName(c: Int): String
    @JvmStatic external fun markFromName(name: String): Int
    @JvmStatic external fun markCategory(m: Int): Int
    @JvmStatic external fun nameCount(table: Int): Int
    @JvmStatic external fun name(table: Int, id: Int): String
    @JvmStatic external fun nameId(table: Int, name: String): Int
    @JvmStatic external fun version(): Int
    @JvmStatic external fun engineName(): String
}
