#pragma once

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
#include "vm/rtti.hpp"
#include "vm/vm.hpp"

namespace vm {

namespace detail {

inline void checkRef(const Value& v, const char* op) {
  if (v.data == 0) {
    throw NullReferenceError(
        std::string(op) + ": null reference dereference");
  }
}

inline std::string formatReal(double v) {
  if (std::isinf(v)) return v > 0 ? "+Infinity" : "-Infinity";
  if (std::isnan(v)) return "NaN";
  std::string s = fmt::format("{}", v);
  auto e_pos = s.find('e');
  if (e_pos != std::string::npos && e_pos + 1 < s.size() && s[e_pos + 1] == '-') {
    s = fmt::format("{:.17f}", v);
    auto dot = s.find('.');
    if (dot != std::string::npos) {
      auto last = s.find_last_not_of('0');
      s = (last > dot) ? s.substr(0, last + 1) : s.substr(0, dot + 2);
    }
    return s;
  }
  if (s.find('.') == std::string::npos && s.find('e') == std::string::npos)
    s += ".0";
  return s;
}

inline void printValue(const Value& v, const RttiTable& rtti,
                       std::vector<HeapObject*>& ancestors);

inline void printObject(HeapObject* obj, const Value& ref,
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

  if (is_array) {
    fmt::print("[ ");
    for (const Value& elem : obj->fields) {
      printValue(elem, rtti, ancestors);
      fmt::print(", ");
    }
    fmt::print("]");
  } else {
    fmt::print("{{ ");
    if (rtti.has(ref.type_id) && rtti.isRecord(ref.type_id)) {
      const auto& rec = std::get<RecordRtti>(rtti.lookup(ref.type_id));
      std::vector<std::size_t> order(rec.field_names.size());
      std::iota(order.begin(), order.end(), 0);
      std::sort(order.begin(), order.end(), [&](std::size_t a, std::size_t b) {
        return rec.field_names[a] < rec.field_names[b];
      });
      for (std::size_t idx : order) {
        const std::string& fname = rec.field_names[idx];
        if (fname.find('\'') != std::string::npos)
          fmt::print("\"{}\": ", fname);
        else
          fmt::print("{}: ", fname);
        printValue(obj->fields[idx], rtti, ancestors);
        fmt::print(", ");
      }
    } else {
      for (std::size_t i = 0; i < obj->fields.size(); ++i) {
        fmt::print("field_{}: ", i);
        printValue(obj->fields[i], rtti, ancestors);
        fmt::print(", ");
      }
    }
    fmt::print("}}");
  }

  ancestors.pop_back();
}

inline void printValue(const Value& v, const RttiTable& rtti,
                       std::vector<HeapObject*>& ancestors) {
  if (v.type_id == kIntegerTypeId) { fmt::print("{}", v.asInteger()); return; }
  if (v.type_id == kBooleanTypeId) { fmt::print("{}", v.asBoolean() ? "true" : "false"); return; }
  if (v.type_id == kRealTypeId)    { fmt::print("{}", formatReal(v.asReal())); return; }
  if (v.type_id == kNullTypeId || v.data == 0) { fmt::print("null"); return; }
  if (v.type_id == kAddressTypeId) { fmt::print("<address>"); return; }

  auto* obj = std::bit_cast<HeapObject*>(v.data);
  printObject(obj, v, rtti, ancestors);
}

}

template <Opcode kOpcode>
void OpcodeHandler<kOpcode>::execute(Vm& /*vm*/, const Instruction& /*instr*/) {
  throw RuntimeError(
      fmt::format("Unknown or unimplemented opcode: {}",
                  static_cast<int>(kOpcode)));
}

template <>
struct OpcodeHandler<Opcode::kDrop> {
  static void execute(Vm& vm, const Instruction&) { vm.pop(); }
};

template <>
struct OpcodeHandler<Opcode::kDup> {
  static void execute(Vm& vm, const Instruction&) {
    vm.push(vm.top());
  }
};

template <>
struct OpcodeHandler<Opcode::kSwap> {
  static void execute(Vm& vm, const Instruction&) {
    Value a = vm.pop();
    Value b = vm.pop();
    vm.push(a);
    vm.push(b);
  }
};

template <>
struct OpcodeHandler<Opcode::kBinOp> {
  static void execute(Vm& vm, const Instruction& instr) {
    Value rhs = vm.pop();
    Value lhs = vm.pop();

    switch (instr.subopcode) {
      case binop::kEqEq:
        if (lhs.type_id == kRealTypeId)
          vm.push(Value::makeBoolean(lhs.asReal() == rhs.asReal()));
        else
          vm.push(Value::makeBoolean(lhs.data == rhs.data));
        return;
      case binop::kEqNe:
        if (lhs.type_id == kRealTypeId)
          vm.push(Value::makeBoolean(lhs.asReal() != rhs.asReal()));
        else
          vm.push(Value::makeBoolean(lhs.data != rhs.data));
        return;

      case binop::kRealAdd:
        vm.push(Value::makeReal(lhs.asReal() + rhs.asReal())); return;
      case binop::kRealSub:
        vm.push(Value::makeReal(lhs.asReal() - rhs.asReal())); return;
      case binop::kRealMul:
        vm.push(Value::makeReal(lhs.asReal() * rhs.asReal())); return;
      case binop::kRealDiv:
        vm.push(Value::makeReal(lhs.asReal() / rhs.asReal())); return;
      case binop::kRealLe:
        vm.push(Value::makeBoolean(lhs.asReal() <= rhs.asReal())); return;
      case binop::kRealLt:
        vm.push(Value::makeBoolean(lhs.asReal() <  rhs.asReal())); return;
      case binop::kRealGt:
        vm.push(Value::makeBoolean(lhs.asReal() >  rhs.asReal())); return;
      case binop::kRealGe:
        vm.push(Value::makeBoolean(lhs.asReal() >= rhs.asReal())); return;

      case binop::kIntAdd:
        vm.push(Value::makeInteger(lhs.asInteger() + rhs.asInteger())); return;
      case binop::kIntSub:
        vm.push(Value::makeInteger(lhs.asInteger() - rhs.asInteger())); return;
      case binop::kIntMul:
        vm.push(Value::makeInteger(lhs.asInteger() * rhs.asInteger())); return;
      case binop::kIntDiv: {
        if (rhs.asInteger() == 0)
          throw DivisionByZeroError("integer division by zero");
        vm.push(Value::makeInteger(lhs.asInteger() / rhs.asInteger()));
        return;
      }
      case binop::kIntMod: {
        if (rhs.asInteger() == 0)
          throw DivisionByZeroError("integer modulo by zero");
        vm.push(Value::makeInteger(lhs.asInteger() % rhs.asInteger()));
        return;
      }
      case binop::kIntLe:
        vm.push(Value::makeBoolean(lhs.asInteger() <= rhs.asInteger())); return;
      case binop::kIntLt:
        vm.push(Value::makeBoolean(lhs.asInteger() <  rhs.asInteger())); return;
      case binop::kIntGt:
        vm.push(Value::makeBoolean(lhs.asInteger() >  rhs.asInteger())); return;
      case binop::kIntGe:
        vm.push(Value::makeBoolean(lhs.asInteger() >= rhs.asInteger())); return;

      case binop::kBoolAnd:
        vm.push(Value::makeBoolean(lhs.asBoolean() && rhs.asBoolean())); return;
      case binop::kBoolOr:
        vm.push(Value::makeBoolean(lhs.asBoolean() || rhs.asBoolean())); return;
      case binop::kBoolXor:
        vm.push(Value::makeBoolean(lhs.asBoolean() != rhs.asBoolean())); return;

      default:
        throw RuntimeError(
            fmt::format("Unknown BinOp subopcode: 0x{:02x}", instr.subopcode));
    }
  }
};

template <>
struct OpcodeHandler<Opcode::kUnOp> {
  static void execute(Vm& vm, const Instruction& instr) {
    Value v = vm.pop();
    switch (instr.subopcode) {
      case unop::kIntNeg:
        vm.push(Value::makeInteger(-v.asInteger())); return;
      case unop::kRealNeg:
        vm.push(Value::makeReal(-v.asReal())); return;
      case unop::kBoolNeg:
        vm.push(Value::makeBoolean(!v.asBoolean())); return;
      default:
        throw RuntimeError(
            fmt::format("Unknown UnOp subopcode: {}", instr.subopcode));
    }
  }
};

template <>
struct OpcodeHandler<Opcode::kIntToBool> {
  static void execute(Vm& vm, const Instruction&) {
    int64_t i = vm.pop().asInteger();
    vm.push(Value::makeBoolean(i != 0));
  }
};

template <>
struct OpcodeHandler<Opcode::kRealToInt> {
  static void execute(Vm& vm, const Instruction&) {
    double r = vm.pop().asReal();
    if (std::isnan(r))
      throw ConversionError("RealToInt: NaN cannot be converted to integer");
    double truncated = std::trunc(r);
    if (truncated >= static_cast<double>(INT64_MAX))
      vm.push(Value::makeInteger(INT64_MAX));
    else if (truncated <= static_cast<double>(INT64_MIN))
      vm.push(Value::makeInteger(INT64_MIN));
    else
      vm.push(Value::makeInteger(static_cast<int64_t>(truncated)));
  }
};

template <>
struct OpcodeHandler<Opcode::kIntToReal> {
  static void execute(Vm& vm, const Instruction&) {
    int64_t i = vm.pop().asInteger();
    vm.push(Value::makeReal(static_cast<double>(i)));
  }
};

template <>
struct OpcodeHandler<Opcode::kIntConst> {
  static void execute(Vm& vm, const Instruction& instr) {
    vm.push(Value::makeInteger(static_cast<int64_t>(instr.arg64)));
  }
};

template <>
struct OpcodeHandler<Opcode::kRealConst> {
  static void execute(Vm& vm, const Instruction& instr) {
    vm.push(Value::makeReal(std::bit_cast<double>(instr.arg64)));
  }
};

template <>
struct OpcodeHandler<Opcode::kLoad> {
  static void execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    vm.push(vm.getVariable(kind, index));
  }
};

