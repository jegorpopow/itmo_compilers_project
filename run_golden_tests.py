#!/usr/bin/env python3
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).parent

COMPILER_CANDIDATES = [
    ROOT / "target" / "release" / "compiler",
    ROOT / "target" / "debug" / "compiler",
]

VM_CANDIDATES = [
    ROOT / "vm" / "build" / "src" / "vm",
    ROOT / "vm" / "build" / "vm" / "vm",
    ROOT / "vm" / "build" / "Release" / "vm" / "vm",
]

TESTS_SRC   = ROOT / "tests" / "src"
TESTS_PASS  = ROOT / "tests" / "run" / "pass"


def find_binary(candidates):
    for p in candidates:
        if p.exists():
            return p
    return None


def collect_cases():
    cases = []
    for stdout_file in sorted(TESTS_PASS.glob("*.stdout")):
        name = stdout_file.stem
        src = TESTS_SRC / (f"\xa1{name}!")
        if src.exists():
            cases.append((name, src, stdout_file))
    return cases


def run_case(compiler, vm, name, src, expected_file):
    with tempfile.NamedTemporaryFile(suffix=".obj", delete=False) as f:
        obj = Path(f.name)
    try:
        r = subprocess.run(
            [str(compiler), str(src), str(obj)],
            capture_output=True, timeout=30,
        )
        if r.returncode != 0:
            return False, f"compiler failed (exit {r.returncode}):\n{r.stderr.decode()}"

        r = subprocess.run(
            [str(vm), "-i", str(obj)],
            capture_output=True, timeout=30,
        )
        actual = r.stdout.decode()
        expected = expected_file.read_text()

        if actual == expected:
            return True, None
        else:
            diff_lines = []
            a_lines = actual.splitlines()
            e_lines = expected.splitlines()
            for i, (a, e) in enumerate(zip(a_lines, e_lines), 1):
                if a != e:
                    diff_lines.append(f"  line {i}: got {a!r}, want {e!r}")
            if len(a_lines) != len(e_lines):
                diff_lines.append(f"  got {len(a_lines)} lines, want {len(e_lines)} lines")
            return False, "\n".join(diff_lines)
    except subprocess.TimeoutExpired:
        return False, "timed out"
    finally:
        obj.unlink(missing_ok=True)


def main():
    compiler = find_binary(COMPILER_CANDIDATES)
    vm       = find_binary(VM_CANDIDATES)

    if not compiler:
        print("ERROR: compiler not found; run `cargo build --release` first")
        sys.exit(1)
    if not vm:
        print("ERROR: VM binary not found; run `cmake --build vm/build` first")
        sys.exit(1)

    print(f"compiler : {compiler}")
    print(f"vm       : {vm}")
    print()

    cases = collect_cases()
    if not cases:
        print("No test cases found.")
        sys.exit(1)

    passed = failed = 0
    for name, src, expected in cases:
        ok, msg = run_case(compiler, vm, name, src, expected)
        if ok:
            print(f"  PASS  {name}")
            passed += 1
        else:
            print(f"  FAIL  {name}")
            if msg:
                print(msg)
            failed += 1

    print()
    print(f"{passed}/{passed + failed} tests passed")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
