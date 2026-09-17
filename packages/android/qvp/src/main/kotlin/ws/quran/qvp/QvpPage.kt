package ws.quran.qvp

import android.graphics.Path

/** Engine-wide helpers (names, Arabic text tools, layout maths). */
object QvpEngine {
    /** The engine version, e.g. `0.2.0`. */
    fun version() = QvpNative.version()
    /** The page format version the engine reads. */
    fun formatVersion() = QvpNative.formatVersion()
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
}

/**
 * One loaded page. Geometry is copied out of the engine once; everything that decides
 * (hit-testing, layout, styles, highlights, masks, search) stays inside the engine.
 * Mirrors web/qvp.js — see docs/API.md. Call [close] when done.
 */
class QvpPage(bytes: ByteArray) : AutoCloseable {
    private var nativeHandle: Long = QvpNative.pageLoad(bytes)
    private val h: Long get() = nativeHandle.takeIf { it != 0L } ?: error("QvpPage is closed")
    /** False once [close] has been called. Every call into the engine needs an open page. */
    val isOpen: Boolean get() = nativeHandle != 0L
    init { require(nativeHandle != 0L) { "qvp_page_load failed (not a QVP1 file?)" } }

    val isClosed: Boolean get() = nativeHandle == 0L

    val width: Float; val height: Float; val pageNo: Int
    val nLines: Int; val nAyahs: Int; val nWords: Int; val nPaths: Int; val nDecorations: Int
    val ops: ByteArray; val pts: FloatArray
    /** stride 8 per path: opStart, opCount, ptStart, ptCount, flags, word, line, extra */
    val table: IntArray
    val words: List<QvpWord>; val ayahs: List<QvpAyah>; val lines: List<QvpLine>; val decorations: List<QvpDecoration>
    val lineSpacing: Float
    var currentLayout: QvpLayout? = null; private set
    var defaultInk: Int = QvpDefaults.INK; private set

