<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-12 -->

# lib

## Purpose

Shared utility functions used across the frontend codebase.

## Key Files

| File | Description |
|------|-------------|
| `utils.ts` | `cn()` utility — merges Tailwind classes using `clsx` + `tailwind-merge` for shadcan/ui components |

## For AI Agents

### Working In This Directory

- Keep utilities small and focused — this is not for complex business logic
- `cn()` is the standard pattern for conditional class merging in shadcan components
- Add new utilities only when used by 2+ components

<!-- MANUAL: Any manually added notes below this line are preserved on regeneration -->
