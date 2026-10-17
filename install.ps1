# ==============================================================================
# Experienced LLMs: Windows 11 PowerShell One-Line Installer
# Usage:
#   irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex
# ==============================================================================
$ErrorActionPreference = "Stop"

$Repo = "developer88/experienced-llms"
$VersionTag = "latest"
$AssetName = "experienced-llms-windows-x86_64.exe"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Experienced LLMs: Autonomous Cognitive Memory Installer" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

$InstallDir = "$env:LOCALAPPDATA\Programs\experienced-llms\bin"
$DestExe = Join-Path $InstallDir "experienced-llms.exe"
$ExperiencedHome = "$env:USERPROFILE\.experienced-llms"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ExperiencedHome | Out-Null

# 1. Check local build or dist first (if running within repository)
$InstalledLocal = $false
$CurrentScriptDir = $PSScriptRoot

if ($CurrentScriptDir) {
    $LocalDist = Join-Path $CurrentScriptDir "dist\$AssetName"
    $LocalRelease = Join-Path $CurrentScriptDir "target\release\experienced-llms.exe"

    if (Test-Path $LocalDist) {
        Write-Host "[+] Installing from local dist: $LocalDist" -ForegroundColor Green
        Copy-Item -Path $LocalDist -Destination $DestExe -Force
        $InstalledLocal = $true
    } elseif (Test-Path $LocalRelease) {
        Write-Host "[+] Installing from local Cargo release: $LocalRelease" -ForegroundColor Green
        Copy-Item -Path $LocalRelease -Destination $DestExe -Force
        $InstalledLocal = $true
    } elseif ((Test-Path (Join-Path $CurrentScriptDir "Cargo.toml")) -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "[+] Compiling standalone binary from local source via Cargo..." -ForegroundColor Green
        Push-Location $CurrentScriptDir
        cargo build --release
        Pop-Location
        if (Test-Path $LocalRelease) {
            Copy-Item -Path $LocalRelease -Destination $DestExe -Force
            $InstalledLocal = $true
        }
    }
}

# 2. Download from GitHub Release if not local
if (-not $InstalledLocal) {
    $DownloadUrl = "https://github.com/$Repo/releases/$VersionTag/download/$AssetName"
    Write-Host "[+] Downloading native Windows binary from: $DownloadUrl" -ForegroundColor Yellow

    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $DestExe -UseBasicParsing
}

Write-Host "[✔] Installed standalone native binary to: $DestExe" -ForegroundColor Green

# 3. PATH Registration (Persistent User PATH)
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    $NewUserPath = "$InstallDir;$UserPath".TrimEnd(';')
    [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host "[+] Added $InstallDir to User PATH environment variable." -ForegroundColor Green
}

# 4. Initialize SQLite storage and directories
Write-Host "[+] Initializing Experienced LLMs storage..." -ForegroundColor Cyan
& "$DestExe" init

# 5. Configure AI Agent integrations (Antigravity, Claude, Copilot, Cursor, Pi)
Write-Host "[+] Configuring AI Agent integrations..." -ForegroundColor Cyan
& "$DestExe" integrate --target all

# 6. Configure Nightly Consolidation Schedule
Write-Host "[+] Installing nightly consolidation schedule in Windows Task Scheduler..." -ForegroundColor Cyan
& "$DestExe" schedule --install --time "02:00"

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  [✔] Experienced LLMs Successfully Installed & Active!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
& "$DestExe" status
Write-Host ""
Write-Host "Quick Commands:" -ForegroundColor Yellow
Write-Host "  experienced-llms log `"Always run npm in WSL`" --category user_preference"
Write-Host "  experienced-llms show"
Write-Host "  experienced-llms status"
Write-Host "  experienced-llms setup --provider [gemini|claude|openai|ollama]"
Write-Host "  experienced-llms consolidate"
Write-Host "==========================================================" -ForegroundColor Cyan
