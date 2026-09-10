package net.quranpedia.qvp

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
        set(v) { field = v; base = null; baseKey = ""; selectionHandle = 0; relayout(); resetView(); invalidate() }
    // layout knobs (viewport size comes from the view)
    var padTop = 0f; var padBottom = 0f; var padSide = 0f; var lineSpacing = 1f; var lineGap = 0f; var fillHeight = false
    var paperColor: Int = Color.TRANSPARENT           // ARGB
    var selectionBand: Int = 0x2d6fd640                // 0xRRGGBBAA
    var onWordTap: ((QvpWord, QvpHitEx) -> Unit)? = null
    var onDecoTap: ((QvpDecoration, QvpHitEx) -> Unit)? = null
    var onEmptyTap: (() -> Unit)? = null
    var onSelectionChanged: ((IntArray) -> Unit)? = null
    var zoomEnabled = true
    var selectionEnabled = true
    var hitOptions = QvpHitOptions(maxDistance = 6f)

    var viewScale = 1f; var viewOx = 0f; var viewOy = 0f
    private var base: Bitmap? = null
    private var baseKey = ""
    /** The dress, rasterised: it never changes while a highlight animates. */
    private var ornBitmap: Bitmap? = null
    private var ornKey = ""
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val m = Matrix()
    private val rect = RectF()
    private var selectionHandle = 0
    private var selAnchor = -1
    private var selecting = false
    var lastBaseMs = 0.0; var lastOverlayMs = 0.0; var lastBasePaths = 0; var lastOverlayPaths = 0; var lastBands = 0; var lastHitUs = 0.0
        private set
    var animating = false; private set

    private val frameCb = object : Choreographer.FrameCallback {
        override fun doFrame(frameTimeNanos: Long) { animating = false; invalidate() }
    }

    private val scaleDetector = ScaleGestureDetector(context, object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScale(d: ScaleGestureDetector): Boolean {
            if (!zoomEnabled || selecting) return false
            val ns = (viewScale * d.scaleFactor).coerceIn(0.5f, 12f); val kk = ns / viewScale
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

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_UP || event.actionMasked == MotionEvent.ACTION_CANCEL) {
            if (selecting) { selecting = false; parent?.requestDisallowInterceptTouchEvent(false) }
        }
        if (!selecting) scaleDetector.onTouchEvent(event)
        gestureDetector.onTouchEvent(event); return true
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) { relayout(); resetView() }

    /** Recompute the engine layout for the current size/knobs. */
    fun relayout() {
        val p = page ?: return
        if (width == 0 || height == 0) return
        p.layout(QvpLayoutSpec(width.toFloat(), height.toFloat(), padTop, padBottom, padSide, padSide, lineSpacing, lineGap, fillHeight))
        baseKey = ""; invalidate()
    }
    /**
     * Fit the content and centre it.
     *
     * A DRESSED PAGE IS BIGGER THAN ITS TEXT: the border was drawn around it, so
     * fitting to the content box alone crops the border off. The engine says by
     * how much, on all four sides, and nothing here has to know why.
     */
    fun resetView() {
        val p = page; val l = p?.currentLayout
        if (p == null || l == null || width <= 0 || height <= 0) { viewScale = 1f; viewOx = 0f; viewOy = 0f; invalidate(); return }
        val o = p.dressOverflow()
        val left = o[0]; val top = o[1]; val right = o[2]; val bottom = o[3]
        val w = l.contentW + left + right; val h = l.contentH + top + bottom
        viewScale = minOf(if (h > height && h > 0) height / h else 1f, if (w > width && w > 0) width / w else 1f)
        viewOx = ((width - w * viewScale) / 2f).coerceAtLeast(0f) + left * viewScale
        viewOy = ((height - h * viewScale) / 2f).coerceAtLeast(0f) + top * viewScale
        invalidate()
    }
    /**
     * Matrix mapping page units of [line] to view px (layout + pan/zoom). A
     * negative line takes no line shift at all: that is the page's own frame,
     * which the border is placed from and which no layout moves.
     */
    fun lineMatrix(line: Int, out: Matrix = m): Matrix {
        val l = page?.currentLayout
        val ls = l?.scale ?: 1f; val lox = l?.ox ?: 0f; val loy = (l?.oy ?: 0f) + (if (line >= 0) l?.lineDy?.getOrNull(line) ?: 0f else 0f) * ls
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

    /**
     * The ornament layer, BEHIND everything else. A draw carries the page line it
     * was measured against, so it moves with that line's ink under a layout that
     * respaces the page; the border belongs to no line and never moves.
     *
     * RASTERISED ONCE, like the base ink. A tiled border is hundreds of filled
     * and stroked outlines and nothing about it changes while a highlight
     * animates, so re-drawing it every frame is what makes playback stutter.
     */
    private fun drawOrnaments(canvas: Canvas, p: QvpPage) {
        val rev = p.dressRevision()
        if (rev == 0) { ornBitmap = null; ornKey = ""; return }
        val key = "$viewScale|$viewOx|$viewOy|$rev|${width}x$height"
        var b = ornBitmap
        if (b == null || key != ornKey || b.width != width || b.height != height) {
            if (width <= 0 || height <= 0) return
            if (b == null || b.width != width || b.height != height) { b = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888); ornBitmap = b }
            b.eraseColor(Color.TRANSPARENT)
            val bc = Canvas(b)
            bc.save()
            var cur = -2
            for (o in p.dressGeometry()) {
                if (o.line != cur) { bc.restore(); bc.save(); bc.concat(lineMatrix(o.line)); cur = o.line }
                paint.color = QvpColor.argb(o.color)
                if (o.stroke) {
                    paint.style = Paint.Style.STROKE; paint.strokeWidth = o.strokeWidth
                    paint.strokeCap = Paint.Cap.ROUND; paint.strokeJoin = Paint.Join.ROUND
                    bc.drawPath(o.path, paint)
                    paint.style = Paint.Style.FILL
                } else {
                    bc.drawPath(o.path, paint)
                }
            }
            bc.restore()
            ornKey = key
        }
        canvas.drawBitmap(ornBitmap!!, 0f, 0f, null)
    }

    override fun onDraw(canvas: Canvas) {
        val p = page ?: return
        if (p.currentLayout == null) relayout()
        val l = p.currentLayout ?: return
        val moving = p.tick(System.nanoTime() / 1e6)
        val paths = p.buildPaths()
        val styled = p.styled()
        val styledSet = HashSet<Int>(styled.size / 2); var i = 0
        while (i < styled.size) { styledSet.add(styled[i]); i += 2 }
        val ink = p.defaultInk
        val key = "$viewScale|$viewOx|$viewOy|$ink|${l.pitch}|${l.lineDy.contentHashCode()}|${styledSet.sorted().hashCode()}|$width|$height"
        if (paperColor != Color.TRANSPARENT) { paint.color = paperColor; canvas.drawRect(viewOx, viewOy, viewOx + l.contentW * viewScale, viewOy + l.contentH * viewScale, paint) }
        drawOrnaments(canvas, p)
        val bands = p.highlightBoxes()
        drawBoxes(canvas, bands)
        var b = base
        if (b == null || key != baseKey || b.width != width || b.height != height) {
            val t0 = System.nanoTime()
            if (b == null || b.width != width || b.height != height) { b = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888); base = b }
            b.eraseColor(Color.TRANSPARENT)
            val bc = Canvas(b); paint.color = QvpColor.argb(ink)
            var cur = -1; var n = 0
            for (pi in 0 until p.nPaths) {
                if (pi in styledSet) continue
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
}
