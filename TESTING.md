# Testing Guidelines for Experienced LLMs

> **CRITICAL DIRECTIVE FOR AI CODING AGENTS (LLMs) & DEVELOPERS**:
> Never run tests, installers, or validation passes that mutate or pollute the host user's active environment (`~/.experienced-llms`, live SQLite `memory.db`, host crontab, or host Windows Task Scheduler).
> Always use the isolated testing tools provided below.

---

## 🚀 One-Command Pre-Release Verification

Before tagging or publishing any release, execute the master pre-release integration runner locally:

```bash
# In Linux, macOS, or WSL:
bash scripts/run_all_tests.sh
```

```powershell
# In Windows 11 PowerShell:
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\test_install_windows.ps1
```

This validates all quality gates with **zero host pollution**:
1. **Rust Engine Test Suite (`cargo test`)**: 100% hermetic unit & integration tests covering models, configuration, SQLite migrations, consolidator algorithms, prompt builder, agent integrations, and AgenticSkills compliance.
2. **POSIX Installer Acceptance (`test_install_posix.sh`)**: Tests `install.sh` in local repo mode, streamed remote download mode, and fallback error handling.
3. **Windows Installer Acceptance (`test_install_windows.ps1`)**: Tests `install.ps1` in isolated local sandboxes, streamed remote download mode, and error handling.
4. **Isolated Full-Cycle Sandbox (`test_sandbox.sh`)**: Tests native binary execution, deterministic intraday logging, and consolidation in an ephemeral `/tmp/` sandbox.

---

## 1. Rust Engine Tests (Cargo)

The native Rust engine includes a hermetic test suite covering all CLI subcommands, database transactions, consolidator algorithms, and AgenticSkills compliance:

```bash
cargo test
```

- **Speed**: Fast, parallel execution.
- **Coverage**: Database initialization, migrations, fact insertions, superseding, deterministic intraday logging, interday consolidator fallback, prompt template resolution, agent integrations, and `SKILL.md` packaging.

---

## 2. Automated Installer Tests

### POSIX (`install.sh`)
```bash
bash tests/test_install_posix.sh
```
- Tests local repo installation mode.
- Tests streamed piped execution (`cat install.sh | bash`) downloading from live GitHub CDN.
- Tests failure detection and Cargo fallback instructions.

### Windows (`install.ps1`)
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\test_install_windows.ps1
```
- Tests sandboxed local installation mode.
- Tests streamed execution (`Invoke-Expression`) downloading Windows `.exe` from GitHub CDN.
- Verifies PATH registration and Task Scheduler setup.
- Restores user PATH with zero host pollution.

---

## 3. Containerized Local Testing (Docker - Zero Cloud Minutes)

To test the entire release pipeline (installers, standalone Rust binary, unit tests, SQLite persistence, and CLI commands) inside an isolated Linux container with **zero host pollution** and **zero GitHub Actions cloud costs**:

```bash
# Run isolated container test suite (100% hermetic)
bash scripts/pipeline.sh test

# Build release binaries for Linux (musl & gnu) and Windows locally:
bash scripts/pipeline.sh build
```

---

## 4. Multi-Agent CLI Testing (`agy`, `claude`, etc.)

When writing acceptance tests that invoke AI coding agent CLIs:
- Never feed prompts that write test assertions to the developer's live memory.
- If testing agent skill activation with `agy` or `claude`, ensure `EXPERIENCED_LLMS_HOME` is overridden to a test directory, or explicitly verify that any recorded test fact is pruned/cleaned up afterward.
