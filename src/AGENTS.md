<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-21 -->

# src

## Purpose

React frontend source code for the GlimmerX application. Contains all UI components, state management, hooks, utilities, and styles.

## Key Files

| File | Description |
|------|-------------|
| `main.tsx` | React entry point, renders App into #root with StrictMode and imports global styles |
| `App.tsx` | Root component with router setup and authentication flow |
| `vite-env.d.ts` | TypeScript declarations for Vite-specific globals (`import.meta.env`) |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `components/` | UI components organized by feature: dashboard, accounts, transactions, categories, ui (shadcan) |
| `pages/` | Page-level components: DashboardPage, AccountsPage, TransactionsPage, etc. |
| `hooks/` | React Query hooks: useDashboardSummary, useMonthlyChart, useCategoryBreakdown, useTopExpenses |
| `utils/` | Utilities: api.ts (Tauri invoke wrapper), date.ts (local timezone helpers), format.ts (amount formatting) |
| `types/` | TypeScript interfaces matching backend structs: dashboard.ts, account.ts, transaction.ts, category.ts |
| `constants/` | App constants: chart colors, dashboard limits, date formats |
| `assets/` | Image/SVG assets imported via Vite (see `assets/AGENTS.md`) |
| `styles/` | Global CSS including Tailwind v4 directives and shadcan theme variables (see `styles/AGENTS.md`) |

## For AI Agents

### Working In This Directory

- All files use TypeScript (.ts/.tsx)
- Path alias `@/*` resolves to `./src/*` — prefer `@/components/ui/Button` over relative paths
- No unused locals or parameters enforced (`noUnusedLocals`, `noUnusedParameters`)
- `noUncheckedIndexedAccess: true` — array/object access returns `T | undefined`
- Changes here are validated by pre-commit hook (tsc + eslint + prettier)

### Testing Requirements

- Unit tests use Vitest + Testing Library
- Component tests in `.test.tsx` files co-located with components

### Common Patterns

- Functional components with explicit prop interfaces
- Tailwind CSS utility classes via shadcan/ui components
- Amount display uses `formatAmount()` from `utils/format.ts` — always format cents to yuan/dollars
- **Date handling: use `utils/date.ts` helpers** — NEVER use `toISOString().split("T")[0]` (UTC shift bug). Use `formatLocalDate()`, `todayLocalDate()`, `currentMonthBoundsLocal()`.

### Dashboard Module

The dashboard (概览) module displays aggregated financial data on the home page:

| Component | Purpose |
|-----------|---------|
| `DashboardSummaryCards` | 4-card grid: monthly/yearly income/expense |
| `FinancialHealthCards` | 3-card grid: assets, debt, net worth |
| `MonthlyIncomeExpenseChart` | Daily income/expense line chart (Recharts) |
| `CategoryBreakdownChart` | Pie chart of expense by category |
| `TopExpensesList` | Top 10 expense transactions with links |
| `RecentTransactionsCard` | Recent 8 transactions with links |
| `AccountBalanceList` | Collapsible list of account balances |

All hooks use `staleTime: 0, gcTime: 0, refetchOnMount: "always"` per project rule (no caching).

## Dependencies

### Internal

- `src-tauri/` — All data comes through Tauri `invoke()` calls via `utils/api.ts`
- `src/lib/utils.ts` — `cn()` utility for conditional class names
- `src/utils/date.ts` — Local timezone date helpers
- `src/utils/format.ts` — Amount formatting utilities

### External

- `react` / `react-dom` — React 19 framework
- `@tauri-apps/api` — Tauri frontend API for `invoke`, filesystem, etc.
- `@tanstack/react-query` — Data fetching (no caching)
- `clsx` + `tailwind-merge` — Class name utilities
- `lucide-react` — Icon components
- `date-fns` — Date formatting (local timezone aware)
- `recharts` — Dashboard charts
- `react-i18next` — Internationalization
