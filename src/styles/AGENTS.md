# Shared Styles

## Overview

Defines application-wide design tokens and semantic CSS for shell layout, headings, tables, forms,
feedback, search, and rosters. `src/app.css` composes these sheets in Tailwind's base layer so utilities
can override defaults; component-local styles remain outside this directory.

## Directory Structure

```text
styles/            # Shared sheets imported by src/app.css
├── tokens.css     # Fonts and theme properties
├── base.css       # Elements, focus, panel, muted helpers
├── layout.css     # Shell grid, chrome, content scrolling
├── head.css       # Headings, eyebrows, introductions
├── table.css      # Tables, sticky actions, pagination
├── feedback.css   # Terminal blocks and state banners
├── search.css     # Toolbar and flexible inputs
├── forms.css      # Fields, grids, pinned actions, variants, permission rows
├── roster.css     # Architecture cards and roster rows
├── scrollbar.css  # Standard and WebKit scrolling containers
└── responsive.css # Mobile, drawer, reduced motion
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Theme and defaults | `tokens.css`, `base.css` | Space Grotesk/JetBrains Mono, semantic tokens, app defaults, inherited form fonts, and visible focus. |
| Scroll ownership | `layout.css`, `forms.css` | Viewport-height shell and pinned form headings with in-flow actions. |
| Table geometry | `table.css` | Separate borders and sticky actions on opaque panel backgrounds. |
| Adaptation | `responsive.css` | Shared mobile and reduced-motion overrides. |

## Conventions

- **Composition**: Import shared sheets through `src/app.css` with `layer(base)`, after `tailwindcss` and
  `tw-animate-css`; keep `tokens.css` first and `responsive.css` last.
- **Theme contract**: Reuse shared semantic tokens instead of equivalent literals; add shared values
  in `tokens.css` and extend both dark and light definitions for color-bearing tokens.
- **File ownership**: Extend the sheet matching the semantic selector's responsibility; keep page
  heading, search, feedback, form, table, and roster rules out of shell `layout.css`.
- **Scroll boundaries**: Preserve `min-width: 0` on workspace/content and `min-height: 0` on flexing
  scroll regions; shell/workspace hide overflow while `.content` scrolls vertically.
- **Form boundaries**: Keep form headings visible, body content scrollable, and actions in flow rather
  than viewport-fixed; preserve the same body/actions boundary in height-constrained dialog editors.
- **Safety feedback**: Keep ordinary help short and action-oriented; never mute warning/error banners,
  credential notices, read-only state, or overwrite risks into ordinary field help.
- **Selector contracts**: Preserve `[aria-current="page"]` navigation, `.error`/`.success` banners, and
  `.active` architecture cards/roster rows together with their consumers when changing shared styles.
- **Adaptive coverage**: Keep shared mobile overrides together and update standard/WebKit scrollbar
  selectors together; preserve drawer button display overrides against the Button primitive's utilities.

## Anti-patterns

- Do not import shared sheets without `layer(base)`; unlayered rules override Tailwind's layered
  utilities, including link-button colors.
- Do not make `td.row-actions` a flex container; it breaks table column-width calculation and sticky
  action alignment.

## Details

| Section | File |
| --- | --- |
| Layout and presentation contracts | [docs/layout-contracts.md](docs/layout-contracts.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
