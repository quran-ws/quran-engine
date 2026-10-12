package ws.quran.qvp.rn

import android.graphics.Canvas
import android.graphics.Paint
import android.util.Log
import android.view.View
import com.facebook.react.uimanager.ThemedReactContext
import ws.quran.qvp.*

/**
 * The React Native host of a passage: a range of complete ayahs, or a surah's basmalah, laid out on
 * rows of this view's width and drawn here. The engine loads, measures and places every word; this
 * class reads the pages the props name, hands the engine a spec in dp, draws the layout and reports
 * it to JS. Nothing is decided here.
 */
class QvpRnPassageView(private val ctx: ThemedReactContext) : View(ctx) {
    private val density = ctx.resources.displayMetrics.density
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)

    // ── desired props (set by the manager) ──
    var pageUris: List<String> = emptyList()
    var surahProp = 0
    var fromProp = 0
    var toProp = 0
    var basmalahProp = false
    /** dp per page unit; [lineHeightProp] wins when it is set. */
    var scaleProp = 1f
    /** dp per printed line: the scale is this over the passage's printed line spacing. */
    var lineHeightProp = 0f
    var lineSpacingProp = 1f
    var paddingProp = 2f
    var alignProp: String? = null
    var maxRowsProp = 0
    var keepAyahMarkProp = false
    var ellipsisWidthProp = 0f
    var maxStretchProp = 0f
    var inkProp: String? = null
    var paperColor = 0

    // ── applied state ──
    private var loadedKey: String? = null
    private var pages: List<QvpPage> = emptyList()
    private var pagesByNumber: Map<Int, QvpPage> = emptyMap()
    private var passage: QvpPassage? = null
    private var layoutKey: String? = null
    private var layout: QvpPassageLayout? = null

    fun destroy() {
        passage?.close(); passage = null
        pages.forEach { it.close() }; pages = emptyList(); pagesByNumber = emptyMap()
        loadedKey = null; layoutKey = null; layout = null
    }

    /** Called by the manager after every prop batch. */
    fun commit() {
        try {
            ensurePassage()
            relayout()
            invalidate()
        } catch (e: Exception) {
            Log.e("QvpRn", "passage commit failed", e)
            emit("onError", mapOf("message" to (e.message ?: e.toString())))
        }
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        if (w != oldw) commit()
    }

    private fun ensurePassage() {
        val key = "${pageUris.joinToString("|")}#$surahProp:$fromProp-$toProp:$basmalahProp"
        if (key == loadedKey) return
        destroy()
        loadedKey = key
        if (pageUris.isEmpty() || surahProp <= 0) return
        // The passage copies what it needs, but a draw names a path by its page, so the pages stay
        // open for as long as the passage is drawn. Each page goes into `pages` as it opens, so a
        // URI that fails after it leaves nothing for `destroy` to miss.
        val loaded = ArrayList<QvpPage>(pageUris.size)
        pages = loaded
        for (uri in pageUris) loaded.add(QvpPage(QvpRnBytes.read(ctx, uri)).also { p -> p.buildPaths() })
        pagesByNumber = loaded.associateBy { it.pageNo }
        passage = if (basmalahProp) QvpPassage.basmalah(loaded, surahProp)
                  else QvpPassage.load(loaded, surahProp, fromProp, if (toProp > 0) toProp else fromProp)
    }

    private fun relayout() {
        val p = passage ?: return
        if (width <= 0) return
        val widthDp = width / density
        // dp per page unit: the host's scale, or the dp it wants one printed line to take
        val scale = if (lineHeightProp > 0f && p.lineSpacing > 0f) lineHeightProp / p.lineSpacing else scaleProp
        val align = when (alignProp) {
            "center", "centre" -> QvpPassageAlign.CENTER
            "justified" -> QvpPassageAlign.JUSTIFIED
            else -> QvpPassageAlign.RIGHT
        }
        val maxRows = maxRowsProp.coerceAtLeast(0)
        // The view draws the ellipsis after a cut, so it knows how wide one is: three dots as
        // tall as a row, unless the host reserved a width of its own.
        val ellipsisWidth = if (ellipsisWidthProp > 0f || maxRows == 0) ellipsisWidthProp else {
            paint.textSize = scale * p.lineSpacing
            paint.measureText(ELLIPSIS)
        }
        val spec = QvpPassageSpec(width = widthDp, scale = scale, lineSpacing = lineSpacingProp, padding = paddingProp,
                                  align = align, maxRows = maxRows, keepAyahMark = keepAyahMarkProp,
                                  ellipsisWidth = ellipsisWidth, maxStretch = maxStretchProp)
        val key = spec.toString()
        if (key == layoutKey) return
        val l = p.layout(spec)
        layoutKey = key
        layout = l
        if (l == null) {
            emit("onError", mapOf("message" to "the passage spec is out of range: $spec"))
            return
        }
        emit("onPassageLayout", Marshal.passageLayoutMap(l, p))
    }

    override fun onDraw(canvas: Canvas) {
        val l = layout ?: return
        if (paperColor != 0) { paint.color = paperColor; canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), paint) }
        paint.color = Marshal.color(inkProp, QvpDefaults.INK).let { QvpColor.argb(it) }
        canvas.save()
        // the layout is in dp; the canvas is in px
        canvas.scale(density, density)
        l.draw(canvas, pagesByNumber, paint)
        // The engine leaves the ellipsis to the host: three dots in the ink, as tall as the row
        // the cut left room on.
        l.ellipsis?.let { e ->
            paint.textSize = e.height()
            paint.textAlign = Paint.Align.CENTER
            canvas.drawText(ELLIPSIS, e.centerX(), e.bottom, paint)
        }
        canvas.restore()
    }

    private fun emit(name: String, payload: Map<String, Any?>) = QvpRnEvents.emit(ctx, this, name, payload)

    private companion object {
        const val ELLIPSIS = "\u2026"
    }
}
