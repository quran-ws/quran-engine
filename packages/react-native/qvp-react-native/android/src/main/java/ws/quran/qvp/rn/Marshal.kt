package ws.quran.qvp.rn

import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.ReadableArray
import com.facebook.react.bridge.ReadableMap
import com.facebook.react.bridge.WritableArray
import com.facebook.react.bridge.WritableMap
import ws.quran.qvp.*
import ws.quran.qvp.Target

/**
 * Marshalling only: JS shapes (docs/API.md) ↔ Kotlin library types. No hit-testing, layout or
 * styling decisions live here — every call goes to [QvpPage] / [QvpAtlas], which go to the engine.
 *
 * Targets from JS: "page" | "2:255" | "2:255:3" | "2:255-257" | "line:7" | "surah:2" | word index |
 * [word indices] | {target, a, b, c, words} (target = QVP_TARGET_* or its name).
 * Selectors from JS: {selector, a, b, c} (selector = QVP_SELECTOR_* or its name). Colours: '#rgb' | '#rrggbb' | '#rrggbbaa'.
 */
object Marshal {
    // ── colours ──
    fun color(v: Any?, default: Int): Int = when (v) {
        null -> default
        is String -> if (v.isBlank()) default else runCatching { QvpColor.parse(v) }.getOrDefault(default)
        is Number -> v.toLong().toInt()
        else -> default
    }
    fun colorOrNull(v: Any?): Int? = if (v == null) null else color(v, 0).let { if (it == 0) null else it }

    private fun num(v: Any?): Int? = (v as? Number)?.toInt() ?: (v as? String)?.toIntOrNull()
    private fun flt(v: Any?, d: Float): Float = (v as? Number)?.toFloat() ?: (v as? String)?.toFloatOrNull() ?: d
    private fun bool(v: Any?, d: Boolean): Boolean = (v as? Boolean) ?: (v as? Number)?.let { it.toInt() != 0 } ?: d
    private fun ints(v: Any?): IntArray = when (v) {
        is Iterable<*> -> v.mapNotNull { num(it) }.toIntArray()
        is IntArray -> v
        is Number -> intArrayOf(v.toInt())
        else -> IntArray(0)
    }

    private val TARGET_KINDS = listOf("page", "word", "words", "ayah", "ayahRange", "line", "surah", "range")
    private val SEL_KINDS = listOf("page", "path", "wordPath", "wordMark", "wordMarkNamed", "wordBody", "wordMarks", "word", "ayah", "line", "mark", "category", "family", "kind", "deco", "decoIdx")
    private fun kindOf(v: Any?, names: List<String>): Int = when (v) {
        is Number -> v.toInt()
        is String -> names.indexOf(v).let { if (it >= 0) it else v.toIntOrNull() ?: -1 }
        else -> -1
    }

    /** JS target (already converted with toHashMap/toArrayList or a plain value) → [Target]. Unknown → empty word list. */
    fun target(v: Any?, page: QvpPage?): Target = when (v) {
        null -> Target.page()
        is String -> runCatching { Target.parse(v, page) }.getOrElse { Target.words(IntArray(0)) }
        is Number -> Target.word(v.toInt())
        is Iterable<*> -> Target.words(ints(v))
        is IntArray -> Target.words(v)
        is Map<*, *> -> {
            val a = num(v["a"]) ?: 0; val b = num(v["b"]) ?: 0; val c = num(v["c"]) ?: 0
            when (kindOf(v["target"], TARGET_KINDS)) {
                0 -> Target.page(); 1 -> Target.word(a); 2 -> Target.words(ints(v["words"] ?: v["a"]))
                3 -> Target.ayah(a, b); 4 -> Target.ayahRange(a, b, c); 5 -> Target.line(a); 6 -> Target.surah(a); 7 -> Target.range(a, b)
                else -> Target.words(IntArray(0))
            }
        }
        else -> Target.words(IntArray(0))
    }

    /** JS selector {selector, a, b, c} → [Selector], or null when malformed. */
    fun selector(v: Any?): Selector? {
        val m = v as? Map<*, *> ?: return null
        val a = num(m["a"]) ?: 0; val b = num(m["b"]) ?: 0; val c = num(m["c"]) ?: 0
        return when (kindOf(m["selector"], SEL_KINDS)) {
            0 -> Selector.page(); 1 -> Selector.path(a); 2 -> Selector.wordPath(a, b); 3 -> Selector.wordMark(a, b)
            4 -> Selector.wordMarkNamed(a, (m["mark"] as? String) ?: QvpEngine.markName(b), c)
            5 -> Selector.wordBody(a); 6 -> Selector.wordMarks(a); 7 -> Selector.word(a); 8 -> Selector.ayah(a, b); 9 -> Selector.line(a)
            10 -> (m["mark"] as? String)?.let { Selector.mark(it) } ?: Selector.mark(a)
            11 -> Selector.category(a); 12 -> Selector.family(a); 13 -> Selector.kind(a); 14 -> Selector.deco(a); 15 -> Selector.decoIdx(a)
            else -> null
        }
    }

