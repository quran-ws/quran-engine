package ws.quran.qvp

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.util.AttributeSet
import android.view.Choreographer
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.View

/**
 * Host-canvas renderer for a [QvpPage]. Draw order per frame:
 * highlight bands (one path per highlight, behind the ink) → cached base ink → styled ink → mask boxes.
 * Each frame calls `page.tick(now)` and keeps animating while the engine says so.
 * Gestures: tap → gap-aware hit-test → [onWordTap]/[onDecoTap]/[onEmptyTap]; long-press-drag → whole-word
 * selection (engine `select`, band in the selection layer); pinch/pan on top of the engine layout.
 */
class QvpPageView @JvmOverloads constructor(context: Context, attrs: AttributeSet? = null) : View(context, attrs) {
    var page: QvpPage? = null
        set(v) {
            if (field === v) return
            Choreographer.getInstance().removeFrameCallback(frameCb)
            animating = false
            field = v
            releaseBase()
            baseKey = ""
            selectionHandle = 0
            relayout()
            resetView()
            invalidate()
        }
    // layout knobs (viewport size comes from the view)
    var padTop = 0f; var padBottom = 0f; var padSide = 0f; var lineSpacing = 1f; var lineGap = 0f; var fillHeight = false
    var paperColor: Int = Color.TRANSPARENT           // ARGB
    var selectionBand: Int = QvpDefaults.SELECTION_BAND  // 0xRRGGBBAA
    var onWordTap: ((QvpWord, QvpHitEx) -> Unit)? = null
    var onDecoTap: ((QvpDecoration, QvpHitEx) -> Unit)? = null
    var onEmptyTap: (() -> Unit)? = null
    var onSelectionChanged: ((IntArray) -> Unit)? = null
    var zoomEnabled = true
    var selectionEnabled = true
    var hitOptions = QvpHitOptions(maxDistance = QvpDefaults.TAP_DISTANCE)

    var viewScale = 1f; var viewOx = 0f; var viewOy = 0f
    private var base: Bitmap? = null
    private var baseKey = ""
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val m = Matrix()
    private val rect = RectF()
    private val styledPaths = HashSet<Int>()
    private var selectionHandle = 0
    private var selAnchor = -1
    private var selecting = false
    var lastBaseMs = 0.0; var lastOverlayMs = 0.0; var lastBasePaths = 0; var lastOverlayPaths = 0; var lastBands = 0; var lastHitUs = 0.0
        private set
    var animating = false; private set

    private val frameCb = object : Choreographer.FrameCallback {
        override fun doFrame(frameTimeNanos: Long) { animating = false; invalidate() }
    }

    private fun releaseBase() {
        base?.recycle()
        base = null
    }

