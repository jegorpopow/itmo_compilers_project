# Compilers Project — Implementation Notes

## Overview

This project implements a compiler and virtual machine for a simple imperative language (`.i` source files).

Pipeline: **Source** → Lexer → Parser → Type Checker → **Codegen** → **Binary `.obj`** → **VM**

The compiler is written in Rust; the VM is written in C++.

---

## Building

### Compiler (Rust)

Requires Rust toolchain via `rustup` (≥ 1.80). The `apt` version of `cargo`/`rustc` is too old and must not be used.

```sh
# From the project root:
cargo build --release
# Binary: ./target/release/compiler
```

### VM (C++)

Requires CMake ≥ 3.25, a C++23 compiler, and system packages:
```sh
sudo apt-get install -y libgtest-dev libgflags-dev libfmt-dev
```

```sh
mkdir -p vm/build_rel
cd vm/build_rel
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
# VM binary:      ./src/vm
# Test binary:    ./tests/vm_tests
```

For a debug + sanitizer build (AddressSanitizer + UBSan):
```sh
mkdir -p vm/build
cd vm/build
cmake .. -DCMAKE_BUILD_TYPE=Debug
make -j$(nproc)
```

---

## Running

### Compiling a source file

```sh
./target/release/compiler path/to/source.i output.obj
# (output path defaults to source.obj if omitted)
```

### Running a compiled program

```sh
./vm/build_rel/src/vm -i output.obj
```

### Running the VM unit tests

```sh
cd vm/build_rel && ctest --output-on-failure
# or directly:
./tests/vm_tests
```

The unit tests include:
- `Print.*` — primitive and compound value formatting
- `Golden.AllPassTests` — compile every `tests/src/¡name!` source and compare VM output against `tests/run/pass/name.stdout`

### Running all golden tests manually

```sh
# From the project root:
for src in tests/src/'¡'*'!'; do
  name=$(basename "$src" | sed 's/^¡//;s/!$//')
  expected="tests/run/pass/${name}.stdout"
  [ ! -f "$expected" ] && continue
  obj="/tmp/vm_test_${name}.obj"
  ./target/release/compiler "$src" "$obj" 2>/dev/null || { echo "COMPILE FAIL: $name"; continue; }
  actual=$(./vm/build_rel/src/vm -i "$obj" 2>&1)
  rm -f "$obj"
  [ "$actual" = "$(cat "$expected")" ] && echo "PASS: $name" || echo "FAIL: $name"
done
```

---

## Binary File Format (`.obj`)

All integers are little-endian.

### Header (36 bytes)

| Offset | Size | Field            | Description                              |
|--------|------|------------------|------------------------------------------|
| 0      | 4    | `magic`          | `0x494D564D` ("MVMI" in LE)              |
| 4      | 4    | `version`        | Must be `1`                              |
| 8      | 4    | `code_offset`    | Byte offset of the instruction stream    |
| 12     | 4    | `code_length`    | Number of instructions                   |
| 16     | 4    | `fn_table_offset`| Byte offset of the function table        |
| 20     | 4    | `fn_table_length`| Number of function records               |
| 24     | 4    | `rtti_offset`    | Byte offset of the RTTI section          |
| 28     | 4    | `rtti_length`    | Number of RTTI entries                   |
| 32     | 4    | `global_count`   | Number of global variables               |

### Instruction Stream

Each instruction is exactly **16 bytes**:

| Offset | Size | Field       |
|--------|------|-------------|
| 0      | 1    | `opcode`    |
| 1      | 1    | `subopcode` |
| 2      | 2    | `arg16`     |
| 4      | 4    | `arg32`     |
| 8      | 8    | `arg64`     |

### Opcode Table

| Opcode | Name              | Fields used                              |
|--------|-------------------|------------------------------------------|
| 1      | Drop              | —                                        |
| 2      | Dup               | —                                        |
| 3      | Swap              | —                                        |
| 4      | BinOp             | subopcode = operator (see below)         |
| 5      | UnOp              | subopcode = operator                     |
| 6      | IntToBool         | —                                        |
| 7      | RealToInt         | —                                        |
| 8      | IntToReal         | —                                        |
| 9      | IntConst          | arg64 = value (i64 as u64)               |
| 10     | RealConst         | arg64 = value (f64 bit-pattern)          |
| 11     | Load              | subopcode = location kind; arg16 = index |
| 12     | Store             | subopcode = location kind; arg16 = index |
| 13     | AddressOf         | subopcode = location kind; arg16 = index |
| 14     | StoreAddress      | —                                        |
| 15     | LoadAddress       | —                                        |
| 16     | AllocRecord       | arg32 = type_id; arg64 = num_fields      |
| 17     | AllocArray        | arg32 = type_id; arg64 = num_elements    |
| 18     | ArraySize         | —                                        |
| 19     | ElementAddress    | —  (pops idx on top, then array ref)     |
| 20     | FieldAddress      | arg64 = field index (0-based)            |
| 21     | Label             | arg64 = label id (no-op at runtime)      |
| 22     | Jump              | arg64 = target label id                  |
| 23     | JumpCond          | subopcode 0=JumpZero, 1=JumpNotZero; arg64 = target label id |
| 24     | Call              | arg64 = function label id                |
| 25     | Ret               | —                                        |
| 26     | Print             | arg32 = type_id (informational)          |
| 27     | Panic             | arg64 = code; arg32 = line; arg16 = col  |
| 28     | NullConst         | —  (pushes null value)                   |
| 29     | DropMany          | arg16 = count                            |
| 30     | AllocArrayDynamic | arg32 = type_id (pops count from stack)  |

