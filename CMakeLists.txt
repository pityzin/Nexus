cmake_minimum_required(VERSION 3.20)
project(NexusNative VERSION 0.1.0 LANGUAGES C CXX)

include(CTest)
set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(NEXUS_ARCHIVE_DIR "${CMAKE_BINARY_DIR}/lib" CACHE PATH "Directory for the NEXUS static library")
find_program(NASM_EXECUTABLE NAMES nasm)
set(NEXUS_ASSEMBLY_SOURCE "${CMAKE_CURRENT_SOURCE_DIR}/core/assembly/x86_64/add_i64_fallback.cpp")
if(NASM_EXECUTABLE AND WIN32 AND CMAKE_SIZEOF_VOID_P EQUAL 8)
    set(CMAKE_ASM_NASM_OBJECT_FORMAT win64)
    enable_language(ASM_NASM)
    set(NEXUS_ASSEMBLY_SOURCE "${CMAKE_CURRENT_SOURCE_DIR}/core/assembly/x86_64/add_i64_win64.asm")
elseif(MSVC AND CMAKE_SIZEOF_VOID_P EQUAL 8)
    enable_language(ASM_MASM)
    set(NEXUS_ASSEMBLY_SOURCE "${CMAKE_CURRENT_SOURCE_DIR}/core/assembly/x86_64/add_i64_win64_masm.asm")
endif()

add_library(nexus_native STATIC core/c/src/version.c core/cpp/src/native.cpp ${NEXUS_ASSEMBLY_SOURCE})
target_include_directories(nexus_native PUBLIC core/c/include)
target_compile_features(nexus_native PUBLIC cxx_std_20)
set_target_properties(nexus_native PROPERTIES
    ARCHIVE_OUTPUT_DIRECTORY "${NEXUS_ARCHIVE_DIR}"
    ARCHIVE_OUTPUT_DIRECTORY_DEBUG "${NEXUS_ARCHIVE_DIR}"
    ARCHIVE_OUTPUT_DIRECTORY_RELEASE "${NEXUS_ARCHIVE_DIR}"
    ARCHIVE_OUTPUT_DIRECTORY_RELWITHDEBINFO "${NEXUS_ARCHIVE_DIR}"
    ARCHIVE_OUTPUT_DIRECTORY_MINSIZEREL "${NEXUS_ARCHIVE_DIR}")

if(BUILD_TESTING)
    find_package(Threads REQUIRED)
    add_executable(nexus_native_tests tests/native/native_tests.cpp)
    target_link_libraries(nexus_native_tests PRIVATE nexus_native Threads::Threads)
    add_test(NAME nexus_native_tests COMMAND nexus_native_tests)

    add_executable(nexus_native_c_tests tests/native/native_c_tests.c)
    target_link_libraries(nexus_native_c_tests PRIVATE nexus_native)
    add_test(NAME nexus_native_c_tests COMMAND nexus_native_c_tests)
endif()
