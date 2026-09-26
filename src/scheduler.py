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

def install_schedule(time_str: str = "02:00") -> Tuple[bool, str]:
    """
    Installs a scheduled job to run `experienced-llms consolidate` daily at `time_str`.
    Uses crontab on Linux/macOS/WSL, and schtasks on native Windows.
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

    if sys.platform.startswith("win32"):
        # Windows Task Scheduler
        cmd_str = f'"{python_bin}" "{cli_path}" consolidate'
        st_val = f"{hour:02d}:{minute:02d}"
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

    else:
        # Linux / WSL / macOS crontab
        cron_comment = "# Experienced-LLMs Nightly Consolidation"
        cron_line = f"{minute} {hour} * * * cd {config.BASE_DIR} && {python_bin} -m src.cli consolidate >> {log_file} 2>&1 {cron_comment}"

        try:
            # Read existing crontab
            read_proc = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
            existing_lines = read_proc.stdout.splitlines() if read_proc.returncode == 0 else []

            # Filter out existing experienced-llms lines
            new_lines = [l for l in existing_lines if cron_comment not in l and "src.cli consolidate" not in l]
            new_lines.append(cron_line)

            # Write updated crontab
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

def uninstall_schedule() -> Tuple[bool, str]:
    """Removes the scheduled job."""
    if sys.platform.startswith("win32"):
        try:
            res = subprocess.run(["schtasks", "/delete", "/tn", TASK_NAME, "/f"], capture_output=True, text=True)
            if res.returncode == 0:
                return True, f"Task '{TASK_NAME}' deleted from Windows Task Scheduler."
            return False, f"Task '{TASK_NAME}' not found or deletion failed."
        except Exception as e:
            return False, f"Windows Task Scheduler error: {e}"
    else:
        cron_comment = "# Experienced-LLMs Nightly Consolidation"
        try:
            read_proc = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
            if read_proc.returncode != 0:
                return True, "No crontab currently configured."
            lines = [l for l in read_proc.stdout.splitlines() if cron_comment not in l and "src.cli consolidate" not in l]
            subprocess.run(["crontab", "-"], input="\n".join(lines) + "\n", text=True, capture_output=True)
            return True, "Removed Experienced-LLMs entry from crontab."
        except Exception as e:
            return False, f"Crontab uninstall error: {e}"

def get_schedule_status() -> str:
    """Checks whether the schedule is active."""
    if sys.platform.startswith("win32"):
        try:
            res = subprocess.run(["schtasks", "/query", "/tn", TASK_NAME], capture_output=True, text=True)
            if res.returncode == 0:
                return f"ACTIVE (Windows Task Scheduler: '{TASK_NAME}')"
            return "NOT CONFIGURED"
        except Exception:
            return "UNKNOWN (schtasks query error)"
    else:
        try:
            res = subprocess.run(["crontab", "-l"], capture_output=True, text=True)
            if res.returncode == 0 and "src.cli consolidate" in res.stdout:
                matching = [l for l in res.stdout.splitlines() if "src.cli consolidate" in l]
                return f"ACTIVE in crontab:\n  {matching[0]}"
            return "NOT CONFIGURED"
        except Exception:
            return "UNKNOWN (crontab unavailable)"
