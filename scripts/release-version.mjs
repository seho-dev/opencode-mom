import { execFileSync } from 'node:child_process';
import { appendFileSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const VERSION_FILES = [
  'package.json',
  'package-lock.json',
  'src-tauri/Cargo.toml',
  'src-tauri/Cargo.lock',
  'src-tauri/tauri.conf.json',
];

export function parseVersion(version) {
  if (
    typeof version !== 'string' ||
    version.trim() !== version ||
    !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)
  ) {
    throw new Error(`Expected a stable version (X.Y.Z), got ${JSON.stringify(version)}`);
  }
  const parts = version.split('.').map(Number);
  if (!parts.every((part) => Number.isSafeInteger(part) && part <= 65535)) {
    throw new Error('Version components must be integers in 0..65535 (Windows VERSIONINFO limit)');
  }
  return parts;
}

function compareVersions(left, right) {
  const a = parseVersion(left);
  const b = parseVersion(right);
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) return a[i] > b[i] ? 1 : -1;
  }
  return 0;
}

export function selectVersion(current, { subject = '', version = '', manual = false } = {}) {
  const parts = parseVersion(current);
  if (!manual && !/^release\b/.test(subject)) return null;
  let explicit = version.trim();
  if (!manual) {
    const payload = /^release:\s*(.*)$/.exec(subject)?.[1].trim() ?? '';
    // Numeric-looking subjects are version requests, not descriptions with a silent fallback.
    if (/^[vV]?[+-]?\d/.test(payload)) explicit = payload;
  }
  const next = explicit || `${parts[0]}.${parts[1]}.${parts[2] + 1}`;
  parseVersion(next);
  if (compareVersions(next, current) <= 0) throw new Error(`Version ${next} must be newer than ${current}`);
  return next;
}

function tomlValue(block, key) {
  const matches = [...block.matchAll(new RegExp(`^${key}\\s*=\\s*"([^"\\r\\n]+)"[^\\r\\n]*\\r?$`, 'gm'))];
  if (matches.length !== 1) throw new Error(`Expected exactly one TOML ${key}`);
  return matches[0][1];
}

