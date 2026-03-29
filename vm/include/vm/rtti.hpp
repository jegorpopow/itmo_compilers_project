#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

#include "vm/value.hpp"

namespace vm {

// Runtime type info for a record type.
struct RecordRtti {
  uint32_t id;
  std::vector<uint32_t> field_type_ids;  // type_id of each field, in order

  uint64_t FieldCount() const { return field_type_ids.size(); }
};

// Runtime type info for an array type.
struct ArrayRtti {
  uint32_t id;
  uint32_t element_type_id;
};

// Which of the three primitive types this entry represents.
enum class PrimitiveKind { kInteger, kReal, kBoolean };

// Runtime type info for a primitive type.
struct PrimitiveRtti {
  uint32_t id;
  PrimitiveKind kind;
};

// A single entry in the RTTI table.
using RttiEntry = std::variant<PrimitiveRtti, RecordRtti, ArrayRtti>;

// The complete RTTI table for a program.
// Maps TypeId -> RttiEntry.
class RttiTable {
 public:
  void Register(RttiEntry entry);

  bool Has(uint32_t type_id) const;

  const RttiEntry& Lookup(uint32_t type_id) const;

  bool IsPrimitive(uint32_t type_id) const;
  bool IsRecord(uint32_t type_id) const;
  bool IsArray(uint32_t type_id) const;

  // Returns the PrimitiveKind for a primitive TypeId.
  // Throws if not a primitive.
  PrimitiveKind GetPrimitiveKind(uint32_t type_id) const;

  // Populates the table with the three built-in primitive types
  // using the well-known TypeIds (kIntegerTypeId, kRealTypeId, kBooleanTypeId).
  void RegisterBuiltinPrimitives();

 private:
  std::unordered_map<uint32_t, RttiEntry> entries_;
};

}  // namespace vm
