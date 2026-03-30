#include <gtest/gtest.h>

#include <bit>

#include "vm/error.hpp"
#include "vm/loader.hpp"
#include "vm/opcodes.hpp"
#include "vm/vm.hpp"

using namespace vm;

// ---- Helpers ----------------------------------------------------------------

static Instruction Op(Opcode op) {
  Instruction i;
  i.opcode = static_cast<uint8_t>(op);
  return i;
}

static Instruction IntConst(int64_t v) {
  Instruction i = Op(Opcode::kIntConst);
  i.arg64 = static_cast<uint64_t>(v);
  return i;
}

static Instruction RealConst(double v) {
  Instruction i = Op(Opcode::kRealConst);
  i.arg64 = std::bit_cast<uint64_t>(v);
  return i;
}

static Instruction BinOp(uint8_t sub) {
  Instruction i = Op(Opcode::kBinOp);
  i.subopcode = sub;
  return i;
}

static Instruction UnaryOp(uint8_t sub) {
  Instruction i = Op(Opcode::kUnOp);
  i.subopcode = sub;
  return i;
}

// Runs a program and returns the top of the eval stack.
// Prepends Label(0) so MakeTestProgram can find main's entry.
static Value RunAndPeek(std::vector<Instruction> instrs) {
  Instruction lbl = Op(Opcode::kLabel);
  lbl.arg64 = 0;
  instrs.insert(instrs.begin(), lbl);

  Program prog = Loader::MakeTestProgram(std::move(instrs));
  Vm vm(std::move(prog));
  vm.Run();
  return vm.eval_stack().back();
}

// ---- Integer arithmetic -----------------------------------------------------

TEST(Arithmetic, IntAdd) {
  auto v = RunAndPeek({IntConst(7), IntConst(3), BinOp(binop::kIntAdd)});
  EXPECT_EQ(v.type_id, kIntegerTypeId);
  EXPECT_EQ(v.AsInteger(), 10);
}

TEST(Arithmetic, IntSub) {
  auto v = RunAndPeek({IntConst(7), IntConst(3), BinOp(binop::kIntSub)});
  EXPECT_EQ(v.AsInteger(), 4);
}

TEST(Arithmetic, IntMul) {
  auto v = RunAndPeek({IntConst(7), IntConst(3), BinOp(binop::kIntMul)});
  EXPECT_EQ(v.AsInteger(), 21);
}

TEST(Arithmetic, IntDivTruncTowardZero) {
  auto v1 = RunAndPeek({IntConst(7),  IntConst(3), BinOp(binop::kIntDiv)});
  EXPECT_EQ(v1.AsInteger(), 2);
  auto v2 = RunAndPeek({IntConst(-7), IntConst(3), BinOp(binop::kIntDiv)});
  EXPECT_EQ(v2.AsInteger(), -2);
  auto v3 = RunAndPeek({IntConst(7),  IntConst(-3), BinOp(binop::kIntDiv)});
  EXPECT_EQ(v3.AsInteger(), -2);
}

TEST(Arithmetic, IntMod) {
  // Result has the sign of the dividend.
  auto v1 = RunAndPeek({IntConst(7),  IntConst(3),  BinOp(binop::kIntMod)});
  EXPECT_EQ(v1.AsInteger(),  1);
  auto v2 = RunAndPeek({IntConst(-7), IntConst(3),  BinOp(binop::kIntMod)});
  EXPECT_EQ(v2.AsInteger(), -1);
  auto v3 = RunAndPeek({IntConst(7),  IntConst(-3), BinOp(binop::kIntMod)});
  EXPECT_EQ(v3.AsInteger(),  1);
  auto v4 = RunAndPeek({IntConst(-7), IntConst(-3), BinOp(binop::kIntMod)});
  EXPECT_EQ(v4.AsInteger(), -1);
}

TEST(Arithmetic, IntNeg) {
  auto v = RunAndPeek({IntConst(42), UnaryOp(unop::kIntNeg)});
  EXPECT_EQ(v.AsInteger(), -42);
}

