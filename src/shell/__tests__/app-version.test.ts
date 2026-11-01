import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import { version } from '../../../package.json';
import AppSidebar from '../AppSidebar.svelte';
import MobileNavigation from '../MobileNavigation.svelte';

vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/settings') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));

test.each([
  ['en', 'Main navigation', 'Open navigation menu', 'Mobile navigation', `VERSION ${version}`],
  ['zh', '主导航', '打开导航菜单', '移动端导航', `版本 ${version}`],
] as const)(
  'sidebar and mobile drawer show the package version in %s',
  async (locale, sidebar, openMenu, drawer, label) => {
    const config = {
      preferences: { locale, theme: 'dark' },
      reloading: false,
      switching: false,
      reloadOpencode: vi.fn(),
    } as unknown as ConfigStore;

    render(AppSidebar, {}, { wrapper: Harness, wrapperProps: { config } });
    expect(within(screen.getByRole('complementary', { name: sidebar })).getByText(label)).toBeTruthy();
    render(MobileNavigation, {}, { wrapper: Harness, wrapperProps: { config } });
    await fireEvent.click(screen.getByRole('button', { name: openMenu }));
    expect(within(screen.getByRole('dialog', { name: drawer })).getByText(label)).toBeTruthy();
  },
);
