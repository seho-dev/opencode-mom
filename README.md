<div align="center">

![opencode-mom logo](shared/mom-logo.svg)

# opencode-mom

**The model-group manager for OpenCode v2.**<br />
One window. Three runtimes — **Native**, **Slim**, **OMO**. One click to switch your whole agent setup, without losing a single hand-tuned config key.

[![Latest release](https://img.shields.io/github/v/release/seho-dev/opencode-mom?label=release&color=00E5FF)](https://github.com/seho-dev/opencode-mom/releases/latest)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-6AA4FF)
![OpenCode v2 only](https://img.shields.io/badge/OpenCode-v2%20only-FF6B6B)
![License: MIT](https://img.shields.io/badge/license-MIT-3DA639)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)
![Personal goal: 100 stars](https://img.shields.io/badge/personal%20goal-100%20stars-yellow?logo=apple&logoColor=white)
[![GitHub stars](https://img.shields.io/github/stars/seho-dev/opencode-mom?style=social)](https://github.com/seho-dev/opencode-mom/stargazers)

[English](README.md) · [简体中文](README.zh-CN.md)

</div>

> [!IMPORTANT]
> **opencode-mom only supports OpenCode v2.** MOM reads and writes the v2 config surface — `provider/model#variant` model selectors, v2 permission rules, and v2 provider/model schemas. V1 configs are neither migrated nor managed. Make sure your OpenCode install is on v2 before switching groups.

> [!NOTE]
> **⭐ A personal goal: 100 stars.**
> macOS builds currently use ad-hoc signing, not an Apple Developer ID certificate, and are not notarized. At 100 stars I plan to self-fund Apple Developer Program membership and work toward signed, notarized releases.
> If MOM has helped you, a Star is welcome but entirely optional: it never gates downloads or features, and is encouragement, not payment for a certificate. Thank you for supporting the project; you can also build and sign your own copy if you prefer.

> [!WARNING]
> **macOS first launch requires a deliberate trust decision.**
> After dragging `opencode-mom.app` into `/Applications`, try opening it once. If blocked, prefer **Open Anyway** in **System Settings → Privacy & Security**; this GUI step is normally sufficient.
>
> The command below is **not mandatory**. Only if a trusted official copy is still blocked, or **Open Anyway** is absent, consider this optional fallback to remove quarantine **only for this app**, without `sudo`:
>
> ```bash
> xattr -dr com.apple.quarantine "/Applications/opencode-mom.app"
> ```
>
> 🔒 Only consider these steps for a copy downloaded from this repository's official [Releases](https://github.com/seho-dev/opencode-mom/releases/latest) page after reviewing its source and release information. Do not bypass quarantine for an untrusted build or disable Gatekeeper globally.

<div align="center">

![opencode-mom dashboard in light and dark themes](shared/app.png)

*MOM at a glance — Native, Slim and OMO runtimes, live OpenCode service status, token usage and an activity heatmap, in light and dark.*

</div>

## 🌟 What is opencode-mom?

**opencode-mom** ("MOM") is a cross-platform desktop app that keeps a multi-runtime OpenCode v2 setup organized and instantly switchable. It manages named **model groups**, each binding providers, models, agents and categories for one of three runtimes:

- **Native** — OpenCode's own agents, written into `~/.config/opencode/opencode.jsonc` (or `opencode.json`).
- **Slim** — Oh My OpenCode Slim, written into `~/.config/opencode/oh-my-opencode-slim.json`.
- **OMO** — Oh My OpenAgent, written into `~/.omo/omo.jsonc`.

Model-group projection uses one-to-one key patches that leave keys the group doesn't own exactly as they were.

## ✨ Features

### 🧩 Model groups — switch everything in one click

- Named groups per runtime, with category mappings and per-agent model bindings, including `#variant` selection.
- Switch from the **Dashboard**, the **Groups** page, or the **tray popup**; the selected group is projected immediately.
- Live OpenCode service health and working-session counts, plus a configuration reload reminder after switches when OpenCode needs one.
- Non-destructive: hand-tuned keys are preserved, and changing a group's type removes only the keys that group owned.

### 🧠 Providers, models & agents

- **Providers** — register custom OpenAI-compatible, Anthropic or custom endpoints with API keys, base URLs and extra headers, all validated.
- **Models** — a catalog with modalities (text, audio, image, video, pdf), context limits, input/output/cache pricing and custom variants; enabled models from the selected local service are merged with configured models, including disabled models.
- **Agents** — browse the builtin catalog (24 agents across native / slim / omo) or define your own: mode, `provider/model#variant` selector, system prompt, color, and visual v2 permission rules (`allow` / `ask` / `deny`). Store definitions inline in config or as global Markdown; edits retain their original source snapshot and reject stale content or a retargeted source.

### 🔌 MCP & 📚 Skills

- **MCP** — see every registered server across config sources, inspect diagnostics, keep credentials masked, edit raw JSON, and delete safely; failed file replacements retain recovery `.bak` snapshots. Includes a built-in MCP HTTP Proxy gateway bridging local stdio MCP servers over HTTP/SSE for clients like OpenCode/Claude desktop with bearer token authentication and cross-platform process tree cleanup.
- **Skills** — discover local and remote skills, view and edit content, watch diagnostics. Remote skills stay read-only.

#### Global and saved project roots

- Only the **MCP, Agents and Skills list pages** have a scope selector. Add project roots with the native folder chooser, switch among saved roots, or return directly to **Global**. These pages share the last scope choice; adding the same folder again does not create another record.
- **Project scope is read-only** in this first version: browse lists and details. **Show global configuration** adds a read-only comparison, not OpenCode's effective runtime configuration. Global create/edit/delete remains available independently of the saved project selection. Providers, Models, Groups, Settings, the tray, and OpenCode's running group/service are unaffected.
- Removing a root only forgets its saved record, never its files. If the selected record is removed, the selector returns to Global. A missing folder stays saved and reports `not_found` rather than falling back to Global.

For a selected root `R`, the expected project paths are:

| Resource | Project paths |
| :--- | :--- |
| MCP / JSON agents | `R/opencode.json`, `R/opencode.jsonc`, `R/.opencode/opencode.json`, `R/.opencode/opencode.jsonc` |
| Markdown agents | `R/.opencode/agent`, `R/.opencode/agents` |
| Skills | `R/.opencode/skill`, `R/.opencode/skills` |

Only that exact root is inspected: no ancestor, sibling or descendant project discovery and no project `.claude` / `.agents` paths. Neither scope uses `mode` / `modes` directories. This is a limited source browser, not OpenCode's full discovery or effective state.

Adding and reading a project reject filesystem/volume roots and canonical overlaps between its fixed sources and MOM's captured Global config files/resource roots, including overrides. There is no blanket home-directory or minimum-depth exclusion; path checks do not guarantee protection against concurrent symlink changes.

Global recognizes native `agent` / `agents` and `skill` / `skills` directory aliases, retaining the existing `~/.claude/skills` and `~/.agents/skills` roots. Configured relative skill entries such as `./team-skills` are ignored in **both** scopes. Global retains configured absolute paths, home-relative `~/` paths and HTTP(S) Remote Catalogs; Project ignores **all** configured extra skill sources, including absolute/home paths and URLs, using only the fixed native directories above.

### 📊 Dashboard, usage & tray

- Real-time status: active group and runtime, OpenCode service health, running sessions.
- Token usage (input / output / cache) and an activity heatmap with daily, weekly, monthly and yearly views.
- Tray popup: switch groups, reload the OpenCode service configuration, open the app or settings, quit. Closing the window hides MOM to the tray.

### 🏝️ MOM Mini Island — first version (macOS only)

- **Settings switch, off by default:** changes are persisted and applied immediately. There are three views: a small island with one status indicator and no brand label, a medium quick area revealed by hover or keyboard focus without stealing focus, and a large mini session window opened only through the medium view's downward disclosure control. Every screen defaults to the small island. Native notch geometry controls information density: notched screens keep the single indicator, while non-notched screens also show short status and counts within the same small view. Clicking the small island reveals the medium view, not the large one. New pending requests never enlarge it automatically. Each view changes the actual native window size. Browse recent root sessions and globally active sessions, including active children—not the complete history. The selected session shows up to 20 public user/assistant messages from the newest bounded message page, truncated to 16 KiB per message; reasoning, tool output and hidden/internal content are excluded.
- **Explicit interactions:** send plain text, allow a permission once or reject it, answer supported string/multiselect question fields (including custom input when allowed), or explicitly interrupt a session. **Allow once** remains the default; its dropdown offers **Always for this project** only when OpenCode supplies nonempty `save` rules. Hover opens options but never approves. Always saves the request's action and `save` patterns for its OpenCode Project and may also allow matching pending requests; this is unrelated to MOM's selected configuration scope. Unsupported forms remain read-only. Uses the selected local **OpenCode V2 service**: default Shared Service, or an explicitly configured fixed local OpenCode backend endpoint owned by OpenChamber. Shared mode may start its service through the CLI; fixed mode never does. No V1, remote server or plugin integration. ⚠️ The HTTP API is experimental; the transparent macOS window enables `macOSPrivateApi` and targets `.app`/DMG distribution, not the Mac App Store.
- **Local lifecycle and privacy:** disabling aborts the monitor, clears private cached state and destroys the window, without rejecting requests or stopping OpenCode. No persistent Island SSE stream or polling runs while off; an already-running bounded connection worker may continue for up to 20 seconds after disable and keeps target changes busy until it finishes. Dropping a local write cannot withdraw a POST already accepted by the server. Credentials stay in Rust; Island code does not log raw responses or message text. Notch, display, Spaces/fullscreen and wake behavior still require the separate manual acceptance below.

### 💤 Lid protection — work with the lid closed (macOS / Windows)

- Keeps the laptop awake **only while OpenCode sessions are actually working** in the selected local service, then restores your original power policy automatically.
- macOS flips `SleepDisabled` through a bundled helper behind a one-time administrator authorization; Windows journals and restores the active power scheme's AC/DC lid-close actions.
- Covered by a manual lid-close acceptance checklist — automated tests do not replace it (see [Development](#-development)).

### 🌗 Native feel

- English / 简体中文 UI, dark (default) and light themes, launch-at-login, a single-window console plus a tray for quick access.

## 🏝️ Roadmap

- **Developer ID signed & notarized macOS builds** — planned, with self-funded membership at the personal goal of ⭐ 100 stars; stars do not purchase a certificate.

## 📦 Install

Grab the latest build from **[GitHub Releases](https://github.com/seho-dev/opencode-mom/releases/latest)**:

| Platform | Artifact |
| :--- | :--- |
| Windows (x64) | `opencode-mom_*_x64-setup.exe` — NSIS installer |
| macOS (Apple Silicon) | `opencode-mom_*_aarch64.dmg` |
| macOS (Intel) | `opencode-mom_*_x64.dmg` |

The workflow also provides `.app.tar.gz` archives for both macOS architectures. Windows installers are unsigned and may show a **SmartScreen** warning; review the publisher/source and only proceed for the official Release you intended to install.

### macOS first launch

Builds are ad-hoc signed but not Developer ID signed or notarized — **Open Anyway** in System Settings is preferred and normally sufficient; the scoped quarantine command above is only an optional fallback.

### Manual updates and Settings checks

**Settings** shows the actual running app version. Click **Check for updates** to compare stable `X.Y.Z` versions numerically against the latest stable GitHub Release. Results distinguish an available update, up to date, no published release, and errors; an unreadable or invalid version, network failure or rate limit is not confirmation that you are up to date. When an update is available, **View GitHub downloads** opens the official Releases page in your browser. Download and install manually: there are no automatic checks or installations.

- **macOS:** First choose **Quit** from the tray menu (closing the window only hides it). Download the Apple Silicon (`aarch64`) or Intel (`x64`) `.dmg` for your Mac, open it, drag `opencode-mom.app` into `/Applications` and choose **Replace**, then reopen the app. If blocked, follow the **Open Anyway** guidance above; the quarantine command is not a required update step.
- **Windows:** Quit MOM from the tray, then run the new unsigned NSIS `opencode-mom_*_x64-setup.exe` installer and reopen the app. Only proceed past **SmartScreen** if you trust the intended official Release download.

After reopening, verify that the actual version shown in **Settings** matches the Release you installed, and check launch-at-login and lid-protection status yourself. Enabling lid protection changes system power policy; use the manual acceptance checklist below rather than assuming an upgrade has verified it.

### Requirements

- **OpenCode v2** for service features: status, sessions, model discovery, reload, token usage and Mini Island. Shared-service bootstrap and CLI installation management find the binary via `$OPENCODE_BIN`, standard install locations, then `$PATH`.
- Editing configs works without the CLI or service; unavailable service operations report errors, never an idle session count.

### OpenCode V2 installation and updates

See the [official V2 documentation](https://opencode.ai/v2/docs/) for platform prerequisites and current installation instructions. Official CLI installation choices include:

```sh
curl -fsSL https://opencode.ai/v2/install | bash
npm install -g @opencode/cli@2
bun install -g @opencode/cli@2
pnpm add -g --allow-build=@opencode/cli @opencode/cli@2
yarn global add @opencode/cli@2
vp install -g @opencode/cli@2
brew install anomalyco/tap/opencode-v2
```

The official V2 docs also cover AUR, manual binary downloads, the Desktop app and Docker; follow the platform-specific instructions there.

🔒 Review downloaded installers and use only official sources. **Settings → OpenCode** inspects the selected CLI locally; update checks and upgrades run only when requested. MOM offers automatic upgrade only for strictly proven Unix curl/npm/bun/pnpm/yarn installations whose current version is stable V2 **≥ 2.0.25**, using the selected binary's native `selected_binary upgrade <stable-V2-target> --method <method>` command. It checks the expected path/version and ownership before dispatch, then reads back the same path to verify the version. Windows, Homebrew, Vite Plus, older V2, bundled OpenChamber binaries, unproven and manual installations remain manual. MOM never uses `sudo`, upgrades bundled binaries, or stops/restarts a service. A CLI version is not the running backend version: upgrading a binary can leave an older service running. Inspect its status and use the owner's documented lifecycle controls when safe.

### Local OpenCode service target

- **Shared Service (default):** validates `opencode/service.json` under `$XDG_STATE_HOME` or `~/.local/state`, authenticates with Basic user `opencode`, and verifies `/api/info` PID/version against the registration. API operations may perform one bounded CLI bootstrap when registration is missing, unreachable or identity-mismatched. Invalid registration and explicit HTTP errors such as Unauthorized fail closed. The Settings status probe never bootstraps.
- **Fixed local endpoint (optional):** use the direct OpenCode V2 backend endpoint owned by your local OpenChamber setup, not its web UI port or UI proxy. Only `http://127.0.0.1:PORT`, `http://[::1]:PORT` or exact `http://localhost:PORT` (normalized to `127.0.0.1` without DNS) are accepted. With a configured backend service password, requests use Basic user `opencode`; without one, they send no Authorization header. MOM does not infer Bearer or UI-pairing authentication, and 401/403 never trigger auth fallback. A successful `/api/info` probe confirms PID/version reachability, not access to protected routes. A failed fixed endpoint never falls back to Shared Service, another CLI-discovered backend, or a MOM-owned service, and never bootstraps. Save is allowed while offline; connection failure does not undo a successful save.
- Active sessions, model catalog, reload, usage, lid monitoring and Mini Island all use this one target. `/api/session/active` describes only that backend process, including its active children; it is not a machine-wide inventory. A model catalog's `location[directory]` selects service context, not another server; MOM's read-only configuration project scope is separate from the running OpenCode service/location.
- Saving a target automatically reconnects enabled **Mini Island** and refreshes **lid protection**, preserving both switches and the current power authorization. Disabled features stay disabled. Old Island messages/drafts and old session counts are invalidated; lid protection restores the old policy before querying the new target, and a failed restore remains visible rather than claiming protection or idle. Saving never interrupts external sessions or restarts a service. In-flight bootstrap, reload, Island writes or initial power authorization return `busy` on target changes; wait for the operation to finish and retry. In-flight reads reject stale target results, and Island messages/actions must match the connection generation displayed by their caller. Writes recheck PID/version before dispatch and are never automatically retried; this check cannot atomically prevent an external restart between health and dispatch.
- 🔒 Endpoint/password are persisted by the backend only in `~/.config/opencode-mom/opencode-service.json`, outside the aggregate app config. Unix stores plaintext in a regular account-owned `0600` file capped at 8 KiB. Windows stores `encryptedPassword` as numeric bytes protected by native current-account DPAPI, with a 32 KiB file limit and an 8 KiB ciphertext limit; there is no plaintext fallback, and decryption failures fail closed. Shared Service registration credentials are handled separately. Use a dedicated backend password: neither Unix permissions nor DPAPI protect against account compromise, same-account malicious processes or administrators. Passwords are never returned to the frontend or included in DTOs/events/errors/logs. Windows-native DPAPI tests have not run on this macOS host; no verified Windows ACL protection is claimed.
- Omitted password retains it only for the unchanged normalized URL; empty password explicitly clears it; changing URL or returning to Shared drops it. Saves publish atomically; a post-publication directory-sync warning does not undo the committed selection.

For Windows process/service diagnostics without dumping passwords or command lines:

```powershell
Get-CimInstance Win32_Process | Where-Object Name -Match '^(opencode|openchamber)(\.exe)?$' | Select-Object ProcessId, Name, ExecutablePath
Get-CimInstance Win32_Service | Where-Object Name -Match 'opencode|openchamber' | Select-Object Name, State, ProcessId
```

Compare the backend PID/version shown in Settings with its owner; process presence alone does not establish a healthy or authenticated API. Do not paste registration files, environment dumps or process command lines containing credentials into diagnostics.

## 🔧 How MOM writes config

MOM keeps its own groups, selection state, saved project roots and shared last scope choice in `~/.config/opencode-mom/config.json`, separately from the files it manages:

The Island switch is stored as `state.islandEnabled` (default `false`), outside theme/locale preferences; preference saves and group changes preserve it.

| Group type | Target file | Patched keys |
| :--- | :--- | :--- |
| `native` | `~/.config/opencode/opencode.jsonc` (fallback `opencode.json`) | `agents.<name>.model` only |
| `slim` | `~/.config/opencode/oh-my-opencode-slim.json` | agent model bindings |
| `omo` | `~/.omo/omo.jsonc` | categories and agent overrides |

- **Non-destructive projection.** Keys outside the group definition are preserved as residual configuration. Selecting a different group or deleting a group leaves target files untouched; changing a group's type removes only the keys that group owned.
- **App config publication.** MOM's own config read/modify/write operations share the same in-process mutex. Saves stage a complete file in the destination directory, sync it, then publish by atomic rename; they create no `.bak`. General target-config writes remain plain writes without locks or backups.
- **Resource recovery.** Existing-file inline agent CRUD, Markdown agent updates, MCP replacements and local skill updates use checked replacement with recovery `.bak` files retained on failure and cleaned up after success. Static Global symlinks remain supported; stale content and detected retargets are rejected. Expected paths identify discovered sources, not arbitrary write destinations; these checks do not eliminate filesystem races.
- **Overrides.** `OPENCODE_CONFIG_DIR` overrides the OpenCode config directory and `OPENCODE_CONFIG` pins an explicit managed file — which is also the highest-priority source for Global MCP and Skills discovery. Project browsing uses only the selected root's paths above.
- **Resource boundaries.** Project resources are read-only. New Global Markdown agents and skills use plural `agents` / `skills` directories; older directories are not migrated, and updates use the original paths or report ambiguity for duplicate sources. Saving or selecting roots does not migrate or rename files, reload the OpenCode service configuration, execute plugins, or launch MCP servers.
- **Skip when empty.** OpenCode sync is skipped when a group has no effective OpenCode overrides, so a missing OpenCode config only blocks operations that actually need it.

## 🛠️ Development

Requirements: Node.js 22.23.0 + npm, Rust ≥ 1.77.2 (CI/release pin 1.96.0), and the platform prerequisites for Tauri 2 desktop builds.

```bash
npm ci                # install dependencies
npm run tauri dev     # run the app in development
npm run tauri build   # package the current platform
```

Checks:

```bash
npm run check                 # Biome format + lint
npm run test:components      # Vitest component tests (jsdom)
npm run test:unit            # node --test unit tests
node --test scripts/release-version.test.mjs
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Never edit generated output (`build/`, `.svelte-kit/`, `src-tauri/target/`, `src-tauri/gen/`).

<details>
<summary><b>Manual Mini Island acceptance (macOS; not yet performed)</b></summary>

🔒 Sending, approving/rejecting, answering forms and interrupting intentionally change OpenCode sessions. Use only a disposable project and requests you have explicitly approved for testing. This is not a power-policy test; do not change lid-protection settings. These native checks remain unverified until performed on the relevant Mac/display setup; automated tests do not replace them.

1. Start with fresh MOM app data and confirm **Settings → Mini Island** is off with no capsule/Island monitor. For a source build, use the development command above (`npm run tauri dev`).
2. Enable it over a light desktop background and confirm the small island appears without stealing focus, with a continuous black surface, no center background leak, no MOM label and no extra right-side icons. While the local service has active work, confirm the single status indicator shows a visible waveform (static when reduced motion is enabled). Hover or click any exposed part of the small island to reveal the medium quick area without loading messages or approving anything; keep the pointer still and confirm it stays open, then move away and confirm the actual native rectangle shrinks. The physical camera notch is not an exposed input surface. Keep a permission dropdown or approval in progress and confirm pointer leave does not interrupt it. Hovering the medium view's downward control must not open the large view; clicking it must. Type in the large view's textarea, then test Escape (permission dropdown first) and clicking another app: collapse must leave the small island visible, not a large transparent input blocker.
3. Test notched and non-notched screens, external displays with different DPI, Spaces, fullscreen apps and sleep/wake. Both screen types must start and collapse to the small view; only non-notched screens add compact status text and counts, without loading chat. Confirm anchoring and interaction; record limitations rather than assuming NSPanel behavior. These hardware/desktop cases are explicitly unverified.
4. In the disposable project, create safe test sessions, permission requests and forms. Check running/pending and active-child coverage, a plain-text message, allow-once/reject, supported string/multiselect/custom answers, read-only unsupported forms and explicit interrupt. Confirm new requests only update the compact indicator. Test Always with a nonempty `save` rule only after checking the displayed action/patterns and possible matching pending requests; no-save requests must not offer it. Resolve another request outside MOM and confirm it disappears.
5. Disconnect/reconnect the Shared Service and use Refresh; check recovery without write retries. Rapidly toggle off/on, then quit/restart with Island enabled and verify restoration without stale state or focus theft.
   With lid protection off, save a different verified local backend while Island stays enabled: old messages/drafts must clear before the new target's data appears, even for matching session IDs. An offline fixed target must stay selected and show unavailable status. Repeat with Island off and confirm it remains off. Test combined power behavior only under the separate power acceptance procedure below.
6. Disable and confirm the window disappears and OpenCode sessions continue working; no request is automatically rejected. Allow for the bounded connection worker's possible 20-second finish. Finally choose **Quit** from the tray and confirm the Island monitor is cleared; closing the main window is not quitting.

</details>

### macOS lid helper

`src-tauri/build.rs` builds `src-tauri/native/macos-helper` as a standalone locked workspace, ad-hoc signs it, and embeds it. On a fresh Cargo cache, fetch it first:

```bash
cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/native/macos-helper/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
cargo test --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
```

Do not run the helper directly or with `sudo`; its tests use mock policy backends.

<details>
<summary><b>Manual lid-protection acceptance (macOS / Windows)</b></summary>

🔒 This opt-in check changes real system power settings. Use only a laptop you control, save your work, keep it ventilated, and supervise brief trials. Never leave an awake, closed laptop in a bag. Keep the lid open unless the app confirms **Protected**; **Checking**, **Unknown** or **Error** is not confirmation. Only working sessions in the selected local OpenCode service are monitored.

1. Record the baseline with read-only commands: macOS `pmset -g` (`SleepDisabled`); Windows `powercfg /getactivescheme` and `powercfg /qh SCHEME_CURRENT SUB_BUTTONS` (including hidden AC/DC lid-close values).
2. macOS only: enable the setting and cancel the administrator prompt; confirm an error/non-protected state with an unchanged baseline. Restart the app before an approved authorization attempt. Windows has no equivalent app authorization prompt.
3. Enable protection, start a real working session in the selected service, wait for **Protected** with a positive working-session count, close the lid briefly, reopen it, and verify the work continued.
4. Let all monitored work finish, wait for **Idle**, compare policy readback with the baseline, and confirm a brief lid close again follows the original behavior.
   With the lid open and both monitors enabled, save a different verified local backend. Confirm both switches stay on, no new authorization prompt appears, the old count/grace is cleared and baseline policy is restored before the new target can show Protected. An offline target must show unavailable/Unknown, not the old count or Idle. Repeat with protection disabled and confirm it stays disabled.
5. Repeat with **Quit MOM** while protected and compare the baseline after exit. Closing the main window only hides it to the tray; it is not an exit test.

If restoration is unconfirmed, keep the lid open and stop testing; restart the app for recovery. Automated tests and a successful policy readback do not replace real lid-close acceptance on each supported laptop/OS.

</details>

## 🚢 Release

The **Release** workflow runs on pushes to `main` (the current default branch), `v*` tag pushes, or **Actions → Release → Run workflow**. For commit-triggered releases on `main`, only the **HEAD commit subject** is inspected, case-sensitively:

- `release` or `release: description` → next patch version.
- `release: 2.1.0` → explicit stable `2.1.0` (the rule is `release: X.Y.Z`), strictly newer than the current app version and existing stable release tags. Malformed numeric versions, prereleases, equal versions and older versions are rejected. Each component must be `0..65535` for Windows VERSIONINFO compatibility; patch overflow is rejected.
- Subjects without the lowercase `release` word prefix (including `releases` and `chore: release`) do not release. No earlier commits in the push are scanned.
- Manual dispatch must target the default branch; its optional `version` input accepts stable `X.Y.Z`, or leave it empty for the next patch. If the default branch is renamed, update the workflow's push branch filter too.

Preparation updates `package.json`, both root version entries in `package-lock.json`, the app package in `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`. Preparation also prepends a new section to `CHANGELOG.md` using subjects of commits since the last stable tag (release commits excluded) and includes it in the release commit. It commits `chore(release): vX.Y.Z` and atomically pushes that commit and tag. The separate macOS helper keeps its own package version. Repository **Actions permissions** and branch/tag rules must allow this workflow's `GITHUB_TOKEN` to create that release commit and tag; protection rules are not bypassed.

Preparation is serialized and checks that the default-branch HEAD has not changed; a racing change rejects the push instead of overwriting it. A rerun reuses only the exact release child commit/tag from its original trigger. Rerun the failed run to retry a build without another bump. You can also push a pre-existing `vX.Y.Z` tag yourself: it must match **all** app version sources, and that path builds without bumping. Do not move published tags.

First merge the current `v2.0` work into default `main`, then trigger a release or push a release tag. App manifests use `2.0.0`; the corresponding tag is `v2.0.0`. From current `2.0.0`, `release` / `release: description` select `2.0.1`, while `release: 2.0.0` is rejected. To publish **current `2.0.0` unchanged**, merge without a release-prefixed subject and use the pre-existing tag path above: `v2.0.0` must point at the merged commit with all app version sources synced to `2.0.0`, rather than a bumping trigger.

The **same workflow run** then builds on `macos-15` (Apple Silicon), `macos-15-intel` (Intel) and `windows-2022` (x64 NSIS), using Node **22.23.0** and Rust **1.96.0**. macOS jobs prefetch the standalone helper's locked dependencies before its offline build. macOS uses ad-hoc signing identity `-`, without Developer ID signing or notarization; Windows is unsigned. No signing/updater secrets are required.

Build jobs have read-only repository permissions and upload artifacts. Only after **all three builds succeed** does a separate write-enabled job upload all five assets and publish the GitHub Release (a new release stays draft until uploads complete). Publication reruns replace matching assets rather than creating another release. Generated `GITHUB_TOKEN` pushes do not start another workflow, so the dependent build jobs are intentional. Validate the version logic locally with `node --test scripts/release-version.test.mjs`; actual hosted builds remain unverified until the workflow runs on GitHub.

## 📄 License

Released under the [MIT License](LICENSE). © 2026 seho-dev.