TEST(Arithmetic, DivByZeroThrows) {
  EXPECT_THROW(
      RunAndPeek({IntConst(5), IntConst(0), BinOp(binop::kIntDiv)}),
      DivisionByZeroError);
}

TEST(Arithmetic, ModByZeroThrows) {
  EXPECT_THROW(
      RunAndPeek({IntConst(5), IntConst(0), BinOp(binop::kIntMod)}),
      DivisionByZeroError);
}

// ---- Real arithmetic --------------------------------------------------------

TEST(Arithmetic, RealAdd) {
  auto v = RunAndPeek({RealConst(5.5), RealConst(2.0), BinOp(binop::kRealAdd)});
  EXPECT_DOUBLE_EQ(v.AsReal(), 7.5);
}

TEST(Arithmetic, RealSub) {
  auto v = RunAndPeek({RealConst(5.5), RealConst(2.0), BinOp(binop::kRealSub)});
  EXPECT_DOUBLE_EQ(v.AsReal(), 3.5);
}

TEST(Arithmetic, RealMul) {
  auto v = RunAndPeek({RealConst(5.5), RealConst(2.0), BinOp(binop::kRealMul)});
  EXPECT_DOUBLE_EQ(v.AsReal(), 11.0);
}

TEST(Arithmetic, RealDiv) {
  auto v = RunAndPeek({RealConst(5.5), RealConst(2.0), BinOp(binop::kRealDiv)});
  EXPECT_DOUBLE_EQ(v.AsReal(), 2.75);
}

TEST(Arithmetic, RealNeg) {
  auto v = RunAndPeek({RealConst(5.5), UnaryOp(unop::kRealNeg)});
  EXPECT_DOUBLE_EQ(v.AsReal(), -5.5);
}

// ---- Comparisons ------------------------------------------------------------

TEST(Comparison, IntLt) {
  auto v1 = RunAndPeek({IntConst(3), IntConst(7), BinOp(binop::kIntLt)});
  EXPECT_TRUE(v1.AsBoolean());
  auto v2 = RunAndPeek({IntConst(7), IntConst(3), BinOp(binop::kIntLt)});
  EXPECT_FALSE(v2.AsBoolean());
}

TEST(Comparison, IntGt) {
  auto v = RunAndPeek({IntConst(10), IntConst(3), BinOp(binop::kIntGt)});
  EXPECT_TRUE(v.AsBoolean());
}

TEST(Comparison, IntLe) {
  auto v1 = RunAndPeek({IntConst(5), IntConst(5), BinOp(binop::kIntLe)});
  EXPECT_TRUE(v1.AsBoolean());
  auto v2 = RunAndPeek({IntConst(6), IntConst(5), BinOp(binop::kIntLe)});
  EXPECT_FALSE(v2.AsBoolean());
}

TEST(Comparison, EqInt) {
  auto v1 = RunAndPeek({IntConst(5), IntConst(5), BinOp(binop::kEqEq)});
  EXPECT_TRUE(v1.AsBoolean());
  auto v2 = RunAndPeek({IntConst(5), IntConst(6), BinOp(binop::kEqEq)});
  EXPECT_FALSE(v2.AsBoolean());
}

TEST(Comparison, NeInt) {
  auto v = RunAndPeek({IntConst(5), IntConst(6), BinOp(binop::kEqNe)});
  EXPECT_TRUE(v.AsBoolean());
}

// ---- Boolean logic ----------------------------------------------------------

TEST(BoolLogic, And) {
  auto vTT = RunAndPeek({
      IntConst(1), Op(Opcode::kIntToBool),
      IntConst(1), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolAnd)});
  EXPECT_TRUE(vTT.AsBoolean());

  auto vTF = RunAndPeek({
      IntConst(1), Op(Opcode::kIntToBool),
      IntConst(0), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolAnd)});
  EXPECT_FALSE(vTF.AsBoolean());
}

