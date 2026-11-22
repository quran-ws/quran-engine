// Replays conformance/scenarios/letters.json against the engine through the Kotlin wrapper: a
// page's letter layer read, selected, coloured, drawn and hit-tested. The scenario file and its
// synthetic pages and layers reach the test assets through scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpLettersConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets
    private val replay = LettersReplay { name -> assets.open(name).use { it.readBytes() } }

    /** Every letter, member, path role and offset of each fixture, as the engine reads them. */
    @Test fun lettersMatchTheEngine() = replay.letters()

    /** Each letter selector: the paths it colours and the draw list that follows. */
    @Test fun selectionsMatchTheEngine() = replay.selections()

    /** Each letter query: what it finds, and what its rule colours and draws. */
    @Test fun queriesMatchTheEngine() = replay.queries()

    /** The letter under a point, in page units and in viewport px, printed and reflowed. */
    @Test fun hitsMatchTheEngine() = replay.hits()

    /** The ordered calls: attach, refuse, replace, fade, swap and the per-frame order. */
    @Test fun sequencesMatchTheEngine() = replay.sequences()

    /** Attach a layer, colour one letter and read it back: the piece of the lam is red, and the
     * draw list holds the other piece in place of the joined outline. */
    @Test
    fun attachColourOneLetterAndReadItBack() {
        QvpPage(replay.hex("qvp1-letters.hex")).use { p ->
            assertEquals(8, p.nPaths)
            p.buildPaths()
            val kept = p.path(0)
            assertTrue(p.attachLetters(replay.hex("qvl1-letters.hex")))
            assertEquals(10, p.nPaths)
            assertEquals(80, p.table.size)
            assertEquals(10, p.buildPaths().size)
            // the page's own paths keep the outlines already built
            assertTrue(kept === p.path(0))
            p.layout(QvpLayoutSpec(200f, 200f))
            val red = replay.red
            assertTrue(p.style(Selector.letter(0, 1), red) != 0)
            p.tick(0.0)
            assertEquals(10, p.colors().size)
            assertEquals(red, p.colors()[9])
            assertTrue(p.layoutDrawList().any { it.path == 8 })
            assertFalse(p.layoutDrawList().any { it.path == 0 })
            assertEquals(2, p.layoutDrawRevision())
            assertEquals(listOf(9), p.letters(0)[1].members.map { it.path })
            // a refused attach changes nothing
            assertFalse(p.attachLetters(byteArrayOf(1, 2, 3)))
            assertEquals(10, p.nPaths)
        }
    }
}

/**
 * The replay of letters.json, given a reader of the files the sync script copies. It calls only
 * the public Kotlin API, so a host sees what it checks.
 */
internal class LettersReplay(private val read: (String) -> ByteArray) {
    private val json = JSONObject(read("letters.json").toString(Charsets.UTF_8))
    private val tolerance = json.getDouble("tolerance")
    val red = json.getLong("red").toInt()
    private val fixtures = json.getJSONObject("fixtures")
    private val layouts = json.getJSONObject("layouts")
    private val words = json.getString("words")

    /** A hex fixture as bytes, whitespace ignored. */
    fun hex(name: String): ByteArray = hexOf(read(name).toString(Charsets.UTF_8))

    private fun JSONArray.objects() = List(length()) { getJSONObject(it) }
    private fun JSONArray.ints() = List(length()) { getLong(it).toInt() }
    private fun JSONArray.pairs() = List(length()) { getJSONArray(it).let { p -> p.getInt(0) to p.getInt(1) } }
    private fun close(got: Float, want: Double, what: String) =
        assertTrue("$what: got $got, engine says $want", Math.abs(got - want) <= tolerance * Math.max(1.0, Math.abs(want)))

    private fun pageOf(fixture: String) = QvpPage(hex(fixtures.getJSONObject(fixture).getString("page")))
    private fun layerOf(fixture: String) = hex(fixtures.getJSONObject(fixture).getString("letters"))

    private fun spec(name: String): QvpLayoutSpec {
        val s = layouts.getJSONObject(name)
        fun f(k: String) = s.getDouble(k).toFloat()
        val reflow = s.optJSONObject("reflow")?.let { QvpReflowSpec(it.getDouble("zoom").toFloat()) }
        return QvpLayoutSpec(f("viewportW"), f("viewportH"), f("padTop"), f("padBottom"), f("padLeft"), f("padRight"), reflow = reflow)
    }

