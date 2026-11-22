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
        // alpha rounds to the nearest step: 0.5 is 0x80 in every wrapper, not 0x7f
        assertEquals(0x1a73e880, QvpColor.withAlpha(0x1a73e8ff, 0.5f))
        assertEquals(0x1a73e880, QvpColor.parse("#1a73e8", 0.5f))
        assertEquals(0xd6a3264d.toInt(), QvpColor.parse("#d6a326", 0.3f))
        // a NaN alpha is 0 and an out-of-range one clamps, in every wrapper
        assertEquals(0x1a73e800, QvpColor.parse("#1a73e8", Float.NaN))
        assertEquals(0x1a73e8ff, QvpColor.withAlpha(0x1a73e800, 2f))
    }

    /** `QvpDefaults.all` names every constant of the object, so a host that reads the table whole misses none. */
    @Test
    fun defaultsTableListsEveryConstant() {
        val camel = { name: String -> name.lowercase().split('_').let { it[0] + it.drop(1).joinToString("") { p -> p.replaceFirstChar(Char::uppercase) } } }
        val constants = QvpDefaults::class.java.declaredFields
            .filter { java.lang.reflect.Modifier.isStatic(it.modifiers) && it.type.isPrimitive }
            .map { camel(it.name) }.toSet()
        assertEquals(constants, QvpDefaults.all.keys)
        assertEquals(QvpDefaults.SELECTION_PAD_X, QvpDefaults.all["selectionPadX"])
    }

    /** What the React Native bridge exports equals the decimals in qvp.h, not a widened float
     * (0.6f widened is 0.6000000238418579). The header copy is read from the module's sources. */
    @Test
    fun exportedDefaultsEqualTheHeaderDecimals() {
        val header = generateSequence(java.io.File(System.getProperty("user.dir"))) { it.parentFile }
            .flatMap { sequenceOf(java.io.File(it, "src/main/cpp/qvp.h"), java.io.File(it, "packages/android/qvp/src/main/cpp/qvp.h")) }
            .firstOrNull { it.isFile } ?: error("qvp.h not found above " + System.getProperty("user.dir"))
        val camel = { name: String -> name.lowercase().split('_').let { it[0] + it.drop(1).joinToString("") { p -> p.replaceFirstChar(Char::uppercase) } } }
        val fromHeader = Regex("""#define QVP_DEFAULT_([A-Z_]+)\s+(0x[0-9a-fA-F]+u?|[0-9.]+f?)""").findAll(header.readText()).associate { m ->
            val raw = m.groupValues[2]
            // `u` closes a hex colour and `f` a decimal; a hex digit f is part of the number
            camel(m.groupValues[1]) to (if (raw.startsWith("0x")) raw.removePrefix("0x").removeSuffix("u").toLong(16).toDouble() else raw.removeSuffix("f").toDouble())
        }
        assertEquals(fromHeader, QvpDefaults.decimal)
        assertEquals(0.6, QvpDefaults.decimal["selectionPadX"])
        assertEquals(1.2, QvpDefaults.decimal["highlightPadX"])
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

    /** The letter selectors carry the header's ids (QVP_SELECTOR_LETTER 16 to WORD_CHARS 19) and
     * their arguments in the order qvp_jni.c reads them: {selector, a, b, c}. */
    @Test
    fun letterSelectorsCarryTheHeaderIds() {
        assertArrayEquals(intArrayOf(16, 3, 1, 0), Selector.letter(3, 1).arr)
        assertArrayEquals(intArrayOf(16, 3, 1, 2), Selector.letter(3, 1, QvpLetterPart.MARKS).arr)
        assertArrayEquals(intArrayOf(17, 3, 1, 4), Selector.letterMark(3, 1, 4).arr)
        assertArrayEquals(intArrayOf(18, 3, 1, 16), Selector.letterMarkNamed(3, 1, 16).arr)
        assertArrayEquals(intArrayOf(19, 3, 0, 2), Selector.wordChars(3, 0, 2).arr)
        // QVP_NONE goes in as -1, which the bridge passes on as 0xFFFFFFFF
        assertArrayEquals(intArrayOf(19, 0, -1, -1), Selector.wordChars(0, -1, -1).arr)
        assertEquals(listOf(0, 1, 2), listOf(QvpLetterPart.ALL, QvpLetterPart.BODY_AND_DOTS, QvpLetterPart.MARKS))
    }

    /** A query reaches the bridge as {text, mark, part}, then the scope's target record. */
    @Test
    fun letterQueryRecordMatchesTheBridge() {
        assertArrayEquals(intArrayOf(0, 0, 0, 0, 0, 0, 0), QvpLetterQuery().ints())
        assertArrayEquals(intArrayOf(0x644, 16, 1, 1, 2, 0, 0), QvpLetterQuery(Target.word(2), 0x644, 16, QvpLetterPart.BODY_AND_DOTS).ints())
        assertArrayEquals(intArrayOf(0, 0, 2, 2, 0, 0, 0, 1, 1, 5), QvpLetterQuery(Target.words(intArrayOf(1, 1, 5)), part = QvpLetterPart.MARKS).ints())
        assertArrayEquals(intArrayOf(0, 255, 0, 4, 1, 1, 2), QvpLetterQuery(Target.ayahRange(1, 1, 2), mark = 255).ints())
        // a letter given as a character goes in as its code point; no mark name needs the engine here
        assertArrayEquals(intArrayOf(0x1F600, 0, 0, 0, 0, 0, 0), QvpLetterQuery.of(letter = "\uD83D\uDE00").ints())
        assertArrayEquals(intArrayOf(0x644, 0, 0, 0, 0, 0, 0), QvpLetterQuery.of(letter = "\u0644").ints())
    }

    /** The bridge's letter records map to the friendly types field by field: a swap of two fields
     * shows here without a device. */
    @Test
    fun letterRecordsMatchTheBridge() {
        val names = mapOf(0 to "", 1 to "fathah", 12 to "omitted_alif", 16 to "dot")
        val name = { m: Int -> names.getValue(m) }
        val bits = { f: Float -> f.toRawBits() }
        val members = QvpLetterMember.list(intArrayOf(8, 0, 0, 1, 1, 1, 0, 16, 0, 0, 2, 1, 1, 0, 0, 7, -1, 12, 0, 0), name)
        assertEquals(listOf(QvpLetterMember(8, 0, 0, "", isBody = true, isCut = true),
                            QvpLetterMember(1, 0, 16, "dot", isBody = false, isCut = false),
                            QvpLetterMember(2, 1, 1, "fathah", isBody = false, isCut = false),
                            QvpLetterMember(7, -1, 12, "omitted_alif", isBody = false, isCut = false)), members)
        val letter = QvpLetter.of(intArrayOf(4, 1, 2, 0x628, 4, 3, 5, 1, bits(50f), bits(6f), bits(60.5f), bits(24f), 1), members)
        assertEquals(QvpLetter(4, 1, 2, 0x628, "\u0628", 5, 1, 4, 3, 50f, 6f, 60.5f, 24f, true, members), letter)
        assertEquals(listOf(1, 2, 7), letter.marks.map { it.path })
        assertEquals(false, QvpLetter.of(intArrayOf(0, 0, -1, 0x648, 0, 0, -1, 0, 0, 0, 0, 0, 0), emptyList()).hasInk)
        val hit = QvpLetterHit.of(intArrayOf(2, -1, 5, 3, 1, bits(0.25f), 12, 0, 1), name)
        assertEquals(QvpLetterHit(2, -1, 5, 3, 1, 0.25f, 12, "omitted_alif", isExact = false, isBody = true), hit)
        assertEquals(QvpLetterHit(0, 0, 8, 0, 0, 0f, 0, "", isExact = true, isBody = false),
                     QvpLetterHit.of(intArrayOf(0, 0, 8, 0, 0, 0, 0, 1, 0), name))
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
    fun theOrnamentLayerIsTheBandTheBaseInkIs() {
        // A dressed page's band starts above the content, at the border's top: 14 px over it, and
        // the page's top is 483 px down the screen. The layer's top row is the band's top, so the
        // content is drawn 14 px down the bitmap, and the bitmap goes 14 px above the page's top.
        val b = QvpPageView.bandBitmap(-14f, 1625.2f, 483f)
        assertEquals(QvpPageView.BandBitmap(1626, 14f, 469f), b)
        // a magnified band far down the page: one screen of slack above the reader
        assertEquals(QvpPageView.BandBitmap(4096, -2544f, 456f), QvpPageView.bandBitmap(2544f, 4096f, -2088f))
        // under one row it still has one
        assertEquals(1, QvpPageView.bandBitmap(0f, 0.2f, 0f).rows)
    }

    @Test
    fun aBandBitmapHasWholeRowsAndAtLeastOne() {
        // the base ink and the ornament layer of a dressed page take the same rows
        assertEquals(1000, QvpPageView.bandRows(999.2f))
        assertEquals(1000, QvpPageView.bandRows(1000f))
        assertEquals(647, QvpPageView.bandRows(646.7246f))
        assertEquals(1, QvpPageView.bandRows(0f))
        assertEquals(1, QvpPageView.bandRows(0.2f))
        assertEquals(1, QvpPageView.bandRows(-5f))
        assertEquals(1, QvpPageView.bandRows(Float.NaN))
    }
}
