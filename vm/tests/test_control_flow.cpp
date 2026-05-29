#include <gtest/gtest.h>

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

static Instruction binOp(uint8_t sub) {
  Instruction i = op(Opcode::kBinOp);
  i.subopcode = sub;
  return i;
}

static Instruction makeJump(uint64_t label_id) {
  Instruction i = op(Opcode::kJump);
  i.arg64 = label_id;
  return i;
}

static Instruction makeJumpZero(uint64_t label_id) {
  Instruction i = op(Opcode::kJumpCond);
  i.subopcode = jumpcond::kJumpZero;
  i.arg64 = label_id;
  return i;
}

static Instruction makeJumpNotZero(uint64_t label_id) {
  Instruction i = op(Opcode::kJumpCond);
  i.subopcode = jumpcond::kJumpNotZero;
  i.arg64 = label_id;
  return i;
}

static Instruction storeLocal(uint16_t idx) {
  Instruction i = op(Opcode::kStore);
  i.subopcode = static_cast<uint8_t>(LocationKind::kLocal);
  i.arg16 = idx;
  return i;
}

static Instruction loadLocal(uint16_t idx) {
  Instruction i = op(Opcode::kLoad);
  i.subopcode = static_cast<uint8_t>(LocationKind::kLocal);
  i.arg16 = idx;
  return i;
}

static Instruction storeGlobal(uint16_t idx) {
  Instruction i = op(Opcode::kStore);
  i.subopcode = static_cast<uint8_t>(LocationKind::kGlobal);
  i.arg16 = idx;
  return i;
}

static Instruction loadGlobal(uint16_t idx) {
  Instruction i = op(Opcode::kLoad);
  i.subopcode = static_cast<uint8_t>(LocationKind::kGlobal);
  i.arg16 = idx;
  return i;
}

static std::vector<Instruction> boolLiteral(bool v) {
  return {intConst(v ? 1 : 0), op(Opcode::kIntToBool)};
}

static Vm runProgram(std::vector<Instruction> instrs,
                     uint32_t global_count = 0) {
  instrs.insert(instrs.begin(), makeLabel(0));
  Program prog = Loader::makeTestProgram(std::move(instrs), global_count);
  Vm vm(std::move(prog));
  vm.run();
  return vm;
}

TEST(ControlFlow, UnconditionalJump) {
  auto vm = runProgram({
      makeJump(1),
      intConst(99),
      makeLabel(1),
      intConst(42),
  });
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 42);
}

TEST(ControlFlow, JumpZero_TakenOnFalse) {
  auto instrs = boolLiteral(false);
  instrs.push_back(makeJumpZero(1));
  instrs.push_back(intConst(99));
  instrs.push_back(makeLabel(1));
  instrs.push_back(intConst(42));

  auto vm = runProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 42);
}

TEST(ControlFlow, JumpZero_NotTakenOnTrue) {
  auto instrs = boolLiteral(true);
  instrs.push_back(makeJumpZero(1));
  instrs.push_back(intConst(99));
  instrs.push_back(makeLabel(1));

  auto vm = runProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 99);
}

TEST(ControlFlow, JumpNotZero_TakenOnTrue) {
  auto instrs = boolLiteral(true);
  instrs.push_back(makeJumpNotZero(1));
  instrs.push_back(intConst(99));
  instrs.push_back(makeLabel(1));
  instrs.push_back(intConst(42));

  auto vm = runProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 42);
}

TEST(ControlFlow, JumpNotZero_NotTakenOnFalse) {
  auto instrs = boolLiteral(false);
  instrs.push_back(makeJumpNotZero(1));
  instrs.push_back(intConst(99));
  instrs.push_back(makeLabel(1));

  auto vm = runProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 99);
}

TEST(ControlFlow, BackwardJumpLoop) {
  std::vector<Instruction> instrs = {
      intConst(0), storeLocal(0),
      makeLabel(1),
        loadLocal(0), intConst(5),
        binOp(binop::kIntLt),
        makeJumpZero(2),
        loadLocal(0), intConst(1),
        binOp(binop::kIntAdd),
        storeLocal(0),
        makeJump(1),
      makeLabel(2),
      loadLocal(0),
  };

  auto vm = runProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 5);
}

TEST(Variables, StoreAndLoad) {
  auto vm = runProgram({intConst(55), storeLocal(0), loadLocal(0)});
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 55);
}

TEST(Variables, TwoLocals) {
  auto vm = runProgram({
      intConst(10), storeLocal(0),
      intConst(32), storeLocal(1),
      loadLocal(0), loadLocal(1), binOp(binop::kIntAdd),
  });
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 42);
}

TEST(Variables, GlobalStoreAndLoad) {
  auto vm = runProgram(
      {intConst(77), storeGlobal(0), loadGlobal(0)},
      1);
  EXPECT_EQ(vm.eval_stack().back().asInteger(), 77);
}

TEST(ControlFlow, PanicThrows) {
  Instruction p = op(Opcode::kPanic);
  p.arg64 = 42;
  EXPECT_THROW(runProgram({p}), PanicError);
}
