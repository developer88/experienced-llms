# ==============================================================================
# Automated Test Suite for Windows Installer (install.ps1)
# Tests local install, streamed download from GitHub Releases, and error handling
# ==============================================================================
$ErrorActionPreference = "Stop"

$ScriptDir = $PSScriptRoot
$RepoRoot = (Resolve-Path "$ScriptDir\..").Path

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "  Running Windows Installer Automated Tests (install.ps1)" -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

function Run-EncodedScript {
    param(
        [Parameter(Mandatory=$true)][string]$ScriptText,
        [string]$StdOutPath,
        [string]$StdErrPath
    )
    $bytes = [System.Text.Encoding]::Unicode.GetBytes($ScriptText)
    $encoded = [Convert]::ToBase64String($bytes)
    
    $argsList = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-EncodedCommand", $encoded)
    $procInfo = @{
        FilePath = "powershell.exe"
        ArgumentList = $argsList
        Wait = $true
        PassThru = $true
        NoNewWindow = $true
    }
    if ($StdOutPath) { $procInfo["RedirectStandardOutput"] = $StdOutPath }
    if ($StdErrPath) { $procInfo["RedirectStandardError"] = $StdErrPath }

    $p = Start-Process @procInfo
    return $p.ExitCode
}

# Preserve original User PATH to guarantee zero host pollution
$OrigUserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$SandboxRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("exp_win_test_" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $SandboxRoot | Out-Null

Write-Host "[+] Isolated test sandbox created at: $SandboxRoot" -ForegroundColor Yellow

try {
    # --------------------------------------------------------------------------
    # Test 1: Local Repo Installation Mode
    # --------------------------------------------------------------------------
    Write-Host ""
    Write-Host "--- [Test 1] Testing Local Repo Installation Mode ---" -ForegroundColor Cyan
    $Sandbox1 = Join-Path $SandboxRoot "test1_local"
    $LocalApp1 = Join-Path $Sandbox1 "LocalAppData"
    $UserProf1 = Join-Path $Sandbox1 "UserProfile"

    $ExpHome1 = Join-Path $UserProf1 ".experienced-llms"
    $test1Script = @(
        ('$env:LOCALAPPDATA = "' + $LocalApp1 + '"'),
        ('$env:USERPROFILE = "' + $UserProf1 + '"'),
        ('$env:EXPERIENCED_LLMS_HOME = "' + $ExpHome1 + '"'),
        ('Set-Location "' + $RepoRoot + '"'),
        ('& .\install.ps1')
    ) -join "`n"

    $exit1 = Run-EncodedScript -ScriptText $test1Script
    if ($exit1 -ne 0) {
        throw ("Test 1 FAILED: install.ps1 exited with code " + $exit1)
    }

    $ExpectedExe1 = Join-Path $LocalApp1 "Programs\experienced-llms\bin\experienced-llms.exe"
    if (-not (Test-Path $ExpectedExe1)) {
        throw ("Test 1 FAILED: Expected binary " + $ExpectedExe1 + " does not exist.")
    }

    $ExpectedDb1 = Join-Path $UserProf1 ".experienced-llms\memory.db"
    if (-not (Test-Path $ExpectedDb1)) {
        throw ("Test 1 FAILED: Expected SQLite database " + $ExpectedDb1 + " does not exist.")
    }

    $ExpectedMd1 = Join-Path $UserProf1 ".experienced-llms\EXPERIENCE.md"
    if (-not (Test-Path $ExpectedMd1)) {
        throw ("Test 1 FAILED: Expected master experience file " + $ExpectedMd1 + " does not exist.")
    }

    Write-Host "[PASS] Test 1: Local repo installation succeeded in sandbox." -ForegroundColor Green

    # --------------------------------------------------------------------------
    # Test 2: Streamed Remote Download Mode (simulating irm ... | iex)
    # --------------------------------------------------------------------------
    Write-Host ""
    Write-Host "--- [Test 2] Testing Streamed Remote Download Mode ---" -ForegroundColor Cyan
    $Sandbox2 = Join-Path $SandboxRoot "test2_streamed"
    $LocalApp2 = Join-Path $Sandbox2 "LocalAppData"
    $UserProf2 = Join-Path $Sandbox2 "UserProfile"
    $EmptyDir = Join-Path $SandboxRoot "empty_workspace"
    New-Item -ItemType Directory -Force -Path $EmptyDir | Out-Null

    # When piped via irm | iex, $PSScriptRoot is null
    $installerContent = Get-Content (Join-Path $RepoRoot "install.ps1") -Raw
    $installerBase64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($installerContent))

    $ExpHome2 = Join-Path $UserProf2 ".experienced-llms"
    $test2Script = @(
        ('$env:LOCALAPPDATA = "' + $LocalApp2 + '"'),
        ('$env:USERPROFILE = "' + $UserProf2 + '"'),
        ('$env:EXPERIENCED_LLMS_HOME = "' + $ExpHome2 + '"'),
        ('Set-Location "' + $EmptyDir + '"'),
        ('$scriptToRun = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String("' + $installerBase64 + '"))'),
        ('Invoke-Expression $scriptToRun')
    ) -join "`n"

    $exit2 = Run-EncodedScript -ScriptText $test2Script
    if ($exit2 -ne 0) {
        throw ("Test 2 FAILED: Streamed install.ps1 exited with code " + $exit2)
    }

    $ExpectedExe2 = Join-Path $LocalApp2 "Programs\experienced-llms\bin\experienced-llms.exe"
    if (-not (Test-Path $ExpectedExe2)) {
        throw ("Test 2 FAILED: Expected downloaded binary " + $ExpectedExe2 + " does not exist.")
    }

    $verOutput = & "$ExpectedExe2" --version
    Write-Host "[+] Downloaded Windows binary version: $verOutput" -ForegroundColor Green

    $ExpectedDb2 = Join-Path $UserProf2 ".experienced-llms\memory.db"
    if (-not (Test-Path $ExpectedDb2)) {
        throw ("Test 2 FAILED: Expected SQLite database " + $ExpectedDb2 + " does not exist.")
    }

    Write-Host "[PASS] Test 2: Remote download installation succeeded." -ForegroundColor Green

    # --------------------------------------------------------------------------
    # Test 3: Fallback and Error Handling on Download Failure
    # --------------------------------------------------------------------------
    Write-Host ""
    Write-Host "--- [Test 3] Testing Download Failure & No-Cargo Instructions ---" -ForegroundColor Cyan
    $Sandbox3 = Join-Path $SandboxRoot "test3_fail"
    $LocalApp3 = Join-Path $Sandbox3 "LocalAppData"
    $UserProf3 = Join-Path $Sandbox3 "UserProfile"
    New-Item -ItemType Directory -Force -Path $Sandbox3 | Out-Null

    # Script pointing to non-existent repo to trigger download failure
    $failingScript = $installerContent.Replace('Repo = "developer88/experienced-llms"', 'Repo = "developer88/nonexistent-repo-test-12345"')
    $failingBase64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($failingScript))

    $ExpHome3 = Join-Path $UserProf3 ".experienced-llms"
    $test3Script = @(
        ('$env:LOCALAPPDATA = "' + $LocalApp3 + '"'),
        ('$env:USERPROFILE = "' + $UserProf3 + '"'),
        ('$env:EXPERIENCED_LLMS_HOME = "' + $ExpHome3 + '"'),
        ('$env:PATH = "C:\Windows\System32;C:\Windows"'),
        ('Set-Location "' + $EmptyDir + '"'),
        ('$scriptToRun = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String("' + $failingBase64 + '"))'),
        ('Invoke-Expression $scriptToRun')
    ) -join "`n"

    $test3Log = Join-Path $Sandbox3 "fail_output.log"
    $test3Err = Join-Path $Sandbox3 "fail_err.log"

    $exit3 = Run-EncodedScript -ScriptText $test3Script -StdOutPath $test3Log -StdErrPath $test3Err

    $logContent = ""
    if (Test-Path $test3Log) { $logContent += (Get-Content $test3Log -Raw) }
    if (Test-Path $test3Err) { $logContent += (Get-Content $test3Err -Raw) }

    if ($logContent -match "Failed to download prebuilt Windows binary" -and $logContent -match "rustup.rs") {
        Write-Host "[PASS] Test 3: Expected error message and Rustup instructions cleanly displayed." -ForegroundColor Green
    } else {
        Write-Host ("[-] Test 3 Output was: " + $logContent) -ForegroundColor Yellow
        throw "Test 3 FAILED: Clean error message and instructions were not found."
    }

    Write-Host ""
    Write-Host "=================================================================" -ForegroundColor Cyan
    Write-Host "[SUCCESS] All Windows installer test suites passed perfectly!" -ForegroundColor Green
    Write-Host "=================================================================" -ForegroundColor Cyan
} finally {
    # Clean up and restore User PATH
    Write-Host "[+] Restoring original User PATH environment variable..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("Path", $OrigUserPath, "User")
    
    # Remove sandbox files
    if (Test-Path $SandboxRoot) {
        Remove-Item -Recurse -Force $SandboxRoot -ErrorAction SilentlyContinue
    }
}
