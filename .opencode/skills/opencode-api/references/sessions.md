# Sessions API Reference

The Session API provides 44 endpoints controlling the agent execution engine, transcript history, message dispatch, branching, and interactive boundaries.

## Core Concepts

- **Session**: The execution container running the agent loop (`Session.ID` format: `ses_...` or similar).
- **Session Drain**: The active background/foreground runner driving model turns and tool executions.
- **Delivery Modes**: Defines when an input is queued (`queue` for next turn idle, `steer` to intervene at next tool step boundary).
- **Turn & Context**: Each turn accumulates user messages, assistant tool calls, tool responses, and compaction boundaries.

---

## 1. Session Lifecycle

### `GET /api/session`
List sessions with ordering, search, pagination, and directory filtering.
- **Operation ID**: `session.list`
- **Query Parameters**:
  - `limit` (string, default: 50): Page size.
  - `order` (`"asc" | "desc"`): Newest first (`desc`) or oldest first (`asc`).
  - `search` (string): Search query in titles/messages.
  - `parentID` (`Session.ID | "null"`): Filter child sessions, or pass `"null"` for root sessions only.
  - `directory` (AbsolutePath): Filter by location directory.
  - `project` (`Project.ID`): Filter by project.
  - `subpath` (RelativePath): Relative path filter.
  - `cursor` (`SessionsCursor`): Pagination cursor (`cursor.next` / `cursor.previous`).
- **Response**: `200` `SessionsResponse` (`{ data: Session.Info[], cursor?: SessionsCursor }`)

### `POST /api/session`
Create a new session or linked child session.
- **Operation ID**: `session.create`
- **Request Body**:
  ```json
  {
    "id": "optional-custom-id",
    "parentID": "parent-session-id-for-subagent",
    "title": "Feature implementation",
    "agent": "fixer",
    "model": { "providerID": "anthropic", "modelID": "claude-3-7-sonnet" },
    "location": { "directory": "/absolute/path/to/project" },
    "metadata": { "custom": "field" },
    "permissions": []
  }
  ```
- **Response**: `200` `{ data: Session.Info }`

### `GET /api/session/{sessionID}`
Retrieve session details by ID.
- **Operation ID**: `session.get`
- **Response**: `200` `{ data: Session.Info }`

### `PATCH /api/session/{sessionID}`
Update mutable session properties (title, metadata, permissions).
- **Operation ID**: `session.update`
- **Request Body**: `{ title?, metadata?, permissions? }`
- **Response**: `204 No Content`

### `DELETE /api/session/{sessionID}`
Delete a session and all its child sessions.
- **Operation ID**: `session.remove`
- **Response**: `204 No Content`

### `GET /api/session/active`
List active foreground session drains currently running in the server.
- **Operation ID**: `session.active`
- **Response**: `200` `{ data: Record<SessionID, SessionActive> }`

---

## 2. Interaction & Messaging

### `POST /api/session/{sessionID}/prompt`
Send a user message, attach files/agents/skills, and schedule agent-loop execution.
- **Operation ID**: `session.prompt`
- **Request Body**:
  ```json
  {
    "text": "Please analyze this component and fix test failures",
    "files": [
      {
        "uri": "file:///path/to/Component.svelte",
        "name": "Component.svelte"
      }
    ],
    "agents": [{ "agent": "explorer" }],
    "skills": [{ "id": "best-practices" }],
    "delivery": "queue",
    "resume": true
  }
  ```
- **Parameters**:
  - `delivery`: `"queue"` (wait until current step idle) or `"steer"` (interrupt at next tool boundary).
  - `resume`: boolean (defaults to `true`). If `false`, enqueues message without launching execution.
- **Response**: `200` `{ data: Session.Inbox.User }`

### `POST /api/session/{sessionID}/command`
Execute a slash command callback immediately.
- **Operation ID**: `session.command`
- **Request Body**: `{ name: string, text: string, files?, agents?, skills?, delivery? }`
- **Response**: `204 No Content`

### `POST /api/session/{sessionID}/synthetic`
Append a synthetic/system-generated turn (e.g. tool output inject or background task notification).
- **Operation ID**: `session.synthetic`
- **Request Body**: `{ text: string, description?: string, metadata?: object, delivery?, resume? }`
- **Response**: `200` `{ data: Session.Inbox.Synthetic }`

### `POST /api/session/{sessionID}/shell`
Run an isolated shell command in the session's working directory. Emits `shell.started` and `shell.ended`.
- **Operation ID**: `session.shell`
- **Request Body**: `{ command: string }`
- **Response**: `204 No Content`

### `POST /api/experimental/session/{sessionID}/skill`
Activate a skill by appending a skill message and resuming execution.
- **Operation ID**: `experimental.session.skill`
- **Request Body**: `{ id: Skill.ID, resume?: boolean }`
- **Response**: `204 No Content`

