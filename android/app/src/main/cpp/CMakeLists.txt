cmake_minimum_required(VERSION 3.18.1)
project("ptyjni")

add_library(ptyjni SHARED
    ${CMAKE_CURRENT_SOURCE_DIR}/ptyjni.c
)

target_compile_features(ptyjni PRIVATE c_std_11)

target_link_libraries(ptyjni
    log
)