    /** A selector through the public factories, so each id is checked against the header. */
    private fun selector(o: JSONObject): Selector {
        val a = o.getLong("a").toInt(); val b = o.getLong("b").toInt(); val c = o.getLong("c").toInt()
        return when (val id = o.getInt("selector")) {
            1 -> Selector.path(a)
            7 -> Selector.word(a)
            16 -> Selector.letter(a, b, c)
            17 -> Selector.letterMark(a, b, c)
            18 -> Selector.letterMarkNamed(a, b, c)
            19 -> Selector.wordChars(a, b, c)
            else -> error("no selector $id in letters.json")
        }
    }

    private fun target(o: JSONObject): Target {
        val a = o.getInt("a"); val b = o.getInt("b"); val c = o.getInt("c")
        return when (val t = o.getInt("target")) {
            0 -> Target.page()
            1 -> Target.word(a)
            2 -> Target.words(o.getJSONArray("words").ints())
            3 -> Target.ayah(a, b)
            4 -> Target.ayahRange(a, b, c)
            5 -> Target.line(a)
            6 -> Target.surah(a)
            7 -> Target.range(a, b)
            else -> error("no target $t in letters.json")
        }
    }

    private fun query(o: JSONObject) = QvpLetterQuery(target(o.getJSONObject("scope")), o.getInt("text"), o.getInt("mark"), o.getInt("part"))

    private fun options(o: JSONObject?) =
        if (o == null) QvpHitOptions()   // the defaults are what a NULL QvpHitOptions gives
        else QvpHitOptions(o.getDouble("maxDistance").toFloat(), o.getDouble("gapBias").toFloat(), o.getBoolean("preferExact"))

    /** A fresh page of a case: its fixture with the layer, the sidecar when asked, a layout, tick 0. */
    private fun start(c: JSONObject, layout: String?): QvpPage {
        val p = pageOf(c.getString("fixture"))
        assertTrue("${c.getString("name")}: attach", p.attachLetters(layerOf(c.getString("fixture"))))
        if (c.optBoolean("words", false)) p.attachWords(words)
        if (layout != null) p.layout(spec(layout))
        p.tick(0.0)
        return p
    }

    private fun sameMembers(got: List<QvpLetterMember>, want: JSONArray, what: String) {
        assertEquals("$what members", want.length(), got.size)
        for ((m, w) in got.zip(want.objects())) {
            assertEquals("$what member", listOf(w.getInt("path"), w.getInt("at"), w.getString("mark"), w.getBoolean("isBody"), w.getBoolean("isCut")),
                         listOf(m.path, m.at, m.markName, m.isBody, m.isCut))
        }
    }

    private fun sameLetter(got: QvpLetter?, want: JSONObject?, what: String) {
        if (want == null) return assertNull("$what: got a letter, engine says none", got)
        assertNotNull("$what: got no letter", got)
        got!!
        assertEquals(what, listOf(want.getInt("word"), want.getInt("index"), want.getInt("at"), want.getInt("text"), want.getString("char"),
                                  want.getInt("nMembers"), want.getInt("nMarks"), want.getInt("moreAt"), want.getInt("nMoreAt"), want.getBoolean("hasInk")),
                     listOf(got.word, got.index, got.at, got.text, got.character, got.nMembers, got.nMarks, got.moreAt, got.nMoreAt, got.hasInk))
        close(got.x0, want.getDouble("x0"), "$what x0"); close(got.y0, want.getDouble("y0"), "$what y0")
        close(got.x1, want.getDouble("x1"), "$what x1"); close(got.y1, want.getDouble("y1"), "$what y1")
        want.optJSONArray("members")?.let { sameMembers(got.members, it, what) }
        assertEquals("$what marks", got.members.filter { !it.isBody }, got.marks)
    }

    private fun samePathLetter(got: QvpPathLetter?, want: JSONObject?, what: String) {
        if (want == null) return assertNull(what, got)
        assertEquals(what, QvpPathLetter(want.getInt("letter"), want.getInt("at")), got)
    }

    private fun sameHit(got: QvpLetterHit?, want: JSONObject?, what: String) {
        if (want == null) return assertNull("$what: got a hit, engine says none", got)
        assertNotNull("$what: got no hit", got)
        got!!
        assertEquals(what, listOf(want.getInt("word"), want.getInt("letter"), want.getInt("path"), want.getInt("at"), want.getInt("line"),
                                  want.getString("mark"), want.getBoolean("isExact"), want.getBoolean("isBody")),
                     listOf(got.word, got.letter, got.path, got.at, got.line, got.markName, got.isExact, got.isBody))
        close(got.distance, want.getDouble("distance"), "$what distance")
    }

    private fun drawList(p: QvpPage) = p.layoutDrawList().map { it.path to it.placement }
    private fun redPaths(p: QvpPage) = p.colors().withIndex().filter { it.value == red }.map { it.index }

