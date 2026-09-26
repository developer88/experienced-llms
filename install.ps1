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
    } else {
        Write-Host "[-] Warning: Neither Windows Python 3 nor WSL was detected." -ForegroundColor Red
    }
}

# 2. Install Antigravity Skill in Windows User Profile if directory exists
$GeminiDir = Join-Path $HOME ".gemini"
if (Test-Path $GeminiDir) {
    Write-Host "[+] Antigravity agent detected in Windows profile! Installing agent skill..." -ForegroundColor Green
    if (-not (Test-Path $SkillDir)) {
        New-Item -ItemType Directory -Path $SkillDir -Force | Out-Null
    }
    Copy-Item -Path (Join-Path $ScriptDir "skill\SKILL.md") -Destination (Join-Path $SkillDir "SKILL.md") -Force
    Write-Host "[+] Skill installed to: $SkillDir\SKILL.md" -ForegroundColor Green
}

Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "[✔] Installation Completed!" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Cyan
