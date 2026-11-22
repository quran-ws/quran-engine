// Replays conformance/scenarios/ink-band.json against the engine through the Kotlin wrapper.
// The page, the scenario file and the ornament set reach the test assets through
// scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpInkBandConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets

    /**
     * Every wrapper replays conformance/scenarios/ink-band.json: the band of the page a host
     * keeps drawn, for a page that fits, a magnified page drawn a band at a time, pans inside
     * one step and across it, the host's own limits, and no screen at all; then the same page
     * dressed.
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
            fun replay(cases: JSONArray, tag: String) {
                for (i in 0 until cases.length()) {
                    val c = cases.getJSONObject(i)
                    val v = c.getJSONObject("view")
                    val view = QvpView(v.getDouble("scale").toFloat(), v.getDouble("offsetX").toFloat(), v.getDouble("offsetY").toFloat())
                    val got = p.layoutInkBand(view, c.getDouble("viewportW").toFloat(), c.getDouble("viewportH").toFloat(),
                        c.getDouble("maxHeight").toFloat(), c.getDouble("maxPixels").toFloat())
                    val want = c.getJSONObject("band")
                    close(got.top, want.getDouble("top"), "$tag ${c.getString("name")} top")
                    close(got.height, want.getDouble("height"), "$tag ${c.getString("name")} height")
                }
            }
            replay(cases, "ink band")
            assertEquals(14, cases.length())
            // The same page dressed in the synthetic set: the band is that of the dressed box, which
            // holds the border above and below the content.
            val dressed = json.getJSONObject("dressed")
            val hex = assets.open(dressed.getString("ornaments")).use { it.readBytes() }.toString(Charsets.UTF_8).filterNot { it.isWhitespace() }
            val bytes = ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
            QvpOrnaments(bytes).use { set ->
                val d = dressed.getJSONObject("dress")
                val spec = QvpDressSpec(ayahMark = QvpOrnamentRef.Index(d.getInt("ayahMark")), surahHeader = QvpOrnamentRef.Index(d.getInt("surahHeader")),
                    pageFrame = QvpOrnamentRef.Index(d.getInt("pageFrame")), gap = d.getDouble("gap").toFloat(), bandGap = d.getDouble("bandGap").toFloat(),
                    ayahMarkScale = d.getDouble("ayahMarkScale").toFloat(), pageFrameScale = d.getDouble("pageFrameScale").toFloat())
                val info = p.dress(set, spec)
                assertNotNull("the engine refused the dress", info)
                val o = info!!.overflow
                val want = dressed.getJSONArray("overflow")
                listOf(o.left, o.top, o.right, o.bottom).forEachIndexed { i, v -> close(v, want.getDouble(i), "overflow[$i]") }
                val dressedCases = dressed.getJSONArray("cases")
                replay(dressedCases, "dressed ink band")
                assertEquals(17, dressedCases.length())
            }
        }
    }
}
