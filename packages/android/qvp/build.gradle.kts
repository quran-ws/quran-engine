plugins { id("com.android.library"); id("org.jetbrains.kotlin.android") }

android {
    namespace = "ws.quran.qvp"
    compileSdk = 35
    ndkVersion = "27.2.12479018"
    defaultConfig {
        minSdk = 24
        externalNativeBuild { cmake { arguments += listOf("-DANDROID_STL=none") } }
        ndk { abiFilters += listOf("arm64-v8a", "x86_64", "armeabi-v7a") }
    }
    externalNativeBuild { cmake { path = file("src/main/cpp/CMakeLists.txt"); version = "3.22.1" } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
    publishing { singleVariant("release") }
}
