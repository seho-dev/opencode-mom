# Tools & Integrations API Reference

This reference covers Model Context Protocol (MCP), Skills, Plugins, External Service Integrations, Credentials, Permissions, and User Forms (36 endpoints).

---

## 1. MCP (Model Context Protocol) (6 endpoints)

Manage local stdio and remote SSE MCP servers dynamically at runtime.

### Endpoints
- `GET /api/mcp` (`mcp.list`): List registered MCP servers and connection status (`connected`, `disabled`, `needs_auth`, `failed`).
- `PUT /api/experimental/mcp/{server}` (`experimental.mcp.add`): Add or update an MCP server configuration.
  - Body (Local stdio):
    ```json
    {
      "config": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-postgres"],
        "env": { "DATABASE_URL": "..." }
      }
    }
    ```
  - Body (Remote SSE):
    ```json
    {
      "config": {
        "url": "https://mcp.internal.example/sse"
      }
    }
    ```
- `DELETE /api/experimental/mcp/{server}` (`experimental.mcp.remove`): Disconnect and remove MCP server.
- `POST /api/experimental/mcp/{server}/connect` (`experimental.mcp.connect`): Reconnect a disconnected or failed server.
- `POST /api/experimental/mcp/{server}/disconnect` (`experimental.mcp.disconnect`): Sever active connection.
- `GET /api/mcp/resource` (`mcp.resource.catalog`): Query unified catalog of MCP resources and prompt templates across all servers.

---

## 2. Skills & Plugins (5 endpoints)

### Skills
- `GET /api/skill` (`skill.list`): Discover all loaded local and global skills (`id`, `name`, `description`).

### Plugins
- `GET /api/plugin` (`plugin.list`): List installed plugins and enabled hooks.
- `POST /api/plugin/check` (`plugin.check`): Query package registries for plugin version updates (`{ target?: string }`).
- `POST /api/plugin/update` (`plugin.update`): Update plugins and trigger zero-downtime hot reload (`{ targets: string[] }`).
- `POST /api/rpc/{rpcID}/{method}` (`rpc.call`): Call a custom RPC method exposed by a running plugin.

---

## 3. Integrations & Credentials (17 endpoints)

Connect third-party accounts (GitHub, GitLab, Jira, Cloud providers) via API keys, OAuth, or CLI commands.

### Integrations
- `GET /api/integration` (`integration.list`): List available integrations and connection state.
- `GET /api/integration/{integrationID}` (`integration.get`): Get single integration status.
- `POST /api/experimental/integration/wellknown` (`experimental.integration.wellknown.add`): Register well-known provider.
- `POST /api/integration/{integrationID}/connect/key` (`integration.connect.key`): Connect using direct token/key.
- `POST /api/integration/{integrationID}/connect/external` (`integration.connect.external`): Connect using existing system credentials.

### OAuth Connection Lifecycle
1. `POST /api/integration/{integrationID}/connect/oauth` (`integration.oauth.connect`): Initiate OAuth flow; returns authorization URL and `attemptID`.
2. `GET /api/integration/{integrationID}/connect/oauth/{attemptID}` (`integration.oauth.status`): Poll callback settlement.
3. `POST /api/integration/{integrationID}/connect/oauth/{attemptID}/complete` (`integration.oauth.complete`): Finalize tokens.
4. `DELETE /api/integration/{integrationID}/connect/oauth/{attemptID}` (`integration.oauth.cancel`): Cancel OAuth flow.

### Command-Based Connection Lifecycle
- `POST /api/integration/{integrationID}/connect/command`: Run CLI command flow (e.g. `gh auth login`).
- `GET /api/integration/{integrationID}/connect/command/{attemptID}`: Read command login status.
- `DELETE /api/integration/{integrationID}/connect/command/{attemptID}`: Abort command connection.

### Credentials Storage
- `GET /api/credential` (`credential.list`): List saved credentials.
- `POST /api/credential` (`credential.create`): Store encrypted credential.
- `PATCH /api/credential/{credentialID}` (`credential.update`): Update credential attributes.
- `DELETE /api/credential/{credentialID}` (`credential.remove`): Delete credential.
- `POST /api/credential/{credentialID}/activate` (`credential.activate`): Activate credential as current profile.

---

## 4. Permissions & Forms (8 endpoints)

Security and user interaction boundaries during tool execution.

### Permission Requests
When an agent attempts a sensitive operation (filesystem write, shell command, network access) that violates or exceeds default rules:
- `GET /api/permission/request` (`permission.request.list`): Global list of pending permission requests across sessions.
- `GET /api/session/{sessionID}/permission` (`session.permission.list`): List requests for a specific session.
- `GET /api/session/{sessionID}/permission/{requestID}` (`session.permission.get`): Inspect request scope, tool name, and payload.
- `POST /api/session/{sessionID}/permission/{requestID}/reply` (`session.permission.reply`):
  - Body:
    ```json
    {
      "decision": "once",   // "once" | "always" | "reject"
      "message": "Approved for current file only"
    }
    ```
- `GET /api/permission/saved` (`permission.saved.list`): View persistent rules saved from `always` decisions.
- `DELETE /api/permission/saved/{id}` (`permission.saved.remove`): Revoke a saved permission rule.

### Interactive Forms
- `GET /api/form` (`form.list`): List all pending forms globally.
- Form lifecycle within session: see [references/sessions.md](sessions.md#6-interactive-boundaries-forms--permissions).