function manifestPackage(text) {
  const matches = [...text.matchAll(/^\[package\]\r?\n[\s\S]*?(?=^\[|$(?![\s\S]))/gm)];
  if (matches.length !== 1) throw new Error('Expected exactly one Cargo [package]');
  return matches[0][0];
}

function lockPackage(text) {
  const blocks = text.match(/^\[\[package\]\]\r?\n[\s\S]*?(?=^\[\[package\]\]|$(?![\s\S]))/gm) ?? [];
  const matches = blocks.filter((block) => tomlValue(block, 'name') === 'opencode-mom' && !/^source\s*=/m.test(block));
  if (matches.length !== 1) throw new Error('Expected exactly one local opencode-mom Cargo.lock package');
  return matches[0];
}

function sources(root, read = (path) => readFileSync(resolve(root, path), 'utf8')) {
  return Object.fromEntries(VERSION_FILES.map((path) => [path, read(path)]));
}

function sourceVersion(files, tag) {
  const pkg = JSON.parse(files['package.json']);
  const lock = JSON.parse(files['package-lock.json']);
  const config = JSON.parse(files['src-tauri/tauri.conf.json']);
  const manifest = manifestPackage(files['src-tauri/Cargo.toml']);
  if (pkg.name !== 'opencode-mom' || lock.name !== pkg.name || tomlValue(manifest, 'name') !== pkg.name) {
    throw new Error('Unexpected app package name');
  }
  const versions = [
    pkg.version,
    lock.version,
    lock.packages?.['']?.version,
    config.version,
    tomlValue(manifest, 'version'),
    tomlValue(lockPackage(files['src-tauri/Cargo.lock']), 'version'),
  ];
  for (const version of versions) parseVersion(version);
  if (!versions.every((version) => version === pkg.version)) {
    throw new Error(`App versions are out of sync: ${versions.join(', ')}`);
  }
  if (tag !== undefined && tag !== `v${pkg.version}`) throw new Error(`Tag ${tag} does not match v${pkg.version}`);
  return pkg.version;
}

export function verifyVersions(root, tag, read) {
  return sourceVersion(sources(root, read), tag);
}

function updatedSources(files, version) {
  parseVersion(version);
  sourceVersion(files);
  const updated = { ...files };
  for (const path of ['package.json', 'src-tauri/tauri.conf.json']) {
    updated[path] = files[path].replace(/"version"\s*:\s*"[^"]+"/, `"version": "${version}"`);
  }
  const lock = JSON.parse(files['package-lock.json']);
  lock.version = version;
  lock.packages[''].version = version;
  updated['package-lock.json'] = `${JSON.stringify(lock, null, 2)}\n`;
  for (const [path, block] of [
    ['src-tauri/Cargo.toml', manifestPackage(files['src-tauri/Cargo.toml'])],
    ['src-tauri/Cargo.lock', lockPackage(files['src-tauri/Cargo.lock'])],
  ]) {
    updated[path] = files[path].replace(block, block.replace(/^(version\s*=\s*")[^"]+/m, `$1${version}`));
  }
  sourceVersion(updated, `v${version}`);
  return updated;
}

export function syncVersions(root, version) {
  const updated = updatedSources(sources(root), version);
  for (const [path, text] of Object.entries(updated)) writeFileSync(resolve(root, path), text);
}

export function generateChangelogSection({ version, date, subjects }) {
  return `## ${version} - ${date}\n\n${subjects.map((subject) => `- ${subject}\n`).join('')}`;
}

export function prepareRelease({ root, eventName, event = {}, ref, sha, defaultBranch, runGit }) {
  const git =
    runGit ??
    ((...args) =>
      execFileSync('git', args, { cwd: root, encoding: 'utf8', env: { ...process.env, HUSKY: '0' } }).trim());
  const skip = { release: 'false' };
  if (!defaultBranch) throw new Error('Missing default branch');
  git('check-ref-format', `refs/heads/${defaultBranch}`);
  if (!/^[a-f0-9]{40,64}$/.test(sha ?? '') || git('rev-parse', 'HEAD') !== sha) {
    throw new Error('Checkout must match the triggering commit');
  }
  if (eventName === 'push' && ref.startsWith('refs/tags/')) {
    const tag = ref.slice('refs/tags/'.length);
    const version = verifyVersions(root, tag);
    if (git('rev-parse', `${tag}^{commit}`) !== sha) throw new Error('Tag no longer matches the triggering commit');
    return { release: 'true', version, tag, sha };
  }
  if (ref !== `refs/heads/${defaultBranch}`) {
    if (eventName === 'workflow_dispatch') throw new Error('Dispatch releases only from the default branch');
    return skip;
  }
  if (!['push', 'workflow_dispatch'].includes(eventName)) return skip;
  if (eventName === 'push' && event.after !== sha) throw new Error('Push SHA does not match the checkout');
  const files = sources(root);
  const current = sourceVersion(files);
  const version = selectVersion(current, {
    subject: git('show', '-s', '--format=%s', sha),
    manual: eventName === 'workflow_dispatch',
    version: event.inputs?.version ?? '',
  });
  if (version === null) return skip;
  if (git('status', '--porcelain')) throw new Error('Release preparation requires a clean checkout');
  git('fetch', '--tags', 'origin', `refs/heads/${defaultBranch}:refs/remotes/origin/${defaultBranch}`);
  const tag = `v${version}`;
  const message = `chore(release): ${tag}`;
  const updated = updatedSources(files, version);
  const tags = git('tag', '--list').split('\n');
  if (tags.includes(tag)) {
    const taggedSha = git('rev-parse', `${tag}^{commit}`);
    const parents = git('rev-list', '--parents', '-n', '1', taggedSha).split(' ');
    const changed = git('diff', '--name-only', sha, taggedSha).split('\n');
    const COMMIT_FILES = [...VERSION_FILES, 'CHANGELOG.md'];
    // A rerun may reuse only the exact release child commit of its original trigger.
    if (
      parents.length !== 2 ||
      parents[1] !== sha ||
      git('show', '-s', '--format=%s', taggedSha) !== message ||
      changed.some((path) => !COMMIT_FILES.includes(path)) ||
      VERSION_FILES.some((path) => git('show', `${taggedSha}:${path}`) !== updated[path].trim())
    ) {
      throw new Error(`Existing ${tag} is not this run's release commit`);
    }
    return { release: 'true', version, tag, sha: taggedSha };
  }
  if (git('rev-parse', `refs/remotes/origin/${defaultBranch}`) !== sha) {
    throw new Error('Default branch changed since this run started; release from its new HEAD');
  }
  for (const existing of tags) {
    if (/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(existing)) {
      if (compareVersions(version, existing.slice(1)) <= 0) throw new Error(`${tag} is not newer than ${existing}`);
    }
  }
  let range = 'HEAD';
  try {
    range = `${git('describe', '--tags', '--match', 'v[0-9]*', '--abbrev=0')}..HEAD`;
  } catch {
    // The first release has no matching tag; include its full history.
  }
  const subjects = git('log', '--format=%s', range)
    .split('\n')
    .filter((subject) => subject && !subject.startsWith('chore(release):'));
  const section = generateChangelogSection({ version, date: git('show', '-s', '--format=%cs', 'HEAD'), subjects });
  const changelogPath = resolve(root, 'CHANGELOG.md');
  let changelog = '# Changelog\n';
  try {
    changelog = readFileSync(changelogPath, 'utf8');
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
  const header = /^# Changelog(?:\r?\n|$)(?:\r?\n)*/m;
  const entry = `# Changelog\n\n${section}\n`;
  changelog = header.test(changelog) ? changelog.replace(header, () => entry) : entry + changelog;
  for (const [path, text] of Object.entries(updated)) writeFileSync(resolve(root, path), text);
  writeFileSync(changelogPath, changelog);
  git('add', '--', ...VERSION_FILES, 'CHANGELOG.md');
  git('-c', 'commit.gpgsign=false', 'commit', '--no-verify', '-m', message);
  const releaseSha = git('rev-parse', 'HEAD');
  git('-c', 'tag.gpgSign=false', 'tag', tag, releaseSha);
  // Atomic compare-and-swap: neither ref moves if the branch or tag changed meanwhile.
  git(
    'push',
    '--atomic',
    `--force-with-lease=refs/heads/${defaultBranch}:${sha}`,
    'origin',
    `${releaseSha}:refs/heads/${defaultBranch}`,
    `refs/tags/${tag}:refs/tags/${tag}`,
  );
  return { release: 'true', version, tag, sha: releaseSha };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    let result;
    if (process.argv[2] === 'verify') {
      const tag = process.argv[3];
      if (!tag) throw new Error('Usage: node scripts/release-version.mjs verify vX.Y.Z');
      result = { version: verifyVersions(process.cwd(), tag) };
    } else if (process.argv[2] === 'prepare') {
      result = prepareRelease({
        root: process.cwd(),
        eventName: process.env.GITHUB_EVENT_NAME,
        event: JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8')),
        ref: process.env.GITHUB_REF,
        sha: process.env.GITHUB_SHA,
        defaultBranch: process.env.DEFAULT_BRANCH,
      });
    } else {
      throw new Error('Usage: node scripts/release-version.mjs prepare | verify vX.Y.Z');
    }
    if (process.env.GITHUB_OUTPUT) {
      appendFileSync(
        process.env.GITHUB_OUTPUT,
        Object.entries(result)
          .map(([key, value]) => `${key}=${value}\n`)
          .join(''),
      );
    }
    console.log(JSON.stringify(result));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
