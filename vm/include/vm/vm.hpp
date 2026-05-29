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

class Vm {
 public:
  explicit Vm(Program program);

  void run();

  void push(Value v);
  Value pop();
  Value& top();
  const Value& top() const;

  Value& getVariable(LocationKind kind, uint16_t index);
  void setVariable(LocationKind kind, uint16_t index, Value v);

  Value& getGlobal(uint16_t index);
  Value& getLocal(uint16_t index);
  Value& getArgument(uint16_t index);

  Value makeVarAddress(LocationKind kind, uint16_t index);
  Value makeHeapFieldAddress(HeapObject* obj, uint64_t field_idx);

  Value loadAddress(const Value& addr_val);

  void storeAddress(const Value& addr_val, Value value);

  void jump(uint64_t label_id);

  void call(uint64_t function_label_id);

  void ret();

  void halt();

  HeapObject* allocRecord(uint32_t type_id, uint64_t num_fields);
  HeapObject* allocArray(uint32_t type_id, uint64_t num_elements);

  std::vector<Value>&      eval_stack()  { return eval_stack_; }
  std::vector<Value>&      globals()     { return globals_; }
  std::vector<CallFrame>&  call_stack()  { return call_stack_; }
  const Program&           program()     { return program_; }

 private:
  void maybeCollect();

  Program             program_;
  std::vector<Value>  eval_stack_;
  std::vector<Value>  globals_;
  std::vector<CallFrame> call_stack_;
  std::size_t         pc_ = 0;
  bool                halted_ = false;

  GarbageCollector    gc_;

  static constexpr std::size_t kMinGcThreshold = 10000;
  std::size_t next_gc_at_ = kMinGcThreshold;
};

}
