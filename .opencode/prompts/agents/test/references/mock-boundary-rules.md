# Mock Boundary Rules

## Core Rule

Mock external volatility and host boundaries; never mock the internal business logic or state machines under test.

## Usually Safe to Mock

- **Tauri IPC boundary**: Mock methods on `CommandAdapter` (`src/config/adapter.ts`) or stub `invoke` calls.
- **Network requests**: External HTTP calls (e.g. remote skill fetching, update checks, external API queries).
- **Platform integrations**: Clipboard, system notification services, or native file picker dialogs.
- **Asynchronous timing**: Timers and intervals where time progression is strictly controlled.
- **Environment variables**: System environment lookups and platform identifiers; serialize tests modifying process-wide environment variables to prevent test concurrency races. Restore changed environment variables and mock state after each test.

## Usually Not Worth Mocking

- **Pure utilities**: Mappers, validators, constants, and schema parsers.
- **Reactive state and runes**: Svelte 5 `$state` and `$derived` variables; let them execute naturally.
- **Bits UI controls**: Allow Bits UI components to render and fire real events unless missing browser APIs in jsdom require focused polyfills.
- **Local collaborators**: Co-located domain helpers that form the unit's public contract.

## AppError and CommandError Contracts

When mocking failure responses from the backend, use real error contracts:

| Error Code | Emitting Layer | Typical Trigger |
| :--- | :--- | :--- |
| `not_found` | Rust `AppError` & Frontend | Resource does not exist (missing agent, deleted command) |
| `references_blocked` | Rust `AppError` & Frontend | Deleting a provider, model, or agent that still has active references, or deleting a built-in agent |
| `validation_failed` | Rust `AppError` & Frontend | Input failed validation (malformed schema, invalid identifier, missing snapshot fields) |
| `configuration_failed` | Rust `AppError` & Frontend | Target write failed, or snapshot content/path changed; `AppError::io` carries operation & path context |
| `busy` | Rust `AppError` & Frontend | Operation blocked because another guarded task is active |
| `conflict` | Frontend `CommandError` | Service-target generation guard in `ConfigStore` |
| `ipc_error` | Frontend `CommandError` | Unhandled IPC bridge exceptions or transport failures |

Mock failures should reject with plain `CommandError` objects reflecting these exact error codes, or reject with a generic Error whose normalization to `ipc_error` is being tested. Do not mock an ad-hoc HTTP status code or invent generic error objects.

## Shared vs Local Test Utilities

Follow strict guidelines for extracting test helpers:

- **Shared helpers (`src/test-utils/`)**:
  - Reserved for cross-domain infrastructure (e.g., `harness.svelte`, `setup.ts`).
  - Must remain domain-agnostic.
  - Must not embed business data fixtures for specific features.
- **Domain-local helpers (`src/<domain>/__tests__/`)**:
  - Keep domain fixtures, mock responses, and scenario factories in the domain's test directory.
  - Do not create overarching `mockEverythingForFeature()` functions that obscure test intent.

## Safety and Live Target Protection

- **Protected file paths**: Tests must never read from or write to live user config paths:
  - `~/.config/opencode-mom`
  - `~/.config/opencode`
  - `~/.omo`
- **Privileged actions**: Never invoke system commands (`sudo`, `pmset`, `powercfg`) in tests or mock setups.
- **Domain privacy rules**: Never use real credentials, private user files, or real tokens. Synthetic sentinel values (e.g. `test-token-123`) are explicitly permitted to test authentication wire contracts, token rotation, and redaction logic without leaking real secrets.

## Review Questions

- Does this mock replicate the exact wire payload expected from Rust?
- If the mock is removed, can the code run truthfully with real collaborators?
- Does the test fail cleanly when the mocked error contract changes?

## Source References

- [Adapter regression](../../../../../src/config/__tests__/adapter.test.mjs) - Adapter command mocking and payload verification.
- [Config contracts](../../../../../src/config/docs/command-contracts.md) - Generation contracts and Island wire rules.
- [Backend contracts](../../../../../src-tauri/src/docs/config-contracts.md) - Error shapes and reference protection.
