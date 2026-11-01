# Island connection, requests and message safety

Source-relative paths below are relative to `src/routes/island/`, not this docs directory.

## Permission consent and question inputs

- **Permission replies**: Both views share exact-ID/live-presence/mounted/connected/enabled/busy guards. Allow once is primary; the same hover-open/click-toggle disclosure offers Always for this project only with nonempty server save rules and explicit consent. OpenCode persists action plus request.save in the request's OpenCode Project, not MOM's selected folder; never derive rules from resources. Show that Project, action, exact patterns in a keyboard-scrollable read-only field and same-Project matching-pending caveat. Contain peek menus within 96 px; expanded menus stay in flow and scroll into view without focus. Hover never authorizes; dropdown Escape returns trigger focus before another Escape compacts. Success requests refresh/state readback, never optimistic removal. Pending/ambiguous replies stay locked until connected enabled read confirms removal or explicit Refresh; no automatic write retries. Permission auto-refresh never confirms uncertain message drafts.

- **Copy and metadata**: All fixed copy uses `island.*` keys in both dictionaries; backend `answerable` owns form metadata and constraint validation.

- **QuestionForm**: Multiselect options/custom values use shared Checkbox with explicit `id`/`for`, Island gold styling and busy/read-only disabled props; preserve field keys/option values. Space toggles; Enter on either checkbox never submits. Keyboard/ARIA/explicit-answer regressions live in `../../components/checkbox/__tests__/checkbox.test.ts`.

## Uncertain sends and explicit recovery

- **Uncertain send confirmation**: Only explicit Refresh confirms an uncertain attempt from a public user message with the exact submission ID. Same text/assistant IDs prove nothing; 409 is not success.

- **Missing send readback**: If Refresh finds no matching user ID, preserve the attempt for an explicit same-ID retry or offer local discard. Discard clears text/ID and never sends; a later Send is a new user action.

- **Editable rejections**: Validation/not-found and HTTP 400/401/403/404/422 rejections leave drafts editable. Editing resets the ID; unchanged manual retry retains it.

- **Write safety**: Compact running counts require a connected snapshot. Writes also require `enabled === true`. Collapse, unmount and discard never reject requests or interrupt sessions.

## Connection generation guards

- **Connection generations**: Cache-only snapshots expose monotonic `connectionGeneration`; newer generations clear selection/messages/drafts/message IDs/permission attempts/menus/keyed question forms, not view/hover/native motion/focus. Ignore older generations and late prior-generation message/action results even with matching IDs. Keep snapshot reads serialized/event-coalesced independently of per-generation serialized message reads, so stalled old messages cannot block invalidation. Never retry mutations across generations.

- **Caller generation**: Messages/send/permission or question reply/question cancellation/interruption append displayed snapshot/form generation as required `expectedConnectionGeneration`; capture before invocation, never substitute latest backend generation or replay old payloads after conflict/refresh. This guards clicks after native target switch but before new UI snapshot.

- **Resize generation**: New generations invalidate pending resize identities without changing committed view/hover; late old grow cannot open controls/focus/read messages/restore errors. Validation/conflict send rejections request cache readback only, never automatic Refresh/resend.

## Route integration

- **Island safety**: Ambiguous send/409 failures retain Session drafts/caller message IDs; retry requires explicit Refresh, never a replacement ID. Disconnected/failed snapshots show unknown, not Idle. Backend `answerable` owns metadata/constraint checks; UI accepts only string/multiselect fields with field-key/option-value answers. Collapse/disable/teardown never reply or stop sessions.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Question responses | `QuestionForm.svelte` | Field-key/option-value answers, shared Checkbox multiselect and read-only fallback. |
| Interaction coverage | `__tests__/island.test.ts` | Interactions, presentation lifecycle, send recovery, event bounds and cleanup. |
