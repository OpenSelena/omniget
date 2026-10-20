---
name: ui-lint
description: Run targeted rendered UI verification with lintapple (`pnpm lint:ui <target>`). Use whenever modifying Svelte routes (`src/routes/`), UI components (`src/components/`), CSS stylesheets (`src/app.css`, `src/lib/style/`), the landing page (`index.html`), or documentation pages (`docs/`).
---

# Targeted UI Lint (`lintapple`)

Verify rendered layout, typography scale, concentric border radii, contrast, and accessibility on the exact surface you modified before completing any UI or documentation task.

## Workflow

1. Identify the target route or page matching the files you modified using the table below.
2. Run `pnpm lint:ui <target>` (add `--json` to inspect full DOM selectors and suggested fixes).
3. Fix every new finding reported against your changes and re-run `pnpm lint:ui <target>` until it exits `0`.

## File-to-Target Mapping

| Modified Path | Command to Run | Policy Applied |
| :--- | :--- | :--- |
| `src/routes/<feature>/**` | `pnpm lint:ui /<feature>` | `ui-policy.app.json` + baseline |
| `src/routes/+page.svelte` | `pnpm lint:ui /` | `ui-policy.app.json` + durable baseline |
| `src/lib/style/settings.css` | `pnpm lint:ui /settings` | `ui-policy.app.json` + baseline |
| `src/lib/style/queue-kinds.css` | `pnpm lint:ui /downloads` | `ui-policy.app.json` + baseline |
| `src/lib/style/notes.css` | `pnpm lint:ui /study` | `ui-policy.app.json` + baseline |
| `src/routes/+layout.svelte`, `src/components/shell/**`, `src/lib/style/buttons.css`, `src/lib/style/primitives.css` | `pnpm lint:ui --shell` | `ui-policy.app.json` + baseline |
| `src/app.css` | `pnpm lint:ui /settings --all-themes` | `ui-policy.app.json` across all 14 themes |
| `index.html` | `pnpm lint:ui index.html` | `ui-policy.docs.json` (0 findings required) |
| `docs/<section>/index.html` | `pnpm lint:ui docs/<section>/index.html` | `ui-policy.docs.json` (0 findings required) |

## Policies & Baseline Rules

- **Desktop App ([`ui-policy.app.json`](../../../ui-policy.app.json))**:
  - Enforces [src/app.css](../../../src/app.css) typography, control sizing, spacing, and third-party canvas exceptions across default and minimum Tauri window viewports.
  - Compares per-theme rule counts against [`scripts/ui-lint-baseline.json`](../../../scripts/ui-lint-baseline.json). The check exits `0` when no rule count increases on the target route.
  - When adding a new token size to `src/app.css`, update `ui-policy.app.json` in the same commit.
  - When cleaning up pre-existing UI findings on a route, ratchet its baseline down with `pnpm lint:ui:update /<feature>`.
- **Landing Page & Docs ([`ui-policy.docs.json`](../../../ui-policy.docs.json))**:
  - Enforces desktop and mobile viewports across `dark` and `light` themes, including `bannedCopy` rules.
  - Requires strict zero findings (`0` tolerance).
