#include <gtest/gtest.h>

#include <bit>

#include "vm/loader.hpp"
#include "vm/opcodes.hpp"
#include "vm/value.hpp"
#include "vm/vm.hpp"

using namespace vm;

static Instruction op(Opcode o) {
  Instruction i;
  i.opcode = static_cast<uint8_t>(o);
  return i;
}

static Instruction makeLabel(uint64_t id) {
  Instruction i = op(Opcode::kLabel);
  i.arg64 = id;
  return i;
}

static Instruction intConst(int64_t v) {
  Instruction i = op(Opcode::kIntConst);
  i.arg64 = static_cast<uint64_t>(v);
  return i;
}

static Instruction realConst(double v) {
  Instruction i = op(Opcode::kRealConst);
  i.arg64 = std::bit_cast<uint64_t>(v);
  return i;
}

static Instruction printInstr(uint32_t type_id) {
  Instruction i = op(Opcode::kPrint);
  i.arg32 = type_id;
  return i;
}

static std::string runAndCapture(std::vector<Instruction> instrs,
                                 uint32_t global_count = 0) {
  instrs.insert(instrs.begin(), makeLabel(0));
  Program prog = Loader::makeTestProgram(std::move(instrs), global_count);
  Vm vm(std::move(prog));

  testing::internal::CaptureStdout();
  vm.run();
  return testing::internal::GetCapturedStdout();
}

TEST(Print, Integer) {
  auto out = runAndCapture({intConst(42), printInstr(kIntegerTypeId)});
  EXPECT_EQ(out, "42\n");
}

TEST(Print, NegativeInteger) {
  auto out = runAndCapture({intConst(-7), printInstr(kIntegerTypeId)});
  EXPECT_EQ(out, "-7\n");
}

TEST(Print, RealWholeNumber) {
  auto out = runAndCapture({realConst(11.0), printInstr(kRealTypeId)});
  EXPECT_EQ(out, "11.0\n");
}

TEST(Print, RealFractional) {
  auto out = runAndCapture({realConst(2.75), printInstr(kRealTypeId)});
  EXPECT_EQ(out, "2.75\n");
}

TEST(Print, RealNegative) {
  auto out = runAndCapture({realConst(-5.5), printInstr(kRealTypeId)});
  EXPECT_EQ(out, "-5.5\n");
}

TEST(Print, BoolTrue) {
  auto out = runAndCapture({
      intConst(1), op(Opcode::kIntToBool), printInstr(kBooleanTypeId)});
  EXPECT_EQ(out, "true\n");
}

TEST(Print, BoolFalse) {
  auto out = runAndCapture({
      intConst(0), op(Opcode::kIntToBool), printInstr(kBooleanTypeId)});
  EXPECT_EQ(out, "false\n");
}

TEST(Print, MultiplePrints) {
  auto out = runAndCapture({
      intConst(7), intConst(3),
      op(Opcode::kBinOp),
      printInstr(kIntegerTypeId),
      intConst(7), intConst(3),
      op(Opcode::kBinOp),
      printInstr(kIntegerTypeId),
  });
  std::vector<Instruction> instrs;
  {
    Instruction add = op(Opcode::kBinOp);
    add.subopcode = binop::kIntAdd;
    Instruction sub = op(Opcode::kBinOp);
    sub.subopcode = binop::kIntSub;
    instrs = {intConst(7), intConst(3), add, printInstr(kIntegerTypeId),
              intConst(7), intConst(3), sub, printInstr(kIntegerTypeId)};
  }
  auto out2 = runAndCapture(std::move(instrs));
  EXPECT_EQ(out2, "10\n4\n");
}

TEST(Print, EmptyRecord) {
  constexpr uint32_t kTypeId = 10;

  std::vector<Instruction> instrs;
  {
    Instruction alloc = op(Opcode::kAllocRecord);
    alloc.arg32 = kTypeId;
    alloc.arg64 = 0;
    instrs = {alloc, printInstr(kTypeId)};
  }
  auto out = runAndCapture(std::move(instrs));
  EXPECT_EQ(out, "{ }\n");
}

TEST(Print, RecordWithPrimitiveFields) {
  constexpr uint32_t kTypeId = 11;

  auto mkFieldAddr = [](uint64_t idx) {
    Instruction i = op(Opcode::kFieldAddress);
    i.arg64 = idx;
    return i;
  };

  std::vector<Instruction> instrs;
  {
    Instruction alloc = op(Opcode::kAllocRecord);
    alloc.arg32 = kTypeId;
    alloc.arg64 = 3;

    instrs = {
        alloc,
        op(Opcode::kDup), mkFieldAddr(0),
        intConst(42), op(Opcode::kStoreAddress),
        op(Opcode::kDup), mkFieldAddr(1),
        realConst(3.14), op(Opcode::kStoreAddress),
        op(Opcode::kDup), mkFieldAddr(2),
        intConst(1), op(Opcode::kIntToBool), op(Opcode::kStoreAddress),
        printInstr(kTypeId),
    };
  }

  auto out = runAndCapture(std::move(instrs));
  EXPECT_EQ(out, "{ field_0: 42, field_1: 3.14, field_2: true, }\n");
}

TEST(Print, ArrayOfIntegers) {
  constexpr uint32_t kTypeId = 20;

  auto mkAlloc = []() {
    Instruction i = op(Opcode::kAllocArray);
    i.arg32 = kTypeId;
    i.arg64 = 3;
    return i;
  };

  std::vector<Instruction> instrs = {
      mkAlloc(),
      op(Opcode::kDup), intConst(1), op(Opcode::kSwap), op(Opcode::kElementAddress),
      intConst(10), op(Opcode::kStoreAddress),
      op(Opcode::kDup), intConst(2), op(Opcode::kSwap), op(Opcode::kElementAddress),
      intConst(20), op(Opcode::kStoreAddress),
      op(Opcode::kDup), intConst(3), op(Opcode::kSwap), op(Opcode::kElementAddress),
      intConst(30), op(Opcode::kStoreAddress),
      printInstr(kTypeId),
  };

  auto out = runAndCapture(std::move(instrs));
  EXPECT_EQ(out, "[ 10, 20, 30, ]\n");
}

TEST(Print, NullReference) {
  auto out = runAndCapture({op(Opcode::kNullConst), printInstr(kNullTypeId)});
  EXPECT_EQ(out, "null\n");
}
