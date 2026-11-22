package ws.quran.qvp

import org.junit.Assert.assertArrayEquals
import org.junit.Test

/** The styled pass draws what the draw list holds, and nothing else. */
class StyledDrawsTest {
    private fun draws(vararg pairs: Pair<Int, Int>) = pairs.map { QvpDraw(it.first, it.second) }

    @Test
    fun a_styled_path_outside_the_list_is_not_drawn() {
        val styled = intArrayOf(0, 0xff0000ff.toInt(), 1, 0x00ff00ff, 2, 0x0000ffff)
        assertArrayEquals(
            intArrayOf(0, 0, 0xff0000ff.toInt(), 2, 1, 0x0000ffff),
            styledDraws(draws(0 to 0, 2 to 1), styled, 3),
        )
    }

    @Test
    fun a_repeated_path_is_drawn_at_each_placement() {
        assertArrayEquals(
            intArrayOf(4, 0, 0x112233ff, 4, 7, 0x112233ff),
            styledDraws(draws(4 to 0, 4 to 7), intArrayOf(4, 0x112233ff), 5),
        )
    }

    @Test
    fun a_hidden_colour_is_skipped() {
        assertArrayEquals(IntArray(0), styledDraws(draws(4 to 0), intArrayOf(4, 0x11223300), 5))
    }
}
