cmake_minimum_required(VERSION 3.14)
project(qvp_flutter LANGUAGES CXX)

set(PLUGIN_NAME "qvp_flutter_plugin")

set(QVP_SO_PATH "${CMAKE_CURRENT_SOURCE_DIR}/libqvp_ffi.so")

# If not present in plugin directory, check the Cargo build: target/dist (the shipped
# profile) or target/release (a host build for tests), whichever is newer
if(NOT EXISTS "${QVP_SO_PATH}")
  set(CARGO_DIST_SO "${CMAKE_CURRENT_SOURCE_DIR}/../../../../target/dist/libqvp_ffi.so")
  set(CARGO_RELEASE_SO "${CMAKE_CURRENT_SOURCE_DIR}/../../../../target/release/libqvp_ffi.so")
  if(EXISTS "${CARGO_DIST_SO}" AND EXISTS "${CARGO_RELEASE_SO}")
    if("${CARGO_RELEASE_SO}" IS_NEWER_THAN "${CARGO_DIST_SO}")
      set(QVP_SO_PATH "${CARGO_RELEASE_SO}")
    else()
      set(QVP_SO_PATH "${CARGO_DIST_SO}")
    endif()
  elseif(EXISTS "${CARGO_DIST_SO}")
    set(QVP_SO_PATH "${CARGO_DIST_SO}")
  elseif(EXISTS "${CARGO_RELEASE_SO}")
    set(QVP_SO_PATH "${CARGO_RELEASE_SO}")
  endif()
endif()

if(NOT EXISTS "${QVP_SO_PATH}")
  message(FATAL_ERROR "libqvp_ffi.so not found in ${CMAKE_CURRENT_SOURCE_DIR}, target/dist or target/release. Build the Cargo crate first.")
endif()
message(STATUS "qvp_flutter: bundling ${QVP_SO_PATH}")

# Expose prebuilt libqvp_ffi.so to be bundled with the final application binary
set(qvp_flutter_bundled_libraries
  "${QVP_SO_PATH}"
  PARENT_SCOPE
)

# Dummy target for IDE integration and clean project definition
add_custom_target(${PLUGIN_NAME})
