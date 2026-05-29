#include "vm/heap.hpp"

#include <bit>
#include <fmt/format.h>

#include "vm/address.hpp"
#include "vm/error.hpp"
#include "vm/vm.hpp"

namespace vm {

Value& HeapObject::elementAt(int64_t lang_index) {
  if (lang_index < 1 || static_cast<uint64_t>(lang_index) > fields.size())
    throw IndexOutOfBoundsError(
        fmt::format("array index {} out of bounds [1,{}]",
                    lang_index, fields.size()));
  return fields[static_cast<std::size_t>(lang_index - 1)];
}

const Value& HeapObject::elementAt(int64_t lang_index) const {
  if (lang_index < 1 || static_cast<uint64_t>(lang_index) > fields.size())
    throw IndexOutOfBoundsError(
        fmt::format("array index {} out of bounds [1,{}]",
                    lang_index, fields.size()));
  return fields[static_cast<std::size_t>(lang_index - 1)];
}

HeapObject* GarbageCollector::allocate(uint32_t type_id,
                                       HeapObjectKind kind,
                                       uint64_t count,
                                       Value default_val) {
  auto* obj = new HeapObject();
  obj->type_id = type_id;
  obj->kind    = kind;
  obj->fields.resize(static_cast<std::size_t>(count), default_val);
  objects_.push_back(obj);
  return obj;
}

void GarbageCollector::collect(Vm& vm) {
  mark(vm);
  sweep();
}

void GarbageCollector::mark(Vm& vm) {
  for (const Value& v : vm.globals())
    markValue(v);

  for (const CallFrame& frame : vm.call_stack()) {
    for (const Value& v : frame.arguments) markValue(v);
  }

  for (const Value& v : vm.eval_stack())
    markValue(v);

  for (const Value& v : vm.eval_stack()) {
    if (!v.isAddress()) continue;
    if (isHeapFieldAddress(v) && heapFieldObject(v))
      markObject(heapFieldObject(v));
  }
}

void GarbageCollector::markValue(const Value& v) {
  if (!v.isRef()) return;
  auto* obj = std::bit_cast<HeapObject*>(v.data);
  if (obj) markObject(obj);
}

void GarbageCollector::markObject(HeapObject* obj) {
  if (!obj || obj->marked) return;
  obj->marked = true;
  for (const Value& field : obj->fields)
    markValue(field);
}

void GarbageCollector::sweep() {
  std::vector<HeapObject*> live;
  for (HeapObject* obj : objects_) {
    if (obj->marked) {
      obj->marked = false;
      live.push_back(obj);
    } else {
      delete obj;
    }
  }
  objects_ = std::move(live);
}

}
