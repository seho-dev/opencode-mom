# Product terminology and security copy

Paths name source modules; `settings/SettingsSearch.svelte` is relative to `src/routes/`.

## App, CLI and service terminology

- **Project and update messages**: `header.*` covers the GitHub opener; `settings.*` covers manual version checks, macOS trust guidance, and optional Star support. Keep security steps/quarantine commands equivalent across locales; Star support never gates downloads.

- **CLI update messages**: `settings.opencode*` covers local installation details, every backend method/reason, explicit stable checks, exact path/version confirmation, and separate command/readback failures. Keep local-only/no-sudo/no-service-restart warnings equivalent across locales; app updates stay separate.

- **Service operation messages**: Model catalog loading/errors and lid-monitor scope describe the selected local OpenCode service, not CLI transport. Keep CLI wording for installation, updates and Shared Service bootstrap only.

- **Service target messages**: `settings.service*` separates saved target, live backend identity and CLI version; explains direct V2 HTTP loopback APIs, Basic backend passwords, no-auth without a password, and no OpenChamber UI/Bearer fallback. Busy asks users to wait/retry; validation covers direct HTTP loopback URL, positive port and backend password without claiming authentication checked. Never require manual Mini Island/lid-protection disabling or power restoration; backend monitor refresh preserves enabled preferences. Errors omit private bodies. Keep Unix account-owned 0600 plaintext storage, Windows current-account DPAPI, dedicated backend-password guidance and no account-compromise protection equivalent across locales. Never expose credentials/claim verified Windows ACL protection; Windows-native DPAPI acceptance is outside macOS host tests.

## Resource and settings terminology

- **Resource scope copy**: `scope.*` covers list-only Global/project selection, exact-root read-only notices, separate Global comparison, and forgetting folders without deleting files; describe configuration files, never live MCP status or inherited runtime configuration.

- **MOM HTTP copy**: `settings.http.*` covers the General tab listener port (default 18765), LAN access/security warning, runtime status, and saves. `settings.proxy.httpHint` directs proxy users to General; proxy-only controls retain `settings.proxy.*`.

- **Settings search**: `settings.search*` labels the local finder, result count, navigation hint, clear action and empty state; keep both locales concise/action-oriented. `settings/SettingsSearch.svelte` explicitly indexes existing labels/descriptions and technical keys across dictionaries, never user values; preserve key identities. `groups.enableAria`/`groups.disableAria` require matching `{name}` placeholders.

## Dashboard and Island terminology

- **Dashboard copy**: `dashboard.*` covers runtime selection, Quick Actions, configured-resource counts, lazy MCP Proxy status, and independent usage periods. Translate input/output chart abbreviations; unavailable proxy readback never claims zero services or a running gateway.

- **Island copy**: `island.*` covers compact native UI, public messages, permissions/questions, unknown/disconnected status and explicit ambiguous/409 retry states; `island.compact.*` supplies non-notched short status/count labels with full accessible counts in existing keys. Every screen defaults compact; notch geometry changes density only. `settings.island*` covers the macOS-only switch and local V2 Shared Service; closing/disabling never means rejecting requests or interrupting sessions.

- **Dashboard MCP Proxy labels**: Reuse `mcp.proxy.running`/`mcp.proxy.stopped` for runtime state; `settings.proxy.*` is settings copy, not gateway state. Cover successful running/stopped rendering in both locales and unavailable readback.
