# Server & Configuration API Reference

This reference covers server metadata, authentication pairing, location reloads, global configuration, debug diagnostics, and v1 migration status (11 endpoints).

---

## 1. Server Identity & Pairing (3 endpoints)

### `GET /api/info`
Return the server daemon identity, version, PID, connection URLs, temporary paths, and host capabilities.
- **Operation ID**: `server.info`
- **Response**: `200`
  ```json
  {
    "version": "2.0.0",
    "pid": 48291,
    "urls": ["http://127.0.0.1:49374"],
    "paths": { "tmp": "/tmp/opencode" },
    "capabilities": {
      "persistentPty": true
    }
  }
  ```

### `POST /api/pair`
Generate a short-lived, single-use pairing code for web/client authentication.
- **Operation ID**: `server.pair`
- **Response**: `200` `{ "code": "abc123xyz", "expires_in": 300 }`

### `GET /auth/connect/{code}`
Redeem a pairing code to establish a session.
- **Operation ID**: `server.connect`
- **Behavior**:
  - Browser requests: Sets session cookie and redirects to web console.
  - API / JSON requests (`Accept: application/json`): Returns `{ "token": "session_auth_token" }` to be used as HTTP Basic Auth password or Bearer token.

---

## 2. Location Management (4 endpoints)

A Location represents a project directory runtime environment.

### `GET /api/location`
Resolve the requested location directory or return the server default location.
- **Operation ID**: `location.get`
- **Query**: `location.directory` (optional)
- **Response**: `200` `{ "directory": "/absolute/path", "project": "project-id" }`

### `POST /api/location/reload`
Hot-reload configuration for all active locations.
- **Operation ID**: `location.reload`
- **Behavior**: Pending permissions and forms are cancelled; active session drains pick up new services at the next step boundary.
- **Emits**: `location.shutdown` SSE event for client recovery.
- **Response**: `204 No Content`

### Debug Locations
- `GET /api/debug/location` (`debug.location.list`): List all locations currently loaded in server memory.
- `DELETE /api/debug/location` (`debug.location.evict`): Evict a stale or unreferenced location from memory (`?directory=...`).

---

## 3. Configuration & System (4 endpoints)

### `GET /api/config`
Retrieve parsed active configuration (merged from defaults, user config, project config, and environment).
- **Operation ID**: `config.get`
- **Response**: `200` Merged config object.

### `GET /api/config/shell`
List shells detected and available on the host system (bash, zsh, fish, pwsh, cmd).
- **Operation ID**: `config.shells`
- **Response**: `200` `{ data: string[] }`

### `PATCH /api/experimental/config`
Update global configuration in memory and on disk.
- **Operation ID**: `experimental.config.update`
- **Request Body**: Partial config patch.
- **Response**: `200`

### `GET /api/experimental/migration/v1`
Query migration status, orphaned v1 configs, and compatibility notes.
- **Operation ID**: `experimental.migration.v1.status`
- **Response**: `200` Migration report.