    private val scaleDetector = ScaleGestureDetector(context, object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScale(d: ScaleGestureDetector): Boolean {
            if (!zoomEnabled || selecting) return false
            val ns = (viewScale * d.scaleFactor).coerceIn(MIN_ZOOM, MAX_ZOOM); val kk = ns / viewScale
            viewOx = d.focusX - (d.focusX - viewOx) * kk; viewOy = d.focusY - (d.focusY - viewOy) * kk; viewScale = ns
            invalidate(); return true
        }
    })
    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onScroll(e1: MotionEvent?, e2: MotionEvent, dx: Float, dy: Float): Boolean {
            if (selecting) { extendSelection(e2.x, e2.y); return true }
            if (!zoomEnabled) return false
            viewOx -= dx; viewOy -= dy; invalidate(); return true
        }
        override fun onDoubleTap(e: MotionEvent): Boolean { resetView(); return true }
        override fun onLongPress(e: MotionEvent) {
            if (!selectionEnabled) return
            val h = hitAt(e.x, e.y) ?: return
            if (h.word < 0) return
            selecting = true; selAnchor = h.word
            page?.select(h.word, h.word); paintSelection()
            parent?.requestDisallowInterceptTouchEvent(true)
        }
        override fun onSingleTapConfirmed(e: MotionEvent): Boolean {
            val p = page ?: return false
            val hit = hitAt(e.x, e.y)
            when {
                hit == null -> onEmptyTap?.invoke()
                hit.word >= 0 -> onWordTap?.invoke(p.words[hit.word], hit)
                hit.deco >= 0 -> onDecoTap?.invoke(p.decos[hit.deco], hit)
                else -> onEmptyTap?.invoke()
            }
            performClick()
            return true
        }
    })

    private fun hitAt(x: Float, y: Float): QvpHitEx? {
        val p = page ?: return null
        val t0 = System.nanoTime()
        val h = p.hitTestViewEx((x - viewOx) / viewScale, (y - viewOy) / viewScale, hitOptions)
        lastHitUs = (System.nanoTime() - t0) / 1000.0
        return h
    }
    private fun extendSelection(x: Float, y: Float) {
        val p = page ?: return
        val h = p.hitTestViewEx((x - viewOx) / viewScale, (y - viewOy) / viewScale, QvpHitOptions()) ?: return
        if (h.word < 0) return
        p.select(selAnchor, h.word); paintSelection()
    }
    private fun paintSelection() {
        val p = page ?: return
        val ws = p.selection()
        val t = Target.words(ws)
        if (selectionHandle != 0) p.rehighlight(selectionHandle, t)
        else selectionHandle = p.highlight(t, QvpHighlightStyle(mode = HighlightMode.BAND, band = selectionBand, padX = 0.6f, layer = QvpLayer.SELECTION))
        onSelectionChanged?.invoke(ws); invalidate()
    }
    /** Clear the selection band and the engine selection. */
    fun clearSelection() {
        val p = page ?: return
        p.clearSelection(); if (selectionHandle != 0) { p.unhighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?.invoke(IntArray(0)); invalidate()
    }

    @SuppressLint("ClickableViewAccessibility") // GestureDetector calls performClick for confirmed taps.
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_UP || event.actionMasked == MotionEvent.ACTION_CANCEL) {
            if (selecting) { selecting = false; parent?.requestDisallowInterceptTouchEvent(false) }
        }
        if (!selecting) scaleDetector.onTouchEvent(event)
        gestureDetector.onTouchEvent(event); return true
    }

    override fun performClick(): Boolean {
        super.performClick()
        return true
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) { relayout(); resetView() }

    override fun onDetachedFromWindow() {
        Choreographer.getInstance().removeFrameCallback(frameCb)
        animating = false
        releaseBase()
        super.onDetachedFromWindow()
    }

    /** Recompute the engine layout for the current size/knobs. */
    /** The spec `relayout()` hands the engine for the current size and knobs. */
    fun layoutSpec() = QvpLayoutSpec(width.toFloat(), height.toFloat(), padTop, padBottom, padSide, padSide, lineSpacing, lineGap, fillHeight)
    fun relayout() {
        val p = page ?: return
        if (width == 0 || height == 0) return
        p.layout(layoutSpec())
        baseKey = ""; invalidate()
    }
    fun resetView() {
        val l = page?.currentLayout
        viewScale = l?.fitScale ?: 1f; viewOx = l?.fitX ?: 0f; viewOy = l?.fitY ?: 0f
        invalidate()
    }
    /** Matrix mapping page units of [line] to view px (layout + pan/zoom). */
    fun lineMatrix(line: Int, out: Matrix = m): Matrix {
        val l = page?.currentLayout
        val ls = l?.scale ?: 1f; val lox = l?.ox ?: 0f; val loy = (l?.oy ?: 0f) + (l?.lineDy?.getOrNull(line) ?: 0f) * ls
        out.reset(); out.setScale(viewScale * ls, viewScale * ls); out.postTranslate(viewOx + viewScale * lox, viewOy + viewScale * loy)
        return out
    }
    private fun drawBoxes(canvas: Canvas, boxes: List<QvpBox>) {
        if (boxes.isEmpty()) return
        canvas.save(); canvas.translate(viewOx, viewOy); canvas.scale(viewScale, viewScale)
        var curId = Int.MIN_VALUE; var curColor = 0; var path: Path? = null
        fun flush() { path?.let { paint.color = QvpColor.argb(curColor); canvas.drawPath(it, paint) }; path = null }
        for (b in boxes) {
            if (path == null || b.id != curId || b.color != curColor) { flush(); path = Path().apply { fillType = Path.FillType.WINDING }; curId = b.id; curColor = b.color }
            rect.set(b.x0, b.y0, b.x1, b.y1)
            if (b.radius > 0f) path!!.addRoundRect(rect, b.radius, b.radius, Path.Direction.CW) else path!!.addRect(rect, Path.Direction.CW)
        }
        flush(); canvas.restore()
    }

    override fun onDraw(canvas: Canvas) {
        val p = page ?: return
        if (p.currentLayout == null) relayout()
        val l = p.currentLayout ?: return
        val moving = p.tick(System.nanoTime() / 1e6)
        val paths = p.buildPaths()
        val styled = p.styled()
        styledPaths.clear(); var i = 0; var styledKey = 1
        while (i < styled.size) {
            val pi = styled[i]
            styledPaths.add(pi)
            styledKey = 31 * styledKey + pi
            i += 2
        }
        val ink = p.defaultInk
        val key = "$viewScale|$viewOx|$viewOy|$ink|${l.pitch}|${l.lineDy.contentHashCode()}|$styledKey|$width|$height"
        if (paperColor != Color.TRANSPARENT) { paint.color = paperColor; canvas.drawRect(viewOx, viewOy, viewOx + l.contentW * viewScale, viewOy + l.contentH * viewScale, paint) }
        val bands = p.highlightBoxes()
        drawBoxes(canvas, bands)
        var b = base
        if (b == null || key != baseKey || b.width != width || b.height != height) {
            val t0 = System.nanoTime()
            if (b == null || b.width != width || b.height != height) {
                b?.recycle()
                b = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
                base = b
            }
            b.eraseColor(Color.TRANSPARENT)
            val bc = Canvas(b); paint.color = QvpColor.argb(ink)
            var cur = -1; var n = 0
            for (pi in 0 until p.nPaths) {
                if (pi in styledPaths) continue
                val ln = p.pathLine(pi); if (ln != cur) { bc.setMatrix(lineMatrix(ln)); cur = ln }
                bc.drawPath(paths[pi], paint); n++
            }
            baseKey = key; lastBaseMs = (System.nanoTime() - t0) / 1e6; lastBasePaths = n
        }
        canvas.drawBitmap(b, 0f, 0f, null)
        val t1 = System.nanoTime()
        var cur = -1; i = 0
        canvas.save()
        while (i < styled.size) {
            val pi = styled[i]; val col = styled[i + 1]; i += 2
            if (col and 0xff == 0) continue
            val ln = p.pathLine(pi)
            if (ln != cur) { canvas.restore(); canvas.save(); canvas.concat(lineMatrix(ln)); cur = ln }
            paint.color = QvpColor.argb(col); canvas.drawPath(paths[pi], paint)
        }
        canvas.restore()
        drawBoxes(canvas, p.maskBoxes())
        lastOverlayMs = (System.nanoTime() - t1) / 1e6; lastOverlayPaths = styled.size / 2; lastBands = bands.size
        if (moving) { animating = true; Choreographer.getInstance().postFrameCallback(frameCb) }
    }

    companion object {
        /** Pinch limits as multiples of the fitted scale; the same pair on every platform. */
        const val MIN_ZOOM = 0.5f
        const val MAX_ZOOM = 12f
    }
}
