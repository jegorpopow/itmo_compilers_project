#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

#include "vm/rtti.hpp"
#include "vm/value.hpp"

namespace vm {

struct Instruction {
  uint8_t  opcode     = 0;
  uint8_t  subopcode  = 0;
  uint16_t arg16      = 0;
  uint32_t arg32      = 0;
  uint64_t arg64      = 0;
};

struct FunctionRecord {
  std::string          name;
  uint64_t             label_id;
  std::vector<uint32_t> arg_type_ids;
  uint32_t             return_type_id;
};

struct Program {
  std::vector<Instruction> instructions;

  std::unordered_map<uint64_t, std::size_t> label_map;

  std::vector<FunctionRecord> functions;

  std::unordered_map<std::string, std::size_t> function_name_map;

  std::unordered_map<uint64_t, std::size_t> function_label_map;

  uint32_t global_count = 0;

  RttiTable rtti;

  std::size_t resolveLabel(uint64_t label_id) const;

  const FunctionRecord& functionByLabel(uint64_t label_id) const;

  const FunctionRecord& functionByName(const std::string& name) const;
};

}