    init {
        val i = QvpNative.pageInfo(h)
        width = i[0]; height = i[1]; pageNo = i[2].toInt(); nLines = i[3].toInt(); nAyahs = i[4].toInt(); nWords = i[5].toInt(); nPaths = i[6].toInt(); nDecorations = i[7].toInt()
        ops = QvpNative.geomOps(h); pts = QvpNative.geomPts(h); table = QvpNative.geomTable(h)
        words = List(nWords) { k -> val v = QvpNative.wordInfo(h, k)!!; QvpWord(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5].toInt(), v[6], v[7], v[8], v[9], QvpNative.wordText(h, k) ?: "", v[10].toInt(), v[11].toInt()) }
        ayahs = List(nAyahs) { k -> val v = QvpNative.ayahInfo(h, k)!!; QvpAyah(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4].toInt(), v[5].toInt(), v[6].toInt(), v[7].toInt(), v[8].toInt(), v[9], v[10], v[11], v[12]) }
        lines = List(nLines) { k -> val v = QvpNative.lineInfo(h, k)!!; QvpLine(k, v[0].toInt(), v[1] > 0.5f, v[2].toInt(), v[3].toInt(), v[4], v[5], v[6], v[7], v[8], v[9], v[10]) }
        decorations = List(nDecorations) { k -> val v = QvpNative.decorationInfo(h, k)!!; QvpDecoration(k, v[0].toInt(), v[1].toInt(), v[2].toInt(), v[3].toInt(), v[4], v[5], v[6], v[7], QvpNative.decorationText(h, k) ?: "", v[8].toInt(), v[9].toInt()) }
        lineSpacing = QvpNative.pageLineSpacing(h)
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
    /** Every path of the page, each one built the first time it is drawn.
     *
     * A page carries hundreds of outlines and a reader sees one screen of them, so building them
     * all before the page appears is work done in front of the reader for ink they may never
     * scroll to. Each is built once and kept.
     */
    fun buildPaths(): Array<Path> {
        lazyPaths?.let { return it }
        val out = Array(nPaths) { LAZY }
        lazyPaths = out
        return out
    }

    /** The path at [i], built if this is the first time it is asked for. */
    fun path(i: Int): Path {
        val all = buildPaths()
        val kept = all[i]
        if (kept !== LAZY) return kept
        val p = Path()
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
        all[i] = p
        return p
    }

    private var lazyPaths: Array<Path>? = null
    private companion object { val LAZY = Path() }

    // ── words / text ──
    fun wordKey(i: Int) = words[i].wordKey
    fun wordForm(i: Int, form: Form = Form.RASM_UTHMANI): String = QvpNative.wordForm(h, i, form.id) ?: ""
    fun findWord(surah: Int, ayah: Int, word: Int): Int = QvpNative.findWord(h, surah, ayah, word)
    fun target(s: String) = Target.parse(s, this)
    fun targetWords(t: Target): IntArray = QvpNative.targetWords(h, t.arr)
    fun targetWords(s: String) = targetWords(target(s))
    fun text(t: Target, form: Form = Form.RASM_UTHMANI, wordSep: String = " ", lineSep: String = "\n"): String = QvpNative.text(h, t.arr, form.id, wordSep, lineSep)
    fun text(s: String, form: Form = Form.RASM_UTHMANI, wordSep: String = " ", lineSep: String = "\n") = text(target(s), form, wordSep, lineSep)
    fun search(query: String, form: Form = Form.SEARCH, mode: SearchMode = SearchMode.INCLUDES, normalize: Boolean = true, looseMatch: Boolean = true, limit: Int = 0): List<QvpMatch> {
        val v = QvpNative.search(h, query, form.id, mode.id, normalize, looseMatch, limit)
        return List(v.size / 3) { k -> val w = v[k * 3]; QvpMatch(w, v[k * 3 + 1], v[k * 3 + 2] != 0, wordKey(w), words[w].text) }
    }
    fun citation(ws: IntArray): String = QvpNative.citation(h, ws)
    /** attach a words sidecar ({"s:a:w": {"rasmImlai","qpc","rasm","search"}}); returns words updated, -1 on bad JSON */
    fun attachWords(json: ByteArray): Int = QvpNative.attachWords(h, json)
    fun attachWords(json: String) = attachWords(json.toByteArray())
    fun hasForm(form: Form) = QvpNative.hasForm(h, form.id)

    // ── metadata ──
    fun surahs(): List<QvpSurah> = List(QvpNative.surahCount(h)) { i -> val n = QvpNative.surahNums(h, i)!!; val s = QvpNative.surahNames(h, i)!!
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
    private fun hit(v: FloatArray?) = v?.let { QvpHit(it[0].toInt(), it[1].toInt(), it[2].toInt(), it[3].toInt(), it[4], it[5] > 0.5f) }
    fun hitTestExact(x: Float, y: Float) = hit(QvpNative.hitTestExact(h, x, y))
    fun hitTestExactView(viewX: Float, viewY: Float) = hit(QvpNative.hitTestExactView(h, viewX, viewY))
    /** gap-aware: every point on a printed line resolves to the word the reader meant */
    fun hitTest(x: Float, y: Float, o: QvpHitOptions = QvpHitOptions()) = hit(QvpNative.hitTest(h, x, y, o.maxDistance, o.gapBias, o.preferExact))
    fun hitTestView(viewX: Float, viewY: Float, o: QvpHitOptions = QvpHitOptions()) = hit(QvpNative.hitTestView(h, viewX, viewY, o.maxDistance, o.gapBias, o.preferExact))
    fun lineBands(): List<QvpLineBand> { val v = QvpNative.lineBands(h); return List(v.size / 7) { k -> val o = k * 7; QvpLineBand(v[o].toInt(), v[o + 1].toInt(), v[o + 2], v[o + 3], v[o + 4], v[o + 5], v[o + 6]) } }
    fun hitAreas(gapBias: Float = QvpDefaults.GAP_BIAS): List<QvpHitArea> { val v = QvpNative.hitAreas(h, gapBias); return List(v.size / 10) { k -> val o = k * 10; QvpHitArea(v[o].toInt(), v[o + 1].toInt(), v[o + 2], v[o + 3], v[o + 4], v[o + 5], v[o + 6], v[o + 7], v[o + 8], v[o + 9]) } }

    // ── layout ──
    fun layout(spec: QvpLayoutSpec): QvpLayout {
        val v = QvpNative.layout(h, spec.floats())
        val n = v[6].toInt()
        return QvpLayout(v[0], v[1], v[2], v[3], v[4], v[5], FloatArray(n) { v[10 + it * 3] }, FloatArray(n) { v[11 + it * 3] }, FloatArray(n) { v[12 + it * 3] }, v[7], v[8], v[9]).also { currentLayout = it }
    }
    /** Leading (page units) that makes this page fill the padded viewport of [spec]; max 0 = unlimited. */
    fun layoutLineSpacingToFill(spec: QvpLayoutSpec, max: Float = 0f): Float = QvpNative.layoutLineSpacingToFill(h, spec.floats(), max)
    /** The share of the padded viewport left empty when the page is fitted to width. */
    fun layoutWastedFraction(spec: QvpLayoutSpec): Float = QvpNative.layoutWastedFraction(h, spec.floats())
    /** Read back the layout the page already has, without computing one: what to call after the
     * engine laid the page out itself, as the zoom control does. */
    fun readLayout(): QvpLayout? {
        val v = QvpNative.layoutCurrent(h) ?: return null
        val n = v[6].toInt()
        return QvpLayout(v[0], v[1], v[2], v[3], v[4], v[5], FloatArray(n) { v[12 + it * 3] }, FloatArray(n) { v[13 + it * 3] }, FloatArray(n) { v[14 + it * 3] },
                         v[7], v[8], v[9], v[10] != 0f, v[11].toInt()).also { currentLayout = it }
    }

    // ── the reader's zoom control ──
    /** One frame of a pinch: `factor` is the distance between the fingers against their distance
     * when they went down, and (x, y) the point between them. */
    fun zoomPinch(spec: QvpLayoutSpec, zoom: QvpZoom, view: QvpView, factor: Float, x: Float, y: Float): QvpZoomChange =
        change(QvpNative.zoomPinch(h, spec.floats(), zoom.floats(), view.floats(), factor, x, y))
    /** The control moved straight to a step: a size button, a double tap, a reset. Step 0 is printed. */
    fun zoomToStep(spec: QvpLayoutSpec, zoom: QvpZoom, step: Int, view: QvpView): QvpZoomChange =
        change(QvpNative.zoomToStep(h, spec.floats(), zoom.floats(), step, view.floats()))
    /** The same control under another policy, keeping the size the reader is at. */
    fun zoomMode(spec: QvpLayoutSpec, zoom: QvpZoom, mode: QvpZoomMode): QvpZoom =
        QvpZoom.of(QvpNative.zoomMode(h, spec.floats(), zoom.floats(), mode.id))
    /** The same control on this page: what a page turn keeps. A step carries as a step. */
    fun zoomCarried(spec: QvpLayoutSpec, zoom: QvpZoom): QvpZoom =
        QvpZoom.of(QvpNative.zoomCarried(h, spec.floats(), zoom.floats()))
    /** `spec` with this control's zoom in it: what the host lays out and draws with. */
    fun zoomSpec(spec: QvpLayoutSpec, zoom: QvpZoom): QvpLayoutSpec {
        val v = QvpNative.zoomSpec(h, spec.floats(), zoom.floats())
        val r = if (v[12] > 0f) QvpReflowSpec(v[12], spec.reflow?.fill ?: QvpFill.CENTRED, spec.reflow?.breaks ?: QvpBreaks.FITTED,
                                              spec.reflow?.gaps ?: QvpGapMode.UNIFORM, spec.reflow?.wordGap ?: 1f,
                                              spec.reflow?.relax ?: QvpDefaults.REFLOW_RELAX, spec.reflow?.maxStretch ?: QvpDefaults.REFLOW_MAX_STRETCH) else null
        return spec.copy(reflow = r)
    }
    /** The reflow zoom one step of this page's control means; step 0 is the printed page. */
    fun zoomAtStep(spec: QvpLayoutSpec, step: Int): Float = QvpNative.zoomAtStep(h, spec.floats(), step)
    /** The zoom steps this page ships with, lowest first. Zoom 1, the printed page, is the step before them. */
    fun zoomSteps(spec: QvpLayoutSpec): FloatArray = QvpNative.zoomSteps(h, spec.floats())
    /** The zoom each step of a reader's control lands on for this page, searched rather than read. */
    fun zoomLevels(spec: QvpLayoutSpec, nominals: FloatArray? = null, band: Float = 0f): FloatArray = QvpNative.zoomLevels(h, spec.floats(), nominals, band)
    /** Every zoom the search weighs for one step, as {zoom, cost} pairs. */
    fun zoomLevelCandidates(spec: QvpLayoutSpec, nominal: Float, band: Float = 0f, floor: Float = 0f): FloatArray =
        QvpNative.zoomLevelCandidates(h, spec.floats(), nominal, band, floor)
    /** The largest reflow zoom at which every word of this page still fits a row. */
    fun reflowMaxZoom(spec: QvpLayoutSpec): Float = QvpNative.reflowMaxZoom(h, spec.floats())
    /** What a sideways drag on this page means: pan it, or turn the page. */
    fun sidewaysDrag(zoom: QvpZoom, view: QvpView, fitScale: Float = 0f): QvpSideways =
        if (QvpNative.sidewaysDrag(h, zoom.floats(), view.floats(), fitScale) == 1) QvpSideways.TURN_PAGE else QvpSideways.PAN
    private fun change(v: FloatArray) = QvpZoomChange(QvpZoom.of(v), QvpView(v[3], v[4], v[5]), v[6] != 0f)
        .also { if (it.relaid) readLayout() }

    // ── a reflowed page: where the ink went ──
    /** Everything the current layout draws, in drawing order. `band` holds it to a band of the
     * laid-out page, in viewport px; null draws the whole page. */
    fun layoutDrawList(band: Pair<Float, Float>? = null): List<QvpDraw> {
        val v = QvpNative.layoutDrawList(h, band?.first ?: 0f, band?.second ?: 0f)
        return (0 until v.size / 2).map { QvpDraw(v[it * 2], v[it * 2 + 1]) }
    }
    /** What a draw list's placement indexes: every group, then every repeat. */
    fun layoutPlacements(): List<QvpPlacement> {
        val v = QvpNative.layoutPlacements(h)
        return (0 until v.size / 4).map { QvpPlacement(v[it * 4], v[it * 4 + 1], v[it * 4 + 2], v[it * 4 + 3]) }
    }
    /** The words of a reflowed row, in reading order. */
    fun rowWords(row: Int): IntArray = QvpNative.layoutRowWords(h, row)
    /** The row a word landed on in a reflowed layout, or null. */
    fun wordRow(word: Int): Int? = QvpNative.layoutWordRow(h, word).let { if (it == -1) null else it }

    /** The grid this page is laid out inside. */
    val grid: QvpGrid get() = QvpNative.pageGrid(h).let { QvpGrid(it[0].toInt(), it[1]) }
    fun wordBoundsView(i: Int): FloatArray? = QvpNative.wordBoundsView(h, i)

    // ── styles (handles undo exactly) ──
    fun style(sel: Selector, rgba: Int, transitionMs: Int = 0, layer: Int = QvpLayer.BASE): Int = QvpNative.styleAdd(h, layer, sel.arr, rgba, transitionMs)
    fun styleTarget(t: Target, rgba: Int, transitionMs: Int = 0, layer: Int = QvpLayer.BASE): Int = QvpNative.styleAddTarget(h, layer, t.arr, rgba, transitionMs)
    fun removeStyle(handle: Int) = QvpNative.styleRemove(h, handle)
    fun recolorStyle(handle: Int, rgba: Int, transitionMs: Int = 0) = QvpNative.styleRecolor(h, handle, rgba, transitionMs)
    fun hide(sel: Selector): Int = QvpNative.hide(h, sel.arr)
    fun clearStyles() = QvpNative.styleClear(h)
    fun clearLayer(layer: Int) = QvpNative.styleClearLayer(h, layer)
    fun setDefaultColor(rgba: Int) { defaultInk = rgba; QvpNative.styleDefaultColor(h, rgba) }
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
    fun colors(): IntArray = QvpNative.colors(h)
    /** flat pairs [pathIdx, rgba, ...] for paths ≠ default ink */
    fun styledPaths(): IntArray = QvpNative.styledPaths(h)
    fun colorOf(path: Int) = QvpNative.colorOf(h, path)

    // ── highlights ──
    fun highlight(t: Target, style: QvpHighlightStyle = QvpHighlightStyle()): Int = QvpNative.highlight(h, t.arr, style.ints(), style.floats())
    fun highlight(s: String, style: QvpHighlightStyle = QvpHighlightStyle()) = highlight(target(s), style)
    fun moveHighlight(handle: Int, t: Target) = QvpNative.moveHighlight(h, handle, t.arr)
    fun restyleHighlight(handle: Int, style: QvpHighlightStyle) = QvpNative.restyleHighlight(h, handle, style.ints(), style.floats())
    fun removeHighlight(handle: Int) = QvpNative.removeHighlight(h, handle)
    fun clearHighlights() = QvpNative.clearHighlights(h)
    fun highlightHandles(): IntArray = QvpNative.highlightHandles(h)
    fun highlightWords(handle: Int): IntArray = QvpNative.highlightWords(h, handle)
    private fun boxes(v: IntArray): List<QvpBox> = List(v.size / 8) { k -> val o = k * 8; QvpBox(v[o], v[o + 1], Float.fromBits(v[o + 2]), Float.fromBits(v[o + 3]), Float.fromBits(v[o + 4]), Float.fromBits(v[o + 5]), v[o + 6], Float.fromBits(v[o + 7])) }
    /** animated band boxes in viewport px; draw each id as one nonzero path behind the ink */
    fun highlightBoxesView(): List<QvpBox> = boxes(QvpNative.highlightBoxesView(h))
    fun wordBands(ws: IntArray, height: BandHeight = BandHeight.LINE_SPACING, padX: Float = QvpDefaults.HIGHLIGHT_PAD_X, padY: Float = QvpDefaults.HIGHLIGHT_PAD_Y) = boxes(QvpNative.wordBands(h, ws, height.id, padX, padY))

    // ── selection ──
    fun select(anchor: Int, focus: Int = anchor) = QvpNative.select(h, anchor, focus)
    fun clearSelection() = QvpNative.select(h, -1, -1)
    fun selection(): IntArray = QvpNative.selection(h)
    fun selectionText(form: Form = Form.RASM_UTHMANI, includeCitation: Boolean = false) = QvpNative.selectionText(h, form.id, includeCitation)

    // ── memorisation ──
    fun mask(t: Target, mode: MaskMode = MaskMode.HIDE) = QvpNative.mask(h, t.arr, mode.id)
    fun maskFrom(wordIndex: Int, mode: MaskMode = MaskMode.HIDE) = QvpNative.maskFrom(h, wordIndex, mode.id)
    fun maskOptions(blockColor: Int = QvpDefaults.MASK_BLOCK, padX: Float = QvpDefaults.MASK_PAD, padY: Float = QvpDefaults.MASK_PAD, radius: Float = QvpDefaults.MASK_RADIUS, reverse: Boolean = false) = QvpNative.maskOptions(h, blockColor, padX, padY, radius, reverse)
    fun unmaskNext(n: Int = 1) = QvpNative.unmaskNext(h, n)
    fun maskBack(n: Int = 1) = QvpNative.maskBack(h, n)
    fun unmaskWord(wordIndex: Int) = QvpNative.unmaskWord(h, wordIndex)
    fun maskWord(wordIndex: Int) = QvpNative.maskWord(h, wordIndex)
    fun unmaskAll() = QvpNative.unmaskAll(h)
    fun maskAll() = QvpNative.maskAll(h)
    fun unmask() = QvpNative.unmask(h)
    fun maskHidden(): IntArray = QvpNative.maskHidden(h)
    fun maskWords(): IntArray = QvpNative.maskWords(h)
    fun maskBoxesView(): List<QvpBox> = boxes(QvpNative.maskBoxesView(h))
    /** greyed page with a lit window; returns steps */
    fun revealStart(lit: Int = QvpDefaults.REVEAL_LIT, byAyah: Boolean = false, grey: Int = QvpDefaults.REVEAL_GREY, ink: Int = QvpDefaults.INK, ayahMarks: Boolean = true, transitionMs: Int = 0) = QvpNative.revealStart(h, lit, byAyah, grey, ink, ayahMarks, transitionMs)
    fun revealGoto(at: Long) = QvpNative.revealGoto(h, at)
    fun revealPosition(): Long? = QvpNative.revealPosition(h).let { if (it == -2L) null else it }
    fun revealStepCount() = QvpNative.revealStepCount(h)
    fun revealStepOf(wordIndex: Int) = QvpNative.revealStepOf(h, wordIndex)
    fun revealStop() = QvpNative.revealStop(h)

    // ── crop ──
    fun cropBounds(t: Target, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Boolean = true): QvpCropBounds? = QvpNative.cropBounds(h, t.arr, pad, keepAyahMarks)?.let { QvpCropBounds(it[0], it[1], it[2], it[3], it[4].toInt(), it[5].toInt()) }
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
    fun pageCount() = QvpNative.atlasPageCount(h)
    fun surah(n: Int) = surah(QvpNative.atlasSurah(h, n))
    fun surahs(): List<QvpAtlasSurah> = List(QvpNative.atlasSurahCount(h)) { surah(QvpNative.atlasSurahAt(h, it))!! }
    fun pageOfSurah(n: Int) = surah(n)?.page
    fun division(division: Division, n: Int): QvpAtlasRubuAlHizb? = QvpNative.atlasDivision(h, division.id, n)?.let { QvpAtlasRubuAlHizb(it[0], it[1], it[2], it[3]) }
    fun juz(n: Int) = division(Division.JUZ, n)
    fun hizb(n: Int) = division(Division.HIZB, n)
    fun rubuAlHizb(n: Int) = division(Division.RUBU_AL_HIZB, n)
    fun divisionOf(division: Division, surah: Int, ayah: Int): Int? = QvpNative.atlasDivisionOf(h, division.id, surah, ayah).let { if (it < 0) null else it }
    fun juzOf(surah: Int, ayah: Int) = divisionOf(Division.JUZ, surah, ayah)
    fun pagesOfJuz(n: Int): Pair<Int, Int>? = QvpNative.atlasPagesOfJuz(h, n)?.let { it[0] to it[1] }
    fun searchSurahs(text: String): List<QvpAtlasSurah> = QvpNative.atlasSearchSurahs(h, text).toList().mapNotNull { n -> surah(n) }
    @Synchronized
    override fun close() {
        val handle = nativeHandle
        if (handle != 0L) {
            nativeHandle = 0L
            QvpNative.atlasFree(handle)
        }
    }
}
