# i18n

## Overview

Provides typed English and Chinese messages, parameter interpolation, and a context-shared translator that follows the config store's locale preference. This directory owns message lookup and context access, not preference persistence or UI rendering.

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
| :--- | :--- | :--- |
| Message schema | `dictionaries/en.ts` | Defines `en` as a const object and `MessageKey = keyof typeof en`. |
| Chinese messages | `dictionaries/zh.ts` | Implements every English key through `Record<MessageKey, string>`. |
| Translator | `i18n.svelte.ts` | Exposes `locale` and `t(key, params)` using the supplied locale getter. |
| Context | `context.ts` | Shares an `I18n` instance through `setI18n` and `getI18n`. |

## Conventions

- **Key schema**: Define new keys in `en.ts`; mirror every key in the typed `zh.ts` map rather than maintaining a separate key union.
- **Parameters**: Use matching `{name}` placeholders in both locales and pass string or number values to `t`; with a params object, missing values become empty strings, while omitting params leaves placeholders intact.
- **Locale getter**: Keep `createI18n(() => config.preferences.locale)` in context providers so lookup follows preference changes; this module does not persist the locale.
- **Context access**: Install the translator with `setI18n` in a parent and consume it with `getI18n` in descendants; `context.ts` owns the private Symbol key.
- **Fallback**: Preserve lookup order of selected locale, English, then the key itself.
- **Existing coverage**: Keep interpolation, locale-change, and unknown-key cases in `__tests__/i18n.test.ts`; its `tray.*` loop checks matching placeholder names across locales.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
