// Replays conformance/scenarios/passage.json against the engine through the Kotlin wrapper.
// The pages and the scenario file reach the test assets through scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpPassageConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets
    private fun page(n: Int) = QvpPage(assets.open("%03d.qvp".format(n)).use { it.readBytes() })

    /** Every wrapper replays conformance/scenarios/passage.json and must match the engine's numbers. */
    @Test
    fun passageScenariosMatchTheEngine() {
        val json = JSONObject(assets.open("passage.json").use { it.readBytes() }.toString(Charsets.UTF_8))
        val tolerance = json.getDouble("tolerance")
        val cases = json.getJSONArray("cases")
        fun close(got: Float, want: Double, what: String) =
            assertTrue("$what: got $got, engine says $want", Math.abs(got.toDouble() - want) <= tolerance)
        fun JSONArray.ints() = List(length()) { getInt(it) }
        fun JSONArray.doubles() = List(length()) { getDouble(it) }

        for (i in 0 until cases.length()) {
            val c = cases.getJSONObject(i)
            val surah = c.getInt("surah"); val from = c.getInt("from"); val to = c.getInt("to")
            val basmalah = c.optBoolean("basmalah", false)
            val pages = c.getJSONArray("pages").ints().map { page(it) }
            val passage = if (basmalah) QvpPassage.basmalah(pages, surah) else QvpPassage.load(pages, surah, from, to)
            // The passage keeps what it needs.
            pages.forEach { it.close() }
            val s = c.getJSONObject("spec"); val want = c.getJSONObject("layout")
            val align = when (s.getString("align")) { "center" -> QvpPassageAlign.CENTER; "justified" -> QvpPassageAlign.JUSTIFIED; else -> QvpPassageAlign.RIGHT }
            val spec = QvpPassageSpec(width = s.getDouble("width").toFloat(), scale = s.getDouble("scale").toFloat(),
                                      lineSpacing = s.getDouble("lineSpacing").toFloat(), padding = s.getDouble("padding").toFloat(),
                                      align = align, maxRows = s.getInt("maxRows"), keepAyahMark = s.getBoolean("keepAyahMark"),
                                      ellipsisWidth = s.getDouble("ellipsisWidth").toFloat(), maxStretch = s.getDouble("maxStretch").toFloat())
            val tag = "${if (basmalah) "basmalah of " else ""}$surah:$from-$to at ${spec.width} ${s.getString("align")} rows ${spec.maxRows}"
            val l = passage.layout(spec)
            assertNotNull(tag, l); l!!
            close(l.width, want.getDouble("width"), "$tag width"); close(l.height, want.getDouble("height"), "$tag height")
            close(l.scale, want.getDouble("scale"), "$tag scale"); close(l.lineSpacing, want.getDouble("lineSpacing"), "$tag lineSpacing")
            assertEquals(tag, want.getBoolean("isTruncated"), l.isTruncated)
            assertEquals(tag, want.getInt("draws"), l.draws.size)
            val rows = want.getJSONArray("rows"); val words = want.getJSONArray("words")
            assertEquals(tag, rows.length(), l.rows.size)
            assertEquals(tag, words.length(), l.words.size)
            for (k in 0 until rows.length()) {
                val r = l.rows[k]; val g = rows.getJSONArray(k).doubles()
                listOf(r.x0, r.y0, r.x1, r.y1, r.baseline).zip(g).forEach { (v, w) -> close(v, w, "$tag row $k") }
            }
            for (k in 0 until words.length()) {
                val w = l.words[k]; val g = words.getJSONArray(k).doubles()
                assertEquals(tag, g.take(4).map { it.toInt() }, listOf(w.page, w.ayah, w.word, w.row))
                listOf(w.x0, w.y0, w.x1, w.y1).zip(g.drop(4)).forEach { (v, e) -> close(v, e, "$tag word $k") }
            }
            if (want.isNull("ellipsis")) {
                assertNull(tag, l.ellipsis)
            } else {
                val e = l.ellipsis; assertNotNull(tag, e); e!!
                listOf(e.left, e.top, e.right, e.bottom).zip(want.getJSONArray("ellipsis").doubles()).forEach { (v, w) -> close(v, w, "$tag ellipsis") }
            }
            assertTrue(tag, l.placements.size > (l.draws.maxOfOrNull { it.placement } ?: -1))
            passage.close()
        }
        assertEquals(58, cases.length())
    }

    /** A surah's basmalah is a passage with no ayahs. Surah 1 has none of its own, and a page that
     * does not print it has nothing to load. A closed passage refuses every call. */
    @Test
    fun basmalahAndLifecycle() {
        val two = page(2); val one = page(1)
        try {
            val basmalah = QvpPassage.basmalah(listOf(two), 2)
            assertTrue(basmalah.ayahs.isEmpty())
            val first = QvpPassage.load(listOf(two), 2, 1)
            assertEquals(1, first.ayahs.size)
            assertEquals(2, first.ayahs[0].surah); assertEquals(1, first.ayahs[0].ayah)
            assertTrue(first.ayahs[0].text.isNotBlank())
            assertEquals(first.lineSpacing, basmalah.lineSpacing, 0f)
            val l = basmalah.layout(QvpPassageSpec(width = 366f, scale = 22f / basmalah.lineSpacing, align = QvpPassageAlign.CENTER))
            assertNotNull(l); l!!
            assertEquals(1, l.rows.size)
            assertTrue(l.words.isEmpty())
            assertFalse(l.draws.isEmpty())
            assertTrue(basmalah.currentLayout === l)
            assertThrows(IllegalArgumentException::class.java) { QvpPassage.basmalah(listOf(one), 1) }
            assertThrows(IllegalArgumentException::class.java) { QvpPassage.basmalah(listOf(one), 2) }
            assertThrows(IllegalArgumentException::class.java) { QvpPassage.load(listOf(one), 2, 1) }
            assertThrows(IllegalArgumentException::class.java) { QvpPassage.load(listOf(two, two), 2, 1) }
            first.close(); basmalah.close()
            assertTrue(first.isClosed); assertFalse(first.isOpen)
            first.close()
            assertThrows(IllegalStateException::class.java) { first.layout(QvpPassageSpec()) }
        } finally {
            two.close(); one.close()
        }
        assertThrows(IllegalArgumentException::class.java) { QvpPassage.load(listOf(two), 2, 1) }
    }
}
