# opencode-mom

opencode-mom is a cross-platform desktop app for switching Oh My OpenAgent model groups. It maintains named category mappings, Oh My OpenAgent agent overrides, and optional OpenCode agent model overrides, then syncs the selected group into the matching config files.

The active application is built with Tauri 2. The Svelte 5 and TypeScript frontend provides a single-window management console, while the Rust backend owns config persistence, projection, backups, and native Windows/macOS behavior.

## Features

- Model groups with category and agent mappings.
- Single-window desktop management console.
- Conditional OpenCode agent model overrides.
- Native Windows NSIS and macOS app/DMG packages.

## Requirements

- Node.js 22.23.0 and npm.
- Rust 1.77.2 or later with Cargo. CI and release workflows build with Rust 1.96.0.
- Platform prerequisites required by Tauri 2 for Windows or macOS desktop builds.
- Oh My OpenAgent config location: `~/.omo/omo.jsonc`.
- Slim config location: `~/.config/opencode/oh-my-opencode-slim.jsonc`.
- OpenCode config location: `~/.config/opencode/opencode.jsonc`, falling back to `opencode.json` when only the latter exists. `OPENCODE_CONFIG_DIR` can override the directory; `OPENCODE_CONFIG` selects an arbitrary managed config file.

## Download

Tagged release workflows produce these Tauri artifacts:

- GitHub Release asset: `opencode-mom_0.1.0_x64-setup.exe` (Windows NSIS installer).
- GitHub Release asset: `opencode-mom_0.1.0_aarch64.dmg` (macOS disk image).
- GitHub Release asset: `opencode-mom_0.1.0_aarch64.app.tar.gz` (macOS app bundle archive preserving bundle metadata and executable permissions).

<https://github.com/seho-dev/opencode-mom/releases/latest>

## Unsigned macOS builds

Current macOS release artifacts are not Developer ID-signed or notarized. macOS may block the app or require approval in **System Settings > Privacy & Security**. After moving `opencode-mom.app` into `/Applications`, the quarantine attribute can be removed manually:

🔒 Only bypass quarantine for an artifact whose source you trust.

```bash
sudo xattr -dr com.apple.quarantine /Applications/opencode-mom.app
```

The release workflow does not Developer ID-sign or notarize the app. The embedded macOS lid helper is separately ad-hoc signed during the Rust build.

## Configuration behavior

opencode-mom stores its own groups and selection state in one config file, separately from target application configs.

| File                                                     | Purpose                                                                            |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `~/.config/opencode-mom/config.json`                     | opencode-mom group definitions, selection, and write metadata.                     |
| `~/.omo/omo.jsonc`                                       | Merged when switching groups or saving the active group.                           |
| `~/.config/opencode/oh-my-opencode-slim.json`            | Merged when switching Slim-type groups.                                            |
| `~/.config/opencode/opencode.jsonc` (or `opencode.json`) | Patched only for Native-type groups with effective OpenCode agent model overrides. |

Before rewriting target configs, opencode-mom creates backups under its config directory. Only `config.json` is read or written for opencode-mom data; legacy split files are ignored and are not migrated.

MCP and Skills discovery includes the explicit `OPENCODE_CONFIG` file as the highest-priority managed source, even when it is a project file. Unselected project configs are not automatically discovered. Selecting an explicit file does not exclude the other managed config sources.

When switching groups, opencode-mom merges the Oh My OpenAgent projection into the existing file. Saving the active group reapplies that projection immediately. Both the Oh My OpenAgent and Slim projections are one-to-one key patches: entries not defined by the group are preserved as residual configuration. Changing a group's type removes only the keys that group owned, while selecting a different group or deleting a group leaves the target files untouched. OpenCode sync is skipped when no effective OpenCode overrides exist, so a missing OpenCode config file only blocks operations that actually require OpenCode changes.

For OpenCode, opencode-mom patches only `agents.<name>.model` (the V2 `provider/model#variant` selector). Existing fields inside agent objects and unrelated top-level keys such as `$schema`, `plugins`, and `providers` are preserved.

## Development

Install dependencies:

```bash
npm ci
```

Run the Tauri app in development:

```bash
npm run tauri dev
```

Format frontend code:

```bash
npm run format
```

Build the frontend and check the Rust backend independently:

```bash
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
```

Build native Tauri packages for the current platform:

```bash
npm run tauri build
```

The configured package identity is `dev.seho.opencode-mom`, product name `opencode-mom`, version `0.1.0`. Tauri targets are Windows NSIS plus macOS app and DMG.

