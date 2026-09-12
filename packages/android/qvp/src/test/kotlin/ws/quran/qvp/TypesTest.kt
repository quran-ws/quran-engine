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
}
