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
    @ReactMethod fun word(tag: Int, idx: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.words.getOrNull(idx)?.let { Marshal.word(p, it) } }
    @ReactMethod fun ayahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahs.map { Marshal.ayah(it) } }
    @ReactMethod fun lines(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.lines.map { Marshal.line(it) } }
    @ReactMethod fun decos(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.decos.map { Marshal.deco(it) } }
    @ReactMethod fun findWord(tag: Int, s: Int, a: Int, w: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.findWord(s, a, w) }
    @ReactMethod fun resolve(tag: Int, target: Dynamic, promise: Promise) = withPage(tag, promise) { _, p -> p.resolve(tgt(target, p)).toList() }
    @ReactMethod fun wordForm(tag: Int, idx: Int, form: String?, promise: Promise) = withPage(tag, promise) { _, p -> p.wordForm(idx, Marshal.form(form)) }
    @ReactMethod fun hasForm(tag: Int, form: String?, promise: Promise) = withPage(tag, promise) { _, p -> p.hasForm(Marshal.form(form)) }
    @ReactMethod fun attachWords(tag: Int, json: String, promise: Promise) = withPage(tag, promise) { _, p -> p.attachWords(json) }

    // ── metadata ──
    @ReactMethod fun surahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.surahs().map { Marshal.surah(it) } }
    @ReactMethod fun divisions(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.divisions().map { Marshal.division(it) } }
    @ReactMethod fun ayahMarks(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahMarks().map { Marshal.ayahMark(it) } }
    @ReactMethod fun rosettes(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.rosettes().map { Marshal.rosette(it) } }
    @ReactMethod fun sajdahs(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.sajdahs().map { Marshal.sajdah(it) } }
    @ReactMethod fun ayahKeys(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahKeys().map { mapOf("surah" to it.first, "ayah" to it.second, "ayahKey" to "${it.first}:${it.second}") } }
    @ReactMethod fun ayahWordCount(tag: Int, s: Int, a: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahWordCount(s, a).let { mapOf("count" to it.first, "complete" to it.second) } }
    @ReactMethod fun reciteMap(tag: Int, s: Int, a: Int, n: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.reciteMap(s, a, n)?.toList() }
    @ReactMethod fun wordLabel(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.wordLabel(i) }
    @ReactMethod fun ayahLabel(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.ayahLabel(i) }

    // ── text & search ──
    @ReactMethod fun text(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.text(tgt(target, p), Marshal.form(o["form"]), (o["wordSep"] as? String) ?: " ", (o["lineSep"] as? String) ?: "\n") }
    @ReactMethod fun search(tag: Int, query: String, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts)
        p.search(query, Marshal.form(o["form"], Form.SEARCH), Marshal.searchMode(o["mode"]), o["normalize"] != false, o["loose"] != false, (o["limit"] as? Number)?.toInt() ?: 0).map { Marshal.match(it) } }
    @ReactMethod fun citation(tag: Int, words: ReadableArray, promise: Promise) = withPage(tag, promise) { _, p -> p.citation(ints(words)) }
    @ReactMethod fun arabic(kind: String, s: String, promise: Promise) = ui(promise) { when (kind) { "strip" -> QvpEngine.strip(s); "fold" -> QvpEngine.fold(s); "normalize" -> QvpEngine.normalize(s); "loose", "looseKey" -> QvpEngine.looseKey(s); else -> s } }

    // ── hit testing / layout (the view already does gestures; these are for scroll-into-view etc.) ──
    @ReactMethod fun hitTestViewEx(tag: Int, x: Double, y: Double, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { v, p ->
        val o = opt(opts); val d = v.resources.displayMetrics.density
        p.hitTestViewEx(((x * d).toFloat() - v.inner.viewOx) / v.inner.viewScale, ((y * d).toFloat() - v.inner.viewOy) / v.inner.viewScale, QvpHitOptions((o["maxDistance"] as? Number)?.toFloat() ?: 6f, (o["gapBias"] as? Number)?.toFloat() ?: 0.6f, o["exactFirst"] != false))?.let { Marshal.hit(p, it) } }
    @ReactMethod fun wordBoxView(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> val d = v.resources.displayMetrics.density
        p.wordBoxView(i)?.let { b -> mapOf("x0" to (v.inner.viewOx + b[0] * v.inner.viewScale) / d, "y0" to (v.inner.viewOy + b[1] * v.inner.viewScale) / d, "x1" to (v.inner.viewOx + b[2] * v.inner.viewScale) / d, "y1" to (v.inner.viewOy + b[3] * v.inner.viewScale) / d) } }
    @ReactMethod fun currentLayout(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.currentLayout?.let { Marshal.layout(it) } }
    @ReactMethod fun relayout(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.inner.relayout(); v.resetView(); null }
    @ReactMethod fun resetView(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.resetView(); null }
    @ReactMethod fun gapToFill(pw: Double, ph: Double, lines: Int, vw: Double, vh: Double, max: Double, promise: Promise) = ui(promise) { QvpEngine.gapToFill(pw.toFloat(), ph.toFloat(), lines, vw.toFloat(), vh.toFloat(), max.toFloat()) }
    @ReactMethod fun wastedFraction(pw: Double, ph: Double, vw: Double, vh: Double, promise: Promise) = ui(promise) { QvpEngine.wastedFraction(pw.toFloat(), ph.toFloat(), vw.toFloat(), vh.toFloat()) }
    @ReactMethod fun stats(tag: Int, promise: Promise) = ui(promise) { view(tag)?.stats() }
    @ReactMethod fun invalidate(tag: Int, promise: Promise) = ui(promise) { view(tag)?.inner?.invalidate(); null }

    // ── selection ──
    @ReactMethod fun select(tag: Int, anchor: Int, focus: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.select(anchor, focus); v.inner.invalidate(); Marshal.selection(p) }
    @ReactMethod fun clearSelection(tag: Int, promise: Promise) = withPage(tag, promise) { v, _ -> v.inner.clearSelection(); null }
    @ReactMethod fun selection(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> Marshal.selection(p) }
    @ReactMethod fun selectionText(tag: Int, form: String?, citation: Boolean, promise: Promise) = withPage(tag, promise) { _, p -> p.selectionText(Marshal.form(form), citation) }

    // ── memorisation (stepwise ops; `mask` / `reveal` props hold the declarative part) ──
    @ReactMethod fun revealNext(tag: Int, n: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.revealNext(n).also { v.inner.invalidate() } }
    @ReactMethod fun hideBack(tag: Int, n: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.hideBack(n).also { v.inner.invalidate() } }
    @ReactMethod fun revealWord(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.revealWord(i).also { v.inner.invalidate() } }
    @ReactMethod fun hideWord(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.hideWord(i).also { v.inner.invalidate() } }
    @ReactMethod fun revealAll(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.revealAll(); v.inner.invalidate(); null }
    @ReactMethod fun hideAll(tag: Int, promise: Promise) = withPage(tag, promise) { v, p -> p.hideAll(); v.inner.invalidate(); null }
    @ReactMethod fun maskHidden(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.maskHidden().toList() }
    @ReactMethod fun maskWords(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.maskWords().toList() }
    @ReactMethod fun revealSteps(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealSteps() }
    @ReactMethod fun revealAt(tag: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealAt()?.toDouble() }
    @ReactMethod fun revealStepOf(tag: Int, i: Int, promise: Promise) = withPage(tag, promise) { _, p -> p.revealStepOf(i).toDouble() }

    // ── crop ──
    @ReactMethod fun cropBox(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.cropBox(tgt(target, p), (o["pad"] as? Number)?.toFloat() ?: 2f, o["keepAyahMarks"] != false)?.let { Marshal.cropBox(it) } }
    @ReactMethod fun cropSvg(tag: Int, target: Dynamic, opts: ReadableMap?, promise: Promise) = withPage(tag, promise) { _, p ->
        val o = opt(opts); p.cropSvg(tgt(target, p), (o["pad"] as? Number)?.toFloat() ?: 2f, o["keepAyahMarks"] != false, Marshal.color(o["background"], 0)) }

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
    @ReactMethod fun atlasPages(id: Int, promise: Promise) = withAtlas(id, promise) { it.pages() }
    @ReactMethod fun atlasSurah(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.surah(n)?.let { s -> Marshal.atlasSurah(s) } }
    @ReactMethod fun atlasSurahs(id: Int, promise: Promise) = withAtlas(id, promise) { it.surahs().map { s -> Marshal.atlasSurah(s) } }
    @ReactMethod fun atlasPageOfSurah(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.pageOfSurah(n) }
    @ReactMethod fun atlasDivision(id: Int, kind: String, n: Int, promise: Promise) = withAtlas(id, promise) { it.division(Marshal.division(kind), n)?.let { r -> Marshal.atlasRubuAlHizb(r) } }
    @ReactMethod fun atlasDivisionAt(id: Int, kind: String, s: Int, a: Int, promise: Promise) = withAtlas(id, promise) { it.divisionAt(Marshal.division(kind), s, a) }
    @ReactMethod fun atlasJuzAt(id: Int, s: Int, a: Int, promise: Promise) = withAtlas(id, promise) { it.juzAt(s, a) }
    @ReactMethod fun atlasPagesOfJuz(id: Int, n: Int, promise: Promise) = withAtlas(id, promise) { it.pagesOfJuz(n)?.let { r -> listOf(r.first, r.second) } }
    @ReactMethod fun atlasFindSurah(id: Int, text: String, promise: Promise) = withAtlas(id, promise) { it.findSurah(text).map { s -> Marshal.atlasSurah(s) } }

    // ── names ──
    @ReactMethod fun markName(m: Int, promise: Promise) = ui(promise) { QvpEngine.markName(m) }
    @ReactMethod fun markFromName(s: String, promise: Promise) = ui(promise) { QvpEngine.markFromName(s) }
    override fun getConstants(): Map<String, Any> = mapOf("version" to QvpEngine.version(), "marks" to QVP_MARKS,
        "kinds" to (0..4).map { QvpEngine.kindName(it) }, "families" to (0..7).map { QvpEngine.familyName(it) }, "categories" to (0..8).map { QvpEngine.categoryName(it) })

    override fun invalidate() { atlases.values.forEach { it.close() }; atlases.clear(); atlasByUri.clear(); super.invalidate() }
    companion object { const val NAME = "QvpModule" }
}
