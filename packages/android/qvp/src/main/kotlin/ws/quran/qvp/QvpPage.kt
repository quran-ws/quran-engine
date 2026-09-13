package ws.quran.qvp

import android.graphics.Path

/** Engine-wide helpers (names, Arabic text tools, layout maths). */
object QvpEngine {
    fun version() = QvpNative.version()
    fun engineName() = QvpNative.engineName()
    fun markName(m: Int) = QvpNative.markName(m)
    fun familyName(f: Int) = QvpNative.familyName(f)
    fun kindName(k: Int) = QvpNative.kindName(k)
    fun categoryName(c: Int) = QvpNative.categoryName(c)
    fun markFromName(name: String) = QvpNative.markFromName(name)
    fun markCategory(m: Int) = QvpNative.markCategory(m)
    /** The engine's name tables (QVP_NAMES_*): no wrapper carries a table of its own. */
    object Names { const val MARK = 0; const val KIND = 1; const val FAMILY = 2; const val CATEGORY = 3; const val DECORATION = 4; const val DIVISION = 5; const val PLACE = 6 }
    fun nameCount(table: Int) = QvpNative.nameCount(table)
    fun name(table: Int, id: Int) = QvpNative.name(table, id)
    /** 255 when the table has no such name. */
    fun nameId(table: Int, name: String) = QvpNative.nameId(table, name)
    /** Every name of a table, index = id. */
    fun names(table: Int): List<String> = (0 until nameCount(table)).map { name(table, it) }
    fun decorationName(k: Int) = name(Names.DECORATION, k)
    fun divisionName(d: Int) = name(Names.DIVISION, d)
    fun placeName(p: Int) = name(Names.PLACE, p)
    fun strip(s: String) = QvpNative.arabic(0, s)
    fun fold(s: String) = QvpNative.arabic(1, s)
    fun normalize(s: String) = QvpNative.arabic(2, s)
    fun looseKey(s: String) = QvpNative.arabic(3, s)
    fun gapToFill(pageW: Float, pageH: Float, lines: Int, viewW: Float, viewH: Float, max: Float = 0f) = QvpNative.gapToFill(pageW, pageH, lines, viewW, viewH, max)
    fun wastedFraction(pageW: Float, pageH: Float, viewW: Float, viewH: Float) = QvpNative.wastedFraction(pageW, pageH, viewW, viewH)
}

/**
 * One loaded page. Geometry is copied out of the engine once; everything that decides
 * (hit-testing, layout, styles, highlights, masks, search) stays inside the engine.
 * Mirrors web/qvp.js — see docs/API.md. Call [close] when done.
 */
class QvpPage(bytes: ByteArray) : AutoCloseable {
    private var nativeHandle: Long = QvpNative.pageLoad(bytes)
    private val h: Long get() = nativeHandle.takeIf { it != 0L } ?: error("QvpPage is closed")
    init { require(nativeHandle != 0L) { "qvp_page_load failed (not a QVP1 file?)" } }

    val isClosed: Boolean get() = nativeHandle == 0L

    val width: Float; val height: Float; val pageNo: Int
    val nLines: Int; val nAyahs: Int; val nWords: Int; val nPaths: Int; val nDecos: Int
    val ops: ByteArray; val pts: FloatArray
    /** stride 8 per path: opStart, opCount, ptStart, ptCount, flags, word, line, extra */
    val table: IntArray
    val words: List<QvpWord>; val ayahs: List<QvpAyah>; val lines: List<QvpLine>; val decos: List<QvpDecoration>
    val naturalPitch: Float
    var currentLayout: QvpLayout? = null; private set
    var defaultInk: Int = QvpDefaults.INK; private set
    private var paths: Array<Path>? = null

