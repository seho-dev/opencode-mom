# Settings

## Overview

`MiniIsland.svelte` owns the independent macOS-only Island enable switch. It reads `getIslandState`, writes `setIslandEnabled`, uses native readback, preserves confirmed state on errors, and does not expose the switch in browser or non-macOS modes.

`OpenCodeSettings.svelte` owns background local CLI inspection, explicit stable-version checks, and guarded upgrades. Settings show only CLI updates, not a standalone installation-details panel. App updates remain in `AppUpdates.svelte`; CLI metadata is not part of the global config snapshot.

`OpenCodeService.svelte` owns the saved Shared/fixed local V2 target, local connection probes, and component-local password drafts. Service credentials never join the config snapshot or draft recovery.

## Directory Structure

```text
settings/
├── +page.svelte                   # Autostart and independent settings sections
├── LidProtection.svelte           # Native lid-protection lifecycle
├── AppUpdates.svelte              # Explicit update checks and project pages
├── MiniIsland.svelte              # macOS-only Island state and enable readback
├── OpenCodeSettings.svelte        # Local CLI inspection, explicit checks, upgrade confirmation/readback
├── OpenCodeService.svelte         # Saved target, read-only connection probes, secret-local drafts
└── __tests__/                     # Settings lifecycle coverage
    ├── lid-protection.test.ts     # Deferred target reloads, 2s fake-timer polling, stale read/save and teardown
    ├── mini-island.test.ts        # Visibility, pending guards, readback, errors, teardown
    └── opencode-settings.test.ts  # Native reads, CLI confirmation, target drafts, safe errors and cross-panel guards
```

## Conventions

- Keep the switch off by default and disabled until native app info and Island state are confirmed.
- Use only the typed config facade; never call Tauri `invoke` from the component.
- Keep the V2 Shared Service description explicit, including that enabling may start the service and closing the Island does not interrupt work.
- Disabling calls only `setIslandEnabled(false)`; session interruption and request replies are never settings side effects.
- Successful target saves increment the config facade's read-only `serviceTargetVersion`; Island and lid sections reread native state without remounting or toggling. Reject old-version read/save results, defer a latest read through pending operations, and never treat an old backend's protected count as current while refreshing. Failed target saves leave the version and confirmed states unchanged.
- Lid status clears its queued target reload only when a read actually starts; pending read/save completion drains one latest read, then ordinary polling resumes after two seconds without overlapping reads. Teardown clears timers and prevents queued reads.
- OpenCode CLI inspection first confirms native app info; browser mode makes no CLI or update calls. Mount and refresh are local reads only; release checks require a click.
- App update check/retry, CLI refresh/check, inspection retry, and connection refresh are icon-only buttons with translated accessible names and title tooltips. CLI refresh/check share the updates header; installation path/current version appear only in upgrade confirmation.
- Only checked, upgradeable installations expose Upgrade. The Bits UI confirmation snapshots the exact path, current version, and target version for the guarded command.
- Pending CLI operations disable connection controls; pending service reads/saves disable CLI actions and prevent upgrade confirmation. Invalidate reads on teardown; no late readback, retries, or toasts. CLI errors retain confirmed details and require explicit refresh before retrying an upgrade.
- A successful upgrade rereads CLI installation and service status once, without global refresh or service restart. Keep CLI update feedback separate from backend version. Unsupported installations show no metadata or manual command; only an explicit available-update result adds a short manual-update hint. CLI errors show translated hints without raw backend messages.
- Service mount and explicit Refresh probe only the confirmed target without bootstrap; refresh never overwrites URL/password/clear-password drafts. Failed reads cannot show stale Connected/PID/version. Failed saves preserve confirmed target and local drafts; successful offline saves commit the returned normalized target.
- Password inputs never read back a secret and are cleared on successful save and teardown. Blank omits the password argument, explicit clearing sends an empty string, Shared sends null URL. Describe direct V2 loopback APIs and Basic backend passwords, not OpenChamber UI URLs/passwords or Bearer fallback.
- Service errors render only translated generic or `busy`/`validation_failed` action hints; never render raw message/detail/status.error or log credentials. Busy asks users to wait for the ongoing operation and retry; validation points only to the direct HTTP loopback URL, positive port, and backend password. Do not ask users to disable Mini Island or lid protection before changing targets; the backend owns monitor refresh and input validation, with enabled preferences preserved. Settings never toggles monitors or power policy to save a target. Credential copy states plaintext/non-encrypted storage and does not claim Windows ACL validation before native verification.

## Sync

Update this file when settings ownership, CLI lifecycle, or the macOS Island switch lifecycle changes.
