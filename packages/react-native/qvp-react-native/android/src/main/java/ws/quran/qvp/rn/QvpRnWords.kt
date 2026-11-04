package ws.quran.qvp.rn

import ws.quran.qvp.Form
import ws.quran.qvp.QvpPage

/**
 * The bridge record of each word of one page, built the first time the word is asked for and
 * kept. A record holds the word's text forms, its label and the names of its paths. The words
 * sidecar is attached once per page, so a record does not change until the next attach, and
 * the view drops this table then. The module's query thread and the UI thread can both call it.
 */
class QvpRnWords(val page: QvpPage) {
    /** The forms this page has. Each one is a scan of every word in the engine, so read them once. */
    val forms: List<Form> by lazy { Marshal.forms(page) }
    private val records = arrayOfNulls<Map<String, Any?>>(page.nWords)

    /** The record of word [index], or null when the page has no such word. */
    @Synchronized
    fun word(index: Int): Map<String, Any?>? {
        val w = page.words.getOrNull(index) ?: return null
        return records[index] ?: Marshal.word(page, w, forms).also { records[index] = it }
    }

    /** Every word's record, in page order. */
    fun all(): List<Map<String, Any?>> = List(page.nWords) { word(it)!! }
}
