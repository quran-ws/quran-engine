package ws.quran.qvp.demo

import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.Gravity
import android.view.MenuItem
import android.view.View
import android.view.ViewGroup
import android.view.animation.DecelerateInterpolator
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AppCompatDelegate
import androidx.coordinatorlayout.widget.CoordinatorLayout
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.core.view.updatePadding
import androidx.core.widget.NestedScrollView
import com.google.android.material.appbar.AppBarLayout
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDragHandleView
import com.google.android.material.button.MaterialButton
import com.google.android.material.button.MaterialButtonToggleGroup
import com.google.android.material.chip.Chip
import com.google.android.material.chip.ChipGroup
import com.google.android.material.color.MaterialColors
import com.google.android.material.divider.MaterialDivider
import com.google.android.material.snackbar.Snackbar
import com.google.android.material.slider.Slider
import com.google.android.material.textfield.MaterialAutoCompleteTextView
import com.google.android.material.textfield.TextInputEditText
import com.google.android.material.textfield.TextInputLayout
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
    private lateinit var chips: ChipGroup
    private lateinit var hud: TextView
    private lateinit var meta: TextView
    private lateinit var root: CoordinatorLayout
    private lateinit var revealSeek: Slider
    private lateinit var revealVal: TextView
    private lateinit var zoomLabel: TextView
    private lateinit var sheetBehavior: BottomSheetBehavior<LinearLayout>
    private lateinit var toolbar: MaterialToolbar
    private lateinit var appBar: AppBarLayout
    private var chromeOn = true
    private var appBarHeight = 0
    private var topInset = 0f; private var bottomInset = 0f; private var sideInset = 0f

    private var selWordIdx = -1; private var selAyah: Pair<Int, Int>? = null
    private var hlSel = 0; private var hlAyah = 0; private var hlSearch = 0; private var hlPlay = 0
    private val pathHandles = HashMap<Int, Int>()
    private var tajwid = 0; private var hideMarksH = 0; private var ayahMarksH = 0
    private var playing = false; private var playIdx = 0
    private var theme = "light"; private var hlMode = HighlightMode.BOTH; private var hlMs = 250; private var revealOn = false
    private val handler = Handler(Looper.getMainLooper())
    private val themes = mapOf("light" to Triple(0x231f20ff.toInt(), 0xFFFFFDF7.toInt(), 0xFFF6F1E7.toInt()), "sepia" to Triple(0x3b2a14ff.toInt(), 0xFFF3E7CF.toInt(), 0xFFE9DCC3.toInt()), "dark" to Triple(0xe8e4dcff.toInt(), 0xFF1E2126.toInt(), 0xFF15171B.toInt()))
    private val d get() = resources.displayMetrics.density
    private companion object { const val KEY_PAGE = "page"; const val KEY_SHEET = "sheet" }
    private fun dp(v: Int) = (v * d).toInt()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Edge to edge is the Material 3 default: the page runs behind the system bars and the
        // chrome insets itself out of their way.
        WindowCompat.setDecorFitsSystemWindows(window, false)
        atlas = runCatching { QvpAtlas(assets.open("pages/atlas.qva").readBytes()) }.getOrNull()

        val scaffold = CoordinatorLayout(this)

        // top app bar
        appBar = AppBarLayout(this)
        // The top app bar carries the title and the page turns, which is where Material 3 puts a
        // screen's own actions. The page number is the title, so it needs no label of its own.
        toolbar = MaterialToolbar(this).apply {
            title = "Mushaf"
            menu.add(0, 1, 0, "Previous page").apply {
                setIcon(R.drawable.ic_page_prev); setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
                setOnMenuItemClickListener { stepPage(-1); true }
            }
            menu.add(0, 2, 1, "Next page").apply {
                setIcon(R.drawable.ic_page_next); setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
                setOnMenuItemClickListener { stepPage(+1); true }
            }
            menu.add(0, 3, 2, "Controls").apply {
                setIcon(R.drawable.ic_settings); setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
                setOnMenuItemClickListener { sheetBehavior.state = BottomSheetBehavior.STATE_EXPANDED; true }
            }
            menu.add(0, 4, 3, "Full screen").apply {
                setIcon(R.drawable.ic_fullscreen); setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
                setOnMenuItemClickListener { setChrome(false); true }
            }
        }
        appBar.addView(toolbar, AppBarLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))

        appBar.fitsSystemWindows = false
        appBar.layoutParams = CoordinatorLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)

        // the page: everything under the app bar, with the sheet floating over it
        view = QvpPageView(this).apply {
            padTop = dp(12).toFloat(); padBottom = dp(12).toFloat(); padSide = dp(8).toFloat()
            onWordTap = { w, _ -> selectWord(w.index) }
            onDecorationTap = { dec, _ -> if (dec.ayah != 0) selectAyah(dec.surah, dec.ayah) }
            onEmptyTap = { if (!chromeOn) setChrome(true) else selectWord(-1) }
            onSelectionChanged = { showSelection() }
            // a sideways flick on a page with no sideways travel turns it: the engine says when
            onSwipe = { pages -> stepPage(pages) }
            contentDescription = "Musḥaf page"
        }
        scaffold.addView(view, CoordinatorLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        scaffold.addView(appBar)

        // The controls are a Material bottom sheet. It rests collapsed on its drag handle, so the
        // page has the whole screen until the reader pulls the sheet up.
        val sheet = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(MaterialColors.getColor(this, com.google.android.material.R.attr.colorSurfaceContainerLow))
        }
        val dragHandle = BottomSheetDragHandleView(this)
        sheet.addView(dragHandle)
        val panel = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(16), 0, dp(16), dp(24)) }
        panel.gap = dp(12)
        sheet.addView(NestedScrollView(this).apply { addView(panel) },
                      LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f))
        scaffold.addView(sheet, CoordinatorLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT).apply {
            behavior = BottomSheetBehavior<LinearLayout>()
        })
        sheetBehavior = BottomSheetBehavior.from(sheet).apply {
            isHideable = true; skipCollapsed = true
            // open to a little under two thirds of the screen, so the page stays in view behind it
            maxHeight = (resources.displayMetrics.heightPixels * 0.62f).toInt()
            // the sheet is opened from the app bar, so it always starts shut rather than where
            // it happened to be when the process went away
            saveFlags = BottomSheetBehavior.SAVE_NONE
            state = BottomSheetBehavior.STATE_HIDDEN
        }
        dragHandle.setOnClickListener { sheetBehavior.state = BottomSheetBehavior.STATE_HIDDEN }
        dragHandle.contentDescription = "Hide the controls"

        panel.addSpaced(section("Go to"))
        val gotoBox = textBox("Ayah, surah or juz", "2:255 \u00b7 Yasin \u00b7 juz 30", done = { goto(it) })
        gotoField = gotoBox.editText
        panel.addSpaced(gotoBox.layout)
        val pageBox = textBox("Page", null, numeric = true, done = { s -> s.toIntOrNull()?.let { loadPage(it) } })
        pageField = pageBox.editText
        panel.addSpaced(pageBox.layout)

        // the reader's zoom control
        panel.addSpaced(divider()); panel.addSpaced(section("Size"))
        panel.addSpaced(menuBox("Pinch", listOf("steps", "free zoom", "magnify")) { pos -> view.zoomMode = QvpZoomMode.entries[pos]; showZoom() })
        val steps = MaterialButtonToggleGroup(this).apply { isSingleSelection = true; isSelectionRequired = true }
        listOf("printed", "1", "2", "3").forEachIndexed { n, t ->
            steps.addView(segButton(t) { view.zoomToStep(n); showZoom() })
        }
        panel.addSpaced(HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(steps) })
        zoomLabel = body("")
        panel.addSpaced(zoomLabel)
        panel.addSpaced(hint("The page keeps the screen's width and the ink grows: fewer words fit a row, the rest move down, and the page scrolls. A sideways flick turns the page."))

        // search
        panel.addSpaced(divider()); panel.addSpaced(section("Search this page"))
        val searchBox = textBox("Search", "\u0627\u0644\u0644\u0647 \u00b7 \u0627\u0644\u0631\u062d\u0645\u0627\u0646", rtl = true, live = { runSearch() })
        searchField = searchBox.editText
        panel.addSpaced(searchBox.layout)
        results = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        panel.addSpaced(results)

        // selection
        panel.addSpaced(divider()); panel.addSpaced(section("Selection"))
        selWord = TextView(this).apply { textSize = 20f; textDirection = View.TEXT_DIRECTION_RTL; gravity = Gravity.END; text = "\u2014" }
        selInfo = body("")
        chips = ChipGroup(this).apply { isSingleLine = true }
        panel.addSpaced(selWord); panel.addSpaced(selInfo)
        panel.addSpaced(HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(chips) })
        panel.addSpaced(row(
            textButton("Copy + citation") { copySelection() },
            textButton("Crop \u2192 SVG") { cropSelection() }))
        panel.addSpaced(hint("Tap a word \u00b7 long-press and drag to select \u00b7 tap an ayah mark \u00b7 chips recolour one path (e.g. 2nd diacritic)"))

        // highlights
        panel.addSpaced(divider()); panel.addSpaced(section("Highlights (engine-animated)"))
        panel.addSpaced(menuBox("Highlight", listOf("band + ink", "band", "ink")) { pos ->
            hlMode = listOf(HighlightMode.BOTH, HighlightMode.BAND, HighlightMode.INK)[pos]
            if (selWordIdx >= 0) { val i = selWordIdx; selWordIdx = -1; selectWord(i) }
        })
        val playBtn = toggleButton("Follow words") { if (it) startPlay() else stopPlay() }
        panel.addSpaced(row(playBtn))
        panel.addSpaced(slider("fade ms", 0, 800, 250) { hlMs = it })

        // styling
        panel.addSpaced(divider()); panel.addSpaced(section("Styling (each toggle is one engine handle)"))
        val bTaj = toggleButton("Mark colours") { on -> page?.let { p -> if (tajwid != 0) { p.removeStyle(tajwid); tajwid = 0 }; if (on) tajwid = p.theme(QvpTheme(diacritics = 0x1a73e8ff.toInt(), dots = 0xc62828ff.toInt(), waqf = 0x0a7d32ff.toInt(), sifr = 0xef6c00ff.toInt(), transitionMs = 200)); view.invalidate() } }
        val bHide = toggleButton("Hide marks") { on -> page?.let { p -> if (hideMarksH != 0) { p.removeStyle(hideMarksH); hideMarksH = 0 }; if (on) hideMarksH = p.hide(Selector.kind(QvpKind.MARK)); view.invalidate() } }
        val bMk = toggleButton("Gold ayah marks") { on -> page?.let { p -> if (ayahMarksH != 0) { p.removeStyle(ayahMarksH); ayahMarksH = 0 }; if (on) ayahMarksH = p.style(Selector.decoration(QvpDecorationKind.AYAH_MARK), 0xb8860bff.toInt(), 300, QvpLayer.THEME + 1); view.invalidate() } }
        panel.addSpaced(HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(row(bTaj, bHide, bMk)) })
        panel.addSpaced(menuBox("Paper", listOf("Light", "Sepia", "Dark")) { pos -> setPaper(listOf("light", "sepia", "dark")[pos]) })
        panel.addSpaced(row(textButton("Clear all") {
            clearAll(); listOf(bTaj, bHide, bMk, playBtn).forEach { it.isChecked = false }
        }))

        // memorisation
        panel.addSpaced(divider()); panel.addSpaced(section("Memorisation"))
        var maskBlock = false
        panel.addSpaced(menuBox("Mask style", listOf("hide", "block")) { pos -> maskBlock = pos == 1 })
        panel.addSpaced(HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; addView(row(
            textButton("Mask ayah") { page?.let { p -> val t = selAyah?.let { Target.ayah(it.first, it.second) } ?: (if (selWordIdx >= 0) p.words[selWordIdx].let { Target.ayah(it.surah, it.ayah) } else Target.page()); p.maskOptions(blockColor = QvpColor.rgba(themes[theme]!!.third)); p.mask(t, if (maskBlock) MaskMode.BLOCK else MaskMode.HIDE); view.invalidate() } },
            textButton("Reveal") { page?.unmaskNext(1); view.invalidate() },
            textButton("Hide back") { page?.maskBack(1); view.invalidate() },
            textButton("Unmask") { page?.unmask(); view.invalidate() })) })
        val rrow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        rrow.addView(toggleButton("Greyed page") { on -> page?.let { p -> revealOn = on; if (on) { val n = p.revealStart(lit = 2, grey = if (theme == "dark") 0x4a4f57ff.toInt() else 0xc9c4b8ff.toInt(), ink = themes[theme]!!.first, transitionMs = 150); revealSeek.valueTo = (n - 1).toFloat().coerceAtLeast(1f); revealSeek.value = 0f; revealSeek.isEnabled = true; p.revealGoto(-1) } else { p.revealStop(); revealSeek.isEnabled = false }; view.invalidate() } })
        revealSeek = Slider(this).apply { isEnabled = false; valueFrom = 0f; valueTo = 1f; stepSize = 1f
            addOnChangeListener { _, v, _ -> if (revealOn) { page?.revealGoto(v.toLong()); revealVal.text = "${v.toInt() + 1}/${page?.revealStepCount()}"; view.invalidate() } } }
        revealVal = body("")
        rrow.addView(revealSeek, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)); rrow.addView(revealVal)
        panel.addSpaced(rrow)

        // layout
        panel.addSpaced(divider()); panel.addSpaced(section("Layout (engine)"))
        panel.addSpaced(slider("line spacing \u00d7100", 100, 220, 100) { v -> view.lineSpacing = v / 100f; view.fillHeight = false; view.relayout(); view.resetView(); hud() })
        panel.addSpaced(slider("pad top", 0, 120, 12) { v -> view.padTop = v * d; view.relayout(); view.resetView(); hud() })
        panel.addSpaced(slider("pad bottom", 0, 120, 12) { v -> view.padBottom = v * d; view.relayout(); view.resetView(); hud() })
        panel.addSpaced(row(
            toggleButton("Fill screen height") { view.fillHeight = it; view.relayout(); view.resetView(); hud() },
            textButton("Leading to fill") { page?.let { p -> view.fillHeight = false; view.lineSpacing = p.layoutLineSpacingToFill(view.layoutSpec()); view.relayout(); view.resetView(); hud() } }))
        panel.addSpaced(hint("Leading only grows \u2014 the printed lineSpacing is the floor, so the lines never close up \u2014 and the text width is always the screen's."))

        panel.addSpaced(divider()); panel.addSpaced(section("Page")); meta = body(""); panel.addSpaced(meta)
        panel.addSpaced(divider()); panel.addSpaced(section("Engine")); hud = body("").apply { typeface = Typeface.MONOSPACE; textSize = 10.5f }; panel.addSpaced(hud)

        root = scaffold
        setContentView(scaffold)
        ViewCompat.setOnApplyWindowInsetsListener(scaffold) { _, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
            appBar.updatePadding(top = bars.top, left = bars.left, right = bars.right)
            panel.updatePadding(bottom = dp(24) + bars.bottom)
            sideInset = maxOf(bars.left, bars.right).toFloat()
            topInset = bars.top.toFloat(); bottomInset = bars.bottom.toFloat()
            appBar.post { fitPage() }
            insets
        }
        ViewCompat.requestApplyInsets(scaffold)
        setPaper("light")
        loadPage(savedInstanceState?.getInt(KEY_PAGE) ?: pages.first())
        appBar.addOnLayoutChangeListener { _, _, t, _, b, _, _, _, _ ->
            if (b - t != appBarHeight) { appBarHeight = b - t; fitPage() }
        }
        view.post { hud() }
    }

    /** Keep the page clear of the chrome. The app bar and the collapsed sheet sit over the page,
     * so the engine lays the text out inside a margin that leaves room for them; with the chrome
     * gone the page takes the whole screen but for the display's own cutouts. */
    private fun fitPage() {
        val top = if (chromeOn) appBarHeight.toFloat() else topInset
        val bottom = bottomInset
        view.padTop = top + dp(12); view.padBottom = bottom + dp(12); view.padSide = sideInset + dp(8)
        view.relayout(); view.resetView(); hud()
    }

    /** Show or hide everything that is not the page. */
    private fun setChrome(on: Boolean) {
        chromeOn = on
        appBar.visibility = if (on) View.VISIBLE else View.GONE
        sheetBehavior.state = BottomSheetBehavior.STATE_HIDDEN
        val c = WindowCompat.getInsetsController(window, view)
        c.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        if (on) c.show(WindowInsetsCompat.Type.systemBars()) else c.hide(WindowInsetsCompat.Type.systemBars())
        fitPage()
    }

    /** Come back to the page and the sheet the reader left, the way any Android screen restores
     * itself after the system takes the process away. */
    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        outState.putInt(KEY_PAGE, pageNo)
    }

    /** What the zoom control says it is on, so the reader can see what a pinch did. */
    private fun showZoom() {
        val l = page?.currentLayout
        val steps = view.zoomSteps.joinToString(" · ") { "×%.2f".format(it) }
        zoomLabel.text = if (view.zoom.step > 0 || view.zoom.zoom > 1.0001f)
            "step ${view.zoom.step} · ×%.2f · %d rows".format(view.zoom.zoom, l?.rows ?: 0)
        else "printed · steps $steps"
        hud()
    }

    /** Every control in the sheet is spaced the same, rather than each one carrying its own
     * padding. `gap` is the space a child leaves under itself. */
    private var LinearLayout.gap: Int
        get() = getTag(R.id.qvp_gap) as? Int ?: 0
        set(v) = setTag(R.id.qvp_gap, v)
    private fun LinearLayout.addSpaced(v: View) = addView(v, LinearLayout.LayoutParams(
        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply { bottomMargin = gap })

    // ---- Material 3 building blocks -------------------------------------------------------
    // Every control below is the Material component for its job, built in code because this demo
    // has no layout files. The style attribute in each constructor is what makes it Material.

    private fun divider() = MaterialDivider(this)
    private fun section(t: String) = TextView(this, null, 0, com.google.android.material.R.style.TextAppearance_Material3_TitleSmall).apply {
        text = t; setPadding(0, dp(8), 0, 0)
        setTextColor(MaterialColors.getColor(this, com.google.android.material.R.attr.colorPrimary))
    }
    private fun body(t: String) = TextView(this, null, 0, com.google.android.material.R.style.TextAppearance_Material3_BodySmall).apply { text = t }
    private fun hint(t: String) = body(t).apply {
        setTextColor(MaterialColors.getColor(this, com.google.android.material.R.attr.colorOnSurfaceVariant))
        setPadding(0, dp(2), 0, dp(4))
    }
    private fun row(vararg v: View) = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL
        v.forEach { addView(it, LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT).apply { marginEnd = dp(8) }) }
    }

    private fun textButton(t: String, on: () -> Unit) =
        MaterialButton(this, null, com.google.android.material.R.attr.materialButtonOutlinedStyle).apply {
            text = t; setOnClickListener { on() }
        }
    private fun iconButton(t: String, label: String, on: () -> Unit) =
        MaterialButton(this, null, com.google.android.material.R.attr.materialIconButtonStyle).apply {
            text = t; contentDescription = label; setOnClickListener { on() }
        }

    /** Say what just happened. A snackbar is the Material 3 form of this, and unlike a toast it
     * belongs to the window, so it sits above the sheet instead of over the system bars. */
    private fun say(t: String, long: Boolean = false) =
        Snackbar.make(root, t, if (long) Snackbar.LENGTH_LONG else Snackbar.LENGTH_SHORT).show()
    /** A Material button that stays down: the M3 replacement for ToggleButton. */
    private fun toggleButton(t: String, on: (Boolean) -> Unit) =
        MaterialButton(this, null, com.google.android.material.R.attr.materialButtonOutlinedStyle).apply {
            text = t; isCheckable = true
            addOnCheckedChangeListener { _, c -> on(c) }
        }
    private fun segButton(t: String, on: () -> Unit) =
        MaterialButton(this, null, com.google.android.material.R.attr.materialButtonOutlinedStyle).apply {
            text = t; setOnClickListener { on() }
        }

    private class Box(val layout: TextInputLayout, val editText: TextInputEditText)
    /** An outlined text field: the Material form of every text input here. */
    private fun textBox(label: String, hintText: String?, numeric: Boolean = false, rtl: Boolean = false,
                        live: (() -> Unit)? = null, done: ((String) -> Unit)? = null): Box {
        val til = TextInputLayout(this, null, com.google.android.material.R.attr.textInputOutlinedStyle).apply {
            hint = label; isHintEnabled = true
        }
        if (hintText != null) til.placeholderText = hintText
        val et = TextInputEditText(til.context).apply {
            setSingleLine()
            if (numeric) inputType = InputType.TYPE_CLASS_NUMBER
            if (rtl) textDirection = View.TEXT_DIRECTION_RTL
            imeOptions = android.view.inputmethod.EditorInfo.IME_ACTION_GO
            if (done != null) setOnEditorActionListener { _, _, _ -> done(text.toString()); true }
            if (live != null) addTextChangedListener(object : android.text.TextWatcher {
                override fun afterTextChanged(s: android.text.Editable?) { live() }
                override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
                override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {} })
        }
        til.addView(et)
        return Box(til, et)
    }

    /** An exposed dropdown menu: the Material replacement for Spinner. */
    private fun menuBox(label: String, items: List<String>, on: (Int) -> Unit): View {
        val til = TextInputLayout(this, null, com.google.android.material.R.attr.textInputOutlinedExposedDropdownMenuStyle).apply { hint = label }
        val tv = MaterialAutoCompleteTextView(til.context).apply {
            setSimpleItems(items.toTypedArray())
            setText(items[0], false)
            setOnItemClickListener { _, _, pos, _ -> on(pos) }
        }
        til.addView(tv)
        return til
    }

    /** A Material slider with its label and its value beside it. */
    private fun slider(label: String, min: Int, max: Int, init: Int, on: (Int) -> Unit): View {
        val r = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        val lbl = body(label).apply { minWidth = dp(96) }
        val v = body("$init").apply { minWidth = dp(34); gravity = Gravity.END }
        val s = Slider(this).apply {
            valueFrom = min.toFloat(); valueTo = max.toFloat(); stepSize = 1f; value = init.toFloat()
            addOnChangeListener { _, value, fromUser -> v.text = "${value.toInt()}"; if (fromUser) on(value.toInt()) }
        }
        r.addView(lbl); r.addView(s, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)); r.addView(v)
        return r
    }

    /** The reader's paper and ink. The controls keep the Material colours: only the page follows
     * this choice, which is why nothing here repaints the panel. */
    private fun setPaper(t: String) {
        theme = t
        val (ink, paper, _) = themes[t]!!
        view.paperColor = paper
        // The screen is the page: the app bar and the ground behind it take the paper colour and
        // the title and the icons take the ink, so the reader sees one sheet and not a page in a
        // frame of another colour.
        // The ink in `themes` is the engine's RGBA, which Android would read as ARGB and draw
        // almost transparent. Android takes the same colour with the alpha moved to the front.
        val inkArgb = (ink ushr 8) or (ink shl 24)
        val night = if (t == "dark") AppCompatDelegate.MODE_NIGHT_YES else AppCompatDelegate.MODE_NIGHT_NO
        if (AppCompatDelegate.getDefaultNightMode() != night) { AppCompatDelegate.setDefaultNightMode(night); return }
        root.setBackgroundColor(paper)
        appBar.setBackgroundColor(paper)
        toolbar.setTitleTextColor(inkArgb)
        for (i in 0 until toolbar.menu.size()) toolbar.menu.getItem(i).icon?.setTint(inkArgb)
        WindowCompat.getInsetsController(window, view).let {
            it.isAppearanceLightStatusBars = t != "dark"
            it.isAppearanceLightNavigationBars = t != "dark"
        }
        page?.setDefaultColor(ink); view.invalidate()
    }

    private fun stepPage(dir: Int) {
        val i = pages.indexOf(pageNo)
        pages.getOrNull(i + dir)?.let { turnDir = dir; loadPage(it) }
    }
    private fun goto(s: String) {
        val a = atlas ?: return
        Regex("^(\\d+):(\\d+)").find(s)?.let { g -> val su = g.groupValues[1].toInt(); val ay = g.groupValues[2].toInt(); a.pageOf(su, ay)?.let { loadPage(it); selectAyah(su, ay) }; return }
        Regex("^juz\\s*(\\d+)", RegexOption.IGNORE_CASE).find(s)?.let { g -> a.juz(g.groupValues[1].toInt())?.let { loadPage(it.page) }; return }
        a.searchSurahs(s).firstOrNull()?.let { loadPage(it.page) }
    }

    /** Reading the page off disk and decoding it is work, and doing it on the main thread makes
     * a page turn wait in front of the reader. It happens on a thread of its own and the page is
     * handed over when it is ready. */
    private fun loadPage(n: Int) {
        val target = if (n in pages) n else pages.minByOrNull { kotlin.math.abs(it - n) }!!
        val name = "%03d".format(target)
        pending = target
        // the page the reader is turning to is often already sitting in the neighbour cache
        ready.remove(target)?.let { showPage(it, target); return }
        Thread {
            val bytes = assets.open("pages/$name.qvp").readBytes()
            val t0 = System.nanoTime()
            val p = runCatching { QvpPage(bytes) }.getOrNull() ?: return@Thread
            val ms = (System.nanoTime() - t0) / 1e6
            runCatching { p.attachWords(assets.open("pages/$name.words.json").readBytes()) }
            runOnUiThread {
                // another turn may have overtaken this one while it loaded, or the screen itself
                // may be gone: switching paper between day and night builds the activity again
                if (isFinishing || isDestroyed || pending != target) { p.close(); return@runOnUiThread }
                loadMs = ms; pageBytes = bytes.size
                showPage(p, target)
            }
        }.start()
    }

    private var pending = -1
    /** Which way the last turn went: +1 towards the next page, 0 for a jump. */
    private var turnDir = 0
    /** Neighbours decoded ahead of the reader. Nothing in here has ever been shown, so no
     * highlight or style of an earlier reading can ride along with it. */
    private val ready = HashMap<Int, QvpPage?>()   // a null entry is a decode in flight

    /** Decode the pages on either side while the reader is looking at this one, so the next
     * turn only has to draw. */
    private fun prepareNeighbours() {
        val i = pages.indexOf(pageNo)
        val want = listOf(i - 1, i + 1).mapNotNull { pages.getOrNull(it) }
        ready.keys.filter { it !in want }.forEach { ready.remove(it)?.close() }
        for (n in want) {
            if (n in ready) continue
            ready[n] = null                 // claim the slot so two turns cannot both fetch it
            val name = "%03d".format(n)
            Thread {
                val p = runCatching { QvpPage(assets.open("pages/$name.qvp").readBytes()) }.getOrNull()
                runCatching { p?.attachWords(assets.open("pages/$name.words.json").readBytes()) }
                runOnUiThread {
                    if (isFinishing || isDestroyed) { p?.close(); return@runOnUiThread }
                    if (p == null) { ready.remove(n); return@runOnUiThread }
                    if (!ready.containsKey(n) || ready[n] != null) p.close() else ready[n] = p
                }
            }.start()
        }
    }

    private fun showPage(p: QvpPage, target: Int) {
        stopPlay(); page?.close(); page = p; pageNo = target
        pageField.setText("$target")
        toolbar.title = "Page $target"
        selWordIdx = -1; selAyah = null; hlSel = 0; hlAyah = 0; hlSearch = 0; hlPlay = 0; pathHandles.clear(); tajwid = 0; hideMarksH = 0; ayahMarksH = 0; revealOn = false; revealSeek.isEnabled = false
        p.setDefaultColor(themes[theme]!!.first)
        view.page = p
        showSelection(); showMeta(); runSearch()
        slideIn(); prepareNeighbours()
    }

    /** A turn that swaps the ink in place reads as a glitch. The new page comes in from the
     * side it would come from in the book: the musḥaf is read right to left, so the next page
     * arrives from the left. */
    private fun slideIn() {
        val d = turnDir; turnDir = 0
        if (d == 0) return
        view.animate().cancel()
        view.translationX = -d * view.width * 0.16f
        view.alpha = 0.55f
        view.animate().translationX(0f).alpha(1f).setDuration(150)
            .setInterpolator(DecelerateInterpolator()).start()
    }
    private fun showMeta() {
        val p = page ?: return
        val su = p.surahs().joinToString(", ") { "${it.number}${if (it.latin.isNotEmpty()) " " + it.latin else ""}${if (it.hasBanner) " (banner)" else ""}" }
        val dv = p.divisions().joinToString(", ") { "${it.division.name.lowercase()} ${it.number} at ${it.surah}:${it.ayah}" }
        val j = atlas?.juzOf(p.words[0].surah, p.words[0].ayah)
        meta.text = "surahs: $su" + (if (dv.isNotEmpty()) "\nstarts here: $dv" else "") + (if (j != null) "\njuz $j · pages ${atlas!!.pagesOfJuz(j)}" else "") + "\nayahs: " + p.ayahKeys().joinToString(" ") { "${it.first}:${it.second}" }
    }

    private fun selectWord(i: Int) {
        val p = page ?: return
        selAyah = null; view.clearSelection(); if (hlAyah != 0) { p.removeHighlight(hlAyah); hlAyah = 0 }
        pathHandles.values.forEach { p.removeStyle(it) }; pathHandles.clear()
        if (i < 0 || i == selWordIdx) { selWordIdx = -1; if (hlSel != 0) { p.removeHighlight(hlSel); hlSel = 0 } }
        else {
            selWordIdx = i
            val st = QvpHighlightStyle(mode = hlMode, ink = 0x1a73e8ff.toInt(), band = QvpColor.withAlpha(0x1a73e8ff.toInt(), 0.18f), radius = 1.5f, transitionMs = hlMs, layer = QvpLayer.SELECTION)
            if (hlSel != 0) p.moveHighlight(hlSel, Target.word(i)) else hlSel = p.highlight(Target.word(i), st)
        }
        showSelection(); view.invalidate()
    }
    private fun selectAyah(s: Int, a: Int) {
        val p = page ?: return
        if (hlSel != 0) { p.removeHighlight(hlSel); hlSel = 0 }; selWordIdx = -1; view.clearSelection()
        selAyah = s to a
        val st = QvpHighlightStyle(mode = hlMode, ink = 0x0a7d32ff.toInt(), band = QvpColor.withAlpha(0x0a7d32ff.toInt(), 0.14f), radius = 1.5f, transitionMs = hlMs, layer = QvpLayer.SELECTION)
        if (hlAyah != 0) p.moveHighlight(hlAyah, Target.ayah(s, a)) else hlAyah = p.highlight(Target.ayah(s, a), st)
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
            if (p.hasForm(Form.RASM_IMLAI)) append("rasmImlai ${p.wordForm(w.index, Form.RASM_IMLAI)} · search ${p.wordForm(w.index, Form.SEARCH)}\n")
            append(p.wordLabel(w.index))
        }
        for (i in w.firstPath until w.firstPath + w.nPaths) {
            val kind = p.pathKind(i); val nth = p.pathNthMark(i)
            val label = if (kind == QvpKind.MARK) "${QvpEngine.markName(p.pathMark(i))} #$nth" else QvpEngine.kindName(kind)
            chips.addView(Chip(this).apply { text = label; isCheckable = true; isChecked = pathHandles.containsKey(i)
                setOnCheckedChangeListener { _, c -> if (c) pathHandles[i] = if (kind == QvpKind.MARK && nth >= 0) p.style(Selector.wordMark(w.index, nth), 0xef6c00ff.toInt(), 200, QvpLayer.TOP) else p.style(Selector.path(i), 0xef6c00ff.toInt(), 200, QvpLayer.TOP)
                    else pathHandles.remove(i)?.let { p.removeStyle(it) }; view.invalidate() } })
        }
    }
    private fun copySelection() {
        val p = page ?: return
        val text = when { p.selection().isNotEmpty() -> p.selectionText(Form.RASM_UTHMANI, true); selAyah != null -> "${p.text(Target.ayah(selAyah!!.first, selAyah!!.second))} (${selAyah!!.first}:${selAyah!!.second})"; selWordIdx >= 0 -> "${p.words[selWordIdx].text} (${p.citation(intArrayOf(selWordIdx))})"; else -> return }
        (getSystemService(CLIPBOARD_SERVICE) as android.content.ClipboardManager).setPrimaryClip(android.content.ClipData.newPlainText("quran", text))
        say(text)
    }
    private fun cropSelection() {
        val p = page ?: return
        val t = when { p.selection().isNotEmpty() -> Target.words(p.selection()); selAyah != null -> Target.ayah(selAyah!!.first, selAyah!!.second); selWordIdx >= 0 -> Target.word(selWordIdx); else -> return }
        val svg = p.cropSvg(t, 3f, true, QvpColor.rgba(themes[theme]!!.second)) ?: return
        val cb = p.cropBounds(t, 3f, true)
        say("SVG ${svg.length / 1024} KB · box ${"%.0f×%.0f".format(cb!!.x1 - cb.x0, cb.y1 - cb.y0)} units · marker ${if (cb.ayahMarkDecoration >= 0) "kept" else "no"}", long = true)
    }
    private fun runSearch() {
        val p = page ?: return
        results.removeAllViews()
        if (hlSearch != 0) { p.removeHighlight(hlSearch); hlSearch = 0 }
        val q = searchField.text.toString().trim()
        if (q.isEmpty()) { view.invalidate(); return }
        val m = p.search(q)
        if (m.isNotEmpty()) hlSearch = p.highlight(Target.words(m.map { it.word }), QvpHighlightStyle(mode = HighlightMode.BOTH, ink = 0xc62828ff.toInt(), band = QvpColor.withAlpha(0xc62828ff.toInt(), 0.12f), height = BandHeight.INK, padY = 1f, radius = 1f, transitionMs = hlMs))
        for (x in m.take(8)) results.addView(TextView(this).apply { text = "${x.text}  ${x.wordKey}${if (x.isLooseMatch) " ~" else ""}"; textDirection = View.TEXT_DIRECTION_RTL; textSize = 14f; setPadding(4, 2, 4, 2); setOnClickListener { selectWord(x.word) } })
        if (m.isEmpty()) results.addView(TextView(this).apply { text = "no match on this page"; textSize = 11f; alpha = 0.6f })
        view.invalidate()
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
            pg.moveHighlight(hlPlay, Target.word(playIdx)); view.invalidate(); handler.postDelayed(this, 320)
        } }
        handler.postDelayed(tick, 320); view.invalidate()
    }
    private fun stopPlay() { playing = false; handler.removeCallbacksAndMessages(null); page?.let { if (hlPlay != 0) { it.removeHighlight(hlPlay); hlPlay = 0 } }; view.invalidate() }

    private fun hud() {
        val p = page ?: return
        val l = p.currentLayout
        hud.text = "engine v${QvpEngine.version()} · JNI over qvp.h${if (atlas != null) " · atlas" else ""}\n" +
            "page %03d      %d KB, load %.2f ms\n".format(pageNo, pageBytes / 1024, loadMs) +
            "content       ${p.nWords} words · ${p.nPaths} paths · ${p.nLines} lines\n" +
            "base layer    ${view.lastBasePaths} paths in %.2f ms (cached)\n".format(view.lastBaseMs) +
            "overlay       ${view.lastOverlayPaths} styled + ${view.lastBands} bands in %.2f ms\n".format(view.lastOverlayMs) +
            "hit-test      %.1f µs · %d handles · %d highlights\n".format(view.lastHitUs, p.styleHandles().size, p.highlightHandles().size) +
            "layout        ${if (view.fillHeight) "fill height" else "spacing ×%.2f".format(view.lineSpacing)} · lineSpacing %.1f u".format(l?.lineSpacing ?: 0f)
        handler.postDelayed({ hud() }, 1000)
    }
    override fun onDestroy() {
        stopPlay(); page?.close()
        ready.values.forEach { it?.close() }; ready.clear()
        atlas?.close(); super.onDestroy()
    }
}
