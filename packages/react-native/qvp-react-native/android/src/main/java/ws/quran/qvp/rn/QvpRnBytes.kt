package ws.quran.qvp.rn

import android.content.Context
import android.util.Base64
import java.io.File

/** The bytes a prop-given URI names: an asset, a file, an absolute path or inline base64. */
object QvpRnBytes {
    fun read(ctx: Context, uri: String): ByteArray {
        val u = uri.trim()
        return when {
            u.startsWith("asset://") -> ctx.assets.open(u.removePrefix("asset://")).use { it.readBytes() }
            u.startsWith("asset:") -> ctx.assets.open(u.removePrefix("asset:")).use { it.readBytes() }
            u.startsWith("file://") -> File(u.removePrefix("file://")).readBytes()
            u.startsWith("/") -> File(u).readBytes()
            u.startsWith("base64:") -> Base64.decode(u.removePrefix("base64:"), Base64.DEFAULT)
            else -> ctx.assets.open(u).use { it.readBytes() }
        }
    }
}
