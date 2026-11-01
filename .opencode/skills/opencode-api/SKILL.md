---
name: opencode-api
description: OpenCode v2 HTTP API reference and integration guide. Load when interacting with or integrating OpenCode v2 HTTP/REST server, managing sessions, sending prompts, streaming events via SSE, controlling PTY/terminals, managing tools/MCP, or configuring workspace/VCS.
---

# OpenCode v2 HTTP API

OpenCode v2 exposes an experimental HTTP/REST API (OpenAPI 3.1.0) with 143 operations covering the full agent lifecycle, terminal control, tools, and workspace management.

## Architecture & Mental Model

```
┌─────────────────────────────────────────────────────────────┐
│                      OpenCode Server                        │
│   (Auth / Pairing / Global Config / SSE Event Stream)       │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
       ┌───────▼────────┐              ┌───────▼────────┐
       │ Location (Dir) │              │ Location (Dir) │
       └───────┬────────┘              └───────┬────────┘
               │                               │
       ┌───────▼────────┐              ┌───────▼────────┐
       │    Session     │              │    Session     │
       │  (Agent Loop)  │              │  (Agent Loop)  │
       └───┬────┬────┬──┘              └───┬────┬────┬──┘
           │    │    │                     │    │    │
       Messages │    Permissions       Messages │    Permissions
             Tools/MCP                       Tools/MCP
```

- **Server**: Top-level daemon instance managing authentication, locations, and the global SSE bus.
- **Location**: A project root directory (`directory` parameter or server default) containing agent configurations, plugins, and git state.
- **Session**: State machine executing the agent loop. Sessions own transcripts, message history, active tools, and child sessions.
- **Drain / Active**: A running foreground execution loop on a session.

---

## Authentication & Connection

### 1. Connection URL & Pairing

Start the OpenCode server or pairing service:
```bash
opencode pair
# Output:
# URLs http://127.0.0.1:49374
# Username opencode
# Password <SESSION_TOKEN>
```

- **Base URL**: Typically `http://127.0.0.1:<PORT>`
- **Pairing API**:
  - `POST /api/pair`: Generates short-lived pairing code (`{ code, expires_in }`).
  - `GET /auth/connect/{code}`: Redeems pairing code. JSON requests receive `{ token }`.

### 2. Authorization Header

All protected endpoints accept HTTP Basic Auth or Bearer Token:

```http
Authorization: Basic <base64("opencode:<TOKEN>")>
```
or
```http
Authorization: Bearer <TOKEN>
```

### 3. Standard Error Envelope

Errors follow the Effect-TS encoded error shape:

```json
{
  "_tag": "SessionNotFoundError",
  "message": "Session not found"
}
```

Common status codes:
- `200` OK / `204` No Content
- `400` `InvalidRequestError` / `InvalidCursorError`
- `401` `UnauthorizedError`
- `404` `*NotFoundError` (Session, Location, Agent, Form, Permission, etc.)
- `409` `ConflictError` / `SessionBusyError`
- `503` `ServiceUnavailableError`

---

## Progressive Disclosure: Domain References

Consult domain-specific reference documents based on the task:

| Domain | Scope & Key Endpoints | Reference Doc |
|---|---|---|
| **Sessions** | Lifecycle, prompt input, message history, fork/move, reverts, inbox, instructions (44 ops) | [references/sessions.md](references/sessions.md) |
| **Agents & Models** | Agent registry, model catalog, providers, text generation, voice sessions (8 ops) | [references/agents-models.md](references/agents-models.md) |
| **Terminal & Shell** | Ephemeral PTY, Persistent PTY, background shell command execution (23 ops) | [references/terminal-execution.md](references/terminal-execution.md) |
| **Tools & Integrations** | MCP servers/resources, plugins, RPC, OAuth/integrations, permissions, forms (36 ops) | [references/tools-integrations.md](references/tools-integrations.md) |
| **Workspace & VCS** | Filesystem CRUD, project meta, worktrees, git VCS status/diff/branches, websearch (20 ops) | [references/workspace-vcs.md](references/workspace-vcs.md) |
| **Server & Config** | Server info, pairing, location reload, global config, debug, migration (11 ops) | [references/server-config.md](references/server-config.md) |
| **Events & Streaming** | Real-time Server-Sent Events (SSE) bus (`/api/event`), stream lifecycle | [references/events-streaming.md](references/events-streaming.md) |

---

## Recipes & Implementation Guides

End-to-end integration patterns:

- [examples/session-workflow.md](examples/session-workflow.md) — Create session, send prompt with file attachments, await response, and inspect transcript.
- [examples/streaming-events.md](examples/streaming-events.md) — Real-time event consumption via SSE in Node.js, Python, and cURL.
- [examples/tools-and-permissions.md](examples/tools-and-permissions.md) — Handling interactive user forms, granting tool permissions, and dynamic MCP servers.
