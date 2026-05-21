#pragma once

// All OpcodeHandler specialisations.
// This header must be included AFTER vm.hpp so that Vm is fully defined.
// It is included only from vm.cpp to avoid multiple-definition issues.

#include <algorithm>
#include <bit>
#include <cmath>
#include <numeric>
#include <string>

#include <fmt/format.h>

#include "vm/error.hpp"
#include "vm/heap.hpp"
#include "vm/interpreter.hpp"
#include "vm/opcodes.hpp"
#include "vm/vm.hpp"

namespace vm {

// ============================================================
//  Helpers
// ============================================================

namespace detail {

inline void CheckRef(const Value& v, const char* op) {
  if (v.data == 0) {
    throw NullReferenceError(
        std::string(op) + ": null reference dereference");
  }
}

inline std::string FormatReal(double v) {
  if (std::isinf(v)) return v > 0 ? "+Infinity" : "-Infinity";
  if (std::isnan(v)) return "NaN";
  std::string s = fmt::format("{}", v);
  if (s.find('.') == std::string::npos && s.find('e') == std::string::npos)
    s += ".0";
  return s;
}

inline void PrintValue(const Value& v, const RttiTable& rtti,
                       std::vector<HeapObject*>& ancestors);

inline void PrintObject(HeapObject* obj, const Value& ref,
                        const RttiTable& rtti,
                        std::vector<HeapObject*>& ancestors) {
  for (std::size_t d = 0; d < ancestors.size(); ++d) {
    if (ancestors[d] == obj) {
      std::size_t depth = ancestors.size() - d;
      if (depth == 1)
        fmt::print("/* repeated 1 level above */");
      else
        fmt::print("/* repeated {} levels above */", depth);
      return;
    }
  }

  ancestors.push_back(obj);

  bool is_array = (obj->kind == HeapObjectKind::kArray);
  if (rtti.Has(ref.type_id))
    is_array = rtti.IsArray(ref.type_id);

  if (is_array) {
    fmt::print("[ ");
    for (const Value& elem : obj->fields) {
      PrintValue(elem, rtti, ancestors);
      fmt::print(", ");
    }
    fmt::print("]");
  } else {
    fmt::print("{{ ");
    if (rtti.Has(ref.type_id) && rtti.IsRecord(ref.type_id)) {
      const auto& rec = std::get<RecordRtti>(rtti.Lookup(ref.type_id));
      std::vector<std::size_t> order(rec.field_names.size());
      std::iota(order.begin(), order.end(), 0);
      std::sort(order.begin(), order.end(), [&](std::size_t a, std::size_t b) {
        return rec.field_names[a] < rec.field_names[b];
      });
      for (std::size_t idx : order) {
        fmt::print("{}: ", rec.field_names[idx]);
        PrintValue(obj->fields[idx], rtti, ancestors);
        fmt::print(", ");
      }
    } else {
      for (std::size_t i = 0; i < obj->fields.size(); ++i) {
        fmt::print("field_{}: ", i);
        PrintValue(obj->fields[i], rtti, ancestors);
        fmt::print(", ");
      }
    }
    fmt::print("}}");
  }

  ancestors.pop_back();
}

inline void PrintValue(const Value& v, const RttiTable& rtti,
                       std::vector<HeapObject*>& ancestors) {
  if (v.type_id == kIntegerTypeId) { fmt::print("{}", v.AsInteger()); return; }
  if (v.type_id == kBooleanTypeId) { fmt::print("{}", v.AsBoolean() ? "true" : "false"); return; }
  if (v.type_id == kRealTypeId)    { fmt::print("{}", FormatReal(v.AsReal())); return; }
  if (v.type_id == kNullTypeId || v.data == 0) { fmt::print("null"); return; }
  if (v.type_id == kAddressTypeId) { fmt::print("<address>"); return; }

  auto* obj = std::bit_cast<HeapObject*>(v.data);
  PrintObject(obj, v, rtti, ancestors);
}

}  // namespace detail

// ============================================================
//  Opcode 0 — unused / unknown (primary template body)
// ============================================================

template <uint8_t kOpcode>
void OpcodeHandler<kOpcode>::Execute(Vm& /*vm*/, const Instruction& /*instr*/) {
  throw RuntimeError(
      fmt::format("Unknown or unimplemented opcode: {}", kOpcode));
}

// ============================================================
//  Stack manipulation
// ============================================================

// 1: Drop
template <>
struct OpcodeHandler<1> {
  static void Execute(Vm& vm, const Instruction&) { vm.Pop(); }
};

// 2: Dup
template <>
struct OpcodeHandler<2> {
  static void Execute(Vm& vm, const Instruction&) {
    vm.Push(vm.Top());
  }
};

// 3: Swap
template <>
struct OpcodeHandler<3> {
  static void Execute(Vm& vm, const Instruction&) {
    Value a = vm.Pop();
    Value b = vm.Pop();
    vm.Push(a);
    vm.Push(b);
  }
};

// ============================================================
//  Binary operations  (opcode 4)
// ============================================================

template <>
struct OpcodeHandler<4> {
  static void Execute(Vm& vm, const Instruction& instr) {
    Value rhs = vm.Pop();
    Value lhs = vm.Pop();

    switch (instr.subopcode) {
      // --- Equality ---
      case binop::kEqEq:
        if (lhs.type_id == kRealTypeId)
          vm.Push(Value::MakeBoolean(lhs.AsReal() == rhs.AsReal()));
        else
          vm.Push(Value::MakeBoolean(lhs.data == rhs.data));
        return;
      case binop::kEqNe:
        if (lhs.type_id == kRealTypeId)
          vm.Push(Value::MakeBoolean(lhs.AsReal() != rhs.AsReal()));
        else
          vm.Push(Value::MakeBoolean(lhs.data != rhs.data));
        return;

      // --- Real arithmetic ---
      case binop::kRealAdd:
        vm.Push(Value::MakeReal(lhs.AsReal() + rhs.AsReal())); return;
      case binop::kRealSub:
        vm.Push(Value::MakeReal(lhs.AsReal() - rhs.AsReal())); return;
      case binop::kRealMul:
        vm.Push(Value::MakeReal(lhs.AsReal() * rhs.AsReal())); return;
      case binop::kRealDiv:
        vm.Push(Value::MakeReal(lhs.AsReal() / rhs.AsReal())); return;
      case binop::kRealLe:
        vm.Push(Value::MakeBoolean(lhs.AsReal() <= rhs.AsReal())); return;
      case binop::kRealLt:
        vm.Push(Value::MakeBoolean(lhs.AsReal() <  rhs.AsReal())); return;
      case binop::kRealGt:
        vm.Push(Value::MakeBoolean(lhs.AsReal() >  rhs.AsReal())); return;
      case binop::kRealGe:
        vm.Push(Value::MakeBoolean(lhs.AsReal() >= rhs.AsReal())); return;

      // --- Integer arithmetic ---
      case binop::kIntAdd:
        vm.Push(Value::MakeInteger(lhs.AsInteger() + rhs.AsInteger())); return;
      case binop::kIntSub:
        vm.Push(Value::MakeInteger(lhs.AsInteger() - rhs.AsInteger())); return;
      case binop::kIntMul:
        vm.Push(Value::MakeInteger(lhs.AsInteger() * rhs.AsInteger())); return;
      case binop::kIntDiv: {
        if (rhs.AsInteger() == 0)
          throw DivisionByZeroError("integer division by zero");
        vm.Push(Value::MakeInteger(lhs.AsInteger() / rhs.AsInteger()));
        return;
      }
      case binop::kIntMod: {
        if (rhs.AsInteger() == 0)
          throw DivisionByZeroError("integer modulo by zero");
        vm.Push(Value::MakeInteger(lhs.AsInteger() % rhs.AsInteger()));
        return;
      }
      case binop::kIntLe:
        vm.Push(Value::MakeBoolean(lhs.AsInteger() <= rhs.AsInteger())); return;
      case binop::kIntLt:
        vm.Push(Value::MakeBoolean(lhs.AsInteger() <  rhs.AsInteger())); return;
      case binop::kIntGt:
        vm.Push(Value::MakeBoolean(lhs.AsInteger() >  rhs.AsInteger())); return;
      case binop::kIntGe:
        vm.Push(Value::MakeBoolean(lhs.AsInteger() >= rhs.AsInteger())); return;

      // --- Boolean logic ---
      case binop::kBoolAnd:
        vm.Push(Value::MakeBoolean(lhs.AsBoolean() && rhs.AsBoolean())); return;
      case binop::kBoolOr:
        vm.Push(Value::MakeBoolean(lhs.AsBoolean() || rhs.AsBoolean())); return;
      case binop::kBoolXor:
        vm.Push(Value::MakeBoolean(lhs.AsBoolean() != rhs.AsBoolean())); return;

      default:
        throw RuntimeError(
            fmt::format("Unknown BinOp subopcode: 0x{:02x}", instr.subopcode));
    }
  }
};

// ============================================================
//  Unary operations  (opcode 5)
// ============================================================

template <>
struct OpcodeHandler<5> {
  static void Execute(Vm& vm, const Instruction& instr) {
    Value v = vm.Pop();
    switch (instr.subopcode) {
      case unop::kIntNeg:
        vm.Push(Value::MakeInteger(-v.AsInteger())); return;
      case unop::kRealNeg:
        vm.Push(Value::MakeReal(-v.AsReal())); return;
      case unop::kBoolNeg:
        vm.Push(Value::MakeBoolean(!v.AsBoolean())); return;
      default:
        throw RuntimeError(
            fmt::format("Unknown UnOp subopcode: {}", instr.subopcode));
    }
  }
};

// ============================================================
//  Type conversions  (opcodes 6-8)
// ============================================================

// 6: IntToBool
template <>
struct OpcodeHandler<6> {
  static void Execute(Vm& vm, const Instruction&) {
    int64_t i = vm.Pop().AsInteger();
    if (i == 0) { vm.Push(Value::MakeBoolean(false)); return; }
    if (i == 1) { vm.Push(Value::MakeBoolean(true));  return; }
    throw ConversionError(
        fmt::format("IntToBool: integer {} is not 0 or 1", i));
  }
};

// 7: RealToInt — round to nearest integer
template <>
struct OpcodeHandler<7> {
  static void Execute(Vm& vm, const Instruction&) {
    double r = vm.Pop().AsReal();
    if (std::isnan(r))
      throw ConversionError("RealToInt: NaN cannot be converted to integer");
    double rounded = std::round(r);
    // Clamp ±INF to int64 limits (implementation-defined per spec).
    if (rounded >= static_cast<double>(INT64_MAX))
      vm.Push(Value::MakeInteger(INT64_MAX));
    else if (rounded <= static_cast<double>(INT64_MIN))
      vm.Push(Value::MakeInteger(INT64_MIN));
    else
      vm.Push(Value::MakeInteger(static_cast<int64_t>(rounded)));
  }
};

// 8: IntToReal
template <>
struct OpcodeHandler<8> {
  static void Execute(Vm& vm, const Instruction&) {
    int64_t i = vm.Pop().AsInteger();
    vm.Push(Value::MakeReal(static_cast<double>(i)));
  }
};

// ============================================================
//  Constants  (opcodes 9-10)
// ============================================================

// 9: IntConst
template <>
struct OpcodeHandler<9> {
  static void Execute(Vm& vm, const Instruction& instr) {
    vm.Push(Value::MakeInteger(static_cast<int64_t>(instr.arg64)));
  }
};

// 10: RealConst — arg64 stores the double's bit pattern
template <>
struct OpcodeHandler<10> {
  static void Execute(Vm& vm, const Instruction& instr) {
    vm.Push(Value::MakeReal(std::bit_cast<double>(instr.arg64)));
  }
};

// ============================================================
//  Variable access  (opcodes 11-13)
// ============================================================

// 11: Load
template <>
struct OpcodeHandler<11> {
  static void Execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    vm.Push(vm.GetVariable(kind, index));
  }
};

