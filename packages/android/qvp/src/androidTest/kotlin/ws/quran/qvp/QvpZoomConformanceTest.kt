// Replays conformance/scenarios/zoom.json against the engine through the Kotlin wrapper: the
// reader's pan and zoom, from the fit through a pinch under each reflowing control. The page and
// the scenario file reach the test assets through scripts/sync-android-test-data.sh.
package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpZoomConformanceTest {
    private val assets = InstrumentationRegistry.getInstrumentation().context.assets

    /** Every wrapper replays conformance/scenarios/zoom.json and must match the engine's numbers. */
    @Test
    fun zoomScenariosMatchTheEngine() {
        val json = JSONObject(assets.open("zoom.json").use { it.readBytes() }.toString(Charsets.UTF_8))
        val tolerance = json.getDouble("tolerance")
        fun close(got: Float, want: Double, what: String) =
            assertTrue("$what: got $got, engine says $want", Math.abs(got.toDouble() - want) <= tolerance * Math.max(1.0, Math.abs(want)))
        fun JSONObject.f(k: String) = getDouble(k).toFloat()
        fun view(o: JSONObject) = QvpView(o.f("scale"), o.f("offsetX"), o.f("offsetY"))
        fun same(v: QvpView, o: JSONObject, what: String) {
            close(v.scale, o.getDouble("scale"), "$what scale")
            close(v.offsetX, o.getDouble("offsetX"), "$what offsetX")
            close(v.offsetY, o.getDouble("offsetY"), "$what offsetY")
        }
        fun mode(s: String) = when (s) { "continuous" -> QvpZoomMode.CONTINUOUS; "magnify" -> QvpZoomMode.MAGNIFY; else -> QvpZoomMode.STEPPED }
        fun zoom(o: JSONObject) = QvpZoom(mode(o.getString("mode")), o.getInt("step"), o.f("zoom"))
        fun fill(s: String) = when (s) { "ragged" -> QvpFill.RAGGED; "justified" -> QvpFill.JUSTIFIED; else -> QvpFill.CENTRED }
        fun JSONArray.objects() = List(length()) { getJSONObject(it) }

        val s = json.getJSONObject("spec"); val r = s.getJSONObject("reflow")
        val spec = QvpLayoutSpec(s.f("viewportW"), s.f("viewportH"), s.f("padTop"), s.f("padBottom"), s.f("padLeft"), s.f("padRight"),
                                 s.f("lineSpacing"), s.getBoolean("fillHeight"), s.getInt("gridLines"), s.f("cropLeft"), s.f("cropRight"), s.f("maxAspectSlack"),
                                 reflow = QvpReflowSpec(r.f("zoom"), fill = fill(r.getString("fill")), wordGap = r.f("wordGap"), maxStretch = r.f("maxStretch")),
                                 surahFrames = s.getBoolean("surahFrames"))
        val page = QvpPage(assets.open("042.qvp").use { it.readBytes() })
        try {
            val l = page.layout(spec)
            val fit = json.getJSONObject("fit")
            close(l.fitScale, fit.getDouble("fitScale"), "fit fitScale"); close(l.fitX, fit.getDouble("fitX"), "fit fitX"); close(l.fitY, fit.getDouble("fitY"), "fit fitY")
            close(l.contentW, fit.getDouble("contentW"), "fit contentW"); close(l.contentH, fit.getDouble("contentH"), "fit contentH")

            for (c in json.getJSONArray("zoomAbout").objects()) {
                val got = QvpEngine.viewZoomAbout(view(c.getJSONObject("view")), c.f("focalX"), c.f("focalY"), c.f("factor"), c.f("min"), c.f("max"))
                same(got, c.getJSONObject("result"), "zoomAbout x${c.f("factor")} max ${c.f("max")}")
            }
            val pan = json.getJSONObject("pan")
            same(QvpEngine.viewPan(view(pan.getJSONObject("view")), pan.f("dx"), pan.f("dy")), pan.getJSONObject("result"), "pan")
            for (c in json.getJSONArray("clamp").objects()) {
                val v = view(c.getJSONObject("view"))
                same(QvpEngine.viewClamp(v, l.contentW, l.contentH, spec.viewportW, spec.viewportH), c.getJSONObject("result"), "clamp of ${v.offsetX}, ${v.offsetY}")
            }
            val toLayout = json.getJSONObject("viewToLayout")
            val held = view(toLayout.getJSONObject("view"))
            val points = toLayout.getJSONArray("points")
            for (i in 0 until points.length()) {
                val p = points.getJSONArray(i)
                val (x, y) = page.viewToLayout(held, p.getDouble(0).toFloat(), p.getDouble(1).toFloat())
                close(x, p.getDouble(2), "viewToLayout(${p.getDouble(0)}, ${p.getDouble(1)}) x")
                close(y, p.getDouble(3), "viewToLayout(${p.getDouble(0)}, ${p.getDouble(1)}) y")
            }
            for (c in json.getJSONArray("isZoomed").objects()) {
                val v = view(c.getJSONObject("view"))
                assertEquals("isZoomed at scale ${v.scale}", c.getBoolean("result"), QvpEngine.zoomIsZoomed(zoom(c.getJSONObject("zoom")), v, c.f("fitScale")))
            }
            val fitted = QvpView(l.fitScale, l.fitX, l.fitY)
            for (c in json.getJSONArray("pinches").objects()) {
                val tag = "${c.getString("mode")} pinch"
                // each pinch starts from the printed layout, as the generator did
                page.layout(spec)
                val got = page.zoomPinch(spec, zoom(c.getJSONObject("start")), fitted, c.f("factor"), c.f("focalX"), c.f("focalY"))
                val want = zoom(c.getJSONObject("zoom"))
                assertEquals(tag, want.mode, got.zoom.mode); assertEquals(tag, want.step, got.zoom.step)
                close(got.zoom.zoom, want.zoom.toDouble(), "$tag zoom")
                same(got.view, c.getJSONObject("view"), tag)
                assertEquals(tag, c.getBoolean("relaid"), got.relaid)
                assertEquals("$tag zoomed after", c.getBoolean("zoomedAfter"), QvpEngine.zoomIsZoomed(got.zoom, got.view, l.fitScale))
                // the spec the control asks for comes back whole: the zoom in force and the host's knobs
                val w = c.getJSONObject("spec")
                val asks = page.zoomSpec(spec, got.zoom)
                val reflow = asks.reflow
                assertNotNull("$tag reflows", reflow); reflow!!
                close(asks.viewportW, w.getDouble("viewportW"), "$tag spec viewportW")
                close(asks.padTop, w.getDouble("padTop"), "$tag spec padTop")
                assertEquals("$tag spec surahFrames", w.getBoolean("surahFrames"), asks.surahFrames)
                close(reflow.zoom, w.getDouble("reflowZoom"), "$tag spec reflow zoom")
                assertEquals("$tag spec fill", fill(w.getString("fill")), reflow.fill)
                close(reflow.wordGap, w.getDouble("wordGap"), "$tag spec wordGap")
                close(reflow.maxStretch, w.getDouble("maxStretch"), "$tag spec maxStretch")
            }
            // On the printed page the control reflows nothing, and the host's knobs come back as it set them.
            val printed = json.getJSONObject("printedSpec"); val pw = printed.getJSONObject("spec")
            val asks = page.zoomSpec(spec, zoom(printed.getJSONObject("zoom")))
            val kept = asks.reflow
            assertNotNull("printed spec keeps the reflow knobs", kept); kept!!
            close(asks.viewportW, pw.getDouble("viewportW"), "printed spec viewportW")
            close(asks.padTop, pw.getDouble("padTop"), "printed spec padTop")
            assertEquals("printed spec surahFrames", pw.getBoolean("surahFrames"), asks.surahFrames)
            close(kept.zoom, pw.getDouble("reflowZoom"), "printed spec reflow zoom")
            assertEquals("printed spec fill", fill(pw.getString("fill")), kept.fill)
            close(kept.wordGap, pw.getDouble("wordGap"), "printed spec wordGap")
            close(kept.maxStretch, pw.getDouble("maxStretch"), "printed spec maxStretch")
            // A pinch whose fingers drift, replayed as a page view runs it: the engine works from the
            // control and the focal point the gesture began on, with the factor of the whole gesture;
            // the magnifying glass zooms the view the pinch began on, and the reflowing controls take
            // the view the last frame returned. The host pans the answer by the drift and holds it.
            for (g in json.getJSONArray("drifts").objects()) {
                page.layout(spec)
                val start = zoom(g.getJSONObject("start")); val began = view(g.getJSONObject("view"))
                val fx = g.f("focalX"); val fy = g.f("focalY")
                var base = began
                g.getJSONArray("frames").objects().forEachIndexed { i, f ->
                    val tag = "${g.getString("mode")} drifting pinch, frame $i"
                    val c = page.zoomPinch(spec, start, if (start.mode == QvpZoomMode.MAGNIFY) began else base, f.f("factor"), fx, fy)
                    base = c.view
                    val drawn = page.dressClamp(QvpEngine.viewPan(c.view, f.f("focalX") - fx, f.f("focalY") - fy), spec.viewportW, spec.viewportH)
                    val want = zoom(f.getJSONObject("zoom"))
                    assertEquals(tag, want.mode, c.zoom.mode); assertEquals(tag, want.step, c.zoom.step)
                    close(c.zoom.zoom, want.zoom.toDouble(), "$tag zoom")
                    assertEquals("$tag relaid", f.getBoolean("relaid"), c.relaid)
                    same(drawn, f.getJSONObject("view"), tag)
                }
            }
        } finally {
            page.close()
        }
    }
}
