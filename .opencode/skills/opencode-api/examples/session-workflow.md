# Session Integration Workflow

This recipe demonstrates how to create a session, send a prompt with file attachments, await completion, and inspect the resulting message transcript using cURL and Node.js.

---

## 1. Using cURL

### Step 1: Create a Session
```bash
SESSION_RESPONSE=$(curl -s -X POST http://127.0.0.1:49374/api/session \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Bug Fix Session",
    "agent": "fixer"
  }')

SESSION_ID=$(echo $SESSION_RESPONSE | jq -r '.data.id')
echo "Created session: $SESSION_ID"
```

### Step 2: Send Prompt with Attachment
```bash
curl -s -X POST "http://127.0.0.1:49374/api/session/$SESSION_ID/prompt" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Please fix the failing assertions in this test file.",
    "files": [
      {
        "uri": "file:///workspace/src/__tests__/app.test.ts",
        "name": "app.test.ts"
      }
    ],
    "resume": true
  }'
```

### Step 3: Wait for Execution Turn to Complete
```bash
# Blocks until the agent loop settles
curl -s -X POST "http://127.0.0.1:49374/api/experimental/session/$SESSION_ID/wait" \
  -H "Authorization: Bearer $TOKEN"
```

### Step 4: Retrieve Transcript Messages
```bash
curl -s "http://127.0.0.1:49374/api/session/$SESSION_ID/message" \
  -H "Authorization: Bearer $TOKEN" | jq '.data[] | { type: .type, text: .text }'
```

---

## 2. Using Node.js (TypeScript)

```typescript
const BASE_URL = 'http://127.0.0.1:49374';
const TOKEN = process.env.OPENCODE_TOKEN;

const headers = {
  'Authorization': `Bearer ${TOKEN}`,
  'Content-Type': 'application/json'
};

async function runSession() {
  // 1. Create session
  const createRes = await fetch(`${BASE_URL}/api/session`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      title: 'Automated Refactor',
      agent: 'fixer'
    })
  });
  const { data: session } = await createRes.json();
  console.log(`Session created: ${session.id}`);

  // 2. Dispatch prompt
  await fetch(`${BASE_URL}/api/session/${session.id}/prompt`, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      text: 'Run tests and fix any failing specs.',
      resume: true
    })
  });

  // 3. Wait for agent loop to settle
  await fetch(`${BASE_URL}/api/experimental/session/${session.id}/wait`, {
    method: 'POST',
    headers
  });

  // 4. Read final messages
  const msgRes = await fetch(`${BASE_URL}/api/session/${session.id}/message`, {
    headers
  });
  const { data: messages } = await msgRes.json();
  const lastMessage = messages[messages.length - 1];
  console.log('Final response:', lastMessage);
}

runSession().catch(console.error);
```
