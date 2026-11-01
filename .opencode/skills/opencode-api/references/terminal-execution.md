# Terminal & Shell Execution API Reference

OpenCode v2 provides three distinct execution mechanisms for commands and terminals:
1. **Ephemeral PTY**: Full interactive pseudo-terminals over WebSockets.
2. **Persistent PTY**: Long-lived, resumable terminals bound to sessions with snapshotting and handoff.
3. **Background Shells**: Detached, non-interactive shell commands with output buffering.

---

## 1. Ephemeral PTY (7 endpoints)

Interactive PTY instances suitable for TUI apps, real-time command streaming, or terminal emulators.

### Endpoints
- `GET /api/pty` (`pty.list`): List running PTY processes.
- `POST /api/pty` (`pty.create`): Create a new PTY session.
  - Body:
    ```json
    {
      "command": "bash",
      "args": ["-l"],
      "cwd": "/project/path",
      "title": "Build terminal",
      "env": { "TERM": "xterm-256color" }
    }
    ```
  - Response: `200` `{ data: Pty.Info }`
- `GET /api/pty/{ptyID}` (`pty.get`): Get PTY status, PID, and dimensions.
- `PUT /api/pty/{ptyID}` (`pty.update`): Resize terminal (`{ cols: number, rows: number }`).
- `DELETE /api/pty/{ptyID}` (`pty.remove`): Terminate the PTY process.
- `POST /api/pty/{ptyID}/connect-token` (`pty.connect.token`): Generate a single-use WebSocket token.
- `GET /api/pty/{ptyID}/connect` (`pty.connect`): Connect to the interactive bidirectional terminal stream via WebSocket (pass `token` query parameter).

---

## 2. Persistent PTY (11 endpoints)

Persistent PTYs survive disconnects, persist screen state/scrollback, and allow seamless handoff between the user and AI agent loops.

### Session-Scoped Persistent Terminals
- `GET /api/experimental/session/{sessionID}/terminal/read` (`server.experimental.persistentPty.read`): Read output/screen of the session's active terminal.
- `GET /api/experimental/session/{sessionID}/terminal` (`server.experimental.persistentPty.list`): List terminals assigned to a session.
- `POST /api/experimental/session/{sessionID}/terminal` (`server.experimental.persistentPty.create`): Spawn a new persistent PTY attached to the session.

### Global Persistent Terminal Lifecycle
- `GET /api/experimental/persistent-pty/{ptyID}` (`server.experimental.persistentPty.get`): Get status and terminal properties.
- `PUT /api/experimental/persistent-pty/{ptyID}` (`server.experimental.persistentPty.update`): Resize or reconfigure.
- `DELETE /api/experimental/persistent-pty/{ptyID}` (`server.experimental.persistentPty.remove`): Close and delete terminal.
- `GET /api/experimental/persistent-pty/{ptyID}/snapshot` (`server.experimental.persistentPty.snapshot`): Capture current terminal screen buffer and cursor position.
- `POST /api/experimental/persistent-pty/shutdown` (`server.experimental.persistentPty.shutdown`): Terminate background persistent daemon.
- `POST /api/experimental/persistent-pty/handoff` (`server.experimental.persistentPty.handoff`): Transfer terminal control between human and agent.
- `POST /api/experimental/persistent-pty/{ptyID}/connect-token` (`server.experimental.persistentPty.connectToken`): Get WebSocket authentication token.
- `GET /api/experimental/persistent-pty/{ptyID}/connect` (`persistentPty.connect`): Stream persistent terminal via WebSocket.

---

## 3. Background Shell Commands (5 endpoints)

Non-interactive process runner with output accumulation and status polling.

### Endpoints
- `GET /api/shell` (`shell.list`): List background command executions.
- `POST /api/shell` (`shell.create`): Start a background shell command.
  - Body:
    ```json
    {
      "command": "npm run test:unit",
      "cwd": "/project/path",
      "timeout": 30000,
      "metadata": { "lane": "test" }
    }
    ```
  - Response: `200` `{ data: Shell.Info }`
- `GET /api/shell/{id}` (`shell.get`): Check process exit code, status, and execution duration.
- `GET /api/shell/{id}/output` (`shell.output`): Read stdout/stderr output lines generated so far.
- `DELETE /api/shell/{id}` (`shell.remove`): Kill running process or purge completed command from buffer.
