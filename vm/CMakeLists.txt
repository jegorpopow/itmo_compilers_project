cmake_minimum_required(VERSION 3.25)
project(itmo_vm CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(CMAKE_EXPORT_COMPILE_COMMANDS true)

# Diagnostics
add_compile_options(-Wall -Wextra -Wpedantic)
if (CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
    add_compile_options(-ferror-limit=0)
elseif (CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    add_compile_options(-fmax-errors=0)
endif ()

# Debug flags
if (CMAKE_BUILD_TYPE MATCHES Debug)
    add_compile_options(-DDEBUG -g -ggdb3 -O0 -fno-omit-frame-pointer)
endif ()

# Enable assertions in std (both libstdc++ & libc++) for Debug and RelWithAssert builds
if (CMAKE_BUILD_TYPE MATCHES Debug|RelWithAssert)
    if (NOT (MSVC))
        add_definitions("-D_GLIBCXX_DEBUG")
        add_definitions("-D_LIBCPP_HARDENING_MODE=_LIBCPP_HARDENING_MODE_DEBUG")

        include(CheckSourceCompiles)

        check_source_compiles(CXX "
#define _LIBCPP_ENABLE_ASSERTIONS 1
#include <iostream>
int main() { std::cout << \"Hello, world!\" << std::endl; }
" OK_WITH_OLD_LIBCPP_FLAG)
        if (OK_WITH_OLD_LIBCPP_FLAG)
            add_definitions("-D_LIBCPP_ENABLE_ASSERTIONS=1")
        endif ()
    endif ()
endif ()

# Enable sanitizers for Debug builds
include(cmake/sanitizers.cmake)
if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    add_sanitizers()
endif ()

# Enable LTO for non-Debug builds (if supported)
include(CheckIPOSupported)
check_ipo_supported(RESULT lto_supported OUTPUT error)
if (NOT (lto_supported))
    message(STATUS "LTO not supported: <${error}>")
elseif (CMAKE_BUILD_TYPE STREQUAL "Debug")
    message(STATUS "LTO supported, but not enabled (Debug build)")
else ()
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
    message(STATUS "LTO enabled")
endif ()

include(FetchContent)

FetchContent_Declare(
  googletest
  GIT_REPOSITORY https://github.com/google/googletest.git
  GIT_TAG        v1.15.2
  GIT_SHALLOW    TRUE
)
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(googletest)

FetchContent_Declare(
  gflags
  GIT_REPOSITORY https://github.com/gflags/gflags.git
  GIT_TAG        v2.2.2
  GIT_SHALLOW    TRUE
)
set(GFLAGS_BUILD_TESTING OFF CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(gflags)

FetchContent_Declare(
  fmt
  GIT_REPOSITORY https://github.com/fmtlib/fmt.git
  GIT_TAG        11.1.4
  GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(fmt)

add_subdirectory(src)
enable_testing()
add_subdirectory(tests)
