import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { isDocumentationOnly } from './ci-changes.mjs';

// realpath: macOS /tmp is a symlink, so argv[1] would not equal import.meta.url.
const SCRIPT = realpathSync(resolve('scripts/ci-changes.mjs'));

// --- pure classifier: one branch per case, so a miss cannot hide in a mix ---

const CLASSIFIER = [
  ['root README', 'README.md', true],
  ['localized README', 'README.zh-CN.md', true],
  ['root AGENTS', 'AGENTS.md', true],
  ['nested AGENTS', 'src-tauri/src/island/AGENTS.md', true],
  ['root docs', 'docs/guide.md', true],
  ['src nested docs', 'src/routes/island/docs/img/diagram.svg', true],
  ['src-tauri docs', 'src-tauri/src/docs/config-contracts.md', true],
  ['helper docs', 'src-tauri/native/macos-helper/docs/service-lifecycle.md', true],
  ['backend island docs image', 'src-tauri/src/island/docs/overlay.png', true],
  ['changelog is code', 'CHANGELOG.md', false],
  ['skill markdown is code', '.opencode/skills/x/SKILL.md', false],
  ['AGENTS casing is exact', 'src/agents.md', false],
  ['docs prefix needs the dir', 'docs2/note.md', false],
  ['src/docs.md is a file', 'src/docs.md', false],
  ['docs keeps only doc extensions', 'docs/script.js', false],
];

for (const [label, path, expected] of CLASSIFIER) {
  test(`classifier: ${label}`, () => {
    assert.equal(isDocumentationOnly([path]), expected, path);
  });
}

test('classifier: empty and mixed inputs are not documentation only', () => {
  assert.equal(isDocumentationOnly([]), false);
  assert.equal(isDocumentationOnly(['']), false);
  assert.equal(isDocumentationOnly(['README.md', 'src/config/adapter.ts']), false);
});

// --- fixture ---------------------------------------------------------------

function fixture(t) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), 'mom-ci-changes-')));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const env = { ...process.env, HUSKY: '0', GIT_CONFIG_NOSYSTEM: '1' };
  for (const key of Object.keys(env)) {
    if (/^GITHUB_|^CI$|^CI_REF$/.test(key)) delete env[key];
  }
  const git = (...args) =>
    execFileSync(
      'git',
      ['-c', 'commit.gpgsign=false', '-c', 'tag.gpgSign=false', '-c', 'core.quotepath=false', ...args],
      { cwd: root, encoding: 'utf8', env, stdio: ['ignore', 'pipe', 'pipe'] },
    );
  git('init', '-q', '-b', 'main');
  git('config', 'user.name', 'test');
  git('config', 'user.email', 'test@example.com');

  const commit = (message) => {
    git('add', '-A');
    git('commit', '-q', '--no-verify', '-m', message);
    return git('rev-parse', 'HEAD').trim();
  };
  const write = (path, content) => {
    const slash = path.lastIndexOf('/');
    if (slash > 0) mkdirSync(join(root, path.slice(0, slash)), { recursive: true });
    writeFileSync(join(root, path), content);
  };
  // Branch switching without `git checkout -b`, which the local git wrapper blocks.
  const useBranch = (name) => git('symbolic-ref', 'HEAD', `refs/heads/${name}`);
  return { root, env, git, commit, write, useBranch };
}

// Real payload on disk, empty CI_REF: exactly how GitHub invokes the script.
function run(t, state, { event, eventName, sha, extraEnv = {} }) {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), 'mom-ci-run-')));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const eventPath = join(dir, 'event.json');
  const outputPath = join(dir, 'github-output');
  writeFileSync(eventPath, JSON.stringify(event));
  writeFileSync(outputPath, '');

  const child = spawnSync(process.execPath, [SCRIPT], {
    cwd: state.root,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...state.env,
      GITHUB_EVENT_NAME: eventName,
      GITHUB_EVENT_PATH: eventPath,
      GITHUB_SHA: sha,
      GITHUB_OUTPUT: outputPath,
      ...extraEnv,
    },
  });
  const outputs = {};
  for (const line of readFileSync(outputPath, 'utf8').split('\n')) {
    const eq = line.indexOf('=');
    if (eq > 0) outputs[line.slice(0, eq)] = line.slice(eq + 1);
  }
  return { status: child.status, stderr: String(child.stderr ?? ''), outputs };
}

function push(t, state, base, head, extraEnv = {}) {
  return run(t, state, { event: { before: base }, eventName: 'push', sha: head, extraEnv });
}

function pullRequest(t, state, base, head) {
  return run(t, state, {
    event: { pull_request: { base: { sha: base } } },
    eventName: 'pull_request',
    sha: head,
  });
}

// Two-parent merge commit built with plumbing; equivalent to a clean `git merge`.
function mergeCommits(state, first, second) {
  const tree = state.git('merge-tree', '--write-tree', first, second).split('\n')[0].trim();
  return state.git('commit-tree', tree, '-p', first, '-p', second, '-m', `merge ${second}`).trim();
}

