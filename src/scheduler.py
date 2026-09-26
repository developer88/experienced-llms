"""Scheduler Subsystem: Cross-platform cron / Task Scheduler manager for nightly consolidation."""

import os
import sys
import shutil
import subprocess
from pathlib import Path
from typing import Tuple

import src.config as config

TASK_NAME = "ExperiencedLLMsConsolidator"

def get_python_executable() -> str:
    return sys.executable

def install_schedule(time_str: str = "02:00", scheduler_type: str = "auto") -> Tuple[bool, str]:
    """
    Installs a scheduled job to run `experienced-llms consolidate` daily at `time_str`.
    Supports standard crontab (Linux/macOS/WSL) and Windows Task Scheduler (Windows/WSL).
    """
    python_bin = get_python_executable()
    cli_path = config.BASE_DIR / "src" / "cli.py"
    log_file = config.EXPERIENCED_HOME / "cron.log"
    config.ensure_directories()

    # Parse HH:MM
    try:
        parts = time_str.split(":")
        hour = int(parts[0])
        minute = int(parts[1]) if len(parts) > 1 else 0
    except Exception:
        hour, minute = 2, 0

    st_val = f"{hour:02d}:{minute:02d}"

    # 1. Native Windows schtasks
    if sys.platform.startswith("win32") and scheduler_type in ("auto", "schtasks"):
        cmd_str = f'"{python_bin}" "{cli_path}" consolidate'
        schtasks_cmd = [
            "schtasks", "/create",
            "/tn", TASK_NAME,
            "/tr", cmd_str,
            "/sc", "daily",
            "/st", st_val,
            "/f"
        ]
        try:
            res = subprocess.run(schtasks_cmd, capture_output=True, text=True)
            if res.returncode == 0:
                return True, f"Windows Scheduled Task '{TASK_NAME}' registered daily at {st_val}."
            return False, f"Failed to register task: {res.stderr.strip()}"
        except Exception as e:
            return False, f"Windows Task Scheduler error: {e}"

    # 2. WSL with Windows Task Scheduler preference
    if scheduler_type == "schtasks" or (scheduler_type == "auto" and shutil.which("schtasks.exe")):
        wsl_cmd = 'wsl.exe -d Ubuntu -- bash -lc "experienced-llms consolidate"'
        schtasks_cmd = [
            "schtasks.exe", "/create",
            "/tn", TASK_NAME,
            "/tr", wsl_cmd,
            "/sc", "daily",
            "/st", st_val,
            "/f"
        ]
        try:
            res = subprocess.run(schtasks_cmd, capture_output=True, text=True)
            if res.returncode == 0:
                return True, f"Windows Scheduled Task '{TASK_NAME}' registered daily at {st_val}."
        except Exception:
            pass

    # 3. Standard Linux / macOS / Unix crontab
    cron_comment = "# Experienced-LLMs Nightly Consolidation"
    cron_line = f"{minute} {hour} * * * cd {config.BASE_DIR} && {python_bin} -m src.cli consolidate >> {log_file} 2>&1 {cron_comment}"

    try:
        read_proc = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
        existing_lines = read_proc.stdout.splitlines() if read_proc.returncode == 0 else []

        new_lines = [l for l in existing_lines if cron_comment not in l and "src.cli consolidate" not in l]
        new_lines.append(cron_line)

        write_proc = subprocess.run(
            ["crontab", "-"],
            input="\n".join(new_lines) + "\n",
            text=True,
            capture_output=True
        )
        if write_proc.returncode == 0:
            return True, f"Crontab entry installed daily at {hour:02d}:{minute:02d}."
        return False, f"Failed to update crontab: {write_proc.stderr.strip()}"
    except Exception as e:
        return False, f"Crontab error: {e}"

def uninstall_schedule(scheduler_type: str = "auto") -> Tuple[bool, str]:
    """Removes the scheduled job across Windows Task Scheduler and/or Crontab."""
    messages = []
    
    # Clean up Windows Task Scheduler if applicable
    if sys.platform.startswith("win32") or shutil.which("schtasks.exe"):
        bin_name = "schtasks" if sys.platform.startswith("win32") else "schtasks.exe"
        if scheduler_type in ("auto", "schtasks"):
            try:
                res = subprocess.run([bin_name, "/delete", "/tn", TASK_NAME, "/f"], capture_output=True, text=True)
                if res.returncode == 0:
                    messages.append(f"Task '{TASK_NAME}' deleted from Windows Task Scheduler.")
            except Exception:
                pass

    # Clean up Crontab if applicable
    if scheduler_type in ("auto", "cron"):
        cron_comment = "# Experienced-LLMs Nightly Consolidation"
        try:
            read_proc = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
            if read_proc.returncode == 0 and "src.cli consolidate" in read_proc.stdout:
                lines = [l for l in read_proc.stdout.splitlines() if cron_comment not in l and "src.cli consolidate" not in l]
                subprocess.run(["crontab", "-"], input="\n".join(lines) + "\n", text=True, capture_output=True)
                messages.append("Removed entry from crontab.")
        except Exception:
            pass

    if messages:
        return True, " ".join(messages)
    return True, "No active schedule found to remove."

def get_schedule_status() -> str:
    """Checks and reports all active schedules (Windows Task Scheduler, Crontab)."""
    statuses = []

    # Check Windows Task Scheduler
    bin_name = "schtasks" if sys.platform.startswith("win32") else "schtasks.exe"
    if sys.platform.startswith("win32") or shutil.which("schtasks.exe"):
        try:
            res = subprocess.run([bin_name, "/query", "/tn", TASK_NAME], capture_output=True, text=True)
            if res.returncode == 0:
                statuses.append(f"ACTIVE (Windows Task Scheduler: '{TASK_NAME}')")
        except Exception:
            pass

    # Check Crontab
    try:
        res = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
        if res.returncode == 0 and "src.cli consolidate" in res.stdout:
            matching = [l for l in res.stdout.splitlines() if "src.cli consolidate" in l]
            statuses.append(f"ACTIVE (Crontab: '{matching[0].strip()}')")
    except Exception:
        pass

    if statuses:
        return "\n  ".join(statuses)
    return "NOT CONFIGURED"
