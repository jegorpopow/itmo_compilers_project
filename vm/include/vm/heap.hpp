#pragma once

#include <cstdint>
#include <vector>

#include "vm/value.hpp"

namespace vm {

class Vm;

// Discriminates between records and arrays at runtime.
enum class HeapObjectKind : uint8_t { kRecord, kArray };

// A heap-allocated object (record or array).
// Fields vector stores:
//   - for records: field values indexed by field index (0-based)
//   - for arrays:  element values indexed by element index (0-based)
//                  (language arrays are 1-based; subtract 1 when indexing)
struct HeapObject {
  uint32_t      type_id;
  HeapObjectKind kind;
  std::vector<Value> fields;  // fields or elements
  bool marked = false;        // GC mark bit

  // Convenience: number of fields/elements.
  uint64_t Size() const noexcept { return fields.size(); }

  // For arrays: language index is 1-based.
  Value& ElementAt(int64_t lang_index);
  const Value& ElementAt(int64_t lang_index) const;
};

// Mark-Sweep garbage collector.
// Owns all heap objects; performs collection when requested.
class GarbageCollector {
 public:
  // Allocates a new HeapObject with `count` default-initialised slots.
  HeapObject* Allocate(uint32_t type_id, HeapObjectKind kind, uint64_t count);

  // Triggers a full mark-sweep collection.
  // `vm` provides the GC roots (globals + call stack + eval stack).
  void Collect(Vm& vm);

  // Current number of live objects (after last collection or since start).
  std::size_t ObjectCount() const { return objects_.size(); }

 private:
  void Mark(Vm& vm);
  void MarkValue(const Value& v);
  void MarkObject(HeapObject* obj);
  void Sweep();

  std::vector<HeapObject*> objects_;  // all allocated objects
};

}  // namespace vm
