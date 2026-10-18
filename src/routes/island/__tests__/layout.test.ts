import { render } from '@testing-library/svelte';
import { createRawSnippet, tick } from 'svelte';
import { expect, test, vi } from 'vitest';
import Layout from '../../+layout.svelte';

const native = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn(() => true), listen: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke, isTauri: native.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/island') } }));
vi.mock('$src/config/adapter.js', async (importOriginal) => {
  const adapter = await importOriginal<typeof import('$src/config/adapter.js')>();
  return { ...adapter, createCommandAdapter: () => adapter.createTauriAdapter(native.invoke) };
});

test('island layout renders independently without AppShell, splash, config/catalog preload, or main/tray listeners', async () => {
  const view = render(Layout, {
    children: createRawSnippet(() => ({ render: () => '<span>Island child</span>' })),
  });
  await tick();
  expect(view.getByText('Island child')).toBeTruthy();
  expect(native.invoke).not.toHaveBeenCalled();
  expect(native.listen).not.toHaveBeenCalled();
  expect(view.container.querySelector('.app-shell')).toBeNull();
  expect(view.container.querySelector('[role="status"]')).toBeNull();
  expect(document.documentElement.classList.contains('island-window')).toBe(true);
  view.unmount();
  expect(document.documentElement.classList.contains('island-window')).toBe(false);
});
