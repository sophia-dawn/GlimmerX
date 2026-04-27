<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-12 -->

# hooks

## Purpose

Git hooks directory for enforcing code quality checks before commits. Configured via `git config core.hooksPath hooks`.

## Key Files

| File | Description |
|------|-------------|
| `pre-commit` | Runs `make check` (tsc, eslint, prettier, cargo fmt, clippy) on staged changes before allowing commit |

## For AI Agents

### Working In This Directory

- Do not modify the hook unless changing the check pipeline
- Hook uses `set -e` — any check failure aborts the commit
- Hook is smart: only runs frontend checks if frontend files changed, backend checks if backend files changed

### Testing Requirements

- Test by staging changes and running `git commit` — hook should run automatically

<!-- MANUAL: Any manually added notes below this line are preserved on regeneration -->
