#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "vm/value.hpp"

namespace vm {

struct CallFrame {
  std::string function_name;

  std::size_t return_pc;

  std::vector<Value> arguments;

  std::size_t eval_stack_base = 0;

  uint32_t return_type_id;
};

}
