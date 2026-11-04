// Replays conformance/scenarios/hit.json against the engine through the Kotlin wrapper.
// The page and the scenario file reach the test assets through scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpHitConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets

    /**
     * Every wrapper replays conformance/scenarios/hit.json: the exact and the gap-aware hit
     * tests on a word's outline, a word's box alone, a decoration's outline and a decoration's
     * box alone, in page units and in viewport px.
     */
    @Test
    fun hitScenariosMatchTheEngine() {
        val json = JSONObject(assets.open("hit.json").use { it.readBytes() }.toString(Charsets.UTF_8))
        val tolerance = json.getDouble("tolerance")
        val cases = json.getJSONArray("cases")
        val s = json.getJSONObject("spec")
        fun f(k: String) = s.getDouble(k).toFloat()
        QvpPage(assets.open("042.qvp").use { it.readBytes() }).use { p ->
            p.layout(QvpLayoutSpec(f("viewportW"), f("viewportH"), f("padTop"), f("padBottom"), f("padLeft"), f("padRight")))
            fun same(got: QvpHit?, want: JSONObject?, what: String) {
                if (want == null) return assertNull("$what: got a hit, engine says none", got)
                assertNotNull("$what: got no hit", got)
                got!!
                assertEquals(what, listOf(want.getInt("word"), want.getInt("path"), want.getInt("decoration"), want.getInt("line")),
                    listOf(got.word, got.path, got.decoration, got.line))
                assertEquals("$what isExact", want.getBoolean("isExact"), got.isExact)
                assertTrue("$what distance: got ${got.distance}, engine says ${want.getDouble("distance")}",
                    Math.abs(got.distance - want.getDouble("distance")) <= tolerance)
            }
            for (i in 0 until cases.length()) {
                val c = cases.getJSONObject(i)
                val x = c.getDouble("x").toFloat()
                val y = c.getDouble("y").toFloat()
                val view = c.getString("units") == "view"
                val tag = "hit ${c.getString("name")} (${c.getString("units")} $x, $y)"
                same(if (view) p.hitTestExactView(x, y) else p.hitTestExact(x, y), c.optJSONObject("exact"), "$tag exact")
                same(if (view) p.hitTestView(x, y) else p.hitTest(x, y), c.optJSONObject("hit"), "$tag gap-aware")
            }
            assertEquals(8, cases.length())
        }
    }
}
