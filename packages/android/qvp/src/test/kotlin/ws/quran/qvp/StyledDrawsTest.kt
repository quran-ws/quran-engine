package ws.quran.qvp

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertSame
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
    fun the_draws_keep_the_draw_list_order_not_the_path_order() {
        // a cut path's pieces sit past the page's paths and are drawn in its place
        val styled = intArrayOf(1, 0x111111ff, 8, 0x222222ff, 9, 0x333333ff)
        assertArrayEquals(
            intArrayOf(9, 0, 0x333333ff, 1, 0, 0x111111ff, 8, 2, 0x222222ff),
            styledDraws(draws(9 to 0, 3 to 0, 1 to 0, 8 to 2), styled, 10),
        )
    }

    @Test
    fun kept_buffers_answer_each_frame_of_a_fade() {
        val kept = StyledDraws(draws(2 to 0, 5 to 1), 6)
        kept.update(intArrayOf(5, 0x10101080, 2, 0x20202080))
        assertArrayEquals(intArrayOf(2, 0, 0x20202080, 5, 1, 0x10101080), kept.triples.copyOf(kept.size))
        val buffer = kept.triples
        // the next frame: one path faded back to the ink, which leaves the styled paths
        kept.update(intArrayOf(5, 0x101010c0))
        assertArrayEquals(intArrayOf(5, 1, 0x101010c0), kept.triples.copyOf(kept.size))
        assertSame(buffer, kept.triples)
    }

    @Test
    fun a_hidden_colour_is_skipped() {
        assertArrayEquals(IntArray(0), styledDraws(draws(4 to 0), intArrayOf(4, 0x11223300), 5))
    }
}
