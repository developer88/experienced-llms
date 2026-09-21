# Experienced LLMs

> **Self-Learning Memory & JIT Skill Consolidation for Local & Cloud LLMs**

Experienced LLMs provides an autonomous cognitive pipeline that learns from user interactions, eliminating the need to manually author instructions or prompt skills.

It is specifically tailored to the local execution constraints of 7B/14B models on consumer hardware (e.g. NVIDIA RTX 4060 8GB VRAM) and hybrid setups with Gemini 3.8 Flash:
- **Zero Memory Bloat**: Small local models (2k–4k context limit) cannot digest long chat histories or huge skill registries.
- **Automated JIT Skill Synthesis**: Repeated lessons, user preferences, and mistake corrections are automatically compiled into **150–300 token operational constraint wrappers** stored in `./skills/auto_*.md`.
- **Core User Profile**: Key universal directives are condensed into `core_profile.md` (< 200 tokens) for instant injection into system prompts.
- **Transparent & Human-in-the-Loop**: All memories are stored as plain Markdown (fully compatible with Obsidian) and indexed in a lightweight SQLite database (`PRAGMA foreign_keys = ON`).

---

## The 4-Step Pipeline

```
  [Chat Session Transcript]
              │
              ▼ (Step 1: Session Extractor)
   memory/sessions/YYYY-MM-DD_HHMM.md
              │
              ▼ (Step 2: Daily Aggregation)
      memory/daily/YYYY-MM-DD.md
              │
              ▼ (Step 3: Reconciliation & JIT Skill Compiler)
  ┌───────────────────────────────────────────────┐
  │  1. Invalidate superseded facts               │
  │  2. Synthesize skills (skills/auto_*.md)      │
  │  3. Condense core profile (core_profile.md)   │
  └───────────────────────────────────────────────┘
              │
              ▼ (Step 4: Next Session Runtime Context)
  Injected into 7B Worker prompt:
  - core_profile.md (< 200 tokens)
  - Exactly one relevant JIT skill (150-300 tokens)
```

---

## Directory Structure

```
experienced-llms/
├── schema.sql                     # SQLite schema (sessions, memory_facts, skills)
├── src/
│   ├── config.py                  # Cross-platform paths (Windows/WSL/Obsidian)
│   ├── db.py                      # Database access & foreign key enforcement
│   ├── models.py                  # Domain dataclasses
│   ├── llm_client.py              # Ollama, Gemini 3.8 Flash, and Mock clients
│   ├── extractors/
│   │   └── session_extractor.py   # Step 1: High-signal fact extraction
│   ├── consolidator/
│   │   ├── daily_consolidator.py  # Step 2: Daily aggregation & conflict resolution
│   │   └── skill_compiler.py      # Step 3: Auto-skill synthesis & core profile
│   └── cli.py                     # Command-line interface
├── skills/                        # Generated JIT skills (150-300 tokens each)
├── memory/                        # Plain Markdown memory vault (Obsidian-compatible)
│   ├── sessions/                  # Raw session extractions
│   ├── daily/                     # Daily consolidated reports
│   └── core_profile.md            # Active master profile (<200 tokens)
├── tests/
│   └── test_pipeline.py           # Comprehensive automated test suite
└── examples/
    ├── sample_transcript.txt      # Example user-assistant conversation
    └── run_demo.py                # End-to-end pipeline demonstration
```

---

## Quick Start (WSL / Linux / Windows)

### 1. Initialize Storage & Database
```bash
python3 -m src.cli init
```

### 2. Extract Memory from a Session
```bash
python3 -m src.cli extract --file examples/sample_transcript.txt --scope "experienced-llms"
```

### 3. Run Daily Consolidation
```bash
python3 -m src.cli consolidate --date 2026-09-21
```

### 4. Compile JIT Skills & Core Profile
```bash
python3 -m src.cli compile-skills
```

### 5. View Status & Runtime Context
```bash
python3 -m src.cli status
python3 -m src.cli show-context
```

## How LLMs Understand and Use This (The Prompt Protocol)

When you start a session with an LLM (Gemini 3.8 Flash, Claude 3.5, GPT-4o, or local Ollama), how does the model know what to do?

The system exports a self-contained operating protocol that you feed directly into the LLM's system prompt (or paste into Claude Projects / Cursor rules / Antigravity):

### 1. For High-Level Orchestrators (Cloud or Local Planner)
Run:
```bash
python3 -m src.cli prompt --target orchestrator
```
Or simply load **`memory/ACTIVE_CONTEXT.md`** (which is auto-generated on each consolidation pass).

**What the LLM sees:**
- **Active User Profile & Directives**: Explicit bullet points of what rules and constraints must be strictly obeyed.
- **JIT Skills Catalog**: A lightweight enum/index of available modular skills (e.g. `auto_wsl_node_runner: Ensures node/npm scripts run in WSL`). The orchestrator knows it can delegate this skill to workers.
- **The Self-Learning Feedback Contract**: An explicit directive instructing the LLM:
  > *"At the end of your session or milestone, if new user preferences, architectural decisions, or mistake corrections occurred, emit a `<session_learning>` block."*

### 2. For Small Local Workers (7B/14B in 8GB VRAM)
Small models cannot read the entire skill catalog without attention degradation. Instead, generate a tight, 200–300 token execution prompt with exactly **one** targeted skill:
```bash
python3 -m src.cli prompt --target worker --skill auto_wsl_node_runner
```
**What the 7B model sees:**
- The compact core rules (<100 tokens).
- The exact input/output operational wrapper for that specific task.
- Zero conversational fluff.

---

## The Autonomous Learning Loop

```
1. You chat with the LLM (Gemini / Claude / Local) with ACTIVE_CONTEXT.md in its system prompt.
2. During the session, the LLM makes a mistake and you correct it ("Always run npm in WSL!").
3. At the end of the session, the LLM emits:
     <session_learning>
     - [user_preference] (global): Always run npm in WSL | Windows host node path fails
     </session_learning>
4. Session Extractor ingests the session (either via CLI hook or transcript file).
5. Nightly Cron runs:
     python3 -m src.cli consolidate
     python3 -m src.cli compile-skills
6. Next morning, ACTIVE_CONTEXT.md has the new rule consolidated, and skills/auto_wsl_runner.md is ready!
```

---

## Running Automated Tests

```bash
python3 -m unittest discover -s tests
```

