import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ProjectPage, UpdateCheck } from '$src/types/app.js';
import { version } from '../../../package.json';
import AppSidebar from '../AppSidebar.svelte';
import MobileNavigation from '../MobileNavigation.svelte';
import { toast } from '../toast.svelte.js';

vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/settings') } }));
vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
  beforeNavigate: vi.fn(),
  afterNavigate: vi.fn(),
}));
vi.mock('../toast.svelte.js', () => ({ toast: vi.fn() }));

const behind: UpdateCheck = { currentVersion: version, latestVersion: '9.9.9', updateAvailable: true };
const current: UpdateCheck = { currentVersion: version, latestVersion: null, updateAvailable: false };

function mount(locale: 'en' | 'zh' = 'en', updateCheck: UpdateCheck | null = null) {
  const config = {
    preferences: { locale, theme: 'dark' },
    reloading: false,
    switching: false,
    reloadOpencode: vi.fn(),
    openProjectPage: vi.fn<(page: ProjectPage) => Promise<void>>().mockResolvedValue(undefined),
    updateCheck,
  };
  const view = render(AppSidebar, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return { config, view };
}

async function openDrawer() {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    reloading: false,
    switching: false,
    reloadOpencode: vi.fn(),
    openProjectPage: vi.fn<(page: ProjectPage) => Promise<void>>().mockResolvedValue(undefined),
    updateCheck: null,
  };
  const view = render(
    MobileNavigation,
    {},
    { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } },
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Open navigation menu' }));
  return { config, view, drawer: screen.getByRole('dialog', { name: 'Mobile navigation' }) };
}

test.each([
  ['en', 'Main navigation', 'Open navigation menu', 'Mobile navigation'],
  ['zh', '主导航', '打开导航菜单', '移动端导航'],
] as const)(
  'sidebar and mobile drawer show the bare version without a label (%s)',
  async (locale, sidebar, openMenu, drawer) => {
    const sidebarConfig = {
      preferences: { locale, theme: 'dark' },
      reloading: false,
      switching: false,
      reloadOpencode: vi.fn(),
      openProjectPage: vi.fn(),
      updateCheck: null,
    };
    const sidebarView = render(
      AppSidebar,
      {},
      {
        wrapper: Harness,
        wrapperProps: { config: sidebarConfig as unknown as ConfigStore },
      },
    );
    const sidebarRegion = within(screen.getByRole('complementary', { name: sidebar }));
    expect(sidebarRegion.getByText(version, { exact: true })).toBeTruthy();
    expect(sidebarRegion.queryByText(/VERSION|版本/)).toBeNull();
    sidebarView.unmount();

    const drawerView = render(
      MobileNavigation,
      {},
      {
        wrapper: Harness,
        wrapperProps: { config: sidebarConfig as unknown as ConfigStore },
      },
    );
    await fireEvent.click(screen.getByRole('button', { name: openMenu }));
    const drawerRegion = within(screen.getByRole('dialog', { name: drawer }));
    expect(drawerRegion.getByText(version, { exact: true })).toBeTruthy();
    expect(drawerRegion.queryByText(/VERSION|版本/)).toBeNull();
    drawerView.unmount();
  },
);

test('sidebar and mobile drawer never render the OpenCode restart control', async () => {
  const { config, view } = mount();
  const sidebar = screen.getByRole('complementary', { name: 'Main navigation' });

  expect(within(sidebar).queryByRole('button')).toBeNull();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
  view.unmount();

  const { drawer } = await openDrawer();
  const drawerScope = within(drawer);
  // Drawer keeps its own close button; the restart action must not be one of the dialog buttons.
  for (const button of drawerScope.getAllByRole('button')) {
    expect(button.textContent?.trim()).not.toMatch(/Restart|Restarting/);
  }
  expect(drawerScope.queryByText('Reload OpenCode')).toBeNull();
});

test('no update hint while running the latest version', () => {
  const { view } = mount('en', current);
  const sidebar = within(screen.getByRole('complementary', { name: 'Main navigation' }));

  expect(sidebar.queryByText(/Update \d/)).toBeNull();
  expect(sidebar.queryByRole('button')).toBeNull();
  view.unmount();
});

test('update hint opens the release page once and guards repeated pending clicks', async () => {
  let finish: () => void = () => {};
  const { config } = mount('en', behind);
  config.openProjectPage = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );

  const hint = screen.getByRole('button', { name: 'Version 9.9.9 available — open the release page in your browser' });

  expect(hint.getAttribute('title')).toBe('Version 9.9.9 available — open the release page in your browser');
  expect(hint.textContent).toContain('Update 9.9.9');
  await fireEvent.click(hint);
  expect(config.openProjectPage).toHaveBeenCalledExactlyOnceWith('releases');
  expect((hint as HTMLButtonElement).disabled).toBe(true);
  expect(hint.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(hint);
  expect(config.openProjectPage).toHaveBeenCalledOnce();
  finish();
  await waitFor(() => expect((hint as HTMLButtonElement).disabled).toBe(false));
  expect(hint.getAttribute('aria-busy')).toBe('false');
  expect(toast).not.toHaveBeenCalled();
});

test('failed release opener reports a translated error and leaves the hint usable', async () => {
  const { config } = mount('en', behind);
  config.openProjectPage = vi
    .fn()
    .mockRejectedValueOnce(new Error('No default browser'))
    .mockResolvedValueOnce(undefined);

  const hint = screen.getByRole('button', { name: 'Version 9.9.9 available — open the release page in your browser' });

  await fireEvent.click(hint);
  await waitFor(() =>
    expect(toast).toHaveBeenCalledExactlyOnceWith({
      variant: 'error',
      description: 'Could not open GitHub in your default browser: No default browser',
    }),
  );
  expect((hint as HTMLButtonElement).disabled).toBe(false);
  await fireEvent.click(hint);
  await waitFor(() => expect(config.openProjectPage).toHaveBeenCalledTimes(2));
});

test('late opener failure after sidebar teardown does not create a stale toast', async () => {
  let reject: (cause: Error) => void = () => {};
  const { config, view } = mount('en', behind);
  config.openProjectPage = vi.fn(
    () =>
      new Promise<void>((_resolve, fail) => {
        reject = fail;
      }),
  );

  await fireEvent.click(screen.getByRole('button', { name: /Version 9\.9\.9 available/ }));
  view.unmount();
  reject(new Error('Late browser error'));
  await Promise.resolve();
  expect(toast).not.toHaveBeenCalled();
});

test('Chinese update hint is localized and opens the release page', async () => {
  const { config } = mount('zh', behind);
  const hint = screen.getByRole('button', { name: `发现新版本 9.9.9，在浏览器中打开发布页面` });

  expect(hint.textContent).toContain('新版本 9.9.9');
  await fireEvent.click(hint);
  expect(config.openProjectPage).toHaveBeenCalledExactlyOnceWith('releases');
});
