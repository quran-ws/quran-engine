package net.quranpedia.qvp

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.View

/**
 * Host-canvas renderer for a [QvpPage]: cached base layer (unstyled ink) + overlay of
 * styled paths, drawn per line with the engine layout. Pan/zoom sits on top of the layout.
 * Tap → engine hit-test → [onWordTap] / [onDecoTap] / [onEmptyTap].
 */
class QvpPageView @JvmOverloads constructor(context: Context, attrs: AttributeSet? = null) : View(context, attrs) {
    var page: QvpPage? = null
        set(v) { field = v; base = null; baseKey = ""; relayout(); invalidate() }
    /** Layout knobs; viewport size is taken from the view. */
    var padTop = 0f; var padBottom = 0f; var padSide = 0f; var lineSpacing = 1f; var fillHeight = false
    var ink: Int = 0x231f20ff.toInt()            // 0xRRGGBBAA default ink (also pushed to the engine)
    var paperColor: Int = Color.TRANSPARENT     // ARGB
    var selectionColor: Int = 0x2A1A73E8        // ARGB, drawn behind the selected word
    var selectedWord = -1
    var selectedAyah: Pair<Int, Int>? = null
    var onWordTap: ((QvpWord, QvpHit) -> Unit)? = null
    var onDecoTap: ((QvpDecoration, QvpHit) -> Unit)? = null
    var onEmptyTap: (() -> Unit)? = null
    var zoomEnabled = true

    // pan/zoom on top of the layout (view px)
    var viewScale = 1f; var viewOx = 0f; var viewOy = 0f
    private var base: Bitmap? = null
    private var baseKey = ""
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val m = Matrix()
    var lastBaseMs = 0.0; var lastOverlayMs = 0.0; var lastBasePaths = 0; var lastOverlayPaths = 0; var lastHitUs = 0.0
        private set

