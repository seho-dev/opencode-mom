# CI and Release Pipelines

Sources: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `scripts/ci-changes.mjs`.

## Change detection

The `changes` job runs on every trigger (push to `main`, `pull_request`, `workflow_dispatch`, `workflow_call`) and emits `code=true|false`.

`scripts/ci-changes.mjs` diffs the push event's `before..head` range or the pull request's `base...head` range, then classifies paths:

| Path | Docs-only |
| :--- | :--- |
| `README.md`, `README.xx.md` | yes |
| `AGENTS.md` (any directory) | yes |
| top-level `docs/**` and any `**/docs/**` under `src/` or `src-tauri/` | only `.md`/`.markdown`/image extensions |

Missing or unavailable base commits, manual runs, and nonempty `CI_REF` inputs force full validation (`code=true`). Invalid HEAD SHAs or diff errors fail `changes`, so the required gates cannot pass.

No `paths-ignore` exists on the trigger. Docs-only pushes and PRs run the `changes` job and the required aggregator gates; the heavy jobs skip, the gates report `success` against an expected `skipped`, so branch protection keeps a stable check set.

Workflow and script changes always require full validation. Editing only `docs/ci.md` runs the lightweight change detector and gates, not the heavy jobs.

## CI jobs

| Job | Runner | Runs when | Purpose |
| :--- | :--- | :--- | :--- |
| `changes` | ubuntu-24.04 | always | classify diff |
| `frontend` | ubuntu-24.04 | `code` | version check, `node --test` (release-version, ci-changes), `npm run check` (Biome only), Rust fmt (parent + helper), unit + component tests, `npm run build`, upload `frontend-${{ github.run_id }}` |
| `native-checks-windows` | windows-2022 | `code` | Clippy `-D warnings` + backend tests |
| `native-checks-macos` | macos-15 | `code` | helper fetch/check/test/clippy, backend clippy + tests |
| `native-package-windows` | windows-2022 | `code` | frontend artifact → NSIS build → exact-artifact assert |
| `native-package-macos` | macos-15 | `code` | frontend artifact → app + DMG, archive when `inputs.ref` set |
| `rust-msrv` | macos-15 | `code` | backend on 1.90.0, helper on 1.77.2 |
| `tauri-windows`, `tauri-macos` | ubuntu-24.04 | `always()` | required gates: frontend, platform checks/package and MSRV must be `success` (code) or `skipped` (docs-only) |

`frontend` builds the SPA once; packaging jobs download that artifact and pass `--config '{"build":{"beforeBuildCommand":null}}'` rather than rebuilding Vite.

Artifact uploads use `overwrite: true`, so rerunning jobs in the same workflow run can replace their earlier outputs without name conflicts.

Concurrency: `pull_request` → `<workflow>-pr-<number>` with `cancel-in-progress: true` (unless `inputs.ref` is set); all other events → `run-<run_id>`, never cancelled.

### Caching

- Node: `actions/setup-node@v4` `cache: npm` on `package-lock.json`. `node_modules` is never cached; `npm ci` reinstalls.
- Rust: `Swatinem/rust-cache@v2`, per-job `workspaces`, per-target `shared-key`, `cache-bin: false`. `target/` is never uploaded as an artifact.
- Keys include the Rust toolchain id and the lockfile hash, so a toolchain or dependency bump writes fresh entries.
- `save-if: ${{ inputs.ref == '' || github.ref == 'refs/heads/main' }}` — ordinary CI and main-branch release validation can save Rust caches; tag-triggered reusable validation only restores them. Failed builds may save compiled dependencies with `cache-on-failure: true`.

Ten GB is the repository cache budget; eviction is repo-wide, not per lane. An unauthenticated audit measured ~2.82 GiB before this change; the post-change figure is unmeasured.

### Toolchain pinning

CI, packaging and release pin Rust 1.96.0. `rust-msrv` separately verifies the declared minimums: parent 1.90.0, standalone macOS helper 1.77.2. Never build the parent with the helper toolchain.

## Release pipeline

Trigger: push to `main`, tag `v*`, or manual `workflow_dispatch`.

1. `prepare` (ubuntu): runs `scripts/release-version.test.mjs`, then `scripts/release-version.mjs prepare`, outputting `release`, `version`, `tag`, `sha`.
2. `validation`: calls this workflow via `uses: ./.github/workflows/ci.yml` with `ref: needs.prepare.outputs.sha`, validating the exact commit to be tagged.
3. `build` (macos-15-intel): Intel x64 DMG + app tarball, in parallel with `validation`. It builds its own frontend; only the CI ARM and Windows packages are reused at publish time.
4. `publish` (ubuntu) waits on `prepare`, `validation` and `build`, so one failing platform or gate publishes nothing. It downloads the Windows, macOS ARM and Intel artifacts, asserts `<tag>^{commit} == sha` and exactly 5 files (two DMGs, two app tarballs, x64 NSIS), then creates the draft release and uploads.

Release concurrency: `release-prepare-<repo>` and `release-publish-<tag>`, `cancel-in-progress: false`, `queue: max` so a queued release waits instead of cancelling.

### History and timing

The last 35 runs (23 CI, 12 Release) hold 17 step details; the rest have expired. Recent CI: macOS job 15m24s, frontend build 36s, Clippy ~2m24s, backend tests ~3m37s, Tauri package ~7m05s, post-cache step ~43s. Last Release 20m05s: Intel build 19m29s, bundle step 17m41s. The warm-run bottleneck is unknown — no auth or logs were available to attribute it.

## Verification

```bash
# Workflows. The only ignore is GitHub's official `queue` key, unknown to actionlint 1.7.12.
actionlint -shellcheck= -ignore '^unexpected key "queue" for "concurrency" section' \
  .github/workflows/ci.yml .github/workflows/release.yml

node --test scripts/release-version.test.mjs scripts/ci-changes.test.mjs
npm run test:unit && npm run test:components
npm run check
```

`-shellcheck=` turns off ShellCheck: optional, not installed by default, not part of the gate. Add no other ignores; no other warning is expected.

## Caveats

- `queue: max` is a GitHub Actions concurrency feature. The `node --test` runner does not validate workflow concurrency.
- Frontend type checking is not a gate: `npm run check` is Biome only, with no `tsc`/`svelte-check` step. That gap predates this pipeline and was not widened here.
