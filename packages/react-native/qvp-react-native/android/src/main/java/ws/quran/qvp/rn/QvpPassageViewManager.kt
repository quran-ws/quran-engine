package ws.quran.qvp.rn

import com.facebook.react.bridge.ReadableArray
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.annotations.ReactProp
import ws.quran.qvp.QvpColor

/** `<QvpPassageView />`: props are marshalled onto [QvpRnPassageView]; [onAfterUpdateTransaction] commits them in one pass. */
class QvpPassageViewManager : SimpleViewManager<QvpRnPassageView>() {
    override fun getName() = NAME
    override fun createViewInstance(c: ThemedReactContext) = QvpRnPassageView(c)
    override fun onDropViewInstance(view: QvpRnPassageView) { view.destroy(); super.onDropViewInstance(view) }
    override fun onAfterUpdateTransaction(view: QvpRnPassageView) { super.onAfterUpdateTransaction(view); view.commit() }

    override fun getExportedCustomDirectEventTypeConstants(): MutableMap<String, Any> = QvpRnEvents.constants(EVENTS)

    @ReactProp(name = "pageUris") fun setPageUris(v: QvpRnPassageView, a: ReadableArray?) { v.pageUris = Marshal.plain(a)?.map { it.toString() } ?: emptyList() }
    @ReactProp(name = "surah", defaultInt = 0) fun setSurah(v: QvpRnPassageView, i: Int) { v.surahProp = i }
    @ReactProp(name = "from", defaultInt = 0) fun setFrom(v: QvpRnPassageView, i: Int) { v.fromProp = i }
    @ReactProp(name = "to", defaultInt = 0) fun setTo(v: QvpRnPassageView, i: Int) { v.toProp = i }
    @ReactProp(name = "basmalah", defaultBoolean = false) fun setBasmalah(v: QvpRnPassageView, b: Boolean) { v.basmalahProp = b }
    @ReactProp(name = "scale", defaultFloat = 1f) fun setScale(v: QvpRnPassageView, f: Float) { v.scaleProp = f }
    @ReactProp(name = "lineHeight", defaultFloat = 0f) fun setLineHeight(v: QvpRnPassageView, f: Float) { v.lineHeightProp = f }
    @ReactProp(name = "lineSpacing", defaultFloat = 1f) fun setLineSpacing(v: QvpRnPassageView, f: Float) { v.lineSpacingProp = f }
    @ReactProp(name = "padding", defaultFloat = 2f) fun setPadding(v: QvpRnPassageView, f: Float) { v.paddingProp = f }
    @ReactProp(name = "align") fun setAlign(v: QvpRnPassageView, s: String?) { v.alignProp = s }
    @ReactProp(name = "maxRows", defaultInt = 0) fun setMaxRows(v: QvpRnPassageView, i: Int) { v.maxRowsProp = i }
    @ReactProp(name = "keepAyahMark", defaultBoolean = false) fun setKeepAyahMark(v: QvpRnPassageView, b: Boolean) { v.keepAyahMarkProp = b }
    @ReactProp(name = "ellipsisWidth", defaultFloat = 0f) fun setEllipsisWidth(v: QvpRnPassageView, f: Float) { v.ellipsisWidthProp = f }
    @ReactProp(name = "maxStretch", defaultFloat = 0f) fun setMaxStretch(v: QvpRnPassageView, f: Float) { v.maxStretchProp = f }
    @ReactProp(name = "ink") fun setInk(v: QvpRnPassageView, s: String?) { v.inkProp = s }
    @ReactProp(name = "paperColor") fun setPaperColor(v: QvpRnPassageView, s: String?) { v.paperColor = Marshal.color(s, 0).let { if (it == 0) 0 else QvpColor.argb(it) } }

    companion object {
        const val NAME = "QvpPassageView"
        val EVENTS = listOf("onPassageLayout", "onError")
    }
}
