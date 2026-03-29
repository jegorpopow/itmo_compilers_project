#include <gtest/gtest.h>

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

static Instruction MakeLabel(uint64_t id) {
  Instruction i = Op(Opcode::kLabel);
  i.arg64 = id;
  return i;
}

static Instruction IntConst(int64_t v) {
  Instruction i = Op(Opcode::kIntConst);
  i.arg64 = static_cast<uint64_t>(v);
  return i;
}

static Instruction BinOp(uint8_t sub) {
  Instruction i = Op(Opcode::kBinOp);
  i.subopcode = sub;
  return i;
}

static Instruction MakeJump(uint64_t label_id) {
  Instruction i = Op(Opcode::kJump);
  i.arg64 = label_id;
  return i;
}

static Instruction MakeJumpZero(uint64_t label_id) {
  Instruction i = Op(Opcode::kJumpCond);
  i.subopcode = jumpcond::kJumpZero;
  i.arg64 = label_id;
  return i;
}

static Instruction MakeJumpNotZero(uint64_t label_id) {
  Instruction i = Op(Opcode::kJumpCond);
  i.subopcode = jumpcond::kJumpNotZero;
  i.arg64 = label_id;
  return i;
}

static Instruction StoreLocal(uint16_t idx) {
  Instruction i = Op(Opcode::kStore);
  i.subopcode = static_cast<uint8_t>(LocationKind::kLocal);
  i.arg16 = idx;
  return i;
}

static Instruction LoadLocal(uint16_t idx) {
  Instruction i = Op(Opcode::kLoad);
  i.subopcode = static_cast<uint8_t>(LocationKind::kLocal);
  i.arg16 = idx;
  return i;
}

static Instruction StoreGlobal(uint16_t idx) {
  Instruction i = Op(Opcode::kStore);
  i.subopcode = static_cast<uint8_t>(LocationKind::kGlobal);
  i.arg16 = idx;
  return i;
}

static Instruction LoadGlobal(uint16_t idx) {
  Instruction i = Op(Opcode::kLoad);
  i.subopcode = static_cast<uint8_t>(LocationKind::kGlobal);
  i.arg16 = idx;
  return i;
}

// Push a boolean literal (via IntConst + IntToBool).
static std::vector<Instruction> BoolLiteral(bool v) {
  return {IntConst(v ? 1 : 0), Op(Opcode::kIntToBool)};
}

// Runs a program (prepending Label(0) for main entry) and returns eval stack.
static Vm RunProgram(std::vector<Instruction> instrs,
                     uint32_t global_count = 0) {
  instrs.insert(instrs.begin(), MakeLabel(0));
  Program prog = Loader::MakeTestProgram(std::move(instrs), global_count);
  Vm vm(std::move(prog));
  vm.Run();
  return vm;
}

// ---- Unconditional jump -----------------------------------------------------

TEST(ControlFlow, UnconditionalJump) {
  // Label(0) <-- main entry (prepended by RunProgram)
  // Jump(1)   -- skip IntConst(99)
  // IntConst(99)
  // Label(1)
  // IntConst(42)
  auto vm = RunProgram({
      MakeJump(1),
      IntConst(99),
      MakeLabel(1),
      IntConst(42),
  });
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 42);
}

// ---- Conditional jumps ------------------------------------------------------

TEST(ControlFlow, JumpZero_TakenOnFalse) {
  // cond=false -> jump to label 1, skip 99, push 42
  auto instrs = BoolLiteral(false);
  instrs.push_back(MakeJumpZero(1));
  instrs.push_back(IntConst(99));  // skipped
  instrs.push_back(MakeLabel(1));
  instrs.push_back(IntConst(42));

  auto vm = RunProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 42);
}

TEST(ControlFlow, JumpZero_NotTakenOnTrue) {
  // cond=true -> do NOT jump, push 99
  auto instrs = BoolLiteral(true);
  instrs.push_back(MakeJumpZero(1));
  instrs.push_back(IntConst(99));  // executed
  instrs.push_back(MakeLabel(1));

  auto vm = RunProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 99);
}

TEST(ControlFlow, JumpNotZero_TakenOnTrue) {
  auto instrs = BoolLiteral(true);
  instrs.push_back(MakeJumpNotZero(1));
  instrs.push_back(IntConst(99));  // skipped
  instrs.push_back(MakeLabel(1));
  instrs.push_back(IntConst(42));

  auto vm = RunProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 42);
}

TEST(ControlFlow, JumpNotZero_NotTakenOnFalse) {
  auto instrs = BoolLiteral(false);
  instrs.push_back(MakeJumpNotZero(1));
  instrs.push_back(IntConst(99));  // executed
  instrs.push_back(MakeLabel(1));

  auto vm = RunProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 99);
}

// ---- Simple loop via backward jump ------------------------------------------

// Emulates: var i = 0; while i < 5 { i = i + 1 }; push i
// Labels: 1 = loop header, 2 = loop exit
TEST(ControlFlow, BackwardJumpLoop) {
  // local[0] = counter
  std::vector<Instruction> instrs = {
      IntConst(0), StoreLocal(0),         // i = 0
      MakeLabel(1),                        // loop_start:
        LoadLocal(0), IntConst(5),
        BinOp(binop::kIntLt),             // i < 5
        MakeJumpZero(2),                  // if not (i<5) -> exit
        LoadLocal(0), IntConst(1),
        BinOp(binop::kIntAdd),
        StoreLocal(0),                    // i = i + 1
        MakeJump(1),                      // -> loop_start
      MakeLabel(2),                        // exit:
      LoadLocal(0),                        // push i (should be 5)
  };

  auto vm = RunProgram(std::move(instrs));
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 5);
}

// ---- Local variables --------------------------------------------------------

TEST(Variables, StoreAndLoad) {
  auto vm = RunProgram({IntConst(55), StoreLocal(0), LoadLocal(0)});
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 55);
}

TEST(Variables, TwoLocals) {
  auto vm = RunProgram({
      IntConst(10), StoreLocal(0),
      IntConst(32), StoreLocal(1),
      LoadLocal(0), LoadLocal(1), BinOp(binop::kIntAdd),
  });
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 42);
}

// ---- Global variables -------------------------------------------------------

TEST(Variables, GlobalStoreAndLoad) {
  auto vm = RunProgram(
      {IntConst(77), StoreGlobal(0), LoadGlobal(0)},
      /*global_count=*/1);
  EXPECT_EQ(vm.eval_stack().back().AsInteger(), 77);
}

// ---- Panic ------------------------------------------------------------------

TEST(ControlFlow, PanicThrows) {
  Instruction p = Op(Opcode::kPanic);
  p.arg64 = 42;
  EXPECT_THROW(RunProgram({p}), PanicError);
}