**Location kinds** (subopcode for Load/Store/AddressOf):

| Value | Kind     |
|-------|----------|
| 0     | Global   |
| 1     | Local    |
| 2     | Argument |

**BinOp subcodes** (subopcode for opcode 4):

| Range  | Operators                                         |
|--------|---------------------------------------------------|
| 0x00–0x01 | Eq/Ne (any type)                               |
| 0x10–0x17 | Real: Le, Lt, Gt, Ge, Add, Sub, Mul, Div       |
| 0x20–0x28 | Int: Le, Lt, Gt, Ge, Add, Sub, Mul, Div, Mod  |
| 0x30–0x32 | Bool: And, Or, Xor                              |

### Function Table

Immediately follows the instruction stream. Each record:

```
u32  name_length
u8[] name (UTF-8)
u64  label_id
u32  arg_count
u32  arg_type_id[arg_count]
u32  return_type_id
```

### RTTI Section

Each entry starts with a `kind` byte:

**kind = 0 — Primitive**
```
u8   kind = 0
u32  type_id
```
(Type IDs 0–3 are reserved for builtins; only skipped at load time.)

**kind = 1 — Record**
```
u8   kind = 1
u32  type_id
u32  field_count
  [for each field:]
  u32  name_length
  u8[] name (UTF-8)
  u32  field_type_id
```

**kind = 2 — Array**
```
u8   kind = 2
u32  type_id
u32  element_type_id
```

### Built-in Type IDs

| ID | Type    |
|----|---------|
| 0  | Integer |
| 1  | Boolean |
| 2  | Real    |
| 3  | Null    |

User-defined types start at ID 4 and are assigned sequentially by the compiler's interner.

---

## VM Execution Model

- **Eval stack** — the main working stack; also holds local variables.
- **Call frame** — one per active function call; stores `eval_stack_base` (where locals start), `return_pc`, function name, and a copy of the arguments.
- **Locals** live at `eval_stack_[base + index]`. A function call sets `base = eval_stack_.size()` *after* arguments have been popped off.
- **Arguments** are passed by copying them into the `CallFrame.arguments` vector (popped right-to-left: arg[0] is the leftmost argument).
- **Return** pops the top value, truncates the stack back to `base`, then pushes the return value.
- **DropMany(n)** discards the top n elements at block exit to clean up locals.
- **Global variables** live in a separate `globals_` vector sized at load time by `global_count`.
- **Addresses** are interned `AddressDescriptor` objects; they may point to a stack variable (kind, index) or a heap field (object pointer, field index).
- **Heap** — mark-sweep GC triggered every `kGcInterval` allocations. Roots: globals, call-frame arguments, and the entire eval stack.

### Entry Point

The compiler emits a "global init" function at **label 0**. It initialises global variables, then `Call`s `main()`, `Drop`s the return value, pushes `NullConst`, and `Ret`s. The VM always begins execution at label 0.

### Print Format

Matches the reference Rust interpreter:

- Integer: `42`, `-7`
- Real: `3.14`, `11.0` (always has a decimal point), `+Infinity`, `-Infinity`, `NaN`
- Boolean: `true`, `false`
- Null: `null`
- Array: `[ elem1, elem2, ]` (trailing comma, space-padded brackets)
- Record: `{ field1: val1, field2: val2, }` (fields in alphabetical order, trailing comma)
- Cycles: `/* repeated N levels above */`
- Each `print` statement appends a newline.

---

## Current Test Status

Tested with: `cargo build --release` + Release VM build.

### PASS (16 / 32)

arithmetic_operations, comparison_operators, complex_expressions, function_parameters, function_return, identifiers, local_type, nested_control, new_array, operator_precedence, parse_minus, real_comparisons, recursive_function, references, type_aliases, type_conversions

### FAIL (13 / 32)