template <>
struct OpcodeHandler<Opcode::kStore> {
  static void execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    Value v = vm.pop();
    vm.setVariable(kind, index, v);
  }
};

template <>
struct OpcodeHandler<Opcode::kAddressOf> {
  static void execute(Vm& vm, const Instruction& instr) {
    auto kind  = static_cast<LocationKind>(instr.subopcode);
    auto index = static_cast<uint16_t>(instr.arg16);
    vm.push(vm.makeVarAddress(kind, index));
  }
};

template <>
struct OpcodeHandler<Opcode::kStoreAddress> {
  static void execute(Vm& vm, const Instruction&) {
    Value val  = vm.pop();
    Value addr = vm.pop();
    vm.storeAddress(addr, val);
  }
};

template <>
struct OpcodeHandler<Opcode::kLoadAddress> {
  static void execute(Vm& vm, const Instruction&) {
    Value addr = vm.pop();
    vm.push(vm.loadAddress(addr));
  }
};

template <>
struct OpcodeHandler<Opcode::kAllocRecord> {
  static void execute(Vm& vm, const Instruction& instr) {
    HeapObject* obj =
        vm.allocRecord(instr.arg32, static_cast<uint64_t>(instr.arg64));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.push(ref);
  }
};

template <>
struct OpcodeHandler<Opcode::kAllocArray> {
  static void execute(Vm& vm, const Instruction& instr) {
    HeapObject* obj =
        vm.allocArray(instr.arg32, static_cast<uint64_t>(instr.arg64));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.push(ref);
  }
};

