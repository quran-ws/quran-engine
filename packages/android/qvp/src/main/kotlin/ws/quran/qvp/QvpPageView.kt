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
 * Gestures: tap → gap-aware hit-test → [onWordTap]/[onDecorationTap]/[onEmptyTap]; long-press-drag → whole-word
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
            // A step means the same size to the reader on any page: the engine chose every page's
            // steps so the ink barely changes across a turn. So the step carries over, and the new
            // page says what it means.
            if (v != null && v.isOpen && width > 0) zoom = v.zoomCarried(baseSpec(), zoom)
            relayout()
            resetView()
            invalidate()
        }
    // layout knobs (viewport size comes from the view)
    var padTop = 0f; var padBottom = 0f; var padSide = 0f; var lineSpacing = 1f; var fillHeight = false
    var paperColor: Int = Color.TRANSPARENT           // ARGB
    var selectionBand: Int = QvpDefaults.SELECTION_BAND  // 0xRRGGBBAA
    var onWordTap: ((QvpWord, QvpHit) -> Unit)? = null
    var onDecorationTap: ((QvpDecoration, QvpHit) -> Unit)? = null
    var onEmptyTap: (() -> Unit)? = null
    var onSelectionChanged: ((IntArray) -> Unit)? = null
    /** A sideways flick while the page has no sideways travel to spend, in pages: +1 the page
     * after this one, -1 the page before it. The muṣḥaf's own order, so a host adds it to the
     * page it is on and never has to think about which way the book runs. */
    var onSwipe: ((Int) -> Unit)? = null
    /** The size the reader is at changed: a pinch committed, or a step was asked for. */
    var onZoomChanged: ((QvpZoom) -> Unit)? = null
    var zoomEnabled = true
    var selectionEnabled = true
    /** What a pinch does to the page. Stepped is what a reader gets: the pinch lands on one of
     * the page's own zoom steps and the page breaks its rows again at that size. */
    var zoomMode: QvpZoomMode = QvpZoomMode.STEPPED
        set(v) {
            if (field == v) return
            field = v
            val p = page ?: return
            if (p.isOpen && width > 0) { zoom = p.zoomMode(baseSpec(), zoom, v); relayout(); resetView() }
        }
    /** Where the reader's zoom control stands, as the engine last answered. */
    var zoom = QvpZoom(); private set
    /** The zoom steps this page ships with: what stepped mode lands on, lowest first. */
    val zoomSteps: FloatArray get() = page?.takeIf { it.isOpen && width > 0 }?.zoomSteps(baseSpec()) ?: FloatArray(0)
    /** Move the control straight to a step. 0 is the printed page. */
    fun zoomToStep(step: Int) {
        val p = page ?: return
        if (!p.isOpen || width == 0) { pendingStep = step; return }
        pendingStep = null
        apply(p.zoomToStep(layoutSpec(), zoom, step, currentView))
    }
    private var pendingStep: Int? = null
    /** How much ink to keep ready at once. A page that fits is drawn once, the parts past both
     * ends of the screen included, and scrolling then draws nothing. A taller one keeps a band
     * of itself two screens tall, redrawn when the reader scrolls out of it. */
    var inkBudget = 64 shl 20
    /** Ink drawn earlier, kept so a page the reader turns to — or back to — is ready. One page's
     * worth by default: a bitmap of a whole page runs to tens of megabytes, and holding several
     * churns native memory until the collector catches up, which a reader feels as a scroll that
     * stutters for a while and then settles. */
    var inkCacheBudget = 48 shl 20
    private val inkCache = ArrayList<Triple<String, Bitmap, Int>>()
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
    private var pinchZoom = QvpZoom()
    private var pinchView = QvpView()
    private var pinchFactor = 1f
    var lastBaseMs = 0.0; var lastOverlayMs = 0.0; var lastBasePaths = 0; var lastOverlayPaths = 0; var lastBands = 0; var lastHitUs = 0.0
        private set
    var animating = false; private set

    private val frameCb = object : Choreographer.FrameCallback {
        override fun doFrame(frameTimeNanos: Long) { animating = false; invalidate() }
    }
    /** A reflowed page is taller than the screen, so a fling carries on under the platform's own
     * deceleration rather than stopping dead under the finger. */
    private val scroller = android.widget.OverScroller(context)
    private val flingCb = object : Choreographer.FrameCallback {
        override fun doFrame(frameTimeNanos: Long) {
            if (!scroller.computeScrollOffset()) return
            viewOy = -scroller.currY.toFloat()
            clampView(); postInvalidateOnAnimation()
            if (!scroller.isFinished) Choreographer.getInstance().postFrameCallback(this)
        }
    }
    private fun fling(vy: Float) {
        val l = page?.currentLayout ?: return
        val max = kotlin.math.max(0f, l.contentH * viewScale - height).toInt()
        scroller.forceFinished(true)
        scroller.fling(0, (-viewOy).toInt(), 0, (-vy).toInt(), 0, 0, 0, max)
        Choreographer.getInstance().postFrameCallback(flingCb)
    }
    /** Hold the page against the viewport, so no drag opens a blank strip beside it. */
    private fun clampView() {
        val l = page?.currentLayout ?: return
        val v = QvpView.of(QvpNative.viewClamp(currentView.floats(), l.contentW, l.contentH, width.toFloat(), height.toFloat()))
        // Land the page on whole device pixels. A fractional offset makes every blit of the ink
        // a resample and the text reads as soft or ragged; the shift is under one pixel, so the
        // scroll loses nothing a reader can see.
        viewOx = kotlin.math.round(v.offsetX); viewOy = kotlin.math.round(v.offsetY)
    }

    /** Scroll the page by [dy] points, held against the viewport. */
    fun scrollPage(by: Float) {
        if (page?.currentLayout == null) return
        viewOy += by; clampView(); invalidate()
    }

    /** Let go of the ink in hand. The cache owns what it holds, so nothing is recycled here: a
     * bitmap recycled while the cache still keeps it comes back as a hit and crashes the draw. */
    private fun releaseBase() {
        base = null
    }

    private val scaleDetector = ScaleGestureDetector(context, object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScaleBegin(d: ScaleGestureDetector): Boolean {
            // the gesture is measured from where it began, so a slow pinch walks the steps one at
            // a time instead of running away to the last of them
            pinchZoom = zoom; pinchView = currentView; pinchFactor = 1f
            return zoomEnabled && !selecting
        }
        override fun onScale(d: ScaleGestureDetector): Boolean {
            if (!zoomEnabled || selecting) return false
            val p = page
            pinchFactor *= d.scaleFactor
            if (p == null || !p.isOpen || zoomMode == QvpZoomMode.MAGNIFY) {
                // never below the settled page: a pinch magnifies the print, it does not shrink
                // the page inside the screen
                val ns = (viewScale * d.scaleFactor).coerceIn(fitScale.coerceAtLeast(MIN_ZOOM), MAX_ZOOM)
                val kk = ns / viewScale
                viewOx = d.focusX - (d.focusX - viewOx) * kk; viewOy = d.focusY - (d.focusY - viewOy) * kk; viewScale = ns
                invalidate(); return true
            }
            apply(p.zoomPinch(layoutSpecInForce(), pinchZoom, pinchView, pinchFactor, d.focusX, d.focusY))
            return true
        }
    })
    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onScroll(e1: MotionEvent?, e2: MotionEvent, dx: Float, dy: Float): Boolean {
            if (selecting) { extendSelection(e2.x, e2.y); return true }
            if (!zoomEnabled) return false
            // a page with no sideways travel scrolls up and down only, and its sideways drag is
            // a page turn, which the fling below answers
            if (sideways == QvpSideways.PAN) viewOx -= dx
            viewOy -= dy
            clampView()
            // ask for the next frame rather than a redraw per touch event: the scroll then moves
            // on the display's own clock, which is what makes it read as smooth
            postInvalidateOnAnimation(); return true
        }
        override fun onFling(e1: MotionEvent?, e2: MotionEvent, vx: Float, vy: Float): Boolean {
            if (selecting || !zoomEnabled) return false
            val turn = onSwipe
            if (turn != null && sideways == QvpSideways.TURN_PAGE && e1 != null) {
                val dir = QvpNative.swipePages(e2.x - e1.x, e2.y - e1.y, vx, vy)
                if (dir != 0) { turn(dir); return true }
            }
            // a reflowed page is read by scrolling, so a fling up or down carries on
            if (isReflowed) { fling(vy) ; return true }
            return false
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
                hit.decoration >= 0 -> onDecorationTap?.invoke(p.decorations[hit.decoration], hit)
                else -> onEmptyTap?.invoke()
            }
            performClick()
            return true
        }
    })

    private fun hitAt(x: Float, y: Float): QvpHit? {
        val p = page ?: return null
        val t0 = System.nanoTime()
        val h = p.hitTestView((x - viewOx) / viewScale, (y - viewOy) / viewScale, hitOptions)
        lastHitUs = (System.nanoTime() - t0) / 1000.0
        return h
    }
    private fun extendSelection(x: Float, y: Float) {
        val p = page ?: return
        val h = p.hitTestView((x - viewOx) / viewScale, (y - viewOy) / viewScale, QvpHitOptions()) ?: return
        if (h.word < 0) return
        p.select(selAnchor, h.word); paintSelection()
    }
    private fun paintSelection() {
        val p = page ?: return
        val ws = p.selection()
        val t = Target.words(ws)
        if (selectionHandle != 0) p.moveHighlight(selectionHandle, t)
        else selectionHandle = p.highlight(t, QvpHighlightStyle(mode = HighlightMode.BAND, band = selectionBand, padX = 0.6f, layer = QvpLayer.SELECTION))
        onSelectionChanged?.invoke(ws); invalidate()
    }
    /** Clear the selection band and the engine selection. */
    fun clearSelection() {
        val p = page ?: return
        p.clearSelection(); if (selectionHandle != 0) { p.removeHighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?.invoke(IntArray(0)); invalidate()
    }

    @SuppressLint("ClickableViewAccessibility") // GestureDetector calls performClick for confirmed taps.
    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_UP || event.actionMasked == MotionEvent.ACTION_CANCEL) {
            if (selecting) { selecting = false; parent?.requestDisallowInterceptTouchEvent(false) }
        }
        if (event.actionMasked == MotionEvent.ACTION_DOWN) settleSideways()
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
    fun layoutSpec() = QvpLayoutSpec(width.toFloat(), height.toFloat(), padTop, padBottom, padSide, padSide, lineSpacing, fillHeight)
    /** The layout the current size and knobs ask for, before the zoom control has its say. */
    internal fun baseSpec() = QvpLayoutSpec(width.toFloat(), height.toFloat(), padTop, padBottom, padSide, padSide, lineSpacing, fillHeight)
    /** The spec in force: the knobs with the reader's zoom control folded in. */
    fun layoutSpecInForce(): QvpLayoutSpec {
        val p = page ?: return baseSpec()
        return if (p.isOpen) p.zoomSpec(baseSpec(), zoom) else baseSpec()
    }
    /** The reader's pan and zoom as the engine has it. */
    internal val currentView get() = QvpView(viewScale, viewOx, viewOy)
    /** True when the page is broken onto rows of its own, and so is taller than the screen. */
    private val isReflowed get() = page?.currentLayout?.reflowed == true
    /** What a sideways drag means here: pan a magnified page, or turn the page. */
    /** What this drag means, settled when the finger lands. Asking the engine on every frame of
     * a scroll put a call and two allocations inside the touch path for an answer that cannot
     * change while the finger is down. */
    private var gestureSideways = QvpSideways.TURN_PAGE
    private val sideways: QvpSideways get() = gestureSideways
    private fun settleSideways() {
        gestureSideways = page?.sidewaysDrag(zoom, currentView, fitScale) ?: QvpSideways.TURN_PAGE
    }
    /** True once the reader has zoomed in, by either road. The engine draws the line. */
    val isZoomed: Boolean get() = QvpNative.zoomIsZoomed(zoom.floats(), currentView.floats(), fitScale)
    private var fitScale = 1f

    /** Take what a gesture produced: the page is already laid out at the new size, and the view
     * already holds the word the fingers were on. */
    private fun apply(c: QvpZoomChange) {
        val moved = c.zoom.step != zoom.step || c.zoom.zoom != zoom.zoom || c.zoom.mode != zoom.mode
        zoom = c.zoom
        if (moved) onZoomChanged?.invoke(c.zoom)
        viewScale = c.view.scale; viewOx = c.view.offsetX; viewOy = c.view.offsetY
        if (c.relaid) { baseKey = "" }
        invalidate()
    }

    fun relayout() {
        val p = page ?: return
        if (width == 0 || height == 0) return
        p.layout(layoutSpecInForce())
        baseKey = ""; invalidate()
    }
    fun resetView() {
        val l = page?.currentLayout
        // A reflowed page is taller than the screen on purpose: fitting its height would undo the
        // size the reader asked for. It opens at its top and the reader scrolls.
        if (l != null && l.reflowed) { viewScale = 1f; fitScale = 1f; viewOx = l.fitX; viewOy = 0f; invalidate(); return }
        viewScale = l?.fitScale ?: 1f; fitScale = viewScale; viewOx = l?.fitX ?: 0f; viewOy = l?.fitY ?: 0f
        invalidate()
    }
    /** Matrix mapping page units of [line] to view px (layout + pan/zoom). */
    fun lineMatrix(line: Int, out: Matrix = m): Matrix {
        val l = page?.currentLayout
        val ls = l?.scale ?: 1f; val lox = l?.offsetX ?: 0f; val loy = (l?.offsetY ?: 0f) + (l?.lineDy?.getOrNull(line) ?: 0f) * ls
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

    /** Page units → view px under one placement (engine layout + pan/zoom). */
    fun placementMatrix(q: QvpPlacement, offsetY: Float = viewOy, out: Matrix = m): Matrix {
        val l = page?.currentLayout
        val ls = l?.scale ?: 1f
        val s = viewScale * ls
        out.reset(); out.setScale(s * q.kx, s * q.ky)
        out.postTranslate(viewOx + viewScale * ((l?.offsetX ?: 0f) + q.dx * ls), offsetY + viewScale * ((l?.offsetY ?: 0f) + q.dy * ls))
        return out
    }

    /** Where the cached ink starts and how tall it is, for the page and place in hand. A page
     * that fits the budget is kept whole, so scrolling draws nothing; a taller one keeps a band
     * two screens tall. A bitmap taller than this cannot be drawn on every device, whatever the
     * budget says. */
    /** What the layout in hand draws and where, read once per band rather than once a frame.
     * Asking the engine on every frame — and once per styled path — was most of what a scroll
     * was spending its time on. */
    private class DrawList(val draws: List<QvpDraw>, val places: List<QvpPlacement>, val key: String) {
        /** The placement of one path, for the styled pass: a lookup, not a scan of the page. */
        val ofPath = HashMap<Int, Int>(draws.size * 2).apply { for (d in draws) putIfAbsent(d.path, d.placement) }
    }
    private var drawList: DrawList? = null
    private fun drawListNow(key: String, top: Float, bandH: Float): DrawList {
        drawList?.let { if (it.key == key) return it }
        val p = page!!
        val s = kotlin.math.max(viewScale, 0.001f)
        val q = DrawList(p.layoutDrawList((top / s) to ((top + bandH) / s)), p.layoutPlacements(), key)
        drawList = q
        return q
    }

    private fun band(): Pair<Float, Float> {
        val screen = kotlin.math.max(height.toFloat(), 1f)
        val l = page?.currentLayout
        if (l != null && l.reflowed) {
            val contentH = l.contentH * viewScale
            // The whole page at once, when it fits: scrolling then draws nothing at all. A bitmap
            // taller than the GPU will take is not drawn either, so that bounds it as well.
            val fits = contentH <= MAX_INK_PX && width.toLong() * contentH.toLong() * 4 <= inkBudget
            if (fits) return 0f to kotlin.math.max(contentH, 1f)
        }
        // Otherwise a band around the reader, measured down the page — the screen starts at
        // -viewOy in the page's own space. Its top moves in steps rather than following the
        // finger, so a drag inside the band is one blit and nothing is drawn again; a band that
        // tracked the finger exactly drew the whole page on every frame of a magnified drag.
        val tall = kotlin.math.min(screen * 2f, MAX_INK_PX)
        val step = kotlin.math.max((tall - screen) / 2f, 1f)
        return kotlin.math.floor((-viewOy - step) / step) * step to tall
    }

    override fun onDraw(canvas: Canvas) {
        val p = page ?: return
        if (p.currentLayout == null) relayout()
        val l = p.currentLayout ?: return
        val moving = p.tick(System.nanoTime() / 1e6)
        val styled = p.styledPaths()
        styledPaths.clear(); var i = 0; var styledKey = 1
        while (i < styled.size) {
            val pi = styled[i]
            styledPaths.add(pi)
            styledKey = 31 * styledKey + pi
            i += 2
        }
        val ink = p.defaultInk
        val (top, bandH) = band()
        val bandPx = kotlin.math.ceil(bandH).toInt().coerceAtLeast(1)
        // The key holds where the band starts, never where the reader has scrolled to, so a drag
        // inside the band is one blit and nothing is drawn again.
        val key = "${p.pageNo}|$viewScale|$viewOx|$top|$ink|${l.lineSpacing}|${l.lineDy.contentHashCode()}|$styledKey|$width|$bandPx|${zoom.zoom}|${l.rows}"
        if (TRACE) android.util.Log.i("QVPTRACE", "band top=$top h=$bandH px=$bandPx contentH=${l.contentH * viewScale} viewOy=$viewOy viewScale=$viewScale height=$height rows=${l.rows}")
        if (paperColor != Color.TRANSPARENT) { paint.color = paperColor; canvas.drawRect(viewOx, viewOy, viewOx + l.contentW * viewScale, viewOy + l.contentH * viewScale, paint) }
        val bands = p.highlightBoxesView()
        drawBoxes(canvas, bands)
        var b = base
        if (b == null || key != baseKey || b.width != width || b.height != bandPx) {
            val t0 = System.nanoTime()
            val kept = inkCache.firstOrNull { it.first == key }?.second
            if (kept != null) { base = kept; b = kept; baseKey = key }
            else {
                // Never draw over ink the cache is holding: it would keep the old key and hand
                // back the wrong page later. Only ink nobody else owns is drawn over again.
                val reusable = b != null && b.width == width && b.height == bandPx && inkCache.none { it.second === b }
                if (!reusable) {
                    b = Bitmap.createBitmap(width, bandPx, Bitmap.Config.ARGB_8888)
                    b.density = Bitmap.DENSITY_NONE   // blit it one to one, never by density
                    base = b
                }
                b.eraseColor(Color.TRANSPARENT)
                val bc = Canvas(b); paint.color = QvpColor.argb(ink)
                // the band is drawn under its own vertical offset: its top row is `top`
                val q = drawListNow(key, top, bandH)
                val places = q.places
                var cur = -1; var n = 0
                for (d in q.draws) {
                    if (d.path in styledPaths) continue
                    if (d.placement != cur) { bc.setMatrix(placementMatrix(places.getOrElse(d.placement) { QvpPlacement.IDENTITY }, -top)); cur = d.placement }
                    bc.drawPath(p.path(d.path), paint); n++
                }
                baseKey = key; lastBaseMs = (System.nanoTime() - t0) / 1e6; lastBasePaths = n
                keepInk(key, b)
            }
        }
        // the ink sits at `viewOy + top` on screen; scrolling inside the band only moves it
        canvas.drawBitmap(b!!, 0f, viewOy + top, null)
        val t1 = System.nanoTime()
        val q = drawListNow(key, top, bandH)
        val places = q.places
        var cur = -1; i = 0
        canvas.save()
        while (i < styled.size) {
            val pi = styled[i]; val col = styled[i + 1]; i += 2
            if (col and 0xff == 0) continue
            val g = q.ofPath[pi] ?: p.pathLine(pi)
            if (g != cur) { canvas.restore(); canvas.save(); canvas.concat(placementMatrix(places.getOrElse(g) { QvpPlacement.IDENTITY })); cur = g }
            paint.color = QvpColor.argb(col); canvas.drawPath(p.path(pi), paint)
        }
        canvas.restore()
        drawBoxes(canvas, p.maskBoxesView())
        lastOverlayMs = (System.nanoTime() - t1) / 1e6; lastOverlayPaths = styled.size / 2; lastBands = bands.size
        if (moving) { animating = true; Choreographer.getInstance().postFrameCallback(frameCb) }
    }

    /** Keep ink already drawn, by page and size, so a page the reader turns back to is ready. */
    private fun keepInk(key: String, bmp: Bitmap) {
        inkCache.removeAll { it.first == key }
        inkCache.add(0, Triple(key, bmp, bmp.byteCount))
        // Dropped ink is freed when nothing holds it. Recycling it here would pull the pixels out
        // from under a draw that is still using them.
        var total = 0
        val keep = ArrayList<Triple<String, Bitmap, Int>>()
        for (e in inkCache) { total += e.third; if (total <= inkCacheBudget) keep.add(e) }
        inkCache.clear(); inkCache.addAll(keep)
    }

    /** Draw a page the reader has not reached yet, at the size they are reading, and keep it. */
    fun prepare(other: QvpPage) {
        if (!other.isOpen || width == 0 || other === page) return
        val z = other.zoomCarried(baseSpec(), zoom)
        other.layout(other.zoomSpec(baseSpec(), z))
    }

    companion object {
        /** Pinch limits as multiples of the fitted scale; the same pair on every platform. */
        const val MIN_ZOOM = 0.5f
        const val MAX_ZOOM = 12f
        /** A bitmap taller than this cannot be drawn on every device, so the ink keeps a band. */
        private const val MAX_INK_PX = 4096f
        /** Log the band arithmetic, for tracking down ink that does not reach the screen. */
        var TRACE = false
    }
}
