# Model Selection and Usage Calculation Contracts

## Shared vocabulary and helpers

`constants.ts` defines readonly value tuples, derived unions, `MessageKey` labels, options, and OMO
categories. `index.ts` composes classes, maps the shared agent catalog, validates token strings,
builds/splits/joins model choices, and formats/parses JSON objects. `dashboard.ts` projects runtime
bindings, aggregates timezone-aware daily usage, and builds period summaries.

- **Domain values**: Extend the tuples in `constants.ts` and derive unions/options from them; group values here are `native`, `slim`, and `omo`.
- **Label keys**: Keep `STORAGE_LABELS` and `GROUP_TYPE_LABELS` typed as `MessageKey`; components translate option labels with `getI18n().t()`.

## Model choices and runtime mappings

- **Model choices**: Preserve catalog priority over provider entries, ref sorting, the leading empty option, and unavailable current selections in `buildModelOptions`; `buildModelVariants` also prefers the catalog.
- **Runtime bindings**: `groupBindings` includes only the group's active runtime mappings and removes bindings with blank names or model refs; OMO includes categories.

## Token counts and calendar periods

- **Token inputs**: `isValidTokenCount` accepts trimmed digit strings representing safe integers, not unit suffixes or decimals; dashboard aggregation rejects invalid counts and overflow with `RangeError`.
- **Calendar semantics**: Bucket timestamps in the supplied timezone, then use ISO dates and UTC date arithmetic for periods; weeks start Monday, cells are Sunday-aligned, and future/padding usage stays null rather than zero.

## Regression coverage

`__tests__/dashboard.test.ts` covers dates, periods, counts, and runtime bindings;
`__tests__/model-limit.test.mjs` covers raw token-count validation;
`__tests__/model-options.test.mjs` covers catalog priority and unavailable selections.

- **Existing coverage**: Extend the matching helper test; the two `.test.mjs` files load `index.ts` through Vite's `ssrLoadModule` and close the server in `finally`.