---

## 3. Execution Control & State Switching

| Method & Path | Operation ID | Description & Body |
|---|---|---|
| `POST /api/session/{sessionID}/agent` | `session.switchAgent` | Switch primary agent: `{ "agent": "Agent.ID" }` |
| `POST /api/session/{sessionID}/model` | `session.switchModel` | Switch active model: `{ "model": { "providerID": "...", "modelID": "..." } }` |
| `POST /api/session/{sessionID}/interrupt` | `session.interrupt` | Interrupt currently running execution loop |
| `POST /api/session/{sessionID}/background` | `session.background` | Move blocking tools to background execution |
| `POST /api/session/{sessionID}/compact` | `session.compact` | Trigger transcript compaction to manage context size |
| `POST /api/experimental/session/{sessionID}/wait` | `experimental.session.wait` | Block HTTP response until current session turn settles |
| `POST /api/session/{sessionID}/fork` | `session.fork` | Fork session before a specific message: `{ "before": "MessageID" }` |
| `POST /api/session/{sessionID}/move` | `session.move` | Move session to new directory: `{ "directory": AbsolutePath }` |
| `PUT /api/session/{sessionID}/environment` | `session.environment` | Update session environment variables |
| `POST /api/session/{sessionID}/view` | `session.view` | Update client view state on session |

---

## 4. History, Messages & Context

### `GET /api/session/{sessionID}/message`
Get all transcript messages for a session.
- **Operation ID**: `session.message.list`
- **Response**: `200` `{ data: Session.Message.Info[] }`
- **Message Types**:
  - `User`: user text prompt and attachments.
  - `AgentSelected` / `ModelSelected`: routing switches.
  - `Synthetic`: synthesized system inputs.
  - `Assistant`: agent response parts (text, tool calls, tool results).

### `GET /api/session/{sessionID}/message/{messageID}`
Retrieve a single message by ID.
- **Operation ID**: `session.message.get`

### `GET /api/session/{sessionID}/context`
Inspect compiled session context sent to LLM (token counts, active instructions, tools).
- **Operation ID**: `session.context`

### `GET /api/session/{sessionID}/diff`
Return Git/VCS diffs produced across session turns.
- **Operation ID**: `session.diff`

### `GET /api/experimental/session/{sessionID}/log`
Stream or read internal session debug log entries.
- **Operation ID**: `session.log`

### `GET /api/experimental/session/stats`
Aggregate usage, token consumption, and tool reliability metrics.
- **Operation ID**: `experimental.session.stats`
- **Query**: `from`, `to`, `project`, `timezone`, `tools` (`"none" | "summary" | "detail"`)

---

## 5. Inbox & Dynamic Instructions

The Inbox stores uncommitted or queued inputs.
- `GET /api/session/{sessionID}/inbox`: List pending queue items.
- `PATCH /api/session/{sessionID}/inbox/{inboxID}`: Update queued item text or attachments.
- `DELETE /api/session/{sessionID}/inbox/{inboxID}`: Cancel/remove queued item before execution.

Dynamic Instructions:
- `GET /api/experimental/session/{sessionID}/instructions/entries`: List active instruction entries.
- `PUT /api/experimental/session/{sessionID}/instructions/entries/{key}`: Set dynamic prompt instruction.
- `DELETE /api/experimental/session/{sessionID}/instructions/entries/{key}`: Evict dynamic instruction.

---

## 6. Interactive Boundaries: Forms & Permissions

When an agent needs human confirmation or input:

### Interactive Forms
- `GET /api/session/{sessionID}/form`: List active forms.
- `POST /api/session/{sessionID}/form`: Create form request.
- `GET /api/session/{sessionID}/form/{formID}`: Get form schema/fields.
- `POST /api/session/{sessionID}/form/{formID}/reply`: Submit answers `{ answers: [...] }`.
- `DELETE /api/session/{sessionID}/form/{formID}`: Cancel form.

### Session Permissions
- `GET /api/session/{sessionID}/permission`: List pending permission requests.
- `POST /api/session/{sessionID}/permission`: Request permission for a tool/file action.
- `GET /api/session/{sessionID}/permission/{requestID}`: Get permission request details.
- `POST /api/session/{sessionID}/permission/{requestID}/reply`: Grant or deny (`{ reply: "once" | "always" | "reject" }`).

---

## 7. Revert & Transcripts

Staged revert allows rolling back files and transcript to a previous turn:
- `POST /api/session/{sessionID}/revert/stage`: Preview rollback to message ID.
- `DELETE /api/session/{sessionID}/revert`: Discard staged revert.
- `POST /api/session/{sessionID}/revert/commit`: Apply revert to disk and history.

Transcripts:
- `GET /api/experimental/session/{sessionID}/export?sanitize=true|false`: Export complete transferable session JSON.
- `POST /api/experimental/session/import`: Import session transcript at target location.
