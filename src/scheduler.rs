use std::io::Write;
use std::process::{Command, Stdio};

use crate::config;

pub const TASK_NAME: &str = "ExperiencedLLMsConsolidator";
pub const CRON_COMMENT: &str = "# Experienced-LLMs Nightly Consolidation";

pub fn parse_time(time_str: &str) -> (u32, u32) {
    let parts: Vec<&str> = time_str.split(':').collect();
    let hour = parts.first().and_then(|h| h.parse::<u32>().ok()).unwrap_or(2);
    let minute = parts.get(1).and_then(|m| m.parse::<u32>().ok()).unwrap_or(0);
    (hour, minute)
}

fn command_exists(cmd: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn get_executable_command() -> String {
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy().to_string();
        if !exe_str.contains("target/debug/deps") && !exe_str.contains("target/release/deps") {
            return format!("\"{}\"", exe_str);
        }
    }
    "experienced-llms".to_string()
}

pub fn install_schedule(
    time_str: &str,
    scheduler_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (hour, minute) = parse_time(time_str);
    let st_val = format!("{:02}:{:02}", hour, minute);

    config::ensure_directories()?;
    let exe_cmd = get_executable_command();

    // 1. Native Windows Task Scheduler
    #[cfg(windows)]
    if scheduler_type == "auto" || scheduler_type == "schtasks" {
        let tr_val = format!("{} consolidate", exe_cmd);
        let output = Command::new("schtasks")
            .args([
                "/create",
                "/tn",
                TASK_NAME,
                "/tr",
                &tr_val,
                "/sc",
                "daily",
                "/st",
                &st_val,
                "/f",
            ])
            .output()?;

        if output.status.success() {
            return Ok(format!(
                "Windows Scheduled Task '{}' registered daily at {}.",
                TASK_NAME, st_val
            ));
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(format!("Failed to register Windows Scheduled Task: {}", err).into());
        }
    }

    // 2. WSL with Windows Task Scheduler preference
    #[cfg(not(windows))]
    if (scheduler_type == "schtasks" || (scheduler_type == "auto" && command_exists("schtasks.exe")))
        && command_exists("schtasks.exe")
    {
        let wsl_cmd = "wsl.exe -d Ubuntu -- bash -lc \"experienced-llms consolidate\"";
        let output = Command::new("schtasks.exe")
            .args([
                "/create",
                "/tn",
                TASK_NAME,
                "/tr",
                wsl_cmd,
                "/sc",
                "daily",
                "/st",
                &st_val,
                "/f",
            ])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                return Ok(format!(
                    "Windows Scheduled Task '{}' registered daily at {}.",
                    TASK_NAME, st_val
                ));
            }
        }
        if scheduler_type == "schtasks" {
            return Err("Failed to register Windows Task Scheduler via schtasks.exe".into());
        }
    }

    // 3. POSIX Crontab
    #[cfg(not(windows))]
    {
        let log_file = config::get_experienced_home().join("cron.log");
        let cron_line = format!(
            "{} {} * * * {} consolidate >> {} 2>&1 {}",
            minute,
            hour,
            exe_cmd,
            log_file.display(),
            CRON_COMMENT
        );

        let read_output = Command::new("crontab").arg("-l").output();
        let existing_lines = match read_output {
            Ok(ref out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        let mut new_lines: Vec<String> = existing_lines
            .into_iter()
            .filter(|l| !l.contains(CRON_COMMENT) && !l.contains("consolidate"))
            .collect();
        new_lines.push(cron_line);

        let mut child = Command::new("crontab")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn crontab: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            let input_text = new_lines.join("\n") + "\n";
            stdin.write_all(input_text.as_bytes())?;
        }

        let write_output = child.wait_with_output()?;
        if write_output.status.success() {
            return Ok(format!(
                "Crontab entry installed daily at {:02}:{:02}.",
                hour, minute
            ));
        } else {
            let err = String::from_utf8_lossy(&write_output.stderr).trim().to_string();
            return Err(format!("Failed to update crontab: {}", err).into());
        }
    }

    #[cfg(windows)]
    Ok(format!(
        "Schedule configured daily at {:02}:{:02}.",
        hour, minute
    ))
}

pub fn uninstall_schedule(
    scheduler_type: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut messages = Vec::new();

    // 1. Remove from Windows Task Scheduler
    #[cfg(windows)]
    if scheduler_type == "auto" || scheduler_type == "schtasks" {
        let output = Command::new("schtasks")
            .args(["/delete", "/tn", TASK_NAME, "/f"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                messages.push(format!("Task '{}' deleted from Windows Task Scheduler.", TASK_NAME));
            }
        }
    }

    #[cfg(not(windows))]
    if (scheduler_type == "auto" || scheduler_type == "schtasks") && command_exists("schtasks.exe") {
        let output = Command::new("schtasks.exe")
            .args(["/delete", "/tn", TASK_NAME, "/f"])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                messages.push(format!("Task '{}' deleted from Windows Task Scheduler.", TASK_NAME));
            }
        }
    }

    // 2. Remove from Crontab
    #[cfg(not(windows))]
    if scheduler_type == "auto" || scheduler_type == "cron" {
        let read_output = Command::new("crontab").arg("-l").output();
        if let Ok(out) = read_output {
            if out.status.success() {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                if stdout_str.contains(CRON_COMMENT) || stdout_str.contains("consolidate") {
                    let new_lines: Vec<&str> = stdout_str
                        .lines()
                        .filter(|l| !l.contains(CRON_COMMENT) && !l.contains("consolidate"))
                        .collect();

                    let child = Command::new("crontab")
                        .arg("-")
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn();

                    if let Ok(mut c) = child {
                        if let Some(mut stdin) = c.stdin.take() {
                            let input_text = new_lines.join("\n") + "\n";
                            let _ = stdin.write_all(input_text.as_bytes());
                        }
                        if let Ok(res) = c.wait_with_output() {
                            if res.status.success() {
                                messages.push("Removed entry from crontab.".to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    if messages.is_empty() {
        Ok("No active schedule found to remove.".to_string())
    } else {
        Ok(messages.join(" "))
    }
}

pub fn get_schedule_status(scheduler_type: &str) -> String {
    let mut statuses = Vec::new();

    // Check Windows Task Scheduler
    #[cfg(windows)]
    if scheduler_type == "auto" || scheduler_type == "schtasks" {
        if let Ok(out) = Command::new("schtasks").args(["/query", "/tn", TASK_NAME]).output() {
            if out.status.success() {
                statuses.push(format!("ACTIVE (Windows Task Scheduler: '{}')", TASK_NAME));
            }
        }
    }

    #[cfg(not(windows))]
    if (scheduler_type == "auto" || scheduler_type == "schtasks") && command_exists("schtasks.exe") {
        if let Ok(out) = Command::new("schtasks.exe").args(["/query", "/tn", TASK_NAME]).output() {
            if out.status.success() {
                statuses.push(format!("ACTIVE (Windows Task Scheduler: '{}')", TASK_NAME));
            }
        }
    }

    // Check Crontab
    #[cfg(not(windows))]
    if scheduler_type == "auto" || scheduler_type == "cron" {
        if let Ok(out) = Command::new("crontab").arg("-l").output() {
            if out.status.success() {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                for line in stdout_str.lines() {
                    if line.contains(CRON_COMMENT) || line.contains("consolidate") {
                        statuses.push(format!("ACTIVE (Crontab: '{}')", line.trim()));
                        break;
                    }
                }
            }
        }
    }

    if statuses.is_empty() {
        "NOT CONFIGURED".to_string()
    } else {
        statuses.join("\n  ")
    }
}
