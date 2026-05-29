#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

#include "vm/value.hpp"

namespace vm {

struct RecordRtti {
  uint32_t id;
  std::vector<std::string>  field_names;
  std::vector<uint32_t>     field_type_ids;

  uint64_t fieldCount() const { return field_type_ids.size(); }
};

struct ArrayRtti {
  uint32_t id;
  uint32_t element_type_id;
};

enum class PrimitiveKind { kInteger, kReal, kBoolean };

struct PrimitiveRtti {
  uint32_t id;
  PrimitiveKind kind;
};

using RttiEntry = std::variant<PrimitiveRtti, RecordRtti, ArrayRtti>;

class RttiTable {
 public:
  void registerEntry(RttiEntry entry);

  bool has(uint32_t type_id) const;

  const RttiEntry& lookup(uint32_t type_id) const;

  bool isPrimitive(uint32_t type_id) const;
  bool isRecord(uint32_t type_id) const;
  bool isArray(uint32_t type_id) const;

  PrimitiveKind getPrimitiveKind(uint32_t type_id) const;

  void registerBuiltinPrimitives();

 private:
  std::unordered_map<uint32_t, RttiEntry> entries_;
};

}
