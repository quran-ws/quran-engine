package ws.quran.qvp

import android.graphics.Bitmap
import android.graphics.Canvas
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** A view keeps what it read from the engine until the page changes. Its frames must match the
 * frames of a view that has kept nothing. */
@RunWith(AndroidJUnit4::class)
class QvpPageViewFrameTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val bytes by lazy { instrumentation.context.assets.open("042.qvp").use { it.readBytes() } }

    private fun view(page: QvpPage) = QvpPageView(instrumentation.targetContext).apply {
        this.page = page
        layout(0, 0, W, H)
    }

    private fun frame(view: QvpPageView): Bitmap =
        Bitmap.createBitmap(W, H, Bitmap.Config.ARGB_8888).also { view.draw(Canvas(it)) }

    /** The same frame from a view that has never drawn this page. */
    private fun fresh(page: QvpPage): Bitmap {
        val other = view(page)
        try { return frame(other) } finally { other.page = null }
    }

    @Test
    fun aKeptFrameMatchesAFreshOne() {
        instrumentation.runOnMainSync {
            val page = QvpPage(bytes)
            val shown = view(page)
            try {
                frame(shown)
                val h = page.highlight(Target.ayah(2, 255), QvpHighlightStyle(mode = HighlightMode.BOTH))
                page.mask(Target.words(intArrayOf(0, 1, 2)), MaskMode.BLOCK)
                frame(shown)
                assertTrue(shown.lastBands > 0)
                assertTrue(shown.lastOverlayPaths > 0)
                // nothing moved since the frame before, so this one draws what it kept
                assertTrue(frame(shown).sameAs(fresh(page)))

                // a step lays the page out again, which moves the bands and the blocks
                shown.zoomToStep(1)
                val stepped = frame(shown)
                assertEquals(page.highlightBoxesView().size, shown.lastBands)
                val other = view(page)
                try {
                    other.zoomToStep(1)
                    assertTrue(stepped.sameAs(frame(other)))
                } finally {
                    other.page = null
                }

                // a removed highlight takes its bands and its ink with it
                shown.zoomToStep(0)
                page.removeHighlight(h)
                page.unmask()
                frame(shown)
                assertEquals(0, shown.lastBands)
                assertEquals(0, shown.lastOverlayPaths)
                assertTrue(frame(shown).sameAs(fresh(page)))
            } finally {
                shown.page = null
                page.close()
            }
        }
    }

    private companion object {
        const val W = 1080
        const val H = 1920
    }
}
