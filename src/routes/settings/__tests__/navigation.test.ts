import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import { page } from '$app/state';
import type { ConfigStore } from '$src/config/store.svelte.js';
import AppSidebar from '$src/shell/AppSidebar.svelte';
import MobileNavigation from '$src/shell/MobileNavigation.svelte';
import { isNavigationItemActive } from '$src/shell/navigation.js';
import Harness from '$src/test-utils/harness.svelte';

vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/mcp/docs%2Ftest') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
const config = {
  preferences: { locale: 'en', theme: 'dark' },
  reloading: false,
  switching: false,
  reloadOpencode: vi.fn(),
} as unknown as ConfigStore;

test('sidebar and mobile menu expose all new routes and detail active state', async () => {
  render(AppSidebar, {}, { wrapper: Harness, wrapperProps: { config } });
  for (const [name, href] of [
    ['MCP', '/mcp'],
    ['Skills', '/skills'],
    ['Settings', '/settings'],
  ]) {
    expect(screen.getByRole('link', { name }).getAttribute('href')).toBe(href);
  }
  expect(screen.getByRole('link', { name: 'MCP' }).getAttribute('aria-current')).toBe('page');
  render(MobileNavigation, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(screen.getByRole('button', { name: 'Open navigation menu' }));
  const menu = screen.getByRole('dialog', { name: 'Mobile navigation' });
  for (const [name, href] of [
    ['MCP', '/mcp'],
    ['Skills', '/skills'],
    ['Settings', '/settings'],
  ]) {
    expect(within(menu).getByRole('link', { name }).getAttribute('href')).toBe(href);
  }
  expect(within(menu).getByRole('link', { name: 'MCP' }).getAttribute('aria-current')).toBe('page');
  expect(isNavigationItemActive('/skills/docs%2Ftest', '/skills')).toBe(true);
  expect(isNavigationItemActive('/settings', '/settings')).toBe(true);
  expect(isNavigationItemActive('/mcproxy', '/mcp')).toBe(false);
  expect(isNavigationItemActive(page.url.pathname, '/')).toBe(false);
  const close = within(menu).getByRole('button', { name: 'Close navigation menu' });
  expect(document.activeElement).toBe(close);
  await fireEvent.keyDown(close, { key: 'Tab', shiftKey: true });
  expect(document.activeElement).toBe(within(menu).getByRole('button', { name: 'Reload OpenCode' }));
  await fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'Tab' });
  expect(document.activeElement).toBe(close);
  await fireEvent.keyDown(close, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Open navigation menu' }));
});
