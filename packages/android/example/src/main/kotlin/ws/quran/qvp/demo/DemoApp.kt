package ws.quran.qvp.demo

import android.app.Application
import androidx.appcompat.app.AppCompatDelegate
import com.google.android.material.color.DynamicColors

/**
 * Material 3 is set up once for the whole app rather than per screen.
 *
 * Day or night in particular: asking for it from an activity that is already running tears that
 * activity down and builds it again, so it is settled here, before any screen exists. The Paper
 * control may still switch it later, which is a real change the reader asked for.
 */
class DemoApp : Application() {
    override fun onCreate() {
        super.onCreate()
        // the demo opens on light paper, so the controls around it open light too
        AppCompatDelegate.setDefaultNightMode(AppCompatDelegate.MODE_NIGHT_NO)
        DynamicColors.applyToActivitiesIfAvailable(this)
    }
}
