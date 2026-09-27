# Testing Guidelines for Experienced LLMs

> **CRITICAL DIRECTIVE FOR AI CODING AGENTS (LLMs) & DEVELOPERS**:
> Never run tests, installers, or validation passes that mutate or pollute the host user's active environment (`~/.experienced-llms`, live SQLite `memory.db`, host crontab, or host Windows Task Scheduler).
> Always use the isolated testing tools provided below.

---

## 1. Fast Unit Tests (Hermetic)

The unit test suite uses `tempfile.TemporaryDirectory()` and mocked LLM providers to ensure 100% isolation from the developer's live memory:

```bash
# Run via python (Linux, macOS, or WSL)
python3 -m unittest discover -s tests
```

- **Speed**: < 0.5s execution.
- **Coverage**: Database initialization, migrations, fact insertions, superseding, deterministic intraday logging, interday consolidator fallback, prompt generation, and agent integrations.

---

## 2. Isolated Full-Cycle Sandbox Testing (No Docker Required)

To test the entire end-to-end user lifecycle (installer execution, directory initialization, CLI command wrappers, real SQLite indexing, intraday logging, and consolidation) without Docker and with **zero host pollution**:

### Linux / macOS / WSL:
```bash
bash scripts/test_sandbox.sh
```

### Cross-Platform Python:
```bash
python3 scripts/test_sandbox.py
```

### How the Sandbox Works:
1. Allocates an ephemeral directory in `/tmp/` (`mktemp -d`).
2. Redirects `$HOME`, `EXPERIENCED_LLMS_HOME`, and `EXPERIENCED_LLMS_DB` to the sandbox.
3. Runs `install.sh` and verifies that all files and CLI wrappers are created exclusively inside the sandbox.
4. Executes live CLI operations (`experienced-llms log`, `status`, `consolidate`, `show`).
5. Runs the unit test suite against the sandboxed state.
6. Automatically destroys and cleans up the sandbox on exit.

---

## 3. Containerized Testing (Docker)

For CI pipelines or developers with Docker installed:

```bash
# Run automated container build & verification
bash scripts/test_in_docker.sh
```

Or run directly using Docker CLI:
```bash
docker build -f Dockerfile.test -t experienced-llms-test .
docker run --rm experienced-llms-test
```

This runs the entire setup inside a clean Ubuntu 24.04 image with fresh Python 3 and cron dependencies.

---

## 4. Multi-Agent CLI Testing (`agy`, `claude`, etc.)

When writing acceptance tests that invoke AI coding agent CLIs:
- Never feed prompts that write test assertions to the developer's live memory.
- If testing agent skill activation with `agy` or `claude`, ensure `EXPERIENCED_LLMS_HOME` is overridden to a test directory, or explicitly verify that any recorded test fact is pruned/cleaned up afterward.
