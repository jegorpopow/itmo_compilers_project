#pragma once

#include <cstdint>

namespace vm {

// Opcode constants matching the compiler's bytecode encoding.
// See compiler/compiler/src/bytecode.rs — Instruction::encode().
enum class Opcode : uint8_t {
  kDrop          = 1,
  kDup           = 2,
  kSwap          = 3,
  kBinOp         = 4,
  kUnOp          = 5,
  kIntToBool     = 6,
  kRealToInt     = 7,
  kIntToReal     = 8,
  kIntConst      = 9,
  kRealConst     = 10,
  kLoad          = 11,
  kStore         = 12,
  kAddressOf     = 13,
  kStoreAddress  = 14,
  kLoadAddress   = 15,
  kAllocRecord   = 16,
  kAllocArray    = 17,
  kArraySize     = 18,
  kElementAddress = 19,
  kFieldAddress  = 20,
  kLabel         = 21,
  kJump          = 22,
  kJumpCond      = 23,  // subopcode: 0=JumpZero, 1=JumpNotZero
  kCall          = 24,
  kRet           = 25,
  kPrint         = 26,
  kPanic         = 27,
};

// Location kind for Load/Store/AddressOf subopcode.
// Matches Location::encode() in bytecode.rs.
enum class LocationKind : uint8_t {
  kGlobal   = 0,
  kLocal    = 1,
  kArgument = 2,
};

// Subopcode values for BinOp (SemanticBinaryOperator::encode()).
namespace binop {
// Equality
inline constexpr uint8_t kEqEq = 0x00;
inline constexpr uint8_t kEqNe = 0x01;
// Real
inline constexpr uint8_t kRealLe  = 0x10;
inline constexpr uint8_t kRealLt  = 0x11;
inline constexpr uint8_t kRealGt  = 0x12;
inline constexpr uint8_t kRealGe  = 0x13;
inline constexpr uint8_t kRealAdd = 0x14;
inline constexpr uint8_t kRealSub = 0x15;
inline constexpr uint8_t kRealMul = 0x16;
inline constexpr uint8_t kRealDiv = 0x17;
// Integer
inline constexpr uint8_t kIntLe  = 0x20;
inline constexpr uint8_t kIntLt  = 0x21;
inline constexpr uint8_t kIntGt  = 0x22;
inline constexpr uint8_t kIntGe  = 0x23;
inline constexpr uint8_t kIntAdd = 0x24;
inline constexpr uint8_t kIntSub = 0x25;
inline constexpr uint8_t kIntMul = 0x26;
inline constexpr uint8_t kIntDiv = 0x27;
inline constexpr uint8_t kIntMod = 0x28;
// Boolean
inline constexpr uint8_t kBoolAnd = 0x30;
inline constexpr uint8_t kBoolOr  = 0x31;
inline constexpr uint8_t kBoolXor = 0x32;
}  // namespace binop

// Subopcode values for UnOp (SemanticUnaryOperator as u8, repr(u8)).
namespace unop {
inline constexpr uint8_t kIntNeg  = 25;
inline constexpr uint8_t kRealNeg = 26;
inline constexpr uint8_t kBoolNeg = 27;
}  // namespace unop

// Subopcode values for JumpCond.
namespace jumpcond {
inline constexpr uint8_t kJumpZero    = 0;
inline constexpr uint8_t kJumpNotZero = 1;
}  // namespace jumpcond

}  // namespace vm