// 12: Store
template <>
struct OpcodeHandler<12> {
  static void Execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    Value v = vm.Pop();
    vm.SetVariable(kind, index, v);
  }
};

// 13: AddressOf
template <>
struct OpcodeHandler<13> {
  static void Execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    vm.Push(vm.MakeVarAddress(kind, index));
  }
};

// ============================================================
//  Indirect access  (opcodes 14-15)
// ============================================================

// 14: StoreAddress — pop value, pop address, write value to *address
template <>
struct OpcodeHandler<14> {
  static void Execute(Vm& vm, const Instruction&) {
    Value val  = vm.Pop();
    Value addr = vm.Pop();
    vm.StoreAddress(addr, val);
  }
};

// 15: LoadAddress — pop address, push value at *address
template <>
struct OpcodeHandler<15> {
  static void Execute(Vm& vm, const Instruction&) {
    Value addr = vm.Pop();
    vm.Push(vm.LoadAddress(addr));
  }
};

// ============================================================
//  Heap allocation  (opcodes 16-17)
// ============================================================

// 16: AllocRecord — arg32=type_id, arg64=num_fields
template <>
struct OpcodeHandler<16> {
  static void Execute(Vm& vm, const Instruction& instr) {
    HeapObject* obj =
        vm.AllocRecord(instr.arg32, static_cast<uint64_t>(instr.arg64));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.Push(ref);
  }
};

