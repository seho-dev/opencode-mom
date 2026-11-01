# Agents & Models API Reference

This reference covers agent discovery, model catalogs, provider connections, and direct generation services (8 endpoints).

## 1. Agents

Agents are specialized personas defined with specific models, prompts, tools, and permissions (e.g. `explorer`, `fixer`, `designer`, `oracle`).

### `GET /api/agent`
List all registered agents available at the location.
- **Operation ID**: `agent.list`
- **Query Parameters**:
  - `location.directory` (string, optional): Target project directory.
- **Response**: `200`
  ```json
  {
    "location": { "directory": "/Users/..." },
    "data": [
      {
        "id": "fixer",
        "name": "Fixer",
        "description": "Fast implementation specialist",
        "model": { "providerID": "anthropic", "modelID": "claude-3-7-sonnet" },
        "system": "Agent prompt instructions...",
        "mode": "primary"
      }
    ]
  }
  ```

### `GET /api/agent/{agentID}`
Retrieve full configuration and metadata for a specific agent.
- **Operation ID**: `agent.get`
- **Parameters**: `agentID` (path), `location.directory` (query)
- **Response**: `200` `{ location: Location.PublicRef, data: Agent.Info }`
- **Errors**: `404 AgentNotFoundError`

---

## 2. Models & Providers

### `GET /api/model`
List all configured LLM models across active providers.
- **Operation ID**: `model.list`
- **Response**: `200`
  ```json
  {
    "data": [
      {
        "id": "anthropic/claude-3-7-sonnet",
        "providerID": "anthropic",
        "modelID": "claude-3-7-sonnet",
        "family": "claude",
        "name": "Claude 3.7 Sonnet",
        "capabilities": {
          "reasoning": true,
          "tools": true
        },
        "limits": {
          "context": 200000,
          "output": 8192
        }
      }
    ]
  }
  ```

### `GET /api/model/default`
Retrieve the current default model configured for the instance.
- **Operation ID**: `model.default`
- **Response**: `200` `{ data: Model.Info }`

### `GET /api/provider`
List all supported and active AI providers (OpenAI, Anthropic, Ollama, Google, OpenCode Go, etc.).
- **Operation ID**: `provider.list`
- **Response**: `200` `{ data: Provider.Info[] }`

### `GET /api/provider/{providerID}`
Retrieve single provider details, credentials status, and activation mode.
- **Operation ID**: `provider.get`
- **Response**: `200` `{ data: Provider.Info }`

---

## 3. Direct Generation & Voice

### `POST /api/experimental/generate`
Invoke model completion directly without creating a long-lived session.
- **Operation ID**: `experimental.generate.text`
- **Request Body**:
  ```json
  {
    "model": { "providerID": "anthropic", "modelID": "claude-3-7-sonnet" },
    "system": "You are a concise code generator.",
    "prompt": "Write a bash script to test network ports."
  }
  ```
- **Response**: `200` `{ text: string, usage: { input: number, output: number } }`

### `POST /api/experimental/voice/session`
Initialize a real-time bidirectional voice conversation session.
- **Operation ID**: `experimental.voice.session`
- **Response**: `200` Voice session credentials and WebRTC / WebSocket connection endpoints.
