package net.quranpedia.qvp.rn

import android.util.Base64
import android.util.Log
import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.WritableMap
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.events.Event
import android.widget.FrameLayout
import net.quranpedia.qvp.*
import net.quranpedia.qvp.Target
import java.io.File

/**
 * The React Native host of [QvpPageView]. It owns the loaded [QvpPage], loads bytes from the prop-given
 * URI, and reconciles the declarative props (theme / styles / highlights / mask / reveal) against engine
 * handles so JS never sees a handle. Every decision (hit-test, layout, style resolution, bands, masks)
 * is the engine's; this class only marshals and forwards.
 */
class QvpRnPageView(private val ctx: ThemedReactContext) : FrameLayout(ctx) {
    private val density = ctx.resources.displayMetrics.density
    /** The Kotlin library's renderer (final class, so hosted rather than subclassed). */
    val inner = QvpPageView(ctx).also { addView(it, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT)) }
    var page: QvpPage?
        get() = inner.page
        set(v) { inner.page = v }
    var paperColor: Int get() = inner.paperColor; set(v) { inner.paperColor = v; inner.invalidate() }
    var selectionBand: Int get() = inner.selectionBand; set(v) { inner.selectionBand = v }
    var selectionEnabled: Boolean get() = inner.selectionEnabled; set(v) { inner.selectionEnabled = v }
    var zoomEnabled: Boolean get() = inner.zoomEnabled; set(v) { inner.zoomEnabled = v }
    var hitOptions: QvpHitOptions get() = inner.hitOptions; set(v) { inner.hitOptions = v }

    /** React Native sizes this host directly (no measure pass), so size the child by hand. */
    override fun onLayout(changed: Boolean, l: Int, t: Int, r: Int, b: Int) {
        val w = r - l; val h = b - t
        inner.measure(MeasureSpec.makeMeasureSpec(w, MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec(h, MeasureSpec.EXACTLY))
        inner.layout(0, 0, w, h)
        centre()
    }
    /** Fit + centre: the library's resetView fits by height and pins the pan at x = 0; centre the pan horizontally (pan is host state). */
    fun resetView() { inner.resetView(); centre() }
    private fun centre() {
        val l = page?.currentLayout ?: return
        inner.viewOx = ((inner.width - l.contentW * inner.viewScale) / 2f).coerceAtLeast(0f); inner.invalidate()
    }

    // ── desired props (set by the manager) ──
    var pageUri: String? = null
    var pageBase64: String? = null
    var wordsUri: String? = null
    var defaultInkProp: String? = null
    var themeProp: Map<String, Any?>? = null
    var stylesProp: List<Any?>? = null
    var highlightsProp: List<Any?>? = null
    var maskProp: Map<String, Any?>? = null
    var revealProp: Map<String, Any?>? = null
    var layoutDirty = true

    // ── applied state ──
    private var loadedKey: String? = null
    private var appliedTheme: Map<String, Any?>? = null
    private var themeHandle = 0
    private class Rule(val handle: Int, val entry: Map<*, *>)
    private val styleHandles = LinkedHashMap<String, Rule>()
    private val highlightHandles = LinkedHashMap<String, Rule>()
    private var appliedMask: Map<String, Any?>? = null
    private var appliedReveal: Map<String, Any?>? = null   // without "at"
    private var appliedRevealAt: Long? = null
    private var appliedInk: Int? = null
    var revealSteps = 0; private set
    var loadMs = 0.0; private set
    var pageBytes = 0; private set

    init {
        inner.onWordTap = { w, h -> page?.let { p -> emit("onWordTap", mapOf("word" to Marshal.word(p, w), "hit" to Marshal.hit(p, h))) } }
        inner.onDecoTap = { d, h -> page?.let { p -> emit("onDecoTap", mapOf("deco" to Marshal.deco(d), "hit" to Marshal.hit(p, h))) } }
        inner.onEmptyTap = { emit("onEmptyTap", emptyMap()) }
        inner.onSelectionChanged = { page?.let { p -> emit("onSelectionChanged", Marshal.selection(p)) } }
    }

    override fun onAttachedToWindow() { super.onAttachedToWindow(); QvpRegistry.register(this) }
    override fun onDetachedFromWindow() { QvpRegistry.unregister(this); super.onDetachedFromWindow() }

    fun destroy() { QvpRegistry.unregister(this); page?.close(); page = null; loadedKey = null }

    /** Padding props arrive in dp. */
    fun setPadTopDp(v: Float) { inner.padTop = v * density; layoutDirty = true }
    fun setPadBottomDp(v: Float) { inner.padBottom = v * density; layoutDirty = true }
    fun setPadSideDp(v: Float) { inner.padSide = v * density; layoutDirty = true }
    fun setLineSpacingProp(v: Float) { inner.lineSpacing = v; layoutDirty = true }
    fun setLineGapProp(v: Float) { inner.lineGap = v; layoutDirty = true }
    fun setFillHeightProp(v: Boolean) { inner.fillHeight = v; layoutDirty = true }

    // ── bytes ──
    private fun readUri(uri: String): ByteArray {
        val u = uri.trim()
        return when {
            u.startsWith("asset://") -> ctx.assets.open(u.removePrefix("asset://")).use { it.readBytes() }
            u.startsWith("asset:") -> ctx.assets.open(u.removePrefix("asset:")).use { it.readBytes() }
            u.startsWith("file://") -> File(u.removePrefix("file://")).readBytes()
            u.startsWith("/") -> File(u).readBytes()
            u.startsWith("base64:") -> Base64.decode(u.removePrefix("base64:"), Base64.DEFAULT)
            else -> ctx.assets.open(u).use { it.readBytes() }
        }
    }

    /** Called by the manager after every prop batch. */
    fun commit() {
        try {
            ensurePage()
            val p = page
            if (p != null) {
                applyInk(p); applyTheme(p); applyStyles(p); applyHighlights(p); applyMask(p); applyReveal(p)
            }
            if (layoutDirty) { layoutDirty = false; inner.relayout(); resetView() }
            inner.invalidate()
        } catch (e: Exception) {
            Log.e("QvpRn", "commit failed", e)
            emit("onError", mapOf("message" to (e.message ?: e.toString())))
        }
    }

    private fun ensurePage() {
        val key = pageBase64?.let { "b64:" + it.hashCode() } ?: pageUri
        if (key == loadedKey) return
        // drop everything bound to the old page: handles die with it
        themeHandle = 0; appliedTheme = null; styleHandles.clear(); highlightHandles.clear(); appliedMask = null; appliedReveal = null; appliedRevealAt = null; appliedInk = null; revealSteps = 0
        page?.close(); page = null; loadedKey = key
        if (key == null) return
        val bytes = pageBase64?.let { Base64.decode(it, Base64.DEFAULT) } ?: readUri(pageUri!!)
        val t0 = System.nanoTime()
        val p = QvpPage(bytes); p.buildPaths()
        loadMs = (System.nanoTime() - t0) / 1e6; pageBytes = bytes.size
        wordsUri?.let { runCatching { p.attachWords(readUri(it)) }.onFailure { e -> Log.w("QvpRn", "words sidecar: $e") } }
        page = p; layoutDirty = true
        emit("onPageLoad", Marshal.pageInfo(p) + mapOf("loadMs" to loadMs, "bytes" to pageBytes, "uri" to pageUri))
    }

    private fun applyInk(p: QvpPage) {
        val ink = Marshal.color(defaultInkProp, 0x231f20ff.toInt())
        if (appliedInk != ink) { appliedInk = ink; p.setDefaultInk(ink) }
    }
    private fun applyTheme(p: QvpPage) {
        if (themeProp == appliedTheme) return
        if (themeHandle != 0) { p.unstyle(themeHandle); themeHandle = 0 }
        appliedTheme = themeProp
        themeProp?.let { themeHandle = p.theme(Marshal.theme(it)) }
    }
    private fun entries(list: List<Any?>?): LinkedHashMap<String, Map<*, *>> {
        val out = LinkedHashMap<String, Map<*, *>>()
        list?.forEach { e -> (e as? Map<*, *>)?.let { m -> (m["id"]?.toString())?.let { out[it] = m } } }
        return out
    }
    private fun applyStyles(p: QvpPage) {
        val want = entries(stylesProp)
        for (id in styleHandles.keys.toList()) if (!want.containsKey(id)) { p.unstyle(styleHandles.remove(id)!!.handle) }
        for ((id, e) in want) {
            val old = styleHandles[id]
            if (old != null && old.entry == e) continue
            val sameShape = old != null && old.entry["selector"] == e["selector"] && old.entry["target"] == e["target"] && old.entry["layer"] == e["layer"] && old.entry["hide"] == e["hide"] && e["hide"] != true
            if (sameShape) { p.restyle(old!!.handle, Marshal.color(e["color"], 0), (e["ms"] as? Number)?.toInt() ?: 0); styleHandles[id] = Rule(old.handle, e); continue }
            if (old != null) p.unstyle(old.handle)
            val ms = (e["ms"] as? Number)?.toInt() ?: 0
            val layer = (e["layer"] as? Number)?.toInt() ?: QvpLayer.BASE
            val color = Marshal.color(e["color"], 0)
            val h = when {
                e["hide"] == true -> Marshal.selector(e["selector"])?.let { p.hide(it) } ?: 0
                e["selector"] != null -> Marshal.selector(e["selector"])?.let { p.style(it, color, ms, layer) } ?: 0
                e["target"] != null -> p.styleTarget(Marshal.target(e["target"], p), color, ms, layer)
                else -> 0
            }
            if (h != 0) styleHandles[id] = Rule(h, e) else styleHandles.remove(id)
        }
    }
    private fun applyHighlights(p: QvpPage) {
        val want = entries(highlightsProp)
        for (id in highlightHandles.keys.toList()) if (!want.containsKey(id)) { p.unhighlight(highlightHandles.remove(id)!!.handle) }
        for ((id, e) in want) {
            val old = highlightHandles[id]
            if (old != null && old.entry == e) continue
            val style = Marshal.highlightStyle(e["style"])
            if (old == null) { val h = p.highlight(Marshal.target(e["target"], p), style); if (h != 0) highlightHandles[id] = Rule(h, e); continue }
            if (old.entry["style"] != e["style"]) p.restyleHighlight(old.handle, style)
            if (old.entry["target"] != e["target"]) p.rehighlight(old.handle, Marshal.target(e["target"], p))
            highlightHandles[id] = Rule(old.handle, e)
        }
    }
    private fun applyMask(p: QvpPage) {
        if (maskProp == appliedMask) return
        appliedMask = maskProp
        p.unmask()
        val m = maskProp ?: return
        val paper = Marshal.color(m["blockColor"], 0xd9d4c8ff.toInt())
        p.maskOptions(paper, (m["padX"] as? Number)?.toFloat() ?: 0.6f, (m["padY"] as? Number)?.toFloat() ?: 0.6f, (m["radius"] as? Number)?.toFloat() ?: 0.8f, m["reverse"] == true)
        p.mask(Marshal.target(m["target"], p), Marshal.maskMode(m["mode"]))
    }
    private fun applyReveal(p: QvpPage) {
        val r = revealProp
        val cfg = r?.filterKeys { it != "at" }
        if (cfg != appliedReveal) {
            appliedReveal = cfg; appliedRevealAt = null
            if (cfg == null) { p.revealStop(); revealSteps = 0; emit("onRevealChanged", mapOf("steps" to 0, "at" to null)); return }
            revealSteps = p.revealStart((cfg["lit"] as? Number)?.toInt() ?: 1, cfg["byAyah"] == true, Marshal.color(cfg["grey"], 0xc9c4b8ff.toInt()), Marshal.color(cfg["ink"], appliedInk ?: 0x231f20ff.toInt()),
                cfg["ayahMarks"] != false, (cfg["ms"] as? Number)?.toInt() ?: 0)
            emit("onRevealChanged", mapOf("steps" to revealSteps, "at" to -1))
        }
        if (r == null) return
        val at = (r["at"] as? Number)?.toLong() ?: -1L
        if (at != appliedRevealAt) { appliedRevealAt = at; p.revealGoto(at) }
    }

    // ── events ──
    private class QvpEvent(surfaceId: Int, viewTag: Int, private val name: String, private val payload: WritableMap) : Event<QvpEvent>(surfaceId, viewTag) {
        override fun getEventName() = name
        override fun getEventData(): WritableMap = payload
    }
    fun emit(name: String, payload: Map<String, Any?>) {
        if (id <= 0) return
        val dispatcher = UIManagerHelper.getEventDispatcherForReactTag(ctx, id) ?: return
        dispatcher.dispatchEvent(QvpEvent(UIManagerHelper.getSurfaceId(this), id, name, Marshal.toMap(payload)))
    }

    fun stats(): Map<String, Any?> = mapOf("loadMs" to loadMs, "bytes" to pageBytes, "baseMs" to inner.lastBaseMs, "overlayMs" to inner.lastOverlayMs, "basePaths" to inner.lastBasePaths, "overlayPaths" to inner.lastOverlayPaths,
        "bands" to inner.lastBands, "hitUs" to inner.lastHitUs, "animating" to inner.animating, "styleHandles" to (page?.styleHandles()?.size ?: 0), "highlightHandles" to (page?.highlightHandles()?.size ?: 0),
        "engineVersion" to QvpEngine.version(), "viewScale" to inner.viewScale, "layout" to page?.currentLayout?.let { Marshal.layout(it) })
}

/** Views by react tag, so the module can find the page behind a `viewTag` without going through the UIManager. */
object QvpRegistry {
    private val views = HashMap<Int, QvpRnPageView>()
    fun register(v: QvpRnPageView) { if (v.id > 0) views[v.id] = v }
    fun unregister(v: QvpRnPageView) { views.entries.removeAll { it.value === v } }
    fun get(tag: Int): QvpRnPageView? = views[tag]
}
