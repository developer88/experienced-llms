# One-Click Installer for Experienced LLMs (Windows PowerShell)
$ErrorActionPreference = "Continue"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExperiencedHome = Join-Path $HOME ".experienced-llms"
$SkillDir = Join-Path $HOME ".gemini\config\skills\experienced-llms"

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "  Experienced LLMs: Windows Installer" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

# 1. Check Python on Windows or fallback to WSL
$hasWindowsPython = $false
try {
    $pyVer = & python --version 2>&1
    if ($LASTEXITCODE -eq 0 -and "$pyVer" -match "Python 3") {
        $hasWindowsPython = $true
    }
} catch {}

if ($hasWindowsPython) {
    Write-Host "[+] Native Windows Python 3 detected." -ForegroundColor Green
    & python -c "import sys; sys.path.insert(0, r'$ScriptDir'); from src.config import ensure_directories; ensure_directories(); from src.db import init_db; init_db()"

    # Register Windows Task Scheduler
    Write-Host "[+] Registering Windows Task Scheduler nightly consolidation..." -ForegroundColor Green
    & python (Join-Path $ScriptDir "src\cli.py") schedule --install --time "02:00"
} else {
    $hasWsl = Get-Command wsl -ErrorAction SilentlyContinue
    if ($hasWsl) {
        Write-Host "[!] Native Windows Python 3 not found. WSL detected!" -ForegroundColor Yellow
        Write-Host "[+] Delegating installation to WSL environment..." -ForegroundColor Green
        # Convert path to WSL mount path
        $wslPath = "/mnt/" + $ScriptDir.Substring(0,1).ToLower() + $ScriptDir.Substring(2).Replace("\", "/")
        wsl -e bash -c "cd '$wslPath' && bash install.sh"

        # Link ~/.experienced-llms in Windows to WSL storage
        if (-not (Test-Path $ExperiencedHome)) {
            try {
                New-Item -ItemType SymbolicLink -Path $ExperiencedHome -Target "\\wsl.localhost\Ubuntu\home\$env:USERNAME\.experienced-llms" -ErrorAction SilentlyContinue | Out-Null
            } catch {}
        }

        # Install Windows CLI shim to an existing PATH folder
        $candidatePaths = @(
            (Join-Path $env:LOCALAPPDATA "agy\bin"),
            (Join-Path $HOME ".gemini\antigravity\bin"),
            (Join-Path $env:APPDATA "npm")
        )
        foreach ($binFolder in $candidatePaths) {
            if (Test-Path $binFolder) {
                Set-Content -Path (Join-Path $binFolder "experienced-llms.cmd") -Value "@wsl -e bash -c 'experienced-llms %*'" -Encoding ASCII
                break
            }
        }

        # Configure Windows Task Scheduler for WSL consolidation (replaces WSL crontab)
        Write-Host "[+] Registering Windows Task Scheduler nightly consolidation (02:00 AM)..." -ForegroundColor Green
        wsl -e bash -lc "experienced-llms schedule --uninstall" | Out-Null
        $trCmd = 'wsl.exe -d Ubuntu -- bash -lc \"experienced-llms consolidate\"'
        schtasks /create /tn "ExperiencedLLMsConsolidator" /tr $trCmd /sc daily /st 02:00 /f | Out-Null

        # Also configure Windows startup trigger so consolidation runs whenever you turn on your PC
        $StartupFolder = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Startup"
        if (Test-Path $StartupFolder) {
            Set-Content -Path (Join-Path $StartupFolder "ExperiencedLLMsConsolidateOnBoot.cmd") -Value '@start /b "" wsl.exe -d Ubuntu -- bash -lc "experienced-llms consolidate"' -Encoding ASCII
        }
    } else {
        Write-Host "[-] Warning: Neither Windows Python 3 nor WSL was detected." -ForegroundColor Red
    }
}

# 2. Configure Agent Integrations (Antigravity, Claude, Copilot, Cursor, Pi)
Write-Host "[+] Configuring AI Agent integrations..." -ForegroundColor Green
if ($hasWindowsPython) {
    & python (Join-Path $ScriptDir "src\cli.py") integrate --target all
} else {
    # Antigravity fallback
    $GeminiDir = Join-Path $HOME ".gemini"
    if (Test-Path $GeminiDir) {
        if (-not (Test-Path $SkillDir)) {
            New-Item -ItemType Directory -Path $SkillDir -Force | Out-Null
        }
        Copy-Item -Path (Join-Path $ScriptDir "skill\SKILL.md") -Destination (Join-Path $SkillDir "SKILL.md") -Force
        Write-Host "[+] Antigravity skill installed: $SkillDir\SKILL.md" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "[SUCCESS] Installation Completed!" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Cyan

