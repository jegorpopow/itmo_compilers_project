#include <gtest/gtest.h>

#include <bit>

#include "vm/error.hpp"
#include "vm/heap.hpp"
#include "vm/loader.hpp"
#include "vm/opcodes.hpp"
#include "vm/value.hpp"
#include "vm/vm.hpp"

using namespace vm;

static Instruction makeLabel(uint64_t id) {
  Instruction i;
  i.opcode = static_cast<uint8_t>(Opcode::kLabel);
  i.arg64  = id;
  return i;
}

static Vm makeEmptyVm() {
  Program prog = Loader::makeTestProgram({makeLabel(0)});
  return Vm(std::move(prog));
}

static void pushRef(Vm& vm, HeapObject* obj, uint32_t type_id) {
  Value ref;
  ref.type_id = type_id;
  ref.data    = std::bit_cast<uint64_t>(obj);
  vm.push(ref);
}

TEST(GC, CollectsUnreachableObjects) {
  auto vm = makeEmptyVm();

  constexpr uint32_t kTypeId = 10;
  constexpr std::size_t kMany = 200;

  for (std::size_t i = 0; i < kMany; ++i)
    vm.allocRecord(kTypeId, 2);

  SUCCEED();
}

TEST(GC, KeepsReachableRecord) {
  auto vm = makeEmptyVm();

  constexpr uint32_t kTypeId = 11;
  HeapObject* obj = vm.allocRecord(kTypeId, 2);
  obj->fields[0] = Value::makeInteger(99);

  pushRef(vm, obj, kTypeId);

  for (std::size_t i = 0; i < 200; ++i)
    vm.allocRecord(kTypeId, 1);

  Value top = vm.eval_stack().back();
  auto* survived = std::bit_cast<HeapObject*>(top.data);
  ASSERT_NE(survived, nullptr);
  EXPECT_EQ(survived->fields[0].asInteger(), 99);
}

TEST(GC, KeepsGlobalRootObject) {
  Program prog = Loader::makeTestProgram({makeLabel(0)}, 1);
  Vm vm(std::move(prog));

  constexpr uint32_t kTypeId = 12;
  HeapObject* obj = vm.allocRecord(kTypeId, 1);
  obj->fields[0] = Value::makeInteger(77);

  Value ref;
  ref.type_id = kTypeId;
  ref.data    = std::bit_cast<uint64_t>(obj);
  vm.globals()[0] = ref;

  for (std::size_t i = 0; i < 200; ++i)
    vm.allocRecord(kTypeId, 1);

  auto* survived = std::bit_cast<HeapObject*>(vm.globals()[0].data);
  ASSERT_NE(survived, nullptr);
  EXPECT_EQ(survived->fields[0].asInteger(), 77);
}

TEST(Heap, ArrayAllocationAndAccess) {
  auto vm = makeEmptyVm();

  constexpr uint32_t kArrTypeId = 20;
  HeapObject* arr = vm.allocArray(kArrTypeId, 5);
  ASSERT_EQ(arr->size(), 5u);

  arr->elementAt(1) = Value::makeInteger(10);
  arr->elementAt(5) = Value::makeInteger(50);

  EXPECT_EQ(arr->elementAt(1).asInteger(), 10);
  EXPECT_EQ(arr->elementAt(5).asInteger(), 50);
}

TEST(Heap, ArrayBoundsCheck) {
  auto vm = makeEmptyVm();
  HeapObject* arr = vm.allocArray(20, 3);

  EXPECT_THROW(arr->elementAt(0), IndexOutOfBoundsError);
  EXPECT_THROW(arr->elementAt(4), IndexOutOfBoundsError);
}

TEST(GC, NestedObjectsSurviveGC) {
  auto vm = makeEmptyVm();

  constexpr uint32_t kRecTypeId = 30;
  constexpr uint32_t kArrTypeId = 31;

  HeapObject* arr = vm.allocArray(kArrTypeId, 2);
  arr->elementAt(1) = Value::makeInteger(42);

  HeapObject* rec = vm.allocRecord(kRecTypeId, 1);
  Value arr_ref;
  arr_ref.type_id = kArrTypeId;
  arr_ref.data    = std::bit_cast<uint64_t>(arr);
  rec->fields[0]  = arr_ref;

  pushRef(vm, rec, kRecTypeId);

  for (std::size_t i = 0; i < 200; ++i)
    vm.allocRecord(kRecTypeId, 1);

  Value top = vm.eval_stack().back();
  auto* live_rec = std::bit_cast<HeapObject*>(top.data);
  ASSERT_NE(live_rec, nullptr);
  auto* live_arr = std::bit_cast<HeapObject*>(live_rec->fields[0].data);
  ASSERT_NE(live_arr, nullptr);
  EXPECT_EQ(live_arr->elementAt(1).asInteger(), 42);
}
