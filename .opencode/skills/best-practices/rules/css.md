# CSS & Styling Rules

## Core Principles

- **Scoped Component Styles**: Styles must remain strictly scoped to their owning component or feature domain. Never leak selectors into the global namespace.
- **Design Tokens over Magic Literals**: Use CSS custom properties or theme tokens for colors, spacing, borders, and typography.
- **Minimal Inline Styles**: Restrict inline `style` attributes to purely dynamic runtime values (e.g. calculated dimensions, translation coordinates, dynamic colors). Static styling belongs in classes.
- **Namespace Third-Party Overrides**: When overriding third-party component library classes, always prefix the selector with a local feature namespace wrapper.

## MUST

| Rule | Description |
| ---- | ----------- |
| Scoped overrides | Third-party UI overrides must be nested under a unique feature or page class container |
| No magic color/spacing literals | Use CSS custom properties (`var(--color-primary)`) or project design tokens |
| Dynamic inline styles only | Inline `style` is reserved for runtime calculations; static CSS properties must use classes |
| No deep relative style imports | Use path aliases (e.g. `@/styles/...`) or localized component style imports |
| Uniform scrollbar theming | Every scrollable container (`pre`, `textarea`, `.form-body`, `.table-wrap`, drawer) must share the centralized scrollbar theme utility |
| Motion reduction compliance | Continuous animations (spinners, pulses) must respect `prefers-reduced-motion` or provide `motion-reduce:animate-none` fallbacks |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Consistent class naming | Use consistent naming (e.g. BEM, CSS Modules, or project utility classes) |
| Layout vs Presentation | Keep layout properties (margin, grid placement) on the parent; component root handles internal padding |

---

## Examples

See [../examples/css.md](../examples/css.md) for concrete patterns and code examples.
