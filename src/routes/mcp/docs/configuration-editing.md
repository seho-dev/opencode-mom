# MCP routing and configuration display

Source-relative paths below are relative to `src/routes/mcp/` unless a reference table names another scope.

## Ownership

This directory provides the management UI for both standard MCP configuration files (global and project-scoped) and the built-in MCP HTTP Proxy gateway. The page supports tab navigation (`?tab=config` | `?tab=proxy`, default `"config"`).

## Tabs and active refresh

- **Tab Routing**: Use `?tab=config` (default)/`?tab=proxy` and update query with `replaceState` on switching; config-tab `?project=`, `?readonly=1`, and `?edit=1` remain functional.

- **Header Refresh**: The header refresh button only operates on the currently active tab: refreshing standard MCP list on the config tab, and refreshing proxy state on the proxy tab.

## Credential-safe display

- **MCP display**: Read-only targets/search use `maskTarget`; detail fields use `maskConfig`. Only the explicit JSON editor receives unmasked configuration.

## Implementation references

Paths relative to `src/routes/mcp/`:

| Responsibility | File | Description |
| --- | --- | --- |
| Container | `+page.svelte` | Tab switching via `?tab=config` and `?tab=proxy`. Preserves `?project=`, `?readonly=1`, and `?edit=1` on the config tab. Dispatches header refresh to the active tab. |
| Standard MCP | `McpForm.svelte`, `DeleteMcpDialog.svelte` | Scoped MCP server creation, JSON editing, and deletion. |

Paths relative to `src/routes/`:

| Responsibility | File | Description |
| --- | --- | --- |
| MCP display | `mcp/config-view.ts` | Masks credentials in targets and nested configuration without modifying source values. |