    private fun sameNPaths(p: QvpPage, want: Int, what: String) {
        assertEquals("$what nPaths", want, p.nPaths)
        assertEquals("$what geometry rows", want, p.table.size / 8)
        assertEquals("$what colours", want, p.colors().size)
    }

    fun letters() {
        val cases = json.getJSONArray("letters").objects()
        for (c in cases) {
            val name = c.getString("name")
            start(c, null).use { p ->
                assertEquals("$name hasLetters", c.getBoolean("hasLetters"), p.hasLetters())
                sameNPaths(p, c.getInt("nPaths"), name)
                assertEquals("$name counts", c.getJSONArray("counts").ints(), List(c.getJSONArray("counts").length()) { p.letterCount(it) })
                val items = c.getJSONArray("letters").objects()
                for (w in items.map { it.getInt("word") }.distinct()) {
                    val want = items.filter { it.getInt("word") == w }
                    val got = p.letters(w)
                    assertEquals("$name word $w letters", want.size, got.size)
                    for ((g, x) in got.zip(want)) sameLetter(g, x, "$name letter ${x.getInt("word")}/${x.getInt("index")}")
                }
                for (m in c.getJSONArray("missing").objects()) {
                    val w = m.getInt("word"); val l = m.getInt("letter")
                    sameLetter(p.letter(w, l), m.optJSONObject("info"), "$name missing $w/$l")
                    sameMembers(p.letterMembers(w, l), m.getJSONArray("members"), "$name missing $w/$l")
                }
                for (r in c.getJSONArray("pathLetters").objects()) {
                    samePathLetter(p.pathLetter(r.getInt("path")), r.optJSONObject("value"), "$name pathLetter ${r.getInt("path")}")
                }
                val offsets = c.getJSONArray("offsets")
                for (i in 0 until offsets.length()) {
                    val o = offsets.getJSONArray(i)
                    val want = if (o.isNull(2)) null else o.getInt(2)
                    assertEquals("$name letterAtOffset ${o.getInt(0)} ${o.getInt(1)}", want, p.letterAtOffset(o.getInt(0), o.getInt(1)))
                }
            }
        }
        assertEquals(4, cases.size)
    }

    fun selections() {
        val cases = json.getJSONArray("selections").objects()
        for (c in cases) {
            val name = c.getString("name")
            start(c, "printed").use { p ->
                assertEquals("$name handle", c.getBoolean("handle"), p.style(selector(c.getJSONObject("selector")), red, 0, 0) != 0)
                p.tick(0.0)
                assertEquals("$name red", c.getJSONArray("red").ints(), redPaths(p))
                assertEquals("$name drawRevision", c.getInt("drawRevision"), p.layoutDrawRevision())
                assertEquals("$name drawList", c.getJSONArray("drawList").pairs(), drawList(p))
            }
        }
        assertEquals(42, cases.size)
    }

    fun queries() {
        val cases = json.getJSONArray("queries").objects()
        for (c in cases) {
            val name = c.getString("name")
            start(c, "printed").use { p ->
                val q = query(c.getJSONObject("query"))
                assertEquals("$name found", c.getJSONArray("found").pairs(), p.findLetters(q))
                assertEquals("$name handle", c.getBoolean("handle"), p.styleLetters(q, red, 0, 0) != 0)
                p.tick(0.0)
                assertEquals("$name red", c.getJSONArray("red").ints(), redPaths(p))
                assertEquals("$name drawList", c.getJSONArray("drawList").pairs(), drawList(p))
            }
        }
        assertEquals(15, cases.size)
    }

    fun hits() {
        val cases = json.getJSONArray("hits").objects()
        for (c in cases) {
            val name = c.getString("name")
            start(c, if (c.isNull("layout")) null else c.getString("layout")).use { p ->
                val x = c.getDouble("x").toFloat(); val y = c.getDouble("y").toFloat()
                val o = options(c.optJSONObject("options"))
                val got = if (c.getString("units") == "view") p.hitTestLetterView(x, y, o) else p.hitTestLetter(x, y, o)
                sameHit(got, c.optJSONObject("hit"), "$name (${c.getString("units")} $x, $y)")
            }
        }
        assertEquals(34, cases.size)
    }

    fun sequences() {
        val cases = json.getJSONArray("sequences").objects()
        for (c in cases) {
            val name = c.getString("name")
            val fixture = c.getString("fixture")
            pageOf(fixture).use { p ->
                val handles = HashMap<String, Int>()
                for ((k, s) in c.getJSONArray("steps").objects().withIndex()) {
                    val what = "$name, step $k ${s.getString("call")}"
                    step(p, fixture, s, handles, what)
                }
            }
        }
        assertEquals(11, cases.size)
    }

