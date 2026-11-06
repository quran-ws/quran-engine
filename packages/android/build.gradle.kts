import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import com.vanniktech.maven.publish.MavenPublishBaseExtension
import io.gitlab.arturbosch.detekt.extensions.DetektExtension

plugins {
    id("com.android.application") version "8.7.3" apply false
    id("com.android.library") version "8.7.3" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
    id("com.vanniktech.maven.publish") version "0.35.0" apply false
    id("io.gitlab.arturbosch.detekt") version "1.23.8" apply false
}

// Publishing belongs to this build, not to the library module. `:qvp` is also included by
// other root builds — the React Native example's — which know nothing of the publishing plugin
// and only need the module to compile. So the module's own build file carries no publishing,
// and this root applies it to the module here (`:qvp:publishAndReleaseToMavenCentral`).
project(":qvp") {
    // once the module has applied the Android plugin, which the publishing plugin reads
    plugins.withId("com.android.library") {
        apply(plugin = "com.vanniktech.maven.publish")
        val qvpVersion = providers.gradleProperty("qvpVersion").getOrElse("0.8.0")
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
    // The doc-block check of docs/standards/CODE-STYLE.md, applied here for the same reason as
    // publishing: a root that includes `:qvp` only to compile it needs no detekt. detekt.yml turns
    // on the two doc rules and nothing else. detekt-baseline.xml lists the items that had no doc
    // block when the check went in; a new item without one fails. `./gradlew :qvp:detekt` runs it,
    // and `:qvp:detektBaseline` writes the list again after items are documented.
    apply(plugin = "io.gitlab.arturbosch.detekt")
    configure<DetektExtension> {
        buildUponDefaultConfig = true
        config.setFrom(rootProject.file("detekt.yml"))
        baseline = rootProject.file("detekt-baseline.xml")
        source.setFrom("src/main/kotlin")
    }
}
