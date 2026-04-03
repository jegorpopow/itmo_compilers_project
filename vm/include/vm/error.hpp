#pragma once

#include <stdexcept>
#include <string>

namespace vm {

// Base class for all VM errors.
class VmError : public std::runtime_error {
  using std::runtime_error::runtime_error;
};

// Errors that occur during program execution.
class RuntimeError : public VmError {
  using VmError::VmError;
};

// Type system violations detected at runtime.
class TypeMismatchError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// Stack underflow or empty stack access.
class StackError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// Null reference dereference.
class NullReferenceError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// Array index out of bounds.
class IndexOutOfBoundsError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// Division by zero or modulo by zero.
class DivisionByZeroError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// Invalid conversion (e.g. integer not in {0,1} converted to boolean).
class ConversionError : public RuntimeError {
  using RuntimeError::RuntimeError;
};

// FIXME: add position info
// Explicit Panic instruction or fatal VM state.
class PanicError : public RuntimeError {
 public:
  explicit PanicError(uint64_t code)
      : RuntimeError("panic with code " + std::to_string(code)),
        code_(code) {}

  uint64_t code() const { return code_; }

 private:
  uint64_t code_;
};

// Errors in the program structure (e.g. missing main, bad label).
class LoadError : public VmError {
  using VmError::VmError;
};

}  // namespace vm
