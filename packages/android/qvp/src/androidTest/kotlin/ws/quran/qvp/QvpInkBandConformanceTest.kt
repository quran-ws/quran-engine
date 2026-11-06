// Replays conformance/scenarios/ink-band.json against the engine through the Kotlin wrapper.
// The page and the scenario file reach the test assets through scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpInkBandConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets

    /**
     * Every wrapper replays conformance/scenarios/ink-band.json: the band of the page a host
     * keeps drawn, for a page that fits, a magnified page drawn a band at a time, pans inside
     * one step and across it, the host's own limits, and no screen at all.
     */
    @Test
    fun inkBandScenariosMatchTheEngine() {
        val json = JSONObject(assets.open("ink-band.json").use { it.readBytes() }.toString(Charsets.UTF_8))
        val tolerance = json.getDouble("tolerance")
        val cases = json.getJSONArray("cases")
        val s = json.getJSONObject("spec")
        fun f(k: String) = s.getDouble(k).toFloat()
        fun close(got: Float, want: Double, what: String) =
            assertTrue("$what: got $got, engine says $want", Math.abs(got - want) <= tolerance * Math.max(1.0, Math.abs(want)))
        QvpPage(assets.open("042.qvp").use { it.readBytes() }).use { p ->
            val l = p.layout(QvpLayoutSpec(f("viewportW"), f("viewportH"), f("padTop"), f("padBottom"), f("padLeft"), f("padRight")))
            close(l.contentH, json.getDouble("contentH"), "contentH")
            for (i in 0 until cases.length()) {
                val c = cases.getJSONObject(i)
                val v = c.getJSONObject("view")
                val view = QvpView(v.getDouble("scale").toFloat(), v.getDouble("offsetX").toFloat(), v.getDouble("offsetY").toFloat())
                val got = p.layoutInkBand(view, c.getDouble("viewportW").toFloat(), c.getDouble("viewportH").toFloat(),
                    c.getDouble("maxHeight").toFloat(), c.getDouble("maxPixels").toFloat())
                val want = c.getJSONObject("band")
                close(got.top, want.getDouble("top"), "ink band ${c.getString("name")} top")
                close(got.height, want.getDouble("height"), "ink band ${c.getString("name")} height")
            }
            assertEquals(14, cases.length())
        }
    }
}
