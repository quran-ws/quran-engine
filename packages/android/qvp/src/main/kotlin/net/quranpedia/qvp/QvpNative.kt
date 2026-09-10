package net.quranpedia.qvp

/**
 * Raw JNI surface over the QVP C ABI (qvp.h). Use [QvpPage] / [QvpAtlas] instead.
 * Record layouts (see qvp_jni.c): targets are IntArray {kind, a, b, c, words...};
 * selectors IntArray {kind, a, b, c}; boxes are 8 ints per box {id, line, x0, y0, x1, y1 (float bits), colour, radius (float bits)}.
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
    @JvmStatic external fun decoInfo(h: Long, i: Int): FloatArray?
    @JvmStatic external fun decoText(h: Long, i: Int): String?
    @JvmStatic external fun findWord(h: Long, s: Int, a: Int, w: Int): Int
    @JvmStatic external fun resolve(h: Long, target: IntArray): IntArray
    @JvmStatic external fun naturalPitch(h: Long): Float
    // metadata
    @JvmStatic external fun surahsCount(h: Long): Int
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
    @JvmStatic external fun textTarget(h: Long, target: IntArray, form: Int, wordSep: String, lineSep: String): String
    @JvmStatic external fun search(h: Long, query: String, form: Int, mode: Int, normalize: Boolean, loose: Boolean, limit: Int): IntArray
    @JvmStatic external fun arabic(kind: Int, s: String): String
    @JvmStatic external fun citation(h: Long, words: IntArray): String
    @JvmStatic external fun attachWords(h: Long, json: ByteArray): Int
    @JvmStatic external fun hasForm(h: Long, form: Int): Boolean
    // hit testing
    @JvmStatic external fun hitTest(h: Long, x: Float, y: Float): IntArray?
    @JvmStatic external fun hitTestView(h: Long, x: Float, y: Float): IntArray?
    @JvmStatic external fun hitTestEx(h: Long, x: Float, y: Float, maxDistance: Float, gapBias: Float, exactFirst: Boolean): FloatArray?
    @JvmStatic external fun hitTestViewEx(h: Long, x: Float, y: Float, maxDistance: Float, gapBias: Float, exactFirst: Boolean): FloatArray?
    @JvmStatic external fun lineBands(h: Long): FloatArray
    @JvmStatic external fun hitBoxes(h: Long, gapBias: Float): FloatArray
    // layout
    @JvmStatic external fun layout(h: Long, spec: FloatArray): FloatArray
    @JvmStatic external fun gapToFill(pw: Float, ph: Float, lines: Int, vw: Float, vh: Float, max: Float): Float
    @JvmStatic external fun wastedFraction(pw: Float, ph: Float, vw: Float, vh: Float): Float
    @JvmStatic external fun wordBoxView(h: Long, i: Int): FloatArray?
    // styles
    @JvmStatic external fun styleAdd(h: Long, layer: Int, sel: IntArray, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleAddTarget(h: Long, layer: Int, target: IntArray, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleRemove(h: Long, handle: Int): Int
    @JvmStatic external fun styleRepaint(h: Long, handle: Int, rgba: Int, ms: Int): Int
    @JvmStatic external fun styleClear(h: Long)
    @JvmStatic external fun styleClearLayer(h: Long, layer: Int)
    @JvmStatic external fun styleDefault(h: Long, rgba: Int)
    @JvmStatic external fun hide(h: Long, sel: IntArray): Int
    @JvmStatic external fun theme(h: Long, theme: IntArray): Int
    @JvmStatic external fun styleHandles(h: Long): IntArray
    // clock & display list
    @JvmStatic external fun tick(h: Long, nowMs: Double): Boolean
    @JvmStatic external fun paint(h: Long): IntArray
    @JvmStatic external fun styled(h: Long): IntArray
    @JvmStatic external fun colorOf(h: Long, path: Int): Int
    // highlights
    @JvmStatic external fun highlight(h: Long, target: IntArray, styleInts: IntArray, styleFloats: FloatArray): Int
    @JvmStatic external fun rehighlight(h: Long, handle: Int, target: IntArray): Boolean
    @JvmStatic external fun restyleHighlight(h: Long, handle: Int, styleInts: IntArray, styleFloats: FloatArray): Boolean
    @JvmStatic external fun unhighlight(h: Long, handle: Int): Boolean
    @JvmStatic external fun clearHighlights(h: Long)
    @JvmStatic external fun highlightHandles(h: Long): IntArray
    @JvmStatic external fun highlightWords(h: Long, handle: Int): IntArray
    @JvmStatic external fun highlightBoxes(h: Long): IntArray
    @JvmStatic external fun bandBoxes(h: Long, words: IntArray, height: Int, padX: Float, padY: Float): IntArray
    // selection
    @JvmStatic external fun select(h: Long, anchor: Int, focus: Int)
    @JvmStatic external fun selection(h: Long): IntArray
    @JvmStatic external fun selectionText(h: Long, form: Int, citation: Boolean): String
    // memorisation
    @JvmStatic external fun mask(h: Long, target: IntArray, mode: Int)
    @JvmStatic external fun maskFrom(h: Long, wi: Int, mode: Int)
    @JvmStatic external fun maskOptions(h: Long, blockColor: Int, padX: Float, padY: Float, radius: Float, reverse: Boolean)
    @JvmStatic external fun revealNext(h: Long, n: Int): Int
    @JvmStatic external fun hideBack(h: Long, n: Int): Int
    @JvmStatic external fun revealWord(h: Long, wi: Int): Boolean
    @JvmStatic external fun hideWord(h: Long, wi: Int): Boolean
    @JvmStatic external fun revealAll(h: Long)
    @JvmStatic external fun hideAll(h: Long)
    @JvmStatic external fun unmask(h: Long)
    @JvmStatic external fun maskHidden(h: Long): IntArray
    @JvmStatic external fun maskWords(h: Long): IntArray
    @JvmStatic external fun maskBoxes(h: Long): IntArray
    @JvmStatic external fun revealStart(h: Long, lit: Int, byAyah: Boolean, grey: Int, ink: Int, ayahMarks: Boolean, ms: Int): Int
    @JvmStatic external fun revealGoto(h: Long, at: Long): Boolean
    @JvmStatic external fun revealAt(h: Long): Long
    @JvmStatic external fun revealSteps(h: Long): Int
    @JvmStatic external fun revealStepOf(h: Long, wi: Int): Long
    @JvmStatic external fun revealStop(h: Long)
    // crop
    @JvmStatic external fun cropBox(h: Long, target: IntArray, pad: Float, keepAyahMarks: Boolean): FloatArray?
    @JvmStatic external fun cropSvg(h: Long, target: IntArray, pad: Float, keepAyahMarks: Boolean, background: Int): String?
    // dress: another mushaf's ornaments
    @JvmStatic external fun ornamentsLoad(bytes: ByteArray): Long
    @JvmStatic external fun ornamentsFree(h: Long)
    @JvmStatic external fun ornamentStyles(h: Long): Int
    @JvmStatic external fun ornamentFindStyle(h: Long, name: String): Int
    @JvmStatic external fun ornamentStyleInts(h: Long, i: Int): IntArray?
    @JvmStatic external fun ornamentStyleStrings(h: Long, i: Int): Array<String>?
    @JvmStatic external fun ornamentPart(h: Long, style: Int, i: Int): IntArray?
    @JvmStatic external fun ornamentPartName(h: Long, style: Int, i: Int): String?
    @JvmStatic external fun dress(h: Long, set: Long, style: Int, gap: Float, lineArt: Boolean, ayahMarks: Boolean, surahHeaders: Boolean, pageFrame: Boolean, colors: IntArray?): Boolean
    @JvmStatic external fun undress(h: Long)
    @JvmStatic external fun dressInfo(h: Long): FloatArray?
    @JvmStatic external fun dressOps(h: Long): ByteArray
    @JvmStatic external fun dressPts(h: Long): FloatArray
    @JvmStatic external fun dressTable(h: Long): IntArray
    @JvmStatic external fun pageViewBox(h: Long): FloatArray
    @JvmStatic external fun dressOverflow(h: Long): FloatArray
    @JvmStatic external fun contentBox(h: Long): FloatArray
    // atlas
    @JvmStatic external fun atlasLoad(bytes: ByteArray): Long
    @JvmStatic external fun atlasFree(h: Long)
    @JvmStatic external fun atlasPageOf(h: Long, s: Int, a: Int): Int
    @JvmStatic external fun atlasPageRange(h: Long, page: Int): IntArray?
    @JvmStatic external fun atlasPages(h: Long): Int
    @JvmStatic external fun atlasSurahs(h: Long): Int
    @JvmStatic external fun atlasSurah(h: Long, n: Int): Array<String>?
    @JvmStatic external fun atlasSurahAt(h: Long, i: Int): Array<String>?
    @JvmStatic external fun atlasDivision(h: Long, kind: Int, n: Int): IntArray?
    @JvmStatic external fun atlasDivisionAt(h: Long, kind: Int, s: Int, a: Int): Int
    @JvmStatic external fun atlasPagesOfJuz(h: Long, n: Int): IntArray?
    @JvmStatic external fun atlasFindSurah(h: Long, text: String): IntArray
    // names
    @JvmStatic external fun markName(m: Int): String
    @JvmStatic external fun familyName(f: Int): String
    @JvmStatic external fun kindName(k: Int): String
    @JvmStatic external fun categoryName(c: Int): String
    @JvmStatic external fun markFromName(name: String): Int
    @JvmStatic external fun markCategory(m: Int): Int
    @JvmStatic external fun version(): Int
}