    fun form(v: Any?, d: Form = Form.RASM_UTHMANI): Form = when (v) {
        is Number -> Form.entries.getOrElse(v.toInt()) { d }
        is String -> Form.entries.firstOrNull { it.name.equals(v, true) } ?: d
        else -> d
    }
    fun searchMode(v: Any?): SearchMode = when (v) {
        is Number -> SearchMode.entries.getOrElse(v.toInt()) { SearchMode.INCLUDES }
        is String -> SearchMode.entries.firstOrNull { it.name.equals(v, true) } ?: SearchMode.INCLUDES
        else -> SearchMode.INCLUDES
    }
    fun maskMode(v: Any?): MaskMode = when (v) {
        is Number -> MaskMode.entries.getOrElse(v.toInt()) { MaskMode.HIDE }
        is String -> MaskMode.entries.firstOrNull { it.name.equals(v, true) } ?: MaskMode.HIDE
        else -> MaskMode.HIDE
    }
    fun division(v: Any?): Division = when (v) {
        is Number -> Division.entries.getOrElse(v.toInt()) { Division.JUZ }
        is String -> Division.entries.firstOrNull { it.name.equals(v, true) } ?: Division.JUZ
        else -> Division.JUZ
    }

    /** {mode, ink, band, height, padX, padY, radius, seam, ms, layer} → [QvpHighlightStyle] (missing keys keep the library defaults). */
    fun highlightStyle(v: Any?): QvpHighlightStyle {
        val m = v as? Map<*, *> ?: return QvpHighlightStyle()
        val d = QvpHighlightStyle()
        val mode = when (val x = m["mode"]) { is Number -> HighlightMode.entries.getOrElse(x.toInt()) { d.mode }; is String -> HighlightMode.entries.firstOrNull { it.name.equals(x, true) } ?: d.mode; else -> d.mode }
        val height = when (val x = m["height"]) { is Number -> BandHeight.entries.getOrElse(x.toInt()) { d.height }; is String -> BandHeight.entries.firstOrNull { it.name.equals(x, true) } ?: d.height; else -> d.height }
        return QvpHighlightStyle(mode = mode, ink = color(m["ink"], d.ink), band = color(m["band"], d.band), height = height, padX = flt(m["padX"], d.padX), padY = flt(m["padY"], d.padY),
            radius = flt(m["radius"], d.radius), seam = flt(m["seam"], d.seam), transitionMs = num(m["ms"] ?: m["transitionMs"]) ?: d.transitionMs, layer = num(m["layer"]) ?: d.layer)
    }
    /** {ink, diacritics, dots, waqf, sifr, ayahMark, numeral, headers, marks: {name: colour}, ms} → [QvpTheme]. */
    fun theme(v: Any?): QvpTheme {
        val m = v as? Map<*, *> ?: return QvpTheme()
        val marks = (m["marks"] as? Map<*, *>)?.entries?.mapNotNull { (k, c) -> colorOrNull(c)?.let { (k as String) to it } }?.toMap() ?: emptyMap()
        return QvpTheme(ink = colorOrNull(m["ink"]), diacritics = colorOrNull(m["diacritics"]), dots = colorOrNull(m["dots"]), waqf = colorOrNull(m["waqf"]), sifr = colorOrNull(m["sifr"]),
            ayahMark = colorOrNull(m["ayahMark"]), numeral = colorOrNull(m["numeral"]), headers = colorOrNull(m["headers"]), marks = marks, transitionMs = num(m["ms"] ?: m["transitionMs"]) ?: 0)
    }

