cmake_minimum_required(VERSION 3.14)
project(qvp_flutter LANGUAGES CXX)

set(PLUGIN_NAME "qvp_flutter_plugin")

set(QVP_SO_PATH "${CMAKE_CURRENT_SOURCE_DIR}/libqvp_ffi.so")

# If not present in plugin directory, check the Cargo build: target/dist (the shipped
# profile), then target/release (a host build for tests)
if(NOT EXISTS "${QVP_SO_PATH}")
  foreach(profile dist release)
    set(CARGO_SO "${CMAKE_CURRENT_SOURCE_DIR}/../../../../target/${profile}/libqvp_ffi.so")
    if(EXISTS "${CARGO_SO}")
      set(QVP_SO_PATH "${CARGO_SO}")
      break()
    endif()
  endforeach()
endif()

if(NOT EXISTS "${QVP_SO_PATH}")
  message(FATAL_ERROR "libqvp_ffi.so not found in ${CMAKE_CURRENT_SOURCE_DIR}, target/dist or target/release. Build the Cargo crate first.")
endif()

# Expose prebuilt libqvp_ffi.so to be bundled with the final application binary
set(qvp_flutter_bundled_libraries
  "${QVP_SO_PATH}"
  PARENT_SCOPE
)

# Dummy target for IDE integration and clean project definition
add_custom_target(${PLUGIN_NAME})
