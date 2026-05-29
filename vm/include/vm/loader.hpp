#pragma once

#include <filesystem>
#include <string>

#include "vm/program.hpp"

namespace vm {

class Loader {
 public:
  static Program loadFromFile(const std::filesystem::path& path);

  static Program makeTestProgram(std::vector<Instruction> instructions,
                                 uint32_t global_count = 0);
};

}
