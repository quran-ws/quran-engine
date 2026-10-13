package ws.quran.qvp

import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The dress section of the ABI through the JNI bridge: an ornament set, a dress, its readout and its geometry. */
@RunWith(AndroidJUnit4::class)
class QvpDressTest {
    private fun pageBytes(): ByteArray = InstrumentationRegistry.getInstrumentation()
        .context.assets.open("001.qvp").use { it.readBytes() }

    /** Three ornaments of the style "test": a font medallion, a scanned band and a scanned border without
     * slices, each with a fill part "c1" and a stroke part "line". The bytes are those of `ornament_bytes()`
     * in crates/qvp-ffi/tests/abi.rs, written by the engine's own encoder. */
    private val ornamentSet: ByteArray = Base64.decode(
            "UVZPMegDAwABBHRlc3QOSGFmcyAnYW4gJ0FzaW0PQ0MtQlktTkMtU0EtNC4wC3Byb3Zpc2lvbmFsAAZhIHRlc3QCAmMx/5tkAAAE" +
            "bGluZf9BNjABAAAAAAAAAAAAAKBCAADIQgEAAKBBAACgQQAAIEIAAHBCAAIAAQAEdGVzdA5IYWZzICdhbiAnQXNpbQ9DQy1CWS1O" +
            "Qy1TQS00LjALcHJvdmlzaW9uYWwABmEgdGVzdAICYzH/m2QAAARsaW5l/0E2MAEAAAAAAAAAAAAASEQAAMhCAQAAyEIAACBBAAAW" +
            "RAAAoEICAgACAAR0ZXN0DkhhZnMgJ2FuICdBc2ltD0NDLUJZLU5DLVNBLTQuMAtwcm92aXNpb25hbAAGYSB0ZXN0AgJjMf+bZAAA" +
            "BGxpbmX/QTYwAQAAAAAAAAAAAACMQgAAyEIBAACgQAAAoEAAAHBCAAC0QgQCAAYAAAAAEwECyAETEwAAACYTAQLIATkTAAAATBMB" +
            "AsgBXxNyAAAAAYDiCQABAMCaDAH/4QkABAAAAAGA4gkAAQDAmgwB/+EJAAQAAAABgNRhAAEAwJoMAf/TYQAEAAAAAYDUYQABAMCa" +
            "DAH/02EABAAAAAHgxQgAAQDAmgwB38UIAAQAAAAB4MUIAAEAwJoMAd/FCAAE",
        Base64.DEFAULT)

    @Test
    fun theSetReadsBack() {
        QvpOrnaments(ornamentSet).use { set ->
            assertEquals(3, set.count)
            assertEquals(3, set.list.size)
            val mark = set.list[0]
            assertEquals(QvpOrnamentKind.AYAH_MARK, mark.kind)
            assertEquals(QvpLineage.FONT, mark.lineage)
            assertEquals("test", mark.style)
            assertEquals("Hafs 'an 'Asim", mark.riwayah)
            assertFalse(mark.hasSlices)
            assertEquals(QvpLicence("CC-BY-NC-SA-4.0", "provisional", false, "a test"), mark.licence)
            assertEquals(2, mark.parts.size)
            assertEquals(QvpOrnamentPart(1, 0x303641ff, true, "line"), mark.parts[1])
            assertEquals(QvpLineage.SCAN, set.list[1].lineage)

            assertEquals(2, set.find(QvpOrnamentKind.PAGE_FRAME, "test")?.index)
            assertNull(set.find(QvpOrnamentKind.AYAH_MARK, "nope"))
            assertEquals(listOf(set.list[1]), set.ofKind(QvpOrnamentKind.SURAH_HEADER))
            val styles = set.styles()
            assertEquals(1, styles.size)
            assertEquals(set.list[0], styles[0].ayahMark)
            assertEquals(set.list[1], styles[0].surahHeader)
            assertEquals(set.list[2], styles[0].pageFrame)
        }
        assertThrows(IllegalArgumentException::class.java) { QvpOrnaments(byteArrayOf(1, 2, 3)) }
    }

