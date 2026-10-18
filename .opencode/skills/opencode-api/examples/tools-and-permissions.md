# Tools, Permissions & Forms Recipe

This recipe demonstrates approving pending permission requests, answering interactive user forms, and registering dynamic MCP servers over HTTP.

---

## 1. Handling Permission Requests

When an agent needs to execute an unprivileged tool (e.g. running destructive shell commands or writing sensitive files), it emits a `permission.requested` event and waits.

### Step 1: Query Pending Permissions
```bash
curl -s "http://127.0.0.1:49374/api/permission/request" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### Step 2: Approve or Reject Permission
```bash
SESSION_ID="ses_123"
REQUEST_ID="perm_456"

# Grant permission once:
curl -s -X POST "http://127.0.0.1:49374/api/session/$SESSION_ID/permission/$REQUEST_ID/reply" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "decision": "once",
    "message": "Approved execution"
  }'

# Or reject permission:
# { "decision": "reject", "message": "Disallowed by security policy" }
```

---

## 2. Interactive Forms

When an agent needs structured input or choice from the user:

### Step 1: Query Session Forms
```bash
curl -s "http://127.0.0.1:49374/api/session/$SESSION_ID/form" \
  -H "Authorization: Bearer $TOKEN" | jq .
```

### Step 2: Submit Form Answers
```bash
FORM_ID="form_789"

curl -s -X POST "http://127.0.0.1:49374/api/session/$SESSION_ID/form/$FORM_ID/reply" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "answers": ["Option A", "production-cluster"]
  }'
```

---

## 3. Dynamically Adding an MCP Server

Register an external MCP server to supply custom tools and resources to all sessions:

```bash
# Add a local Postgres MCP server
curl -s -X PUT "http://127.0.0.1:49374/api/experimental/mcp/postgres" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": {
        "DATABASE_URL": "postgresql://user:pass@localhost:5432/mydb"
      }
    }
  }'

# Query active MCP catalog to verify tools
curl -s "http://127.0.0.1:49374/api/mcp" \
  -H "Authorization: Bearer $TOKEN" | jq .
```
