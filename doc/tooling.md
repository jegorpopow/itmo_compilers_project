# Tooling Guide

This document describes how to build and run every tool in the project: the
`¡` compiler (Rust), the bytecode virtual machine (C++), the test suites, and
how to render the raytracer demo to an image.

```
.
├── compiler/     Rust workspace: lexer, parser, ast, codegen, bytecode, compiler (CLI), repl
├── vm/           C++ bytecode virtual machine (CMake)
├── tests/        Test fixtures: src/ (programs) + per-stage golden outputs
├── xtask/        Dev tasks runner (cargo x ...)
└── run_golden_tests.py   End-to-end compiler+VM golden test runner
```

The pipeline is:

```
source (tests/src/¡name!)  --[compiler]-->  program.obj  --[vm]-->  stdout
```

---

## 1. Prerequisites

### Rust (compiler, xtask)

A recent Rust toolchain (edition 2024, rustc ≥ 1.95). Install via
[rustup](https://rustup.rs/):

```bash
rustup update stable
```

### C++ (virtual machine)

- A C++23 compiler (GCC ≥ 13 or Clang ≥ 16)
- CMake ≥ 3.25
- Libraries (found via `find_package`): **GoogleTest** ≥ 1.12, **gflags** ≥ 2.2.2, **fmt** ≥ 9

On Debian/Ubuntu:

```bash
sudo apt install build-essential cmake libgtest-dev libgflags-dev libfmt-dev
```

### Image rendering (optional)

The raytracer-output → PNG converter is built into `xtask` (`cargo x render`),
so no extra dependency is required.

---

## 2. The compiler

### Build

```bash
cargo build --release          # builds the whole workspace
```

The compiler binary lands at `target/release/compiler`
(use `target/debug/compiler` for an unoptimized `cargo build`).

### Run

The compiler takes an input source file and an optional output path. If the
output is omitted, it writes next to the input with the `.obj` extension.

```bash
# explicit output
target/release/compiler path/to/program.src program.obj

# implicit output -> path/to/program.obj
target/release/compiler path/to/program.src
```

Via cargo (no need to locate the binary):

```bash
cargo run --release -p compiler -- path/to/program.src program.obj
```

The compiler runs lexing → parsing → typecheck → codegen → bytecode
serialization. Any failing stage reports an error and a non-zero exit code.

---

## 3. Compiling a test program

Test programs live in `tests/src/` and are named `¡<name>!`
(an inverted exclamation mark, the stem, and a trailing exclamation mark),
for example `tests/src/¡arithmetic_operations!`.

```bash
# compile a fixture into an .obj
cargo run --release -p compiler -- "tests/src/¡arithmetic_operations!" /tmp/arith.obj
```

### Rust test suite

The Rust crates have unit/golden tests for each compiler stage
(`tests/lexer`, `tests/parser`, `tests/ast`, `tests/codegen`, ...):

```bash
cargo test                     # run all workspace tests
cargo test -p codegen          # run tests for a single crate
```

### `cargo x` dev tasks

`cargo x` is an alias for `cargo run -p xtask --`:

```bash
cargo x add-test <name> [fail-stage]   # scaffold a new test case
cargo x update-listings                # refresh test listings from tests/src/
cargo x render <input> <output.png>    # see section 6
```

---

## 4. The virtual machine

The VM is a standalone CMake project under `vm/`.

> **Build type matters a lot.** A `Debug` build compiles with `-O0` and turns on
> `_GLIBCXX_DEBUG` (bounds-checked `std::vector`), which makes execution roughly
> **~70× slower**. Always use a **Release** build for running real programs;
> use Debug only for development/debugging.

### Configure & build (Release — for running programs)

```bash
cmake -S vm -B vm/build_rel -DCMAKE_BUILD_TYPE=Release
cmake --build vm/build_rel -j
```

Binaries:

- VM executable: `vm/build_rel/src/vm`
- Unit tests:    `vm/build_rel/tests/vm_tests`

### Configure & build (Debug — for development)

```bash
cmake -S vm -B vm/build -DCMAKE_BUILD_TYPE=Debug
cmake --build vm/build -j
```

### Run the VM on a bytecode file

The VM takes the compiler's `.obj` output via `-i`:

```bash
vm/build_rel/src/vm -i program.obj
```

It loads the program, runs `main`, and prints program output to stdout.
Runtime/load errors go to stderr with a non-zero exit code.

### VM unit tests

```bash
cd vm/build_rel && ctest --output-on-failure
# or run the gtest binary directly:
vm/build_rel/tests/vm_tests
```

---

## 5. End-to-end & golden tests

### Manual end-to-end run

```bash
cargo build --release
cmake -S vm -B vm/build_rel -DCMAKE_BUILD_TYPE=Release && cmake --build vm/build_rel -j

target/release/compiler "tests/src/¡print!" /tmp/print.obj
vm/build_rel/src/vm -i /tmp/print.obj
```

### Automated golden tests (compiler + VM)

`run_golden_tests.py` compiles every program in `tests/src/` that has a matching
`tests/run/pass/<name>.stdout`, runs it through the VM, and diffs the output.

It auto-discovers the binaries; build them first:

```bash
cargo build --release
cmake --build vm/build_rel -j     # or vm/build

python3 run_golden_tests.py
```

It looks for the VM at `vm/build/src/vm` (and a couple of fallbacks). If you
only built the Release tree under `vm/build_rel`, either build `vm/build` too or
symlink the binary, e.g.:

```bash
mkdir -p vm/build/src && ln -sf ../../build_rel/src/vm vm/build/src/vm
```

---

## 6. Rendering the raytracer

The raytracer is `tests/src/¡raytracer!`. It prints a tiny text image format:

```
<height>
<width>
{ blue: B, green: G, red: R, }      # height*width pixel lines
...
```

The resolution is set by the last line of the program:
`render_canvas(trace_rays(<height>, <width>, scene))` — default `36 × 64`.

### Recipe (change resolution → compile → run → PNG)

```bash
# 1. make a copy at the desired resolution (e.g. 720p = 1280x720)
sed 's/trace_rays(36, 64, scene)/trace_rays(720, 1280, scene)/' \
    "tests/src/¡raytracer!" > /tmp/ray_720p.src

# 2. compile
target/release/compiler /tmp/ray_720p.src /tmp/ray_720p.obj

# 3. render (Release VM!) — capture stdout
vm/build_rel/src/vm -i /tmp/ray_720p.obj > /tmp/ray_720p.out

# 4. convert the text output to a PNG
cargo x render /tmp/ray_720p.out raytracer720.png
```

`<height>` is the first `trace_rays` argument, `<width>` the second; the demo
scene uses a 16:9 aspect ratio (so `720 × 1280`, `1080 × 1920`, `2160 × 3840`,
etc.).

### Approximate cost (Release VM)

| Resolution            | Pixels      | Time    | Peak RSS |
|-----------------------|-------------|---------|----------|
| 360 × 480             | 172 800     | ~2 s    | ~37 MB   |
| 720p  (1280 × 720)    | 921 600     | ~12 s   | ~184 MB  |
| 4K    (3840 × 2160)   | 8 294 400   | ~2 min  | ~1.5 GB  |

Time scales roughly linearly with the pixel count. Memory is dominated by the
canvas (it holds every pixel record live for the whole render).

---

## 7. Quick reference

| Action                         | Command                                                            |
|--------------------------------|--------------------------------------------------------------------|
| Build compiler                 | `cargo build --release`                                            |
| Compile a program              | `target/release/compiler in.src out.obj`                           |
| Run Rust tests                 | `cargo test`                                                       |
| Configure VM (Release)         | `cmake -S vm -B vm/build_rel -DCMAKE_BUILD_TYPE=Release`            |
| Build VM                       | `cmake --build vm/build_rel -j`                                    |
| Run a bytecode file            | `vm/build_rel/src/vm -i out.obj`                                   |
| Run VM unit tests              | `cd vm/build_rel && ctest`                                         |
| Run golden tests               | `python3 run_golden_tests.py`                                      |
| Render raytracer output to PNG | `cargo x render output.txt image.png`                              |
