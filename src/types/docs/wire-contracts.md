# Domain Wire Contracts

## App, model, and resource records

| Responsibility | File | Contract |
| :--- | :--- | :--- |
| App contract | `app.ts` | `AppState`, `ProjectFolder`, `AppPreferences`, `AppInfo`, `UpdateCheck`, `ProjectPage`, and `CommandError`; project folders, selections, and diagnostics are optional snapshot fields. |
| OpenCode installation | `app.ts` | `OpenCodeInstallation` and `OpenCodeUpdateCheck`; exact nullable binary/version, backend-owned method/reason, manual command, and stable-update availability. |
| Service target | `app.ts` | `OpenCodeServiceTarget` and `OpenCodeServiceStatus`; Shared/fixed mode, public password presence and nullable connected identity/error fields. |
| Group contract | `groups.ts` | `Group`, `AgentModelBinding`, and `CategoryMapping`; preset-specific collections can be `null`. |
| Model contract | `models.ts` | `ModelRef`, `ModelDef`, and `ModelCatalogEntry`; catalog limits and variants have distinct optional/nullable shapes. |
| Agent contract | `agents.ts` | `AgentDefinition`, typed optional `AgentSourcePreview[]` and `AgentFieldOverride[]`, permissive source records, permission rules, and `AgentWrite` mutation metadata. |
| Resource writes | `mcp.ts`, `skills.ts` | Separate saved entries, creation drafts, and updates carrying expected source/content snapshots. |
| MCP Proxy contract | `mcp-proxy.ts` | Gateway state, proxy-only settings, MOM HTTP listener settings, proxy server DTOs, import request, and unit close request. |

- **Constant-derived unions**: `agents.ts` and `groups.ts` import and re-export their domain unions from `src/utils/constants.ts`; extend those source constants rather than duplicating unions here.
- **References and dictionaries**: Use `ModelRef` (`${string}/${string}`) for model bindings; `ProviderDef.models` is a `Record<string, ModelDef>`, while model variants are arrays.
- **Optional versus null**: Preserve required nullable preset collections in `Group` and optional nullable catalog fields; `AppState.selectedGroupId` can be absent or `null`.
- **Project folders**: `ProjectFolder` stores only `id`/`path`; UI derives display names. Optional `AppState.projects`/`selectedProjectId` preserve older payload/mock compatibility.
- **Open-ended data**: Retain `Record<string, unknown>` for raw/config/settings objects, `unknown` for variant bodies and error detail, and open strings for agent modes and permission effects.
- **Permission results**: Preserve the `ok` discriminator in `PermissionParse` and `mode` discriminator in `PermissionView`; only successful/rules branches carry rules.
- **Usage shapes**: Keep timestamped `TokenUsageRecord.time` separate from aggregated `DailyTokenUsage.date`; both retain numeric input/output counts.

## Native notification records

`notifications.ts` owns `NotificationSettings` (`enabled`, `agentCompleted`, `approvalRequired`, `agentError`, all required booleans) and the separate `NotificationPermission` union (`prompt | granted | denied | unavailable | unsupported`). Rust persists preferences as `state.notifications`, not `AppPreferences` or `AppState`; OS authorization is queried independently and never inferred from the enabled preference. A granted status does not guarantee presentation under Focus / Do Not Disturb. Permission get/request IPC carries no arguments; opening system settings resolves void.

## Diagnostic log records

`logging.ts` owns `LoggingState`, `LogRecord`, `LogQuery` and `LogPage`. `LOG_TYPES` supplies the fixed type vocabulary; `LogStatus` is strictly `success | error`. Records use UTC RFC3339 timestamps, a session UUID, a sequence and an operation UUID to connect checkpoints and final results. Optional duration/error-code/context fields are not raw error objects or arbitrary configuration dictionaries.

Keep logging state outside `AppState` and `AppPreferences`; `enabled` is the persisted preference while `recording` confirms the current writer state. Queries carry optional filters and an opaque cursor, and return bounded pages with a nullable next cursor and invalid-line count. Never add credentials, payload bodies, prompts or messages to the logging wire contract.

