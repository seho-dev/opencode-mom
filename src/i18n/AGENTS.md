# i18n

## Overview

Typed English and Chinese dictionaries feed a context-shared translator that follows the config locale. This module owns lookup and interpolation, not preference persistence or UI rendering.

## Directory Structure

```text
i18n/                         # Translation data and context API
├── AGENTS.md                 # Directory-specific guidance
├── context.ts                # Symbol-keyed setI18n/getI18n access
├── i18n.svelte.ts             # createI18n lookup and interpolation
├── index.ts                  # Public exports
├── dictionaries/             # Locale message maps
│   ├── en.ts                 # English messages and MessageKey type
│   └── zh.ts                 # Chinese Record<MessageKey, string>
└── __tests__/                # Translation behavior coverage
    └── i18n.test.ts          # Locale changes, fallback, and tray placeholders
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Message schema | `dictionaries/en.ts` | English keys define the shared message type. |
| Chinese messages | `dictionaries/zh.ts` | Typed counterpart for every English key. |
| Translation lookup | `i18n.svelte.ts` | Reactive locale, interpolation, and fallback behavior. |
| Context API | `context.ts` | Parent-installed translator access. |
| Compatibility checks | `__tests__/i18n.test.ts` | Locale changes, key parity, and placeholders. |

## Conventions

- **Dictionary schema**: Define keys in English and mirror them in the typed Chinese map; do not create
  another key union. Follow [schema contracts](docs/translator-contracts.md#schema-and-interpolation).

- **Reactive context**: Install the translator in a parent using the config locale getter; descendants consume
  context. Locale persistence stays outside this module’s [reactive
  context](docs/translator-contracts.md#reactive-context-and-regression-coverage).

- **Parameters**: Keep placeholder names aligned and pass supported values; preserve missing-parameter and
  fallback behavior. Consult [interpolation rules](docs/translator-contracts.md#schema-and-interpolation)
  before changing lookup semantics.

- **Product terminology**: Distinguish app updates, CLI operations, live services, and configuration scopes.
  Use the matching [product-copy contracts](docs/product-copy.md) rather than implying runtime behavior from
  configuration.

- **Security copy**: Keep credential-storage guidance and platform limitations equivalent across locales;
  never expose secrets or claim unverified protection. Preserve [service
  wording](docs/product-copy.md#app-cli-and-service-terminology).

- **UI status**: Keep dashboard, settings-search, and Island labels concise without losing accessible counts
  or unknown states. Preserve their [domain
  terminology](docs/product-copy.md#dashboard-and-island-terminology) and static search keys.

- **Regression coverage**: Keep both dictionaries aligned on keys and placeholder names; retain locale-change,
  interpolation, and unknown-key checks. Use the existing [translator regression
  coverage](docs/translator-contracts.md#reactive-context-and-regression-coverage).

## Sync

Keep this guide and linked docs in sync with changes to this directory.
