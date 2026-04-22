# omo-switch

omo-switch is a lightweight macOS menu bar app for switching Oh My OpenAgent model groups. It lets you maintain named groups of category mappings, Oh My OpenAgent agent overrides, and optional OpenCode agent model overrides, then sync the selected group into the matching config files.

## Features

- Menu bar app with no Dock icon.
- Create and edit named model groups.
- Switch active groups directly from the menu bar.
- Save changes to the active group and automatically sync them to the relevant target config files.
- Preserve existing unknown fields in the Oh My OpenAgent config where possible.
- Conditionally sync `~/.config/opencode/opencode.json` when the selected or active group contains OpenCode overrides with non-empty model values.
- Patch only `agent.<name>.model` on the OpenCode side, while keeping other agent fields and unrelated top-level config untouched.
- Writes model references without escaping forward slashes, for example `openai/gpt-5.4`.

## Requirements

- macOS 13.0 or later.
- Oh My OpenAgent config location: `~/.config/opencode/oh-my-openagent.json`.
- Optional OpenCode config location: `~/.config/opencode/opencode.json`.

## Download

Download the latest DMG from the GitHub Releases page:

<https://github.com/seho-dev/omo-switch/releases/latest>

Each tagged release uploads a DMG asset built by GitHub Actions. Download the `.dmg`, open it, and drag `omo-switch.app` into `Applications`.

## First launch on macOS

Current GitHub release builds are distributed without Apple notarization. macOS Gatekeeper may show a warning such as:

- “omo-switch cannot be opened because Apple cannot check it for malicious software.”
- “omo-switch is damaged and can’t be opened.”
- “This app is from an unidentified developer.”

If macOS blocks the app:

1. Open **System Settings**.
2. Go to **Privacy & Security**.
3. Scroll to the **Security** section.
4. Find the message about `omo-switch` being blocked.
5. Click **Open Anyway** or **Allow Anyway**.
6. Launch `omo-switch` again and confirm the prompt.

You can also right-click `omo-switch.app` and choose **Open** for the first launch.

## Usage

1. Launch `omo-switch`.
2. Click the `OMO` item in the macOS menu bar.
3. Open **Settings**.
4. Create or edit a group.
5. Add category mappings and agent overrides.
6. Click **Save**.
7. Use the menu bar group list to switch active groups directly.

When you switch to another group, omo-switch rewrites `~/.config/opencode/oh-my-openagent.json` for that group.

When you save the currently active group, omo-switch immediately reapplies that active group's projection to the same target config.

If the selected or active group contains OpenCode overrides with effective model values, omo-switch also syncs `~/.config/opencode/opencode.json` in the same operation.

If the group has no effective OpenCode overrides, omo-switch skips `opencode.json` entirely. A missing OpenCode config is only a blocker when the group actually needs OpenCode sync.

On the OpenCode side, omo-switch only patches `agent.<name>.model`. It does not rewrite other fields inside the agent object, and it does not touch unrelated top-level keys such as `$schema`, `plugin`, or `provider`.

## Configuration files

omo-switch stores its own group data separately from the target app configs.

| File | Purpose |
| --- | --- |
| `~/.config/omo-switch/groups.json` | omo-switch group definitions |
| `~/.config/omo-switch/state.json` | currently selected group and write metadata |
| `~/.config/opencode/oh-my-openagent.json` | target Oh My OpenAgent config rewritten on switch and when saving the active group |
| `~/.config/opencode/opencode.json` | conditional OpenCode sync target, patched only when the group includes effective OpenCode agent model overrides |

Before rewriting target configs, omo-switch creates backups under the omo-switch config directory.

## Development

Run the test suite:

```bash
swift test
```

Run the task-specific verification commands:

```bash
swift test --filter EndToEndSwitchingTests
swift test --filter AppStoreTests
swift test
```

Generate the Xcode project from `project.yml`:

```bash
brew install xcodegen
xcodegen generate
```

Build the app locally:

```bash
xcodebuild \
  -project omo-switch.xcodeproj \
  -scheme OMOSwitch \
  -configuration Debug \
  -destination 'platform=macOS' \
  CODE_SIGN_IDENTITY="" \
  CODE_SIGNING_REQUIRED=NO \
  build
```

## Release process

Releases are built and published by GitHub Actions.

To publish a release:

```bash
git tag v1.0.0
git push origin v1.0.0
```

The release workflow will:

1. Run tests.
2. Generate the Xcode project with XcodeGen.
3. Build `omo-switch.app` in Release configuration.
4. Package the app into a DMG.
5. Create or update the GitHub Release for the tag.
6. Upload the DMG as a downloadable release asset.

## Notes about signing

The current release workflow builds an unsigned app. This keeps the project releasable from public GitHub Actions without Apple Developer credentials, but it also means macOS may require the manual **Privacy & Security → Open Anyway** approval described above.

Future releases can add Developer ID signing and notarization by configuring Apple certificate and notarization secrets in GitHub Actions.

## License

No license has been declared yet.
