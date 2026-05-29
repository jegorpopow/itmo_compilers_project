add_executable(vm_tests
  test_arithmetic.cpp
  test_control_flow.cpp
  test_gc.cpp
  test_print.cpp
)

target_link_libraries(vm_tests PRIVATE
  vm_lib
  GTest::gtest_main
)

include(GoogleTest)
gtest_discover_tests(vm_tests)
