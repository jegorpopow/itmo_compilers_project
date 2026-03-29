#pragma once

#include <cstdint>
#include <vector>
#include <memory>

#include "vm/opcodes.hpp"

namespace vm {

struct HeapObject;
struct Value;

// Describes the target of an address value on the evaluation stack.
// Used by AddressOf, StoreAddress, LoadAddress, ElementAddress, FieldAddress.
struct AddressDescriptor {
  enum class Kind : uint8_t {
    kVariable,  // global / local / argument
    kHeapField, // field or element inside a HeapObject
  };

  Kind kind;

  // For kVariable:
  LocationKind loc_kind;
  uint16_t     var_index;

  // For kHeapField:
  HeapObject* object    = nullptr;
  uint64_t    field_idx = 0;

  // Factory helpers.
  static AddressDescriptor Variable(LocationKind lk, uint16_t idx) {
    AddressDescriptor d;
    d.kind     = Kind::kVariable;
    d.loc_kind = lk;
    d.var_index = idx;
    return d;
  }

  static AddressDescriptor HeapField(HeapObject* obj, uint64_t idx) {
    AddressDescriptor d;
    d.kind      = Kind::kHeapField;
    d.object    = obj;
    d.field_idx = idx;
    return d;
  }
};

// Append-only pool that owns AddressDescriptor objects.
// Lifetime: tied to the VM execution (never freed until VM is destroyed).
// This is intentional — addresses are small and programs are short-lived.
class AddressPool {
 public:
  // Allocates a descriptor and returns a stable pointer.
  AddressDescriptor* Intern(AddressDescriptor desc) {
    storage_.push_back(std::make_unique<AddressDescriptor>(std::move(desc)));
    return storage_.back().get();
  }

  void Clear() { storage_.clear(); }

 private:
  std::vector<std::unique_ptr<AddressDescriptor>> storage_;
};

}  // namespace vm
