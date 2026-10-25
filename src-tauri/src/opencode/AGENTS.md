# OpenCode Integration

## Overview

Integrates with the CLI for binary discovery, installation inspection and explicit stable V2 upgrades. One selected local service powers model catalogs, reload, active counts, usage and Mini Island: validated Shared Service by default, or an explicit fixed loopback V2 endpoint without fallback/bootstrap. Usage scans bounded HTTP pages with fork-aware attribution; Island uses the same pinned transport asynchronously.

## Directory Structure

```text
opencode/                     # Local CLI and managed-service integration
├── AGENTS.md                 # Directory-specific guidance
├── mod.rs                    # Public function and DTO re-exports
├── binary.rs                 # Ordered binary candidates and resolver
├── command.rs                # Deadline- and byte-bounded stdout collector
├── installation.rs           # Local ownership detection, stable V2 checks and guarded upgrades
├── installation/             # Installation test submodule
│   └── tests.rs              # Fake binaries/managers/installers, HTTP bounds and upgrade guards
├── target.rs                 # Endpoint persistence, revision, short transitions and in-flight reservations
├── target/                   # URL/password/private-file and monitor transition tests
├── transport.rs              # Shared registration, identity, bootstrap and HTTP requests
├── transport/                # Transport test submodule
│   └── tests.rs              # Trust checks, bootstrap, response bounds, deadlines
├── models.rs                 # V2 model API refs merged with local provider metadata
├── models/                   # Catalog identity and configured disabled metadata tests
├── reload.rs                 # Bounded non-retried HTTP reload
├── active.rs                 # Strict V2 running-session map parser
├── island.rs                 # Crate-private DTOs, async connection and guarded writes
├── island/                   # Bounded Mini Island V2 API lane
│   ├── projection.rs         # Recent roots, active children, pending forms and public text
│   ├── events.rs             # Bounded SSE frames and native event classification
│   └── tests.rs              # Synthetic HTTP, DTO/privacy, answer guards and SSE
├── token_usage.rs            # Paging, budgets, worker scan, fork attribution
├── command/                  # Collector test submodule
│   └── tests.rs              # EOF, timeout, limits, child ownership, FD cleanup
├── reload/                   # Reload test submodule
│   └── tests.rs              # 204 success, private errors and no retries
├── active/                   # Active-session test submodule
│   └── tests.rs              # Shape validation, redaction, count and bounds
└── token_usage/              # Usage tests and shared fake HTTP fixtures
    └── tests.rs              # Paging, forks, limits, fake HTTP, ignored live scan
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Public boundary | `mod.rs` | Re-exports service operations, binary resolution, `ModelCatalogEntry`, `TokenUsageRecord`, `OpenCodeInstallation`, and `OpenCodeUpdateCheck`; installation operations stay crate-private. |
| Binary discovery | `binary.rs` | `opencode_candidates` and `resolve_opencode_binary`; final bare command resolves through PATH. |
| Installation | `installation.rs` | Inspects exact executable ownership without service access; checks the fixed stable V2 endpoint and explicitly runs guarded native upgrades for proven Unix curl/npm/bun/pnpm/yarn installations. |
| Bounded collection | `command.rs` | `collect_output` drains stdout without blocking indefinitely and reaps only its direct child. |
| Service target | `target.rs` | Captured private path/registration/directory, atomic saves with Unix 0600 plaintext or Windows current-account DPAPI credentials, normalized loopback URLs, password-retention rules, revisions and monitor transition guards. |
| Model catalog | `models.rs` | GET `/api/model` with captured `location[directory]`, enabled `providerID/modelID` refs plus configured models/metadata. |
| Reload | `reload.rs` | `reload_via_service` uses POST `/api/location/reload`, bounded five-second dispatch, 204 success, no retry. |
| Active work | `active.rs` | `active_sessions_via_service` uses GET `/api/session/active`, shared strict map/object parser and running-session count. |
| Mini Island | `island.rs`, `island/` | Async managed V2 requests, public camelCase DTOs, bounded snapshots/messages, guarded writes and live-only SSE. |
| Usage attribution | `token_usage.rs` | `token_usage_records_via_service` scans timelines and returns only `{time, input, output}`. |
| Managed transport | `transport.rs` | Crate-private shared transport: validated registration, Basic auth, health identity checks, optional CLI bootstrap, bounded HTTP pages and credential-private async RequestBuilder; imports existing usage paging types/helpers. |

## Conventions

- **Candidate order**: Reuse `opencode_candidates`: existing trimmed `OPENCODE_BIN`, known install paths, then bare `opencode`. Skip missing explicit paths but still try the final PATH candidate.
- **Installation safety**: An invalid explicit `OPENCODE_BIN` never falls back for installation operations. Automatic upgrade requires strictly proven Unix curl/npm/bun/pnpm/yarn ownership and current stable V2 >=2.0.25; Windows/Homebrew/Vite Plus/older V2/unproven/manual/bundled sources remain manual. Execute only `selected_binary upgrade <stable-V2-target> --method <method>` with expected path/version and latest stable target guards, reject concurrent upgrades with `busy`, revalidate ownership before dispatch and read back the same path. Never use sudo, upgrade bundled binaries, or start/stop/restart the OpenCode service; CLI version is not running backend version. Installer cleanup signals only its isolated process group. Tests use fake processes and local HTTP, never real upgrades.
- **Target authority**: Initialize `target` before power/Island startup. Missing private config selects Shared; invalid/unreadable/nonregular/oversized config fails closed without crashing management. Fixed URLs accept only HTTP literal 127.0.0.1/[::1] with positive explicit port or exact localhost normalized without DNS. No credentials/path/query/fragment, remote hosts, shortened/numeric loopback, fallback or explicit bootstrap. Status probes never bootstrap.
- **Target credentials**: Persist the endpoint and backend-only credential in `opencode-service.json`, outside AppConfig. Unix uses a regular account-owned 0600 plaintext file capped at 8 KiB. Windows uses numeric-byte `encryptedPassword` protected by native current-account DPAPI (32 KiB file/8 KiB ciphertext), with no plaintext fallback; credential decryption failures fail closed. Normal Shared registration credentials are handled separately. Never expose passwords to frontend DTOs/errors/logs/events. Neither Unix permissions nor DPAPI protect against account compromise, same-account malicious processes or administrators. Windows-native DPAPI tests have not run on this macOS host; native acceptance remains outside automated host tests and Windows ACL protection is not claimed verified.
- **Temporary credentials**: Wrap owned target-file reads, serialized target bytes, and decrypted password bytes immediately in `protected_data::SecretBytes`; parse through `&bytes.0` so all return paths wipe the temporary buffer. This does not wipe long-lived selection strings or serializer-internal allocations.
- **Target saves**: Use create_new owned UUID temporary and atomic publication; sync the final Unix directory, treating post-commit sync failures as private warnings rather than leaving runtime behind disk. Null URL clears Shared/password; omitted password retains only on unchanged normalized URL, empty clears. The global transition mutex also guards PowerService::set and IslandService::set_enabled inside main-thread dispatch. Saves validate the selection, prepare pure monitor guards, persist and publish the new selection/revision, then infallibly invalidate old consumers under the same transition/monitor guards; failed preparation/persistence leaves monitors unchanged. Lock order is transition -> monitor effects/shared/cache -> target state write, dropped before commit. No native/network/await work runs under these guards; finish restarts readers/native work only after transition release and before the five-second status probe. A stale/unavailable post-save probe returns disconnected public status for the committed selection, never a failed save. Normal status/read revision checks remain fail-closed. Bootstrap/reload/Island writes acquire a runtime-counted RAII reservation under transition+revision check, then release the mutex before network/await/native dispatch; target saves return busy while reserved. Island blocking workers own their reservation even when their async caller is dropped.
- **Bootstrap fallback**: Only Shared may bootstrap missing/unreachable/identity-mismatched registration; invalid/auth/HTTP errors never bootstrap. CLI candidates stop after successful spawn even when bootstrap fails. The resolver checks existence, not executable validity.
- **Collector ownership**: Bounded operations pipe stdout and null stdin/stderr. Success requires both successful exit and stdout EOF; nonzero exit discards stdout immediately. On failure, kill/reap only the direct CLI child, never a process group that might contain the service.
- **Timeout scope**: Active queries use 5 seconds/1 MiB; bootstrap at most 10 seconds/32 KiB within the operation deadline. Catalog uses 20 seconds overall, five-second requests/8 MiB; reload uses 20 seconds overall and five-second POST. Blocking catalog/status/saves run in spawn_blocking.
- **Active-session contract**: Accept only an object envelope with a `data` map of unique `ses_` IDs with nonempty ASCII alphanumeric/underscore/hyphen suffixes and object values of type `running`. Count children too; malformed responses and nonzero exits are errors, not idle.
- **Error privacy**: All service operations retain only error kind/category/location/status, never response bodies, identifiers, credentials or raw serde/HTTP/CLI stderr/stdout errors. Read failures are not idle.
- **Registration trust**: Read the regular, at-most-8-KiB `opencode/service.json` under absolute `XDG_STATE_HOME` or the captured ConfigPaths home `~/.local/state`, never inferred config parents. Require a UUID, positive PID, bounded valid major-2 semantic version, nonempty bounded password, and explicit-port HTTP URL on literal `127.0.0.1` or `[::1]` only. An uninitialized runtime fails closed; library tests inject runtimes/paths rather than touching live registration.
- **Managed identity**: With a password use Basic auth user `opencode`; a passwordless fixed endpoint sends no Authorization on health/read/reload/async requests. No Bearer/UI pairing inference or 401/403 fallback; only direct OpenCode backend endpoints, not UI proxies. Disable proxies/redirects/retries, strictly parse object `/api/info` with positive PID/bounded valid V2 version. Shared requires exact registration PID/version; fixed captures the probed identity without registration. Requests pin endpoint/auth/identity/revision and writes recheck health before dispatch; an external restart between health and dispatch remains unpreventable without server support. A reachable info route is not proof of protected-route access; no implicit restart for a CLI version mismatch.
- **Island transport**: Run initial reservation/connect/health/bootstrap inside the existing `spawn_blocking` closure. Async write preparation clones an `Arc` to the pinned Snapshot and acquires its reservation in `spawn_blocking`, never polling the transition mutex or sleeping on an async worker; the returned RAII guard covers fresh health and the caller's request through response completion. Dropping a waiter does not cancel started blocking work; an unclaimed result drops its reservation automatically. Subsequent requests/write-health/chunks remain async and cancellation drops local in-flight work. Blocking bootstrap keeps its reservation until actual worker completion. A cancelled POST may already have been accepted by the server; never claim withdrawal or retry. Credentials stay inside transport, async clients disable all automatic retries, write-health plus dispatch share a five-second deadline, and bodies/retained snapshots cap at 2 MiB. No detached async write task is introduced.
- **Island snapshots**: Fetch only the latest 20 root candidates and the global validated running map; fetch missing active details including children using the existing root-ID set. Borrow session IDs for detail/pending reads and count serialized bytes without allocating JSON buffers. Check per-session pending permission/form lists for both active sessions and recent roots, never location-scoped global pending lists. Four in-flight reads, 15-second snapshot ceiling, 100 total sessions/pending requests; bounds fail explicitly instead of returning partial data. Public directory is `location.directory`; active state wins over last failed outcome and `connected` never determines idle.
- **Island interaction**: Keep original request/session/form IDs and field keys. Permission replies permit only `once`/`always`/`reject`; Always requires explicit user consent and rereads the per-session pending list, requiring the exact session/request identity and nonempty server-provided `save` rules. Missing save projects to `[]`; supplied rules use the same bounded strings/plaintext checks as resources. Stale, invalid, or unsavable Always fails validation before POST, never falls back to Once, and sends only `{decision}`; the server owns rule/project binding, with no custom rules or MOM permission persistence. GET and POST stay in one cancellation-owned future, never a queued background write. Send posts to `/api/session/:id/prompt` with caller `msg_` ID and queue delivery (`/message` is read-only). Duplicate message IDs return HTTP 409 as a rejected write, never success; keep the stable ID and never automatically resend. Question replies reread form detail `state.status=pending` and validate required/custom options before posting `{answer}`. Only top-level form `metadata.kind=question` string/multiselect forms are answerable; inert `hidden=false`/`when=[]` are accepted, multiselect `minItems=0/1` maps to optional/required, and other constraints fail closed. Empty optional strings/selections are omitted before option validation; required blanks remain rejected. No metadata/discriminator fallback; fieldless/unsupported forms stay read-only. Writes are never retried; failures retain generic acceptance ambiguity, not raw bodies.
- **Island public data**: Retain only `user.text` and assistant content of type `text`, excluding synthetic/hidden/internal flags at both value and metadata levels, and all reasoning/tool/provider state. Fetch one newest 20-message page, project chronological DTOs, cap each UTF-8 text at 16 KiB, and mark truncation. IDs/plaintext/date numbers are validated; malformed V1/unverified shapes fail closed.
- **Island events**: `/api/event` uses native `{id,type,created,location,data}` JSON with object `data` (the client JSON-decodes the SSE frame once), live only with no last-event replay or plugin hooks. Parse fragmented multiline/comment/CRLF frames with a 64-KiB frame cap and 2-MiB chunk cap. Native execution/session status/idle/pending lifecycles, `server.connected` recovery, and `location.shutdown` invalidate snapshots; snapshot-only session/permission/location events do not assume a shared ID shape (`session.created` owns `data.session.id`). Public text, `session.message.content.updated`, and inbox delivery require `data.sessionID` and invalidate messages, never per-token snapshots. `form.created` owns its session under `data.form`; non-session global forms are ignored. Reconnect, debounce, cache, and service lifecycle belong to the native Island consumer.
- **HTTP paging**: Fetch sessions at limit 1000 and messages at limit 100, initially ascending; thereafter follow cursors until an empty page or no next cursor. Encode session IDs as path segments and cursors as query values; reject duplicate IDs and cursor loops.
- **Scan limits**: Share one 60-second deadline across connection, pages, bodies, workers, and aggregation. Use at most four timeline workers, 32 MiB per page, 300,000 metadata items/64 MiB retained metadata, and 100,000 records; failures never return partial usage.
- **Recorded usage**: Include recorded root `tokens.input/output` only for non-pending assistant/compaction messages, including terminal cancellation/error cases. Missing usage is not zero billing; exclude reasoning/cache fields and enforce safe JavaScript integer/date bounds.
- **Fork ownership**: Validate origin chains, before/through boundaries, native copy eligibility, and copied metadata projections before excluding inherited prefixes; missing origins fall back to root attribution, while missing boundaries, cycles, and mismatches fail the scan.
- **Catalog editing**: `list_models_via_service` uses GET `/api/model`, returning `{location,data: Model.Info[]}`; use enabled entries' `providerID/modelID`, not `id` or providerInfo. Keep configured models including disabled ones; config metadata is authoritative and provider filtering remains client-side.
- **Test fixtures**: Service tests use injected snapshots/paths, dual FakeApi servers and synthetic Basic credentials, never global environment or real API/bootstrap. Collector/bootstrap fake shells remain Unix-gated. Keep `process::read_only_live_scan` ignored.

## Anti-patterns

- Do not fetch large usage pages through CLI stdout: CLI `process.exit` can truncate responses; stdout is reserved for the small bootstrap health result.
- Do not deduplicate fork usage by message IDs or timestamp cutoffs: copied messages receive new IDs, and owned child messages may have older timestamps.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
