<!-- Generated: 2026-04-12 | Updated: 2026-04-21 -->

# GlimmerX

## Purpose

A cross-platform personal double-entry bookkeeping desktop application built with Tauri 2 + React 19 + TypeScript. Data is stored locally in an encrypted SQLCipher SQLite database with user password protection.

## Key Files

| File                           | Description                                                           |
| ------------------------------ | --------------------------------------------------------------------- |
| `package.json`                 | Frontend dependencies and scripts                                     |
| `Cargo.toml` (in `src-tauri/`) | Rust backend dependencies                                             |
| `vite.config.ts`               | Vite build config with Tailwind plugin and @/ alias                   |
| `tsconfig.json`                | TypeScript strict mode config with path aliases                       |
| `tsconfig.node.json`           | TypeScript config for Vite (Node.js context)                          |
| `Makefile`                     | Unified dev commands: `make dev`, `make check`, `make build`, etc.    |
| `eslint.config.js`             | ESLint flat config with react-hooks and react-refresh plugins         |
| `.prettierrc`                  | Prettier formatting rules: 80 columns, double quotes, trailing commas |
| `components.json`              | shadcan/ui configuration (New York style, neutral base color)         |
| `index.html`                   | Frontend entry HTML                                                   |
| `hooks/pre-commit`             | Pre-commit hook that runs `make check` on staged changes              |

## Subdirectories

| Directory    | Purpose                                                               |
| ------------ | --------------------------------------------------------------------- |
| `src/`       | React frontend source code (see `src/AGENTS.md`)                      |
| `src-tauri/` | Rust backend source code and Tauri config (see `src-tauri/AGENTS.md`) |
| `public/`    | Static assets served as-is (see `public/AGENTS.md`)                   |
| `hooks/`     | Git hooks for code quality enforcement (see `hooks/AGENTS.md`)        |

## For AI Agents

### Working In This Directory

- Run `make check` after modifying any config files to verify no lint/type errors
- Run `make setup` to install all dependencies and configure hooks
- Use `make dev` to start the Tauri development server
- Use `make bundle` for production builds targeting all platforms

### Testing Requirements

- `make check` — runs tsc + eslint + prettier + cargo fmt check + clippy
- `cargo test` in `src-tauri/` — runs Rust unit tests
- Coverage requirement: Rust unit tests >= 95%

### Common Patterns

- TypeScript strict mode with `noUncheckedIndexedAccess: true`
- Path alias `@/*` maps to `./src/*`
- Amounts stored as integers (cents) in the database, never floats
- Every transaction's postings must sum to zero (double-entry rule)
- Pre-commit hook blocks commits if checks fail
- **Time handling: use local timezone (chrono::Local)** — Desktop app runs on single machine with consistent timezone. Timestamps stored as RFC3339 with local offset. Business dates (transactions.date) stored as YYYY-MM-DD strings. Use `utils::time` module functions for consistency.

### Hard Rules

- **Database changes require user confirmation.** Any modification to database schema, table structure, column additions/deletions, or migration scripts MUST be confirmed by the user before implementation.
- **Error handling: backend returns friendly messages, frontend displays as-is.** All user-facing error messages must be written in the Rust backend (using `thiserror` with human-readable `Display` implementations or explicit message strings). The frontend must not transform, translate, or reconstruct error messages — receive the error string from the backend invocation and render it directly to the user.
- **Frontend must not compute amounts or use caches.** All monetary calculations must be done on the Rust backend. The frontend is a display layer only — receive computed values via API and render them directly. Do not use any form of client-side caching (React Query staleTime, local state persistence, useMemo for derived financial data, etc.).
- **SQL injection prevention: always use parameterized queries.** NEVER concatenate, interpolate, or format user input into SQL strings. All database queries using rusqlite MUST use parameterized statements (`?` placeholders with bound parameters) for any dynamic values. This includes: WHERE clauses, INSERT values, UPDATE SET clauses, and any other user-provided or dynamic data.
- **Date string construction: use local timezone helpers.** NEVER use `toISOString().split("T")[0]` in frontend — it converts local midnight to UTC and may shift the date. Use `formatLocalDate()`, `todayLocalDate()`, or `currentMonthBoundsLocal()` from `utils/date.ts`.

## Dependencies

### External

- **React 19** — UI framework
- **TypeScript 5.8** — Type safety
- **Vite 7** — Build tool
- **Tailwind CSS 4** — Utility-first CSS (CSS-first config, no tailwind.config.ts)
- **shadcan/ui** — Source-level UI components (New York style)
- **ESLint 9** — Linting (flat config)
- **Prettier 3** — Code formatting
- **Tauri 2** — Desktop shell
- **rusqlite 0.39** — SQLite with bundled-sqlcipher-vendored-openssl
- **serde/serde_json** — Rust serialization
- **chrono** — Date/time handling (use Local timezone)
- **uuid v4** — UUID generation
- **thiserror 2** — Error handling
- **clsx + tailwind-merge** — Conditional class names
- **lucide-react** — Icons
- **date-fns** — Frontend date formatting
- **Recharts** — Dashboard charts
