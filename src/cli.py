import argparse
import json
from pathlib import Path
import sys

_BASE_DIR = Path(__file__).resolve().parent.parent
if str(_BASE_DIR) not in sys.path:
    sys.path.insert(0, str(_BASE_DIR))

import src.config as config
from src.db import init_db, get_active_facts, get_active_skills
from src.llm_client import get_llm_client
from src.intraday import log_intraday
from src.consolidator.interday_consolidator import InterdayConsolidator
from src.consolidator.daily_consolidator import DailyConsolidator
from src.consolidator.skill_compiler import SkillCompiler
from src.extractors.session_extractor import SessionExtractor
from src.scheduler import install_schedule, uninstall_schedule, get_schedule_status

def main():
    parser = argparse.ArgumentParser(
        description="Experienced LLMs: Autonomous Self-Learning & Memory Consolidation"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # 1. log (intraday deterministic logger)
    log_parser = subparsers.add_parser("log", help="Log an intraday rule or correction (zero tokens, instant, offline)")
    log_parser.add_argument("statement", type=str, help="Concise operational rule statement")
    log_parser.add_argument("--category", "-c", type=str, default="user_preference",
                            choices=["defensive_heuristic", "user_preference", "technical_decision", "mistake_correction", "project_gotcha"],
                            help="Classification of the rule")
    log_parser.add_argument("--scope", "-s", type=str, default="global", help="Scope / repository tag")
    log_parser.add_argument("--reason", "-r", type=str, default="", help="Context or reason behind this rule")
    log_parser.add_argument("--session-id", type=str, default="", help="Optional session identifier")

    # 2. consolidate (interday scheduled consolidator)
    cons_parser = subparsers.add_parser("consolidate", help="Consolidate multi-day experiences into EXPERIENCE.md via LLM")
    cons_parser.add_argument("--date", "-d", type=str, default=None, help="Target date YYYY-MM-DD")
    cons_parser.add_argument("--provider", "-p", type=str, default=None, choices=["gemini", "claude", "openai", "ollama"])

    # 3. show (display master experience)
    subparsers.add_parser("show", help="Display the active master EXPERIENCE.md")

    # 4. schedule (manage cron / Task Scheduler)
    sched_parser = subparsers.add_parser("schedule", help="Manage scheduled nightly consolidation daemon")
    sched_parser.add_argument("--install", action="store_true", help="Install the scheduled job")
    sched_parser.add_argument("--uninstall", action="store_true", help="Uninstall the scheduled job")
    sched_parser.add_argument("--status", action="store_true", help="Check schedule status")
    sched_parser.add_argument("--time", type=str, default="02:00", help="Daily execution time in HH:MM format (default: 02:00)")

    # 5. setup (configuration wizard)
    setup_parser = subparsers.add_parser("setup", help="Configure LLM providers and paths")
    setup_parser.add_argument("--provider", type=str, choices=["gemini", "claude", "openai", "ollama"])
    setup_parser.add_argument("--key", type=str, help="API key for selected provider")
    setup_parser.add_argument("--model", type=str, help="Model identifier")

    # 6. status
    subparsers.add_parser("status", help="Show active facts, auto-skills, and schedule state")

    # 7. prompt (for orchestrator or local worker prompt injection)
    prompt_parser = subparsers.add_parser("prompt", help="Generate ready-to-inject prompt for LLMs")
    prompt_parser.add_argument("--target", "-t", type=str, default="orchestrator", choices=["orchestrator", "worker"])
    prompt_parser.add_argument("--skill", type=str, default=None, help="Specific skill key for worker prompt")
    prompt_parser.add_argument("--scope", type=str, default=None, help="Scope/project filter")

    # 8. integrate (install instructions for Antigravity, Claude, Copilot, Cursor, Pi)
    integ_parser = subparsers.add_parser("integrate", help="Install instructions into AI agents (Antigravity, Claude, Copilot, Cursor, Pi)")
    integ_parser.add_argument("--target", "-t", type=str, default="all", choices=["antigravity", "claude", "copilot", "cursor", "pi", "all"])
    integ_parser.add_argument("--workspace", "-w", type=str, default=None, help="Target workspace path (defaults to current directory)")

    # Legacy subcommands (init, extract, compile-skills, show-context)
    subparsers.add_parser("init", help="Initialize SQLite schema and directories")
    ext_parser = subparsers.add_parser("extract", help="Extract facts from a session transcript file")
    ext_parser.add_argument("--file", "-f", type=str)
    ext_parser.add_argument("--text", "-t", type=str)
    ext_parser.add_argument("--session-id", "-s", type=str, default=None)
    ext_parser.add_argument("--scope", type=str, default="global")
    ext_parser.add_argument("--provider", type=str, default=None)

    subparsers.add_parser("compile-skills", help="Synthesize JIT skills")
    subparsers.add_parser("show-context", help="Display context")

    args = parser.parse_args()

    # Dispatching
    if args.command == "init":
        init_db(config.DB_PATH)
        print(f"Initialized Experienced LLMs at: {config.EXPERIENCED_HOME}")

    elif args.command == "log":
        fact = log_intraday(
            rule_statement=args.statement,
            category=args.category,
            scope=args.scope,
            context_reason=args.reason,
            session_id=args.session_id
        )
        print(f"Logged [{fact.category.value.upper()}] ({fact.scope}): {fact.rule_statement}")

    elif args.command == "consolidate":
        client = get_llm_client(args.provider)
        consolidator = InterdayConsolidator(client)
        path, summary = consolidator.consolidate(args.date)
        print(f"Consolidation complete: {path}")
        print(f"Summary: {summary}")

    elif args.command == "show":
        if config.MASTER_EXPERIENCE_FILE.exists():
            with open(config.MASTER_EXPERIENCE_FILE, "r", encoding="utf-8") as f:
                print(f.read())
        elif config.CORE_PROFILE_FILE.exists():
            with open(config.CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
                print(f.read())
        else:
            print(f"No master experience found yet at {config.MASTER_EXPERIENCE_FILE}.")
            print("Log some directives with 'experienced-llms log' and run 'experienced-llms consolidate'.")

    elif args.command == "schedule":
        if args.install:
            ok, msg = install_schedule(args.time)
            print(msg)
        elif args.uninstall:
            ok, msg = uninstall_schedule()
            print(msg)
        else:
            print("Scheduler Status:")
            print(f"  {get_schedule_status()}")

    elif args.command == "setup":
        cfg = config.load_config()
        if args.provider:
            cfg["provider"] = args.provider
        if args.key:
            if cfg["provider"] == "gemini":
                cfg["gemini_api_key"] = args.key
            elif cfg["provider"] == "claude":
                cfg["claude_api_key"] = args.key
            elif cfg["provider"] == "openai":
                cfg["openai_api_key"] = args.key
        if args.model:
            if cfg["provider"] == "gemini":
                cfg["gemini_model"] = args.model
            elif cfg["provider"] == "claude":
                cfg["claude_model"] = args.model
            elif cfg["provider"] == "openai":
                cfg["openai_model"] = args.model
            elif cfg["provider"] == "ollama":
                cfg["ollama_model"] = args.model

        config.save_config(cfg)
        print(f"Configuration saved to {config.CONFIG_FILE}:")
        safe_cfg = cfg.copy()
        for k in ["gemini_api_key", "claude_api_key", "openai_api_key"]:
            if safe_cfg.get(k):
                safe_cfg[k] = safe_cfg[k][:6] + "..." + safe_cfg[k][-4:] if len(safe_cfg[k]) > 10 else "***"
        print(json.dumps(safe_cfg, indent=2))

    elif args.command == "status":
        facts = get_active_facts()
        skills = get_active_skills()
        print(f"Home: {config.EXPERIENCED_HOME}")
        print(f"Master Experience: {'Exists' if config.MASTER_EXPERIENCE_FILE.exists() else 'Not yet created'}")
        print(f"Schedule: {get_schedule_status()}")
        print("\n=== Active Memory Facts ===")
        if not facts:
            print("  (None recorded yet)")
        for f in facts[:15]:
            print(f"  #{f.id} [{f.category.value}] ({f.scope}): {f.rule_statement}")
        if len(facts) > 15:
            print(f"  ... and {len(facts) - 15} more.")

        print("\n=== Auto-Synthesized Skills ===")
        if not skills:
            print("  (None generated yet)")
        for s in skills:
            print(f"  * {s.key} ({s.file_path})")

    elif args.command == "prompt":
        from src.prompt_builder import build_orchestrator_prompt, build_worker_prompt
        if args.target == "worker":
            print(build_worker_prompt(active_skill_key=args.skill))
        else:
            print(build_orchestrator_prompt(scope=args.scope))

    elif args.command == "integrate":
        from src.agent_integrations import (
            install_antigravity, install_claude, install_copilot,
            install_cursor, install_pi_generic, auto_detect_and_install_all
        )
        ws_path = Path(args.workspace) if args.workspace else None
        if args.target == "antigravity":
            ok, msg = install_antigravity()
            print(f"[+] {msg}")
        elif args.target == "claude":
            ok, msg = install_claude(ws_path)
            print(f"[+] {msg}")
        elif args.target == "copilot":
            ok, msg = install_copilot(ws_path)
            print(f"[+] {msg}")
        elif args.target == "cursor":
            ok, msg = install_cursor(ws_path)
            print(f"[+] {msg}")
        elif args.target == "pi":
            ok, msg = install_pi_generic(ws_path)
            print(f"[+] {msg}")
        else:
            results = auto_detect_and_install_all(ws_path)
            for r in results:
                print(r)

    elif args.command == "extract":
        text = ""
        if args.file:
            with open(args.file, "r", encoding="utf-8") as f:
                text = f.read()
        elif args.text:
            text = args.text
        else:
            print("Error: Either --file or --text is required for extraction.", file=sys.stderr)
            sys.exit(1)

        client = get_llm_client(args.provider)
        extractor = SessionExtractor(client)
        result = extractor.extract_from_text(
            transcript=text,
            session_id=args.session_id,
            scope_default=args.scope
        )
        print(f"Extraction successful for session: {result.session_id}")
        print(f"Extracted {len(result.facts)} key facts/rules.")

    elif args.command == "compile-skills":
        client = get_llm_client(args.provider)
        compiler = SkillCompiler(client)
        skills = compiler.compile_all()
        print(f"Generated {len(skills)} skills.")

    elif args.command == "show-context":
        if config.MASTER_EXPERIENCE_FILE.exists():
            with open(config.MASTER_EXPERIENCE_FILE, "r", encoding="utf-8") as f:
                print(f.read())
        elif config.CORE_PROFILE_FILE.exists():
            with open(config.CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
                print(f.read())
        else:
            print("No compiled context found.")

if __name__ == "__main__":
    main()