| Test                | Notes                                                        |
|---------------------|--------------------------------------------------------------|
| arrays_and_records  | Crash: invalid pointer in ElementAddress (see bug below)    |
| conditionals        | Produces only first line of expected multi-line output      |
| constant            | Produces no output                                           |
| deep_conditionals   | Produces only first line                                     |
| for_loops           | Crash: invalid pointer in ElementAddress                    |
| lazy_operators      | Output differs (likely extra/missing boolean coercion)      |
| length              | Output differs                                               |
| logical_operators   | Output differs                                               |
| raytracer           | Produces no output                                           |
| real_literals       | Output differs (formatting edge case)                       |
| records             | Runtime error: FieldAddress: field 0 out of bounds [0,0)    |
| shadow              | Produces only first line                                     |
| variable_declarations | Produces only first line                                  |

### COMPILE FAIL (3 / 32)

| Test            | Notes                                              |
|-----------------|----------------------------------------------------|
| default_init    | Stack overflow in compiler (recursive type)        |
| print           | Stack overflow in compiler (recursive type)        |
| recursive_types | Stack overflow in compiler (recursive type)        |

---

## Known Bugs

### 1. `compile_lvalue_expr` for Member and Index (codegen)

**`LvalueExpression::Member { lhs, member_offset }`** compiles as:
```
compile_lvalue_expr(lhs)  → address of lhs variable
FieldAddress { field_offset }
```
But `FieldAddress` (opcode 20) expects a **heap object reference** on the stack, not a variable address. The `LoadAddress` step to dereference the variable is missing.

**`LvalueExpression::Index { lhs, index }`** compiles as:
```
compile_expr(index)       → index value (pushed first)
compile_lvalue_expr(lhs)  → address of array variable
ElementAddress
```
Two problems: (a) `lhs` is compiled as a variable address rather than an array reference, and (b) the order is inverted — `ElementAddress` pops index first (top of stack) then the array reference.

**Fix** (both in `compiler/codegen/src/lib.rs`):
```rust
LvalueExpression::Member { lhs, member_offset, .. } => {
    self.compile_lvalue_expr(lhs)?;
    self.bytecode.push(Instruction::LoadAddress);          // dereference to get record ref
    self.bytecode.push(Instruction::FieldAddress { field_offset: *member_offset });
}
LvalueExpression::Index { lhs, index } => {
    self.compile_lvalue_expr(lhs)?;
    self.bytecode.push(Instruction::LoadAddress);          // dereference to get array ref
    self.compile_expr(index)?;                             // index goes on top
    self.bytecode.push(Instruction::ElementAddress);
}
```

This fixes: records, arrays_and_records, for_loops, raytracer (and any test accessing fields/elements).

### 2. Boolean literal typing

`Literal::Bool { value }` is compiled as `IntConst { value: value as i64 }`, which produces a value with `type_id = kIntegerTypeId`. When printed, it prints `0`/`1` instead of `false`/`true`. An `IntToBool` instruction must follow the literal to convert to a proper boolean value.

Affected tests: variable_declarations, shadow, conditionals, deep_conditionals, and others that print booleans.

### 3. Stack overflow on recursive types (compiler)

Computing the default initialiser for a recursive record type causes infinite recursion in the compiler. Tests `default_init`, `print`, and `recursive_types` trigger this.

### 4. Minor output mismatches (lazy_operators, logical_operators, real_literals, length)

These likely relate to bugs 2 above or small differences in expression evaluation order / coercion that can be diagnosed after bugs 1 and 2 are fixed.

---

## Project Structure

```
compiler/             Rust compiler
  codegen/src/
    bytecode.rs       Instruction encoding & binary serialisation
    lib.rs            AST → bytecode compiler
  compiler/src/
    main.rs           Entry point (lex → parse → typecheck → codegen → write .obj)

vm/                   C++ virtual machine
  include/vm/
    handlers.hpp      Opcode dispatch table (OpcodeHandler specialisations)
    opcodes.hpp       Opcode / subopcode constants
    program.hpp       Program / Instruction / FunctionRecord structs
    rtti.hpp          Runtime type info (Record, Array, Primitive)
    value.hpp         Value struct (type_id + u64 data)
    vm.hpp            Vm class declaration
    heap.hpp          HeapObject + GarbageCollector
    call_frame.hpp    CallFrame struct
  src/vm/
    vm.cpp            Vm implementation (stack ops, call/return, GC hooks)
    heap.cpp          GC mark-sweep
    loader.cpp        Binary decoder (LoadFromFile + MakeTestProgram)
  tests/
    test_print.cpp    Unit tests for Print opcode
    test_golden.cpp   End-to-end golden tests

tests/
  src/               Source files (named ¡<name>!)
  run/pass/          Expected stdout files (<name>.stdout)
```