### macOS lid helper build and safe checks

For macOS targets, `src-tauri/build.rs` builds `src-tauri/native/macos-helper` as a separate Cargo workspace with its own `Cargo.lock`. Even a parent `cargo check` builds the helper with `--release --offline --locked`, the parent's target triple, and an isolated output directory. It then uses `/usr/bin/codesign` to ad-hoc sign and verify the helper and `/usr/bin/shasum` to hash the signed bytes before embedding them.

Use a macOS host with the Xcode Command Line Tools/macOS SDK, the Rust target required by the parent build, and `rustfmt`. Both native tools above must be available. The helper's locked dependencies (currently `libc = 0.2.186`) and registry metadata must already be cached in the Cargo environment used by the build. Offline builds work with a populated cache; on a fresh developer or CI cache, fetch the standalone workspace first while network access is available:

```bash
cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked
```

Do not assume fetching only the parent workspace provisions the helper. A pre-provisioned cache is also sufficient. The standalone helper is not covered by the parent's formatting/test commands; run these separately on macOS:

```bash
cargo fmt --manifest-path src-tauri/native/macos-helper/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
cargo test --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked
```

These checks require no administrator authorization and do not change real power policy: helper tests use mock policy backends and local test sockets. Do not run the helper executable directly or run these commands with `sudo`. Hardware acceptance below is separate and optional for local development.

### Manual lid protection acceptance (macOS / Windows)

🔒 This opt-in check changes real system power settings. Use only a laptop you control, save your work, keep it ventilated, and supervise brief trials. Never leave an awake, closed laptop in a bag. Keep the lid open unless the app confirms **Protected**; **Checking**, **Unknown**, or **Error** is not confirmation. Do not change power settings in another tool during the trial or manually disable sleep to make the test pass.

On macOS, the administrator-authorized helper changes the global `SleepDisabled` policy, not just display/idle sleep. On Windows, protection temporarily changes the active power scheme's AC/DC lid-close actions and requests system wakefulness. Only working sessions in the OpenCode service connected through the CLI are monitored; other instances are not covered.

1. Record the original policy with read-only commands: macOS `/usr/bin/pmset -g` (`SleepDisabled`); Windows `powercfg /getactivescheme` and `powercfg /qh SCHEME_CURRENT SUB_BUTTONS` (including hidden AC/DC lid-close values). If the baseline already prevents lid sleep, check restoration without changing it just to manufacture a sleep test.
2. On macOS, enable the setting and cancel the administrator prompt. Confirm an error/non-protected state, unchanged baseline, and no repeated prompt in that app run. Restart the app before an approved authorization attempt. Windows has no equivalent app authorization prompt; if OS policy denies writes, confirm an error/non-protected state and unchanged or restored values. Do not weaken OS policy to manufacture this denial case.
3. Enable protection (approve macOS authorization only if trusted), start a real working session in the CLI-connected service, and wait for **Protected** with a positive working-session count. Close the lid briefly, reopen it, and verify the work continued rather than relying only on the status label. Repeat on AC and battery only where safe and explicitly approved.
4. Let all monitored work finish. Wait for **Idle**, compare policy readback with the baseline, and check that a brief lid close again follows the original behavior. Restoration does not itself force sleep.
5. With another working session and **Protected** confirmed, turn the setting off and verify restoration. Re-enable as needed, then repeat using **Quit MOM** while protected and compare the baseline after exit. Closing the main window only hides it to the tray; it is not an exit test.

If restoration is unconfirmed, keep the lid open and stop testing; do not delete recovery journals or issue ad-hoc power-policy writes. Restart the app for recovery (macOS requires explicitly enabling and authorizing the helper again), then verify readback. Native failures or forced helper termination can leave macOS recovery necessary; abrupt Windows app termination can leave a journaled policy until a subsequent app launch. Automated tests and a successful policy readback do not replace real lid-close acceptance on each supported laptop/OS.

## Release process

Pushing a `v*` tag starts the release workflow. Each build job first requires the pushed tag to exactly equal `v<version>` from `src-tauri/tauri.conf.json`, then uses the pinned Node and Rust versions above to build and assert the exact Tauri artifact paths. The macOS job packages `opencode-mom.app` as `opencode-mom_0.1.0_aarch64.app.tar.gz` with macOS `tar` so bundle metadata and executable permissions survive transfer. A least-privilege publishing job downloads each platform artifact into an explicit directory and creates or updates the GitHub Release with the exact NSIS installer, app archive, and DMG.

## License

No license has been declared yet.
