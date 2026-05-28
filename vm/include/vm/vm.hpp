#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "vm/address.hpp"
#include "vm/call_frame.hpp"
#include "vm/heap.hpp"
#include "vm/interpreter.hpp"
#include "vm/program.hpp"
#include "vm/value.hpp"

namespace vm {

// Main virtual machine class.
//
// Execution model:
//   - Stack-based: all intermediate values live on eval_stack_.
//   - Global variables live in globals_ (size = program.global_count).
//   - Each function call creates a CallFrame on call_stack_.
//   - The program counter pc_ indexes into program_.instructions.
//
// GC is triggered before each heap allocation (AllocRecord, AllocArray).
class Vm {
 public:
  explicit Vm(Program program);

  // Runs the program starting from "main".
  // Throws VmError on any runtime error or LoadError if main is missing.
  void Run();

  // ---- Evaluation stack operations ----------------------------------------

  void Push(Value v);
  Value Pop();
  Value& Top();
  const Value& Top() const;

  // ---- Variable access -----------------------------------------------------

  Value& GetVariable(LocationKind kind, uint16_t index);
  void SetVariable(LocationKind kind, uint16_t index, Value v);

  Value& GetGlobal(uint16_t index);
  Value& GetLocal(uint16_t index);
  Value& GetArgument(uint16_t index);

  // ---- Address operations --------------------------------------------------

  // Allocates an AddressDescriptor and returns a Value of kAddressTypeId.
  Value MakeVarAddress(LocationKind kind, uint16_t index);
  Value MakeHeapFieldAddress(HeapObject* obj, uint64_t field_idx);

  // Reads the value pointed to by an address Value.
  Value LoadAddress(const Value& addr_val);

  // Writes `value` to the location pointed to by an address Value.
  void StoreAddress(const Value& addr_val, Value value);

  // ---- Control flow --------------------------------------------------------

  // Jumps to the instruction at label_id (sets pc_).
  void Jump(uint64_t label_id);

  // Sets up a new CallFrame and jumps to function entry.
  // `arg_count` values are popped from eval_stack_ as arguments.
  void Call(uint64_t function_label_id);

  // Returns from the current function.
  // Pops the top CallFrame; if the function has a return type, leaves the
  // return value on the eval stack for the caller.
  void Return();

  // Halts the execution loop.
  void Halt();

  // ---- Heap / GC -----------------------------------------------------------

  HeapObject* AllocRecord(uint32_t type_id, uint64_t num_fields);
  HeapObject* AllocArray(uint32_t type_id, uint64_t num_elements);

  // ---- Accessors for GC roots (used by GarbageCollector) ------------------

  std::vector<Value>&      eval_stack()  { return eval_stack_; }
  std::vector<Value>&      globals()     { return globals_; }
  std::vector<CallFrame>&  call_stack()  { return call_stack_; }
  AddressPool&             address_pool(){ return address_pool_; }
  const Program&           program()     { return program_; }

 private:
  Program             program_;
  std::vector<Value>  eval_stack_;
  std::vector<Value>  globals_;
  std::vector<CallFrame> call_stack_;
  std::size_t         pc_ = 0;
  bool                halted_ = false;

  GarbageCollector    gc_;
  AddressPool         address_pool_;

  // GC trigger: collect every kGcInterval allocations.
  static constexpr std::size_t kGcInterval = 128;
  std::size_t alloc_count_ = 0;
};

}  // namespace vm
