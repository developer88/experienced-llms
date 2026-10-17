#!/usr/bin/env python3
"""
Pre-Release Master Test Runner for Experienced LLMs.

Executes all verification gates locally before a release:
1. Source syntax & compile check
2. Hermetic unit test suite (35+ tests)
3. Isolated full-cycle sandbox integration test (install.sh, CLI shims, SQLite, 2-file containment)
4. E2E synthetic consolidation test (Pathway A & Pathway B)

Zero pollution of host system (~/.experienced-llms or crontab).
"""

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

def run_step(step_name: str, cmd: list) -> bool:
    print(f"\n[>] RUNNING: {step_name}...")
    start = time.time()
    res = subprocess.run(cmd, cwd=str(REPO_ROOT))
    elapsed = time.time() - start
    if res.returncode == 0:
        print(f"[✓] {step_name} PASSED ({elapsed:.2f}s)")
        return True
    else:
        print(f"[-] {step_name} FAILED with exit code {res.returncode} ({elapsed:.2f}s)")
        return False

def main():
    print("=" * 65)
    print("   Experienced LLMs: Pre-Release Local Integration Runner")
    print("=" * 65)

    results = {}

    # 1. Syntax check
    py_files = list((REPO_ROOT / "src").rglob("*.py")) + list((REPO_ROOT / "tests").rglob("*.py"))
    print(f"[+] Checking Python syntax across {len(py_files)} files...")
    compile_cmd = [sys.executable, "-m", "py_compile"] + [str(p) for p in py_files]
    results["Syntax Check"] = run_step("Python Syntax & Compilation Check", compile_cmd)
    if not results["Syntax Check"]:
        sys.exit(1)

    # 2. Hermetic Unit Tests
    unit_cmd = [sys.executable, "-m", "unittest", "discover", "-s", "tests"]
    results["Unit Tests"] = run_step("Hermetic Unit Test Suite (35+ tests)", unit_cmd)
    if not results["Unit Tests"]:
        sys.exit(1)

    # 3. Sandbox Integration Acceptance (Installer & CLI)
    sandbox_cmd = [sys.executable, str(REPO_ROOT / "scripts" / "test_sandbox.py")]
    results["Sandbox Integration"] = run_step("Isolated Sandbox Acceptance (Installer, SQLite, 2-File Containment)", sandbox_cmd)
    if not results["Sandbox Integration"]:
        sys.exit(1)

    # 4. E2E Synthetic Consolidation (Checks agy.exe or mock LLM)
    has_agy = shutil.which("agy.exe") is not None or shutil.which("agy") is not None
    if has_agy:
        synth_cmd = [sys.executable, str(REPO_ROOT / "scripts" / "e2e_synthetic_consolidation.py")]
        results["Synthetic Consolidation"] = run_step("E2E Synthetic Consolidation (Gemini 3.8 Flash Low)", synth_cmd)
    else:
        print("\n[*] agy.exe not in PATH; running sandbox bash runner...")
        synth_cmd = ["bash", str(REPO_ROOT / "scripts" / "test_sandbox.sh")]
        results["Synthetic Consolidation"] = run_step("Isolated Bash Sandbox & Consolidation", synth_cmd)

    if not results["Synthetic Consolidation"]:
        sys.exit(1)

    # 5. Rust Engine Test Suite (if cargo is present)
    cargo_path = shutil.which("cargo")
    if not cargo_path:
        default_cargo = Path.home() / ".cargo" / "bin" / "cargo"
        if default_cargo.exists():
            cargo_path = str(default_cargo)

    if cargo_path and (REPO_ROOT / "Cargo.toml").exists():
        rust_cmd = [cargo_path, "test"]
        results["Rust Engine Tests"] = run_step("Rust Engine Test Suite (rusqlite, WAL, bidirectional compat)", rust_cmd)
        if not results["Rust Engine Tests"]:
            sys.exit(1)

    # Summary
    print("\n" + "=" * 65)
    print("   PRE-RELEASE LOCAL VERIFICATION SUMMARY")
    print("=" * 65)
    all_passed = True
    for test_name, passed in results.items():
        status = "[✓] PASSED" if passed else "[-] FAILED"
        if not passed:
            all_passed = False
        print(f"  {status:14} : {test_name}")
    print("=" * 65)

    if all_passed:
        print("[SUCCESS] All pre-release integration tests passed! Safe to release.")
        sys.exit(0)
    else:
        print("[-] Pre-release verification failed!")
        sys.exit(1)

if __name__ == "__main__":
    main()
