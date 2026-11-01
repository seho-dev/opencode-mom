# Shared Layout and Presentation Contracts

## Theme values

- **Token reuse**: Reuse `var(--surface-*)`, `--text-*`, `--border-*`, `--accent-*`, `--status-*`, font, radius, focus, and control-height tokens over equivalent literals; define new shared theme values in `tokens.css`.
- **Theme overrides**: Keep dark defaults in `:root` and light color overrides in the following `[data-theme="light"]` block; extend both when adding a color-bearing token.

## Forms and tables

`layout.css` keeps shell/workspace viewport-height and makes `.content` the scrolling region.
`forms.css` uses explicit `.form-page` frames to keep headings visible; only `.form-body` scrolls above
in-flow `.form-actions`. Shared `.field-hint` and `.form-hint` style ordinary help. `table.css` keeps
rightmost action cells sticky with opaque panel backgrounds.

- **Pinned forms**: `.content > .form-page` fills the workspace; `.form-panel` flexes from heading to bottom edge. Direct `.form-body` alone scrolls; `.form-actions` stays in flow, never viewport-fixed; nested `.form-grid` only arranges fields. Keep destination notices, help, and validation inside the body to retain field space on short viewports; dialog editors share this body/actions boundary in a height-constrained flex container.
- **Help hierarchy**: Use `.field-hint` for field help and `.form-hint` for standalone introductions: 11 px, 18 px line height, `--text-secondary`. Keep copy short/action-oriented; keep warning/error banners, credential notices, read-only state, and overwrite risks separate, never muted as ordinary help.
- **Table actions**: Retain `border-collapse: separate`, zero border spacing, and sticky `th.th-actions`/`td.row-actions`; action buttons are spaced with sibling margins inside a plain table cell.

## Mobile, motion, and scrollbars

`responsive.css` adjusts navigation/forms/tables at 760 px and reduces animation/transition duration.

- **Responsive overrides**: Keep shared mobile changes in the 760 px media query. Preserve the `.mobile-menu` display overrides with `!important`, which counter the Button primitive's `inline-flex` utility.
- **Scrollbar coverage**: Update standard `scrollbar-*` and matching WebKit track/thumb/state selectors together for new shared scrolling containers; drawer and textarea tracks use `--scrollbar-track-raised`.