## Frozen source snapshots

- **Agent previews**: Optional `sources` retain storage, logical `path`, `resolvedPath`, backend-generated `content` string, `raw`, fields, and nullable prompt. Echo `content` unchanged: inline agent JSON or full Markdown; regenerating JavaScript `raw` can alter floats/large integers. Keep new fields optional for legacy payloads/mocks; current backend reads supply them. Optional `overrides` require `field`, `inline`, `markdown`, and `effective`.
- **Guarded updates**: Retain `McpUpdate.expectedConfig`/`expectedSourcePath` and `SkillUpdate.expectedContent`/`expectedPath`. Agent `mutation.expectedPath`/`expectedContent` echo frozen preview `resolvedPath`/`content`; backend updates reject missing snapshots with `validation_failed`. Agent creates remain unchanged, without expected fields.

## Installation and service metadata

- **Manual updates**: `AppInfo.platform` is `macos`/`windows`/`linux`/`other`; required nullable `UpdateCheck.latestVersion` is null without a stable release. `ProjectPage` allows only `repository`/`releases`; metadata/check results stay outside `AppState`.
- **CLI updates**: Keep installation method/upgrade-reason unions wire-compatible. `OpenCodeInstallation.binaryPath`/`version`/`upgradeReason`/`manualCommand` are required nullable; `OpenCodeUpdateCheck.latestVersion` is non-null. Backend values alone determine version ordering and upgrade capability.
- **Service target**: Mode is `shared`/`explicit`; url is required nullable; `hasPassword` is the sole credential metadata. Keep status connected/url/pid/version/error separate from target/local CLI metadata, with required nullable identity/error fields. Never return passwords in DTOs/AppState; preserve `busy`/`conflict` error codes for guarded operations.

## Proxy and listener separation

`mcp-proxy.ts` also exports `MomHttpSettings` (`port`, `allowLan`) for the shared application HTTP
listener. These fields are absent from `McpProxySettingsDto`; `McpProxyStateDto` retains read-only HTTP
network metadata alongside proxy state.

- **MCP Proxy state**: `actual`/`desired` and `desiredState` are booleans; `actualState` is `running`/`stopped`. Keep `boundAddr` required nullable and expose separate numeric `activeUnits` and `activeSessions` counters.
- **MOM HTTP settings**: `MomHttpSettings` contains port and allowLan only; `McpProxySettingsDto` contains enabled, idleTimeout, maxUnits, and revision. Keep listener settings separate from revision-guarded proxy writes.
- **Shared HTTP token**: `rotate_mom_http_key` returns a string through `rotateMomHttpKey()` without arguments. Keep the token and its verifier out of settings DTOs and `AppState`; HTTP settings have no revision. The General dialog owns the ephemeral result, independently of Proxy availability or enablement.

## Island requests and prompt generation

`prompt-generation.ts` defines the shared generation request and constant-derived preset/kind types.

- **Island contract**: Keep wire records in `island.ts`; required numeric `connectionGeneration` is the native cache generation, not a URL/credential, and monotonically invalidates target-local UI state. Required numeric `completionSequence` counts deduplicated successful main-session events within that generation, never inferred status transitions; initial or reconnected UI reads establish a baseline without replay. `IslandView` is `compact`/`peek`/`expanded`; permission `save?: string[]` holds only server-provided rules; `IslandPermissionDecision` is `once`/`always`/`reject`. Always needs explicit consent and a fresh backend check of the exact pending request with nonempty save rules; never infer rules from resources. Question answers use field keys/option values; messages expose only public user/assistant text.
- **Prompt generation**: Derive kind/preset unions from `src/utils/constants.ts`; kinds are `agent`, `command` and `behavior` (Global `AGENTS.md`). Optional `model: ModelRef` and separate optional `variant` default to the OpenCode service model when both are omitted; preset instructions and provider credentials never enter this wire contract.
