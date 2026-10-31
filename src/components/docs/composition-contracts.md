# Page, resource and dashboard composition

Source-relative paths below are relative to `src/components/`, not this docs directory.

## Ownership

Reusable controls, overlays, feedback, and app-level table, form, dashboard, and configuration components. Bits UI powers selected headless primitives; other families wrap native HTML or Lucide icons. Domain-aware dashboard components sit alongside otherwise prop-driven UI building blocks.

## Forms and tables

- **Table ownership**: `DataTable` receives a table-content snippet and total count, with page size defaulting to five; callers own row selection. Keep PageHead's explicit actions snippet precedence over children.

- **Form ownership**: Wrap headings/editors in `FormPage`; `active={false}` keeps read-only details in normal flow without remounting children. Keep `.form-body`/`FormActions` siblings inside native forms so only fields/notices scroll and submit/keyboard behavior stays native; dialog editors reuse this boundary within available height. Ordinary help uses `.field-hint`/`.form-hint`; safety feedback retains separate semantics.

## Resource and prompt workflows

- **Scope picker**: Select supports opt-in always-search, pinned choice, stacked path hints, and globe icons. ResourceScope persists only through config, restores confirmed selection on failure, treats native picker cancellation as a no-op, and disables management during writes, not list reads. Forgetting a folder never deletes files.

- **Prompt generation**: `PromptGenerator.svelte` shares the Agent/Command preview dialog; it reads model options only while open and replaces the local draft only after confirmation, without saving.

## Dashboard hierarchy

- **Dashboard hierarchy**: Keep runtime tiles compact/readable when switching is unavailable; the featured workbench owns current-group identity/bindings. Telemetry panels retain independent controls and panel-boundary alignment; responsive changes preserve Heatmap scrolling and roving keyboard focus.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Resource browsing | `ResourceScope.svelte`, `ResourceScopeNotice.svelte`, `GlobalDestination.svelte` | Saved root selection shared only by MCP/Agent/Skill lists; read-only details and explicit Global forms. |
| Table composition | `DataTable.svelte`, `Pagination.svelte`, `EmptyTableRow.svelte` | Table contents come from callers; pagination controls do not slice rows. |
| Page/form composition | `PageHead.svelte`, `FormPage.svelte`, `FormActions.svelte` | Snippet-based heading, available-height page frame and in-flow footer. |
| Dashboard | `dashboard/RuntimeCard.svelte`, `dashboard/SwitchGroupDialog.svelte`, `dashboard/ActivityHeatmap.svelte`, `dashboard/TokenUsageChart.svelte` | Runtime switching, activity-calendar display, and token trend visualization. |
| Configuration feedback | `configuration/Diagnostics.svelte`, `configuration/read-error.ts` | Diagnostic messages and inspection of unknown errors. |
