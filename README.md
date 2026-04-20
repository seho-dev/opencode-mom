# omo-switch

omo-switch is a lightweight macOS menu bar app for switching Oh My OpenAgent model groups. It lets you maintain named groups of category mappings and agent overrides, then write the selected group into your `oh-my-openagent.json` configuration.

## Features

- Menu bar app with no Dock icon.
- Create and edit named model groups.
- Switch active groups directly from the menu bar.
- Save changes to the active group and automatically sync them to `oh-my-openagent.json`.
- Preserve existing unknown fields in the Oh My OpenAgent config where possible.
- Writes model references without escaping forward slashes, for example `openai/gpt-5.4`.

## Requirements

- macOS 13.0 or later.
- Oh My OpenAgent config location: `~/.config/opencode/oh-my-openagent.json`.

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

When you save the currently active group, omo-switch immediately rewrites `oh-my-openagent.json` with the latest values. When you switch to another group, omo-switch also writes that group to `oh-my-openagent.json`.

## Configuration files

omo-switch stores its own group data separately from the target Oh My OpenAgent config.

| File | Purpose |
| --- | --- |
| `~/.config/omo-switch/groups.json` | omo-switch group definitions |
| `~/.config/omo-switch/state.json` | currently selected group and write metadata |
| `~/.config/opencode/oh-my-openagent.json` | target Oh My OpenAgent config rewritten on switch/save |

Before rewriting `oh-my-openagent.json`, omo-switch creates backups under the omo-switch config directory.

## Development

Run the test suite:

```bash
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
