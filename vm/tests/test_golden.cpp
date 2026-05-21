#include <gtest/gtest.h>

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "vm/loader.hpp"
#include "vm/vm.hpp"

namespace fs = std::filesystem;

static const fs::path kCompilerSrc  = COMPILER_SRC_DIR;
static const fs::path kTestsSrcDir  = TESTS_SRC_DIR;
static const fs::path kTestsPassDir = TESTS_PASS_DIR;

static fs::path FindCompilerBinary() {
  for (const fs::path& p : {
           kCompilerSrc / "target" / "release" / "compiler",
           kCompilerSrc / "target" / "debug"   / "compiler",
           kCompilerSrc.parent_path() / "target" / "release" / "compiler",
           kCompilerSrc.parent_path() / "target" / "debug"   / "compiler",
       })
    if (fs::exists(p)) return p;
  return {};
}

static std::string ReadFile(const fs::path& p) {
  std::ifstream f(p);
  std::ostringstream ss;
  ss << f.rdbuf();
  return ss.str();
}

struct GoldenCase {
  std::string name;
  fs::path    source;
  fs::path    expected;
};

static std::vector<GoldenCase> CollectCases() {
  std::vector<GoldenCase> cases;
  if (!fs::is_directory(kTestsPassDir)) return cases;
  for (const auto& entry : fs::directory_iterator(kTestsPassDir)) {
    if (entry.path().extension() != ".stdout") continue;
    std::string name = entry.path().stem().string();
    fs::path src = kTestsSrcDir / ("\xc2\xa1" + name + "!");
    if (!fs::exists(src)) continue;
    cases.push_back({name, src, entry.path()});
  }
  return cases;
}

TEST(Golden, AllPassTests) {
  fs::path compiler = FindCompilerBinary();
  if (compiler.empty())
    GTEST_SKIP() << "compiler binary not found; run `cargo build --release` first";

  std::vector<GoldenCase> cases = CollectCases();
  if (cases.empty())
    GTEST_SKIP() << "no golden test cases found";

  for (const GoldenCase& tc : cases) {
    SCOPED_TRACE(tc.name);

    fs::path obj = fs::temp_directory_path() /
                   ("vm_golden_" + tc.name + ".obj");

    std::string cmd = compiler.string() + " " +
                      tc.source.string() + " " +
                      obj.string() + " 2>/dev/null";
    int rc = std::system(cmd.c_str());
    if (rc != 0) {
      ADD_FAILURE() << "compiler failed (exit " << rc << ") for: " << tc.name;
      continue;
    }

    std::string actual;
    try {
      vm::Program prog = vm::Loader::LoadFromFile(obj);
      vm::Vm machine(std::move(prog));
      testing::internal::CaptureStdout();
      machine.Run();
      actual = testing::internal::GetCapturedStdout();
    } catch (const std::exception& e) {
      ADD_FAILURE() << tc.name << ": VM threw: " << e.what();
      fs::remove(obj);
      continue;
    }

    fs::remove(obj);

    EXPECT_EQ(actual, ReadFile(tc.expected)) << "mismatch for: " << tc.name;
  }
}
