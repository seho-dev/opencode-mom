# Diagnostic Log Viewer

## Overview

Independent `/logs` navigation page for local diagnostic history. Settings owns the recording switch; this page can read and clear retained history while recording is disabled.

## Directory Structure

```text
logs/
├── +page.svelte          # Filters, bounded history, operation trace and confirmed deletion
├── LogDetails.svelte    # Read-only fields and lazy, bounded inline operation trace
├── log-summary.ts       # Fixed backend summary translations without altering stored records
└── __tests__/           # Viewer, setting, navigation and lifecycle regression coverage
```

## Conventions

- Use the typed config facade; keep records and filters local, never in shared recovery or persistent browser state.
- Read on entry or explicit interactions only. Do not poll, automatically upload or capture console output.
- Preserve the query's time window while consuming cursors; filter changes and refresh start new queries. Reject late responses after query changes, clear, trace close or teardown.
- Keep the displayed history page and operation traces bounded, without making older retained pages inaccessible behind a hard record cap. Explain expired cursors, unreadable lines, dropped records and retained/partial results rather than implying a complete audit trail.
- Render text as text, retain technical identifiers and timestamps, and translate display copy through both dictionaries. A successful checkpoint is not proof that its entire operation succeeded.
- Log details and trace copy write complete LogRecord objects as pretty JSON to the clipboard; trace copy exports only already-loaded retained records (currently bounded to 100 chronological events) without fetching or exporting unretrieved history. Feedback on copy success or failure is localized via toasts, without secrets enrichment or unredacted payload data.
- Confirm destructive deletion with cancel-first focus and preserve the recording preference. Keep enablement controls exclusively in Settings.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