// 17: AllocArray — arg32=type_id, arg64=num_elements
template <>
struct OpcodeHandler<17> {
  static void Execute(Vm& vm, const Instruction& instr) {
    HeapObject* obj =
        vm.AllocArray(instr.arg32, static_cast<uint64_t>(instr.arg64));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.Push(ref);
  }
};

// ============================================================
//  Array / record field operations  (opcodes 18-20)
// ============================================================

// 18: ArraySize — pop array ref, push its element count
template <>
struct OpcodeHandler<18> {
  static void Execute(Vm& vm, const Instruction&) {
    Value ref = vm.Pop();
    detail::CheckRef(ref, "ArraySize");
    auto* obj = std::bit_cast<HeapObject*>(ref.data);
    vm.Push(Value::MakeInteger(static_cast<int64_t>(obj->Size())));
  }
};

// 19: ElementAddress — pop index (int, 1-based), pop array ref,
//                      push address of array[index-1]
template <>
struct OpcodeHandler<19> {
  static void Execute(Vm& vm, const Instruction&) {
    Value idx_val = vm.Pop();
    Value ref     = vm.Pop();
    detail::CheckRef(ref, "ElementAddress");
    auto* obj  = std::bit_cast<HeapObject*>(ref.data);
    int64_t idx = idx_val.AsInteger();
    if (idx < 1 || static_cast<uint64_t>(idx) > obj->Size())
      throw IndexOutOfBoundsError(
          fmt::format("ElementAddress: index {} out of bounds [1,{}]",
                      idx, obj->Size()));
    vm.Push(vm.MakeHeapFieldAddress(obj,
                                    static_cast<uint64_t>(idx - 1)));
  }
};

