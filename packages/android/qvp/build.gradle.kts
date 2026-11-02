// Publishing is applied and configured by this build's root (packages/android/build.gradle.kts),
// so a root that only wants the module to compile — the React Native example — can include it
// without the publishing plugin.
//
// `-PqvpSkipNative=true` drops the CMake build, the NDK version and the ABI filters. The
// Android Gradle plugin reads the NDK when it configures a project that declares a native
// build, so a root that compiles only the Kotlin pays for an NDK it never calls. The AAR
// this repository publishes is always built without the property, so it always holds
// libqvp_jni.so. A root that sets the property gets Kotlin that compiles and no engine
// behind it, which suits a compile check and suits nothing else.
plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

val skipNative = (project.findProperty("qvpSkipNative") as String?).toBoolean()

android {
    namespace = "ws.quran.qvp"
    compileSdk = 35
    if (!skipNative) ndkVersion = "27.2.12479018"
    defaultConfig {
        minSdk = 24
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
        if (!skipNative) {
            externalNativeBuild {
                cmake {
                    arguments += listOf("-DANDROID_STL=none", "-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON")
                }
            }
            ndk { abiFilters += listOf("arm64-v8a", "x86_64", "armeabi-v7a") }
        }
    }
    if (!skipNative) {
        externalNativeBuild { cmake { path = file("src/main/cpp/CMakeLists.txt"); version = "3.22.1" } }
    }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
}
