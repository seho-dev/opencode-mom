# Translator API and dictionary compatibility

Source-relative paths below are relative to `src/i18n/`, not this docs directory.

## Ownership

Provides typed English and Chinese messages, parameter interpolation, and a context-shared translator that follows the config store's locale preference. This directory owns message lookup and context access, not preference persistence or UI rendering.

## Schema and interpolation

- **Key schema**: Define new keys in `en.ts`; mirror every key in the typed `zh.ts` map rather than maintaining a separate key union.

- **Parameters**: Use matching `{name}` placeholders in both locales and pass string or number values to `t`; with a params object, missing values become empty strings, while omitting params leaves placeholders intact.

- **Fallback**: Preserve lookup order of selected locale, English, then the key itself.

## Reactive context and regression coverage

- **Locale getter**: Keep `createI18n(() => config.preferences.locale)` in context providers so lookup follows preference changes; this module does not persist the locale.

- **Context access**: Install the translator with `setI18n` in a parent and consume it with `getI18n` in descendants; `context.ts` owns the private Symbol key.

- **Existing coverage**: Keep interpolation, locale-change, and unknown-key cases in `__tests__/i18n.test.ts`; its dictionary loop checks matching keys and placeholder names across both locales.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Message schema | `dictionaries/en.ts` | Defines `en` as a const object and `MessageKey = keyof typeof en`. |
| Chinese messages | `dictionaries/zh.ts` | Implements every English key through `Record<MessageKey, string>`. |
| Translator | `i18n.svelte.ts` | Exposes `locale` and `t(key, params)` using the supplied locale getter. |
| Context | `context.ts` | Shares an `I18n` instance through `setI18n` and `getI18n`. |