    init {
        val i = QvpNative.pageInfo(h)
        width = i[0]; height = i[1]; pageNo = i[2].toInt(); nLines = i[3].toInt(); nAyahs = i[4].toInt(); nWords = i[5].toInt(); nPaths = i[6].toInt(); nDecos = i[7].toInt()
        ops = QvpNative.geomOps(h); pts = QvpNative.geomPts(h); table = QvpNative.geomTable(h)
        words = List(nWords) { k -> val v = QvpNative.wordInfo(h, k)!!; QvpWord(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5].toInt(), v[6], v[7], v[8], v[9], QvpNative.wordText(h, k) ?: "", v[10].toInt(), v[11].toInt()) }
        ayahs = List(nAyahs) { k -> val v = QvpNative.ayahInfo(h, k)!!; QvpAyah(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5].toInt(), v[6].toInt(), v[7].toInt(), v[8].toInt(), v[9], v[10], v[11], v[12]) }
        lines = List(nLines) { k -> val v = QvpNative.lineInfo(h, k)!!; QvpLine(k, v[0].toInt(), v[1] > 0.5f, v[2].toInt(), v[3].toInt(), v[4], v[5], v[6], v[7], v[8], v[9], v[10]) }
        decos = List(nDecos) { k -> val v = QvpNative.decoInfo(h, k)!!; QvpDecoration(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4], v[5], v[6], v[7], QvpNative.decoText(h, k) ?: "", v[8].toInt(), v[9].toInt()) }
        naturalPitch = QvpNative.naturalPitch(h)
    }

    // ── geometry ──
    fun pathFlags(i: Int) = table[i * 8 + 4]
    fun pathKind(i: Int) = pathFlags(i) and 0xff
    fun pathMark(i: Int) = (pathFlags(i) shr 8) and 0xff
    fun pathFamily(i: Int) = (pathFlags(i) shr 16) and 0xff
    fun pathEvenOdd(i: Int) = ((pathFlags(i) ushr 24) and 1) == 1
    fun pathWord(i: Int) = table[i * 8 + 5]
    fun pathLine(i: Int) = table[i * 8 + 6]
    fun pathCategory(i: Int) = table[i * 8 + 7] and 0xff
    fun pathNthInWord(i: Int) = (table[i * 8 + 7] shr 8) and 0xff
    fun pathNthMark(i: Int) = ((table[i * 8 + 7] shr 16) and 0xff).let { if (it == 0xff) -1 else it }
    /** android.graphics.Path per engine path, in page units, built once. */
    fun buildPaths(): Array<Path> {
        paths?.let { return it }
        val out = Array(nPaths) { Path() }
        for (i in 0 until nPaths) {
            val p = out[i]; var o = table[i * 8]; val oe = o + table[i * 8 + 1]; var k = table[i * 8 + 2]
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
        paths = out; return out
    }

    // ── words / text ──
    fun wordKey(i: Int) = words[i].wordKey
    fun wordForm(i: Int, form: Form = Form.RASM_UTHMANI): String = QvpNative.wordForm(h, i, form.id) ?: ""
    fun findWord(surah: Int, ayah: Int, word: Int): Int = QvpNative.findWord(h, surah, ayah, word)
    fun target(s: String) = Target.parse(s, this)
    fun resolve(t: Target): IntArray = QvpNative.resolve(h, t.arr)
    fun resolve(s: String) = resolve(target(s))
    fun text(t: Target, form: Form = Form.RASM_UTHMANI, wordSep: String = " ", lineSep: String = "\n"): String = QvpNative.textTarget(h, t.arr, form.id, wordSep, lineSep)
    fun text(s: String, form: Form = Form.RASM_UTHMANI, wordSep: String = " ", lineSep: String = "\n") = text(target(s), form, wordSep, lineSep)
    fun search(query: String, form: Form = Form.SEARCH, mode: SearchMode = SearchMode.INCLUDES, normalize: Boolean = true, loose: Boolean = true, limit: Int = 0): List<QvpMatch> {
        val v = QvpNative.search(h, query, form.id, mode.id, normalize, loose, limit)
        return List(v.size / 3) { k -> val w = v[k * 3]; QvpMatch(w, v[k * 3 + 1], v[k * 3 + 2] != 0, wordKey(w), words[w].text) }
    }
    fun citation(ws: IntArray): String = QvpNative.citation(h, ws)
    /** attach a words sidecar ({"s:a:w": {"rasmImlai","qpc","rasm","search"}}); returns words updated, -1 on bad JSON */
    fun attachWords(json: ByteArray): Int = QvpNative.attachWords(h, json)
    fun attachWords(json: String) = attachWords(json.toByteArray())
    fun hasForm(form: Form) = QvpNative.hasForm(h, form.id)

    // ── metadata ──
    fun surahs(): List<QvpSurah> = List(QvpNative.surahsCount(h)) { i -> val n = QvpNative.surahNums(h, i)!!; val s = QvpNative.surahNames(h, i)!!
        QvpSurah(n[0].toInt(), n[1].toInt(), n[2] > 0.5f, n[3] > 0.5f, QvpEngine.placeName(n[4].toInt()), n[5].toInt(), s[0], s[1], s[2]) }
    fun divisions(): List<QvpDivision> { val v = QvpNative.divisions(h); return List(v.size / 6) { k -> QvpDivision(Division.entries[v[k * 6]], v[k * 6 + 2], v[k * 6 + 3], v[k * 6 + 4], v[k * 6 + 1], v[k * 6 + 5]) } }
    fun ayahMarks(): List<QvpAyahMark> { val v = QvpNative.ayahMarks(h); return List(v.size / 9) { k -> val o = k * 9; QvpAyahMark(v[o].toInt(), v[o + 1].toInt(), v[o + 2].toInt(), v[o + 3].toInt(), v[o + 4], v[o + 5], v[o + 6], v[o + 7].toInt(), v[o + 8].toInt()) } }
    fun ayahMarkOf(surah: Int, ayah: Int) = ayahMarks().firstOrNull { it.surah == surah && it.ayah == ayah }
    fun rosettes(): List<QvpRosette> { val v = QvpNative.rosettes(h); return List(v.size / 8) { k -> val o = k * 8; QvpRosette(v[o], v[o + 1], v[o + 2], v[o + 3], v[o + 4], v[o + 5], v[o + 6], v[o + 7]) } }
    fun sajdahs(): List<QvpSajdah> { val v = QvpNative.sajdahs(h); return List(v.size / 4) { k -> QvpSajdah(v[k * 4], v[k * 4 + 1], v[k * 4 + 2], v[k * 4 + 3]) } }
    fun ayahKeys(): List<Pair<Int, Int>> = QvpNative.ayahKeys(h).map { (it ushr 16) to (it and 0xffff) }
    /** (count on this page, whole ayah is here) */
    fun ayahWordCount(surah: Int, ayah: Int): Pair<Int, Boolean> { val v = QvpNative.ayahWordCount(h, surah, ayah); return v[0] to (v[1] != 0) }
    /** words for n recitation segments, or null when the counts disagree (follow the ayah whole) */
    fun reciteMap(surah: Int, ayah: Int, nSegments: Int): IntArray? = QvpNative.reciteMap(h, surah, ayah, nSegments)
    fun wordLabel(i: Int) = QvpNative.wordLabel(h, i)
    fun ayahLabel(i: Int) = QvpNative.ayahLabel(h, i)

    // ── hit testing ──
    private fun hit(v: IntArray?) = v?.let { QvpHit(it[0], it[1], it[2]) }
    private fun hitEx(v: FloatArray?) = v?.let { QvpHitEx(it[0].toInt(), it[1].toInt(), it[2].toInt(), it[3].toInt(), it[4], it[5] > 0.5f) }
    fun hitTest(x: Float, y: Float) = hit(QvpNative.hitTest(h, x, y))
    fun hitTestView(vx: Float, vy: Float) = hit(QvpNative.hitTestView(h, vx, vy))
    /** gap-aware: every point on a printed line resolves to the word the reader meant */
    fun hitTestEx(x: Float, y: Float, o: QvpHitOptions = QvpHitOptions()) = hitEx(QvpNative.hitTestEx(h, x, y, o.maxDistance, o.gapBias, o.exactFirst))
    fun hitTestViewEx(vx: Float, vy: Float, o: QvpHitOptions = QvpHitOptions()) = hitEx(QvpNative.hitTestViewEx(h, vx, vy, o.maxDistance, o.gapBias, o.exactFirst))
    fun lineBands(): List<QvpLineBand> { val v = QvpNative.lineBands(h); return List(v.size / 7) { k -> val o = k * 7; QvpLineBand(v[o].toInt(), v[o + 1].toInt(), v[o + 2], v[o + 3], v[o + 4], v[o + 5], v[o + 6]) } }
    fun hitBoxes(gapBias: Float = QvpDefaults.GAP_BIAS): List<QvpHitBox> { val v = QvpNative.hitBoxes(h, gapBias); return List(v.size / 10) { k -> val o = k * 10; QvpHitBox(v[o].toInt(), v[o + 1].toInt(), v[o + 2], v[o + 3], v[o + 4], v[o + 5], v[o + 6], v[o + 7], v[o + 8], v[o + 9]) } }

    // ── layout ──
    fun layout(spec: QvpLayoutSpec): QvpLayout {
        val v = QvpNative.layout(h, spec.floats())
        val n = v[6].toInt()
        return QvpLayout(v[0], v[1], v[2], v[3], v[4], v[5], FloatArray(n) { v[10 + it * 3] }, FloatArray(n) { v[11 + it * 3] }, FloatArray(n) { v[12 + it * 3] }, v[7], v[8], v[9]).also { currentLayout = it }
    }
    /** Leading (page units) that makes this page fill the padded viewport of [spec]; max 0 = unlimited. */
    fun layoutGapToFill(spec: QvpLayoutSpec, max: Float = 0f): Float = QvpNative.layoutGapToFill(h, spec.floats(), max)
    fun wordBoxView(i: Int): FloatArray? = QvpNative.wordBoxView(h, i)

    // ── styles (handles undo exactly) ──
    fun style(sel: Selector, rgba: Int, transitionMs: Int = 0, layer: Int = QvpLayer.BASE): Int = QvpNative.styleAdd(h, layer, sel.arr, rgba, transitionMs)
    fun styleTarget(t: Target, rgba: Int, transitionMs: Int = 0, layer: Int = QvpLayer.BASE): Int = QvpNative.styleAddTarget(h, layer, t.arr, rgba, transitionMs)
    fun unstyle(handle: Int) = QvpNative.styleRemove(h, handle)
    fun restyle(handle: Int, rgba: Int, transitionMs: Int = 0) = QvpNative.styleRepaint(h, handle, rgba, transitionMs)
    fun hide(sel: Selector): Int = QvpNative.hide(h, sel.arr)
    fun clearStyles() = QvpNative.styleClear(h)
    fun clearLayer(layer: Int) = QvpNative.styleClearLayer(h, layer)
    fun setDefaultInk(rgba: Int) { defaultInk = rgba; QvpNative.styleDefault(h, rgba) }
    fun theme(t: QvpTheme): Int {
        val z = { c: Int? -> c ?: 0 }
        val base = intArrayOf(z(t.ink), z(t.diacritics), z(t.dots), z(t.waqf), z(t.sifr), z(t.ayahMark), z(t.numeral), z(t.headers), t.transitionMs)
        val marks = t.marks.flatMap { (m, c) -> listOf(markId(m), c) }.toIntArray()
        return QvpNative.theme(h, base + marks)
    }
    fun styleHandles(): IntArray = QvpNative.styleHandles(h)

    // ── clock & display list ──
    /** advance animations; true while something is still moving */
    fun tick(nowMs: Double): Boolean = QvpNative.tick(h, nowMs)
    fun paint(): IntArray = QvpNative.paint(h)
    /** flat pairs [pathIdx, rgba, ...] for paths ≠ default ink */
    fun styled(): IntArray = QvpNative.styled(h)
    fun colorOf(path: Int) = QvpNative.colorOf(h, path)

    // ── highlights ──
    fun highlight(t: Target, style: QvpHighlightStyle = QvpHighlightStyle()): Int = QvpNative.highlight(h, t.arr, style.ints(), style.floats())
    fun highlight(s: String, style: QvpHighlightStyle = QvpHighlightStyle()) = highlight(target(s), style)
    fun rehighlight(handle: Int, t: Target) = QvpNative.rehighlight(h, handle, t.arr)
    fun restyleHighlight(handle: Int, style: QvpHighlightStyle) = QvpNative.restyleHighlight(h, handle, style.ints(), style.floats())
    fun unhighlight(handle: Int) = QvpNative.unhighlight(h, handle)
    fun clearHighlights() = QvpNative.clearHighlights(h)
    fun highlightHandles(): IntArray = QvpNative.highlightHandles(h)
    fun highlightWords(handle: Int): IntArray = QvpNative.highlightWords(h, handle)
    private fun boxes(v: IntArray): List<QvpBox> = List(v.size / 8) { k -> val o = k * 8; QvpBox(v[o], v[o + 1], Float.fromBits(v[o + 2]), Float.fromBits(v[o + 3]), Float.fromBits(v[o + 4]), Float.fromBits(v[o + 5]), v[o + 6], Float.fromBits(v[o + 7])) }
    /** animated band boxes in viewport px; draw each id as one nonzero path behind the ink */
    fun highlightBoxes(): List<QvpBox> = boxes(QvpNative.highlightBoxes(h))
    fun bandBoxes(ws: IntArray, height: BandHeight = BandHeight.PITCH, padX: Float = QvpDefaults.HIGHLIGHT_PAD_X, padY: Float = QvpDefaults.HIGHLIGHT_PAD_Y) = boxes(QvpNative.bandBoxes(h, ws, height.id, padX, padY))

    // ── selection ──
    fun select(anchor: Int, focus: Int = anchor) = QvpNative.select(h, anchor, focus)
    fun clearSelection() = QvpNative.select(h, -1, -1)
    fun selection(): IntArray = QvpNative.selection(h)
    fun selectionText(form: Form = Form.RASM_UTHMANI, citation: Boolean = false) = QvpNative.selectionText(h, form.id, citation)

    // ── memorisation ──
    fun mask(t: Target, mode: MaskMode = MaskMode.HIDE) = QvpNative.mask(h, t.arr, mode.id)
    fun maskFrom(wi: Int, mode: MaskMode = MaskMode.HIDE) = QvpNative.maskFrom(h, wi, mode.id)
    fun maskOptions(blockColor: Int = QvpDefaults.MASK_BLOCK, padX: Float = QvpDefaults.MASK_PAD, padY: Float = QvpDefaults.MASK_PAD, radius: Float = QvpDefaults.MASK_RADIUS, reverse: Boolean = false) = QvpNative.maskOptions(h, blockColor, padX, padY, radius, reverse)
    fun revealNext(n: Int = 1) = QvpNative.revealNext(h, n)
    fun hideBack(n: Int = 1) = QvpNative.hideBack(h, n)
    fun revealWord(wi: Int) = QvpNative.revealWord(h, wi)
    fun hideWord(wi: Int) = QvpNative.hideWord(h, wi)
    fun revealAll() = QvpNative.revealAll(h)
    fun hideAll() = QvpNative.hideAll(h)
    fun unmask() = QvpNative.unmask(h)
    fun maskHidden(): IntArray = QvpNative.maskHidden(h)
    fun maskWords(): IntArray = QvpNative.maskWords(h)
    fun maskBoxes(): List<QvpBox> = boxes(QvpNative.maskBoxes(h))
    /** greyed page with a lit window; returns steps */
    fun revealStart(lit: Int = QvpDefaults.REVEAL_LIT, byAyah: Boolean = false, grey: Int = QvpDefaults.REVEAL_GREY, ink: Int = QvpDefaults.INK, ayahMarks: Boolean = true, transitionMs: Int = 0) = QvpNative.revealStart(h, lit, byAyah, grey, ink, ayahMarks, transitionMs)
    fun revealGoto(at: Long) = QvpNative.revealGoto(h, at)
    fun revealAt(): Long? = QvpNative.revealAt(h).let { if (it == -2L) null else it }
    fun revealSteps() = QvpNative.revealSteps(h)
    fun revealStepOf(wi: Int) = QvpNative.revealStepOf(h, wi)
    fun revealStop() = QvpNative.revealStop(h)

    // ── crop ──
    fun cropBox(t: Target, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Boolean = true): QvpCropBox? = QvpNative.cropBox(h, t.arr, pad, keepAyahMarks)?.let { QvpCropBox(it[0], it[1], it[2], it[3], it[4].toInt(), it[5].toInt()) }
    /** standalone SVG with the current colours; background alpha 0 = transparent */
    fun cropSvg(t: Target, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Boolean = true, background: Int = 0): String? = QvpNative.cropSvg(h, t.arr, pad, keepAyahMarks, background)

    @Synchronized
    override fun close() {
        val handle = nativeHandle
        if (handle != 0L) {
            nativeHandle = 0L
            QvpNative.pageFree(handle)
        }
    }
}

