#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "vm/value.hpp"

namespace vm {

// A single activation record (stack frame) for a function call.
struct CallFrame {
  std::string function_name;

  // Index into Program::instructions where execution resumes after Ret.
  std::size_t return_pc;

  // Arguments passed to this function (copied from eval stack before call).
  std::vector<Value> arguments;

  // Local variables declared inside this function.
  // Accessed by index via Location::Local.
  std::vector<Value> locals;

  // TypeId of the return value (kVoidTypeId for procedures).
  uint32_t return_type_id;

  // True if this frame expects a return value to be left on eval stack.
  bool has_return_value() const { return return_type_id != kVoidTypeId; }
};

}  // namespace vm
