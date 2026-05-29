#pragma once

#include <array>
#include <cstdint>
#include <utility>

#include "vm/opcodes.hpp"
#include "vm/program.hpp"

namespace vm {

class Vm;

using HandlerFn = void (*)(Vm&, const Instruction&);

template <Opcode kOpcode>
struct OpcodeHandler {
  static void execute(Vm& vm, const Instruction& instr);
};

namespace detail {

template <std::size_t... Is>
constexpr std::array<HandlerFn, 256> buildDispatchTable(
    std::index_sequence<Is...>) {
  return {&OpcodeHandler<static_cast<Opcode>(Is)>::execute...};
}

}

const std::array<HandlerFn, 256>& getDispatchTable();

}
