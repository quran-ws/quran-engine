package net.quranpedia.qvp

/** Raw JNI surface over the QVP C ABI (see qvp.h). Use [QvpPage] instead. */
internal object QvpNative {
    init { System.loadLibrary("qvp_jni") }
    @JvmStatic external fun pageLoad(bytes: ByteArray): Long
    @JvmStatic external fun pageFree(h: Long)
    @JvmStatic external fun pageInfo(h: Long): FloatArray
    @JvmStatic external fun geomOps(h: Long): ByteArray
    @JvmStatic external fun geomPts(h: Long): FloatArray
    @JvmStatic external fun geomTable(h: Long): IntArray
    @JvmStatic external fun wordInfo(h: Long, idx: Int): FloatArray?
    @JvmStatic external fun wordText(h: Long, idx: Int): String?
    @JvmStatic external fun ayahInfo(h: Long, idx: Int): FloatArray?
    @JvmStatic external fun lineInfo(h: Long, idx: Int): FloatArray?
    @JvmStatic external fun decoInfo(h: Long, idx: Int): FloatArray?
    @JvmStatic external fun decoText(h: Long, idx: Int): String?
    @JvmStatic external fun hitTest(h: Long, x: Float, y: Float): IntArray?
    @JvmStatic external fun hitTestView(h: Long, x: Float, y: Float): IntArray?
    @JvmStatic external fun findWord(h: Long, sura: Int, ayah: Int, word: Int): Int
    @JvmStatic external fun layout(h: Long, vw: Float, vh: Float, padTop: Float, padBottom: Float, padLeft: Float, padRight: Float, spacing: Float, fill: Boolean, nominal: Int): FloatArray
    @JvmStatic external fun style(h: Long, sel: Int, a: Int, b: Int, c: Int, rgba: Int, on: Boolean): Int
    @JvmStatic external fun styleClear(h: Long)
    @JvmStatic external fun styleDefault(h: Long, rgba: Int)
    @JvmStatic external fun paint(h: Long): IntArray
    @JvmStatic external fun styled(h: Long): IntArray
    @JvmStatic external fun markName(m: Int): String
    @JvmStatic external fun familyName(f: Int): String
    @JvmStatic external fun kindName(k: Int): String
    @JvmStatic external fun version(): Int
}