    // ── Kotlin → JS ──
    fun word(p: QvpPage, w: QvpWord): Map<String, Any?> {
        val forms = LinkedHashMap<String, String>()
        for (f in Form.entries) if (f == Form.RASM_UTHMANI || p.hasForm(f)) forms[f.name.lowercase()] = p.wordForm(w.idx, f)
        val paths = (w.firstPath until w.firstPath + w.nPaths).map { i ->
            val kind = p.pathKind(i); val mark = p.pathMark(i); val cat = p.pathCategory(i); val fam = p.pathFamily(i)
            mapOf("idx" to i, "kind" to kind, "kindName" to QvpEngine.kindName(kind), "mark" to mark, "markName" to QvpEngine.markName(mark), "nthMark" to p.pathNthMark(i),
                "nthInWord" to p.pathNthInWord(i), "category" to cat, "categoryName" to QvpEngine.categoryName(cat), "family" to fam, "familyName" to QvpEngine.familyName(fam), "line" to p.pathLine(i))
        }
        return mapOf("idx" to w.idx, "surah" to w.surah, "ayah" to w.ayah, "word" to w.word, "line" to w.line, "lineIdx" to w.lineIdx, "ayahIdx" to w.ayahIdx,
            "x0" to w.x0, "y0" to w.y0, "x1" to w.x1, "y1" to w.y1, "text" to w.text, "firstPath" to w.firstPath, "nPaths" to w.nPaths,
            "wordKey" to w.wordKey, "ayahKey" to w.ayahKey, "forms" to forms, "label" to p.wordLabel(w.idx), "paths" to paths)
    }
    fun deco(d: QvpDecoration): Map<String, Any?> = mapOf("idx" to d.idx, "decoration" to d.decoration, "decorationName" to QvpEngine.decorationName(d.decoration).ifEmpty { "other" },
        "surah" to d.surah, "ayah" to d.ayah, "line" to d.line, "x0" to d.x0, "y0" to d.y0, "x1" to d.x1, "y1" to d.y1, "text" to d.text, "firstPath" to d.firstPath, "nPaths" to d.nPaths)
    fun hit(p: QvpPage, h: QvpHit): Map<String, Any?> {
        val w = if (h.word >= 0) p.words[h.word] else null
        return mapOf("word" to h.word, "path" to h.path, "deco" to h.deco, "line" to h.line, "distance" to h.distance, "isExact" to h.isExact, "wordKey" to w?.wordKey, "ayahKey" to w?.ayahKey)
    }
    fun ayah(a: QvpAyah): Map<String, Any?> = mapOf("idx" to a.idx, "surah" to a.surah, "ayah" to a.ayah, "fragment" to a.fragment, "fragments" to a.fragments, "flags" to a.flags, "rubuAlHizb" to a.rubuAlHizb, "firstWord" to a.firstWord, "nWords" to a.nWords,
        "ayahMarkDeco" to a.ayahMarkDeco, "bbox" to listOf(a.x0, a.y0, a.x1, a.y1))
    fun line(l: QvpLine): Map<String, Any?> = mapOf("idx" to l.idx, "lineNo" to l.lineNo, "isHeader" to l.isHeader, "firstWord" to l.firstWord, "nWords" to l.nWords, "bbox" to listOf(l.x0, l.y0, l.x1, l.y1),
        "bandY0" to l.bandY0, "bandY1" to l.bandY1, "centre" to l.centre)
    fun surah(s: QvpSurah): Map<String, Any?> = mapOf("number" to s.number, "ayahCount" to s.ayahCount, "hasBanner" to s.hasBanner, "hasBasmalah" to s.hasBasmalah, "place" to s.place, "bannerDeco" to s.bannerDeco,
        "arabic" to s.arabic, "latin" to s.latin, "english" to s.english)
    fun division(d: QvpDivision): Map<String, Any?> = mapOf("division" to d.division.name.lowercase(), "n" to d.n, "surah" to d.surah, "ayah" to d.ayah, "line" to d.line, "ayahIdx" to d.ayahIdx)
    fun ayahMark(m: QvpAyahMark): Map<String, Any?> = mapOf("deco" to m.deco, "surah" to m.surah, "ayah" to m.ayah, "line" to m.line, "cx" to m.cx, "cy" to m.cy, "r" to m.r, "ornamentPath" to m.ornamentPath, "numeralPath" to m.numeralPath)
    fun rosette(r: QvpRosette): Map<String, Any?> = mapOf("deco" to r.deco, "surah" to r.surah, "ayah" to r.ayah, "juz" to r.juz, "hizb" to r.hizb, "nisf" to r.nisf, "rubuAlHizb" to r.rubuAlHizb, "rubuAlHizbInHizb" to r.rubuAlHizbInHizb)
    fun sajdah(s: QvpSajdah): Map<String, Any?> = mapOf("deco" to s.deco, "surah" to s.surah, "ayah" to s.ayah, "signPath" to s.signPath)
    fun match(m: QvpMatch): Map<String, Any?> = mapOf("word" to m.word, "index" to m.index, "isLooseMatch" to m.isLooseMatch, "wordKey" to m.wordKey, "text" to m.text)
    fun cropBounds(c: QvpCropBounds): Map<String, Any?> = mapOf("x0" to c.x0, "y0" to c.y0, "x1" to c.x1, "y1" to c.y1, "nWords" to c.nWords, "ayahMarkDeco" to c.ayahMarkDeco)
    fun atlasSurah(s: QvpAtlasSurah): Map<String, Any?> = mapOf("n" to s.n, "number" to s.n, "page" to s.page, "ayahCount" to s.ayahCount, "place" to s.place, "arabic" to s.arabic, "latin" to s.latin, "english" to s.english)
    fun atlasRubuAlHizb(r: QvpAtlasRubuAlHizb): Map<String, Any?> = mapOf("rubuAlHizb" to r.rubuAlHizb, "surah" to r.surah, "ayah" to r.ayah, "page" to r.page, "ayahKey" to r.ayahKey)
    fun layout(l: QvpLayout): Map<String, Any?> = mapOf("scale" to l.scale, "ox" to l.ox, "oy" to l.oy, "contentW" to l.contentW, "contentH" to l.contentH, "pitch" to l.pitch, "fitScale" to l.fitScale, "fitX" to l.fitX, "fitY" to l.fitY, "lineDy" to l.lineDy.toList(),
        "slots" to l.slotTop.indices.map { listOf(l.slotTop[it], l.slotBottom[it]) })
    fun pageInfo(p: QvpPage): Map<String, Any?> = mapOf("page" to p.pageNo, "width" to p.width, "height" to p.height, "nLines" to p.nLines, "nAyahs" to p.nAyahs, "nWords" to p.nWords, "nPaths" to p.nPaths, "nDecos" to p.nDecos,
        "naturalPitch" to p.naturalPitch, "forms" to Form.entries.filter { it == Form.RASM_UTHMANI || p.hasForm(it) }.map { it.name.lowercase() })
    fun selection(p: QvpPage): Map<String, Any?> {
        val ws = p.selection()
        return mapOf("words" to ws.toList(), "text" to (if (ws.isEmpty()) "" else p.text(Target.words(ws))), "citation" to (if (ws.isEmpty()) "" else p.citation(ws)),
            "textWithCitation" to (if (ws.isEmpty()) "" else p.selectionText(Form.RASM_UTHMANI, true)))
    }

