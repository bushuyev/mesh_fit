cmake_minimum_required(VERSION 3.20)
project(blender_shim LANGUAGES C CXX)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT DEFINED BLENDER_SOURCE_DIR OR BLENDER_SOURCE_DIR STREQUAL "")
    message(FATAL_ERROR
            "BLENDER_SOURCE_DIR is missing or empty.\n"
            "Please configure with -DBLENDER_SOURCE_DIR=/path/to/blender")
endif()

if(NOT IS_DIRECTORY "${BLENDER_SOURCE_DIR}")
    message(FATAL_ERROR
            "BLENDER_SOURCE_DIR does not point to a directory: ${BLENDER_SOURCE_DIR}")
endif()

set(BLENDER_VERSION_HEADER
        "${BLENDER_SOURCE_DIR}/source/blender/blenkernel/BKE_blender_version.h")

if(NOT EXISTS "${BLENDER_VERSION_HEADER}")
    message(FATAL_ERROR
            "Could not find Blender version header at: ${BLENDER_VERSION_HEADER}\n"
            "Set -DBLENDER_SOURCE_DIR=/path/to/blender")
endif()

set(BLENDER_BLENKERNEL_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/blenkernel")
set(BLENDER_BLENLIB_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/blenlib")
set(BLENDER_GUARDEDALLOC_DIR
        "${BLENDER_SOURCE_DIR}/intern/guardedalloc")
set(BLENDER_CLOG_DIR
        "${BLENDER_SOURCE_DIR}/intern/clog")
set(BLENDER_MAKESDNA_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/makesdna")
set(BLENDER_BLENLOADER_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/blenloader")
set(BLENDER_IMBUF_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/imbuf")
set(BLENDER_EDITORS_DIR
        "${BLENDER_SOURCE_DIR}/source/blender/editors/include")

add_library(blender_shim STATIC
        src-cpp/blender_shim.cpp
)

target_include_directories(blender_shim
        PUBLIC
        "${CMAKE_CURRENT_SOURCE_DIR}/src-cpp"
        "${BLENDER_BLENKERNEL_DIR}"
        "${BLENDER_BLENLIB_DIR}"
        "${BLENDER_GUARDEDALLOC_DIR}"
        "${BLENDER_CLOG_DIR}"
        "${BLENDER_MAKESDNA_DIR}"
        "${BLENDER_BLENLOADER_DIR}"
        "${BLENDER_EDITORS_DIR}"
        "${BLENDER_IMBUF_DIR}"
)

target_compile_features(blender_shim PUBLIC cxx_std_23)

if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(blender_shim PRIVATE -Wall -Wextra -Wpedantic)
endif()

install(TARGETS blender_shim
        ARCHIVE DESTINATION lib
        LIBRARY DESTINATION lib
        RUNTIME DESTINATION bin
)