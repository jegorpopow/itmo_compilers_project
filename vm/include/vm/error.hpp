#pragma once

#include <stdexcept>
#include <string>

namespace vm {

class VmError : public std::runtime_error {
  using std::runtime_error::runtime_error;
};

class RuntimeError : public VmError {
  using VmError::VmError;
};

class TypeMismatchError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class StackError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class NullReferenceError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class IndexOutOfBoundsError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class DivisionByZeroError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class ConversionError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

class PanicError : public RuntimeError {
 public:
  explicit PanicError(uint64_t code)
      : RuntimeError("panic with code " + std::to_string(code)),
        code_(code) {}

  uint64_t code() const { return code_; }

 private:
  uint64_t code_;
};

class LoadError : public VmError {
  using VmError::VmError;
};

}
