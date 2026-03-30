add_library(vm_lib
  vm/heap.cpp
  vm/loader.cpp
  vm/vm.cpp
)

target_include_directories(vm_lib PUBLIC
  ${CMAKE_CURRENT_SOURCE_DIR}/../include
)

target_link_libraries(vm_lib PUBLIC fmt::fmt)

add_executable(vm main.cpp)
target_link_libraries(vm PRIVATE vm_lib gflags::gflags)
