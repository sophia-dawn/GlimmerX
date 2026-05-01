.PHONY: help setup dev dev-web lint lint-frontend lint-backend fmt fmt-frontend fmt-backend test test-frontend test-backend check release release-windows release-linux release-mac portable portable-windows portable-linux clean _collect-release

SHELL := /bin/bash

# ── Help ──────────────────────────────────────────────────────
help:
	@echo "GlimmerX - Makefile Commands"
	@echo ""
	@echo "  make setup        Install all dependencies (npm + cargo)"
	@echo "  make dev          Start Tauri dev mode (React HMR + desktop app)"
	@echo "  make dev-web      Start frontend only (browser debugging)"
	@echo "  make lint         Run ESLint + Clippy (all warnings as errors)"
	@echo "  make fmt          Format code (Prettier + cargo fmt)"
	@echo "  make test         Run all tests (Vitest + cargo test)"
	@echo "  make check        Full check (tsc + lint + fmt check + clippy)"
	@echo ""
	@echo "  Release (安装版):"
	@echo "  make release          Build installers for current platform"
	@echo "  make release-windows  Windows: NSIS + MSI + MSIX"
	@echo "  make release-linux    Linux: AppImage + deb + rpm"
	@echo "  make release-mac      macOS: dmg + app"
	@echo "  → Output: release/ (unified directory)"
	@echo ""
	@echo "  Portable (便携版):"
	@echo "  make portable-windows Windows: single exe"
	@echo "  → Output: release/"
	@echo ""
	@echo "  make clean        Remove build artifacts"

# ── Setup ─────────────────────────────────────────────────────
setup:
	@OS=$$(uname -s); \
	if [ "$$OS" = "Linux" ]; then \
		if command -v lsb_release >/dev/null 2>&1 && [ "$$(lsb_release -is)" = "Ubuntu" ]; then \
			echo "→ Detected Ubuntu, running setup-ubuntu.sh..."; \
			bash ./setup-ubuntu.sh; \
		else \
			echo "→ Detected Linux ($$(cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d= -f2 | tr -d '"'))"; \
			echo "  Only Ubuntu is currently supported."; \
			exit 1; \
		fi \
	elif echo "$$OS" | grep -qiE 'MINGW|MSYS|CYGWIN|Windows_NT'; then \
		echo "→ Windows detected, running setup-windows.ps1..."; \
		if command -v pwsh >/dev/null 2>&1; then \
			pwsh -ExecutionPolicy Bypass -File ./setup-windows.ps1; \
		elif command -v powershell >/dev/null 2>&1; then \
			powershell -ExecutionPolicy Bypass -File ./setup-windows.ps1; \
		else \
			echo "  PowerShell not found. Install PowerShell or run setup-windows.ps1 manually."; \
			exit 1; \
		fi \
	elif [ "$$OS" = "Darwin" ]; then \
		echo "→ macOS detected. Setup script not yet available."; \
		exit 1; \
	else \
		echo "→ Unsupported OS: $$OS. Only Ubuntu and Windows are currently supported."; \
		exit 1; \
	fi

# ── Dev ───────────────────────────────────────────────────────
dev:
	npm run tauri dev

dev-web:
	npm run dev

# ── Lint ──────────────────────────────────────────────────────
lint: lint-frontend lint-backend

lint-frontend:
	npx eslint . --max-warnings=0

lint-backend:
	cd src-tauri && cargo clippy -- -D warnings

# ── Format ────────────────────────────────────────────────────
fmt: fmt-frontend fmt-backend

fmt-frontend:
	npx prettier --write "src/**/*.{ts,tsx,css,json}"

fmt-backend:
	cd src-tauri && cargo fmt
fmt-md:
	find . \
	-name "*.md " \
	-type f \
	-not -path "*/node_modules/*" \
	-not -path "*/dist/*" \
	-not -path "*/src-tauri/target/*" \
	-not -path "*/.git/*" -exec node_modules/markdownlint-cli/markdownlint.js --fix  {} \;
# ── Test ──────────────────────────────────────────────────────
test: test-frontend test-backend

test-frontend:
	npx vitest run

test-backend:
	cd src-tauri && cargo test

# ── Check ─────────────────────────────────────────────────────
check:
	@echo "→ TypeScript type check..."
	npx tsc --noEmit
	@echo "→ ESLint check..."
	npx eslint . --max-warnings=0
	@echo "→ Prettier check..."
	npx prettier --check "src/**/*.{ts,tsx,css,json}" --write
	@echo "→ Rust fmt check..."
	cd src-tauri && cargo fmt -- --check
	@echo "→ Rust clippy..."
	cd src-tauri && cargo clippy -- -D warnings
	@echo "✓ All checks passed"

# ── Release (安装版) ─────────────────────────────────────────────

# Helper: collect artifacts to release/ directory
_collect-release:
	@mkdir -p release
	@rm -rf release/*.deb release/*.rpm release/*.AppImage release/*.exe release/*.msi release/*.msix release/*.dmg release/*.app 2>/dev/null || true
	@if [ -d "src-tauri/target/release/bundle/deb" ]; then \
		cp src-tauri/target/release/bundle/deb/*.deb release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/rpm" ]; then \
		cp src-tauri/target/release/bundle/rpm/*.rpm release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/appimage" ]; then \
		cp src-tauri/target/release/bundle/appimage/*.AppImage release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/nsis" ]; then \
		cp src-tauri/target/release/bundle/nsis/*.exe release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/msi" ]; then \
		cp src-tauri/target/release/bundle/msi/*.msi release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/msix" ]; then \
		cp src-tauri/target/release/bundle/msix/*.msix release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/dmg" ]; then \
		cp src-tauri/target/release/bundle/dmg/*.dmg release/ 2>/dev/null || true; \
	fi
	@if [ -d "src-tauri/target/release/bundle/macos" ]; then \
		cp src-tauri/target/release/bundle/macos/*.app release/ 2>/dev/null || true; \
	fi
	@echo "✓ Artifacts collected to: release/"

release:
	@OS=$$(uname -s); \
	if echo "$$OS" | grep -qiE 'MINGW|MSYS|CYGWIN|Windows_NT'; then \
		$(MAKE) release-windows; \
	elif [ "$$OS" = "Linux" ]; then \
		$(MAKE) release-linux; \
	elif [ "$$OS" = "Darwin" ]; then \
		$(MAKE) release-mac; \
	else \
		echo "Unsupported OS: $$OS"; exit 1; \
	fi

release-windows:
	@echo "→ Building Windows installers (NSIS + MSI)"
	@# Use Strawberry Perl for OpenSSL build (Git Bash Perl lacks required modules)
	@export OPENSSL_SRC_PERL="C:/Strawberry/perl/bin/perl.exe"; \
	npm run tauri build
	$(MAKE) _collect-release

release-linux:
	@echo "→ Building Linux packages (AppImage + deb + rpm)"
	npm run tauri build
	$(MAKE) _collect-release

release-mac:
	@echo "→ Building macOS packages (dmg + app)"
	npm run tauri build
	$(MAKE) _collect-release

# ── Portable (便携版) ────────────────────────────────────────────
portable-windows:
	@echo "→ Building Windows portable exe"
	npm run tauri build -- --no-bundle
	@mkdir -p release
	@cp src-tauri/target/release/glimmerx.exe release/Glimmerx-portable.exe
	@echo "✓ Output: release/Glimmerx-portable.exe"


# ── Clean ─────────────────────────────────────────────────────
clean:
	rm -rf dist release
	cd src-tauri && cargo clean
