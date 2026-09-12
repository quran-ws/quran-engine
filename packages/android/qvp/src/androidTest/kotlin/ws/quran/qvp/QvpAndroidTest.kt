package ws.quran.qvp

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QvpAndroidTest {
    private fun pageBytes(): ByteArray = InstrumentationRegistry.getInstrumentation()
        .context.assets.open("001.qvp").use { it.readBytes() }

    @Test
    fun pageGeometryAndEngineOperationsConform() {
        assertTrue(QvpEngine.version() > 0)
        assertEquals("mark", QvpEngine.kindName(QvpKind.MARK))

        val page = QvpPage(pageBytes())
        try {
            assertEquals(1, page.pageNo)
            assertTrue(page.width > 0)
            assertTrue(page.height > 0)
            assertTrue(page.nWords > 0)
            assertTrue(page.nPaths > 0)
            assertEquals(page.nWords, page.words.size)
            assertEquals(page.nPaths * 8, page.table.size)
            assertEquals(page.nWords, page.resolve(Target.page()).size)
            assertEquals(page.nWords, page.hitBoxes().size)

            val paths = page.buildPaths()
            assertEquals(page.nPaths, paths.size)
            assertSame(paths, page.buildPaths())

            val layout = page.layout(QvpLayoutSpec(690f, 1100f, 24f, 24f, 16f, 16f))
            assertTrue(layout.scale > 0)
            assertEquals(page.nLines, layout.lineDy.size)
            assertFalse(page.isClosed)
        } finally {
            page.close()
        }

        assertTrue(page.isClosed)
        page.close()
        assertThrows(IllegalStateException::class.java) { page.resolve(Target.page()) }
    }
}
