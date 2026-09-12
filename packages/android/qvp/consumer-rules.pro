# QvpNative uses name-based JNI symbols. Keep the bridge when the consuming app enables R8.
-keep class ws.quran.qvp.QvpNative { *; }
