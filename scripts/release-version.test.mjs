import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import {
  generateChangelogSection,
  parseVersion,
  prepareRelease,
  releaseNotes,
  selectVersion,
  syncVersions,
  VERSION_FILES,
  verifyVersions,
} from './release-version.mjs';

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'mom-release-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const files = {
    'package.json': '{"name":"opencode-mom","version":"0.1.0","dependencies":{"other":"0.1.0"}}\n',
    'package-lock.json': `${JSON.stringify(
      {
        name: 'opencode-mom',
        version: '0.1.0',
        lockfileVersion: 3,
        packages: {
          '': { name: 'opencode-mom', version: '0.1.0' },
          'node_modules/other': { version: '0.1.0', integrity: 'unchanged' },
        },
      },
      null,
      2,
    )}\n`,
    'src-tauri/tauri.conf.json': '{"version":"0.1.0","bundle":{"macOS":{"signingIdentity":"-"}}}\n',
    'src-tauri/Info.plist':
      '<?xml version="1.0"?><plist version="1.0"><dict>\n' +
      '<key>CFBundleShortVersionString</key><string>0.1.0</string>\n' +
      '<key>CFBundleVersion</key><string>0.1.0</string>\n</dict></plist>\n',
    'src-tauri/Cargo.toml': '[package]\nname = "opencode-mom"\nversion = "0.1.0"\n\n[dependencies]\nother = "0.1.0"\n',
    'src-tauri/Cargo.lock':
      'version = 4\n\n[[package]]\nname = "other"\nversion = "0.1.0"\nsource = "registry+example"\n\n' +
      '[[package]]\nname = "opencode-mom"\nversion = "0.1.0"\ndependencies = ["other"]\n\n' +
      '[[package]]\nname = "opencode-mom"\nversion = "0.0.9"\nsource = "registry+example"\n',
    'src-tauri/native/macos-helper/Cargo.toml': '[package]\nname = "opencode-lid-helper"\nversion = "0.1.0"\n',
    'src-tauri/native/macos-helper/Cargo.lock':
      'version = 4\n\n[[package]]\nname = "opencode-lid-helper"\nversion = "0.1.0"\n',
  };
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return { root, files };
}

function repository(t, subject = 'release') {
  const { root, files } = fixture(t);
  const sha = 'a'.repeat(40);
  const releaseSha = 'b'.repeat(40);
  const state = {
    head: sha,
    defaultHead: sha,
    tags: {},
    calls: [],
    race: false,
    message: '',
    subjects: [subject],
    date: '2026-10-07',
    lastStableTag: null,
    changed: [...VERSION_FILES, 'CHANGELOG.md'],
  };
  const runGit = (...args) => {
    state.calls.push(args);
    const [command, ...rest] = args;
    if (command === 'check-ref-format' || command === 'fetch' || command === 'add') return '';
    if (command === 'status') return '';
    if (command === 'tag') return Object.keys(state.tags).join('\n');
    if (command === 'describe') {
      if (!state.lastStableTag) throw new Error('No matching tag');
      return state.lastStableTag;
    }
    if (command === 'log') return state.subjects.join('\n');
    if (command === 'rev-parse') {
      if (rest[0] === 'HEAD') return state.head;
      if (rest[0] === 'refs/remotes/origin/main') return state.defaultHead;
      return state.tags[rest[0].replace(/\^\{commit\}$/, '')]?.sha ?? '';
    }
    if (command === 'show') {
      const object = rest.at(-1);
      if (rest.includes('--format=%cs')) return state.date;
      if (rest[0] === '-s') return object === sha ? subject : state.message;
      const path = object.slice(object.indexOf(':') + 1);
      return state.tags[Object.keys(state.tags).find((tag) => state.tags[tag].sha === releaseSha)].files[path].trim();
    }
    if (command === 'rev-list') return `${releaseSha} ${state.tags[Object.keys(state.tags)[0]].parent}`;
    if (command === 'diff') return state.changed.join('\n');
    if (command === '-c' && rest.includes('commit')) {
      state.head = releaseSha;
      state.message = rest.at(-1);
      return '';
    }
    if (command === '-c' && rest.includes('tag')) {
      const tag = rest[rest.indexOf('tag') + 1];
      state.tags[tag] = {
        sha: releaseSha,
        parent: sha,
        files: Object.fromEntries(
          [...VERSION_FILES, 'CHANGELOG.md'].map((path) => [path, readFileSync(join(root, path), 'utf8')]),
        ),
      };
      return '';
    }
    if (command === 'push') {
      assert.ok(rest.includes('--atomic'));
      assert.ok(rest.includes(`--force-with-lease=refs/heads/main:${sha}`));
      if (state.race) throw new Error('Atomic push rejected: default branch changed');
      state.defaultHead = releaseSha;
      return '';
    }
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };
  const options = {
    root,
    eventName: 'push',
    event: { after: sha },
    ref: 'refs/heads/main',
    sha,
    defaultBranch: 'main',
    runGit,
  };
  const restoreTrigger = () => {
    state.head = sha;
    for (const [path, text] of Object.entries(files)) writeFileSync(join(root, path), text);
    rmSync(join(root, 'CHANGELOG.md'), { force: true });
  };
  return { root, files, state, options, restoreTrigger };
}

