# ==============================================================================
# Experienced LLMs: Windows 11 PowerShell One-Line Installer
# Usage:
#   irm https://github.com/developer88/experienced-llms/releases/latest/download/install.ps1 | iex
# ==============================================================================
param(
  [string]$Tag = "latest",
  [string]$InstallDir = "$env:LOCALAPPDATA\Programs\experienced-llms\bin",
  [switch]$Yes,
  [switch]$NoAnsi
)

$ErrorActionPreference = "Stop"

$Repo = "developer88/experienced-llms"
$VersionTag = $Tag
$AssetName = "experienced-llms-windows-x86_64.exe"

# ------------------------------------------------------------------------------
# Terminal & ANSI Color Capabilities
# ------------------------------------------------------------------------------
$Esc = [char]27
$Reset = "$Esc[0m"
$Bold = "$Esc[1m"
$Dim = "$Esc[2m"

# Project Palette (Matches assets/logo.jpg):
# Obsidian dark background, electric cyan, neon turquoise, deep violet/indigo, vibrant cobalt, radiant white
$ColorCyan   = "$Esc[38;2;0;229;255m"
$ColorTurq   = "$Esc[38;2;29;233;182m"
$ColorIndigo = "$Esc[38;2;124;77;255m"
$ColorViolet = "$Esc[38;2;179;136;255m"
$ColorBlue   = "$Esc[38;2;41;121;255m"
$ColorWhite  = "$Esc[38;2;255;255;255m"
$ColorGreen  = "$Esc[38;2;0;230;118m"
$ColorYellow = "$Esc[38;2;255;215;64m"
$ColorRed    = "$Esc[38;2;255;82;82m"
$ColorGray   = "$Esc[38;2;148;163;184m"

$Block = ([string][char]0x2588) * 2

function Test-InstallerInteractiveOutput {
  return (-not [Console]::IsOutputRedirected) -and ($env:TERM -ne "dumb")
}

