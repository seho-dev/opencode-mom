# Mini Island

## Overview

The independent native Island window presents compact attention, session browsing, and guarded request actions without global preload. Native geometry owns window shape; messages load only when expanded.

## Directory Structure

```text
island/
├── +page.svelte                 # Three-shape notch island, CSS content motion, quick permission queue, lazy sessions
├── QuestionForm.svelte          # Field-key/option-value answers, shared Checkbox multiselect and read-only fallback
└── __tests__/
    ├── island.test.ts           # Interactions, presentation lifecycle, send recovery, event bounds and cleanup
    └── layout.test.ts           # Independent window without global preload
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Window controller | `+page.svelte` | View transitions, snapshots, and guarded actions. |
| Question inputs | `QuestionForm.svelte` | Field-key responses and accessible multiselect. |
| Interaction checks | `__tests__/island.test.ts` | Lifecycle, recovery, and event-boundary coverage. |
| Window isolation | `__tests__/layout.test.ts` | No global configuration preload. |

## Conventions

- **Native geometry**: Start and collapse compact on every screen; use backend geometry, not device guesses.
  Follow [presentation contracts](docs/presentation.md#geometry-density-and-attention) for density, safe gaps,
  and unavailable counts.

- **View ownership**: Only explicit disclosure opens sessions; hover may reveal attention but never authorizes
  actions. Preserve [transition ownership](docs/presentation.md#css-and-appkit-frame-transitions) and
  authoritative collapse without restoring obsolete controls.

- **Permission consent**: Reply only to live, exact requests with explicit consent; project persistence uses
  server-provided rules. Preserve [permission and question
  contracts](docs/connection-lifecycle.md#permission-consent-and-question-inputs), never optimistic removal.

- **Send recovery**: Keep uncertain drafts and submission IDs until explicit readback confirms them; never
  infer success or resend automatically. Follow [recovery
  contracts](docs/connection-lifecycle.md#uncertain-sends-and-explicit-recovery).

- **Connection generations**: Bind reads and mutations to the displayed connection generation; invalidate old
  drafts, actions, and resize results. Follow [generation
  guards](docs/connection-lifecycle.md#connection-generation-guards) without cross-target retries.

- **Event lifecycle**: Coalesce snapshot invalidations and clean listeners, hover state, and timers on
  teardown. Preserve [window event contracts](docs/presentation.md#window-events-and-manual-acceptance),
  including late registrations and separate hover sources.

- **Accessible responses**: Keep translated status and accessible counts; preserve backend-owned answerability
  and exact field/option identities. Use the shared Checkbox and [question-input
  rules](docs/connection-lifecycle.md#permission-consent-and-question-inputs).

- **Manual acceptance**: Automated checks cannot prove native motion, focus, seams, or camera-area behavior.
  Perform the rebuilt-app [acceptance checks](docs/presentation.md#window-events-and-manual-acceptance) on
  notched and non-notched screens.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
