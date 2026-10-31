# MCP Management and MCP Proxy

## Overview

This route manages Global and project-scoped MCP configuration alongside the built-in MCP HTTP Proxy gateway. Query-driven tabs keep configuration browsing separate from Proxy runtime controls.

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
| --- | --- | --- |
| Route container | `+page.svelte` | Tab navigation and active-panel refresh. |
| Proxy runtime | `ProxyPanel.svelte` | Status, server actions, and unit management. |
| Proxy drafts | `ProxyServerForm.svelte` | Server editing and configuration import. |
| Configuration editing | `McpForm.svelte`, `DeleteMcpDialog.svelte` | Full JSON writes and guarded deletion. |
| Safe display | `config-view.ts` | Non-mutating credential masking. |

## Conventions

- **Tab routing**: Preserve configuration scope and edit query parameters when switching tabs; configuration
  remains the default. Follow the existing [routing
  contract](docs/configuration-editing.md#tabs-and-active-refresh).

- **Active refresh**: The header refreshes only the active tab; configuration and Proxy keep separate data
  ownership. Preserve [refresh routing](docs/configuration-editing.md#tabs-and-active-refresh) rather than
  refreshing both panels.

- **Async ownership**: Proxy polling must not overlap reads, and teardown invalidates late results before
  feedback or follow-up writes. Preserve [polling and completion
  guards](docs/proxy-lifecycle.md#polling-teardown-and-async-completion).

- **Revision snapshots**: Keep the captured revision with each edit or toggle; polling never advances draft
  expectations. Preserve backend conflict detection and [mutation
  snapshots](docs/proxy-lifecycle.md#revision-snapshots).

- **Literal values**: Preserve command arguments and environment values exactly; do not add shell parsing or
  normalize meaningful whitespace. Follow [form
  fidelity](docs/proxy-lifecycle.md#literal-form-values-and-private-drafts).

- **Private drafts**: Keep tokens and sensitive environment values out of global state, recovery, storage, and
  logs. Maintain the existing [secret
  boundaries](docs/proxy-lifecycle.md#literal-form-values-and-private-drafts).

- **Masked display**: Read-only views and search use non-mutating masks; only the explicit JSON editor
  receives raw configuration. Preserve [credential-safe
  display](docs/configuration-editing.md#credential-safe-display).

## Sync

Keep this guide and linked docs in sync with changes to this directory.
