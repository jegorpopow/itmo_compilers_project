#include "vm/loader.hpp"

#include <cstring>
#include <fmt/format.h>
#include <fstream>
#include <vector>

#include "vm/error.hpp"
#include "vm/opcodes.hpp"
#include "vm/value.hpp"

namespace vm {

namespace {

constexpr uint32_t kMagic   = 0x494D564Du;
constexpr uint32_t kVersion = 4;

struct RawInstruction {
  uint8_t opcode;
  uint8_t subopcode;
  uint8_t arg16[2];
  uint8_t arg32[4];
  uint8_t arg64[8];
};
static_assert(sizeof(RawInstruction) == 16);

Instruction decodeInstruction(const RawInstruction& raw) {
  Instruction instr;
  instr.opcode    = raw.opcode;
  instr.subopcode = raw.subopcode;
  std::memcpy(&instr.arg16, raw.arg16, 2);
  std::memcpy(&instr.arg32, raw.arg32, 4);
  std::memcpy(&instr.arg64, raw.arg64, 8);
  return instr;
}

void buildLabelMap(Program& prog) {
  for (std::size_t i = 0; i < prog.instructions.size(); ++i) {
    if (prog.instructions[i].opcode == static_cast<uint8_t>(Opcode::kLabel)) {
      uint64_t id = prog.instructions[i].arg64;
      prog.label_map[id] = i;
    }
  }
}

void buildFunctionMaps(Program& prog) {
  for (std::size_t i = 0; i < prog.functions.size(); ++i) {
    prog.function_name_map[prog.functions[i].name]       = i;
    prog.function_label_map[prog.functions[i].label_id] = i;
  }
}

template <typename T>
T readLE(const std::vector<uint8_t>& buf, std::size_t& pos) {
  if (pos + sizeof(T) > buf.size())
    throw LoadError("File truncated");
  T val{};
  std::memcpy(&val, buf.data() + pos, sizeof(T));
  pos += sizeof(T);
  return val;
}

std::string readString(const std::vector<uint8_t>& buf, std::size_t& pos) {
  uint32_t len = readLE<uint32_t>(buf, pos);
  if (pos + len > buf.size())
    throw LoadError("File truncated in string");
  std::string s(reinterpret_cast<const char*>(buf.data() + pos), len);
  pos += len;
  return s;
}

}

Program Loader::loadFromFile(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary | std::ios::ate);
  if (!file)
    throw LoadError("Cannot open file: " + path.string());

  std::streamsize size = file.tellg();
  file.seekg(0);
  std::vector<uint8_t> buf(static_cast<std::size_t>(size));
  if (!file.read(reinterpret_cast<char*>(buf.data()), size))
    throw LoadError("Failed to read file: " + path.string());

  std::size_t pos = 0;

  uint32_t magic   = readLE<uint32_t>(buf, pos);
  uint32_t version = readLE<uint32_t>(buf, pos);

  if (magic != kMagic)
    throw LoadError(fmt::format("Bad magic: expected 0x{:08X}, got 0x{:08X}",
                                kMagic, magic));
  if (version != kVersion)
    throw LoadError(fmt::format("Unsupported version: {}", version));

  Program prog;
  prog.global_count = readLE<uint32_t>(buf, pos);

  uint32_t rtti_count = readLE<uint32_t>(buf, pos);
  for (uint32_t i = 0; i < rtti_count; ++i) {
    uint8_t kind = readLE<uint8_t>(buf, pos);
    if (kind == 0) {
      uint8_t type_byte = readLE<uint8_t>(buf, pos);
      switch (type_byte) {
        case 1:
          prog.rtti.registerEntry(PrimitiveRtti{i, PrimitiveKind::kInteger});
          break;
        case 2:
          prog.rtti.registerEntry(PrimitiveRtti{i, PrimitiveKind::kBoolean});
          break;
        case 3:
          prog.rtti.registerEntry(PrimitiveRtti{i, PrimitiveKind::kReal});
          break;
        default:
          break;
      }
    } else if (kind == 1) {
      RecordRtti rec;
      rec.id = i;
      uint32_t fc = readLE<uint32_t>(buf, pos);
      rec.field_names.resize(fc);
      rec.field_type_ids.resize(fc);
      for (uint32_t j = 0; j < fc; ++j) {
        rec.field_names[j]    = readString(buf, pos);
        rec.field_type_ids[j] = readLE<uint32_t>(buf, pos);
      }
      prog.rtti.registerEntry(std::move(rec));
    } else if (kind == 2) {
      ArrayRtti arr;
      arr.id              = i;
      arr.element_type_id = readLE<uint32_t>(buf, pos);
      prog.rtti.registerEntry(std::move(arr));
    } else {
      throw LoadError(fmt::format("Unknown RTTI kind: {}", kind));
    }
  }

  uint32_t fn_count = readLE<uint32_t>(buf, pos);
  prog.functions.reserve(fn_count);
  for (uint32_t i = 0; i < fn_count; ++i) {
    FunctionRecord fn;
    fn.name           = readString(buf, pos);
    fn.label_id       = readLE<uint64_t>(buf, pos);
    uint32_t argc     = readLE<uint32_t>(buf, pos);
    fn.arg_type_ids.resize(argc);
    for (uint32_t j = 0; j < argc; ++j)
      fn.arg_type_ids[j] = readLE<uint32_t>(buf, pos);
    fn.return_type_id = readLE<uint32_t>(buf, pos);
    prog.functions.push_back(std::move(fn));
  }

  uint32_t instr_count = readLE<uint32_t>(buf, pos);
  prog.instructions.reserve(instr_count);
  for (uint32_t i = 0; i < instr_count; ++i) {
    RawInstruction raw{};
    std::memcpy(&raw, buf.data() + pos, sizeof(RawInstruction));
    pos += sizeof(RawInstruction);
    prog.instructions.push_back(decodeInstruction(raw));
  }

  buildLabelMap(prog);
  buildFunctionMaps(prog);
  return prog;
}

Program Loader::makeTestProgram(std::vector<Instruction> instructions,
                                uint32_t global_count) {
  Program prog;
  prog.instructions = std::move(instructions);
  prog.global_count = global_count;
  prog.rtti.registerBuiltinPrimitives();

  buildLabelMap(prog);

  FunctionRecord fn;
  fn.name           = "main";
  fn.label_id       = 0;
  fn.return_type_id = kVoidTypeId;
  prog.functions.push_back(fn);
  buildFunctionMaps(prog);

  return prog;
}

}
