import com.vanniktech.maven.publish.AndroidSingleVariantLibrary

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("com.vanniktech.maven.publish")
}

val qvpVersion = providers.gradleProperty("qvpVersion").getOrElse("0.1.0-SNAPSHOT")

android {
    namespace = "ws.quran.qvp"
    compileSdk = 35
    ndkVersion = "27.2.12479018"
    defaultConfig {
        minSdk = 24
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
        externalNativeBuild {
            cmake {
                arguments += listOf("-DANDROID_STL=none", "-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON")
            }
        }
        ndk { abiFilters += listOf("arm64-v8a", "x86_64", "armeabi-v7a") }
    }
    externalNativeBuild { cmake { path = file("src/main/cpp/CMakeLists.txt"); version = "3.22.1" } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
}

mavenPublishing {
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
