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

// ---- Binary format helpers --------------------------------------------------

constexpr uint32_t kMagic   = 0x494D564Du;  // "IMVM"
constexpr uint32_t kVersion = 1;

struct RawInstruction {
  uint8_t  opcode;
  uint8_t  subopcode;
  uint8_t  arg16[2];
  uint8_t  arg32[4];
  uint8_t  arg64[8];
};
static_assert(sizeof(RawInstruction) == 16);

Instruction DecodeInstruction(const RawInstruction& raw) {
  Instruction instr;
  instr.opcode    = raw.opcode;
  instr.subopcode = raw.subopcode;
  std::memcpy(&instr.arg16, raw.arg16, 2);
  std::memcpy(&instr.arg32, raw.arg32, 4);
  std::memcpy(&instr.arg64, raw.arg64, 8);
  return instr;
}

struct FileHeader {
  uint32_t magic;
  uint32_t version;
  uint32_t code_offset;
  uint32_t code_length;      // number of instructions
  uint32_t fn_table_offset;
  uint32_t fn_table_length;  // number of function records
  uint32_t rtti_offset;
  uint32_t rtti_length;      // number of RTTI entries
  uint32_t global_count;
};

void BuildLabelMap(Program& prog) {
  for (std::size_t i = 0; i < prog.instructions.size(); ++i) {
    if (prog.instructions[i].opcode ==
        static_cast<uint8_t>(Opcode::kLabel)) {
      uint64_t id = prog.instructions[i].arg64;
      prog.label_map[id] = i;
    }
  }
}

void BuildFunctionMaps(Program& prog) {
  for (std::size_t i = 0; i < prog.functions.size(); ++i) {
    prog.function_name_map[prog.functions[i].name]              = i;
    prog.function_label_map[prog.functions[i].label_id]        = i;
  }
}

// ---- Read helpers -----------------------------------------------------------

template <typename T>
T ReadLE(const std::vector<uint8_t>& buf, std::size_t& pos) {
  T val{};
  std::memcpy(&val, buf.data() + pos, sizeof(T));
  pos += sizeof(T);
  return val;
}

std::string ReadString(const std::vector<uint8_t>& buf, std::size_t& pos) {
  uint32_t len = ReadLE<uint32_t>(buf, pos);
  std::string s(reinterpret_cast<const char*>(buf.data() + pos), len);
  pos += len;
  return s;
}

}  // namespace

// ---- Loader::LoadFromFile ---------------------------------------------------
//
// TODO: This is a stub implementation.  It reads the binary format described
// in loader.hpp but has not been tested against real compiler output since the
// compiler is not yet complete.  Extend / fix once the compiler emits .obj
// files.

Program Loader::LoadFromFile(const std::filesystem::path& path) {
  std::ifstream file(path, std::ios::binary | std::ios::ate);
  if (!file)
    throw LoadError("Cannot open file: " + path.string());

  std::streamsize size = file.tellg();
  file.seekg(0);
  std::vector<uint8_t> buf(static_cast<std::size_t>(size));
  if (!file.read(reinterpret_cast<char*>(buf.data()), size))
    throw LoadError("Failed to read file: " + path.string());

  if (buf.size() < sizeof(FileHeader))
    throw LoadError("File too small to contain a valid header");

  std::size_t pos = 0;
  FileHeader hdr{};
  std::memcpy(&hdr, buf.data(), sizeof(hdr));
  pos = sizeof(hdr);

  if (hdr.magic != kMagic)
    throw LoadError(fmt::format("Bad magic: expected 0x{:08X}, got 0x{:08X}",
                                kMagic, hdr.magic));
  if (hdr.version != kVersion)
    throw LoadError(
        fmt::format("Unsupported version: {}", hdr.version));

  Program prog;
  prog.global_count = hdr.global_count;
  prog.rtti.RegisterBuiltinPrimitives();

  // ---- Instructions --------------------------------------------------------
  pos = hdr.code_offset;
  prog.instructions.reserve(hdr.code_length);
  for (uint32_t i = 0; i < hdr.code_length; ++i) {
    RawInstruction raw{};
    std::memcpy(&raw, buf.data() + pos, sizeof(RawInstruction));
    pos += sizeof(RawInstruction);
    prog.instructions.push_back(DecodeInstruction(raw));
  }

  // ---- Function table ------------------------------------------------------
  pos = hdr.fn_table_offset;
  prog.functions.reserve(hdr.fn_table_length);
  for (uint32_t i = 0; i < hdr.fn_table_length; ++i) {
    FunctionRecord fn;
    fn.name           = ReadString(buf, pos);
    fn.label_id       = ReadLE<uint64_t>(buf, pos);
    uint32_t argc     = ReadLE<uint32_t>(buf, pos);
    fn.arg_type_ids.resize(argc);
    for (uint32_t j = 0; j < argc; ++j)
      fn.arg_type_ids[j] = ReadLE<uint32_t>(buf, pos);
    fn.return_type_id = ReadLE<uint32_t>(buf, pos);
    prog.functions.push_back(std::move(fn));
  }

  // ---- RTTI ----------------------------------------------------------------
  // kind: 0=Primitive, 1=Record, 2=Array
  pos = hdr.rtti_offset;
  for (uint32_t i = 0; i < hdr.rtti_length; ++i) {
    uint8_t  kind    = ReadLE<uint8_t>(buf, pos);
    uint32_t type_id = ReadLE<uint32_t>(buf, pos);
    if (kind == 0) {
      // Primitive: already registered via RegisterBuiltinPrimitives.
      // Just skip; the compiler assigns the well-known IDs.
    } else if (kind == 1) {
      RecordRtti rec;
      rec.id = type_id;
      uint32_t fc = ReadLE<uint32_t>(buf, pos);
      rec.field_type_ids.resize(fc);
      for (uint32_t j = 0; j < fc; ++j)
        rec.field_type_ids[j] = ReadLE<uint32_t>(buf, pos);
      prog.rtti.Register(std::move(rec));
    } else if (kind == 2) {
      ArrayRtti arr;
      arr.id              = type_id;
      arr.element_type_id = ReadLE<uint32_t>(buf, pos);
      prog.rtti.Register(std::move(arr));
    } else {
      throw LoadError(fmt::format("Unknown RTTI kind: {}", kind));
    }
  }

  BuildLabelMap(prog);
  BuildFunctionMaps(prog);
  return prog;
}

// ---- Loader::MakeTestProgram ------------------------------------------------

Program Loader::MakeTestProgram(std::vector<Instruction> instructions,
                                uint32_t global_count) {
  Program prog;
  prog.instructions  = std::move(instructions);
  prog.global_count  = global_count;
  prog.rtti.RegisterBuiltinPrimitives();

  BuildLabelMap(prog);

  // Create a synthetic "main" function starting at label 0.
  FunctionRecord fn;
  fn.name           = "main";
  fn.label_id       = 0;
  fn.return_type_id = kVoidTypeId;
  prog.functions.push_back(fn);
  BuildFunctionMaps(prog);

  return prog;
}

}  // namespace vm
