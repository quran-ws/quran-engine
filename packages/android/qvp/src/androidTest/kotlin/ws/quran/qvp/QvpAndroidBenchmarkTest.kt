package ws.quran.qvp

import android.graphics.Bitmap
import android.graphics.Canvas
import android.os.Build
import android.os.Bundle
import android.os.Debug
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.filters.LargeTest
import androidx.test.platform.app.InstrumentationRegistry
import kotlin.system.measureNanoTime
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Reports comparable device timings without imposing a device-specific pass threshold. */
@LargeTest
@RunWith(AndroidJUnit4::class)
class QvpAndroidBenchmarkTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val bytes by lazy { instrumentation.context.assets.open("042.qvp").use { it.readBytes() } }

    @Test
    fun pagePipeline() {
        repeat(3) { QvpPage(bytes).close() }

        val load = samples(15) { QvpPage(bytes).close() }
        val paths = samples(10) {
            val page = QvpPage(bytes)
            try {
                page.buildPaths()
            } finally {
                page.close()
            }
        }

        var firstDraw = emptyList<Double>()
        var cachedDraw = emptyList<Double>()
        var cachedStyledDraw = emptyList<Double>()
        var cachedAllocations = 0.0
        var cachedStyledAllocations = 0.0
        instrumentation.runOnMainSync {
            val bitmap = Bitmap.createBitmap(1080, 1920, Bitmap.Config.ARGB_8888)
            try {
                firstDraw = List(10) {
                    val page = QvpPage(bytes)
                    val view = QvpPageView(instrumentation.targetContext)
                    try {
                        view.page = page
                        view.layout(0, 0, bitmap.width, bitmap.height)
                        milliseconds(measureNanoTime { view.draw(Canvas(bitmap)) })
                    } finally {
                        view.page = null
                        page.close()
                        bitmap.eraseColor(0)
                    }
                }

                val page = QvpPage(bytes)
                val view = QvpPageView(instrumentation.targetContext)
                try {
                    view.page = page
                    view.layout(0, 0, bitmap.width, bitmap.height)
                    val canvas = Canvas(bitmap)
                    view.draw(canvas)
                    cachedDraw = samples(200) { view.draw(canvas) }
                    cachedAllocations = allocationsPerCall(200) { view.draw(canvas) }
                } finally {
                    view.page = null
                    page.close()
                }

                // The same page while a reader follows a recitation: a band and ink on one ayah,
                // and a few words blocked out. Nothing moves between the frames.
                val styled = QvpPage(bytes)
                val styledView = QvpPageView(instrumentation.targetContext)
                try {
                    styledView.page = styled
                    styledView.layout(0, 0, bitmap.width, bitmap.height)
                    styled.highlight(Target.ayah(2, 255), QvpHighlightStyle(mode = HighlightMode.BOTH))
                    styled.mask(Target.words(intArrayOf(0, 1, 2)), MaskMode.BLOCK)
                    val canvas = Canvas(bitmap)
                    styledView.draw(canvas)
                    cachedStyledDraw = samples(200) { styledView.draw(canvas) }
                    cachedStyledAllocations = allocationsPerCall(200) { styledView.draw(canvas) }
                } finally {
                    styledView.page = null
                    styled.close()
                }
            } finally {
                bitmap.recycle()
            }
        }

        val result = buildString {
            append("{")
            append("\"device\":\"").append(Build.MANUFACTURER).append(' ').append(Build.MODEL).append("\",")
            append("\"sdk\":").append(Build.VERSION.SDK_INT).append(',')
            append("\"page\":42,")
            append("\"bytes\":").append(bytes.size).append(',')
            append("\"loadMedianMs\":").append(median(load)).append(',')
            append("\"pathsMedianMs\":").append(median(paths)).append(',')
            append("\"firstDrawMedianMs\":").append(median(firstDraw)).append(',')
            append("\"cachedDrawMedianMs\":").append(median(cachedDraw)).append(',')
            append("\"cachedStyledDrawMedianMs\":").append(median(cachedStyledDraw)).append(',')
            append("\"cachedDrawAllocations\":").append(cachedAllocations).append(',')
            append("\"cachedStyledDrawAllocations\":").append(cachedStyledAllocations)
            append("}")
        }
        Log.i(TAG, result)
        instrumentation.sendStatus(0, Bundle().apply { putString(TAG, result) })

        assertTrue(load.all { it.isFinite() && it >= 0 })
        assertTrue(paths.all { it.isFinite() && it >= 0 })
        assertTrue(firstDraw.all { it.isFinite() && it >= 0 })
        assertTrue(cachedDraw.all { it.isFinite() && it >= 0 })
        assertTrue(cachedStyledDraw.all { it.isFinite() && it >= 0 })
    }

    private fun samples(count: Int, block: () -> Unit): List<Double> =
        List(count) { milliseconds(measureNanoTime(block)) }

    /** Objects the calling thread allocates per call of [block], over [count] calls. The counter
     * is deprecated but ART still keeps it, and it is the one that needs no profiler. */
    @Suppress("DEPRECATION")
    private fun allocationsPerCall(count: Int, block: () -> Unit): Double {
        Debug.resetThreadAllocCount()
        Debug.startAllocCounting()
        try {
            repeat(count) { block() }
        } finally {
            Debug.stopAllocCounting()
        }
        return Debug.getThreadAllocCount().toDouble() / count
    }

    private fun median(values: List<Double>): Double = values.sorted()[values.size / 2]

    private fun milliseconds(nanos: Long): Double = nanos / 1_000_000.0

    private companion object { const val TAG = "QvpBenchmark" }
}