template <>
struct OpcodeHandler<Opcode::kArraySize> {
  static void execute(Vm& vm, const Instruction&) {
    Value ref = vm.pop();
    detail::checkRef(ref, "ArraySize");
    auto* obj = std::bit_cast<HeapObject*>(ref.data);
    vm.push(Value::makeInteger(static_cast<int64_t>(obj->size())));
  }
};

template <>
struct OpcodeHandler<Opcode::kElementAddress> {
  static void execute(Vm& vm, const Instruction&) {
    Value top     = vm.pop();
    Value ref     = top.isAddress() ? vm.loadAddress(top) : top;
    Value idx_val = vm.pop();
    detail::checkRef(ref, "ElementAddress");
    auto* obj  = std::bit_cast<HeapObject*>(ref.data);
    int64_t idx = idx_val.asInteger();
    if (idx < 1 || static_cast<uint64_t>(idx) > obj->size())
      throw IndexOutOfBoundsError(
          fmt::format("ElementAddress: index {} out of bounds [1,{}]",
                      idx, obj->size()));
    vm.push(vm.makeHeapFieldAddress(obj,
                                    static_cast<uint64_t>(idx - 1)));
  }
};

template <>
struct OpcodeHandler<Opcode::kFieldAddress> {
  static void execute(Vm& vm, const Instruction& instr) {
    Value top = vm.pop();
    Value ref = top.isAddress() ? vm.loadAddress(top) : top;
    detail::checkRef(ref, "FieldAddress");
    auto* obj = std::bit_cast<HeapObject*>(ref.data);
    uint64_t fi = instr.arg64;
    if (fi >= obj->size())
      throw IndexOutOfBoundsError(
          fmt::format("FieldAddress: field {} out of bounds [0,{})",
                      fi, obj->size()));
    vm.push(vm.makeHeapFieldAddress(obj, fi));
  }
};