TEST(BoolLogic, Or) {
  auto vFF = RunAndPeek({
      IntConst(0), Op(Opcode::kIntToBool),
      IntConst(0), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolOr)});
  EXPECT_FALSE(vFF.AsBoolean());

  auto vTF = RunAndPeek({
      IntConst(1), Op(Opcode::kIntToBool),
      IntConst(0), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolOr)});
  EXPECT_TRUE(vTF.AsBoolean());
}

TEST(BoolLogic, Xor) {
  auto vTF = RunAndPeek({
      IntConst(1), Op(Opcode::kIntToBool),
      IntConst(0), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolXor)});
  EXPECT_TRUE(vTF.AsBoolean());

  auto vTT = RunAndPeek({
      IntConst(1), Op(Opcode::kIntToBool),
      IntConst(1), Op(Opcode::kIntToBool),
      BinOp(binop::kBoolXor)});
  EXPECT_FALSE(vTT.AsBoolean());
}

TEST(BoolLogic, Not) {
  auto v = RunAndPeek({IntConst(1), Op(Opcode::kIntToBool),
                       UnaryOp(unop::kBoolNeg)});
  EXPECT_FALSE(v.AsBoolean());
}

// ---- Type conversions -------------------------------------------------------

TEST(Conversion, IntToReal) {
  auto v = RunAndPeek({IntConst(7), Op(Opcode::kIntToReal)});
  EXPECT_EQ(v.type_id, kRealTypeId);
  EXPECT_DOUBLE_EQ(v.AsReal(), 7.0);
}

TEST(Conversion, RealToIntRoundsToNearest) {
  auto v1 = RunAndPeek({RealConst(2.75), Op(Opcode::kRealToInt)});
  EXPECT_EQ(v1.type_id, kIntegerTypeId);
  EXPECT_EQ(v1.AsInteger(), 3);

  auto v2 = RunAndPeek({RealConst(2.4), Op(Opcode::kRealToInt)});
  EXPECT_EQ(v2.AsInteger(), 2);

  auto v3 = RunAndPeek({RealConst(-2.5), Op(Opcode::kRealToInt)});
  EXPECT_EQ(v3.AsInteger(), -3);  // round to nearest (away from zero)
}

TEST(Conversion, IntToBoolValid) {
  auto v0 = RunAndPeek({IntConst(0), Op(Opcode::kIntToBool)});
  EXPECT_EQ(v0.type_id, kBooleanTypeId);
  EXPECT_FALSE(v0.AsBoolean());

  auto v1 = RunAndPeek({IntConst(1), Op(Opcode::kIntToBool)});
  EXPECT_TRUE(v1.AsBoolean());
}

TEST(Conversion, IntToBoolInvalidThrows) {
  EXPECT_THROW(
      RunAndPeek({IntConst(2), Op(Opcode::kIntToBool)}),
      ConversionError);
}

// ---- Stack operations -------------------------------------------------------

TEST(Stack, Dup) {
  // Push 42, dup, add => 84
  auto v = RunAndPeek({IntConst(42), Op(Opcode::kDup), BinOp(binop::kIntAdd)});
  EXPECT_EQ(v.AsInteger(), 84);
}

TEST(Stack, Swap) {
  // Push 10, push 3, swap => 3 on bottom, 10 on top
  // then sub => 3 - 10 = -7
  auto v = RunAndPeek({
      IntConst(10), IntConst(3),
      Op(Opcode::kSwap),
      BinOp(binop::kIntSub)});
  EXPECT_EQ(v.AsInteger(), -7);
}

TEST(Stack, Drop) {
  // Push 99, push 42, drop => only 99 on stack
  auto prog = Loader::MakeTestProgram({
      Op(Opcode::kLabel),  // label 0
      IntConst(99),
      IntConst(42),
      Op(Opcode::kDrop),
  });
  prog.instructions[0].arg64 = 0;
  Vm vm(std::move(prog));
  vm.Run();
  ASSERT_EQ(vm.eval_stack().size(), 1u);
  EXPECT_EQ(vm.eval_stack()[0].AsInteger(), 99);
}
