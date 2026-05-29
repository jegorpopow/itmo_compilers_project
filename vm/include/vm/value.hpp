#pragma once

#include <bit>
#include <cstdint>
#include <string>

namespace vm {

inline constexpr uint32_t kIntegerTypeId = 0;
inline constexpr uint32_t kBooleanTypeId = 1;
inline constexpr uint32_t kRealTypeId    = 2;
inline constexpr uint32_t kNullTypeId    = 3;

inline constexpr uint32_t kAddressTypeId = 0xFFFF'FFFFu;

inline constexpr uint32_t kVoidTypeId = 0xFFFF'FFFEu;

struct Value {
  uint32_t type_id = kIntegerTypeId;
  uint32_t aux     = 0;
  uint64_t data    = 0;

  static Value makeInteger(int64_t v) noexcept {
    return {.type_id = kIntegerTypeId, .data = static_cast<uint64_t>(v)};
  }
  static Value makeReal(double v) noexcept {
    return {.type_id = kRealTypeId, .data = std::bit_cast<uint64_t>(v)};
  }
  static Value makeBoolean(bool v) noexcept {
    return {.type_id = kBooleanTypeId, .data = v ? 1ULL : 0ULL};
  }

  int64_t asInteger() const noexcept {
    return static_cast<int64_t>(data);
  }
  double asReal() const noexcept {
    return std::bit_cast<double>(data);
  }
  bool asBoolean() const noexcept {
    return data != 0;
  }

  bool isInteger() const noexcept { return type_id == kIntegerTypeId; }
  bool isReal()    const noexcept { return type_id == kRealTypeId; }
  bool isBoolean() const noexcept { return type_id == kBooleanTypeId; }
  bool isNull()    const noexcept { return type_id == kNullTypeId; }
  bool isAddress() const noexcept { return type_id == kAddressTypeId; }
  bool isRef()     const noexcept {
    return type_id != kIntegerTypeId && type_id != kBooleanTypeId &&
           type_id != kRealTypeId    && type_id != kNullTypeId &&
           type_id != kAddressTypeId && type_id != kVoidTypeId;
  }

  std::string typeName() const;
};

}
