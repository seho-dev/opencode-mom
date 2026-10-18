# Events & Streaming API Reference

OpenCode v2 exposes a unified Server-Sent Events (SSE) stream for real-time notifications across all sessions, tools, and background processes.

---

## 1. SSE Connection

### `GET /api/event`
Subscribe to native events and plugin RPC events across all server locations.
- **Operation ID**: `event.subscribe`
- **Request Headers**:
  ```http
  GET /api/event HTTP/1.1
  Host: 127.0.0.1:49374
  Authorization: Bearer <TOKEN>
  Accept: text/event-stream
  Cache-Control: no-cache
  ```
- **Response**: `200 text/event-stream`

---

## 2. Event Stream Contract & Resilience

- **Volatile Stream**: OpenCode's event bus does not store an infinite replay backlog. Slow consumers that fail to drain the socket will overflow the server buffer and trigger a stream disconnect.
- **Disconnection Handling**: Events emitted during network disconnections are dropped. Clients must query REST endpoints (e.g. `GET /api/session/{id}/message`, `GET /api/session/active`) to reconcile state after reconnecting.
- **Stream Failure Event**:
  ```http
  event: effect/http-api/stream/failure
  data: { "_tag": "Fail", "error": { ... } }
  ```

---

## 3. SSE Frame Structure

Every SSE frame emitted follows standard SSE formatting:

```http
id: 1042
event: message.part
data: {"sessionID":"ses_123","messageID":"msg_456","delta":"function calculateTotal"}
```

---

## 4. Primary Event Categories

### Session & Transcript Events
- `session.created`: Emitted when a new session or subagent is created.
- `session.updated`: Emitted on title, agent, model, or metadata change.
- `session.removed`: Emitted when a session is deleted.
- `message.created`: New turn or message initiated.
- `message.part`: Incremental token delta streamed from the LLM.
- `message.updated`: Message finished or updated with full content.

### Tool Execution Events
- `tool.invoked`: Agent initiated a tool call with arguments.
- `tool.progress`: Long-running tool reported progress.
- `tool.completed`: Tool returned execution results.
- `tool.failed`: Tool call failed or produced an error.

### Interactive Boundaries
- `permission.requested`: Agent blocked awaiting human permission approval.
- `permission.resolved`: Permission granted or rejected.
- `form.requested`: Agent requested interactive form input.
- `form.resolved`: Form answered or cancelled.

### Shell & System Events
- `shell.started`: Background command started (`{ id, command, cwd }`).
- `shell.ended`: Background command settled (`{ id, code, stdout, stderr }`).
- `location.shutdown`: Triggered during `POST /api/location/reload` notifying clients to rebuild local caches.
