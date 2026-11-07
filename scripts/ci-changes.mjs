import { execFileSync, spawnSync } from 'node:child_process';
import { appendFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function isDocumentationOnly(paths) {
  return (
    paths.length > 0 &&
    paths.every(
      (path) =>
        /^README(?:\.[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*)?\.md$/.test(path) ||
        /(?:^|\/)AGENTS\.md$/.test(path) ||
        (/^(?:docs\/|(?:src|src-tauri)\/(?:[^/]+\/)*docs\/)/.test(path) &&
          /\.(?:md|markdown|png|jpe?g|gif|svg|webp|avif|ico|bmp|tiff?)$/i.test(path)),
    )
  );
}

function requiresCode() {
  const eventName = process.env.GITHUB_EVENT_NAME;
  if (process.env.CI_REF || !['push', 'pull_request'].includes(eventName)) return true;

  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'));
  const base = eventName === 'pull_request' ? event.pull_request?.base?.sha : event.before;
  const head = process.env.GITHUB_SHA;
  const sha = /^[a-f0-9]{40,64}$/i;
  if (!sha.test(head ?? '') || /^0+$/.test(head)) throw new Error('Invalid GITHUB_SHA');
  if (typeof base !== 'string' || !sha.test(base) || /^0+$/.test(base)) return true;

  const available = spawnSync('git', ['cat-file', '-e', `${base}^{commit}`], { stdio: 'pipe' });
  if (available.error) throw available.error;
  if (available.status !== 0) return true;

  const range = `${base}${eventName === 'pull_request' ? '...' : '..'}${head}`;
  const paths = execFileSync('git', ['diff', '--name-only', '--no-renames', '-z', range, '--'], {
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
  }).split('\0');
  if (paths.at(-1) === '') paths.pop();
  return !isDocumentationOnly(paths);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const code = requiresCode();
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `code=${code}\n`);
    console.log(`code=${code}`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
