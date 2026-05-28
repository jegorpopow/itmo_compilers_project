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

  // Index into Vm::eval_stack_ where this frame's locals begin.
  // Locals live on the eval stack at eval_stack_[eval_stack_base + local_index].
  std::size_t eval_stack_base = 0;

  // TypeId of the return value (kVoidTypeId for procedures).
  uint32_t return_type_id;
};

}  // namespace vm
