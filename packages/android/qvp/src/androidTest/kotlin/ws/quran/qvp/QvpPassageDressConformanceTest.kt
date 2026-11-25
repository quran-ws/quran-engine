// Replays the passages of conformance/scenarios/dress.json against the engine through the Kotlin
// wrapper. The page, the scenario file and the ornament set reach the test assets through
// scripts/sync-android-test-data.sh.
package ws.quran.qvp

import android.graphics.RectF
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpPassageDressConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets

    /**
     * Every wrapper replays the passages of conformance/scenarios/dress.json: ayahs of page 042 in
     * the synthetic medallion, on one row, on wrapped rows, cut with the medallion kept, with no
     * medallions, and a medallion alone. The layout makes room for each design, and each outline
     * is listed under the placement of its word, in its colour.
     */
    @Test
    fun dressedPassagesMatchTheEngine() {
        val json = JSONObject(assets.open("dress.json").use { it.readBytes() }.toString(Charsets.UTF_8))
        val tolerance = json.getDouble("tolerance")
        fun close(got: Float, want: Double, what: String) =
            assertTrue("$what: got $got, engine says $want", Math.abs(got - want) <= tolerance * Math.max(1.0, Math.abs(want)))
        val hex = assets.open(json.getString("ornaments")).use { it.readBytes() }.toString(Charsets.UTF_8).filterNot { it.isWhitespace() }
        val bytes = ByteArray(hex.length / 2) { hex.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
        val passages = json.getJSONArray("passages")
        assertEquals(5, passages.length())
        QvpOrnaments(bytes).use { set ->
            QvpPage(assets.open("042.qvp").use { it.readBytes() }).use { page ->
                for (i in 0 until passages.length()) {
                    val c = passages.getJSONObject(i)
                    val (surah, from, to) = Triple(c.getInt("surah"), c.getInt("from"), c.getInt("to"))
                    val alone = c.getBoolean("ayahMark")
                    val tag = "dressed passage $surah:$from-$to${if (alone) " medallion" else ""}"
                    val d = c.getJSONObject("dress")
                    val lineArt = d.getJSONArray("lineArt")
                    val colors = d.getJSONArray("colors")
                    val dress = QvpDressSpec(
                        ayahMark = QvpOrnamentRef.Index(d.getInt("ayahMark")),
                        surahHeader = if (d.has("surahHeader")) QvpOrnamentRef.Index(d.getInt("surahHeader")) else null,
                        pageFrame = if (d.has("pageFrame")) QvpOrnamentRef.Index(d.getInt("pageFrame")) else null,
                        ayahMarkScale = d.optDouble("ayahMarkScale", 0.0).toFloat(),
                        lineArt = (0 until lineArt.length()).map { k -> QvpOrnamentKind.entries.first { it.name.lowercase() == lineArt.getString(k) } }.toSet(),
                        colors = (0 until colors.length()).map { k -> colors.getJSONArray(k).let { QvpPartColor(it.getInt(0), it.getInt(1), it.getLong(2).toInt()) } },
                    )
                    val s = c.getJSONObject("spec")
                    val spec = QvpPassageSpec(
                        width = s.getDouble("width").toFloat(), scale = s.getDouble("scale").toFloat(),
                        align = QvpPassageAlign.entries.first { it.name.lowercase() == s.getString("align") },
                        maxRows = s.getInt("maxRows"), keepAyahMark = s.getBoolean("keepAyahMark"),
                        omitAyahMarks = s.getBoolean("omitAyahMarks"), ellipsisWidth = s.getDouble("ellipsisWidth").toFloat(),
                    )
                    val passage = if (alone) QvpPassage.ayahMark(listOf(page), surah, from) else QvpPassage.load(listOf(page), surah, from, to)
                    passage.use { p ->
                        p.layout(spec)
                        assertTrue("$tag: the engine refused the dress", p.dress(set, dress))
                        assertNull("$tag: the dress drops the layout", p.currentLayout)
                        val l = p.layout(spec)!!
                        val want = c.getJSONObject("layout")
                        close(l.width, want.getDouble("width"), "$tag width")
                        close(l.height, want.getDouble("height"), "$tag height")
                        close(l.scale, want.getDouble("scale"), "$tag scale")
                        assertEquals("$tag draws", want.getInt("draws"), l.draws.size)
                        assertEquals("$tag words", want.getInt("words"), l.words.size)
                        assertEquals("$tag placements", want.getInt("placements"), l.placements.size)
                        val ornaments = c.getJSONArray("ornaments")
                        assertEquals("$tag ornaments", ornaments.length(), l.ornaments.size)
                        for (k in 0 until ornaments.length()) {
                            val o = ornaments.getJSONArray(k)
                            val got = l.ornaments[k]
                            assertEquals("$tag ornament $k placement", o.getInt(0), got.placement)
                            assertEquals("$tag ornament $k colour", o.getLong(1).toInt(), got.color)
                            assertEquals("$tag ornament $k kind", QvpOrnamentKind.AYAH_MARK, got.kind)
                            assertEquals("$tag ornament $k stroke", o.getInt(2) and 0x200 != 0, got.stroke)
                            close(got.strokeWidth, o.getDouble(3), "$tag ornament $k stroke width")
                            // the synthetic set's outlines are straight lines, so the path's box is the points'
                            val b = RectF()
                            got.path.computeBounds(b, true)
                            listOf(b.left, b.top, b.right, b.bottom).forEachIndexed { j, v -> close(v, o.getDouble(4 + j), "$tag ornament $k box[$j]") }
                        }
                        p.undress()
                        assertTrue("$tag undressed", p.layout(spec)!!.ornaments.isEmpty())
                    }
                }
            }
        }
    }
}
