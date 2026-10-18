# Streaming Events Recipe (SSE)

This recipe shows how to subscribe to `/api/event` and process real-time token streams, tool events, and execution notifications.

---

## 1. Using cURL

```bash
curl -N -H "Authorization: Bearer $TOKEN" \
        -H "Accept: text/event-stream" \
        http://127.0.0.1:49374/api/event
```

---

## 2. Using Node.js

```javascript
const BASE_URL = 'http://127.0.0.1:49374';
const TOKEN = process.env.OPENCODE_TOKEN;

async function streamEvents(targetSessionID) {
  const response = await fetch(`${BASE_URL}/api/event`, {
    headers: {
      'Authorization': `Bearer ${TOKEN}`,
      'Accept': 'text/event-stream',
      'Cache-Control': 'no-cache'
    }
  });

  if (!response.ok) {
    throw new Error(`SSE failed with status: ${response.status}`);
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n\n');
    buffer = lines.pop() || '';

    for (const frame of lines) {
      if (!frame.trim()) continue;

      let eventName = 'message';
      let eventData = '';

      for (const line of frame.split('\n')) {
        if (line.startsWith('event: ')) {
          eventName = line.slice(7).trim();
        } else if (line.startsWith('data: ')) {
          eventData = line.slice(6).trim();
        }
      }

      if (!eventData) continue;

      try {
        const payload = JSON.parse(eventData);

        // Filter events for the target session if specified
        if (targetSessionID && payload.sessionID && payload.sessionID !== targetSessionID) {
          continue;
        }

        switch (eventName) {
          case 'message.part':
            // Incremental token delta
            process.stdout.write(payload.delta || '');
            break;

          case 'tool.invoked':
            console.log(`\n[TOOL] Executing ${payload.toolName}...`);
            break;

          case 'permission.requested':
            console.log(`\n[AUTH] Tool ${payload.toolName} requires user approval!`);
            break;

          case 'message.updated':
            console.log('\n[DONE] Message finished.');
            break;

          case 'effect/http-api/stream/failure':
            console.error('\n[STREAM ERROR]', payload);
            break;
        }
      } catch (err) {
        console.error('Failed to parse SSE payload:', err, eventData);
      }
    }
  }
}

streamEvents().catch(console.error);
```
