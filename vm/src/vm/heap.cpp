#include "vm/heap.hpp"

#include <bit>
#include <fmt/format.h>
#include <stdexcept>

#include "vm/address.hpp"
#include "vm/error.hpp"
#include "vm/vm.hpp"

namespace vm {

// ---- HeapObject -------------------------------------------------------------

Value& HeapObject::ElementAt(int64_t lang_index) {
  if (lang_index < 1 || static_cast<uint64_t>(lang_index) > fields.size())
    throw IndexOutOfBoundsError(
        fmt::format("array index {} out of bounds [1,{}]",
                    lang_index, fields.size()));
  return fields[static_cast<std::size_t>(lang_index - 1)];
}

const Value& HeapObject::ElementAt(int64_t lang_index) const {
  if (lang_index < 1 || static_cast<uint64_t>(lang_index) > fields.size())
    throw IndexOutOfBoundsError(
        fmt::format("array index {} out of bounds [1,{}]",
                    lang_index, fields.size()));
  return fields[static_cast<std::size_t>(lang_index - 1)];
}

// ---- GarbageCollector -------------------------------------------------------

HeapObject* GarbageCollector::Allocate(uint32_t type_id,
                                       HeapObjectKind kind,
                                       uint64_t count) {
  auto* obj = new HeapObject();
  obj->type_id = type_id;
  obj->kind    = kind;
  obj->fields.resize(static_cast<std::size_t>(count));
  objects_.push_back(obj);
  return obj;
}

void GarbageCollector::Collect(Vm& vm) {
  Mark(vm);
  Sweep();
}

void GarbageCollector::Mark(Vm& vm) {
  // Mark all values reachable from GC roots.

  // Root set 1: global variables.
  for (const Value& v : vm.globals())
    MarkValue(v);

  // Root set 2: call stack arguments (locals live on eval_stack).
  for (const CallFrame& frame : vm.call_stack()) {
    for (const Value& v : frame.arguments) MarkValue(v);
  }

  // Root set 3: evaluation stack.
  for (const Value& v : vm.eval_stack())
    MarkValue(v);

  // Root set 4: heap addresses stored on the evaluation stack
  //             that reference heap objects indirectly.
  for (const Value& v : vm.eval_stack()) {
    if (!v.IsAddress()) continue;
    auto* desc = std::bit_cast<AddressDescriptor*>(v.data);
    if (desc->kind == AddressDescriptor::Kind::kHeapField && desc->object)
      MarkObject(desc->object);
  }
}

void GarbageCollector::MarkValue(const Value& v) {
  if (!v.IsRef()) return;
  auto* obj = std::bit_cast<HeapObject*>(v.data);
  if (obj) MarkObject(obj);
}

void GarbageCollector::MarkObject(HeapObject* obj) {
  if (!obj || obj->marked) return;
  obj->marked = true;
  for (const Value& field : obj->fields)
    MarkValue(field);
}

void GarbageCollector::Sweep() {
  std::vector<HeapObject*> live;
  for (HeapObject* obj : objects_) {
    if (obj->marked) {
      obj->marked = false;  // reset for next cycle
      live.push_back(obj);
    } else {
      delete obj;
    }
  }
  objects_ = std::move(live);
}

}  // namespace vm
