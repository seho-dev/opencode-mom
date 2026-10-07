import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';
import {
  generateChangelogSection,
  parseVersion,
  prepareRelease,
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

test('out-of-range release requests abort before git mutations or fixture writes', (t) => {
  for (const version of ['65536.1.1', '1.65536.1', '1.1.65536', '']) {
    for (const manual of [false, true]) {
      const { root, files, state, options } = repository(t, version ? `release: ${version}` : 'release');
      if (!version) syncVersions(root, '0.1.65535');
      const before = Object.fromEntries(
        Object.keys(files).map((path) => [path, readFileSync(join(root, path), 'utf8')]),
      );
      const request = manual ? { ...options, eventName: 'workflow_dispatch', event: { inputs: { version } } } : options;
      assert.throws(() => prepareRelease(request), /0\.\.65535/);
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
    ['log', '--format=%s', 'v0.1.0..HEAD'],
  );
  assert.ok(state.calls.some((args) => args.join(' ') === 'show -s --format=%cs HEAD'));
});

test('preparation creates a missing changelog, falls back to full history and stages it with version files', (t) => {
  const { root, state, options } = repository(t);
  prepareRelease(options);
  const changelog = '# Changelog\n\n## 0.1.1 - 2026-10-07\n\n- release\n\n';
  assert.equal(readFileSync(join(root, 'CHANGELOG.md'), 'utf8'), changelog);
  assert.equal(state.tags['v0.1.1'].files['CHANGELOG.md'], changelog);
  assert.deepEqual(
    state.calls.find((args) => args[0] === 'describe'),
    ['describe', '--tags', '--match', 'v[0-9]*', '--abbrev=0'],
  );
  assert.deepEqual(
    state.calls.find((args) => args[0] === 'log'),
    ['log', '--format=%s', 'HEAD'],
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
  assert.deepEqual(prepareRelease(tagPush), { release: 'true', version: '0.1.0', tag: 'v0.1.0', sha: options.sha });
  assert.equal(state.defaultHead, options.sha);
  assert.ok(state.calls.every((args) => args[0] !== 'push'));
  assert.throws(() => readFileSync(join(root, 'CHANGELOG.md')), /ENOENT/);
  assert.throws(() => prepareRelease({ ...tagPush, ref: 'refs/tags/v0.2.0' }), /does not match/);
});
