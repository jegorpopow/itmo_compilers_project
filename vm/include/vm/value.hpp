#pragma once

#include <bit>
#include <cstdint>
#include <string>

namespace vm {

// Well-known TypeIds for primitive types.
// These must match what the compiler assigns in the RTTI table.
inline constexpr uint32_t kIntegerTypeId = 0;
inline constexpr uint32_t kRealTypeId    = 1;
inline constexpr uint32_t kBooleanTypeId = 2;

// Internal TypeId for address values — never appears in user RTTI.
inline constexpr uint32_t kAddressTypeId = 0xFFFF'FFFFu;

// TypeId for void return (functions with no return type).
inline constexpr uint32_t kVoidTypeId = 0xFFFF'FFFEu;

// A tagged value on the evaluation stack.
//
// Layout:
//   type_id == kIntegerTypeId  => data is int64_t (bit-cast)
//   type_id == kRealTypeId     => data is double  (bit-cast)
//   type_id == kBooleanTypeId  => data is 0 (false) or 1 (true)
//   type_id == kAddressTypeId  => data is AddressDescriptor* (bit-cast)
//   type_id >= 3 (user type)   => data is HeapObject* (bit-cast)
struct Value {
  uint32_t type_id = kIntegerTypeId;
  uint64_t data    = 0;

  // Factories.
  static Value MakeInteger(int64_t v) noexcept {
    return {kIntegerTypeId, static_cast<uint64_t>(v)};
  }
  static Value MakeReal(double v) noexcept {
    return {kRealTypeId, std::bit_cast<uint64_t>(v)};
  }
  static Value MakeBoolean(bool v) noexcept {
    return {kBooleanTypeId, v ? 1ULL : 0ULL};
  }

  // Accessors — callers must guarantee the correct type_id.
  int64_t AsInteger() const noexcept {
    return static_cast<int64_t>(data);
  }
  double AsReal() const noexcept {
    return std::bit_cast<double>(data);
  }
  bool AsBoolean() const noexcept {
    return data != 0;
  }

  // Type predicates.
  bool IsInteger() const noexcept { return type_id == kIntegerTypeId; }
  bool IsReal()    const noexcept { return type_id == kRealTypeId; }
  bool IsBoolean() const noexcept { return type_id == kBooleanTypeId; }
  bool IsAddress() const noexcept { return type_id == kAddressTypeId; }
  bool IsRef()     const noexcept {
    return type_id != kIntegerTypeId && type_id != kRealTypeId &&
           type_id != kBooleanTypeId && type_id != kAddressTypeId &&
           type_id != kVoidTypeId;
  }

  std::string TypeName() const;
};

}  // namespace vm
