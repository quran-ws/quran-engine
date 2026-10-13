package ws.quran.qvp

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class TypesTest {
    @Test
    fun colorsRoundTripBetweenRgbaAndArgb() {
        val rgba = 0x1a73e8cc
        assertEquals(rgba, QvpColor.rgba(QvpColor.argb(rgba)))
        assertEquals(0x1a73e8ff, QvpColor.parse("#1a73e8"))
        assertEquals(0xaabbccff.toInt(), QvpColor.parse("#abc"))
        assertEquals(0x1a73e87f, QvpColor.withAlpha(0x1a73e8ff, 0.5f))
    }

    @Test
    fun targetParserMatchesTypedTargets() {
        assertArrayEquals(Target.page().arr, Target.parse("page").arr)
        assertArrayEquals(Target.ayah(2, 255).arr, Target.parse("2:255").arr)
        assertArrayEquals(Target.ayahRange(2, 255, 257).arr, Target.parse("2:255-257").arr)
        assertArrayEquals(Target.line(7).arr, Target.parse("line:7").arr)
        assertArrayEquals(Target.surah(36).arr, Target.parse("surah:36").arr)
        assertThrows(IllegalArgumentException::class.java) { Target.parse("not-a-target") }
    }

    /** The record the JNI bridge reads a passage spec from, in the order qvp_jni.c states. */
    @Test
    fun passageSpecRecordMatchesTheBridge() {
        assertArrayEquals(floatArrayOf(0f, 1f, 1f, 2f, 0f, 0f, 0f, 0f, QvpDefaults.REFLOW_MAX_STRETCH), QvpPassageSpec().floats(), 0f)
        val spec = QvpPassageSpec(width = 366f, scale = 0.8f, lineSpacing = 1.2f, padding = 4f, align = QvpPassageAlign.JUSTIFIED,
                                  maxRows = 2, keepAyahMark = true, ellipsisWidth = 12f, maxStretch = -1f)
        assertArrayEquals(floatArrayOf(366f, 0.8f, 1.2f, 4f, 2f, 1f, 2f, 12f, -1f), spec.floats(), 0f)
    }

    /** The dress spec and readout records, in the order qvp_jni.c states. */
    @Test
    fun dressRecordsMatchTheBridge() {
        assertEquals(0, QvpDressSpec().lineArtBits())
        assertEquals(7, QvpDressSpec(lineArt = QvpDressSpec.ALL_LINE_ART).lineArtBits())
        assertEquals(2, QvpDressSpec(lineArt = setOf(QvpOrnamentKind.SURAH_HEADER)).lineArtBits())
        assertEquals(null, QvpDressSpec().colorTriples())
        assertArrayEquals(intArrayOf(2, 0, 0xff0000ff.toInt(), 1, 1, 0x00ff00ff),
                          QvpDressSpec(colors = listOf(QvpPartColor(2, 0, 0xff0000ff.toInt()), QvpPartColor(1, 1, 0x00ff00ff))).colorTriples())
        val bits = { f: Float -> f.toRawBits() }
        val info = QvpDressInfo.of(intArrayOf(0, 1, -1, 7, 1, 0, 1, 18, 3,
                                              bits(-34.5f), bits(-23.5f), bits(414f), bits(622f), bits(50f), bits(45f), bits(49f), bits(44f)))
        assertEquals(QvpDressInfo(0, 1, null, 7, 1, 0, true, 18, 3, QvpViewBox(-34.5f, -23.5f, 414f, 622f), QvpInsets(50f, 45f, 49f, 44f)), info)
    }

    @Test
    fun ornamentLayerStaysWithinABitmapAndTheBudget() {
        val max = 4096
        assertEquals(1000, QvpPageView.ornamentLayerHeight(999.2f, 1080, 64 shl 20))
        // band plus border past the bitmap limit is cut at the bottom, never past 4096
        assertEquals(max, QvpPageView.ornamentLayerHeight(8000f, 1080, 64 shl 20))
        // the budget shrinks it: 1080 px wide is 4320 bytes a row
        assertEquals(1000, QvpPageView.ornamentLayerHeight(3000f, 1080, 1080 * 4 * 1000))
        // not one row fits, or no width yet: skipped
        assertEquals(0, QvpPageView.ornamentLayerHeight(3000f, 1080, 1000))
        assertEquals(0, QvpPageView.ornamentLayerHeight(3000f, 0, 64 shl 20))
        assertEquals(0, QvpPageView.ornamentLayerHeight(3000f, 1080, 0))
    }
}
