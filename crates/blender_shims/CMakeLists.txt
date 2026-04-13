cmake_minimum_required(VERSION 3.20)
project(blender_shim LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT DEFINED BLENDER_SOURCE_DIR)
    set(BLENDER_SOURCE_DIR "/home/bu/dvl/cpp/blender")
endif()

set(BLENDER_VERSION_HEADER
        "${BLENDER_SOURCE_DIR}/source/blender/blenkernel/BKE_blender_version.h")

if(NOT EXISTS "${BLENDER_VERSION_HEADER}")
    message(FATAL_ERROR
            "Could not find Blender version header at: ${BLENDER_VERSION_HEADER}\n"
            "Set -DBLENDER_SOURCE_DIR=/path/to/blender")
endif()

add_library(blender_shim STATIC
        src-cpp/blender_shim.cpp
)

target_include_directories(blender_shim
        PUBLIC
        "${CMAKE_CURRENT_SOURCE_DIR}/src-cpp"
        "${BLENDER_SOURCE_DIR}/source/blender/blenkernel"
)

target_compile_features(blender_shim PUBLIC cxx_std_17)

# Nice-to-have warnings.
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(blender_shim PRIVATE -Wall -Wextra -Wpedantic)
endif()

install(TARGETS blender_shim
        ARCHIVE DESTINATION lib
        LIBRARY DESTINATION lib
        RUNTIME DESTINATION bin
)