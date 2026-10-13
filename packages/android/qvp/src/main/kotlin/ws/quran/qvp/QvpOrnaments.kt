// The ornaments of other mushafs and fonts (ornaments.qvo), over the dress section of the C ABI in
// qvp.h. The engine places every ornament; this file marshals the calls. Mirrors QvpOrnaments in
// web/qvp.js. See docs/API.md, "Dress".
package ws.quran.qvp

import android.graphics.Path

/**
 * A set of ornaments: the medallions, bands and borders of other mushafs and fonts. None of these
 * outlines is part of a page. Each ornament says what it may be published under in its
 * [QvpOrnament.licence]. A page that wears an ornament keeps what it needs, so the set can be
 * closed after a dress. Call [close] when done.
 */
class QvpOrnaments(bytes: ByteArray) : AutoCloseable {
    private var nativeHandle: Long = QvpNative.ornamentsLoad(bytes)
    private val h: Long get() = nativeHandle.takeIf { it != 0L } ?: error("QvpOrnaments is closed")
    /** The engine's handle, for a page to dress from. Throws once the set is closed. */
    internal val handle: Long get() = h
    init { require(nativeHandle != 0L) { "qvp_ornaments_load failed (not a QVO1 file?)" } }

    val isClosed: Boolean get() = nativeHandle == 0L
    /** How many ornaments the set holds. */
    val count: Int = QvpNative.ornamentCount(h)
    /** Every ornament, in the set's order: `list[i].index == i`. */
    val list: List<QvpOrnament> = try {
        List(count) { ornament(it) }
    } catch (e: Throwable) {
        // the set is not handed out, so nobody could close it
        close()
        throw e
    }

    private fun ornament(i: Int): QvpOrnament {
        val v = QvpNative.ornamentInfo(h, i) ?: error("qvp_ornament_info failed for $i")
        val s = QvpNative.ornamentStrings(h, i) ?: error("qvp_ornament_info failed for $i")
        val parts = List(v[2]) { k ->
            val p = QvpNative.ornamentPart(h, i, k) ?: error("qvp_ornament_part failed for $i, $k")
            QvpOrnamentPart(k, p[0], p[1] != 0, QvpNative.ornamentPartName(h, i, k) ?: "")
        }
        return QvpOrnament(i, kindOf(v[0], i), lineageOf(v[1], i), s[0], s[1], v[3] != 0,
                           QvpLicence(s[2], s[3], v[4] != 0, s[4]), parts)
    }

    private fun kindOf(k: Int, i: Int) = QvpOrnamentKind.entries.getOrNull(k)
        ?: throw IllegalStateException("ornament $i has an unknown kind $k")
    private fun lineageOf(l: Int, i: Int) = QvpLineage.entries.getOrNull(l)
        ?: throw IllegalStateException("ornament $i has an unknown lineage $l")

    /** The ornament of [kind] in [style], or null when that mushaf has none of that kind. */
    fun find(kind: QvpOrnamentKind, style: String): QvpOrnament? =
        QvpNative.ornamentFind(h, kind.id, style).let { if (it < 0) null else list[it] }

    /** The ornaments of one kind. */
    fun ofKind(kind: QvpOrnamentKind): List<QvpOrnament> = list.filter { it.kind == kind }

    /** Every style in the set, with the ornament it has of each kind or null, in the set's order. */
    fun styles(): List<QvpOrnamentStyle> {
        val out = LinkedHashMap<String, QvpOrnamentStyle>()
        for (o in list) {
            val s = out[o.style] ?: QvpOrnamentStyle(o.style, o.riwayah, o.lineage, null, null, null)
            out[o.style] = when (o.kind) {
                QvpOrnamentKind.AYAH_MARK -> s.copy(ayahMark = o)
                QvpOrnamentKind.SURAH_HEADER -> s.copy(surahHeader = o)
                QvpOrnamentKind.PAGE_FRAME -> s.copy(pageFrame = o)
            }
        }
        return out.values.toList()
    }

    @Synchronized
    override fun close() {
        val handle = nativeHandle
        if (handle != 0L) {
            nativeHandle = 0L
            QvpNative.ornamentsFree(handle)
        }
    }
}

/**
 * One draw of a dressed page's ornament layer, in page units. Paint it behind the page ink, under
 * the layout placement [placement] (the same index a page draw carries); null is the border, drawn
 * in layout space with no placement. A [stroke] draw is stroked [strokeWidth] wide, page units, with
 * round caps and joins; any other draw is filled, with the even-odd rule when [evenOdd].
 * [color] is 0xRRGGBBAA.
 */
class QvpOrnamentDraw(val path: Path, val color: Int, val kind: QvpOrnamentKind, val evenOdd: Boolean, val stroke: Boolean,
                      val strokeWidth: Float, val part: Int, val placement: Int?)

/** Build the draws of a dress geometry: the ops, the points and 9 ints per draw, as qvp_jni.c returns them. */
internal fun ornamentDraws(ops: ByteArray, pts: FloatArray, t: IntArray): List<QvpOrnamentDraw> = List(t.size / 9) { i ->
    val r = i * 9
    val p = Path()
    var o = t[r]; val oe = o + t[r + 1]; var k = t[r + 2]
    while (o < oe) {
        when (ops[o].toInt()) {
            0 -> { p.moveTo(pts[k], pts[k + 1]); k += 2 }
            1 -> { p.lineTo(pts[k], pts[k + 1]); k += 2 }
            2 -> { p.quadTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3]); k += 4 }
            3 -> { p.cubicTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3], pts[k + 4], pts[k + 5]); k += 6 }
            4 -> p.close()
        }
        o++
    }
    val flags = t[r + 5]
    val evenOdd = flags and DRAW_EVENODD != 0
    p.fillType = if (evenOdd) Path.FillType.EVEN_ODD else Path.FillType.WINDING
    QvpOrnamentDraw(p, t[r + 4], QvpOrnamentKind.entries[flags and 0xff], evenOdd, flags and DRAW_STROKE != 0,
                    Float.fromBits(t[r + 6]), t[r + 7], t[r + 8].let { if (it == -1) null else it })
}

/** QVP_DRAW_EVENODD and QVP_DRAW_STROKE: a dress draw's flags above its kind. */
private const val DRAW_EVENODD = 1 shl 8
private const val DRAW_STROKE = 2 shl 8
