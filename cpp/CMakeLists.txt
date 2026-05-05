cmake_minimum_required(VERSION 3.20)

project(smart_cache_cpp_examples LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_library(smart_cache_headers INTERFACE)
target_include_directories(
    smart_cache_headers
    INTERFACE
        ${CMAKE_CURRENT_SOURCE_DIR}/include
)

find_library(
    SMART_CACHE_FFI_LIBRARY
    NAMES smart_cache_ffi
    PATHS
        ${CMAKE_CURRENT_LIST_DIR}/../target/debug
        ${CMAKE_CURRENT_LIST_DIR}/../target/release
    NO_DEFAULT_PATH
)

if(NOT SMART_CACHE_FFI_LIBRARY)
    message(FATAL_ERROR "smart_cache_ffi library not found. Run `cargo build -p smart_cache_ffi` first.")
endif()

add_executable(smart_cache_basic examples/basic.cpp)
target_link_libraries(
    smart_cache_basic
    PRIVATE
        smart_cache_headers
        ${SMART_CACHE_FFI_LIBRARY}
)

enable_testing()
add_test(NAME smart_cache_basic COMMAND smart_cache_basic)
