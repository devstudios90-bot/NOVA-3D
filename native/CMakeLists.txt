cmake_minimum_required(VERSION 3.22.1)
project(nova_renderer LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Paths supplied by Gradle or defaulted relative to native/
if(NOT DEFINED NOVA_ROOT_DIR)
    get_filename_component(NOVA_ROOT_DIR "${CMAKE_CURRENT_SOURCE_DIR}/.." ABSOLUTE)
endif()

set(FILAMENT_DIR "${NOVA_ROOT_DIR}/third_party/filament/filament")
set(FILAMENT_INCLUDE_DIR "${FILAMENT_DIR}/include")
set(FILAMENT_LIB_DIR "${FILAMENT_DIR}/lib/${ANDROID_ABI}")

# Map Android ABI to Rust target triple for cargo-ndk staticlib lookup
if(ANDROID_ABI STREQUAL "arm64-v8a")
    set(RUST_TARGET "aarch64-linux-android")
elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
    set(RUST_TARGET "armv7-linux-androideabi")
elseif(ANDROID_ABI STREQUAL "x86_64")
    set(RUST_TARGET "x86_64-linux-android")
elseif(ANDROID_ABI STREQUAL "x86")
    set(RUST_TARGET "i686-linux-android")
else()
    message(FATAL_ERROR "Unsupported ANDROID_ABI: ${ANDROID_ABI}")
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(RUST_PROFILE "debug")
else()
    set(RUST_PROFILE "release")
endif()

set(NOVA_CORE_LIB "${NOVA_ROOT_DIR}/rust/nova_core/target/${RUST_TARGET}/${RUST_PROFILE}/libnova_core.a")
if(NOT EXISTS "${NOVA_CORE_LIB}")
    # Fallback to release staticlib if debug wasn't built separately
    set(NOVA_CORE_LIB_FALLBACK "${NOVA_ROOT_DIR}/rust/nova_core/target/${RUST_TARGET}/release/libnova_core.a")
    if(EXISTS "${NOVA_CORE_LIB_FALLBACK}")
        set(NOVA_CORE_LIB "${NOVA_CORE_LIB_FALLBACK}")
    else()
        message(FATAL_ERROR "Rust static library not found at ${NOVA_CORE_LIB}. Run cargo-ndk build first.")
    endif()
endif()

add_library(nova_core STATIC IMPORTED)
set_target_properties(nova_core PROPERTIES IMPORTED_LOCATION "${NOVA_CORE_LIB}")

function(add_filament_lib NAME)
    add_library("filament_${NAME}" STATIC IMPORTED)
    set_target_properties("filament_${NAME}" PROPERTIES
        IMPORTED_LOCATION "${FILAMENT_LIB_DIR}/lib${NAME}.a"
    )
endfunction()

set(FILAMENT_LIBS
    filament
    backend
    bluevk
    filabridge
    filaflat
    filamat
    shaders
    geometry
    ibl
    utils
    smol-v
    abseil
    zstd
    perfetto
)

foreach(LIB ${FILAMENT_LIBS})
    add_filament_lib(${LIB})
endforeach()

add_library(nova_renderer SHARED
    renderer/filament_backend.cpp
    bridge/nova_jni.cpp
)

target_include_directories(nova_renderer PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/renderer"
    "${FILAMENT_INCLUDE_DIR}"
)

target_compile_options(nova_renderer PRIVATE
    -Wall
    -Wextra
    -Wno-unused-parameter
    -fexceptions
)

# Filament static archives have mutual references (e.g. filament <-> backend <-> utils <-> filamat)
target_link_libraries(nova_renderer PRIVATE
    -Wl,--start-group
    filament_filament
    filament_backend
    filament_bluevk
    filament_filabridge
    filament_filaflat
    filament_filamat
    filament_shaders
    filament_geometry
    filament_ibl
    filament_utils
    filament_smol-v
    filament_abseil
    filament_zstd
    filament_perfetto
    -Wl,--end-group
    nova_core
    android
    log
    EGL
    GLESv3
    jnigraphics
    dl
    m
    z
)
