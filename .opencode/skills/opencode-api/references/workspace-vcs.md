# Workspace & VCS API Reference

This reference covers direct filesystem operations, projects, Git worktrees, VCS diffs, slash commands, references, and web search (20 endpoints).

---

## 1. Filesystem (4 endpoints)

Direct file operations scoped to server-managed locations.

- `GET /api/fs/read/{path...}` (`fs.read`): Read file contents at relative path.
  - Query: `location.directory` (optional)
  - Response: File stream / text content.
- `GET /api/fs/list` (`fs.list`): List directory entries.
  - Query: `path`, `location.directory`, `recursive` (boolean)
- `GET /api/fs/find` (`fs.find`): Fast pattern search across project files.
  - Query: `pattern`, `directory`, `limit`
- `POST /api/experimental/fs/write` (`experimental.fs.write`): Write binary or text stream directly to a file path.
  - Headers: `Content-Type: application/octet-stream`
  - Query: `path`, `location.directory`

---

## 2. Projects & Worktrees (6 endpoints)

Manage project workspaces and parallel Git worktree lanes.

### Projects
- `GET /api/project` (`project.list`): List known project roots, workspace settings, and active locations.
- `PATCH /api/project/{projectID}` (`project.update`): Update project display name or workspace configurations.

### Worktrees
Worktrees provide isolated workspace directories for concurrent subagents or task branches without dirtying the main working tree.
- `GET /api/worktree` (`worktree.list`): List active worktrees for the current project.
- `POST /api/worktree` (`worktree.create`): Create a new isolated Git worktree.
  - Body:
    ```json
    {
      "projectID": "project-id",
      "branch": "feature/refactor-api",
      "name": "refactor-lane",
      "from": "/path/to/main/repo",
      "directory": "/path/to/worktree/directory"
    }
    ```
- `DELETE /api/worktree` (`worktree.remove`): Clean up and prune a worktree directory.
- `POST /api/worktree/refresh` (`worktree.refresh`): Sync git metadata across active worktrees.

---

## 3. Version Control (VCS / Git) (6 endpoints)

Read Git commit graph, branches, working tree status, and staging diffs without invoking shell commands.

- `POST /api/vcs/init` (`vcs.init`): Initialize a Git repository at the current location.
- `GET /api/vcs` (`vcs.get`): Get repo status, remote URLs, active branch, and HEAD commit SHA.
- `GET /api/vcs/base` (`vcs.base`): Detect merge-base or review base commit against default branch.
- `GET /api/vcs/status` (`vcs.status`): Get staged, unstaged, and untracked file status list.
- `GET /api/vcs/branch` (`vcs.branch.list`): List local and remote branches.
- `GET /api/vcs/diff` (`vcs.diff`): Retrieve unified diff between commits or against working directory.

---

## 4. Commands, References & Web Search (4 endpoints)

- `GET /api/command` (`command.list`): List registered slash commands (e.g. `/review`, `/test`, custom user commands).
- `GET /api/reference` (`reference.list`): List referenced context files, bookmarks, or docs.
- `GET /api/websearch/provider` (`websearch.providers`): List available web search providers (Brave, Tavily, Google, etc.).
- `POST /api/websearch` (`websearch.query`): Execute web search query (`{ "query": string }`).
