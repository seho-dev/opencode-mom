# CLI upgrades and local service targets

Source-relative paths below are relative to `src/routes/settings/` unless a reference table names another scope.

## Explicit local CLI checks and upgrades

- **CLI inspection**: OpenCode CLI inspection first confirms native app info; browser mode makes no CLI or update calls. Mount and refresh are local reads only; release checks require a click.

- **Update controls**: App check/retry, CLI refresh/check, inspection retry and connection refresh are icon-only with translated accessible names/title tooltips. CLI refresh/check share the updates header; installation path/current version appear only in upgrade confirmation.

- **Upgrade confirmation**: Only checked, upgradeable installations expose Upgrade. The Bits UI confirmation snapshots the exact path, current version, and target version for the guarded command.

- **Cross-panel guards**: CLI pending work disables connection controls; service reads/saves disable CLI actions/prevent confirmation. Invalidate reads on teardown: no late readback/retries/toasts. Errors retain confirmed CLI details and require explicit refresh before upgrade retry.

- **Upgrade completion**: Success rereads installation/service once while active, without global refresh/service restart; inactive completion retains refresh-required state without readback. Keep CLI feedback separate from backend version. Unsupported installations show no metadata/manual command; only explicit available-update results add short manual hints. CLI errors use translated hints, never raw backend messages.

## Service drafts and credential safety

- **Service drafts**: Mount/explicit Refresh probe only confirmed target without bootstrap and preserve URL/password/clear-password drafts. Failed reads never show stale Connected/PID/version; failed saves preserve confirmed target/drafts, while successful offline saves commit returned normalized target.

- **Passwords**: Never read back secrets; clear on save success/teardown. Blank omits password, explicit clear sends empty string, Shared sends null URL. Describe direct V2 loopback APIs/Basic backend passwords, never OpenChamber UI URLs/passwords or Bearer fallback.

- **Service errors**: Show only translated generic or `busy`/`validation_failed` hints, never raw message/detail/status.error or credential logs. Busy asks wait/retry; validation mentions only direct HTTP loopback URL/positive port/backend password. Never require Mini Island/lid disabling to change targets: backend owns monitor refresh/validation with enabled preferences preserved; settings never toggles monitors/power policy for saves. Credential copy states plaintext/non-encrypted storage and never claims Windows ACL validation before native verification.

## Route CLI and target integration

- **CLI updates**: Keep `OpenCodeSettings` separate from app updates. Native mount confirms app info, reads installation and independently probes the saved service without bootstrap; browsers make no calls. Keep metadata in background state; group accessible icon-only local refresh/check in the update header, with icon-only inspection retry/service refresh. Explicit checks own installation/version results. Upgrade needs `canUpgrade`, an available checked update and exact path/version confirmation only in the dialog; CLI/service pending work guards both panels. Failure preserves known details and requires explicit refresh before check/upgrade; success reads installation/service once without reload/restart.

- **Service target**: `OpenCodeService` retains local URL/password/clear drafts across refresh/failure and commits only successful saves, including offline targets. Never read back passwords; clear them on success/teardown. Keep saved target/connected URL/PID/backend version separate from draft/CLI version; read/status errors cannot advertise stale connections. Show generic/code-based busy/validation hints, never backend bodies/credentials.

## Implementation references

Paths relative to `src/routes/settings/`:

| Responsibility | File | Description |
| --- | --- | --- |
| OpenCode CLI | `OpenCodeSettings.svelte` | Local CLI inspection, explicit checks, upgrade confirmation/readback. |
| Service target | `OpenCodeService.svelte` | Saved target, read-only connection probes, secret-local drafts. |

Paths relative to `src/routes/`:

| Responsibility | File | Description |
| --- | --- | --- |
| OpenCode CLI | `settings/OpenCodeSettings.svelte` | Reads native CLI details locally; checks stable updates explicitly and confirms exact upgrade path/versions. |
| Service target | `settings/OpenCodeService.svelte` | Lazy local target probes, draft-preserving saves, safe errors, and independent backend version. |
