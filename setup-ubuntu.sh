#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
# setup-ubuntu.sh — Initialize GlimmerX dev environment on Ubuntu
#
# Installs:
#   - Rust toolchain (via rustup)
#   - Ubuntu system libraries required by Tauri 2
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

# ── 1. Detect Ubuntu ─────────────────────────────────────────
if ! command -v lsb_release &>/dev/null || [[ "$(lsb_release -is)" != "Ubuntu" ]]; then
    warn "This script is designed for Ubuntu. You may need to adjust package names."
    read -rp "Continue anyway? [y/N] " ans
    [[ "$ans" =~ ^[Yy]$ ]] || error "Aborted."
fi

# ── 2. System dependencies ────────────────────────────────────
# Tauri 2 requires: WebKit2GTK, AppIndicator, and various dev libs.
# rusqlite with bundled-sqlcipher-vendored-openssl needs:
#   build-essential, pkg-config, libssl-dev (for openssl-sys)
#   libsqlite3-dev is not strictly needed with bundled but useful for tooling.

info "Updating package lists..."
sudo apt-get update -qq

info "Installing system build dependencies..."
sudo apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    wget \
    file \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    libxdo-dev \
    fonts-liberation \
    libasound2-dev \
    libfontconfig1-dev \
    libfreetype-dev \
    libgtk-3-dev

info "System dependencies installed."

# ── 3. Rust toolchain ────────────────────────────────────────
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

# ── 4. Tauri CLI ──────────────────────────────────────────────
if cargo tauri --version &>/dev/null; then
    info "Tauri CLI already installed ($(cargo tauri --version))."
else
    info "Installing Tauri CLI..."
    cargo install tauri-cli --version "^2" --locked
    info "Tauri CLI installed."
fi

# ── 5. Node.js check ──────────────────────────────────────────
if command -v node &>/dev/null; then
    info "Node.js is already installed ($(node --version))."
else
    warn "Node.js not found. Install it manually:"
    warn "  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -"
    warn "  sudo apt-get install -y nodejs"
    error "Node.js is required. Please install it and re-run this script."
fi

if ! command -v npm &>/dev/null; then
    error "npm not found. Please install npm alongside Node.js."
fi

# ── 6. Frontend dependencies ─────────────────────────────────
info "Installing frontend dependencies (npm install)..."
npm install
info "Frontend dependencies installed."

# ── 7. Git hooks ──────────────────────────────────────────────
if [[ -d hooks ]]; then
    info "Configuring git hooks..."
    git config core.hooksPath hooks
    info "Git hooks configured."
else
    warn "No hooks/ directory found, skipping git hooks setup."
fi

# ── 8. Verify ────────────────────────────────────────────────
echo ""
info "=== Environment Summary ==="
info "  Rust:    $(rustc --version)"
info "  Cargo:   $(cargo --version)"
info "  Node:    $(node --version)"
info "  npm:     $(npm --version)"
info "  Tauri:   $(cargo tauri --version 2>/dev/null || cargo tauri --version)"
info ""
info "Dev environment is ready! Run 'make dev' to start."
