#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

#include "vm/rtti.hpp"
#include "vm/value.hpp"

namespace vm {

// A decoded instruction ready for the execution engine.
// Field values are decoded from little-endian bytes at load time.
struct Instruction {
  uint8_t  opcode     = 0;
  uint8_t  subopcode  = 0;
  uint16_t arg16      = 0;
  uint32_t arg32      = 0;
  uint64_t arg64      = 0;
};

// Metadata for a single function (routine).
struct FunctionRecord {
  std::string          name;
  uint64_t             label_id;      // entry-point label id in the instruction stream
  std::vector<uint32_t> arg_type_ids; // TypeId of each parameter
  uint32_t             return_type_id; // kVoidTypeId for procedures
};

// A fully loaded, ready-to-execute program.
struct Program {
  // Flat list of decoded instructions.
  std::vector<Instruction> instructions;

  // Maps label_id -> instruction index (resolved at load time).
  // Label instructions themselves remain in the stream as no-ops.
  std::unordered_map<uint64_t, std::size_t> label_map;

  // All functions declared in this program.
  std::vector<FunctionRecord> functions;

  // Maps function name -> index into `functions`.
  std::unordered_map<std::string, std::size_t> function_name_map;

  // Maps function label_id -> index into `functions`.
  std::unordered_map<uint64_t, std::size_t> function_label_map;

  // Number of global variables.
  uint32_t global_count = 0;

  // RTTI table.
  RttiTable rtti;

  // Returns the instruction index for a label.
  // Throws LoadError if not found.
  std::size_t ResolveLabel(uint64_t label_id) const;

  // Returns the FunctionRecord for a label_id.
  // Throws LoadError if not found.
  const FunctionRecord& FunctionByLabel(uint64_t label_id) const;

  // Returns the FunctionRecord by name.
  // Throws LoadError if not found.
  const FunctionRecord& FunctionByName(const std::string& name) const;
};

}  // namespace vm