template <>
struct OpcodeHandler<Opcode::kLabel> {
  static void execute(Vm&, const Instruction&) {}
};

template <>
struct OpcodeHandler<Opcode::kJump> {
  static void execute(Vm& vm, const Instruction& instr) {
    vm.jump(instr.arg64);
  }
};

template <>
struct OpcodeHandler<Opcode::kJumpCond> {
  static void execute(Vm& vm, const Instruction& instr) {
    Value cond = vm.pop();
    bool taken = false;
    if (instr.subopcode == jumpcond::kJumpZero)
      taken = !cond.asBoolean();
    else
      taken = cond.asBoolean();
    if (taken)
      vm.jump(instr.arg64);
  }
};

template <>
struct OpcodeHandler<Opcode::kCall> {
  static void execute(Vm& vm, const Instruction& instr) {
    vm.call(instr.arg64);
  }
};

template <>
struct OpcodeHandler<Opcode::kRet> {
  static void execute(Vm& vm, const Instruction&) {
    vm.ret();
  }
};

template <>
struct OpcodeHandler<Opcode::kPrint> {
  static void execute(Vm& vm, const Instruction& instr) {
    Value v = vm.pop();
    if (vm.program().rtti.isPrimitive(instr.arg32) &&
        vm.program().rtti.getPrimitiveKind(instr.arg32) == PrimitiveKind::kBoolean &&
        v.isInteger()) {
      v = Value::makeBoolean(v.asInteger() != 0);
    }
    std::vector<HeapObject*> ancestors;
    detail::printValue(v, vm.program().rtti, ancestors);
    fmt::print("\n");
  }
};

template <>
struct OpcodeHandler<Opcode::kPanic> {
  static void execute(Vm&, const Instruction& instr) {
    throw PanicError(instr.arg64);
  }
};

template <>
struct OpcodeHandler<Opcode::kNullConst> {
  static void execute(Vm& vm, const Instruction&) {
    Value v;
    v.type_id = kNullTypeId;
    v.data    = 0;
    vm.push(v);
  }
};

template <>
struct OpcodeHandler<Opcode::kDropMany> {
  static void execute(Vm& vm, const Instruction& instr) {
    uint16_t count = static_cast<uint16_t>(instr.arg16);
    for (uint16_t i = 0; i < count; ++i)
      vm.pop();
  }
};

template <>
struct OpcodeHandler<Opcode::kBoolToInt> {
  static void execute(Vm& vm, const Instruction&) {
    Value v = vm.pop();
    vm.push(Value::makeInteger(v.asBoolean() ? 1LL : 0LL));
  }
};

template <>
struct OpcodeHandler<Opcode::kAllocArrayDynamic> {
  static void execute(Vm& vm, const Instruction& instr) {
    int64_t count = vm.pop().asInteger();
    if (count < 0)
      throw RuntimeError(
          fmt::format("AllocArrayDynamic: negative count {}", count));
    HeapObject* obj =
        vm.allocArray(instr.arg32, static_cast<uint64_t>(count));
    Value ref;
    ref.type_id = instr.arg32;
    ref.data    = std::bit_cast<uint64_t>(obj);
    vm.push(ref);
  }
};

}
