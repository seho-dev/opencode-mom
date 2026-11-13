# Changelog

## 2.0.3 - 2026-11-12

- release: 2.0.3
- feat: preserve pagination across resource navigation
- test: cover malformed command frontmatter repair
- docs: condense feature summaries in both READMEs
- Update CHANGELOG.md

## 2.0.2 - 2026-11-12

- Preserve malformed commands for repair. (#16)
- Improve MCP Proxy diagnostics, forwarding, cleanup and process management. (#12, #13, #14, #15, #17)
- Support MCP enable toggles and project-scoped editing and deletion. (#11)
- Stream and cache large token usage histories. (#18)
- Check for updates automatically, refresh the Dashboard and confirm OpenCode restarts. (#19)
- Prevent model catalog error notification loops.
- Streamline notification documentation.

## 2.0.1 - 2026-11-10

- fix(release): repair the CI gates blocking the v2.0.1 release
- feat: reload OpenCode on tray group switch
- fix: fall back to model id when name is missing
- feat(console): improve provider and model workflows
- fix(config): omit absent fields and normalize group variants
- refactor(routes): update island, logs, plugins, settings, and tray views
- refactor(routes): update catalog and resource management views
- refactor(routes): update dashboard, groups, and core layout
- refactor(components): streamline shared components and shell controls
- refactor(config): refine config store, adapter, and test harnesses
- refactor(core): clean up types, constants, utils, and i18n dictionaries
- refactor(ui): remove unused components, obsolete styles, and update contracts
- refactor(backend): streamline tauri commands and internal modules
- feat(notifications): implement native system notifications and settings
- feat(island): enhance mini island geometry, perimeter light, and projection
- feat(groups): support group duplication and add new route verification
- ci: add change-detection script and improve workflow concurrency
- fix(macos): use an overlay titlebar and safe drag regions (#6)
- feat(logs): copy record details and retained operation traces (#5)
- fix(plugins): honor update acknowledgements and timeout budget (#4)
- fix(styles): use system UI fonts with CJK fallbacks (#3)
- fix(island): improve non-notched compact layout (#2)
