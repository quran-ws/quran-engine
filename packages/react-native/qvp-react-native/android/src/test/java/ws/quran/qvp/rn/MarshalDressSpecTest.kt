package ws.quran.qvp.rn

import org.junit.Assert.assertEquals
import org.junit.Test
import ws.quran.qvp.QvpDressSpec
import ws.quran.qvp.QvpOrnamentKind
import ws.quran.qvp.QvpOrnamentRef
import ws.quran.qvp.QvpPartColor

/**
 * [Marshal.dressSpec] on the plain values that `ReadableMap.toHashMap()` gives: a number is a
 * Double, an array is an ArrayList, a map is a HashMap.
 */
class MarshalDressSpecTest {
    private val defaults = QvpDressSpec()

    /** The QVP_LINE_ART_* bits, as the library computes them for the bridge. */
    private fun lineArtBits(s: QvpDressSpec): Int = s.lineArt.fold(0) { m, k -> m or (1 shl k.id) }

    /** The flat colour record, as the library gives it to the bridge. */
    private fun colorTriples(s: QvpDressSpec): List<Int> = s.colors.flatMap { listOf(it.ornament, it.part, it.color) }

    @Test
    fun fullSpecGivesEveryField() {
        val spec = Marshal.dressSpec(hashMapOf(
            "ayahMark" to 3.0,
            "surahHeader" to "madinah",
            "pageFrame" to 0.0,
            "gap" to 7.5,
            "bandGap" to 2.0,
            "ayahMarkScale" to 1.25,
            "pageFrameScale" to 0.5,
            "lineArt" to arrayListOf("ayah_mark", "page_frame"),
            "colors" to arrayListOf(
                arrayListOf(3.0, 1.0, "#1a73e8"),
                arrayListOf(0.0, 2.0, "#abc"),
                arrayListOf(0.0, 4.0, "#11223344"),
                arrayListOf(1.0, 0.0, 4278190335.0),
            ),
        ))
        assertEquals(QvpOrnamentRef.Index(3), spec.ayahMark)
        assertEquals(QvpOrnamentRef.Style("madinah"), spec.surahHeader)
        assertEquals(QvpOrnamentRef.Index(0), spec.pageFrame)
        assertEquals(null, spec.style)
        assertEquals(7.5f, spec.gap, 0f)
        assertEquals(2f, spec.bandGap, 0f)
        assertEquals(1.25f, spec.ayahMarkScale, 0f)
        assertEquals(0.5f, spec.pageFrameScale, 0f)
        assertEquals(setOf(QvpOrnamentKind.AYAH_MARK, QvpOrnamentKind.PAGE_FRAME), spec.lineArt)
        assertEquals(0b101, lineArtBits(spec))
        assertEquals(listOf(
            3, 1, 0x1a73e8ff,
            0, 2, 0xaabbccff.toInt(),
            0, 4, 0x11223344,
            1, 0, 0xff0000ff.toInt(),
        ), colorTriples(spec))
    }

    @Test
    fun lineArtTakesNamesInAnyCaseAndIndices() {
        val spec = Marshal.dressSpec(hashMapOf("lineArt" to arrayListOf("SURAH_HEADER", 0.0, 2.0)))
        assertEquals(QvpDressSpec.ALL_LINE_ART, spec.lineArt)
        assertEquals(0b111, lineArtBits(spec))
    }

    @Test
    fun missingKeysKeepTheDefaults() {
        val spec = Marshal.dressSpec(hashMapOf<String, Any?>())
        assertEquals(QvpOrnamentRef.None, spec.ayahMark)
        assertEquals(QvpOrnamentRef.None, spec.surahHeader)
        assertEquals(QvpOrnamentRef.None, spec.pageFrame)
        assertEquals(defaults.gap, spec.gap, 0f)
        assertEquals(defaults.bandGap, spec.bandGap, 0f)
        assertEquals(defaults.ayahMarkScale, spec.ayahMarkScale, 0f)
        assertEquals(defaults.pageFrameScale, spec.pageFrameScale, 0f)
        assertEquals(0, lineArtBits(spec))
        assertEquals(emptyList<QvpPartColor>(), spec.colors)
    }

    @Test
    fun aValueThatIsNotAMapGivesTheDefaultSpec() {
        assertEquals(defaults, Marshal.dressSpec(null))
        assertEquals(defaults, Marshal.dressSpec("madinah"))
        assertEquals(defaults, Marshal.dressSpec(arrayListOf(1.0, 2.0)))
    }

    @Test
    fun anUnparseableColourIsZero() {
        val spec = Marshal.dressSpec(hashMapOf("colors" to arrayListOf(
            arrayListOf(0.0, 1.0, "#zzzzzz"),
            arrayListOf(0.0, 2.0, ""),
            arrayListOf(0.0, 3.0, true),
            arrayListOf(0.0, 4.0),
        )))
        assertEquals(listOf(0, 1, 0, 0, 2, 0, 0, 3, 0, 0, 4, 0), colorTriples(spec))
    }

    @Test
    fun aColourEntryWithoutAnOrnamentOrPartIsDropped() {
        val spec = Marshal.dressSpec(hashMapOf("colors" to arrayListOf(
            arrayListOf("x", 1.0, "#ffffff"),
            arrayListOf(0.0, null, "#ffffff"),
            arrayListOf(0.0),
            "#ffffff",
            null,
            arrayListOf("2", "5", "#000000"),
        )))
        assertEquals(listOf(QvpPartColor(2, 5, 0x000000ff)), spec.colors)
    }

    @Test
    fun wrongTypesFallBack() {
        val spec = Marshal.dressSpec(hashMapOf(
            "ayahMark" to true,
            "surahHeader" to hashMapOf("style" to "madinah"),
            "pageFrame" to arrayListOf(1.0),
            "gap" to true,
            "bandGap" to "3.5",
            "ayahMarkScale" to "big",
            "pageFrameScale" to arrayListOf(2.0),
            "lineArt" to "ayah_mark",
            "colors" to hashMapOf("0" to "#ffffff"),
        ))
        assertEquals(QvpOrnamentRef.None, spec.ayahMark)
        assertEquals(QvpOrnamentRef.None, spec.surahHeader)
        assertEquals(QvpOrnamentRef.None, spec.pageFrame)
        assertEquals(defaults.gap, spec.gap, 0f)
        assertEquals(3.5f, spec.bandGap, 0f)
        assertEquals(defaults.ayahMarkScale, spec.ayahMarkScale, 0f)
        assertEquals(defaults.pageFrameScale, spec.pageFrameScale, 0f)
        assertEquals(emptySet<QvpOrnamentKind>(), spec.lineArt)
        assertEquals(emptyList<QvpPartColor>(), spec.colors)
    }

    @Test
    fun unknownLineArtKindsAreDropped() {
        val spec = Marshal.dressSpec(hashMapOf("lineArt" to arrayListOf("frame", 9.0, -1.0, null, "ayah_mark")))
        assertEquals(setOf(QvpOrnamentKind.AYAH_MARK), spec.lineArt)
        assertEquals(0b1, lineArtBits(spec))
    }
}
