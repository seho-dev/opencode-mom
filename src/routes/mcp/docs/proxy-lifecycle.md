# MCP Proxy polling, drafts and mutations

Source-relative paths below are relative to `src/routes/mcp/`, not this docs directory.

## Polling, teardown and async completion

- **Proxy Polling**: `ProxyPanel` executes non-overlapping 2-second polling while mounted. Teardown cancels the timer and invalidates in-flight sequences.

- **Proxy async safety**: Ignore late lookup/mutation results/errors after teardown and recheck mount between prerequisite reads/writes; late mutations never close/navigate/emit feedback. Pending guards deduplicate lookups/toggles/saves/imports. Delete/unit-close completion resumes polling even on failure; unit/session counts use separate DTO fields, and loading/failed status never falls back to Stopped.

## Literal form values and private drafts

- **Proxy form fidelity**: Each command line is one literal argument, preserving blank empty arguments, quotes and surrounding whitespace; never shell-unquote, and require a nonblank executable. Preserve environment values after the first `=` verbatim, including whitespace/empty/quotes/additional `=`; ignore only blank/comment environment lines, without shell-identifier key restrictions.

- **Ephemeral Secrets**: Plaintext tokens and sensitive environment variables are never stored in `AppState`, global `draftRecovery`, `localStorage`, or debug logs.

## Revision snapshots

- **Server Mutation**: Capture `revision` before full-server reads and retain it with edit/toggle drafts; polling never advances `expectedRevision`, and stale writes rely on backend conflicts. New servers read revision before creation. Enabled controls use native Switch `bind:checked` with an accessible name.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Proxy panel | `ProxyPanel.svelte` | Gateway status overview (actual/desired state, bound address, active units, revision), proxy server table with enable toggle, edit, delete, and unit termination. 2-second non-overlapping polling. |
| Server form | `ProxyServerForm.svelte` | Dialog for creating/editing proxy servers (ID, name, enabled, command args, cwd, env, protocol, timeouts) and importing configurations from existing MCP servers. |

## Listener and settings ownership

General MOM HTTP settings, the MCP Proxy settings tab, guarded readback and token rotation are documented in [settings lifecycle](../../settings/docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility). Proxy server polling and forms remain local to this route.
