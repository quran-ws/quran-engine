package ws.quran.qvp.rn

import org.junit.Assert.assertEquals
import org.junit.Test
import ws.quran.qvp.BandHeight
import ws.quran.qvp.Division
import ws.quran.qvp.Form
import ws.quran.qvp.HighlightMode
import ws.quran.qvp.MaskMode
import ws.quran.qvp.SearchMode

/**
 * The names the bridge reads and sends for the library's enums. JavaScript sends the camelCase
 * names of the TypeScript types (`'rasmImlai'`), and older callers send the snake or upper
 * spellings. All three name the same constant, and a word's `forms` keys come back in camelCase.
 */
class MarshalNamesTest {
    @Test
    fun aFormTakesEverySpellingOfItsName() {
        for (name in listOf("rasmImlai", "rasm_imlai", "RASM_IMLAI", "RasmImlai")) {
            assertEquals(name, Form.RASM_IMLAI, Marshal.form(name))
        }
        assertEquals(Form.RASM_UTHMANI, Marshal.form("rasmUthmani", Form.SEARCH))
        assertEquals(Form.QPC, Marshal.form("qpc"))
        assertEquals(Form.SEARCH, Marshal.form("search"))
        assertEquals(Form.RASM, Marshal.form(3.0))
    }

    @Test
    fun anUnknownFormGivesTheDefault() {
        assertEquals(Form.RASM_UTHMANI, Marshal.form("notAForm"))
        assertEquals(Form.SEARCH, Marshal.form("", Form.SEARCH))
        assertEquals(Form.SEARCH, Marshal.form(null, Form.SEARCH))
        assertEquals(Form.RASM_UTHMANI, Marshal.form(9.0))
    }

    @Test
    fun aFormKeyIsTheTypeScriptName() {
        assertEquals(listOf("rasmUthmani", "rasmImlai", "qpc", "rasm", "search"), Form.entries.map { Marshal.jsName(it) })
        for (f in Form.entries) assertEquals(f, Marshal.form(Marshal.jsName(f)))
    }

    @Test
    fun theOtherEnumsTakeTheTypeScriptNames() {
        assertEquals(Division.RUBU_AL_HIZB, Marshal.division("rubuAlHizb"))
        assertEquals(Division.HIZB, Marshal.division("hizb"))
        assertEquals(SearchMode.PREFIX, Marshal.searchMode("prefix"))
        assertEquals(MaskMode.BLUR, Marshal.maskMode("blur"))
        val style = Marshal.highlightStyle(hashMapOf("mode" to "both", "height" to "lineSpacing"))
        assertEquals(HighlightMode.BOTH, style.mode)
        assertEquals(BandHeight.LINE_SPACING, style.height)
        assertEquals(BandHeight.INK, Marshal.highlightStyle(hashMapOf("height" to "ink")).height)
    }
}
