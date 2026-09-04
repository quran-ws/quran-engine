plugins { id("com.android.application"); id("org.jetbrains.kotlin.android") }

android {
    namespace = "net.quranpedia.qvp.demo"
    compileSdk = 35
    defaultConfig {
        applicationId = "net.quranpedia.qvp.demo"
        minSdk = 24; targetSdk = 35; versionCode = 1; versionName = "0.1"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    buildTypes { release { isMinifyEnabled = false } }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    implementation(project(":qvp"))
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.15.0")
}
