import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import com.vanniktech.maven.publish.MavenPublishBaseExtension

plugins {
    id("com.android.application") version "8.7.3" apply false
    id("com.android.library") version "8.7.3" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
    id("com.vanniktech.maven.publish") version "0.35.0" apply false
}

// Publishing belongs to this build, not to the library module. `:qvp` is also included by
// other root builds — the React Native example's — which know nothing of the publishing plugin
// and only need the module to compile. So the module's own build file carries no publishing,
// and this root applies it to the module here (`:qvp:publishAndReleaseToMavenCentral`).
project(":qvp") {
    // once the module has applied the Android plugin, which the publishing plugin reads
    plugins.withId("com.android.library") {
        apply(plugin = "com.vanniktech.maven.publish")
        val qvpVersion = providers.gradleProperty("qvpVersion").getOrElse("0.1.0-SNAPSHOT")
        configure<MavenPublishBaseExtension> {
            configure(AndroidSingleVariantLibrary(variant = "release", sourcesJar = true, publishJavadocJar = true))
            coordinates("ws.quran", "qvp-android", qvpVersion)
            publishToMavenCentral()

            pom {
                name.set("QVP Android")
                description.set("Android wrapper and Canvas renderer for the QVP vector Mushaf engine")
                inceptionYear.set("2026")
                url.set("https://github.com/quran-ws/quran-engine")
                licenses {
                    license {
                        name.set("MIT")
                        url.set("https://opensource.org/license/mit/")
                        distribution.set("repo")
                    }
                }
                developers {
                    developer {
                        id.set("quran-ws")
                        name.set("Quran.ws")
                        url.set("https://quran.ws")
                    }
                }
                scm {
                    url.set("https://github.com/quran-ws/quran-engine")
                    connection.set("scm:git:https://github.com/quran-ws/quran-engine.git")
                    developerConnection.set("scm:git:ssh://git@github.com/quran-ws/quran-engine.git")
                }
            }
        }
    }
}
