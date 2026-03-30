#pragma once

#include <array>
#include <cstdint>
#include <utility>

#include "vm/program.hpp"

namespace vm {

class Vm;

// Signature for opcode handler functions.
using HandlerFn = void (*)(Vm&, const Instruction&);

// Primary template: called for unknown opcodes.
// Specialise this struct for each handled opcode.
template <uint8_t kOpcode>
struct OpcodeHandler {
  static void Execute(Vm& vm, const Instruction& instr);
};

namespace detail {

// Builds the 256-entry dispatch table by instantiating OpcodeHandler<N>
// for every N in [0, 256).  All specialisations must be visible before
// this function is called (i.e. handlers.hpp must be included first in
// the .cpp that defines the dispatch table).
template <std::size_t... Is>
constexpr std::array<HandlerFn, 256> BuildDispatchTable(
    std::index_sequence<Is...>) {
  return {&OpcodeHandler<static_cast<uint8_t>(Is)>::Execute...};
}

}  // namespace detail

// Returns the singleton dispatch table.
// Defined in vm.cpp after including handlers.hpp.
const std::array<HandlerFn, 256>& GetDispatchTable();

}  // namespace vm
