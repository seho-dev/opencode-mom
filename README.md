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
- OpenCode config location: `~/.config/opencode/opencode.jsonc`, falling back to `opencode.json` when only the latter exists (`OPENCODE_CONFIG_DIR` can override).

## Download

Tagged release workflows produce these Tauri artifacts:

- GitHub Release asset: `opencode-mom_0.1.0_x64-setup.exe` (Windows NSIS installer).
- GitHub Release asset: `opencode-mom_0.1.0_aarch64.dmg` (macOS disk image).
- GitHub Release asset: `opencode-mom_0.1.0_aarch64.app.tar.gz` (macOS app bundle archive preserving bundle metadata and executable permissions).

<https://github.com/seho-dev/opencode-mom/releases/latest>

## Unsigned macOS builds

Current macOS release artifacts are unsigned and not notarized. macOS may block the app or require approval in **System Settings > Privacy & Security**. After moving `opencode-mom.app` into `/Applications`, the quarantine attribute can be removed manually:

```bash
sudo xattr -dr com.apple.quarantine /Applications/opencode-mom.app
```

No signing or notarization is performed by the current release workflow.

## Configuration behavior

opencode-mom stores its own groups and selection state in one config file, separately from target application configs.

| File                                           | Purpose                                                           |
| ---------------------------------------------- | ----------------------------------------------------------------- |
| `~/.config/opencode-mom/config.json`           | opencode-mom group definitions, selection, and write metadata.    |
| `~/.omo/omo.jsonc`                             | Rewritten when switching groups or saving the active group.       |
| `~/.config/opencode/oh-my-opencode-slim.jsonc` | Rewritten when switching Slim-type groups.                        |
| `~/.config/opencode/opencode.jsonc` (or `opencode.json`) | Patched only when effective OpenCode agent model overrides exist. |

Before rewriting target configs, opencode-mom creates backups under its config directory. Only `config.json` is read or written for opencode-mom data; legacy split files are ignored and are not migrated.

When switching groups, opencode-mom rewrites the Oh My OpenAgent projection. Saving the active group reapplies that projection immediately. OpenCode sync is skipped when no effective OpenCode overrides exist, so a missing OpenCode config file only blocks operations that actually require OpenCode changes.

For OpenCode, opencode-mom patches only `agent.<name>.model`. Existing fields inside agent objects and unrelated top-level keys such as `$schema`, `plugin`, and `provider` are preserved.

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

## Release process

Pushing a `v*` tag starts the release workflow. Each build job first requires the pushed tag to exactly equal `v<version>` from `src-tauri/tauri.conf.json`, then uses the pinned Node and Rust versions above to build and assert the exact Tauri artifact paths. The macOS job packages `opencode-mom.app` as `opencode-mom_0.1.0_aarch64.app.tar.gz` with macOS `tar` so bundle metadata and executable permissions survive transfer. A least-privilege publishing job downloads each platform artifact into an explicit directory and creates or updates the GitHub Release with the exact NSIS installer, app archive, and DMG.

## License

No license has been declared yet.
