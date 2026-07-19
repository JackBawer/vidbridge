cmake_minimum_required(VERSION 3.16)
project(vidbridge C)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(PkgConfig REQUIRED)
pkg_check_modules(AVFORMAT REQUIRED IMPORTED_TARGET libavformat)
pkg_check_modules(AVCODEC REQUIRED IMPORTED_TARGET libavcodec)
pkg_check_modules(AVUTIL REQUIRED IMPORTED_TARGET libavutil)

add_library(vidbridge STATIC
    src/decoder.c
    src/demuxer.c
    src/encoder.c
    src/muxer.c
    src/frame_utils.c
    src/common.c
)

target_include_directories(vidbridge PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(vidbridge PUBLIC
    PkgConfig::AVFORMAT
    PkgConfig::AVCODEC
    PkgConfig::AVUTIL
)

option(ENABLE_SANITIZERS "Build with ASan/UBSan" OFF)
if(ENABLE_SANITIZERS)
    target_compile_options(vidbridge PUBLIC -fsanitize=address,undefined -g)
    target_link_options(vidbridge PUBLIC -fsanitize=address,undefined)
endif()

option(BUILD_TESTS "Build the C test suite" ON)
if(BUILD_TESTS)
    add_executable(vidbridge_test tests/test_pipeline.c)
    target_link_libraries(vidbridge_test PRIVATE vidbridge)
endif()

install(TARGETS vidbridge
    ARCHIVE DESTINATION .
    LIBRARY DESTINATION .
    RUNTIME DESTINATION .
)
