package ws.quran.qvp

import java.util.concurrent.CountDownLatch
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/** The handle every page, atlas, ornament set and passage holds. No engine is loaded here. */
class NativeHandleTest {
    @Test
    fun aClosedHandleThrowsAndNamesTheObject() {
        val handle = NativeHandle("QvpPage", 42L) {}
        assertTrue(handle.isOpen)
        assertEquals(42L, handle.get())
        handle.close()
        assertFalse(handle.isOpen)
        val e = assertThrows(IllegalStateException::class.java) { handle.get() }
        assertEquals("QvpPage is closed", e.message)
    }

    @Test
    fun aSecondCloseFreesNothing() {
        val freed = mutableListOf<Long>()
        val handle = NativeHandle("QvpAtlas", 7L) { freed += it }
        handle.close()
        handle.close()
        assertEquals(listOf(7L), freed)
    }

    @Test
    fun closesRacingOnManyThreadsFreeOnce() {
        val frees = AtomicInteger()
        repeat(200) {
            val handle = NativeHandle("QvpPassage", 1L) { frees.incrementAndGet() }
            val start = CountDownLatch(1)
            val threads = List(8) { Thread { start.await(); handle.close() }.apply { start() } }
            start.countDown()
            threads.forEach { it.join() }
        }
        assertEquals(200, frees.get())
    }
}
