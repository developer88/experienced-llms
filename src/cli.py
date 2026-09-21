import argparse
import sys
from pathlib import Path
from src.config import ensure_directories, CORE_PROFILE_FILE, MEMORY_ROOT
from src.db import init_db, get_active_facts, get_active_skills, get_connection
from src.llm_client import get_llm_client
from src.extractors.session_extractor import SessionExtractor
from src.consolidator.daily_consolidator import DailyConsolidator
from src.consolidator.skill_compiler import SkillCompiler

def main():
    parser = argparse.ArgumentParser(description="Experienced LLMs: Memory and Auto-Skill Engine")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # init
    subparsers.add_parser("init", help="Initialize SQLite schema and directories")

    # extract
    ext_parser = subparsers.add_parser("extract", help="Extract key learning points from a session transcript")
    ext_parser.add_argument("--file", "-f", type=str, help="Path to transcript file")
    ext_parser.add_argument("--text", "-t", type=str, help="Transcript text directly")
    ext_parser.add_argument("--session-id", "-s", type=str, default=None, help="Custom session ID")
    ext_parser.add_argument("--scope", type=str, default="global", help="Scope / project tag")
    ext_parser.add_argument("--provider", type=str, default=None, choices=["ollama", "gemini"])

    # consolidate
    cons_parser = subparsers.add_parser("consolidate", help="Consolidate daily session experiences")
    cons_parser.add_argument("--date", "-d", type=str, default=None, help="Target date YYYY-MM-DD (defaults to today)")
    cons_parser.add_argument("--provider", type=str, default=None, choices=["ollama", "gemini"])

    # compile-skills
    skill_parser = subparsers.add_parser("compile-skills", help="Synthesize auto JIT skills and compile core profile")
    skill_parser.add_argument("--provider", type=str, default=None, choices=["ollama", "gemini"])

    # status
    subparsers.add_parser("status", help="Show active memory facts and auto-generated skills")

    # prompt / context generator
    prompt_parser = subparsers.add_parser("prompt", help="Generate full system prompt ready for LLM consumption")
    prompt_parser.add_argument("--target", "-t", type=str, default="orchestrator", choices=["orchestrator", "worker"], help="Target model level")
    prompt_parser.add_argument("--skill", type=str, default=None, help="Specific skill key for worker prompt")
    prompt_parser.add_argument("--scope", type=str, default=None, help="Scope/project filter")

    # show-context
    subparsers.add_parser("show-context", help="Display compiled initial context for local LLMs")

    args = parser.parse_args()

    if args.command == "init":
        init_db()
        print(f"Initialized database and memory directories at {MEMORY_ROOT}")

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
        for f in result.facts:
            print(f"  [{f.category.value}] ({f.scope}): {f.rule_statement}")

    elif args.command == "consolidate":
        client = get_llm_client(args.provider)
        consolidator = DailyConsolidator(client)
        daily_path = consolidator.consolidate_date(args.date)
        print(f"Consolidation complete: {daily_path}")

    elif args.command == "compile-skills":
        client = get_llm_client(args.provider)
        compiler = SkillCompiler(client)
        skills = compiler.compile_all()
        print(f"Skill compilation finished. Core profile updated. Generated/updated {len(skills)} skills:")
        for s in skills:
            print(f"  - {s.key}: {s.file_path} (~{s.token_count} words)")

    elif args.command == "status":
        facts = get_active_facts()
        skills = get_active_skills()
        print("=== Active Memory Facts ===")
        if not facts:
            print("  (None recorded yet)")
        for f in facts:
            print(f"  #{f.id} [{f.category.value}] ({f.scope}): {f.rule_statement}")

        print("\n=== Auto-Synthesized JIT Skills ===")
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

    elif args.command == "show-context":
        active_ctx = MEMORY_ROOT / "ACTIVE_CONTEXT.md"
        if active_ctx.exists():
            with open(active_ctx, "r", encoding="utf-8") as f:
                print(f.read())
        elif CORE_PROFILE_FILE.exists():
            with open(CORE_PROFILE_FILE, "r", encoding="utf-8") as f:
                print(f.read())
        else:
            print("No compiled context found. Run 'compile-skills' first.")

if __name__ == "__main__":
    main()
