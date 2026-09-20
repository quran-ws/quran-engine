package ws.quran.qvp.rn

import com.facebook.react.bridge.*
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.UIManagerHelper
import ws.quran.qvp.*
import ws.quran.qvp.Target

/**
 * Promise-based queries on the page behind a `<QvpPageView />` (looked up by react tag) plus engine-wide
 * helpers and the atlas. Everything runs on the UI thread, where the view draws, so the engine is never
 * touched from two threads. Names follow docs/API.md.
 */
@ReactModule(name = QvpModule.NAME)
class QvpModule(private val ctx: ReactApplicationContext) : ReactContextBaseJavaModule(ctx) {
    override fun getName() = NAME
    private val atlases = HashMap<Int, QvpAtlas>()
    private val atlasByUri = HashMap<String, Int>()
    private var nextAtlas = 1

    private fun view(tag: Int): QvpRnPageView? = QvpRegistry.get(tag) ?: runCatching { UIManagerHelper.getUIManagerForReactTag(ctx, tag)?.resolveView(tag) as? QvpRnPageView }.getOrNull()
    // Every @ReactMethod must be `void` for the TurboModule interop layer: these helpers return Unit on purpose.
    private fun ui(promise: Promise, block: () -> Any?) {
        UiThreadUtil.runOnUiThread {
            try { promise.resolve(Marshal.toJs(block())) } catch (e: Throwable) { promise.reject("qvp", e.message ?: e.toString(), e) }
        }
    }
    private fun withPage(tag: Int, promise: Promise, block: (QvpRnPageView, QvpPage) -> Any?) {
        ui(promise) {
            val v = view(tag) ?: throw IllegalStateException("no QvpPageView with tag $tag")
            val p = v.page ?: throw IllegalStateException("QvpPageView $tag has no page loaded")
            block(v, p)
        }
    }
    private fun withAtlas(id: Int, promise: Promise, block: (QvpAtlas) -> Any?) { ui(promise) { block(atlases[id] ?: throw IllegalStateException("no atlas $id")) } }
    private fun opt(m: ReadableMap?): Map<String, Any?> = m?.toHashMap() ?: emptyMap()
    private fun tgt(d: Dynamic, p: QvpPage): Target = Marshal.target(when (d.type) {
        ReadableType.String -> d.asString(); ReadableType.Number -> d.asDouble(); ReadableType.Array -> d.asArray()?.toArrayList(); ReadableType.Map -> d.asMap()?.toHashMap(); else -> null }, p)
    private fun ints(a: ReadableArray?): IntArray = a?.toArrayList()?.mapNotNull { (it as? Number)?.toInt() }?.toIntArray() ?: IntArray(0)

