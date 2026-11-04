package ws.quran.qvp

import java.util.concurrent.atomic.AtomicLong

/**
 * The engine's handle for one page, atlas, ornament set or passage. [get] throws an
 * [IllegalStateException] that names the object once it is closed, so no call gives the engine a
 * null handle. [close] frees the handle once: a second call, from any thread, does nothing. The
 * handle is atomic, so two threads that close the object free it once. A call already inside
 * the engine still holds the handle, so close an object only when no other thread is calling it.
 */
internal class NativeHandle(private val name: String, handle: Long, private val free: (Long) -> Unit) {
    private val value = AtomicLong(handle)

    /** False once [close] has been called, or when the engine refused to load the object. */
    val isOpen: Boolean get() = value.get() != 0L

    fun get(): Long = value.get().takeIf { it != 0L } ?: throw IllegalStateException("$name is closed")

    fun close() {
        val handle = value.getAndSet(0L)
        if (handle != 0L) free(handle)
    }
}
