package ws.quran.qvp.rn

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import ws.quran.qvp.QvpLetter
import ws.quran.qvp.QvpLetterHit
import ws.quran.qvp.QvpLetterMember
import ws.quran.qvp.QvpLetterPart
import ws.quran.qvp.QvpLetterQuery
import ws.quran.qvp.QvpPathLetter
import ws.quran.qvp.Selector
import ws.quran.qvp.Target

/**
 * The letter marshalling on the plain values that `ReadableMap.toHashMap()` gives: a number is a
 * Double, an array is an ArrayList, a map is a HashMap. A mark given by name goes to the engine,
 * which these JVM tests do not load, so they give marks by id.
 */
class MarshalLettersTest {
    /** The record the library hands the bridge for a target or a selector: {code, a, b, c, words...}. */
    private fun record(o: Any): IntArray = o.javaClass.getDeclaredField("arr").apply { isAccessible = true }.get(o) as IntArray

    @Test
    fun aQueryMarshalsFromAJsMap() {
        val q = Marshal.letterQuery(hashMapOf(
            "scope" to hashMapOf("target" to 3.0, "a" to 2.0, "b" to 255.0),
            "text" to "ب",
            "mark" to 12.0,
            "part" to 1.0,
        ), null)
        assertEquals(0x0628, q.codePoint)
        assertEquals("ب", q.text)
        assertEquals(12, q.mark)
        assertEquals(QvpLetterPart.BODY_AND_DOTS, q.part)
        assertArrayEquals(intArrayOf(3, 2, 255, 0), record(q.scope))
    }

    @Test
    fun aQueryTakesTheCodePointTheScopeByNameAndAWordList() {
        // `text` as a number is the code point; a target code by its name; a word list
        val q = Marshal.letterQuery(hashMapOf("text" to 1604.0, "scope" to hashMapOf("target" to "words", "words" to arrayListOf(4.0, 9.0))), null)
        assertEquals(0x0644, q.codePoint)
        assertEquals("\u0644", q.text)
        assertArrayEquals(intArrayOf(2, 0, 0, 0, 4, 9), record(q.scope))
        // a character past the BMP is one code point, not a surrogate
        assertEquals(0x1D7D8, Marshal.letterQuery(hashMapOf("text" to "𝟘"), null).codePoint)
        assertEquals("𝟘", Marshal.letterQuery(hashMapOf("text" to "𝟘"), null).text)
        assertArrayEquals(intArrayOf(5, 7, 0, 0), record(Marshal.letterQuery(hashMapOf("scope" to "line:7"), null).scope))
        // a string that holds a number is a character, not a code point
        assertEquals('5'.code, Marshal.letterQuery(hashMapOf("text" to "5"), null).codePoint)
    }

    @Test
    fun aTextOfMoreThanOneCharacterThrows() {
        for (bad in listOf("لا", "بَ", "\uD800")) {
            assertThrows(IllegalArgumentException::class.java) { Marshal.letterQuery(hashMapOf("text" to bad), null) }
        }
    }

    @Test
    fun aMissingFieldIsAny() {
        for (q in listOf(Marshal.letterQuery(hashMapOf<String, Any?>(), null), Marshal.letterQuery(null, null), Marshal.letterQuery(hashMapOf("text" to "", "mark" to ""), null))) {
            assertEquals(0, q.codePoint)
            assertEquals("", q.text)
            assertEquals(0, q.mark)
            assertEquals(QvpLetterPart.ALL, q.part)
            assertArrayEquals(record(Target.page()), record(q.scope))
        }
        assertEquals(QvpLetterQuery().part, Marshal.letterQuery(null, null).part)
    }

    @Test
    fun theLetterSelectorsCarryIds16To19() {
        val cases = mapOf(
            hashMapOf("selector" to 16.0, "a" to 4.0, "b" to 2.0, "c" to 2.0) to intArrayOf(16, 4, 2, 2),
            hashMapOf("selector" to "letter", "a" to 4.0, "b" to 2.0) to intArrayOf(16, 4, 2, 0),
            hashMapOf("selector" to 17.0, "a" to 4.0, "b" to 2.0, "c" to 1.0) to intArrayOf(17, 4, 2, 1),
            hashMapOf("selector" to "letterMarkNamed", "a" to 4.0, "b" to 2.0, "c" to 12.0) to intArrayOf(18, 4, 2, 12),
            hashMapOf("selector" to 19.0, "a" to 4.0, "b" to 1.0, "c" to 3.0) to intArrayOf(19, 4, 1, 3),
        )
        for ((js, want) in cases) assertArrayEquals(js.toString(), want, record(Marshal.selector(js)!!))
        assertArrayEquals(record(Selector.letter(4, 2, QvpLetterPart.MARKS)), record(Marshal.selector(hashMapOf("selector" to "letter", "a" to 4.0, "b" to 2.0, "c" to 2.0))!!))
        assertEquals(19, Codes.SELECTORS["wordChars"])
    }

    @Test
    fun theLetterRecordHoldsItsMembersAndItsMarksInTheEngineOrder() {
        val body = QvpLetterMember(path = 9, at = 2, mark = 0, markName = "", isBody = true, isCut = true)
        val dot = QvpLetterMember(path = 4, at = 2, mark = 16, markName = "dot", isBody = false, isCut = false)
        val fathah = QvpLetterMember(path = 5, at = -1, mark = 1, markName = "fathah", isBody = false, isCut = false)
        val l = QvpLetter(word = 3, index = 1, at = 2, codePoint = 0x0628, text = "ب", moreAt = -1, nMoreAt = 0, nMembers = 3, nMarks = 2,
                          x0 = 1f, y0 = 2f, x1 = 3f, y1 = 4f, hasInk = true, members = listOf(body, dot, fathah))
        val m = Marshal.letter(l)
        assertEquals(listOf("word", "index", "at", "codePoint", "text", "moreAt", "nMoreAt", "nMembers", "nMarks", "x0", "y0", "x1", "y1", "hasInk", "members", "marks"), m.keys.toList())
        assertEquals(listOf(3, 1, 2, 0x0628, "ب", -1, 0, 3, 2, 1f, 2f, 3f, 4f, true), m.values.take(14))
        val member = mapOf("path" to 9, "at" to 2, "mark" to 0, "markName" to "", "isBody" to true, "isCut" to true)
        assertEquals(member, Marshal.letterMember(body))
        assertEquals(listOf(9, 4, 5), (m["members"] as List<*>).map { (it as Map<*, *>)["path"] })
        assertEquals(listOf(4, 5), (m["marks"] as List<*>).map { (it as Map<*, *>)["path"] })
        assertEquals(-1, ((m["marks"] as List<*>)[1] as Map<*, *>)["at"])
    }

    @Test
    fun theHitRecordSaysWhetherThePathIsBody() {
        val h = QvpLetterHit(word = 3, letter = -1, path = 7, at = 5, line = 2, distance = 0.5f, mark = 12, markName = "omitted_alif", isExact = false, isBody = true)
        assertEquals(mapOf("word" to 3, "letter" to -1, "path" to 7, "at" to 5, "line" to 2, "distance" to 0.5f, "mark" to 12, "markName" to "omitted_alif",
                           "isExact" to false, "isBody" to true), Marshal.letterHit(h))
        assertEquals(mapOf("letter" to -1, "at" to 4), Marshal.pathLetter(QvpPathLetter(-1, 4)))
        assertEquals(listOf(mapOf("word" to 0, "letter" to 1), mapOf("word" to 3, "letter" to 0)), Marshal.foundLetters(listOf(0 to 1, 3 to 0)))
    }
}
