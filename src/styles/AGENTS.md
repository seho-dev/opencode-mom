# Shared Styles

## Overview

Defines application-wide design tokens and semantic CSS for shell layout, page headings, tables, forms, feedback, search, and roster controls. `src/app.css` composes these sheets in Tailwind's base layer so utilities can override shared defaults; component-local styles remain outside this directory.

## Directory Structure

```text
styles/           # Shared sheets imported by src/app.css
├── tokens.css    # Font faces, dark defaults, and light-theme custom properties
├── base.css      # Element defaults, focus treatment, panel, and muted helpers
├── layout.css    # Shell grid, sidebar, navigation, header, and content scrolling
├── head.css      # Page headings, eyebrow labels, and descriptive introductions
├── table.css     # Scrollable tables, sticky action cells, and pagination
├── feedback.css  # Terminal blocks and error/success state banners
├── search.css    # Search toolbar and flexible input sizing
├── forms.css     # Form fields/grids, pinned actions, variants, and permission rows
├── roster.css    # Architecture selection cards and roster rows
├── scrollbar.css # Standard and WebKit scrollbar styling for explicit containers
└── responsive.css # Mobile overrides, drawer layout, and reduced-motion handling
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Theme contract | `tokens.css` | Loads Space Grotesk/JetBrains Mono and defines shared semantic CSS custom properties. |
| Global defaults | `base.css` | Applies app surface/text/fonts, form font inheritance, and visible focus styling. |
| Shell geometry | `layout.css` | Keeps shell/workspace viewport-height and makes `.content` the scrolling region. |
| Table behavior | `table.css` | Uses separate borders and sticky rightmost action cells with opaque panel backgrounds. |
| Form behavior | `forms.css` | Direct-child form selectors scroll `.form-grid` while keeping `.form-actions` pinned. |
| Adaptive overrides | `responsive.css` | Adjusts navigation/forms/tables at 760 px and reduces animation/transition duration. |

## Conventions

- **Composition**: Import shared sheets through `src/app.css` with `layer(base)`, after `tailwindcss` and `tw-animate-css`; keep `tokens.css` first and `responsive.css` last.
- **Token reuse**: Use existing `var(--surface-*)`, `--text-*`, `--border-*`, `--accent-*`, `--status-*`, font, radius, focus, and control-height tokens instead of equivalent ad-hoc literals. Define new shared theme values in `tokens.css`.
- **Theme overrides**: Keep dark defaults in `:root` and light color overrides in the following `[data-theme="light"]` block; extend both when adding a color-bearing token.
- **File ownership**: Extend the sheet matching the semantic selector's responsibility; keep page heading, search, feedback, form, table, and roster rules out of shell `layout.css`.
- **Scroll boundaries**: Preserve `min-width: 0` on the workspace/content and `min-height: 0` on flexing scroll regions; shell/workspace hide overflow while `.content` scrolls vertically.
- **Pinned forms**: The pinned footer depends on `.content > .form-panel > .form-grid` and sibling `.form-actions`; preserve this direct-child structure when relying on these rules.
- **Table actions**: Retain `border-collapse: separate`, zero border spacing, and sticky `th.th-actions`/`td.row-actions`; action buttons are spaced with sibling margins inside a plain table cell.
- **Responsive overrides**: Keep shared mobile changes in the 760 px media query. Preserve the `.mobile-menu` display overrides with `!important`, which counter the Button primitive's `inline-flex` utility.
- **Scrollbar coverage**: Update standard `scrollbar-*` rules and matching WebKit track/thumb/state selectors together when adding a shared scrolling container; drawer and textarea tracks use `--scrollbar-track-raised`.
- **State selectors**: Navigation styling follows `[aria-current="page"]`; banners use `.error`/`.success`, while architecture cards and roster rows use `.active`. Preserve these selector contracts with their consumers.

## Anti-patterns

- Do not import shared sheets without `layer(base)`; unlayered rules override Tailwind's layered utilities, including link-button colors.
- Do not make `td.row-actions` a flex container; it breaks table column-width calculation and sticky action alignment.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
