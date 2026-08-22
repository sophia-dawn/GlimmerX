#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# setup-macos.sh — Initialize GlimmerX dev environment on macOS
#
# Installs:
#   - Rust toolchain (via rustup)
#   - macOS system dependencies via Homebrew
#   - Build dependencies for rusqlite (bundled-sqlcipher-vendored-openssl)
#   - Frontend and Rust tooling
# ──────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[setup]${NC} $*"; }
warn()  { echo -e "${YELLOW}[warn]${NC}  $*"; }
error() { echo -e "${RED}[error]${NC} $*"; exit 1; }

# ── 1. Check for Homebrew ────────────────────────────────────
if ! command -v brew &>/dev/null; then
    warn "Homebrew not found. Installing..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    
    # Add Homebrew to PATH for Apple Silicon Macs
    if [[ $(uname -m) == "arm64" ]]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    else
        eval "$(/usr/local/bin/brew shellenv)"
    fi
fi

info "Homebrew version: $(brew --version | head -1)"

# ── 2. Xcode Command Line Tools ──────────────────────────────
if ! xcode-select -p &>/dev/null; then
    info "Installing Xcode Command Line Tools..."
    xcode-select --install
    echo "Press any key after Xcode Command Line Tools installation completes..."
    read -n 1 -s
fi

info "Xcode Command Line Tools: $(xcode-select -p)"

# ── 3. System dependencies via Homebrew ──────────────────────
info "Installing system dependencies..."

# Handle qt@5 / qtbase conflict (common on macOS)
if brew list qt@5 &>/dev/null && brew list qtbase &>/dev/null; then
    warn "Detected qt@5 and qtbase conflict. Unlinking qtbase..."
    brew unlink qtbase --force 2>/dev/null || true
fi

brew install pkg-config openssl@3 sqlite

# ── 4. Rust toolchain ────────────────────────────────────────
if command -v rustup &>/dev/null; then
    info "Rust is already installed ($(rustc --version)). Updating..."
    rustup update stable
else
    info "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi

info "Rust version: $(rustc --version)"
info "Cargo version: $(cargo --version)"

# Ensure the cargo bin dir is on PATH for the current session
export PATH="$HOME/.cargo/bin:$PATH"

# ── 5. Tauri CLI ──────────────────────────────────────────────
if cargo tauri --version &>/dev/null; then
    info "Tauri CLI already installed ($(cargo tauri --version))."
else
    info "Installing Tauri CLI..."
    cargo install tauri-cli --version "^2" --locked
    info "Tauri CLI installed."
fi

# ── 6. Node.js check ──────────────────────────────────────────
if command -v node &>/dev/null; then
    info "Node.js is already installed ($(node --version))."
else
    warn "Node.js not found. Installing via Homebrew..."
    brew install node@22
    brew link --force node@22
fi

if ! command -v npm &>/dev/null; then
    error "npm not found. Please install npm alongside Node.js."
fi

# ── 7. Frontend dependencies ─────────────────────────────────
info "Installing frontend dependencies (npm install)..."
npm install
info "Frontend dependencies installed."

# ── 8. Git hooks ──────────────────────────────────────────────
if [[ -d hooks ]]; then
    info "Configuring git hooks..."
    git config core.hooksPath hooks
    info "Git hooks configured."
else
    warn "No hooks/ directory found, skipping git hooks setup."
fi

# ── 9. Verify ────────────────────────────────────────────────
echo ""
info "=== Environment Summary ==="
info "  Rust:    $(rustc --version)"
info "  Cargo:   $(cargo --version)"
info "  Node:    $(node --version)"
info "  npm:     $(npm --version)"
info "  Tauri:   $(cargo tauri --version 2>/dev/null || cargo tauri --version)"
info ""
info "Dev environment is ready! Run 'make dev' to start."