    @Test
    fun aDressIsPlacedDrawnAndTakenOff() {
        val page = QvpPage(pageBytes())
        val set = QvpOrnaments(ornamentSet)
        try {
            val layout = page.layout(QvpLayoutSpec(690f, 1100f, 24f, 24f, 16f, 16f))
            assertNull(page.dressInfo())
            assertEquals(0, page.dressRevision())
            assertTrue(page.dressGeometry().isEmpty())

            // the style fills all three kinds; the border's fill is repainted red
            val info = page.dress(set, QvpDressSpec(style = "test", colors = listOf(QvpPartColor(2, 0, 0xff0000ff.toInt()))))
            assertNotNull(info)
            info!!
            assertEquals(0, info.ayahMark); assertEquals(1, info.surahHeader); assertEquals(2, info.pageFrame)
            assertTrue(info.ayahMarks > 0)
            assertTrue(info.surahHeaders > 0)
            assertTrue("no slices in this set", info.frameStretched)
            assertTrue(info.draws > 0)
            assertTrue(info.viewBox.x < 0f && info.viewBox.y < 0f && info.viewBox.width > page.width && info.viewBox.height > page.height)
            assertEquals(info, page.dressInfo())
            assertEquals(info.revision, page.dressRevision())

            // the dress refit the page: the held layout is the engine's, and the border is in view
            val fit = page.currentLayout!!
            assertTrue(fit.fitScale <= layout.fitScale)
            val held = page.dressClamp(QvpView(fit.fitScale, fit.fitX, fit.fitY), 690f, 1100f)
            assertEquals(fit.fitX, held.offsetX, 0.5f)
            assertEquals(fit.fitY, held.offsetY, 0.5f)

            // a host that scrolls the page: the size is content plus overflow, the border's top at 0
            val shown = page.dressView(1f, 690f)
            val o = info.overflow
            assertEquals(fit.contentW + o.left + o.right, shown.width, 1e-3f)
            assertEquals(fit.contentH + o.top + o.bottom, shown.height, 1e-3f)
            assertEquals(0f, shown.view.offsetY - shown.view.scale * o.top, 1e-3f)

            // one draw per table row, each under a placement of the layout or the border's none
            val draws = page.dressGeometry()
            assertEquals(info.draws, draws.size)
            val placements = page.layoutPlacements().size
            assertTrue(draws.all { d -> d.placement == null || d.placement in 0 until placements })
            assertTrue(draws.any { it.placement == null && it.kind == QvpOrnamentKind.PAGE_FRAME })
            assertTrue(draws.any { it.stroke && it.strokeWidth > 0f })
            assertTrue(draws.any { it.kind == QvpOrnamentKind.PAGE_FRAME && it.color == 0xff0000ff.toInt() })
            assertTrue(draws.all { !it.path.isEmpty })
            // the same revision hands back the same list
            assertTrue(draws === page.dressGeometry())

            // a new layout re-places the dress
            page.layout(QvpLayoutSpec(690f, 1300f, 24f, 24f, 16f, 16f))
            assertTrue(page.dressRevision() != info.revision)
            assertTrue(layout.contentH > 0f)

            // a kind key overrides the style, and line art reaches the engine
            val mixed = page.dress(set, QvpDressSpec(style = "test", pageFrame = QvpOrnamentRef.None,
                                                     ayahMark = QvpOrnamentRef.Index(0), lineArt = QvpDressSpec.ALL_LINE_ART))
            assertNotNull(mixed)
            assertNull(mixed!!.pageFrame)
            assertTrue(page.dressGeometry().all { it.stroke })

            // refused: an ornament of another kind, and nothing to dress
            assertNull(page.dress(set, QvpDressSpec(ayahMark = set.list[2])))
            assertNull(page.dress(set, QvpDressSpec()))
            assertNull(page.dress(set, QvpDressSpec(style = "nope")))

            page.dress(set, QvpDressSpec(style = "test"))
            page.undress()
            assertNull(page.dressInfo())
            assertTrue(page.dressGeometry().isEmpty())

            // the page keeps what it needs: the set can close while the page is dressed
            assertNotNull(page.dress(set, QvpDressSpec(surahHeader = QvpOrnamentRef.Style("test"))))
            set.close()
            assertTrue(set.isClosed)
            assertEquals(page.dressInfo()!!.draws, page.dressGeometry().size)
        } finally {
            set.close()
            page.close()
        }
    }
}
