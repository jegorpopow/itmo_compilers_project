#pragma once

#include <cstdint>
#include <vector>

#include "vm/value.hpp"

namespace vm {

class Vm;

enum class HeapObjectKind : uint8_t { kRecord, kArray };

struct HeapObject {
  uint32_t      type_id;
  HeapObjectKind kind;
  std::vector<Value> fields;
  bool marked = false;

  uint64_t size() const noexcept { return fields.size(); }

  Value& elementAt(int64_t lang_index);
  const Value& elementAt(int64_t lang_index) const;
};

class GarbageCollector {
 public:
  HeapObject* allocate(uint32_t type_id, HeapObjectKind kind, uint64_t count,
                       Value default_val = Value{.type_id = kNullTypeId});

  void collect(Vm& vm);

  std::size_t objectCount() const { return objects_.size(); }

 private:
  void mark(Vm& vm);
  void markValue(const Value& v);
  void markObject(HeapObject* obj);
  void sweep();

  std::vector<HeapObject*> objects_;
};

}
