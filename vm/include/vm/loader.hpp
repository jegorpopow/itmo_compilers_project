#pragma once

#include <filesystem>
#include <string>

#include "vm/program.hpp"

namespace vm {

// Loads a compiled program from a binary .obj file.
//
// Binary format (all integers little-endian):
//
//   Header (fixed size):
//     magic           : u32  = 0x494D564D ("IMVM")
//     version         : u32  = 1
//     code_offset     : u32  — byte offset of instruction array
//     code_length     : u32  — number of instructions (each 16 bytes)
//     fn_table_offset : u32  — byte offset of function table
//     fn_table_length : u32  — number of function records
//     rtti_offset     : u32  — byte offset of RTTI table
//     rtti_length     : u32  — number of RTTI entries
//     global_count    : u32
//
//   Instructions (16 bytes each):
//     [opcode:u8][subopcode:u8][arg16:u16le][arg32:u32le][arg64:u64le]
//
//   Function table entries (variable length, UTF-8 strings):
//     name_len  : u32
//     name      : name_len bytes
//     label_id  : u64le
//     arg_count : u32
//     arg_types : arg_count * u32le
//     return_type : u32le
//
//   RTTI entries (variable length):
//     kind      : u8  (0=Primitive, 1=Record, 2=Array)
//     type_id   : u32le
//     [kind-specific fields — see implementation]
//
// NOTE: This is a stub. The compiler is not yet complete; the binary
// format above is a draft and will be finalised when the compiler
// emits bytecode. Until then, programs are constructed programmatically
// for testing purposes.
class Loader {
 public:
  // Loads a program from the given file.
  // Throws LoadError on any format or I/O error.
  static Program LoadFromFile(const std::filesystem::path& path);

  // Constructs a minimal Program from a flat instruction vector and
  // a single "main" function starting at label 0.
  // Used by unit tests to bypass the binary format.
  static Program MakeTestProgram(std::vector<Instruction> instructions,
                                 uint32_t global_count = 0);
};

}  // namespace vm