test('release boundary, description patch bump, stable explicit version and manual selection', () => {
  for (const subject of ['release', 'release: description', 'release fix', 'release:']) {
    assert.equal(selectVersion('0.1.0', { subject }), '0.1.1');
  }
  assert.equal(selectVersion('0.1.0', { subject: 'release: 1.2.3' }), '1.2.3');
  assert.equal(selectVersion('1.9.9', { subject: 'release: 1.10.0' }), '1.10.0');
  assert.equal(selectVersion('0.1.0', { manual: true }), '0.1.1');
  assert.equal(selectVersion('0.1.0', { manual: true, version: '2.0.0' }), '2.0.0');
  for (const subject of [
    'releases',
    'released',
    'prerelease',
    'Release',
    ' release',
    'release_candidate',
    'chore: release',
  ]) {
    assert.equal(selectVersion('0.1.0', { subject }), null);
  }
});

test('invalid, older and unchanged explicit versions fail instead of becoming patch releases', () => {
  for (const version of [
    '1.2',
    'v1.2.3',
    'V1.2.3',
    '01.2.3',
    '1.2.3-beta',
    '1.2.3+meta',
    '1.2.3 extra',
    '-1.2.3',
    '0.1.0',
    '0.0.9',
  ]) {
    assert.throws(() => selectVersion('0.1.0', { subject: `release: ${version}` }));
    assert.throws(() => selectVersion('0.1.0', { manual: true, version }));
  }
  assert.throws(() => parseVersion('9007199254740992.0.0'));
  assert.throws(() => parseVersion('1.2.3\n'));
  assert.throws(() => selectVersion('0.1.9007199254740991', { subject: 'release' }));
  assert.throws(() => selectVersion('0.1.0', { manual: true, version: 'banana' }));
});

test('version components respect the Windows 16-bit bound in parsing and release selection', () => {
  const maximum = '65535.65535.65535';
  assert.deepEqual(parseVersion(maximum), [65535, 65535, 65535]);
  assert.equal(selectVersion('0.1.0', { subject: `release: ${maximum}` }), maximum);
  assert.equal(selectVersion('0.1.0', { manual: true, version: maximum }), maximum);
  for (const version of ['65536.1.1', '1.65536.1', '1.1.65536']) {
    assert.throws(() => parseVersion(version), /0\.\.65535/);
    assert.throws(() => selectVersion('0.1.0', { subject: `release: ${version}` }), /0\.\.65535/);
    assert.throws(() => selectVersion('0.1.0', { manual: true, version }), /0\.\.65535/);
  }
  assert.throws(() => selectVersion('0.1.65535', { subject: 'release' }), /0\.\.65535/);
  assert.throws(() => selectVersion('0.1.65535', { manual: true }), /0\.\.65535/);
});

test('beta versions compare numerically, increment within the channel, and require explicit stable promotion', () => {
  assert.deepEqual(parseVersion('2.0.1-beta.1'), [2, 0, 1, 1]);
  assert.equal(selectVersion('2.0.0', { subject: 'release: 2.0.1-beta.1' }), '2.0.1-beta.1');
  assert.equal(selectVersion('2.0.1-beta.9', { subject: 'release' }), '2.0.1-beta.10');
  assert.equal(selectVersion('2.0.1-beta.9', { manual: true }), '2.0.1-beta.10');
  assert.equal(selectVersion('2.0.1-beta.9', { manual: true, version: '2.0.1' }), '2.0.1');
  assert.equal(selectVersion('2.0.1-beta.9', { manual: true, version: '2.0.2-beta.1' }), '2.0.2-beta.1');
  for (const [current, version] of [
    ['2.0.1-beta.9', '2.0.1-beta.8'],
    ['2.0.1-beta.9', '2.0.1-beta.9'],
    ['2.0.1-beta.9', '2.0.0'],
    ['2.0.1', '2.0.1-beta.10'],
  ]) {
    assert.throws(() => selectVersion(current, { manual: true, version }), /must be newer/);
  }
});

test('beta syntax and native macOS limits fail closed instead of silently releasing stable', () => {
  for (const version of [
    '2.0.1-beta',
    '2.0.1-beta.0',
    '2.0.1-beta.01',
    '2.0.1-beta.-1',
    '2.0.1-beta.1.1',
    '2.0.1-beta.1+build',
    '2.0.1-alpha.1',
    '2.0.1-rc.1',
    '2.0.1-beta.256',
    '0.1.1-beta.1',
    '10000.0.1-beta.1',
    '1.100.1-beta.1',
    '1.0.100-beta.1',
  ]) {
    assert.throws(() => parseVersion(version), undefined, version);
    assert.throws(() => selectVersion('2.0.0', { subject: `release: ${version}` }), undefined, version);
  }
  assert.throws(() => parseVersion('2.0.1-beta.1\n'));
  assert.deepEqual(parseVersion('9999.99.99-beta.255'), [9999, 99, 99, 255]);
  assert.throws(() => selectVersion('2.0.1-beta.255', { manual: true }), /1\.\.255/);
});

test('out-of-range release requests abort before git mutations or fixture writes', (t) => {
  for (const version of ['65536.1.1', '1.65536.1', '1.1.65536', '1.0.1-beta.256', '1.0.100-beta.1', '']) {
    for (const manual of [false, true]) {
      const { root, files, state, options } = repository(t, version ? `release: ${version}` : 'release');
      if (!version) syncVersions(root, '0.1.65535');
      const before = Object.fromEntries(
        Object.keys(files).map((path) => [path, readFileSync(join(root, path), 'utf8')]),
      );
      const request = manual ? { ...options, eventName: 'workflow_dispatch', event: { inputs: { version } } } : options;
      assert.throws(() => prepareRelease(request), /0\.\.65535|macOS bundle limits/);
      assert.ok(
        state.calls.every((args) => !['add', 'commit', 'tag', 'push'].includes(args[0] === '-c' ? args[2] : args[0])),
      );
      assert.equal(state.head, options.sha);
      assert.deepEqual(state.tags, {});
      for (const [path, text] of Object.entries(before)) assert.equal(readFileSync(join(root, path), 'utf8'), text);
    }
  }
});

