package ws.quran.qvp.demo

import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import ws.quran.qvp.*
import ws.quran.qvp.Target

/** Mushaf Vector Reader — Android demo. Every visual state is engine state; the view only paints. */
class MainActivity : AppCompatActivity() {
    private val pages = (1..21).toList() + (440..445).toList() + listOf(582, 604)
    private var pageNo = 1
    private var page: QvpPage? = null
    private var atlas: QvpAtlas? = null
    private var loadMs = 0.0; private var pageBytes = 0

    private lateinit var view: QvpPageView
    private lateinit var pageField: EditText
    private lateinit var gotoField: EditText
    private lateinit var searchField: EditText
    private lateinit var results: LinearLayout
    private lateinit var selWord: TextView
    private lateinit var selInfo: TextView
    private lateinit var chips: LinearLayout
    private lateinit var hud: TextView
    private lateinit var meta: TextView
    private lateinit var root: LinearLayout
    private lateinit var revealSeek: SeekBar
    private lateinit var revealVal: TextView

    private var selWordIdx = -1; private var selAyah: Pair<Int, Int>? = null
    private var hlSel = 0; private var hlAyah = 0; private var hlSearch = 0; private var hlPlay = 0
    private val pathHandles = HashMap<Int, Int>()
    private var tajwid = 0; private var hideMarksH = 0; private var ayahMarksH = 0
    private var playing = false; private var playIdx = 0
    private var theme = "light"; private var hlMode = HighlightMode.BOTH; private var hlMs = 250; private var revealOn = false
    private val handler = Handler(Looper.getMainLooper())
    private val themes = mapOf("light" to Triple(0x231f20ff.toInt(), 0xFFFFFDF7.toInt(), 0xFFF6F1E7.toInt()), "sepia" to Triple(0x3b2a14ff.toInt(), 0xFFF3E7CF.toInt(), 0xFFE9DCC3.toInt()), "dark" to Triple(0xe8e4dcff.toInt(), 0xFF1E2126.toInt(), 0xFF15171B.toInt()))
    private val d get() = resources.displayMetrics.density
    private fun dp(v: Int) = (v * d).toInt()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        atlas = runCatching { QvpAtlas(assets.open("pages/atlas.qva").readBytes()) }.getOrNull()

