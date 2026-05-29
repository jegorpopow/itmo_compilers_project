#include <gtest/gtest.h>

#include <bit>

#include "vm/error.hpp"
#include "vm/loader.hpp"
#include "vm/opcodes.hpp"
#include "vm/vm.hpp"

using namespace vm;

static Instruction op(Opcode o) {
  Instruction i;
  i.opcode = static_cast<uint8_t>(o);
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

static Instruction binOp(uint8_t sub) {
  Instruction i = op(Opcode::kBinOp);
  i.subopcode = sub;
  return i;
}

static Instruction unaryOp(uint8_t sub) {
  Instruction i = op(Opcode::kUnOp);
  i.subopcode = sub;
  return i;
}

static Value runAndPeek(std::vector<Instruction> instrs) {
  Instruction lbl = op(Opcode::kLabel);
  lbl.arg64 = 0;
  instrs.insert(instrs.begin(), lbl);

  Program prog = Loader::makeTestProgram(std::move(instrs));
  Vm vm(std::move(prog));
  vm.run();
  return vm.eval_stack().back();
}

TEST(Arithmetic, IntAdd) {
  auto v = runAndPeek({intConst(7), intConst(3), binOp(binop::kIntAdd)});
  EXPECT_EQ(v.type_id, kIntegerTypeId);
  EXPECT_EQ(v.asInteger(), 10);
}

TEST(Arithmetic, IntSub) {
  auto v = runAndPeek({intConst(7), intConst(3), binOp(binop::kIntSub)});
  EXPECT_EQ(v.asInteger(), 4);
}

TEST(Arithmetic, IntMul) {
  auto v = runAndPeek({intConst(7), intConst(3), binOp(binop::kIntMul)});
  EXPECT_EQ(v.asInteger(), 21);
}

TEST(Arithmetic, IntDivTruncTowardZero) {
  auto v1 = runAndPeek({intConst(7),  intConst(3), binOp(binop::kIntDiv)});
  EXPECT_EQ(v1.asInteger(), 2);
  auto v2 = runAndPeek({intConst(-7), intConst(3), binOp(binop::kIntDiv)});
  EXPECT_EQ(v2.asInteger(), -2);
  auto v3 = runAndPeek({intConst(7),  intConst(-3), binOp(binop::kIntDiv)});
  EXPECT_EQ(v3.asInteger(), -2);
}

TEST(Arithmetic, IntMod) {
  auto v1 = runAndPeek({intConst(7),  intConst(3),  binOp(binop::kIntMod)});
  EXPECT_EQ(v1.asInteger(),  1);
  auto v2 = runAndPeek({intConst(-7), intConst(3),  binOp(binop::kIntMod)});
  EXPECT_EQ(v2.asInteger(), -1);
  auto v3 = runAndPeek({intConst(7),  intConst(-3), binOp(binop::kIntMod)});
  EXPECT_EQ(v3.asInteger(),  1);
  auto v4 = runAndPeek({intConst(-7), intConst(-3), binOp(binop::kIntMod)});
  EXPECT_EQ(v4.asInteger(), -1);
}

TEST(Arithmetic, IntNeg) {
  auto v = runAndPeek({intConst(42), unaryOp(unop::kIntNeg)});
  EXPECT_EQ(v.asInteger(), -42);
}

TEST(Arithmetic, DivByZeroThrows) {
  EXPECT_THROW(
      runAndPeek({intConst(5), intConst(0), binOp(binop::kIntDiv)}),
      DivisionByZeroError);
}

TEST(Arithmetic, ModByZeroThrows) {
  EXPECT_THROW(
      runAndPeek({intConst(5), intConst(0), binOp(binop::kIntMod)}),
      DivisionByZeroError);
}

TEST(Arithmetic, RealAdd) {
  auto v = runAndPeek({realConst(5.5), realConst(2.0), binOp(binop::kRealAdd)});
  EXPECT_DOUBLE_EQ(v.asReal(), 7.5);
}

TEST(Arithmetic, RealSub) {
  auto v = runAndPeek({realConst(5.5), realConst(2.0), binOp(binop::kRealSub)});
  EXPECT_DOUBLE_EQ(v.asReal(), 3.5);
}

TEST(Arithmetic, RealMul) {
  auto v = runAndPeek({realConst(5.5), realConst(2.0), binOp(binop::kRealMul)});
  EXPECT_DOUBLE_EQ(v.asReal(), 11.0);
}

TEST(Arithmetic, RealDiv) {
  auto v = runAndPeek({realConst(5.5), realConst(2.0), binOp(binop::kRealDiv)});
  EXPECT_DOUBLE_EQ(v.asReal(), 2.75);
}

TEST(Arithmetic, RealNeg) {
  auto v = runAndPeek({realConst(5.5), unaryOp(unop::kRealNeg)});
  EXPECT_DOUBLE_EQ(v.asReal(), -5.5);
}

TEST(Comparison, IntLt) {
  auto v1 = runAndPeek({intConst(3), intConst(7), binOp(binop::kIntLt)});
  EXPECT_TRUE(v1.asBoolean());
  auto v2 = runAndPeek({intConst(7), intConst(3), binOp(binop::kIntLt)});
  EXPECT_FALSE(v2.asBoolean());
}

TEST(Comparison, IntGt) {
  auto v = runAndPeek({intConst(10), intConst(3), binOp(binop::kIntGt)});
  EXPECT_TRUE(v.asBoolean());
}

TEST(Comparison, IntLe) {
  auto v1 = runAndPeek({intConst(5), intConst(5), binOp(binop::kIntLe)});
  EXPECT_TRUE(v1.asBoolean());
  auto v2 = runAndPeek({intConst(6), intConst(5), binOp(binop::kIntLe)});
  EXPECT_FALSE(v2.asBoolean());
}

TEST(Comparison, EqInt) {
  auto v1 = runAndPeek({intConst(5), intConst(5), binOp(binop::kEqEq)});
  EXPECT_TRUE(v1.asBoolean());
  auto v2 = runAndPeek({intConst(5), intConst(6), binOp(binop::kEqEq)});
  EXPECT_FALSE(v2.asBoolean());
}

TEST(Comparison, NeInt) {
  auto v = runAndPeek({intConst(5), intConst(6), binOp(binop::kEqNe)});
  EXPECT_TRUE(v.asBoolean());
}

TEST(BoolLogic, And) {
  auto vTT = runAndPeek({
      intConst(1), op(Opcode::kIntToBool),
      intConst(1), op(Opcode::kIntToBool),
      binOp(binop::kBoolAnd)});
  EXPECT_TRUE(vTT.asBoolean());

  auto vTF = runAndPeek({
      intConst(1), op(Opcode::kIntToBool),
      intConst(0), op(Opcode::kIntToBool),
      binOp(binop::kBoolAnd)});
  EXPECT_FALSE(vTF.asBoolean());
}

TEST(BoolLogic, Or) {
  auto vFF = runAndPeek({
      intConst(0), op(Opcode::kIntToBool),
      intConst(0), op(Opcode::kIntToBool),
      binOp(binop::kBoolOr)});
  EXPECT_FALSE(vFF.asBoolean());

  auto vTF = runAndPeek({
      intConst(1), op(Opcode::kIntToBool),
      intConst(0), op(Opcode::kIntToBool),
      binOp(binop::kBoolOr)});
  EXPECT_TRUE(vTF.asBoolean());
}

TEST(BoolLogic, Xor) {
  auto vTF = runAndPeek({
      intConst(1), op(Opcode::kIntToBool),
      intConst(0), op(Opcode::kIntToBool),
      binOp(binop::kBoolXor)});
  EXPECT_TRUE(vTF.asBoolean());

  auto vTT = runAndPeek({
      intConst(1), op(Opcode::kIntToBool),
      intConst(1), op(Opcode::kIntToBool),
      binOp(binop::kBoolXor)});
  EXPECT_FALSE(vTT.asBoolean());
}

TEST(BoolLogic, Not) {
  auto v = runAndPeek({intConst(1), op(Opcode::kIntToBool),
                       unaryOp(unop::kBoolNeg)});
  EXPECT_FALSE(v.asBoolean());
}

TEST(Conversion, IntToReal) {
  auto v = runAndPeek({intConst(7), op(Opcode::kIntToReal)});
  EXPECT_EQ(v.type_id, kRealTypeId);
  EXPECT_DOUBLE_EQ(v.asReal(), 7.0);
}

TEST(Conversion, RealToIntTruncatesTowardZero) {
  auto v1 = runAndPeek({realConst(2.75), op(Opcode::kRealToInt)});
  EXPECT_EQ(v1.type_id, kIntegerTypeId);
  EXPECT_EQ(v1.asInteger(), 2);

  auto v2 = runAndPeek({realConst(2.4), op(Opcode::kRealToInt)});
  EXPECT_EQ(v2.asInteger(), 2);

  auto v3 = runAndPeek({realConst(-2.5), op(Opcode::kRealToInt)});
  EXPECT_EQ(v3.asInteger(), -2);
}

TEST(Conversion, IntToBoolValid) {
  auto v0 = runAndPeek({intConst(0), op(Opcode::kIntToBool)});
  EXPECT_EQ(v0.type_id, kBooleanTypeId);
  EXPECT_FALSE(v0.asBoolean());

  auto v1 = runAndPeek({intConst(1), op(Opcode::kIntToBool)});
  EXPECT_TRUE(v1.asBoolean());
}

TEST(Conversion, IntToBoolNonZeroIsTrue) {
  auto v = runAndPeek({intConst(2), op(Opcode::kIntToBool)});
  EXPECT_EQ(v.type_id, kBooleanTypeId);
  EXPECT_TRUE(v.asBoolean());
}

TEST(Stack, Dup) {
  auto v = runAndPeek({intConst(42), op(Opcode::kDup), binOp(binop::kIntAdd)});
  EXPECT_EQ(v.asInteger(), 84);
}

TEST(Stack, Swap) {
  auto v = runAndPeek({
      intConst(10), intConst(3),
      op(Opcode::kSwap),
      binOp(binop::kIntSub)});
  EXPECT_EQ(v.asInteger(), -7);
}

TEST(Stack, Drop) {
  auto prog = Loader::makeTestProgram({
      op(Opcode::kLabel),
      intConst(99),
      intConst(42),
      op(Opcode::kDrop),
  });
  prog.instructions[0].arg64 = 0;
  Vm vm(std::move(prog));
  vm.run();
  ASSERT_EQ(vm.eval_stack().size(), 1u);
  EXPECT_EQ(vm.eval_stack()[0].asInteger(), 99);
}
