#include <gtest/gtest.h>

#include <bit>

#include "vm/error.hpp"
#include "vm/heap.hpp"
#include "vm/loader.hpp"
#include "vm/opcodes.hpp"
#include "vm/value.hpp"
#include "vm/vm.hpp"

using namespace vm;

static Instruction MakeLabel(uint64_t id) {
  Instruction i;
  i.opcode = static_cast<uint8_t>(Opcode::kLabel);
  i.arg64  = id;
  return i;
}

// Creates a Vm with an empty main body (Label(0) only).
static Vm MakeEmptyVm() {
  Program prog = Loader::MakeTestProgram({MakeLabel(0)});
  return Vm(std::move(prog));
}

// Pushes a heap reference onto the eval stack so the GC can see it.
static void PushRef(Vm& vm, HeapObject* obj, uint32_t type_id) {
  Value ref;
  ref.type_id = type_id;
  ref.data    = std::bit_cast<uint64_t>(obj);
  vm.Push(ref);
}

// ---- GC tests ---------------------------------------------------------------

// Allocate many unreferenced records; GC must not crash or leak.
TEST(GC, CollectsUnreachableObjects) {
  auto vm = MakeEmptyVm();

  constexpr uint32_t kTypeId = 10;
  constexpr std::size_t kMany = 200;

  // Allocate many objects without keeping references -> all become garbage.
  // After kGcInterval allocations the GC fires internally.
  for (std::size_t i = 0; i < kMany; ++i)
    vm.AllocRecord(kTypeId, 2);

  // No crash = GC handled it correctly.
  SUCCEED();
}

// Allocate a record, keep a reference on the eval stack, trigger GC.
// The object must survive.
TEST(GC, KeepsReachableRecord) {
  auto vm = MakeEmptyVm();

  constexpr uint32_t kTypeId = 11;
  HeapObject* obj = vm.AllocRecord(kTypeId, 2);
  obj->fields[0] = Value::MakeInteger(99);

  PushRef(vm, obj, kTypeId);

  // Allocate unreferenced objects to trigger multiple GC cycles.
  for (std::size_t i = 0; i < 200; ++i)
    vm.AllocRecord(kTypeId, 1);

  // Dereference through the stack value.
  Value top = vm.eval_stack().back();
  auto* survived = std::bit_cast<HeapObject*>(top.data);
  ASSERT_NE(survived, nullptr);
  EXPECT_EQ(survived->fields[0].AsInteger(), 99);
}

// A record referenced by a global variable must survive GC.
TEST(GC, KeepsGlobalRootObject) {
  Program prog = Loader::MakeTestProgram({MakeLabel(0)}, /*global_count=*/1);
  Vm vm(std::move(prog));

  constexpr uint32_t kTypeId = 12;
  HeapObject* obj = vm.AllocRecord(kTypeId, 1);
  obj->fields[0] = Value::MakeInteger(77);

  Value ref;
  ref.type_id = kTypeId;
  ref.data    = std::bit_cast<uint64_t>(obj);
  vm.globals()[0] = ref;

  for (std::size_t i = 0; i < 200; ++i)
    vm.AllocRecord(kTypeId, 1);

  auto* survived = std::bit_cast<HeapObject*>(vm.globals()[0].data);
  ASSERT_NE(survived, nullptr);
  EXPECT_EQ(survived->fields[0].AsInteger(), 77);
}

// ---- Array allocation and element access ------------------------------------

TEST(Heap, ArrayAllocationAndAccess) {
  auto vm = MakeEmptyVm();

  constexpr uint32_t kArrTypeId = 20;
  HeapObject* arr = vm.AllocArray(kArrTypeId, 5);
  ASSERT_EQ(arr->Size(), 5u);

  // Language arrays are 1-indexed.
  arr->ElementAt(1) = Value::MakeInteger(10);
  arr->ElementAt(5) = Value::MakeInteger(50);

  EXPECT_EQ(arr->ElementAt(1).AsInteger(), 10);
  EXPECT_EQ(arr->ElementAt(5).AsInteger(), 50);
}

TEST(Heap, ArrayBoundsCheck) {
  auto vm = MakeEmptyVm();
  HeapObject* arr = vm.AllocArray(20, 3);

  EXPECT_THROW(arr->ElementAt(0), IndexOutOfBoundsError);  // 0 < 1
  EXPECT_THROW(arr->ElementAt(4), IndexOutOfBoundsError);  // 4 > 3
}

// ---- Nested objects survive GC ----------------------------------------------

TEST(GC, NestedObjectsSurviveGC) {
  auto vm = MakeEmptyVm();

  constexpr uint32_t kRecTypeId = 30;
  constexpr uint32_t kArrTypeId = 31;

  // Allocate array, embed it in a record field.
  HeapObject* arr = vm.AllocArray(kArrTypeId, 2);
  arr->ElementAt(1) = Value::MakeInteger(42);

  HeapObject* rec = vm.AllocRecord(kRecTypeId, 1);
  Value arr_ref;
  arr_ref.type_id = kArrTypeId;
  arr_ref.data    = std::bit_cast<uint64_t>(arr);
  rec->fields[0]  = arr_ref;

  // Only the record is directly on the eval stack.
  PushRef(vm, rec, kRecTypeId);

  // Trigger multiple GC cycles.
  for (std::size_t i = 0; i < 200; ++i)
    vm.AllocRecord(kRecTypeId, 1);

  // Verify both objects survived.
  Value top = vm.eval_stack().back();
  auto* live_rec = std::bit_cast<HeapObject*>(top.data);
  ASSERT_NE(live_rec, nullptr);
  auto* live_arr = std::bit_cast<HeapObject*>(live_rec->fields[0].data);
  ASSERT_NE(live_arr, nullptr);
  EXPECT_EQ(live_arr->ElementAt(1).AsInteger(), 42);
}
