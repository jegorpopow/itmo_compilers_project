#include <filesystem>
#include <iostream>

#include <gflags/gflags.h>

#include "vm/error.hpp"
#include "vm/loader.hpp"
#include "vm/vm.hpp"

DEFINE_string(i, "", "Path to the compiled .obj bytecode file");

int main(int argc, char** argv) {
  gflags::SetUsageMessage("Usage: vm -i <program.obj>");
  gflags::ParseCommandLineFlags(&argc, &argv, true);

  if (FLAGS_i.empty()) {
    std::cerr << "Error: -i <file> is required\n";
    gflags::ShowUsageWithFlags(argv[0]);
    return 2;
  }

  std::filesystem::path path(FLAGS_i);

  try {
    vm::Program prog = vm::Loader::LoadFromFile(path);
    vm::Vm      machine(std::move(prog));
    machine.Run();
  } catch (const vm::LoadError& e) {
    std::cerr << "Load error: " << e.what() << '\n';
    return 1;
  } catch (const vm::PanicError& e) {
    std::cerr << "Program panicked (code " << e.code() << "): "
              << e.what() << '\n';
    return 1;
  } catch (const vm::RuntimeError& e) {
    std::cerr << "Runtime error: " << e.what() << '\n';
    return 1;
  } catch (const vm::VmError& e) {
    std::cerr << "VM error: " << e.what() << '\n';
    return 1;
  }

  return 0;
}
