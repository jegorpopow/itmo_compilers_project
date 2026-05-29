#include "vm/vm.hpp"

#include <fmt/format.h>

#include "vm/handlers.hpp"

#include <bit>
#include <variant>

#include "vm/error.hpp"
#include "vm/opcodes.hpp"
#include "vm/rtti.hpp"

namespace vm {

namespace {
const std::array<HandlerFn, 256> kDispatchTable =
    detail::buildDispatchTable(std::make_index_sequence<256>{});
}

const std::array<HandlerFn, 256>& getDispatchTable() {
  return kDispatchTable;
}

std::size_t Program::resolveLabel(uint64_t label_id) const {
  auto it = label_map.find(label_id);
  if (it == label_map.end())
    throw LoadError(fmt::format("Undefined label: {}", label_id));
  return it->second;
}

const FunctionRecord& Program::functionByLabel(uint64_t label_id) const {
  auto it = function_label_map.find(label_id);
  if (it == function_label_map.end())
    throw LoadError(
        fmt::format("No function found for label: {}", label_id));
  return functions[it->second];
}

const FunctionRecord& Program::functionByName(const std::string& name) const {
  auto it = function_name_map.find(name);
  if (it == function_name_map.end())
    throw LoadError("Function not found: " + name);
  return functions[it->second];
}

void RttiTable::registerEntry(RttiEntry entry) {
  uint32_t id = std::visit([](auto& e) { return e.id; }, entry);
  entries_[id] = std::move(entry);
}

bool RttiTable::has(uint32_t type_id) const {
  return entries_.count(type_id) > 0;
}

const RttiEntry& RttiTable::lookup(uint32_t type_id) const {
  auto it = entries_.find(type_id);
  if (it == entries_.end())
    throw RuntimeError(
        fmt::format("Unknown type_id in RTTI: {}", type_id));
  return it->second;
}

bool RttiTable::isPrimitive(uint32_t type_id) const {
  return has(type_id) && std::holds_alternative<PrimitiveRtti>(lookup(type_id));
}

bool RttiTable::isRecord(uint32_t type_id) const {
  return has(type_id) && std::holds_alternative<RecordRtti>(lookup(type_id));
}

bool RttiTable::isArray(uint32_t type_id) const {
  return has(type_id) && std::holds_alternative<ArrayRtti>(lookup(type_id));
}

PrimitiveKind RttiTable::getPrimitiveKind(uint32_t type_id) const {
  return std::get<PrimitiveRtti>(lookup(type_id)).kind;
}

void RttiTable::registerBuiltinPrimitives() {
  registerEntry(PrimitiveRtti{kIntegerTypeId, PrimitiveKind::kInteger});
  registerEntry(PrimitiveRtti{kBooleanTypeId, PrimitiveKind::kBoolean});
  registerEntry(PrimitiveRtti{kRealTypeId,    PrimitiveKind::kReal});
}

std::string Value::typeName() const {
  if (isInteger()) return "integer";
  if (isReal())    return "real";
  if (isBoolean()) return "boolean";
  if (isAddress()) return "<address>";
  return fmt::format("<type_id={}>", type_id);
}

Vm::Vm(Program program) : program_(std::move(program)) {
  globals_.resize(program_.global_count);
}

void Vm::push(Value v) {
  eval_stack_.push_back(v);
}

Value Vm::pop() {
  if (eval_stack_.empty())
    throw StackError("Pop on empty evaluation stack");
  Value v = eval_stack_.back();
  eval_stack_.pop_back();
  return v;
}

Value& Vm::top() {
  if (eval_stack_.empty())
    throw StackError("Top on empty evaluation stack");
  return eval_stack_.back();
}

const Value& Vm::top() const {
  if (eval_stack_.empty())
    throw StackError("Top on empty evaluation stack");
  return eval_stack_.back();
}

Value& Vm::getGlobal(uint16_t index) {
  if (index >= globals_.size())
    throw RuntimeError(
        fmt::format("Global index {} out of bounds (size={})",
                    index, globals_.size()));
  return globals_[index];
}

Value& Vm::getLocal(uint16_t index) {
  std::size_t base = call_stack_.back().eval_stack_base;
  std::size_t abs  = base + static_cast<std::size_t>(index);
  if (abs >= eval_stack_.size())
    eval_stack_.resize(abs + 1);
  return eval_stack_[abs];
}

Value& Vm::getArgument(uint16_t index) {
  auto& args = call_stack_.back().arguments;
  if (index >= args.size())
    throw RuntimeError(
        fmt::format("Argument index {} out of bounds (count={})",
                    index, args.size()));
  return args[index];
}

Value& Vm::getVariable(LocationKind kind, uint16_t index) {
  switch (kind) {
    case LocationKind::kGlobal:   return getGlobal(index);
    case LocationKind::kLocal:    return getLocal(index);
    case LocationKind::kArgument: return getArgument(index);
  }
  throw RuntimeError("Invalid LocationKind");
}

void Vm::setVariable(LocationKind kind, uint16_t index, Value v) {
  getVariable(kind, index) = v;
}

Value Vm::makeVarAddress(LocationKind kind, uint16_t index) {
  return makeVariableAddressValue(kind, index);
}

Value Vm::makeHeapFieldAddress(HeapObject* obj, uint64_t field_idx) {
  return makeHeapFieldAddressValue(obj, field_idx);
}

Value Vm::loadAddress(const Value& addr_val) {
  if (!addr_val.isAddress())
    throw TypeMismatchError("LoadAddress: expected address value");
  if (!isHeapFieldAddress(addr_val))
    return getVariable(variableAddressKind(addr_val),
                       variableAddressIndex(addr_val));
  HeapObject* obj = heapFieldObject(addr_val);
  if (!obj)
    throw NullReferenceError("LoadAddress: null heap reference");
  return obj->fields[static_cast<std::size_t>(heapFieldIndex(addr_val))];
}

void Vm::storeAddress(const Value& addr_val, Value value) {
  if (!addr_val.isAddress())
    throw TypeMismatchError("StoreAddress: expected address value");
  if (!isHeapFieldAddress(addr_val)) {
    setVariable(variableAddressKind(addr_val),
                variableAddressIndex(addr_val), value);
    return;
  }
  HeapObject* obj = heapFieldObject(addr_val);
  if (!obj)
    throw NullReferenceError("StoreAddress: null heap reference");
  obj->fields[static_cast<std::size_t>(heapFieldIndex(addr_val))] = value;
}

void Vm::jump(uint64_t label_id) {
  pc_ = program_.resolveLabel(label_id);
}

void Vm::call(uint64_t function_label_id) {
  const FunctionRecord& fn = program_.functionByLabel(function_label_id);

  std::size_t argc = fn.arg_type_ids.size();
  if (eval_stack_.size() < argc)
    throw RuntimeError(
        fmt::format("Call '{}': expected {} arguments on stack, have {}",
                    fn.name, argc, eval_stack_.size()));

  CallFrame frame;
  frame.function_name  = fn.name;
  frame.return_pc      = pc_;
  frame.return_type_id = fn.return_type_id;
  frame.arguments.resize(argc);
  for (std::size_t i = argc; i-- > 0;)
    frame.arguments[i] = pop();

  frame.eval_stack_base = eval_stack_.size();
  call_stack_.push_back(std::move(frame));
  pc_ = program_.resolveLabel(function_label_id);
}

void Vm::ret() {
  if (call_stack_.empty())
    throw RuntimeError("Ret: call stack is empty");

  CallFrame frame = std::move(call_stack_.back());
  call_stack_.pop_back();

  Value ret = pop();
  eval_stack_.resize(frame.eval_stack_base);
  push(ret);

  pc_ = frame.return_pc;

  if (call_stack_.empty())
    halted_ = true;
}

void Vm::halt() {
  halted_ = true;
}

void Vm::maybeCollect() {
  if (gc_.objectCount() < next_gc_at_) return;
  gc_.collect(*this);
  std::size_t live = gc_.objectCount();
  next_gc_at_ = live * 2 > kMinGcThreshold ? live * 2 : kMinGcThreshold;
}

HeapObject* Vm::allocRecord(uint32_t type_id, uint64_t num_fields) {
  maybeCollect();
  return gc_.allocate(type_id, HeapObjectKind::kRecord, num_fields);
}

HeapObject* Vm::allocArray(uint32_t type_id, uint64_t num_elements) {
  maybeCollect();
  Value default_val{.type_id = kNullTypeId};
  if (program_.rtti.isArray(type_id)) {
    uint32_t elem_tid = std::get<ArrayRtti>(program_.rtti.lookup(type_id)).element_type_id;
    if (program_.rtti.isPrimitive(elem_tid)) {
      switch (program_.rtti.getPrimitiveKind(elem_tid)) {
        case PrimitiveKind::kInteger: default_val = Value::makeInteger(0); break;
        case PrimitiveKind::kBoolean: default_val = Value::makeBoolean(false); break;
        case PrimitiveKind::kReal:    default_val = Value::makeReal(0.0); break;
      }
    }
  }
  return gc_.allocate(type_id, HeapObjectKind::kArray, num_elements, default_val);
}

void Vm::run() {
  CallFrame frame;
  frame.function_name   = "<global_init>";
  frame.return_pc       = program_.instructions.size();
  frame.return_type_id  = kVoidTypeId;
  frame.eval_stack_base = 0;
  call_stack_.push_back(std::move(frame));

  pc_ = program_.resolveLabel(0);

  const auto& table = getDispatchTable();
  while (!halted_ && pc_ < program_.instructions.size()) {
    const Instruction& instr = program_.instructions[pc_++];
    table[instr.opcode](*this, instr);
  }

  gc_.collect(*this);
}

}
