#pragma once

#include <bit>
#include <cstdint>

#include "vm/opcodes.hpp"
#include "vm/value.hpp"

namespace vm {

struct HeapObject;

inline constexpr uint32_t kHeapFieldAddrFlag = 1u;

inline Value makeVariableAddressValue(LocationKind kind, uint16_t index) {
  Value v;
  v.type_id = kAddressTypeId;
  v.data    = (static_cast<uint64_t>(kind) << 16) | index;
  v.aux     = 0;
  return v;
}

inline Value makeHeapFieldAddressValue(HeapObject* obj, uint64_t field_idx) {
  Value v;
  v.type_id = kAddressTypeId;
  v.data    = std::bit_cast<uint64_t>(obj);
  v.aux     = static_cast<uint32_t>((field_idx << 1) | kHeapFieldAddrFlag);
  return v;
}

inline bool isHeapFieldAddress(const Value& v) {
  return (v.aux & kHeapFieldAddrFlag) != 0;
}

inline HeapObject* heapFieldObject(const Value& v) {
  return std::bit_cast<HeapObject*>(v.data);
}

inline uint64_t heapFieldIndex(const Value& v) {
  return v.aux >> 1;
}

inline LocationKind variableAddressKind(const Value& v) {
  return static_cast<LocationKind>((v.data >> 16) & 0xFF);
}

inline uint16_t variableAddressIndex(const Value& v) {
  return static_cast<uint16_t>(v.data & 0xFFFF);
}

}
