package ws.quran.qvp.rn

import android.view.View
import com.facebook.react.bridge.WritableMap
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.events.Event

/** The event plumbing both native views share: a named event with a map payload, sent to the view's tag. */
object QvpRnEvents {
    private class QvpEvent(surfaceId: Int, viewTag: Int, private val name: String, private val payload: WritableMap) : Event<QvpEvent>(surfaceId, viewTag) {
        override fun getEventName() = name
        override fun getEventData(): WritableMap = payload
    }

    fun emit(ctx: ThemedReactContext, view: View, name: String, payload: Map<String, Any?>) {
        if (view.id <= 0) return
        val dispatcher = UIManagerHelper.getEventDispatcherForReactTag(ctx, view.id) ?: return
        dispatcher.dispatchEvent(QvpEvent(UIManagerHelper.getSurfaceId(view), view.id, name, Marshal.toMap(payload)))
    }

    /** The direct event map a manager exports: each name registers under itself. */
    fun constants(names: List<String>): MutableMap<String, Any> {
        val m = HashMap<String, Any>()
        for (e in names) m[e] = mapOf("registrationName" to e)
        return m
    }
}