    private val scaleDetector = ScaleGestureDetector(context, object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScale(d: ScaleGestureDetector): Boolean {
            if (!zoomEnabled) return false
            val k = d.scaleFactor
            val ns = (viewScale * k).coerceIn(0.5f, 12f); val kk = ns / viewScale
            viewOx = d.focusX - (d.focusX - viewOx) * kk; viewOy = d.focusY - (d.focusY - viewOy) * kk; viewScale = ns
            invalidate(); return true
        }
    })
    private val gestureDetector = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onScroll(e1: MotionEvent?, e2: MotionEvent, dx: Float, dy: Float): Boolean {
            if (!zoomEnabled) return false
            viewOx -= dx; viewOy -= dy; invalidate(); return true
        }
        override fun onDoubleTap(e: MotionEvent): Boolean { resetView(); return true }
        override fun onSingleTapConfirmed(e: MotionEvent): Boolean {
            val p = page ?: return false
            val vx = (e.x - viewOx) / viewScale; val vy = (e.y - viewOy) / viewScale
            val t0 = System.nanoTime()
            val hit = p.hitTestView(vx, vy)
            lastHitUs = (System.nanoTime() - t0) / 1000.0
            when {
                hit == null -> onEmptyTap?.invoke()
                hit.word >= 0 -> onWordTap?.invoke(p.words[hit.word], hit)
                hit.deco >= 0 -> onDecoTap?.invoke(p.decos[hit.deco], hit)
                else -> onEmptyTap?.invoke()
            }
            return true
        }
    })

    override fun onTouchEvent(event: MotionEvent): Boolean {
        scaleDetector.onTouchEvent(event); gestureDetector.onTouchEvent(event); return true
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) { relayout(); resetView() }

    /** Recompute the engine layout for the current size/knobs. */
    fun relayout() {
        val p = page ?: return
        if (width == 0 || height == 0) return
        val l = p.layout(QvpLayoutSpec(width.toFloat(), height.toFloat(), padTop, padBottom, padSide, padSide, lineSpacing, fillHeight))
        l.contentW = width.toFloat()
        baseKey = ""; invalidate()
    }
    fun resetView() {
        val l = page?.currentLayout
        viewScale = if (l != null && l.contentH > height) height / l.contentH else 1f
        viewOx = 0f; viewOy = if (l != null) ((height - l.contentH * viewScale) / 2f).coerceAtLeast(0f) else 0f
        invalidate()
    }

    /** Matrix mapping page units of [line] to view px (layout + pan/zoom). */
    fun lineMatrix(line: Int, out: Matrix = m): Matrix {
        val p = page; val l = p?.currentLayout
        val ls = l?.scale ?: 1f; val lox = l?.ox ?: 0f; val loy = (l?.oy ?: 0f) + (l?.lineDy?.getOrNull(line) ?: 0f) * ls
        out.reset(); out.setScale(viewScale * ls, viewScale * ls); out.postTranslate(viewOx + viewScale * lox, viewOy + viewScale * loy)
        return out
    }

    override fun onDraw(canvas: Canvas) {
        val p = page ?: return
        if (p.currentLayout == null) relayout()
        val l = p.currentLayout ?: return
        p.styleDefault(ink)
        val paths = p.buildPaths()
        val styled = p.styled()
        val styledSet = HashSet<Int>(styled.size / 2); var i = 0
        while (i < styled.size) { styledSet.add(styled[i]); i += 2 }
        val key = "$viewScale|$viewOx|$viewOy|$ink|${l.pitch}|${l.lineDy.contentHashCode()}|${styledSet.sorted().hashCode()}|$width|$height"
        if (paperColor != Color.TRANSPARENT) {
            paint.color = paperColor
            canvas.drawRect(viewOx, viewOy, viewOx + l.contentW * viewScale, viewOy + l.contentH * viewScale, paint)
        }
        // selection backgrounds
        selectedAyah?.let { (s, a) -> for (ay in p.ayahs) if (ay.sura == s && ay.ayah == a) {
            val w0 = p.words.getOrNull(ay.firstWord) ?: continue
            drawBox(canvas, ay.x0, ay.y0, ay.x1, ay.y1, w0.line, 0x140A7D32)
        } }
        if (selectedWord >= 0) p.words.getOrNull(selectedWord)?.let { drawBox(canvas, it.x0, it.y0, it.x1, it.y1, it.line, selectionColor) }
        // base layer
        var b = base
        if (b == null || key != baseKey || b.width != width || b.height != height) {
            val t0 = System.nanoTime()
            if (b == null || b.width != width || b.height != height) { b = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888); base = b }
            b.eraseColor(Color.TRANSPARENT)
            val bc = Canvas(b)
            paint.color = QvpPage.argb(ink)
            var cur = -1; var n = 0
            for (pi in 0 until p.nPaths) {
                if (pi in styledSet) continue
                val ln = p.pathLine(pi)
                if (ln != cur) { bc.setMatrix(lineMatrix(ln)); cur = ln }
                bc.drawPath(paths[pi], paint); n++
            }
            baseKey = key; lastBaseMs = (System.nanoTime() - t0) / 1e6; lastBasePaths = n
        }
        canvas.drawBitmap(b, 0f, 0f, null)
        // overlay
        val t1 = System.nanoTime()
        var cur = -1; i = 0
        canvas.save()
        while (i < styled.size) {
            val pi = styled[i]; val col = styled[i + 1]; i += 2
            if (col and 0xff == 0) continue
            val ln = p.pathLine(pi)
            if (ln != cur) { canvas.restore(); canvas.save(); canvas.concat(lineMatrix(ln)); cur = ln }
            paint.color = QvpPage.argb(col)
            canvas.drawPath(paths[pi], paint)
        }
        canvas.restore()
        lastOverlayMs = (System.nanoTime() - t1) / 1e6; lastOverlayPaths = styled.size / 2
    }

    private val rect = RectF()
    private fun drawBox(canvas: Canvas, x0: Float, y0: Float, x1: Float, y1: Float, lineNo: Int, argb: Int) {
        val p = page ?: return
        val li = p.lines.indexOfFirst { it.lineNo == lineNo }.coerceAtLeast(0)
        canvas.save(); canvas.concat(lineMatrix(li))
        paint.color = argb; rect.set(x0 - 1.2f, y0 - 1.2f, x1 + 1.2f, y1 + 1.2f)
        canvas.drawRoundRect(rect, 1.5f, 1.5f, paint)
        canvas.restore()
    }
}
