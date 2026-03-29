#include "vm/vm.hpp"

#include <fmt/format.h>

// handlers.hpp must be included here — after vm.hpp — so that all
// OpcodeHandler specialisations are visible before the dispatch table
// is instantiated below.
#include "vm/handlers.hpp"

#include <bit>

#include "vm/error.hpp"
#include "vm/opcodes.hpp"

namespace vm {

// ---- Dispatch table ---------------------------------------------------------
// Built once (at static-init time) from all OpcodeHandler specialisations.

namespace {
const std::array<HandlerFn, 256> kDispatchTable =
    detail::BuildDispatchTable(std::make_index_sequence<256>{});
}  // namespace

const std::array<HandlerFn, 256>& GetDispatchTable() {
  return kDispatchTable;
}

// ---- Program helpers --------------------------------------------------------

std::size_t Program::ResolveLabel(uint64_t label_id) const {
  auto it = label_map.find(label_id);
  if (it == label_map.end())
    throw LoadError(fmt::format("Undefined label: {}", label_id));
  return it->second;
}

const FunctionRecord& Program::FunctionByLabel(uint64_t label_id) const {
  auto it = function_label_map.find(label_id);
  if (it == function_label_map.end())
    throw LoadError(
        fmt::format("No function found for label: {}", label_id));
  return functions[it->second];
}

const FunctionRecord& Program::FunctionByName(const std::string& name) const {
  auto it = function_name_map.find(name);
  if (it == function_name_map.end())
    throw LoadError("Function not found: " + name);
  return functions[it->second];
}

// ---- RttiTable --------------------------------------------------------------

void RttiTable::Register(RttiEntry entry) {
  uint32_t id = std::visit([](auto& e) { return e.id; }, entry);
  entries_[id] = std::move(entry);
}

bool RttiTable::Has(uint32_t type_id) const {
  return entries_.count(type_id) > 0;
}

const RttiEntry& RttiTable::Lookup(uint32_t type_id) const {
  auto it = entries_.find(type_id);
  if (it == entries_.end())
    throw RuntimeError(
        fmt::format("Unknown type_id in RTTI: {}", type_id));
  return it->second;
}

bool RttiTable::IsPrimitive(uint32_t type_id) const {
  return Has(type_id) && std::holds_alternative<PrimitiveRtti>(Lookup(type_id));
}

bool RttiTable::IsRecord(uint32_t type_id) const {
  return Has(type_id) && std::holds_alternative<RecordRtti>(Lookup(type_id));
}

bool RttiTable::IsArray(uint32_t type_id) const {
  return Has(type_id) && std::holds_alternative<ArrayRtti>(Lookup(type_id));
}

PrimitiveKind RttiTable::GetPrimitiveKind(uint32_t type_id) const {
  return std::get<PrimitiveRtti>(Lookup(type_id)).kind;
}

void RttiTable::RegisterBuiltinPrimitives() {
  Register(PrimitiveRtti{kIntegerTypeId, PrimitiveKind::kInteger});
  Register(PrimitiveRtti{kRealTypeId,    PrimitiveKind::kReal});
  Register(PrimitiveRtti{kBooleanTypeId, PrimitiveKind::kBoolean});
}

// ---- Value ------------------------------------------------------------------

std::string Value::TypeName() const {
  if (IsInteger()) return "integer";
  if (IsReal())    return "real";
  if (IsBoolean()) return "boolean";
  if (IsAddress()) return "<address>";
  return fmt::format("<type_id={}>", type_id);
}

// ---- Vm constructor ---------------------------------------------------------

Vm::Vm(Program program) : program_(std::move(program)) {
  globals_.resize(program_.global_count);
}

// ---- Evaluation stack -------------------------------------------------------

void Vm::Push(Value v) {
  eval_stack_.push_back(v);
}

Value Vm::Pop() {
  if (eval_stack_.empty())
    throw StackError("Pop on empty evaluation stack");
  Value v = eval_stack_.back();
  eval_stack_.pop_back();
  return v;
}

Value& Vm::Top() {
  if (eval_stack_.empty())
    throw StackError("Top on empty evaluation stack");
  return eval_stack_.back();
}

const Value& Vm::Top() const {
  if (eval_stack_.empty())
    throw StackError("Top on empty evaluation stack");
  return eval_stack_.back();
}

// ---- Variable access --------------------------------------------------------

void Vm::EnsureLocal(uint16_t index) {
  auto& locals = call_stack_.back().locals;
  if (locals.size() <= index)
    locals.resize(static_cast<std::size_t>(index) + 1);
}

Value& Vm::GetGlobal(uint16_t index) {
  if (index >= globals_.size())
    throw RuntimeError(
        fmt::format("Global index {} out of bounds (size={})",
                    index, globals_.size()));
  return globals_[index];
}

Value& Vm::GetLocal(uint16_t index) {
  auto& locals = call_stack_.back().locals;
  if (index >= locals.size())
    locals.resize(static_cast<std::size_t>(index) + 1);
  return locals[index];
}

Value& Vm::GetArgument(uint16_t index) {
  auto& args = call_stack_.back().arguments;
  if (index >= args.size())
    throw RuntimeError(
        fmt::format("Argument index {} out of bounds (count={})",
                    index, args.size()));
  return args[index];
}

Value& Vm::GetVariable(LocationKind kind, uint16_t index) {
  switch (kind) {
    case LocationKind::kGlobal:   return GetGlobal(index);
    case LocationKind::kLocal:    return GetLocal(index);
    case LocationKind::kArgument: return GetArgument(index);
  }
  throw RuntimeError("Invalid LocationKind");
}

void Vm::SetVariable(LocationKind kind, uint16_t index, Value v) {
  GetVariable(kind, index) = v;
}

// ---- Address operations -----------------------------------------------------

Value Vm::MakeVarAddress(LocationKind kind, uint16_t index) {
  auto* desc = address_pool_.Intern(AddressDescriptor::Variable(kind, index));
  Value v;
  v.type_id = kAddressTypeId;
  v.data    = std::bit_cast<uint64_t>(desc);
  return v;
}

Value Vm::MakeHeapFieldAddress(HeapObject* obj, uint64_t field_idx) {
  auto* desc = address_pool_.Intern(
      AddressDescriptor::HeapField(obj, field_idx));
  Value v;
  v.type_id = kAddressTypeId;
  v.data    = std::bit_cast<uint64_t>(desc);
  return v;
}

Value Vm::LoadAddress(const Value& addr_val) {
  if (!addr_val.IsAddress())
    throw TypeMismatchError("LoadAddress: expected address value");
  auto* desc = std::bit_cast<AddressDescriptor*>(addr_val.data);
  if (desc->kind == AddressDescriptor::Kind::kVariable) {
    return GetVariable(desc->loc_kind, desc->var_index);
  }
  // kHeapField
  if (!desc->object)
    throw NullReferenceError("LoadAddress: null heap reference");
  return desc->object->fields[static_cast<std::size_t>(desc->field_idx)];
}

void Vm::StoreAddress(const Value& addr_val, Value value) {
  if (!addr_val.IsAddress())
    throw TypeMismatchError("StoreAddress: expected address value");
  auto* desc = std::bit_cast<AddressDescriptor*>(addr_val.data);
  if (desc->kind == AddressDescriptor::Kind::kVariable) {
    SetVariable(desc->loc_kind, desc->var_index, value);
    return;
  }
  // kHeapField
  if (!desc->object)
    throw NullReferenceError("StoreAddress: null heap reference");
  desc->object->fields[static_cast<std::size_t>(desc->field_idx)] = value;
}

// ---- Control flow -----------------------------------------------------------

void Vm::Jump(uint64_t label_id) {
  pc_ = program_.ResolveLabel(label_id);
}

void Vm::Call(uint64_t function_label_id) {
  const FunctionRecord& fn = program_.FunctionByLabel(function_label_id);

  // Pop arguments from eval stack (they were pushed left-to-right, so
  // the last argument is on top; pop in reverse to get correct order).
  std::size_t argc = fn.arg_type_ids.size();
  if (eval_stack_.size() < argc)
    throw RuntimeError(
        fmt::format("Call '{}': expected {} arguments on stack, have {}",
                    fn.name, argc, eval_stack_.size()));

  CallFrame frame;
  frame.function_name   = fn.name;
  frame.return_pc       = pc_;
  frame.return_type_id  = fn.return_type_id;
  frame.arguments.resize(argc);
  for (std::size_t i = argc; i-- > 0;)
    frame.arguments[i] = Pop();

  call_stack_.push_back(std::move(frame));
  pc_ = program_.ResolveLabel(function_label_id);
}

void Vm::Return() {
  if (call_stack_.empty())
    throw RuntimeError("Ret: call stack is empty");

  CallFrame frame = std::move(call_stack_.back());
  call_stack_.pop_back();

  pc_ = frame.return_pc;

  // If this was the outermost call (main), halt.
  if (call_stack_.empty()) {
    halted_ = true;
  }
}

void Vm::Halt() {
  halted_ = true;
}

// ---- Heap / GC --------------------------------------------------------------

HeapObject* Vm::AllocRecord(uint32_t type_id, uint64_t num_fields) {
  if (++alloc_count_ % kGcInterval == 0)
    gc_.Collect(*this);
  return gc_.Allocate(type_id, HeapObjectKind::kRecord, num_fields);
}

HeapObject* Vm::AllocArray(uint32_t type_id, uint64_t num_elements) {
  if (++alloc_count_ % kGcInterval == 0)
    gc_.Collect(*this);
  return gc_.Allocate(type_id, HeapObjectKind::kArray, num_elements);
}

// ---- Run --------------------------------------------------------------------

void Vm::Run() {
  const FunctionRecord& main_fn = program_.FunctionByName("main");

  // Push initial call frame for main.
  CallFrame frame;
  frame.function_name  = "main";
  frame.return_pc      = program_.instructions.size();  // sentinel: past end
  frame.return_type_id = main_fn.return_type_id;
  call_stack_.push_back(std::move(frame));

  pc_ = program_.ResolveLabel(main_fn.label_id);

  const auto& table = GetDispatchTable();
  while (!halted_ && pc_ < program_.instructions.size()) {
    const Instruction& instr = program_.instructions[pc_++];
    table[instr.opcode](*this, instr);
  }

  // Final GC.
  gc_.Collect(*this);
}

}  // namespace vm
