package net.quranpedia.qvp.demo

import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import net.quranpedia.qvp.*
import org.json.JSONObject

/** Mushaf Vector Reader — Android demo. Every visual state goes through the engine's style state. */
class MainActivity : AppCompatActivity() {
    private val pages = (1..21).toList() + (440..445).toList() + listOf(582, 604)
    private var pageNo = 1
    private var page: QvpPage? = null
    private var wordsJson = JSONObject()
    private var loadMs = 0.0; private var pageBytes = 0

    private lateinit var view: QvpPageView
    private lateinit var pageField: EditText
    private lateinit var selWord: TextView
    private lateinit var selInfo: TextView
    private lateinit var chips: LinearLayout
    private lateinit var hud: TextView
    private lateinit var panel: LinearLayout
    private lateinit var root: LinearLayout

    // style state mirrored from the web demo
    private var tajweed = false; private var hideMarks = false; private var markers = false
    private var selectedWord = -1; private var selectedAyah: Pair<Int, Int>? = null
    private val pathColors = HashMap<Int, Int>()
    private var playing = false; private var playIdx = 0
    private var theme = "light"
    private val handler = Handler(Looper.getMainLooper())
    private val palette = mapOf(QvpFamily.DIACRITIC to 0x1a73e8ff.toInt(), QvpFamily.TANWEEN to 0x8e24aaff.toInt(), QvpFamily.DOTS to 0xc62828ff.toInt(),
        QvpFamily.WAQF to 0x0a7d32ff.toInt(), QvpFamily.SIFR to 0xef6c00ff.toInt(), QvpFamily.SAJDAH to 0x6d4c41ff.toInt())
    private val themes = mapOf("light" to Triple(0x231f20ff.toInt(), 0xFFFFFDF7.toInt(), 0xFFF6F1E7.toInt()),
        "sepia" to Triple(0x3b2a14ff.toInt(), 0xFFF3E7CF.toInt(), 0xFFE9DCC3.toInt()),
        "dark" to Triple(0xe8e4dcff.toInt(), 0xFF1E2126.toInt(), 0xFF15171B.toInt()))

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val d = resources.displayMetrics.density
        fun dp(v: Int) = (v * d).toInt()
        root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }

        // top bar
        val bar = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL; setPadding(dp(8), dp(6), dp(8), dp(6)) }
        val title = TextView(this).apply { text = "Mushaf Vector Reader"; textSize = 15f; setTypeface(null, Typeface.BOLD) }
        pageField = EditText(this).apply { setText("1"); inputType = android.text.InputType.TYPE_CLASS_NUMBER; minWidth = dp(64); gravity = Gravity.CENTER; textSize = 14f
            setOnEditorActionListener { _, _, _ -> text.toString().toIntOrNull()?.let { loadPage(it) }; true } }
        val prev = Button(this).apply { text = "◀"; setOnClickListener { stepPage(-1) } }
        val next = Button(this).apply { text = "▶"; setOnClickListener { stepPage(+1) } }
        bar.addView(title, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f))
        bar.addView(prev); bar.addView(pageField); bar.addView(next)
        root.addView(bar)

        // page view
        view = QvpPageView(this).apply {
            padTop = dp(16).toFloat(); padBottom = dp(16).toFloat(); padSide = dp(10).toFloat()
            onWordTap = { w, h -> selectedWord = if (selectedWord == w.idx) -1 else w.idx; selectedAyah = null; pathColors.clear(); applyStyles(); showSelection(h) }
            onDecoTap = { dec, _ -> if (dec.ayah != 0) { selectedAyah = dec.sura to dec.ayah; selectedWord = -1; applyStyles(); showSelection(null) } }
            onEmptyTap = { selectedWord = -1; selectedAyah = null; pathColors.clear(); applyStyles(); showSelection(null) }
        }
        root.addView(view, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f))

        // bottom panel
        panel = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(12), dp(8), dp(12), dp(8)) }
        val scroll = ScrollView(this).apply { addView(panel) }
        root.addView(scroll, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(300)))

        selWord = TextView(this).apply { textSize = 30f; textDirection = View.TEXT_DIRECTION_RTL; gravity = Gravity.END; text = "—" }
        selInfo = TextView(this).apply { textSize = 12f }
        chips = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val chipsScroll = HorizontalScrollView(this).apply { addView(chips) }
        panel.addView(section("Selection")); panel.addView(selWord); panel.addView(selInfo); panel.addView(chipsScroll)

        panel.addView(section("Live styling (engine style state)"))
        val row1 = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        val bTaj = toggleButton("Mark colours") { tajweed = it; applyStyles() }
        val bHide = toggleButton("Hide marks") { hideMarks = it; applyStyles() }
        val bMk = toggleButton("Gold markers") { markers = it; applyStyles() }
        val bPlay = toggleButton("Play words") { if (it) startPlay() else stopPlay() }
        listOf(bTaj, bHide, bMk, bPlay).forEach { row1.addView(it) }
        panel.addView(HorizontalScrollView(this).apply { addView(row1) })
        val row2 = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        val spinner = Spinner(this).apply { adapter = ArrayAdapter(context, android.R.layout.simple_spinner_dropdown_item, listOf("Light", "Sepia", "Dark"))
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(p: AdapterView<*>?, v: View?, pos: Int, id: Long) { setTheme(listOf("light", "sepia", "dark")[pos]) }
                override fun onNothingSelected(p: AdapterView<*>?) {}
            } }
        val clear = Button(this).apply { text = "Clear styles"; setOnClickListener {
            tajweed = false; hideMarks = false; markers = false; selectedWord = -1; selectedAyah = null; pathColors.clear(); stopPlay()
            listOf(bTaj, bHide, bMk, bPlay).forEach { (it as ToggleButton).isChecked = false }; applyStyles(); showSelection(null) } }
        row2.addView(TextView(this).apply { text = "Theme " }); row2.addView(spinner); row2.addView(clear)
        panel.addView(row2)

        panel.addView(section("Layout (engine)"))
        panel.addView(slider("line spacing", 60, 220, 100) { v -> view.lineSpacing = v / 100f; view.fillHeight = false; view.relayout(); view.resetView(); hud() })
        panel.addView(slider("pad top", 0, 120, 16) { v -> view.padTop = v * d; view.relayout(); view.resetView(); hud() })
        panel.addView(slider("pad bottom", 0, 120, 16) { v -> view.padBottom = v * d; view.relayout(); view.resetView(); hud() })
        panel.addView(toggleButton("Fill screen height") { view.fillHeight = it; view.relayout(); view.resetView(); hud() })

        panel.addView(section("Engine"))
        hud = TextView(this).apply { typeface = Typeface.MONOSPACE; textSize = 11f }
        panel.addView(hud)

        setContentView(root)
        setTheme("light")
        loadPage(pages.first())
        view.post { hud() }
    }

    private fun section(t: String) = TextView(this).apply { text = t.uppercase(); textSize = 11f; setTypeface(null, Typeface.BOLD); alpha = 0.7f; setPadding(0, 18, 0, 4) }
    private fun toggleButton(t: String, on: (Boolean) -> Unit) = ToggleButton(this).apply { textOn = t; textOff = t; text = t; setOnCheckedChangeListener { _, c -> on(c) } }
    private fun slider(label: String, min: Int, max: Int, init: Int, on: (Int) -> Unit): View {
        val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        val lbl = TextView(this).apply { text = label; textSize = 12f; minWidth = 260 }
        val v = TextView(this).apply { text = "$init"; textSize = 12f; minWidth = 90; gravity = Gravity.END }
        val sb = SeekBar(this).apply { this.max = max - min; progress = init - min
            setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                override fun onProgressChanged(s: SeekBar?, p: Int, fromUser: Boolean) { v.text = "${p + min}"; on(p + min) }
                override fun onStartTrackingTouch(s: SeekBar?) {}; override fun onStopTrackingTouch(s: SeekBar?) {} }) }
        row.addView(lbl); row.addView(sb, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)); row.addView(v)
        return row
    }

    private fun stepPage(dir: Int) { val i = pages.indexOf(pageNo); val n = pages.getOrNull(i + dir) ?: return; loadPage(n) }

    private fun loadPage(n: Int) {
        val target = if (n in pages) n else pages.minByOrNull { kotlin.math.abs(it - n) }!!
        val name = "%03d".format(target)
        val bytes = assets.open("pages/$name.qvp").readBytes()
        val t0 = System.nanoTime()
        val p = QvpPage(bytes); p.buildPaths()
        loadMs = (System.nanoTime() - t0) / 1e6; pageBytes = bytes.size
        wordsJson = runCatching { JSONObject(assets.open("pages/$name.words.json").bufferedReader().readText()) }.getOrDefault(JSONObject())
        page?.close(); page = p; pageNo = target
        pageField.setText("$target")
        selectedWord = -1; selectedAyah = null; pathColors.clear(); playIdx = 0
        view.page = p
        applyStyles(); showSelection(null)
    }

    /** Rebuild the engine style state from the UI state (same precedence as the web demo). */
    private fun applyStyles() {
        val p = page ?: return
        p.styleClear()
        val (ink, _, _) = themes[theme]!!
        view.ink = ink; p.styleDefault(ink)
        if (tajweed) palette.forEach { (f, c) -> p.styleFamily(f, c) }
        if (hideMarks) p.styleKind(QvpKind.MARK, 0)
        if (markers) p.styleDeco(QvpDeco.AYAH_MARKER, 0xb8860bff.toInt())
        selectedAyah?.let { (s, a) -> p.styleAyah(s, a, 0x0a7d32ff.toInt()) }
        if (selectedWord >= 0) { val w = p.words[selectedWord]; p.styleWord(w.sura, w.ayah, w.word, 0x1a73e8ff.toInt()) }
        pathColors.forEach { (pi, c) -> p.stylePath(pi, c) }
        if (playing && playIdx < p.nWords) { val w = p.words[playIdx]; p.styleWord(w.sura, w.ayah, w.word, 0xd81b60ff.toInt()) }
        view.selectedWord = selectedWord; view.selectedAyah = selectedAyah
        view.invalidate()
        view.post { hud() }
    }

    private fun showSelection(hit: QvpHit?) {
        val p = page ?: return
        chips.removeAllViews()
        val w = if (selectedWord >= 0) p.words[selectedWord] else null
        if (w == null) {
            val a = selectedAyah
            if (a != null) {
                val ws = p.words.filter { it.sura == a.first && it.ayah == a.second }
                selWord.text = ws.joinToString(" ") { it.text }
                selInfo.text = "ayah ${a.first}:${a.second} · ${ws.size} words on this page"
            } else { selWord.text = "—"; selInfo.text = "Tap a word, or an ayah marker. Pinch to zoom, double-tap to reset." }
            return
        }
        selWord.text = w.text
        val wid = "${w.sura}:${w.ayah}:${w.word}"
        val t = wordsJson.optJSONObject(wid)
        selInfo.text = buildString {
            append("wid $wid · line ${w.line} · ${w.nPaths} paths\n")
            if (t != null) { append("imlaei ${t.optString("imlaei")}\nqpc ${t.optString("qpc")}\nrasm ${t.optString("rasm")} · search ${t.optString("search")}\n") }
            append("bbox %.1f, %.1f → %.1f, %.1f".format(w.x0, w.y0, w.x1, w.y1))
        }
        for (i in w.firstPath until w.firstPath + w.nPaths) {
            val kind = p.pathKind(i)
            val label = if (kind == QvpKind.MARK) QvpPage.markName(p.pathMark(i)) else QvpPage.kindName(kind)
            val chip = ToggleButton(this).apply {
                val s = "#${i - w.firstPath} $label"; textOn = s; textOff = s; text = s; textSize = 11f; isChecked = pathColors.containsKey(i)
                setOnCheckedChangeListener { _, c -> if (c) pathColors[i] = 0xef6c00ff.toInt() else pathColors.remove(i); applyStyles() }
            }
            chips.addView(chip)
        }
    }

    private fun setTheme(t: String) {
        theme = t
        val (ink, paper, bg) = themes[t]!!
        view.paperColor = paper; root.setBackgroundColor(bg)
        val fg = if (t == "dark") Color.WHITE else Color.BLACK
        listOf(selWord, selInfo, hud).forEach { it.setTextColor(fg) }
        applyStyles()
    }

    private fun startPlay() {
        playing = true; playIdx = if (selectedWord >= 0) selectedWord else 0
        val tick = object : Runnable { override fun run() {
            if (!playing) return
            val p = page ?: return
            playIdx++
            if (playIdx >= p.nWords) { val i = pages.indexOf(pageNo); if (i + 1 < pages.size) { loadPage(pages[i + 1]); playing = true } else { stopPlay(); return } }
            applyStyles(); handler.postDelayed(this, 320)
        } }
        applyStyles(); handler.postDelayed(tick, 320)
    }
    private fun stopPlay() { playing = false; handler.removeCallbacksAndMessages(null); applyStyles() }

    private fun hud() {
        val p = page ?: return
        val l = p.currentLayout
        hud.text = "engine v${QvpPage.engineVersion()} (JNI over qvp.h)\n" +
            "page %03d      %d KB, load+decode %.2f ms\n".format(pageNo, pageBytes / 1024, loadMs) +
            "content       ${p.nWords} words · ${p.nPaths} paths · ${p.nAyahs} ayah parts · ${p.nLines} lines\n" +
            "base layer    ${view.lastBasePaths} paths in %.2f ms (cached)\n".format(view.lastBaseMs) +
            "overlay       ${view.lastOverlayPaths} styled paths in %.2f ms\n".format(view.lastOverlayMs) +
            "hit-test      %.1f µs (JNI)\n".format(view.lastHitUs) +
            "layout        ${if (view.fillHeight) "fill height" else "spacing ×%.2f".format(view.lineSpacing)} · pad %.0f/%.0f px · pitch %.1f u".format(view.padTop, view.padBottom, l?.pitch ?: 0f)
    }

    override fun onDestroy() { stopPlay(); page?.close(); super.onDestroy() }
}
