cmake_minimum_required(VERSION 3.22)
project(qvp_jni C)
# Rust engine, prebuilt per ABI by: cargo ndk -t arm64-v8a -t x86_64 -t armeabi-v7a build -p qvp-ffi --release
add_library(qvp_ffi STATIC IMPORTED)
set_target_properties(qvp_ffi PROPERTIES IMPORTED_LOCATION ${CMAKE_SOURCE_DIR}/../../../prebuilt/${ANDROID_ABI}/libqvp_ffi.a)
add_library(qvp_jni SHARED qvp_jni.c)
target_include_directories(qvp_jni PRIVATE ${CMAKE_SOURCE_DIR})
target_link_libraries(qvp_jni qvp_ffi log dl m)
