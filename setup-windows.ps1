#Requires -Version 5.1
<#
.SYNOPSIS
    Initialize GlimmerX dev environment on Windows.
.DESCRIPTION
    Installs/checks:
      - Rust toolchain (via rustup)
      - Visual Studio C++ Build Tools (required by Tauri 2 / WebView2)
      - WebView2 Runtime (built into Win10/11, but verified)
      - Node.js (if missing)
      - Frontend dependencies (npm install)
      - Tauri CLI (cargo install)
#>
$ErrorActionPreference = "Stop"

$Green  = @{ ForegroundColor = "Green" }
$Yellow = @{ ForegroundColor = "Yellow" }
$Red    = @{ ForegroundColor = "Red" }

function Write-Info  { Write-Host "[setup] $($args -join ' ')" @Green }
function Write-Warn  { Write-Host "[warn]  $($args -join ' ')" @Yellow }
function Write-Error { Write-Host "[error] $($args -join ' ')" @Red; exit 1 }

# ── 1. Administrator check ────────────────────────────────────
function Test-Admin {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $p  = New-Object Security.Principal.WindowsPrincipal($id)
    return $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# ── 2. Rust toolchain ────────────────────────────────────────
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    Write-Info "Rust is already installed ($(rustc --version)). Updating..."
    rustup update stable
} else {
    Write-Info "Installing Rust via rustup..."
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupExe = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe
    & $rustupExe -y --default-toolchain stable
    Remove-Item $rustupExe -Force
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}
Write-Info "Rust version: $(rustc --version)"

# ── 3. C++ Build Tools ────────────────────────────────────────
# Tauri 2 on Windows requires the Visual Studio C++ build tools
# (msvc toolchain for compiling native deps like openssl-sys)

$hasVcBuildTools = $false
try {
    # Check for VS 2019 or 2022 with C++ workload
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vsWhere) {
        $vcPackages = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property displayName
        if ($vcPackages) { $hasVcBuildTools = $true }
    }
    # Also check if cl.exe is on PATH
    if (Get-Command cl -ErrorAction SilentlyContinue) { $hasVcBuildTools = $true }
} catch {
    # Ignore errors, fall through to install
}

if (-not $hasVcBuildTools) {
    Write-Info "Installing Visual Studio Build Tools (C++)..."
    if (-not (Test-Admin)) {
        Write-Error "Admin rights required for VS Build Tools. Re-run as Administrator."
    }
    $vsExe = "$env:TEMP\vs_BuildTools.exe"
    $vsUrl = "https://aka.ms/vs/17/release/vs_BuildTools.exe"
    Invoke-WebRequest -Uri $vsUrl -OutFile $vsExe
    $vsArgs = @(
        "--quiet", "--wait",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--add", "Microsoft.VisualStudio.Component.Windows10SDK"
    )
    Start-Process -FilePath $vsExe -ArgumentList $vsArgs -Wait -NoNewWindow
    Remove-Item $vsExe -Force
    Write-Info "Visual Studio Build Tools installed."
} else {
    Write-Info "C++ Build Tools already present."
}

# ── 4. WebView2 Runtime ──────────────────────────────────────
# Windows 10 (21H1+) and Windows 11 ship with WebView2.
# Check registry to verify.

$regPaths = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)
$hasWebView2 = $false
foreach ($path in $regPaths) {
    if (Test-Path $path) { $hasWebView2 = $true; break }
}

if (-not $hasWebView2) {
    Write-Info "Installing WebView2 Runtime..."
    $wvUrl = "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
    $wvExe = "$env:TEMP\MicrosoftEdgeWebview2Setup.exe"
    Invoke-WebRequest -Uri $wvUrl -OutFile $wvExe
    Start-Process -FilePath $wvExe -ArgumentList "/silent", "/install" -Wait -NoNewWindow
    Remove-Item $wvExe -Force
    Write-Info "WebView2 Runtime installed."
} else {
    Write-Info "WebView2 Runtime already present."
}

# ── 5. Node.js check ─────────────────────────────────────────
if (Get-Command node -ErrorAction SilentlyContinue) {
    Write-Info "Node.js is already installed ($(node --version))."
} else {
    Write-Warn "Node.js not found. Install it from:"
    Write-Warn "  https://nodejs.org/"
    Write-Warn "Or via winget: winget install OpenJS.NodeJS.LTS"
    Write-Error "Node.js is required. Please install it and re-run this script."
}

# ── 6. Tauri CLI ──────────────────────────────────────────────
if (cargo tauri --version 2>$null) {
    Write-Info "Tauri CLI already installed ($(cargo tauri --version))."
} else {
    Write-Info "Installing Tauri CLI..."
    cargo install tauri-cli --version "^2" --locked
    Write-Info "Tauri CLI installed."
}

# ── 7. Frontend dependencies ─────────────────────────────────
Write-Info "Installing frontend dependencies (npm install)..."
& npm install
Write-Info "Frontend dependencies installed."

# ── 8. Git hooks ──────────────────────────────────────────────
if (Test-Path "hooks") {
    Write-Info "Configuring git hooks..."
    git config core.hooksPath hooks
    Write-Info "Git hooks configured."
} else {
    Write-Warn "No hooks/ directory found, skipping git hooks setup."
}

# ── 9. Verify ────────────────────────────────────────────────
Write-Host ""
Write-Info "=== Environment Summary ==="
Write-Info "  Rust:    $(rustc --version)"
Write-Info "  Cargo:   $(cargo --version)"
Write-Info "  Node:    $(node --version)"
Write-Info "  npm:     $(npm --version)"
Write-Info "  Tauri:   $(cargo tauri --version 2>$null)"
Write-Host ""
Write-Info "Dev environment is ready! Run 'make dev' to start."