test('synchronizes app versions only, preserving dependency versions and standalone helper', (t) => {
  const { root, files } = fixture(t);
  syncVersions(root, '1.2.3');
  assert.equal(verifyVersions(root, 'v1.2.3'), '1.2.3');
  const lock = JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8'));
  assert.equal(lock.version, '1.2.3');
  assert.equal(lock.packages[''].version, '1.2.3');
  assert.deepEqual(lock.packages['node_modules/other'], { version: '0.1.0', integrity: 'unchanged' });
  assert.equal(JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).dependencies.other, '0.1.0');
  assert.equal(
    readFileSync(join(root, 'src-tauri/Cargo.lock'), 'utf8'),
    files['src-tauri/Cargo.lock'].replace(
      'name = "opencode-mom"\nversion = "0.1.0"',
      'name = "opencode-mom"\nversion = "1.2.3"',
    ),
  );
  assert.equal(
    readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8'),
    files['src-tauri/Cargo.toml'].replace('version = "0.1.0"', 'version = "1.2.3"'),
  );
  for (const path of Object.keys(files).filter((path) => path.includes('macos-helper'))) {
    assert.equal(readFileSync(join(root, path), 'utf8'), files[path]);
  }
  assert.equal(
    JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8')).bundle.macOS.signingIdentity,
    '-',
  );
});

test('tag verification checks every app source and rejects malformed tags or inconsistent input before writes', (t) => {
  const { root, files } = fixture(t);
  assert.equal(verifyVersions(root, 'v0.1.0'), '0.1.0');
  for (const tag of ['0.1.0', 'v0.1.1', 'v0.1.0-beta']) assert.throws(() => verifyVersions(root, tag));
  const path = 'src-tauri/tauri.conf.json';
  writeFileSync(join(root, path), files[path].replace('0.1.0', '0.2.0'));
  assert.throws(() => verifyVersions(root, 'v0.1.0'), /out of sync/);
  assert.throws(() => syncVersions(root, '0.3.0'), /out of sync/);
  assert.equal(readFileSync(join(root, 'package.json'), 'utf8'), files['package.json']);
});

test('beta app sources retain SemVer while macOS metadata uses the native beta suffix and detects drift', (t) => {
  const { root, files } = fixture(t);
  syncVersions(root, '2.0.1-beta.1');
  assert.equal(verifyVersions(root, 'v2.0.1-beta.1'), '2.0.1-beta.1');
  assert.throws(() => verifyVersions(root, 'v2.0.1'), /does not match/);
  const path = join(root, 'src-tauri/Info.plist');
  const plist = readFileSync(path, 'utf8');
  assert.match(plist, /<key>CFBundleShortVersionString<\/key><string>2\.0\.1<\/string>/);
  assert.match(plist, /<key>CFBundleVersion<\/key><string>2\.0\.1b1<\/string>/);
  const output = join(root, 'github_output');
  const stdout = execFileSync('node', [resolve('scripts/release-version.mjs'), 'verify', 'v2.0.1-beta.1'], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, GITHUB_OUTPUT: output },
  });
  assert.deepEqual(JSON.parse(stdout), { version: '2.0.1-beta.1', tag: 'v2.0.1-beta.1', prerelease: 'true' });
  assert.match(readFileSync(output, 'utf8'), /(?:^|\n)prerelease=true\n/);
  for (const value of ['2.0.1</string>', '2.0.1b1</string>']) {
    writeFileSync(path, plist.replace(value, '2.0.0</string>'));
    assert.throws(() => verifyVersions(root), /out of sync/);
    assert.throws(() => syncVersions(root, '2.0.1-beta.2'), /out of sync/);
  }
  writeFileSync(path, plist);
  syncVersions(root, '2.0.1');
  assert.equal(verifyVersions(root, 'v2.0.1'), '2.0.1');
  assert.ok(!readFileSync(path, 'utf8').includes('b1'));
  for (const path of Object.keys(files).filter((path) => path.includes('macos-helper'))) {
    assert.equal(readFileSync(join(root, path), 'utf8'), files[path]);
  }
});

test('verification supports Windows CRLF and detects drift in each app source', (t) => {
  const { root, files } = fixture(t);
  for (const [path, text] of Object.entries(files)) writeFileSync(join(root, path), text.replaceAll('\n', '\r\n'));
  assert.equal(verifyVersions(root, 'v0.1.0'), '0.1.0');
  for (const path of VERSION_FILES) {
    const changed =
      path === 'src-tauri/Cargo.lock'
        ? files[path].replace('name = "opencode-mom"\nversion = "0.1.0"', 'name = "opencode-mom"\nversion = "0.2.0"')
        : files[path].replace('0.1.0', '0.2.0');
    writeFileSync(join(root, path), changed);
    assert.throws(() => verifyVersions(root, 'v0.1.0'), /out of sync/);
    writeFileSync(join(root, path), files[path]);
  }
  const lock = JSON.parse(files['package-lock.json']);
  lock.packages[''].version = '0.2.0';
  writeFileSync(join(root, 'package-lock.json'), JSON.stringify(lock));
  assert.throws(() => verifyVersions(root, 'v0.1.0'), /out of sync/);
});

test('changelog sections contain a dated version header, subject bullets and a trailing newline', () => {
  assert.equal(
    generateChangelogSection({ version: '1.2.3', date: '2026-10-07', subjects: ['fix: crash', 'feat: updates'] }),
    '## 1.2.3 - 2026-10-07\n\n- fix: crash\n- feat: updates\n',
  );
});