function Enable-VirtualTerminalOutput {
  if (-not (Test-InstallerInteractiveOutput)) {
    return $false
  }
  if ($null -ne $script:VirtualTerminalEnabled) {
    return $script:VirtualTerminalEnabled
  }

  try {
    if (-not ("ConsoleMode.NativeMethods" -as [Type])) {
      Add-Type -Namespace ConsoleMode -Name NativeMethods -MemberDefinition @"
[DllImport("kernel32.dll", SetLastError = true)]
public static extern System.IntPtr GetStdHandle(int nStdHandle);

[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool GetConsoleMode(System.IntPtr hConsoleHandle, out uint lpMode);

[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool SetConsoleMode(System.IntPtr hConsoleHandle, uint dwMode);
"@
    }

    $stdOutputHandle = -11
    $enableVirtualTerminalProcessing = 0x0004
    $handle = [ConsoleMode.NativeMethods]::GetStdHandle($stdOutputHandle)
    $mode = [uint32]0
    $script:VirtualTerminalEnabled = [ConsoleMode.NativeMethods]::GetConsoleMode($handle, [ref]$mode) -and 
                                      [ConsoleMode.NativeMethods]::SetConsoleMode($handle, ($mode -bor $enableVirtualTerminalProcessing))
    return $script:VirtualTerminalEnabled
  } catch {
    $script:VirtualTerminalEnabled = $false
    return $false
  }
}

function Test-InstallerAnsiOutput {
  if ($NoAnsi) { return $false }
  return Enable-VirtualTerminalOutput
}

function Test-TerminalSupportsUnicode {
  if ($env:WT_SESSION -or $env:TERMINAL_EMULATOR -or ($env:TERM_PROGRAM -in @("vscode", "WezTerm", "Apple_Terminal", "iTerm.app"))) {
    return $true
  }
  $locale = "$env:LC_ALL$env:LC_CTYPE$env:LANG"
  if ($locale -match "(?i)utf-?8") { return $true }
  return $true
}

function Get-SpinnerFrame {
  param([int]$Step)
  if (Test-TerminalSupportsUnicode) {
    $frames = @([char]0x280B, [char]0x2819, [char]0x2839, [char]0x2838, [char]0x283C, [char]0x2834, [char]0x2826, [char]0x2827, [char]0x2807, [char]0x280F)
    return [string]$frames[$Step % $frames.Length]
  }
  switch ($Step % 4) {
    0 { return "-" }
    1 { return "\" }
    2 { return "|" }
    default { return "/" }
  }
}

function Get-VisibleTextLength {
  param([string]$Text)
  $escaped = [regex]::Escape([string]$Esc)
  $plain = [regex]::Replace($Text, "${escaped}\[[0-9;?]*[ -/]*[@-~]", "")
  return $plain.Length
}

# ------------------------------------------------------------------------------
# Logo & Animated Intro
# ------------------------------------------------------------------------------
$script:LogoMap = @(
  @("sp",     "sp",     "sp",     "turq",   "sp",     "turq",   "sp",     "sp",     "sp"),
  @("sp",     "sp",     "turq",   "white",  "cyan",   "white",  "turq",   "sp",     "sp"),
  @("sp",     "indigo", "sp",     "cyan",   "white",  "cyan",   "sp",     "blue",   "sp"),
  @("indigo", "violet", "indigo", "sp",     "cyan",   "sp",     "blue",   "cyan",   "blue"),
  @("indigo", "sp",     "violet", "cyan",   "white",  "cyan",   "blue",   "sp",     "blue"),
  @("indigo", "violet", "indigo", "sp",     "cyan",   "sp",     "blue",   "cyan",   "blue"),
  @("sp",     "indigo", "sp",     "indigo", "white",  "blue",   "sp",     "blue",   "sp"),
  @("sp",     "sp",     "indigo", "violet", "sp",     "cyan",   "blue",   "sp",     "sp"),
  @("sp",     "sp",     "sp",     "indigo", "sp",     "blue",   "sp",     "sp",     "sp")
)

function Write-StaticLogo {
  $b = [string][char]0x2588
  $b2 = $b * 2
  $b4 = $b * 4
  $b6 = $b * 6
  $b10 = $b * 10
  Write-Host ""
  Write-Host "          $b2  $b2"
  Write-Host "        $b10"
  Write-Host "      $b2  $b6  $b2"
  Write-Host "    $b6  $b2  $b6"
  Write-Host "    $b2  $b10  $b2"
  Write-Host "    $b6  $b2  $b6"
  Write-Host "      $b2  $b6  $b2"
  Write-Host "        $b4  $b4"
  Write-Host "          $b2  $b2"
  Write-Host ""
}

function Render-LogoFrame {
  param(
    [int]$Step = 9,
    [int]$FlashRow = -1,
    [bool]$PulseCore = $false
  )

  $cells = @{
    sp     = "$Reset  "
    cyan   = "${ColorCyan}$Block"
    turq   = "${ColorTurq}$Block"
    indigo = "${ColorIndigo}$Block"
    violet = "${ColorViolet}$Block"
    blue   = "${ColorBlue}$Block"
    white  = "${ColorWhite}$Block"
  }

  $clear = "$Esc[H"
  $out = $clear
  for ($r = 0; $r -lt 9; $r++) {
    $line = "    "
    for ($c = 0; $c -lt 9; $c++) {
      $type = $script:LogoMap[$r][$c]
      if ($type -eq "sp") {
        $line += $cells["sp"]
        continue
      }

      $dist = [Math]::Abs($r - 4) + [Math]::Abs($c - 4)
      if ($dist -gt $Step) {
        $line += $cells["sp"]
        continue
      }

      if ($FlashRow -eq $r) {
        $line += "${ColorWhite}$Block"
      } elseif ($PulseCore -and ($r -eq 4) -and ($c -eq 4)) {
        $line += "${ColorWhite}$Block"
      } else {
        $line += $cells[$type]
      }
    }
    $out += "$line$Reset`n"
  }
  [Console]::Write($out)
}

function Show-ExperiencedLogoAnimation {
  if (-not (Test-InstallerAnsiOutput)) {
    Write-StaticLogo
    return
  }

  # Hide cursor and clear viewport
  [Console]::Write("${Esc}[?25l${Esc}[2J${Esc}[H")

  # 1. Neural Spark & Axon Ripple outwards from core
  for ($s = 0; $s -le 8; $s++) {
    Render-LogoFrame -Step $s
    Start-Sleep -Milliseconds 55
  }

  # 2. Central synaptic core pulse
  Render-LogoFrame -Step 8 -PulseCore $true
  Start-Sleep -Milliseconds 120
  Render-LogoFrame -Step 8 -PulseCore $false
  Start-Sleep -Milliseconds 90
  Render-LogoFrame -Step 8 -PulseCore $true
  Start-Sleep -Milliseconds 140
  Render-LogoFrame -Step 8 -PulseCore $false
  Start-Sleep -Milliseconds 100

  # 3. Quantum energy wave cascading from top to bottom
  for ($r = 0; $r -lt 9; $r++) {
    Render-LogoFrame -Step 8 -FlashRow $r
    Start-Sleep -Milliseconds 40
  }
  Render-LogoFrame -Step 8
  Start-Sleep -Milliseconds 180

  # Restore cursor
  [Console]::Write("${Esc}[?25h`n")
}

function Write-InstallerTitle {
  if (Test-InstallerAnsiOutput) {
    [Console]::Write("${Bold}  Experienced LLMs Installer${Reset}`n${ColorTurq}  Autonomous Cognitive Memory & Heuristic Ledger for AI Agents${Reset}`n`n")
  } else {
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host "  Experienced LLMs: Autonomous Cognitive Memory Installer" -ForegroundColor Cyan
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host ""
  }
}

# ------------------------------------------------------------------------------
# Progress Reporting & Status Output
# ------------------------------------------------------------------------------
function Write-InstallProgress {
  param(
    [int]$Step,
    [string]$Frame,
    [string]$Label,
    [string]$Title,
    [int]$Percent = -1
  )

  if (-not (Test-InstallerAnsiOutput)) { return }

  $width = 24
  $trail = 6
  $head = $Step % ($width + $trail)
  $bar = ""

  for ($i = 0; $i -lt $width; $i++) {
    $age = $head - $i
    if ($Percent -ge 0 -and $i -le [int](($Percent / 100.0) * $width)) {
      $bar += "${ColorCyan}#${Reset}"
    } elseif ($age -ge 0 -and $age -lt $trail) {
      switch ($age) {
        { $_ -in 0, 1 } { $cell = "${ColorWhite}#${Reset}"; break }
        { $_ -in 2, 3 } { $cell = "${ColorCyan}#${Reset}"; break }
        4               { $cell = "${ColorTurq}#${Reset}"; break }
        default         { $cell = "${ColorIndigo}#${Reset}"; break }
      }
      $bar += $cell
    } else {
      $bar += "${Dim}-${Reset}"
    }
  }

  $pctText = if ($Percent -ge 0) { " ${ColorTurq}${Percent}%${Reset}" } else { "" }
  $line = "  ${ColorCyan}${Frame}${Reset} ${bar}${pctText} ${Bold}${Title}${Reset} ${ColorGray}${Label}${Reset}"

  $visibleLen = Get-VisibleTextLength $line
  $pad = ""
  if ($script:LastProgressWidth -gt $visibleLen) {
    $pad = " " * ($script:LastProgressWidth - $visibleLen)
  }
  $script:LastProgressWidth = $visibleLen

  [Console]::Write("`r${line}${pad}")
}

function Finish-InstallProgress {
  if ($script:LastProgressWidth -gt 0) {
    [Console]::Write("`r" + (" " * $script:LastProgressWidth) + "`r")
  } else {
    [Console]::Write("`r")
  }
  $script:LastProgressWidth = 0
}

function Write-StepSuccess {
  param([string]$Message, [string]$Detail = "")
  $check = if (Test-TerminalSupportsUnicode) { [string][char]0x2714 } else { "+" }
  if (Test-InstallerAnsiOutput) {
    $detailStr = if ($Detail) { " ${ColorGray}(${Detail})${Reset}" } else { "" }
    [Console]::WriteLine("  ${ColorGreen}${check}${Reset} ${Bold}${Message}${Reset}${detailStr}")
  } else {
    $detailStr = if ($Detail) { " ($Detail)" } else { "" }
    Write-Host "[+] $Message$detailStr" -ForegroundColor Green
  }
}

function Write-StepWarning {
  param([string]$Message)
  $warn = if (Test-TerminalSupportsUnicode) { [string][char]0x26A0 } else { "!" }
  if (Test-InstallerAnsiOutput) {
    [Console]::WriteLine("  ${ColorYellow}${warn}${Reset} ${ColorYellow}${Message}${Reset}")
  } else {
    Write-Host "[!] $Message" -ForegroundColor Yellow
  }
}

function Write-StepError {
  param([string]$Message)
  $cross = if (Test-TerminalSupportsUnicode) { [string][char]0x2716 } else { "x" }
  if (Test-InstallerAnsiOutput) {
    [Console]::WriteLine("  ${ColorRed}${cross}${Reset} ${ColorRed}${Message}${Reset}")
  } else {
    Write-Host "[-] Error: $Message" -ForegroundColor Red
  }
}

# ------------------------------------------------------------------------------
# Environment & PATH Management
# ------------------------------------------------------------------------------
function Publish-EnvChange {
  try {
    if (-not ("Win32.NativeMethods" -as [Type])) {
      Add-Type -Namespace Win32 -Name NativeMethods -MemberDefinition @"
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern System.IntPtr SendMessageTimeout(
    System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
"@
    }
    $hwndBroadcast = [System.IntPtr]0xffff
    $wmSettingChange = 0x1a
    $result = [System.UIntPtr]::Zero
    [Win32.NativeMethods]::SendMessageTimeout($hwndBroadcast, $wmSettingChange, [System.UIntPtr]::Zero, "Environment", 2, 3000, [ref]$result) | Out-Null
  } catch {}
}

function Set-UserEnv {
  param([string]$Key, [string]$Value)
  $registerKey = Get-Item -Path "HKCU:"
  $envRegisterKey = $registerKey.OpenSubKey("Environment", $true)
  if ($null -eq $Value) {
    $envRegisterKey.DeleteValue($Key)
  } else {
    $registryValueKind = if ($Value.Contains("%")) {
      [Microsoft.Win32.RegistryValueKind]::ExpandString
    } elseif ($envRegisterKey.GetValue($Key)) {
      $envRegisterKey.GetValueKind($Key)
    } else {
      [Microsoft.Win32.RegistryValueKind]::String
    }
    $envRegisterKey.SetValue($Key, $Value, $registryValueKind)
  }
  Publish-EnvChange
}

function Add-UserPathEntry {
  param([string]$Directory)
  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if ($userPath -notlike "*$Directory*") {
    $newUserPath = if ($userPath) { "$Directory;$userPath".TrimEnd(';') } else { $Directory }
    Set-UserEnv -Key "Path" -Value $newUserPath
    $env:Path = "$Directory;$env:Path"
    Write-StepSuccess "User PATH registered" $Directory
  } else {
    $env:Path = "$Directory;$env:Path"
  }
}

# ------------------------------------------------------------------------------
# Summary Card
# ------------------------------------------------------------------------------
function Write-FinishCard {
  param(
    [string]$DestExe,
    [string]$ExperiencedHome,
    [string]$Version
  )

  $cardWidth = [Math]::Max(84, $DestExe.Length + 16)
  
  $chTL = [string][char]0x256D
  $chTR = [string][char]0x256E
  $chBL = [string][char]0x2570
  $chBR = [string][char]0x256F
  $chH  = [string][char]0x2500
  $chV  = [string][char]0x2502
  $chML = [string][char]0x251C
  $chMR = [string][char]0x2524
  $chCheck = [string][char]0x2714

  $lineTop = $chTL + ($chH * ($cardWidth - 2)) + $chTR
  $lineMid = $chML + ($chH * ($cardWidth - 2)) + $chMR
  $lineBot = $chBL + ($chH * ($cardWidth - 2)) + $chBR

  if (-not (Test-TerminalSupportsUnicode)) {
    $chV = "|"
    $lineTop = "+" + ("-" * ($cardWidth - 2)) + "+"
    $lineMid = "+" + ("-" * ($cardWidth - 2)) + "+"
    $lineBot = "+" + ("-" * ($cardWidth - 2)) + "+"
    $chCheck = "+"
  }

  function Format-CardLine {
    param([string]$Content)
    $visLen = Get-VisibleTextLength $Content
    $pad = " " * [Math]::Max(0, ($cardWidth - 4 - $visLen))
    return "  ${ColorCyan}${chV}${Reset}  ${Content}${pad}${ColorCyan}${chV}${Reset}"
  }

  if (Test-InstallerAnsiOutput) {
    [Console]::WriteLine("")
    [Console]::WriteLine("  ${ColorCyan}${lineTop}${Reset}")
    [Console]::WriteLine((Format-CardLine "${ColorGreen}${chCheck}${Reset} ${Bold}${ColorWhite}Experienced LLMs v${Version} is ready!${Reset}"))
    [Console]::WriteLine((Format-CardLine ""))
    [Console]::WriteLine((Format-CardLine "${ColorTurq}Binary:${Reset}   ${ColorGray}${DestExe}${Reset}"))
    [Console]::WriteLine((Format-CardLine "${ColorTurq}Storage:${Reset}  ${ColorGray}${ExperiencedHome}${Reset}"))
    [Console]::WriteLine("  ${ColorCyan}${lineMid}${Reset}")
    [Console]::WriteLine((Format-CardLine "${Bold}${ColorWhite}Quick Commands:${Reset}"))
    [Console]::WriteLine((Format-CardLine "  ${ColorYellow}experienced-llms status${Reset}      ${ColorGray}Inspect active facts & schedule${Reset}"))
    [Console]::WriteLine((Format-CardLine "  ${ColorYellow}experienced-llms log <text>${Reset}  ${ColorGray}Record operational wisdom${Reset}"))
    [Console]::WriteLine((Format-CardLine "  ${ColorYellow}experienced-llms config${Reset}      ${ColorGray}Configure LLM CLI & options${Reset}"))
    [Console]::WriteLine((Format-CardLine "  ${ColorYellow}experienced-llms update${Reset}      ${ColorGray}In-place self-update to latest${Reset}"))
    [Console]::WriteLine("  ${ColorCyan}${lineBot}${Reset}")
    [Console]::WriteLine("")
  } else {
    Write-Host ""
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host "  [OK] Experienced LLMs Successfully Installed & Active!" -ForegroundColor Green
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host "Binary:  $DestExe"
    Write-Host "Storage: $ExperiencedHome"
    Write-Host ""
    Write-Host "Quick Commands:" -ForegroundColor Yellow
    Write-Host "  experienced-llms log `"Always run npm in WSL`" --category user_preference"
    Write-Host "  experienced-llms status"
    Write-Host "  experienced-llms config"
    Write-Host "  experienced-llms update"
    Write-Host "==========================================================" -ForegroundColor Cyan
  }
}

# ==============================================================================
# Installer Execution Entrypoint
# ==============================================================================
$DestExe = Join-Path $InstallDir "experienced-llms.exe"
$ExperiencedHome = if ($env:EXPERIENCED_LLMS_HOME) { $env:EXPERIENCED_LLMS_HOME } else { "$env:USERPROFILE\.experienced-llms" }

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ExperiencedHome | Out-Null

Show-ExperiencedLogoAnimation
Write-InstallerTitle

# 1. Check local build or dist first (if running within repository)
$InstalledLocal = $false
$CurrentScriptDir = $PSScriptRoot

if ($CurrentScriptDir) {
  $LocalDist = Join-Path $CurrentScriptDir "dist\$AssetName"
  $LocalRelease = Join-Path $CurrentScriptDir "target\release\experienced-llms.exe"

  if (Test-Path $LocalDist) {
    Write-StepSuccess "Installing from local dist" $LocalDist
    Copy-Item -Path $LocalDist -Destination $DestExe -Force
    $InstalledLocal = $true
  } elseif (Test-Path $LocalRelease) {
    Write-StepSuccess "Installing from local Cargo release" $LocalRelease
    Copy-Item -Path $LocalRelease -Destination $DestExe -Force
    $InstalledLocal = $true
  } elseif ((Test-Path (Join-Path $CurrentScriptDir "Cargo.toml")) -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-StepSuccess "Compiling standalone binary from local source via Cargo..."
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
  $DownloadSuccess = $false

  if (Test-InstallerAnsiOutput) {
    # Interactive animated download with glowing comet and spinner
    try {
      [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
      $wc = New-Object System.Net.WebClient
      $prog = [PSCustomObject]@{ Done = $false; Error = $null; Percent = -1; Received = 0; Total = 0 }

      Register-ObjectEvent -InputObject $wc -EventName DownloadProgressChanged -Action {
        $prog.Percent = $EventArgs.ProgressPercentage
        $prog.Received = $EventArgs.BytesReceived
        $prog.Total = $EventArgs.TotalBytesToReceive
      } | Out-Null

      Register-ObjectEvent -InputObject $wc -EventName DownloadFileCompleted -Action {
        $prog.Done = $true
        if ($EventArgs.Error) { $prog.Error = $EventArgs.Error }
      } | Out-Null

      $tempTarget = "$DestExe.download"
      $wc.DownloadFileAsync([Uri]$DownloadUrl, $tempTarget)

      $step = 0
      while (-not $prog.Done) {
        $frame = Get-SpinnerFrame $step
        $label = if ($prog.Total -gt 0) {
          $mbRecv = [math]::Round($prog.Received / 1MB, 1)
          $mbTot = [math]::Round($prog.Total / 1MB, 1)
          "($mbRecv MB / $mbTot MB)"
        } else {
          "resolving release asset"
        }
        Write-InstallProgress $step $frame $label "Downloading release binary" $prog.Percent
        $step++
        Start-Sleep -Milliseconds 50
      }

      Finish-InstallProgress
      $wc.Dispose()

      if ($prog.Error) {
        throw $prog.Error
      }

      if (Test-Path $tempTarget) {
        Move-Item -Path $tempTarget -Destination $DestExe -Force
        $DownloadSuccess = $true
      }
    } catch {
      Finish-InstallProgress
      Write-StepWarning "Prebuilt binary download failed: $_"
      if (Test-Path "$DestExe.download") { Remove-Item "$DestExe.download" -Force -ErrorAction SilentlyContinue }
    }
  } else {
    # Non-interactive / CI / plain fallback
    try {
      [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
      Invoke-WebRequest -Uri $DownloadUrl -OutFile $DestExe -UseBasicParsing
      $DownloadSuccess = $true
    } catch {
      Write-StepWarning "Prebuilt binary download failed: $_"
    }
  }

  if (-not $DownloadSuccess) {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
      Write-StepSuccess "Cargo detected. Compiling and installing directly from source..."
      cargo install --git "https://github.com/$Repo" --bin experienced-llms --root "$env:LOCALAPPDATA\Programs\experienced-llms"
      if (-not (Test-Path $DestExe)) {
        Write-StepError "Cargo build completed but $DestExe was not found."
        exit 1
      }
    } else {
      Write-StepError "Failed to download prebuilt Windows binary and Cargo is not installed on this system."
      Write-Host ""
      Write-Host "To install Experienced LLMs from source:" -ForegroundColor Yellow
      Write-Host "  1. Install Rust toolchain: Download and run rustup-init.exe from https://rustup.rs"
      Write-Host "  2. Run:                   cargo install --git https://github.com/$Repo"
      Write-Host ""
      exit 1
    }
  }
}

Write-StepSuccess "Installed standalone native binary to: $DestExe"

# 3. PATH Registration (Persistent User PATH)
Add-UserPathEntry -Directory $InstallDir

# 4. Initialize SQLite storage and directories
$InitHelp = & "$DestExe" init --help 2>&1
if ($InitHelp -match "--yes") {
  & "$DestExe" init --yes 2>&1 | Out-Null
} else {
  & "$DestExe" init 2>&1 | Out-Null
}
Write-StepSuccess "SQLite memory engine initialized" $ExperiencedHome

# 5. Extract installed version
$installedVersion = "1.0.0rc6"
try {
  $verStr = & "$DestExe" --version 2>&1
  if ($verStr -match "experienced-llms ([\w\.\-]+)") {
    $installedVersion = $matches[1]
  }
} catch {}

# 6. Display Finish Card & Active Status
Write-FinishCard -DestExe $DestExe -ExperiencedHome $ExperiencedHome -Version $installedVersion
& "$DestExe" status
