import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import tauriConfigText from '../../src-tauri/tauri.conf.json?raw';
import RootRoute from './+page.svelte';
import SettingsRoute from './settings/+page.svelte';

const { invoke } = vi.hoisted(() => ({
  invoke: vi.fn(() => Promise.reject(new Error('Tauri IPC is unavailable in route tests.')))
}));
const currentPage = vi.hoisted(() => ({
  url: new URL('http://localhost/')
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('$app/state', () => ({
  page: {
    get url() {
      return currentPage.url;
    }
  }
}));

type TauriConfig = Readonly<{
  app: Readonly<{
    windows: readonly Readonly<{
      label: string;
      url?: string;
    }>[];
  }>;
}>;

const tauriConfig = JSON.parse(tauriConfigText) as TauriConfig;

const renderRootRoute = (path: string) => {
  cleanup();
  invoke.mockClear();
  window.history.replaceState({}, '', path);
  currentPage.url = new URL(path, 'http://localhost');
  return render(RootRoute);
};

const renderSettingsRoute = (path: string) => {
  cleanup();
  invoke.mockClear();
  window.history.replaceState({}, '', path);
  currentPage.url = new URL(path, 'http://localhost');
  return render(SettingsRoute);
};

afterEach(() => {
  cleanup();
  invoke.mockClear();
  window.history.replaceState({}, '', '/');
  currentPage.url = new URL('http://localhost/');
});

describe('route bootstrapping', () => {
  it('Given the explicit Quick Switch fixture query When the root route mounts Then it composes the fixture without invoking Tauri IPC', async () => {
    await renderRootRoute('/?fixture=quick-switch');

    expect(await screen.findByRole('main', { name: 'Default' })).toBeInstanceOf(HTMLElement);
    expect(document.querySelector('[data-fixture-root="quick-switch"]')).toBeInstanceOf(HTMLElement);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('Given an unsupported root fixture query When the root route mounts Then it remains the product Quick Switch route', async () => {
    await renderRootRoute('/?fixture=ui-primitives');

    expect(document.querySelector('main[data-surface="quick-switch"]')).toBeInstanceOf(HTMLElement);
    expect(document.querySelector('[data-fixture-root]')).toBeNull();
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('load_app_state'));
  });

  it('Given the explicit Settings fixture query When the settings route mounts Then it composes the fixture without invoking Tauri IPC', async () => {
    await renderSettingsRoute('/settings?fixture=settings');

    expect(await screen.findByRole('heading', { name: 'Group Settings' })).toBeInstanceOf(HTMLElement);
    expect(document.querySelector('[data-fixture-root="settings"]')).toBeInstanceOf(HTMLElement);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('Given a degraded query without the Settings fixture When the settings route mounts Then it remains the product route', async () => {
    await renderSettingsRoute('/settings?degraded=opencode');

    expect(document.querySelector('[data-fixture-root]')).toBeNull();
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('load_app_state'));
  });

  it('Given an explicit degraded Settings fixture query When the settings route mounts Then it exposes the degraded fixture state', async () => {
    await renderSettingsRoute('/settings?fixture=settings&degraded=opencode');

    expect(await screen.findByText('OpenCode agent discovery warning: OpenCode config is malformed.')).toBeInstanceOf(HTMLElement);
    expect(document.querySelector('[data-fixture-root="settings"]')).toBeInstanceOf(HTMLElement);
    expect(invoke).not.toHaveBeenCalled();
  });

  it('Given the native windows When their startup URLs are resolved Then Settings opens /settings and Quick Switch keeps the root default', () => {
    const settingsWindow = tauriConfig.app.windows.find((window) => window.label === 'settings');
    const quickSwitchWindow = tauriConfig.app.windows.find((window) => window.label === 'quick-switch');

    expect(settingsWindow).toMatchObject({ label: 'settings', url: '/settings' });
    expect(quickSwitchWindow).toMatchObject({ label: 'quick-switch' });
    expect(quickSwitchWindow).not.toHaveProperty('url');
  });
});