    private fun step(p: QvpPage, fixture: String, s: JSONObject, handles: MutableMap<String, Int>, what: String) {
        fun int(k: String) = s.getLong(k).toInt()
        fun value() = s.get("value")
        when (s.getString("call")) {
            "tick" -> assertEquals(what, s.getBoolean("animating"), p.tick(s.getDouble("ms")))
            "attach" -> assertEquals(what, int("result"), if (p.attachLetters(layerOf(s.getString("letters")))) 0 else -1)
            "attachBytes" -> assertEquals(what, int("result"), if (p.attachLetters(hexOf(s.getString("hex")))) 0 else -1)
            "attachWords" -> assertEquals(what, int("result"), p.attachWords(words))
            "layout" -> p.layout(spec(s.getString("spec")))
            "style" -> p.style(selector(s.getJSONObject("selector")), int("rgba"), int("ms"), 0).let {
                assertEquals(what, s.getBoolean("handle"), it != 0); handles[s.getString("as")] = it
            }
            "styleLetters" -> p.styleLetters(query(s.getJSONObject("query")), int("rgba"), int("ms"), 0).let {
                assertEquals(what, s.getBoolean("handle"), it != 0); handles[s.getString("as")] = it
            }
            "remove" -> assertEquals(what, int("result"), p.removeStyle(handles.getValue(s.getString("handle"))))
            "theme" -> p.theme(QvpTheme(ink = int("ink"), dots = int("dots"), transitionMs = int("ms")))
            "highlight" -> handles[s.getString("as")] =
                p.highlight(Target.word(int("word")), QvpHighlightStyle(mode = HighlightMode.INK, ink = int("ink"), transitionMs = int("ms")))
            "removeHighlight" -> assertEquals(what, s.getBoolean("result"), p.removeHighlight(handles.getValue(s.getString("handle"))))
            // The Kotlin wrapper has no binding for qvp_mask_transition (a gap docs/API-PARITY.md
            // declares). No answer of the sequence depends on it: the theme's and the highlight's
            // fades outlast the mask's, so every `animating` is the same with an instant mask.
            "maskTransition" -> Unit
            "mask" -> p.mask(Target.word(int("word")), MaskMode.HIDE)
            "unmask" -> p.unmask()
            "hasLetters" -> assertEquals(what, s.getBoolean("value"), p.hasLetters())
            "nPaths" -> sameNPaths(p, int("value"), what)
            "rows" -> {
                val first = int("first"); val count = int("count")
                val want = s.getJSONArray("value").let { rows -> List(rows.length()) { rows.getJSONArray(it).ints() } }
                assertEquals(what, want, List(count) { r -> List(8) { p.table[(first + r) * 8 + it] } })
            }
            "colors" -> assertEquals(what, s.getJSONArray("value").ints(), p.colors().toList())
            "styledPaths" -> assertEquals(what, s.getJSONArray("value").let { a -> List(a.length()) { a.getJSONArray(it).ints() } },
                                          p.styledPaths().toList().chunked(2))
            "drawRevision" -> assertEquals(what, int("value"), p.layoutDrawRevision())
            "drawList" -> assertEquals(what, s.getJSONArray("value").pairs(), drawList(p))
            "letterCount" -> assertEquals(what, int("value"), p.letterCount(int("word")))
            "letterInfo" -> sameLetter(p.letter(int("word"), int("letter")), s.optJSONObject("value"), what)
            "letterMembers" -> sameMembers(p.letterMembers(int("word"), int("letter")), s.getJSONArray("value"), what)
            "pathLetter" -> samePathLetter(p.pathLetter(int("path")), s.optJSONObject("value"), what)
            "letterAtOffset" -> assertEquals(what, if (value() == JSONObject.NULL) null else int("value"), p.letterAtOffset(int("word"), int("at")))
            "findLetters" -> assertEquals(what, s.getJSONArray("value").pairs(), p.findLetters(query(s.getJSONObject("query"))))
            "hitTestLetter" -> sameHit(p.hitTestLetter(s.getDouble("x").toFloat(), s.getDouble("y").toFloat(), options(s.optJSONObject("options"))),
                                       s.optJSONObject("value"), what)
            "hitTestLetterView" -> sameHit(p.hitTestLetterView(s.getDouble("x").toFloat(), s.getDouble("y").toFloat(), options(s.optJSONObject("options"))),
                                           s.optJSONObject("value"), what)
            else -> error("$what: no such call in letters.json (fixture $fixture)")
        }
    }

    /** Hex text as bytes, whitespace ignored. */
    private fun hexOf(text: String): ByteArray {
        val t = text.filterNot { it.isWhitespace() }
        return ByteArray(t.length / 2) { t.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
    }
}
