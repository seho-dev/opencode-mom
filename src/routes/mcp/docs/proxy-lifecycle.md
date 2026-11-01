# MCP Proxy refresh, drafts and mutations

Source-relative paths below are relative to `src/routes/mcp/`, not this docs directory.

## Refresh, teardown and async completion

- **Proxy refresh**: `ProxyPanel` reads once on mount (including tab reentry), on explicit refresh, and after successful mutations. Reads never overlap; mutation readback queues once if another read is pending. There is no frontend periodic refresh. Teardown invalidates in-flight sequences and suppresses queued work. Native server/session operation, idle sweeping, leases, and transport health remain independent.

- **Proxy async safety**: Ignore late lookup/mutation results/errors after teardown and recheck mount between prerequisite reads/writes; late mutations never close/navigate/emit feedback. Pending guards deduplicate lookups/toggles/saves/imports. Failed actions retain existing error feedback without starting another read; unit/session counts use separate DTO fields, and loading/failed status never falls back to Stopped.

## Literal form values and private drafts

- **Proxy form fidelity**: Each command line is one literal argument, preserving blank empty arguments, quotes and surrounding whitespace; never shell-unquote, and require a nonblank executable. Preserve environment values after the first `=` verbatim, including whitespace/empty/quotes/additional `=`; ignore only blank/comment environment lines, without shell-identifier key restrictions.

- **Ephemeral Secrets**: Plaintext tokens and sensitive environment variables are never stored in `AppState`, global `draftRecovery`, `localStorage`, or debug logs.

## Revision snapshots

- **Server Mutation**: Capture `revision` before full-server reads and retain it with edit/toggle drafts; refresh never advances `expectedRevision`, and stale writes rely on backend conflicts. New servers read revision before creation. Enabled controls use native Switch `bind:checked` with an accessible name.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Proxy panel | `ProxyPanel.svelte` | Gateway status overview (actual/desired state, bound address, active units, revision), proxy server table with enable toggle, edit, delete, and unit termination. Entry, explicit refresh, and mutation readback only. |
| Server form | `ProxyServerForm.svelte` | Dialog for creating/editing proxy servers (ID, name, enabled, command args, cwd, env, protocol, timeouts) and importing configurations from existing MCP servers. |

## Listener and settings ownership

General owns MOM HTTP listener settings and the shared token used by MCP Proxy. The MCP Proxy settings tab owns only Proxy configuration, with no duplicate token controls. Guarded readback and one-time rotation are documented in [settings lifecycle](../../settings/docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility). Proxy server refresh and forms remain local to this route.

## External HTTP endpoint

Clients connect to `http://127.0.0.1:18765/mcp-proxy?server=<server-id>` using the configured MOM HTTP listener address and shared MOM HTTP Bearer token. Rotate that token in Settings → General → MOM HTTP; rotation invalidates the old token and terminates active client connections, including MCP sessions. POST requests, legacy GET streams, and legacy DELETE session cleanup share this path and retain the `server` query field and `Mcp-Session-Id` header. There is no `/mcp` compatibility endpoint; it returns HTTP 404. The management console's Svelte route remains `/mcp`, and upstream MCP server URLs are unchanged.
