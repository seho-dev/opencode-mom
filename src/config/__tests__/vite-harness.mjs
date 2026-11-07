import { createServer as createViteServer } from 'vite';

export const DEFAULT_VITE_OPTIONS = {
  server: { middlewareMode: true },
  appType: 'custom',
  logLevel: 'error',
  optimizeDeps: { noDiscovery: true },
};

export async function createServer(options = {}) {
  return await createViteServer({
    ...DEFAULT_VITE_OPTIONS,
    ...options,
    server: {
      ...DEFAULT_VITE_OPTIONS.server,
      ...options.server,
    },
  });
}

export async function withServer(fn, options = {}) {
  const vite = await createServer(options);

  try {
    return await fn(vite);
  } finally {
    await vite.close();
  }
}

export function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}
