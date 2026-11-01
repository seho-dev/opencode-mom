# Review Checklist

## Scope and Strategy

- Is the chosen test layer (component, logic unit, util, integration, Rust backend) appropriate for the contract?
- Does the test protect a specific business outcome rather than internal implementation details?
- If the test is removed, is the contract verified elsewhere or does it create a regression blindspot?

## Assertion Rigor

- Are assertions behavior-centric and outcome-focused?
- Are UI queries using accessible roles and labels (`getByRole`, `getByLabelText`) rather than DOM selectors?
- Does the test avoid asserting purely cosmetic CSS classes, styles, or element hierarchies?
- Does the test assert real contract text, status states, and standardized error codes where relevant?

## Mocking and IPC Boundaries

- Are only external boundaries and host bridges mocked?
- Does `CommandAdapter` mocking accurately reflect `AppError` (`not_found`, `references_blocked`, `validation_failed`, `configuration_failed`, `busy`) and frontend `CommandError` codes (including `conflict` and `ipc_error`)?
- Does wire testing instantiate `createTauriAdapter(stubInvoke)` to assert exact command names and argument serialization?
- Does store testing instantiate `createConfigStore(stubAdapter, false, false)` without assuming automatic startup reads?
- Does the test avoid inventing fake HTTP network layers for Tauri IPC commands?
- Is `Harness` used with valid props (`wrapperProps: { config }`) only when context is required, and omitted for isolated UI controls?
- Are test doubles and fixtures clean, minimal, and free of over-mocking?

## Asynchronous Determinism and Cleanup

- Are async transitions coordinated with `waitFor`, deferred promises, or event channels rather than sleep calls?
- Are polling loops and debounce intervals deterministically advanced and cleaned up using controlled timers?
- Are test states, stores, timers, event listeners, and fetch mocks explicitly reset and closed between tests (e.g., `finally { await vite.close(); }`)?

## Safety and Confidentiality

- Are live system directories strictly untouched (`~/.config/opencode-mom`, `~/.config/opencode`, `~/.omo`)?
- Are tests free of privileged or hardware-modifying commands (`sudo`, `pmset`, `powercfg`)?
- Are real credentials, private user files, and real secrets strictly excluded from test fixtures and logs; are synthetic sentinel tokens used where wire authentication or secret redaction testing is required?
- Does the test avoid claiming that native power, lid sleep, or notch geometry behavior is verified by jsdom or mocks?

## Toolchain and Commands

- Do frontend tests target the proper runner (`npm run test:components` for Vitest, `npm run test:unit` for Node)?
- Are Rust commands locked and aligned with toolchain requirements (main crate MSRV 1.90; helper MSRV 1.77.2 with `--offline --locked`)?
- Does verification run targeted checks for focused changes rather than full workspace suites on documentation edits?

## Source References

- [Test utilities](../../../../../src/test-utils/AGENTS.md) - Context harness conventions and isolation guidelines.
- [Config contracts](../../../../../src/config/docs/command-contracts.md) - Command boundary and error normalization rules.
- [Backend contracts](../../../../../src-tauri/src/docs/config-contracts.md) - Storage and atomic replacement contracts.