    // ── page facts ──
    @ReactMethod fun info(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> Marshal.pageInfo(p) + mapOf("loadMs" to v.loadMs, "bytes" to v.pageBytes) }
    @ReactMethod fun words(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.words.map { Marshal.word(p, it) } }
    @ReactMethod fun word(tag: Int, index: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.words.getOrNull(index)?.let { Marshal.word(p, it) } }
    @ReactMethod fun ayahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahs.map { Marshal.ayah(it) } }
    @ReactMethod fun lines(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.lines.map { Marshal.line(it) } }
    @ReactMethod fun decorations(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.decorations.map { Marshal.decoration(it) } }
    @ReactMethod fun findWord(tag: Int, s: Int, a: Int, w: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.findWord(s, a, w) }
    @ReactMethod fun targetWords(tag: Int, target: Dynamic, promise: Promise) = withPage(tag, promise) { _, p -> p.targetWords(tgt(target, p)).toList() }
    @ReactMethod fun wordForm(tag: Int, index: Int, form: String?, promise: Promise) = withPage(tag, promise) { _, p -> p.wordForm(index, Marshal.form(form)) }
    @ReactMethod fun hasForm(tag: Int, form: String?, promise: Promise) = withPage(tag, promise) { _, p -> p.hasForm(Marshal.form(form)) }
    @ReactMethod fun attachWords(tag: Int, json: String, promise: Promise) = withPage(tag, promise) { _, p -> p.attachWords(json) }

    // ── metadata ──
    @ReactMethod fun surahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.surahs().map { Marshal.surah(it) } }
    @ReactMethod fun divisions(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.divisions().map { Marshal.division(it) } }
    @ReactMethod fun ayahMarks(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahMarks().map { Marshal.ayahMark(it) } }
    @ReactMethod fun rosettes(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.rosettes().map { Marshal.rosette(it) } }
    @ReactMethod fun sajdahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.sajdahs().map { Marshal.sajdah(it) } }
    @ReactMethod fun ayahKeys(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahKeys().map { mapOf("surah" to it.first, "ayah" to it.second, "ayahKey" to "${it.first}:${it.second}") } }
    @ReactMethod fun ayahWordCount(tag: Int, s: Int, a: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahWordCount(s, a).let { mapOf("count" to it.first, "isComplete" to it.second) } }
    @ReactMethod fun reciteMap(tag: Int, s: Int, a: Int, n: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.reciteMap(s, a, n)?.toList() }
    @ReactMethod fun wordLabel(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.wordLabel(i) }
    @ReactMethod fun ayahLabel(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahLabel(i) }

    // ── text & search ──
    @ReactMethod fun text(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.text(tgt(target, p), Marshal.form(o["form"]), (o["wordSep"] as? String) ?: " ", (o["lineSep"] as? String) ?: "\n") }
    @ReactMethod fun search(tag: Int, query: String, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts)
        p.search(query, Marshal.form(o["form"], Form.SEARCH), Marshal.searchMode(o["mode"]), o["normalize"] != false, o["looseMatch"] != false, (o["limit"] as? Number)?.toInt() ?: 0).map { Marshal.match(it) } }
    @ReactMethod fun citation(tag: Int, words: ReadableArray, promise: Promise) = withPage(tag, promise) { _, p -> p.citation(ints(words)) }
    @ReactMethod fun arabic(op: String, s: String, promise: Promise) = ui(promise) { when (op) { "strip" -> QvpEngine.strip(s); "fold" -> QvpEngine.fold(s); "normalize" -> QvpEngine.normalize(s); "loose", "looseKey" -> QvpEngine.looseKey(s); else -> s } }

    // ── hit testing / layout (the view already does gestures; these are for scroll-into-view etc.) ──
    private fun hitOptions(o: Map<String, Any?>) = QvpHitOptions((o["maxDistance"] as? Number)?.toFloat() ?: QvpDefaults.TAP_DISTANCE, (o["gapBias"] as? Number)?.toFloat() ?: QvpDefaults.GAP_BIAS, o["preferExact"] != false)
    /** Exact outline hit at a point in page units. */
    @ReactMethod fun hitTestExact(tag: Int, x: Double, y: Double, promise: Promise) = withPage(tag, promise) { _, p -> p.hitTestExact(x.toFloat(), y.toFloat())?.let { Marshal.hit(p, it) } }
    /** Gap-aware hit at a point in page units. */
    @ReactMethod fun hitTest(tag: Int, x: Double, y: Double, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p -> p.hitTest(x.toFloat(), y.toFloat(), hitOptions(opt(opts)))?.let { Marshal.hit(p, it) } }
    /** Exact outline hit at a point in view dp (density and pan/zoom removed here, the layout in the engine). */
    @ReactMethod fun hitTestExactView(tag: Int, x: Double, y: Double, promise: Promise) = withPage(tag, promise) { v, p -> val d = v.resources.displayMetrics.density
        p.hitTestExactView(((x * d).toFloat() - v.inner.viewOx) / v.inner.viewScale, ((y * d).toFloat() - v.inner.viewOy) / v.inner.viewScale)?.let { Marshal.hit(p, it) } }
    @ReactMethod fun hitTestView(tag: Int, x: Double, y: Double, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { v, p ->
        val o = opt(opts); val d = v.resources.displayMetrics.density
        p.hitTestView(((x * d).toFloat() - v.inner.viewOx) / v.inner.viewScale, ((y * d).toFloat() - v.inner.viewOy) / v.inner.viewScale, hitOptions(o))?.let { Marshal.hit(p, it) } }
    @ReactMethod fun wordBoundsView(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> val d = v.resources.displayMetrics.density
        p.wordBoundsView(i)?.let { b -> mapOf("x0" to (v.inner.viewOx + b[0] * v.inner.viewScale) / d, "y0" to (v.inner.viewOy + b[1] * v.inner.viewScale) / d, "x1" to (v.inner.viewOx + b[2] * v.inner.viewScale) / d, "y1" to (v.inner.viewOy + b[3] * v.inner.viewScale) / d) } }
    @ReactMethod fun currentLayout(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.currentLayout?.let { Marshal.layout(it) } }
    @ReactMethod fun relayout(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.inner.relayout(); v.resetView(); null }
    @ReactMethod fun resetView(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.resetView(); null }
    /** The leading that fills this view's viewport, from the spec the view lays out with. */
    @ReactMethod fun layoutLineSpacingToFill(tag: Int, max: Double, promise: Promise) = withPage(tag, promise) { v, p -> p.layoutLineSpacingToFill(v.inner.layoutSpec(), max.toFloat()) }
    @ReactMethod fun layoutWastedFraction(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.layoutWastedFraction(v.inner.layoutSpec()) }
    /** The page's height at the printed pitch for the spec this view lays out with. */
    @ReactMethod fun layoutPrintedHeight(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.layoutPrintedHeight(v.inner.layoutSpec()) }
    @ReactMethod fun grid(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> mapOf("lines" to p.grid.lines, "lineSpacing" to p.grid.lineSpacing) }
    @ReactMethod fun stats(tag: Int, promise: Promise) = ui(promise) { view(tag)?.stats() }
    @ReactMethod fun invalidate(tag: Int, promise: Promise) = ui(promise) { view(tag)?.inner?.invalidate(); null }

    // ── selection ──
    @ReactMethod fun select(tag: Int, anchor: Int, focus: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.select(anchor, focus); v.inner.invalidate(); Marshal.selection(p) }
    @ReactMethod fun clearSelection(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.inner.clearSelection(); null }
    @ReactMethod fun selection(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> Marshal.selection(p) }
    @ReactMethod fun selectionText(tag: Int, form: String?, includeCitation: Boolean, promise: Promise) = withPage(tag, promise) { _, p -> p.selectionText(Marshal.form(form), includeCitation) }

    // ── memorisation (stepwise ops; `mask` / `reveal` props hold the declarative part) ──
    @ReactMethod fun unmaskNext(tag: Int, n: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.unmaskNext(n).also { v.inner.invalidate() } }
    @ReactMethod fun maskBack(tag: Int, n: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.maskBack(n).also { v.inner.invalidate() } }
    @ReactMethod fun unmaskWord(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.unmaskWord(i).also { v.inner.invalidate() } }
    @ReactMethod fun maskWord(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.maskWord(i).also { v.inner.invalidate() } }
    @ReactMethod fun unmaskAll(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.unmaskAll(); v.inner.invalidate(); null }
    @ReactMethod fun maskAll(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.maskAll(); v.inner.invalidate(); null }
    @ReactMethod fun maskHidden(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.maskHidden().toList() }
    @ReactMethod fun maskWords(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.maskWords().toList() }
    @ReactMethod fun revealStepCount(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealStepCount() }
    @ReactMethod fun revealPosition(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealPosition()?.toDouble() }
    @ReactMethod fun revealStepOf(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealStepOf(i).toDouble() }

    // ── crop ──
    @ReactMethod fun cropBounds(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.cropBounds(tgt(target, p), (o["pad"] as? Number)?.toFloat() ?: QvpDefaults.CROP_PAD, o["keepAyahMarks"] != false)?.let { Marshal.cropBounds(it) } }
    @ReactMethod fun cropSvg(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.cropSvg(tgt(target, p), (o["pad"] as? Number)?.toFloat() ?: QvpDefaults.CROP_PAD, o["keepAyahMarks"] != false, Marshal.color(o["background"], 0)) }

    // ── atlas ──
    @ReactMethod fun loadAtlas(uri: String, promise: Promise) = ui(promise) {
        atlasByUri[uri] ?: run {
            val bytes = when {
                uri.startsWith("asset://") -> ctx.assets.open(uri.removePrefix("asset://")).use { it.readBytes() }
                uri.startsWith("asset:") -> ctx.assets.open(uri.removePrefix("asset:")).use { it.readBytes() }
                uri.startsWith("file://") -> java.io.File(uri.removePrefix("file://")).readBytes()
                uri.startsWith("/") -> java.io.File(uri).readBytes()
                uri.startsWith("base64:") -> android.util.Base64.decode(uri.removePrefix("base64:"), android.util.Base64.DEFAULT)
                else -> ctx.assets.open(uri).use { it.readBytes() }
            }
            val id = nextAtlas++; atlases[id] = QvpAtlas(bytes); atlasByUri[uri] = id; id
        }
    }
    @ReactMethod fun freeAtlas(id: Int, promise: Promise) = ui(promise) { atlases.remove(id)?.close(); atlasByUri.entries.removeAll { it.value == id }; null }
    @ReactMethod fun atlasPageOf(id: Int, s: Int, a: Int, promise: Promise) = withAtlas(id, promise) { it.pageOf(s, a) }
    @ReactMethod fun atlasPageRange(id: Int, page: Int, promise: Promise) = withAtlas(id, promise) { it.pageRange(page)?.let { r -> mapOf("first" to mapOf("surah" to r.first.first, "ayah" to r.first.second), "last" to mapOf("surah" to r.second.first, "ayah" to r.second.second)) } }
    @ReactMethod fun atlasPageCount(id: Int, promise: Promise) = withAtlas(id, promise) { it.pageCount() }
    @ReactMethod fun atlasSurah(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.surah(n)?.let { s -> Marshal.atlasSurah(s) } }
    @ReactMethod fun atlasSurahs(id: Int, promise: Promise) = withAtlas(id, promise) { it.surahs().map { s -> Marshal.atlasSurah(s) } }
    @ReactMethod fun atlasPageOfSurah(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.pageOfSurah(n) }
    @ReactMethod fun atlasDivision(id: Int, division: String, n: Int, promise: Promise) = withAtlas(id, promise) { it.division(Marshal.division(division), n)?.let { r -> Marshal.atlasRubuAlHizb(r) } }
    @ReactMethod fun atlasDivisionOf(id: Int, division: String, s: Int, a: Int, promise: Promise) = withAtlas(id, promise) { it.divisionOf(Marshal.division(division), s, a) }
    @ReactMethod fun atlasJuzOf(id: Int, s: Int, a: Int, promise: Promise) = withAtlas(id, promise) { it.juzOf(s, a) }
    @ReactMethod fun atlasPagesOfJuz(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.pagesOfJuz(n)?.let { r -> listOf(r.first, r.second) } }
    @ReactMethod fun atlasSearchSurahs(id: Int, text: String, promise: Promise) = withAtlas(id, promise) { it.searchSurahs(text).map { s -> Marshal.atlasSurah(s) } }

    // ── names ──
    @ReactMethod fun markName(m: Int, promise: Promise) = ui(promise) { QvpEngine.markName(m) }
    @ReactMethod fun markFromName(s: String, promise: Promise) = ui(promise) { QvpEngine.markFromName(s) }
    @ReactMethod fun markCategory(m: Int, promise: Promise) = ui(promise) { QvpEngine.markCategory(m) }
    @ReactMethod fun engineName(promise: Promise) = ui(promise) { QvpEngine.engineName() }
    /** Every name table, read from the engine; the JavaScript side resolves names with these. */
    override fun getConstants(): Map<String, Any> = mapOf("version" to QvpEngine.version(), "formatVersion" to QvpEngine.formatVersion(),
        "marks" to QvpEngine.names(QvpEngine.Names.MARK), "kinds" to QvpEngine.names(QvpEngine.Names.KIND),
        "families" to QvpEngine.names(QvpEngine.Names.FAMILY), "categories" to QvpEngine.names(QvpEngine.Names.CATEGORY),
        "decorations" to QvpEngine.names(QvpEngine.Names.DECORATION), "divisions" to QvpEngine.names(QvpEngine.Names.DIVISION),
        "places" to QvpEngine.names(QvpEngine.Names.PLACE))

    override fun invalidate() { atlases.values.forEach { it.close() }; atlases.clear(); atlasByUri.clear(); super.invalidate() }
    companion object { const val NAME = "QvpModule" }
}