        // top bar
        val bar = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL; setPadding(dp(8), dp(4), dp(8), dp(4)) }
        gotoField = EditText(this).apply { hint = "2:255 · Yasin · juz 30"; textSize = 13f; setSingleLine(); imeOptions = android.view.inputmethod.EditorInfo.IME_ACTION_GO
            setOnEditorActionListener { _, _, _ -> goto(text.toString()); true } }
        pageField = EditText(this).apply { setText("1"); inputType = InputType.TYPE_CLASS_NUMBER; minWidth = dp(56); gravity = Gravity.CENTER; textSize = 13f
            setOnEditorActionListener { _, _, _ -> text.toString().toIntOrNull()?.let { loadPage(it) }; true } }
        bar.addView(gotoField, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
        bar.addView(Button(this).apply { text = "◀"; setOnClickListener { stepPage(-1) } }); bar.addView(pageField); bar.addView(Button(this).apply { text = "▶"; setOnClickListener { stepPage(+1) } })
        root.addView(bar)

        view = QvpPageView(this).apply {
            padTop = dp(12).toFloat(); padBottom = dp(12).toFloat(); padSide = dp(8).toFloat()
            onWordTap = { w, _ -> selectWord(w.idx) }
            onDecoTap = { dec, _ -> if (dec.ayah != 0) selectAyah(dec.surah, dec.ayah) }
            onEmptyTap = { selectWord(-1) }
            onSelectionChanged = { showSelection() }
        }
        root.addView(view, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f))

        val panel = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(12), dp(6), dp(12), dp(8)) }
        root.addView(ScrollView(this).apply { addView(panel) }, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(330)))

        // search
        panel.addView(section("Search this page"))
        searchField = EditText(this).apply { hint = "الله · الرحمان"; textDirection = View.TEXT_DIRECTION_RTL; setSingleLine(); textSize = 14f
            addTextChangedListener(object : android.text.TextWatcher { override fun afterTextChanged(s: android.text.Editable?) { runSearch() }; override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}; override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {} }) }
        panel.addView(searchField)
        results = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        panel.addView(results)

        // selection
        panel.addView(section("Selection"))
        selWord = TextView(this).apply { textSize = 26f; textDirection = View.TEXT_DIRECTION_RTL; gravity = Gravity.END; text = "—" }
        selInfo = TextView(this).apply { textSize = 12f }
        chips = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        panel.addView(selWord); panel.addView(selInfo); panel.addView(HorizontalScrollView(this).apply { addView(chips) })
        val selRow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        selRow.addView(Button(this).apply { text = "Copy + citation"; setOnClickListener { copySelection() } })
        selRow.addView(Button(this).apply { text = "Crop → SVG"; setOnClickListener { cropSelection() } })
        panel.addView(selRow)
        panel.addView(TextView(this).apply { text = "Tap a word · long-press and drag to select · tap an ayah mark · chips recolour one path (e.g. 2nd diacritic)"; textSize = 11f; alpha = 0.7f })

        // highlights
        panel.addView(section("Highlights (engine-animated)"))
        val hlRow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        hlRow.addView(Spinner(this).apply { adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item, listOf("band + ink", "band", "ink"))
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(p: AdapterView<*>?, v: View?, pos: Int, id: Long) { hlMode = listOf(HighlightMode.BOTH, HighlightMode.BAND, HighlightMode.INK)[pos]; if (selWordIdx >= 0) { val i = selWordIdx; selWordIdx = -1; selectWord(i) } }
                override fun onNothingSelected(p: AdapterView<*>?) {} } })
        val playBtn = toggleButton("Follow words") { if (it) startPlay() else stopPlay() }
        hlRow.addView(playBtn)
        panel.addView(hlRow)
        panel.addView(slider("fade ms", 0, 800, 250) { hlMs = it })

        // styling
        panel.addView(section("Styling (each toggle is one engine handle)"))
        val row1 = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val bTaj = toggleButton("Mark colours") { on -> page?.let { p -> if (tajwid != 0) { p.unstyle(tajwid); tajwid = 0 }; if (on) tajwid = p.theme(QvpTheme(diacritics = 0x1a73e8ff.toInt(), dots = 0xc62828ff.toInt(), waqf = 0x0a7d32ff.toInt(), sifr = 0xef6c00ff.toInt(), transitionMs = 200)); view.invalidate() } }
        val bHide = toggleButton("Hide marks") { on -> page?.let { p -> if (hideMarksH != 0) { p.unstyle(hideMarksH); hideMarksH = 0 }; if (on) hideMarksH = p.hide(Selector.kind(QvpKind.MARK)); view.invalidate() } }
        val bMk = toggleButton("Gold ayah marks") { on -> page?.let { p -> if (ayahMarksH != 0) { p.unstyle(ayahMarksH); ayahMarksH = 0 }; if (on) ayahMarksH = p.style(Selector.deco(QvpDeco.AYAH_MARK), 0xb8860bff.toInt(), 300, QvpLayer.THEME + 1); view.invalidate() } }
        listOf(bTaj, bHide, bMk).forEach { row1.addView(it) }
        panel.addView(HorizontalScrollView(this).apply { addView(row1) })
        val row2 = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        row2.addView(TextView(this).apply { text = "Theme " })
        row2.addView(Spinner(this).apply { adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item, listOf("Light", "Sepia", "Dark"))
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(p: AdapterView<*>?, v: View?, pos: Int, id: Long) { setTheme(listOf("light", "sepia", "dark")[pos]) }
                override fun onNothingSelected(p: AdapterView<*>?) {} } })
        row2.addView(Button(this).apply { text = "Clear all"; setOnClickListener { clearAll(); listOf(bTaj, bHide, bMk, playBtn).forEach { (it as ToggleButton).isChecked = false } } })
        panel.addView(row2)

        // memorisation
        panel.addView(section("Memorisation"))
        val mrow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val maskMode = Spinner(this).apply { adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item, listOf("hide", "block")) }
        mrow.addView(Button(this).apply { text = "Mask ayah"; setOnClickListener { page?.let { p -> val t = selAyah?.let { Target.ayah(it.first, it.second) } ?: (if (selWordIdx >= 0) p.words[selWordIdx].let { Target.ayah(it.surah, it.ayah) } else Target.page()); p.maskOptions(blockColor = QvpColor.rgba(themes[theme]!!.third)); p.mask(t, if (maskMode.selectedItemPosition == 1) MaskMode.BLOCK else MaskMode.HIDE); view.invalidate() } } })
        mrow.addView(maskMode)
        mrow.addView(Button(this).apply { text = "Reveal"; setOnClickListener { page?.revealNext(1); view.invalidate() } })
        mrow.addView(Button(this).apply { text = "Hide back"; setOnClickListener { page?.hideBack(1); view.invalidate() } })
        mrow.addView(Button(this).apply { text = "Unmask"; setOnClickListener { page?.unmask(); view.invalidate() } })
        panel.addView(HorizontalScrollView(this).apply { addView(mrow) })
        val rrow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        rrow.addView(toggleButton("Greyed page") { on -> page?.let { p -> revealOn = on; if (on) { val steps = p.revealStart(lit = 2, grey = if (theme == "dark") 0x4a4f57ff.toInt() else 0xc9c4b8ff.toInt(), ink = themes[theme]!!.first, transitionMs = 150); revealSeek.max = steps - 1; revealSeek.progress = 0; revealSeek.isEnabled = true; p.revealGoto(-1) } else { p.revealStop(); revealSeek.isEnabled = false }; view.invalidate() } })
        revealSeek = SeekBar(this).apply { isEnabled = false; setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(s: SeekBar?, v: Int, fromUser: Boolean) { if (revealOn) { page?.revealGoto(v.toLong()); revealVal.text = "${v + 1}/${page?.revealSteps()}"; view.invalidate() } }
            override fun onStartTrackingTouch(s: SeekBar?) {}; override fun onStopTrackingTouch(s: SeekBar?) {} }) }
        revealVal = TextView(this).apply { textSize = 11f; minWidth = dp(44) }
        rrow.addView(revealSeek, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)); rrow.addView(revealVal)
        panel.addView(rrow)

        // layout
        panel.addView(section("Layout (engine)"))
        panel.addView(slider("line spacing ×100", 100, 220, 100) { v -> view.lineSpacing = v / 100f; view.lineGap = 0f; view.fillHeight = false; view.relayout(); view.resetView(); hud() })
        panel.addView(slider("pad top", 0, 120, 12) { v -> view.padTop = v * d; view.relayout(); view.resetView(); hud() })
        panel.addView(slider("pad bottom", 0, 120, 12) { v -> view.padBottom = v * d; view.relayout(); view.resetView(); hud() })
        val lrow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        lrow.addView(toggleButton("Fill screen height") { view.fillHeight = it; view.relayout(); view.resetView(); hud() })
        lrow.addView(Button(this).apply { text = "Leading to fill"; setOnClickListener { page?.let { p -> view.fillHeight = false; view.lineSpacing = 1f; view.lineGap = p.layoutGapToFill(QvpLayoutSpec(view.width.toFloat(), view.height.toFloat(), view.padTop, view.padBottom, view.padSide, view.padSide)); view.relayout(); view.resetView(); hud() } } })
        panel.addView(lrow)
        panel.addView(TextView(this).apply { textSize = 11f; text = "Leading only grows — the printed pitch is the floor, so the lines never close up — and the text width is always the screen's." })

        panel.addView(section("Page")); meta = TextView(this).apply { textSize = 11f }; panel.addView(meta)
        panel.addView(section("Engine")); hud = TextView(this).apply { typeface = Typeface.MONOSPACE; textSize = 10.5f }; panel.addView(hud)

        setContentView(root)
        setTheme("light")
        loadPage(pages.first())
        view.post { hud() }
    }

    private fun section(t: String) = TextView(this).apply { text = t.uppercase(); textSize = 10.5f; setTypeface(null, Typeface.BOLD); alpha = 0.7f; setPadding(0, 14, 0, 2) }
    private fun toggleButton(t: String, on: (Boolean) -> Unit) = ToggleButton(this).apply { textOn = t; textOff = t; text = t; textSize = 12f; setOnCheckedChangeListener { _, c -> on(c) } }
    private fun slider(label: String, min: Int, max: Int, init: Int, on: (Int) -> Unit): View {
        val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        val lbl = TextView(this).apply { text = label; textSize = 11f; minWidth = dp(96) }
        val v = TextView(this).apply { text = "$init"; textSize = 11f; minWidth = dp(30); gravity = Gravity.END }
        val sb = SeekBar(this).apply { this.max = max - min; progress = init - min
            setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(s: SeekBar?, p: Int, fromUser: Boolean) { v.text = "${p + min}"; if (fromUser) on(p + min) }
                override fun onStartTrackingTouch(s: SeekBar?) {}; override fun onStopTrackingTouch(s: SeekBar?) {} }) }
        row.addView(lbl); row.addView(sb, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)); row.addView(v)
        return row
    }

    private fun stepPage(dir: Int) { val i = pages.indexOf(pageNo); pages.getOrNull(i + dir)?.let { loadPage(it) } }
    private fun goto(s: String) {
        val a = atlas ?: return
        Regex("^(\\d+):(\\d+)").find(s)?.let { g -> val su = g.groupValues[1].toInt(); val ay = g.groupValues[2].toInt(); a.pageOf(su, ay)?.let { loadPage(it); selectAyah(su, ay) }; return }
        Regex("^juz\\s*(\\d+)", RegexOption.IGNORE_CASE).find(s)?.let { g -> a.juz(g.groupValues[1].toInt())?.let { loadPage(it.page) }; return }
        a.findSurah(s).firstOrNull()?.let { loadPage(it.page) }
    }

    private fun loadPage(n: Int) {
        val target = if (n in pages) n else pages.minByOrNull { kotlin.math.abs(it - n) }!!
        val name = "%03d".format(target)
        val bytes = assets.open("pages/$name.qvp").readBytes()
        val t0 = System.nanoTime()
        val p = QvpPage(bytes); p.buildPaths()
        loadMs = (System.nanoTime() - t0) / 1e6; pageBytes = bytes.size
        runCatching { p.attachWords(assets.open("pages/$name.words.json").readBytes()) }
        stopPlay(); page?.close(); page = p; pageNo = target
        pageField.setText("$target")
        selWordIdx = -1; selAyah = null; hlSel = 0; hlAyah = 0; hlSearch = 0; hlPlay = 0; pathHandles.clear(); tajwid = 0; hideMarksH = 0; ayahMarksH = 0; revealOn = false; revealSeek.isEnabled = false
        p.setDefaultInk(themes[theme]!!.first)
        view.page = p
        showSelection(); showMeta(); runSearch()
    }
    private fun showMeta() {
        val p = page ?: return
        val su = p.surahs().joinToString(", ") { "${it.number}${if (it.latin.isNotEmpty()) " " + it.latin else ""}${if (it.hasBanner) " (banner)" else ""}" }
        val dv = p.divisions().joinToString(", ") { "${it.kind.name.lowercase()} ${it.n} at ${it.surah}:${it.ayah}" }
        val j = atlas?.juzAt(p.words[0].surah, p.words[0].ayah)
        meta.text = "surahs: $su" + (if (dv.isNotEmpty()) "\nstarts here: $dv" else "") + (if (j != null) "\njuz $j · pages ${atlas!!.pagesOfJuz(j)}" else "") + "\nayahs: " + p.ayahKeys().joinToString(" ") { "${it.first}:${it.second}" }
    }

    private fun selectWord(i: Int) {
        val p = page ?: return
        selAyah = null; view.clearSelection(); if (hlAyah != 0) { p.unhighlight(hlAyah); hlAyah = 0 }
        pathHandles.values.forEach { p.unstyle(it) }; pathHandles.clear()
        if (i < 0 || i == selWordIdx) { selWordIdx = -1; if (hlSel != 0) { p.unhighlight(hlSel); hlSel = 0 } }
        else {
            selWordIdx = i
            val st = QvpHighlightStyle(mode = hlMode, ink = 0x1a73e8ff.toInt(), band = QvpColor.withAlpha(0x1a73e8ff.toInt(), 0.18f), radius = 1.5f, transitionMs = hlMs, layer = QvpLayer.SELECTION)
            if (hlSel != 0) p.rehighlight(hlSel, Target.word(i)) else hlSel = p.highlight(Target.word(i), st)
        }
        showSelection(); view.invalidate()
    }
    private fun selectAyah(s: Int, a: Int) {
        val p = page ?: return
        if (hlSel != 0) { p.unhighlight(hlSel); hlSel = 0 }; selWordIdx = -1; view.clearSelection()
        selAyah = s to a
        val st = QvpHighlightStyle(mode = hlMode, ink = 0x0a7d32ff.toInt(), band = QvpColor.withAlpha(0x0a7d32ff.toInt(), 0.14f), radius = 1.5f, transitionMs = hlMs, layer = QvpLayer.SELECTION)
        if (hlAyah != 0) p.rehighlight(hlAyah, Target.ayah(s, a)) else hlAyah = p.highlight(Target.ayah(s, a), st)
        showSelection(); view.invalidate()
    }
    private fun showSelection() {
        val p = page ?: return
        chips.removeAllViews()
        val sel = p.selection()
        if (sel.size > 1) { selWord.text = p.text(Target.words(sel)); selInfo.text = "selection · ${sel.size} words · ${p.citation(sel)}"; return }
        val w = if (selWordIdx >= 0) p.words[selWordIdx] else null
        if (w == null) {
            selAyah?.let { (s, a) -> val (count, complete) = p.ayahWordCount(s, a); selWord.text = p.text(Target.ayah(s, a)); selInfo.text = "ayah $s:$a · $count words${if (complete) "" else " (continues on another page)"}"; return }
            selWord.text = "—"; selInfo.text = ""; return
        }
        selWord.text = w.text
        selInfo.text = buildString {
            append("wordKey ${w.wordKey} · line ${w.line} · ${w.nPaths} paths\n")
            if (p.hasForm(Form.RASM_IMLAI)) append("rasmImlai ${p.wordForm(w.idx, Form.RASM_IMLAI)} · search ${p.wordForm(w.idx, Form.SEARCH)}\n")
            append(p.wordLabel(w.idx))
        }
        for (i in w.firstPath until w.firstPath + w.nPaths) {
            val kind = p.pathKind(i); val nth = p.pathNthMark(i)
            val label = if (kind == QvpKind.MARK) "${QvpEngine.markName(p.pathMark(i))} #$nth" else QvpEngine.kindName(kind)
            chips.addView(ToggleButton(this).apply { textOn = label; textOff = label; text = label; textSize = 10f; isChecked = pathHandles.containsKey(i)
                setOnCheckedChangeListener { _, c -> if (c) pathHandles[i] = if (kind == QvpKind.MARK && nth >= 0) p.style(Selector.wordMark(w.idx, nth), 0xef6c00ff.toInt(), 200, QvpLayer.TOP) else p.style(Selector.path(i), 0xef6c00ff.toInt(), 200, QvpLayer.TOP)
                    else pathHandles.remove(i)?.let { p.unstyle(it) }; view.invalidate() } })
        }
    }
    private fun copySelection() {
        val p = page ?: return
        val text = when { p.selection().isNotEmpty() -> p.selectionText(Form.RASM_UTHMANI, true); selAyah != null -> "${p.text(Target.ayah(selAyah!!.first, selAyah!!.second))} (${selAyah!!.first}:${selAyah!!.second})"; selWordIdx >= 0 -> "${p.words[selWordIdx].text} (${p.citation(intArrayOf(selWordIdx))})"; else -> return }
        (getSystemService(CLIPBOARD_SERVICE) as android.content.ClipboardManager).setPrimaryClip(android.content.ClipData.newPlainText("quran", text))
        Toast.makeText(this, text, Toast.LENGTH_SHORT).show()
    }
    private fun cropSelection() {
        val p = page ?: return
        val t = when { p.selection().isNotEmpty() -> Target.words(p.selection()); selAyah != null -> Target.ayah(selAyah!!.first, selAyah!!.second); selWordIdx >= 0 -> Target.word(selWordIdx); else -> return }
        val svg = p.cropSvg(t, 3f, true, QvpColor.rgba(themes[theme]!!.second)) ?: return
        val cb = p.cropBounds(t, 3f, true)
        Toast.makeText(this, "SVG ${svg.length / 1024} KB · box ${"%.0f×%.0f".format(cb!!.x1 - cb.x0, cb.y1 - cb.y0)} units · marker ${if (cb.ayahMarkDeco >= 0) "kept" else "no"}", Toast.LENGTH_LONG).show()
    }
    private fun runSearch() {
        val p = page ?: return
        results.removeAllViews()
        if (hlSearch != 0) { p.unhighlight(hlSearch); hlSearch = 0 }
        val q = searchField.text.toString().trim()
        if (q.isEmpty()) { view.invalidate(); return }
        val m = p.search(q)
        if (m.isNotEmpty()) hlSearch = p.highlight(Target.words(m.map { it.word }), QvpHighlightStyle(mode = HighlightMode.BOTH, ink = 0xc62828ff.toInt(), band = QvpColor.withAlpha(0xc62828ff.toInt(), 0.12f), height = BandHeight.INK, padY = 1f, radius = 1f, transitionMs = hlMs))
        for (x in m.take(8)) results.addView(TextView(this).apply { text = "${x.text}  ${x.wordKey}${if (x.loose) " ~" else ""}"; textDirection = View.TEXT_DIRECTION_RTL; textSize = 14f; setPadding(4, 2, 4, 2); setOnClickListener { selectWord(x.word) } })
        if (m.isEmpty()) results.addView(TextView(this).apply { text = "no match on this page"; textSize = 11f; alpha = 0.6f })
        view.invalidate()
    }
    private fun setTheme(t: String) {
        theme = t
        val (ink, paper, bg) = themes[t]!!
        view.paperColor = paper; root.setBackgroundColor(bg)
        val fg = if (t == "dark") Color.WHITE else Color.BLACK
        listOf(selWord, selInfo, hud, meta).forEach { it.setTextColor(fg) }
        page?.setDefaultInk(ink); view.invalidate()
    }
    private fun clearAll() {
        val p = page ?: return
        p.clearStyles(); p.clearHighlights(); p.unmask(); p.revealStop(); view.clearSelection()
        hlSel = 0; hlAyah = 0; hlSearch = 0; hlPlay = 0; tajwid = 0; hideMarksH = 0; ayahMarksH = 0; pathHandles.clear(); selWordIdx = -1; selAyah = null; revealOn = false; revealSeek.isEnabled = false
        searchField.setText(""); stopPlay(); showSelection(); view.invalidate()
    }
    private fun startPlay() {
        val p = page ?: return
        playing = true; playIdx = if (selWordIdx >= 0) selWordIdx else 0
        val st = QvpHighlightStyle(mode = hlMode, ink = 0xd81b60ff.toInt(), band = QvpColor.withAlpha(0xd81b60ff.toInt(), 0.14f), radius = 1.5f, transitionMs = hlMs)
        hlPlay = p.highlight(Target.word(playIdx), st)
        val tick = object : Runnable { override fun run() {
            if (!playing) return
            val pg = page ?: return
            playIdx++
            if (playIdx >= pg.nWords) { stepPage(+1); return }
            pg.rehighlight(hlPlay, Target.word(playIdx)); view.invalidate(); handler.postDelayed(this, 320)
        } }
        handler.postDelayed(tick, 320); view.invalidate()
    }
    private fun stopPlay() { playing = false; handler.removeCallbacksAndMessages(null); page?.let { if (hlPlay != 0) { it.unhighlight(hlPlay); hlPlay = 0 } }; view.invalidate() }

    private fun hud() {
        val p = page ?: return
        val l = p.currentLayout
        hud.text = "engine v${QvpEngine.version()} · JNI over qvp.h${if (atlas != null) " · atlas" else ""}\n" +
            "page %03d      %d KB, load %.2f ms\n".format(pageNo, pageBytes / 1024, loadMs) +
            "content       ${p.nWords} words · ${p.nPaths} paths · ${p.nLines} lines\n" +
            "base layer    ${view.lastBasePaths} paths in %.2f ms (cached)\n".format(view.lastBaseMs) +
            "overlay       ${view.lastOverlayPaths} styled + ${view.lastBands} bands in %.2f ms\n".format(view.lastOverlayMs) +
            "hit-test      %.1f µs · %d handles · %d highlights\n".format(view.lastHitUs, p.styleHandles().size, p.highlightHandles().size) +
            "layout        ${if (view.fillHeight) "fill height" else if (view.lineGap > 0) "gap +%.1f u".format(view.lineGap) else "spacing ×%.2f".format(view.lineSpacing)} · pitch %.1f u".format(l?.pitch ?: 0f)
        handler.postDelayed({ hud() }, 1000)
    }
    override fun onDestroy() { stopPlay(); page?.close(); atlas?.close(); super.onDestroy() }
}