// 20: FieldAddress — arg64=field_index (0-based), pop record ref,
//                    push address of record.fields[field_index]
template <>
struct OpcodeHandler<20> {
  static void Execute(Vm& vm, const Instruction& instr) {
    Value ref = vm.Pop();
    detail::CheckRef(ref, "FieldAddress");
    auto* obj = std::bit_cast<HeapObject*>(ref.data);
    uint64_t fi = instr.arg64;
    if (fi >= obj->Size())
      throw IndexOutOfBoundsError(
          fmt::format("FieldAddress: field {} out of bounds [0,{})",
                      fi, obj->Size()));
    vm.Push(vm.MakeHeapFieldAddress(obj, fi));
  }
};

// ============================================================
//  Labels and jumps  (opcodes 21-23)
// ============================================================

// 21: Label — no-op at runtime (labels are resolved at load time)
template <>
struct OpcodeHandler<21> {
  static void Execute(Vm&, const Instruction&) { /* no-op */ }
};

// 22: Jump — unconditional branch
template <>
struct OpcodeHandler<22> {
  static void Execute(Vm& vm, const Instruction& instr) {
    vm.Jump(instr.arg64);
  }
};

// 23: JumpCond — conditional branch
//   subopcode 0 = JumpZero    (jump if top == false / 0)
//   subopcode 1 = JumpNotZero (jump if top == true  / nonzero)
template <>
struct OpcodeHandler<23> {
  static void Execute(Vm& vm, const Instruction& instr) {
    Value cond = vm.Pop();
    bool taken = false;
    if (instr.subopcode == jumpcond::kJumpZero)
      taken = !cond.AsBoolean();
    else
      taken = cond.AsBoolean();
    if (taken)
      vm.Jump(instr.arg64);
  }
};

// ============================================================
//  Function call / return  (opcodes 24-25)
// ============================================================

// 24: Call
template <>
struct OpcodeHandler<24> {
  static void Execute(Vm& vm, const Instruction& instr) {
    vm.Call(instr.arg64);
  }
};

// 25: Ret
template <>
struct OpcodeHandler<25> {
  static void Execute(Vm& vm, const Instruction&) {
    vm.Return();
  }
};

// ============================================================
//  I/O and termination  (opcodes 26-27)
// ============================================================

// 26: Print — arg32 = type_id of value to print
template <>
struct OpcodeHandler<26> {
  static void Execute(Vm& vm, const Instruction&) {
    Value v = vm.Pop();
    std::vector<HeapObject*> ancestors;
    detail::PrintValue(v, vm.program().rtti, ancestors);
    fmt::print("\n");
  }
};

// 27: Panic
template <>
struct OpcodeHandler<27> {
  static void Execute(Vm&, const Instruction& instr) {
    throw PanicError(instr.arg64);
  }
};

// 28: NullConst — push a null value
template <>
struct OpcodeHandler<28> {
  static void Execute(Vm& vm, const Instruction&) {
    Value v;
    v.type_id = kNullTypeId;
    v.data    = 0;
    vm.Push(v);
  }
};

// 29: DropMany — drop arg16 values from the eval stack
template <>
struct OpcodeHandler<29> {
  static void Execute(Vm& vm, const Instruction& instr) {
    uint16_t count = static_cast<uint16_t>(instr.arg16);
    for (uint16_t i = 0; i < count; ++i)
      vm.Pop();
  }
};

// 30: AllocArrayDynamic — pop count (int) from stack, alloc array of that size
template <>
struct OpcodeHandler<30> {
  static void Execute(Vm& vm, const Instruction& instr) {
    int64_t count = vm.Pop().AsInteger();
    if (count < 0)
      throw RuntimeError(
          fmt::format("AllocArrayDynamic: negative count {}", count));
    HeapObject* obj =
        vm.AllocArray(instr.arg32, static_cast<uint64_t>(count));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.Push(ref);
  }
};

}  // namespace vm
