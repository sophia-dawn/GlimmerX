<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-04-12 | Updated: 2026-04-12 -->

# styles

## Purpose

Global CSS including Tailwind CSS v4 directives and shadcan/ui theme variables.

## Key Files

| File | Description |
|------|-------------|
| `globals.css` | Tailwind `@import "tailwindcss"`, `@theme` block with shadcan color variables (neutral base), and base layer styles |

## For AI Agents

### Working In This Directory

- Tailwind v4 uses CSS-first config — no `tailwind.config.ts` needed
- Theme variables are defined via CSS custom properties in `@theme` block
- Color system uses `oklch()` color space for better perceptual uniformity
- shadcan color tokens: `--color-background`, `--color-foreground`, `--color-primary`, etc.
- Import this file in `main.tsx` — do not import in individual components

<!-- MANUAL: Any manually added notes below this line are preserved on regeneration -->
