cmake_minimum_required(VERSION 3.25)
project(itmo_vm CXX)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_compile_options(-Wall -Wextra -Wpedantic)

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