test('empty changelog sections contain only the header and a blank line', () => {
  assert.equal(
    generateChangelogSection({ version: '1.2.3', date: '2026-10-07', subjects: [] }),
    '## 1.2.3 - 2026-10-07\n\n',
  );
});

test('preparation inserts the newest section after the header and logs commits since the last stable tag', (t) => {
  const { root, state, options } = repository(t);
  const older = '## 0.1.0 - 2026-10-01\n\n- Initial release\n';
  writeFileSync(join(root, 'CHANGELOG.md'), `# Changelog\n\n${older}`);
  state.lastStableTag = 'v0.1.0';
  state.tags['v0.1.0'] = { sha: options.sha };
  state.subjects = ['release', 'fix: crash'];
  state.date = '2026-10-06';
  prepareRelease(options);
  assert.equal(
    readFileSync(join(root, 'CHANGELOG.md'), 'utf8'),
    `# Changelog\n\n## 0.1.1 - 2026-10-06\n\n- release\n- fix: crash\n\n${older}`,
  );
  assert.deepEqual(
    state.calls.find((args) => args[0] === 'log'),
    ['log', '--format=%s', '--no-merges', 'v0.1.0..HEAD'],
  );
  assert.ok(state.calls.some((args) => args.join(' ') === 'show -s --format=%cs HEAD'));
});

