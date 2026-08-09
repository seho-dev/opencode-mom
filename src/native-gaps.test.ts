import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const characterize = process.env["OMO_CHARACTERIZE_NATIVE_GAPS"] === "1";
const gapSuite = characterize ? describe : describe.skip;

const read = (path: string): string => readFileSync(resolve(path), "utf8");

const activeProductFiles = [
  "package.json",
  "package-lock.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
  "src-tauri/src",
  "src-tauri/tests",
  "src-tauri/capabilities",
  "src-tauri/permissions",
  "src",
  ".github/workflows",
] as const;

gapSuite("Task 5 native and release gaps", () => {
  it("removes updater dependencies, commands, types, UI, config, and workflow inputs", () => {
    const retainedSources = [
      read("src-tauri/Cargo.toml"),
      read("src-tauri/src/lib.rs"),
      read("src-tauri/src/core/system/mod.rs"),
      read("src-tauri/Cargo.lock"),
      read("src-tauri/src/commands/system.rs"),
      read("src/lib/tauriClient.ts"),
      read("src-tauri/tauri.conf.json"),
      read(".github/workflows/ci.yml"),
    ].join("\n");

    expect(retainedSources.toLowerCase()).not.toContain("updater");
    expect(retainedSources).not.toMatch(
      /check_for_updates|install_update|Updates|Check for Updates/,
    );
    expect(activeProductFiles).not.toContain(".omo");

    const ignoredDirectories = read(".gitignore").split(/\r?\n/);
    expect(ignoredDirectories).toEqual(
      expect.arrayContaining([
        ".omo/",
        "node_modules/",
        ".svelte-kit/",
        "build/",
        "dist/",
        "src-tauri/target/",
      ]),
    );

    const readme = read("README.md");
    expect(readme).toContain(
      "$env:OMO_CHARACTERIZE_NATIVE_GAPS='1'; npm test -- --run src/native-gaps.test.ts",
    );
    expect(readme).toContain(
      "OMO_CHARACTERIZE_NATIVE_GAPS=1 npm test -- --run src/native-gaps.test.ts",
    );
    expect(readme).not.toMatch(
      /^npm test -- --run src\/native-gaps\.test\.ts$/m,
    );
  });

  it("uses the approved product identity and synchronized release version", () => {
    const tauri = JSON.parse(read("src-tauri/tauri.conf.json")) as {
      readonly productName: string;
      readonly version: string;
      readonly identifier: string;
    };
    const packageJson = JSON.parse(read("package.json")) as {
      readonly name: string;
      readonly version: string;
    };
    const packageLock = JSON.parse(read("package-lock.json")) as {
      readonly name: string;
      readonly version: string;
      readonly packages: Readonly<
        Record<string, Readonly<{ name?: string; version?: string }>>
      >;
    };
    const cargo = read("src-tauri/Cargo.toml");

    expect(tauri).toMatchObject({
      productName: "omo-switch",
      version: "0.1.0",
      identifier: "dev.seho.omo-switch",
    });
    expect(packageJson).toMatchObject({
      name: "omo-switch",
      version: tauri.version,
    });
    expect(packageLock).toMatchObject({
      name: "omo-switch",
      version: tauri.version,
    });
    expect(packageLock.packages[""]).toMatchObject({
      name: "omo-switch",
      version: tauri.version,
    });
    expect(cargo).toContain('name = "omo-switch"');
    expect(cargo).toContain(`version = "${tauri.version}"`);
    expect(cargo).toContain('default-run = "omo-switch"');
    expect(cargo).not.toMatch(/spike|0\.0\.0|omo-switch-tauri/);
  });

  it("sets an explicit CSP and complete non-empty icon inputs", () => {
    const tauri = JSON.parse(read("src-tauri/tauri.conf.json")) as {
      readonly app: { readonly security: { readonly csp: string | null } };
      readonly bundle: { readonly icon: readonly string[] };
    };

    expect(tauri.app.security.csp).toEqual(expect.any(String));
    expect(tauri.app.security.csp).not.toHaveLength(0);
    expect(tauri.bundle.icon.length).toBeGreaterThan(0);
    for (const icon of tauri.bundle.icon) {
      expect(existsSync(resolve("src-tauri", icon))).toBe(true);
    }
  });

  it("defines least-privilege capabilities for quick-switch and settings windows", () => {
    expect(
      existsSync(resolve("src-tauri/capabilities/quick-switch.json")),
    ).toBe(true);
    expect(existsSync(resolve("src-tauri/capabilities/settings.json"))).toBe(
      true,
    );

    const quickSwitch = read("src-tauri/capabilities/quick-switch.json");
    const settings = read("src-tauri/capabilities/settings.json");
    const capabilityText = `${quickSwitch}\n${settings}`;

    expect(quickSwitch).toContain("quick-switch");
    expect(settings).toContain("settings");
    expect(capabilityText).not.toMatch(
      /"windows"\s*:\s*\[\s*"\*"|core:default|\*/,
    );
  });

  it("pins NSIS, macOS app, and DMG targets with product-specific artifact names", () => {
    const tauri = JSON.parse(read("src-tauri/tauri.conf.json")) as {
      readonly bundle: { readonly targets: readonly string[] };
    };
    const workflow = read(".github/workflows/ci.yml");
    const releaseWorkflow = read(".github/workflows/release.yml");

    expect(tauri.bundle.targets).toEqual(["nsis", "app", "dmg"]);
    expect(workflow).toContain("omo-switch_0.1.0_x64-setup.exe");
    expect(workflow).toContain("omo-switch.app");
    expect(workflow).toContain("omo-switch_0.1.0_aarch64.dmg");
    expect(workflow).not.toContain("tauri-spike");

    expect(releaseWorkflow).toContain("runs-on: macos-15");
    expect(releaseWorkflow).toContain('$expectedTag = "v$version"');
    expect(releaseWorkflow).toContain('expected_tag="v${version}"');
    expect(releaseWorkflow.match(/GITHUB_REF_NAME/g)).toHaveLength(4);
    expect(releaseWorkflow).toContain(
      "src-tauri/target/release/bundle/nsis/omo-switch_0.1.0_x64-setup.exe",
    );
    expect(releaseWorkflow).toContain(
      "tar -C 'src-tauri/target/release/bundle/macos' -czf 'release-assets/omo-switch_0.1.0_aarch64.app.tar.gz' 'omo-switch.app'",
    );
    expect(releaseWorkflow).toContain(
      "src-tauri/target/release/bundle/dmg/omo-switch_0.1.0_aarch64.dmg",
    );
    expect(releaseWorkflow).toContain(
      "release-assets/windows/omo-switch_0.1.0_x64-setup.exe",
    );
    expect(releaseWorkflow).toContain(
      "release-assets/macos/omo-switch_0.1.0_aarch64.app.tar.gz",
    );
    expect(releaseWorkflow).toContain(
      "release-assets/macos/omo-switch_0.1.0_aarch64.dmg",
    );
    expect(releaseWorkflow).toContain("fail_on_unmatched_files: true");
    expect(releaseWorkflow).not.toMatch(/swift|xcode|xcodegen|swiftpm/i);
  });

  it("requires deterministic Windows and macOS Tauri pull request gates", () => {
    const workflow = read(".github/workflows/ci.yml");

    expect(workflow).toContain("pull_request:");
    expect(workflow).toContain("tauri-windows:");
    expect(workflow).toContain("runs-on: windows-latest");
    expect(workflow).toContain("tauri-macos:");
    expect(workflow).toContain("runs-on: macos-latest");
    expect(workflow.match(/run: npm ci/g)).toHaveLength(2);
    expect(workflow.match(/run: npm test\r?\n/g)).toHaveLength(2);
    expect(workflow.match(/run: npm run build/g)).toHaveLength(2);
    expect(
      workflow.match(
        /cargo fmt --manifest-path src-tauri\/Cargo.toml --all -- --check/g,
      ),
    ).toHaveLength(2);
    expect(
      workflow.match(
        /cargo test --manifest-path src-tauri\/Cargo.toml -- --nocapture/g,
      ),
    ).toHaveLength(2);
    expect(workflow.match(/OMO_CHARACTERIZE_NATIVE_GAPS: '1'/g)).toHaveLength(
      2,
    );
    expect(workflow.match(/npm run tauri build/g)).toHaveLength(2);
    expect(workflow).toContain("permissions:\n  contents: read");
    expect(workflow).not.toMatch(
      /contents:\s*write|releaseDraft|prerelease|APPLE_|TAURI_SIGNING|updater|ubuntu|linux/i,
    );
  });
});