// --- CLI -------------------------------------------------------------------

test('cli: docs-only push resolves code=false', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  state.write('src/config/docs/command-contracts.md', 'a\n');
  const base = state.commit('one');
  state.write('README.md', 'b\n');
  state.write('src/routes/island/docs/presentation.md', 'b\n');
  const head = state.commit('two');

  const { status, stderr, outputs } = push(t, state, base, head);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false');
});

test('cli: backend and helper docs trees are doc-only', (t) => {
  const state = fixture(t);
  state.write('src-tauri/src/island/docs/native-window.md', 'a\n');
  state.write('src-tauri/src/power/docs/state.png', 'a\n');
  state.write('src-tauri/native/macos-helper/docs/service-lifecycle.md', 'a\n');
  const base = state.commit('one');
  state.write('src-tauri/src/island/docs/native-window.md', 'b\n');
  state.write('src-tauri/native/macos-helper/docs/protocol.md', 'b\n');
  const head = state.commit('two');

  const { status, stderr, outputs } = push(t, state, base, head);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false', stderr);
});

test('cli: code, changelog and skill markdown resolve code=true', (t) => {
  for (const changed of ['src/config/adapter.ts', 'CHANGELOG.md', '.opencode/skills/x/SKILL.md']) {
    const state = fixture(t);
    state.write('README.md', 'a\n');
    const base = state.commit('one');
    state.write(changed, 'b\n');
    const head = state.commit('two');

    const { status, stderr, outputs } = push(t, state, base, head);
    assert.equal(status, 0, `${changed}: ${stderr}`);
    assert.equal(outputs.code, 'true', changed);
  }
});

test('cli: push covers code committed before the latest commit', (t) => {
  const state = fixture(t);
  state.write('src/lib.ts', 'a\n');
  const base = state.commit('one');
  state.write('src/lib.ts', 'b\n');
  state.commit('two');
  state.write('src/routes/docs/editor-contracts.md', 'c\n');
  const head = state.commit('three');

  const { status, stderr, outputs } = push(t, state, base, head);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'true');
});

test('cli: pull_request diffs the PR range, not the merge commit alone', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  const base = state.commit('base');

  state.write('README.md', 'docs\n');
  const prHead = state.commit('pr docs only');
  state.write('src-tauri/src/main.rs', 'main\n');
  const mainHead = state.commit('unrelated main commit');
  const merge = mergeCommits(state, mainHead, prHead);

  // `mainHead..merge` is docs-only (the merge's own diff); only the full
  // `base...merge` range sees the earlier code commit.
  const { status, stderr, outputs } = pullRequest(t, state, base, merge);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'true');
});

test('cli: PR excluded from advanced main still reports only the PR docs', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  const base = state.commit('base');

  // main advances with code while the PR branch only touches docs.
  state.write('src/main.ts', 'main code\n');
  const mainHead = state.commit('main advanced code');

  // Branch off the PR base, so the merge keeps its real history.
  state.useBranch('feature');
  state.git('update-ref', 'refs/heads/feature', base);
  state.write('README.md', 'docs\n');
  const featureHead = state.commit('feature docs');

  const merge = mergeCommits(state, mainHead, featureHead);
  // Sanity: main's own code change is inside the merged tree, so a `base..merge`
  // diff includes it while `mainHead..merge` does not.
  assert.match(state.git('diff', '--name-only', `${base}..${merge}`, '--'), /src\/main\.ts/);
  assert.doesNotMatch(state.git('diff', '--name-only', `${mainHead}..${merge}`, '--'), /src\/main\.ts/);

  // Base is main's advanced tip: only the feature docs are new to main.
  const { status, stderr, outputs } = pullRequest(t, state, mainHead, merge);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false');
});

test('cli: pull_request with a docs-only PR resolves code=false', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  const base = state.commit('base');
  state.write('README.md', 'b\n');
  state.write('src/docs/notes.md', 'c\n');
  const head = state.commit('docs');

  const { status, stderr, outputs } = pullRequest(t, state, base, head);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false');
});

test('cli: code renamed into docs is full, docs renamed into docs stays docs', (t) => {
  const moved = fixture(t);
  moved.write('src/data.ts', 'x\n');
  const movedBase = moved.commit('one');
  moved.write('src/docs/data.ts', 'x\n');
  rmSync(join(moved.root, 'src/data.ts'));
  const movedHead = moved.commit('two');
  const movedRun = push(t, moved, movedBase, movedHead);
  assert.equal(movedRun.status, 0, movedRun.stderr);
  assert.equal(movedRun.outputs.code, 'true', 'the removed code path must count');

  const docs = fixture(t);
  docs.write('docs/old.md', 'x\n');
  const docsBase = docs.commit('one');
  docs.write('docs/new.md', 'x\n');
  rmSync(join(docs.root, 'docs/old.md'));
  const docsHead = docs.commit('two');
  const docsRun = push(t, docs, docsBase, docsHead);
  assert.equal(docsRun.status, 0, docsRun.stderr);
  assert.equal(docsRun.outputs.code, 'false');
});