test('preparation creates a missing changelog, falls back to full history and stages it with version files', (t) => {
  const { root, state, options } = repository(t);
  prepareRelease(options);
  const changelog = '# Changelog\n\n## 0.1.1 - 2026-10-07\n\n- release\n\n';
  assert.equal(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'), changelog);
  assert.equal(state.tags['v0.1.1'].files['CHANGELOG.md'], changelog);
  assert.equal(
    state.calls.find((args) => args[0] === 'describe'),
    undefined,
  );
  assert.deepEqual(
    state.calls.find((args) => args[0] === 'log'),
    ['log', '--format=%s', '--no-merges', 'HEAD'],
  );
  assert.deepEqual(
    state.calls.find((args) => args[0] === 'add'),
    ['add', '--', ...VERSION_FILES, 'CHANGELOG.md'],
  );
  assert.ok(!VERSION_FILES.includes('CHANGELOG.md'));
});

test('preparation prepends the header and new section when an existing changelog has no header', (t) => {
  const { root, options } = repository(t);
  const older = '## 0.1.0 - 2026-10-01\n\n- Initial release\n';
  writeFileSync(join(root, 'CHANGELOG.md'), older);
  prepareRelease(options);
  assert.equal(
    readFileSync(join(root, 'CHANGELOG.md'), 'utf8'),
    `# Changelog\n\n## 0.1.1 - 2026-10-07\n\n- release\n\n${older}`,
  );
});

test('preparation excludes release commit subjects while keeping normal subjects', (t) => {
  const { root, state, options } = repository(t);
  state.subjects = ['release', 'chore(release): v1.2.3', 'fix: crash', 'docs: explain chore(release): commits', ''];
  prepareRelease(options);
  assert.equal(
    readFileSync(join(root, 'CHANGELOG.md'), 'utf8'),
    '# Changelog\n\n## 0.1.1 - 2026-10-07\n\n- release\n- fix: crash\n- docs: explain chore(release): commits\n\n',
  );
});

test('reruns allow changelog changes without regenerating it and still reject unrelated changed files', (t) => {
  const { root, state, options, restoreTrigger } = repository(t);
  const result = prepareRelease(options);
  restoreTrigger();
  assert.ok(state.changed.includes('CHANGELOG.md'));
  assert.deepEqual(prepareRelease(options), result);
  assert.throws(() => readFileSync(join(root, 'CHANGELOG.md')), /ENOENT/);
  assert.equal(state.calls.filter((args) => args[0] === 'log').length, 1);
  assert.equal(state.calls.filter((args) => args[0] === 'push').length, 1);
  state.changed.push('README.md');
  assert.throws(() => prepareRelease(options), /not this run/);
});

test('branch preparation requests an atomic leased push; reruns reuse its exact release child tag', (t) => {
  const { root, state, options, restoreTrigger } = repository(t);
  const result = prepareRelease(options);
  assert.equal(result.tag, 'v0.1.1');
  assert.equal(result.release, 'true');
  assert.notEqual(result.sha, options.sha);
  assert.equal(verifyVersions(root, result.tag), '0.1.1');
  assert.equal(state.defaultHead, result.sha);
  assert.equal(state.tags[result.tag].sha, result.sha);
  assert.ok(state.calls.some((args) => args.includes('chore(release): v0.1.1')));
  restoreTrigger();
  assert.deepEqual(prepareRelease(options), result);
  state.defaultHead = 'c'.repeat(40);
  assert.deepEqual(prepareRelease(options), result);
  assert.equal(state.calls.filter((args) => args[0] === 'push').length, 1);
});

test('nonrelease pushes skip and commit text is never executed', (t) => {
  const nonrelease = repository(t, 'released: 1.2.3');
  assert.deepEqual(prepareRelease(nonrelease.options), { release: 'false' });
  assert.equal(verifyVersions(nonrelease.root), '0.1.0');
  assert.throws(() => readFileSync(join(nonrelease.root, 'CHANGELOG.md')), /ENOENT/);
  const injected = repository(t, 'release: $(touch injected)');
  assert.equal(prepareRelease(injected.options).tag, 'v0.1.1');
  assert.throws(() => readFileSync(join(injected.root, 'injected')), /ENOENT/);
  assert.ok(injected.state.calls.every((args) => !args.some((arg) => arg.includes('$(touch injected)'))));
});

test('changed default-branch HEAD fails without committing or tagging', (t) => {
  const { root, state, options } = repository(t);
  const newHead = 'c'.repeat(40);
  state.defaultHead = newHead;
  assert.throws(() => prepareRelease(options), /Default branch changed/);
  assert.equal(state.head, options.sha);
  assert.deepEqual(state.tags, {});
  assert.equal(state.defaultHead, newHead);
  assert.equal(verifyVersions(root), '0.1.0');
});

test('a rejected atomic push is surfaced instead of returning a buildable release', (t) => {
  const { state, options } = repository(t);
  state.race = true;
  assert.throws(() => prepareRelease(options), /Atomic push rejected/);
  assert.equal(state.defaultHead, options.sha);
});

test('unrelated existing tags and versions behind existing releases cannot be reused', (t) => {
  const existing = repository(t);
  existing.state.tags['v0.1.1'] = { sha: existing.options.sha, parent: 'c'.repeat(40) };
  assert.throws(() => prepareRelease(existing.options), /not this run/);
  const older = repository(t);
  older.state.tags['v1.0.0'] = { sha: older.options.sha };
  assert.throws(() => prepareRelease(older.options), /not newer than/);
  assert.equal(verifyVersions(older.root), '0.1.0');
});

test('reruns reject a same-parent tag containing unexpected version-source content', (t) => {
  const { state, options, restoreTrigger } = repository(t);
  const result = prepareRelease(options);
  state.tags[result.tag].files['package.json'] += 'unexpected';
  restoreTrigger();
  assert.throws(() => prepareRelease(options), /not this run/);
  assert.equal(state.calls.filter((args) => args[0] === 'push').length, 1);
});

test('dispatch requires the default branch, accepts an explicit stable version, and supports reruns', (t) => {
  const { options, restoreTrigger } = repository(t, 'ordinary work');
  const dispatch = { ...options, eventName: 'workflow_dispatch', event: { inputs: { version: '2.0.0' } } };
  assert.throws(() => prepareRelease({ ...dispatch, ref: 'refs/heads/v2.0' }), /default branch/);
  assert.throws(() => prepareRelease({ ...dispatch, event: { inputs: { version: '0.1.0' } } }), /must be newer/);
  const result = prepareRelease(dispatch);
  assert.equal(result.tag, 'v2.0.0');
  restoreTrigger();
  assert.deepEqual(prepareRelease(dispatch), result);
});

test('existing tag builds verify versions and never bump', (t) => {
  const { root, state, options } = repository(t, 'ordinary work');
  state.tags['v0.1.0'] = { sha: options.sha };
  const tagPush = { ...options, ref: 'refs/tags/v0.1.0' };
  assert.deepEqual(prepareRelease(tagPush), {
    release: 'true',
    version: '0.1.0',
    tag: 'v0.1.0',
    prerelease: 'false',
    sha: options.sha,
  });
  assert.equal(state.defaultHead, options.sha);
  assert.ok(state.calls.every((args) => args[0] !== 'push'));
  assert.throws(() => readFileSync(join(root, 'CHANGELOG.md')), /ENOENT/);
  assert.throws(() => prepareRelease({ ...tagPush, ref: 'refs/tags/v0.2.0' }), /does not match/);
});

test('beta preparation, reruns and existing tag builds always report prerelease=true', (t) => {
  const { root, state, options, restoreTrigger } = repository(t, 'release: 2.0.1-beta.1');
  const result = prepareRelease(options);
  assert.equal(result.prerelease, 'true');
  assert.equal(result.tag, 'v2.0.1-beta.1');
  assert.equal(verifyVersions(root), '2.0.1-beta.1');
  restoreTrigger();
  assert.deepEqual(prepareRelease(options), result);
  syncVersions(root, result.version);
  state.head = result.sha;
  assert.deepEqual(prepareRelease({ ...options, ref: `refs/tags/${result.tag}`, sha: result.sha }), result);
  assert.equal(state.calls.filter((args) => args[0] === 'push').length, 1);
});

test('beta dry runs do not write, reject non-monotonic tags and use earlier beta history only for beta releases', (t) => {
  const { root, state, options } = repository(t);
  const dispatch = {
    ...options,
    eventName: 'workflow_dispatch',
    event: { inputs: { version: '2.0.1-beta.10', dry_run: true } },
  };
  state.tags = { 'v2.0.0': {}, 'v2.0.1-beta.9': {} };
  state.lastStableTag = 'v2.0.1-beta.9';
  const result = prepareRelease(dispatch);
  assert.equal(result.release, 'false');
  assert.equal(result.prerelease, 'true');
  assert.equal(verifyVersions(root), '0.1.0');
  assert.ok(state.calls.some((args) => args.join(' ') === 'log --format=%s --no-merges v2.0.1-beta.9..HEAD'));
  assert.ok(!state.calls.some((args) => args[0] === 'push' || args[0] === '-c'));
  state.tags['v2.0.1-beta.11'] = {};
  assert.throws(() => prepareRelease(dispatch), /not newer than v2.0.1-beta.11/);
  delete state.tags['v2.0.1-beta.11'];
  state.tags['v2.0.1'] = {};
  assert.throws(() => prepareRelease(dispatch), /not newer than v2.0.1/);
  delete state.tags['v2.0.1'];
  state.calls = [];
  state.lastStableTag = 'v2.0.0';
  const stable = prepareRelease({ ...dispatch, event: { inputs: { version: '2.0.1', dry_run: true } } });
  assert.equal(stable.prerelease, 'false');
  const stableLog = state.calls.filter((args) => args[0] === 'log').at(-1);
  assert.deepEqual(stableLog, ['log', '--format=%s', '--no-merges', 'v2.0.0..HEAD']);
});

test('beta release notes select the exact beta section', (t) => {
  const { root } = fixture(t);
  syncVersions(root, '2.0.1-beta.1');
  writeFileSync(
    join(root, 'CHANGELOG.md'),
    '# Changelog\n\n## 2.0.1 - 2026-10-09\n\n- stable\n\n## 2.0.1-beta.1 - 2026-10-07\n\n- beta\n\n## 2.0.0 - 2026-10-01\n\n- old\n',
  );
  const runGit = (...args) => {
    if (args[0] === 'status') return '';
    if (args[0] === 'rev-parse') return 'a'.repeat(40);
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };
  assert.equal(releaseNotes({ root, tag: 'v2.0.1-beta.1', runGit }), '## 2.0.1-beta.1 - 2026-10-07\n\n- beta\n');
});

test('publish workflow keeps beta prerelease and non-latest on creation and reruns, failing closed on missing channel', (t) => {
  const workflow = readFileSync(resolve('.github/workflows/release.yml'), 'utf8');
  const publish = workflow.split('      - name: Publish only after every platform succeeded\n')[1];
  const script = publish.split('        run: |\n')[1].replace(/^ {10}/gm, '');
  for (const [channel, exists] of [
    ['true', false],
    ['true', true],
    ['false', false],
    ['', false],
  ]) {
    const { root } = fixture(t);
    const bin = join(root, 'bin');
    mkdirSync(bin);
    const scripts = {
      git: '#!/bin/sh\nprintf "%s\\n" "$RELEASE_SHA"\n',
      node: '#!/bin/sh\nprintf "Release notes\\n"\n',
      gh: '#!/bin/sh\nprintf "%s\\n" "$*" >> "$GH_LOG"\nif [ "$2" = view ]; then exit "$GH_EXISTS"; fi\n',
    };
    for (const [name, text] of Object.entries(scripts)) {
      writeFileSync(join(bin, name), text);
      chmodSync(join(bin, name), 0o755);
    }
    mkdirSync(join(root, 'release-assets'));
    const version = channel === 'false' ? '2.0.1' : '2.0.1-beta.1';
    for (const suffix of ['aarch64.dmg', 'x64.dmg', 'aarch64.app.tar.gz', 'x64.app.tar.gz', 'x64-setup.exe']) {
      writeFileSync(join(root, 'release-assets', `opencode-mom_${version}_${suffix}`), 'fixture');
    }
    const log = join(root, 'gh.log');
    const run = () =>
      execFileSync('bash', ['-c', script], {
        cwd: root,
        env: {
          ...process.env,
          PATH: `${bin}:${process.env.PATH}`,
          PRERELEASE: channel,
          RELEASE_VERSION: version,
          RELEASE_TAG: `v${version}`,
          RELEASE_SHA: 'a'.repeat(40),
          RUNNER_TEMP: root,
          GH_TOKEN: '',
          GH_LOG: log,
          GH_EXISTS: exists ? '0' : '1',
        },
        stdio: 'pipe',
      });
    if (!channel) {
      assert.throws(run);
      assert.throws(() => readFileSync(log), /ENOENT/);
      continue;
    }
    run();
    const calls = readFileSync(log, 'utf8').trim().split('\n');
    for (const call of calls.filter((call) => /^release (create|edit) /.test(call))) {
      assert.ok(call.includes(`--prerelease=${channel}`), call);
      if (channel === 'true') assert.ok(call.includes('--latest=false'), call);
    }
    assert.equal(calls.filter((call) => call.startsWith('release create ')).length, exists ? 0 : 1);
    assert.ok(calls.some((call) => call.startsWith('release edit ') && call.includes('--draft=false')));
  }
});

test('releaseNotes extracts exact version section without bleeding into adjacent sections across CRLF and LF', (t) => {
  const { root } = fixture(t);
  const headSha = 'a'.repeat(40);
  const runGit = (...args) => {
    if (args[0] === 'status') return '';
    if (args[0] === 'rev-parse') return headSha;
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };

  // Middle section with newer section before and older section after
  const changelog =
    '# Changelog\r\n\r\n' +
    '## 0.2.0 - 2026-10-10\r\n\r\n' +
    '- feat: new thing\r\n\r\n' +
    '## 0.1.0 - 2026-10-01\r\n\r\n' +
    '- initial release\r\n\r\n' +
    '## 0.0.9 - 2026-09-01\r\n\r\n' +
    '- legacy\r\n';
  writeFileSync(join(root, 'CHANGELOG.md'), changelog);

  const notes = releaseNotes({ root, tag: 'v0.1.0', runGit });
  assert.equal(notes, '## 0.1.0 - 2026-10-01\r\n\r\n- initial release\n');
  assert.ok(!notes.includes('0.2.0'));
  assert.ok(!notes.includes('0.0.9'));

  // Unix LF variant
  writeFileSync(join(root, 'CHANGELOG.md'), changelog.replaceAll('\r\n', '\n'));
  const notesLf = releaseNotes({ root, tag: 'v0.1.0', runGit });
  assert.equal(notesLf, '## 0.1.0 - 2026-10-01\n\n- initial release\n');
  assert.ok(!notesLf.includes('0.2.0'));
  assert.ok(!notesLf.includes('0.0.9'));
});

test('releaseNotes falls back to git commit history excluding target and newer tags when section is missing', (t) => {
  const { root } = fixture(t);
  const headSha = 'a'.repeat(40);
  const logCalls = [];
  const runGit = (...args) => {
    const [command, ...rest] = args;
    if (command === 'status') return '';
    if (command === 'rev-parse') return headSha;
    if (command === 'tag') {
      // Returns older tag v0.0.9, target tag v0.1.0, and newer tag v0.2.0
      return 'v0.0.9\nv0.1.0\nv0.2.0\n';
    }
    if (command === 'log') {
      logCalls.push(args);
      return 'feat: unreleased change\nchore(release): v0.1.0\nfix: something\n';
    }
    if (command === 'show') {
      if (rest.includes('--format=%cs')) return '2026-10-15';
      if (rest[0] === '-s') return 'feat: commit';
    }
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };

  // Missing CHANGELOG.md
  const fallbackNotes = releaseNotes({ root, tag: 'v0.1.0', runGit });
  assert.equal(fallbackNotes, '## 0.1.0 - 2026-10-15\n\n- feat: unreleased change\n- fix: something\n');
  assert.deepEqual(logCalls[0], ['log', '--format=%s', '--no-merges', 'v0.0.9..HEAD']);
  assert.deepEqual(logCalls.length, 1);

  // Existing CHANGELOG.md but missing requested tag
  writeFileSync(join(root, 'CHANGELOG.md'), '# Changelog\n\n## 0.0.9 - 2026-09-01\n\n- old\n');
  const fallbackNotes2 = releaseNotes({ root, tag: 'v0.1.0', runGit });
  assert.equal(fallbackNotes2, '## 0.1.0 - 2026-10-15\n\n- feat: unreleased change\n- fix: something\n');
});

test('release notes baseline uses the newest release tag even when it is not an ancestor', (t) => {
  const { root } = fixture(t);
  const headSha = 'a'.repeat(40);
  const logCalls = [];
  const runGit = (...args) => {
    const [command, ...rest] = args;
    if (command === 'status') return '';
    if (command === 'rev-parse') return headSha;
    // A history rewrite orphans v2.0.0: no tag is an ancestor of HEAD any more.
    if (command === 'tag') return args.includes('--merged') ? '' : 'v0.0.6\nv2.0.0\nv2.0.2\n';
    if (command === 'log') {
      logCalls.push(args);
      return 'feat: reload on tray group switch\nfix(release): repair CI gates\n';
    }
    if (command === 'show') {
      if (rest.includes('--format=%cs')) return '2026-11-10';
      if (rest[0] === '-s') return 'chore(release): v2.0.1';
    }
    if (command === 'rev-list') return `${headSha} ${'b'.repeat(40)}`;
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };
  syncVersions(root, '2.0.1');
  assert.equal(
    releaseNotes({ root, tag: 'v2.0.1', runGit }),
    '## 2.0.1 - 2026-11-10\n\n- feat: reload on tray group switch\n- fix(release): repair CI gates\n',
  );
  // v2.0.0 is the newest tag below 2.0.1 even though it lives on another branch; v2.0.2 is newer and skipped.
  assert.deepEqual(logCalls[0], ['log', '--format=%s', '--no-merges', 'v2.0.0..HEAD']);
});

test('releaseNotes propagates git errors and non-ENOENT filesystem errors', (t) => {
  const { root } = fixture(t);
  const headSha = 'a'.repeat(40);
  const runGit = (...args) => {
    if (args[0] === 'status') return '';
    if (args[0] === 'rev-parse') return headSha;
    if (args[0] === 'tag') throw new Error('git tag fatal error');
    throw new Error(`Unexpected git call: ${args.join(' ')}`);
  };

  // Git failure during fallback history collection must propagate, not be swallowed
  assert.throws(() => releaseNotes({ root, tag: 'v0.1.0', runGit }), /git tag fatal error/);

  // Create CHANGELOG.md as a directory to trigger EISDIR
  const runGitClean = (...args) => {
    if (args[0] === 'status') return '';
    if (args[0] === 'rev-parse') return headSha;
    return '';
  };
  mkdirSync(join(root, 'CHANGELOG.md'));
  assert.throws(
    () => releaseNotes({ root, tag: 'v0.1.0', runGit: runGitClean }),
    (err) => err.code === 'EISDIR',
  );
});

test('releaseNotes throws on malformed tag or checkout mismatch', (t) => {
  const { root } = fixture(t);
  assert.throws(() => releaseNotes({ root, tag: '0.1.0' }), /Expected a release tag/);
  assert.throws(() => releaseNotes({ root, tag: 'v0.1.0-beta' }), /Expected a release tag/);

  const runGitMismatch = (...args) => {
    if (args[0] === 'status') return '';
    if (args[0] === 'rev-parse') {
      return args[1].includes('HEAD') ? 'a'.repeat(40) : 'b'.repeat(40);
    }
    return '';
  };
  assert.throws(
    () => releaseNotes({ root, tag: 'v0.1.0', runGit: runGitMismatch }),
    /Checkout must match the requested tag/,
  );

  const runGitDirty = (...args) => {
    if (args[0] === 'status') return ' M CHANGELOG.md\n';
    return '';
  };
  assert.throws(
    () => releaseNotes({ root, tag: 'v0.1.0', runGit: runGitDirty }),
    /Release notes require clean version files and CHANGELOG\.md/,
  );
});

test('prepareRelease dry_run returns release=false, version, tag, sha and notes without mutations or git writes', (t) => {
  for (const dryRunVal of [true, 'true']) {
    const { root, files, state, options } = repository(t, 'release: 0.2.0');
    // Seed an origin remote commit to verify dry-run does not advance it
    state.defaultHead = options.sha;
    const beforeFiles = Object.fromEntries(Object.keys(files).map((p) => [p, readFileSync(join(root, p), 'utf8')]));

    const dispatch = {
      ...options,
      eventName: 'workflow_dispatch',
      event: { inputs: { version: '0.2.0', dry_run: dryRunVal } },
    };

    const result = prepareRelease(dispatch);

    assert.equal(result.release, 'false');
    assert.equal(result.version, '0.2.0');
    assert.equal(result.tag, 'v0.2.0');
    assert.equal(result.sha, options.sha);
    assert.ok(result.notes?.startsWith('## 0.2.0 - '));

    // Verify no git write calls
    const writes = state.calls.filter((args) => {
      const cmd = args[0] === '-c' ? args[2] : args[0];
      return ['add', 'commit', 'push'].includes(cmd) || (cmd === 'tag' && !args.includes('--list'));
    });
    assert.deepEqual(writes, []);
    assert.equal(state.head, options.sha);
    assert.equal(state.defaultHead, options.sha);
    assert.deepEqual(state.tags, {});

    // Verify no tracked files or CHANGELOG written
    for (const [p, content] of Object.entries(beforeFiles)) {
      assert.equal(readFileSync(join(root, p), 'utf8'), content);
    }
    assert.throws(() => readFileSync(join(root, 'CHANGELOG.md')), /ENOENT/);
  }
});

test('prepareRelease dry_run strict coercion: false, "false", or undefined runs normal release', (t) => {
  for (const dryRunVal of [false, 'false', undefined]) {
    const { state, options } = repository(t, 'release: 0.2.0');
    const dispatch = {
      ...options,
      eventName: 'workflow_dispatch',
      event: { inputs: { version: '0.2.0', dry_run: dryRunVal } },
    };

    const result = prepareRelease(dispatch);
    assert.equal(result.release, 'true');
    assert.equal(result.tag, 'v0.2.0');
    assert.ok(state.calls.some((args) => args[0] === 'push'));
  }
});

test('cli notes and dry_run commands work in real git worktree without push or corruption', (t) => {
  const { root } = fixture(t);
  writeFileSync(join(root, 'CHANGELOG.md'), '# Changelog\n\n## 0.1.0 - 2026-10-01\n\n- initial release\n');
  const outputFile = join(root, 'github_output');
  writeFileSync(outputFile, '');

  const env = { ...process.env, HUSKY: '0', GITHUB_OUTPUT: outputFile };
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', env }).trim();
  git('-c', 'init.defaultBranch=main', 'init');
  git('config', 'user.name', 'test');
  git('config', 'user.email', 'test@example.com');
  git('-c', 'commit.gpgsign=false', 'add', '.');
  git('-c', 'commit.gpgsign=false', 'commit', '--no-verify', '-m', 'initial');
  git('-c', 'tag.gpgSign=false', 'tag', 'v0.1.0');

  // Verify CLI notes command prints raw markdown directly to stdout with no GITHUB_OUTPUT pollution
  const stdout = execFileSync('node', [resolve('scripts/release-version.mjs'), 'notes', 'v0.1.0'], {
    cwd: root,
    encoding: 'utf8',
    env,
  });
  assert.equal(stdout, '## 0.1.0 - 2026-10-01\n\n- initial release\n');
  assert.equal(readFileSync(outputFile, 'utf8'), '');

  const originDir = mkdtempSync(join(tmpdir(), 'mom-origin-'));
  t.after(() => rmSync(originDir, { recursive: true, force: true }));
  execFileSync('git', ['init', '--bare', originDir], { encoding: 'utf8', env });
  execFileSync(
    'git',
    ['--git-dir', originDir, 'fetch', root, 'refs/heads/main:refs/heads/main', 'refs/tags/v0.1.0:refs/tags/v0.1.0'],
    { encoding: 'utf8', env },
  );
  git('remote', 'add', 'origin', originDir);

  const headBefore = git('rev-parse', 'HEAD');
  const eventPayloadDir = mkdtempSync(join(tmpdir(), 'mom-event-'));
  t.after(() => rmSync(eventPayloadDir, { recursive: true, force: true }));
  const eventPayloadPath = join(eventPayloadDir, 'event.json');
  writeFileSync(eventPayloadPath, JSON.stringify({ inputs: { version: '0.2.0', dry_run: true } }));

  const dryRunEnv = {
    ...env,
    GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_EVENT_PATH: eventPayloadPath,
    GITHUB_REF: 'refs/heads/main',
    GITHUB_SHA: headBefore,
    DEFAULT_BRANCH: 'main',
  };

  const dryRunStdout = execFileSync('node', [resolve('scripts/release-version.mjs'), 'prepare'], {
    cwd: root,
    encoding: 'utf8',
    env: dryRunEnv,
  });
  const dryRunOutput = JSON.parse(dryRunStdout.split('\n')[0]);
  assert.equal(dryRunOutput.release, 'false');
  assert.equal(dryRunOutput.version, '0.2.0');
  assert.equal(dryRunOutput.tag, 'v0.2.0');
  assert.equal(dryRunOutput.sha, headBefore);

  const githubOutput = readFileSync(outputFile, 'utf8');
  assert.ok(githubOutput.includes('release=false\n'));
  assert.ok(!githubOutput.includes('notes='));

  assert.equal(git('rev-parse', 'HEAD'), headBefore);
  assert.ok(!git('tag', '--list').split('\n').includes('v0.2.0'));
  const originTags = execFileSync('git', ['--git-dir', originDir, 'tag', '--list'], {
    encoding: 'utf8',
    env,
  }).trim();
  assert.ok(!originTags.includes('v0.2.0'));
});