/** Cross-page lookup from atlas.qva. */
class QvpAtlas(bytes: ByteArray) : AutoCloseable {
    private var nativeHandle: Long = QvpNative.atlasLoad(bytes)
    private val h: Long get() = nativeHandle.takeIf { it != 0L } ?: error("QvpAtlas is closed")
    init { require(nativeHandle != 0L) { "qvp_atlas_load failed" } }
    val isClosed: Boolean get() = nativeHandle == 0L
    private fun surah(v: Array<String>?) = v?.let { QvpAtlasSurah(it[0].toInt(), it[1].toInt(), it[2].toInt(), QvpEngine.placeName(it[3].toIntOrNull() ?: 255), it[4], it[5], it[6]) }
    fun pageOf(surah: Int, ayah: Int): Int? = QvpNative.atlasPageOf(h, surah, ayah).let { if (it < 0) null else it }
    fun pageRange(page: Int): Pair<Pair<Int, Int>, Pair<Int, Int>>? = QvpNative.atlasPageRange(h, page)?.let { (it[0] to it[1]) to (it[2] to it[3]) }
    fun pages() = QvpNative.atlasPages(h)
    fun surah(n: Int) = surah(QvpNative.atlasSurah(h, n))
    fun surahs(): List<QvpAtlasSurah> = List(QvpNative.atlasSurahs(h)) { surah(QvpNative.atlasSurahAt(h, it))!! }
    fun pageOfSurah(n: Int) = surah(n)?.page
    fun division(kind: Division, n: Int): QvpAtlasRubuAlHizb? = QvpNative.atlasDivision(h, kind.id, n)?.let { QvpAtlasRubuAlHizb(it[0], it[1], it[2], it[3]) }
    fun juz(n: Int) = division(Division.JUZ, n)
    fun hizb(n: Int) = division(Division.HIZB, n)
    fun rubuAlHizb(n: Int) = division(Division.RUBU_AL_HIZB, n)
    fun divisionAt(kind: Division, surah: Int, ayah: Int): Int? = QvpNative.atlasDivisionAt(h, kind.id, surah, ayah).let { if (it < 0) null else it }
    fun juzAt(surah: Int, ayah: Int) = divisionAt(Division.JUZ, surah, ayah)
    fun pagesOfJuz(n: Int): Pair<Int, Int>? = QvpNative.atlasPagesOfJuz(h, n)?.let { it[0] to it[1] }
    fun findSurah(text: String): List<QvpAtlasSurah> = QvpNative.atlasFindSurah(h, text).toList().mapNotNull { n -> surah(n) }
    @Synchronized
    override fun close() {
        val handle = nativeHandle
        if (handle != 0L) {
            nativeHandle = 0L
            QvpNative.atlasFree(handle)
        }
    }
}