    /** Any Kotlin value (maps, lists, arrays, primitives) → a bridge value. */
    fun toJs(v: Any?): Any? = when (v) {
        null -> null
        is Boolean, is String, is Int, is Double -> v
        is Float -> v.toDouble(); is Long -> v.toDouble(); is Short -> v.toInt(); is Byte -> v.toInt()
        is Map<*, *> -> toMap(v)
        is Iterable<*> -> toArray(v)
        is IntArray -> toArray(v.toList()); is FloatArray -> toArray(v.toList()); is Array<*> -> toArray(v.toList())
        is Pair<*, *> -> toArray(listOf(v.first, v.second))
        is Enum<*> -> v.name.lowercase()
        else -> v.toString()
    }
    fun toMap(m: Map<*, *>): WritableMap {
        val out = Arguments.createMap()
        for ((k, v) in m) put(out, k.toString(), toJs(v))
        return out
    }
    fun toArray(l: Iterable<*>): WritableArray {
        val out = Arguments.createArray()
        for (v in l) when (val j = toJs(v)) {
            null -> out.pushNull(); is Boolean -> out.pushBoolean(j); is Int -> out.pushInt(j); is Double -> out.pushDouble(j); is String -> out.pushString(j)
            is WritableMap -> out.pushMap(j); is WritableArray -> out.pushArray(j); else -> out.pushString(j.toString())
        }
        return out
    }
    private fun put(out: WritableMap, k: String, j: Any?) { when (j) {
        null -> out.putNull(k); is Boolean -> out.putBoolean(k, j); is Int -> out.putInt(k, j); is Double -> out.putDouble(k, j); is String -> out.putString(k, j)
        is WritableMap -> out.putMap(k, j); is WritableArray -> out.putArray(k, j); else -> out.putString(k, j.toString())
    } }
    fun plain(m: ReadableMap?): HashMap<String, Any?>? = m?.toHashMap()
    fun plain(a: ReadableArray?): ArrayList<Any?>? = a?.toArrayList()
}
