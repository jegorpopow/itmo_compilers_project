add_executable(vm_tests
  test_arithmetic.cpp
  test_control_flow.cpp
  test_gc.cpp
  test_print.cpp
  test_golden.cpp
)

target_link_libraries(vm_tests PRIVATE
  vm_lib
  GTest::gtest_main
)

target_compile_definitions(vm_tests PRIVATE
  COMPILER_SRC_DIR="${COMPILER_SRC_DIR}"
  TESTS_SRC_DIR="${TESTS_SRC_DIR}"
  TESTS_PASS_DIR="${TESTS_PASS_DIR}"
)

include(GoogleTest)
gtest_discover_tests(vm_tests)
