package ws.quran.qvp.rn

import com.facebook.react.bridge.ReadableArray
import com.facebook.react.bridge.ReadableMap
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.annotations.ReactProp
import ws.quran.qvp.QvpColor

/** `<QvpPageView />`: props are marshalled onto [QvpRnPageView]; [onAfterUpdateTransaction] commits them in one pass. */
class QvpPageViewManager : SimpleViewManager<QvpRnPageView>() {
    override fun getName() = NAME
    override fun createViewInstance(c: ThemedReactContext) = QvpRnPageView(c)
    override fun onDropViewInstance(view: QvpRnPageView) { view.destroy(); super.onDropViewInstance(view) }
    override fun onAfterUpdateTransaction(view: QvpRnPageView) { super.onAfterUpdateTransaction(view); view.commit() }

    override fun getExportedCustomDirectEventTypeConstants(): MutableMap<String, Any> {
        val m = HashMap<String, Any>()
        for (e in EVENTS) m[e] = mapOf("registrationName" to e)
        return m
    }

    @ReactProp(name = "pageUri") fun setPageUri(v: QvpRnPageView, s: String?) { v.pageUri = s }
    @ReactProp(name = "pageBase64") fun setPageBase64(v: QvpRnPageView, s: String?) { v.pageBase64 = s }
    @ReactProp(name = "wordsUri") fun setWordsUri(v: QvpRnPageView, s: String?) { v.wordsUri = s }
    @ReactProp(name = "padTop", defaultFloat = 0f) fun setPadTop(v: QvpRnPageView, f: Float) = v.setPadTopDp(f)
    @ReactProp(name = "padBottom", defaultFloat = 0f) fun setPadBottom(v: QvpRnPageView, f: Float) = v.setPadBottomDp(f)
    @ReactProp(name = "padSide", defaultFloat = 0f) fun setPadSide(v: QvpRnPageView, f: Float) = v.setPadSideDp(f)
    @ReactProp(name = "lineSpacing", defaultFloat = 1f) fun setLineSpacing(v: QvpRnPageView, f: Float) = v.setLineSpacingProp(f)
    @ReactProp(name = "lineGap", defaultFloat = 0f) fun setLineGap(v: QvpRnPageView, f: Float) = v.setLineGapProp(f)
    @ReactProp(name = "fillHeight", defaultBoolean = false) fun setFillHeight(v: QvpRnPageView, b: Boolean) = v.setFillHeightProp(b)
    @ReactProp(name = "paperColor") fun setPaperColor(v: QvpRnPageView, s: String?) { v.paperColor = if (s.isNullOrBlank()) 0 else QvpColor.argb(QvpColor.parse(s)); v.invalidate() }
    @ReactProp(name = "defaultInk") fun setDefaultInk(v: QvpRnPageView, s: String?) { v.defaultInkProp = s }
    @ReactProp(name = "selectionBand") fun setSelectionBand(v: QvpRnPageView, s: String?) { if (!s.isNullOrBlank()) v.selectionBand = QvpColor.parse(s) }
    @ReactProp(name = "selectionEnabled", defaultBoolean = true) fun setSelectionEnabled(v: QvpRnPageView, b: Boolean) { v.selectionEnabled = b }
    @ReactProp(name = "zoomEnabled", defaultBoolean = true) fun setZoomEnabled(v: QvpRnPageView, b: Boolean) { v.zoomEnabled = b }
    @ReactProp(name = "hitMaxDistance", defaultFloat = QvpDefaults.TAP_DISTANCE) fun setHitMaxDistance(v: QvpRnPageView, f: Float) { v.hitOptions = v.hitOptions.copy(maxDistance = f) }
    @ReactProp(name = "theme") fun setTheme(v: QvpRnPageView, m: ReadableMap?) { v.themeProp = Marshal.plain(m) }
    @ReactProp(name = "styles") fun setStyles(v: QvpRnPageView, a: ReadableArray?) { v.stylesProp = Marshal.plain(a) }
    @ReactProp(name = "highlights") fun setHighlights(v: QvpRnPageView, a: ReadableArray?) { v.highlightsProp = Marshal.plain(a) }
    @ReactProp(name = "mask") fun setMask(v: QvpRnPageView, m: ReadableMap?) { v.maskProp = Marshal.plain(m) }
    @ReactProp(name = "reveal") fun setReveal(v: QvpRnPageView, m: ReadableMap?) { v.revealProp = Marshal.plain(m) }

    companion object {
        const val NAME = "QvpPageView"
        val EVENTS = listOf("onWordTap", "onDecoTap", "onEmptyTap", "onSelectionChanged", "onPageLoad", "onRevealChanged", "onError")
    }
}
