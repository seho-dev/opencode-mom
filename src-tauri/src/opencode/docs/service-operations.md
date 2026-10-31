# OpenCode Operations and Bounded Data Contracts

## Overview

Integrates with the CLI for binary discovery, installation inspection and explicit stable V2 upgrades. One selected local service powers model catalogs, reload, active counts, usage and Mini Island: validated Shared Service by default, or an explicit fixed loopback V2 endpoint without fallback/bootstrap. Usage scans bounded HTTP pages with fork-aware attribution; Island uses the same pinned transport asynchronously.

Paths in this table are relative to `src-tauri/src/opencode/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Public boundary | `mod.rs` | Re-exports service operations, binary resolution, `ModelCatalogEntry`, `TokenUsageRecord`, `OpenCodeInstallation`, and `OpenCodeUpdateCheck`; installation operations stay crate-private. |
| Binary discovery | `binary.rs` | `opencode_candidates` and `resolve_opencode_binary`; final bare command resolves through PATH. |
| Installation | `installation.rs` | Inspects exact executable ownership without service access; checks the fixed stable V2 endpoint and explicitly runs guarded native upgrades for proven Unix curl/npm/bun/pnpm/yarn installations. |
| Bounded collection | `command.rs` | `collect_output` drains stdout without blocking indefinitely and reaps only its direct child. |
| Model catalog | `models.rs` | GET `/api/model` with captured `location[directory]`, enabled `providerID/modelID` refs plus configured models/metadata. |
| Reload | `reload.rs` | `reload_via_service` uses POST `/api/location/reload`, bounded five-second dispatch, 204 success, no retry. |
| Active work | `active.rs` | `active_sessions_via_service` uses GET `/api/session/active`, shared strict map/object parser and running-session count. |
| Mini Island | `island.rs`, `island/` | Async managed V2 requests, public camelCase DTOs, bounded snapshots/messages, guarded writes and live-only SSE. |
| Usage attribution | `token_usage.rs` | `token_usage_records_via_service` scans timelines and returns only `{time, input, output}`. |
| Prompt generation | `generate.rs`, `generate/tests.rs` | Bounded one-shot prompt assistance and mock-only transport checks. |

## Conventions

- **Candidate order**: Reuse `opencode_candidates`: existing trimmed `OPENCODE_BIN`, known install paths, then bare `opencode`. Skip missing explicit paths but still try the final PATH candidate.
- **Installation safety**: Invalid explicit OPENCODE_BIN never falls back for installation. Auto-upgrade only proven Unix curl/npm/bun/pnpm/yarn stable V2 >=2.0.25 via `selected_binary upgrade <stable-V2-target> --method <method>` with expected path/version/latest stable target, concurrent `busy`, ownership revalidation, and same-path readback; all Windows/Homebrew/Vite Plus/older V2/unproven/manual/bundled sources stay manual. Never sudo, upgrade bundled binaries, or change service lifecycle/equate CLI with backend version; cleanup signals only the isolated installer process group, and tests use fake processes/local HTTP.
- **Collector ownership**: Bounded operations pipe stdout/null stdin/stderr; success requires exit success plus EOF, while nonzero exit discards stdout immediately. On failure kill/reap only direct CLI child, never a process group containing the service.
- **Timeout scope**: Active queries use 5 seconds/1 MiB; bootstrap at most 10 seconds/32 KiB within the operation deadline. Catalog uses 20 seconds overall, five-second requests/8 MiB; reload uses 20 seconds overall and five-second POST. Blocking catalog/status/saves run in spawn_blocking.
- **Active-session contract**: Accept only an object envelope with a `data` map of unique `ses_` IDs with nonempty ASCII alphanumeric/underscore/hyphen suffixes and object values of type `running`. Count children too; malformed responses and nonzero exits are errors, not idle.
- **Error privacy**: All service operations retain only error kind/category/location/status, never response bodies, identifiers, credentials or raw serde/HTTP/CLI stderr/stdout errors. Read failures are not idle.
- **Island snapshots**: Fetch latest 20 roots/global validated running map and missing active details including children via existing root-ID set; borrow IDs and count serialized bytes without JSON buffers. Read per-session pending permissions/forms for active/recent roots, never location-global lists; cap four concurrent reads/15 seconds/100 total sessions-pending requests and fail rather than return partial data. Public directory is location.directory; active wins over failed outcome, and connected never implies idle.
- **Island interaction**: Preserve request/session/form IDs/field keys; permissions allow once/always/reject, with explicit Always consent plus fresh exact pending identity/nonempty server save rules, bounded plaintext validation, missing save→[], and invalid/unsavable Always rejected before POST without Once fallback. Send only {decision}; server owns rule/project binding, never MOM rules/persistence. Keep GET+POST in one cancellation-owned future, never queued writes; `/api/session/:id/prompt` uses caller msg_ ID/queue delivery, `/message` is read-only, and 409 is rejection retaining stable ID without resend. Reread pending question detail and validate required/custom options before {answer}; only top-level metadata.kind=question string/multiselect is answerable, accepting hidden=false/when=[] and minItems 0/1 optional/required, all other constraints fail closed. Omit optional empty strings/selections before option checks, reject required blanks; unsupported/fieldless forms stay read-only without discriminator fallbacks. Never retry writes; report generic acceptance ambiguity, not bodies.
- **Island public data**: Retain only user.text/assistant text, excluding synthetic/hidden/internal flags at value/metadata levels and all reasoning/tool/provider state. Fetch newest 20-message page, emit chronological DTOs, truncate each UTF-8 text at 16 KiB with marker; validate IDs/plaintext/date numbers and fail closed on V1/unverified shapes.
- **Island events**: Decode native `/api/event` SSE once as {id,type,created,location,data} with object data, live-only/no replay/hooks; support fragmented multiline/comment/CRLF frames capped 64 KiB and chunks 2 MiB. Execution/session status/idle/pending/server.connected/location.shutdown invalidate snapshots; snapshot-only events need no shared-ID assumption (session.created uses data.session.id). Public text/session.message.content.updated/inbox require data.sessionID and invalidate messages, never per-token snapshots; form.created uses data.form session and ignores global forms. Native consumer owns reconnect/debounce/cache/lifecycle.
- **HTTP paging**: Fetch sessions at limit 1000 and messages at limit 100, initially ascending; thereafter follow cursors until an empty page or no next cursor. Encode session IDs as path segments and cursors as query values; reject duplicate IDs and cursor loops.
- **Scan limits**: Share one 60-second deadline across connection, pages, bodies, workers, and aggregation. Use at most four timeline workers, 32 MiB per page, 300,000 metadata items/64 MiB retained metadata, and 100,000 records; failures never return partial usage.
- **Recorded usage**: Include recorded root `tokens.input/output` only for non-pending assistant/compaction messages, including terminal cancellation/error cases. Missing usage is not zero billing; exclude reasoning/cache fields and enforce safe JavaScript integer/date bounds.
- **Fork ownership**: Validate origin chains, before/through boundaries, native copy eligibility, and copied metadata projections before excluding inherited prefixes; missing origins fall back to root attribution, while missing boundaries, cycles, and mismatches fail the scan.
- **Catalog editing**: `list_models_via_service` uses GET `/api/model`, returning `{location,data: Model.Info[]}`; use enabled entries' `providerID/modelID`, not `id` or providerInfo. Keep configured models including disabled ones; config metadata is authoritative and provider filtering remains client-side.
- **Test fixtures**: Service tests use injected snapshots/paths, dual FakeApi servers and synthetic Basic credentials, never global environment or real API/bootstrap. Collector/bootstrap fake shells remain Unix-gated. Keep `process::read_only_live_scan` ignored.
- **Prompt generation**: Validate kind/preset IDs/text/model before connecting; generate.rs prepends backend editing instructions and calls only POST /api/experimental/generate with target pinned through bootstrap/health/POST/bounded response. Never create sessions, mutate agent/command files, log private text, or automatically retry possibly billable requests.

## Backend Integration

Paths in this table are relative to `src-tauri/src/`.

### Backend Installation Boundary

| Responsibility | File | Description |
| :--- | :--- | :--- |
| OpenCode installation | `opencode/installation.rs` | Local exact-binary ownership inspection, explicit stable V2 checks and guarded Unix curl/npm/bun/pnpm/yarn native upgrades without OpenCode service lifecycle actions. |

### Publication and Lifecycle Rules

- **OpenCode upgrades**: Offload inspection/check/upgrade to blocking tasks; auto-upgrade only proven Unix curl/npm/bun/pnpm/yarn stable V2 >=2.0.25 via `selected_binary upgrade <stable-V2-target> --method <method>`, guarding expected path/version/latest stable target and concurrent `busy`, revalidating ownership and rereading the same path. Windows/Homebrew/Vite Plus/older V2/unproven/manual/bundled remain manual; never sudo, upgrade bundled binaries, or change service lifecycle, and never equate CLI/backend versions. Use fake tests only.