test('cli: deleting docs is doc-only while deleting code is full', (t) => {
  const docs = fixture(t);
  docs.write('src/config/docs/a.md', 'x\n');
  docs.write('README.md', 'x\n');
  const docsBase = docs.commit('one');
  rmSync(join(docs.root, 'src/config/docs/a.md'));
  const docsHead = docs.commit('two');
  const docsRun = push(t, docs, docsBase, docsHead);
  assert.equal(docsRun.status, 0, docsRun.stderr);
  assert.equal(docsRun.outputs.code, 'false');

  const code = fixture(t);
  code.write('src/utils/index.ts', 'x\n');
  code.write('README.md', 'x\n');
  const codeBase = code.commit('one');
  rmSync(join(code.root, 'src/utils/index.ts'));
  const codeHead = code.commit('two');
  const codeRun = push(t, code, codeBase, codeHead);
  assert.equal(codeRun.status, 0, codeRun.stderr);
  assert.equal(codeRun.outputs.code, 'true');
});

test('cli: docs paths with spaces, quotes and glob characters stay doc-only', (t) => {
  const state = fixture(t);
  const names = ['docs/sp ace.md', 'docs/quote"name.md', 'docs/star*name.md', 'docs/brace{x}.svg'];
  for (const name of names) state.write(name, 'a\n');
  const base = state.commit('one');
  for (const name of names) state.write(name, 'b\n');
  const head = state.commit('two');

  const { status, stderr, outputs } = push(t, state, base, head);
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false', stderr);
});

test('cli: zero, missing and non-hex bases plus manual dispatch are full', (t) => {
  const zero = '0000000000000000000000000000000000000000';
  const cases = [
    ['zero base SHA', { event: { before: zero }, eventName: 'push' }],
    ['missing before', { event: {}, eventName: 'push' }],
    ['non-hex base', { event: { before: 'not-a-sha' }, eventName: 'push' }],
    ['manual dispatch', { event: {}, eventName: 'workflow_dispatch' }],
  ];

  for (const [label, options] of cases) {
    const state = fixture(t);
    state.write('README.md', 'a\n');
    state.commit('one');
    state.write('README.md', 'b\n');
    const head = state.commit('two');

    const { status, stderr, outputs } = run(t, state, { ...options, sha: head });
    assert.equal(status, 0, `${label}: ${stderr}`);
    assert.equal(outputs.code, 'true', label);
  }
});

test('cli: unknown but well-formed base SHA is full, not an error', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  const base = state.commit('one');
  state.write('README.md', 'b\n');
  const head = state.commit('two');
  const unknown = 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeef';

  const { status, stderr, outputs } = push(t, state, base, head, {});
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false', 'control: docs-only push is not full');

  const missing = push(t, state, unknown, head);
  assert.equal(missing.status, 0, missing.stderr);
  assert.equal(missing.outputs.code, 'true');
});

test('cli: nonempty CI_REF forces full even on a docs-only payload', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  state.write('src/config/docs/x.md', 'a\n');
  const base = state.commit('one');
  state.write('README.md', 'b\n');
  state.write('src/config/docs/x.md', 'b\n');
  const head = state.commit('two');

  const baseline = push(t, state, base, head);
  assert.equal(baseline.status, 0, baseline.stderr);
  assert.equal(baseline.outputs.code, 'false', 'same commit pair is docs-only without CI_REF');

  for (const ref of ['HEAD', base]) {
    const forced = push(t, state, base, head, { CI_REF: ref });
    assert.equal(forced.status, 0, `${ref}: ${forced.stderr}`);
    assert.equal(forced.outputs.code, 'true', ref);
  }
});

test('cli: empty CI_REF behaves as unset', (t) => {
  const state = fixture(t);
  state.write('README.md', 'a\n');
  const base = state.commit('one');
  state.write('README.md', 'b\n');
  const head = state.commit('two');

  const { status, stderr, outputs } = push(t, state, base, head, { CI_REF: '' });
  assert.equal(status, 0, stderr);
  assert.equal(outputs.code, 'false');
});

test('cli: invalid or unavailable head SHA exits nonzero without output', (t) => {
  const cases = [
    ['unavailable head', 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeef'],
    ['invalid format', 'not-a-sha'],
    ['zero head', '0000000000000000000000000000000000000000'],
  ];

  for (const [label, head] of cases) {
    const state = fixture(t);
    state.write('README.md', 'a\n');
    const base = state.commit('one');

    const { status, outputs } = push(t, state, base, head);
    assert.notEqual(status, 0, label);
    assert.equal(outputs.code, undefined, label);
  }
});
