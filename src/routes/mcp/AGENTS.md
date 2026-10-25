# MCP Management and MCP Proxy

## Overview

This directory provides the management UI for both standard MCP configuration files (global and project-scoped) and the built-in MCP HTTP Proxy gateway. The page supports tab navigation (`?tab=config` | `?tab=proxy`, default `"config"`).

## Directory Structure

```text
mcp/
├── +page.svelte           # Tabbed container for configuration list and MCP Proxy panel
├── ProxyPanel.svelte      # MCP Proxy runtime status, server list, and unit management
├── ProxyServerForm.svelte # Add, edit, and MCP-import dialog for proxy servers
├── McpForm.svelte         # Full JSON creation/editing for standard MCP entries
├── DeleteMcpDialog.svelte # Deletion confirmation for standard MCP entries
├── config-view.ts         # Non-mutating credential masking for display
└── __tests__/             # Unit and component regression tests
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Container | `+page.svelte` | Tab switching via `?tab=config` and `?tab=proxy`. Preserves `?project=`, `?readonly=1`, and `?edit=1` on the config tab. Dispatches header refresh to the active tab. |
| Proxy panel | `ProxyPanel.svelte` | Gateway status overview (actual/desired state, bound address, active units, revision), proxy server table with enable toggle, edit, delete, and unit termination. 2-second non-overlapping polling. |
| Server form | `ProxyServerForm.svelte` | Dialog for creating/editing proxy servers (ID, name, enabled, command args, cwd, env, protocol, timeouts) and importing configurations from existing MCP servers. |
| Standard MCP | `McpForm.svelte`, `DeleteMcpDialog.svelte` | Scoped MCP server creation, JSON editing, and deletion. |

## Conventions

- **Tab Routing**: Use `?tab=config` (default) and `?tab=proxy`. Switching tabs updates the URL query with `replaceState`. When on the config tab, project scopes (`?project=`), read-only flags (`?readonly=1`), and edit intents (`?edit=1`) remain fully functional.
- **Header Refresh**: The header refresh button only operates on the currently active tab: refreshing standard MCP list on the config tab, and refreshing proxy state on the proxy tab.
- **Proxy Polling**: `ProxyPanel` executes non-overlapping 2-second polling while mounted. Teardown cancels the timer and invalidates in-flight sequences.
- **Proxy async safety**: Server lookups and mutations ignore late results/errors after teardown. Recheck mounted state between prerequisite reads and writes; late mutations never close forms, navigate, or emit feedback. Pending guards deduplicate server lookups/toggles and form saves/imports. Delete and unit-close completion restore polling even on failure. Unit and session counts render their separate DTO fields. Loading or failed status reads never display Stopped as a fallback.
- **Proxy form fidelity**: Each command line is one literal argument, including blank lines (empty arguments), quotes, and surrounding whitespace; no shell unquoting applies, and the executable must be nonblank. Environment values remain literal after the first `=`, including whitespace, empty values, quotes, and additional `=` characters. Only environment blank/comment lines are ignored, without imposing shell-identifier restrictions on keys.
- **Ephemeral Secrets**: Plaintext tokens and sensitive environment variables are never stored in `AppState`, global `draftRecovery`, `localStorage`, or debug logs.
- **Server Mutation**: Capture `revision` before reading a full server and keep that snapshot with the edit draft or enable toggle. Polling must not advance its `expectedRevision`; stale submissions rely on backend conflict detection. New servers read a revision before creation. Enabled controls use the native Switch's `bind:checked` with an accessible name.

## Sync

Update this file when MCP routing, proxy management, or dialog interactions change.
